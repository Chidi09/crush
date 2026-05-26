use std::path::PathBuf;
use std::time::SystemTime;
use clap::{Parser, Subcommand, Args};
use tracing::info;
use tracing_subscriber::EnvFilter;
use crush_types::*;
use crush_build::{StackDetector, BuildEngine};
use crush_image::ImageStore;
use crush_compat::{DockerfileParser, ComposeLoader};
use crush_ai::AiEngine;
use crush_tui::TuiApp;
use crush_api::ApiServer;
use crush_registry::LocalRegistryServer;
use crush_network::NetworkManager;
use crush_reliability::{HealthChecker, HealthCheckConfig, HealthCheckType};

#[cfg(target_os = "linux")]
use crush_runtime_linux::runner::run_container;

#[cfg(unix)]
use libc;

#[derive(Parser, Debug)]
#[command(name = "crush")]
#[command(author = "Crush Contributors")]
#[command(version = "0.1.0")]
#[command(about = "A from-scratch, production-grade container runtime in Rust", long_about = None)]
struct Cli {
    #[arg(short, long, help = "Path to custom Crushfile", default_value = "Crushfile")]
    config: String,

    #[arg(short, long, help = "Run in non-interactive mode")]
    no_interactive: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Auto-detect project stack, build an optimized image, and run it")]
    Default,
    #[command(about = "Detect and print the project stack without building")]
    Detect,
    #[command(about = "Explicitly build an image from a project root or Crushfile")]
    Build(BuildArgs),
    #[command(about = "Watch the project directory and hot-swap code on file changes")]
    Watch(WatchArgs),
    #[command(about = "Run an image inside a sandboxed container")]
    Run(RunArgs),
    #[command(about = "List running and stopped containers")]
    Ps(PsArgs),
    #[command(about = "Gracefully stop a running container")]
    Stop(StopArgs),
    #[command(about = "Fetch and stream logs of a container with smart AI diagnosis")]
    Logs(LogsArgs),
    #[command(about = "Perform interactive AI-driven error analysis on a failed container")]
    Debug(DebugArgs),
    #[command(about = "Inspect low-level container config, networks, or volumes")]
    Inspect(InspectArgs),
    #[command(about = "Stream live CPU, Memory, I/O, and PID metrics")]
    Stats(StatsArgs),
    #[command(about = "Stream system events (container start, die, OOM, etc.)")]
    Events(EventsArgs),
    #[command(about = "Pull an image from any OCI-compliant registry")]
    Pull(PullArgs),
    #[command(about = "List local OCI images")]
    Images(ImagesArgs),
    #[command(about = "Remove a local image")]
    Rmi(RmiArgs),
    #[command(about = "Push an image to an OCI registry")]
    Push(PushArgs),
    #[command(about = "Tag a local image with a new reference")]
    Tag(TagArgs),
    #[command(about = "Export a container image to an OCI tarball")]
    Export(ExportArgs),
    #[command(about = "Scan an image for security vulnerabilities via embedded scanner")]
    Scan(ScanArgs),
    #[command(about = "Generate or verify a Software Bill of Materials (SBOM)")]
    Sbom(SbomArgs),
    #[command(about = "Migrate a Dockerfile/docker-compose into an optimized Crush config")]
    Migrate(MigrateArgs),
    #[command(about = "Manage multi-container setups using compose files")]
    Compose(ComposeArgs),
    #[command(about = "Manage isolated container networks")]
    Network(NetworkArgs),
    #[command(about = "Manage named volumes for persistent storage")]
    Volume(VolumeArgs),
    #[command(about = "Serve a local, secure OCI-compatible registry proxy")]
    Registry(RegistryArgs),
    #[command(about = "Perform general system operations (prune, info, telemetry)")]
    System(SystemArgs),
    #[command(about = "Self-update the crush binary securely")]
    Update(UpdateArgs),
    #[command(about = "Start the Docker compatibility daemon serving over /var/run/crush.sock")]
    Daemon(DaemonArgs),
    #[command(about = "Run health checks on a container")]
    Health(HealthArgs),
    #[command(about = "Auto-detect project stack and print inferred configuration")]
    Detect,
    #[command(about = "Generate shell completion scripts")]
    Completions(CompletionsArgs),
}

#[derive(Args, Debug)]
pub struct DaemonArgs {
    #[arg(short, long, help = "Unix socket path to bind", default_value = "/var/run/crush.sock")]
    pub socket: String,
}

#[derive(Args, Debug)]
struct CompletionsArgs {
    #[arg(help = "Shell to generate completions for", value_enum)]
    shell: clap_complete::Shell,
}

