use std::cell::RefCell;
use std::mem;
use std::process::ExitCode;
use std::ptr;
use std::sync::atomic::{AtomicU32, Ordering};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::NIN_SELECT;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CheckMenuRadioItem, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
    DestroyWindow, DispatchMessageW, FindWindowW, GetCursorPos, GetMessageW, HMENU, KillTimer,
    MF_BYCOMMAND, MF_CHECKED, MF_GRAYED, MF_POPUP, MF_SEPARATOR, MF_STRING, MSG,
    PBT_APMRESUMEAUTOMATIC, PostMessageW, PostQuitMessage, RegisterClassExW,
    RegisterWindowMessageW, SetForegroundWindow, SetTimer, TPM_NONOTIFY, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, TrackPopupMenu, TranslateMessage, WM_APP, WM_CONTEXTMENU, WM_DESTROY, WM_NULL,
    WM_POWERBROADCAST, WM_SETTINGCHANGE, WM_TIMER, WNDCLASSEXW, WS_OVERLAPPED,
};

use crate::autostart;
use crate::engine::Mode;
use crate::i18n::{self, Language, Translations};
use crate::settings::Settings;
use crate::sync::{self, App as InstalledApp};
use crate::theme;
use crate::tray::TrayIcon;
use crate::wide::{from_wide, to_wide};

// Constants
const WINDOW_CLASS: &str = "TrayReveal.Window";
const WINDOW_TITLE: &str = "TrayReveal";
const WM_TRAY: u32 = WM_APP + 1;
const WM_REGISTRY_CHANGED: u32 = WM_APP + 2;
const WM_ALREADY_RUNNING: u32 = WM_APP + 3;
const NIN_KEYSELECT: u32 = NIN_SELECT | 1;
const TIMER_DEBOUNCE: usize = 1;
const TIMER_PERIODIC: usize = 2;
const DEBOUNCE_DELAY_MS: u32 = 500;
const PERIODIC_DELAY_MS: u32 = 15 * 60 * 1000;
const MAX_NAMES_IN_NOTIFICATION: usize = 5;

const ID_MODE_OFF: u32 = 1;
const ID_MODE_NEW_ICONS: u32 = 2;
const ID_MODE_ALL_ICONS: u32 = 3;
const ID_SHOW_ALL: u32 = 10;
const ID_HIDE_ALL: u32 = 11;
const ID_NOTIFICATIONS: u32 = 12;
const ID_AUTOSTART: u32 = 13;
const ID_QUIT: u32 = 14;
const ID_LANGUAGE_AUTOMATIC: u32 = 100;
const ID_LANGUAGE_FIRST: u32 = 101;
const ID_EXCEPTION_FIRST: u32 = 1000;

// State
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

struct State {
    tray: TrayIcon,
    settings: Settings,
    translations: Translations,
    unavailable: bool,
}

struct MenuModel {
    texts: MenuTexts,
    mode: Mode,
    notifications: bool,
    autostart: bool,
    language: Option<String>,
    languages: Vec<Language>,
    apps: Vec<InstalledApp>,
    exceptions: Vec<bool>,
}

struct MenuTexts {
    mode_off: String,
    mode_new_icons: String,
    mode_all_icons: String,
    show_all: String,
    hide_all: String,
    exceptions: String,
    exceptions_hint: String,
    exceptions_empty: String,
    notifications: String,
    language: String,
    language_automatic: String,
    autostart: String,
    quit: String,
}

struct Menu(HMENU);

impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            DestroyMenu(self.0);
        }
    }
}

fn with_state<R>(action: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|state| state.try_borrow_mut().ok()?.as_mut().map(action))
}

// Running instance
pub fn signal_running_instance() {
    let class = to_wide(WINDOW_CLASS);
    let window = unsafe { FindWindowW(class.as_ptr(), ptr::null()) };
    if !window.is_null() {
        unsafe {
            PostMessageW(window, WM_ALREADY_RUNNING, 0, 0);
        }
    }
}

