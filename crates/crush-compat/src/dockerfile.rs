use std::path::{Path, PathBuf};
use std::fs;
use crush_types::{Result, CrushError};

#[derive(Debug, Clone)]
pub struct Dockerfile {
    pub stages: Vec<DockerfileStage>,
    pub base_images: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DockerfileStage {
    pub name: Option<String>,
    pub base_image: Option<String>,
    pub instructions: Vec<DockerInstruction>,
}

#[derive(Debug, Clone)]
pub enum DockerInstruction {
    From { image: String, name: Option<String>, platform: Option<String> },
    Run { command: String, is_heredoc: bool },
    Cmd { args: Vec<String>, is_json: bool },
    Label { labels: Vec<(String, String)> },
    Expose { ports: Vec<String> },
    Env { pairs: Vec<(String, String)> },
    Add { src: String, dest: String },
    Copy { src: String, dest: String, from: Option<String>, chown: Option<String> },
    Entrypoint { args: Vec<String>, is_json: bool },
    Volume { paths: Vec<String> },
    User { user: String },
    Workdir { path: String },
    Arg { name: String, default: Option<String> },
    OnBuild { instruction: String },
    Stopsignal { signal: String },
    Healthcheck { cmd: Vec<String>, interval: Option<String>, timeout: Option<String>, retries: Option<u32> },
    Shell { shell: Vec<String> },
    Comment { text: String },
    Maintainer { email: String },
    Unknown { raw: String },
}

pub struct DockerfileParserV2;

impl DockerfileParserV2 {
    pub fn new() -> Self { Self }

    pub fn parse_path(&self, path: &Path) -> Result<Dockerfile> {
        let content = fs::read_to_string(path)
            .map_err(|e| CrushError::StorageError(format!("Failed to read Dockerfile: {}", e)))?;
        self.parse(&content)
    }

