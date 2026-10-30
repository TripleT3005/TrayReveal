use std::io;

use windows_sys::Win32::System::Registry::{
    HKEY_CURRENT_USER, KEY_ALL_ACCESS, KEY_READ, KEY_SET_VALUE,
};

use crate::engine::{Change, Entry};
use crate::identity::identity_key;
use crate::registry::RegistryKey;

// Constants
pub const SETTINGS_PATH: &str = r"Control Panel\NotifyIconSettings";
const EXECUTABLE_PATH: &str = "ExecutablePath";
const IS_PROMOTED: &str = "IsPromoted";
const INITIAL_TOOLTIP: &str = "InitialTooltip";
const ICON_GUID: &str = "IconGuid";
const UID: &str = "UID";
const ICON_SNAPSHOT: &str = "IconSnapshot";

// Types
#[derive(Clone, Debug)]
pub struct NotifyIcon {
    pub id: String,
    pub executable_path: String,
    pub tooltip: String,
    pub key: String,
    pub promoted: Option<bool>,
    pub updated: u64,
    pub snapshot: Vec<u8>,
}

impl NotifyIcon {
    pub fn entry(&self) -> Entry {
        Entry {
            id: self.id.clone(),
            key: self.key.clone(),
            promoted: self.promoted,
            updated: self.updated,
        }
    }
}

// Reading
pub fn read_all() -> io::Result<Vec<NotifyIcon>> {
    let settings = RegistryKey::open(HKEY_CURRENT_USER, SETTINGS_PATH, KEY_READ)?;
    let mut icons = Vec::new();
    for id in settings.subkey_names()? {
        let Ok(key) = settings.open_subkey(&id, KEY_READ) else {
            continue;
        };
        let Some(executable_path) = key
            .string(EXECUTABLE_PATH)
            .filter(|path| !path.trim().is_empty())
        else {
            continue;
        };
        let slot = key
            .string(ICON_GUID)
            .filter(|guid| !guid.trim().is_empty())
            .or_else(|| key.dword(UID).map(|uid| uid.to_string()));
        icons.push(NotifyIcon {
            key: identity_key(&executable_path, slot.as_deref()),
            promoted: key.dword(IS_PROMOTED).map(|value| value != 0),
            updated: key.last_write_time(),
            snapshot: key.binary(ICON_SNAPSHOT).unwrap_or_default(),
            tooltip: key
                .string(INITIAL_TOOLTIP)
                .unwrap_or_default()
                .trim()
                .to_owned(),
            executable_path,
            id,
        });
    }
    Ok(icons)
}

// Writing
pub fn apply(changes: &[Change]) -> io::Result<()> {
    if changes.is_empty() {
        return Ok(());
    }
    let settings = RegistryKey::open(HKEY_CURRENT_USER, SETTINGS_PATH, KEY_READ)?;
    let mut last_error = None;
    for change in changes {
        let result = settings
            .open_subkey(&change.id, KEY_SET_VALUE)
            .and_then(|key| key.set_dword(IS_PROMOTED, u32::from(change.promoted)));
        if let Err(error) = result
            && error.kind() != io::ErrorKind::NotFound
        {
            last_error = Some(error);
        }
    }
    last_error.map_or(Ok(()), Err)
}

pub fn delete(ids: &[String]) -> io::Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let settings = RegistryKey::open(HKEY_CURRENT_USER, SETTINGS_PATH, KEY_ALL_ACCESS)?;
    let mut last_error = None;
    for id in ids {
        if let Err(error) = settings.delete_subkey(id) {
            last_error = Some(error);
        }
    }
    last_error.map_or(Ok(()), Err)
}