// Application
pub fn run() -> ExitCode {
    let Some(window) = create_window() else {
        return ExitCode::FAILURE;
    };
    let taskbar_created = to_wide("TaskbarCreated");
    TASKBAR_CREATED.store(
        unsafe { RegisterWindowMessageW(taskbar_created.as_ptr()) },
        Ordering::Relaxed,
    );

    let settings = Settings::load();
    let translations = Translations::load(settings.language.as_deref());
    let mut state = State {
        tray: TrayIcon::new(window),
        settings,
        translations,
        unavailable: false,
    };
    theme::apply();
    crate::watcher::spawn(window, WM_REGISTRY_CHANGED);
    state.synchronize();
    state.tray.add(WM_TRAY);
    STATE.with(|cell| *cell.borrow_mut() = Some(state));
    schedule_synchronization(window);
    unsafe {
        SetTimer(window, TIMER_PERIODIC, PERIODIC_DELAY_MS, None);
    }

    let mut message: MSG = unsafe { mem::zeroed() };
    while unsafe { GetMessageW(&raw mut message, ptr::null_mut(), 0, 0) } > 0 {
        unsafe {
            TranslateMessage(&raw const message);
            DispatchMessageW(&raw const message);
        }
    }
    STATE.with(|cell| cell.borrow_mut().take());
    ExitCode::SUCCESS
}

fn create_window() -> Option<HWND> {
    let instance = unsafe { GetModuleHandleW(ptr::null()) };
    let class = to_wide(WINDOW_CLASS);
    let title = to_wide(WINDOW_TITLE);
    let description = WNDCLASSEXW {
        cbSize: u32::try_from(mem::size_of::<WNDCLASSEXW>()).unwrap_or(0),
        lpfnWndProc: Some(window_procedure),
        hInstance: instance,
        lpszClassName: class.as_ptr(),
        ..unsafe { mem::zeroed() }
    };
    if unsafe { RegisterClassExW(&raw const description) } == 0 {
        return None;
    }
    let window = unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            instance,
            ptr::null(),
        )
    };
    (!window.is_null()).then_some(window)
}

// Messages
unsafe extern "system" fn window_procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_TRAY => {
            let event = u32::try_from(lparam & 0xFFFF).unwrap_or(0);
            if matches!(event, WM_CONTEXTMENU | NIN_SELECT | NIN_KEYSELECT) {
                show_menu(window);
            }
            0
        }
        WM_REGISTRY_CHANGED => {
            schedule_synchronization(window);
            0
        }
        WM_TIMER => {
            if wparam == TIMER_DEBOUNCE {
                unsafe {
                    KillTimer(window, TIMER_DEBOUNCE);
                }
            }
            if with_state(State::synchronize).is_none() {
                schedule_synchronization(window);
            }
            0
        }
        WM_ALREADY_RUNNING => {
            with_state(|state| {
                let text = state.translations.text(i18n::NOTIFICATION_ALREADY_RUNNING);
                state.tray.notify(WINDOW_TITLE, &text);
            });
            0
        }
        WM_SETTINGCHANGE => {
            if is_color_set_change(lparam) {
                theme::apply();
            }
            0
        }
        WM_POWERBROADCAST => {
            if wparam == PBT_APMRESUMEAUTOMATIC as usize {
                schedule_synchronization(window);
            }
            1
        }
        WM_DESTROY => {
            with_state(|state| state.tray.remove());
            unsafe {
                PostQuitMessage(0);
            }
            0
        }
        _ if message != 0 && message == TASKBAR_CREATED.load(Ordering::Relaxed) => {
            with_state(|state| state.tray.add(WM_TRAY));
            schedule_synchronization(window);
            0
        }
        _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
    }
}

fn is_color_set_change(lparam: LPARAM) -> bool {
    if lparam == 0 {
        return false;
    }
    let text = lparam as *const u16;
    let length = (0..=theme::COLOR_SET_CHANGED.len())
        .take_while(|&index| unsafe { *text.add(index) } != 0)
        .count();
    let units = unsafe { std::slice::from_raw_parts(text, length) };
    from_wide(units) == theme::COLOR_SET_CHANGED
}

fn schedule_synchronization(window: HWND) {
    unsafe {
        SetTimer(window, TIMER_DEBOUNCE, DEBOUNCE_DELAY_MS, None);
    }
}

