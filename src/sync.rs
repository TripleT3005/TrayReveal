use std::collections::BTreeSet;
use std::io;
use std::path::PathBuf;

use crate::engine::{self, Change, Entry};
use crate::file_info::display_name;
use crate::known_folders;
use crate::notify_icons::{self, NotifyIcon};
use crate::settings::Settings;

// Types
#[derive(Clone, Debug)]
pub struct App {
    pub key: String,
    pub name: String,
    pub tooltip: String,
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
    if settings.known.as_ref() != Some(&plan.known) {
        settings.known = Some(plan.known);
        settings.save_known()?;
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
        });
    }
    apps.sort_by_cached_key(|app| (app.name.to_lowercase(), app.tooltip.to_lowercase()));
    Ok(apps)
}
