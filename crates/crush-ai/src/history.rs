use std::path::PathBuf;
use std::fs::{OpenOptions, File};
use std::io::{BufRead, BufReader, Write};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub language: String,
    pub exception_type: String,
    pub file: String,
    pub line: u32,
    pub resolved: bool,
}

impl ErrorEvent {
    pub fn generate_id(language: &str, file: &str, line: u32) -> String {
        let mut hasher = Sha256::new();
        hasher.update(language.as_bytes());
        hasher.update(file.as_bytes());
        hasher.update(line.to_string().as_bytes());
        hex::encode(hasher.finalize())
    }
}

#[derive(Debug, Clone)]
pub struct ErrorHistory {
    pub file_path: PathBuf,
}

impl ErrorHistory {
    pub fn new(data_dir: PathBuf) -> Self {
        let errors_dir = data_dir.join("errors");
        let _ = std::fs::create_dir_all(&errors_dir);
        let file_path = errors_dir.join("history.jsonl");
        Self { file_path }
    }

    fn read_all(&self) -> Vec<ErrorEvent> {
        let mut list = Vec::new();
        if !self.file_path.exists() {
            return list;
        }
        if let Ok(file) = File::open(&self.file_path) {
            let reader = BufReader::new(file);
            for line in reader.lines().flatten() {
                if let Ok(event) = serde_json::from_str::<ErrorEvent>(&line) {
                    list.push(event);
                }
            }
        }
        list
    }

    fn write_all(&self, events: &[ErrorEvent]) -> std::io::Result<()> {
        let mut file = File::create(&self.file_path)?;
        for event in events {
            let serialized = serde_json::to_string(event)?;
            writeln!(file, "{}", serialized)?;
        }
        Ok(())
    }

    pub fn record(&self, mut event: ErrorEvent) {
        let all = self.read_all();
        // check if same id already present and unresolved (dedup)
        let dup = all.iter().any(|e| e.id == event.id && !e.resolved);
        if dup {
            return;
        }

        if event.id.is_empty() {
            event.id = ErrorEvent::generate_id(&event.language, &event.file, event.line);
        }

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
        {
            if let Ok(serialized) = serde_json::to_string(&event) {
                let _ = writeln!(file, "{}", serialized);
            }
        }
    }

    pub fn mark_resolved(&self, id: &str) {
        let mut all = self.read_all();
        let mut modified = false;
        for event in &mut all {
            if event.id == id {
                event.resolved = true;
                modified = true;
            }
        }
        if modified {
            let _ = self.write_all(&all);
        }
    }

    pub fn recent(&self, limit: usize) -> Vec<ErrorEvent> {
        let mut all = self.read_all();
        // Sort descending by timestamp
        all.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all.truncate(limit);
        all
    }
}
