//! Auto blue-green deployment.
//!
//! Zero-downtime releases without the operator designing anything: crush picks
//! the idle color, brings the new release up *beside* the old one on its own
//! loopback port, health-checks it, and only then flips the gateway's target
//! file ([`crate::gateway`] on the host re-reads it per connection). The old
//! release drains and is retired. If the new release fails its health check,
//! nothing flips — the old one keeps serving and the new one is removed.
//!
//! The mechanics are expressed against [`BlueGreenOps`] so the orchestration is
//! host-agnostic (SSH today) and unit-testable with a fake host.

use std::path::Path;
use async_trait::async_trait;

/// Which release slot is live. Deploys alternate between the two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Blue,
    Green,
}

impl Color {
    pub fn as_str(&self) -> &'static str {
        match self {
            Color::Blue => "blue",
            Color::Green => "green",
        }
    }
    pub fn other(&self) -> Color {
        match self {
            Color::Blue => Color::Green,
            Color::Green => Color::Blue,
        }
    }
    pub fn from_str(s: &str) -> Option<Color> {
        match s.trim().to_lowercase().as_str() {
            "blue" => Some(Color::Blue),
            "green" => Some(Color::Green),
            _ => None,
        }
    }
}

/// The computed deploy plan — what crush will do, derived from current state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Slot the new release goes into.
    pub new_color: Color,
    /// Container/app name for the new release, e.g. `myapp-green`.
    pub new_container: String,
    /// Loopback port the new release listens on.
    pub new_port: u16,
    /// The currently-live container to retire after the flip (None on first deploy).
    pub old_container: Option<String>,
    /// Public port the gateway listens on.
    pub public_port: u16,
    /// Host path of the gateway target file to flip.
    pub target_file: String,
}

/// Loopback port for a release slot. Blue/green get distinct ports so both can
/// run during the swap. Derived from the public port, clamped so `+2` can't wrap.
pub fn internal_port(public_port: u16, color: Color) -> u16 {
    let base = public_port.min(60000);
    match color {
        Color::Blue => base + 1,
        Color::Green => base + 2,
    }
}

/// Host path of a project's gateway target file.
pub fn target_file_path(project: &str) -> String {
    format!("/var/lib/crush/gateway/{project}.target")
}

/// Compute the next deploy plan from the currently-live color.
pub fn plan(project: &str, public_port: u16, current: Option<Color>) -> Plan {
    let new_color = match current {
        Some(c) => c.other(),
        None => Color::Blue, // first deploy lands on blue
    };
    Plan {
        new_color,
        new_container: format!("{project}-{}", new_color.as_str()),
        new_port: internal_port(public_port, new_color),
        old_container: current.map(|c| format!("{project}-{}", c.as_str())),
        public_port,
        target_file: target_file_path(project),
    }
}

/// Host operations the orchestrator drives. Implemented for SSH; faked in tests.
#[async_trait]
pub trait BlueGreenOps {
    /// Which color is currently live, if any.
    async fn current_color(&self, project: &str) -> anyhow::Result<Option<Color>>;
    /// Load an image archive onto the host; return a reference to run.
    async fn load_image(&self, image_tar: &Path) -> anyhow::Result<String>;
    /// Start `container` from `image_ref` listening on loopback `port`.
    async fn run_release(&self, container: &str, image_ref: &str, port: u16, env: &[String]) -> anyhow::Result<()>;
    /// True when the release on `port` answers `health_path` (HTTP 2xx/3xx).
    async fn health_check(&self, port: u16, health_path: &str) -> anyhow::Result<bool>;
    /// Make sure the public gateway is running and pointed at `target_file`.
    async fn ensure_gateway(&self, public_port: u16, target_file: &str) -> anyhow::Result<()>;
    /// Atomically flip the gateway's upstream to `port`.
    async fn switch_gateway(&self, target_file: &str, port: u16) -> anyhow::Result<()>;
    /// Stop a release container (drains existing connections first).
    async fn stop_release(&self, container: &str) -> anyhow::Result<()>;
    /// Remove a stopped release container.
    async fn remove_release(&self, container: &str) -> anyhow::Result<()>;
    /// Sleep `secs` (overridable so tests don't actually wait).
    async fn sleep_secs(&self, secs: u64) {
        tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
    }
}

/// Tunables for a blue-green run.
#[derive(Debug, Clone)]
pub struct Options {
    pub health_path: String,
    pub health_retries: u32,
    pub health_interval_secs: u64,
    pub drain_secs: u64,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            health_path: "/".to_string(),
            health_retries: 20,
            health_interval_secs: 2,
            drain_secs: 5,
        }
    }
}

/// What happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub new_color: Color,
    pub retired: Option<String>,
    pub flipped: bool,
}

