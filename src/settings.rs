use std::collections::BTreeSet;
use std::io;

use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};

use crate::engine::Mode;
use crate::registry::RegistryKey;

// Constants
const SETTINGS_PATH: &str = r"Software\TrayReveal";
const MODE: &str = "Mode";
const NOTIFICATIONS: &str = "Notifications";
const LANGUAGE: &str = "Language";
const EXCEPTIONS: &str = "Exceptions";
const KNOWN_ICONS: &str = "KnownIcons";

// Settings
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub mode: Mode,
    pub notifications: bool,
    pub language: Option<String>,
    pub exceptions: BTreeSet<String>,
    pub known: Option<BTreeSet<String>>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: Mode::default(),
            notifications: true,
            language: None,
            exceptions: BTreeSet::new(),
            known: None,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let Ok(key) = RegistryKey::open(HKEY_CURRENT_USER, SETTINGS_PATH, KEY_READ) else {
            return Self::default();
        };
        let defaults = Self::default();
        Self {
            mode: key
                .dword(MODE)
                .and_then(mode_from_value)
                .unwrap_or(defaults.mode),
            notifications: key
                .dword(NOTIFICATIONS)
                .map_or(defaults.notifications, |value| value != 0),
            language: key.string(LANGUAGE).filter(|language| !language.is_empty()),
            exceptions: key
                .multi_string(EXCEPTIONS)
                .unwrap_or_default()
                .into_iter()
                .collect(),
            known: key
                .multi_string(KNOWN_ICONS)
                .map(|ids| ids.into_iter().collect()),
        }
    }

    pub fn save(&self) -> io::Result<()> {
        let key = open_for_writing()?;
        key.set_dword(MODE, mode_to_value(self.mode))?;
        key.set_dword(NOTIFICATIONS, u32::from(self.notifications))?;
        match &self.language {
            Some(language) => key.set_string(LANGUAGE, language)?,
            None => key.delete_value(LANGUAGE)?,
        }
        key.set_multi_string(
            EXCEPTIONS,
            &self.exceptions.iter().cloned().collect::<Vec<_>>(),
        )?;
        self.save_known_with(&key)
    }

    pub fn save_known(&self) -> io::Result<()> {
        self.save_known_with(&open_for_writing()?)
    }

    fn save_known_with(&self, key: &RegistryKey) -> io::Result<()> {
        match &self.known {
            Some(known) => {
                key.set_multi_string(KNOWN_ICONS, &known.iter().cloned().collect::<Vec<_>>())
            }
            None => key.delete_value(KNOWN_ICONS),
        }
    }
}

fn open_for_writing() -> io::Result<RegistryKey> {
    RegistryKey::create(HKEY_CURRENT_USER, SETTINGS_PATH, KEY_READ | KEY_WRITE)
}

fn mode_from_value(value: u32) -> Option<Mode> {
    match value {
        0 => Some(Mode::Off),
        1 => Some(Mode::NewIcons),
        2 => Some(Mode::AllIcons),
        _ => None,
    }
}

fn mode_to_value(mode: Mode) -> u32 {
    match mode {
        Mode::Off => 0,
        Mode::NewIcons => 1,
        Mode::AllIcons => 2,
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_every_mode() {
        for mode in [Mode::Off, Mode::NewIcons, Mode::AllIcons] {
            assert_eq!(mode_from_value(mode_to_value(mode)), Some(mode));
        }
        assert_eq!(mode_from_value(9), None);
    }

    #[test]
    fn defaults_to_showing_new_icons_with_notifications() {
        let settings = Settings::default();
        assert_eq!(settings.mode, Mode::NewIcons);
        assert!(settings.notifications);
        assert!(settings.known.is_none());
    }
}
