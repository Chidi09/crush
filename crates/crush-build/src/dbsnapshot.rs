//! Auto-snapshot before destructive ORM migrations (R3.3).
//!
//! When the run-loop is about to execute a command that looks like a schema
//! migration (`prisma migrate`, `drizzle-kit push`, `flyway migrate`, …), we
//! first take a `pg_dump` of the crush-managed Postgres so a corrupting
//! migration is always one restore away. Best-effort: if Postgres isn't
//! reachable or `pg_dump` is missing, we skip silently — a migration must never
//! be blocked by the safety net.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Recognise a schema-migration command. Returns the tool name when matched so
/// callers can tag the snapshot and surface a message.
pub fn is_migration_command(cmd: &str) -> Option<&'static str> {
    let c = cmd.to_lowercase();
    // Each entry: (needle-substrings that must ALL be present, label).
    const PATTERNS: &[(&[&str], &str)] = &[
        (&["prisma", "migrate"], "prisma"),
        (&["prisma", "db", "push"], "prisma"),
        (&["drizzle-kit", "push"], "drizzle"),
        (&["drizzle-kit", "migrate"], "drizzle"),
        (&["flyway", "migrate"], "flyway"),
        (&["alembic", "upgrade"], "alembic"),
        (&["rails", "db:migrate"], "rails"),
        (&["rake", "db:migrate"], "rails"),
        (&["knex", "migrate"], "knex"),
        (&["sequelize", "db:migrate"], "sequelize"),
        (&["typeorm", "migration:run"], "typeorm"),
        (&["atlas", "migrate", "apply"], "atlas"),
        (&["goose", "up"], "goose"),
        (&["migrate", "-path"], "golang-migrate"),
        (&["dbmate", "up"], "dbmate"),
    ];
    PATTERNS.iter()
        .find(|(needles, _)| needles.iter().all(|n| c.contains(n)))
        .map(|(_, label)| *label)
}

/// Directory where auto snapshots are written (`~/.crush/backups`), matching the
/// GUI DB studio's backup list so they show up there too.
fn backups_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".crush").join("backups"))
}

/// Take a custom-format `pg_dump` of the local crush Postgres into
/// `~/.crush/backups/<tag>_<unixsecs>.dump`. Returns the file path on success.
///
/// Uses the same default credentials as the GUI auto-backup (localhost:5432,
/// postgres/postgres) — i.e. the crush-managed instance. External databases are
/// out of scope for the automatic safety net.
pub async fn snapshot_postgres(tag: &str) -> anyhow::Result<PathBuf> {
    let dir = backups_dir().ok_or_else(|| anyhow::anyhow!("no home dir"))?;
    std::fs::create_dir_all(&dir)?;

    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let file = dir.join(format!("{}_{}.dump", tag, secs));

    let pg_dump = if cfg!(target_os = "windows") { "pg_dump.exe" } else { "pg_dump" };
    let status = tokio::process::Command::new(pg_dump)
        .args([
            "-h", "localhost",
            "-p", "5432",
            "-U", "postgres",
            "-F", "c",
            "-f", &file.to_string_lossy(),
            "postgres",
        ])
        .env("PGPASSWORD", "postgres")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await?;

    if !status.success() {
        // Clean up the empty/partial file so it doesn't masquerade as a backup.
        let _ = std::fs::remove_file(&file);
        anyhow::bail!("pg_dump exited with {:?}", status.code());
    }
    Ok(file)
}

/// Whether the local crush Postgres is reachable — cheap gate before attempting
/// a dump, so we don't spawn `pg_dump` against a dead port.
pub async fn postgres_reachable() -> bool {
    tokio::net::TcpStream::connect("127.0.0.1:5432").await.is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_common_migration_commands() {
        assert_eq!(is_migration_command("npx prisma migrate dev"), Some("prisma"));
        assert_eq!(is_migration_command("pnpm prisma db push"), Some("prisma"));
        assert_eq!(is_migration_command("drizzle-kit push:pg"), Some("drizzle"));
        assert_eq!(is_migration_command("flyway -url=... migrate"), Some("flyway"));
        assert_eq!(is_migration_command("alembic upgrade head"), Some("alembic"));
        assert_eq!(is_migration_command("bundle exec rails db:migrate"), Some("rails"));
        assert_eq!(is_migration_command("knex migrate:latest"), Some("knex"));
    }

    #[test]
    fn ignores_non_migration_commands() {
        assert_eq!(is_migration_command("npm run dev"), None);
        assert_eq!(is_migration_command("next build"), None);
        assert_eq!(is_migration_command("prisma generate"), None); // generate ≠ migrate
        assert_eq!(is_migration_command("vite"), None);
    }
}
