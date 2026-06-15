//! `crush db` — database snapshots ("Time Machine").
//!
//! Wraps the native dump/restore tools crush already ships for its managed
//! databases (`pg_dump`/`pg_restore` for Postgres, `mysqldump`/`mysql` for
//! MySQL) so a developer can freeze a database state and jump back to it:
//!
//!   crush db snapshot cart-populated     # freeze current state
//!   crush db restore  cart-populated     # jump back to it later
//!   crush db ls                          # list snapshots for this project
//!   crush db rm cart-populated           # delete one
//!
//! The connection is resolved (in order) from `--url`, then the project's env
//! files (`DATABASE_URL`/`POSTGRES_URL`/`MYSQL_URL`/…), then a crush-managed
//! native service for the project. Snapshots live under
//! `<data_dir>/db-snapshots/<project>/`.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use anyhow::{anyhow, bail, Context, Result};
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Engine {
    Postgres,
    MySql,
}

impl Engine {
    fn as_str(&self) -> &'static str {
        match self {
            Engine::Postgres => "postgres",
            Engine::MySql => "mysql",
        }
    }
    /// File extension for this engine's dump format.
    fn ext(&self) -> &'static str {
        match self {
            Engine::Postgres => "dump", // pg custom format (-Fc)
            Engine::MySql => "sql",
        }
    }
}

/// Sidecar metadata stored next to each snapshot.
#[derive(Debug, Serialize, Deserialize)]
struct SnapshotMeta {
    name: String,
    engine: String,
    database: String,
    created_ms: u64,
    size_bytes: u64,
}

/// Entry point dispatched from main. `action` is the verb, `name`/`url` its
/// arguments.
pub async fn exec(
    action: &str,
    name: Option<String>,
    url: Option<String>,
    yes: bool,
    data_dir: PathBuf,
) -> Result<()> {
    let root = std::env::current_dir()?;
    let project = project_name(&root);
    let snap_dir = data_dir.join("db-snapshots").join(&project);

    match action {
        "snapshot" | "save" | "create" => {
            let name = name.ok_or_else(|| anyhow!("a snapshot name is required: {}", "crush db snapshot <name>".bold()))?;
            let conn = resolve_connection(url, &root, &project, &data_dir)?;
            snapshot(&snap_dir, &name, &conn).await
        }
        "restore" => {
            let name = name.ok_or_else(|| anyhow!("a snapshot name is required: {}", "crush db restore <name>".bold()))?;
            let conn = resolve_connection(url, &root, &project, &data_dir)?;
            restore(&snap_dir, &name, &conn, yes).await
        }
        "ls" | "list" | "snapshots" => list(&snap_dir, &project),
        "rm" | "delete" | "remove" => {
            let name = name.ok_or_else(|| anyhow!("a snapshot name is required: {}", "crush db rm <name>".bold()))?;
            remove(&snap_dir, &name)
        }
        other => bail!("unknown db action '{other}' — use snapshot, restore, ls, or rm"),
    }
}

// ── connection resolution ───────────────────────────────────────────────────

struct Connection {
    engine: Engine,
    url: String,
    database: String,
}

fn resolve_connection(
    explicit: Option<String>,
    root: &Path,
    project: &str,
    data_dir: &Path,
) -> Result<Connection> {
    if let Some(url) = explicit {
        return parse_connection(&url);
    }
    if let Some(url) = url_from_env_files(root) {
        return parse_connection(&url);
    }
    if let Some(url) = url_from_native_service(project, data_dir) {
        return parse_connection(&url);
    }
    bail!(
        "couldn't find a database to snapshot.\n   {} pass one with {} (e.g. postgresql://user:pass@localhost:5432/db),\n   {} or set DATABASE_URL in this project's .env, or start one with `crush services start postgres`.",
        "↳".cyan(), "--url".bold(), "↳".cyan()
    )
}

