use crush_types::{Result, CrushError};

#[derive(Debug, Clone, PartialEq)]
pub enum NetworkMode {
    Bridge,
    Host,
    None,
    Container(String),
}

impl NetworkMode {
    pub fn parse(s: &str) -> Self {
        match s {
            "host" => NetworkMode::Host,
            "none" => NetworkMode::None,
            "bridge" => NetworkMode::Bridge,
            other if other.starts_with("container:") => {
                let id = other.trim_start_matches("container:");
                NetworkMode::Container(id.to_string())
            }
            _ => NetworkMode::Bridge,
        }
    }

    pub fn validate(&self) -> Result<()> {
        match self {
            NetworkMode::Bridge => Ok(()),
            NetworkMode::Host => Ok(()),
            NetworkMode::None => Ok(()),
            NetworkMode::Container(id) => {
                if id.is_empty() {
                    Err(CrushError::NetworkError("Empty container ID in container: mode".to_string()))
                } else {
                    Ok(())
                }
            }
        }
    }

    pub fn needs_network_setup(&self) -> bool {
        matches!(self, NetworkMode::Bridge)
    }

    pub fn needs_loopback(&self) -> bool {
        !matches!(self, NetworkMode::None)
    }

    pub fn description(&self) -> &str {
        match self {
            NetworkMode::Bridge => "bridge",
            NetworkMode::Host => "host",
            NetworkMode::None => "none",
            NetworkMode::Container(_) => "from container",
        }
    }
}
