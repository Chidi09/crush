//! Curated catalog of popular images users commonly pull, so they're
//! discoverable from `crush catalog` and the GUI instead of having to remember
//! exact registry references. Shared by the CLI and the desktop app.

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    /// Friendly display name.
    pub name: &'static str,
    /// Pullable reference (what `crush pull` receives).
    pub reference: &'static str,
    /// Grouping for display: database, cache, search, storage, messaging,
    /// proxy, observability, tool.
    pub category: &'static str,
    /// One-line description.
    pub description: &'static str,
    /// True when crush already runs this natively as a managed service
    /// (`crush services`), so users often don't need the image at all.
    pub native: bool,
}

/// The curated set. Kept intentionally small and high-signal.
pub fn catalog() -> &'static [CatalogEntry] {
    &CATALOG
}

/// Case-insensitive filter over name / reference / description / category.
pub fn search(query: &str) -> Vec<&'static CatalogEntry> {
    let q = query.trim().to_lowercase();
    CATALOG.iter().filter(|e| {
        q.is_empty()
            || e.name.to_lowercase().contains(&q)
            || e.reference.to_lowercase().contains(&q)
            || e.description.to_lowercase().contains(&q)
            || e.category.to_lowercase().contains(&q)
    }).collect()
}

macro_rules! entry {
    ($name:expr, $reference:expr, $category:expr, $native:expr, $desc:expr) => {
        CatalogEntry { name: $name, reference: $reference, category: $category, native: $native, description: $desc }
    };
}

static CATALOG: [CatalogEntry; 28] = [
    // databases — most have native crush service equivalents
    entry!("PostgreSQL", "postgres:16", "database", true, "The default relational database."),
    entry!("MySQL", "mysql:8", "database", true, "Popular relational database."),
    entry!("MariaDB", "mariadb:11", "database", true, "Community MySQL fork."),
    entry!("MongoDB", "mongo:7", "database", true, "Document database."),
    entry!("SQLite (browser)", "coleifer/sqlite-web:latest", "database", false, "Web UI for SQLite files."),
    entry!("CockroachDB", "cockroachdb/cockroach:latest", "database", false, "Distributed SQL."),
    // cache / kv
    entry!("Redis", "redis:7", "cache", true, "In-memory data store."),
    entry!("Valkey", "valkey/valkey:8", "cache", true, "Open-source Redis fork."),
    entry!("Memcached", "memcached:1", "cache", false, "Distributed memory cache."),
    // search
    entry!("Elasticsearch", "elasticsearch:8.13.0", "search", false, "Full-text search engine."),
    entry!("Meilisearch", "getmeili/meilisearch:latest", "search", false, "Fast, typo-tolerant search."),
    entry!("Typesense", "typesense/typesense:latest", "search", false, "Lightweight search engine."),
    // storage
    entry!("MinIO", "minio/minio:latest", "storage", true, "S3-compatible object storage."),
    // messaging
    entry!("RabbitMQ", "rabbitmq:3-management", "messaging", false, "Message broker (with UI)."),
    entry!("NATS", "nats:latest", "messaging", false, "High-performance messaging."),
    entry!("Apache Kafka", "apache/kafka:latest", "messaging", false, "Distributed event streaming."),
    entry!("Mosquitto (MQTT)", "eclipse-mosquitto:2", "messaging", false, "Lightweight MQTT broker."),
    // proxy / web
    entry!("Nginx", "nginx:alpine", "proxy", false, "Web server / reverse proxy."),
    entry!("Caddy", "caddy:latest", "proxy", false, "Auto-HTTPS web server."),
    entry!("Traefik", "traefik:latest", "proxy", false, "Cloud-native reverse proxy."),
    entry!("FlareSolverr", "ghcr.io/flaresolverr/flaresolverr:latest", "tool", false, "Proxy that solves Cloudflare/anti-bot challenges."),
    // observability
    entry!("Grafana", "grafana/grafana:latest", "observability", false, "Dashboards & visualization."),
    entry!("Prometheus", "prom/prometheus:latest", "observability", false, "Metrics & monitoring."),
    entry!("Uptime Kuma", "louislam/uptime-kuma:1", "observability", false, "Self-hosted uptime monitor."),
    // tools
    entry!("Adminer", "adminer:latest", "tool", false, "Lightweight database UI."),
    entry!("n8n", "n8nio/n8n:latest", "tool", false, "Workflow automation."),
    entry!("Gitea", "gitea/gitea:latest", "tool", false, "Self-hosted Git service."),
    entry!("Vaultwarden", "vaultwarden/server:latest", "tool", false, "Self-hosted Bitwarden server."),
];
