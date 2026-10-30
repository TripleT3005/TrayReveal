// Constants
const VERSION_PLACEHOLDER: &str = "*";
const MIN_HASH_LENGTH: usize = 12;

// Identity
pub fn identity_key(executable_path: &str, slot: Option<&str>) -> String {
    let path = normalize_path(executable_path);
    match slot.map(str::trim).filter(|slot| !slot.is_empty()) {
        Some(slot) => format!("{path}|{}", slot.to_lowercase()),
        None => path,
    }
}

pub fn normalize_path(path: &str) -> String {
    path.trim()
        .split(['\\', '/'])
        .filter(|segment| !segment.is_empty())
        .map(normalize_segment)
        .collect::<Vec<_>>()
        .join("\\")
}

fn normalize_segment(segment: &str) -> String {
    let lower = segment.to_lowercase();
    if lower.starts_with('{') && lower.ends_with('}') {
        return lower;
    }
    let mut normalized = String::with_capacity(lower.len());
    let mut token = String::new();
    for character in lower.chars() {
        if character.is_alphanumeric() || character == '.' {
            token.push(character);
        } else {
            push_token(&mut normalized, &token);
            token.clear();
            normalized.push(character);
        }
    }
    push_token(&mut normalized, &token);
    normalized
}

fn push_token(normalized: &mut String, token: &str) {
    if is_version(token) || is_hash(token) {
        normalized.push_str(VERSION_PLACEHOLDER);
    } else {
        normalized.push_str(token);
    }
}

fn is_version(token: &str) -> bool {
    let token = token.strip_prefix('v').unwrap_or(token);
    let mut parts = 0;
    for part in token.split('.') {
        if part.is_empty() || !part.chars().all(|character| character.is_ascii_digit()) {
            return false;
        }
        parts += 1;
    }
    parts >= 2
}

fn is_hash(token: &str) -> bool {
    token.len() >= MIN_HASH_LENGTH
        && token.chars().all(|character| character.is_ascii_hexdigit())
        && token.chars().any(|character| character.is_ascii_digit())
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_versioned_folders() {
        assert_eq!(
            normalize_path(r"C:\Users\Name\AppData\Local\Discord\app-1.0.9265\Discord.exe"),
            normalize_path(r"C:\Users\Name\AppData\Local\Discord\app-1.0.9312\Discord.exe"),
        );
        assert_eq!(
            normalize_path(r"C:\Tools\Example-1.0.1-windows-x64\Example.exe"),
            r"c:\tools\example-*-windows-x64\example.exe"
        );
    }

    #[test]
    fn ignores_package_versions() {
        let old = r"{6D809377-6AF0-444B-8957-A3773F02200E}\WindowsApps\5319275A.WhatsAppDesktop_2.2587.8.0_x64__cv1g1gvanyjgm\WhatsApp.Root.exe";
        let new = r"{6D809377-6AF0-444B-8957-A3773F02200E}\WindowsApps\5319275A.WhatsAppDesktop_2.2587.9.0_x64__cv1g1gvanyjgm\WhatsApp.Root.exe";
        assert_eq!(normalize_path(old), normalize_path(new));
        assert_eq!(
            normalize_path(old),
            r"{6d809377-6af0-444b-8957-a3773f02200e}\windowsapps\5319275a.whatsappdesktop_*_x64__cv1g1gvanyjgm\whatsapp.root.exe"
        );
    }

    #[test]
    fn ignores_driver_store_hashes() {
        assert_eq!(
            normalize_path(
                r"{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\DriverStore\FileRepository\nvmii.inf_amd64_52a4c0ca7914a7b3\Display.NvContainer\NVDisplay.Container.exe"
            ),
            normalize_path(
                r"{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\DriverStore\FileRepository\nvmii.inf_amd64_a55e7dc894061997\Display.NvContainer\NVDisplay.Container.exe"
            ),
        );
    }

    #[test]
    fn keeps_different_programs_apart() {
        assert_ne!(
            normalize_path(r"C:\Program Files\Vendor A\Tool\tool.exe"),
            normalize_path(r"C:\Program Files\Vendor B\Tool\tool.exe"),
        );
        assert_ne!(
            normalize_path(r"C:\Program Files\Microsoft Office\root\Office16\OUTLOOK.EXE"),
            normalize_path(r"C:\Program Files\Microsoft Office\root\Office15\OUTLOOK.EXE"),
        );
        assert_eq!(
            normalize_path(r"C:\Program Files\obs-studio\bin\64bit\obs64.exe"),
            r"c:\program files\obs-studio\bin\64bit\obs64.exe"
        );
    }

    #[test]
    fn adds_the_icon_slot() {
        assert_eq!(identity_key(r"C:\A\b.exe", Some("12")), r"c:\a\b.exe|12");
        assert_eq!(
            identity_key(r"C:\A\b.exe", Some("{ABC-1}")),
            r"c:\a\b.exe|{abc-1}"
        );
        assert_eq!(identity_key(r"C:\A\b.exe", Some(" ")), r"c:\a\b.exe");
        assert_eq!(identity_key(r"C:\A\b.exe", None), r"c:\a\b.exe");
    }
}
