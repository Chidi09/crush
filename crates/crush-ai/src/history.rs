use std::path::{Path, PathBuf};
use std::fs;
use std::io::Write;
use crush_types::{Result, CrushError};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ErrorRecord {
    pub timestamp: String,
    pub error_type: String,
    pub message_hash: String,
    pub language: String,
    pub file: String,
    pub line: usize,
    pub count: u32,
    pub resolved: bool,
    pub fix_applied: bool,
}

pub struct ErrorHistory {
    log_dir: PathBuf,
    records: Vec<ErrorRecord>,
}

impl ErrorHistory {
    pub fn new(base_dir: &Path) -> Self {
        let log_dir = base_dir.join("errors");
        fs::create_dir_all(&log_dir).ok();
        let records = Self::load_records(&log_dir);
        Self { log_dir, records }
    }

    pub fn log(&mut self, error_type: &str, language: &str, file: &str, line: usize) -> ErrorRecord {
        let hash = self.compute_hash(error_type, file, line);

        if let Some(existing) = self.records.iter_mut().find(|r| r.message_hash == hash) {
            existing.count += 1;
            existing.timestamp = chrono::Utc::now().to_rfc3339();
            return existing.clone();
        }

        let record = ErrorRecord {
            timestamp: chrono::Utc::now().to_rfc3339(),
            error_type: error_type.to_string(),
            message_hash: hash.clone(),
            language: language.to_string(),
            file: file.to_string(),
            line,
            count: 1,
            resolved: false,
            fix_applied: false,
        };

        self.records.push(record.clone());
        self.persist(&record).ok();
        record
    }

    pub fn mark_resolved(&mut self, hash: &str) {
        if let Some(record) = self.records.iter_mut().find(|r| r.message_hash == hash) {
            record.resolved = true;
        }
    }

    pub fn recent(&self, limit: usize) -> Vec<&ErrorRecord> {
        let mut sorted: Vec<_> = self.records.iter().collect();
        sorted.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        sorted.truncate(limit);
        sorted
    }

    pub fn frequency(&self) -> Vec<(&str, u32)> {
        let mut freq: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
        for r in &self.records {
            *freq.entry(&r.error_type).or_insert(0) += r.count;
        }
        let mut v: Vec<_> = freq.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v
    }

    fn compute_hash(&self, error_type: &str, file: &str, line: usize) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(error_type.as_bytes());
        hasher.update(file.as_bytes());
        hasher.update(line.to_string().as_bytes());
        hex::encode(hasher.finalize())
    }

    fn persist(&self, record: &ErrorRecord) -> Result<()> {
        let path = self.log_dir.join(format!("{}.json", record.message_hash));
        let data = serde_json::to_string(record)
            .map_err(|e| CrushError::ImageError(e.to_string()))?;
        let mut file = fs::File::create(&path)
            .map_err(|e| CrushError::StorageError(e.to_string()))?;
        writeln!(file, "{}", data)
            .map_err(|e| CrushError::StorageError(e.to_string()))?;
        Ok(())
    }

    fn load_records(dir: &Path) -> Vec<ErrorRecord> {
        let mut records = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(record) = serde_json::from_str::<ErrorRecord>(&content) {
                            records.push(record);
                        }
                    }
                }
            }
        }
        records
    }
}
