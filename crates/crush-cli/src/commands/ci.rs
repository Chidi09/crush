//! `crush ci` — generate a working CI/CD pipeline for the detected stack.
//!
//! Supports GitHub Actions, AppVeyor, and Codemagic. The pipeline is tuned to
//! the stack crush already detects (toolchain setup + install/build/test), so
//! users get a runnable config instead of a blank template.

use std::path::Path;
use owo_colors::OwoColorize;
use crush_build::StackDetector;

/// Which CI system to target.
#[derive(Clone, Copy, PartialEq)]
enum System { GitHub, AppVeyor, Codemagic }

impl System {
    fn parse(s: &str) -> Option<System> {
        match s.to_lowercase().replace(['-', '_'], "").as_str() {
            "github" | "githubactions" | "actions" | "gha" => Some(System::GitHub),
            "appveyor" => Some(System::AppVeyor),
            "codemagic" => Some(System::Codemagic),
            _ => None,
        }
    }
    fn out_path(&self) -> &'static str {
        match self {
            System::GitHub => ".github/workflows/crush.yml",
            System::AppVeyor => "appveyor.yml",
            System::Codemagic => "codemagic.yaml",
        }
    }
}

/// Normalized stack facts the templates need.
struct Stack {
    /// Short language key: node, python, rust, go, java, flutter, react-native, …
    lang: String,
    runtime_version: String,
    install: String,
    build: String,
    test: String,
}

pub async fn exec(system: Option<String>, force: bool) -> anyhow::Result<()> {
    let root = std::env::current_dir()?;
    let detector = StackDetector::new();
    let detected = detector.detect(&root).await
        .map_err(|e| anyhow::anyhow!("stack detection failed: {e}"))?;
    let lang = detected.language.split(' ').next().unwrap_or("generic").to_lowercase();

    // Auto-pick a sensible system when none is given: Flutter/RN → Codemagic
    // (built for mobile), everything else → GitHub Actions.
    let sys = match system {
        Some(s) => System::parse(&s)
            .ok_or_else(|| anyhow::anyhow!("unknown CI system '{s}' — use github, appveyor, or codemagic"))?,
        None => if lang == "flutter" || lang == "react-native" { System::Codemagic } else { System::GitHub },
    };

    let install = if detected.dev_install_command.trim().is_empty() {
        default_install(&lang)
    } else { detected.dev_install_command.trim().to_string() };
    // The detector sometimes reports the install command as the "build" (e.g.
    // Node's `npm install`); don't emit a redundant build step — use a real one.
    let detected_build = detected.build_command.trim();
    let build = if detected_build.is_empty() || detected_build == install || detected_build.contains("install") {
        default_build(&lang)
    } else { detected_build.to_string() };
    let stack = Stack {
        lang: lang.clone(),
        runtime_version: detected.runtime_version.clone(),
        install,
        build,
        test: default_test(&lang),
    };

    let out_rel = sys.out_path();
    let out_path = root.join(out_rel);
    if out_path.exists() && !force {
        anyhow::bail!("{out_rel} already exists — pass --force to overwrite");
    }
    if let Some(parent) = out_path.parent() { std::fs::create_dir_all(parent).ok(); }

    let content = match sys {
        System::GitHub => github_actions(&stack),
        System::AppVeyor => appveyor(&stack),
        System::Codemagic => codemagic(&stack),
    };
    std::fs::write(&out_path, content)?;

    println!(" {} wrote {}", "✓".green().bold(), out_rel.bold());
    println!("   {} stack: {} · build: {}", "↳".cyan(), stack.lang.bold(), stack.build.dimmed());
    match sys {
        System::GitHub => println!("   {} commit it and push — Actions runs on the next push/PR", "↳".cyan()),
        System::AppVeyor => println!("   {} connect the repo at ci.appveyor.com, then push", "↳".cyan()),
        System::Codemagic => println!("   {} connect the repo at codemagic.io, then start a build", "↳".cyan()),
    }
    Ok(())
}

fn default_install(lang: &str) -> String {
    match lang {
        "node" | "typescript" | "react-native" => "npm ci".into(),
        "python" => "pip install -r requirements.txt".into(),
        "rust" => "cargo fetch".into(),
        "go" => "go mod download".into(),
        "java" => "./gradlew dependencies".into(),
        "flutter" => "flutter pub get".into(),
        _ => "echo 'no install step'".into(),
    }
}

fn default_build(lang: &str) -> String {
    match lang {
        "node" | "typescript" => "npm run build --if-present".into(),
        "python" => "python -m compileall .".into(),
        "rust" => "cargo build --release".into(),
        "go" => "go build ./...".into(),
        "java" => "./gradlew build".into(),
        "flutter" => "flutter build apk --release".into(),
        "react-native" => "npx react-native build-android --mode=release".into(),
        _ => "echo 'no build step'".into(),
    }
}

