use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::engine::{self, Change, Cleanup, Entry, Mode, Presence};
use crate::file_info::display_name;
use crate::known_folders;
use crate::notify_icons::{self, NotifyIcon};
use crate::settings::Settings;

// Constants
const STALE_GRACE_PERIOD: Duration = Duration::from_secs(3 * 24 * 60 * 60);

// Types
#[derive(Clone, Debug)]
pub struct App {
    pub key: String,
    pub name: String,
    pub tooltip: String,
    pub snapshot: Vec<u8>,
}

// Synchronization
pub fn synchronize(settings: &mut Settings) -> io::Result<Vec<String>> {
    let icons = notify_icons::read_all()?;
    let entries = entries(&icons);
    let plan = engine::plan(
        &entries,
        settings.known.as_ref(),
        settings.mode,
        &settings.exceptions,
    );
    notify_icons::apply(&plan.changes)?;
    let cleanup = if settings.mode == Mode::Off {
        Cleanup {
            missing_since: settings.missing_since.clone(),
            ..Cleanup::default()
        }
    } else {
        engine::cleanup(
            &presence(&icons),
            &settings.missing_since,
            now(),
            STALE_GRACE_PERIOD.as_secs(),
        )
    };
    notify_icons::delete(&cleanup.delete)?;
    let mut known = plan.known;
    known.retain(|id| !cleanup.delete.contains(id));
    if settings.known.as_ref() != Some(&known) || settings.missing_since != cleanup.missing_since {
        settings.known = Some(known);
        settings.missing_since = cleanup.missing_since;
        settings.save_state()?;
    }
    Ok(plan
        .revealed
        .iter()
        .filter_map(|id| icons.iter().find(|icon| &icon.id == id))
        .map(|icon| display_name(&known_folders::resolve(&icon.executable_path)))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect())
}

pub fn set_all(settings: &Settings, promoted: bool) -> io::Result<()> {
    change(|entries| engine::set_all(entries, &settings.exceptions, promoted))
}

pub fn set_app(key: &str, promoted: bool) -> io::Result<()> {
    change(|entries| engine::set_key(entries, key, promoted))
}

fn change(plan: impl FnOnce(&[Entry]) -> Vec<Change>) -> io::Result<()> {
    let icons = notify_icons::read_all()?;
    notify_icons::apply(&plan(&entries(&icons)))
}

fn entries(icons: &[NotifyIcon]) -> Vec<Entry> {
    icons.iter().map(NotifyIcon::entry).collect()
}

fn presence(icons: &[NotifyIcon]) -> Vec<Presence> {
    icons
        .iter()
        .map(|icon| Presence {
            id: icon.id.clone(),
            exists: file_presence(&known_folders::resolve(&icon.executable_path)),
        })
        .collect()
}

fn file_presence(path: &Path) -> Option<bool> {
    if !path.is_absolute() {
        return None;
    }
    let root = path.ancestors().last()?;
    root.exists().then(|| path.is_file())
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

// Apps
pub fn installed_apps() -> io::Result<Vec<App>> {
    let mut apps: Vec<App> = Vec::new();
    for icon in notify_icons::read_all()? {
        if apps.iter().any(|app| app.key == icon.key) {
            continue;
        }
        let path: PathBuf = known_folders::resolve(&icon.executable_path);
        if !path.is_file() {
            continue;
        }
        apps.push(App {
            name: display_name(&path),
            key: icon.key,
            tooltip: icon.tooltip,
            snapshot: icon.snapshot,
        });
    }
    apps.sort_by_cached_key(|app| (app.name.to_lowercase(), app.tooltip.to_lowercase()));
    Ok(apps)
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_whether_a_program_still_exists() {
        let windows = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        let windows = Path::new(&windows);
        assert_eq!(file_presence(&windows.join("explorer.exe")), Some(true));
        assert_eq!(
            file_presence(&windows.join("TrayRevealMissing.exe")),
            Some(false)
        );
        assert_eq!(file_presence(Path::new(r"{0000}\app.exe")), None);
    }
}
