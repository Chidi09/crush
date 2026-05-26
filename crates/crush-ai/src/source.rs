use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceContext {
    pub file: String,
    pub line: u32,
    pub before: Vec<String>,
    pub target_line: String,
    pub after: Vec<String>,
    pub column_indicator: Option<String>,
}

impl SourceContext {
    pub fn format(&self) -> String {
        let mut out = String::new();
        let start_line = self.line.saturating_sub(self.before.len() as u32);

        for (i, line) in self.before.iter().enumerate() {
            out.push_str(&format!("  {:4} | {}\n", start_line + i as u32, line));
        }

        out.push_str(&format!(">>>{:4} | {}\n", self.line, self.target_line));

        if let Some(ref ind) = self.column_indicator {
            out.push_str(&format!("       | {}\n", ind));
        }

        for (i, line) in self.after.iter().enumerate() {
            out.push_str(&format!("  {:4} | {}\n", self.line + 1 + i as u32, line));
        }

        out
    }
}

pub fn extract_context(file: &str, line: u32, radius: usize) -> Option<SourceContext> {
    extract_context_with_column(file, line, None, radius)
}

pub fn extract_context_with_column(file: &str, line: u32, column: Option<u32>, radius: usize) -> Option<SourceContext> {
    let path = std::path::Path::new(file);
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    let lines: Vec<&str> = content.lines().collect();
    if line == 0 || line as usize > lines.len() {
        return None;
    }

    let target_idx = (line - 1) as usize;
    let target_line = lines[target_idx].to_string();

    let start = target_idx.saturating_sub(radius);
    let end = (target_idx + radius).min(lines.len() - 1);

    let mut before = Vec::new();
    for i in start..target_idx {
        before.push(lines[i].to_string());
    }

    let mut after = Vec::new();
    for i in (target_idx + 1)..=end {
        after.push(lines[i].to_string());
    }

    let column_indicator = column.map(|col| {
        let spaces = " ".repeat((col.saturating_sub(1)) as usize);
        format!("{}^~~~", spaces)
    });

    Some(SourceContext {
        file: file.to_string(),
        line,
        before,
        target_line,
        after,
        column_indicator,
    })
}
