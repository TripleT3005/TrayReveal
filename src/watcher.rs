use std::ptr;
use std::thread;
use std::time::Duration;

use windows_sys::Win32::Foundation::{CloseHandle, ERROR_SUCCESS, HANDLE, HWND, WAIT_OBJECT_0};
use windows_sys::Win32::System::Registry::{
    HKEY_CURRENT_USER, KEY_NOTIFY, REG_NOTIFY_CHANGE_LAST_SET, REG_NOTIFY_CHANGE_NAME,
    RegNotifyChangeKeyValue,
};
use windows_sys::Win32::System::Threading::{CreateEventW, INFINITE, WaitForSingleObject};
use windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW;

use crate::notify_icons::SETTINGS_PATH;
use crate::registry::RegistryKey;

// Constants
const RETRY_DELAY: Duration = Duration::from_secs(30);

// Types
struct Event(HANDLE);

impl Drop for Event {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

// Watching
pub fn spawn(window: HWND, message: u32) {
    let window = window as usize;
    thread::spawn(move || {
        loop {
            watch(window as HWND, message);
            thread::sleep(RETRY_DELAY);
            notify(window as HWND, message);
        }
    });
}

fn watch(window: HWND, message: u32) {
    let Ok(key) = RegistryKey::open(HKEY_CURRENT_USER, SETTINGS_PATH, KEY_NOTIFY) else {
        return;
    };
    let handle = unsafe { CreateEventW(ptr::null(), 0, 0, ptr::null()) };
    if handle.is_null() {
        return;
    }
    let event = Event(handle);
    loop {
        let result = unsafe {
            RegNotifyChangeKeyValue(
                key.handle(),
                1,
                REG_NOTIFY_CHANGE_NAME | REG_NOTIFY_CHANGE_LAST_SET,
                event.0,
                1,
            )
        };
        if result != ERROR_SUCCESS {
            return;
        }
        if unsafe { WaitForSingleObject(event.0, INFINITE) } != WAIT_OBJECT_0 {
            return;
        }
        if !notify(window, message) {
            return;
        }
    }
}

fn notify(window: HWND, message: u32) -> bool {
    unsafe { PostMessageW(window, message, 0, 0) != 0 }
}
