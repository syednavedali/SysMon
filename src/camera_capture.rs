use std::process::Command;
use std::io::{self};
use log::{error, info};

pub fn capture_image_with_ffmpeg(file_path: &str) -> io::Result<()> {
    // Run the ffmpeg command to capture the image
    info!("Image captured started");
    let output = Command::new("ffmpeg")
        .args(&[
            "-y", "-f", "dshow", "-i", "video=BisonCam,NB Pro", "-frames:v", "1", file_path,
        ])
        .output(); // `output()` already returns a Result, no need for `expect`

    match output {
        Ok(output) => {
            // If the command was successful, print the corresponding message
            if output.status.success() {
                info!("Image captured and saved as '{}'", file_path);
            } else {
                // If ffmpeg failed, print the error from stderr
                info!("Image captured failed");
                error!(
                    "Failed to capture image: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
        Err(err) => {
            // Handle the case where the `ffmpeg` command itself failed to start
            error!("Failed to execute FFmpeg: {}", err);
            eprintln!("Failed to execute FFmpeg: {}", err);
        }
    }

    Ok(())
}
