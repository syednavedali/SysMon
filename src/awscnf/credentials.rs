use std::env::current_dir;
// src/credentials.rs
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use std::error::Error;
use log::{debug, info};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AwsCredentials {
    pub access_key: String,
    pub secret_key: String,
    pub region: String,
    pub bucket_name: String,
}

pub struct CredentialsManager {
    credentials: Arc<RwLock<Option<AwsCredentials>>>,
}

impl CredentialsManager {
    pub fn new() -> Self {
        debug!("Initializing CredentialsManager");
        Self {
            credentials: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn get_credentials(&self) -> Result<AwsCredentials, Box<dyn Error>> {
        debug!("Attempting to retrieve credentials");

        if let Some(creds) = self.credentials.read().await.as_ref() {
            debug!("Retrieved credentials from cache");
            return Ok(creds.clone());
        }

        info!("Initializing hardcoded credentials");
        let creds = AwsCredentials {
            access_key: "".to_string(),
            secret_key: "".to_string(),
            region: "ap-south-1".to_string(),
            bucket_name: "sysmon261224".to_string(),
        };

        *self.credentials.write().await = Some(creds.clone());
        Ok(creds)
    }
}