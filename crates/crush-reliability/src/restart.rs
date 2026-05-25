use std::time::Duration;
use rand::Rng;
use serde::{Serialize, Deserialize};
use crush_types::{Result, CrushError};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RestartPolicy {
    No,
    Always,
    OnFailure { max_retries: Option<u32> },
    UnlessStopped,
}

impl Default for RestartPolicy { fn default() -> Self { Self::No } }

pub struct RestartManager {
    policy: RestartPolicy,
    current_attempt: u32,
    base_delay_ms: u64,
    max_delay_ms: u64,
}

impl RestartManager {
    pub fn new(policy: RestartPolicy) -> Self {
        Self {
            policy,
            current_attempt: 0,
            base_delay_ms: 1000,
            max_delay_ms: 300_000,
        }
    }

    pub fn should_restart(&mut self, exit_code: i32, explicitly_stopped: bool) -> bool {
        match self.policy {
            RestartPolicy::No => false,
            RestartPolicy::Always => !explicitly_stopped,
            RestartPolicy::OnFailure { max_retries } => {
                if exit_code == 0 || explicitly_stopped { return false; }
                if let Some(max) = max_retries {
                    self.current_attempt < max
                } else { true }
            }
            RestartPolicy::UnlessStopped => !explicitly_stopped,
        }
    }

    pub fn backoff_delay(&self) -> Duration {
        if self.current_attempt == 0 { return Duration::from_millis(100); }

        let exp = self.base_delay_ms * (2u64.pow(self.current_attempt.saturating_sub(1)));
        let capped = exp.min(self.max_delay_ms);

        let jitter = rand::thread_rng().gen_range(0..=capped / 5);
        Duration::from_millis(capped + jitter)
    }

    pub fn record_attempt(&mut self) {
        self.current_attempt += 1;
    }

    pub fn reset(&mut self) {
        self.current_attempt = 0;
    }

    pub fn attempt(&self) -> u32 { self.current_attempt }
    pub fn policy(&self) -> &RestartPolicy { &self.policy }
}
