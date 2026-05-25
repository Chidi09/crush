use std::path::{Path, PathBuf};
use std::fs;
use std::os::unix::io::{RawFd, AsRawFd};
use nix::sched::{setns, CloneFlags};
use nix::unistd::close;
use crush_types::{Result, CrushError};

const NETNS_DIR: &str = "/run/crush/netns";

pub struct NetworkNamespace {
    id: String,
    ns_file: Option<fs::File>,
    ns_path: PathBuf,
}

impl NetworkNamespace {
    pub fn create(id: &str) -> Result<Self> {
        fs::create_dir_all(NETNS_DIR)
            .map_err(|e| CrushError::NetworkError(e.to_string()))?;
        let ns_path = Path::new(NETNS_DIR).join(id);

        // ⚠ FIX: Keep the File handle alive to prevent dangling fd
        // as_raw_fd() borrows the File — the File must outlive the mount operation
        let ns_file = fs::File::create(&ns_path)
            .map_err(|e| CrushError::NetworkError(e.to_string()))?;

        Ok(Self { id: id.to_string(), ns_file: Some(ns_file), ns_path })
    }

    pub fn open(id: &str) -> Result<Self> {
        let ns_path = Path::new(NETNS_DIR).join(id);
        let ns_file = fs::File::open(&ns_path)
            .map_err(|e| CrushError::ContainerNotFound(e.to_string()))?;
        Ok(Self { id: id.to_string(), ns_file: Some(ns_file), ns_path })
    }

    pub fn path(&self) -> &Path { &self.ns_path }

    pub fn fd(&self) -> Option<RawFd> {
        self.ns_file.as_ref().map(|f| f.as_raw_fd())
    }

    pub fn enter(&self) -> Result<()> {
        if let Some(ref file) = self.ns_file {
            setns(file.as_raw_fd(), CloneFlags::CLONE_NEWNET)
                .map_err(|e| CrushError::NamespaceError(e.to_string()))
        } else {
            Err(CrushError::NamespaceError("Namespace file closed".to_string()))
        }
    }

    pub fn bind_mount(&self) -> Result<()> {
        let src = "/proc/self/ns/net";
        let dst = self.ns_path.to_string_lossy().to_string();
        let result = std::process::Command::new("mount")
            .args(["--bind", src, &dst]).status();
        match result {
            Ok(s) if s.success() => Ok(()),
            _ => std::process::Command::new("ln")
                .args(["-sf", src, &dst]).status().map(|_| ())
                .map_err(|e| CrushError::NamespaceError(e.to_string()))
        }
    }

    pub fn delete(id: &str) -> Result<()> {
        let path = Path::new(NETNS_DIR).join(id);
        if path.exists() {
            let _ = std::process::Command::new("ip").args(["netns", "delete", id]).output();
            let _ = fs::remove_file(&path);
        }
        Ok(())
    }

    pub fn list() -> Result<Vec<String>> {
        let mut names = Vec::new();
        if let Ok(entries) = fs::read_dir(NETNS_DIR) {
            for entry in entries.flatten() {
                names.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        Ok(names)
    }
}

impl Drop for NetworkNamespace {
    fn drop(&mut self) {
        if let Some(file) = self.ns_file.take() {
            let fd = file.as_raw_fd();
            let _ = close(fd);
        }
    }
}