/// Run one blue-green deploy. On health-check failure the new release is removed
/// and the old one keeps serving (`Err`). Never leaves both colors live.
pub async fn execute<H: BlueGreenOps + Sync>(
    ops: &H,
    project: &str,
    public_port: u16,
    image_tar: &Path,
    env: &[String],
    opts: &Options,
) -> anyhow::Result<Outcome> {
    let current = ops.current_color(project).await?;
    let plan = plan(project, public_port, current);

    let image_ref = ops.load_image(image_tar).await?;
    ops.run_release(&plan.new_container, &image_ref, plan.new_port, env).await?;

    // Health-gate the new release before any traffic sees it.
    let mut healthy = false;
    for attempt in 0..opts.health_retries.max(1) {
        if ops.health_check(plan.new_port, &opts.health_path).await.unwrap_or(false) {
            healthy = true;
            break;
        }
        if attempt + 1 < opts.health_retries.max(1) {
            ops.sleep_secs(opts.health_interval_secs).await;
        }
    }

    if !healthy {
        // Roll back: tear down the new release, leave the old one untouched.
        let _ = ops.stop_release(&plan.new_container).await;
        let _ = ops.remove_release(&plan.new_container).await;
        anyhow::bail!(
            "new release {} failed health check on :{} — rolled back, {} still serving",
            plan.new_container,
            plan.new_port,
            plan.old_container.as_deref().unwrap_or("(none)")
        );
    }

    // Flip traffic atomically.
    ops.ensure_gateway(plan.public_port, &plan.target_file).await?;
    ops.switch_gateway(&plan.target_file, plan.new_port).await?;

    // Drain and retire the old release.
    let mut retired = None;
    if let Some(old) = &plan.old_container {
        ops.sleep_secs(opts.drain_secs).await;
        let _ = ops.stop_release(old).await;
        let _ = ops.remove_release(old).await;
        retired = Some(old.clone());
    }

    Ok(Outcome { new_color: plan.new_color, retired, flipped: true })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn first_deploy_lands_on_blue() {
        let p = plan("app", 80, None);
        assert_eq!(p.new_color, Color::Blue);
        assert_eq!(p.new_container, "app-blue");
        assert_eq!(p.old_container, None);
        assert_eq!(p.new_port, internal_port(80, Color::Blue));
    }

    #[test]
    fn second_deploy_flips_to_green_and_retires_blue() {
        let p = plan("app", 80, Some(Color::Blue));
        assert_eq!(p.new_color, Color::Green);
        assert_eq!(p.new_container, "app-green");
        assert_eq!(p.old_container.as_deref(), Some("app-blue"));
    }

    #[test]
    fn blue_and_green_get_distinct_ports() {
        assert_ne!(internal_port(80, Color::Blue), internal_port(80, Color::Green));
        // clamp prevents +2 wrap near u16::MAX
        let _ = internal_port(65535, Color::Green);
    }

    // A fake host that records the order of operations and can be told to fail
    // health checks.
    #[derive(Default)]
    struct FakeHost {
        current: Option<Color>,
        healthy: bool,
        log: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl BlueGreenOps for FakeHost {
        async fn current_color(&self, _project: &str) -> anyhow::Result<Option<Color>> {
            Ok(self.current)
        }
        async fn load_image(&self, _t: &Path) -> anyhow::Result<String> {
            self.log.lock().unwrap().push("load".into());
            Ok("img:latest".into())
        }
        async fn run_release(&self, c: &str, _i: &str, port: u16, _e: &[String]) -> anyhow::Result<()> {
            self.log.lock().unwrap().push(format!("run {c}:{port}"));
            Ok(())
        }
        async fn health_check(&self, _port: u16, _p: &str) -> anyhow::Result<bool> {
            Ok(self.healthy)
        }
        async fn ensure_gateway(&self, port: u16, _f: &str) -> anyhow::Result<()> {
            self.log.lock().unwrap().push(format!("gateway {port}"));
            Ok(())
        }
        async fn switch_gateway(&self, _f: &str, port: u16) -> anyhow::Result<()> {
            self.log.lock().unwrap().push(format!("switch {port}"));
            Ok(())
        }
        async fn stop_release(&self, c: &str) -> anyhow::Result<()> {
            self.log.lock().unwrap().push(format!("stop {c}"));
            Ok(())
        }
        async fn remove_release(&self, c: &str) -> anyhow::Result<()> {
            self.log.lock().unwrap().push(format!("remove {c}"));
            Ok(())
        }
        async fn sleep_secs(&self, _s: u64) {} // no waiting in tests
    }

    #[tokio::test]
    async fn happy_path_flips_then_retires_old() {
        let host = FakeHost { current: Some(Color::Blue), healthy: true, ..Default::default() };
        let out = execute(&host, "app", 80, Path::new("/tmp/x.tar"), &[], &Options::default()).await.unwrap();
        assert_eq!(out.new_color, Color::Green);
        assert_eq!(out.retired.as_deref(), Some("app-blue"));
        assert!(out.flipped);
        let log = host.log.lock().unwrap().clone();
        // new release runs, then switch happens, then old is stopped — in order.
        let run_i = log.iter().position(|l| l.starts_with("run app-green")).unwrap();
        let switch_i = log.iter().position(|l| l.starts_with("switch")).unwrap();
        let stop_i = log.iter().position(|l| l == "stop app-blue").unwrap();
        assert!(run_i < switch_i && switch_i < stop_i, "order wrong: {log:?}");
    }

    #[tokio::test]
    async fn unhealthy_rolls_back_without_flipping() {
        let host = FakeHost { current: Some(Color::Blue), healthy: false, ..Default::default() };
        let err = execute(&host, "app", 80, Path::new("/tmp/x.tar"), &[], &Options { health_retries: 2, ..Default::default() }).await.unwrap_err();
        assert!(err.to_string().contains("rolled back"));
        let log = host.log.lock().unwrap().clone();
        assert!(log.iter().any(|l| l == "remove app-green"), "new release not cleaned up: {log:?}");
        assert!(!log.iter().any(|l| l.starts_with("switch")), "must not flip when unhealthy: {log:?}");
        assert!(!log.iter().any(|l| l == "stop app-blue"), "must not touch old when unhealthy: {log:?}");
    }
}
