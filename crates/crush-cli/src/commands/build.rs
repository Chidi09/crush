use std::path::Path;
use tracing::info;
use crush_types::{CrushError};
use crush_build::StackDetector;
use crate::BuildArgs;

pub async fn exec(
    args: &BuildArgs,
    data_dir: &Path,
) -> anyhow::Result<()> {
    if let Some(ref plat) = args.platform {
        std::env::set_var("CRUSH_DEFAULT_PLATFORM", plat);
    }
    info!("Building image: {} (platforms: {:?})", args.tag, args.platform);
    let detector = StackDetector::new();
    let project_root = std::env::current_dir()?;
    let stack = detector.detect(&project_root).await?;

    let cache_dir = data_dir.join("cache");
    let cache = crush_build::BuildCache::new(cache_dir.clone());
    let pipeline = crush_build::BuildPipeline::new(cache).with_progress();

    let crushfile_path = project_root.join("Crushfile");
    let stages = if crushfile_path.exists() {
        println!("Parsing Crushfile at: {}", crushfile_path.display());
        let parsed = crush_build::CrushfileParser::parse(&crushfile_path)
            .map_err(|e| CrushError::StorageError(format!("Failed to parse Crushfile: {}", e)))?;
        parsed.stages.unwrap_or_default()
    } else {
        println!("No Crushfile found, synthesising stages from stack detection...");
        let base_img = stack.base_image.clone();
        vec![
            crush_build::CrushfileStage {
                name: Some("base".to_string()),
                stage_type: "base".to_string(),
                image: Some(base_img),
                command: None,
                rule: None,
                from: None,
                target: None,
                platforms: None,
            },
            crush_build::CrushfileStage {
                name: Some("deps".to_string()),
                stage_type: "run".to_string(),
                image: None,
                command: Some(stack.build_command.clone()),
                rule: None,
                from: None,
                target: None,
                platforms: None,
            },
            crush_build::CrushfileStage {
                name: Some("source".to_string()),
                stage_type: "copy".to_string(),
                image: None,
                command: None,
                rule: Some(".".to_string()),
                from: None,
                target: None,
                platforms: None,
            },
            crush_build::CrushfileStage {
                name: Some("final".to_string()),
                stage_type: "config".to_string(),
                image: None,
                command: None,
                rule: None,
                from: None,
                target: None,
                platforms: None,
            },
        ]
    };

    let pipeline_result = pipeline.execute(&project_root, &stages, &std::collections::HashMap::new()).await?;
    let digest = pipeline_result.digest;
    println!("Built image {} -> digest: {}", args.tag, digest);
    Ok(())
}
