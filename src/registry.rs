use std::io;
use std::ptr;

use windows_sys::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_MORE_DATA, ERROR_SUCCESS, FILETIME, WIN32_ERROR,
};
use windows_sys::Win32::System::Registry::{
    HKEY, REG_DWORD, REG_EXPAND_SZ, REG_MULTI_SZ, REG_OPTION_NON_VOLATILE, REG_SAM_FLAGS, REG_SZ,
    REG_VALUE_TYPE, RegCloseKey, RegCreateKeyExW, RegDeleteKeyW, RegDeleteTreeW, RegDeleteValueW,
    RegEnumKeyExW, RegOpenKeyExW, RegQueryInfoKeyW, RegQueryValueExW, RegSetValueExW,
};

use crate::wide::{from_wide, to_wide, units_from_bytes};

// Constants
const MAX_KEY_NAME: usize = 256;

// Key
pub struct RegistryKey(HKEY);

impl RegistryKey {
    pub fn open(parent: HKEY, path: &str, access: REG_SAM_FLAGS) -> io::Result<Self> {
        let path = to_wide(path);
        let mut handle = ptr::null_mut();
        check(unsafe { RegOpenKeyExW(parent, path.as_ptr(), 0, access, &raw mut handle) })?;
        Ok(Self(handle))
    }

    pub fn create(parent: HKEY, path: &str, access: REG_SAM_FLAGS) -> io::Result<Self> {
        let path = to_wide(path);
        let mut handle = ptr::null_mut();
        check(unsafe {
            RegCreateKeyExW(
                parent,
                path.as_ptr(),
                0,
                ptr::null(),
                REG_OPTION_NON_VOLATILE,
                access,
                ptr::null(),
                &raw mut handle,
                ptr::null_mut(),
            )
        })?;
        Ok(Self(handle))
    }

    pub fn open_subkey(&self, name: &str, access: REG_SAM_FLAGS) -> io::Result<Self> {
        Self::open(self.0, name, access)
    }

    pub fn handle(&self) -> HKEY {
        self.0
    }

    pub fn last_write_time(&self) -> u64 {
        let mut time = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let result = unsafe {
            RegQueryInfoKeyW(
                self.0,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                &raw mut time,
            )
        };
        if result == ERROR_SUCCESS {
            (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime)
        } else {
            0
        }
    }

    pub fn delete_subkey(&self, name: &str) -> io::Result<()> {
        Self::delete_tree(self.0, name)
    }

    pub fn delete_tree(parent: HKEY, path: &str) -> io::Result<()> {
        let path = to_wide(path);
        match unsafe { RegDeleteTreeW(parent, path.as_ptr()) } {
            ERROR_SUCCESS | ERROR_FILE_NOT_FOUND => {}
            error => return Err(os_error(error)),
        }
        match unsafe { RegDeleteKeyW(parent, path.as_ptr()) } {
            ERROR_SUCCESS | ERROR_FILE_NOT_FOUND => Ok(()),
            error => Err(os_error(error)),
        }
    }

