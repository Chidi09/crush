use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::io::Write;
use clap::{Parser, Subcommand, Args};
use tracing::info;
use tracing_subscriber::EnvFilter;
use crush_types::*;
use crush_build::{StackDetector, BuildEngine};
use crush_build::run::RunEvent;
use crush_build::{
    detect_backend, parse_compose, parse_spring_config,
    stop_dep_service,
    rewrite_env_hostnames,
    save_service_state, load_service_state, clear_service_state,
    ServiceState, RunningContainer,
    StartedService, start_dep_service_smart,
    synthesize_dep_env,
};
use crush_services::{
    save_native_state, load_native_state, clear_native_state,
    NativeServiceState, RunningService as NativeRunningService,
    ServiceDriver,
};
use crush_image::ImageStore;
use crush_compat::{DockerfileParser, ComposeLoader, DockerInstruction};
use owo_colors::OwoColorize;
use crush_ai::AiEngine;
use crush_tui::TuiApp;

use crush_registry::LocalRegistryServer;
#[cfg(target_os = "linux")]
use crush_network::NetworkManager;
use crush_volume::{LocalDriver, VolumeDriver, VolumeMounter};
use crush_reliability::{
    HealthChecker, HealthCheckConfig, HealthCheckType, RestartManager, RestartPolicy,
    OomMonitor, OomPolicy, OomEvent, SecretManager, SecretSpec, SecretSource, SecretDestination,
    VaultEngine
};
mod runtime;
use runtime::StatelessEngine;
mod job_object;
mod commands;
use std::sync::Arc;
use crush_build::run::Stream;

#[cfg(target_os = "windows")]
use crush_runtime_windows::WindowsRuntime;

#[cfg(target_os = "linux")]
use crush_runtime_linux::runner::run_container;

#[cfg(unix)]
use libc;

#[derive(Parser, Debug)]
#[command(name = "crush")]
#[command(author = "Crush Contributors")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "A from-scratch, production-grade container runtime in Rust", long_about = None)]
#[command(subcommand_required = false, arg_required_else_help = false)]
struct Cli {
    #[arg(short, long, help = "Path to custom Crushfile", default_value = "Crushfile")]
    config: String,

    #[arg(short, long, help = "Run in non-interactive mode")]
    no_interactive: bool,

    #[arg(long, short = 'D', help = "Run dev-mode (HMR / watch / reload) instead of prod")]
    dev: bool,

    #[arg(long, help = "Watch source files and restart affected services on change")]
    watch: bool,

    #[arg(long, help = "Force a rebuild even if existing artifacts look fresh")]
    rebuild: bool,

    #[arg(long, help = "Force re-packing the image even if sources unchanged")]
    repack: bool,

    #[arg(long, value_parser = parse_size, help = "Memory cap (e.g. 4G, 512M). Process tree dies on exceed.")]
    memory: Option<u64>,
    #[arg(long, value_parser = parse_cpu_fraction, help = "CPU cap (e.g. 0.5, 2). 1.0 = one core.")]
    cpus: Option<f32>,
    #[arg(long, help = "Boost process priority. Values: high, above-normal. Windows only.")]
    priority: Option<String>,

    #[arg(long, help = "Disable the built-in reverse proxy")]
    no_proxy: bool,

    #[arg(long, help = "Expose the app's port to the public internet (webhooks) once it binds")]
    tunnel: bool,
    #[arg(long, help = "Tunnel provider: cloudflared (default), ngrok, or outray")]
    tunnel_provider: Option<String>,

    #[arg(long, help = "Capture the app's outgoing email into a local SMTP sink on :1025 (run `crush mail` to view)")]
    mail: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Auto-detect project stack, build an optimized image, and run it")]
    Default,
    #[command(about = "Detect and print the project stack without building")]
    Detect(DetectArgs),
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
    #[command(about = "Run a command in a running container's environment")]
    Exec(ExecArgs),
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
    #[command(about = "Log in to a remote OCI-compliant registry")]
    Login(LoginArgs),
    #[command(about = "Perform general system operations (prune, info, telemetry)")]
    System(SystemArgs),
    #[command(about = "Self-update the crush binary securely")]
    Update(UpdateArgs),
    #[command(about = "Start the Docker compatibility daemon serving over /var/run/crush.sock")]
    Daemon(DaemonArgs),
    #[command(about = "Install crush to a system directory and add it to PATH")]
    Install,
    #[command(about = "Manage crush-started dependency services (postgres, redis, etc.)")]
    Services(ServicesArgs),
    #[command(about = "Generate production Dockerfile + docker-compose from current detection (stop relying on crush)")]
    Eject(EjectArgs),
    #[command(about = "Remove build artifacts, dependency installs, and tool caches from a project")]
    Clean(CleanArgs),
    #[command(about = "Generate a CI/CD pipeline (GitHub Actions, AppVeyor, or Codemagic) for the detected stack")]
    Ci(CiArgs),
    #[command(about = "Browse a curated catalog of popular images you can pull")]
    Catalog(CatalogArgs),
    #[command(about = "Expose a local port to the public internet (webhooks/Paystack/Stripe) via a free tunnel")]
    Tunnel(TunnelArgs),
    #[command(about = "Snapshot & restore database state (Postgres/MySQL) — a Time Machine for your data")]
    Db(DbArgs),
    #[command(about = "Check for cross-OS landmines (case-sensitive import paths) before deploying to Linux")]
    Lint(LintArgs),
    #[command(about = "Run a local mail catcher (SMTP sink on :1025) that captures dev emails instead of sending them")]
    Mail(MailArgs),
    #[command(about = "Run the blue-green traffic gateway (or --set its upstream). Used by zero-downtime deploys.")]
    Gateway(GatewayArgs),
    #[command(about = "Run the L7 proxy (HTTP/TLS) reading mapped domains from a file")]
    L7Gateway(L7GatewayArgs),
    #[command(about = "List servers from your ~/.ssh/config and connect (crush ssh <host>)")]
    Ssh(SshArgs),
    #[command(about = "Run health checks on a container")]
    Health(HealthArgs),
    #[command(about = "Deploy a project to cloud infrastructure defined in the Crushfile")]
    Deploy(DeployArgs),
    #[command(about = "Configure Docker CLI and tools to use Crush as the container backend")]
    DockerContext(DockerContextArgs),
    #[command(about = "Generate shell completion scripts")]
    Completions(CompletionsArgs),
    #[command(about = "Show recent build history (data_dir/build-history.json)")]
    History(HistoryArgs),
    #[command(hide = true)]
    InternalRun(InternalRunArgs),
    #[command(hide = true)]
    __Complete(CompleteArgs),
}

