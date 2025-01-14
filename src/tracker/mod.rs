pub mod task_execution_tracker;

use std::sync::{Arc, Mutex};
use std::collections::HashSet;
#[derive(Clone)]
pub struct TaskTracker {
    active_urls: Arc<Mutex<HashSet<String>>>,
    active_apps: Arc<Mutex<HashSet<String>>>
}

impl TaskTracker {
    pub fn new() -> Self {
        TaskTracker {
            active_urls: Arc::new(Mutex::new(HashSet::new())),
            active_apps: Arc::new(Mutex::new(HashSet::new()))
        }
    }

    pub fn is_url_active(&self, url: &str) -> bool {
        self.active_urls.lock()
            .map(|urls| urls.contains(url))
            .unwrap_or(false)
    }

    pub fn is_app_active(&self, app_path: &str) -> bool {
        self.active_apps.lock()
            .map(|apps| apps.contains(app_path))
            .unwrap_or(false)
    }

    pub fn add_url(&self, url: String) {
        if let Ok(mut urls) = self.active_urls.lock() {
            urls.insert(url);
        }
    }

    pub fn add_app(&self, app_path: String) {
        if let Ok(mut apps) = self.active_apps.lock() {
            apps.insert(app_path);
        }
    }

    pub fn remove_url(&self, url: &str) {
        if let Ok(mut urls) = self.active_urls.lock() {
            urls.remove(url);
        }
    }
}
