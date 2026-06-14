//! Compose-honor test: proves crush parses a real, production-grade
//! `docker-compose.yml` and exposes every field the run path needs to honor it.
//!
//! This is the schema-and-ordering half of the "work with Docker users without
//! Docker" guarantee: if a colleague hands you their compose file, crush must
//! understand all of it. (The execution half — that `crush compose up` actually
//! applies these — is covered by the helper unit tests in crush-cli and, end to
//! end, by a docker-load conformance test.)

use std::path::Path;
use crush_compat::ComposeParser;

/// The exact production compose file from the Docker reference: web + backend +
/// postgres + redis, with ports, volumes, networks, depends_on, deploy limits,
/// restart policies, build context, and a healthcheck.
const PROD_COMPOSE: &str = r#"
version: '3.8'

services:
  web:
    image: nginx:latest
    container_name: production_web
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - web_data:/usr/share/nginx/html
    networks:
      - frontend_network
    depends_on:
      - app_backend
    restart: always

  app_backend:
    build:
      context: ./backend
      dockerfile: Dockerfile
    container_name: production_api
    environment:
      NODE_ENV: production
      DB_HOST: postgres_db
      CACHE_HOST: redis_cache
    command: node server.js
    entrypoint: ["/usr/bin/tini", "--"]
    ports:
      - "3000:3000"
    networks:
      - frontend_network
      - backend_network
    depends_on:
      - postgres_db
      - redis_cache
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 5
    deploy:
      resources:
        limits:
          cpus: '0.50'
          memory: 512M
    restart: on-failure

  postgres_db:
    image: postgres:15
    container_name: production_db
    environment:
      POSTGRES_USER: admin
      POSTGRES_PASSWORD: securepassword
      POSTGRES_DB: main_db
    volumes:
      - db_data:/var/lib/postgresql/data
    networks:
      - backend_network
    restart: unless-stopped

  redis_cache:
    image: redis:alpine
    container_name: production_cache
    ports:
      - "6379:6379"
    networks:
      - backend_network
    restart: always

volumes:
  web_data:
    driver: local
  db_data:
    driver: local

networks:
  frontend_network:
    driver: bridge
  backend_network:
    driver: bridge
"#;

fn parse() -> crush_compat::ComposeV2 {
    ComposeParser::new()
        .parse(PROD_COMPOSE, Path::new("docker-compose.yml"))
        .expect("production compose file should parse")
}

#[test]
fn parses_top_level_sections() {
    let c = parse();
    assert_eq!(c.version.as_deref(), Some("3.8"));
    let services = c.services.as_ref().expect("services present");
    assert_eq!(services.len(), 4);
    assert!(c.networks.as_ref().unwrap().contains_key("frontend_network"));
    assert!(c.networks.as_ref().unwrap().contains_key("backend_network"));
    assert!(c.volumes.as_ref().unwrap().contains_key("web_data"));
    assert!(c.volumes.as_ref().unwrap().contains_key("db_data"));
}

#[test]
fn parses_image_ports_volumes_networks_restart() {
    let c = parse();
    let services = c.services.unwrap();
    let web = &services["web"];

    assert_eq!(web.image.as_deref(), Some("nginx:latest"));
    assert_eq!(web.container_name.as_deref(), Some("production_web"));
    assert_eq!(web.ports.as_ref().unwrap(), &vec!["80:80".to_string(), "443:443".to_string()]);
    assert_eq!(web.volumes.as_ref().unwrap(), &vec!["web_data:/usr/share/nginx/html".to_string()]);
    assert_eq!(web.networks.as_ref().unwrap(), &vec!["frontend_network".to_string()]);
    assert_eq!(web.restart.as_deref(), Some("always"));
}

#[test]
fn parses_build_environment_command_entrypoint() {
    let c = parse();
    let services = c.services.unwrap();
    let backend = &services["app_backend"];

    // build: { context, dockerfile }
    let build = backend.build.as_ref().expect("build present");
    assert_eq!(build.get("context").and_then(|v| v.as_str()), Some("./backend"));
    assert_eq!(build.get("dockerfile").and_then(|v| v.as_str()), Some("Dockerfile"));

    // environment (map form)
    let env = backend.environment.as_ref().unwrap();
    assert_eq!(env.get("NODE_ENV").and_then(|v| v.as_str()), Some("production"));
    assert_eq!(env.get("DB_HOST").and_then(|v| v.as_str()), Some("postgres_db"));

    // command (string form) and entrypoint (array form) — both override the image's
    assert_eq!(backend.command.as_ref().unwrap().as_str(), Some("node server.js"));
    let entry = backend.entrypoint.as_ref().unwrap().as_array().unwrap();
    assert_eq!(entry[0].as_str(), Some("/usr/bin/tini"));
}

#[test]
fn parses_healthcheck_and_deploy_limits() {
    let c = parse();
    let services = c.services.unwrap();
    let backend = &services["app_backend"];

    let hc = backend.healthcheck.as_ref().expect("healthcheck present");
    let test = hc.test.as_ref().unwrap().as_array().unwrap();
    assert_eq!(test[0].as_str(), Some("CMD"));
    assert_eq!(test[1].as_str(), Some("curl"));
    assert_eq!(hc.interval.as_deref(), Some("30s"));
    assert_eq!(hc.timeout.as_deref(), Some("10s"));
    assert_eq!(hc.retries, Some(5));

    let limits = backend.deploy.as_ref().unwrap()
        .resources.as_ref().unwrap()
        .limits.as_ref().unwrap();
    assert_eq!(limits.cpus.as_deref(), Some("0.50"));
    assert_eq!(limits.memory.as_deref(), Some("512M"));
}

#[test]
fn dependency_order_respects_depends_on() {
    let c = parse();
    let order = ComposeParser::get_dependency_order(&c).expect("dependency order resolvable");

    let pos = |name: &str| order.iter().position(|s| s == name)
        .unwrap_or_else(|| panic!("{} missing from order: {:?}", name, order));

    // postgres_db and redis_cache must start before app_backend, which starts before web.
    assert!(pos("postgres_db") < pos("app_backend"), "db before backend: {:?}", order);
    assert!(pos("redis_cache") < pos("app_backend"), "cache before backend: {:?}", order);
    assert!(pos("app_backend") < pos("web"), "backend before web: {:?}", order);
}
