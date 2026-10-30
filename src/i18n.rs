use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::ptr;

use windows_sys::Win32::Globalization::{GetUserPreferredUILanguages, MUI_LANGUAGE_NAME};

use crate::paths;

// Constants
const ENGLISH: &str = include_str!("../lang/en.lang");
const ENGLISH_CODE: &str = "en";
const EXTENSION: &str = "lang";
const MAX_FILE_SIZE: u64 = 256 * 1024;
const MAX_CODE_LENGTH: usize = 16;
const RIGHT_TO_LEFT: &str = "rtl";
const SIMPLIFIED_CHINESE: &str = "zh-Hans";
const TRADITIONAL_CHINESE: &str = "zh-Hant";
const TRADITIONAL_CHINESE_REGIONS: [&str; 3] = ["TW", "HK", "MO"];

pub const LANGUAGE_NAME: &str = "language.name";
pub const LANGUAGE_FALLBACK: &str = "language.fallback";
pub const LANGUAGE_DIRECTION: &str = "language.direction";
pub const MENU_MODE_OFF: &str = "menu.mode.off";
pub const MENU_MODE_NEW_ICONS: &str = "menu.mode.new_icons";
pub const MENU_MODE_ALL_ICONS: &str = "menu.mode.all_icons";
pub const MENU_SHOW_ALL: &str = "menu.show_all";
pub const MENU_HIDE_ALL: &str = "menu.hide_all";
pub const MENU_EXCEPTIONS: &str = "menu.exceptions";
pub const MENU_EXCEPTIONS_EMPTY: &str = "menu.exceptions.empty";
pub const MENU_EXCEPTIONS_HINT: &str = "menu.exceptions.hint";
pub const MENU_NOTIFICATIONS: &str = "menu.notifications";
pub const MENU_LANGUAGE: &str = "menu.language";
pub const MENU_LANGUAGE_AUTOMATIC: &str = "menu.language.automatic";
pub const MENU_AUTOSTART: &str = "menu.autostart";
pub const MENU_QUIT: &str = "menu.quit";
pub const NOTIFICATION_REVEALED_TITLE: &str = "notification.revealed.title";
pub const NOTIFICATION_REVEALED_ONE: &str = "notification.revealed.one";
pub const NOTIFICATION_REVEALED_MANY: &str = "notification.revealed.many";
pub const NOTIFICATION_ALREADY_RUNNING: &str = "notification.already_running";
pub const NOTIFICATION_UNAVAILABLE: &str = "notification.unavailable";

#[cfg(test)]
const KEYS: [&str; 19] = [
    LANGUAGE_NAME,
    MENU_MODE_OFF,
    MENU_MODE_NEW_ICONS,
    MENU_MODE_ALL_ICONS,
    MENU_SHOW_ALL,
    MENU_HIDE_ALL,
    MENU_EXCEPTIONS,
    MENU_EXCEPTIONS_EMPTY,
    MENU_NOTIFICATIONS,
    MENU_LANGUAGE,
    MENU_LANGUAGE_AUTOMATIC,
    MENU_AUTOSTART,
    MENU_QUIT,
    NOTIFICATION_REVEALED_TITLE,
    NOTIFICATION_REVEALED_ONE,
    NOTIFICATION_REVEALED_MANY,
    NOTIFICATION_ALREADY_RUNNING,
    NOTIFICATION_UNAVAILABLE,
    MENU_EXCEPTIONS_HINT,
];

#[cfg(test)]
const OPTIONAL_KEYS: [&str; 2] = [LANGUAGE_FALLBACK, LANGUAGE_DIRECTION];

// Types
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Language {
    pub code: String,
    pub name: String,
}

pub struct Translations {
    texts: HashMap<String, String>,
}

// Translations
impl Translations {
    pub fn load(preference: Option<&str>) -> Self {
        let directories = paths::language_directories();
        let codes = available_codes(&directories);
        let code = resolve_code(preference, &preferred_ui_languages(), &codes);
        Self {
            texts: load_texts(&code, &directories),
        }
    }

