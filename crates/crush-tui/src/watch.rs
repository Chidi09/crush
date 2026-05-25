use std::path::Path;
use std::time::Duration;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use crush_types::{Result, CrushError};

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeClass {
    SourceOnly,
    LockfileChanged,
    CrushfileChanged,
    FullRebuild,
    Unknown,
}

pub fn classify_change(path: &Path) -> ChangeClass {
    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if filename == "Crushfile" || filename == "crushfile.toml" {
        return ChangeClass::CrushfileChanged;
    }

    let lockfiles = ["package-lock.json", "yarn.lock", "pnpm-lock.yaml",
        "Cargo.lock", "go.sum", "Gemfile.lock", "poetry.lock", "composer.lock"];
    if lockfiles.contains(&filename) {
        return ChangeClass::LockfileChanged;
    }

    let extensions = [".rs", ".go", ".py", ".js", ".ts", ".jsx", ".tsx", ".css", ".scss", ".html", ".vue", ".svelte"];
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_str = format!(".{}", ext);
        if extensions.contains(&ext_str.as_str()) {
            return ChangeClass::SourceOnly;
        }
    }

    ChangeClass::Unknown
}

pub struct FileWatcher {
    _watcher: RecommendedWatcher,
}

impl FileWatcher {
    pub fn new<F>(paths: &[&Path], callback: F) -> Result<Self>
    where F: Fn(ChangeClass) + Send + 'static
    {
        let (tx, rx) = std::sync::mpsc::channel::<ChangeClass>();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    if let Some(path) = event.paths.first() {
                        if event.kind == EventKind::Modify(notify::event::ModifyKind::Data(_))
                            || event.kind == EventKind::Create(notify::event::CreateKind::File)
                        {
                            let classification = classify_change(path);
                            tx.send(classification).ok();
                        }
                    }
                }
            },
            Config::default().with_poll_interval(Duration::from_millis(200)),
        ).map_err(|e| CrushError::ApiError(format!("File watcher failed: {}", e)))?;

        for path in paths {
            if path.exists() {
                watcher.watch(path, RecursiveMode::Recursive)
                    .map_err(|e| CrushError::ApiError(format!("Watch error: {}", e)))?;
            }
        }

        // Debounce: aggregate classifications over 100ms window, emit strongest
        std::thread::spawn(move || {
            let mut last_change: Option<ChangeClass> = None;
            loop {
                match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(classification) => {
                        last_change = Some(match (last_change.take(), classification) {
                            (Some(ChangeClass::FullRebuild), _) => ChangeClass::FullRebuild,
                            (_, ChangeClass::FullRebuild) => ChangeClass::FullRebuild,
                            (Some(ChangeClass::CrushfileChanged), _) => ChangeClass::CrushfileChanged,
                            (_, ChangeClass::CrushfileChanged) => ChangeClass::CrushfileChanged,
                            (Some(ChangeClass::LockfileChanged), _) => ChangeClass::LockfileChanged,
                            (_, ChangeClass::LockfileChanged) => ChangeClass::LockfileChanged,
                            (_, ChangeClass::SourceOnly) => ChangeClass::SourceOnly,
                            (Some(existing), _) => existing,
                            (None, _) => ChangeClass::Unknown,
                        });
                    }
                    Err(_) => {
                        if let Some(class) = last_change.take() {
                            callback(class);
                        }
                    }
                }
            }
        });

        Ok(Self { _watcher: watcher })
    }
}
