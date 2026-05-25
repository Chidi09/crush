use std::time::Duration;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use crush_types::{Result, CrushError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub check_type: HealthCheckType,
    pub interval_secs: u64,
    pub timeout_secs: u64,
    pub retries: u32,
    pub start_period_secs: u64,
    pub start_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthCheckType {
    Http {
        url: String,
        expected_status: u16,
        expected_body_regex: Option<String>,
    },
    Tcp {
        host: String,
        port: u16,
    },
    Exec {
        command: Vec<String>,
    },
    Grpc {
        target: String,
        service: Option<String>,
    },
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            check_type: HealthCheckType::Tcp { host: "127.0.0.1".into(), port: 8080 },
            interval_secs: 30,
            timeout_secs: 30,
            retries: 3,
            start_period_secs: 0,
            start_interval_secs: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Starting,
    Healthy,
    Unhealthy,
}

pub struct HealthState {
    pub status: HealthStatus,
    pub consecutive_failures: u32,
    pub last_check: Option<DateTime<Utc>>,
    pub last_output: Option<String>,
}

pub struct HealthChecker {
    pub config: HealthCheckConfig,
    pub state: Arc<Mutex<HealthState>>,
}

impl HealthChecker {
    pub fn new(config: HealthCheckConfig) -> Self {
        Self {
            state: Arc::new(Mutex::new(HealthState {
                status: HealthStatus::Starting,
                consecutive_failures: 0,
                last_check: None,
                last_output: None,
            })),
            config,
        }
    }

    pub async fn check(&self) -> HealthStatus {
        let result = match &self.config.check_type {
            HealthCheckType::Http { url, expected_status, expected_body_regex } => {
                self.check_http(url, *expected_status, expected_body_regex).await
            }
            HealthCheckType::Tcp { host, port } => {
                self.check_tcp(host, *port).await
            }
            HealthCheckType::Exec { command } => {
                self.check_exec(command).await
            }
            HealthCheckType::Grpc { target, service: _ } => {
                self.check_grpc(target).await
            }
        };

        let mut state = self.state.lock().await;
        state.last_check = Some(Utc::now());

        match result {
            Ok(output) => {
                state.consecutive_failures = 0;
                state.status = HealthStatus::Healthy;
                state.last_output = output;
            }
            Err(e) => {
                state.consecutive_failures += 1;
                state.last_output = Some(e.to_string());
                if state.consecutive_failures >= self.config.retries {
                    state.status = HealthStatus::Unhealthy;
                }
            }
        }

        state.status.clone()
    }

    async fn check_http(&self, url: &str, expected_status: u16, body_regex: &Option<String>) -> Result<Option<String>> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(self.config.timeout_secs))
            .build()
            .map_err(|e| CrushError::ApiError(e.to_string()))?;

        let resp = client.get(url).send().await
            .map_err(|e| CrushError::ApiError(format!("HTTP health check failed: {}", e)))?;

        if resp.status().as_u16() != expected_status {
            return Err(CrushError::ApiError(format!(
                "HTTP {} expected, got {}", expected_status, resp.status()
            )));
        }

        if let Some(ref regex) = body_regex {
            let body = resp.text().await
                .map_err(|e| CrushError::ApiError(format!("Body read failed: {}", e)))?;
            let re = regex::Regex::new(regex)
                .map_err(|e| CrushError::ApiError(format!("Bad regex: {}", e)))?;
            if !re.is_match(&body) {
                return Err(CrushError::ApiError("Body regex did not match".to_string()));
            }
        }

        Ok(None)
    }

    async fn check_tcp(&self, host: &str, port: u16) -> Result<Option<String>> {
        let addr = format!("{}:{}", host, port);
        tokio::time::timeout(
            Duration::from_secs(self.config.timeout_secs),
            tokio::net::TcpStream::connect(&addr),
        ).await
            .map_err(|_| CrushError::ApiError("TCP health check timed out".to_string()))?
            .map_err(|e| CrushError::ApiError(format!("TCP health check failed: {}", e)))?;

        Ok(None)
    }

    async fn check_exec(&self, command: &[String]) -> Result<Option<String>> {
        let output = tokio::process::Command::new(&command[0])
            .args(&command[1..])
            .output()
            .await
            .map_err(|e| CrushError::ApiError(format!("Exec health check failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CrushError::ApiError(format!(
                "Exec returned code {:?}: {}", output.status.code(), stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(Some(stdout))
    }

    async fn check_grpc(&self, target: &str) -> Result<Option<String>> {
        // Simplified gRPC health check via HTTP/2 to the health endpoint
        let url = format!("http://{}/grpc.health.v1.Health/Check", target);
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(self.config.timeout_secs))
            .build()
            .map_err(|e| CrushError::ApiError(e.to_string()))?;

        let resp = client.post(&url)
            .header("Content-Type", "application/grpc")
            .send().await
            .map_err(|e| CrushError::ApiError(format!("gRPC health check failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(CrushError::ApiError(format!("gRPC returned {}", resp.status())));
        }

        Ok(None)
    }
}