fn parse_connection(url: &str) -> Result<Connection> {
    let lower = url.trim();
    let engine = if lower.starts_with("postgres://") || lower.starts_with("postgresql://") {
        Engine::Postgres
    } else if lower.starts_with("mysql://") || lower.starts_with("mariadb://") {
        Engine::MySql
    } else {
        bail!("unsupported database URL '{url}' — only postgres:// and mysql:// are supported");
    };
    let database = database_from_url(lower).unwrap_or_default();
    if database.is_empty() {
        bail!("connection URL is missing a database name: {url}");
    }
    Ok(Connection { engine, url: lower.to_string(), database })
}

/// Extract the database name (path component) from a connection URL, stripping
/// any `?query` suffix.
fn database_from_url(url: &str) -> Option<String> {
    let after_scheme = url.split("://").nth(1)?;
    let path = after_scheme.split('/').nth(1)?; // host[:port] / db
    let db = path.split(['?', ';']).next().unwrap_or(path);
    if db.is_empty() { None } else { Some(db.to_string()) }
}

/// Look for a connection URL in the project's env files. First hit wins.
fn url_from_env_files(root: &Path) -> Option<String> {
    const KEYS: &[&str] = &[
        "DATABASE_URL", "POSTGRES_URL", "POSTGRESQL_URL", "PG_URL",
        "MYSQL_URL", "DATABASE_URI", "DB_URL",
    ];
    let files = [".env", ".env.local", ".env.development", ".env.development.local"];
    for f in &files {
        let path = root.join(f);
        let Ok(content) = std::fs::read_to_string(&path) else { continue };
        for line in content.lines() {
            let line = line.trim().trim_start_matches("export ").trim_start();
            if line.is_empty() || line.starts_with('#') { continue; }
            let Some((k, v)) = line.split_once('=') else { continue };
            let k = k.trim();
            let v = v.trim().trim_matches(['"', '\'']);
            if KEYS.contains(&k) && (v.starts_with("postgres") || v.starts_with("mysql") || v.starts_with("mariadb")) {
                return Some(v.to_string());
            }
        }
    }
    None
}

/// Build a connection URL from a crush-managed native DB service, if one is
/// recorded for this project.
fn url_from_native_service(project: &str, data_dir: &Path) -> Option<String> {
    use crush_services::{load_native_state, driver::ServiceKind};
    let state_dir = data_dir.join("services");
    // The run path stores under the normalized project name; try both.
    let candidates = [project.to_string(), project.replace('-', "_")];
    for key in candidates.iter() {
        let Some(state) = load_native_state(&state_dir, key) else { continue };
        for svc in &state.services {
            match svc.kind {
                ServiceKind::Postgres => {
                    let db = key.replace('-', "_");
                    return Some(format!("postgresql://{db}:{db}@localhost:{}/{db}", svc.port));
                }
                ServiceKind::MySQL => {
                    let db = key.replace('-', "_");
                    return Some(format!("mysql://root@localhost:{}/{db}", svc.port));
                }
                _ => {}
            }
        }
    }
    None
}

// ── operations ──────────────────────────────────────────────────────────────