    pub fn parse(&self, content: &str) -> Result<Dockerfile> {
        let mut stages = Vec::new();
        let mut current_stage = DockerfileStage {
            name: None, base_image: None, instructions: Vec::new(),
        };
        let mut base_images = Vec::new();
        let mut continuation = String::new();
        let mut in_heredoc = false;
        let mut heredoc_content = String::new();
        let mut heredoc_delimiter = String::new();

        for raw_line in content.lines() {
            let line = raw_line.trim().to_string();

            if in_heredoc {
                if line.trim() == heredoc_delimiter {
                    in_heredoc = false;
                    if !heredoc_content.is_empty() {
                        current_stage.instructions.push(DockerInstruction::Run {
                            command: heredoc_content.trim().to_string(),
                            is_heredoc: true,
                        });
                        heredoc_content.clear();
                    }
                    continue;
                }
                heredoc_content.push_str(&line);
                heredoc_content.push('\n');
                continue;
            }

            if line.ends_with('\\') && !line.ends_with("\\\\") {
                continuation.push_str(line.trim_end_matches('\\'));
                continuation.push(' ');
                continue;
            }
            let full_line = if !continuation.is_empty() {
                let result = continuation.trim().to_string() + " " + line.trim();
                continuation.clear();
                result
            } else {
                line.clone()
            };

            if full_line.is_empty() || full_line == "#" { continue; }
            if let Some(comment) = full_line.strip_prefix("# ") {
                current_stage.instructions.push(DockerInstruction::Comment { text: comment.to_string() });
                if full_line.starts_with("# syntax=") || full_line.starts_with("# syntax =") {
                    // BuildKit directive
                }
                continue;
            }

            if full_line.starts_with('#') { continue; }

            let upper = full_line.to_uppercase();

            if let Some(val) = upper.strip_prefix("FROM ") {
                if !current_stage.instructions.is_empty() || current_stage.base_image.is_some() {
                    stages.push(current_stage);
                    current_stage = DockerfileStage { name: None, base_image: None, instructions: Vec::new() };
                }
                let rest = &full_line[5..];
                let (image, name, platform) = Self::parse_from(rest);
                if let Some(ref img) = image {
                    base_images.push(img.clone());
                }
                current_stage.base_image = image.clone();
                current_stage.name = name;
                current_stage.instructions.push(DockerInstruction::From {
                    image: image.unwrap_or_default(),
                    name, platform,
                });
            } else if let Some(val) = full_line.strip_prefix("RUN ") {
                let cmd = val.trim().to_string();
                if cmd.contains("<<") || cmd.contains("<<-") {
                    in_heredoc = true;
                    let after = cmd.split("<<").nth(1).or_else(|| cmd.split("<<-").nth(1))
                        .unwrap_or("EOF").trim();
                    heredoc_delimiter = after.to_string();
                    let before = cmd.split("<<").next().or_else(|| cmd.split("<<-").next())
                        .unwrap_or("").trim();
                    if !before.is_empty() {
                        current_stage.instructions.push(DockerInstruction::Run {
                            command: before.to_string(),
                            is_heredoc: false,
                        });
                    }
                } else {
                    current_stage.instructions.push(DockerInstruction::Run { command: cmd, is_heredoc: false });
                }
            } else if let Some(val) = full_line.strip_prefix("CMD ") {
                current_stage.instructions.push(Self::parse_cmd_entrypoint(val, DockerInstruction::Cmd { args: vec![], is_json: false }));
            } else if let Some(val) = full_line.strip_prefix("ENTRYPOINT ") {
                current_stage.instructions.push(Self::parse_cmd_entrypoint(val, DockerInstruction::Entrypoint { args: vec![], is_json: false }));
            } else if let Some(val) = full_line.strip_prefix("LABEL ") {
                let labels = Self::parse_key_value(val);
                current_stage.instructions.push(DockerInstruction::Label { labels });
            } else if let Some(val) = full_line.strip_prefix("EXPOSE ") {
                let ports: Vec<String> = val.split_whitespace().map(|s| s.to_string()).collect();
                current_stage.instructions.push(DockerInstruction::Expose { ports });
            } else if let Some(val) = full_line.strip_prefix("ENV ") {
                let pairs = Self::parse_key_value(val);
                current_stage.instructions.push(DockerInstruction::Env { pairs });
            } else if let Some(val) = full_line.strip_prefix("ADD ") {
                let (src, dest) = Self::parse_copy_add(val);
                current_stage.instructions.push(DockerInstruction::Add { src, dest });
            } else if let Some(val) = full_line.strip_prefix("COPY ") {
                let (src, dest, from, chown) = Self::parse_copy_full(val);
                current_stage.instructions.push(DockerInstruction::Copy { src, dest, from, chown });
            } else if let Some(val) = full_line.strip_prefix("VOLUME ") {
                let paths = Self::parse_json_or_space(val);
                current_stage.instructions.push(DockerInstruction::Volume { paths });
            } else if let Some(val) = full_line.strip_prefix("USER ") {
                current_stage.instructions.push(DockerInstruction::User { user: val.trim().to_string() });
            } else if let Some(val) = full_line.strip_prefix("WORKDIR ") {
                current_stage.instructions.push(DockerInstruction::Workdir {
                    path: val.trim().trim_matches('"').to_string(),
                });
            } else if let Some(val) = full_line.strip_prefix("ARG ") {
                let (name, default) = Self::parse_arg(val);
                current_stage.instructions.push(DockerInstruction::Arg { name, default });
            } else if let Some(val) = full_line.strip_prefix("ONBUILD ") {
                current_stage.instructions.push(DockerInstruction::OnBuild { instruction: val.to_string() });
            } else if let Some(val) = full_line.strip_prefix("STOPSIGNAL ") {
                current_stage.instructions.push(DockerInstruction::Stopsignal { signal: val.trim().to_string() });
            } else if let Some(val) = full_line.strip_prefix("HEALTHCHECK ") {
                current_stage.instructions.push(DockerInstruction::Healthcheck {
                    cmd: vec![val.to_string()], interval: None, timeout: None, retries: None,
                });
            } else if let Some(val) = full_line.strip_prefix("SHELL ") {
                let shell = Self::parse_json_array(val);
                current_stage.instructions.push(DockerInstruction::Shell { shell });
            } else if let Some(val) = full_line.strip_prefix("MAINTAINER ") {
                current_stage.instructions.push(DockerInstruction::Maintainer { email: val.trim().to_string() });
            } else if !full_line.is_empty() {
                current_stage.instructions.push(DockerInstruction::Unknown { raw: full_line });
            }
        }

        if !current_stage.instructions.is_empty() || current_stage.base_image.is_some() {
            stages.push(current_stage);
        }

        Ok(Dockerfile { stages, base_images })
    }

