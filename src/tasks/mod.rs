// src/tasks/mod.rs
pub mod url;
pub mod application;
pub mod login_notice;
pub mod scheduler;
pub mod processor;

use crate::config::ConfigAws;
use crate::tracker::TaskTracker;
use std::error::Error as StdError;
use log::{error};
use crate::tasks::login_notice::{process_login_notice_tasks};
use crate::tasks::url::{ process_url_tasks};
use crate::tasks::application::{process_application_tasks};
pub async fn process_all_tasks(config: &ConfigAws, tracker: &TaskTracker) -> anyhow::Result<(), Box<dyn StdError>> {
    
        // Process login notices first
        if let Err(e) = process_login_notice_tasks(config).await {
            error!("Error processing login notice tasks: {}", e);
        }
        // TODO: there is some problem in application task parallel execution with URL task need to fix it.
        // let (url_result, app_result) = tokio::join!(
        //     process_url_tasks(config, tracker),
        //     process_application_tasks(config, tracker)
        // );
        //
        // // Handle URL tasks result
        // if let Err(e) = url_result {
        //     error!("Error processing URL tasks: {}", e);
        // }

        // Handle application tasks result
        // if let Err(e) = app_result {
        //     error!("Error processing application tasks: {}", e);
        // }

        // Process URL tasks
        if let Err(e) = process_url_tasks(config, tracker).await {
            error!("Error processing URL tasks: {}", e);
        }
        // Process application tasks
        // if let Err(e) = process_application_tasks(config, tracker).await {
        //     error!("Error processing application tasks: {}", e);
        // }

        Ok(())
}