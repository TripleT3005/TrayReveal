use std::path::PathBuf;
use std::ptr;

use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::UI::Shell::{KF_FLAG_DEFAULT, SHGetKnownFolderPath};
use windows_sys::core::GUID;

// Constants
const GUID_TEXT_LENGTH: usize = 38;
const MAX_PATH_LENGTH: usize = 32_767;

// Resolution
pub fn resolve(path: &str) -> PathBuf {
    let Some((folder, rest)) = split_known_folder(path) else {
        return PathBuf::from(path);
    };
    match known_folder_path(&folder) {
        Some(base) => base.join(rest),
        None => PathBuf::from(path),
    }
}

fn split_known_folder(path: &str) -> Option<(GUID, &str)> {
    let guid = path.get(..GUID_TEXT_LENGTH)?;
    let rest = path.get(GUID_TEXT_LENGTH..)?;
    let rest = rest.strip_prefix('\\').unwrap_or(rest);
    Some((parse_guid(guid)?, rest))
}

fn parse_guid(text: &str) -> Option<GUID> {
    let inner = text.strip_prefix('{')?.strip_suffix('}')?;
    let groups: Vec<&str> = inner.split('-').collect();
    let lengths: Vec<usize> = groups.iter().map(|group| group.len()).collect();
    if lengths != [8, 4, 4, 4, 12] {
        return None;
    }
    let digits = groups.concat();
    if !digits
        .chars()
        .all(|character| character.is_ascii_hexdigit())
    {
        return None;
    }
    u128::from_str_radix(&digits, 16).ok().map(GUID::from_u128)
}

fn known_folder_path(folder: &GUID) -> Option<PathBuf> {
    let mut buffer = ptr::null_mut();
    let flags = u32::try_from(KF_FLAG_DEFAULT).unwrap_or(0);
    let result = unsafe { SHGetKnownFolderPath(folder, flags, ptr::null_mut(), &raw mut buffer) };
    if buffer.is_null() {
        return None;
    }
    let path = (result >= 0).then(|| {
        let length = (0..MAX_PATH_LENGTH)
            .take_while(|&index| unsafe { *buffer.add(index) } != 0)
            .count();
        let units = unsafe { std::slice::from_raw_parts(buffer, length) };
        PathBuf::from(String::from_utf16_lossy(units))
    });
    unsafe { CoTaskMemFree(buffer.cast()) };
    path
}

// Tests
#[cfg(test)]
mod tests {
    use windows_sys::Win32::UI::Shell::FOLDERID_ProgramFilesX64;

    use super::*;

    fn same(left: &GUID, right: &GUID) -> bool {
        (left.data1, left.data2, left.data3, left.data4)
            == (right.data1, right.data2, right.data3, right.data4)
    }

    #[test]
    fn parses_known_folder_ids() {
        let guid = parse_guid("{6D809377-6AF0-444B-8957-A3773F02200E}").unwrap();
        assert!(same(&guid, &FOLDERID_ProgramFilesX64));
        assert!(parse_guid("{6D809377-6AF0-444B-8957-A3773F02200}").is_none());
        assert!(parse_guid("{6D809377-6AF0-444B-8957-A3773F02200X}").is_none());
        assert!(parse_guid("6D809377-6AF0-444B-8957-A3773F02200E").is_none());
    }

    #[test]
    fn splits_the_folder_from_the_rest() {
        let (_, rest) =
            split_known_folder(r"{6D809377-6AF0-444B-8957-A3773F02200E}\Vendor\app.exe").unwrap();
        assert_eq!(rest, r"Vendor\app.exe");
        assert!(split_known_folder(r"C:\Vendor\app.exe").is_none());
    }

    #[test]
    fn resolves_program_files() {
        let path = resolve(r"{6D809377-6AF0-444B-8957-A3773F02200E}\Vendor\app.exe");
        assert!(path.is_absolute());
        assert!(path.ends_with(r"Vendor\app.exe"));
        assert_eq!(
            resolve(r"C:\Tools\app.exe"),
            PathBuf::from(r"C:\Tools\app.exe")
        );
    }
}
