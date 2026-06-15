//! Detect *where a project deploys* from config markers in the tree.
//!
//! Most platforms drop a tell-tale file: `vercel.json`/`.vercel`, `netlify.toml`,
//! `render.yaml`, `fly.toml`, `wrangler.toml`, … We scan the repo root and one
//! level of common monorepo subdirs (so `apps/web/vercel.json` counts), plus the
//! Crushfile `[deploy] provider`. The result feeds the GUI so a project shows
//! "Deploys to: Vercel · Hetzner" without anyone configuring it.

use std::path::Path;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeployTarget {
    /// Display name, e.g. "Vercel".
    pub platform: String,
    /// The marker that revealed it (relative path or "Crushfile"), for the tooltip.
    pub source: String,
    /// TechIcon key, e.g. "vercel".
    pub icon: String,
    /// Inferred one-click deploy command (run in the project dir, in a terminal).
    pub deploy_command: String,
}

/// The canonical deploy command for a platform — what a one-click "Deploy"
/// should run. Git-integration platforms (Render, GitHub Pages) deploy on push.
pub fn deploy_command_for(platform: &str) -> String {
    match platform {
        "Vercel" => "vercel --prod",
        "Netlify" => "netlify deploy --prod",
        "Fly.io" => "fly deploy",
        "Railway" => "railway up",
        "Cloudflare" => "wrangler deploy",
        "AWS" => "serverless deploy",
        "Render" | "GitHub Pages" => "git push",
        // crush-managed providers go through the crush pipeline.
        "Hetzner" | "DigitalOcean" | "Google Cloud" | "SSH / VPS" => "crush deploy",
        _ => "crush deploy",
    }
    .to_string()
}

/// (platform, icon, is_dir, marker filename). File-existence checks only.
const MARKERS: &[(&str, &str, bool, &str)] = &[
    ("Vercel",     "vercel",       false, "vercel.json"),
    ("Vercel",     "vercel",       true,  ".vercel"),
    ("Vercel",     "vercel",       false, "now.json"),
    ("Netlify",    "netlify",      false, "netlify.toml"),
    ("Netlify",    "netlify",      true,  ".netlify"),
    ("Render",     "render",       false, "render.yaml"),
    ("Render",     "render",       false, "render.yml"),
    ("Fly.io",     "flydotio",     false, "fly.toml"),
    ("Railway",    "railway",      false, "railway.json"),
    ("Railway",    "railway",      false, "railway.toml"),
    ("Cloudflare", "cloudflare",   false, "wrangler.toml"),
    ("Cloudflare", "cloudflare",   false, "wrangler.jsonc"),
    ("AWS",        "amazonaws",    false, "serverless.yml"),
    ("AWS",        "amazonaws",    false, "samconfig.toml"),
    ("GitHub Pages","github",      false, "CNAME"),
];

/// Subdirectories worth scanning one level deep for monorepos.
const SUBDIR_HINTS: &[&str] = &["apps", "packages", "services", "src", "web", "frontend", "backend", "api"];