    fn parse_from(val: &str) -> (Option<String>, Option<String>, Option<String>) {
        let parts: Vec<&str> = val.split_whitespace().collect();
        let mut image = None;
        let mut name = None;
        let mut platform = None;
        let mut i = 0;
        while i < parts.len() {
            if parts[i] == "--platform" && i + 1 < parts.len() {
                platform = Some(parts[i + 1].to_string());
                i += 2;
                continue;
            }
            if parts[i] == "AS" || parts[i] == "as" && i + 1 < parts.len() {
                name = Some(parts[i + 1].to_string());
                i += 2;
                continue;
            }
            if image.is_none() {
                image = Some(parts[i].to_string());
            }
            i += 1;
        }
        (image, name, platform)
    }

    fn parse_cmd_entrypoint(val: &str, template: DockerInstruction) -> DockerInstruction {
        let val = val.trim();
        let (is_json, args) = if val.starts_with('[') {
            (true, Self::parse_json_array(val))
        } else {
            (false, vec!["/bin/sh".to_string(), "-c".to_string(), val.to_string()])
        };
        match template {
            DockerInstruction::Cmd { .. } => DockerInstruction::Cmd { args, is_json },
            DockerInstruction::Entrypoint { .. } => DockerInstruction::Entrypoint { args, is_json },
            _ => template,
        }
    }

    fn parse_json_array(val: &str) -> Vec<String> {
        serde_json::from_str::<Vec<String>>(val.trim()).unwrap_or_else(|_| {
            val.trim_matches('[').trim_matches(']').split(',')
                .map(|s| s.trim().trim_matches('"').to_string()).collect()
        })
    }

    fn parse_key_value(val: &str) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        if val.contains('=') {
            let mut remaining = val;
            while let Some(eq) = remaining.find('=') {
                let key = remaining[..eq].trim().to_string();
                remaining = remaining[eq + 1..].trim();
                if remaining.starts_with('"') {
                    if let Some(end) = remaining[1..].find('"') {
                        let value = remaining[1..=end].to_string();
                        remaining = remaining[end + 2..].trim();
                        pairs.push((key, value));
                    } else {
                        pairs.push((key, remaining.to_string()));
                        break;
                    }
                } else if let Some(space) = remaining.find(' ') {
                    let value = remaining[..space].to_string();
                    remaining = remaining[space + 1..].trim();
                    pairs.push((key, value));
                } else {
                    pairs.push((key, remaining.to_string()));
                    break;
                }
            }
        } else {
            pairs.push((val.trim().to_string(), String::new()));
        }
        pairs
    }

    fn parse_copy_add(val: &str) -> (String, String) {
        let parts: Vec<&str> = val.split_whitespace().collect();
        if parts.len() >= 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            (String::new(), String::new())
        }
    }

    fn parse_copy_full(val: &str) -> (String, String, Option<String>, Option<String>) {
        let parts: Vec<&str> = val.split_whitespace().collect();
        let mut src = String::new();
        let mut dest = String::new();
        let mut from = None;
        let mut chown = None;
        let mut i = 0;
        while i < parts.len() {
            if parts[i] == "--from" && i + 1 < parts.len() {
                from = Some(parts[i + 1].to_string());
                i += 2; continue;
            }
            if parts[i] == "--chown" && i + 1 < parts.len() {
                chown = Some(parts[i + 1].to_string());
                i += 2; continue;
            }
            if src.is_empty() { src = parts[i].to_string(); }
            else { dest = parts[i].to_string(); }
            i += 1;
        }
        (src, dest, from, chown)
    }

    fn parse_arg(val: &str) -> (String, Option<String>) {
        let val = val.trim();
        if let Some(eq) = val.find('=') {
            (val[..eq].trim().to_string(), Some(val[eq + 1..].trim().trim_matches('"').to_string()))
        } else {
            (val.to_string(), None)
        }
    }

    fn parse_json_or_space(val: &str) -> Vec<String> {
        if val.trim().starts_with('[') {
            Self::parse_json_array(val)
        } else {
            val.split_whitespace().map(|s| s.trim().to_string()).collect()
        }
    }
}
