use std::mem;
use std::ptr;

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIIF_LARGE_ICON,
    NIIF_RESPECT_QUIET_TIME, NIIF_USER, NIM_ADD, NIM_DELETE, NIM_MODIFY, NIM_SETVERSION,
    NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, GetSystemMetrics, HICON, IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTCOLOR, LoadIconW,
    LoadImageW, SM_CXICON, SM_CXSMICON,
};

use crate::wide::copy_to_buffer;

// Constants
const ICON_ID: u32 = 1;
const ICON_RESOURCE: u16 = 1;
const TOOLTIP: &str = "TrayReveal";
const NOTIFICATION_ICON_SCALE: i32 = 2;

// Types
struct Icon {
    handle: HICON,
    owned: bool,
}

impl Icon {
    fn load(size: i32) -> Self {
        let handle = unsafe {
            LoadImageW(
                GetModuleHandleW(ptr::null()),
                usize::from(ICON_RESOURCE) as *const u16,
                IMAGE_ICON,
                size,
                size,
                LR_DEFAULTCOLOR,
            )
        };
        if handle.is_null() {
            Self {
                handle: unsafe { LoadIconW(ptr::null_mut(), IDI_APPLICATION) },
                owned: false,
            }
        } else {
            Self {
                handle,
                owned: true,
            }
        }
    }
}

impl Drop for Icon {
    fn drop(&mut self) {
        if self.owned {
            unsafe {
                DestroyIcon(self.handle);
            }
        }
    }
}

// Tray icon
pub struct TrayIcon {
    window: HWND,
    icon: Icon,
    notification_icon: Icon,
}

impl TrayIcon {
    pub fn new(window: HWND) -> Self {
        let small = unsafe { GetSystemMetrics(SM_CXSMICON) };
        let large = unsafe { GetSystemMetrics(SM_CXICON) } * NOTIFICATION_ICON_SCALE;
        Self {
            window,
            icon: Icon::load(small),
            notification_icon: Icon::load(large),
        }
    }

    pub fn add(&self, callback_message: u32) -> bool {
        let mut data = self.data();
        data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;
        data.uCallbackMessage = callback_message;
        data.hIcon = self.icon.handle;
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
        data.dwInfoFlags = NIIF_USER | NIIF_LARGE_ICON | NIIF_RESPECT_QUIET_TIME;
        data.hBalloonIcon = self.notification_icon.handle;
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
