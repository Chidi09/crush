# Crush v0.2.0 Implementation Plan

This document describes all changes needed for the v0.2.0 release. Each task is
self-contained with exact file paths, what is wrong today, and what the correct
implementation must be. Work through the tasks in order — later tasks sometimes
depend on earlier ones.

---

## Environment

- Workspace root: `crates/` — all paths below are relative to the repo root.
- Cross-compile target: `x86_64-pc-windows-gnu` on Linux.  
  Build command: `cargo build --release --target x86_64-pc-windows-gnu -p crush-cli`
- Workspace `Cargo.toml` is at repo root; workspace-level dep versions apply to
  all crates. Add new deps at workspace level unless they are target-specific.
- Do NOT add `Co-Authored-By: Claude` or any AI trailer to git commits.

---

## Task 1 — Strip debug instrumentation from runner.rs

**File:** `crates/crush-runtime-linux/src/runner.rs`

**Problem:** The entire `pre_exec` closure contains ~40 lines of debug code that
writes to `/tmp/crush_pre_exec_debug.log`. The two helper closures `log` and
`log_err` open, write, and close a file on every step. This is a dev artifact
that must be removed before a production release; it writes to /tmp on every
container start and leaks path information.

**What to do:**

1. Remove the `let log = |msg: &str| { ... };` closure (lines 55–63).
2. Remove the `let log_err = |msg: &str, err: i32| { ... };` closure (lines 65–76).
3. Remove every `log("...")` and `log_err("...")` call throughout the `pre_exec`
   closure. There are approximately 22 such calls.
4. Remove the line at the top of the function that deletes the old log file:
   `let _ = std::fs::remove_file("/tmp/crush_pre_exec_debug.log");`
5. Keep all the actual logic (unshare, mount, pivot_root, chdir, umount2,
   AppArmor, SELinux) — only strip the logging calls and the two helper closures.

After this change the `pre_exec` closure should contain only functional syscalls
and error returns, with no file I/O.

---

## Task 2 — Fix `spawn_container_process` — it hardcodes `/sbin/init`

**File:** `crates/crush-runtime-linux/src/lib.rs`

**Problem:** `LinuxRuntime::spawn_container_process` (line ~219) does:
```rust
let mut cmd = std::process::Command::new("/sbin/init");
cmd.arg(container_id)
```
This is wrong. It starts `/sbin/init` with the container ID as an argument
instead of running the container's actual entrypoint. The function also applies a
seccomp BPF filter but the filter is applied in a fresh `pre_exec` that does
nothing else — it doesn't do the namespace unshare or pivot_root that
`runner.rs::run_container` does.

The `runner.rs::run_container` function is the correct, complete implementation.
`spawn_container_process` should delegate to it.

**What to do:**

Replace `spawn_container_process` so it:

1. Loads the container JSON from disk at
   `dirs_or_default().join("containers").join(container_id).join("container.json")`.
2. Loads the image JSON from disk to get `entrypoint` and `cmd`. The image JSON
   is stored by the CLI at
   `dirs_or_default().join("images").join(&container.image).join("image.json")`.
   If neither file exists, return `Err(CrushError::ContainerNotFound(...))`.
3. Builds the final command vector: `entrypoint + cmd`, falling back to
   `["/bin/sh"]` if both are empty.
4. Builds the rootfs path:
   `dirs_or_default().join("containers").join(container_id).join("rootfs")`.
5. Builds env from `image.env` merged with any `container`-level env (none in
   the type today, so just use `image.env`).
6. Calls `runner::run_container(&rootfs, &command, &env, &container)` inside a
   `tokio::task::spawn_blocking` (because `run_container` blocks).
7. After spawn, writes the child's PID to
   `dirs_or_default().join("containers").join(container_id).join("pid")`.
8. Returns the PID as `u32`.

The seccomp filter that was previously applied here is already applied inside
`runner.rs` via a second `pre_exec` block — no need to duplicate it.

Signature stays `async fn spawn_container_process(&self, container_id: &str) -> Result<u32>`.

---

## Task 3 — Persist and restore container PIDs

**File:** `crates/crush-runtime-linux/src/lib.rs`

**Problem:** `child_pids: Arc<Mutex<HashMap<String, u32>>>` is in-memory only.
After a crash or restart `crush ps` cannot know which containers are still
running.

**What to do:**

