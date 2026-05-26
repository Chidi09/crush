use crush_registry::{RegistryClientHandle, LocalRegistryServer};

#[tokio::test]
async fn test_registry_client_deadlock_regression() {
    // Start local registry server on port 9091
    let server = LocalRegistryServer::new(9091);
    server.start().await.unwrap();

    // Small delay to allow server to bind
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let handle = RegistryClientHandle::default();
    
    // fetch_manifest shouldn't deadlock when called sequentially
    let res = handle.fetch_manifest("127.0.0.1:9091", "test-image", "latest").await;
    // We expect it might fail since the catalog doesn't have the image, but it should NOT hang/deadlock
    // LocalRegistryServer returns 404 for unknown manifests, so we should get an error
    assert!(res.is_err());
    
    // Then fetch_blob on the same handle
    let res = handle.fetch_blob("127.0.0.1:9091", "test-image", "sha256:1234").await;
    assert!(res.is_err());
}
