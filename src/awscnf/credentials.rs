// src/credentials.rs
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use std::error::Error;
use log::{error, info};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AwsCredentials {
    pub access_key: String,
    pub secret_key: String,
    pub region: String,
    pub bucket_name: String,
}

pub struct CredentialsManager {
    credentials: Arc<RwLock<Option<AwsCredentials>>>,
    credentials_path: PathBuf,
}

impl CredentialsManager {
    pub fn new<P: AsRef<Path>>(credentials_path: P) -> Self {
        Self {
            credentials: Arc::new(RwLock::new(None)),
            credentials_path: credentials_path.as_ref().to_path_buf(),
        }
    }

    pub async fn get_credentials(&self) -> Result<AwsCredentials, Box<dyn Error>> {
        if let Some(creds) = self.credentials.read().await.as_ref() {
            return Ok(creds.clone());
        }

        // If not in cache, load from file
        let creds = self.load_credentials_from_file()?;
        *self.credentials.write().await = Some(creds.clone());
        Ok(creds)
    }

    fn load_credentials_from_file(&self) -> Result<AwsCredentials, Box<dyn Error>> {
        let mut file = File::open(&self.credentials_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let creds: AwsCredentials = toml::from_str(&contents)?;
        Ok(creds)
    }
}