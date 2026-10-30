use std::env;
use std::path::PathBuf;

// Constants
const APP_FOLDER: &str = "TrayReveal";
const LANGUAGE_FOLDER: &str = "lang";

// Paths
pub fn executable() -> Option<PathBuf> {
    env::current_exe().ok()
}

pub fn language_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if cfg!(debug_assertions) {
        directories.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(LANGUAGE_FOLDER));
    }
    if let Some(directory) = executable().as_deref().and_then(|path| path.parent()) {
        directories.push(directory.join(LANGUAGE_FOLDER));
    }
    if let Some(directory) = env::var_os("APPDATA") {
        directories.push(
            PathBuf::from(directory)
                .join(APP_FOLDER)
                .join(LANGUAGE_FOLDER),
        );
    }
    directories
}
