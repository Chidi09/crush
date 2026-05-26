use std::path::PathBuf;
use std::collections::HashMap;
use crush_volume::{LocalDriver, VolumeDriver};

#[tokio::test]
async fn e2e_volume_persistence() {
    let tmp = std::env::temp_dir().join("crush_e2e_vol").join(&format!("test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let driver = LocalDriver::new(tmp.clone());
    let mut labels = HashMap::new();
    labels.insert("app".to_string(), "e2e-test".to_string());

    // 1. Create named volume
    let create = driver.create("e2e-data", labels).await;
    assert!(create.is_ok(), "named volume should be created");

    // 2. Verify volume metadata
    let meta = driver.inspect("e2e-data").await.unwrap();
    assert_eq!(meta.name, "e2e-data");
    assert_eq!(meta.driver, "local");
    assert_eq!(meta.labels.get("app"), Some(&"e2e-test".to_string()));
    assert!(meta.mountpoint.exists(), "volume mountpoint should exist on disk");

    // 3. Simulate data written to volume
    let test_file = meta.mountpoint.join("hello.txt");
    std::fs::write(&test_file, b"crush persist test data").unwrap();

    // 4. Simulate container stop + restart — volume persists
    drop(driver);

    let driver2 = LocalDriver::new(tmp.clone());
    let meta2 = driver2.inspect("e2e-data").await.unwrap();
    assert!(meta2.mountpoint.join("hello.txt").exists(), "file should persist");

    let content = std::fs::read_to_string(meta2.mountpoint.join("hello.txt")).unwrap();
    assert_eq!(content, "crush persist test data", "written data should be intact");

    // 5. Cleanup
    let _ = driver2.remove("e2e-data").await;
    let _ = std::fs::remove_dir_all(&tmp);
    println!("e2e: volume create → write → persist → read → delete cycle passed");
}