#[derive(Args, Debug)]
pub struct DockerContextArgs {
    #[arg(long, help = "Create a Docker context named 'crush' (requires docker CLI in PATH)")]
    create: bool,
    #[arg(long, help = "Custom socket path override")]
    socket: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct CompleteArgs {
    #[arg(help = "The category of completions to fetch (containers, images, networks, volumes)")]
    pub category: String,
}

#[derive(Args, Debug)]
pub struct InternalRunArgs {
    #[arg(help = "Container ID to run")]
    pub id: String,
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
struct HistoryArgs {
    #[arg(long, help = "Limit entries shown (newest first)", default_value = "20")]
    limit: usize,
    #[arg(long, help = "Format output (text, json)", default_value = "text")]
    format: String,
}

#[derive(Args, Debug)]
struct DetectArgs {
    /// Output raw JSON instead of formatted table
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
pub struct BuildArgs {
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
pub struct RunArgs {
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
    #[arg(long, help = "Command to run for health checks (e.g. 'curl -f http://localhost:80/')")]
    health_cmd: Option<String>,
    #[arg(long, help = "Interval between health checks in seconds", default_value_t = 30)]
    health_interval: u64,
    #[arg(long, help = "Timeout in seconds to wait for health check", default_value_t = 30)]
    health_timeout: u64,
    #[arg(long, help = "Consecutive failures before marking unhealthy", default_value_t = 3)]
    health_retries: u32,
    #[arg(long, help = "Restart policy (no, always, on-failure[:max-retries], unless-stopped)", default_value = "no")]
    restart: String,
    #[arg(long, help = "Maximum number of PIDs in the container")]
    pids_limit: Option<u32>,
    #[arg(long, help = "Mount the container's root filesystem as read-only")]
    read_only: bool,
    #[arg(long, help = "Security options (e.g. apparmor=default, label=mcs)")]
    security_opt: Vec<String>,
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
struct ExecArgs {
    #[arg(help = "Container ID or name")]
    id: String,
    #[arg(help = "Command to run (defaults to a shell)", trailing_var_arg = true)]
    command: Vec<String>,
    #[arg(short = 'i', long, help = "Keep STDIN open (interactive)")]
    interactive: bool,
    #[arg(short = 't', long, help = "Allocate a TTY")]
    tty: bool,
    #[arg(short = 'w', long, help = "Working directory inside the container")]
    workdir: Option<String>,
    #[arg(short = 'e', long = "env", help = "Set environment variables (KEY=VALUE), repeatable")]
    env: Vec<String>,
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
    #[arg(long, name = "type", help = "Type of resource to inspect (container, image, network)", default_value = "container")]
    inspect_type: String,
    #[arg(long, help = "Format output (pretty, json)", default_value = "pretty")]
    format: String,
}

#[derive(Args, Debug)]
struct ServicesArgs {
    #[command(subcommand)]
    cmd: Option<ServicesSubcommand>,
    /// Format output (text, json). Kept here too so `crush services --format json`
    /// works without an explicit `ps` subcommand. Subcommand-level flag takes
    /// precedence when given.
    #[arg(long, default_value = "text", global = true)]
    format: String,
    #[arg(long, help = "List services for all projects, not just the cwd. Applies to ps/status.", global = true)]
    all_projects: bool,
}

#[derive(Subcommand, Debug)]
enum ServicesSubcommand {
    #[command(about = "Show running crush-managed services for this project")]
    Ps,
    #[command(about = "Show running crush-managed services for this project")]
    Status,
    #[command(about = "Tail logs for a running service")]
    Logs {
        name: String,
        #[arg(short, long, help = "Follow log output")]
        follow: bool,
    },
    #[command(about = "Stop one or all running services")]
    Stop {
        name: Option<String>,
    },
    #[command(about = "Restart a running service")]
    Restart {
        name: Option<String>,
    },
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
    #[arg(long, help = "Target platform (e.g. linux/amd64, linux/arm64)", default_value = None)]
    platform: Option<String>,
}

#[derive(Args, Debug)]
struct ImagesArgs {
    #[arg(long, help = "Show intermediate image layers")]
    all: bool,
    #[arg(long, help = "Format output (text, json)", default_value = "text")]
    format: String,
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
struct EjectArgs {
    #[arg(long, help = "Overwrite Dockerfile / docker-compose.yml if they exist")]
    force: bool,
    #[arg(long, default_value = ".", help = "Directory to write the generated files into")]
    out: String,
}

#[derive(Args, Debug)]
struct CatalogArgs {
    #[arg(help = "Filter by name, category, or description (e.g. 'search', 'flaresolverr')")]
    query: Option<String>,
}

#[derive(Args, Debug)]
struct TunnelArgs {
    #[arg(help = "Local port to expose (e.g. 8000). Defaults to the detected project port.")]
    port: Option<u16>,
    #[arg(long, help = "Force a provider: cloudflared (default), ngrok, or outray")]
    provider: Option<String>,
}

#[derive(Args, Debug)]
struct DbArgs {
    #[command(subcommand)]
    cmd: DbSubcommand,
}

#[derive(Args, Debug)]
struct LintArgs {
    #[arg(help = "Project directory to lint (default: current directory)")]
    path: Option<String>,
}

#[derive(Args, Debug)]
struct MailArgs {
    #[arg(long, help = "Port to listen on", default_value_t = crush_build::mailbox::DEFAULT_PORT)]
    port: u16,
}

#[derive(Args, Debug)]
struct SshArgs {
    #[arg(help = "Host alias from ~/.ssh/config to connect to (omit to list)")]
    host: Option<String>,
    #[arg(long, help = "List configured hosts even when a host is given")]
    list: bool,
    #[arg(help = "Command to run on the remote host", trailing_var_arg = true)]
    command: Vec<String>,
}

#[derive(Args, Debug, Clone)]
struct GatewayArgs {
    #[arg(long, help = "Public port the gateway listens on")]
    listen: Option<u16>,
    #[arg(long, help = "File holding the current upstream port (read per-connection)")]
    target_file: String,
    #[arg(long, help = "Atomically set the upstream port (flip the gateway) and exit")]
    set: Option<u16>,
}

#[derive(Parser, Debug, Clone)]
struct L7GatewayArgs {
    #[arg(long, help = "Path to the domains.json config file")]
    domains: String,
    #[arg(long, help = "Path to the TLS certs directory")]
    certs: String,
}

#[derive(Subcommand, Debug)]
enum DbSubcommand {
    #[command(about = "Save the current database state under a name")]
    Snapshot {
        #[arg(help = "Snapshot name (e.g. cart-populated)")]
        name: String,
        #[arg(long, help = "Database URL (defaults to DATABASE_URL / crush-managed service)")]
        url: Option<String>,
    },
    #[command(about = "Restore the database to a saved snapshot")]
    Restore {
        #[arg(help = "Snapshot name to restore")]
        name: String,
        #[arg(long, help = "Database URL (defaults to DATABASE_URL / crush-managed service)")]
        url: Option<String>,
        #[arg(long, short = 'y', help = "Skip the overwrite confirmation")]
        yes: bool,
    },
    #[command(about = "List saved snapshots for this project")]
    Ls,
    #[command(about = "Delete a saved snapshot")]
    Rm {
        #[arg(help = "Snapshot name to delete")]
        name: String,
    },
}

#[derive(Args, Debug)]
struct CiArgs {
    #[arg(help = "CI system: github, appveyor, or codemagic (default: auto from stack)")]
    system: Option<String>,
    #[arg(long, help = "Overwrite the config file if it already exists")]
    force: bool,
}

#[derive(Args, Debug)]
struct CleanArgs {
    #[arg(help = "Project directory to clean (default: current directory)")]
    path: Option<String>,
    #[arg(long, short = 'n', help = "Show what would be removed without deleting anything")]
    dry_run: bool,
    #[arg(long, short = 'y', help = "Skip the confirmation prompt")]
    yes: bool,
    #[arg(long, help = "Also clear crush's own global build-layer cache")]
    cache: bool,
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
    #[arg(short, long, help = "Output file path")]
    output: Option<String>,
}

#[derive(Args, Debug)]
struct MigrateArgs {
    #[arg(help = "Path to Dockerfile", default_value = "Dockerfile")]
    dockerfile: String,
    #[arg(long, help = "Apply migrations automatically")]
    apply: bool,
}

#[derive(Args, Debug)]
struct ComposeLogsArgs {
    #[arg(short, long, help = "Follow log stream in real time")]
    follow: bool,
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
    Logs(ComposeLogsArgs),
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
    #[command(about = "Display detailed volume information")]
    Inspect { name: String },
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

#[derive(Args, Debug)]
pub struct DeployArgs {
    #[arg(long, help = "Override the provider from Crushfile (hetzner, ssh, aws, gcp, digitalocean, fly, railway)")]
    provider: Option<String>,
    #[arg(long, help = "Show logs of the current deployment (no redeploy)")]
    logs: bool,
    #[arg(long, short = 'f', help = "With --logs: keep streaming new log output")]
    follow: bool,
    #[arg(long, default_value_t = 100, help = "With --logs: number of log lines to fetch")]
    lines: u32,
    #[arg(long, help = "Show current deployment status")]
    status: bool,
    #[arg(long, help = "Stop the deployment without destroying the server/state")]
    stop: bool,
    #[arg(long, help = "Destroy the deployment and remove the server")]
    destroy: bool,
    #[arg(long, default_value = "rolling", help = "Rollout strategy: rolling (default) or blue-green (zero-downtime, SSH/VPS)")]
    strategy: String,
    #[arg(long, help = "Blue-green health check path", default_value = "/")]
    health_path: String,
}

fn format_mem(bytes: u64) -> String {
    let kib = bytes as f64 / 1024.0;
    if kib < 1024.0 {
        format!("{:.1} KB", kib)
    } else {
        let mib = kib / 1024.0;
        if mib < 1024.0 {
            format!("{:.1} MB", mib)
        } else {
            let gib = mib / 1024.0;
            format!("{:.1} GB", gib)
        }
    }
}

#[cfg(target_os = "linux")]
fn read_proc_stat(pid: u32) -> Option<(u64, u64)> {
    let content = std::fs::read_to_string(format!("/proc/{}/stat", pid)).ok()?;
    let fields: Vec<&str> = content.split_whitespace().collect();
    let utime: u64 = fields.get(13)?.parse().ok()?;
    let stime: u64 = fields.get(14)?.parse().ok()?;
    Some((utime + stime, 100))
}

#[cfg(target_os = "linux")]
fn read_proc_mem_kb(pid: u32) -> Option<u64> {
    let content = std::fs::read_to_string(format!("/proc/{}/status", pid)).ok()?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            return rest.trim().split_whitespace().next()?.parse().ok();
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn get_cpu_and_mem(pid: u32) -> Option<(u64, u64)> {
    let stat = read_proc_stat(pid)?;
    let mem_kb = read_proc_mem_kb(pid)?;
    Some((stat.0, mem_kb * 1024))
}

#[cfg(target_os = "windows")]
fn get_cpu_and_mem(pid: u32) -> Option<(u64, u64)> {
    use windows_sys::Win32::System::Threading::{OpenProcess, GetProcessTimes, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME};

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle == 0 {
            return None;
        }
        let mut creation_time = std::mem::zeroed::<FILETIME>();
        let mut exit_time = std::mem::zeroed::<FILETIME>();
        let mut kernel_time = std::mem::zeroed::<FILETIME>();
        let mut user_time = std::mem::zeroed::<FILETIME>();
        let ok = GetProcessTimes(
            handle,
            &mut creation_time,
            &mut exit_time,
            &mut kernel_time,
            &mut user_time,
        );
        let mut counters = std::mem::zeroed::<PROCESS_MEMORY_COUNTERS>();
        counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        let mem_ok = GetProcessMemoryInfo(
            handle,
            &mut counters as *mut _ as *mut _,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        );
        CloseHandle(handle);
        
        let cpu = if ok != 0 {
            let kernel = ((kernel_time.dwHighDateTime as u64) << 32) | (kernel_time.dwLowDateTime as u64);
            let user = ((user_time.dwHighDateTime as u64) << 32) | (user_time.dwLowDateTime as u64);
            kernel + user
        } else {
            0
        };

        let mem = if mem_ok != 0 {
            counters.WorkingSetSize as u64
        } else {
            0
        };

        Some((cpu, mem))
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn get_cpu_and_mem(_pid: u32) -> Option<(u64, u64)> {
    None
}

async fn cmd_install() -> anyhow::Result<()> {
    let current_exe = std::env::current_exe()
        .map_err(|e| anyhow::anyhow!("Cannot determine current exe path: {}", e))?;

    #[cfg(target_os = "windows")]
    {
        install_windows(&current_exe)?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        install_unix(&current_exe)?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn install_windows(current_exe: &std::path::Path) -> anyhow::Result<()> {
    // Target: %LOCALAPPDATA%\crush\bin\crush.exe
    let local_app_data = std::env::var("LOCALAPPDATA")
        .unwrap_or_else(|_| format!("{}\\AppData\\Local", std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Default".to_string())));
    let install_dir = std::path::PathBuf::from(&local_app_data).join("crush").join("bin");
    let install_path = install_dir.join("crush.exe");

    std::fs::create_dir_all(&install_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create install dir {:?}: {}", install_dir, e))?;

    // Copy the executable (skip copy if already running from the install path)
    if current_exe != install_path {
        std::fs::copy(current_exe, &install_path)
            .map_err(|e| anyhow::anyhow!("Failed to copy crush.exe to {:?}: {}", install_path, e))?;
    }

    // Read current HKCU\Environment\Path
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let env_key = hkcu.open_subkey_with_flags("Environment", winreg::enums::KEY_READ | winreg::enums::KEY_WRITE)
        .map_err(|e| anyhow::anyhow!("Failed to open HKCU\\Environment: {}", e))?;

    let current_path: String = env_key.get_value("Path").unwrap_or_default();
    let install_dir_str = install_dir.to_string_lossy().to_string();

    // Only add if not already present
    let already_in_path = current_path
        .split(';')
        .any(|p| p.trim().eq_ignore_ascii_case(&install_dir_str));

    if !already_in_path {
        let new_path = if current_path.is_empty() {
            install_dir_str.clone()
        } else if current_path.ends_with(';') {
            format!("{}{}", current_path, install_dir_str)
        } else {
            format!("{};{}", current_path, install_dir_str)
        };
        env_key.set_value("Path", &new_path)
            .map_err(|e| anyhow::anyhow!("Failed to write PATH to registry: {}", e))?;

        // Broadcast WM_SETTINGCHANGE so Explorer and new terminals pick up the change
        // without requiring a logoff/reboot.
        broadcast_setting_change();
    }

    println!("crush installed to: {}", install_path.display());
    if already_in_path {
        println!("PATH already contains {}  (no change needed)", install_dir_str);
    } else {
        println!("Added {} to user PATH.", install_dir_str);
        println!("Open a new terminal and run: crush --version");
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn broadcast_setting_change() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SendMessageTimeoutW, HWND_BROADCAST, WM_SETTINGCHANGE, SMTO_ABORTIFHUNG};
    use windows_sys::Win32::Foundation::LPARAM;
    let env_wide: Vec<u16> = "Environment\0".encode_utf16().collect();
    unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            env_wide.as_ptr() as LPARAM,
            SMTO_ABORTIFHUNG,
            1000,
            std::ptr::null_mut(),
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn install_unix(current_exe: &std::path::Path) -> anyhow::Result<()> {
    // Prefer ~/.local/bin (no sudo needed, on PATH in modern distros)
    let home = std::env::var("HOME")
        .map_err(|_| anyhow::anyhow!("$HOME not set"))?;
    let install_dir = std::path::PathBuf::from(&home).join(".local").join("bin");
    let install_path = install_dir.join("crush");

    std::fs::create_dir_all(&install_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create {:?}: {}", install_dir, e))?;

    std::fs::copy(current_exe, &install_path)
        .map_err(|e| anyhow::anyhow!("Failed to copy to {:?}: {}", install_path, e))?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&install_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&install_path, perms)?;
    }

    println!("crush installed to: {}", install_path.display());
    // Check if install_dir is in PATH
    let path_env = std::env::var("PATH").unwrap_or_default();
    let in_path = path_env.split(':').any(|p| p == install_dir.to_string_lossy().as_ref());
    if !in_path {
        println!("NOTE: {} is not in your PATH.", install_dir.display());
        println!("Add this line to ~/.bashrc or ~/.zshrc:");
        println!("  export PATH=\"$HOME/.local/bin:$PATH\"");
    } else {
        println!("Open a new terminal and run: crush --version");
    }
    Ok(())
}

/// Interactive log filter mode toggled by single-letter keys after Ready.
#[derive(Clone)]
enum FilterMode { All, OnlyService(String), OnlyErrors, Paused }

/// Decide whether a child output line should be printed given the current
/// filter mode. stderr always counts as "errors" so crashes never disappear.
fn should_show(mode: &FilterMode, service: &str, line: &str, is_err: bool) -> bool {
    match mode {
        FilterMode::All => true,
        FilterMode::Paused => false,
        FilterMode::OnlyService(s) => s == service,
        FilterMode::OnlyErrors => is_err || looks_like_error(line),
    }
}

/// Heuristic: lines that look like errors even when written to stdout.
fn looks_like_error(line: &str) -> bool {
    let l = line.to_lowercase();
    l.contains("error") || l.contains("panic") || l.contains("fatal")
        || l.contains("exception") || l.contains("traceback")
        || l.contains("failed") || l.contains(" denied")
}

/// Probes well-known doc / health paths on a service's port. Returns the
/// list of `(label, url)` that responded with 2xx. Cheap and parallel.
async fn probe_service_links(port: u16) -> Vec<(&'static str, String)> {
    probe_service_links_for(port, "").await
}

/// Framework-aware probe. When `framework` matches a known backend
/// (Spring Boot, FastAPI, NestJS, Express, Fastify), we use a narrower
/// path list to avoid logging 404 noise in the app's stdout for paths
/// the framework would never expose.
async fn probe_service_links_for(port: u16, framework: &str) -> Vec<(&'static str, String)> {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(700))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let fw = framework.to_lowercase();
    // Full path list (used for unknown stacks). First hit per label wins.
    let all: &[(&str, &str)] = &[
        ("/swagger-ui/index.html", "docs"),
        ("/swagger-ui.html", "docs"),
        ("/swagger/index.html", "docs"),
        ("/swagger", "docs"),
        ("/docs", "docs"),
        ("/api-docs", "docs"),
        ("/v3/api-docs", "openapi"),
        ("/openapi.json", "openapi"),
        ("/actuator/health", "health"),
        ("/health", "health"),
        ("/healthz", "health"),
        ("/graphql", "graphql"),
    ];
    // Framework-narrowed lists — probe only what these frameworks actually
    // expose. Avoids 5-10 stack traces in the app log per crush run.
    // Note: /v3/api-docs intentionally omitted — springdoc lazy-inits on
    // first request (~4s), which exceeds our 700ms probe timeout and triggers
    // an AsyncRequestNotUsable stack trace in the app log. swagger-ui is the
    // human-facing URL anyway and back-links to /v3/api-docs internally.
    let spring: &[(&str, &str)] = &[
        ("/swagger-ui/index.html", "docs"),
        ("/actuator/health", "health"),
    ];
    let fastapi: &[(&str, &str)] = &[
        ("/docs", "docs"),
        ("/openapi.json", "openapi"),
        ("/health", "health"),
    ];
    let nest_express: &[(&str, &str)] = &[
        ("/api-docs", "docs"),
        ("/docs", "docs"),
        ("/openapi.json", "openapi"),
        ("/health", "health"),
        ("/healthz", "health"),
        ("/graphql", "graphql"),
    ];
    let probes: &[(&str, &str)] = if fw.contains("spring") {
        spring
    } else if fw.contains("fastapi") {
        fastapi
    } else if fw.contains("nestjs") || fw.contains("express") || fw.contains("fastify") {
        nest_express
    } else {
        all
    };
    // Fingerprint `/` first — SPAs (Vite, Next dev, CRA) return the same
    // index.html for every unmatched route, and we use this to discard false
    // positives where the response is just the SPA shell. Skipped for known
    // backend frameworks where `/` is just another 404 (noise in app logs).
    let is_known_backend = !probes.eq(all);
    let root_body: Option<String> = if is_known_backend {
        None
    } else {
        let root_url = format!("http://localhost:{}/", port);
        match client.get(&root_url).send().await {
            Ok(r) if r.status().is_success() => r.text().await.ok(),
            _ => None,
        }
    };
    let is_spa_shell = |body: &str| -> bool {
        if let Some(root) = root_body.as_ref() {
            // Exact match OR same shell with router-stamped title swap.
            body == root || (body.len() > 200 && body.len() == root.len())
        } else { false }
    };

    let mut hits: Vec<(&'static str, String)> = Vec::new();
    let mut seen_labels = std::collections::HashSet::new();
    for &(path, label) in probes {
        if seen_labels.contains(label) { continue; }
        let url = format!("http://localhost:{}{}", port, path);
        if let Ok(resp) = client.get(&url).send().await {
            if !resp.status().is_success() { continue; }
            let ct = resp.headers().get("content-type")
                .and_then(|h| h.to_str().ok()).unwrap_or("").to_string();
            let body = resp.text().await.unwrap_or_default();

            let plausible = if ct.contains("json") {
                // Real JSON endpoints rarely return the SPA shell.
                true
            } else if ct.contains("html") {
                if is_spa_shell(&body) { false }
                else {
                    // Look for actual content markers for the route type.
                    let lower = body.to_lowercase();
                    match label {
                        "docs" => lower.contains("swagger") || lower.contains("openapi") || lower.contains("redoc"),
                        "graphql" => lower.contains("graphql") || lower.contains("playground"),
                        "health" => lower.contains("status") || lower.contains("\"up\"") || lower.contains("ok"),
                        _ => false,
                    }
                }
            } else if ct.contains("text/plain") && label == "health" {
                let t = body.trim().to_lowercase();
                t == "ok" || t == "up" || t == "healthy"
            } else { false };

            if plausible {
                seen_labels.insert(label);
                let static_label: &'static str = match label {
                    "docs" => "docs",
                    "openapi" => "openapi",
                    "health" => "health",
                    "graphql" => "graphql",
                    _ => "other",
                };
                hits.push((static_label, url));
            }
        }
    }
    hits
}

/// Renders a small, idiomatic Dockerfile for the detected stack. Uses an
/// official base image, copies source, installs deps, and sets CMD.
fn generate_dockerfile(stack: &crush_build::InferredStack) -> String {
    let lang = stack.language.split(' ').next().unwrap_or("").to_lowercase();
    let port = stack.default_port;
    let base = if stack.base_image.is_empty() { "alpine:3.20" } else { &stack.base_image };
    let mut s = String::new();
    s.push_str("# crush:eject\n");
    s.push_str("# Auto-generated by `crush eject`. Review before deploying.\n");
    s.push_str("# Remove the `# crush:eject` line above to have `crush` treat this as a real prod Dockerfile.\n");
    s.push_str(&format!("# Detected: {} (port {})\n\n", stack.language, port));

    match lang.as_str() {
        "node" | "typescript" => {
            s.push_str("# syntax=docker/dockerfile:1\n");
            s.push_str(&format!("FROM {} AS deps\n", base));
            s.push_str("WORKDIR /app\n");
            s.push_str("COPY package.json package-lock.json* pnpm-lock.yaml* yarn.lock* bun.lockb* ./\n");
            s.push_str("RUN if [ -f pnpm-lock.yaml ]; then npm i -g pnpm && pnpm i --frozen-lockfile; \\\n");
            s.push_str("    elif [ -f yarn.lock ]; then yarn install --frozen-lockfile; \\\n");
            s.push_str("    elif [ -f bun.lockb ]; then npm i -g bun && bun install; \\\n");
            s.push_str("    else npm ci; fi\n\n");
            s.push_str(&format!("FROM {}\n", base));
            s.push_str("WORKDIR /app\n");
            s.push_str("COPY --from=deps /app/node_modules ./node_modules\n");
            s.push_str("COPY . .\n");
            s.push_str(&format!("ENV PORT={}\n", port));
            s.push_str(&format!("EXPOSE {}\n", port));
            s.push_str(&format!("CMD [\"{}\"]\n", stack.entry_point.replace('"', "\\\"")));
        }
        "python" => {
            s.push_str(&format!("FROM {}\n", base));
            s.push_str("WORKDIR /app\n");
            s.push_str("RUN pip install --no-cache-dir uv 2>/dev/null || true\n");
            s.push_str("COPY pyproject.toml uv.lock* requirements.txt* ./\n");
            s.push_str("RUN if [ -f uv.lock ]; then uv sync --no-dev --no-install-project; \\\n");
            s.push_str("    elif [ -f requirements.txt ]; then pip install --no-cache-dir -r requirements.txt; fi\n");
            s.push_str("COPY . .\n");
            s.push_str(&format!("ENV PORT={}\n", port));
            s.push_str(&format!("EXPOSE {}\n", port));
            s.push_str(&format!("CMD [\"sh\", \"-c\", \"{}\"]\n", stack.entry_point.replace('"', "\\\"")));
        }
        "go" => {
            s.push_str("FROM golang:1.24-alpine AS build\n");
            s.push_str("WORKDIR /src\n");
            s.push_str("COPY go.* ./\n");
            s.push_str("RUN go mod download\n");
            s.push_str("COPY . .\n");
            s.push_str("RUN CGO_ENABLED=0 go build -ldflags='-s -w' -o /out/app .\n\n");
            s.push_str("FROM alpine:3.20\n");
            s.push_str("RUN apk add --no-cache ca-certificates\n");
            s.push_str("COPY --from=build /out/app /app\n");
            s.push_str(&format!("ENV PORT={}\n", port));
            s.push_str(&format!("EXPOSE {}\n", port));
            s.push_str("CMD [\"/app\"]\n");
        }
        "rust" => {
            // Docker images are always Linux — strip .exe from Windows-detected paths.
            let bin_path = stack.entry_point.trim_end_matches(".exe");
            s.push_str("FROM rust:alpine AS build\n");
            s.push_str("WORKDIR /src\n");
            s.push_str("RUN apk add --no-cache musl-dev\n");
            s.push_str("COPY . .\n");
            s.push_str("RUN cargo build --release\n\n");
            s.push_str("FROM alpine:3.20\n");
            s.push_str("RUN apk add --no-cache ca-certificates\n");
            s.push_str(&format!("COPY --from=build /src/{} /app\n", bin_path));
            s.push_str(&format!("ENV PORT={}\nEXPOSE {}\nCMD [\"/app\"]\n", port, port));
        }
        "java" => {
            s.push_str("FROM eclipse-temurin:21-jdk AS build\n");
            s.push_str("WORKDIR /src\n");
            s.push_str("COPY . .\n");
            s.push_str("RUN ./mvnw -B package -DskipTests 2>/dev/null || mvn -B package -DskipTests\n\n");
            s.push_str("FROM eclipse-temurin:21-jre-alpine\n");
            s.push_str("COPY --from=build /src/target/*.jar /app.jar\n");
            s.push_str(&format!("ENV PORT={}\nEXPOSE {}\nENTRYPOINT [\"java\",\"-jar\",\"/app.jar\"]\n", port, port));
        }
        "ruby" => {
            s.push_str("FROM ruby:3.3-alpine\n");
            s.push_str("RUN apk add --no-cache build-base\n");
            s.push_str("WORKDIR /app\n");
            s.push_str("COPY Gemfile Gemfile.lock ./\n");
            s.push_str("RUN bundle config set without 'development test' && bundle install\n");
            s.push_str("COPY . .\n");
            s.push_str(&format!("ENV PORT={}\nEXPOSE {}\nCMD [\"sh\",\"-c\",\"{}\"]\n",
                port, port, stack.entry_point.replace('"', "\\\"")));
        }
        "php" => {
            s.push_str("FROM php:8.3-cli-alpine\n");
            s.push_str("COPY --from=composer:latest /usr/bin/composer /usr/bin/composer\n");
            s.push_str("WORKDIR /app\n");
            s.push_str("COPY composer.json composer.lock* ./\n");
            s.push_str("RUN composer install --no-dev --no-interaction --optimize-autoloader\n");
            s.push_str("COPY . .\n");
            s.push_str(&format!("ENV PORT={}\nEXPOSE {}\nCMD [\"sh\",\"-c\",\"{}\"]\n",
                port, port, stack.entry_point.replace('"', "\\\"")));
        }
        "dotnet" => {
            // entry_point = "dotnet out/AssemblyName.dll" — extract just the filename.
            let dll = stack.entry_point
                .split('/').last()
                .filter(|s| s.ends_with(".dll"))
                .unwrap_or("app.dll");
            s.push_str("FROM mcr.microsoft.com/dotnet/sdk:8.0 AS build\n");
            s.push_str("WORKDIR /src\n");
            s.push_str("COPY . .\n");
            s.push_str("RUN dotnet publish -c Release -o /out\n\n");
            s.push_str("FROM mcr.microsoft.com/dotnet/aspnet:8.0\n");
            s.push_str("WORKDIR /app\n");
            s.push_str("COPY --from=build /out .\n");
            s.push_str(&format!("ENV ASPNETCORE_URLS=http://+:{}\nEXPOSE {}\nENTRYPOINT [\"dotnet\",\"{}\"]\n", port, port, dll));
        }
        "elixir" => {
            s.push_str("FROM elixir:1.17-alpine AS build\n");
            s.push_str("WORKDIR /src\n");
            s.push_str("COPY mix.* ./\n");
            s.push_str("RUN mix local.hex --force && mix local.rebar --force && mix deps.get --only prod\n");
            s.push_str("COPY . .\n");
            s.push_str("ENV MIX_ENV=prod\n");
            s.push_str("RUN mix release\n\n");
            s.push_str("FROM alpine:3.20\n");
            s.push_str("RUN apk add --no-cache libstdc++ openssl ncurses\n");
            s.push_str("COPY --from=build /src/_build/prod/rel /app\n");
            s.push_str(&format!("ENV PORT={}\nEXPOSE {}\nCMD [\"/app/bin/server\",\"start\"]\n", port, port));
        }
        "deno" => {
            let entry = stack.entry_point.replace('"', "\\\"");
            s.push_str(&format!("FROM {} AS build\n", base));
            s.push_str("WORKDIR /app\n");
            s.push_str("COPY deno.json deno.jsonc* deno.lock* ./\n");
            s.push_str("COPY . .\n");
            // Cache dependencies at build time so the runtime image starts fast.
            s.push_str(&format!("RUN deno cache {}\n\n", entry));
            s.push_str(&format!("FROM {}\n", base));
            s.push_str("WORKDIR /app\n");
            s.push_str("COPY --from=build /app .\n");
            s.push_str(&format!("ENV PORT={}\n", port));
            s.push_str(&format!("EXPOSE {}\n", port));
            s.push_str(&format!(
                "CMD [\"deno\",\"run\",\"--allow-net\",\"--allow-env\",\"--allow-read\",\"{}\"]\n",
                entry
            ));
        }
        "bun" => {
            let entry = stack.entry_point.replace('"', "\\\"");
            s.push_str(&format!("FROM {} AS deps\n", base));
            s.push_str("WORKDIR /app\n");
            s.push_str("COPY package.json bun.lockb* ./\n");
            s.push_str("RUN bun install --frozen-lockfile\n\n");
            s.push_str(&format!("FROM {}\n", base));
            s.push_str("WORKDIR /app\n");
            s.push_str("COPY --from=deps /app/node_modules ./node_modules\n");
            s.push_str("COPY . .\n");
            s.push_str(&format!("ENV PORT={}\n", port));
            s.push_str(&format!("EXPOSE {}\n", port));
            s.push_str(&format!("CMD [\"bun\",\"run\",\"{}\"]\n", entry));
        }
        "swift" => {
            // entry_point = ".build/release/app" — binary name is the last segment.
            let bin_name = stack.entry_point.split('/').last().unwrap_or("app");
            s.push_str(&format!("FROM {} AS build\n", base));
            s.push_str("WORKDIR /src\n");
            s.push_str("COPY . .\n");
            s.push_str("RUN swift build -c release\n\n");
            // Swift runtime is large; copy only the binary into a slim Ubuntu image.
            s.push_str("FROM ubuntu:22.04\n");
            s.push_str("RUN apt-get update && apt-get install -y --no-install-recommends \\\n");
            s.push_str("    libicu-dev libcurl4 libxml2 && rm -rf /var/lib/apt/lists/*\n");
            s.push_str(&format!("COPY --from=build /src/.build/release/{} /app\n", bin_name));
            s.push_str(&format!("ENV PORT={}\nEXPOSE {}\nCMD [\"/app\"]\n", port, port));
        }
        _ => {
            s.push_str(&format!("FROM {}\n", base));
            s.push_str("WORKDIR /app\n");
            s.push_str("COPY . .\n");
            if !stack.build_command.is_empty() {
                s.push_str(&format!("RUN {}\n", stack.build_command));
            }
            s.push_str(&format!("ENV PORT={}\nEXPOSE {}\nCMD [\"sh\",\"-c\",\"{}\"]\n",
                port, port, stack.entry_point.replace('"', "\\\"")));
        }
    }
    s
}

/// Renders a minimal docker-compose.yml that pairs the app service with the
/// detected dep services (postgres, redis, etc.) using crush's inferred env.
fn generate_compose(stack: &crush_build::InferredStack, deps: &[crush_build::DepService]) -> String {
    let mut s = String::new();
    s.push_str("# crush:eject\n");
    s.push_str("# Auto-generated by `crush eject`. Review env vars + ports.\n");
    s.push_str("# Remove the `# crush:eject` line above to have `crush` treat this as a real prod compose file.\n");
    s.push_str("services:\n");
    // App service
    s.push_str("  app:\n");
    s.push_str("    build: .\n");
    s.push_str(&format!("    ports: [\"{}:{}\"]\n", stack.default_port, stack.default_port));
    if !deps.is_empty() {
        s.push_str("    depends_on:\n");
        for d in deps {
            s.push_str(&format!("      - {}\n", d.name));
        }
        s.push_str("    environment:\n");
        for d in deps {
            for (k, v) in crush_build::synthesize_dep_env(d) {
                // Replace "localhost" with the compose service name so the app
                // talks to the dep over the docker network, not its own loopback.
                let v = v.replace("localhost", &d.name);
                s.push_str(&format!("      {}: \"{}\"\n", k, v));
            }
        }
    }
    // Dep services
    for d in deps {
        s.push_str(&format!("  {}:\n", d.name));
        s.push_str(&format!("    image: {}\n", d.image));
        if !d.ports.is_empty() {
            s.push_str("    ports:\n");
            for (hp, cp) in &d.ports {
                s.push_str(&format!("      - \"{}:{}\"\n", hp, cp));
            }
        }
        if !d.env.is_empty() {
            s.push_str("    environment:\n");
            for (k, v) in &d.env {
                s.push_str(&format!("      {}: \"{}\"\n", k, v));
            }
        }
        if !d.volumes.is_empty() {
            s.push_str("    volumes:\n");
            for v in &d.volumes {
                s.push_str(&format!("      - {}\n", v));
            }
        }
    }
    s
}

/// Formats `[name]` prefix in a stable colour picked from a 6-slot palette.
/// First service is cyan, second magenta, then yellow/green/blue/red and wrap.
fn colour_prefix(name: &str, idx: usize) -> String {
    let label = format!("[{}]", name);
    match idx % 6 {
        0 => label.cyan().bold().to_string(),
        1 => label.magenta().bold().to_string(),
        2 => label.yellow().bold().to_string(),
        3 => label.green().bold().to_string(),
        4 => label.blue().bold().to_string(),
        _ => label.red().bold().to_string(),
    }
}

/// Walks `root` and returns the latest mtime among files matching `pred`.
/// Skips heavy/build directories (node_modules, .next, target, dist, .venv, etc).
fn latest_mtime<F: Fn(&std::path::Path) -> bool>(root: &std::path::Path, pred: F) -> Option<std::time::SystemTime> {
    fn is_skip(name: &str) -> bool {
        matches!(name,
            "node_modules" | ".next" | "target" | "dist" | "build" | ".turbo" |
            ".venv" | "venv" | "__pycache__" | ".git" | ".cache" | ".pnpm" |
            "vendor" | "deps" | "_build" | "out" | "bin" | "obj" | ".gradle" | ".mvn")
    }
    let extra_skips: Vec<String> = std::fs::read_to_string(root.join(".crushignore"))
        .ok()
        .map(|content| content.lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| l.trim_end_matches('/').to_string())
            .collect())
        .unwrap_or_default();
    let is_user_skip = |name: &str| extra_skips.iter().any(|s| s == name);
    let mut latest: Option<std::time::SystemTime> = None;
    let mut stack: Vec<std::path::PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) { Ok(e) => e, Err(_) => continue };
        for entry in entries.flatten() {
            let p = entry.path();
            let name = match p.file_name().and_then(|n| n.to_str()) { Some(n) => n.to_string(), None => continue };
            if name.starts_with('.') && name != ".env" { continue; }
            let ft = match entry.file_type() { Ok(t) => t, Err(_) => continue };
            if ft.is_dir() {
                if is_skip(&name) || is_user_skip(&name) { continue; }
                stack.push(p);
            } else if pred(&p) && !is_user_skip(&name) {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(m) = meta.modified() {
                        latest = Some(latest.map_or(m, |cur| cur.max(m)));
                    }
                }
            }
        }
    }
    latest
}

fn mtime(p: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).ok().and_then(|m| m.modified().ok())
}

/// Returns Some(reason) if the existing build artifact is newer than every
/// source/dep input we care about. Returns None when we should rebuild
/// (artifact missing, source newer, or stack unknown).
/// True if `node_modules` exists and is at least as new as the package
/// manifest + lockfile — i.e. running `pnpm install` would be a no-op.
fn node_deps_fresh(root: &std::path::Path) -> bool {
    let nm = root.join("node_modules");
    if !nm.exists() { return false; }
    let nm_mtime = match mtime(&nm) { Some(t) => t, None => return false };
    for lock in ["pnpm-lock.yaml", "package-lock.json", "yarn.lock", "bun.lockb", "package.json"] {
        if let Some(t) = mtime(&root.join(lock)) {
            if t > nm_mtime { return false; }
        }
    }
    true
}

fn build_freshness(root: &std::path::Path, language: &str) -> Option<String> {
    let lang = language.to_lowercase();

    if lang.starts_with("node") || lang.starts_with("javascript") || lang.starts_with("typescript") {
        let artifacts = [".next/BUILD_ID", "dist/index.js", "build/index.html", ".output/server/index.mjs", ".svelte-kit/output/server/index.js"];
        let mut artifact_mtime: Option<std::time::SystemTime> = None;
        let mut artifact_path: Option<String> = None;
        for a in artifacts {
            if let Some(m) = mtime(&root.join(a)) {
                if artifact_mtime.map_or(true, |cur| m > cur) {
                    artifact_mtime = Some(m);
                    artifact_path = Some(a.to_string());
                }
            }
        }
        let am = artifact_mtime?;
        let inputs_latest = latest_mtime(root, |p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            matches!(name, "package.json" | "package-lock.json" | "pnpm-lock.yaml" | "yarn.lock" | "bun.lockb" | "tsconfig.json")
                || p.extension().and_then(|e| e.to_str()).map_or(false, |e|
                    matches!(e, "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte" | "astro" | "css" | "scss" | "html"))
        })?;
        if am > inputs_latest {
            return Some(format!("{} newer than sources", artifact_path.unwrap_or_default()));
        }
        return None;
    }

    if lang.starts_with("go") {
        let candidates = ["bin", "."];
        let mut artifact_mtime: Option<std::time::SystemTime> = None;
        let mut artifact_path: Option<String> = None;
        for dir in candidates {
            if let Ok(entries) = std::fs::read_dir(root.join(dir)) {
                for e in entries.flatten() {
                    let p = e.path();
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let looks_bin = name.ends_with(".exe") || (!name.contains('.') && p.is_file());
                    if !looks_bin { continue; }
                    if let Ok(m) = e.metadata().and_then(|md| md.modified()) {
                        if artifact_mtime.map_or(true, |cur| m > cur) {
                            artifact_mtime = Some(m);
                            artifact_path = Some(format!("{}/{}", dir, name));
                        }
                    }
                }
            }
        }
        let am = artifact_mtime?;
        let inputs_latest = latest_mtime(root, |p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name == "go.mod" || name == "go.sum"
                || p.extension().and_then(|e| e.to_str()) == Some("go")
        })?;
        if am > inputs_latest {
            return Some(format!("{} newer than sources", artifact_path.unwrap_or_default()));
        }
        return None;
    }

    if lang.starts_with("java") {
        let target = root.join("target");
        if !target.is_dir() { return None; }
        let mut artifact_mtime: Option<std::time::SystemTime> = None;
        let mut artifact_path: Option<String> = None;
        if let Ok(entries) = std::fs::read_dir(&target) {
            for e in entries.flatten() {
                let p = e.path();
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !name.ends_with(".jar") { continue; }
                if name.ends_with("-sources.jar") || name.ends_with("-javadoc.jar") { continue; }
                if let Ok(m) = e.metadata().and_then(|md| md.modified()) {
                    if artifact_mtime.map_or(true, |cur| m > cur) {
                        artifact_mtime = Some(m);
                        artifact_path = Some(format!("target/{}", name));
                    }
                }
            }
        }
        let am = artifact_mtime?;
        let src_main = root.join("src").join("main");
        let inputs_latest = latest_mtime(&src_main, |p| {
            p.extension().and_then(|e| e.to_str()).map_or(false, |e|
                matches!(e, "java" | "kt" | "kts" | "scala" | "groovy" | "xml" | "yml" | "yaml" | "properties"))
        }).or_else(|| mtime(&root.join("pom.xml")))?;
        let pom_mtime = mtime(&root.join("pom.xml")).unwrap_or(inputs_latest);
        let cmp = inputs_latest.max(pom_mtime);
        if am > cmp {
            return Some(format!("{} newer than sources", artifact_path.unwrap_or_default()));
        }
        return None;
    }

    None
}

/// Shared sink for URLs discovered in child process output.
type UrlSink = std::sync::Arc<tokio::sync::Mutex<Vec<String>>>;

/// Strips ANSI escape sequences (CSI). Cheap and good enough for log scraping.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            i += 2;
            while i < bytes.len() && !(bytes[i] >= 0x40 && bytes[i] <= 0x7e) { i += 1; }
            if i < bytes.len() { i += 1; }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

/// Scans a line for `http(s)://(localhost|127.0.0.1|0.0.0.0|[::]):PORT...`
/// substrings. Pushes any new ones into the sink.
async fn record_urls(line: &str, sink: &UrlSink) {
    let clean = strip_ansi(line);
    let lower = clean.to_lowercase();
    let mut start = 0usize;
    while let Some(idx) = lower[start..].find("http") {
        let abs = start + idx;
        let rest = &clean[abs..];
        let scheme_len = if rest.starts_with("https://") { 8 }
                         else if rest.starts_with("http://") { 7 }
                         else { start = abs + 4; continue; };
        let after = &rest[scheme_len..];
        let host_ok = ["localhost", "127.0.0.1", "0.0.0.0", "[::]", "[::1]"]
            .iter().any(|h| after.starts_with(h));
        if !host_ok { start = abs + scheme_len; continue; }
        let end = rest.find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '`' || c == ',' || c == ';')
            .unwrap_or(rest.len());
        let mut url = rest[..end].to_string();
        while matches!(url.chars().last(), Some('.') | Some(')') | Some(']')) { url.pop(); }
        let mut s = sink.lock().await;
        if !s.iter().any(|u| u == &url) {
            s.push(url);
        }
        start = abs + end;
    }
}

/// Spawns a command line through the OS shell so PATH lookups resolve `.cmd`,
/// `.bat`, and `.ps1` shims (pnpm, npm, yarn, uvicorn are all .cmd on Windows
/// when installed via nvm/scoop/Volta). On Unix, executes directly via the
/// program parts to preserve argv handling.
/// Rewrites bash-style `$VAR` and `${VAR}` to cmd.exe-style `%VAR%`.
/// Only handles identifier vars (alpha/digit/underscore) — leaves shell
/// special vars like `$@` `$1` alone since cmd.exe has no equivalent.
fn translate_env_refs_for_cmd(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '$' { out.push(c); continue; }
        match chars.peek() {
            Some(&'{') => {
                chars.next(); // consume {
                let mut name = String::new();
                while let Some(&nc) = chars.peek() {
                    if nc == '}' { chars.next(); break; }
                    if nc.is_ascii_alphanumeric() || nc == '_' { name.push(nc); chars.next(); }
                    else { break; }
                }
                if !name.is_empty() {
                    out.push_str(&format!("%{}%", name));
                } else {
                    out.push('$'); out.push('{'); out.push_str(&name); out.push('}');
                }
            }
            Some(&nc) if nc.is_ascii_alphabetic() || nc == '_' => {
                let mut name = String::new();
                while let Some(&nc) = chars.peek() {
                    if nc.is_ascii_alphanumeric() || nc == '_' { name.push(nc); chars.next(); }
                    else { break; }
                }
                out.push_str(&format!("%{}%", name));
            }
            _ => out.push('$'),
        }
    }
    out
}

fn spawn_shell(cmdline: &str, cwd: &std::path::Path, env: &[(String, String)]) -> tokio::process::Command {
    // Use $JAVA_HOME/bin/java when set, so we match the JDK Maven used.
    // The bare `java` on PATH often points to an older JRE that can't
    // load Java 17/21 class files (UnsupportedClassVersionError).
    let cmdline = if cmdline.starts_with("java ") {
        if let Ok(jh) = std::env::var("JAVA_HOME") {
            let bin = if cfg!(target_os = "windows") {
                format!("{}\\bin\\java.exe", jh.trim_end_matches(['\\', '/']))
            } else {
                format!("{}/bin/java", jh.trim_end_matches('/'))
            };
            if std::path::Path::new(&bin).exists() {
                // Only wrap in quotes if the path has spaces — cmd.exe doubly
                // escapes the quotes Tokio adds, producing a literal `\"path\"`
                // that fails to resolve.
                let prefix = if bin.contains(' ') {
                    format!("\"{}\"", bin)
                } else {
                    bin
                };
                format!("{} {}", prefix, &cmdline[5..])
            } else {
                cmdline.to_string()
            }
        } else {
            cmdline.to_string()
        }
    } else {
        cmdline.to_string()
    };
    let cmdline = cmdline.as_str();

    // Expand `target/*.jar` ourselves — cmd.exe doesn't glob, and bash's
    // globbing only triggers if the file actually exists at parse time.
    // We pick the first jar that isn't `.original` (Spring Boot's repackage
    // leaves the pre-repackage jar as `<name>.jar.original`).
    let cmdline = if cmdline.contains("target/*.jar") {
        let target = cwd.join("target");
        let jar = std::fs::read_dir(&target).ok().and_then(|entries| {
            entries.flatten()
                .filter_map(|e| {
                    let p = e.path();
                    let name = p.file_name()?.to_str()?.to_string();
                    if name.ends_with(".jar") && !name.ends_with(".jar.original") {
                        Some(name)
                    } else { None }
                })
                .next()
        });
        if let Some(jar) = jar {
            cmdline.replace("target/*.jar", &format!("target/{}", jar))
        } else {
            cmdline.to_string()
        }
    } else {
        cmdline.to_string()
    };
    let cmdline = cmdline.as_str();
    let mut cmd = if cfg!(target_os = "windows") {
        // cmd.exe parses `./foo` as command `.` with arg `/foo`. Rewrite a
        // leading `./` to `.\` so the binary resolves correctly. Forward
        // slashes mid-path are fine; only the leading ./ is ambiguous.
        let fixed = if cmdline.starts_with("./") {
            format!(".\\{}", &cmdline[2..])
        } else {
            cmdline.to_string()
        };
        // Translate bash-style `$VAR` to cmd.exe-style `%VAR%`. Several
        // detected entries (uvicorn --port $PORT, etc.) use bash syntax;
        // without this cmd.exe passes them literally and the server binds
        // to the wrong port.
        let fixed = translate_env_refs_for_cmd(&fixed);
        let mut c = tokio::process::Command::new("cmd");
        c.arg("/c").arg(fixed);
        c
    } else {
        let parts: Vec<&str> = cmdline.split_whitespace().collect();
        let mut c = tokio::process::Command::new(parts[0]);
        c.args(&parts[1..]);
        c
    };
    cmd.current_dir(cwd);
    for (k, v) in env { cmd.env(k, v); }
    cmd
}

/// Run a single sub-service build command and stream its output with a
/// colour‑prefixed label. Returns Ok(()) on success.
async fn run_build(
    sub: &crush_build::SubService,
    cmdline: &str,
    cwd: &Path,
    dep_env: &[(String, String)],
) -> anyhow::Result<()> {
    let t0 = std::time::Instant::now();
    if cmdline.starts_with("install") || cmdline.contains("install") {
        println!("   {} {}: installing dependencies {}",
            "↳".cyan(), sub.name.bold(), format!("({})", cmdline).dimmed());
    } else {
        println!("   {} {}: building... {}",
            "⚙".yellow().bold(), sub.name.bold(), format!("({})", cmdline).dimmed());
    }

    let mut cmd = spawn_shell(cmdline, cwd, dep_env);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn()
        .map_err(|e| anyhow::anyhow!("spawn build for {} failed: {}", sub.name, e))?;

    job_object::assign(&child);

    if let Some(stdout) = child.stdout.take() {
        let n = sub.name.clone();
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let reader = tokio::io::BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                println!("{} {}", colour_prefix(&n, 0), line);
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let n = sub.name.clone();
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let reader = tokio::io::BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                eprintln!("{} {}", colour_prefix(&n, 0), line);
            }
        });
    }

