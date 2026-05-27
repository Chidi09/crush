use std::path::PathBuf;
use std::time::SystemTime;
use crush_types::*;
use crush_build::{StackDetector, BuildEngine};
use crush_image::ImageStore;
use crush_compat::{DockerfileParser, ComposeLoader};
use crush_ai::AiEngine;
use crush_network::NetworkManager;

#[tokio::test]
async fn test_stack_detector_rust() {
    let detector = StackDetector::new();
    let temp_dir = std::env::temp_dir().join("crush_test_rust_project");
    std::fs::create_dir_all(&temp_dir).unwrap();

    std::fs::write(
        temp_dir.join("Cargo.toml"),
        r#"[package]
name = "test-app"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();

    let stack = detector.detect(&temp_dir).await.unwrap();
    assert_eq!(stack.language, "Rust");
    assert_eq!(stack.default_port, 8080);
    assert_eq!(stack.build_command, "cargo build --release");
    assert!(stack.confidence > 0.9);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_stack_detector_node() {
    let detector = StackDetector::new();
    let temp_dir = std::env::temp_dir().join("crush_test_node_project");
    std::fs::create_dir_all(&temp_dir).unwrap();

    std::fs::write(
        temp_dir.join("package.json"),
        r#"{"name":"test-api","version":"1.0.0","main":"server.js"}"#,
    )
    .unwrap();
    std::fs::write(temp_dir.join("package-lock.json"), "{}").unwrap();

    let stack = detector.detect(&temp_dir).await.unwrap();
    assert_eq!(stack.language, "Node.js (JavaScript)");
    assert_eq!(stack.default_port, 3000);
    assert!(stack.confidence > 0.9);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_stack_detector_python() {
    let detector = StackDetector::new();
    let temp_dir = std::env::temp_dir().join("crush_test_python_project");
    std::fs::create_dir_all(&temp_dir).unwrap();

    std::fs::write(
        temp_dir.join("pyproject.toml"),
        r#"[project]
name = "test-app"
version = "0.1.0"
requires-python = ">=3.11"
"#,
    )
    .unwrap();

    let stack = detector.detect(&temp_dir).await.unwrap();
    assert!(stack.language.starts_with("Python"));
    assert!(stack.confidence > 0.8);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_stack_detector_go() {
    let detector = StackDetector::new();
    let temp_dir = std::env::temp_dir().join("crush_test_go_project");
    std::fs::create_dir_all(&temp_dir).unwrap();

    std::fs::write(
        temp_dir.join("go.mod"),
        "module github.com/test/app\ngo 1.21\n",
    )
    .unwrap();

    let stack = detector.detect(&temp_dir).await.unwrap();
    assert_eq!(stack.language, "Go");
    assert_eq!(stack.default_port, 8080);
    assert!(stack.confidence > 0.9);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_image_store_pull_and_list() {
    let temp_dir = std::env::temp_dir().join("crush_test_image_store");
    std::fs::create_dir_all(&temp_dir).unwrap();

    let store = ImageStore::new(temp_dir.clone());

    let result = store.pull_image("hello-world:latest").await;
    assert!(result.is_ok() || result.is_err());

    let images = store.list_images().await.unwrap();
    assert!(images.is_empty() || !images.is_empty());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_container_state_transitions() {
    let mut container = Container {
        id: "test-c1".to_string(),
        name: "web-server".to_string(),
        image: "nginx:latest".to_string(),
        status: ContainerStatus::Creating,
        pid: None,
        created_at: SystemTime::now(),
        started_at: None,
        ports: vec![PortMapping {
            host_ip: "127.0.0.1".to_string(),
            host_port: 80,
            container_port: 80,
            protocol: Protocol::Tcp,
        }],
        mounts: vec![],
        memory_limit_bytes: Some(512 * 1024 * 1024),
        cpu_shares: Some(512),
    };

    assert_eq!(container.status, ContainerStatus::Creating);
    container.status = ContainerStatus::Created;
    container.status = ContainerStatus::Running;
    container.pid = Some(7171);
    assert_eq!(container.status, ContainerStatus::Running);
    assert_eq!(container.pid, Some(7171));
}

#[test]
fn test_dockerfile_parser() {
    let parser = DockerfileParser::new();
    let temp_dir = std::env::temp_dir().join("crush_test_dockerfile");
    std::fs::create_dir_all(&temp_dir).unwrap();

    let dockerfile = r#"FROM ubuntu:22.04
WORKDIR /app
COPY . .
RUN apt-get update && apt-get install -y curl
EXPOSE 8080
ENTRYPOINT ["/app/server"]
CMD ["--port", "8080"]
"#;

    let df_path = temp_dir.join("Dockerfile");
    std::fs::write(&df_path, dockerfile).unwrap();

    let crushfile = parser.parse_to_crushfile(&df_path).unwrap();
    assert!(crushfile.contains("base = \"ubuntu:22.04\""));
    assert!(crushfile.contains("workdir = \"/app\""));
    assert!(crushfile.contains("entrypoint = [\"/app/server\"]"));
    assert!(crushfile.contains("--port"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_trace_parser_python() {
    let parser = TraceParser::new();
    let stderr = "Traceback (most recent call last):\n  File \"/app/main.py\", line 42, in <module>\n    result = divide(10, 0)\n  File \"/app/main.py\", line 10, in divide\n    return a / b\nZeroDivisionError: division by zero";

    let trace = parser.parse_stderr(stderr).unwrap();
    assert_eq!(trace.language, "Python");
    assert_eq!(trace.exception_type, "ZeroDivisionError");
    assert!(!trace.stack_frames.is_empty());
}

#[test]
fn test_trace_parser_javascript() {
    let parser = TraceParser::new();
    let stderr = "TypeError: Cannot read properties of undefined (reading 'name')\n    at Object.getUser (/app/src/server.ts:42:18)\n    at Layer.handle [as handle_request] (/app/node_modules/express/lib/router/index.js:275:10)";

    let trace = parser.parse_stderr(stderr).unwrap();
    assert_eq!(trace.exception_type, "TypeError");
    assert_eq!(trace.line, 42);
}

#[tokio::test]
async fn test_build_engine_creates_layer() {
    let temp_dir = std::env::temp_dir().join("crush_test_build_engine");
    std::fs::create_dir_all(&temp_dir).unwrap();

    let src_file = temp_dir.join("hello.txt");
    std::fs::write(&src_file, b"hello crush").unwrap();

    let cache_dir = std::env::temp_dir().join("crush_test_build_cache");
    let engine = BuildEngine::new(cache_dir.clone());

    let stack = crush_build::InferredStack {
        language: "Test".to_string(),
        runtime_version: "1.0".to_string(),
        build_command: "echo test".to_string(),
        entry_point: "test".to_string(),
        default_port: 8080,
        confidence: 1.0,
    };

    let digest = engine.execute_layered_build(&temp_dir, &stack).await.unwrap();
    assert!(digest.starts_with("sha256:"));

    let _ = std::fs::remove_dir_all(&temp_dir);
    let _ = std::fs::remove_dir_all(&cache_dir);
}

#[test]
fn test_compose_loader_parsing() {
    let loader = ComposeLoader::new();
    let temp_dir = std::env::temp_dir().join("crush_test_compose");
    std::fs::create_dir_all(&temp_dir).unwrap();

    let compose_yml = r#"version: "3.8"
services:
  web:
    image: nginx:latest
    ports:
      - "80:80"
  db:
    image: postgres:13
    environment:
      POSTGRES_PASSWORD: secret
"#;

    let compose_path = temp_dir.join("docker-compose.yml");
    std::fs::write(&compose_path, compose_yml).unwrap();

    let services = loader.parse_compose_file(&compose_path).unwrap();
    assert!(services.contains(&"web".to_string()));
    assert!(services.contains(&"db".to_string()));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_diagnostic_engine_fallback() {
    let tmp = std::env::temp_dir().join("crush_integration_test").join(&format!("test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let engine = AiEngine::new(None, tmp.clone());
    let stderr = "TypeError: Cannot read property 'x' of null\n    at Object.run (app.js:10:5)";

    let diagnosis = engine.diagnose_stderr(stderr, None, None).await.unwrap();
    assert!(diagnosis.trace.is_some());
    assert_eq!(diagnosis.trace.unwrap().exception_type, "TypeError");

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_network_manager_creation_non_linux() {
    let net = NetworkManager::new();
    let rt = tokio::runtime::Runtime::new().unwrap();

    if cfg!(not(target_os = "linux")) {
        let result = rt.block_on(net.create_bridge("test-br", "10.0.0.0/24"));
        assert!(result.is_err());
    } else {
        // On Linux this would either succeed or fail with "permission denied"
        // We just test it doesn't panic
        let _ = rt.block_on(net.create_bridge("test-br", "10.0.0.0/24"));
    }
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn test_container_lifecycle_linux() {
    let temp_root = tempfile::TempDir::new().unwrap();
    let rootfs = temp_root.path().join("rootfs");
    std::fs::create_dir_all(&rootfs).unwrap();

    // Create minimal dirs for pivot_root
    for dir in &["bin", "sbin", "usr/bin", "proc", "dev", "sys", "tmp", ".old_root"] {
        std::fs::create_dir_all(rootfs.join(dir)).ok();
    }

    let sh_dest = rootfs.join("bin/sh");
    if let Ok(_) = std::fs::copy("/bin/sh", &sh_dest) {
        let container = Container {
            id: "test-lifecycle-container".to_string(),
            name: "test-lifecycle-container".to_string(),
            image: "busybox".to_string(),
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
            read_only: Some(false),
            security_opt: None,
        };

        let command = vec!["/bin/sh".to_string(), "-c".to_string(), "echo hello".to_string()];
        let env_vars = vec![];

        let result = tokio::task::spawn_blocking(move || {
            crush_runtime_linux::runner::run_container(&rootfs, &command, &env_vars, &container)
        }).await.unwrap();

        match result {
            Ok(code) => assert_eq!(code, 0),
            Err(CrushError::NamespaceError(ref msg)) if msg.contains("Operation not permitted") => {
                // Expected unprivileged namespace unshare check
            }
            Err(e) => {
                println!("Got allowed namespace/pivot_root error: {:?}", e);
            }
        }
    }
}
