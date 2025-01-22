use serde::{Deserialize, Serialize};
use chrono::prelude::*;
use std::default::Default;
// Function to get configuration from the AWS Lambda
use log::{info, error};
use std::error::Error as StdError;
use crate::orgdetails::orgdetails::get_org_details;

#[derive(Deserialize, Serialize, Debug)]
pub struct WorkingHours {
    pub start: String,
    pub end: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Settings {
    pub uploadduration: u64,
    pub cameracaptureduration: u64,
    pub screenshotduration: u64,
    pub timezone: String,
    #[serde(rename = "workingHours")]
    pub working_hours: WorkingHours,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Task {
    #[serde(rename = "taskId")]
    pub task_id: String,
    #[serde(rename = "taskType")]
    pub task_type: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(rename = "scheduleType", default)]  // Made scheduleType optional
    pub schedule_type: Option<String>,          // Changed to Option<String>
    #[serde(rename = "startTime", default)]     // Made startTime optional too since login notice might not have it
    pub start_time: Option<String>,
    #[serde(default)]
    pub interval: Option<u64>,
    pub enabled: bool,
    #[serde(default)]                           // Made description optional with default
    pub description: String,
    #[serde(rename = "applicationPath", default)]
    pub application_path: Option<String>,
    #[serde(rename = "dayOfWeek", default)]
    pub day_of_week: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(rename = "notificationMessage", default)]
    pub notification_message: Option<String>,
    #[serde(rename = "notificationTitle", default)]
    pub notification_title: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ConfigAws {
    pub pk: String,
    pub sk: String,
    #[serde(rename = "orgCode")]
    pub org_code: String,
    #[serde(rename = "employeeId")]
    pub employee_id: String,
    pub tasks: Vec<Task>,
    pub settings: Settings,
    #[serde(rename = "lastUpdated")]
    pub last_updated: i64,
    pub version: i64,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
    #[serde(rename = "createdBy")]
    pub created_by: String,
    #[serde(rename = "lastUpdatedBy")]
    pub last_updated_by: String,
}

impl Default for Task {
    fn default() -> Self {
        Task {
            task_id: String::new(),
            task_type: String::new(),
            url: None,
            schedule_type: None,
            start_time: None,
            interval: None,
            enabled: false,
            description: String::new(),
            application_path: None,
            day_of_week: None,
            date: None,
            notification_message: None,
            notification_title: None,
        }
    }
}

impl Default for WorkingHours {
    fn default() -> Self {
        WorkingHours {
            start: "09:00".to_string(),
            end: "18:00".to_string(),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            uploadduration: 30,
            cameracaptureduration: 30,
            screenshotduration: 10,
            timezone: "UTC+00:00".to_string(),
            working_hours: WorkingHours::default(),
        }
    }
}

impl Default for ConfigAws {
    fn default() -> Self {
        ConfigAws {
            pk: String::new(),
            sk: String::new(),
            org_code: String::new(),
            employee_id: String::new(),
            tasks: vec![Task::default()],
            settings: Settings::default(),
            last_updated: 0,
            version: 1,
            created_at: 0,
            created_by: String::new(),
            last_updated_by: String::new(),
        }
    }
}

pub async fn get_config_from_lambda() -> Result<ConfigAws, Box<dyn StdError>> {
    info!(
        "Starting config fetch from Lambda at {}",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    );

    // Get environment variables
    let org_code = get_org_details().get_org_code();

    let employee_id= get_org_details().get_ecd();

    let url = "https://nn46ahpkuecuc3fahl54d6lc5q0owldx.lambda-url.ap-south-1.on.aws/";
    info!("Preparing request to Lambda URL: {}", url);

    let client = reqwest::Client::new();

    // Create the request body
    let body = serde_json::json!({
        "orgcode": org_code,
        "employeeid": employee_id
    });

    info!("Sending request to Lambda with body: {}", 
        serde_json::to_string_pretty(&body).unwrap_or_default()
    );

    // Send request and get response text first
    let response = client
        .post(url)
        .json(&body)
        .send()
        .await?;

    info!("Received response from Lambda with status: {}", response.status());

    // Get the response text for debugging
    let response_text = response.text().await?;
    info!("Raw response body: {}", response_text);

    // Try to parse the response
    match serde_json::from_str::<ConfigAws>(&response_text) {
        Ok(config) => {
            info!("Successfully parsed config response");
            Ok(config)
        },
        Err(e) => {
            error!("Failed to parse config response: {}", e);
            // Print the line and column numbers from the error
            error!("Error at line {}, column {}", e.line(), e.column());
            Err(Box::new(e))
        }
    }
}