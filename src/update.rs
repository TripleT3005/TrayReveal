use std::ffi::c_void;
use std::ptr;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Networking::WinHttp::{
    INTERNET_DEFAULT_HTTPS_PORT, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_ADDREQ_FLAG_ADD,
    WINHTTP_FLAG_SECURE, WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE,
    WinHttpAddRequestHeaders, WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest,
    WinHttpQueryDataAvailable, WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse,
    WinHttpSendRequest, WinHttpSetTimeouts,
};
use windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW;

use crate::wide::to_wide;

// Constants
const HOST: &str = "api.github.com";
const PATH: &str = "/repos/TripleT3005/TrayReveal/releases/latest";
const ACCEPT_HEADER: &str = "Accept: application/vnd.github+json";
pub const RELEASES_URL: &str = "https://github.com/TripleT3005/TrayReveal/releases/latest";
const RELEASE_URL_PREFIX: &str = "https://github.com/TripleT3005/TrayReveal/releases/";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const FIRST_DELAY: Duration = Duration::from_secs(60);
const INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
const TIMEOUT_MS: i32 = 15_000;
const MAX_RESPONSE: usize = 512 * 1024;
const STATUS_OK: u32 = 200;

// Types
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub url: String,
}

struct Handle(*mut c_void);

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            WinHttpCloseHandle(self.0);
        }
    }
}

static AVAILABLE: Mutex<Option<Release>> = Mutex::new(None);

// Checking
pub fn spawn(window: HWND, message: u32, enabled: fn() -> bool) {
    let window = window as usize;
    thread::spawn(move || {
        thread::sleep(FIRST_DELAY);
        loop {
            if enabled()
                && let Some(release) =
                    fetch_latest().filter(|release| is_newer(&release.version, CURRENT_VERSION))
            {
                if let Ok(mut available) = AVAILABLE.lock() {
                    *available = Some(release);
                }
                unsafe {
                    PostMessageW(window as HWND, message, 0, 0);
                }
            }
            thread::sleep(INTERVAL);
        }
    });
}

pub fn available() -> Option<Release> {
    AVAILABLE.lock().ok()?.clone()
}

fn fetch_latest() -> Option<Release> {
    let body = request()?;
    let version = json_string(&body, "tag_name")?;
    let version = version.strip_prefix('v').unwrap_or(&version).to_owned();
    let url = json_string(&body, "html_url")
        .filter(|url| url.starts_with(RELEASE_URL_PREFIX))
        .unwrap_or_else(|| RELEASES_URL.to_owned());
    Some(Release { version, url })
}

fn request() -> Option<String> {
    let agent = to_wide(&format!("TrayReveal/{CURRENT_VERSION}"));
    let session = Handle(unsafe {
        WinHttpOpen(
            agent.as_ptr(),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            ptr::null(),
            ptr::null(),
            0,
        )
    });
    if session.0.is_null() {
        return None;
    }
    unsafe {
        WinHttpSetTimeouts(session.0, TIMEOUT_MS, TIMEOUT_MS, TIMEOUT_MS, TIMEOUT_MS);
    }
    let host = to_wide(HOST);
    let connection =
        Handle(unsafe { WinHttpConnect(session.0, host.as_ptr(), INTERNET_DEFAULT_HTTPS_PORT, 0) });
    if connection.0.is_null() {
        return None;
    }
    let verb = to_wide("GET");
    let path = to_wide(PATH);
    let request = Handle(unsafe {
        WinHttpOpenRequest(
            connection.0,
            verb.as_ptr(),
            path.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            WINHTTP_FLAG_SECURE,
        )
    });
    if request.0.is_null() {
        return None;
    }
    let header = to_wide(ACCEPT_HEADER);
    let sent = unsafe {
        WinHttpAddRequestHeaders(
            request.0,
            header.as_ptr(),
            u32::MAX,
            WINHTTP_ADDREQ_FLAG_ADD,
        );
        WinHttpSendRequest(request.0, ptr::null(), 0, ptr::null(), 0, 0, 0) != 0
            && WinHttpReceiveResponse(request.0, ptr::null_mut()) != 0
    };
    if !sent || status_code(&request)? != STATUS_OK {
        return None;
    }
    let mut body = Vec::new();
    loop {
        let mut available = 0_u32;
        if unsafe { WinHttpQueryDataAvailable(request.0, &raw mut available) } == 0
            || available == 0
        {
            break;
        }
        let start = body.len();
        body.resize(start + available as usize, 0);
        let mut read = 0_u32;
        if unsafe {
            WinHttpReadData(
                request.0,
                body[start..].as_mut_ptr().cast(),
                available,
                &raw mut read,
            )
        } == 0
        {
            return None;
        }
        body.truncate(start + read as usize);
        if body.len() > MAX_RESPONSE {
            return None;
        }
    }
    String::from_utf8(body).ok()
}

fn status_code(request: &Handle) -> Option<u32> {
    let mut status = 0_u32;
    let mut length = u32::try_from(size_of::<u32>()).ok()?;
    let found = unsafe {
        WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            ptr::null(),
            (&raw mut status).cast(),
            &raw mut length,
            ptr::null_mut(),
        )
    };
    (found != 0).then_some(status)
}

// Parsing
fn json_string(json: &str, key: &str) -> Option<String> {
    let start = json.find(&format!("\"{key}\""))? + key.len() + 2;
    let rest = json[start..].trim_start().strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    let mut value = String::new();
    let mut characters = rest.chars();
    while let Some(character) = characters.next() {
        match character {
            '"' => return Some(value),
            '\\' => value.push(match characters.next()? {
                'n' => '\n',
                't' => '\t',
                other => other,
            }),
            other => value.push(other),
        }
    }
    None
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(candidate), Some(current)) => candidate > current,
        _ => false,
    }
}

fn parse_version(version: &str) -> Option<[u32; 3]> {
    if version.contains('-') {
        return None;
    }
    let mut parts = version.split('.').map(str::parse::<u32>);
    let version = [
        parts.next()?.ok()?,
        parts.next().unwrap_or(Ok(0)).ok()?,
        parts.next().unwrap_or(Ok(0)).ok()?,
    ];
    parts.next().is_none().then_some(version)
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_strings_from_the_release_json() {
        let json = r#"{"url": "x", "html_url" : "https://github.com/TripleT3005/TrayReveal/releases/tag/v1.2.0", "tag_name":"v1.2.0", "name": "A \"quoted\" name"}"#;
        assert_eq!(json_string(json, "tag_name").as_deref(), Some("v1.2.0"));
        assert_eq!(
            json_string(json, "html_url").as_deref(),
            Some("https://github.com/TripleT3005/TrayReveal/releases/tag/v1.2.0")
        );
        assert_eq!(
            json_string(json, "name").as_deref(),
            Some("A \"quoted\" name")
        );
        assert_eq!(json_string(json, "missing"), None);
    }

    #[test]
    #[ignore = "needs network access"]
    fn fetches_the_latest_release() {
        let release = fetch_latest().expect("the latest release can be fetched");
        assert!(parse_version(&release.version).is_some());
        assert!(release.url.starts_with(RELEASE_URL_PREFIX));
    }

    #[test]
    fn compares_versions() {
        assert!(is_newer("1.1.0", "1.0.0"));
        assert!(is_newer("2.0", "1.9.9"));
        assert!(is_newer("1.0.10", "1.0.9"));
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("0.9.0", "1.0.0"));
        assert!(!is_newer("1.1.0-beta.1", "1.0.0"));
        assert!(!is_newer("garbage", "1.0.0"));
    }
}
