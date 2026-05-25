pub mod dockerfile;
pub mod compose;
pub mod migrate;
pub mod creds;
pub mod api;
pub mod convert;

use std::path::PathBuf;
use std::fs;
use crush_types::{Result, CrushError};

pub use dockerfile::{DockerfileParserV2, Dockerfile, DockerfileStage, DockerInstruction};
pub use compose::{ComposeParser, ComposeV2, ComposeService};
pub use migrate::{DockerfileMigrator, MigrationReport};
pub use creds::{DockerCredentialHelper, DockerConfig, Credential};
pub use api::DockerApiServer;
pub use convert::{OciImageConverter, ConversionReport};

pub struct DockerfileParser;

impl DockerfileParser {
    pub fn new() -> Self { Self }

    pub fn parse_to_crushfile(&self, dockerfile_path: &PathBuf) -> Result<String> {
        let parser = DockerfileParserV2::new();
        let dockerfile = parser.parse_path(dockerfile_path)?;
        let migrator = DockerfileMigrator::new();
        migrator.generate_crushfile(&dockerfile)
    }
}

pub struct ComposeLoader;

impl ComposeLoader {
    pub fn new() -> Self { Self }

    pub fn parse_compose_file(&self, compose_path: &PathBuf) -> Result<Vec<String>> {
        let parser = ComposeParser::new();
        let compose = parser.parse_path(compose_path)?;
        Ok(ComposeParser::get_service_names(&compose))
    }
}
