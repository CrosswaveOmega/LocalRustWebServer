use std::collections::HashMap;

use serde_json::json;
use std::sync::{Mutex, OnceLock};
#[derive(Debug, Clone)]
pub struct AppObj {
    pub hashstatus: HashMap<String, String>,
    pub logs: Vec<(String, String)>,
}

impl AppObj {
    fn new() -> Self {
        Self {
            hashstatus: HashMap::new(),
            logs: Vec::new(),
        }
    }
}

//An "AppSingleton" object to help with some tasks.
//Such as recording the global state of functions
pub struct AppSingleton {
    pub obj: Mutex<AppObj>,
}

impl AppSingleton {
    fn new() -> Self {
        Self {
            obj: Mutex::new(AppObj::new()),
        }
    }

    pub fn init() {
        APP_SINGLETON.get_or_init(|| AppSingleton::new());
    }

    pub fn instance() -> &'static AppSingleton {
        APP_SINGLETON
            .get()
            .expect("AppSingleton not initialized. Call AppSingleton::init() first.")
    }

    pub fn log_started(&self, script: &str) {
        let mut obj = self.obj.lock().unwrap();
        obj.logs.push((script.to_string(), "started".to_string()));
        tracing::info!("{} started", script);
    }

    pub fn log_finished(&self, script: &str) {
        let mut obj = self.obj.lock().unwrap();
        obj.logs.push((script.to_string(), "finished".to_string()));
        tracing::info!("{} finished", script);
    }

    pub fn get_logs(&self) -> Vec<(String, String)> {
        let obj = self.obj.lock().unwrap();
        obj.logs.clone()
    }

    pub fn insert_status(&self, key: &str, value: &str) {
        let mut obj = self.obj.lock().unwrap();
        obj.hashstatus.insert(key.to_string(), value.to_string());
        println!("Inserted status: {} = {}", key, value);
        tracing::debug!("Inserted status: {} = {}", key, value);
    }

    /// Get all keys from the hashstatus map
    pub fn get_all_keys(&self) -> Vec<String> {
        let obj = self.obj.lock().unwrap();
        obj.hashstatus.keys().cloned().collect()
    }

    /// Retrieve a status entry from the hashstatus map
    pub fn get_status(&self, key: &str) -> Option<String> {
        let obj = self.obj.lock().unwrap();
        let result = obj.hashstatus.get(key).cloned();
        println!("Retrieved status for {}: {:?}", key, result);
        result
    }

    pub fn hashstatus_to_json(&self) -> String {
        let obj = self.obj.lock().unwrap();
        json!(obj.hashstatus).to_string()
    }
}

static APP_SINGLETON: OnceLock<AppSingleton> = OnceLock::new();
