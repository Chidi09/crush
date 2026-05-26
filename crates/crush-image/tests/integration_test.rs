use std::path::PathBuf;
use crush_types::StorageBackend;
use crush_image::ImageStore;
use tempfile::tempdir;

#[tokio::test]
#[ignore = "requires internet connection to Docker Hub"]
async fn test_pull_hello_world_and_ubuntu() {
    let dir = tempdir().unwrap();
    let store = ImageStore::new(dir.path().to_path_buf()).await.unwrap();

    // 1. Pull hello-world:latest
    let img = store.pull_image("docker://hello-world:latest").await.expect("Failed to pull hello-world");
    assert_eq!(img.entrypoint, vec!["/hello"]);

    // 2. Second pull should be cached (no error)
    let img2 = store.pull_image("docker://hello-world:latest").await.expect("Failed to pull cached hello-world");
    assert_eq!(img2.id, img.id);

    // 3. Pull ubuntu:latest and test layer extraction
    let ubuntu = store.pull_image("docker://ubuntu:latest").await.expect("Failed to pull ubuntu");
    let dest = dir.path().join("ubuntu-rootfs");
    store.extract_layers(&ubuntu.id, &dest).await.expect("Failed to extract ubuntu layers");
    
    // Verify /bin/bash exists in the extracted rootfs
    assert!(dest.join("bin").join("bash").exists());
}
