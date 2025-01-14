// src/utils/instance.rs
use std::fs::File;
use std::path::PathBuf;
use fs2::FileExt;
use std::error::Error;
use log::error;

// Function to check for running instance
pub fn ensure_single_instance() -> Result<File, Box<dyn Error>> {
    let app_data = std::env::var_os("APPDATA")
        .ok_or_else(|| "APPDATA environment variable not found".to_string())?;

    let lock_path = PathBuf::from(app_data)
        .join("WinSysMon")
        .join(".lock");

    // Create directory if it doesn't exist
    if let Some(parent) = lock_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let lock_file = File::create(&lock_path)?;

    // Try to acquire exclusive lock
    match lock_file.try_lock_exclusive() {
        Ok(_) => {
            // Lock acquired successfully
            Ok(lock_file)
        },
        Err(e) => {
            error!("Another instance is already running");
            Err(Box::new(e))
        }
    }
}