use crate::tracker::TaskTracker;
use std::error::Error as StdError;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio;
use log::{error, info, debug, warn};
use crate::tasks::scheduler::should_execute_task;
use crate::tracker::task_execution_tracker::TaskExecutionTracker;
use tokio::task::JoinHandle;
use futures::future::join_all;
use anyhow::Context;
use webbrowser;

const TASK_TIMEOUT_SECS: u64 = 300; // 5 minutes timeout

pub async fn process_url_tasks(config: &crate::config::ConfigAws, tracker: &TaskTracker) -> anyhow::Result<(), Box<dyn StdError>> {
    info!("Starting process url tasks");
    debug!("Configuration details: tasks_count={}", config.tasks.len());

    // Early validation;
    if config.tasks.is_empty() {
        info!("No tasks found in configuration");
        return Ok(());
    }

    // Reuse existing execution tracker if possible
    static EXECUTION_TRACKER: Mutex<Option<TaskExecutionTracker>> = Mutex::new(None);
    let mut execution_tracker_guard = EXECUTION_TRACKER.lock().unwrap();

    let execution_tracker = if execution_tracker_guard.is_none() {
        let new_tracker = TaskExecutionTracker::new()
            .context("Failed to initialize execution tracker")?;
        *execution_tracker_guard = Some(new_tracker);
        execution_tracker_guard.as_ref().unwrap()
    } else {
        execution_tracker_guard.as_ref().unwrap()
    };

    // Cleanup old entries
    if let Err(e) = execution_tracker.cleanup_old_entries() {
        warn!("Failed to cleanup old entries: {}. Continuing...", e);
    }

    // Filter and count URL tasks
    let url_tasks: Vec<_> = config.tasks.iter()
        .filter(|task| task.task_type == "URL" && task.enabled)
        .cloned()
        .collect();

    let url_tasks_count = url_tasks.len();
    info!("Found {} enabled URL tasks to process", url_tasks_count);

    if url_tasks_count == 0 {
        info!("No enabled URL tasks found");
        return Ok(());
    }

    let shared_errors = Arc::new(Mutex::new(Vec::<String>::new()));
    let mut task_handles: Vec<JoinHandle<()>> = Vec::new();

    for task in url_tasks {
        let url = match &task.url {
            Some(url) if !url.is_empty() => url.clone(),
            _ => {
                warn!("Invalid or empty URL for task {}, skipping", task.task_id);
                continue;
            }
        };

        if tracker.is_url_active(&url) {
            info!("URL {} is already active, skipping", url);
            continue;
        }

        if !should_execute_task(&task, execution_tracker) {
            info!("Task {} scheduled for later execution", task.task_id);
            continue;
        }

        let task_errors = Arc::clone(&shared_errors);
        let exec_tracker = execution_tracker.clone();
        let task_id = task.task_id.clone();
        let tracker_clone = tracker.clone();
        let interval = task.interval.unwrap_or(0);
        let url_clone = url.clone();

        let handle = tokio::spawn(async move {
            let cleanup = || {
                debug!("Initiating cleanup for URL: {}", url_clone);
                tracker_clone.remove_url(&url_clone);
                debug!("Cleanup completed for URL: {}", url_clone);
            };

            let process_url = async {
                if interval == 0 {
                    // One-time task execution
                    execute_url_task(&url_clone, &task_id, &exec_tracker, &task_errors).await;
                } else {
                    // Recurring task execution
                    info!("Starting recurring task for URL: {}", url_clone);
                    loop {
                        if should_execute_task(&task, &exec_tracker) {
                            execute_url_task(&url_clone, &task_id, &exec_tracker, &task_errors).await;
                        }
                        tokio::time::sleep(Duration::from_secs(60)).await;
                    }
                }
            };

            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(60)) => {
                    execute_url_task(&url_clone, &task_id, &exec_tracker, &task_errors).await;
                        }
                _ = process_url => {
                    info!("Task completed normally for URL: {}", url_clone);
                    }
                }

            cleanup();
        });

        task_handles.push(handle);
    }

    // Wait for one-time tasks
    join_all(task_handles).await;

    // Log accumulated errors
    if let Ok(errors) = shared_errors.lock() {
        for error in errors.iter() {
            error!("Task error occurred: {}", error);
        }
    }

    info!("URL task processing completed");
    Ok(())
}

async fn execute_url_task(
    url: &str,
    task_id: &str,
    exec_tracker: &TaskExecutionTracker,
    task_errors: &Arc<Mutex<Vec<String>>>
) {
    debug!("Executing URL task: {}", url);

    
    //TODO: URL is opening two times every 5 minutes but seperated by 1 minutes
    match webbrowser::open(url) {
        Ok(_) => {
            info!("Successfully opened URL: {}", url);
            if let Err(e) = exec_tracker.mark_executed(task_id) {
                error!("Failed to mark task {} as executed: {}", task_id, e);
            }
        },
        Err(e) => {
            let error_msg = format!("Failed to open URL {}: {}", url, e);
            error!("{}", error_msg);
            if let Ok(mut errors) = task_errors.lock() {
                errors.push(error_msg);
            }
        }
    }
}