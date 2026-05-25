use std::path::Path;

pub fn generate_man_page(command: &str) -> String {
    match command {
        "crush" => CRUSH_MAN,
        "crush-run" => CRUSH_RUN_MAN,
        "crush-build" => CRUSH_BUILD_MAN,
        "crush-ps" => CRUSH_PS_MAN,
        "crush-logs" => CRUSH_LOGS_MAN,
        "crush-debug" => CRUSH_DEBUG_MAN,
        "crush-compose" => CRUSH_COMPOSE_MAN,
        "crush-network" => CRUSH_NETWORK_MAN,
        "crush-volume" => CRUSH_VOLUME_MAN,
        "crush-registry" => CRUSH_REGISTRY_MAN,
        _ => CRUSH_MAN,
    }.to_string()
}

pub fn save_man_pages(output_dir: &Path) -> std::io::Result<()> {
    let commands = ["crush", "crush-run", "crush-build", "crush-ps", "crush-logs",
        "crush-debug", "crush-compose", "crush-network", "crush-volume", "crush-registry"];
    for cmd in &commands {
        let content = generate_man_page(cmd);
        let path = output_dir.join(format!("{}.1", cmd));
        std::fs::write(&path, &content)?;
    }
    Ok(())
}

const CRUSH_MAN: &str = r#".TH CRUSH 1 "May 2026" "crush 0.1.0" "Crush Container Runtime"
.SH NAME
crush \- from-scratch, production-grade container runtime
.SH SYNOPSIS
.B crush
[\fI\,OPTIONS\/\fR] \fI\,COMMAND\/\fR [\fI\,ARGS\/\fR]...
.SH DESCRIPTION
Crush is a container runtime written entirely in Rust. No daemon, no VM on Windows,
no wrapper around existing tools. Every layer is built clean, from the lowest level up.
.SH COMMANDS
.TP
.B crush run
Run an image inside a sandboxed container.
.TP
.B crush build
Build an image from a project root or Crushfile.
.TP
.B crush ps
List running and stopped containers.
.TP
.B crush logs
Fetch and stream logs of a container.
.TP
.B crush debug
AI-driven error analysis on a failed container.
.TP
.B crush compose
Manage multi-container setups using compose files.
.TP
.B crush network
Manage isolated container networks.
.TP
.B crush volume
Manage named volumes for persistent storage.
.TP
.B crush registry
Serve a local OCI-compatible registry proxy.
.SH OPTIONS
.TP
.B \-c, \-\-config
Path to custom Crushfile (default: Crushfile)
.TP
.B \-n, \-\-no-interactive
Run in non-interactive mode
.SH SEE ALSO
.BR crush-run (1),
.BR crush-build (1),
.BR crush-ps (1)
"#;

const CRUSH_RUN_MAN: &str = r#".TH CRUSH-RUN 1 "May 2026" "crush 0.1.0"
.SH NAME
crush-run \- run an image in a sandboxed container
.SH SYNOPSIS
.B crush run
[\fI\,OPTIONS\/\fR] \fI\,IMAGE\/\fR
.SH DESCRIPTION
Pulls the specified image, creates an isolated container, and runs it.
On Linux uses namespaces + cgroups + seccomp. On Windows uses Job Objects + HCS.
On macOS uses Virtualization.framework. On any platform, WASM binaries run
natively via wasmtime with WASI preview 2.
.SH OPTIONS
.TP
.B \-p, \-\-port
Map container ports (e.g. 8080:80)
.TP
.B \-e, \-\-env
Set environment variables (e.g. KEY=VAL)
.TP
.B \-v, \-\-volume
Mount persistent volumes (e.g. my-vol:/data)
.TP
.B \-\-name
Assign a name to the container
.TP
.B \-d, \-\-detach
Run in detached background mode
.TP
.B \-\-memory
Memory limit in MB
"#;

