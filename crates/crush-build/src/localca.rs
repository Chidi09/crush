use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DistinguishedName, DnType,
    ExtendedKeyUsagePurpose, IsCa, KeyPair, KeyUsagePurpose,
};
use std::path::PathBuf;

/// Manage the local root CA for trusted local domains (mkcert-style).
pub struct LocalCa {
    ca_dir: PathBuf,
}

impl LocalCa {
    pub fn new(ca_dir: PathBuf) -> Self {
        Self { ca_dir }
    }

    fn root_cert_path(&self) -> PathBuf {
        self.ca_dir.join("rootCA.pem")
    }

    fn root_key_path(&self) -> PathBuf {
        self.ca_dir.join("rootCA-key.pem")
    }

    /// Deterministic CA parameters — used both when first generating the root and
    /// when reconstructing it on later runs (so the issuer identity is stable).
    fn ca_params() -> anyhow::Result<CertificateParams> {
        let mut params = CertificateParams::new(vec![])?;
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "Crush Local Root CA");
        dn.push(DnType::OrganizationName, "Crush Dev");
        params.distinguished_name = dn;
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        Ok(params)
    }

    /// Ensure the root CA exists, generating it if necessary. Returns the in-memory
    /// CA certificate + key pair for signing leaves.
    ///
    /// `rcgen`'s `from_ca_cert_pem` requires the `x509-parser` feature (not enabled),
    /// so on subsequent runs we reload only the key and rebuild an equivalent issuer
    /// from the deterministic params — the subject DN + key match the installed root,
    /// so leaves still chain to the trusted certificate.
    pub fn ensure_root_ca(&self) -> anyhow::Result<(Certificate, KeyPair)> {
        std::fs::create_dir_all(&self.ca_dir)?;

        let cert_path = self.root_cert_path();
        let key_path = self.root_key_path();

        if key_path.exists() {
            let key_pem = std::fs::read_to_string(&key_path)?;
            let key_pair = KeyPair::from_pem(&key_pem)?;
            let cert = Self::ca_params()?.self_signed(&key_pair)?;
            if !cert_path.exists() {
                std::fs::write(&cert_path, cert.pem())?;
            }
            return Ok((cert, key_pair));
        }

        // Generate a fresh root CA.
        let key_pair = KeyPair::generate()?;
        let cert = Self::ca_params()?.self_signed(&key_pair)?;

        std::fs::write(&cert_path, cert.pem())?;
        std::fs::write(&key_path, key_pair.serialize_pem())?;

        Ok((cert, key_pair))
    }

    /// Generate a leaf certificate (cert PEM, key PEM) signed by the root CA.
    pub fn sign_leaf(&self, domains: Vec<String>) -> anyhow::Result<(String, String)> {
        let (root_cert, root_key) = self.ensure_root_ca()?;

        let mut params = CertificateParams::new(domains)?;
        params.is_ca = IsCa::NoCa;
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];

        let leaf_key = KeyPair::generate()?;
        let leaf_cert = params.signed_by(&leaf_key, &root_cert, &root_key)?;

        Ok((leaf_cert.pem(), leaf_key.serialize_pem()))
    }

    /// Install the root CA into the OS trust store.
    pub fn install_trust(&self) -> anyhow::Result<()> {
        self.ensure_root_ca()?;
        let cert_path = self.root_cert_path();
        let _ = &cert_path;

        #[cfg(target_os = "macos")]
        {
            let status = std::process::Command::new("security")
                .args(["add-trusted-cert", "-d", "-r", "trustRoot", "-k", "/Library/Keychains/System.keychain", cert_path.to_str().unwrap()])
                .status()?;
            if !status.success() {
                anyhow::bail!("Failed to install trust on macOS");
            }
        }

        #[cfg(target_os = "windows")]
        {
            let status = std::process::Command::new("certutil")
                .args(["-addstore", "-user", "Root", cert_path.to_str().unwrap()])
                .status()?;
            if !status.success() {
                anyhow::bail!("Failed to install trust on Windows");
            }
        }

        #[cfg(target_os = "linux")]
        {
            eprintln!("Linux trust install requires copying {} to /usr/local/share/ca-certificates/crush-root.crt and running `sudo update-ca-certificates`", cert_path.display());
        }

        Ok(())
    }

    /// Remove the root CA from the OS trust store.
    pub fn uninstall_trust(&self) -> anyhow::Result<()> {
        #[cfg(target_os = "macos")]
        {
            let status = std::process::Command::new("security")
                .args(["delete-certificate", "-c", "Crush Local Root CA", "-t"])
                .status()?;
            if !status.success() {
                anyhow::bail!("Failed to uninstall trust on macOS");
            }
        }

        #[cfg(target_os = "windows")]
        {
            let status = std::process::Command::new("certutil")
                .args(["-delstore", "-user", "Root", "Crush Local Root CA"])
                .status()?;
            if !status.success() {
                anyhow::bail!("Failed to uninstall trust on Windows");
            }
        }

        #[cfg(target_os = "linux")]
        {
            eprintln!("Linux trust uninstall requires removing the cert from /usr/local/share/ca-certificates and running `sudo update-ca-certificates`");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn root_ca_is_stable_and_signs_leaves() {
        let tmp = TempDir::new().unwrap();
        let ca = LocalCa::new(tmp.path().to_path_buf());

        // Generates a CA + a leaf for a local domain.
        let (leaf_pem, key_pem) = ca.sign_leaf(vec!["app.crush.local".into()]).unwrap();
        assert!(leaf_pem.contains("BEGIN CERTIFICATE"));
        assert!(key_pem.contains("PRIVATE KEY"));

        // Re-opening reuses the same root key (stable issuer).
        let key_before = std::fs::read_to_string(tmp.path().join("rootCA-key.pem")).unwrap();
        let ca2 = LocalCa::new(tmp.path().to_path_buf());
        ca2.sign_leaf(vec!["other.crush.local".into()]).unwrap();
        let key_after = std::fs::read_to_string(tmp.path().join("rootCA-key.pem")).unwrap();
        assert_eq!(key_before, key_after);
    }
}
