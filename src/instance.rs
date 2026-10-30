use std::ptr;

use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows_sys::Win32::System::Threading::CreateMutexW;

use crate::wide::to_wide;

// Constants
const MUTEX_NAME: &str = r"Local\TrayReveal.Instance";

// Instance
pub struct InstanceGuard(HANDLE);

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

pub fn acquire() -> Option<InstanceGuard> {
    let name = to_wide(MUTEX_NAME);
    let handle = unsafe { CreateMutexW(ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        return None;
    }
    let guard = InstanceGuard(handle);
    (unsafe { GetLastError() } != ERROR_ALREADY_EXISTS).then_some(guard)
}
