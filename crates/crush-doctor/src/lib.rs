pub mod check;
pub mod checks;
pub mod registry;

pub use check::{CheckAction, CheckStatus, DoctorCheck};
pub use checks::build_tools::BuildToolsCheck;
pub use checks::manifest_drift::ManifestDriftCheck;
pub use checks::toolchain::ToolchainCheck;
pub use registry::{CheckRegistry, CheckResult};
