use log::error;
use log::info;
use std::path::PathBuf;
use std::fs::{self, File};
use winapi::um::winnt::{
    FILE_ATTRIBUTE_HIDDEN,
    FILE_ATTRIBUTE_SYSTEM,
    GENERIC_READ,
    GENERIC_WRITE,
};
use winapi::um::fileapi::{
    SetFileAttributesW,
    CreateFileW,
    OPEN_EXISTING,
};
use winapi::um::handleapi::CloseHandle;
use std::os::windows::ffi::OsStrExt;
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, NewAead};
use rand::Rng;
use std::io::{Read, Write, Seek, SeekFrom};
use std::ptr;

#[derive(Debug)]
pub enum SecureFolderError {
    IoError(std::io::Error),
    EncryptionError(String),
    DecryptionError(String),
}

impl std::fmt::Display for SecureFolderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecureFolderError::IoError(e) => write!(f, "IO error: {}", e),
            SecureFolderError::EncryptionError(e) => write!(f, "Encryption error: {}", e),
            SecureFolderError::DecryptionError(e) => write!(f, "Decryption error: {}", e),
        }
    }
}

impl std::error::Error for SecureFolderError {}

impl From<std::io::Error> for SecureFolderError {
    fn from(error: std::io::Error) -> Self {
        SecureFolderError::IoError(error)
    }
}

pub struct SecureFolder {
    pub path: PathBuf,
    key: [u8; 32],
}

impl SecureFolder {
    pub fn new(base_path: PathBuf, folder_name: &str) -> Result<Self, SecureFolderError> {
        let folder_path = base_path.join(folder_name);
        let key_file_path = folder_path.join(".key");

        // Create folder if it doesn't exist
        if !folder_path.exists() {
            fs::create_dir_all(&folder_path).map_err(SecureFolderError::IoError)?;
        }

        // Try to read existing key or create new one
        let key = if key_file_path.exists() {
            let mut key_file = File::open(&key_file_path)?;
            let mut key = [0u8; 32];
            key_file.read_exact(&mut key)?;
            key
        } else {
            let key = rand::thread_rng().gen::<[u8; 32]>();
            let mut key_file = File::create(&key_file_path)?;
            key_file.write_all(&key)?;

            // Hide the key file
            let wide_path: Vec<u16> = key_file_path
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            unsafe {
                SetFileAttributesW(
                    wide_path.as_ptr(),
                    FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM
                );
            }

            key
        };

        // Set folder attributes
        let wide_path: Vec<u16> = folder_path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let handle = CreateFileW(
                wide_path.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                ptr::null_mut(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM,
                ptr::null_mut()
            );

            if handle != ptr::null_mut() {
                CloseHandle(handle);
            }

            SetFileAttributesW(
                wide_path.as_ptr(),
                FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM
            );
        }

        Ok(Self {
            path: folder_path,
            key,
        })
    }

    pub fn store_file(&self, filename: &str, data: &[u8]) -> Result<(), SecureFolderError> {
        let cipher = Aes256Gcm::new(Key::from_slice(&self.key));

        let nonce_bytes = rand::thread_rng().gen::<[u8; 12]>();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let encrypted_data = cipher
            .encrypt(nonce, data)
            .map_err(|e| SecureFolderError::EncryptionError(e.to_string()))?;

        let file_path = self.path.join(filename);
        let mut file = File::create(&file_path)?;

        // Write format: [12 bytes nonce][encrypted data]
        file.write_all(&nonce_bytes)?;
        file.write_all(&encrypted_data)?;
        file.flush()?;

        let wide_path: Vec<u16> = file_path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let handle = CreateFileW(
                wide_path.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                ptr::null_mut(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM,
                ptr::null_mut()
            );

            if handle != ptr::null_mut() {
                CloseHandle(handle);
            }

            SetFileAttributesW(
                wide_path.as_ptr(),
                FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM
            );
        }

        Ok(())
    }

    pub fn read_file(&self, filename: &str) -> Result<Vec<u8>, SecureFolderError> {
        let file_path = self.path.join(filename);
        let mut file = File::open(&file_path)?;

        // Read and verify file size
        let file_len = file.metadata()?.len() as usize;
        info!("File size: {} bytes", file_len);

        if file_len < 12 {
            return Err(SecureFolderError::DecryptionError("File too small".to_string()));
        }

        // Read nonce
        let mut nonce_bytes = [0u8; 12];
        file.read_exact(&mut nonce_bytes)?;

        // Read encrypted data
        let mut encrypted_data = vec![0u8; file_len - 12];
        file.read_exact(&mut encrypted_data)?;

        // Decrypt
        let cipher = Aes256Gcm::new(Key::from_slice(&self.key));
        let nonce = Nonce::from_slice(&nonce_bytes);

        match cipher.decrypt(nonce, encrypted_data.as_ref()) {
            Ok(decrypted) => {
                info!("Decryption successful: {} bytes", decrypted.len());
                Ok(decrypted)
            },
            Err(e) => {
                error!("Decryption failed: {}", e);
                Err(SecureFolderError::DecryptionError(e.to_string()))
            }
        }
    }
}