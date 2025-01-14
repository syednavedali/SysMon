use std::process::Command;
use std::io::{self, Write};

pub fn is_camera_in_use() -> bool {
    // Attempt to list available video capture devices using FFmpeg
    let output = Command::new("ffmpeg")
        .args(&["-list_devices", "true", "-f", "dshow", "-i", "dummy"])
        .output()
        .expect("Failed to execute FFmpeg command");

    // Check if FFmpeg's output contains any camera devices
    let output_str = String::from_utf8_lossy(&output.stderr);

    if output_str.contains("BisonCam,NB Pro") { // Replace with your camera name or check for any device name
        println!("Camera is available.");
        return false; // Camera is not in use
    }

    // If the camera is being used, the output may contain errors or warnings
    if output_str.contains("Error opening video device") {
        println!("Camera is in use or unavailable.");
        return true; // Camera is in use or unavailable
    }

    false
}

