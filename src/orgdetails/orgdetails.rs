use lazy_static::lazy_static;
use log::{error, info};
use std::sync::RwLock;
use std::fs;
use std::path::PathBuf;
use std::env;

#[derive(Debug, Clone)]
pub struct OrgDetails {
    org_code: String,
    ecd: String,
}

impl OrgDetails {
    fn new() -> Self {
        // First try to read from file
        if let Ok((org_code, ecd)) = Self::read_from_file() {
            info!("Successfully loaded organization details from file");
            return OrgDetails { org_code, ecd };
        } else {
            return OrgDetails {org_code: "def_orf".to_string(), ecd: "def_emp".to_string() };
        }

        // // Fallback to environment variables if file reading fails
        // let org_code = std::env::var("ORGCD").unwrap_or_else(|e| {
        //     error!("Failed to get ORGCD from environment: {}. Using default value.", e);
        //     "default_org_code".to_string()
        // });
        // 
        // let ecd = std::env::var("ECD").unwrap_or_else(|e| {
        //     error!("Failed to get ECD from environment: {}. Using default value.", e);
        //     "default_ecd".to_string()
        // });

      //  OrgDetails { org_code, ecd }
    }

    fn read_from_file() -> Result<(String, String), Box<dyn std::error::Error>> {
        // Get the executable's directory
        let mut path = env::current_exe()?;
        path.pop(); // Remove executable name to get directory
        path.push("orgdt.cng");

        info!("Attempting to read organization details from: {}", path.display());

        // Read and parse the file
        let contents = fs::read_to_string(&path)?;
        let lines: Vec<&str> = contents.lines().collect();

        if lines.len() < 2 {
            error!("Invalid orgdt.cng file format. Expected at least 2 lines");
            return Err("Invalid file format".into());
        }

        Ok((lines[0].trim().to_string(), lines[1].trim().to_string()))
    }

    pub fn get_org_code(&self) -> String {
        self.org_code.clone()
    }

    pub fn get_ecd(&self) -> String {
        self.ecd.clone()
    }
}

lazy_static! {
    static ref ORG_DETAILS: RwLock<OrgDetails> = RwLock::new(OrgDetails::new());
}

pub fn get_org_details() -> OrgDetails {
    ORG_DETAILS.read().unwrap().clone()
}

pub fn reload_org_details() {
    let mut org_details = ORG_DETAILS.write().unwrap();
    *org_details = OrgDetails::new();
}