1. In `start()`, after `spawn_container_process` succeeds and the PID is
   inserted into `child_pids`, also write the PID to
   `dirs_or_default().join("containers").join(container_id).join("pid")`
   as a plain decimal string (no newline necessary, but a trailing newline is fine).

2. In `stop()`, after removing the PID from `child_pids`, delete the pid file:
   `let _ = std::fs::remove_file(dirs_or_default().join("containers").join(container_id).join("pid"));`

3. Add a `restore_pids()` method to `LinuxRuntime`:
   ```rust
   pub async fn restore_pids(&self) {
       let containers_dir = dirs_or_default().join("containers");
       if let Ok(entries) = std::fs::read_dir(&containers_dir) {
           for entry in entries.flatten() {
               let pid_file = entry.path().join("pid");
               if let Ok(text) = std::fs::read_to_string(&pid_file) {
                   if let Ok(pid) = text.trim().parse::<u32>() {
                       // Verify the process is still alive
                       if unsafe { libc::kill(pid as libc::pid_t, 0) } == 0 {
                           let id = entry.file_name().to_string_lossy().to_string();
                           self.child_pids.lock().await.insert(id, pid);
                       } else {
                           // Process is dead — clean up stale pid file
                           let _ = std::fs::remove_file(&pid_file);
                       }
                   }
               }
           }
       }
   }
   ```

4. Call `restore_pids()` once during `LinuxRuntime::new()` — but since `new()`
   is sync, either make it async or add a `tokio::runtime::Handle::current().block_on(...)`.
   Easiest: call it from the CLI's startup path after constructing the runtime.
   In `crates/crush-cli/src/main.rs`, find where `LinuxRuntime::new()` is called
   (inside the `Commands::Run` / `Commands::Start` handlers) and call
   `runtime.restore_pids().await` immediately after construction.

---

## Task 4 — Fix CPU shares → cgroup v2 weight conversion

**File:** `crates/crush-runtime-linux/src/lib.rs`

**Problem:** In `LinuxRuntime::create()` (around line 87):
```rust
if let Some(cpu_shares) = container.cpu_shares {
    cgroup.enforce_cpu_limit(cpu_shares)?;
}
```
`container.cpu_shares` stores Docker-style CPU shares (range 2–262144, default
1024 = 1 CPU). cgroup v2 `cpu.weight` accepts 1–10000 (default 100). Passing
raw Docker shares to `cpu.weight` would set a value far outside the valid range
(e.g. `1024` is fine by coincidence, but `262144` would be invalid and the write
would fail with EINVAL).

**What to do:**

Replace the block with:
```rust
if let Some(cpu_shares) = container.cpu_shares {
    // Convert Docker cpu_shares (2-262144, default 1024) to cgroup v2
    // cpu.weight (1-10000, default 100). Linear scale: 1024 → 100.
    let weight = ((cpu_shares as u64) * 100 / 1024).clamp(1, 10000);
    cgroup.enforce_cpu_limit(weight)?;
}
```

---

## Task 5 — Make `execute_run` in the build pipeline actually run commands

**File:** `crates/crush-build/src/pipeline.rs`

**Problem:** `execute_run` (line ~174) never runs the command:
```rust
let output = format!("build output for: {}", cmd);
let entry = self.cache.put("deps:latest", output.as_bytes(), "deps", false).await?;
```
It stores a placeholder string instead of executing the build command. This means
`crush build` and `crush default` never actually install dependencies or compile
the project.

**What to do:**

Replace the body of `execute_run` after the cache-hit path with real execution:

```rust
async fn execute_run(&self, root: &Path, cmd: &str, _stage_name: &Option<String>) -> Result<LayerInfo> {
    let lockfile_key = BuildCache::lockfile_key(root)?;

    if !lockfile_key.is_empty() {
        if let Ok(Some(cached)) = self.cache.get(&lockfile_key).await {
            return Ok(LayerInfo {
                name: format!("deps({})", cmd),
                digest: cached.layer_digest,
                size_bytes: cached.size_bytes,
                duration_ms: 0,
                cached: true,
            });
        }
    }

    // Actually run the build command in the project root.
    // Use `sh -c` so shell features (pipes, &&) work.
    let stage_start = std::time::Instant::now();
    let output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .current_dir(root)
        .output()
        .await
        .map_err(|e| CrushError::StorageError(format!("Build command spawn failed: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CrushError::StorageError(format!(
            "Build command `{}` exited with status {:?}:\n{}",
            cmd, output.status.code(), stderr
        )));
    }

    let elapsed = stage_start.elapsed().as_millis() as u64;
    let combined = [output.stdout.as_slice(), output.stderr.as_slice()].concat();
    let size = combined.len() as u64;
    let cache_key = if lockfile_key.is_empty() {
        format!("run:{}", cmd)
    } else {
        lockfile_key
    };
    let entry = self.cache.put(&cache_key, &combined, "deps", false).await?;

    Ok(LayerInfo {
        name: format!("deps({})", cmd),
        digest: entry.layer_digest,
        size_bytes: size,
        duration_ms: elapsed,
        cached: false,
    })
}
```