fn default_test(lang: &str) -> String {
    match lang {
        "node" | "typescript" | "react-native" => "npm test --if-present".into(),
        "python" => "pytest -q || true".into(),
        "rust" => "cargo test".into(),
        "go" => "go test ./...".into(),
        "java" => "./gradlew test".into(),
        "flutter" => "flutter test".into(),
        _ => "echo 'no tests'".into(),
    }
}

/// GitHub Actions setup step(s) for a language.
fn gha_setup(s: &Stack) -> String {
    let v = ver_or(&s.runtime_version, s);
    match s.lang.as_str() {
        "node" | "typescript" | "react-native" =>
            format!("      - uses: actions/setup-node@v4\n        with:\n          node-version: '{v}'\n          cache: npm"),
        "python" =>
            format!("      - uses: actions/setup-python@v5\n        with:\n          python-version: '{v}'"),
        "rust" =>
            "      - uses: dtolnay/rust-toolchain@stable".into(),
        "go" =>
            format!("      - uses: actions/setup-go@v5\n        with:\n          go-version: '{v}'"),
        "java" =>
            format!("      - uses: actions/setup-java@v4\n        with:\n          distribution: temurin\n          java-version: '{v}'"),
        "flutter" =>
            "      - uses: subosito/flutter-action@v2\n        with:\n          channel: stable".into(),
        _ => "      # add your toolchain setup here".into(),
    }
}

/// Pick a concrete version, falling back to a sane default per language.
fn ver_or(v: &str, s: &Stack) -> String {
    let v = v.trim();
    if !v.is_empty() && v != "stable" && v != "lts" && v.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        return v.to_string();
    }
    match s.lang.as_str() {
        "node" | "typescript" | "react-native" => "20".into(),
        "python" => "3.12".into(),
        "go" => "1.22".into(),
        "java" => "21".into(),
        _ => "stable".into(),
    }
}

fn github_actions(s: &Stack) -> String {
    let setup = gha_setup(s);
    // Flutter/RN also need Java for the Android build.
    let extra = if s.lang == "react-native" || s.lang == "flutter" {
        "      - uses: actions/setup-java@v4\n        with:\n          distribution: temurin\n          java-version: '17'\n"
    } else { "" };
    format!(
"# Generated by `crush ci` — tune as needed.
name: CI

on:
  push:
    branches: [ main, master ]
  pull_request:

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
{setup}
{extra}      - name: Install
        run: {install}
      - name: Build
        run: {build}
      - name: Test
        run: {test}
",
        setup = setup,
        extra = extra,
        install = s.install,
        build = s.build,
        test = s.test,
    )
}

fn appveyor(s: &Stack) -> String {
    // AppVeyor runs on Windows by default — good for crush's Windows-first story.
    let install_toolchain = match s.lang.as_str() {
        "node" | "typescript" | "react-native" => "  - ps: Install-Product node 20",
        "rust" => "  - appveyor DownloadFile https://win.rustup.rs/x86_64 -FileName rustup-init.exe\n  - rustup-init.exe -y --default-toolchain stable\n  - set PATH=%USERPROFILE%\\.cargo\\bin;%PATH%",
        "go" => "  - set PATH=C:\\go\\bin;%PATH%",
        "python" => "  - set PATH=C:\\Python312;C:\\Python312\\Scripts;%PATH%",
        _ => "  # add toolchain install here",
    };
    format!(
"# Generated by `crush ci` — tune as needed.
image: Visual Studio 2022

install:
{toolchain}

build_script:
  - {install}
  - {build}

test_script:
  - {test}
",
        toolchain = install_toolchain,
        install = s.install,
        build = s.build,
        test = s.test,
    )
}

fn codemagic(s: &Stack) -> String {
    // Codemagic shines for mobile; emit a Flutter/RN workflow, else a generic one.
    match s.lang.as_str() {
        "flutter" => format!(
"# Generated by `crush ci` — tune as needed.
workflows:
  flutter:
    name: Flutter CI
    instance_type: mac_mini_m2
    environment:
      flutter: stable
    scripts:
      - name: Get packages
        script: flutter pub get
      - name: Analyze
        script: flutter analyze || true
      - name: Test
        script: {test}
      - name: Build APK
        script: {build}
    artifacts:
      - build/**/outputs/**/*.apk
",
            test = s.test, build = s.build,
        ),
        "react-native" => format!(
"# Generated by `crush ci` — tune as needed.
workflows:
  react-native-android:
    name: React Native Android
    instance_type: mac_mini_m2
    environment:
      node: 20
    scripts:
      - name: Install
        script: {install}
      - name: Test
        script: {test}
      - name: Build Android
        script: {build}
    artifacts:
      - android/app/build/outputs/**/*.apk
",
            install = s.install, test = s.test, build = s.build,
        ),
        _ => format!(
"# Generated by `crush ci` — tune as needed.
workflows:
  build:
    name: {lang} CI
    instance_type: linux_x2
    scripts:
      - name: Install
        script: {install}
      - name: Build
        script: {build}
      - name: Test
        script: {test}
",
            lang = s.lang, install = s.install, build = s.build, test = s.test,
        ),
    }
}