// Behavior
impl State {
    fn synchronize(&mut self) {
        let Ok(revealed) = sync::synchronize(&mut self.settings) else {
            self.report(false);
            return;
        };
        self.report(true);
        if self.settings.notifications && !revealed.is_empty() {
            self.notify_revealed(&revealed);
        }
    }

    fn notify_revealed(&self, names: &[String]) {
        let title = self.translations.text(i18n::NOTIFICATION_REVEALED_TITLE);
        let text = if let [name] = names {
            self.translations
                .format(i18n::NOTIFICATION_REVEALED_ONE, &[("name", name)])
        } else {
            let mut listed = names
                .iter()
                .take(MAX_NAMES_IN_NOTIFICATION)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            if names.len() > MAX_NAMES_IN_NOTIFICATION {
                listed.push_str(", …");
            }
            self.translations.format(
                i18n::NOTIFICATION_REVEALED_MANY,
                &[("count", &names.len().to_string()), ("names", &listed)],
            )
        };
        self.tray.notify(&title, &text);
    }

    fn report(&mut self, succeeded: bool) {
        if succeeded {
            self.unavailable = false;
        } else if !self.unavailable {
            self.unavailable = true;
            let text = self.translations.text(i18n::NOTIFICATION_UNAVAILABLE);
            self.tray.notify(WINDOW_TITLE, &text);
        }
    }

    fn save(&mut self) {
        let succeeded = self.settings.save().is_ok();
        self.report(succeeded);
    }

    fn execute(&mut self, command: u32, model: &MenuModel) {
        match command {
            ID_MODE_OFF | ID_MODE_NEW_ICONS | ID_MODE_ALL_ICONS => {
                self.settings.mode = match command {
                    ID_MODE_OFF => Mode::Off,
                    ID_MODE_ALL_ICONS => Mode::AllIcons,
                    _ => Mode::NewIcons,
                };
                self.save();
                self.synchronize();
            }
            ID_SHOW_ALL | ID_HIDE_ALL => {
                let succeeded = sync::set_all(&self.settings, command == ID_SHOW_ALL).is_ok();
                self.report(succeeded);
            }
            ID_NOTIFICATIONS => {
                self.settings.notifications = !self.settings.notifications;
                self.save();
            }
            ID_AUTOSTART => {
                let succeeded = autostart::set_enabled(!model.autostart).is_ok();
                self.report(succeeded);
            }
            ID_LANGUAGE_AUTOMATIC => self.set_language(None),
            _ if command >= ID_EXCEPTION_FIRST => {
                if let Some(app) =
                    index(command, ID_EXCEPTION_FIRST).and_then(|index| model.apps.get(index))
                {
                    self.toggle_exception(&app.key);
                }
            }
            _ if command >= ID_LANGUAGE_FIRST => {
                if let Some(language) =
                    index(command, ID_LANGUAGE_FIRST).and_then(|index| model.languages.get(index))
                {
                    self.set_language(Some(language.code.clone()));
                }
            }
            _ => {}
        }
    }

    fn set_language(&mut self, language: Option<String>) {
        self.settings.language = language;
        self.save();
        self.translations = Translations::load(self.settings.language.as_deref());
    }

    fn toggle_exception(&mut self, key: &str) {
        let excepted = !self.settings.exceptions.remove(key);
        if excepted {
            self.settings.exceptions.insert(key.to_owned());
        }
        self.save();
        if excepted || self.settings.mode != Mode::Off {
            let succeeded = sync::set_app(key, !excepted).is_ok();
            self.report(succeeded);
        }
    }