    pub fn text(&self, key: &str) -> String {
        self.texts
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_owned())
    }

    pub fn format(&self, key: &str, values: &[(&str, &str)]) -> String {
        values.iter().fold(self.text(key), |text, (name, value)| {
            text.replace(&format!("{{{name}}}"), value)
        })
    }

    pub fn is_right_to_left(&self) -> bool {
        self.texts
            .get(LANGUAGE_DIRECTION)
            .is_some_and(|direction| direction.eq_ignore_ascii_case(RIGHT_TO_LEFT))
    }
}

pub fn available_languages() -> Vec<Language> {
    let directories = paths::language_directories();
    let mut languages: Vec<Language> = available_codes(&directories)
        .into_iter()
        .map(|code| {
            let name = load_texts(&code, &directories)
                .remove(LANGUAGE_NAME)
                .unwrap_or_else(|| code.clone());
            Language { code, name }
        })
        .collect();
    languages.sort_by_key(|language| language.name.to_lowercase());
    languages
}

fn load_texts(code: &str, directories: &[PathBuf]) -> HashMap<String, String> {
    let fallback = read_language(code, directories)
        .remove(LANGUAGE_FALLBACK)
        .filter(|fallback| is_valid_code(fallback));
    let mut texts = parse(ENGLISH);
    for code in language_chain(code, fallback.as_deref()) {
        texts.extend(read_language(code, directories));
    }
    texts
}

fn read_language(code: &str, directories: &[PathBuf]) -> HashMap<String, String> {
    let mut texts = HashMap::new();
    for directory in directories {
        if let Some(content) = read_file(&directory.join(format!("{code}.{EXTENSION}"))) {
            texts.extend(parse(&content));
        }
    }
    texts
}

fn language_chain<'a>(code: &'a str, fallback: Option<&'a str>) -> Vec<&'a str> {
    let mut chain = Vec::new();
    let candidates = fallback
        .map(fallback_chain)
        .unwrap_or_default()
        .into_iter()
        .chain(fallback_chain(code));
    for candidate in candidates {
        if !chain
            .iter()
            .any(|known: &&str| known.eq_ignore_ascii_case(candidate))
        {
            chain.push(candidate);
        }
    }
    chain
}

// Languages
fn available_codes(directories: &[PathBuf]) -> Vec<String> {
    let mut codes = vec![ENGLISH_CODE.to_owned()];
    for directory in directories {
        let Ok(files) = fs::read_dir(directory) else {
            continue;
        };
        for file in files.flatten() {
            let path = file.path();
            if !path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case(EXTENSION))
            {
                continue;
            }
            let Some(code) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            if is_valid_code(code) && !codes.iter().any(|known| known.eq_ignore_ascii_case(code)) {
                codes.push(code.to_owned());
            }
        }
    }
    codes
}

fn resolve_code(preference: Option<&str>, preferred: &[String], available: &[String]) -> String {
    let find = |code: &str| {
        available
            .iter()
            .find(|candidate| candidate.eq_ignore_ascii_case(code))
            .cloned()
    };
    preference
        .into_iter()
        .chain(preferred.iter().map(String::as_str))
        .find_map(|code| candidates(code).into_iter().find_map(find))
        .unwrap_or_else(|| ENGLISH_CODE.to_owned())
}

fn candidates(code: &str) -> Vec<&str> {
    let mut candidates: Vec<&str> = code
        .char_indices()
        .filter(|&(_, character)| character == '-')
        .map(|(index, _)| &code[..index])
        .rev()
        .collect();
    candidates.insert(0, code);
    if let Some(script) = script_code(code) {
        candidates.insert(1, script);
    }
    candidates
}

fn script_code(code: &str) -> Option<&'static str> {
    let mut parts = code.split('-');
    if !parts.next()?.eq_ignore_ascii_case("zh") {
        return None;
    }
    let traditional = parts.any(|part| {
        part.eq_ignore_ascii_case("Hant")
            || TRADITIONAL_CHINESE_REGIONS
                .iter()
                .any(|region| part.eq_ignore_ascii_case(region))
    });
    Some(if traditional {
        TRADITIONAL_CHINESE
    } else {
        SIMPLIFIED_CHINESE
    })
}

