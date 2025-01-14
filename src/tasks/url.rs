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

const TASK_TIMEOUT_SECS: u64 = 300; // 5 minutes timeout

pub async fn process_url_tasks(config: &crate::config::ConfigAws, tracker: &TaskTracker) -> anyhow::Result<(), Box<dyn StdError>> {
    info!("Starting process url tasks");
    debug!("Configuration details: tasks_count={}", config.tasks.len());

    // Early validation;
    if config.tasks.is_empty() {
        info!("No tasks found in configuration");
        return Ok(());
    }

    // Initialize error tracking with detailed logging
    debug!("Initializing shared error tracking");
    let shared_errors = Arc::new(Mutex::new(Vec::<String>::new()));
    let mut task_handles: Vec<(JoinHandle<()>, u64)> = Vec::new();

    // Initialize execution tracker with detailed error handling
    let execution_tracker = TaskExecutionTracker::new()
        .context("Failed to initialize execution tracker")?;
    info!("Execution tracker initialized successfully");

    // Cleanup old entries
    if let Err(e) = execution_tracker.cleanup_old_entries() {
        warn!("Failed to cleanup old entries: {}. Continuing...", e);
    } else {
        debug!("Successfully cleaned up old entries");
    }
    info!("------------------> 1");
    // Filter and count URL tasks
    let url_tasks: Vec<_> = config.tasks.iter()
        .filter(|task| task.task_type == "URL" && task.enabled)
        .cloned() // Clone the tasks here
        .collect();
    info!("------------------> 2");
    let url_tasks_count = url_tasks.len();
    info!("Found {} enabled URL tasks to process", url_tasks_count);

    if url_tasks_count == 0 {
        info!("No enabled URL tasks found");
        return Ok(());
    }

    // Process each task
    for (index, task) in url_tasks.iter().enumerate() {
        debug!("Processing task {}/{}: id={}, type={}", 
            index + 1, 
            url_tasks_count,
            task.task_id,
            task.task_type
        );

        // Validate URL
        let url = match &task.url {
            Some(url) if !url.is_empty() => url.clone(),
            _ => {
                warn!("Invalid or empty URL for task {}, skipping", task.task_id);
                continue;
            }
        };

        // Check if URL is already being processed
        if tracker.is_url_active(&url) {
            info!("URL {} is already active, skipping", url);
            continue;
        }

        // Schedule validation with detailed logging
        debug!("Checking schedule for task {}: schedule_type={:?}, start_time={:?}, interval={:?}",
            task.task_id,
            task.schedule_type,
            task.start_time,
            task.interval
        );

        if !should_execute_task(task, &execution_tracker) {
            info!("Task {} scheduled for later execution", task.task_id);
            continue;
        }

        let interval_secs = task.interval.unwrap_or(0) * 60;
        info!("Preparing task execution: url={}, interval={}min", url, interval_secs / 60);

        // Track URL
        tracker.add_url(url.clone());

        // Clone all the necessary data before moving into async block
        let task = task.clone();
        let task_errors = Arc::clone(&shared_errors);
        let exec_tracker = execution_tracker.clone();
        let task_id = task.task_id.clone();
        let tracker_clone = tracker.clone();
        let interval = task.interval.unwrap_or(0);
        let url_clone = url.clone();

        // Spawn task with timeout
        let handle = tokio::spawn(async move {
            let cleanup = || {
                debug!("Initiating cleanup for URL: {}", url_clone);
                tracker_clone.remove_url(&url_clone);
                debug!("Cleanup completed for URL: {}", url_clone);
            };

            let process_url = async {
                if interval == 0 {
                    // One-time task execution
                    info!("Executing one-time task for URL: {}", url_clone);
                    execute_url_task(&url_clone, &task_id, &exec_tracker, &task_errors).await;
                } else {
                    // Recurring task execution
                    info!("Starting recurring task for URL: {}", url_clone);
                    loop {
                        // Check if we should execute the task
                        if should_execute_task(&task, &exec_tracker) {
                            execute_url_task(&url_clone, &task_id, &exec_tracker, &task_errors).await;
                        }

                        // Sleep for a shorter duration to check more frequently
                        tokio::time::sleep(Duration::from_secs(60)).await; // Check every minute
                    }
                }
            };

            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(TASK_TIMEOUT_SECS)) => {
                    error!("Task for URL {} timed out after {}s", url_clone, TASK_TIMEOUT_SECS);
                        }
                _ = process_url => {
                    info!("Task completed normally for URL: {}", url_clone);
                    }
                }

            cleanup();
        });

        task_handles.push((handle, interval));
        info!("Task handle created for URL: {}", url);
    }

    // Handle one-time tasks completion
    let one_time_tasks: Vec<_> = task_handles.into_iter()
        .filter(|(_, interval)| *interval == 0)
        .map(|(handle, _)| handle)
        .collect();

    if !one_time_tasks.is_empty() {
        info!("Waiting for {} one-time tasks to complete", one_time_tasks.len());
        join_all(one_time_tasks).await;
        info!("All one-time tasks completed");
    }

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

    match open::that(url) {
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