    pub fn subkey_names(&self) -> io::Result<Vec<String>> {
        let mut names = Vec::new();
        let mut buffer = [0_u16; MAX_KEY_NAME];
        for index in 0.. {
            let mut length = u32::try_from(buffer.len()).unwrap_or(u32::MAX);
            let result = unsafe {
                RegEnumKeyExW(
                    self.0,
                    index,
                    buffer.as_mut_ptr(),
                    &raw mut length,
                    ptr::null(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            };
            match result {
                ERROR_SUCCESS => names.push(from_wide(&buffer[..length as usize])),
                windows_sys::Win32::Foundation::ERROR_NO_MORE_ITEMS => break,
                error => return Err(os_error(error)),
            }
        }
        Ok(names)
    }

    pub fn string(&self, name: &str) -> Option<String> {
        let (kind, data) = self.query(name)?;
        (kind == REG_SZ || kind == REG_EXPAND_SZ).then(|| from_wide(&units_from_bytes(&data)))
    }

    pub fn dword(&self, name: &str) -> Option<u32> {
        let (kind, data) = self.query(name)?;
        let bytes: [u8; 4] = data.get(..4)?.try_into().ok()?;
        (kind == REG_DWORD).then(|| u32::from_le_bytes(bytes))
    }

    pub fn multi_string(&self, name: &str) -> Option<Vec<String>> {
        let (kind, data) = self.query(name)?;
        if kind != REG_MULTI_SZ {
            return None;
        }
        Some(
            units_from_bytes(&data)
                .split(|&unit| unit == 0)
                .filter(|part| !part.is_empty())
                .map(String::from_utf16_lossy)
                .collect(),
        )
    }

    pub fn set_dword(&self, name: &str, value: u32) -> io::Result<()> {
        self.set(name, REG_DWORD, &value.to_le_bytes())
    }

    pub fn set_string(&self, name: &str, value: &str) -> io::Result<()> {
        self.set(name, REG_SZ, &bytes(&to_wide(value)))
    }

    pub fn set_multi_string(&self, name: &str, values: &[String]) -> io::Result<()> {
        let mut data: Vec<u16> = Vec::new();
        for value in values.iter().filter(|value| !value.is_empty()) {
            data.extend(value.encode_utf16());
            data.push(0);
        }
        data.push(0);
        if data.len() == 1 {
            data.push(0);
        }
        self.set(name, REG_MULTI_SZ, &bytes(&data))
    }

    pub fn delete_value(&self, name: &str) -> io::Result<()> {
        let name = to_wide(name);
        match unsafe { RegDeleteValueW(self.0, name.as_ptr()) } {
            ERROR_SUCCESS | ERROR_FILE_NOT_FOUND => Ok(()),
            error => Err(os_error(error)),
        }
    }

    fn set(&self, name: &str, kind: REG_VALUE_TYPE, data: &[u8]) -> io::Result<()> {
        let name = to_wide(name);
        let length =
            u32::try_from(data.len()).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
        check(unsafe { RegSetValueExW(self.0, name.as_ptr(), 0, kind, data.as_ptr(), length) })
    }

    fn query(&self, name: &str) -> Option<(REG_VALUE_TYPE, Vec<u8>)> {
        let name = to_wide(name);
        let mut data = vec![0_u8; 256];
        loop {
            let mut kind = 0;
            let mut length = u32::try_from(data.len()).ok()?;
            let result = unsafe {
                RegQueryValueExW(
                    self.0,
                    name.as_ptr(),
                    ptr::null(),
                    &raw mut kind,
                    data.as_mut_ptr(),
                    &raw mut length,
                )
            };
            match result {
                ERROR_SUCCESS => {
                    data.truncate(length as usize);
                    return Some((kind, data));
                }
                ERROR_MORE_DATA => data.resize(length as usize, 0),
                _ => return None,
            }
        }
    }
}

impl Drop for RegistryKey {
    fn drop(&mut self) {
        unsafe {
            RegCloseKey(self.0);
        }
    }
}

// Helpers
fn check(result: WIN32_ERROR) -> io::Result<()> {
    if result == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(os_error(result))
    }
}

fn os_error(error: WIN32_ERROR) -> io::Error {
    io::Error::from_raw_os_error(i32::try_from(error).unwrap_or(i32::MAX))
}

fn bytes(units: &[u16]) -> Vec<u8> {
    units.iter().flat_map(|unit| unit.to_le_bytes()).collect()
}

// Tests
#[cfg(test)]
mod tests {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, KEY_ALL_ACCESS};

    use super::*;

    struct TestKey(String);

    impl Drop for TestKey {
        fn drop(&mut self) {
            let _ = RegistryKey::delete_tree(HKEY_CURRENT_USER, &self.0);
        }
    }

    fn test_key(name: &str) -> (TestKey, RegistryKey) {
        let path = format!(r"Software\TrayReveal-Test-{}-{name}", std::process::id());
        let key = RegistryKey::create(HKEY_CURRENT_USER, &path, KEY_ALL_ACCESS).unwrap();
        (TestKey(path), key)
    }

    #[test]
    fn writes_and_reads_values() {
        let (_guard, key) = test_key("values");
        key.set_dword("Number", 7).unwrap();
        key.set_string("Text", "Grüße").unwrap();
        key.set_multi_string("List", &["a".into(), "bc".into()])
            .unwrap();
        assert_eq!(key.dword("Number"), Some(7));
        assert_eq!(key.string("Text").as_deref(), Some("Grüße"));
        assert_eq!(
            key.multi_string("List"),
            Some(vec!["a".into(), "bc".into()])
        );
        assert_eq!(key.string("Number"), None);
        key.delete_value("Number").unwrap();
        key.delete_value("Number").unwrap();
        assert_eq!(key.dword("Number"), None);
    }

    #[test]
    fn writes_an_empty_list() {
        let (_guard, key) = test_key("empty");
        key.set_multi_string("List", &[]).unwrap();
        assert_eq!(key.multi_string("List"), Some(Vec::new()));
    }

    #[test]
    fn lists_subkeys() {
        let (_guard, key) = test_key("subkeys");
        RegistryKey::create(key.handle(), "First", KEY_ALL_ACCESS).unwrap();
        RegistryKey::create(key.handle(), "Second", KEY_ALL_ACCESS).unwrap();
        let mut names = key.subkey_names().unwrap();
        names.sort();
        assert_eq!(names, ["First", "Second"]);
        assert!(
            key.open_subkey("First", KEY_ALL_ACCESS)
                .unwrap()
                .last_write_time()
                > 0
        );
    }

    #[test]
    fn deletes_subkeys_with_their_values() {
        let (_guard, key) = test_key("delete");
        let child = RegistryKey::create(key.handle(), "Child", KEY_ALL_ACCESS).unwrap();
        child.set_dword("Value", 1).unwrap();
        drop(child);
        key.delete_subkey("Child").unwrap();
        key.delete_subkey("Child").unwrap();
        assert_eq!(key.subkey_names().unwrap(), Vec::<String>::new());
    }
}