async fn snapshot(snap_dir: &Path, name: &str, conn: &Connection) -> Result<()> {
    validate_name(name)?;
    std::fs::create_dir_all(snap_dir).context("creating snapshot directory")?;
    let file = snap_dir.join(format!("{name}.{}", conn.engine.ext()));

    println!(
        "{} snapshotting {} database {} → {}",
        "→".cyan(), conn.engine.as_str(), conn.database.bold(), name.bold()
    );

    match conn.engine {
        Engine::Postgres => {
            let pg_dump = find_pg_tool("pg_dump")?;
            // Custom format (-Fc): compact, restores with pg_restore + --clean.
            let status = Command::new(&pg_dump)
                .arg("-Fc")
                .arg("-d").arg(&conn.url)
                .arg("-f").arg(&file)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .await
                .with_context(|| format!("running {}", pg_dump.display()))?;
            if !status.success() {
                let _ = std::fs::remove_file(&file);
                bail!("pg_dump failed");
            }
        }
        Engine::MySql => {
            let mysqldump = find_on_path("mysqldump")
                .ok_or_else(|| anyhow!("`mysqldump` not found on PATH — install the MySQL client tools"))?;
            let my = MyConn::parse(&conn.url)?;
            let out = std::fs::File::create(&file).context("creating snapshot file")?;
            let status = my.apply(Command::new(&mysqldump).arg("--databases").arg(&conn.database))
                .stdout(Stdio::from(out))
                .stderr(Stdio::inherit())
                .status()
                .await
                .with_context(|| format!("running {}", mysqldump.display()))?;
            if !status.success() {
                let _ = std::fs::remove_file(&file);
                bail!("mysqldump failed");
            }
        }
    }

    let size = std::fs::metadata(&file).map(|m| m.len()).unwrap_or(0);
    write_meta(snap_dir, name, conn, size)?;
    println!(
        "{} saved snapshot {} {}",
        "✓".green().bold(), name.bold(), format!("({})", human_size(size)).dimmed()
    );
    println!("   {} restore later with: {}", "↳".cyan(), format!("crush db restore {name}").bold());
    Ok(())
}

async fn restore(snap_dir: &Path, name: &str, conn: &Connection, yes: bool) -> Result<()> {
    let file = snap_dir.join(format!("{name}.{}", conn.engine.ext()));
    if !file.exists() {
        bail!("no snapshot named '{name}' for this project — see `crush db ls`");
    }

    if !yes {
        println!(
            "{} this overwrites the current {} database {}.",
            "⚠".yellow().bold(), conn.engine.as_str(), conn.database.bold()
        );
        print!("   continue? [y/N] ");
        use std::io::Write;
        std::io::stdout().flush().ok();
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer).ok();
        if !matches!(answer.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("   {} aborted", "↳".cyan());
            return Ok(());
        }
    }

    println!("{} restoring {} → {}", "→".cyan(), name.bold(), conn.database.bold());

    match conn.engine {
        Engine::Postgres => {
            let pg_restore = find_pg_tool("pg_restore")?;
            // --clean --if-exists drops existing objects first so restore is a
            // true replacement, not a merge.
            let status = Command::new(&pg_restore)
                .arg("--clean").arg("--if-exists").arg("--no-owner")
                .arg("-d").arg(&conn.url)
                .arg(&file)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .await
                .with_context(|| format!("running {}", pg_restore.display()))?;
            if !status.success() {
                bail!("pg_restore failed");
            }
        }
        Engine::MySql => {
            let mysql = find_on_path("mysql")
                .ok_or_else(|| anyhow!("`mysql` not found on PATH — install the MySQL client tools"))?;
            let my = MyConn::parse(&conn.url)?;
            let input = std::fs::File::open(&file).context("opening snapshot file")?;
            let status = my.apply(&mut Command::new(&mysql))
                .stdin(Stdio::from(input))
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .await
                .with_context(|| format!("running {}", mysql.display()))?;
            if !status.success() {
                bail!("mysql restore failed");
            }
        }
    }

    println!("{} restored {}", "✓".green().bold(), name.bold());
    Ok(())
}

fn list(snap_dir: &Path, project: &str) -> Result<()> {
    let mut metas: Vec<SnapshotMeta> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(snap_dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&p) {
                    if let Ok(m) = serde_json::from_str::<SnapshotMeta>(&content) {
                        metas.push(m);
                    }
                }
            }
        }
    }
    if metas.is_empty() {
        println!("No database snapshots for {}.", project.bold());
        println!("   {} create one with {}", "↳".cyan(), "crush db snapshot <name>".bold());
        return Ok(());
    }
    metas.sort_by(|a, b| b.created_ms.cmp(&a.created_ms));
    println!("{} for {}", "Database snapshots".bold(), project.bold());
    for m in &metas {
        println!(
            "  {:<24} {:<9} {:<8} {}",
            m.name.bold(),
            m.engine.dimmed(),
            human_size(m.size_bytes).dimmed(),
            rel_time(m.created_ms).dimmed()
        );
    }
    println!("\n   {} restore: {}   delete: {}", "↳".cyan(), "crush db restore <name>".bold(), "crush db rm <name>".bold());
    Ok(())
}