**Windows note:** On Windows, `sh -c` may not exist. Add a cfg-gated fallback:
```rust
#[cfg(target_os = "windows")]
let mut proc = tokio::process::Command::new("cmd");
#[cfg(target_os = "windows")]
proc.args(["/C", cmd]);
#[cfg(not(target_os = "windows"))]
let mut proc = {
    let mut c = tokio::process::Command::new("sh");
    c.arg("-c").arg(cmd);
    c
};
let output = proc.current_dir(root).output().await ...
```

---

## Task 6 — Fix Windows runtime stub methods

**File:** `crates/crush-runtime-windows/src/lib.rs`

**Problem:** Four `RuntimeBackend` impl methods in `windows_impl` are stubs:
- `pause()` returns `Ok(())`  — should suspend all threads in the job
- `resume()` returns `Ok(())` — should resume all threads
- `delete()` returns `Ok(())` — should remove job object and clean up state dir
- `exec()` returns `Ok(0)`   — should spawn a process attached to the job
- `get_pid()` returns hardcoded `Some(4242)`

**What to do:**

Add a `pids: Arc<Mutex<HashMap<String, u32>>>` field to `WindowsRuntime` (it
already has `job_handles`, add a parallel map for PIDs).

**`get_pid`:** Look up the PID from `self.pids.lock().unwrap().get(container_id).copied()` and return it. Update `start()` to insert the child PID returned by `ChildProcess::spawn_in_job` into `self.pids`.

**`delete`:** Remove job handle from `job_handles`, remove PID from `pids`, and
delete the container state directory:
```rust
async fn delete(&self, container_id: &str) -> Result<()> {
    self.job_handles.lock().unwrap().remove(container_id);
    self.pids.lock().unwrap().remove(container_id);
    let state_dir = self.data_dir.join("containers").join(container_id);
    let _ = std::fs::remove_dir_all(&state_dir);
    Ok(())
}
```

**`exec`:** Spawn a process inside the existing job object using `ChildProcess::spawn_in_job`. Build a command string from the `command` slice and call:
```rust
async fn exec(&self, container_id: &str, command: &[String], _tty: bool) -> Result<i32> {
    let handles = self.job_handles.lock().unwrap();
    let job = handles.get(container_id).ok_or_else(|| {
        CrushError::ContainerNotFound(container_id.to_string())
    })?;
    let cmd_str = command.iter()
        .map(|s| if s.contains(' ') { format!("\"{}\"", s) } else { s.clone() })
        .collect::<Vec<_>>()
        .join(" ");
    let child = crate::process::ChildProcess::spawn_in_job(&cmd_str, None, job)
        .map_err(|e| CrushError::NamespaceError(e.to_string()))?;
    // Wait for the process (ChildProcess needs a wait method — see note below)
    Ok(0)
}
```

**`pause` / `resume`:** On Windows, suspend/resume all threads in the job.
Use `NtSuspendProcess` / `NtResumeProcess` (available via `windows-sys` or raw
`ntdll`). The simplest approach is to enumerate threads via
`CreateToolhelp32Snapshot` + `Thread32First/Next`, check `th32OwnerProcessID`
against the job's process list, and call `SuspendThread` / `ResumeThread`.

Add `Win32_System_Diagnostics_ToolHelp` to the `windows-sys` features in
`crates/crush-runtime-windows/Cargo.toml` if it isn't already listed.

Alternatively, if that's too invasive, write a clearly-marked partial stub that
returns `Err(CrushError::NamespaceError("pause/resume not yet supported on Windows".to_string()))` — that is better than silently doing nothing.

---

## Task 7 — Remove the hardcoded `"Hello, World!"` in Windows `start()`

