use std::collections::HashMap;
use std::fs;
use chrono::{Local, NaiveDate};
use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use anyhow::{Result, Context}; // Import Context
use lazy_static::lazy_static;
use log::info;

#[derive(Serialize, Deserialize, Default, Clone)] // Clone is important here
struct DailyTaskExecutions {
    executions: HashMap<String, String>  // task_id -> last_execution_date
}

lazy_static! {
    static ref TASK_TRACKER: Mutex<DailyTaskExecutions> = Mutex::new(DailyTaskExecutions::default());
}

#[derive(Clone)]
pub struct TaskExecutionTracker {
    storage_path: PathBuf,
    executions: Arc<Mutex<DailyTaskExecutions>> // Store in memory copy of execution data
}

impl TaskExecutionTracker {
    pub fn new() -> Result<Self> {
        info!("Initializing TaskExecutionTracker");
        let mut storage_path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("hmeorg");

        info!("Creating directory at {:?}", storage_path);
        fs::create_dir_all(&storage_path)
            .context("Failed to create storage directory")?;
        storage_path.push("task_executions.json");

        let executions = Arc::new(Mutex::new(DailyTaskExecutions::default()));

        let tracker = TaskExecutionTracker { storage_path, executions };
        info!("Loading existing executions");
        tracker.load_executions()?;
        info!("TaskExecutionTracker initialized successfully");
        Ok(tracker)
    }

    fn load_executions(&self) -> Result<()> {
        info!("Loading executions from {:?}", self.storage_path);
        if self.storage_path.exists() {
            let content = fs::read_to_string(&self.storage_path)
                .context("Failed to read executions file")?;

            let loaded_executions: DailyTaskExecutions = serde_json::from_str(&content)
                .context("Failed to parse executions JSON")?;

            let mut executions = self.executions.lock().unwrap();
            *executions = loaded_executions;
            info!("Successfully loaded executions from file");
        } else {
            info!("No existing executions file found");
        }
        Ok(())
    }

    fn save_executions(&self) -> Result<()> {
        info!("Starting save_executions");
        let executions = self.executions.lock().unwrap().clone(); // Clone data before releasing lock
        drop(self.executions.lock().unwrap()); // Explicitly release lock

        // Scope the lock to minimize holding time
        let content = serde_json::to_string_pretty(&executions)
            .context("Failed to serialize executions to JSON")?;

        fs::write(&self.storage_path, content)
            .context("Failed to write executions to file")?;

        Ok(())
    }

    pub fn was_executed_today(&self, task_id: &str) -> bool {
        info!("Checking if task {} was executed today", task_id);
        let executions = self.executions.lock().unwrap();
        let today = Local::now().date_naive().to_string();

        let result = executions.executions.get(task_id).map_or(false, |last_date| last_date == &today);
        info!("Task {} was{} executed today", task_id, if result {""} else {" not"});
        result // Explicitly return the result
    }

    pub fn mark_executed(&self, task_id: &str) -> Result<()> {
        info!("Marking task {} as executed", task_id);
        let today = Local::now().date_naive().to_string();
        {
            let mut executions = self.executions.lock().unwrap();
            executions.executions.insert(task_id.to_string(), today);
        }
        self.save_executions()?;
        info!("Successfully saved execution mark for task {}", task_id);
        Ok(())
    }

    pub fn cleanup_old_entries(&self) -> Result<()> {
        info!("Starting cleanup of old entries");
        let today = Local::now().date_naive();
        let mut executions = self.executions.lock().unwrap();

        executions.executions.retain(|_, last_date| {
            NaiveDate::parse_from_str(last_date, "%Y-%m-%d")
                .map_or(false, |date| (today - date).num_days() <= 30)
        });

        drop(executions); //Explicitly release lock
        self.save_executions()?;
        Ok(())
    }
}