fn remove(snap_dir: &Path, name: &str) -> Result<()> {
    let mut removed = false;
    for ext in ["dump", "sql", "json"] {
        let p = snap_dir.join(format!("{name}.{ext}"));
        if p.exists() {
            std::fs::remove_file(&p).ok();
            removed = true;
        }
    }
    if removed {
        println!("{} removed snapshot {}", "✓".green().bold(), name.bold());
    } else {
        println!("No snapshot named '{}' for this project.", name);
    }
    Ok(())
}

// ── helpers ─────────────────────────────────────────────────────────────────

fn write_meta(snap_dir: &Path, name: &str, conn: &Connection, size: u64) -> Result<()> {
    let meta = SnapshotMeta {
        name: name.to_string(),
        engine: conn.engine.as_str().to_string(),
        database: conn.database.clone(),
        created_ms: now_ms(),
        size_bytes: size,
    };
    std::fs::write(
        snap_dir.join(format!("{name}.json")),
        serde_json::to_string_pretty(&meta)?,
    )?;
    Ok(())
}

fn project_name(root: &Path) -> String {
    root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "project".into())
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() || name.contains(['/', '\\', '.', ':']) {
        bail!("invalid snapshot name '{name}' — use letters, numbers, dashes");
    }
    Ok(())
}

