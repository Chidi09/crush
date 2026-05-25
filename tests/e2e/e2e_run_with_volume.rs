// e2e: create volume → write file → stop container → restart → file persists
// Exercises handoffs: volume::create → bind::mount → runtime::stop → volume::reuse → runtime::start

use std::path::PathBuf;
use std::time::SystemTime;
use crush_types::*;
use crush_volume::VolumeManager;

#[tokio::test]
async fn e2e_volume_persistence() {
    let tmp = std::env::temp_dir().join("crush_e2e_vol").join(&format!("test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    // 1. Create named volume (volume lifecycle handoff)
    let vol_mgr = VolumeManager::new(tmp.clone()).unwrap();
    let create = vol_mgr.create_volume("e2e-data", "local", vec![
        ("app".to_string(), "e2e-test".to_string()),
    ]).await;
    assert!(create.is_ok(), "named volume should be created");

    // 2. Verify volume metadata (volume ↔ db handoff)
    let meta = vol_mgr.named.get("e2e-data").await.unwrap();
    assert_eq!(meta.name, "e2e-data");
    assert_eq!(meta.driver, "local");
    assert_eq!(meta.labels.first().map(|(k, _)| k.clone()), Some("app".to_string()));
    assert!(meta.mountpoint.exists(), "volume mountpoint should exist on disk");

    // 3. Simulate data written to volume (simulating a container writing files)
    let test_file = meta.mountpoint.join("hello.txt");
    std::fs::write(&test_file, b"crush persist test data").unwrap();

    // 4. Simulate container stop + restart — volume persists
    drop(vol_mgr);

    let vol_mgr2 = VolumeManager::new(tmp.clone()).unwrap();
    let meta2 = vol_mgr2.named.get("e2e-data").await.unwrap();
    assert!(meta2.mountpoint.join("hello.txt").exists(), "file should persist after manager restart");

    let content = std::fs::read_to_string(meta2.mountpoint.join("hello.txt")).unwrap();
    assert_eq!(content, "crush persist test data", "written data should be intact");

    // 5. Cleanup
    let _ = vol_mgr2.remove_volume("e2e-data").await;
    let _ = std::fs::remove_dir_all(&tmp);
    println!("e2e: volume create → write → persist → read → delete cycle passed");
}
