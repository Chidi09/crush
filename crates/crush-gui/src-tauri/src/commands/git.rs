use serde::{Deserialize, Serialize};
use std::process::Command;
use crate::state::AppState;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubRepo {
    pub owner: String,
    pub repo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommit {
    pub short: String,
    pub message: String,
    pub author: String,
    pub committed_rel: String,
    pub committed_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    pub is_repo: bool,
    pub branch: Option<String>,
    pub remote_url: Option<String>,
    pub parsed_github: Option<GithubRepo>,
    pub head: Option<GitCommit>,
    pub dirty_count: usize,
    pub ahead: Option<usize>,
    pub behind: Option<usize>,
    pub upstream: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
    pub short: Option<String>,
    pub message: Option<String>,
    pub author: Option<String>,
    pub committed_rel: Option<String>,
    pub committed_ms: Option<i64>,
}

fn git_cmd(path: &str) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(path);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    cmd
}

fn run_git_cmd(path: &str, args: &[&str]) -> Option<String> {
    git_cmd(path)
        .args(args)
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
            } else {
                None
            }
        })
}

fn parse_github(url: &str) -> Option<GithubRepo> {
    let mut s = url;
    if s.starts_with("https://github.com/") {
        s = &s["https://github.com/".len()..];
    } else if s.starts_with("git@github.com:") {
        s = &s["git@github.com:".len()..];
    } else {
        return None;
    }
    if s.ends_with(".git") {
        s = &s[..s.len() - 4];
    }
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() == 2 {
        Some(GithubRepo {
            owner: parts[0].to_string(),
            repo: parts[1].to_string(),
        })
    } else {
        None
    }
}

fn parse_commit(log_out: &str) -> Option<GitCommit> {
    let parts: Vec<&str> = log_out.split('\x1f').collect();
    if parts.len() >= 5 {
        Some(GitCommit {
            short: parts[0].to_string(),
            message: parts[1].to_string(),
            author: parts[2].to_string(),
            committed_rel: parts[3].to_string(),
            committed_ms: parts[4].parse().unwrap_or(0),
        })
    } else {
        None
    }
}

#[tauri::command]
pub async fn git_info(path: String) -> Result<GitInfo, String> {
    let is_repo = git_cmd(&path)
        .args(&["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);

    if !is_repo {
        return Ok(GitInfo {
            is_repo: false,
            branch: None,
            remote_url: None,
            parsed_github: None,
            head: None,
            dirty_count: 0,
            ahead: None,
            behind: None,
            upstream: None,
        });
    }

    let branch = run_git_cmd(&path, &["rev-parse", "--abbrev-ref", "HEAD"]);
    let remote_url = run_git_cmd(&path, &["remote", "get-url", "origin"]);
    let parsed_github = remote_url.as_ref().and_then(|u| parse_github(u));
    
    let head_str = run_git_cmd(&path, &["log", "-1", "--format=%h%x1f%s%x1f%an%x1f%cr%x1f%ct"]);
    let head = head_str.as_deref().and_then(parse_commit);

    let dirty_str = run_git_cmd(&path, &["status", "--porcelain"]);
    let dirty_count = dirty_str.map(|s| if s.is_empty() { 0 } else { s.lines().count() }).unwrap_or(0);

    let ahead_behind_str = run_git_cmd(&path, &["rev-list", "--left-right", "--count", "@{u}...HEAD"]);
    let mut ahead = None;
    let mut behind = None;
    if let Some(ab) = ahead_behind_str {
        let parts: Vec<&str> = ab.split_whitespace().collect();
        if parts.len() == 2 {
            behind = parts[0].parse().ok();
            ahead = parts[1].parse().ok();
        }
    }

    Ok(GitInfo {
        is_repo: true,
        branch,
        remote_url,
        parsed_github,
        head,
        dirty_count,
        ahead,
        behind,
        upstream: None,
    })
}

#[tauri::command]
pub async fn git_branches(path: String, fetch: bool) -> Result<Vec<BranchInfo>, String> {
    if fetch {
        let _ = git_cmd(&path)
            .args(&["fetch", "--prune"])
            .output();
    }

    let branches_str = run_git_cmd(&path, &[
        "for-each-ref",
        "--format=%(refname:short)%x1f%(objectname:short)%x1f%(contents:subject)%x1f%(authorname)%x1f%(committerdate:relative)%x1f%(committerdate:unix)",
        "refs/heads", "refs/remotes/origin"
    ]).unwrap_or_default();

    let mut branches = Vec::new();
    let current_branch = run_git_cmd(&path, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_default();

    for line in branches_str.lines() {
        if line.is_empty() { continue; }
        let parts: Vec<&str> = line.split('\x1f').collect();
        if parts.is_empty() { continue; }
        
        let mut name = parts[0].to_string();
        let is_remote = name.starts_with("origin/");
        if is_remote {
            name = name["origin/".len()..].to_string();
            if name == "HEAD" { continue; }
        }

        let is_current = name == current_branch && !is_remote;

        let short = parts.get(1).map(|s| s.to_string());
        let message = parts.get(2).map(|s| s.to_string());
        let author = parts.get(3).map(|s| s.to_string());
        let committed_rel = parts.get(4).map(|s| s.to_string());
        let committed_ms = parts.get(5).and_then(|s| s.parse().ok());

        branches.push(BranchInfo {
            name,
            is_current,
            is_remote,
            short,
            message,
            author,
            committed_rel,
            committed_ms,
        });
    }

    let mut unique_branches: std::collections::HashMap<String, BranchInfo> = std::collections::HashMap::new();
    for b in branches {
        if let Some(existing) = unique_branches.get_mut(&b.name) {
            if !b.is_remote {
                let mut new_b = b.clone();
                new_b.is_remote = true;
                *existing = new_b;
            } else {
                existing.is_remote = true;
            }
        } else {
            unique_branches.insert(b.name.clone(), b);
        }
    }

    let mut res: Vec<BranchInfo> = unique_branches.into_values().collect();
    res.sort_by(|a, b| b.committed_ms.cmp(&a.committed_ms));

    Ok(res)
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

#[tauri::command]
pub async fn preview_branch(path: String, branch: String, state: State<'_, AppState>) -> Result<String, String> {
    let project_name = std::path::Path::new(&path).file_name().unwrap_or_default().to_string_lossy();
    let dir = state.data_dir.join("worktrees").join(sanitize(&project_name)).join(sanitize(&branch));

    if !dir.exists() {
        let status = git_cmd(&path)
            .args(&["worktree", "add", "--force", &dir.to_string_lossy(), &branch])
            .status();
        if status.is_err() || !status.unwrap().success() {
            return Err("Failed to create worktree".to_string());
        }
    } else {
        let _ = git_cmd(&dir.to_string_lossy())
            .args(&["checkout", &branch])
            .status();
        let _ = git_cmd(&dir.to_string_lossy())
            .args(&["pull", "--ff-only"])
            .status();
    }

    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn remove_worktree(path: String, branch: String, state: State<'_, AppState>) -> Result<(), String> {
    let project_name = std::path::Path::new(&path).file_name().unwrap_or_default().to_string_lossy();
    let dir = state.data_dir.join("worktrees").join(sanitize(&project_name)).join(sanitize(&branch));

    let status = git_cmd(&path)
        .args(&["worktree", "remove", "--force", &dir.to_string_lossy()])
        .status();

    if status.is_err() || !status.unwrap().success() {
        return Err("Failed to remove worktree".to_string());
    }
    
    Ok(())
}
