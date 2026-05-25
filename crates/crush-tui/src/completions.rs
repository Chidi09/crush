use clap::Command;
use clap_complete::{Generator, Shell};

pub fn generate_completions(shell: Shell, cmd: &mut Command) -> String {
    let mut buf = Vec::new();
    clap_complete::generate(shell, cmd, "crush", &mut buf);
    String::from_utf8(buf).unwrap_or_default()
}

pub fn generate_all_completions(cmd: &mut Command) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    for shell in [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::PowerShell] {
        let name = format!("{}", shell);
        let content = generate_completions(shell, cmd);
        out.insert(name, content);
    }
    out
}

pub fn save_completions(cmd: &mut Command, output_dir: &std::path::Path) -> std::io::Result<()> {
    let shells = [
        (Shell::Bash, "crush.bash"),
        (Shell::Zsh, "_crush"),
        (Shell::Fish, "crush.fish"),
        (Shell::PowerShell, "_crush.ps1"),
    ];
    for (shell, filename) in &shells {
        let mut buf = Vec::new();
        clap_complete::generate(*shell, cmd, "crush", &mut buf);
        let path = output_dir.join(filename);
        std::fs::write(&path, &buf)?;
    }
    Ok(())
}
