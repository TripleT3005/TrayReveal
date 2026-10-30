use std::mem;
use std::ptr;

use windows_sys::Win32::Foundation::{FARPROC, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{
    GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW,
};
use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, KEY_READ};

use crate::registry::RegistryKey;
use crate::wide::to_wide;

// Constants
const UXTHEME: &str = "uxtheme.dll";
const SET_PREFERRED_APP_MODE: u16 = 135;
const FLUSH_MENU_THEMES: u16 = 136;
const FORCE_DARK: i32 = 2;
const FORCE_LIGHT: i32 = 3;
const PERSONALIZE_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
const SYSTEM_USES_LIGHT_THEME: &str = "SystemUsesLightTheme";
pub const COLOR_SET_CHANGED: &str = "ImmersiveColorSet";

// Types
type SetPreferredAppMode = unsafe extern "system" fn(i32) -> i32;
type FlushMenuThemes = unsafe extern "system" fn();

// Theme
pub fn apply() {
    let module = unsafe {
        LoadLibraryExW(
            to_wide(UXTHEME).as_ptr(),
            ptr::null_mut(),
            LOAD_LIBRARY_SEARCH_SYSTEM32,
        )
    };
    if module.is_null() {
        return;
    }
    let mode = if uses_dark_theme() {
        FORCE_DARK
    } else {
        FORCE_LIGHT
    };
    if let Some(function) = ordinal(module, SET_PREFERRED_APP_MODE) {
        let set_preferred_app_mode: SetPreferredAppMode = unsafe { mem::transmute(function) };
        unsafe {
            set_preferred_app_mode(mode);
        }
    }
    if let Some(function) = ordinal(module, FLUSH_MENU_THEMES) {
        let flush_menu_themes: FlushMenuThemes = unsafe { mem::transmute(function) };
        unsafe {
            flush_menu_themes();
        }
    }
}

fn ordinal(module: HMODULE, ordinal: u16) -> FARPROC {
    unsafe { GetProcAddress(module, usize::from(ordinal) as *const u8) }
}

fn uses_dark_theme() -> bool {
    RegistryKey::open(HKEY_CURRENT_USER, PERSONALIZE_PATH, KEY_READ)
        .ok()
        .and_then(|key| key.dword(SYSTEM_USES_LIGHT_THEME))
        .is_some_and(|value| value == 0)
}
