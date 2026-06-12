use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tracing::info;
use crush_types::*;
use crush_image::ImageStore;
use crush_compat::{DockerfileParserV2, DockerInstruction};
use crush_volume::{LocalDriver, VolumeDriver};

#[cfg(target_os = "windows")]
use crush_runtime_windows::WindowsRuntime;
#[cfg(not(target_os = "windows"))]
use crate::runtime::StatelessEngine;

use crate::{RunArgs, hex_encode_random, copy_project_into_rootfs, job_object};

pub async fn exec(
    args: &RunArgs,
    data_dir: &Path,
    store: &ImageStore,
) -> anyhow::Result<()> {
    // If image is "." or a directory path containing a Dockerfile, build and run from it
    let mut df_entrypoint = None;
    let mut df_cmd = None;
    let mut df_env = Vec::new();
    let mut df_exposed_ports = Vec::new();
    let mut is_df_build = false;
    let mut project_dir = None;

    let resolved_image = if args.image == "." || std::path::Path::new(&args.image).is_dir() {
        let dir = if args.image == "." {
            std::env::current_dir()?
        } else {
            PathBuf::from(&args.image)
        };
        let df_path = dir.join("Dockerfile");
        if df_path.exists() {
            is_df_build = true;
            project_dir = Some(dir.clone());
            // Parse Dockerfile for base image
            let df_parser = DockerfileParserV2::new();
            let df = df_parser.parse_path(&df_path)?;
            let base = df.stages.first()
                .and_then(|s| s.base_image.clone())
                .unwrap_or_else(|| "debian:bookworm-slim".to_string());
            
            if base != "scratch" && store.database().get_image_by_tag(&base).await?.is_none() {
                println!("Pulling base image {}...", base);
                store.pull_image(&base).await?;
            }

            // Extract ENV, EXPOSE, CMD, ENTRYPOINT from the stages
            for stage in &df.stages {
                for instr in &stage.instructions {
                    match instr {
                        DockerInstruction::Env { pairs } => {
                            for (k, v) in pairs { df_env.push(format!("{}={}", k, v)); }
                        }
                        DockerInstruction::Expose { ports } => df_exposed_ports.extend(ports.clone()),
                        DockerInstruction::Cmd { args, .. } => df_cmd = Some(args.clone()),
                        DockerInstruction::Entrypoint { args, .. } => df_entrypoint = Some(args.clone()),
                        _ => {}
                    }
                }
            }

            base
        } else {
            return Err(anyhow::anyhow!("No Dockerfile found in {:?}", dir));
        }
    } else {
        args.image.clone()
    };

    info!("Running image: {}", resolved_image);

    // Check local store first; pull only if not present
    let mut image = match store.database().get_image_by_tag(&resolved_image).await? {
        Some(img) => img,
        None => {
            eprintln!("Image not found locally, pulling {}...", resolved_image);
            store.pull_image(&resolved_image).await?
        }
    };

    if is_df_build {
        if let Some(e) = df_entrypoint { image.entrypoint = e; }
        if let Some(c) = df_cmd { image.cmd = c; }
        if !df_env.is_empty() { image.env.extend(df_env); }
    }

    let container_id = format!("crush_{}", hex_encode_random());
    let container_name = args.name.clone().unwrap_or_else(|| format!("crush_{}", &container_id[6..14]));
    println!("Creating container {} from {}", container_name, image.tag);

    // Extract image layers into a temporary rootfs
    let rootfs = data_dir.join("containers").join(&container_id).join("rootfs");
    tokio::fs::create_dir_all(&rootfs).await
        .map_err(|e| CrushError::StorageError(format!("Failed to create rootfs: {}", e)))?;

    store.extract_layers(&image.id, &rootfs).await?;

    // Copy project files if it's a Dockerfile build
    if is_df_build {
        if let Some(ref p_dir) = project_dir {
            let app_dir = rootfs.join("app");
            tokio::fs::create_dir_all(&app_dir).await?;
            copy_project_into_rootfs(p_dir, &app_dir).await?;
        }
    }

    // Build effective command: entrypoint + cmd, falling back to /bin/sh
    let effective_cmd: Vec<String> = if !image.entrypoint.is_empty() {
        let mut v = image.entrypoint.clone();
        v.extend(image.cmd.iter().cloned());
        v
    } else if !image.cmd.is_empty() {
        image.cmd.clone()
    } else {
        vec!["/bin/sh".to_string()]
    };

    let driver = LocalDriver::new(data_dir.to_path_buf());
    
    // Resolve mounts to proper MountConfigs
    let mut resolved_mounts = Vec::new();
    for spec in &args.volume {
        let parts: Vec<&str> = spec.split(':').collect();
        if parts.len() < 2 {
            continue;
        }
        let (src, dest, readonly) = if parts.len() == 2 {
            (parts[0], parts[1], false)
        } else {
            (parts[0], parts[1], parts[2].eq_ignore_ascii_case("ro"))
        };

        let is_host_path = src.starts_with('/') || src.starts_with('.') || src.contains('\\') || src.contains(':');
        let host_path = if is_host_path {
            PathBuf::from(src)
        } else {
            let vol_name = if driver.inspect(src).await.is_ok() {
                src.to_string()
            } else {
                let anon_name = format!("anon_{}", &container_id[6..14]);
                let mut labels = std::collections::HashMap::new();
                labels.insert("anonymous".to_string(), container_id.clone());
                driver.create(&anon_name, labels).await?;
                anon_name
            };
            driver.path(&vol_name).await?
        };

        resolved_mounts.push(MountConfig {
            host_path,
            container_path: PathBuf::from(dest),
            read_only: readonly,
            is_tmpfs: false,
        });
    }

    let container = Container {
        id: container_id.clone(),
        name: container_name.clone(),
        image: image.tag.clone(),
        status: ContainerStatus::Creating,
        pid: None,
        created_at: SystemTime::now(),
        started_at: None,
        ports: vec![],
        mounts: resolved_mounts,
        memory_limit_bytes: args.memory.map(|m| m * 1024 * 1024),
        cpu_shares: args.cpu,
        health: None,
        restart_count: Some(0),
        restart_policy: Some(args.restart.clone()),
        health_cmd: args.health_cmd.clone(),
        health_interval: Some(args.health_interval),
        health_timeout: Some(args.health_timeout),
        health_retries: Some(args.health_retries),
        pids_limit: args.pids_limit,
        read_only: Some(args.read_only),
        security_opt: Some(args.security_opt.clone()),
    };

    let container_dir = data_dir.join("containers").join(&container_id);
    let container_json_path = container_dir.join("container.json");
    
    #[cfg(target_os = "windows")]
    let backend = WindowsRuntime::new();
    #[cfg(not(target_os = "windows"))]
    let backend = StatelessEngine::new(data_dir.to_path_buf());
    backend.create(&container, &container_dir).await?;

    // Save config.json containing the command and env
    let config_json = serde_json::json!({
        "cmd": effective_cmd,
        "env": image.env.clone(),
    });
    let config_json_path = container_dir.join("config.json");
    let config_json_str = serde_json::to_string_pretty(&config_json)?;
    tokio::fs::write(&config_json_path, config_json_str).await?;

    if args.detach {
        backend.start(&container_id).await?;
        println!("{}", container_id);
    } else {
        // Foreground/synchronous execution: spawn crush internal-run <id> synchronously
        let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("crush"));
        let mut cmd = std::process::Command::new(current_exe);
        cmd.arg("internal-run").arg(&container_id);

        let mut child = cmd.spawn()
            .map_err(|e| CrushError::Internal(anyhow::anyhow!("Failed to spawn foreground container: {}", e)))?;
        job_object::assign_std(&child);

        let pid = child.id();
        let mut c_upd = container.clone();
        c_upd.status = ContainerStatus::Running;
        c_upd.pid = Some(pid);
        c_upd.started_at = Some(SystemTime::now());
        let serialized = serde_json::to_string_pretty(&c_upd)?;
        tokio::fs::write(&container_json_path, serialized).await?;

        let status = child.wait()
            .map_err(|e| CrushError::Internal(anyhow::anyhow!("Failed to wait for foreground container: {}", e)))?;

        // On exit, set status to Stopped
        if let Ok(content) = tokio::fs::read_to_string(&container_json_path).await {
            if let Ok(mut c_exit) = serde_json::from_str::<Container>(&content) {
                c_exit.status = ContainerStatus::Stopped;
                c_exit.pid = None;
                if let Ok(serialized_exit) = serde_json::to_string_pretty(&c_exit) {
                    let _ = tokio::fs::write(&container_json_path, serialized_exit).await;
                }
            }
        }

        if let Some(code) = status.code() {
            if code != 0 {
                std::process::exit(code);
            }
        }
    }
    Ok(())
}