**File:** `crates/crush-runtime-windows/src/lib.rs`

**Problem:** `WindowsRuntime::start()` (line ~185) spawns:
```rust
let _child = ChildProcess::spawn_in_job("cmd.exe /c echo 'Hello, World!'", None, job)
```
This is a placeholder. `start()` is called without a command in the
`RuntimeBackend` trait — the entrypoint must be loaded from the container config,
exactly like Task 2 does for Linux.

**What to do:**

Mirror the Linux Task 2 fix:
1. Load container JSON from `self.data_dir.join("containers").join(container_id).join("container.json")`.
2. Load image JSON to get `entrypoint` + `cmd` (or fall back to `["cmd.exe"]`).
3. Build rootfs path: `self.data_dir.join("containers").join(container_id).join("rootfs")`.
4. Call `self.start_with_config(container_id, &command, &env, &rootfs).await` which
   already does the job-object spawn correctly.

---

## Task 8 — `crush run` on Windows uses wrong runtime path

**File:** `crates/crush-cli/src/main.rs`

**Problem:** The `Commands::Run` handler calls `run_container` from
`crush_runtime_linux::runner`. On Windows this function exists (it compiles) but
calls `pre_exec` which is gated `#[cfg(target_os = "linux")]` and silently
becomes a no-op, meaning it runs the process without any isolation.

**What to do:**

In the `Commands::Run` handler, wrap the runtime call in platform cfg guards:

```rust
#[cfg(target_os = "linux")]
{
    use crush_runtime_linux::runner::run_container;
    let exit = run_container(&rootfs_path, &cmd_vec, &env_vec, &container)?;
    println!("Container exited with code {}", exit);
}
#[cfg(target_os = "windows")]
{
    use crush_runtime_windows::WindowsRuntime;
    let rt = WindowsRuntime::new();
    rt.create(&container, &container_dir).await?;
    rt.start(&container.id).await?;
    println!("Container started on Windows (PID: {:?})", rt.get_pid(&container.id).await?);
}
```

The container and container_dir variables are already constructed before this
block in the existing handler — just wrap the dispatch.

---

## Task 9 — `crush logs --follow` for a running container should tail the file

**File:** `crates/crush-cli/src/main.rs`

**Problem:** The current `--follow` implementation for running containers reads
the log file once and then streams from a tokio mpsc channel. But the channel is
only populated by the initial read — it never receives new bytes as the container
process keeps writing.

**What to do:**

For `--follow`, after printing all existing log content, enter a polling loop
that checks the file size every 200ms and prints any new bytes:

```rust
if follow {
    let mut offset = initial_content.len() as u64;
    loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        if let Ok(mut f) = tokio::fs::File::open(&log_path).await {
            use tokio::io::{AsyncReadExt, AsyncSeekExt};
            if f.seek(std::io::SeekFrom::End(0)).await.ok().unwrap_or(0) > offset {
                f.seek(std::io::SeekFrom::Start(offset)).await?;
                let mut buf = Vec::new();
                f.read_to_end(&mut buf).await?;
                if !buf.is_empty() {
                    print!("{}", String::from_utf8_lossy(&buf));
                    offset += buf.len() as u64;
                }
            }
        }
    }
}
```

Remove the tokio mpsc channel approach that was previously there for `--follow`
since it only streams the initial snapshot.

---

## Task 10 — `crush inspect` for containers should show real data

**File:** `crates/crush-cli/src/main.rs`

**Problem:** `Commands::Inspect` currently prints a minimal JSON stub. It should
load and print the actual container JSON (or image JSON, or network JSON based on
the `--type` flag) from disk.

**What to do:**

In the `Commands::Inspect` handler:

1. If `inspect_type == "container"` (or type not specified): read
   `data_dir/containers/<id>/container.json`, parse it as `crush_types::Container`,
   and pretty-print via `serde_json::to_string_pretty`.
2. If `inspect_type == "image"`: load image from `ImageStore` by ID/tag and
   pretty-print the `crush_types::Image`.
3. If `inspect_type == "network"`: read the network JSON from
   `data_dir/networks/<id>.json`.
4. If the file doesn't exist, print a clear `"Error: <type> '<id>' not found"` message.

---

## Task 11 — `crush health` should check against the container's health command

**File:** `crates/crush-cli/src/main.rs`

