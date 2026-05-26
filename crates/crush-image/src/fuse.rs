use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, Request,
};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};
use libc::ENOENT;
use crate::lazy::LazyLoader;
use tokio::runtime::Handle;

const TTL: Duration = Duration::from_secs(1);
const CHUNK_SIZE: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct InodeMetadata {
    pub ino: u64,
    pub name: String,
    pub kind: FileType,
    pub size: u64,
    pub offset_in_layer: u64,
    pub children: Vec<u64>,
}

pub struct LazyImageFs {
    loader: Arc<LazyLoader>,
    inodes: HashMap<u64, InodeMetadata>,
    parent_map: HashMap<u64, u64>, // ino -> parent_ino
    runtime: Handle,
}

impl LazyImageFs {
    pub fn new(loader: Arc<LazyLoader>, inodes: HashMap<u64, InodeMetadata>) -> Self {
        let mut parent_map = HashMap::new();
        for (ino, meta) in &inodes {
            for child in &meta.children {
                parent_map.insert(*child, *ino);
            }
        }
        
        Self {
            loader,
            inodes,
            parent_map,
            runtime: Handle::current(),
        }
    }

    fn get_attr(&self, ino: u64) -> Option<FileAttr> {
        let meta = self.inodes.get(&ino)?;
        Some(FileAttr {
            ino,
            size: meta.size,
            blocks: (meta.size + 511) / 512,
            atime: UNIX_EPOCH,
            mtime: UNIX_EPOCH,
            ctime: UNIX_EPOCH,
            crtime: UNIX_EPOCH,
            kind: meta.kind,
            perm: if meta.kind == FileType::Directory { 0o755 } else { 0o644 },
            nlink: 1,
            uid: 0,
            gid: 0,
            rdev: 0,
            flags: 0,
            blksize: 512,
        })
    }
}

impl Filesystem for LazyImageFs {
    fn lookup(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let name_str = name.to_string_lossy();
        if let Some(parent_meta) = self.inodes.get(&parent) {
            for child_ino in &parent_meta.children {
                if let Some(child_meta) = self.inodes.get(child_ino) {
                    if child_meta.name == name_str {
                        if let Some(attr) = self.get_attr(*child_ino) {
                            reply.entry(&TTL, &attr, 0);
                            return;
                        }
                    }
                }
            }
        }
        reply.error(ENOENT);
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyAttr) {
        if let Some(attr) = self.get_attr(ino) {
            reply.attr(&TTL, &attr);
        } else {
            reply.error(ENOENT);
        }
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        if let Some(meta) = self.inodes.get(&ino) {
            if meta.kind != FileType::RegularFile {
                reply.error(libc::EISDIR);
                return;
            }

            let file_offset = meta.offset_in_layer;
            let absolute_offset = file_offset + offset as u64;
            
            let loader = self.loader.clone();
            let size = size as usize;
            
            // FUSE callbacks are executed on a dedicated background thread pool created by fuser,
            // not the Tokio runtime workers, so block_on is safe here.
            let result = self.runtime.block_on(async {
                let chunk_index = (absolute_offset / CHUNK_SIZE) as u32;
                let chunk_internal_offset = (absolute_offset % CHUNK_SIZE) as usize;
                
                match loader.get_chunk(chunk_index).await {
                    Ok(data) => {
                        let available = data.len().saturating_sub(chunk_internal_offset);
                        let to_copy = std::cmp::min(size, available);
                        if to_copy > 0 {
                            Ok(data[chunk_internal_offset..chunk_internal_offset + to_copy].to_vec())
                        } else {
                            Ok(vec![])
                        }
                    }
                    Err(e) => Err(e),
                }
            });
            
            match result {
                Ok(data) => reply.data(&data),
                Err(_) => reply.error(libc::EIO),
            }
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if let Some(meta) = self.inodes.get(&ino) {
            let mut entries = vec![
                (ino, FileType::Directory, ".".to_string()),
                (self.parent_map.get(&ino).copied().unwrap_or(ino), FileType::Directory, "..".to_string()),
            ];

            for child_ino in &meta.children {
                if let Some(child_meta) = self.inodes.get(child_ino) {
                    entries.push((*child_ino, child_meta.kind, child_meta.name.clone()));
                }
            }

            for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
                if reply.add(entry.0, (i + 1) as i64, entry.1, &entry.2) {
                    break;
                }
            }
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }
}