    fn menu_model(&self) -> MenuModel {
        let text = |key| self.translations.text(key);
        let apps = sync::installed_apps().unwrap_or_default();
        MenuModel {
            texts: MenuTexts {
                mode_off: text(i18n::MENU_MODE_OFF),
                mode_new_icons: text(i18n::MENU_MODE_NEW_ICONS),
                mode_all_icons: text(i18n::MENU_MODE_ALL_ICONS),
                show_all: text(i18n::MENU_SHOW_ALL),
                hide_all: text(i18n::MENU_HIDE_ALL),
                exceptions: text(i18n::MENU_EXCEPTIONS),
                exceptions_hint: text(i18n::MENU_EXCEPTIONS_HINT),
                exceptions_empty: text(i18n::MENU_EXCEPTIONS_EMPTY),
                notifications: text(i18n::MENU_NOTIFICATIONS),
                language: text(i18n::MENU_LANGUAGE),
                language_automatic: text(i18n::MENU_LANGUAGE_AUTOMATIC),
                autostart: text(i18n::MENU_AUTOSTART),
                quit: text(i18n::MENU_QUIT),
            },
            mode: self.settings.mode,
            notifications: self.settings.notifications,
            autostart: autostart::is_enabled(),
            language: self.settings.language.clone(),
            languages: i18n::available_languages(),
            exceptions: apps
                .iter()
                .map(|app| self.settings.exceptions.contains(&app.key))
                .collect(),
            apps,
        }
    }
}

fn index(command: u32, first: u32) -> Option<usize> {
    usize::try_from(command.checked_sub(first)?).ok()
}

// Menu
fn show_menu(window: HWND) {
    let Some(model) = with_state(|state| state.menu_model()) else {
        return;
    };
    let Some(menu) = build_menu(&model) else {
        return;
    };
    let mut point = POINT { x: 0, y: 0 };
    let command = unsafe {
        GetCursorPos(&raw mut point);
        SetForegroundWindow(window);
        let command = TrackPopupMenu(
            menu.0,
            TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_NONOTIFY,
            point.x,
            point.y,
            0,
            window,
            ptr::null(),
        );
        PostMessageW(window, WM_NULL, 0, 0);
        command
    };
    drop(menu);
    let Ok(command) = u32::try_from(command) else {
        return;
    };
    if command == ID_QUIT {
        unsafe {
            DestroyWindow(window);
        }
    } else if command != 0 {
        with_state(|state| state.execute(command, &model));
    }
}

fn build_menu(model: &MenuModel) -> Option<Menu> {
    let menu = new_menu()?;
    let texts = &model.texts;
    append(&menu, MF_STRING, ID_MODE_OFF, &texts.mode_off);
    append(&menu, MF_STRING, ID_MODE_NEW_ICONS, &texts.mode_new_icons);
    append(&menu, MF_STRING, ID_MODE_ALL_ICONS, &texts.mode_all_icons);
    let mode = match model.mode {
        Mode::Off => ID_MODE_OFF,
        Mode::NewIcons => ID_MODE_NEW_ICONS,
        Mode::AllIcons => ID_MODE_ALL_ICONS,
    };
    unsafe {
        CheckMenuRadioItem(menu.0, ID_MODE_OFF, ID_MODE_ALL_ICONS, mode, MF_BYCOMMAND);
    }
    separator(&menu);
    append(&menu, MF_STRING, ID_SHOW_ALL, &texts.show_all);
    let hide_flags = if model.mode == Mode::AllIcons {
        MF_GRAYED
    } else {
        MF_STRING
    };
    append(&menu, hide_flags, ID_HIDE_ALL, &texts.hide_all);
    append_submenu(&menu, exceptions_menu(model)?, &texts.exceptions);
    separator(&menu);
    append(
        &menu,
        checked(model.notifications),
        ID_NOTIFICATIONS,
        &texts.notifications,
    );
    append_submenu(&menu, languages_menu(model)?, &texts.language);
    append(
        &menu,
        checked(model.autostart),
        ID_AUTOSTART,
        &texts.autostart,
    );
    separator(&menu);
    append(&menu, MF_STRING, ID_QUIT, &texts.quit);
    Some(menu)
}

fn exceptions_menu(model: &MenuModel) -> Option<Menu> {
    let menu = new_menu()?;
    if model.apps.is_empty() {
        append(&menu, MF_GRAYED, 0, &model.texts.exceptions_empty);
        return Some(menu);
    }
    append(&menu, MF_GRAYED, 0, &model.texts.exceptions_hint);
    separator(&menu);
    let labels = app_labels(&model.apps);
    for (index, (label, &excepted)) in labels.iter().zip(&model.exceptions).enumerate() {
        let id = ID_EXCEPTION_FIRST + u32::try_from(index).ok()?;
        append(&menu, checked(excepted), id, label);
    }
    Some(menu)
}

