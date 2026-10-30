use std::io;

use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};

use crate::paths;
use crate::registry::RegistryKey;

// Constants
const RUN_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "TrayReveal";

// Autostart
pub fn is_enabled() -> bool {
    let Some(command) = command() else {
        return false;
    };
    RegistryKey::open(HKEY_CURRENT_USER, RUN_PATH, KEY_READ)
        .ok()
        .and_then(|key| key.string(VALUE_NAME))
        .is_some_and(|value| value.eq_ignore_ascii_case(&command))
}

pub fn set_enabled(enabled: bool) -> io::Result<()> {
    let key = RegistryKey::create(HKEY_CURRENT_USER, RUN_PATH, KEY_READ | KEY_WRITE)?;
    if enabled {
        let command = command().ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        key.set_string(VALUE_NAME, &command)
    } else {
        key.delete_value(VALUE_NAME)
    }
}

fn command() -> Option<String> {
    paths::executable().map(|path| format!("\"{}\"", path.display()))
}
