#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires root and network"]
fn test_cli_pull_and_run() {
    use std::process::Command;
    
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    
    // 1. Pull
    let pull_status = Command::new(&cargo)
        .args(&["run", "-p", "crush-cli", "--", "pull", "hello-world:latest"])
        .status()
        .expect("Failed to execute pull command");
    
    assert!(pull_status.success());
    
    // 2. Run
    let output = Command::new(&cargo)
        .args(&["run", "-p", "crush-cli", "--", "run", "hello-world:latest"])
        .output()
        .expect("Failed to execute run command");
        
    assert!(output.status.success(), "run command failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello from Docker!"));
}
