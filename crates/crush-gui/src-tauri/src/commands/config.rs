use serde::{Deserialize, Serialize};
use std::fs;
use tauri::State;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    // AI
    #[serde(default)]
    pub ai_provider: String,
    #[serde(default)]
    pub ai_api_key: String,
    #[serde(default)]
    pub ai_model: String,
    #[serde(default)]
    pub auto_diagnose: bool,

    // Deploy defaults
    #[serde(default)]
    pub default_provider: String,
    #[serde(default)]
    pub default_region: String,

    // Services
    #[serde(default = "default_postgres_port")]
    pub postgres_port: u16,
    #[serde(default = "default_redis_port")]
    pub redis_port: u16,
    #[serde(default = "default_mongo_port")]
    pub mongo_port: u16,
    #[serde(default = "default_minio_port")]
    pub minio_port: u16,
    #[serde(default)]
    pub services_data_dir: String,
    #[serde(default)]
    pub auto_stop_services: bool,

    // Appearance
    #[serde(default)]
    pub reduce_motion: bool,
    #[serde(default)]
    pub accent_color: String,

    // Updates
    #[serde(default = "default_check_for_updates")]
    pub check_for_updates: bool,
}

fn default_postgres_port() -> u16 { 5432 }
fn default_redis_port() -> u16 { 6379 }
fn default_mongo_port() -> u16 { 27017 }
fn default_minio_port() -> u16 { 9000 }
fn default_check_for_updates() -> bool { true }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ai_provider: String::new(),
            ai_api_key: String::new(),
            ai_model: String::new(),
            auto_diagnose: false,
            default_provider: String::new(),
            default_region: String::new(),
            postgres_port: 5432,
            redis_port: 6379,
            mongo_port: 27017,
            minio_port: 9000,
            services_data_dir: String::new(),
            auto_stop_services: false,
            reduce_motion: false,
            accent_color: String::new(),
            check_for_updates: true,
        }
    }
}

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    let path = state.data_dir.join("config.json");
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let config: AppConfig = serde_json::from_str(&content).unwrap_or_default();
    Ok(config)
}

#[tauri::command]
pub async fn set_config(config: AppConfig, state: State<'_, AppState>) -> Result<(), String> {
    let path = state.data_dir.join("config.json");
    let content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(())
}
