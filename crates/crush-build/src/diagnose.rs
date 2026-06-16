use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Diagnosis {
    pub cause: String,
    pub suggestion: String,
}

/// Run lightweight local pattern matchers against the trailing stderr buffer
/// to suggest a probable cause and fix for a crash.
pub fn diagnose_crash(stderr: &str) -> Option<Diagnosis> {
    if stderr.contains("EADDRINUSE") || stderr.contains("address already in use") {
        return Some(Diagnosis {
            cause: "Port is already in use by another process.".to_string(),
            suggestion: "Pass --port-conflict=kill or --port-conflict=reassign, or stop the conflicting process.".to_string(),
        });
    }

    // Check the Prisma-specific pattern before the generic Node-module matcher,
    // since a missing `.prisma/client` also surfaces as "Cannot find module".
    if stderr.contains(".prisma/client") || stderr.contains("@prisma/client") {
        return Some(Diagnosis {
            cause: "Prisma client is not generated.".to_string(),
            suggestion: "Run `npx prisma generate`.".to_string(),
        });
    }

    if stderr.contains("MODULE_NOT_FOUND") || stderr.contains("Cannot find module") {
        return Some(Diagnosis {
            cause: "A required Node module is missing.".to_string(),
            suggestion: "Run `npm install`, `yarn install`, or `pnpm install`.".to_string(),
        });
    }

    if stderr.contains("EACCES") || stderr.contains("Permission denied") {
        return Some(Diagnosis {
            cause: "Permission denied.".to_string(),
            suggestion: "Check file permissions or try running with elevated privileges.".to_string(),
        });
    }

    if stderr.contains("ModuleNotFoundError") {
        return Some(Diagnosis {
            cause: "A required Python module is missing.".to_string(),
            suggestion: "Run `pip install -r requirements.txt` or ensure your virtualenv is active.".to_string(),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnose_port_in_use() {
        let stderr = "Error: listen EADDRINUSE: address already in use :::3000";
        let diag = diagnose_crash(stderr).unwrap();
        assert!(diag.cause.contains("Port is already in use"));
    }

    #[test]
    fn test_diagnose_prisma() {
        let stderr = "Cannot find module '.prisma/client'";
        let diag = diagnose_crash(stderr).unwrap();
        assert!(diag.cause.contains("Prisma client"));
    }
}
