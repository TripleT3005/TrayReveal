#![cfg_attr(not(test), windows_subsystem = "windows")]

mod app;
mod autostart;
mod engine;
mod file_info;
mod i18n;
#[cfg(test)]
mod icon;
mod identity;
mod instance;
mod known_folders;
mod notify_icons;
mod paths;
mod registry;
mod settings;
mod sync;
mod theme;
mod tray;
mod watcher;
mod wide;

use std::env;
use std::process::ExitCode;

use settings::Settings;

// Constants
const ONCE_ARGUMENT: &str = "--once";

// Entry point
fn main() -> ExitCode {
    if env::args()
        .skip(1)
        .any(|argument| argument == ONCE_ARGUMENT)
    {
        return run_once();
    }
    let Some(_instance) = instance::acquire() else {
        app::signal_running_instance();
        return ExitCode::SUCCESS;
    };
    app::run()
}

fn run_once() -> ExitCode {
    let mut settings = Settings::load();
    match sync::synchronize(&mut settings) {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
