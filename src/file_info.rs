use std::ffi::c_void;
use std::path::Path;
use std::ptr;

use windows_sys::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
};

use crate::wide::{from_wide, to_wide, units_from_bytes};

// Constants
const TRANSLATION: &str = r"\VarFileInfo\Translation";
const FALLBACK_TRANSLATIONS: [(u16, u16); 2] = [(0x0409, 0x04B0), (0x0409, 0x04E4)];

// Names
pub fn display_name(path: &Path) -> String {
    file_description(path)
        .or_else(|| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn file_description(path: &Path) -> Option<String> {
    let file = to_wide(&path.to_string_lossy());
    let mut handle = 0;
    let size = unsafe { GetFileVersionInfoSizeW(file.as_ptr(), &raw mut handle) };
    if size == 0 {
        return None;
    }
    let mut data = vec![0_u8; size as usize];
    if unsafe { GetFileVersionInfoW(file.as_ptr(), 0, size, data.as_mut_ptr().cast()) } == 0 {
        return None;
    }
    let mut translations: Vec<(u16, u16)> = query(&data, TRANSLATION, 1)
        .map(|bytes| {
            units_from_bytes(bytes)
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&[language, code_page]| (language, code_page))
                .collect()
        })
        .unwrap_or_default();
    translations.extend(FALLBACK_TRANSLATIONS);
    translations.into_iter().find_map(|(language, code_page)| {
        let name = format!(r"\StringFileInfo\{language:04x}{code_page:04x}\FileDescription");
        let units = units_from_bytes(query(&data, &name, 2)?);
        Some(from_wide(&units).trim().to_owned()).filter(|text| !text.is_empty())
    })
}

fn query<'a>(data: &'a [u8], name: &str, unit_size: usize) -> Option<&'a [u8]> {
    let name = to_wide(name);
    let mut value: *mut c_void = ptr::null_mut();
    let mut length = 0_u32;
    let found = unsafe {
        VerQueryValueW(
            data.as_ptr().cast(),
            name.as_ptr(),
            &raw mut value,
            &raw mut length,
        )
    };
    if found == 0 || value.is_null() || length == 0 {
        return None;
    }
    let offset = (value as usize).checked_sub(data.as_ptr() as usize)?;
    let byte_length = (length as usize).checked_mul(unit_size)?;
    data.get(offset..offset.checked_add(byte_length)?)
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_description_of_a_system_program() {
        let windows = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        let name = display_name(&Path::new(&windows).join("explorer.exe"));
        assert_ne!(name, "");
        assert_ne!(name, "explorer");
    }

    #[test]
    fn falls_back_to_the_file_name() {
        assert_eq!(
            display_name(Path::new(r"C:\Missing\Some Tool.exe")),
            "Some Tool"
        );
    }
}
