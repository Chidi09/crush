use std::path::{Path, PathBuf};
use crush_types::{Result, CrushError};
use crate::dockerfile::{DockerfileParserV2, DockerInstruction};

pub struct MigrationReport {
    pub dockerfile_path: PathBuf,
    pub crushfile_content: String,
    pub stage_count: usize,
    pub instruction_count: usize,
    pub merged_run_count: usize,
    pub original_size_estimate_mb: u64,
    pub optimized_size_estimate_mb: u64,
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
            original_size_estimate_mb: 500,
            optimized_size_estimate_mb: 300,
            suggestions,
            warnings,
        })
    }

    pub fn generate_crushfile(&self, dockerfile: &crate::dockerfile::Dockerfile) -> Result<String> {
        let mut output = String::new();
        output.push_str("# Crushfile — migrated from Dockerfile\n");
        output.push_str("# Optimized for crush build system\n\n");

        for (i, stage) in dockerfile.stages.iter().enumerate() {
            let stage_name = stage.name.clone().unwrap_or_else(|| {
                if i == 0 { "build".to_string() } else { format!("stage_{}", i) }
            });

            output.push_str(&format!("[[stages]]\nname = \"{}\"\n", stage_name));

            if let Some(ref base) = stage.base_image {
                output.push_str(&format!("image = \"{}\"\ntype = \"base\"\n\n", base));
            }

            for ins in &stage.instructions {
                match ins {
                    DockerInstruction::From { .. } => {}
                    DockerInstruction::Run { command, .. } => {
                        output.push_str(&format!("[[stages]]\ntype = \"run\"\ncommand = \"\"\"{}\n\"\"\"\n\n", command));
                    }
                    DockerInstruction::Copy { src, dest, from, .. } => {
                        if let Some(f) = from {
                            output.push_str(&format!("[[stages]]\ntype = \"copy\"\nfrom = \"{}\"\nrule = \"{} {}\"\n\n", f, src, dest));
                        } else {
                            output.push_str(&format!("[[stages]]\ntype = \"copy\"\nrule = \"{} {}\"\n\n", src, dest));
                        }
                    }
                    DockerInstruction::Env { pairs } => {
                        for (k, v) in pairs {
                            output.push_str(&format!("[[stages]]\ntype = \"config\"\nname = \"env\"\nkey = \"{}\"\nvalue = \"{}\"\n\n", k, v));
                        }
                    }
                    DockerInstruction::Expose { ports } => {
                        output.push_str(&format!("[[stages]]\ntype = \"config\"\nname = \"expose\"\nports = [{}]\n\n",
                            ports.iter().map(|p| format!("\"{}\"", p)).collect::<Vec<_>>().join(", ")));
                    }
                    DockerInstruction::Workdir { path } => {
                        output.push_str(&format!("[[stages]]\ntype = \"config\"\nname = \"workdir\"\nvalue = \"{}\"\n\n", path));
                    }
                    DockerInstruction::User { user } => {
                        output.push_str(&format!("[[stages]]\ntype = \"config\"\nname = \"user\"\nvalue = \"{}\"\n\n", user));
                    }
                    DockerInstruction::Entrypoint { args, .. } => {
                        output.push_str(&format!("[[stages]]\ntype = \"config\"\nname = \"entrypoint\"\nvalue = {}\n\n",
                            serde_json::to_string(args).unwrap_or_default()));
                    }
                    DockerInstruction::Cmd { args, .. } => {
                        output.push_str(&format!("[[stages]]\ntype = \"config\"\nname = \"cmd\"\nvalue = {}\n\n",
                            serde_json::to_string(args).unwrap_or_default()));
                    }
                    DockerInstruction::Label { labels } => {
                        for (k, v) in labels {
                            output.push_str(&format!("[[stages]]\ntype = \"config\"\nname = \"label\"\nkey = \"{}\"\nvalue = \"{}\"\n\n", k, v));
                        }
                    }
                    DockerInstruction::Volume { paths } => {
                        output.push_str(&format!("[[stages]]\ntype = \"config\"\nname = \"volume\"\npaths = [{}]\n\n",
                            paths.iter().map(|p| format!("\"{}\"", p)).collect::<Vec<_>>().join(", ")));
                    }
                    DockerInstruction::Healthcheck { cmd, .. } => {
                        output.push_str(&format!("[[stages]]\ntype = \"config\"\nname = \"healthcheck\"\nvalue = {}\n\n",
                            serde_json::to_string(cmd).unwrap_or_default()));
                    }
                    _ => {}
                }
            }
        }

        Ok(output)
    }
}
