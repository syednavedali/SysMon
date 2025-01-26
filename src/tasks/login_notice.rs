use crate::utils::windows::show_windows_notice;
use log::info;
use serde::de::StdError;

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use once_cell::sync::Lazy;

static LAST_NOTICE_TIME: Lazy<std::sync::Mutex<Option<Instant>>> = Lazy::new(|| std::sync::Mutex::new(None));
static NOTICE_SHOWN_TODAY: AtomicBool = AtomicBool::new(false);

pub(crate) async fn process_login_notice_tasks(config: &crate::config::ConfigAws) -> anyhow::Result<(), Box<dyn StdError>> {
    for task in &config.tasks {
        if task.task_type == "WINDOWS_LOGIN_NOTICE" && task.enabled {
            if let (Some(message), Some(title)) = (&task.notification_message, &task.notification_title) {
                // Check if notice should be shown
                if should_show_notice() {
                    show_windows_notice(message, title);
                    info!("Displayed Windows login notice: {}", title);

                    // Update last notice time and flag
                    update_notice_timestamp();
                }
            }
        }
    }
    Ok(())
}

fn should_show_notice() -> bool {
    let mut last_notice_time = LAST_NOTICE_TIME.lock().unwrap();

    // If never shown before, show the notice
    if last_notice_time.is_none() {
        return true;
    }

    // Check if 24 hours have passed since last notice
    match *last_notice_time {
        Some(last_time) => {
            let now = Instant::now();
            now.duration_since(last_time) >= Duration::from_secs(24 * 60 * 60)
        },
        None => true
    }
}

fn update_notice_timestamp() {
    let mut last_notice_time = LAST_NOTICE_TIME.lock().unwrap();
    *last_notice_time = Some(Instant::now());
}