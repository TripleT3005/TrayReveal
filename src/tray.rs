use std::mem;
use std::ptr;

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIIF_RESPECT_QUIET_TIME, NIIF_USER,
    NIM_ADD, NIM_DELETE, NIM_MODIFY, NIM_SETVERSION, NOTIFYICON_VERSION_4, NOTIFYICONDATAW,
    Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, GetSystemMetrics, HICON, IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTCOLOR, LoadIconW,
    LoadImageW, SM_CXSMICON, SM_CYSMICON,
};

use crate::wide::copy_to_buffer;

// Constants
const ICON_ID: u32 = 1;
const ICON_RESOURCE: u16 = 1;
const TOOLTIP: &str = "TrayReveal";

// Tray icon
pub struct TrayIcon {
    window: HWND,
    icon: HICON,
    owns_icon: bool,
}

impl TrayIcon {
    pub fn new(window: HWND) -> Self {
        let (icon, owns_icon) = load_icon();
        Self {
            window,
            icon,
            owns_icon,
        }
    }

    pub fn add(&self, callback_message: u32) -> bool {
        let mut data = self.data();
        data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;
        data.uCallbackMessage = callback_message;
        data.hIcon = self.icon;
        copy_to_buffer(&mut data.szTip, TOOLTIP);
        if unsafe { Shell_NotifyIconW(NIM_ADD, &raw const data) } == 0 {
            return false;
        }
        data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
        unsafe { Shell_NotifyIconW(NIM_SETVERSION, &raw const data) != 0 }
    }

    pub fn remove(&self) {
        let data = self.data();
        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &raw const data);
        }
    }

    pub fn notify(&self, title: &str, text: &str) {
        let mut data = self.data();
        data.uFlags = NIF_INFO;
        data.dwInfoFlags = NIIF_USER | NIIF_RESPECT_QUIET_TIME;
        copy_to_buffer(&mut data.szInfoTitle, title);
        copy_to_buffer(&mut data.szInfo, text);
        unsafe {
            Shell_NotifyIconW(NIM_MODIFY, &raw const data);
        }
    }

    fn data(&self) -> NOTIFYICONDATAW {
        NOTIFYICONDATAW {
            cbSize: u32::try_from(mem::size_of::<NOTIFYICONDATAW>()).unwrap_or(0),
            hWnd: self.window,
            uID: ICON_ID,
            ..NOTIFYICONDATAW::default()
        }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        if self.owns_icon {
            unsafe {
                DestroyIcon(self.icon);
            }
        }
    }
}

fn load_icon() -> (HICON, bool) {
    let icon = unsafe {
        LoadImageW(
            GetModuleHandleW(ptr::null()),
            usize::from(ICON_RESOURCE) as *const u16,
            IMAGE_ICON,
            GetSystemMetrics(SM_CXSMICON),
            GetSystemMetrics(SM_CYSMICON),
            LR_DEFAULTCOLOR,
        )
    };
    if icon.is_null() {
        (
            unsafe { LoadIconW(ptr::null_mut(), IDI_APPLICATION) },
            false,
        )
    } else {
        (icon, true)
    }
}
