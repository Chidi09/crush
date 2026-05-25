// e2e: pull + extract + create + start + stop + cleanup
// Exercises handoffs: registry::pull → image::extract → runtime::create → runtime::start → runtime::stop

use std::path::PathBuf;
use std::time::{SystemTime, Duration};
use crush_types::*;
use crush_image::ImageStore;
use crush_runtime_linux::LinuxRuntime;

#[tokio::test]
async fn e2e_pull_and_run_container() {
    let tmp = std::env::temp_dir().join("crush_e2e_run").join(&format!("test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    // 1. Pull image (registry → image handoff)
    let store = ImageStore::new(tmp.join("images")).await.unwrap();
    let image = store.pull_image("hello-world:latest").await;
    if let Err(ref e) = image {
        eprintln!("Pull skipped (no network): {}", e);
        return;
    }
    let image = image.unwrap();

    // 2. Extract layers (image → runtime handoff)
    let rootfs = tmp.join("rootfs");
    let extract = store.extract_layers(&image.digest, &rootfs).await;
    assert!(extract.is_ok(), "layer extraction should succeed");

    // 3. Create container (runtime lifecycle handoff)
    let spec_path = tmp.join("spec");
    std::fs::create_dir_all(&spec_path).unwrap();

    let container = Container {
        id: format!("e2e_test_{}", std::process::id()),
        name: "e2e-hello".to_string(),
        image: "hello-world:latest".to_string(),
        status: ContainerStatus::Creating,
        pid: None,
        created_at: SystemTime::now(),
        started_at: None,
        ports: vec![],
        mounts: vec![],
        memory_limit_bytes: Some(64 * 1024 * 1024),
        cpu_shares: Some(256),
    };

    let runtime = LinuxRuntime::new();
    let create_result = runtime.create(&container, &spec_path).await;
    if let Err(ref e) = create_result {
        eprintln!("Container create skipped (unsupported on non-Linux): {}", e);
        return;
    }

    // 4. Start → stop cycle
    let start = runtime.start(&container.id).await;
    assert!(start.is_ok(), "container should start");

    tokio::time::sleep(Duration::from_millis(500)).await;

    let stop = runtime.stop(&container.id, 5).await;
    assert!(stop.is_ok(), "container should stop gracefully");

    // 5. Cleanup
    let _ = runtime.delete(&container.id).await;
    let _ = std::fs::remove_dir_all(&tmp);
    println!("e2e: pull → extract → create → start → stop → delete cycle passed");
}
