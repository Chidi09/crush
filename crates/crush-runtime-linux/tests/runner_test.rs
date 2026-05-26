#[cfg(target_os = "linux")]
use std::path::Path;
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use tempfile::tempdir;
#[cfg(target_os = "linux")]
use crush_runtime_linux::runner::run_container;

#[cfg(target_os = "linux")]
fn setup_rootfs(rootfs: &Path) -> bool {
    fs::create_dir_all(rootfs.join("bin")).unwrap();
    if Path::new("/bin/busybox").exists() {
        let _ = fs::copy("/bin/busybox", rootfs.join("bin").join("echo"));
        let _ = fs::copy("/bin/busybox", rootfs.join("bin").join("sh"));
        let _ = fs::copy("/bin/busybox", rootfs.join("bin").join("ls"));
        return true;
    }
    false
}

#[cfg(target_os = "linux")]
fn mock_container() -> crush_types::Container {
    crush_types::Container {
        id: "test".to_string(),
        name: "test".to_string(),
        image: "test".to_string(),
        status: crush_types::ContainerStatus::Running,
        pid: None,
        created_at: std::time::SystemTime::now(),
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
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_run_container_success() {
    let dir = tempdir().unwrap();
    let rootfs = dir.path().join("rootfs");
    if !setup_rootfs(&rootfs) {
        println!("Skipping test, /bin/busybox not found");
        return;
    }
    let code = run_container(&rootfs, &[String::from("/bin/echo"), String::from("hello")], &[], &mock_container()).unwrap();
    assert_eq!(code, 0);
}

#[cfg(target_os = "linux")]
#[test]
fn test_run_container_failure() {
    let dir = tempdir().unwrap();
    let rootfs = dir.path().join("rootfs");
    if !setup_rootfs(&rootfs) {
        return;
    }
    let code = run_container(&rootfs, &[String::from("/bin/sh"), String::from("-c"), String::from("exit 42")], &[], &mock_container()).unwrap();
    assert_eq!(code, 42);
}

#[cfg(target_os = "linux")]
#[test]
fn test_run_container_namespace_isolation() {
    let dir = tempdir().unwrap();
    let rootfs = dir.path().join("rootfs");
    if !setup_rootfs(&rootfs) {
        return;
    }
    let code = run_container(&rootfs, &[String::from("/bin/sh"), String::from("-c"), String::from("ls /proc > /tmp/out")], &[], &mock_container()).unwrap();
    assert_eq!(code, 0);
    let out = fs::read_to_string(rootfs.join("tmp").join("out")).unwrap();
    assert!(!out.is_empty());
}
