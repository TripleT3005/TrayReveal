#![windows_subsystem = "windows"]

use std::env;
use std::mem;
use std::ptr;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, IDI_APPLICATION,
    LoadIconW, MSG, PostQuitMessage, RegisterClassExW, SetTimer, WM_DESTROY, WM_TIMER, WNDCLASSEXW,
    WS_OVERLAPPED,
};

// Constants
const CLASS_NAME: &str = "TrayReveal.Probe";
const DEFAULT_TOOLTIP: &str = "TrayReveal probe";
const DEFAULT_SECONDS: u32 = 60;
const ICON_ID: u32 = 1;
const TIMER_ID: usize = 1;

// Entry point
fn main() {
    let mut arguments = env::args().skip(1);
    let tooltip = arguments
        .next()
        .unwrap_or_else(|| DEFAULT_TOOLTIP.to_owned());
    let seconds = arguments
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_SECONDS);

    let instance = unsafe { GetModuleHandleW(ptr::null()) };
    let class = wide(CLASS_NAME);
    let description = WNDCLASSEXW {
        cbSize: u32::try_from(mem::size_of::<WNDCLASSEXW>()).unwrap_or(0),
        lpfnWndProc: Some(window_procedure),
        hInstance: instance,
        lpszClassName: class.as_ptr(),
        ..unsafe { mem::zeroed() }
    };
    unsafe { RegisterClassExW(&raw const description) };
    let window = unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            class.as_ptr(),
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

    let mut data = icon_data(window);
    data.uFlags = NIF_ICON | NIF_TIP;
    data.hIcon = unsafe { LoadIconW(ptr::null_mut(), IDI_APPLICATION) };
    for (target, unit) in data.szTip.iter_mut().zip(tooltip.encode_utf16().take(127)) {
        *target = unit;
    }
    unsafe {
        Shell_NotifyIconW(NIM_ADD, &raw const data);
        SetTimer(window, TIMER_ID, seconds.saturating_mul(1000), None);
    }

    let mut message: MSG = unsafe { mem::zeroed() };
    while unsafe { GetMessageW(&raw mut message, ptr::null_mut(), 0, 0) } > 0 {
        unsafe { DispatchMessageW(&raw const message) };
    }
}

// Messages
unsafe extern "system" fn window_procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_TIMER => {
            unsafe { DestroyWindow(window) };
            0
        }
        WM_DESTROY => {
            let data = icon_data(window);
            unsafe {
                Shell_NotifyIconW(NIM_DELETE, &raw const data);
                PostQuitMessage(0);
            }
            0
        }
        _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
    }
}

// Helpers
fn icon_data(window: HWND) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: u32::try_from(mem::size_of::<NOTIFYICONDATAW>()).unwrap_or(0),
        hWnd: window,
        uID: ICON_ID,
        ..NOTIFYICONDATAW::default()
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