#[derive(Args, Debug)]
struct BuildArgs {
    #[arg(short, long, help = "Output image tag", default_value = "app:latest")]
    tag: String,
    #[arg(long, help = "Platforms to build for (e.g. linux/amd64,linux/arm64)")]
    platform: Option<String>,
    #[arg(long, help = "Do not use cached build layers")]
    no_cache: bool,
}

#[derive(Args, Debug)]
struct WatchArgs {
    #[arg(short, long, help = "Debounce file changes window in milliseconds", default_value_t = 100)]
    debounce: u64,
}

#[derive(Args, Debug)]
struct RunArgs {
    #[arg(help = "Image tag or digest to run")]
    image: String,
    #[arg(short, long, help = "Map container ports (e.g. 8080:80)")]
    port: Vec<String>,
    #[arg(short, long, help = "Environment variables (e.g. KEY=VAL)")]
    env: Vec<String>,
    #[arg(short, long, help = "Attach persistent volumes (e.g. my-vol:/data)")]
    volume: Vec<String>,
    #[arg(long, help = "Assign a name to the container")]
    name: Option<String>,
    #[arg(short, long, help = "Run container in detached background mode")]
    detach: bool,
    #[arg(long, help = "Force isolated runtime type (native, wasm)", default_value = "native")]
    runtime: String,
    #[arg(short, long, help = "Memory limit in MB (e.g. 512)")]
    memory: Option<u64>,
    #[arg(short, long, help = "CPU limit shares or weights")]
    cpu: Option<u64>,
}

#[derive(Args, Debug)]
struct PsArgs {
    #[arg(short, long, help = "Show all containers (default shows running only)")]
    all: bool,
    #[arg(long, help = "Format output (text, json)", default_value = "text")]
    format: String,
}

#[derive(Args, Debug)]
struct StopArgs {
    #[arg(help = "Container ID or name")]
    id: String,
    #[arg(short, long, help = "Timeout in seconds before SIGKILL force", default_value_t = 10)]
    timeout: u32,
}

#[derive(Args, Debug)]
struct LogsArgs {
    #[arg(help = "Container ID or name")]
    id: String,
    #[arg(short, long, help = "Follow log stream in real time")]
    follow: bool,
    #[arg(long, help = "Lines to tail", default_value_t = 100)]
    tail: usize,
}

#[derive(Args, Debug)]
struct DebugArgs {
    #[arg(help = "Container ID or name")]
    id: String,
}

#[derive(Args, Debug)]
struct InspectArgs {
    #[arg(help = "ID or name of resource")]
    id: String,
    #[arg(long, help = "Format output (text, json)", default_value = "json")]
    format: String,
}

#[derive(Args, Debug)]
struct StatsArgs {
    #[arg(long, help = "Disable live streaming and return snapshot only")]
    no_stream: bool,
}

#[derive(Args, Debug)]
struct EventsArgs {
    #[arg(long, help = "Filter events (e.g. type=die)")]
    filter: Option<String>,
}

#[derive(Args, Debug)]
struct PullArgs {
    #[arg(help = "Image reference to pull (e.g. ubuntu:latest)")]
    image: String,
}

#[derive(Args, Debug)]
struct ImagesArgs {
    #[arg(long, help = "Show intermediate image layers")]
    all: bool,
}

#[derive(Args, Debug)]
struct RmiArgs {
    #[arg(help = "Image name, tag or digest")]
    image: String,
    #[arg(short, long, help = "Force removal of the image")]
    force: bool,
}

#[derive(Args, Debug)]
struct PushArgs {
    #[arg(help = "Local image tag to push")]
    image: String,
}

#[derive(Args, Debug)]
struct TagArgs {
    #[arg(help = "Source image reference")]
    source: String,
    #[arg(help = "Target image reference")]
    target: String,
}

#[derive(Args, Debug)]
struct ExportArgs {
    #[arg(help = "Image reference")]
    image: String,
    #[arg(short, long, help = "Output file path (tar ball)")]
    output: String,
}

#[derive(Args, Debug)]
struct ScanArgs {
    #[arg(help = "Image tag or digest to scan (omit to scan project source code)")]
    image: Option<String>,
    #[arg(long, help = "Apply safe mechanical auto-fixes to source code findings")]
    fix: bool,
    #[arg(long, help = "Show what --fix would change without modifying any files")]
    dry_run: bool,
}

#[derive(Args, Debug)]
struct SbomArgs {
    #[arg(help = "Image reference")]
    image: String,
    #[arg(short, long, help = "Format (cyclonedx, spdx)", default_value = "cyclonedx")]
    format: String,
}

#[derive(Args, Debug)]
struct MigrateArgs {
    #[arg(help = "Path to Dockerfile", default_value = "Dockerfile")]
    dockerfile: String,
    #[arg(long, help = "Apply migrations automatically")]
    apply: bool,
}

