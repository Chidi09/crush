use std::path::PathBuf;
use std::time::SystemTime;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use async_trait::async_trait;

pub fn dirs_or_default() -> PathBuf {
    let base = if cfg!(target_os = "linux") {
        if let Ok(env_dir) = std::env::var("CRUSH_DATA_DIR") {
            PathBuf::from(env_dir)
        } else {
            let var_lib = PathBuf::from("/var/lib/crush");
            let test_dir = var_lib.join(".access_test");
            if std::fs::create_dir_all(&test_dir).is_ok() {
                let _ = std::fs::remove_dir(&test_dir);
                var_lib
            } else {
                dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("crush")
            }
        }
    } else if cfg!(target_os = "windows") {
        let local_app_data = std::env::var("LOCALAPPDATA")
            .unwrap_or_else(|_| format!("{}\\AppData\\Local",
                std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Default".to_string())));
        PathBuf::from(local_app_data).join("Crush")
    } else {
        dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("crush")
    };
    std::fs::create_dir_all(&base).ok();
    base
}

#[derive(Error, Debug)]
pub enum CrushError {
    #[error("Namespace error: {0}")]
    NamespaceError(String),

    #[error("Cgroup error: {0}")]
    CgroupError(String),

    #[error("Seccomp profile error: {0}")]
    SeccompError(String),

    #[error("Overlay filesystem error: {0}")]
    StorageError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Container not found: {0}")]
    ContainerNotFound(String),

    #[error("Container already exists: {0}")]
    ContainerAlreadyExists(String),

    #[error("Invalid container state transition from {from:?} to {to:?}")]
    InvalidStateTransition {
        from: ContainerStatus,
        to: ContainerStatus,
    },

    #[error("Image error: {0}")]
    ImageError(String),

    #[error("OCI specification error: {0}")]
    OciSpecError(String),

    #[error("WASM runtime error: {0}")]
    WasmError(String),

    #[error("AI intelligence diagnostic failure: {0}")]
    AiError(String),

    #[error("CLI argument error: {0}")]
    CliError(String),

    #[error("API server error: {0}")]
    ApiError(String),

    #[error("Internal dynamic error: {0}")]
    Internal(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, CrushError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerStatus {
    Creating,
    Created,
    Running,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Starting,
    Healthy,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: ContainerStatus,
    pub pid: Option<u32>,
    pub created_at: SystemTime,
    pub started_at: Option<SystemTime>,
    pub ports: Vec<PortMapping>,
    pub mounts: Vec<MountConfig>,
    pub memory_limit_bytes: Option<u64>,
    pub cpu_shares: Option<u64>,
    pub health: Option<HealthStatus>,
    pub restart_count: Option<u32>,
    pub restart_policy: Option<String>,
    pub health_cmd: Option<String>,
    pub health_interval: Option<u64>,
    pub health_timeout: Option<u64>,
    pub health_retries: Option<u32>,
    pub pids_limit: Option<u32>,
    pub read_only: Option<bool>,
    pub security_opt: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub host_ip: String,
    pub host_port: u16,
    pub container_port: u16,
    pub protocol: Protocol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Protocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountConfig {
    pub host_path: PathBuf,
    pub container_path: PathBuf,
    pub read_only: bool,
    pub is_tmpfs: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub id: String,
    pub tag: String,
    pub digest: String,
    pub size_bytes: u64,
    pub layers: Vec<String>,
    pub architecture: String,
    pub os: String,
    /// OCI `os.version` — required for Windows images (host kernel build must match);
    /// `None` for Linux images, where it is absent from the config.
    #[serde(default)]
    pub os_version: Option<String>,
    #[serde(default)]
    pub entrypoint: Vec<String>,
    #[serde(default)]
    pub cmd: Vec<String>,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub config_digest: Option<String>,
    /// Icon/stack hint for the UI (e.g. "node", "python", "redis") — derived from
    /// the base image for crushed images, or the image name for pulled ones.
    /// `None` falls back to a generated identicon.
    #[serde(default)]
    pub stack: Option<String>,
}

#[async_trait]
pub trait RuntimeBackend: Send + Sync {
    async fn create(&self, container: &Container, spec_path: &PathBuf) -> Result<()>;
    async fn start(&self, container_id: &str) -> Result<()>;
    async fn stop(&self, container_id: &str, timeout_seconds: u32) -> Result<()>;
    async fn pause(&self, container_id: &str) -> Result<()>;
    async fn resume(&self, container_id: &str) -> Result<()>;
    async fn delete(&self, container_id: &str) -> Result<()>;
    async fn exec(&self, container_id: &str, command: &[String], tty: bool) -> Result<i32>;
    async fn get_pid(&self, container_id: &str) -> Result<Option<u32>>;
}

#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn pull_image(&self, tag: &str) -> Result<Image>;
    async fn push_image(&self, image_id: &str, registry: &str) -> Result<()>;
    async fn list_images(&self) -> Result<Vec<Image>>;
    async fn delete_image(&self, image_id: &str) -> Result<()>;
    async fn extract_layers(&self, image_id: &str, destination: &PathBuf) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;

    fn make_container() -> Container {
        Container {
            id: "abc123".to_string(),
            name: "test-container".to_string(),
            image: "ubuntu:22.04".to_string(),
            status: ContainerStatus::Running,
            pid: Some(1234),
            created_at: UNIX_EPOCH,
            started_at: Some(UNIX_EPOCH),
            ports: vec![PortMapping {
                host_ip: "0.0.0.0".to_string(),
                host_port: 8080,
                container_port: 80,
                protocol: Protocol::Tcp,
            }],
            mounts: vec![MountConfig {
                host_path: PathBuf::from("/tmp/data"),
                container_path: PathBuf::from("/data"),
                read_only: false,
                is_tmpfs: false,
            }],
            memory_limit_bytes: Some(512 * 1024 * 1024),
            cpu_shares: Some(1024),
            health: Some(HealthStatus::Healthy),
            restart_count: Some(0),
            restart_policy: Some("on-failure:3".to_string()),
            health_cmd: Some("curl -f http://localhost/".to_string()),
            health_interval: Some(30),
            health_timeout: Some(5),
            health_retries: Some(3),
            pids_limit: Some(100),
            read_only: Some(false),
            security_opt: None,
        }
    }

    fn make_image() -> Image {
        Image {
            id: "sha256:deadbeef".to_string(),
            tag: "ubuntu:22.04".to_string(),
            digest: "sha256:deadbeef".to_string(),
            size_bytes: 29_000_000,
            layers: vec!["sha256:layer1".to_string(), "sha256:layer2".to_string()],
            architecture: "amd64".to_string(),
            os: "linux".to_string(),
            os_version: None,
            entrypoint: vec![],
            cmd: vec!["/bin/bash".to_string()],
            env: vec!["PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string()],
            config_digest: Some("sha256:config123".to_string()),
            stack: None,
        }
    }

    #[test]
    fn container_json_round_trip() {
        let c = make_container();
        let json = serde_json::to_string(&c).expect("serialize");
        let back: Container = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.id, c.id);
        assert_eq!(back.name, c.name);
        assert_eq!(back.status, ContainerStatus::Running);
        assert_eq!(back.pid, Some(1234));
        assert_eq!(back.ports.len(), 1);
        assert_eq!(back.ports[0].host_port, 8080);
        assert_eq!(back.ports[0].protocol, Protocol::Tcp);
        assert_eq!(back.mounts.len(), 1);
        assert_eq!(back.mounts[0].container_path, PathBuf::from("/data"));
        assert_eq!(back.memory_limit_bytes, Some(512 * 1024 * 1024));
    }

    #[test]
    fn image_json_round_trip() {
        let img = make_image();
        let json = serde_json::to_string(&img).expect("serialize");
        let back: Image = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.id, img.id);
        assert_eq!(back.tag, "ubuntu:22.04");
        assert_eq!(back.layers.len(), 2);
        assert_eq!(back.cmd, vec!["/bin/bash"]);
        assert_eq!(back.size_bytes, 29_000_000);
        assert_eq!(back.config_digest, Some("sha256:config123".to_string()));
    }

    #[test]
    fn image_missing_optional_fields_default() {
        let json = r#"{
            "id": "sha256:abc",
            "tag": "nginx:latest",
            "digest": "sha256:abc",
            "size_bytes": 0,
            "layers": [],
            "architecture": "amd64",
            "os": "linux"
        }"#;
        let img: Image = serde_json::from_str(json).expect("deserialize with defaults");
        assert!(img.entrypoint.is_empty());
        assert!(img.cmd.is_empty());
        assert!(img.env.is_empty());
        assert!(img.config_digest.is_none());
    }

    #[test]
    fn container_status_all_variants_serialize() {
        for (status, expected) in [
            (ContainerStatus::Creating, "\"Creating\""),
            (ContainerStatus::Created,  "\"Created\""),
            (ContainerStatus::Running,  "\"Running\""),
            (ContainerStatus::Paused,   "\"Paused\""),
            (ContainerStatus::Stopped,  "\"Stopped\""),
        ] {
            let s = serde_json::to_string(&status).unwrap();
            assert_eq!(s, expected, "wrong serialization for {:?}", status);
            let back: ContainerStatus = serde_json::from_str(&s).unwrap();
            assert_eq!(back, status);
        }
    }

    #[test]
    fn port_mapping_udp_protocol() {
        let pm = PortMapping {
            host_ip: "127.0.0.1".to_string(),
            host_port: 53,
            container_port: 53,
            protocol: Protocol::Udp,
        };
        let json = serde_json::to_string(&pm).unwrap();
        let back: PortMapping = serde_json::from_str(&json).unwrap();
        assert_eq!(back.protocol, Protocol::Udp);
        assert_eq!(back.host_ip, "127.0.0.1");
    }

    #[test]
    fn crush_error_display_messages() {
        let cases: &[(CrushError, &str)] = &[
            (CrushError::ContainerNotFound("abc".to_string()), "abc"),
            (CrushError::ImageError("bad digest".to_string()), "bad digest"),
            (CrushError::NetworkError("veth failed".to_string()), "veth failed"),
            (CrushError::StorageError("no space".to_string()), "no space"),
        ];
        for (err, needle) in cases {
            let msg = err.to_string();
            assert!(msg.contains(needle), "error message {msg:?} should contain {needle:?}");
        }
    }

    #[test]
    fn invalid_state_transition_error() {
        let e = CrushError::InvalidStateTransition {
            from: ContainerStatus::Stopped,
            to: ContainerStatus::Paused,
        };
        let msg = e.to_string();
        assert!(msg.contains("Stopped"), "should mention source state: {msg}");
        assert!(msg.contains("Paused"),  "should mention target state: {msg}");
    }

    #[test]
    fn mount_config_tmpfs() {
        let m = MountConfig {
            host_path: PathBuf::from(""),
            container_path: PathBuf::from("/tmp"),
            read_only: false,
            is_tmpfs: true,
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: MountConfig = serde_json::from_str(&json).unwrap();
        assert!(back.is_tmpfs);
        assert_eq!(back.container_path, PathBuf::from("/tmp"));
    }
}