    let status = child.wait().await
        .map_err(|e| anyhow::anyhow!("build for {} wait failed: {}", sub.name, e))?;
    if !status.success() {
        eprintln!("   {} {}: build failed",
            "✗".red().bold(), sub.name.bold());
        anyhow::bail!("Build failed for {}", sub.name);
    }

    println!("   {} {}: built in {:.1}s",
        "✓".green().bold(), sub.name.bold(), t0.elapsed().as_secs_f64());
    Ok(())
}

/// Parse a human-readable size string like "4G", "512M", "1024" into bytes.
fn parse_size(s: &str) -> std::result::Result<u64, String> {
    let s = s.trim();
    let (num_str, mult) = if let Some(rest) = s.strip_suffix(|c| c == 'k' || c == 'K') {
        (rest, 1024u64)
    } else if let Some(rest) = s.strip_suffix(|c| c == 'm' || c == 'M') {
        (rest, 1024u64 * 1024)
    } else if let Some(rest) = s.strip_suffix(|c| c == 'g' || c == 'G') {
        (rest, 1024u64 * 1024 * 1024)
    } else {
        (s, 1u64)
    };
    let num: u64 = num_str.parse().map_err(|_| format!("Invalid size '{}'", s))?;
    Ok(num * mult)
}

/// Parse a CPU fraction like "0.5", "2" into a float representing cores.
fn parse_cpu_fraction(s: &str) -> std::result::Result<f32, String> {
    let v: f32 = s.parse().map_err(|_| format!("Invalid CPU value '{}'", s))?;
    if v < 0.0 {
        return Err(format!("CPU value must be non-negative: {}", v));
    }
    Ok(v)
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB { format!("{:.1} GB", bytes as f64 / GB as f64) }
    else if bytes >= MB { format!("{:.1} MB", bytes as f64 / MB as f64) }
    else if bytes >= KB { format!("{:.1} KB", bytes as f64 / KB as f64) }
    else { format!("{} B", bytes) }
}

/// Scans a Dockerfile for EXPOSE and ENV PORT= to extract the port hint.
/// Returns (port, None) — entry_point is always from stack detection for native runs.
fn extract_dockerfile_hints(path: &std::path::Path) -> (Option<u16>, Option<String>) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return (None, None),
    };
    let mut port: Option<u16> = None;
    for line in content.lines() {
        let trimmed = line.trim();
        if port.is_none() {
            if let Some(rest) = trimmed.strip_prefix("EXPOSE ") {
                port = rest.split('/').next().and_then(|p| p.trim().parse().ok());
            } else if let Some(rest) = trimmed.strip_prefix("ENV PORT=").or_else(|| trimmed.strip_prefix("ENV PORT ")) {
                port = rest.split_whitespace().next().and_then(|p| p.parse().ok());
            }
        }
    }
    (port, None)
}

fn format_detection_line_raw(language: &str, _framework: &str, _confidence: f32) -> String {
    let (runtime_raw, framework) = if let Some(idx) = language.find(" (") {
        let rt = &language[..idx];
        let fw = language[idx + 2..].trim_end_matches(')');
        (rt, if fw == rt || fw == "generic" || fw.is_empty() { "" } else { fw })
    } else {
        (language, "")
    };

    let runtime_display = match runtime_raw {
        "node" | "typescript" => "Node.js",
        "python" => "Python",
        "rust" => "Rust",
        "go" => "Go",
        "java" => "Java",
        "dotnet" => ".NET",
        "ruby" => "Ruby",
        "php" => "PHP",
        "elixir" => "Elixir",
        "swift" => "Swift",
        "deno" => "Deno",
        "bun" => "Bun",
        other => other,
    };

    let lang_part = if runtime_raw == "typescript" { " · TypeScript" } else { "" };

    if framework.is_empty() {
        format!("{}{}", runtime_display, lang_part)
    } else {
        let fw_display = match framework {
            "next" => "Next.js",
            "nuxt" => "Nuxt",
            "express" => "Express",
            "fastapi" => "FastAPI",
            "flask" => "Flask",
            "django" => "Django",
            "rails" => "Rails",
            "sinatra" => "Sinatra",
            "laravel" => "Laravel",
            "symfony" => "Symfony",
            "gin" => "Gin",
            "echo" => "Echo",
            "fiber" => "Fiber",
            "actix" => "Actix-web",
            "rocket" => "Rocket",
            "axum" => "Axum",
            "phoenix" => "Phoenix",
            "remix" => "Remix",
            "astro" => "Astro",
            "svelte" | "sveltekit" => "SvelteKit",
            "vue" => "Vue",
            "angular" => "Angular",
            "react" => "React",
            other => other,
        };
        format!("{}{} · {}", runtime_display, lang_part, fw_display)
    }
}

fn format_detection_line(stack: &crush_build::InferredStack) -> String {
    let (runtime_raw, framework) = if let Some(idx) = stack.language.find(" (") {
        let rt = &stack.language[..idx];
        let fw = stack.language[idx + 2..].trim_end_matches(')');
        (rt, if fw == rt || fw == "generic" || fw.is_empty() { "" } else { fw })
    } else {
        (stack.language.as_str(), "")
    };

    let runtime_display = match runtime_raw {
        "node" | "typescript" => "Node.js",
        "python" => "Python",
        "rust" => "Rust",
        "go" => "Go",
        "java" => "Java",
        "dotnet" => ".NET",
        "ruby" => "Ruby",
        "php" => "PHP",
        "elixir" => "Elixir",
        "swift" => "Swift",
        "deno" => "Deno",
        "bun" => "Bun",
        other => other,
    };

    let v = stack.runtime_version.trim();
    let version_major = if v.is_empty() || v == "latest" || v == "lts" {
        String::new()
    } else {
        v.split('.').next().unwrap_or("").to_string()
    };

    let runtime_with_ver = if version_major.is_empty() {
        runtime_display.to_string()
    } else {
        format!("{} {}", runtime_display, version_major)
    };

    let lang_part = if runtime_raw == "typescript" { " · TypeScript" } else { "" };

    if framework.is_empty() {
        format!("{}{}", runtime_with_ver, lang_part)
    } else {
        let fw_display = match framework {
            "next" => "Next.js",
            "nuxt" => "Nuxt",
            "express" => "Express",
            "fastapi" => "FastAPI",
            "flask" => "Flask",
            "django" => "Django",
            "rails" => "Rails",
            "sinatra" => "Sinatra",
            "laravel" => "Laravel",
            "symfony" => "Symfony",
            "gin" => "Gin",
            "echo" => "Echo",
            "fiber" => "Fiber",
            "actix" => "Actix-web",
            "rocket" => "Rocket",
            "axum" => "Axum",
            "phoenix" => "Phoenix",
            "remix" => "Remix",
            "astro" => "Astro",
            "svelte" | "sveltekit" => "SvelteKit",
            "vue" => "Vue",
            "angular" => "Angular",
            "react" => "React",
            other => other,
        };
        format!("{}{} · {}", runtime_with_ver, lang_part, fw_display)
    }
}

/// `crush eject` — generate a Dockerfile + docker-compose.yml from the detected
/// stack. Pulled out of the main dispatch so it can run *before* the image store
/// (sled) is opened; it never needs the store, and running it lock-free lets the
/// GUI shell out to it while the GUI itself holds the store lock.
async fn run_eject(args: EjectArgs) -> anyhow::Result<()> {
    let project_root = std::env::current_dir()?;
    let out_dir = project_root.join(&args.out);
    std::fs::create_dir_all(&out_dir).ok();
    let detector = StackDetector::new();
    let stack = detector.detect(&project_root).await?;

    let dockerfile_path = out_dir.join("Dockerfile");
    let compose_path = out_dir.join("docker-compose.yml");

    if dockerfile_path.exists() && !args.force {
        anyhow::bail!("Dockerfile already exists at {}. Pass --force to overwrite.",
            dockerfile_path.display());
    }
    if compose_path.exists() && !args.force {
        anyhow::bail!("docker-compose.yml already exists at {}. Pass --force to overwrite.",
            compose_path.display());
    }

    // Parse compose deps if user already declared them, plus spring config fallback.
    let compose_files = ["docker-compose.yml", "docker-compose.yaml", "compose.yml", "compose.yaml"];
    let existing_compose = compose_files.iter().map(|f| project_root.join(f))
        .find(|p| p.exists() && *p != compose_path);
    let mut dep_services: Vec<crush_build::DepService> = Vec::new();
    if let Some(ref cp) = existing_compose {
        if let Ok(parsed) = parse_compose(cp) {
            dep_services = parsed.dep_services;
        }
    }
    if dep_services.is_empty() {
        dep_services = parse_spring_config(&project_root);
    }

    let dockerfile = generate_dockerfile(&stack);
    let compose = generate_compose(&stack, &dep_services);

    std::fs::write(&dockerfile_path, &dockerfile)?;
    std::fs::write(&compose_path, &compose)?;

    println!(" {} wrote {}", "✓".green().bold(), dockerfile_path.display().to_string().bold());
    println!(" {} wrote {}", "✓".green().bold(), compose_path.display().to_string().bold());

    // Cross-OS pre-flight: catch Windows→Linux landmines (case-sensitive import
    // paths) before they break the container build. Eject is exactly the moment
    // a project crosses from NTFS to ext4, so lint here.
    print_cross_os_lint(&crush_build::lint::lint_cross_os(&project_root));

    println!("   {} review and edit, then deploy: {}",
        "↳".cyan(),
        "docker compose up --build".dimmed());
    Ok(())
}

/// Print cross-OS lint findings. Returns true when the project is clean.
fn print_cross_os_lint(findings: &[crush_build::lint::LintFinding]) -> bool {
    if findings.is_empty() {
        println!(" {} cross-OS lint: no case-sensitivity issues found", "✓".green().bold());
        return true;
    }
    println!("\n {} cross-OS lint found {} issue{} that will break the Linux build:",
        "⚠".yellow().bold(),
        findings.len(),
        if findings.len() == 1 { "" } else { "s" });
    for f in findings {
        if f.line > 0 {
            println!("   {} {}:{}  {}", "•".yellow(), f.file.bold(), f.line, f.message);
            println!("       {} change {} → {}", "↳".cyan(), f.specifier.red(), f.suggestion.green());
        } else {
            println!("   {} {}  {}", "•".yellow(), f.file.bold(), f.message);
        }
    }
    println!("   {} these resolve on Windows (NTFS) but fail in the container (ext4).", "↳".dimmed());
    false
}

/// `crush lint` — standalone cross-OS pre-flight. Exits non-zero on findings so
/// it's usable as a CI gate.
fn run_lint(args: LintArgs) -> anyhow::Result<()> {
    let root = match &args.path {
        Some(p) => std::path::PathBuf::from(p),
        None => std::env::current_dir()?,
    };
    if !root.is_dir() {
        anyhow::bail!("not a directory: {}", root.display());
    }
    let clean = print_cross_os_lint(&crush_build::lint::lint_cross_os(&root));
    if !clean {
        std::process::exit(1);
    }
    Ok(())
}

/// `crush mail` — run the local SMTP sink and print captured mail as it arrives.
/// Point an app at SMTP_HOST=localhost / SMTP_PORT=<port> and its outgoing email
/// lands here instead of being delivered.
async fn run_mail_cmd(args: MailArgs) -> anyhow::Result<()> {
    use crush_build::mailbox::{self, MailStore};
    let store = MailStore::new();
    println!("{} mail catcher listening on {}", "→".cyan(), format!("localhost:{}", args.port).bold());
    println!("   {} set {} and {} in your app to capture its email here",
        "↳".cyan(),
        format!("SMTP_HOST=localhost").bold(),
        format!("SMTP_PORT={}", args.port).bold());
    println!("   {} press {} to stop\n", "↳".cyan(), "Ctrl-C".bold());

    let on_new = |m: &mailbox::CapturedMail| {
        println!("{} {}  {}",
            "✉".green().bold(),
            m.subject.bold(),
            format!("from {} → {}", m.from, m.to.join(", ")).dimmed());
        let preview: String = m.body.lines().take(3).collect::<Vec<_>>().join("\n");
        if !preview.trim().is_empty() {
            for line in preview.lines() {
                println!("   {} {}", "│".dimmed(), line.dimmed());
            }
        }
    };

    tokio::select! {
        r = mailbox::serve(args.port, store, on_new) => {
            r.map_err(|e| anyhow::anyhow!("mail catcher failed to bind :{} — {e} (is one already running?)", args.port))?;
        }
        _ = tokio::signal::ctrl_c() => {
            println!("\n{} mail catcher stopped", "↳".cyan());
        }
    }
    Ok(())
}

/// `crush gateway` — the blue-green traffic director. With `--set` it atomically
/// flips the upstream and exits; otherwise it runs the forwarder on `--listen`.
async fn run_gateway_cmd(args: GatewayArgs) -> anyhow::Result<()> {
    use crush_build::gateway;
    let target_file = std::path::PathBuf::from(&args.target_file);

    if let Some(port) = args.set {
        gateway::write_target(&target_file, port)
            .map_err(|e| anyhow::anyhow!("failed to write target file {}: {e}", args.target_file))?;
        println!("{} gateway upstream → :{}", "✓".green().bold(), port);
        return Ok(());
    }

    let listen = args.listen.ok_or_else(|| anyhow::anyhow!("--listen <port> is required to run the gateway"))?;
    let current = gateway::read_target(&target_file)
        .map(|p| p.to_string())
        .unwrap_or_else(|| "(none yet)".into());
    println!("{} gateway listening on :{} → upstream :{} {}",
        "→".cyan(), listen, current, format!("(target: {})", args.target_file).dimmed());

    tokio::select! {
        r = gateway::run_gateway(listen, target_file) => {
            r.map_err(|e| anyhow::anyhow!("gateway failed to bind :{} — {e}", listen))?;
        }
        _ = tokio::signal::ctrl_c() => {
            println!("\n{} gateway stopped", "↳".cyan());
        }
    }
    Ok(())
}

/// `crush l7-gateway`
async fn run_l7_gateway_cmd(args: L7GatewayArgs) -> anyhow::Result<()> {
    use crush_build::gateway;
    let domains_file = std::path::PathBuf::from(&args.domains);
    let certs_dir = std::path::PathBuf::from(&args.certs);
    
    std::fs::create_dir_all(&certs_dir)?;

    tokio::select! {
        r = gateway::run_l7_gateway(domains_file, certs_dir) => {
            r.map_err(|e| anyhow::anyhow!("l7-gateway failed — {e}"))?;
        }
        _ = tokio::signal::ctrl_c() => {
            println!("\n{} l7-gateway stopped", "↳".cyan());
        }
    }
    Ok(())
}

/// `crush clean` — reclaim disk by removing dependency installs, build outputs,
/// and tool caches from a project (node_modules, target, .venv, dist, .gradle,
/// .dart_tool, Pods, …). Pulled out of the main dispatch so it runs before the
/// image store opens; it only touches the project tree (and optionally crush's
/// own cache dir), never the store.
async fn run_clean(args: CleanArgs) -> anyhow::Result<()> {
    // Directory basenames safe to delete: dependency installs, build outputs,
    // and framework/tool caches. Matched anywhere in the tree; once matched we
    // don't descend (the children go with the parent).
    const CLEAN_DIRS: &[&str] = &[
        // JS / TS
        "node_modules", ".next", ".nuxt", ".svelte-kit", ".turbo", ".parcel-cache",
        ".angular", ".vite", ".astro",
        // build outputs
        "dist", "build", "out", ".output",
        // Rust
        "target",
        // Python
        ".venv", "venv", "__pycache__", ".pytest_cache", ".mypy_cache", ".ruff_cache",
        ".tox", ".nox",
        // JVM / Gradle
        ".gradle",
        // Flutter / Dart
        ".dart_tool",
        // iOS / React Native
        "Pods",
        // generic tool cache
        ".cache",
    ];
    // Never descend into or remove these.
    const NEVER: &[&str] = &[".git", ".hg", ".svn"];

    let root = match &args.path {
        Some(p) => std::path::PathBuf::from(p),
        None => std::env::current_dir()?,
    };
    let root = root.canonicalize()
        .map_err(|e| anyhow::anyhow!("cannot access {}: {}", root.display(), e))?;
    if !root.is_dir() {
        anyhow::bail!("not a directory: {}", root.display());
    }

    fn dir_size(path: &std::path::Path) -> u64 {
        let mut total = 0u64;
        let mut stack = vec![path.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            for entry in rd.flatten() {
                let Ok(ft) = entry.file_type() else { continue };
                if ft.is_symlink() { continue; }
                if ft.is_dir() { stack.push(entry.path()); }
                else if let Ok(m) = entry.metadata() { total += m.len(); }
            }
        }
        total
    }

    // Walk the tree, collecting cleanable dirs without descending into matches.
    let mut found: Vec<(std::path::PathBuf, u64)> = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for entry in rd.flatten() {
            let Ok(ft) = entry.file_type() else { continue };
            if !ft.is_dir() || ft.is_symlink() { continue; }
            let name = entry.file_name().to_string_lossy().to_string();
            if NEVER.contains(&name.as_str()) { continue; }
            if CLEAN_DIRS.contains(&name.as_str()) {
                let p = entry.path();
                let size = dir_size(&p);
                found.push((p, size));
            } else {
                stack.push(entry.path());
            }
        }
    }

    // Optionally include crush's own global build-layer cache.
    let mut cache_target: Option<(std::path::PathBuf, u64)> = None;
    if args.cache {
        let cache = dirs_or_default().join("cache");
        if cache.is_dir() {
            let size = dir_size(&cache);
            cache_target = Some((cache, size));
        }
    }

    if found.is_empty() && cache_target.is_none() {
        println!(" {} nothing to clean in {}", "✓".green().bold(), root.display());
        return Ok(());
    }

    found.sort_by(|a, b| b.1.cmp(&a.1));
    let total: u64 = found.iter().map(|(_, s)| *s).sum::<u64>()
        + cache_target.as_ref().map(|(_, s)| *s).unwrap_or(0);

    if args.dry_run { println!("{}", "would remove:".yellow().bold()); }
    else { println!("{}", "will remove:".bold()); }
    for (p, s) in &found {
        let rel = p.strip_prefix(&root).unwrap_or(p);
        println!("   {:>9}  {}", format_bytes(*s).dimmed(), rel.display());
    }
    if let Some((p, s)) = &cache_target {
        println!("   {:>9}  {} {}", format_bytes(*s).dimmed(), p.display(), "(crush cache)".dimmed());
    }
    println!("   {:>9}  {}", format_bytes(total).cyan().bold(), "total".bold());

    if args.dry_run {
        println!(" {} dry run — nothing deleted", "↳".cyan());
        return Ok(());
    }

    if !args.yes {
        use std::io::Write;
        print!("\ndelete these? [y/N] ");
        std::io::stdout().flush().ok();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).ok();
        if !matches!(line.trim().to_lowercase().as_str(), "y" | "yes") {
            println!(" {} aborted", "✗".red());
            return Ok(());
        }
    }

    let mut reclaimed = 0u64;
    let mut failures = 0u32;
    for (p, s) in found.iter().chain(cache_target.iter()) {
        match std::fs::remove_dir_all(p) {
            Ok(()) => reclaimed += *s,
            Err(e) => { eprintln!("   {} {}: {}", "!".red().bold(), p.display(), e); failures += 1; }
        }
    }
    println!(" {} reclaimed {}", "✓".green().bold(), format_bytes(reclaimed).bold());
    if failures > 0 {
        anyhow::bail!("{} path(s) could not be removed (in use? permissions?)", failures);
    }
    Ok(())
}

/// `crush catalog` — print the curated image catalog (optionally filtered),
/// grouped by category, with the exact `crush pull` reference for each.
fn run_catalog(args: CatalogArgs) -> anyhow::Result<()> {
    let query = args.query.unwrap_or_default();
    let hits = crush_types::catalog::search(&query);
    if hits.is_empty() {
        println!("No catalog entries match {:?}", query);
        println!("   {} run {} to see everything", "↳".cyan(), "crush catalog".bold());
        return Ok(());
    }

    // Stable category order; entries already grouped logically in the source.
    let order = ["database", "cache", "search", "storage", "messaging", "proxy", "observability", "tool"];
    let mut shown = std::collections::HashSet::new();
    for cat in order {
        let in_cat: Vec<_> = hits.iter().filter(|e| e.category == cat).collect();
        if in_cat.is_empty() { continue; }
        println!("\n{}", cat.to_uppercase().cyan().bold());
        for e in in_cat {
            shown.insert(e.reference);
            let tag = if e.native { " (native)".green().to_string() } else { String::new() };
            println!("  {:<22} {}{}", e.name.bold(), e.reference.dimmed(), tag);
            println!("  {:<22} {}", "", e.description);
        }
    }
    // Any categories not in the predefined order.
    let leftover: Vec<_> = hits.iter().filter(|e| !shown.contains(e.reference)).collect();
    if !leftover.is_empty() {
        println!("\n{}", "OTHER".cyan().bold());
        for e in leftover {
            println!("  {:<22} {}", e.name.bold(), e.reference.dimmed());
        }
    }

    println!("\n{} pull one with: {}", "↳".cyan(), "crush pull <reference>".bold());
    println!("{} entries tagged {} also run as managed services: {}",
        "↳".cyan(), "(native)".green(), "crush services".bold());
    Ok(())
}

async fn run_tunnel_cmd(args: TunnelArgs) -> anyhow::Result<()> {
    use crush_build::tunnel::{self, TunnelEvent};

    // Resolve the port: explicit flag wins, else fall back to the detected
    // project port so `crush tunnel` "just works" inside a project dir.
    let root = std::env::current_dir()?;
    let detection = crush_build::CrushSpecDetector::new().detect(&root);
    let port = match args.port {
        Some(p) => p,
        None => {
            let p = detection.port;
            if p == 0 {
                anyhow::bail!(
                    "couldn't infer a port for this project — pass one explicitly: {}",
                    "crush tunnel <port>".bold()
                );
            }
            println!("{} no port given — using detected port {}", "↳".cyan(), p.to_string().bold());
            p
        }
    };

    // If the project talks to webhook senders, say which ones this tunnel serves.
    let webhook = crush_build::env::tunnel_providers(&detection.external_services);
    if !webhook.is_empty() {
        let names = webhook.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(", ");
        println!("{} webhook providers detected: {}", "↳".cyan(), names.bold());
    }

    println!("{} opening tunnel to localhost:{}\u{2026}", "→".cyan(), port);

    let (tx, mut rx) = tokio::sync::mpsc::channel::<TunnelEvent>(256);
    // Drain log/exit events in the background; `open` returns once the URL is up.
    let printer = tokio::spawn(async move {
        while let Some(ev) = rx.recv().await {
            match ev {
                TunnelEvent::Log { line } => eprintln!("  {}", line.dimmed()),
                TunnelEvent::Exited { code } => {
                    eprintln!("{} tunnel process exited ({code})", "✗".red());
                }
                TunnelEvent::Ready { .. } => {} // handled on the return value
            }
        }
    });

    let mut handle = match tunnel::open(port, args.provider.as_deref(), tx).await {
        Ok(h) => h,
        Err(e) => {
            printer.abort();
            anyhow::bail!(
                "{e}\n   {} cloudflared is downloaded automatically; ngrok/outray need to be installed \
                 and a token set (NGROK_AUTHTOKEN / OUTRAY_TOKEN)",
                "↳".cyan()
            );
        }
    };

    println!();
    println!("  {}  {}", "Public URL:".dimmed(), handle.url().green().bold());
    println!("  {}  {}", "Provider:  ".dimmed(), handle.provider().as_str());
    println!("  {}  http://localhost:{}", "Forwards → ".dimmed(), port);
    if !webhook.is_empty() {
        println!(
            "\n  {} point your provider's webhook/callback URL at the public URL above.",
            "tip:".yellow()
        );
    }
    println!("\n{} tunnel is live — press {} to stop", "✓".green(), "Ctrl-C".bold());

    // Stay up until the user interrupts or the tunnel process dies on its own.
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("\n{} closing tunnel\u{2026}", "↳".cyan());
            handle.shutdown().await;
        }
        code = handle.wait() => {
            eprintln!("{} tunnel exited unexpectedly ({code})", "✗".red());
        }
    }
    printer.abort();
    Ok(())
}