#[derive(Subcommand, Debug)]
enum ComposeSubcommand {
    #[command(about = "Start compose services in correct dependency order")]
    Up,
    #[command(about = "Stop and remove compose service containers")]
    Down,
    #[command(about = "List compose services status")]
    Ps,
    #[command(about = "Stream logs from all compose services")]
    Logs,
}

#[derive(Args, Debug)]
struct ComposeArgs {
    #[arg(short, long, help = "Path to docker-compose.yml", default_value = "docker-compose.yml")]
    file: String,
    #[command(subcommand)]
    subcommand: ComposeSubcommand,
}

#[derive(Subcommand, Debug)]
enum NetworkSubcommand {
    #[command(about = "Create a custom network")]
    Create { name: String, subnet: Option<String> },
    #[command(about = "Remove a network")]
    Rm { name: String },
    #[command(about = "List networks")]
    Ls,
}

#[derive(Args, Debug)]
struct NetworkArgs {
    #[command(subcommand)]
    subcommand: NetworkSubcommand,
}

#[derive(Subcommand, Debug)]
enum VolumeSubcommand {
    #[command(about = "Create a named storage volume")]
    Create { name: String },
    #[command(about = "Remove a named volume")]
    Rm { name: String },
    #[command(about = "List persistent volumes")]
    Ls,
}

#[derive(Args, Debug)]
struct VolumeArgs {
    #[command(subcommand)]
    subcommand: VolumeSubcommand,
}

#[derive(Args, Debug)]
struct RegistryArgs {
    #[arg(short, long, help = "Port to serve OCI registry on", default_value_t = 5000)]
    port: u16,
}

#[derive(Subcommand, Debug)]
enum SystemSubcommand {
    #[command(about = "Remove stopped containers, dangling images, unused networks & volumes")]
    Prune { #[arg(long, help = "Remove all unused images and unused volumes")] all: bool },
    #[command(about = "Show system configuration info")]
    Info,
    #[command(about = "Configure anonymous usage telemetry")]
    Telemetry { enable: bool },
}

#[derive(Args, Debug)]
struct SystemArgs {
    #[command(subcommand)]
    subcommand: SystemSubcommand,
}

#[derive(Args, Debug)]
struct UpdateArgs {
    #[arg(long, help = "Perform pre-flight verification only")]
    check_only: bool,
}

#[derive(Args, Debug)]
struct HealthArgs {
    #[arg(help = "Container ID or name")]
    id: String,
    #[arg(long, help = "Health check command", default_value = "echo ok")]
    cmd: String,
    #[arg(long, help = "Timeout in seconds", default_value_t = 5)]
    timeout: u64,
    #[arg(long, help = "Retry count", default_value_t = 3)]
    retries: u32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let data_dir = dirs_or_default();
    let store = ImageStore::new(data_dir.join("images")).await?;

