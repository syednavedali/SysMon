use crate::utils::windows::show_windows_notice;
use log::info;
use serde::de::StdError;

pub(crate) async fn process_login_notice_tasks(config: &crate::config::ConfigAws) -> anyhow::Result<(), Box<dyn StdError>> {
    for task in &config.tasks {
        if task.task_type == "WINDOWS_LOGIN_NOTICE" && task.enabled {
            if let (Some(message), Some(title)) = (&task.notification_message, &task.notification_title) {
                show_windows_notice(message, title);
                info!("Displayed Windows login notice: {}", title);
            }
        }
    }
    Ok(())
}