async fn run_internal(args: InternalRunArgs, data_dir: std::path::PathBuf) -> anyhow::Result<()> {
            let container_dir = data_dir.join("containers").join(&args.id);
            let container_json_path = container_dir.join("container.json");
            let config_json_path = container_dir.join("config.json");
            
            if !container_json_path.exists() {
                eprintln!("Container {} not found.", args.id);
                std::process::exit(1);
            }

            let content = std::fs::read_to_string(&container_json_path)?;
            let c: Container = serde_json::from_str(&content)?;

            let config_content = std::fs::read_to_string(&config_json_path)?;
            #[derive(serde::Deserialize)]
            struct ContainerConfig {
                cmd: Vec<String>,
                env: Vec<String>,
            }
            let config: ContainerConfig = serde_json::from_str(&config_content)?;

            let rootfs = container_dir.join("rootfs");
            let mounter = VolumeMounter::new(data_dir.clone());

            for mount in &c.mounts {
                let host_path_str = mount.host_path.to_string_lossy();
                let container_path_str = mount.container_path.to_string_lossy();
                if let Err(e) = mounter.mount_bind(&c.id, &host_path_str, &container_path_str, &rootfs, mount.read_only).await {
                    eprintln!("Error mounting {}: {}", host_path_str, e);
                    let _ = mounter.unmount_all(&c.id).await;
                    std::process::exit(1);
                }
            }

            #[cfg(target_os = "linux")]
            {
                use std::time::Duration;

                // 1. Resolve secrets (Vault / AWS / local DB)
                let secrets_dir = data_dir.join("secrets").join(&c.id);
                std::fs::create_dir_all(&secrets_dir).ok();
                let secret_mgr = SecretManager::new(secrets_dir.clone());
                let secret_mgr = if let (Ok(addr), Ok(tok)) = (std::env::var("VAULT_ADDR"), std::env::var("VAULT_TOKEN")) {
                    secret_mgr.with_vault(addr, tok)
                } else {
                    secret_mgr
                };

                if std::env::var("VAULT_ADDR").is_ok() && std::env::var("VAULT_TOKEN").is_ok() {
                    let spec = SecretSpec {
                        id: "db-password".to_string(),
                        source: SecretSource::Vault {
                            path: "secret/data/db-password".to_string(),
                            field: "value".to_string(),
                            engine: VaultEngine::KvV2,
                        },
                        destination: SecretDestination::File {
                            path: rootfs.join("run/secrets/db-password"),
                            tmpfs: true,
                        },
                        mode: 0o400,
                        uid: 0,
                        gid: 0,
                    };
                    if let Ok(val) = secret_mgr.resolve(&spec).await {
                        let _ = secret_mgr.mount(&spec, &val).await;
                    }
                }

                // 2. Restart policy initialization
                let r_policy = match c.restart_policy.as_deref().unwrap_or("no") {
                    "always" => RestartPolicy::Always,
                    "unless-stopped" => RestartPolicy::UnlessStopped,
                    s if s.starts_with("on-failure") => {
                        let max_retries = s.strip_prefix("on-failure:")
                            .and_then(|r| r.parse::<u32>().ok());
                        RestartPolicy::OnFailure { max_retries }
                    }
                    _ => RestartPolicy::No,
                };
                let mut restart_mgr = RestartManager::new(r_policy);

                // 3. Health check task initialization
                let mut health_handle = None;
                if let Some(ref h_cmd) = c.health_cmd {
                    let interval = c.health_interval.unwrap_or(30);
                    let timeout = c.health_timeout.unwrap_or(30);
                    let retries = c.health_retries.unwrap_or(3);
                    let cmd_parts: Vec<String> = h_cmd.split_whitespace().map(|s| s.to_string()).collect();
                    let h_config = HealthCheckConfig {
                        check_type: HealthCheckType::Exec { command: cmd_parts },
                        interval_secs: interval,
                        timeout_secs: timeout,
                        retries,
                        start_period_secs: 0,
                        start_interval_secs: 5,
                    };
                    let checker = Arc::new(HealthChecker::new(h_config));
                    let checker_clone = checker.clone();
                    let container_json_clone = container_json_path.clone();
                    
                    health_handle = Some(tokio::spawn(async move {
                        loop {
                            let status = checker_clone.check().await;
                            if let Ok(content) = tokio::fs::read_to_string(&container_json_clone).await {
                                if let Ok(mut c_upd) = serde_json::from_str::<Container>(&content) {
                                    c_upd.health = Some(status);
                                    if let Ok(serialized) = serde_json::to_string_pretty(&c_upd) {
                                        let _ = tokio::fs::write(&container_json_clone, serialized).await;
                                    }
                                }
                            }
                            tokio::time::sleep(Duration::from_secs(interval)).await;
                        }
                    }));
                }

                // 4. OOM Monitor initialization
                let mut oom_monitor = OomMonitor::new(&c.id, OomPolicy::Restart);

                // 5. Supervisor Loop
                let mut exit_code = 0;
                loop {
                    let rootfs_clone = rootfs.clone();
                    let c_clone = c.clone();
                    let cmd_clone = config.cmd.clone();
                    let env_clone = config.env.clone();

                    let exit_code_res = tokio::task::spawn_blocking(move || {
                        run_container(&rootfs_clone, &cmd_clone, &env_clone, &c_clone)
                    }).await;

                    let current_exit = match exit_code_res {
                        Ok(Ok(code)) => code,
                        _ => -1,
                    };

                    if let Ok(OomEvent::OomKilled { .. }) = oom_monitor.poll().await {
                        println!("[Supervisor] Container OOM killed!");
                        exit_code = 137;
                    } else {
                        exit_code = current_exit;
                    }

                    let should_restart = restart_mgr.should_restart(exit_code, false);
                    if should_restart {
                        restart_mgr.record_attempt();
                        let delay = restart_mgr.backoff_delay();
                        println!("[Supervisor] Restarting container in {:?}", delay);
                        
                        if let Ok(content) = tokio::fs::read_to_string(&container_json_path).await {
                            if let Ok(mut c_upd) = serde_json::from_str::<Container>(&content) {
                                c_upd.restart_count = Some(restart_mgr.attempt());
                                c_upd.status = ContainerStatus::Running;
                                if let Ok(serialized) = serde_json::to_string_pretty(&c_upd) {
                                    let _ = tokio::fs::write(&container_json_path, serialized).await;
                                }
                            }
                        }
                        tokio::time::sleep(delay).await;
                    } else {
                        break;
                    }
                }

                if let Some(h) = health_handle {
                    h.abort();
                }

                let _ = mounter.unmount_all(&c.id).await;

                if let Ok(content) = tokio::fs::read_to_string(&container_json_path).await {
                    if let Ok(mut c_upd) = serde_json::from_str::<Container>(&content) {
                        c_upd.status = ContainerStatus::Stopped;
                        c_upd.pid = None;
                        if let Ok(serialized) = serde_json::to_string_pretty(&c_upd) {
                            let _ = tokio::fs::write(&container_json_path, serialized).await;
                        }
                    }
                }

                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }
            #[cfg(target_os = "windows")]
            {
                // Read container config
                let rootfs = container_dir.join("rootfs");
                let store = ImageStore::new(data_dir.join("images")).await?;

                // Detect image OS from image tag stored in container.json
                let image_os = store.database().get_image_by_tag(&c.image).await
                    .ok()
                    .flatten()
                    .map(|img| img.os.to_lowercase())
                    .unwrap_or_else(|| "linux".to_string());

                let win_runtime = WindowsRuntime::new();

                if image_os == "windows" {
                    // Windows-native container: Job Objects path
                    win_runtime.create(&c, &container_dir).await
                        .map_err(|e| CrushError::Internal(anyhow::anyhow!("Windows create failed: {}", e)))?;
                    win_runtime.start_with_config(&c.id, &config.cmd, &config.env, &rootfs).await
                        .map_err(|e| CrushError::Internal(anyhow::anyhow!("Windows start failed: {}", e)))?;
                } else {
                    // Linux container: Firecracker microVM path
                    let image_digest = store.database()
                        .get_image_by_tag(&c.image).await
                        .ok().flatten()
                        .map(|img| img.digest.clone())
                        .unwrap_or_else(|| c.image.clone());

                    win_runtime.run_linux_container(
                        &c.id, &rootfs, &config.cmd, &config.env, &c.ports,
                        &image_digest,
                    ).await
                        .map_err(|e| CrushError::Internal(anyhow::anyhow!("Firecracker start failed: {}", e)))?;
                }

                let _ = mounter.unmount_all(&c.id).await;
            }

            #[cfg(all(not(target_os = "linux"), not(target_os = "windows")))]
            {
                eprintln!("Container execution requires Linux or Windows.");
                let _ = mounter.unmount_all(&c.id).await;
            }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    // Set up the Windows Job Object on startup. Every child we spawn after
    // this gets assigned to the job, and Windows kills the entire job tree
    // when crush exits — Ctrl+C, panic, terminal closed, all of it. Closest
    // we can get to docker-on-Linux's clean teardown story.
    let priority_class = cli.priority.as_deref().and_then(|p| match p.to_lowercase().as_str() {
        "high" => Some(job_object::PRIORITY_HIGH),
        "above-normal" | "above_normal" | "abovenormal" => Some(job_object::PRIORITY_ABOVE_NORMAL),
        _ => None,
    });
    let limits = job_object::Limits {
        memory_bytes: cli.memory,
        cpu_percent: cli.cpus.map(|c| (c * 100.0) as u8),
        priority_class,
    };
    job_object::init_with_limits(limits);
    let data_dir = dirs_or_default();

    let command = cli.command.unwrap_or(Commands::Default);
    // `internal-run` is spawned as a child of run/start/default while the parent still
    // holds the image-store sled lock (sled is single-process). It works purely off the
    // already-extracted rootfs, so dispatch it before opening the store to avoid a deadlock.
    if matches!(&command, Commands::InternalRun(_)) {
        let Commands::InternalRun(args) = command else { unreachable!() };
        return run_internal(args, data_dir).await;
    }

    // `eject` only reads the project tree (stack detection + template generation)
    // and never touches the image store. Dispatch it before opening the sled db so
    // it works even while another process (e.g. the GUI app) holds the store lock —
    // sled is single-process, and the GUI shells out to `crush eject`.
    if matches!(&command, Commands::Eject(_)) {
        let Commands::Eject(args) = command else { unreachable!() };
        return run_eject(args).await;
    }

    // `clean` only touches the project tree (and optionally crush's cache dir),
    // never the image store. Dispatch it before the sled db opens so it works
    // even while the GUI holds the store lock.
    if matches!(&command, Commands::Clean(_)) {
        let Commands::Clean(args) = command else { unreachable!() };
        return run_clean(args).await;
    }

    // `ci` only reads the project tree (stack detection + template write) — no store.
    if matches!(&command, Commands::Ci(_)) {
        let Commands::Ci(args) = command else { unreachable!() };
        return commands::ci::exec(args.system, args.force).await;
    }

    // `catalog` is a static listing — no store needed.
    if matches!(&command, Commands::Catalog(_)) {
        let Commands::Catalog(args) = command else { unreachable!() };
        return run_catalog(args);
    }

    // `tunnel` just spawns a tunnel process for a local port — no store needed.
    if matches!(&command, Commands::Tunnel(_)) {
        let Commands::Tunnel(args) = command else { unreachable!() };
        return run_tunnel_cmd(args).await;
    }

    // `lint` only reads the project tree — no store needed.
    if matches!(&command, Commands::Lint(_)) {
        let Commands::Lint(args) = command else { unreachable!() };
        return run_lint(args);
    }

    // `mail` runs a standalone SMTP sink — no store needed.
    if matches!(&command, Commands::Mail(_)) {
        let Commands::Mail(args) = command else { unreachable!() };
        return run_mail_cmd(args).await;
    }

    // `gateway` and `l7-gateway` are traffic directors — no store needed.
    if let Commands::Gateway(args) = &command {
        return run_gateway_cmd(args.clone()).await;
    }
    if let Commands::L7Gateway(args) = &command {
        return run_l7_gateway_cmd(args.clone()).await;
    }

    // `ssh` reads ~/.ssh/config and execs the system ssh — no store needed.
    if matches!(&command, Commands::Ssh(_)) {
        let Commands::Ssh(args) = command else { unreachable!() };
        return commands::ssh::exec(args.host, args.list);
    }

    // `db` wraps native dump/restore tools — no image store needed.
    if matches!(&command, Commands::Db(_)) {
        let Commands::Db(args) = command else { unreachable!() };
        let (action, name, url, yes) = match args.cmd {
            DbSubcommand::Snapshot { name, url } => ("snapshot", Some(name), url, false),
            DbSubcommand::Restore { name, url, yes } => ("restore", Some(name), url, yes),
            DbSubcommand::Ls => ("ls", None, None, false),
            DbSubcommand::Rm { name } => ("rm", Some(name), None, false),
        };
        return commands::db::exec(action, name, url, yes, data_dir).await;
    }

    let store = ImageStore::new(data_dir.join("images")).await?;

    match command {
        Commands::Default => {
            let project_root = std::env::current_dir()?;
            let project_name = project_root.file_name()
                .map(|n| n.to_string_lossy().to_lowercase().replace([' ', '-'], "_"))
                .unwrap_or_else(|| "app".into());

            let options = crush_build::run::RunOptions {
                dev: cli.dev,
                rebuild: cli.rebuild,
                repack: cli.repack,
                no_proxy: cli.no_proxy,
                watch: cli.watch,
                memory_bytes: cli.memory,
                cpu_fraction: cli.cpus,
                priority: cli.priority.clone(),
                assume_yes: cli.no_interactive,
                smtp_capture_port: if cli.mail { Some(crush_build::mailbox::DEFAULT_PORT) } else { None },
            };

            let handle = crush_build::run::run_project(project_root.clone(), data_dir.clone(), options).await?;
            let mut events = handle.events;

            let mut overall_start = std::time::Instant::now();
            let mut image_name = String::new();
            let mut is_multi_service = false;
            let mut dep_count = 0usize;
            let mut per_service_color: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            let mut next_color = 0usize;
            // Holds the live tunnel when `--tunnel` is set; opened once the app
            // binds its port, torn down when the run ends.
            let mut tunnel_handle: Option<crush_build::tunnel::Tunnel> = None;
            let filter: std::sync::Arc<tokio::sync::RwLock<FilterMode>> = std::sync::Arc::new(tokio::sync::RwLock::new(FilterMode::All));
            let multi_service: std::sync::Arc<std::sync::atomic::AtomicBool> = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let mut any_output = false;

            // Keystroke listener for multi-service mode
            {
                let f = filter.clone();
                let ms = multi_service.clone();
                tokio::spawn(async move {
                    use tokio::io::AsyncBufReadExt;
                    let stdin = tokio::io::stdin();
                    let mut reader = tokio::io::BufReader::new(stdin).lines();
                    while let Ok(Some(line)) = reader.next_line().await {
                        if !ms.load(std::sync::atomic::Ordering::Relaxed) { continue; }
                        let key = line.trim().to_lowercase();
                        let mut mode = f.write().await;
                        match key.as_str() {
                            "a" => { *mode = FilterMode::All; eprintln!("  {} showing all logs", "→".cyan()); }
                            "e" => { *mode = FilterMode::OnlyErrors; eprintln!("  {} errors only", "→".cyan()); }
                            "p" => { *mode = FilterMode::Paused; eprintln!("  {} paused", "→".cyan()); }
                            "q" => { eprintln!("  {} quitting", "→".cyan()); break; }
                            _ => {}
                        }
                    }
                });
            }

            while let Some(event) = events.recv().await {
                match event {
                    RunEvent::Detected { language, framework, confidence, is_monorepo, port, dep_count: dc } => {
                        dep_count = dc;
                        is_multi_service = is_monorepo;
                        multi_service.store(is_monorepo, std::sync::atomic::Ordering::Relaxed);
                        let extra_note = if dep_count > 0 {
                            format!(" (+ {} service{})", dep_count, if dep_count == 1 { "" } else { "s" })
                        } else { String::new() };
                        if is_monorepo {
                            any_output = true;
                            let legs = language.split(',')
                                .map(|s| s.trim().bold().to_string())
                                .collect::<Vec<_>>().join(" · ");
                            println!("   {} detected: {} services — {}",
                                "↳".cyan(),
                                "2+".bold(),
                                legs.dimmed());
                        } else {
                            println!("   {} detected: {}{}",
                                "↳".cyan(),
                                format_detection_line_raw(&language, &framework, confidence).bold(),
                                extra_note.dimmed());
                        }
                    }

                    RunEvent::DepStarted { name, image, native } => {
                        let note = if native { "[native]" } else { "[container]" };
                        println!("   ↳ starting {} ({})... ok  {}", name, image, note);
                    }

                    RunEvent::DepFailed { name, error } => {
                        eprintln!("   ↳ starting {}... failed: {}", name, error);
                    }

                    RunEvent::ImageFresh { digest } => {
                        let _ = digest;
                        println!("   {} image fresh — {} {}",
                            "✓".green().bold(),
                            "skipping pack".dimmed(),
                            "(--repack to force)".dimmed());
                        // Mirror v0.7.72 output exactly: cached image
                        // implies layered-deps cache and emits the
                        // "crushed to image" headline at 0 MB.
                        println!("   {} dependencies layer cached {}",
                            "↳".cyan(),
                            "(unchanged)".dimmed());
                        image_name = format!("{}:latest", project_name);
                        println!(" {} crushed to image {} {}",
                            "✓".green().bold(),
                            image_name.bold(),
                            "(0 MB)".dimmed());
                    }

                    RunEvent::ImagePacked { digest, size_bytes, duration_ms } => {
                        let _ = digest;
                        image_name = format!("{}:latest", project_name);
                        let size_mb = size_bytes as f64 / (1024.0 * 1024.0);
                        println!(" {} crushed to image {} {}",
                            "✓".green().bold(),
                            image_name.bold(),
                            format!("({:.1}s · {:.0} MB)", duration_ms as f64 / 1000.0, size_mb).dimmed());
                    }

                    RunEvent::BuildStarted { command, service_name } => {
                        if let Some(svc) = service_name {
                            println!("   {} {}: building... {}", "⚙".yellow().bold(), svc.bold(), format!("({})", command).dimmed());
                        } else {
                            println!("   {} building... {}", "⚙".yellow().bold(), format!("({})", command).dimmed());
                        }
                    }

                    RunEvent::BuildOutput { line, stream, service_name } => {
                        if let Some(svc) = &service_name {
                            let idx = *per_service_color.entry(svc.clone()).or_insert_with(|| { let c = next_color; next_color += 1; c });
                            let show = should_show(&*filter.read().await, svc, &line, stream == Stream::Stderr);
                            if show {
                                match stream {
                                    Stream::Stdout => println!("{} {}", colour_prefix(svc, idx), line),
                                    Stream::Stderr => eprintln!("{} {}", colour_prefix(svc, idx), line),
                                }
                            }
                        } else {
                            match stream {
                                Stream::Stdout => println!("   {}", line),
                                Stream::Stderr => eprintln!("   {}", line),
                            }
                        }
                    }

                    RunEvent::BuildFinished { duration_ms, success, service_name } => {
                        let _ = service_name;
                        if success {
                            println!("   {} built in {:.1}s", "✓".green().bold(), duration_ms as f64 / 1000.0);
                        } else {
                            anyhow::bail!("Build failed");
                        }
                    }

                    RunEvent::Spawning { command, port, service_name } => {
                        let _ = command;
                        if let Some(svc) = &service_name {
                            println!("   {} {}: starting on {}", "↳".cyan(), svc.bold(), format!(":{}", port).cyan());
                        }
                    }

                    RunEvent::AppOutput { line, stream, service_name } => {
                        if let Some(svc) = &service_name {
                            let idx = *per_service_color.entry(svc.clone()).or_insert_with(|| { let c = next_color; next_color += 1; c });
                            let show = should_show(&*filter.read().await, svc, &line, stream == Stream::Stderr);
                            if show {
                                match stream {
                                    Stream::Stdout => println!("{} {}", colour_prefix(svc, idx), line),
                                    Stream::Stderr => eprintln!("{} {}", colour_prefix(svc, idx), line),
                                }
                            }
                        } else {
                            match stream {
                                Stream::Stdout => println!("{}", line),
                                Stream::Stderr => eprintln!("{}", line),
                            }
                        }
                    }

                    RunEvent::PortBound { port, startup_ms, total_ms, urls, service_name } => {
                        if let Some(svc) = &service_name {
                            println!("  {} {}  http://localhost:{}", "✓".green().bold(), svc.bold(), port.to_string().cyan());
                            for (label, url) in &urls {
                                println!("           {} {:<8} {}", "↳".dimmed(), label.dimmed(), url.dimmed());
                            }
                        } else {
                            let startup_s = startup_ms as f64 / 1000.0;
                            let total_s = total_ms as f64 / 1000.0;
                            println!(" {} running natively on {} {}",
                                "✓".green().bold(),
                                format!(":{}", port).cyan().bold(),
                                format!("— started in {:.1}s (total: {:.1}s!)", startup_s, total_s).dimmed());
                            if !urls.is_empty() {
                                println!("   {} open:", "↳".cyan());
                                for (label, url) in &urls {
                                    if label == "discovered" {
                                        println!("     {}", url.cyan());
                                    } else {
                                        println!("     {} {}", format!("[{}]", label).dimmed(), url.cyan());
                                    }
                                }
                            }
                            // `--tunnel`: expose this primary port publicly, once.
                            if cli.tunnel && tunnel_handle.is_none() {
                                println!("   {} opening public tunnel\u{2026}", "→".cyan());
                                let (ttx, mut trx) = tokio::sync::mpsc::channel::<crush_build::tunnel::TunnelEvent>(64);
                                tokio::spawn(async move { while trx.recv().await.is_some() {} });
                                match crush_build::tunnel::open(port, cli.tunnel_provider.as_deref(), ttx).await {
                                    Ok(h) => {
                                        println!("   {} public URL: {}  {}",
                                            "✓".green().bold(),
                                            h.url().green().bold(),
                                            format!("(via {})", h.provider().as_str()).dimmed());
                                        tunnel_handle = Some(h);
                                    }
                                    Err(e) => {
                                        eprintln!("   {} tunnel failed: {}", "⚠".yellow().bold(), e);
                                    }
                                }
                            }
                        }
                    }

                    RunEvent::Exited { code } => {
                        println!("   {} exited with code {}", "✗".red().bold(), code);
                    }

                    RunEvent::Warning { message } => {
                        eprintln!("   {} {}", "⚠".yellow().bold(), message);
                    }

                    RunEvent::WarmRun => {
                        println!("   {} warm run — launching", "↳".cyan());
                    }

                    RunEvent::DepsFresh => {
                        println!("   {} dependencies fresh — node_modules newer than lockfile {}",
                            "✓".green().bold(),
                            "(--rebuild to force)".dimmed());
                    }

                    _ => {}
                }
            }
        }
        Commands::Detect(args) => {
            info!("Detecting project stack...");
            let detector = StackDetector::new();
            let project_root = std::env::current_dir()?;
            let stack = detector.detect(&project_root).await?;

            if args.json {
                println!("{}", serde_json::to_string_pretty(&stack)?);
                return Ok(());
            }

            println!("Detected stack");
            println!("  Language:   {}", stack.language);
            println!("  Confidence: {:.0}%", stack.confidence * 100.0);
            println!("  Base image: {}", stack.base_image);
            println!("  Build cmd:  {}", stack.build_command);
            println!("  Entrypoint: {}", stack.entry_point);
            println!("  Port:       {}", stack.default_port);
            if !stack.services.is_empty() {
                println!("  Services:   {}", stack.services.iter()
                    .map(|s| format!("{} (:{})", s.name, s.port)).collect::<Vec<_>>().join(", "));
            }
            if !stack.external_services.is_empty() {
                println!("  External:   {}", stack.external_services.iter()
                    .map(|s| format!("{} [{}]", s.name, s.kind)).collect::<Vec<_>>().join(", "));
            }
            // Tunnel hint: webhook senders (Paystack/Stripe/Clerk/…) can't reach
            // localhost, so flag that this project wants a public URL in dev.
            let webhook = crush_build::env::tunnel_providers(&stack.external_services);
            if !webhook.is_empty() {
                let names = webhook.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(", ");
                println!("  Tunnel:     {} need a public URL for webhooks", names.yellow());
                println!("              {} {}", "↳".cyan(),
                    format!("crush tunnel {}  (or  crush run --tunnel)", stack.default_port).bold());
            }
        }
        Commands::Build(args) => {
            commands::build::exec(&args, &store).await?;
        }
        Commands::Watch(args) => {
            #[cfg(windows)]
            {
                let _ = args;
                eprintln!(" {} `crush watch` uses Linux overlayfs and isn't supported on Windows.",
                    "✗".red().bold());
                eprintln!("   ↳ use `crush --watch` instead (native watch mode, no containers)");
                std::process::exit(2);
            }
            #[cfg(not(windows))]
            {
            info!("Developer watch active (debounce: {}ms)", args.debounce);
            let project_root = std::env::current_dir()?;
            let cache_dir = data_dir.join("cache");
            let engine = BuildEngine::new(cache_dir.clone());
            let detector = StackDetector::new();
            let stack = detector.detect(&project_root).await?;

            // 1. Initial build & register image
            let outcome = engine.execute_layered_build(&project_root, &stack).await?;
            let digest = outcome.digest.clone();
            println!("[Watch] Initial build complete: {}", digest);

            // Copy layer to image store blobs
            let layer_file = cache_dir.join("layers").join(digest.replace(':', "_"));
            let blob_dest = store.blob_store().path_for_digest(&digest);
            if let Some(parent) = blob_dest.parent() {
                tokio::fs::create_dir_all(parent).await.ok();
            }
            tokio::fs::copy(&layer_file, &blob_dest).await.ok();

            let img = Image {
                id: digest.clone(),
                tag: "app:latest".to_string(),
                digest: digest.clone(),
                size_bytes: tokio::fs::metadata(&layer_file).await.map(|m| m.len()).unwrap_or(0),
                layers: vec![digest.clone()],
                architecture: "amd64".to_string(),
                os: "linux".to_string(),
                os_version: None,
                entrypoint: vec![],
                cmd: vec![stack.entry_point.clone()],
                env: vec![format!("PORT={}", stack.default_port)],
                config_digest: None,
                stack: Some(stack.language.clone()),
            };
            store.database().put_image(&img).await?;

            let container_id = format!("crush_{}", hex_encode_random());
            let container_name = "crush_watch_blue".to_string();
            let rootfs = data_dir.join("containers").join(&container_id).join("rootfs");
            tokio::fs::create_dir_all(&rootfs).await.ok();
            store.extract_layers(&img.id, &rootfs).await?;

            let container = Container {
                id: container_id.clone(),
                name: container_name.clone(),
                image: img.tag.clone(),
                status: ContainerStatus::Creating,
                pid: None,
                created_at: SystemTime::now(),
                started_at: None,
                ports: vec![],
                mounts: vec![],
                memory_limit_bytes: None,
                cpu_shares: None,
                health: None,
                restart_count: None,
                restart_policy: None,
                health_cmd: None,
                health_interval: None,
                health_timeout: None,
                health_retries: None,
                pids_limit: None,
                read_only: None,
                security_opt: None,
            };

            let container_dir = data_dir.join("containers").join(&container_id);
            #[cfg(target_os = "windows")]
            let backend = WindowsRuntime::new();
            #[cfg(not(target_os = "windows"))]
            let backend = StatelessEngine::new(data_dir.clone());
            backend.create(&container, &container_dir).await?;

            let config_json = serde_json::json!({
                "cmd": vec![stack.entry_point.clone()],
                "env": vec![format!("PORT={}", stack.default_port)],
            });
            let config_json_path = container_dir.join("config.json");
            tokio::fs::write(&config_json_path, serde_json::to_string_pretty(&config_json)?).await?;

            // Spin it up in background (detached mode)
            backend.start(&container_id).await?;
            println!("[Watch] Started container (Blue): {}", container_id);

            #[cfg(target_os = "linux")]
            let mut current_net = {
                let orchestrator = crush_network::NetworkOrchestrator::new(data_dir.clone());
                let ports = vec![PortMapping {
                    host_ip: "0.0.0.0".to_string(),
                    host_port: stack.default_port,
                    container_port: stack.default_port,
                    protocol: Protocol::Tcp,
                }];
                match orchestrator.setup_container_network(&container_id, &container_name, crush_network::modes::NetworkMode::Bridge, &ports).await {
                    Ok(net) => {
                        println!("[Watch] Configured network for {}: IP {:?}", container_id, net.container_ip);
                        Some(net)
                    }
                    Err(e) => {
                        eprintln!("[Watch] Network setup warning: {}", e);
                        None
                    }
                }
            };
            
            #[cfg(not(target_os = "linux"))]
            let mut current_net: Option<()> = None;

            let mut active_container_id = container_id;
            let mut active_container_name = container_name;

            let (tx, mut rx) = tokio::sync::mpsc::channel::<crush_tui::ChangeClass>(10);
            let project_root_clone = project_root.clone();
            
            // Start watcher
            let _watcher = crush_tui::watch::FileWatcher::new(&[&project_root_clone], move |change| {
                let _ = tx.blocking_send(change);
            })?;

            while let Some(change) = rx.recv().await {
                println!("[Watch] Change detected: {:?}", change);
                // Perform build
                let digest = match engine.execute_layered_build(&project_root, &stack).await {
                    Ok(o) => o.digest,
                    Err(e) => {
                        eprintln!("[Watch] Build failed: {}", e);
                        continue;
                    }
                };
                
                println!("[Watch] Rebuild complete: {}", digest);

                // Copy layer to image store blobs
                let layer_file = cache_dir.join("layers").join(digest.replace(':', "_"));
                let blob_dest = store.blob_store().path_for_digest(&digest);
                if let Some(parent) = blob_dest.parent() {
                    tokio::fs::create_dir_all(parent).await.ok();
                }
                tokio::fs::copy(&layer_file, &blob_dest).await.ok();

                let img = Image {
                    id: digest.clone(),
                    tag: "app:latest".to_string(),
                    digest: digest.clone(),
                    size_bytes: tokio::fs::metadata(&layer_file).await.map(|m| m.len()).unwrap_or(0),
                    layers: vec![digest.clone()],
                    architecture: "amd64".to_string(),
                    os: "linux".to_string(),
                    os_version: None,
                    entrypoint: vec![],
                    cmd: vec![stack.entry_point.clone()],
                    env: vec![format!("PORT={}", stack.default_port)],
                    config_digest: None,
                stack: Some(stack.language.clone()),
                };
                if let Err(e) = store.database().put_image(&img).await {
                    eprintln!("[Watch] DB register failed: {}", e);
                    continue;
                }

                // --- Blue-Green Hot-Swap Protocol ---
                let green_id = format!("crush_{}", hex_encode_random());
                let green_name = if active_container_name == "crush_watch_blue" {
                    "crush_watch_green".to_string()
                } else {
                    "crush_watch_blue".to_string()
                };

                println!("[Watch] Spawning new container (Green): {} ({})", green_id, green_name);
                let rootfs = data_dir.join("containers").join(&green_id).join("rootfs");
                tokio::fs::create_dir_all(&rootfs).await.ok();
                if let Err(e) = store.extract_layers(&img.id, &rootfs).await {
                    eprintln!("[Watch] Layer extraction failed: {}", e);
                    continue;
                }

                let green_container = Container {
                    id: green_id.clone(),
                    name: green_name.clone(),
                    image: img.tag.clone(),
                    status: ContainerStatus::Creating,
                    pid: None,
                    created_at: SystemTime::now(),
                    started_at: None,
                    ports: vec![],
                    mounts: vec![],
                    memory_limit_bytes: None,
                    cpu_shares: None,
                    health: None,
                    restart_count: None,
                    restart_policy: None,
                    health_cmd: None,
                    health_interval: None,
                    health_timeout: None,
                    health_retries: None,
                    pids_limit: None,
                    read_only: None,
                    security_opt: None,
                };

                let green_dir = data_dir.join("containers").join(&green_id);
                if let Err(e) = backend.create(&green_container, &green_dir).await {
                    eprintln!("[Watch] Create failed: {}", e);
                    continue;
                }

                let config_json = serde_json::json!({
                    "cmd": vec![stack.entry_point.clone()],
                    "env": vec![format!("PORT={}", stack.default_port)],
                });
                let config_json_path = green_dir.join("config.json");
                if let Err(e) = tokio::fs::write(&config_json_path, serde_json::to_string_pretty(&config_json)?).await {
                    eprintln!("[Watch] Config write failed: {}", e);
                    continue;
                }

                if let Err(e) = backend.start(&green_id).await {
                    eprintln!("[Watch] Start failed: {}", e);
                    continue;
                }

                // 2. Wait for health checks to pass (or fallback to 200ms delay)
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

                // 3. Atomically switch port-forwarding rules (`crush_nat`) via CNI/nftables
                #[cfg(target_os = "linux")]
                let green_net = {
                    let orchestrator = crush_network::NetworkOrchestrator::new(data_dir.clone());
                    let ports = vec![PortMapping {
                        host_ip: "0.0.0.0".to_string(),
                        host_port: stack.default_port,
                        container_port: stack.default_port,
                        protocol: Protocol::Tcp,
                    }];
                    match orchestrator.setup_container_network(&green_id, &green_name, crush_network::modes::NetworkMode::Bridge, &ports).await {
                        Ok(net) => {
                            println!("[Watch] Swapped port routing to Green IP: {:?}", net.container_ip);
                            Some(net)
                        }
                        Err(e) => {
                            eprintln!("[Watch] Green network setup failed: {}", e);
                            None
                        }
                    }
                };

                // 4. Stop and clean up the old container (Blue)
                println!("[Watch] Tearing down old container (Blue): {}", active_container_id);
                let _ = backend.stop(&active_container_id, 2).await;
                
                #[cfg(target_os = "linux")]
                {
                    if let Some(net) = current_net.take() {
                        let orchestrator = crush_network::NetworkOrchestrator::new(data_dir.clone());
                        let _ = orchestrator.teardown_container_network(&net).await;
                    }
                }
                
                let _ = backend.delete(&active_container_id).await;

                // Promote Green to Blue
                active_container_id = green_id;
                active_container_name = green_name;
                #[cfg(target_os = "linux")]
                {
                    current_net = green_net;
                }
                println!("[Watch] Blue-Green Swap complete!");
            }
            }
        }
        Commands::Run(args) => {
            commands::run::exec(&args, &data_dir, &store).await?;
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
                                        #[cfg(windows)]
                                        {
                                            use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
                                            use windows_sys::Win32::Foundation::CloseHandle;
                                            use windows_sys::Win32::System::Threading::GetExitCodeProcess;
                                            const STILL_ACTIVE: u32 = 259;
                                            unsafe {
                                                let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
                                                if handle == 0 {
                                                    is_alive = false;
                                                } else {
                                                    let mut exit_code: u32 = 0;
                                                    GetExitCodeProcess(handle, &mut exit_code);
                                                    is_alive = exit_code == STILL_ACTIVE;
                                                    CloseHandle(handle);
                                                }
                                            }
                                        }
                                        #[cfg(all(not(unix), not(windows)))]
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

            // --format json: emit machine-readable output and exit. Consumed
            // by the GUI's containers screen and by any scripting use.
            if args.format == "json" {
                println!("{}", serde_json::to_string_pretty(&container_list)?);
                return Ok(());
            }

            let tui = TuiApp::new(1, data_dir.clone());
            // If there are no containers, opening the alt-screen TUI is jarring —
            // user sees a momentary screen flash and lands back at the prompt with
            // nothing in scrollback. Print the empty-state inline instead.
            if cli.no_interactive || container_list.is_empty() {
                tui.draw_containers_table(&container_list);
                if container_list.is_empty() {
                    println!("   ↳ tip: run `crush` in a project to start one, or `crush ps --all` to include stopped containers");
                }
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
                                        #[cfg(windows)]
                                        {
                                            use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
                                            use windows_sys::Win32::Foundation::CloseHandle;
                                            unsafe {
                                                let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
                                                if handle != 0 {
                                                    TerminateProcess(handle, 1);
                                                    CloseHandle(handle);
                                                }
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
        Commands::Exec(args) => {
            let code = cmd_exec(&args, &data_dir).await;
            if code != 0 {
                std::process::exit(code);
            }
        }
        Commands::Logs(args) => {
            info!("Streaming logs for container: {} (follow: {})", args.id, args.follow);
            // Load all containers so the TUI's navigation still works.
            let containers_dir = data_dir.join("containers");
            let mut all_containers: Vec<Container> = Vec::new();
            let mut target_dir: Option<PathBuf> = None;
            if containers_dir.exists() {
                let mut entries = tokio::fs::read_dir(&containers_dir).await?;
                while let Some(entry) = entries.next_entry().await? {
                    let json_path = entry.path().join("container.json");
                    if json_path.exists() {
                        if let Ok(content) = tokio::fs::read_to_string(&json_path).await {
                            if let Ok(c) = serde_json::from_str::<Container>(&content) {
                                if c.id == args.id || c.name == args.id {
                                    target_dir = Some(entry.path());
                                }
                                all_containers.push(c);
                            }
                        }
                    }
                }
            }

            if target_dir.is_none() {
                eprintln!("Container {} not found", args.id);
            } else if !cli.no_interactive && atty::is(atty::Stream::Stdout) {
                // Interactive terminal: open TUI Logs tab pinned to this container.
                let tui = TuiApp::new(1, data_dir.clone());
                tui.run_logs(all_containers, &args.id)?;
            } else {
                // Piped / non-interactive: raw text, try both log file conventions.
                let dir = target_dir.unwrap();
                let mut printed = false;
                for (out_name, err_name) in &[
                    ("crush-run.log", "crush-run.err"),
                    ("stdout.log",    "stderr.log"),
                ] {
                    let out_path = dir.join(out_name);
                    let err_path = dir.join(err_name);
                    if out_path.exists() || err_path.exists() {
                        if let Ok(s) = tokio::fs::read_to_string(&out_path).await { print!("{}", s); }
                        if let Ok(s) = tokio::fs::read_to_string(&err_path).await  { eprint!("{}", s); }
                        printed = true;
                        if args.follow {
                            let mut out_off = tokio::fs::metadata(&out_path).await.map(|m| m.len()).unwrap_or(0);
                            let mut err_off = tokio::fs::metadata(&err_path).await.map(|m| m.len()).unwrap_or(0);
                            loop {
                                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                                for (path, offset) in [(&out_path, &mut out_off), (&err_path, &mut err_off)] {
                                    if path.exists() {
                                        if let Ok(mut f) = tokio::fs::File::open(path).await {
                                            use tokio::io::{AsyncReadExt, AsyncSeekExt};
                                            let len = f.seek(std::io::SeekFrom::End(0)).await.unwrap_or(0);
                                            if len > *offset {
                                                let _ = f.seek(std::io::SeekFrom::Start(*offset)).await;
                                                let mut buf = Vec::new();
                                                let _ = f.read_to_end(&mut buf).await;
                                                if !buf.is_empty() {
                                                    use std::io::Write;
                                                    print!("{}", String::from_utf8_lossy(&buf));
                                                    let _ = std::io::stdout().flush();
                                                    *offset += buf.len() as u64;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        break;
                    }
                }
                if !printed {
                    eprintln!("No logs found for container {}", args.id);
                }
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
                // Try the common log sinks in order; the stateless runtime writes
                // crush-run.err / crush-run.log, while older paths used stderr.log.
                for candidate in ["stderr.log", "crush-run.err", "crush-run.log"] {
                    let p = cdir.join(candidate);
                    if p.exists() {
                        if let Ok(c) = tokio::fs::read_to_string(&p).await {
                            if !c.trim().is_empty() {
                                stderr_content = Some(c);
                                break;
                            }
                        }
                    }
                }
            }
            let stderr = stderr_content.as_deref().unwrap_or("");
            if stderr.is_empty() {
                println!("No stderr log found for container {}. Nothing to diagnose.", args.id);
            } else {
                // Resolve the AI key: explicit config.json override (ai_api_key)
                // takes precedence, else the environment (Gemini preferred, then
                // Anthropic). End users bring their own key.
                let cfg_key = std::fs::read_to_string(data_dir.join("config.json")).ok()
                    .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
                    .and_then(|j| j.get("ai_api_key").and_then(|v| v.as_str()).map(|s| s.to_string()))
                    .filter(|s| !s.trim().is_empty());
                let engine = AiEngine::new(cfg_key, data_dir.clone());
                match engine.active_provider() {
                    Some(p) => println!("AI provider: {}", p),
                    None => println!("AI provider: offline patterns only (set GEMINI_API_KEY for cloud AI, or CRUSH_AI_PROVIDER=ollama to run a local model — no key, fully on-device)"),
                }
                let project_root = std::env::current_dir().ok();
                let full = engine.diagnose_stderr(
                    stderr,
                    None,
                    project_root.as_deref(),
                ).await?;
                // Format the diagnosis into two pane strings.
                let mut source_pane = stderr.lines().take(40)
                    .enumerate()
                    .map(|(i, l)| format!("{:>3}: {}", i + 1, l))
                    .collect::<Vec<_>>()
                    .join("\n");
                if let Some(ref trace) = full.trace {
                    source_pane = format!(
                        "Language:  {}\nException: {}\nMessage:   {}\nFile:      {}:{}\nFrames:    {}\n\n{}",
                        trace.language, trace.exception_type, trace.message,
                        trace.file, trace.line, trace.stack_frames.len(),
                        source_pane
                    );
                }

                let mut diag_pane = String::new();
                if let Some(ref diag) = full.diagnosis {
                    diag_pane.push_str(&format!("Root cause:\n  {}\n\nFix:\n  {}\n\nConfidence: {:.0}%\n",
                        diag.root_cause, diag.fix_description, diag.confidence * 100.0));
                    if let Some(ref patch) = diag.proposed_patch {
                        diag_pane.push_str(&format!("\nSuggested patch:\n{}\n", patch));
                    }
                }
                for be in &full.build_errors {
                    diag_pane.push_str(&format!("\nBuild error [{:?}]:\n  {} at {}:{}\n",
                        be.kind, be.message,
                        be.file.as_deref().unwrap_or("<unknown>"),
                        be.line.unwrap_or(0)));
                }
                if diag_pane.is_empty() {
                    diag_pane = "No structured diagnosis. See left pane for raw stderr.".to_string();
                }

                if !cli.no_interactive && atty::is(atty::Stream::Stdout) {
                    // Load all containers for TUI navigation, then open Debug tab.
                    let mut all_containers: Vec<Container> = Vec::new();
                    let cdir = data_dir.join("containers");
                    if cdir.exists() {
                        if let Ok(mut entries) = tokio::fs::read_dir(&cdir).await {
                            while let Some(e) = entries.next_entry().await.ok().flatten() {
                                let jp = e.path().join("container.json");
                                if let Ok(c) = tokio::fs::read_to_string(&jp).await
                                    .and_then(|s| serde_json::from_str::<Container>(&s).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
                                {
                                    all_containers.push(c);
                                }
                            }
                        }
                    }
                    let tui = TuiApp::new(1, data_dir.clone());
                    tui.run_debug(all_containers, &args.id, source_pane, diag_pane)?;
                } else {
                    // Non-interactive: plain text output.
                    println!("\n=== AI Debug Diagnosis for container {} ===\n", args.id);
                    println!("{}\n\n---\n\n{}", source_pane, diag_pane);
                }
            }
        }
        Commands::Inspect(args) => {
            info!("Inspecting: {} (type: {})", args.id, args.inspect_type);
            let mut found = false;
            
            if args.inspect_type == "container" {
                let containers_dir = data_dir.join("containers");
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
                                            let status_str = match c.status {
                                                crush_types::ContainerStatus::Running => "running",
                                                crush_types::ContainerStatus::Stopped => "exited",
                                                crush_types::ContainerStatus::Paused  => "paused",
                                                crush_types::ContainerStatus::Created => "created",
                                                crush_types::ContainerStatus::Creating => "creating",
                                            };

                                            let created_ts = c.created_at
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .map(|d| {
                                                    let secs = d.as_secs();
                                                    format!("{}", chrono::DateTime::<chrono::Utc>::from_timestamp(secs as i64, 0)
                                                        .unwrap_or_default()
                                                        .format("%Y-%m-%d %H:%M:%S UTC"))
                                                })
                                                .unwrap_or_else(|_| "unknown".to_string());

                                            println!("Container: {}", c.id);
                                            println!("  Name:     {}", c.name);
                                            println!("  Status:   {}", status_str);
                                            println!("  Image:    {}", c.image);
                                            println!("  Created:  {}", created_ts);
                                            if let Some(pid) = c.pid {
                                                println!("  PID:      {}", pid);
                                            }

                                            if !c.ports.is_empty() {
                                                println!("  Ports:");
                                                for p in &c.ports {
                                                    println!("    {}:{} -> :{}/{}",
                                                        if p.host_ip.is_empty() || p.host_ip == "0.0.0.0" { "*".to_string() } else { p.host_ip.clone() },
                                                        p.host_port,
                                                        p.container_port,
                                                        match p.protocol { crush_types::Protocol::Tcp => "tcp", crush_types::Protocol::Udp => "udp" }
                                                    );
                                                }
                                            }

                                            if !c.mounts.is_empty() {
                                                println!("  Mounts:");
                                                for m in &c.mounts {
                                                    let mode = if m.read_only { "ro" } else { "rw" };
                                                    let kind = if m.is_tmpfs { "tmpfs" } else { "bind" };
                                                    println!("    {} -> {} ({}, {})", m.host_path.display(), m.container_path.display(), kind, mode);
                                                }
                                            }

                                            if c.memory_limit_bytes.is_some() || c.cpu_shares.is_some() {
                                                println!("  Resources:");
                                                if let Some(mem) = c.memory_limit_bytes {
                                                    println!("    Memory limit: {}", format_bytes(mem));
                                                }
                                                if let Some(cpu) = c.cpu_shares {
                                                    println!("    CPU shares:   {}", cpu);
                                                }
                                                if let Some(pids) = c.pids_limit {
                                                    println!("    PIDs limit:   {}", pids);
                                                }
                                            }

                                            if let Some(health) = &c.health {
                                                let h_str = match health {
                                                    crush_types::HealthStatus::Healthy   => "healthy",
                                                    crush_types::HealthStatus::Unhealthy => "unhealthy",
                                                    crush_types::HealthStatus::Starting  => "starting",
                                                };
                                                println!("  Health:   {}", h_str);
                                                if let Some(cmd) = &c.health_cmd {
                                                    println!("    Check: {}", cmd);
                                                }
                                            }

                                            if let Some(policy) = &c.restart_policy {
                                                println!("  Restart:  {} (count: {})", policy, c.restart_count.unwrap_or(0));
                                            }
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
                    eprintln!("Error: container '{}' not found", args.id);
                }
            } else if args.inspect_type == "image" {
                let image_opt = if let Ok(Some(img)) = store.database().get_image_by_tag(&args.id).await {
                    Some(img)
                } else if let Ok(Some(img)) = store.database().get_image_by_digest(&args.id).await {
                    Some(img)
                } else {
                    None
                };

                if let Some(image) = image_opt {
                    if args.format == "json" {
                        println!("{}", serde_json::to_string_pretty(&image)?);
                    } else {
                        println!("Image: {}", image.tag);
                        println!("  ID:           {}", &image.id[..16.min(image.id.len())]);
                        println!("  Digest:       {}", image.digest);
                        println!("  Architecture: {}/{}", image.os, image.architecture);
                        println!("  Size:         {}", format_bytes(image.size_bytes));
                        println!("  Layers:       {}", image.layers.len());
                        if !image.entrypoint.is_empty() {
                            println!("  Entrypoint:   {}", image.entrypoint.join(" "));
                        }
                        if !image.cmd.is_empty() {
                            println!("  Cmd:          {}", image.cmd.join(" "));
                        }
                        if !image.env.is_empty() {
                            println!("  Env:");
                            for e in &image.env {
                                println!("    {}", e);
                            }
                        }
                    }
                    found = true;
                }
                if !found {
                    eprintln!("Error: image '{}' not found", args.id);
                }
            } else if args.inspect_type == "network" {
                let net_path = data_dir.join("networks").join(format!("{}.json", args.id));
                if net_path.exists() {
                    if let Ok(content) = tokio::fs::read_to_string(&net_path).await {
                        if args.format == "json" {
                            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&content) {
                                println!("{}", serde_json::to_string_pretty(&json_val)?);
                            } else {
                                println!("{}", content);
                            }
                        } else {
                            println!("{}", content);
                        }
                        found = true;
                    }
                } else {
                    let networks_dir = data_dir.join("networks");
                    if networks_dir.exists() {
                        let mut entries = tokio::fs::read_dir(&networks_dir).await?;
                        while let Some(entry) = entries.next_entry().await? {
                            if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                                if let Ok(content) = tokio::fs::read_to_string(entry.path()).await {
                                    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&content) {
                                        if json_val.get("name").and_then(|n| n.as_str()) == Some(&args.id) {
                                            if args.format == "json" {
                                                println!("{}", serde_json::to_string_pretty(&json_val)?);
                                            } else {
                                                println!("{:#?}", json_val);
                                            }
                                            found = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if !found {
                    eprintln!("Error: network '{}' not found", args.id);
                }
            } else {
                eprintln!("Error: unknown resource type '{}'", args.inspect_type);
            }
        }
        Commands::Stats(args) => {
            info!("Reading metrics stats (no-stream: {})", args.no_stream);
            let tui = TuiApp::new(1, data_dir.clone());
            // Load running containers from filesystem
            let mut container_list: Vec<Container> = Vec::new();
            let containers_dir = data_dir.join("containers");
            if containers_dir.exists() {
                let mut entries = tokio::fs::read_dir(&containers_dir).await?;
                while let Some(entry) = entries.next_entry().await? {
                    let json_path = entry.path().join("container.json");
                    if json_path.exists() {
                        if let Ok(content) = tokio::fs::read_to_string(&json_path).await {
                            if let Ok(c) = serde_json::from_str::<Container>(&content) {
                                if c.status == ContainerStatus::Running {
                                    container_list.push(c);
                                }
                            }
                        }
                    }
                }
            }
            if cli.no_interactive || args.no_stream {
                if container_list.is_empty() {
                    println!("No running containers.");
                } else {
                    let mut first_samples = std::collections::HashMap::new();
                    for c in &container_list {
                        if let Some(pid) = c.pid {
                            if let Some((ticks, mem)) = get_cpu_and_mem(pid) {
                                first_samples.insert(c.id.clone(), (ticks, mem));
                            }
                        }
                    }

                    let start_time = std::time::Instant::now();
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    let elapsed_secs = start_time.elapsed().as_secs_f64();

                    let num_cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as f64;

                    println!("{:<16} {:<20} {:<12} {:<12}", "CONTAINER ID", "NAME", "CPU %", "MEM USAGE");
                    for c in &container_list {
                        let mut cpu_str = "0.0%".to_string();
                        let mut mem_str = "0.0 KB".to_string();

                        if let Some(pid) = c.pid {
                            if let Some((ticks_before, _)) = first_samples.get(&c.id) {
                                if let Some((ticks_after, mem_after)) = get_cpu_and_mem(pid) {
                                    let delta = if ticks_after >= *ticks_before {
                                        ticks_after - ticks_before
                                    } else {
                                        0
                                    };

                                    #[cfg(target_os = "windows")]
                                    let elapsed_cpu_secs = delta as f64 * 1e-7;
                                    #[cfg(not(target_os = "windows"))]
                                    let elapsed_cpu_secs = delta as f64 / 100.0;

                                    let cpu_pct = (elapsed_cpu_secs / elapsed_secs / num_cpus) * 100.0;
                                    cpu_str = format!("{:.1}%", cpu_pct);
                                    mem_str = format_mem(mem_after);
                                }
                            }
                        }

                        println!("{:<16} {:<20} {:<12} {:<12}",
                            &c.id[..12.min(c.id.len())], c.name, cpu_str, mem_str);
                    }
                }
            } else {
                tui.run_stats(container_list)?;
            }
        }
        Commands::Events(args) => {
            info!("Subscribing to system events with filter: {:?}", args.filter);
            let containers_dir = data_dir.join("containers");
            let mut events: Vec<(u64, String)> = Vec::new();
            if containers_dir.exists() {
                let mut entries = tokio::fs::read_dir(&containers_dir).await?;
                while let Some(entry) = entries.next_entry().await? {
                    let json_path = entry.path().join("container.json");
                    if json_path.exists() {
                        if let Ok(content) = tokio::fs::read_to_string(&json_path).await {
                            if let Ok(c) = serde_json::from_str::<Container>(&content) {
                                let matches_filter = args.filter.as_deref()
                                    .map(|f| c.image.contains(f) || c.name.contains(f) || c.id.contains(f))
                                    .unwrap_or(true);
                                if !matches_filter { continue; }
                                let created_ts = c.created_at.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                                events.push((created_ts, format!("container create  id={}  image={}  name={}", &c.id[..12.min(c.id.len())], c.image, c.name)));
                                if let Some(started) = c.started_at {
                                    let ts = started.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                                    events.push((ts, format!("container start   id={}  image={}  name={}", &c.id[..12.min(c.id.len())], c.image, c.name)));
                                }
                                if c.status == ContainerStatus::Stopped {
                                    events.push((created_ts + 1, format!("container die     id={}  image={}  name={}  exitCode=0", &c.id[..12.min(c.id.len())], c.image, c.name)));
                                }
                            }
                        }
                    }
                }
            }
            events.sort_by_key(|(ts, _)| *ts);
            if events.is_empty() {
                println!("No events found. Start some containers first.");
            } else {
                for (ts, msg) in &events {
                    println!("  [{}] {}", ts, msg);
                }
            }
        }
        Commands::Pull(args) => {
            if let Some(ref plat) = args.platform {
                std::env::set_var("CRUSH_DEFAULT_PLATFORM", plat);
            }
            info!("Pulling image: {}", args.image);
            let image = store.pull_image(&args.image).await?;
            println!("Successfully pulled image:");
            println!("  Reference: {}", args.image);
            println!("  Digest: {}", image.digest);
            println!("  Layers: {}", image.layers.len());

            // On Windows: pre-build ext4 drive so first `crush run` is instant
            #[cfg(target_os = "windows")]
            {
                use crush_runtime_windows::ext4_cache::Ext4Cache;
                let rootfs_staging = data_dir.join("staging").join(&image.id);
                if !rootfs_staging.exists() {
                    tokio::fs::create_dir_all(&rootfs_staging).await.ok();
                    if store.extract_layers(&image.id, &rootfs_staging).await.is_ok() {
                        let cache = Ext4Cache::new(&data_dir);
                        match cache.build(&image.digest, &rootfs_staging) {
                            Ok(path) => println!("  Cached ext4 drive: {}", path.display()),
                            Err(e) => eprintln!("  [warn] ext4 cache build failed: {} (first run will be slower)", e),
                        }
                    }
                }
            }
        }
        Commands::Login(args) => {
            use std::io::Write as _;
            let registry = args.registry.trim_end_matches('/').to_string();
            let username = match args.username {
                Some(u) => u,
                None => {
                    print!("Username: ");
                    std::io::stdout().flush()?;
                    let mut s = String::new();
                    std::io::stdin().read_line(&mut s)?;
                    s.trim().to_string()
                }
            };
            let password = if args.password_stdin {
                let mut s = String::new();
                std::io::stdin().read_line(&mut s)?;
                s.trim().to_string()
            } else if let Some(p) = args.password {
                p
            } else {
                rpassword::prompt_password("Password: ")?
            };

            let mut auth = crush_registry::auth::AuthHandler::new();
            auth.authenticate_basic(&registry, &username, &password).await?;
            auth.save_to_disk(&data_dir.join("auth.json"))?;
            println!("Login succeeded for {}", registry);
        }
        Commands::Images(args) => {
            info!("Listing images (show intermediate: {})", args.all);
            let images = store.list_images().await?;
            if args.format == "json" {
                println!("{}", serde_json::to_string_pretty(&images)?);
                return Ok(());
            }
            if images.is_empty() {
                println!("No images found.");
            } else {
                println!("{:<20} {:<12} {:<12} {:<10}", "REPOSITORY", "TAG", "IMAGE ID", "SIZE");
                for img in &images {
                    let short_id = if img.id.len() > 12 { &img.id[7..19] } else { &img.id };
                    println!("{:<20} {:<12} {:<12} {:<10}", img.tag, "latest", short_id, format_bytes(img.size_bytes));
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
            let image = match store.database().get_image_by_digest(&args.source).await? {
                Some(img) => img,
                None => store.database().get_image_by_tag(&args.source).await?
                    .ok_or_else(|| anyhow::anyhow!("Image {} not found", args.source))?,
            };
            let mut tagged = image.clone();
            tagged.tag = args.target.clone();
            store.database().put_image(&tagged).await?;
            println!("Tagged {} as {}", args.source, args.target);
        }
        Commands::Export(args) => {
            info!("Exporting image: {} to tarball: {}", args.image, args.output);
            store.export_oci_tarball(&args.image, &PathBuf::from(&args.output)).await?;
            println!("Exported {} → {}", args.image, args.output);
            println!("Load on any Docker host: docker load -i {}", args.output);
        }
        // Dispatched before the store opens (see above); unreachable here.
        Commands::Eject(_) => unreachable!("eject is handled before the image store opens"),
        Commands::Clean(_) => unreachable!("clean is handled before the image store opens"),
        Commands::Ci(_) => unreachable!("ci is handled before the image store opens"),
        Commands::Catalog(_) => unreachable!("catalog is handled before the image store opens"),
        Commands::L7Gateway(_) => unreachable!("l7-gateway is handled before the image store opens"),
        Commands::Tunnel(_) => unreachable!("tunnel is handled before the image store opens"),
        Commands::Db(_) => unreachable!("db is handled before the image store opens"),
        Commands::Lint(_) => unreachable!("lint is handled before the image store opens"),
        Commands::Mail(_) => unreachable!("mail is handled before the image store opens"),
        Commands::Gateway(_) => unreachable!("gateway is handled before the image store opens"),
        Commands::Ssh(_) => unreachable!("ssh is handled before the image store opens"),
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

                if which::which("trivy").is_ok() {
                    println!("Trivy binary detected, using Trivy for scan...");
                    let status = std::process::Command::new("trivy")
                        .args(["image", "--format", "table", &image])
                        .status();
                    if let Ok(st) = status {
                        if st.success() {
                            return Ok(());
                        }
                    }
                }

                println!("Scanning {} (extracting packages from rootfs)...", image);
                let tmp = tempfile::TempDir::new()?;
                if let Err(e) = store.extract_layers(&image, &tmp.path().to_path_buf()).await {
                    eprintln!("Failed to extract layers for scanning: {}", e);
                    return Ok(());
                }

                let packages = crush_image::extract_packages(&image, tmp.path()).await;
                if packages.is_empty() {
                    println!("0 packages found in image rootfs. 0 critical, 0 high — clean");
                    return Ok(());
                }

                println!("Querying OSV API for {} packages...", packages.len());

                let queries: Vec<serde_json::Value> = packages.iter().map(|p| {
                    serde_json::json!({
                        "package": {
                            "name": p.name,
                            "ecosystem": p.ecosystem
                        },
                        "version": p.version
                    })
                }).collect();

                let body = serde_json::json!({ "queries": queries });

                let client = reqwest::Client::new();
                let res = client.post("https://api.osv.dev/v1/querybatch")
                    .json(&body)
                    .send()
                    .await;

                match res {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            if let Ok(json_res) = resp.json::<serde_json::Value>().await {
                                let mut critical_cnt = 0;
                                let mut high_cnt = 0;
                                let mut medium_cnt = 0;
                                let mut low_cnt = 0;

                                if let Some(results) = json_res["results"].as_array() {
                                    for (i, result) in results.iter().enumerate() {
                                        if let Some(vulns) = result["vulns"].as_array() {
                                            let pkg = &packages[i];
                                            for vuln in vulns {
                                                let id = vuln["id"].as_str().unwrap_or("unknown");
                                                let summary = vuln["summary"].as_str().unwrap_or("No summary provided");
                                                
                                                let mut severity = "MEDIUM";
                                                if let Some(s_str) = vuln["database_specific"]["severity"].as_str() {
                                                    severity = s_str;
                                                } else if let Some(sevs) = vuln["severity"].as_array() {
                                                    if let Some(s_type) = sevs.first() {
                                                        if let Some(score) = s_type["score"].as_str() {
                                                            severity = score;
                                                        }
                                                    }
                                                }

                                                let severity = severity.to_uppercase();
                                                match severity.as_str() {
                                                    "CRITICAL" => critical_cnt += 1,
                                                    "HIGH" => high_cnt += 1,
                                                    "LOW" => low_cnt += 1,
                                                    _ => medium_cnt += 1,
                                                }

                                                println!("{:<10} {:<15} {:<10} {} — {}", severity, id, pkg.name, pkg.version, summary);
                                            }
                                        }
                                    }
                                }

                                if critical_cnt == 0 && high_cnt == 0 && medium_cnt == 0 && low_cnt == 0 {
                                    println!("0 critical, 0 high, 0 medium — clean");
                                } else {
                                    println!("{} critical, {} high, {} medium, {} low vulnerabilities found.", critical_cnt, high_cnt, medium_cnt, low_cnt);
                                }
                            } else {
                                eprintln!("Failed to parse OSV API JSON response.");
                            }
                        } else {
                            eprintln!("OSV API query failed with status: {}", resp.status());
                        }
                    }
                    Err(e) => {
                        eprintln!("Network error querying OSV API: {}", e);
                    }
                }
            }
        }
        Commands::Sbom(args) => {
            info!("Generating {} SBOM for image: {}", args.format, args.image);
            let tmp = tempfile::TempDir::new()?;
            store.extract_layers(&args.image, &tmp.path().to_path_buf()).await?;
            let components = crush_build::sbom::walk_rootfs(tmp.path());
            
            let sbom = if args.format.to_lowercase() == "spdx" {
                let spdx_components: Vec<serde_json::Value> = components.iter().enumerate().map(|(i, c)| serde_json::json!({
                    "name": c.name,
                    "versionInfo": c.version,
                    "SPDXID": format!("SPDXRef-Package-{}", i),
                    "downloadLocation": "NONE",
                    "filesAnalyzed": false,
                    "externalRefs": [
                        {
                            "referenceCategory": "PACKAGE-MANAGER",
                            "referenceType": "purl",
                            "referenceLocator": c.purl
                        }
                    ]
                })).collect();
                serde_json::json!({
                    "spdxVersion": "SPDX-2.3",
                    "dataLicense": "CC0-1.0",
                    "SPDXID": "SPDXRef-DOCUMENT",
                    "name": format!("{}-sbom", args.image),
                    "creationInfo": {
                        "creators": ["Tool: Crush Sbom Generator"],
                        "created": chrono::Utc::now().to_rfc3339()
                    },
                    "packages": spdx_components
                })
            } else {
                serde_json::json!({
                    "bomFormat": "CycloneDX",
                    "specVersion": "1.4",
                    "serialNumber": format!("urn:uuid:{}", hex_encode_random()),
                    "metadata": {
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "component": {
                            "name": args.image,
                            "type": "container"
                        }
                    },
                    "components": components.iter().map(|c| serde_json::json!({
                        "type": "library",
                        "name": c.name,
                        "version": c.version,
                        "purl": c.purl
                    })).collect::<Vec<_>>()
                })
            };

            if args.output.is_some() {
                tokio::fs::write(args.output.unwrap(), serde_json::to_string_pretty(&sbom)?).await?;
            } else {
                println!("{}", serde_json::to_string_pretty(&sbom)?);
            }
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
            let compose_path = PathBuf::from(&args.file);
            match args.subcommand {
                ComposeSubcommand::Up => {
                    run_compose_up(&compose_path, &data_dir, &store).await?;
                }
                ComposeSubcommand::Down => {
                    let project_name = compose_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                    let state_path = data_dir.join("compose").join(&project_name).with_extension("json");
                    if state_path.exists() {
                        let content = tokio::fs::read_to_string(&state_path).await?;
                        let state: std::collections::HashMap<String, String> = serde_json::from_str(&content)?;
                        #[cfg(target_os = "windows")]
                        let backend = WindowsRuntime::new();
                        #[cfg(not(target_os = "windows"))]
                        let backend = StatelessEngine::new(data_dir.clone());
                        for (svc_name, container_id) in &state {
                            println!("  Stopping {}...", svc_name);
                            let _ = backend.stop(container_id, 10).await;
                            let _ = backend.delete(container_id).await;
                        }
                        tokio::fs::remove_file(&state_path).await.ok();
                        println!("Compose down: all services stopped.");
                    } else {
                        println!("No running compose state found.");
                    }
                }
                ComposeSubcommand::Ps => {
                    let project_name = compose_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                    let state_path = data_dir.join("compose").join(&project_name).with_extension("json");
                    if state_path.exists() {
                        let content = tokio::fs::read_to_string(&state_path).await?;
                        let state: std::collections::HashMap<String, String> = serde_json::from_str(&content)?;
                        println!("{:<20} {:<20} {}", "SERVICE", "CONTAINER ID", "STATUS");
                        for (svc_name, container_id) in &state {
                            let json_path = data_dir.join("containers").join(container_id).join("container.json");
                            let status = if json_path.exists() {
                                let content = std::fs::read_to_string(&json_path).unwrap_or_default();
                                let c: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
                                c["status"].as_str().unwrap_or("unknown").to_string()
                            } else {
                                "not found".to_string()
                            };
                            let short_id = if container_id.len() > 16 { &container_id[..16] } else { container_id.as_str() };
                            println!("{:<20} {:<20} {}", svc_name, short_id, status);
                        }
                    } else {
                        println!("No compose state found. Run `crush compose up` first.");
                    }
                }
                ComposeSubcommand::Logs(logs_args) => {
                    let project_name = compose_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                    let state_path = data_dir.join("compose").join(&project_name).with_extension("json");
                    if state_path.exists() {
                        let content = tokio::fs::read_to_string(&state_path).await?;
                        let state: std::collections::HashMap<String, String> = serde_json::from_str(&content)?;
                        
                        if logs_args.follow {
                            let (tx, mut rx) = tokio::sync::mpsc::channel::<(String, String)>(256);
                            for (svc_name, container_id) in &state {
                                let stdout_path = data_dir.join("containers").join(container_id).join("stdout.log");
                                let stderr_path = data_dir.join("containers").join(container_id).join("stderr.log");
                                
                                let tx_out = tx.clone();
                                let svc_out = svc_name.clone();
                                tokio::spawn(async move {
                                    if let Ok(mut f) = tokio::fs::File::open(&stdout_path).await {
                                        use tokio::io::{AsyncReadExt, AsyncSeekExt};
                                        let _ = f.seek(std::io::SeekFrom::End(0)).await;
                                        let mut buf = [0u8; 1024];
                                        loop {
                                            match f.read(&mut buf).await {
                                                Ok(0) => tokio::time::sleep(tokio::time::Duration::from_millis(200)).await,
                                                Ok(n) => {
                                                    let text = String::from_utf8_lossy(&buf[..n]).to_string();
                                                    for line in text.lines() {
                                                        let _ = tx_out.send((svc_out.clone(), line.to_string())).await;
                                                    }
                                                }
                                                Err(_) => break,
                                            }
                                        }
                                    }
                                });

                                let tx_err = tx.clone();
                                let svc_err = svc_name.clone();
                                tokio::spawn(async move {
                                    if let Ok(mut f) = tokio::fs::File::open(&stderr_path).await {
                                        use tokio::io::{AsyncReadExt, AsyncSeekExt};
                                        let _ = f.seek(std::io::SeekFrom::End(0)).await;
                                        let mut buf = [0u8; 1024];
                                        loop {
                                            match f.read(&mut buf).await {
                                                Ok(0) => tokio::time::sleep(tokio::time::Duration::from_millis(200)).await,
                                                Ok(n) => {
                                                    let text = String::from_utf8_lossy(&buf[..n]).to_string();
                                                    for line in text.lines() {
                                                        let _ = tx_err.send((svc_err.clone(), line.to_string())).await;
                                                    }
                                                }
                                                Err(_) => break,
                                            }
                                        }
                                    }
                                });
                            }
                            drop(tx);
                            println!("Following logs (Ctrl+C to stop)...");
                            while let Some((svc, line)) = rx.recv().await {
                                println!("[{}] {}", svc, line);
                            }
                        } else {
                            for (svc_name, container_id) in &state {
                                let container_dir = data_dir.join("containers").join(container_id);
                                let stdout_path = container_dir.join("stdout.log");
                                let stderr_path = container_dir.join("stderr.log");
                                println!("=== {} ({}) ===", svc_name, &container_id[..12.min(container_id.len())]);
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
                            }
                        }
                    } else {
                        println!("No running compose state. Run `crush compose up` first.");
                    }
                }
            }
        }
        Commands::Network(args) => {
            info!("Network management operation: {:?}", args.subcommand);
            #[cfg(target_os = "linux")]
            {
                let net = NetworkManager::new(data_dir.join("networks"));
                match args.subcommand {
                    NetworkSubcommand::Create { name, subnet } => {
                        let subnet_str = subnet.unwrap_or_else(|| "172.18.0.0/16".to_string());
                        let gateway = subnet_str.replace(".0/16", ".1").replace(".0/24", ".1");
                        net.create(&name, &subnet_str, &gateway).await?;
                        println!("Created network: {} ({})", name, subnet_str);
                    }
                    NetworkSubcommand::Rm { name } => {
                        net.remove(&name).await?;
                        println!("Removed network: {}", name);
                    }
                    NetworkSubcommand::Ls => {
                        let nets = net.list().await?;
                        println!("{:<20} {:<20} {:<18} {}", "NAME", "ID", "SUBNET", "GATEWAY");
                        println!("{}", "-".repeat(72));
                        for n in &nets {
                            println!("{:<20} {:<20} {:<18} {}", n.name, &n.id[..12.min(n.id.len())], n.subnet, n.gateway);
                        }
                        if nets.is_empty() {
                            println!("No user-defined networks.");
                        }
                    }
                }
            }
            #[cfg(not(target_os = "linux"))]
            {
                let _ = args;
                eprintln!("Network management requires Linux.");
            }
        }
        Commands::Volume(args) => {
            info!("Volume management operation: {:?}", args.subcommand);
            let driver = LocalDriver::new(data_dir.clone());
            match args.subcommand {
                VolumeSubcommand::Create { name } => {
                    driver.create(&name, std::collections::HashMap::new()).await?;
                    println!("Created volume: {}", name);
                }
                VolumeSubcommand::Rm { name } => {
                    match driver.remove(&name).await {
                        Ok(_) => println!("Removed volume: {}", name),
                        Err(e) => eprintln!("Error: {}", e),
                    }
                }
                VolumeSubcommand::Ls => {
                    let list = driver.list().await?;
                    println!("{:<20} | {:<10} | {:<30} | {:<20}", "NAME", "DRIVER", "MOUNTPOINT", "CREATED");
                    println!("{}", "-".repeat(90));
                    for vol in list {
                        println!(
                            "{:<20} | {:<10} | {:<30} | {:<20}",
                            vol.name,
                            vol.driver,
                            vol.mountpoint.to_string_lossy(),
                            vol.created_at.to_rfc3339()
                        );
                    }
                }
                VolumeSubcommand::Inspect { name } => {
                    match driver.inspect(&name).await {
                        Ok(vol) => {
                            println!("{}", serde_json::to_string_pretty(&vol)?);
                        }
                        Err(e) => eprintln!("Error: {}", e),
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
                    // Remove stopped containers
                    let mut removed_containers = 0u64;
                    let mut reclaimed_bytes = 0u64;
                    let containers_dir = data_dir.join("containers");
                    if containers_dir.exists() {
                        let mut entries = tokio::fs::read_dir(&containers_dir).await?;
                        while let Some(entry) = entries.next_entry().await? {
                            let json_path = entry.path().join("container.json");
                            if json_path.exists() {
                                if let Ok(content) = tokio::fs::read_to_string(&json_path).await {
                                    if let Ok(c) = serde_json::from_str::<Container>(&content) {
                                        let can_remove = c.status == ContainerStatus::Stopped || all;
                                        if can_remove {
                                            if let Ok(dir_size) = dir_size_bytes(entry.path()).await {
                                                reclaimed_bytes += dir_size;
                                            }
                                            tokio::fs::remove_dir_all(entry.path()).await.ok();
                                            removed_containers += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    println!("  Removed {} stopped container(s)", removed_containers);
                    // Remove dangling images (images with no running container referencing them)
                    let mut removed_images = 0u64;
                    if all {
                        if let Ok(images) = store.list_images().await {
                            for img in images {
                                if store.delete_image(&img.tag).await.is_ok() {
                                    removed_images += 1;
                                }
                            }
                        }
                    }
                    println!("  Removed {} dangling image(s)", removed_images);
                    // Remove anonymous volumes
                    let driver = LocalDriver::new(data_dir.clone());
                    let mut removed_vols = 0;
                    if let Ok(vols) = driver.list().await {
                        for vol in vols {
                            if vol.labels.contains_key("anonymous") {
                                if driver.remove(&vol.name).await.is_ok() {
                                    removed_vols += 1;
                                }
                            }
                        }
                    }
                    println!("  Removed {} unused volume(s)", removed_vols);
                    println!("  Reclaimed {:.1} MB", reclaimed_bytes as f64 / 1_048_576.0);
                }
                SystemSubcommand::Info => {
                    println!("Crush Container Runtime v0.1.0");
                    println!("OS: {}", std::env::consts::OS);
                    println!("Arch: {}", std::env::consts::ARCH);
                    println!("Data dir: {:?}", data_dir);

                    let mut running_count = 0;
                    let mut stopped_count = 0;
                    let containers_dir = data_dir.join("containers");
                    if containers_dir.exists() {
                        if let Ok(entries) = std::fs::read_dir(&containers_dir) {
                            for entry in entries.flatten() {
                                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                                    let json_path = entry.path().join("container.json");
                                    if json_path.exists() {
                                        if let Ok(content) = std::fs::read_to_string(&json_path) {
                                            if let Ok(c) = serde_json::from_str::<Container>(&content) {
                                                let mut is_alive = false;
                                                if let Some(pid) = c.pid {
                                                    #[cfg(unix)]
                                                    {
                                                        is_alive = unsafe { libc::kill(pid as libc::pid_t, 0) == 0 };
                                                    }
                                                    #[cfg(windows)]
                                                    {
                                                        use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
                                                        use windows_sys::Win32::Foundation::CloseHandle;
                                                        use windows_sys::Win32::System::Threading::GetExitCodeProcess;
                                                        const STILL_ACTIVE: u32 = 259;
                                                        unsafe {
                                                            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
                                                            if handle != 0 {
                                                                let mut exit_code: u32 = 0;
                                                                GetExitCodeProcess(handle, &mut exit_code);
                                                                is_alive = exit_code == STILL_ACTIVE;
                                                                CloseHandle(handle);
                                                            }
                                                        }
                                                    }
                                                }
                                                if is_alive && c.status == ContainerStatus::Running {
                                                    running_count += 1;
                                                } else {
                                                    stopped_count += 1;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    println!("Containers: {} running, {} stopped", running_count, stopped_count);

                    let image_count = store.list_images().await.map(|list| list.len()).unwrap_or(0);
                    println!("Images: {}", image_count);

                    let driver = LocalDriver::new(data_dir.clone());
                    let volume_count = driver.list().await.map(|list| list.len()).unwrap_or(0);
                    println!("Volumes: {}", volume_count);
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
            let api_url = "https://api.github.com/repos/Chidi09/crush/releases/latest";
            let client = reqwest::Client::builder().user_agent("crush/0.1.0").build()?;
            let resp: serde_json::Value = client.get(api_url).send().await?.json().await?;

            let latest = resp["tag_name"].as_str().unwrap_or("unknown").trim_start_matches('v');
            let current = env!("CARGO_PKG_VERSION");

            println!("Current: v{}  Latest: v{}", current, latest);

            if latest == current {
                println!("Already up to date.");
                return Ok(());
            }
            if args.check_only { return Ok(()); }

            let target_name = if cfg!(target_os = "windows") {
                format!("crush-{}-windows-x86_64.exe", latest)
            } else {
                format!("crush-{}-linux-x86_64", latest)
            };

            let asset_url = resp["assets"].as_array()
                .and_then(|a| a.iter().find(|a| a["name"].as_str() == Some(&target_name)))
                .and_then(|a| a["browser_download_url"].as_str())
                .ok_or_else(|| anyhow::anyhow!("No release asset found for this platform: {}", target_name))?
                .to_string();

            println!("Downloading {}...", target_name);
            let bytes = client.get(&asset_url).send().await?.bytes().await?;

            let current_exe = std::env::current_exe()?;
            #[cfg(target_os = "windows")]
            {
                // Always install to %LOCALAPPDATA%\crush\bin\crush.exe — this directory
                // is guaranteed writable without admin, and is the canonical install path.
                // Overwriting current_exe directly hits "Access Denied" (os error 5) when
                // the binary lives in a system or admin-owned directory.
                let local_app_data = std::env::var("LOCALAPPDATA")
                    .unwrap_or_else(|_| format!("{}\\AppData\\Local",
                        std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Default".to_string())));
                let install_dir = std::path::PathBuf::from(&local_app_data).join("crush").join("bin");
                let install_path = install_dir.join("crush.exe");
                std::fs::create_dir_all(&install_dir)?;

                tokio::fs::write(&install_path, &bytes).await
                    .map_err(|e| anyhow::anyhow!(
                        "Failed to write crush.exe to {}: {}\n\
                         Try running this terminal as Administrator, or manually download from:\n\
                         https://github.com/Chidi09/crush/releases/latest",
                        install_path.display(), e
                    ))?;

                // Ensure %LOCALAPPDATA%\crush\bin is on PATH (idempotent)
                let _ = cmd_install().await;

                println!("Updated to v{}.", latest);
                if current_exe != install_path {
                    println!("Installed to: {}", install_path.display());
                    println!("If you ran crush from another location, that copy is unchanged.");
                }
                println!("Open a new terminal to use the updated version.");
                return Ok(());
            }
            #[cfg(not(target_os = "windows"))]
            {
                let tmp_path = current_exe.with_extension("tmp");
                tokio::fs::write(&tmp_path, &bytes).await?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o755))?;
                }
                std::fs::rename(&tmp_path, &current_exe)?;
            }

            // After successful download and replace, re-run install to update PATH entry
            // (idempotent — install_windows/install_unix handles existing PATH correctly)
            if let Err(e) = cmd_install().await {
                eprintln!("Warning: self-update succeeded but PATH install failed: {}", e);
            }

            println!("Updated to v{}. Restart crush to use the new version.", latest);
        }
        Commands::Daemon(args) => {
            let socket_path = PathBuf::from(&args.socket);
            
            // Initialize the native or stateless engine backend
            #[cfg(target_os = "windows")]
            let backend: Arc<dyn crush_types::RuntimeBackend> = Arc::new(WindowsRuntime::new());
            #[cfg(target_os = "linux")]
            let backend: Arc<dyn crush_types::RuntimeBackend> = Arc::new(crush_runtime_linux::LinuxRuntime::new());
            #[cfg(all(not(target_os = "linux"), not(target_os = "windows")))]
            let backend: Arc<dyn crush_types::RuntimeBackend> = Arc::new(StatelessEngine::new(data_dir.clone()));

            // Compat API server
            info!("Starting Docker compatibility daemon on socket: {}", args.socket);
            let compat_server = crush_compat::DockerApiServer::new(socket_path.clone(), data_dir.clone(), backend.clone());
            compat_server.start().await?;

            // Standalone API server
            let api_socket_path = socket_path.parent().unwrap_or(&socket_path).join("crush-api.sock");
            info!("Starting Standalone API daemon on socket: {}", api_socket_path.display());
            let api_server = crush_api::ApiServer::new(api_socket_path.clone(), data_dir.clone(), backend.clone());
            #[cfg(unix)]
            api_server.serve_unix_socket().await?;
            #[cfg(windows)]
            api_server.serve_named_pipe().await?;

            println!("Docker compatibility socket running at: {}", args.socket);
            println!("Standalone API socket running at: {}", api_socket_path.display());
            println!("Press Ctrl+C to stop.");
            tokio::signal::ctrl_c().await?;
            
            compat_server.stop().await?;
            api_server.stop().await?;
            println!("Daemon stopped.");
        }
        Commands::Install => {
            cmd_install().await?;
        }
        Commands::Health(args) => {
            info!("Running health check on container: {}", args.id);
            let mut container_found = false;
            let containers_dir = data_dir.join("containers");
            if containers_dir.exists() {
                let mut entries = tokio::fs::read_dir(&containers_dir).await?;
                while let Some(entry) = entries.next_entry().await? {
                    let json_path = entry.path().join("container.json");
                    if json_path.exists() {
                        if let Ok(content) = tokio::fs::read_to_string(&json_path).await {
                            if let Ok(mut c) = serde_json::from_str::<Container>(&content) {
                                if c.id == args.id || c.name == args.id {
                                    container_found = true;
                                    let health_cmd = match &c.health_cmd {
                                        Some(cmd) => cmd.clone(),
                                        None => {
                                            println!("No health check configured for container '{}'", args.id);
                                            std::process::exit(0);
                                        }
                                    };
                                    
                                    let timeout = c.health_timeout.unwrap_or(args.timeout);
                                    println!("Running health check command: {}", health_cmd);
                                    
                                    #[cfg(target_os = "windows")]
                                    let mut p = std::process::Command::new("cmd");
                                    #[cfg(target_os = "windows")]
                                    p.args(["/C", &health_cmd]);
                                    
                                    #[cfg(not(target_os = "windows"))]
                                    let mut p = std::process::Command::new("sh");
                                    #[cfg(not(target_os = "windows"))]
                                    p.args(["-c", &health_cmd]);

                                    let handle = tokio::task::spawn_blocking(move || {
                                        p.status()
                                    });
                                    
                                    let result = tokio::time::timeout(tokio::time::Duration::from_secs(timeout), handle).await;
                                    
                                    let status = match result {
                                        Ok(Ok(Ok(status))) if status.success() => {
                                            println!("Status: healthy");
                                            HealthStatus::Healthy
                                        }
                                        Ok(Ok(Ok(status))) => {
                                            println!("Status: unhealthy (exit code {:?})", status.code());
                                            HealthStatus::Unhealthy
                                        }
                                        _ => {
                                            println!("Status: unhealthy (timeout or spawn failed)");
                                            HealthStatus::Unhealthy
                                        }
                                    };
                                    
                                    c.health = Some(status);
                                    if let Ok(serialized) = serde_json::to_string_pretty(&c) {
                                        let _ = tokio::fs::write(&json_path, serialized).await;
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            if !container_found {
                eprintln!("Error: container '{}' not found", args.id);
            }
        }
        Commands::DockerContext(args) => {
            #[cfg(target_os = "windows")]
            let default_socket = "npipe:////./pipe/crush_engine".to_string();
            #[cfg(not(target_os = "windows"))]
            let default_socket = "unix:///var/run/crush.sock".to_string();

            let socket = args.socket.unwrap_or(default_socket);

            println!("Crush Docker compatibility daemon socket: {}", socket);
            println!();
            println!("Option 1 — Set DOCKER_HOST for this shell session:");
            #[cfg(target_os = "windows")]
            println!("  $env:DOCKER_HOST = \"{}\"", socket);
            #[cfg(not(target_os = "windows"))]
            println!("  export DOCKER_HOST=\"{}\"", socket);
            println!();
            println!("Option 2 — Create a permanent Docker context (requires docker CLI):");
            println!("  docker context create crush --docker \"host={}\"", socket);
            println!("  docker context use crush");
            println!();
            println!("Then start the daemon:");
            #[cfg(target_os = "windows")]
            println!("  crush daemon --socket \\\\.\\pipe\\crush_engine");
            #[cfg(not(target_os = "windows"))]
            println!("  crush daemon --socket /var/run/crush.sock");
            println!();

            if args.create {
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(b"crush");
                let hash = format!("{:x}", hasher.finalize());

                let home_dir = std::env::var("HOME")
                    .map(PathBuf::from)
                    .or_else(|_| std::env::var("USERPROFILE").map(PathBuf::from))
                    .unwrap_or_else(|_| PathBuf::from("."));

                let context_dir = home_dir.join(".docker").join("contexts").join("meta").join(&hash);
                if let Err(e) = std::fs::create_dir_all(&context_dir) {
                    eprintln!("Failed to create context directory: {}", e);
                } else {
                    let meta_json_path = context_dir.join("meta.json");
                    let host_url = if cfg!(target_os = "windows") {
                        "npipe:////./pipe/crush-api".to_string()
                    } else {
                        "unix:///var/run/crush.sock".to_string()
                    };
                    let meta_json = serde_json::json!({
                        "Name": "crush",
                        "Metadata": {},
                        "Endpoints": {
                            "docker": {
                                "Host": host_url,
                                "SkipTLSVerify": false
                            }
                        }
                    });
                    if let Ok(serialized) = serde_json::to_string_pretty(&meta_json) {
                        if std::fs::write(&meta_json_path, serialized).is_ok() {
                            println!("Docker context 'crush' created successfully at: {}", meta_json_path.display());
                            println!("Run: docker context use crush");
                        } else {
                            eprintln!("Failed to write meta.json");
                        }
                    }
                }
            }
        }
        Commands::Completions(args) => {
            use clap::CommandFactory;
            let mut cmd = Cli::command();
            let completions = crush_tui::generate_completions(args.shell, &mut cmd);
            print!("{}", completions);
        }
        Commands::History(args) => {
            let mut history = crush_build::run::read_build_history(&data_dir);
            history.reverse(); // newest first
            history.truncate(args.limit);

            if args.format == "json" {
                println!("{}", serde_json::to_string_pretty(&history)?);
                return Ok(());
            }
            if history.is_empty() {
                println!("No build history yet — run `crush` in a project to record one.");
            } else {
                println!("{:<24} {:<22} {:<10} {:<10}", "PROJECT", "STACK", "TIME", "AGE");
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                for r in &history {
                    let age_s = now_ms.saturating_sub(r.timestamp_ms) / 1000;
                    let age = if age_s < 60 { format!("{}s ago", age_s) }
                        else if age_s < 3600 { format!("{}m ago", age_s / 60) }
                        else if age_s < 86400 { format!("{}h ago", age_s / 3600) }
                        else { format!("{}d ago", age_s / 86400) };
                    let mark = if r.was_cached { "● cached" } else { "○ fresh " };
                    println!("{:<24} {:<22} {:<10} {} {}",
                        r.project_name.chars().take(24).collect::<String>(),
                        r.language.chars().take(22).collect::<String>(),
                        format!("{:.1}s", r.duration_ms as f64 / 1000.0),
                        age,
                        mark);
                }
            }
        }
        Commands::InternalRun(_) => unreachable!("internal-run is dispatched before the image store is opened"),
        Commands::Deploy(args) => {
            use crush_deploy::DeploymentState;
            use crush_build::parser::CrushfileParser;

            let root = std::env::current_dir()?;
            let crushfile_path = root.join("Crushfile");
            let crushfile = CrushfileParser::parse(&crushfile_path)
                .map_err(|e| anyhow::anyhow!("Failed to parse Crushfile: {}", e))?;

            let deploy_config = crushfile.deploy.as_ref()
                .ok_or_else(|| anyhow::anyhow!(
                    "No [deploy] section in Crushfile.\n\
                     Add one like:\n\n  \
                     [deploy]\n  provider = \"hetzner\"\n\n  \
                     [deploy.hetzner]\n  api_token = \"${{HETZNER_API_TOKEN}}\"\n"
                ))?;

            let project = crushfile.project.as_ref()
                .and_then(|p| p.name.clone())
                .unwrap_or_else(|| root.file_name().unwrap_or_default().to_string_lossy().to_string());

            let provider_name = args.provider.as_deref()
                .unwrap_or(&deploy_config.provider)
                .to_string();

            let state = DeploymentState::new();

            if args.status {
                if let Some(info) = state.load(&project) {
                    let provider = build_provider(&provider_name, deploy_config)?;
                    let status = provider.status(&info).await?;
                    println!("Project:  {}", info.project);
                    println!("Provider: {}", info.provider);
                    println!("Server:   {} ({})", info.server_id, info.public_ip);
                    println!("Deployed: {}", info.deployed_at);
                    println!("Status:   {:?}", status);
                    if let Some(ref domain) = info.domain {
                        println!("Domain:   {}", domain);
                    } else {
                        println!("URL:      http://{}:{}", info.public_ip, info.port);
                    }
                } else {
                    println!("No deployment found for '{}'", project);
                }
                return Ok(());
            }

            if args.stop {
                if let Some(info) = state.load(&project) {
                    if info.provider == "ssh" {
                        let ops = crush_deploy::SshProvider::new(&info.public_ip, 22, "root", None);
                        println!("{} Stopping container for project {}...", "■".cyan(), project);
                        if let Ok(sess) = ops.connect() {
                            let (out, err, _code) = ops.exec(&sess, &format!("docker stop {}", project))?;
                            println!("{}{}", out, err);
                        } else {
                            anyhow::bail!("Failed to connect to {}", info.public_ip);
                        }
                        return Ok(());
                    }
                    anyhow::bail!("--stop is currently only implemented for SSH provider");
                }
                anyhow::bail!("No deployment found for '{}'", project);
            }

    if args.destroy {
                if let Some(info) = state.load(&project) {
                    let provider = build_provider(&provider_name, deploy_config)?;
                    println!("Destroying deployment for '{}'...", project);
                    provider.destroy(&info).await?;
                    state.remove(&project);
                    println!("Deployment destroyed.");
                } else {
                    println!("No deployment found for '{}'", project);
                }
                return Ok(());
            }

            // `--logs` is a standalone action: stream logs of the *existing*
            // deployment (no rebuild/redeploy). With `--follow`, poll the
            // provider and print only newly-appended output.
            if args.logs {
                let Some(info) = state.load(&project) else {
                    println!("No deployment found for '{}'", project);
                    return Ok(());
                };
                let provider = build_provider(&provider_name, deploy_config)?;
                if args.follow {
                    use std::io::Write;
                    println!("Streaming logs for '{}' on {} (Ctrl-C to stop)\n", project, info.provider);
                    let mut last = String::new();
                    loop {
                        match provider.logs(&info, args.lines).await {
                            Ok(out) => {
                                // Append-only tail: print just the new suffix; if the
                                // window rolled (no longer a prefix), reprint in full.
                                if !last.is_empty() && out.starts_with(&last) {
                                    print!("{}", &out[last.len()..]);
                                } else {
                                    print!("{out}");
                                }
                                std::io::stdout().flush().ok();
                                last = out;
                            }
                            Err(e) => eprintln!("  log fetch error: {e}"),
                        }
                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    }
                } else {
                    let out = provider.logs(&info, args.lines).await?;
                    print!("{out}");
                }
                return Ok(());
            }

            // Build the project image
            println!("[1/4] Building image...");
            let detector = crush_build::StackDetector::new();
            let stack = detector.detect(&root).await
                .map_err(|e| anyhow::anyhow!("Stack detection failed: {}", e))?;
            println!("  Detected: {} (confidence {:.0}%)", stack.language, stack.confidence * 100.0);
            let cache_dir = data_dir.join("cache");
            let engine = crush_build::BuildEngine::new(cache_dir);
            let outcome = engine.execute_layered_build(&root, &stack).await
                .map_err(|e| anyhow::anyhow!("Build failed: {}", e))?;
            let digest = outcome.digest.clone();
            println!("  Build complete: {}", &digest[..12]);

            // Export to a tarball
            println!("[2/4] Exporting OCI tarball...");
            let tar_path = std::env::temp_dir().join(format!("{}-deploy.tar", project));
            store.export_oci_tarball(&digest, &tar_path).await
                .map_err(|e| anyhow::anyhow!("Export failed: {}", e))?;

            // Zero-downtime path: drive a blue-green rollout over SSH (crush
            // controls the host). Cloud PaaS providers do their own rollout, so
            // for those we note it and fall through to the standard deploy.
            if args.strategy == "blue-green" {
                if provider_name == "ssh" {
                    use crush_deploy::bluegreen::{self, Outcome};
                    use crush_deploy::ssh::SshBlueGreen;
                    use crush_deploy::SshProvider;

                    let s = deploy_config.ssh.as_ref()
                        .ok_or_else(|| anyhow::anyhow!("Missing [deploy.ssh] section"))?;
                    let public_port = stack.default_port;
                    println!("[3/3] Blue-green rollout to {} (public :{})...", s.host, public_port);

                    let ssh = SshProvider::new(
                        &s.host,
                        s.port.unwrap_or(22),
                        s.user.as_deref().unwrap_or("root"),
                        s.key.as_deref(),
                    );
                    let bg = SshBlueGreen::new(ssh, &project, public_port);
                    let env = deploy_config.env.clone().unwrap_or_default();
                    let opts = bluegreen::Options { health_path: args.health_path.clone(), ..Default::default() };

                    println!("  {} bringing up the idle color & health-checking before any traffic flips\u{2026}", "↳".cyan());
                    let Outcome { new_color, retired, .. } =
                        bluegreen::execute(&bg, &project, public_port, &tar_path, &env, &opts).await?;

                    println!("\n{} live on {} {}",
                        "✓".green().bold(),
                        new_color.as_str().bold(),
                        format!("(zero-downtime — gateway flipped on :{})", public_port).dimmed());
                    if let Some(r) = retired {
                        println!("   {} drained & retired {}", "↳".cyan(), r);
                    }

                    let info = crush_deploy::DeploymentInfo {
                        provider: "ssh".to_string(),
                        project: project.clone(),
                        server_id: s.host.clone(),
                        public_ip: s.host.clone(),
                        region: "custom".to_string(),
                        deployed_at: chrono::Utc::now().to_rfc3339(),
                        image_digest: digest.clone(),
                        port: public_port,
                        domain: deploy_config.domain.clone(),
                        status: crush_deploy::DeployStatus::Running,
                    };
                    state.save(&info)?;
                    let _ = std::fs::remove_file(&tar_path);
                    println!("\n  URL: http://{}:{}", s.host, public_port);
                    println!("  tail logs with: {}", "crush deploy --logs -f".dimmed());
                    return Ok(());
                }
                println!("  {} --strategy blue-green is implemented for the `ssh` provider; {} manages its own rollout — doing a standard deploy.",
                    "⚠".yellow(), provider_name.bold());
            }

            // Provision infra
            println!("[3/4] Provisioning {}...", provider_name);
            let provider = build_provider(&provider_name, deploy_config)?;
            let region = deploy_config.region.as_deref().unwrap_or("");
            let size = deploy_config.server_type.as_deref().unwrap_or("");
            let mut info = provider.provision(&project, region, size).await?;
            info.image_digest = digest.clone();
            info.port = stack.default_port;

            // Deploy
            println!("[4/4] Deploying to {}...", info.public_ip);
            let env = deploy_config.env.clone().unwrap_or_default();
            provider.deploy(&info, &tar_path, stack.default_port, &env).await?;
            info.status = crush_deploy::DeployStatus::Running;

            state.save(&info)?;

            println!("\nDeployed successfully!");
            println!("  URL: http://{}:{}", info.public_ip, info.port);
            if let Some(ref domain) = deploy_config.domain {
                println!("  Domain: {} (point DNS A record to {})", domain, info.public_ip);
                info.domain = Some(domain.clone());
                state.save(&info)?;
            }

            let _ = std::fs::remove_file(&tar_path);

            println!("\n  tail logs with: {}", format!("crush deploy --logs -f").dimmed());
        }
        Commands::Services(args) => {
            let project_root = std::env::current_dir()?;
            let project_name = project_root.file_name()
                .map(|n| n.to_string_lossy().to_lowercase().replace([' ', '-'], "_"))
                .unwrap_or_else(|| "app".into());
            let state_dir = data_dir.join("services");

            match args.cmd.unwrap_or(ServicesSubcommand::Ps) {
                ServicesSubcommand::Ps | ServicesSubcommand::Status => {
                    // --format json: collect all matching states and emit
                    // one JSON document. Consumed by the GUI's services screen.
                    if args.format == "json" {
                        #[derive(serde::Serialize)]
                        struct ServiceListing {
                            project: String,
                            native: Option<crush_services::NativeServiceState>,
                            containers: Option<ServiceState>,
                        }
                        let mut listings: Vec<ServiceListing> = Vec::new();
                        if args.all_projects {
                            let native_dir = state_dir.join("native");
                            if let Ok(entries) = std::fs::read_dir(&native_dir) {
                                for e in entries.flatten() {
                                    let p = e.path();
                                    if p.extension().and_then(|s| s.to_str()) != Some("json") { continue; }
                                    let pname = p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                                    let native = load_native_state(&state_dir, &pname);
                                    let containers = load_service_state(&state_dir, &pname);
                                    if native.is_some() || containers.is_some() {
                                        listings.push(ServiceListing { project: pname, native, containers });
                                    }
                                }
                            }
                        } else {
                            let native = load_native_state(&state_dir, &project_name);
                            let containers = load_service_state(&state_dir, &project_name);
                            if native.is_some() || containers.is_some() {
                                listings.push(ServiceListing {
                                    project: project_name.clone(),
                                    native,
                                    containers,
                                });
                            }
                        }
                        println!("{}", serde_json::to_string_pretty(&listings)?);
                        return Ok(());
                    }

                    let mut found = false;
                    if let Some(state) = load_service_state(&state_dir, &project_name) {
                        println!("Container services for {} (backend: {}):", project_name, state.backend);
                        for c in &state.containers {
                            let ports: Vec<String> = c.ports.iter()
                                .map(|(h, _)| format!("localhost:{}", h)).collect();
                            println!("  {} -> {} ({})", c.service_name, c.container_name,
                                if ports.is_empty() { "no ports".into() } else { ports.join(", ") });
                        }
                        found = true;
                    }
                    if let Some(state) = load_native_state(&state_dir, &project_name) {
                        println!("NAME       KIND      PORT     UPTIME    PID");
                        for s in &state.services {
                            let kind_str = match s.kind {
                                crush_services::ServiceKind::Postgres => "native     ",
                                crush_services::ServiceKind::RedisCompat => {
                                    #[cfg(target_os = "windows")]
                                    { "garnet    " }
                                    #[cfg(not(target_os = "windows"))]
                                    { "native    " }
                                }
                                crush_services::ServiceKind::MongoDB => "mongodb    ",
                                crush_services::ServiceKind::ObjectStore => "minio      ",
                                crush_services::ServiceKind::MySQL => "native     ",
                            };
                            let uptime = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs()
                                .saturating_sub(state.started_at);
                            let uptime_str = if uptime < 60 {
                                format!("{}s", uptime)
                            } else if uptime < 3600 {
                                format!("{}m {}s", uptime / 60, uptime % 60)
                            } else {
                                format!("{}h {}m", uptime / 3600, (uptime % 3600) / 60)
                            };
                            println!("{:<10} {:<10} {:<8} {:<9} {}",
                                s.name, kind_str, s.port, uptime_str, s.pid);
                        }
                        found = true;
                    }
                    if !found {
                        println!("No running crush-managed services for {}.", project_name);
                    }
                }
                ServicesSubcommand::Logs { name, follow } => {
                    let log_dir = data_dir.join("services").join("logs").join(&project_name);
                    let log_path = log_dir.join(format!("{}.log", name));

                    if !log_path.exists() {
                        eprintln!("No logs found for '{}'. Services started before v0.7.45 don't have log capture.", name);
                        return Ok(());
                    }

                    let content = std::fs::read_to_string(&log_path)
                        .unwrap_or_default();
                    let lines: Vec<&str> = content.lines().collect();
                    let start = if lines.len() > 50 { lines.len() - 50 } else { 0 };
                    for line in &lines[start..] {
                        println!("{}", line);
                    }

                    if follow {
                        use tokio::io::AsyncBufReadExt;
                        let mut file = tokio::fs::File::open(&log_path).await?;
                        let mut reader = tokio::io::BufReader::new(&mut file);
                        let mut line = String::new();
                        loop {
                            line.clear();
                            match reader.read_line(&mut line).await {
                                Ok(0) => {
                                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                }
                                Ok(_) => {
                                    print!("{}", line);
                                    std::io::stdout().flush().ok();
                                }
                                Err(_) => break,
                            }
                        }
                    }
                }
                ServicesSubcommand::Stop { name } => {
                    let backend = detect_backend();
                    let mut stopped_any = false;

                    if let Some(state) = load_service_state(&state_dir, &project_name) {
                        for c in &state.containers {
                            if let Some(ref n) = name {
                                if &c.service_name != n { continue; }
                            }
                            print!("   ↳ stopping container {}... ", c.service_name);
                            use std::io::Write;
                            let _ = std::io::stdout().flush();
                            match stop_dep_service(&backend, &c.container_name).await {
                                Ok(_) => println!("done"),
                                Err(e) => println!("error: {}", e),
                            }
                            stopped_any = true;
                        }
                        if name.is_none() {
                            clear_service_state(&state_dir, &project_name);
                        }
                    }

                    if let Some(state) = load_native_state(&state_dir, &project_name) {
                        let cache_dir = data_dir.join("cache");
                        let remaining: Vec<_> = state.services.iter().filter(|s| {
                            if let Some(ref n) = name {
                                &s.name == n
                            } else {
                                true
                            }
                        }).collect();
                        for s in &remaining {
                            print!("   ↳ stopping native {}... ", s.name);
                            use std::io::Write;
                            let _ = std::io::stdout().flush();

                            let driver: Box<dyn crush_services::ServiceDriver> = match s.kind {
                                crush_services::ServiceKind::Postgres => Box::new(crush_services::PostgresDriver::new(cache_dir.clone())),
                                crush_services::ServiceKind::RedisCompat => Box::new(crush_services::RedisCompatDriver::new(cache_dir.clone())),
                                crush_services::ServiceKind::MongoDB => Box::new(crush_services::MongoDriver::new(cache_dir.clone())),
                                crush_services::ServiceKind::ObjectStore => Box::new(crush_services::MinioDriver::new(cache_dir.clone())),
                                crush_services::ServiceKind::MySQL => Box::new(crush_services::MysqlDriver::new()),
                            };
                            let stop_res = driver.stop(s).await;

                            match stop_res {
                                Ok(_) => println!("done"),
                                Err(e) => println!("error: {}", e),
                            }
                            stopped_any = true;
                        }
                        if name.is_none() {
                            clear_native_state(&state_dir, &project_name);
                        }
                    }

                    if stopped_any {
                        if let Some(ref n) = name {
                            println!(" ✓ stopped {}", n);
                        } else {
                            println!(" ✓ all services stopped");
                        }
                    } else {
                        println!("No running crush-managed services for {}.", project_name);
                    }
                }
                ServicesSubcommand::Restart { name } => {
                    let cache_dir = data_dir.join("cache");
                    let backend = detect_backend();

                    // Stop
                    if let Some(state) = load_service_state(&state_dir, &project_name) {
                        for c in &state.containers {
                            if let Some(ref n) = name {
                                if &c.service_name != n { continue; }
                            }
                            let _ = stop_dep_service(&backend, &c.container_name).await;
                        }
                    }
                    if let Some(state) = load_native_state(&state_dir, &project_name) {
                        for s in &state.services {
                            if let Some(ref n) = name {
                                if &s.name != n { continue; }
                            }
                            let driver: Box<dyn crush_services::ServiceDriver> = match s.kind {
                                crush_services::ServiceKind::Postgres => Box::new(crush_services::PostgresDriver::new(cache_dir.clone())),
                                crush_services::ServiceKind::RedisCompat => Box::new(crush_services::RedisCompatDriver::new(cache_dir.clone())),
                                crush_services::ServiceKind::MongoDB => Box::new(crush_services::MongoDriver::new(cache_dir.clone())),
                                crush_services::ServiceKind::ObjectStore => Box::new(crush_services::MinioDriver::new(cache_dir.clone())),
                                crush_services::ServiceKind::MySQL => Box::new(crush_services::MysqlDriver::new()),
                            };
                            let _ = driver.stop(s).await;
                        }
                    }

                    // Re-start: re-run compose parsing and start
                    let compose_files = ["docker-compose.yml", "docker-compose.yaml", "compose.yml", "compose.yaml"];
                    let compose_dirs = [".", "infra", "docker", ".docker", "deploy", "ops", "devops"];
                    let mut compose_path: Option<std::path::PathBuf> = None;
                    for d in &compose_dirs {
                        for f in &compose_files {
                            let candidate = project_root.join(d).join(f);
                            if candidate.exists() { compose_path = Some(candidate); break; }
                        }
                        if compose_path.is_some() { break; }
                    }

                    if let Some(ref cp) = compose_path {
                        if let Ok(parsed) = parse_compose(cp) {
                            let to_start: Vec<_> = parsed.dep_services.iter()
                                .filter(|dep| name.as_deref().map_or(true, |n| dep.name == n))
                                .collect();
                            for dep in &to_start {
                                print!("   ↳ restarting {} ({})... ", dep.name, dep.image);
                                use std::io::Write;
                                let _ = std::io::stdout().flush();
                                match start_dep_service_smart(dep, &project_name, &data_dir).await {
                                    Ok(StartedService::Native(r)) => {
                                        let note = match r.kind {
                                            crush_services::ServiceKind::Postgres => "[native]",
                                            crush_services::ServiceKind::RedisCompat => "[garnet]",
                                            crush_services::ServiceKind::MongoDB => "[mongodb]",
                                            crush_services::ServiceKind::ObjectStore => "[minio]",
                                            crush_services::ServiceKind::MySQL => "[mysql]",
                                        };
                                        println!("ok  {}", note);
                                    }
                                    Ok(StartedService::Container(c)) => println!("ok  [container]"),
                                    Err(e) => println!("failed: {}", e),
                                }
                            }
                        }
                    }
                }
            }
        }
        Commands::__Complete(args) => {
            match args.category.as_str() {
                "containers" => {
                    let containers_dir = data_dir.join("containers");
                    if containers_dir.exists() {
                        if let Ok(mut entries) = tokio::fs::read_dir(&containers_dir).await {
                            while let Ok(Some(entry)) = entries.next_entry().await {
                                let json_path = entry.path().join("container.json");
                                if json_path.exists() {
                                    if let Ok(content) = tokio::fs::read_to_string(&json_path).await {
                                        if let Ok(c) = serde_json::from_str::<Container>(&content) {
                                            println!("{}", c.id);
                                            println!("{}", c.name);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                "images" => {
                    if let Ok(images) = store.list_images().await {
                        for img in images {
                            println!("{}", img.tag);
                        }
                    }
                }
                "volumes" => {
                    let driver = LocalDriver::new(data_dir.clone());
                    if let Ok(list) = driver.list().await {
                        for vol in list {
                            println!("{}", vol.name);
                        }
                    }
                }
                "networks" => {
                    println!("crush_nat");
                    #[cfg(target_os = "linux")]
                    {
                        let net = NetworkManager::new(data_dir.join("networks"));
                        if let Ok(list) = net.list().await {
                            for n in list {
                                println!("{}", n.name);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    };

    Ok(())
}

fn build_provider(
    name: &str,
    config: &crush_build::parser::CrushfileDeploy,
) -> anyhow::Result<Box<dyn crush_deploy::DeployProvider>> {
    match name {
        "hetzner" => {
            let h = config.hetzner.as_ref()
                .ok_or_else(|| anyhow::anyhow!("Missing [deploy.hetzner] section"))?;
            Ok(Box::new(crush_deploy::HetznerProvider::new(
                &h.api_token,
                h.ssh_key_name.as_deref(),
            )))
        }
        "ssh" => {
            let s = config.ssh.as_ref()
                .ok_or_else(|| anyhow::anyhow!("Missing [deploy.ssh] section"))?;
            Ok(Box::new(crush_deploy::SshProvider::new(
                &s.host,
                s.port.unwrap_or(22),
                s.user.as_deref().unwrap_or("root"),
                s.key.as_deref(),
            )))
        }
        "aws" => {
            let a = config.aws.as_ref()
                .ok_or_else(|| anyhow::anyhow!("Missing [deploy.aws] section"))?;
            Ok(Box::new(crush_deploy::AwsProvider::new(a)))
        }
        "gcp" => {
            let g = config.gcp.as_ref()
                .ok_or_else(|| anyhow::anyhow!("Missing [deploy.gcp] section"))?;
            Ok(Box::new(crush_deploy::GcpProvider::new(g)))
        }
        "digitalocean" => {
            let d = config.digitalocean.as_ref()
                .ok_or_else(|| anyhow::anyhow!("Missing [deploy.digitalocean] section"))?;
            Ok(Box::new(crush_deploy::DigitalOceanProvider::new(d)))
        }
        "fly" => {
            let f = config.fly.as_ref()
                .ok_or_else(|| anyhow::anyhow!("Missing [deploy.fly] section"))?;
            Ok(Box::new(crush_deploy::FlyProvider::new(f)))
        }
        "railway" => {
            let r = config.railway.as_ref()
                .ok_or_else(|| anyhow::anyhow!("Missing [deploy.railway] section"))?;
            Ok(Box::new(crush_deploy::RailwayProvider::new(r)))
        }
        other => Err(anyhow::anyhow!(
            "Unknown provider '{}'. Options: hetzner, ssh, aws, gcp, digitalocean, fly, railway", other
        )),
    }
}

pub(crate) fn dirs_or_default() -> PathBuf {
    crush_types::dirs_or_default()
}

pub(crate) fn hex_encode_random() -> String {
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

#[allow(dead_code)]
fn which_docker() -> Option<std::path::PathBuf> {
    let candidates = if cfg!(target_os = "windows") {
        vec!["docker.exe", "docker"]
    } else {
        vec!["docker"]
    };
    for name in candidates {
        if let Ok(path) = which::which(name) {
            return Some(path);
        }
    }
    None
}

pub(crate) async fn copy_project_into_rootfs(src: &Path, dest: &Path) -> std::io::Result<()> {
    let skip = ["target", ".git", "node_modules", ".next", "__pycache__", ".venv", "dist"];
    let mut entries = tokio::fs::read_dir(src).await?;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if skip.iter().any(|&s| s == name_str.as_ref()) { continue; }
        let src_path = entry.path();
        let dst_path = dest.join(&name);
        if src_path.is_dir() {
            tokio::fs::create_dir_all(&dst_path).await?;
            // recurse with Box::pin to avoid infinite type
            Box::pin(copy_project_into_rootfs(&src_path, &dst_path)).await?;
        } else {
            tokio::fs::copy(&src_path, &dst_path).await?;
        }
    }
    Ok(())
}

/// Resolve a container directory by ID or name. Returns the container dir and parsed Container.
async fn resolve_container(data_dir: &Path, id_or_name: &str) -> Option<(PathBuf, Container)> {
    let containers_dir = data_dir.join("containers");
    if !containers_dir.exists() {
        return None;
    }
    let mut entries = tokio::fs::read_dir(&containers_dir).await.ok()?;
    while let Some(entry) = entries.next_entry().await.ok()? {
        let json_path = entry.path().join("container.json");
        if let Ok(content) = tokio::fs::read_to_string(&json_path).await {
            if let Ok(c) = serde_json::from_str::<Container>(&content) {
                // Match full id, name, or a short-id prefix (Docker-style).
                if c.id == id_or_name || c.name == id_or_name || c.id.starts_with(id_or_name) {
                    return Some((entry.path(), c));
                }
            }
        }
    }
    None
}

/// `crush exec` — run a command in a running container's environment.
///
/// Crush's default run model is native: a "container" is a tracked host process with a
/// rootfs dir and a `config.json` capturing its cmd/env. `exec` runs the requested command
/// in that same working directory and environment, inheriting stdio so `-it` shells work.
/// Returns the child's exit code (or a non-zero code on failure to launch).
async fn cmd_exec(args: &ExecArgs, data_dir: &Path) -> i32 {
    let (container_dir, container) = match resolve_container(data_dir, &args.id).await {
        Some(v) => v,
        None => {
            eprintln!("Error: container '{}' not found", args.id);
            return 1;
        }
    };

    if container.status != ContainerStatus::Running {
        eprintln!("Error: container '{}' is not running (status: {:?})", args.id, container.status);
        return 1;
    }

    // Pull the container's captured environment from config.json (written at start).
    let mut env: Vec<(String, String)> = Vec::new();
    if let Ok(cfg) = tokio::fs::read_to_string(container_dir.join("config.json")).await {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&cfg) {
            if let Some(arr) = v.get("env").and_then(|e| e.as_array()) {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        if let Some((k, val)) = s.split_once('=') {
                            env.push((k.to_string(), val.to_string()));
                        }
                    }
                }
            }
        }
    }
    // Caller-supplied `-e KEY=VALUE` overrides win.
    for pair in &args.env {
        if let Some((k, val)) = pair.split_once('=') {
            env.push((k.to_string(), val.to_string()));
        }
    }

    // Working dir: explicit -w, else the container rootfs if present, else the container dir.
    let rootfs = container_dir.join("rootfs");
    let cwd = match &args.workdir {
        Some(w) => PathBuf::from(w),
        None if rootfs.exists() => rootfs,
        None => container_dir.clone(),
    };

    // Default to a platform shell when no command is given.
    let command: Vec<String> = if args.command.is_empty() {
        #[cfg(windows)]
        { vec!["cmd.exe".to_string()] }
        #[cfg(not(windows))]
        { vec!["/bin/sh".to_string()] }
    } else {
        args.command.clone()
    };

    let mut cmd = std::process::Command::new(&command[0]);
    cmd.args(&command[1..]);
    cmd.current_dir(&cwd);
    for (k, v) in &env {
        cmd.env(k, v);
    }
    // Inherit stdio so interactive shells (`-it`) work; this is the default for Command.
    let _ = (args.interactive, args.tty); // stdio is inherited regardless; flags kept for Docker familiarity

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW — don't pop a console
    }

    match cmd.status() {
        Ok(status) => status.code().unwrap_or(0),
        Err(e) => {
            eprintln!("Error: failed to exec '{}' in container '{}': {}", command[0], args.id, e);
            127
        }
    }
}

async fn run_compose_up(compose_path: &Path, data_dir: &Path, store: &ImageStore) -> anyhow::Result<()> {
    use crush_compat::{ComposeParser};

    let parser = ComposeParser::new();
    let compose = parser.parse_path(compose_path)?;
    let order = ComposeParser::get_dependency_order(&compose)?;
    let services = compose.services.unwrap_or_default();
    let compose_dir = compose_path.parent().unwrap_or(Path::new(".")).to_path_buf();

    // ── Named volumes ──────────────────────────────────────────────────────
    // Back each top-level named volume with a persistent dir under the data dir,
    // keyed by compose project so two projects don't collide. Bind mounts (host
    // paths in a service's `volumes:`) are resolved later, per service.
    let project = compose_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let volumes_root = data_dir.join("volumes").join(&project);
    if let Some(named) = compose.volumes.as_ref() {
        for name in named.keys() {
            tokio::fs::create_dir_all(volumes_root.join(name)).await?;
        }
    }

    // ── Network membership ─────────────────────────────────────────────────
    // Compose networks isolate who can talk to whom. In crush's native model all
    // services run on localhost, so "honoring networks" means: a service may
    // resolve a peer by name (→ localhost) only if they share a network. Build
    // service → set(networks) so we can compute each service's reachable peers.
    let svc_networks: std::collections::HashMap<String, std::collections::HashSet<String>> = services
        .iter()
        .map(|(name, svc)| {
            let nets: std::collections::HashSet<String> = svc.networks.clone()
                .unwrap_or_default().into_iter().collect();
            (name.clone(), nets)
        })
        .collect();

    // Persist compose state so `compose down` and `compose ps` can find containers
    let state_path = data_dir.join("compose").join(
        compose_path.file_stem().unwrap_or_default().to_string_lossy().as_ref()
    ).with_extension("json");
    tokio::fs::create_dir_all(state_path.parent().unwrap()).await?;
    let mut compose_state: std::collections::HashMap<String, String> = std::collections::HashMap::new(); // service_name → container_id

    for svc_name in &order {
        let svc = match services.get(svc_name) { Some(s) => s, None => continue };

        // Resolve image
        let image_tag = if let Some(img) = &svc.image {
            // Pull if needed
            if store.database().get_image_by_tag(img).await?.is_none() {
                println!("  Pulling {}...", img);
                store.pull_image(img).await
                    .map_err(|e| CrushError::ImageError(format!("Failed to pull {}: {}", img, e)))?;
            }
            img.clone()
        } else if let Some(build_val) = &svc.build {
            // build: context (string) or build: {context: ..., dockerfile: ...}
            let (ctx, df_name) = if build_val.is_string() {
                (build_val.as_str().unwrap_or(".").to_string(), "Dockerfile".to_string())
            } else {
                let ctx = build_val.get("context").and_then(|v| v.as_str()).unwrap_or(".").to_string();
                let df = build_val.get("dockerfile").and_then(|v| v.as_str()).unwrap_or("Dockerfile").to_string();
                (ctx, df)
            };
            let ctx_path = compose_path.parent().unwrap_or(Path::new(".")).join(&ctx);
            let df_path = ctx_path.join(&df_name);

            let _tag = format!("{}:latest", svc_name);

            // Parse Dockerfile for base image, then fall through to generic container build
            if df_path.exists() {
                let df_parser = crush_compat::DockerfileParserV2::new();
                if let Ok(df) = df_parser.parse_path(&df_path) {
                    if let Some(base) = df.stages.first().and_then(|s| s.base_image.clone()) {
                        if base != "scratch" && store.database().get_image_by_tag(&base).await?.is_none() {
                            println!("  Pulling base {} for service {}...", base, svc_name);
                            let _ = store.pull_image(&base).await;
                        }
                        base
                    } else {
                        "debian:bookworm-slim".to_string()
                    }
                } else {
                    "debian:bookworm-slim".to_string()
                }
            } else {
                println!("  [WARN] Dockerfile not found at {:?}, skipping {}", df_path, svc_name);
                continue;
            }
        } else {
            println!("  [WARN] Service {} has no image or build, skipping", svc_name);
            continue;
        };

        let image = match store.database().get_image_by_tag(&image_tag).await? {
            Some(img) => img,
            None => {
                println!("  [WARN] Image {} not available for service {}, skipping", image_tag, svc_name);
                continue;
            }
        };

        // Resolve env
        let env_vars: Vec<String> = match &svc.environment {
            Some(v) if v.is_array() => v.as_array().unwrap()
                .iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect(),
            Some(v) if v.is_object() => v.as_object().unwrap()
                .iter().map(|(k, val)| format!("{}={}", k, val.as_str().unwrap_or(""))).collect(),
            _ => Vec::new(),
        };

        // Env files
        let mut all_env = env_vars;
        if let Some(env_files) = &svc.env_file {
            for ef in env_files {
                let ef_path = compose_dir.join(ef);
                if let Ok(content) = std::fs::read_to_string(&ef_path) {
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() && !trimmed.starts_with('#') {
                            all_env.push(trimmed.to_string());
                        }
                    }
                }
            }
        }

        // ── Honor networks ──────────────────────────────────────────────
        // A service can reach peers that share at least one network with it.
        // Since native services live on localhost, rewrite those peer names in
        // env values to "localhost" so e.g. `DB_HOST=postgres_db` resolves. Peers
        // on no shared network are intentionally left unrewritten (isolated).
        let my_nets = svc_networks.get(svc_name).cloned().unwrap_or_default();
        let reachable_peers: Vec<String> = svc_networks.iter()
            .filter(|(name, nets)| {
                name.as_str() != svc_name.as_str() && (
                    // shared network, or neither side declared any (compose default network)
                    !nets.is_disjoint(&my_nets) || (nets.is_empty() && my_nets.is_empty())
                )
            })
            .map(|(name, _)| name.clone())
            .collect();
        if !reachable_peers.is_empty() {
            let pairs: Vec<(String, String)> = all_env.iter()
                .filter_map(|e| e.split_once('=').map(|(k, v)| (k.to_string(), v.to_string())))
                .collect();
            all_env = crush_build::rewrite_env_hostnames(&pairs, &reachable_peers)
                .into_iter().map(|(k, v)| format!("{}={}", k, v)).collect();
        }

        // ── Honor volumes ───────────────────────────────────────────────
        // Each `volumes:` entry is `source:target[:ro]`. A source that looks like
        // a path (./ ../ / or X:\) is a bind mount resolved against the compose
        // dir; anything else is a named volume backed under the data dir.
        let mounts: Vec<MountConfig> = svc.volumes.as_ref().map(|vols| {
            vols.iter().filter_map(|entry| {
                parse_compose_volume(entry, &compose_dir, &volumes_root)
            }).collect()
        }).unwrap_or_default();

        // Resolve port mappings
        let ports: Vec<PortMapping> = svc.ports.as_ref().map(|ps| {
            ps.iter().filter_map(|p| {
                let parts: Vec<&str> = p.split(':').collect();
                if parts.len() == 2 {
                    let host_port: u16 = parts[0].parse().ok()?;
                    let container_port: u16 = parts[1].split('/').next()?.parse().ok()?;
                    Some(PortMapping { host_ip: "0.0.0.0".to_string(), host_port, container_port, protocol: Protocol::Tcp })
                } else if parts.len() == 1 {
                    let port: u16 = parts[0].parse().ok()?;
                    Some(PortMapping { host_ip: "0.0.0.0".to_string(), host_port: port, container_port: port, protocol: Protocol::Tcp })
                } else { None }
            }).collect()
        }).unwrap_or_default();

        // Container name
        let container_id = format!("crush_{}", hex_encode_random());
        let container_name = svc.container_name.clone()
            .unwrap_or_else(|| format!("{}_{}", svc_name, &container_id[6..12]));

        // Restart policy
        let restart_policy = svc.restart.clone().unwrap_or_else(|| "no".to_string());

        let container = Container {
            id: container_id.clone(),
            name: container_name.clone(),
            image: image_tag.clone(),
            status: ContainerStatus::Creating,
            pid: None,
            created_at: SystemTime::now(),
            started_at: None,
            ports,
            mounts: mounts.clone(),
            memory_limit_bytes: svc.deploy.as_ref()
                .and_then(|d| d.resources.as_ref())
                .and_then(|r| r.limits.as_ref())
                .and_then(|l| l.memory.as_ref())
                .and_then(|m| parse_memory_bytes(m)),
            // compose `deploy.resources.limits.cpus` (e.g. "0.50") → Docker cpu_shares
            // (1024 == one CPU's relative weight), so 0.50 → 512.
            cpu_shares: svc.deploy.as_ref()
                .and_then(|d| d.resources.as_ref())
                .and_then(|r| r.limits.as_ref())
                .and_then(|l| l.cpus.as_ref())
                .and_then(|c| c.parse::<f64>().ok())
                .map(|c| (c * 1024.0).round() as u64),
            health: None,
            restart_count: Some(0),
            restart_policy: Some(restart_policy),
            health_cmd: svc.healthcheck.as_ref()
                .and_then(|h| h.test.as_ref())
                .and_then(compose_healthcheck_cmd),
            health_interval: svc.healthcheck.as_ref()
                .and_then(|h| h.interval.as_ref())
                .and_then(|s| parse_duration_secs(s))
                .or(Some(30)),
            health_timeout: svc.healthcheck.as_ref()
                .and_then(|h| h.timeout.as_ref())
                .and_then(|s| parse_duration_secs(s))
                .or(Some(30)),
            health_retries: svc.healthcheck.as_ref()
                .and_then(|h| h.retries)
                .or(Some(3)),
            pids_limit: None,
            read_only: Some(false),
            security_opt: Some(vec![]),
        };

        let container_dir = data_dir.join("containers").join(&container_id);
        let rootfs = container_dir.join("rootfs");
        tokio::fs::create_dir_all(&rootfs).await?;
        store.extract_layers(&image.id, &rootfs).await?;

        // Apply volume mounts onto the extracted rootfs so reads/writes hit the
        // persistent backing dir. Best-effort: a failed mount logs and continues.
        for m in &mounts {
            if let Err(e) = apply_mount_to_rootfs(&rootfs, m).await {
                println!("  [WARN] volume {:?} → {:?} not mounted: {}", m.host_path, m.container_path, e);
            }
        }

        #[cfg(target_os = "windows")]
        let backend = WindowsRuntime::new();
        #[cfg(not(target_os = "windows"))]
        let backend = StatelessEngine::new(data_dir.to_path_buf());
        backend.create(&container, &container_dir).await?;

        // Service-level `entrypoint`/`command` override the image's, matching Docker Compose:
        // entrypoint replaces the image entrypoint; command replaces the image cmd (args).
        let svc_entrypoint = svc.entrypoint.as_ref().map(compose_str_or_vec).filter(|v| !v.is_empty());
        let svc_command = svc.command.as_ref().map(compose_str_or_vec).filter(|v| !v.is_empty());
        let entrypoint = svc_entrypoint.unwrap_or_else(|| image.entrypoint.clone());
        let cmd_args = svc_command.unwrap_or_else(|| image.cmd.clone());
        let effective_cmd = if !entrypoint.is_empty() {
            let mut v = entrypoint;
            v.extend(cmd_args);
            v
        } else if !cmd_args.is_empty() {
            cmd_args
        } else {
            vec!["/bin/sh".to_string()]
        };

        let config_json = serde_json::json!({"cmd": effective_cmd, "env": all_env});
        tokio::fs::write(container_dir.join("config.json"), serde_json::to_string_pretty(&config_json)?).await?;

        // Start detached
        let current_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("crush"));
        let child = std::process::Command::new(&current_exe)
            .arg("internal-run").arg(&container_id)
            .spawn()
            .map_err(|e| CrushError::Internal(anyhow::anyhow!("Failed to start {}: {}", svc_name, e)))?;

        let pid = child.id();
        let mut c_upd = container.clone();
        c_upd.status = ContainerStatus::Running;
        c_upd.pid = Some(pid);
        c_upd.started_at = Some(SystemTime::now());
        let serialized = serde_json::to_string_pretty(&c_upd)?;
        tokio::fs::write(container_dir.join("container.json"), serialized).await?;

        compose_state.insert(svc_name.clone(), container_id.clone());
        println!("  Started {} ({})", svc_name, &container_id[..16]);
    }

    // Save compose state
    tokio::fs::write(&state_path, serde_json::to_string_pretty(&compose_state)?).await?;
    println!("Compose is up. Run `crush compose ps` to check status.");
    Ok(())
}

async fn dir_size_bytes(path: std::path::PathBuf) -> std::io::Result<u64> {
    let mut total = 0u64;
    let mut stack = vec![path];
    while let Some(dir) = stack.pop() {
        let mut entries = tokio::fs::read_dir(&dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let meta = entry.metadata().await?;
            if meta.is_dir() {
                stack.push(entry.path());
            } else {
                total += meta.len();
            }
        }
    }
    Ok(total)
}

/// Coerce a compose `command`/`entrypoint` value (string or array) into argv.
/// A bare string is treated as a single shell word list split on whitespace,
/// matching Compose's "shell form" behavior closely enough for our exec model.
fn compose_str_or_vec(v: &serde_json::Value) -> Vec<String> {
    if let Some(s) = v.as_str() {
        s.split_whitespace().map(|w| w.to_string()).collect()
    } else if let Some(arr) = v.as_array() {
        arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect()
    } else {
        Vec::new()
    }
}

/// Coerce a compose `healthcheck.test` into a single command string.
/// Handles the three Compose forms: a string, `["CMD", exe, args...]`,
/// `["CMD-SHELL", "cmd string"]`, and `["NONE"]` (disables → None).
fn compose_healthcheck_cmd(v: &serde_json::Value) -> Option<String> {
    if let Some(s) = v.as_str() {
        return if s.is_empty() { None } else { Some(s.to_string()) };
    }
    let arr = v.as_array()?;
    let parts: Vec<String> = arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect();
    match parts.first().map(|s| s.as_str()) {
        Some("NONE") => None,
        Some("CMD-SHELL") => parts.get(1).cloned(),
        Some("CMD") => Some(parts[1..].join(" ")),
        _ if !parts.is_empty() => Some(parts.join(" ")),
        _ => None,
    }
}

/// Parse a compose duration like "30s", "1m30s", "500ms", "2h" into whole seconds.
/// Returns None if no recognizable unit is found.
fn parse_duration_secs(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() { return None; }
    // Bare integer → seconds.
    if let Ok(n) = s.parse::<u64>() { return Some(n); }
    let mut total: u64 = 0;
    let mut num = String::new();
    let mut chars = s.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            num.push(c);
            chars.next();
        } else {
            // Collect the unit (alphabetic run).
            let mut unit = String::new();
            while let Some(&u) = chars.peek() {
                if u.is_ascii_alphabetic() { unit.push(u); chars.next(); } else { break; }
            }
            let val: u64 = num.parse().ok()?;
            num.clear();
            match unit.as_str() {
                "h" => total += val * 3600,
                "m" => total += val * 60,
                "s" => total += val,
                "ms" => total += val / 1000, // sub-second rounds toward zero
                _ => return None,
            }
        }
    }
    if total == 0 && !num.is_empty() { return None; }
    Some(total)
}

/// Parse a compose `volumes:` entry (`source:target[:ro]`) into a MountConfig.
/// A source that looks like a filesystem path (`.`, `..`, `/`, or `X:\`) is a
/// bind mount resolved against the compose dir; otherwise it's a named volume
/// backed under `volumes_root/<name>`. Returns None for anonymous/unparseable
/// entries (e.g. a bare container path with no source).
fn parse_compose_volume(entry: &str, compose_dir: &Path, volumes_root: &Path) -> Option<MountConfig> {
    // Split into at most 3 parts: source : target : mode. Handle Windows drive
    // letters (C:\...) by detecting a single-letter first segment.
    let parts: Vec<&str> = entry.split(':').collect();
    let (source, target, mode) = match parts.as_slice() {
        // C:\host\path:/container  → drive-letter source
        [drive, host_rest, target] if drive.len() == 1 => {
            (format!("{}:{}", drive, host_rest), target.to_string(), None)
        }
        [drive, host_rest, target, mode] if drive.len() == 1 => {
            (format!("{}:{}", drive, host_rest), target.to_string(), Some(*mode))
        }
        [source, target] => (source.to_string(), target.to_string(), None),
        [source, target, mode] => (source.to_string(), target.to_string(), Some(*mode)),
        _ => return None, // anonymous volume or malformed — skip
    };

    let is_path = source.starts_with("./")
        || source.starts_with("../")
        || source.starts_with('/')
        || source.starts_with('.')
        || (source.len() >= 2 && source.as_bytes()[1] == b':'); // C:\
    let host_path = if is_path {
        compose_dir.join(&source)
    } else {
        volumes_root.join(&source) // named volume
    };

    Some(MountConfig {
        host_path,
        container_path: PathBuf::from(target),
        read_only: matches!(mode, Some("ro")),
        is_tmpfs: false,
    })
}

/// Make a mount visible inside the extracted rootfs by linking
/// `rootfs/<container_path>` to the backing `host_path`. Uses a directory
/// symlink on Unix and a junction on Windows; falls back to copying contents
/// in if linking isn't permitted. Best-effort and idempotent.
async fn apply_mount_to_rootfs(rootfs: &Path, mount: &MountConfig) -> anyhow::Result<()> {
    // Ensure the backing dir exists.
    tokio::fs::create_dir_all(&mount.host_path).await?;

    // Compute the in-rootfs target, stripping any leading separator from the
    // container path so it joins relative to the rootfs.
    let rel = mount.container_path.strip_prefix("/").unwrap_or(&mount.container_path);
    let link_path = rootfs.join(rel);

    if let Some(parent) = link_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    // If something already exists at the target (extracted from the image), move
    // it aside so the volume takes precedence (Docker shadows the image path).
    if tokio::fs::symlink_metadata(&link_path).await.is_ok() {
        let _ = tokio::fs::remove_dir_all(&link_path).await;
        let _ = tokio::fs::remove_file(&link_path).await;
    }

    let host = mount.host_path.clone();
    let link = link_path.clone();
    tokio::task::spawn_blocking(move || -> std::io::Result<()> {
        #[cfg(unix)]
        { std::os::unix::fs::symlink(&host, &link) }
        #[cfg(windows)]
        { std::os::windows::fs::symlink_dir(&host, &link) }
        #[cfg(not(any(unix, windows)))]
        { Ok(()) }
    }).await??;

    Ok(())
}

fn parse_memory_bytes(s: &str) -> Option<u64> {
    let s = s.trim();
    if let Ok(n) = s.parse::<u64>() { return Some(n); }
    let (num, unit) = s.split_at(s.len().saturating_sub(1));
    let base: u64 = num.trim().parse().ok()?;
    match unit.to_lowercase().as_str() {
        "k" => Some(base * 1024),
        "m" => Some(base * 1024 * 1024),
        "g" => Some(base * 1024 * 1024 * 1024),
        _ => None,
    }
}

#[derive(Args, Debug)]
pub struct LoginArgs {
    #[arg(help = "OCI Registry URL to log into (e.g. docker.io)")]
    pub registry: String,
    #[arg(short, long, help = "Username")]
    pub username: Option<String>,
    #[arg(short, long, help = "Password")]
    pub password: Option<String>,
    #[arg(long, help = "Read password from stdin")]
    pub password_stdin: bool,
}

#[cfg(test)]
mod compose_honor_helpers {
    use super::*;
    use serde_json::json;

    #[test]
    fn healthcheck_cmd_forms() {
        // CMD form joins the executable + args
        assert_eq!(
            compose_healthcheck_cmd(&json!(["CMD", "curl", "-f", "http://localhost/health"])),
            Some("curl -f http://localhost/health".to_string())
        );
        // CMD-SHELL takes the single shell string
        assert_eq!(
            compose_healthcheck_cmd(&json!(["CMD-SHELL", "pg_isready -U admin"])),
            Some("pg_isready -U admin".to_string())
        );
        // bare string is taken as-is
        assert_eq!(
            compose_healthcheck_cmd(&json!("redis-cli ping")),
            Some("redis-cli ping".to_string())
        );
        // NONE disables the healthcheck
        assert_eq!(compose_healthcheck_cmd(&json!(["NONE"])), None);
    }

    #[test]
    fn str_or_vec_command_forms() {
        // string (shell form) splits on whitespace
        assert_eq!(compose_str_or_vec(&json!("node server.js")), vec!["node", "server.js"]);
        // array (exec form) kept verbatim
        assert_eq!(
            compose_str_or_vec(&json!(["/usr/bin/tini", "--", "node"])),
            vec!["/usr/bin/tini", "--", "node"]
        );
        // non-string/array → empty
        assert!(compose_str_or_vec(&json!(42)).is_empty());
    }

    #[test]
    fn duration_parsing() {
        assert_eq!(parse_duration_secs("30s"), Some(30));
        assert_eq!(parse_duration_secs("1m30s"), Some(90));
        assert_eq!(parse_duration_secs("2h"), Some(7200));
        assert_eq!(parse_duration_secs("45"), Some(45)); // bare int → seconds
        assert_eq!(parse_duration_secs("500ms"), Some(0)); // sub-second rounds toward zero
        assert_eq!(parse_duration_secs("garbage"), None);
    }

    #[test]
    fn cpus_to_cpu_shares() {
        // The mapping used in run_compose_up: cpus * 1024, rounded.
        let shares = |c: f64| (c * 1024.0).round() as u64;
        assert_eq!(shares(0.50), 512);
        assert_eq!(shares(1.0), 1024);
        assert_eq!(shares(0.25), 256);
    }

    #[test]
    fn memory_parsing() {
        assert_eq!(parse_memory_bytes("512M"), Some(512 * 1024 * 1024));
        assert_eq!(parse_memory_bytes("1G"), Some(1024 * 1024 * 1024));
        assert_eq!(parse_memory_bytes("1024"), Some(1024));
    }

    #[test]
    fn volume_parsing() {
        let compose_dir = Path::new("/proj");
        let vroot = Path::new("/data/volumes/proj");

        // named volume → backed under volumes_root
        let named = parse_compose_volume("web_data:/usr/share/nginx/html", compose_dir, vroot).unwrap();
        assert_eq!(named.host_path, vroot.join("web_data"));
        assert_eq!(named.container_path, PathBuf::from("/usr/share/nginx/html"));
        assert!(!named.read_only);

        // relative bind mount → resolved against compose dir
        let bind = parse_compose_volume("./backend:/app", compose_dir, vroot).unwrap();
        assert_eq!(bind.host_path, compose_dir.join("./backend"));
        assert_eq!(bind.container_path, PathBuf::from("/app"));

        // read-only flag honored
        let ro = parse_compose_volume("db_data:/var/lib/postgresql/data:ro", compose_dir, vroot).unwrap();
        assert!(ro.read_only);
        assert_eq!(ro.host_path, vroot.join("db_data"));

        // absolute host bind mount
        let abs = parse_compose_volume("/srv/data:/data", compose_dir, vroot).unwrap();
        assert_eq!(abs.host_path, PathBuf::from("/srv/data"));

        // anonymous / malformed → skipped
        assert!(parse_compose_volume("just_a_path", compose_dir, vroot).is_none());
    }
}
