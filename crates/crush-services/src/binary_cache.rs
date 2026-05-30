use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{self, Read};
use anyhow::{Result, Context};
use sha2::{Sha256, Digest};
use flate2::read::GzDecoder;
use tar::Archive;

#[derive(Debug, Clone)]
pub struct BinarySpec {
    pub service: &'static str,
    pub version: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
    pub archive_type: ArchiveType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveType { Zip, TarGz, Exe }

pub struct BinaryCache {
    pub root: PathBuf,
}

impl BinaryCache {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub async fn ensure(&self, spec: &BinarySpec) -> Result<PathBuf> {
        let dest_dir = self.root.join(spec.service).join(spec.version);
        let verified_sentinel = dest_dir.join(".verified");

        if verified_sentinel.exists() {
            return Ok(dest_dir);
        }

        fs::create_dir_all(&dest_dir).context("Failed to create destination dir")?;

        // 1. Download to temporary file
        let temp_file_path = dest_dir.join("download.tmp");
        
        let response = reqwest::get(spec.url).await
            .context("Failed to send GET request")?;
        
        if !response.status().is_success() {
            anyhow::bail!("Failed to download binary: HTTP status {}", response.status());
        }

        let bytes = response.bytes().await.context("Failed to read response bytes")?;
        fs::write(&temp_file_path, &bytes).context("Failed to write temporary download file")?;

        // 2. Verify SHA-256 if expected hash is provided
        if !spec.sha256.is_empty() && spec.sha256 != "<sha256>" && spec.sha256 != "<sha256-filled-in-at-release-time>" {
            Self::verify_sha256(&temp_file_path, spec.sha256)?;
        } else {
            println!("   ↳ Warning: skipping SHA-256 verification for {} (no hash provided)", spec.service);
        }

        // 3. Extract based on type
        let temp_file_path_clone = temp_file_path.clone();
        let dest_dir_clone = dest_dir.clone();
        let archive_type = spec.archive_type;
        let service = spec.service.to_string();

        tokio::task::spawn_blocking(move || -> Result<()> {
            match archive_type {
                ArchiveType::Zip => {
                    let file = File::open(&temp_file_path_clone)?;
                    let mut archive = zip::ZipArchive::new(file)?;
                    for i in 0..archive.len() {
                        let mut file = archive.by_index(i)?;
                        let outpath = match file.enclosed_name() {
                            Some(path) => dest_dir_clone.join(path),
                            None => continue,
                        };

                        if (*file.name()).ends_with('/') {
                            fs::create_dir_all(&outpath)?;
                        } else {
                            if let Some(p) = outpath.parent() {
                                if !p.exists() {
                                    fs::create_dir_all(p)?;
                                }
                            }
                            let mut outfile = File::create(&outpath)?;
                            io::copy(&mut file, &mut outfile)?;
                        }
                    }
                }
                ArchiveType::TarGz => {
                    let file = File::open(&temp_file_path_clone)?;
                    let tar = GzDecoder::new(file);
                    let mut archive = Archive::new(tar);
                    archive.unpack(&dest_dir_clone)?;
                }
                ArchiveType::Exe => {
                    let exe_name = if cfg!(target_os = "windows") {
                        format!("{}.exe", service)
                    } else {
                        service.clone()
                    };
                    let outpath = dest_dir_clone.join(exe_name);
                    fs::copy(&temp_file_path_clone, &outpath)?;
                }
            }
            Ok(())
        }).await??;

        // Clean up temp file
        let _ = fs::remove_file(temp_file_path);

        // Write verified sentinel
        fs::write(verified_sentinel, "verified")?;

        Ok(dest_dir)
    }

    fn verify_sha256(path: &Path, expected: &str) -> Result<()> {
        let mut file = File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        let result = hasher.finalize();
        let hash_str = hex::encode(result);

        if hash_str.eq_ignore_ascii_case(expected) {
            Ok(())
        } else {
            anyhow::bail!("SHA-256 checksum mismatch for {:?}.\nExpected: {}\nActual:   {}", path, expected, hash_str);
        }
    }
}
