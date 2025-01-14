use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use crate::StdError;
use crate::tracker::TaskTracker;
use log::{debug, error, info};
use crate::config::ConfigAws;
use crate::tasks::scheduler::should_execute_task;
use crate::tracker::task_execution_tracker::TaskExecutionTracker;

pub(crate) async fn process_application_tasks(config: &ConfigAws, tracker: &TaskTracker) -> Result<(), Box<dyn StdError>> {
    let shared_errors = Arc::new(Mutex::new(Vec::<String>::new()));
    let mut task_handles = Vec::new();


    let execution_tracker = TaskExecutionTracker::new()?;
    execution_tracker.cleanup_old_entries()?;

    info!("Starting process application tasks ---------------->Total: {}", config.tasks.len());
    for task in &config.tasks {
        debug!("Processing task: {:?}", task.task_id);
        debug!("Task type: {:?}", task.task_type);
        debug!("Task enabled: {:?}", task.enabled);

        if task.task_type != "APPLICATION" || !task.enabled {
            info!("Skipping task {} - type: {}, enabled: {}",
                task.task_id, task.task_type, task.enabled);
            continue;
        }

        let app_path = match &task.application_path {
            Some(path) => {
                debug!("Application path found: {}", path);
                path.clone()
            },
            None => {
                debug!("No application path specified for task {}", task.task_id);
                continue;
            }
        };

        if app_path.is_empty() {
            debug!("Empty application path for task {}", task.task_id);
            continue;
        }

        if tracker.is_app_active(&app_path) {
            debug!("Application {} is already active, skipping", app_path);
            continue;
        }

        debug!("Checking if should execute task {}", task.task_id);
        if !should_execute_task(task, &execution_tracker) {
            info!("Task {} should not execute at this time. Schedule type: {}, Start time: {}, Interval: {:?}", 
                task.task_id, 
                task.schedule_type.as_ref().unwrap_or(&"none".to_string()),
                task.start_time.as_ref().unwrap_or(&"none".to_string()),
                task.interval);
            continue;
        }

        let interval_secs = task.interval.unwrap_or(0) * 60;
        info!("Starting application task for {} with interval of {} minutes",
            app_path, interval_secs / 60);

        tracker.add_app(app_path.clone());
        let task_errors = Arc::clone(&shared_errors);
        let exec_tracker = execution_tracker.clone();
        let task_id = task.task_id.clone();

        let handle = tokio::spawn(async move {
            loop {
                let mut command = Command::new(&app_path);
                match command.spawn() {
                    Ok(_) => {
                        info!("Launched application: {}", app_path);
                        if let Err(e) = exec_tracker.mark_executed(&task_id) {
                            error!("Failed to mark task as executed: {}", e);
                        }
                    },
                    Err(e) => {
                        let error_msg = format!("Failed to launch application {}: {}", app_path, e);
                        error!("{}", error_msg);
                        if let Ok(mut errors) = task_errors.lock() {
                            errors.push(error_msg);
                        }
                    }
                }

                if interval_secs == 0 {
                    break;
                }

                debug!("Next execution of {} will be in {} minutes",
                    app_path, interval_secs / 60);
                tokio::time::sleep(Duration::from_secs(interval_secs)).await;
            }
        });

        task_handles.push(handle);
    }

    // Don't forget to await the handles
    futures::future::join_all(task_handles).await;
    Ok(())
}