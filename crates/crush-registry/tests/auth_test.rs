use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use crush_registry::auth::AuthHandler;

#[test]
fn test_auth_handler_store_token() {
    let mut auth = AuthHandler::new();

    // initially none
    assert!(auth.get_auth_header("registry.example.com").is_none());

    // store token
    auth.store_token("registry.example.com", "my_secret_token".to_string());

    // retrieve token
    let header = auth.get_auth_header("registry.example.com");
    assert!(header.is_some());
    assert_eq!(header.unwrap(), "Bearer my_secret_token");
}

#[tokio::test]
async fn test_auth_handler_basic_auth() {
    let mut auth = AuthHandler::new();
    let header = auth.authenticate_basic("myreg", "user", "pass").await.unwrap();
    assert_eq!(header, format!("Basic {}", BASE64.encode("user:pass")));
    assert_eq!(auth.get_auth_header("myreg").unwrap(), header);
}
