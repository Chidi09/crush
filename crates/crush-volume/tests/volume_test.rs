use std::collections::HashMap;
use std::path::PathBuf;
use crush_volume::{LocalDriver, VolumeDriver, VolumeMounter};

#[tokio::test]
async fn test_volume_lifecycle() {
    let tmp = std::env::temp_dir().join("crush_volume_unit_test").join(&format!("test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let driver = LocalDriver::new(tmp.clone());
    let mut labels = HashMap::new();
    labels.insert("env".to_string(), "test".to_string());

    // Create volume
    let info = driver.create("test-vol", labels).await.unwrap();
    assert_eq!(info.name, "test-vol");
    assert!(info.mountpoint.exists());

    // Inspect volume
    let inspected = driver.inspect("test-vol").await.unwrap();
    assert_eq!(inspected.name, "test-vol");
    assert_eq!(inspected.ref_count, 0);

    // List volumes
    let list = driver.list().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "test-vol");

    // Remove volume
    driver.remove("test-vol").await.unwrap();
    assert!(!tmp.join("volumes").join("test-vol").exists());

    // Re-create after remove
    let mut labels2 = HashMap::new();
    labels2.insert("env".to_string(), "test2".to_string());
    let info2 = driver.create("test-vol", labels2).await.unwrap();
    assert_eq!(info2.name, "test-vol");
    
    // Clean up
    let _ = driver.remove("test-vol").await;
    let _ = std::fs::remove_dir_all(&tmp);
}

#[tokio::test]
async fn test_bind_mount_validation() {
    let tmp = std::env::temp_dir().join("crush_volume_bind_test").join(&format!("test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let mounter = VolumeMounter::new(tmp.clone());
    let rootfs = tmp.join("rootfs");
    std::fs::create_dir_all(&rootfs).unwrap();

    // Bind mounting non-existent host path must return Err
    let non_existent = tmp.join("non_existent_host_path_xyz").to_string_lossy().to_string();
    let res = mounter.mount_bind("container-1", &non_existent, "/app", &rootfs, false).await;
    assert!(res.is_err(), "non-existent host path should return Err");

    let _ = std::fs::remove_dir_all(&tmp);
}