const CRUSH_BUILD_MAN: &str = r#".TH CRUSH-BUILD 1 "May 2026" "crush 0.1.0"
.SH NAME
crush-build \- build an OCI image from a project root
.SH SYNOPSIS
.B crush build
[\fI\,OPTIONS\/\fR]
.SH DESCRIPTION
Auto-detects the project stack (Node/Rust/Python/Go/Java/.NET),
builds a layered OCI image with content-addressed caching, and outputs
a SHA256 digest. Supports multi-stage builds, build secrets, and
cross-compilation via QEMU.
.SH OPTIONS
.TP
.B \-t, \-\-tag
Output image tag (default: app:latest)
.TP
.B \-\-platform
Target platforms (e.g. linux/amd64,linux/arm64)
.TP
.B \-\-no-cache
Do not use cached build layers
"#;

const CRUSH_PS_MAN: &str = r#".TH CRUSH-PS 1 "May 2026" "crush 0.1.0"
.SH NAME
crush-ps \- list containers
.SH SYNOPSIS
.B crush ps
[\fI\,OPTIONS\/\fR]
.SH DESCRIPTION
Lists all running or stopped containers with their ID, name, image,
status, resource usage, and port mappings.
.SH OPTIONS
.TP
.B \-a, \-\-all
Show all containers (default shows running only)
.TP
.B \-\-format
Output format (text, json). Default: text
"#;

const CRUSH_LOGS_MAN: &str = r#".TH CRUSH-LOGS 1 "May 2026" "crush 0.1.0"
.SH NAME
crush-logs \- fetch container logs
.SH SYNOPSIS
.B crush logs
[\fI\,OPTIONS\/\fR] \fI\,CONTAINER\/\fR
.SH DESCRIPTION
Fetches and streams logs from a running or stopped container.
Supports follow mode, line tailing, and AI-powered error diagnosis.
.SH OPTIONS
.TP
.B \-f, \-\-follow
Follow log stream in real time
.TP
.B \-\-tail
Number of lines to tail from end (default: 100)
"#;

const CRUSH_DEBUG_MAN: &str = r#".TH CRUSH-DEBUG 1 "May 2026" "crush 0.1.0"
.SH NAME
crush-debug \- AI-powered error diagnosis
.SH SYNOPSIS
.B crush debug
\fI\,CONTAINER\/\fR
.SH DESCRIPTION
Performs AI-driven error analysis on a failed container. Parses
stack traces for 10+ languages, retrieves source context, runs
diagnosis via Anthropic API (or offline for common errors), and
optionally applies auto-fixes.
"#;

const CRUSH_COMPOSE_MAN: &str = r#".TH CRUSH-COMPOSE 1 "May 2026" "crush 0.1.0"
.SH NAME
crush-compose \- manage multi-container compose setups
.SH SYNOPSIS
.B crush compose
[\fI\,OPTIONS\/\fR] \fI\,SUBCOMMAND\/\fR
.SH DESCRIPTION
Parses docker-compose.yml (v2/v3), resolves dependency order,
and manages multi-container deployments with networks and volumes.
.SH COMMANDS
.B up, down, ps, logs
"#;

const CRUSH_NETWORK_MAN: &str = r#".TH CRUSH-NETWORK 1 "May 2026" "crush 0.1.0"
.SH NAME
crush-network \- manage container networks
.SH SYNOPSIS
.B crush network
\fI\,SUBCOMMAND\/\fR
.SH DESCRIPTION
Creates, removes, and inspects isolated container networks with
bridge mode, NAT, DNS resolution, and IPv6 dual-stack support.
"#;

const CRUSH_VOLUME_MAN: &str = r#".TH CRUSH-VOLUME 1 "May 2026" "crush 0.1.0"
.SH NAME
crush-volume \- manage persistent storage volumes
.SH SYNOPSIS
.B crush volume
\fI\,SUBCOMMAND\/\fR
.SH DESCRIPTION
Creates and manages named volumes, bind mounts, tmpfs mounts,
and volume drivers (local, NFS, CIFS). Supports backup/restore
via streaming tar with fsfreeze-consistent snapshots.
"#;

const CRUSH_REGISTRY_MAN: &str = r#".TH CRUSH-REGISTRY 1 "May 2026" "crush 0.1.0"
.SH NAME
crush-registry \- local OCI registry proxy
.SH SYNOPSIS
.B crush registry
[\fI\,OPTIONS\/\fR]
.SH DESCRIPTION
Serves a local OCI-compatible registry proxy for testing and
air-gapped deployments. Supports blob push/pull and manifest
management.
"#;
