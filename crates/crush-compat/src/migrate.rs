use std::path::{Path, PathBuf};
use crush_types::{Result, CrushError};
use crate::dockerfile::{DockerfileParserV2, DockerInstruction};

pub struct MigrationReport {
    pub dockerfile_path: PathBuf,
    pub crushfile_content: String,
    pub stage_count: usize,
    pub instruction_count: usize,
    pub merged_run_count: usize,
    pub suggestions: Vec<String>,
    pub warnings: Vec<String>,
}

pub struct DockerfileMigrator;

impl DockerfileMigrator {
    pub fn new() -> Self { Self }

    pub fn analyze(&self, dockerfile_path: &Path) -> Result<MigrationReport> {
        let parser = DockerfileParserV2::new();
        let dockerfile = parser.parse_path(dockerfile_path)?;

        let mut ins_count = 0;
        let mut run_commands = Vec::new();
        let mut has_deps_copy = false;
        let mut has_source_copy = false;
        let mut has_noignore = true;
        let mut has_pinned_base = false;
        let mut suggestions = Vec::new();
        let mut warnings = Vec::new();

        for stage in &dockerfile.stages {
            for ins in &stage.instructions {
                ins_count += 1;
                match ins {
                    DockerInstruction::Run { command, .. } => run_commands.push(command.clone()),
                    DockerInstruction::Copy { src: _src, dest, from: _, chown: _ } => {
                        if dest.contains("node_modules") || dest.contains(".venv") || dest.contains("target") {
                            has_deps_copy = true;
                        }
                        if dest == "." || dest == "./" {
                            has_source_copy = true;
                        }
                    }
                    DockerInstruction::From { ref image, .. } => {
                        if !image.contains(':') || !image.contains('@') {
                            has_pinned_base = false;
                            warnings.push(format!("Base image '{}' is not pinned to a specific tag or digest. Use '{}:latest@sha256:...'", image, image));
                        } else {
                            has_pinned_base = true;
                        }
                    }
                    _ => {}
                }
            }
        }

        if dockerfile.stages.len() > 1 {
            suggestions.push("Multi-stage build: final image can be slimmed by copying only artifacts".to_string());
        }

        if has_noignore && std::path::Path::new(dockerfile_path).parent().map(|p| p.join(".dockerignore").exists()).unwrap_or(false) {
            has_noignore = false;
        }
        if has_noignore {
            suggestions.push("Add a .dockerignore file to reduce build context size".to_string());
        }

        suggestions.push("BuildKit heredoc syntax is supported for complex RUN commands".to_string());
        if !has_pinned_base {
            suggestions.push("Pin base image to a specific digest for reproducible builds".to_string());
        }

        let crushfile = self.generate_crushfile(&dockerfile)?;

        Ok(MigrationReport {
            dockerfile_path: dockerfile_path.to_path_buf(),
            crushfile_content: crushfile,
            stage_count: dockerfile.stages.len(),
            instruction_count: ins_count,
            merged_run_count: run_commands.len(),
            suggestions,
            warnings,
        })
    }

    pub fn generate_crushfile(&self, dockerfile: &crate::dockerfile::Dockerfile) -> Result<String> {
        // Walk all stages to collect the key fields the Crushfile format needs.
        let mut base_image = String::new();
        let mut build_cmds: Vec<String> = Vec::new();
        let mut entry = String::new();
        let mut port: u16 = 0;
        let mut env_vars: Vec<(String, String)> = Vec::new();
        let mut env_comments: Vec<String> = Vec::new();

        for stage in &dockerfile.stages {
            if let Some(ref img) = stage.base_image {
                base_image = img.clone();
            }
            for ins in &stage.instructions {
                match ins {
                    DockerInstruction::Run { command, .. } => build_cmds.push(command.clone()),
                    DockerInstruction::Expose { ports } => {
                        if port == 0 {
                            port = ports.first()
                                .and_then(|p| p.split('/').next())
                                .and_then(|p| p.parse().ok())
                                .unwrap_or(0);
                        }
                    }
                    DockerInstruction::Env { pairs } => {
                        for (k, v) in pairs {
                            if v.is_empty() {
                                env_comments.push(k.clone());
                            } else {
                                env_vars.push((k.clone(), v.clone()));
                            }
                        }
                    }
                    DockerInstruction::Entrypoint { args, .. } => {
                        if entry.is_empty() {
                            entry = args.join(" ");
                        }
                    }
                    DockerInstruction::Cmd { args, .. } => {
                        if entry.is_empty() {
                            entry = args.join(" ");
                        }
                    }
                    _ => {}
                }
            }
        }

        if port == 0 { port = 3000; }
        let build_command = build_cmds.join(" && ");

        // Guess a project name from the base image (e.g. "node:20-alpine" → "node").
        let project_type = base_image.split(':').next()
            .and_then(|s| s.split('/').last())
            .unwrap_or("docker")
            .to_string();

        let mut out = String::new();
        out.push_str("# Crushfile — migrated from Dockerfile by `crush migrate`\n");
        out.push_str("# Review all inferred values before deploying.\n\n");

        out.push_str("[project]\n");
        out.push_str(&format!("type = \"{}\"  # inferred from base image\n\n", project_type));

        out.push_str("[build]\n");
        if !build_command.is_empty() {
            out.push_str(&format!("command = \"{}\"\n", build_command.replace('"', "\\\"")));
        }
        if !entry.is_empty() {
            out.push_str(&format!("entry = \"{}\"\n", entry.replace('"', "\\\"")));
        }
        out.push_str(&format!("port = {}\n", port));

        if !env_vars.is_empty() || !env_comments.is_empty() {
            out.push_str("\n[env]\n");
            for (k, v) in &env_vars {
                out.push_str(&format!("{} = \"{}\"\n", k, v.replace('"', "\\\"")));
            }
            for k in &env_comments {
                out.push_str(&format!("# {} = \"\"\n", k));
            }
        }

        Ok(out)
    }
}
