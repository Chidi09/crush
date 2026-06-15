use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::{command, State};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainRecord {
    pub host: String,
    pub project: String,
    pub port: u16,
}

fn domains_file(data_dir: &PathBuf) -> PathBuf {
    data_dir.join("domains.json")
}

fn read_domains(data_dir: &PathBuf) -> Vec<DomainRecord> {
    let path = domains_file(data_dir);
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Ok(records) = serde_json::from_str::<Vec<DomainRecord>>(&text) {
            return records;
        }
    }
    Vec::new()
}

fn write_domains(data_dir: &PathBuf, records: &[DomainRecord]) -> Result<(), String> {
    let path = domains_file(data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(records).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn list_domains(state: State<'_, AppState>) -> Result<Vec<DomainRecord>, String> {
    Ok(read_domains(&state.data_dir))
}

#[command]
pub async fn add_domain(host: String, project: String, port: u16, state: State<'_, AppState>) -> Result<(), String> {
    let mut domains = read_domains(&state.data_dir);
    domains.retain(|d| d.host != host); // remove if exists
    domains.push(DomainRecord { host, project, port });
    write_domains(&state.data_dir, &domains)
}

#[command]
pub async fn remove_domain(host: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut domains = read_domains(&state.data_dir);
    domains.retain(|d| d.host != host);
    write_domains(&state.data_dir, &domains)
}