fn fallback_chain(code: &str) -> Vec<&str> {
    let mut chain = vec![ENGLISH_CODE];
    for candidate in [base_code(code), code] {
        if !chain
            .iter()
            .any(|known| known.eq_ignore_ascii_case(candidate))
        {
            chain.push(candidate);
        }
    }
    chain
}

fn base_code(code: &str) -> &str {
    code.split('-').next().unwrap_or(code)
}

fn is_valid_code(code: &str) -> bool {
    !code.is_empty()
        && code.len() <= MAX_CODE_LENGTH
        && code
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
}

fn preferred_ui_languages() -> Vec<String> {
    let mut count = 0_u32;
    let mut length = 0_u32;
    if unsafe {
        GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &raw mut count,
            ptr::null_mut(),
            &raw mut length,
        )
    } == 0
    {
        return Vec::new();
    }
    let mut buffer = vec![0_u16; length as usize];
    if unsafe {
        GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &raw mut count,
            buffer.as_mut_ptr(),
            &raw mut length,
        )
    } == 0
    {
        return Vec::new();
    }
    buffer
        .split(|&unit| unit == 0)
        .filter(|part| !part.is_empty())
        .map(String::from_utf16_lossy)
        .collect()
}

// Files
fn read_file(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_SIZE {
        return None;
    }
    fs::read_to_string(path).ok()
}

pub fn parse(content: &str) -> HashMap<String, String> {
    content
        .trim_start_matches('\u{feff}')
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with(';'))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim(), value.trim()))
        .filter(|(key, value)| !key.is_empty() && !value.is_empty())
        .map(|(key, value)| (key.to_owned(), value.replace("\\n", "\n")))
        .collect()
}