/// Locate a Postgres client tool (pg_dump/pg_restore/psql). Prefer one on PATH,
/// else derive it from crush's managed pg_ctl, else fall back to the bare name.
fn find_pg_tool(tool: &str) -> Result<PathBuf> {
    if let Some(p) = find_on_path(tool) {
        return Ok(p);
    }
    use crush_services::postgres::PostgresDriver;
    let driver = PostgresDriver::new(crush_types::dirs_or_default().join("cache"));
    if let Some(pg_ctl) = driver.find_pg_ctl_with_cache() {
        if let Some(dir) = pg_ctl.parent() {
            let exe = if cfg!(windows) { format!("{tool}.exe") } else { tool.to_string() };
            let candidate = dir.join(&exe);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }
    Err(anyhow!(
        "`{tool}` not found — install PostgreSQL client tools, or start crush's managed Postgres with `crush services start postgres`"
    ))
}

fn find_on_path(tool: &str) -> Option<PathBuf> {
    let names: Vec<String> = if cfg!(windows) {
        vec![format!("{tool}.exe"), tool.to_string()]
    } else {
        vec![tool.to_string()]
    };
    for n in names {
        if let Ok(p) = which::which(&n) {
            return Some(p);
        }
    }
    None
}

/// Parsed MySQL connection for the dump/restore CLI flags.
struct MyConn {
    host: String,
    port: u16,
    user: String,
    password: Option<String>,
}

impl MyConn {
    fn parse(url: &str) -> Result<Self> {
        // mysql://user:pass@host:port/db
        let rest = url.split("://").nth(1).ok_or_else(|| anyhow!("bad mysql URL"))?;
        let (auth_host, _db) = rest.split_once('/').unwrap_or((rest, ""));
        let (auth, hostport) = match auth_host.rsplit_once('@') {
            Some((a, h)) => (Some(a), h),
            None => (None, auth_host),
        };
        let (user, password) = match auth {
            Some(a) => match a.split_once(':') {
                Some((u, p)) => (u.to_string(), Some(p.to_string())),
                None => (a.to_string(), None),
            },
            None => ("root".to_string(), None),
        };
        let (host, port) = match hostport.split_once(':') {
            Some((h, p)) => (h.to_string(), p.parse().unwrap_or(3306)),
            None => (hostport.to_string(), 3306u16),
        };
        Ok(MyConn { host: if host.is_empty() { "localhost".into() } else { host }, port, user, password })
    }

    /// Apply host/port/user/password flags to a mysql/mysqldump command.
    fn apply<'a>(&self, cmd: &'a mut Command) -> &'a mut Command {
        cmd.arg(format!("--host={}", self.host))
            .arg(format!("--port={}", self.port))
            .arg(format!("--user={}", self.user));
        if let Some(pw) = &self.password {
            // mysql reads MYSQL_PWD to avoid a password on the command line.
            cmd.env("MYSQL_PWD", pw);
        }
        cmd
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

fn human_size(bytes: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut s = bytes as f64;
    let mut i = 0;
    while s >= 1024.0 && i < U.len() - 1 { s /= 1024.0; i += 1; }
    if i == 0 { format!("{bytes} B") } else { format!("{s:.1} {}", U[i]) }
}

fn rel_time(ms: u64) -> String {
    let now = now_ms();
    let secs = now.saturating_sub(ms) / 1000;
    if secs < 60 { format!("{secs}s ago") }
    else if secs < 3600 { format!("{}m ago", secs / 60) }
    else if secs < 86400 { format!("{}h ago", secs / 3600) }
    else { format!("{}d ago", secs / 86400) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_postgres_engine_and_db() {
        let c = parse_connection("postgresql://u:p@localhost:5432/appdb").unwrap();
        assert_eq!(c.engine, Engine::Postgres);
        assert_eq!(c.database, "appdb");
        assert_eq!(c.engine.ext(), "dump");
    }

    #[test]
    fn detects_mysql_engine_and_db() {
        let c = parse_connection("mysql://root:secret@127.0.0.1:3306/shop").unwrap();
        assert_eq!(c.engine, Engine::MySql);
        assert_eq!(c.database, "shop");
        assert_eq!(c.engine.ext(), "sql");
    }

    #[test]
    fn strips_query_string_from_db_name() {
        assert_eq!(
            database_from_url("postgresql://u:p@host:5432/appdb?sslmode=require").as_deref(),
            Some("appdb")
        );
    }

    #[test]
    fn rejects_unknown_scheme_and_missing_db() {
        assert!(parse_connection("sqlite:///x.db").is_err());
        assert!(parse_connection("postgresql://u:p@localhost:5432/").is_err());
    }

    #[test]
    fn parses_mysql_conn_parts() {
        let m = MyConn::parse("mysql://root:secret@db.example:3307/shop").unwrap();
        assert_eq!(m.host, "db.example");
        assert_eq!(m.port, 3307);
        assert_eq!(m.user, "root");
        assert_eq!(m.password.as_deref(), Some("secret"));
    }

    #[test]
    fn mysql_conn_defaults_when_minimal() {
        let m = MyConn::parse("mysql://localhost/shop").unwrap();
        assert_eq!(m.host, "localhost");
        assert_eq!(m.port, 3306);
        assert_eq!(m.user, "root");
        assert_eq!(m.password, None);
    }

    #[test]
    fn rejects_bad_snapshot_names() {
        assert!(validate_name("cart-populated").is_ok());
        assert!(validate_name("../etc/passwd").is_err());
        assert!(validate_name("a.b").is_err());
        assert!(validate_name("").is_err());
    }

    #[test]
    fn reads_database_url_from_env_file() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(".env"),
            "# comment\nexport DATABASE_URL=\"postgresql://u:p@localhost:5432/appdb\"\n",
        ).unwrap();
        assert_eq!(
            url_from_env_files(dir.path()).as_deref(),
            Some("postgresql://u:p@localhost:5432/appdb")
        );
    }

    #[test]
    fn human_size_scales() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(2048), "2.0 KB");
    }
}