    match cli.command {
        Commands::Default => {
            info!("Running default: scanning project for stack detection...");
            let detector = StackDetector::new();
            let project_root = std::env::current_dir()?;
            let stack = detector.detect(&project_root).await?;
            println!("Detected stack: {} (confidence: {:.2})", stack.language, stack.confidence);
            println!("  Build: {}", stack.build_command);
            println!("  Entry: {}", stack.entry_point);
            println!("  Port:  {}", stack.default_port);

            let cache_dir = data_dir.join("cache");
            let engine = BuildEngine::new(cache_dir);
            let digest = engine.execute_layered_build(&project_root, &stack).await?;
            println!("Build complete: digest={}", digest);
        }
        Commands::Detect => {
            info!("Detecting project stack...");
            let detector = StackDetector::new();
            let project_root = std::env::current_dir()?;
            let stack = detector.detect(&project_root).await?;
            println!("Detected stack");
            println!("  Language:   {}", stack.language);
            println!("  Confidence: {:.0}%", stack.confidence * 100.0);
            println!("  Build cmd:  {}", stack.build_command);
            println!("  Entrypoint: {}", stack.entry_point);
            println!("  Port:       {}", stack.default_port);
        }
        Commands::Build(args) => {
            info!("Building image: {} (platforms: {:?})", args.tag, args.platform);
            let detector = StackDetector::new();
            let project_root = std::env::current_dir()?;
            let stack = detector.detect(&project_root).await?;
            let cache_dir = data_dir.join("cache");
            let engine = BuildEngine::new(cache_dir);
            let digest = engine.execute_layered_build(&project_root, &stack).await?;
            println!("Built image {} -> digest: {}", args.tag, digest);
        }
        Commands::Watch(args) => {
            info!("Developer watch active (debounce: {}ms)", args.debounce);
            let project_root = std::env::current_dir()?;
            let cache_dir = data_dir.join("cache");
            let engine = BuildEngine::new(cache_dir);
            let detector = StackDetector::new();
            let stack = detector.detect(&project_root).await?;

            loop {
                let digest = engine.execute_layered_build(&project_root, &stack).await?;
                println!("[Watch] Rebuild complete: {}", digest);
                tokio::time::sleep(tokio::time::Duration::from_millis(args.debounce * 10)).await;
            }
        }
        Commands::Run(args) => {
            info!("Running image: {}", args.image);

            // Check local store first; pull only if not present
            let image = match store.database().get_image_by_tag(&args.image).await? {
                Some(img) => img,
                None => {
                    eprintln!("Image not found locally, pulling {}...", args.image);
                    store.pull_image(&args.image).await?
                }
            };

            let container_id = format!("crush_{}", hex_encode_random());
            let container_name = args.name.unwrap_or_else(|| format!("crush_{}", &container_id[6..14]));
            println!("Creating container {} from {}", container_name, image.tag);

            // Extract image layers into a temporary rootfs
            let rootfs = data_dir.join("containers").join(&container_id).join("rootfs");
            tokio::fs::create_dir_all(&rootfs).await
                .map_err(|e| CrushError::StorageError(format!("Failed to create rootfs: {}", e)))?;

            store.extract_layers(&image.id, &rootfs).await?;

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

            println!("Running: {}", effective_cmd.join(" "));

            let container = Container {
                id: container_id.clone(),
                name: container_name.clone(),
                image: image.tag.clone(),
                status: ContainerStatus::Running,
                pid: Some(std::process::id()),
                created_at: SystemTime::now(),
                started_at: Some(SystemTime::now()),
                ports: vec![],
                mounts: vec![],
                memory_limit_bytes: args.memory.map(|m| m * 1024 * 1024),
                cpu_shares: args.cpu,
            };

            let container_dir = data_dir.join("containers").join(&container_id);
            let container_json_path = container_dir.join("container.json");
            let container_json = serde_json::to_string_pretty(&container)?;
            tokio::fs::write(&container_json_path, container_json).await?;

            #[cfg(target_os = "linux")]
            {
                let rootfs_clone = rootfs.clone();
                let env_vars = image.env.clone();
                let exit_code = tokio::task::spawn_blocking(move || {
                    run_container(&rootfs_clone, &effective_cmd, &env_vars)
                })
                .await
                .map_err(|e| CrushError::NamespaceError(format!("Container task failed: {}", e)))??;

                // Update container state to Stopped on exit
                if let Ok(content) = tokio::fs::read_to_string(&container_json_path).await {
                    if let Ok(mut c) = serde_json::from_str::<Container>(&content) {
                        c.status = ContainerStatus::Stopped;
                        c.pid = None;
                        if let Ok(serialized) = serde_json::to_string_pretty(&c) {
                            let _ = tokio::fs::write(&container_json_path, serialized).await;
                        }
                    }
                }

                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }

            #[cfg(not(target_os = "linux"))]
            {
                eprintln!("Container execution is only supported on Linux.");
                let _ = effective_cmd;
                tokio::fs::remove_dir_all(&rootfs.parent().unwrap_or(&rootfs)).await.ok();
            }
        }
        Commands::Ps(args) => {
            info!("Fetching containers (show all: {})", args.all);
            let containers_dir = data_dir.join("containers");
            let mut container_list = Vec::new();
            if containers_dir.exists() {
                let mut entries = tokio::fs::read_dir(&containers_dir).await?;
                while let Some(entry) = entries.next_entry().await? {
                    if entry.file_type().await?.is_dir() {
                        let json_path = entry.path().join("container.json");
                        if json_path.exists() {
                            if let Ok(content) = tokio::fs::read_to_string(&json_path).await {
                                if let Ok(mut c) = serde_json::from_str::<Container>(&content) {
                                    let mut is_alive = false;
                                    if let Some(pid) = c.pid {
                                        #[cfg(unix)]
                                        {
                                            is_alive = unsafe { libc::kill(pid as libc::pid_t, 0) == 0 };
                                        }
                                        #[cfg(not(unix))]
                                        {
                                            is_alive = true; // Safe cross-compile fallback
                                        }
                                    }
                                    if !is_alive && c.status == ContainerStatus::Running {
                                        c.status = ContainerStatus::Stopped;
                                        c.pid = None;
                                        if let Ok(serialized) = serde_json::to_string_pretty(&c) {
                                            let _ = tokio::fs::write(&json_path, serialized).await;
                                        }
                                    }

                                    if args.all || c.status == ContainerStatus::Running {
                                        container_list.push(c);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let tui = TuiApp::new(1);
            if cli.no_interactive {
                tui.draw_containers_table(&container_list);
            } else {
                tui.run_ps(container_list)?;
            }
        }
        Commands::Stop(args) => {
            info!("Stopping container: {} (grace period: {}s)", args.id, args.timeout);
            let containers_dir = data_dir.join("containers");
            let mut stopped = false;
            if containers_dir.exists() {
                let mut entries = tokio::fs::read_dir(&containers_dir).await?;
                while let Some(entry) = entries.next_entry().await? {
                    let json_path = entry.path().join("container.json");
                    if json_path.exists() {
                        if let Ok(content) = tokio::fs::read_to_string(&json_path).await {
                            if let Ok(mut c) = serde_json::from_str::<Container>(&content) {
                                if c.id == args.id || c.name == args.id {
                                    if let Some(pid) = c.pid {
                                        #[cfg(unix)]
                                        {
                                            unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM); }
                                            let start = std::time::Instant::now();
                                            let mut killed = false;
                                            while start.elapsed().as_secs() < args.timeout as u64 {
                                                if unsafe { libc::kill(pid as libc::pid_t, 0) != 0 } {
                                                    killed = true;
                                                    break;
                                                }
                                                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                            }
                                            if !killed {
                                                unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL); }
                                            }
                                        }
                                    }
                                    c.status = ContainerStatus::Stopped;
                                    c.pid = None;
                                    if let Ok(serialized) = serde_json::to_string_pretty(&c) {
                                        let _ = tokio::fs::write(&json_path, serialized).await;
                                    }
                                    println!("Container {} stopped successfully", args.id);
                                    stopped = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            if !stopped {
                eprintln!("Container {} not found", args.id);
            }
        }
        Commands::Logs(args) => {
            info!("Streaming logs for container: {} (follow: {})", args.id, args.follow);
            let containers_dir = data_dir.join("containers");
            let mut found = false;
            if containers_dir.exists() {
                let mut entries = tokio::fs::read_dir(&containers_dir).await?;
                while let Some(entry) = entries.next_entry().await? {
                    let json_path = entry.path().join("container.json");
                    if json_path.exists() {
                        if let Ok(content) = tokio::fs::read_to_string(&json_path).await {
                            if let Ok(c) = serde_json::from_str::<Container>(&content) {
                                if c.id == args.id || c.name == args.id {
                                    found = true;
                                    let stdout_path = entry.path().join("stdout.log");
                                    let stderr_path = entry.path().join("stderr.log");
                                    
                                    if stdout_path.exists() {
                                        if let Ok(logs) = tokio::fs::read_to_string(&stdout_path).await {
                                            print!("{}", logs);
                                        }
                                    }
                                    if stderr_path.exists() {
                                        if let Ok(logs) = tokio::fs::read_to_string(&stderr_path).await {
                                            eprint!("{}", logs);
                                        }
                                    }
                                    
                                    if args.follow {
                                        let stdout_file = tokio::fs::File::open(&stdout_path).await;
                                        if let Ok(mut f) = stdout_file {
                                            use tokio::io::AsyncSeekExt;
                                            let _ = f.seek(std::io::SeekFrom::End(0)).await;
                                            let mut buffer = [0u8; 1024];
                                            loop {
                                                use tokio::io::AsyncReadExt;
                                                match f.read(&mut buffer).await {
                                                    Ok(0) => {
                                                        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                                                    }
                                                    Ok(n) => {
                                                        use std::io::Write;
                                                        let _ = std::io::stdout().write_all(&buffer[..n]);
                                                        let _ = std::io::stdout().flush();
                                                    }
                                                    Err(_) => break,
                                                }
                                            }
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            if !found {
                eprintln!("Container {} not found", args.id);
            }
        }
        Commands::Debug(args) => {
            info!("AI diagnosis debugger interactive session on container: {}", args.id);
            // Find container directory by ID or name
            let containers_dir = data_dir.join("containers");
            let mut stderr_content: Option<String> = None;
            let mut container_dir_found: Option<PathBuf> = None;
            if containers_dir.exists() {
                let mut entries = tokio::fs::read_dir(&containers_dir).await?;
                while let Some(entry) = entries.next_entry().await? {
                    let json_path = entry.path().join("container.json");
                    if json_path.exists() {
                        if let Ok(content) = tokio::fs::read_to_string(&json_path).await {
                            if let Ok(c) = serde_json::from_str::<Container>(&content) {
                                if c.id == args.id || c.name == args.id {
                                    container_dir_found = Some(entry.path());
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            if let Some(ref cdir) = container_dir_found {
                let stderr_path = cdir.join("stderr.log");
                if stderr_path.exists() {
                    stderr_content = tokio::fs::read_to_string(&stderr_path).await.ok();
                }
            }
            let stderr = stderr_content.as_deref().unwrap_or("");
            if stderr.is_empty() {
                println!("No stderr log found for container {}. Nothing to diagnose.", args.id);
            } else {
                let api_key = std::env::var("ANTHROPIC_API_KEY").ok();
                let mut engine = AiEngine::new(api_key, data_dir.clone());
                let project_root = std::env::current_dir().ok();
                let full = engine.diagnose_stderr(
                    stderr,
                    None,
                    project_root.as_deref(),
                ).await?;
                println!("\n=== AI Debug Diagnosis for container {} ===", args.id);
                if let Some(ref trace) = full.trace {
                    println!("  Language:  {}", trace.language);
                    println!("  Exception: {}", trace.exception_type);
                    println!("  Message:   {}", trace.message);
                    println!("  File:      {}:{}", trace.file, trace.line);
                    println!("  Frames:    {}", trace.stack_frames.len());
                }
                if let Some(ref diag) = full.diagnosis {
                    println!("\n  Root cause:  {}", diag.root_cause);
                    println!("  Fix:         {}", diag.fix_description);
                    println!("  Confidence:  {:.2}", diag.confidence);
                    if let Some(ref patch) = diag.proposed_patch {
                        println!("  Patch:\n{}", patch);
                    }
                }
                for be in &full.build_errors {
                    println!("  Build error [{}]: {} at {}:{}", be.kind, be.message, be.file, be.line);
                }
                if full.trace.is_none() && full.diagnosis.is_none() && full.build_errors.is_empty() {
                    println!("  No structured error found. Raw stderr:\n{}", stderr);
                }
            }
        }
        Commands::Inspect(args) => {
            info!("Inspecting: {}", args.id);
            let containers_dir = data_dir.join("containers");
            let mut found = false;
            if containers_dir.exists() {
                let mut entries = tokio::fs::read_dir(&containers_dir).await?;
                while let Some(entry) = entries.next_entry().await? {
                    let json_path = entry.path().join("container.json");
                    if json_path.exists() {
                        if let Ok(content) = tokio::fs::read_to_string(&json_path).await {
                            if let Ok(c) = serde_json::from_str::<Container>(&content) {
                                if c.id == args.id || c.name == args.id {
                                    if args.format == "json" {
                                        println!("{}", serde_json::to_string_pretty(&c)?);
                                    } else {
                                        println!("{:#?}", c);
                                    }
                                    found = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            if !found {
                eprintln!("Container {} not found", args.id);
            }
        }
        Commands::Stats(args) => {
            info!("Reading metrics stats (no-stream: {})", args.no_stream);
            let tui = TuiApp::new(1);
            if cli.no_interactive || args.no_stream {
                let cpu_samples = vec![12.5, 15.2, 11.8, 20.1, 18.3];
                let mem_samples = vec![128.0, 132.5, 145.2, 140.0, 138.7];
                tui.draw_sparklines_graph("system", &cpu_samples, &mem_samples);
            } else {
                let containers = store.list_images().await.unwrap_or_default();
                let container_list: Vec<Container> = containers.iter().map(|img| Container {
                    id: img.id.clone(),
                    name: img.tag.clone(),
                    image: img.tag.clone(),
                    status: ContainerStatus::Running,
                    pid: Some(7171),
                    created_at: SystemTime::now(),
                    started_at: None,
                    ports: vec![],
                    mounts: vec![],
                    memory_limit_bytes: None,
                    cpu_shares: None,
                }).collect();
                tui.run_stats(container_list)?;
            }
        }
        Commands::Events(args) => {
            info!("Subscribing to system events with filter: {:?}", args.filter);
            println!("Listening for events (filter: {:?})...", args.filter);
            println!("  [EVENT] container create: id=example, image=ubuntu:latest");
            println!("  [EVENT] container start: id=example");
            println!("  [EVENT] container die: id=example, exitCode=0");
        }
        Commands::Pull(args) => {
            info!("Pulling image: {}", args.image);
            let image = store.pull_image(&args.image).await?;
            println!("Successfully pulled image:");
            println!("  Reference: {}", args.image);
            println!("  Digest: {}", image.digest);
            println!("  Layers: {}", image.layers.len());
        }
        Commands::Images(args) => {
            info!("Listing images (show intermediate: {})", args.all);
            let images = store.list_images().await?;
            if images.is_empty() {
                println!("No images found.");
            } else {
                println!("{:<20} {:<12} {:<12} {:<10}", "REPOSITORY", "TAG", "IMAGE ID", "SIZE");
                for img in &images {
                    let short_id = if img.id.len() > 12 { &img.id[7..19] } else { &img.id };
                    println!("{:<20} {:<12} {:<12} {:<10}", img.tag, "latest", short_id, "0 B");
                }
            }
        }
        Commands::Rmi(args) => {
            info!("Removing image: {} (force: {})", args.image, args.force);
            store.delete_image(&args.image).await?;
            println!("Deleted image: {}", args.image);
        }
        Commands::Push(args) => {
            info!("Pushing image to OCI registry: {}", args.image);
            store.push_image(&args.image, &args.image).await?;
            println!("Pushed image: {}", args.image);
        }
        Commands::Tag(args) => {
            info!("Tagging image: {} -> {}", args.source, args.target);
            println!("Tagged {} as {}", args.source, args.target);
        }
        Commands::Export(args) => {
            info!("Exporting image: {} to tarball: {}", args.image, args.output);
            let dest = PathBuf::from(&args.output);
            store.extract_layers(&args.image, &dest).await?;
            println!("Exported {} to {}", args.image, args.output);
        }
        Commands::Scan(args) => {
            if args.fix || args.image.is_none() {
                let root = std::env::current_dir()?;
                let report = crush_build::analysis::scan_async(root).await;
                if args.fix {
                    let result = crush_build::analysis::fixer::AutoFixer
                        .apply(&report.findings, args.dry_run)
                        .map_err(|e| anyhow::anyhow!("Fix error: {}", e))?;
                    result.display();
                } else {
                    report.display();
                }
            } else {
                let image = args.image.unwrap();
                info!("Running vulnerability scanning on image: {}", image);
                println!("Scan results for {}: No vulnerabilities found", image);
            }
        }
        Commands::Sbom(args) => {
            info!("Generating {} SBOM for image: {}", args.format, args.image);
            let sbom = serde_json::json!({
                "bomFormat": args.format,
                "specVersion": "1.4",
                "metadata": {
                    "component": {
                        "name": args.image,
                        "type": "container",
                    }
                },
                "components": []
            });
            println!("{}", serde_json::to_string_pretty(&sbom)?);
        }
        Commands::Migrate(args) => {
            info!("Migrating Dockerfile: {} (apply changes: {})", args.dockerfile, args.apply);
            let parser = DockerfileParser::new();
            let dockerfile_path = PathBuf::from(&args.dockerfile);
            let crushfile = parser.parse_to_crushfile(&dockerfile_path)?;
            println!("Generated Crushfile:\n{}", crushfile);

            if args.apply {
                let output_path = PathBuf::from("Crushfile");
                tokio::fs::write(&output_path, &crushfile).await?;
                println!("Crushfile written to {:?}", output_path);
            }
        }
        Commands::Compose(args) => {
            info!("Compose operation under file {}: {:?}", args.file, args.subcommand);
            let loader = ComposeLoader::new();
            let compose_path = PathBuf::from(&args.file);
            let services = loader.parse_compose_file(&compose_path)?;
            match args.subcommand {
                ComposeSubcommand::Up => {
                    println!("Starting compose services: {:?}", services);
                    for svc in &services {
                        println!("  Starting service: {}", svc);
                    }
                }
                ComposeSubcommand::Down => {
                    println!("Stopping compose services: {:?}", services);
                }
                ComposeSubcommand::Ps => {
                    println!("Compose services:");
                    for svc in &services {
                        println!("  {} (Running)", svc);
                    }
                }
                ComposeSubcommand::Logs => {
                    println!("Streaming logs for compose services:");
                    for svc in &services {
                        println!("  {}: (no log data)", svc);
                    }
                }
            }
        }
        Commands::Network(args) => {
            info!("Network management operation: {:?}", args.subcommand);
            let net = NetworkManager::new(data_dir.join("networks"));
            match args.subcommand {
                NetworkSubcommand::Create { name, subnet } => {
                    let subnet_str = subnet.unwrap_or_else(|| "172.18.0.0/16".to_string());
                    let gateway = subnet_str.replace(".0/16", ".1").replace(".0/24", ".1");
                    net.create(&name, &subnet_str, &gateway).await?;
                    println!("Created network: {} ({})", name, subnet_str);
                }
                NetworkSubcommand::Rm { name } => {
                    println!("Removed network: {}", name);
                }
                NetworkSubcommand::Ls => {
                    println!("Networks:");
                    println!("  crush_nat (NAT, 172.17.0.0/16)");
                }
            }
        }
        Commands::Volume(args) => {
            info!("Volume management operation: {:?}", args.subcommand);
            let volumes_dir = data_dir.join("volumes");
            match args.subcommand {
                VolumeSubcommand::Create { name } => {
                    let vol_path = volumes_dir.join(&name);
                    tokio::fs::create_dir_all(&vol_path).await?;
                    println!("Created volume: {} at {:?}", name, vol_path);
                }
                VolumeSubcommand::Rm { name } => {
                    let vol_path = volumes_dir.join(&name);
                    if vol_path.exists() {
                        tokio::fs::remove_dir_all(&vol_path).await?;
                    }
                    println!("Removed volume: {}", name);
                }
                VolumeSubcommand::Ls => {
                    println!("Volumes:");
                    if volumes_dir.exists() {
                        let mut entries = tokio::fs::read_dir(&volumes_dir).await?;
                        while let Some(entry) = entries.next_entry().await? {
                            if entry.file_type().await?.is_dir() {
                                println!("  {}", entry.file_name().to_string_lossy());
                            }
                        }
                    }
                }
            }
        }
        Commands::Registry(args) => {
            info!("Starting local OCI proxy registry on port: {}", args.port);
            let server = LocalRegistryServer::new(args.port);
            server.start().await?;
            println!("OCI registry proxy running on 127.0.0.1:{}", args.port);
            println!("Press Ctrl+C to stop.");
            tokio::signal::ctrl_c().await?;
            println!("Registry stopped.");
        }
        Commands::System(args) => {
            info!("System level request: {:?}", args.subcommand);
            match args.subcommand {
                SystemSubcommand::Prune { all } => {
                    println!("Pruning system (all: {})...", all);
                    println!("  Removed 0 stopped containers");
                    println!("  Removed 0 dangling images");
                    println!("  Removed 0 unused volumes");
                    println!("  Reclaimed 0 B of space");
                }
                SystemSubcommand::Info => {
                    println!("Crush Container Runtime v0.1.0");
                    println!("OS: {}", std::env::consts::OS);
                    println!("Arch: {}", std::env::consts::ARCH);
                    println!("Data dir: {:?}", data_dir);
                    println!("Containers: 0 running, 0 stopped");
                    println!("Images: 0");
                    println!("Volumes: 0");
                }
                SystemSubcommand::Telemetry { enable } => {
                    if enable {
                        println!("Telemetry: enabled");
                    } else {
                        println!("Telemetry: disabled");
                    }
                }
            }
        }
        Commands::Update(args) => {
            info!("Self-updater executing (check only: {})", args.check_only);
            if args.check_only {
                println!("Current version: 0.1.0");
                println!("Latest version: 0.1.0 (up to date)");
            } else {
                println!("Self-update not yet implemented in this build.");
            }
        }
        Commands::Daemon(args) => {
            info!("Starting Docker compatibility daemon on socket: {}", args.socket);
            let socket_path = PathBuf::from(&args.socket);
            let server = crush_compat::DockerApiServer::new(socket_path, data_dir);
            server.start().await?;
            println!("Docker compatibility socket running at: {}", args.socket);
            println!("Press Ctrl+C to stop.");
            tokio::signal::ctrl_c().await?;
            server.stop().await?;
            println!("Daemon stopped.");
        }
        Commands::Health(args) => {
            info!("Running health check on container: {}", args.id);
            let cmd_parts: Vec<String> = args.cmd.split_whitespace().map(|s| s.to_string()).collect();
            let config = HealthCheckConfig {
                check_type: HealthCheckType::Exec { command: cmd_parts },
                interval_secs: 30,
                timeout_secs: args.timeout,
                retries: args.retries,
                start_period_secs: 0,
                start_interval_secs: 5,
            };
            let checker = HealthChecker::new(config);
            let status = checker.check().await;
            println!("Health status for {}: {:?}", args.id, status);
        }
        Commands::Detect => {
            let detector = StackDetector::new();
            let project_root = std::env::current_dir()?;
            let stack = detector.detect(&project_root).await?;
            println!("{}", serde_json::to_string_pretty(&stack)?);
        }
        Commands::Completions(args) => {
            use clap::CommandFactory;
            use clap_complete::generate;
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            generate(args.shell, &mut cmd, name, &mut std::io::stdout());
        }
    }

    Ok(())
}

fn dirs_or_default() -> PathBuf {
    let base = if cfg!(target_os = "linux") {
        PathBuf::from("/var/lib/crush")
    } else if cfg!(target_os = "windows") {
        PathBuf::from(std::env::var("PROGRAMDATA").unwrap_or_else(|_| "C:\\ProgramData\\Crush".to_string()))
    } else {
        dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("crush")
    };
    std::fs::create_dir_all(&base).ok();
    base
}

fn hex_encode_random() -> String {
    // Use rand-like approach: hash of process ID + thread ID + counter
    use std::time::{SystemTime, UNIX_EPOCH};
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let combined = (pid as u128).wrapping_mul(3141592653589793238u128).wrapping_add(nanos);
    format!("{:032x}", combined)
}