**Problem:** `Commands::Health` currently prints a placeholder message. The
`Container` type already has `health_cmd`, `health_interval`, `health_timeout`,
and `health_retries` fields.

**What to do:**

1. Load container JSON from `data_dir/containers/<id>/container.json`.
2. If `container.health_cmd` is `None`, print `"No health check configured for container <id>"` and exit.
3. Otherwise run the health command via `std::process::Command::new("sh").arg("-c").arg(&health_cmd)` with a timeout of `container.health_timeout.unwrap_or(30)` seconds.
4. If exit code is 0: print `"Status: healthy"`. If non-zero: print `"Status: unhealthy (exit code <n>)"`.
5. Update `container.health` to `HealthStatus::Healthy` or `HealthStatus::Unhealthy` and write back to disk.

---

## Task 12 — `crush volume` commands should be wired to `VolumeManager`

**File:** `crates/crush-cli/src/main.rs`

**Problem:** `Commands::Volume` subcommands (`create`, `ls`, `rm`, `inspect`)
print stub output. `crush-volume` exists with a `VolumeManager` type.

**What to do:**

1. Add `crush-volume` to `crush-cli/Cargo.toml` dependencies (check it isn't already there; if `crush_volume` is importable, it is).
2. In the `VolumeSubcommand::Create` handler: call `VolumeManager::create_volume(name, driver, labels)`.
3. In `VolumeSubcommand::Ls`: call `VolumeManager::list_volumes()` and print a table.
4. In `VolumeSubcommand::Rm`: call `VolumeManager::remove_volume(name)`.
5. In `VolumeSubcommand::Inspect`: call `VolumeManager::inspect_volume(name)` and pretty-print JSON.

Check `crates/crush-volume/src/lib.rs` for the actual public API before wiring
(method names may differ slightly).

---

## Task 13 — `crush daemon` should actually start the API server

**File:** `crates/crush-cli/src/main.rs`

**Problem:** `Commands::Daemon` currently prints `"Daemon started"` and exits.
`crush-api` has a working `ApiServer` with `serve_unix_socket` (Linux) and
`serve_named_pipe` (Windows).

**What to do:**

In the `Commands::Daemon` handler:

```rust
Commands::Daemon => {
    #[cfg(target_os = "linux")]
    {
        use crush_api::ApiServer;
        use crush_runtime_linux::LinuxRuntime;
        let rt = std::sync::Arc::new(LinuxRuntime::new());
        let socket = std::path::PathBuf::from("/var/run/crush.sock");
        let server = ApiServer::new(socket, data_dir, rt);
        println!("Crush daemon listening on /var/run/crush.sock");
        server.serve_unix_socket().await?;
        // Block forever
        tokio::signal::ctrl_c().await.ok();
    }
    #[cfg(target_os = "windows")]
    {
        use crush_api::ApiServer;
        use crush_runtime_windows::WindowsRuntime;
        let rt = std::sync::Arc::new(WindowsRuntime::new());
        let pipe = std::path::PathBuf::from(r"\\.\pipe\crush-api");
        let server = ApiServer::new(pipe, data_dir, rt);
        println!("Crush daemon listening on \\\\.\\pipe\\crush-api");
        server.serve_named_pipe().await?;
        tokio::signal::ctrl_c().await.ok();
    }
}
```

`serve_unix_socket` / `serve_named_pipe` spawn their accept loop in a background
task and return immediately — that's why the `ctrl_c` await is needed to keep the
process alive.

---

## Task 14 — `crush docker-context` should write a real Docker CLI config

**File:** `crates/crush-cli/src/main.rs`

**Problem:** `Commands::DockerContext` prints `"Docker context configured"`. It
should actually write a Docker CLI context so `docker` commands are routed to
crush.

**What to do:**

Docker contexts are stored in `~/.docker/contexts/meta/<hash>/meta.json`.  
The hash is a SHA256 of the context name.  
Write the following JSON:

```json
{
  "Name": "crush",
  "Metadata": {},
  "Endpoints": {
    "docker": {
      "Host": "unix:///var/run/crush.sock",
      "SkipTLSVerify": false
    }
  }
}
```

On Windows use `npipe:////./pipe/crush-api` as the host.

Steps:
1. Compute `sha256("crush")` as a hex string → `context_hash`.
2. Create `~/.docker/contexts/meta/<context_hash>/` directory.
3. Write `meta.json` with the content above.
4. Print `"Docker context 'crush' created. Run: docker context use crush"`.

Use `sha2::Sha256` (already a transitive dep) to compute the hash.

---

## Task 15 — Build pipeline: wire `Crushfile` stages to `BuildPipeline`

**File:** `crates/crush-cli/src/main.rs` and `crates/crush-build/src/pipeline.rs`

**Problem:** When `crush build` or `crush default` runs, it calls
`BuildPipeline::execute(root, &stages, &args)`, but the `stages` vector is often
empty or synthesised from only detection output. If a `Crushfile` exists in the
project root it should be parsed and its `[[stages]]` sections used.

**What to do:**

In the `Commands::Build` and `Commands::Default` handlers, before creating
`stages`:

1. Check if `Crushfile` exists at `project_root/Crushfile`.
2. If yes: parse it with `CrushfileParser::parse(&path)`, then map
   `crushfile.stages.unwrap_or_default()` directly to `Vec<CrushfileStage>`.
3. If no Crushfile: synthesise stages from `Detection` as today, but also auto-add:
   - A `base` stage using `detection.runtime_version` as the base image tag
     (e.g. `"node:20-alpine"` for a Node project).
   - A `run` stage with `command = detection.build_command`.
   - A `copy` stage with `rule = "."`.
   - A `config` stage.
4. Either way, pass the stages to `BuildPipeline::execute`.

---

## Task 16 — Improve `crush ps` table output

**File:** `crates/crush-cli/src/main.rs`

**Problem:** `crush ps` output is functional but the column widths are fixed.
Containers with long names truncate or overflow. Also: the `STATUS` column
currently shows the raw `ContainerStatus` enum variant — for running containers,
show `"Up <duration>"` like Docker does.

**What to do:**

1. After loading all containers, compute column widths dynamically:
   ```rust
   let name_w = containers.iter().map(|c| c.name.len()).max().unwrap_or(4).max(4);
   let image_w = containers.iter().map(|c| c.image.len()).max().unwrap_or(5).max(5);
   ```
2. For the STATUS column, compute human-readable uptime:
   ```rust
   fn format_status(c: &Container) -> String {
       match c.status {
           ContainerStatus::Running => {
               if let Some(started) = c.started_at {
                   let secs = started.elapsed().unwrap_or_default().as_secs();
                   format!("Up {}", humanize_duration(secs))
               } else {
                   "Up".to_string()
               }
           }
           ContainerStatus::Stopped => "Exited".to_string(),
           ContainerStatus::Paused  => "Paused".to_string(),
           ContainerStatus::Created => "Created".to_string(),
           ContainerStatus::Creating => "Creating".to_string(),
       }
   }
   
   fn humanize_duration(secs: u64) -> String {
       if secs < 60 { format!("{} seconds", secs) }
       else if secs < 3600 { format!("{} minutes", secs / 60) }
       else if secs < 86400 { format!("{} hours", secs / 3600) }
       else { format!("{} days", secs / 86400) }
   }
   ```
3. Print the table with dynamic widths using `format!("{:<width$}", ...)` syntax.

---

## Task 17 — `crush export` should produce a real OCI tarball

**File:** `crates/crush-cli/src/main.rs`

**Problem:** `Commands::Export` currently calls `store.export_oci_tarball()` but
this may or may not be implemented in `crush-image`. Check
`crates/crush-image/src/lib.rs` for `export_oci_tarball`. If it's a stub, implement
it.

**What to do:**

In `crates/crush-image/src/lib.rs`, implement `export_oci_tarball`:

```rust
pub async fn export_oci_tarball(&self, image_id: &str, dest: &Path) -> Result<()> {
    let image = self.database().get_image(image_id).await?
        .ok_or_else(|| CrushError::ImageError(format!("Image not found: {}", image_id)))?;

    let file = std::fs::File::create(dest)
        .map_err(|e| CrushError::StorageError(e.to_string()))?;
    let mut tar = tar::Builder::new(flate2::write::GzEncoder::new(file, flate2::Compression::default()));

    // Write OCI layout marker
    let layout = r#"{"imageLayoutVersion":"1.0.0"}"#;
    let mut header = tar::Header::new_gnu();
    header.set_size(layout.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, "oci-layout", layout.as_bytes())
        .map_err(|e| CrushError::StorageError(e.to_string()))?;

    // Write each blob
    for layer_digest in &image.layers {
        let blob_path = self.blobs.blob_path(layer_digest);
        if blob_path.exists() {
            tar.append_path_with_name(&blob_path, format!("blobs/sha256/{}", &layer_digest[7..]))
                .map_err(|e| CrushError::StorageError(e.to_string()))?;
        }
    }

    // Write index.json
    let index = serde_json::json!({
        "schemaVersion": 2,
        "manifests": [{
            "mediaType": "application/vnd.oci.image.manifest.v1+json",
            "digest": image.digest,
            "size": 0,
            "annotations": { "org.opencontainers.image.ref.name": image.tag }
        }]
    });
    let index_bytes = serde_json::to_vec_pretty(&index)
        .map_err(|e| CrushError::StorageError(e.to_string()))?;
    let mut header = tar::Header::new_gnu();
    header.set_size(index_bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, "index.json", index_bytes.as_slice())
        .map_err(|e| CrushError::StorageError(e.to_string()))?;

    tar.finish().map_err(|e| CrushError::StorageError(e.to_string()))?;
    Ok(())
}
```

Add `tar = "0.4"` and `flate2 = "1"` to `crates/crush-image/Cargo.toml` if not
already present.

---

## Task 18 — Add `--platform` flag to `crush pull` and `crush build`

**File:** `crates/crush-cli/src/main.rs`

**Problem:** `crush pull` and `crush build` have no `--platform` argument. Pulling
multi-arch images always picks the host platform. For cross-builds (e.g.
building an arm64 image on x86_64) the user needs to specify a platform.

**What to do:**

1. Add `platform: Option<String>` to `PullArgs` and `BuildArgs` structs with
   `#[arg(long, default_value = None)]` and help text `"Target platform (e.g. linux/amd64, linux/arm64)"`.
2. In the pull handler, if `platform` is `Some`, pass it to `ImageStore::pull_image`
   via a new optional argument or by pre-setting it on the registry client.
3. Check `crates/crush-image/src/multiarch.rs` for how platform selection works
   and wire the CLI argument through.

---

## Task 19 — Version number: bump to 0.2.0

**File:** `Cargo.toml` (workspace root)

**What to do:**

Find the `[workspace.package]` section and change:
```toml
version = "0.1.0"
```
to:
```toml
version = "0.2.0"
```

All crates inherit `version.workspace = true` so this single change propagates.

---

## Task 20 — Integration test: `crush ps` after `crush run` on Linux

**File:** `crates/crush-cli/tests/integration_test.rs`

**Problem:** The test file exists but likely only tests basic CLI invocation.
Add a test that verifies the container lifecycle end-to-end on Linux.

**What to do:**

Add a test (Linux-only, `#[cfg(target_os = "linux")]`) that:
1. Creates a minimal rootfs in a `tempfile::TempDir` (just `bin/sh` symlinked
   from `/bin/sh` of the host, or use a busybox static binary if available).
2. Calls `run_container` directly from `crush_runtime_linux::runner` with a simple
   command like `["/bin/sh", "-c", "echo hello"]`.
3. Asserts exit code is 0.

This tests that the namespace + pivot_root + exec path works end-to-end without
needing a full image store.

---

## Build and release checklist

After completing all tasks above, run these commands on the Linux VPS:

```sh
# Ensure it compiles for Linux
cargo build --release -p crush-cli

# Ensure it cross-compiles for Windows
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
  cargo build --release --target x86_64-pc-windows-gnu -p crush-cli

# Run tests (Linux only)
cargo test --workspace

# Commit (no Co-Authored-By trailer)
git add -A
git commit -m "feat: crush v0.2.0 — runtime fixes, real build execution, Windows improvements"

# Tag and release
git tag v0.2.0
bash scripts/release.sh
```

---

## Priority order

If you can only do some tasks, do them in this order:

1. Task 1 (strip debug logging) — always do this before any release
2. Task 5 (build pipeline executes commands) — core feature
3. Task 2 + 3 (spawn correct process + PID persistence) — core feature
4. Task 4 (CPU weight conversion) — correctness
5. Task 8 (Windows run path) — cross-platform correctness
6. Task 13 (daemon start) — enables docker-context compatibility
7. Task 16 (ps output) — UX
8. Tasks 6, 7 (Windows stubs) — Windows parity
9. Tasks 9–12, 14–15 — improvements
10. Tasks 17–20 — polish