// Tests
#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn codes(values: &[&str]) -> Vec<String> {
        values.iter().map(|&value| value.to_owned()).collect()
    }

    fn placeholders(text: &str) -> BTreeSet<String> {
        text.split('{')
            .skip(1)
            .filter_map(|part| part.split_once('}'))
            .map(|(name, _)| name.to_owned())
            .collect()
    }

    #[test]
    fn parses_language_files() {
        let texts = parse(
            "\u{feff}# Comment\n; Comment\nmenu.quit = Quit \nbroken\nempty =\nmulti = One\\nTwo\n",
        );
        assert_eq!(texts.len(), 2);
        assert_eq!(texts["menu.quit"], "Quit");
        assert_eq!(texts["multi"], "One\nTwo");
    }

    #[test]
    fn resolves_the_language() {
        let available = codes(&["en", "de", "pt-BR"]);
        assert_eq!(
            resolve_code(None, &codes(&["de-AT", "en-US"]), &available),
            "de"
        );
        assert_eq!(resolve_code(None, &codes(&["pt-br"]), &available), "pt-BR");
        assert_eq!(resolve_code(None, &codes(&["fr-FR"]), &available), "en");
        assert_eq!(
            resolve_code(Some("en"), &codes(&["de-DE"]), &available),
            "en"
        );
        assert_eq!(
            resolve_code(Some("xx"), &codes(&["de-DE"]), &available),
            "de"
        );
    }

    #[test]
    fn shortens_windows_language_codes() {
        let available = codes(&["en", "sr-Latn", "nb"]);
        assert_eq!(
            resolve_code(None, &codes(&["sr-Latn-RS"]), &available),
            "sr-Latn"
        );
        assert_eq!(resolve_code(None, &codes(&["nb-NO"]), &available), "nb");
        assert_eq!(candidates("sr-Latn-RS"), ["sr-Latn-RS", "sr-Latn", "sr"]);
        assert_eq!(candidates("zh-TW"), ["zh-TW", "zh-Hant", "zh"]);
    }

    #[test]
    fn maps_chinese_regions_to_scripts() {
        let available = codes(&["en", "zh-Hans", "zh-Hant"]);
        assert_eq!(
            resolve_code(None, &codes(&["zh-CN"]), &available),
            "zh-Hans"
        );
        assert_eq!(
            resolve_code(None, &codes(&["zh-SG"]), &available),
            "zh-Hans"
        );
        assert_eq!(
            resolve_code(None, &codes(&["zh-TW"]), &available),
            "zh-Hant"
        );
        assert_eq!(
            resolve_code(None, &codes(&["zh-HK"]), &available),
            "zh-Hant"
        );
        assert_eq!(
            resolve_code(None, &codes(&["zh-Hant-TW"]), &available),
            "zh-Hant"
        );
        assert_eq!(script_code("de-DE"), None);
    }

    #[test]
    fn falls_back_through_the_declared_language() {
        assert_eq!(language_chain("sxu", Some("de")), ["en", "de", "sxu"]);
        assert_eq!(
            language_chain("de-x-amtsdt", Some("de")),
            ["en", "de", "de-x-amtsdt"]
        );
        assert_eq!(language_chain("de-CH", None), ["en", "de", "de-CH"]);
    }

    #[test]
    fn reads_the_declared_fallback_and_direction() {
        let directory =
            std::env::temp_dir().join(format!("trayreveal-fallback-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("de.lang"),
            "menu.quit = Beenden\nmenu.language = Sprache\n",
        )
        .unwrap();
        fs::write(
            directory.join("yy.lang"),
            "language.fallback = de\nmenu.quit = Feierabend\n",
        )
        .unwrap();
        fs::write(directory.join("zz.lang"), "language.direction = RTL\n").unwrap();
        let texts = load_texts("yy", std::slice::from_ref(&directory));
        let right_to_left = Translations {
            texts: load_texts("zz", std::slice::from_ref(&directory)),
        };
        fs::remove_dir_all(&directory).unwrap();
        assert_eq!(texts[MENU_QUIT], "Feierabend");
        assert_eq!(texts[MENU_LANGUAGE], "Sprache");
        assert_eq!(texts[MENU_SHOW_ALL], parse(ENGLISH)[MENU_SHOW_ALL]);
        assert!(right_to_left.is_right_to_left());
    }

    #[test]
    fn falls_back_through_english() {
        assert_eq!(fallback_chain("de-AT"), ["en", "de", "de-AT"]);
        assert_eq!(fallback_chain("de"), ["en", "de"]);
        assert_eq!(fallback_chain("en"), ["en"]);
    }

    #[test]
    fn falls_back_per_text() {
        let directory =
            std::env::temp_dir().join(format!("trayreveal-lang-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("xx.lang"), "menu.quit = Exit now\n").unwrap();
        let texts = load_texts("xx", std::slice::from_ref(&directory));
        fs::remove_dir_all(&directory).unwrap();
        assert_eq!(texts[MENU_QUIT], "Exit now");
        assert_eq!(texts[MENU_SHOW_ALL], parse(ENGLISH)[MENU_SHOW_ALL]);
    }

    #[test]
    fn formats_placeholders() {
        let translations = Translations {
            texts: parse("greeting = Hello {name}, {name}!"),
        };
        assert_eq!(
            translations.format("greeting", &[("name", "Ada")]),
            "Hello Ada, Ada!"
        );
        assert_eq!(translations.text("missing"), "missing");
    }

    #[test]
    fn english_contains_every_text() {
        let english = parse(ENGLISH);
        for key in KEYS {
            assert!(english.contains_key(key), "en.lang is missing {key}");
        }
        assert_eq!(
            english.len(),
            KEYS.len(),
            "en.lang contains texts that are not used"
        );
    }

    #[test]
    fn bundled_languages_match_english() {
        let english = parse(ENGLISH);
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("lang");
        for file in fs::read_dir(directory).unwrap().flatten() {
            let path = file.path();
            let texts = parse(&fs::read_to_string(&path).unwrap());
            let name = path.display();
            assert!(
                is_valid_code(path.file_stem().unwrap().to_str().unwrap()),
                "{name} has an invalid name"
            );
            for (key, text) in &texts {
                if OPTIONAL_KEYS.contains(&key.as_str()) {
                    continue;
                }
                let Some(original) = english.get(key) else {
                    panic!("{name} contains the unknown text {key}");
                };
                assert_eq!(
                    placeholders(text),
                    placeholders(original),
                    "{name}: {key} has other placeholders"
                );
            }
            for key in english.keys() {
                assert!(texts.contains_key(key), "{name} is missing {key}");
            }
        }
    }
}