fn languages_menu(model: &MenuModel) -> Option<Menu> {
    let menu = new_menu()?;
    append(
        &menu,
        MF_STRING,
        ID_LANGUAGE_AUTOMATIC,
        &model.texts.language_automatic,
    );
    separator(&menu);
    let mut selected = ID_LANGUAGE_AUTOMATIC;
    let mut last = ID_LANGUAGE_AUTOMATIC;
    for (index, language) in model.languages.iter().enumerate() {
        let id = ID_LANGUAGE_FIRST + u32::try_from(index).ok()?;
        append(&menu, MF_STRING, id, &language.name);
        if model
            .language
            .as_deref()
            .is_some_and(|code| code.eq_ignore_ascii_case(&language.code))
        {
            selected = id;
        }
        last = id;
    }
    unsafe {
        CheckMenuRadioItem(menu.0, ID_LANGUAGE_AUTOMATIC, last, selected, MF_BYCOMMAND);
    }
    Some(menu)
}

fn app_labels(apps: &[InstalledApp]) -> Vec<String> {
    let labels: Vec<String> = apps
        .iter()
        .map(|app| {
            let duplicate = apps
                .iter()
                .filter(|other| other.name.eq_ignore_ascii_case(&app.name))
                .count()
                > 1;
            if duplicate && !app.tooltip.is_empty() && !app.tooltip.eq_ignore_ascii_case(&app.name)
            {
                format!("{} – {}", app.name, app.tooltip)
            } else {
                app.name.clone()
            }
        })
        .collect();
    labels
        .iter()
        .enumerate()
        .map(|(index, label)| {
            let same = |other: &&String| other.eq_ignore_ascii_case(label);
            if labels.iter().filter(same).count() > 1 {
                let number = labels[..index].iter().filter(same).count() + 1;
                format!("{label} ({number})")
            } else {
                label.clone()
            }
        })
        .collect()
}

fn new_menu() -> Option<Menu> {
    let menu = unsafe { CreatePopupMenu() };
    (!menu.is_null()).then_some(Menu(menu))
}

fn append(menu: &Menu, flags: u32, id: u32, text: &str) {
    let text = to_wide(&escape(text));
    unsafe {
        AppendMenuW(menu.0, flags, id as usize, text.as_ptr());
    }
}

fn append_submenu(menu: &Menu, submenu: Menu, text: &str) {
    let text = to_wide(&escape(text));
    let handle = submenu.0;
    mem::forget(submenu);
    unsafe {
        AppendMenuW(menu.0, MF_POPUP, handle as usize, text.as_ptr());
    }
}

fn separator(menu: &Menu) {
    unsafe {
        AppendMenuW(menu.0, MF_SEPARATOR, 0, ptr::null());
    }
}

fn checked(value: bool) -> u32 {
    if value {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&&")
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    fn app(name: &str, tooltip: &str) -> InstalledApp {
        InstalledApp {
            key: format!("{name}|{tooltip}"),
            name: name.into(),
            tooltip: tooltip.into(),
        }
    }

    #[test]
    fn labels_apps_with_the_same_name_by_their_tooltip() {
        let apps = [
            app("Windows Explorer", "Volume"),
            app("Windows Explorer", "Network"),
            app("Steam", "Steam"),
        ];
        assert_eq!(
            app_labels(&apps),
            [
                "Windows Explorer – Volume",
                "Windows Explorer – Network",
                "Steam"
            ]
        );
    }

    #[test]
    fn numbers_apps_that_cannot_be_told_apart() {
        let apps = [
            app("Windows Explorer", ""),
            app("Windows Explorer", ""),
            app("Windows Explorer", "Volume"),
        ];
        assert_eq!(
            app_labels(&apps),
            [
                "Windows Explorer (1)",
                "Windows Explorer (2)",
                "Windows Explorer – Volume"
            ]
        );
    }

    #[test]
    fn escapes_menu_accelerators() {
        assert_eq!(escape("Tom & Jerry"), "Tom && Jerry");
    }
}