/// Detect deploy platforms for the project at `root`. Deduplicated by platform.
pub fn detect_deploy_targets(root: &Path) -> Vec<DeployTarget> {
    let mut found: Vec<DeployTarget> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    let mut add = |platform: &str, icon: &str, source: String, out: &mut Vec<DeployTarget>, seen: &mut std::collections::HashSet<String>| {
        if seen.insert(platform.to_string()) {
            out.push(DeployTarget {
                platform: platform.to_string(),
                icon: icon.to_string(),
                source,
                deploy_command: deploy_command_for(platform),
            });
        }
    };

    // Directories to check: root, then each immediate child of the hint dirs and
    // the hint dirs themselves (covers apps/web, packages/site, etc.).
    let mut scan_dirs: Vec<std::path::PathBuf> = vec![root.to_path_buf()];
    for hint in SUBDIR_HINTS {
        let d = root.join(hint);
        if d.is_dir() {
            scan_dirs.push(d.clone());
            if let Ok(entries) = std::fs::read_dir(&d) {
                for e in entries.flatten() {
                    if e.path().is_dir() { scan_dirs.push(e.path()); }
                }
            }
        }
    }

    for dir in &scan_dirs {
        for (platform, icon, is_dir, marker) in MARKERS {
            let p = dir.join(marker);
            let hit = if *is_dir { p.is_dir() } else { p.is_file() };
            if hit {
                let rel = p.strip_prefix(root).unwrap_or(&p).to_string_lossy().replace('\\', "/");
                add(platform, icon, rel, &mut found, &mut seen);
            }
        }
    }

    // Crushfile [deploy] provider (e.g. provider = "hetzner") — no marker file.
    if let Some(provider) = crushfile_deploy_provider(root) {
        let (platform, icon) = match provider.to_lowercase().as_str() {
            "hetzner" => ("Hetzner", "hetzner"),
            "fly" | "flyio" => ("Fly.io", "flydotio"),
            "railway" => ("Railway", "railway"),
            "digitalocean" => ("DigitalOcean", "digitalocean"),
            "gcp" | "googlecloud" => ("Google Cloud", "googlecloud"),
            "aws" => ("AWS", "amazonaws"),
            "render" => ("Render", "render"),
            "vercel" => ("Vercel", "vercel"),
            "netlify" => ("Netlify", "netlify"),
            "ssh" => ("SSH / VPS", "ssh"),
            other => {
                // Unknown provider: still show it, capitalized.
                add(&capitalize(other), "rocket", "Crushfile".into(), &mut found, &mut seen);
                return found;
            }
        };
        add(platform, icon, "Crushfile".into(), &mut found, &mut seen);
    }

    found
}

/// Read `provider = "..."` under a `[deploy]` table in the Crushfile, if present.
fn crushfile_deploy_provider(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join("Crushfile")).ok()?;
    let mut in_deploy = false;
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            in_deploy = l == "[deploy]";
            continue;
        }
        if in_deploy {
            if let Some(rest) = l.strip_prefix("provider") {
                let v = rest.trim_start_matches([' ', '=']).trim().trim_matches(['"', '\'']);
                if !v.is_empty() {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detects_vercel_at_root() {
        let d = tempfile::TempDir::new().unwrap();
        fs::write(d.path().join("vercel.json"), "{}").unwrap();
        let t = detect_deploy_targets(d.path());
        assert!(t.iter().any(|x| x.platform == "Vercel"));
    }

    #[test]
    fn detects_in_monorepo_subdir() {
        let d = tempfile::TempDir::new().unwrap();
        fs::create_dir_all(d.path().join("apps/web")).unwrap();
        fs::write(d.path().join("apps/web/netlify.toml"), "").unwrap();
        let t = detect_deploy_targets(d.path());
        assert!(t.iter().any(|x| x.platform == "Netlify" && x.source.contains("apps/web")));
    }

    #[test]
    fn dedups_multiple_markers() {
        let d = tempfile::TempDir::new().unwrap();
        fs::write(d.path().join("vercel.json"), "{}").unwrap();
        fs::create_dir(d.path().join(".vercel")).unwrap();
        let t = detect_deploy_targets(d.path());
        assert_eq!(t.iter().filter(|x| x.platform == "Vercel").count(), 1);
    }

    #[test]
    fn reads_crushfile_provider() {
        let d = tempfile::TempDir::new().unwrap();
        fs::write(d.path().join("Crushfile"), "[deploy]\nprovider = \"hetzner\"\n").unwrap();
        let t = detect_deploy_targets(d.path());
        assert!(t.iter().any(|x| x.platform == "Hetzner" && x.source == "Crushfile"));
    }

    #[test]
    fn empty_when_no_markers() {
        let d = tempfile::TempDir::new().unwrap();
        fs::write(d.path().join("package.json"), "{}").unwrap();
        assert!(detect_deploy_targets(d.path()).is_empty());
    }
}
