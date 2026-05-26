use clap::Command;
use clap_complete::Shell;

pub fn generate_completions(shell: Shell, _cmd: &mut Command) -> String {
    match shell {
        Shell::Bash => BASH_COMPLETION.to_string(),
        Shell::Zsh => ZSH_COMPLETION.to_string(),
        Shell::Fish => FISH_COMPLETION.to_string(),
        Shell::PowerShell => POWERSHELL_COMPLETION.to_string(),
        _ => "".to_string(),
    }
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
        let content = generate_completions(*shell, cmd);
        let path = output_dir.join(filename);
        std::fs::write(&path, content)?;
    }
    Ok(())
}

const BASH_COMPLETION: &str = r#"_crush_completions() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    
    opts="run stop logs ps stats watch build completions network volume inspect debug health daemon update"
    
    if [[ ${COMP_CWORD} -eq 1 ]] ; then
        COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
        return 0
    fi
    
    case "${prev}" in
        stop|logs|inspect|debug|health)
            local containers=$(crush __complete containers 2>/dev/null)
            COMPREPLY=( $(compgen -W "${containers}" -- "${cur}") )
            return 0
            ;;
        run)
            local images=$(crush __complete images 2>/dev/null)
            COMPREPLY=( $(compgen -W "${images}" -- "${cur}") )
            return 0
            ;;
        volume)
            local volumes=$(crush __complete volumes 2>/dev/null)
            COMPREPLY=( $(compgen -W "${volumes}" -- "${cur}") )
            return 0
            ;;
        network)
            local networks=$(crush __complete networks 2>/dev/null)
            COMPREPLY=( $(compgen -W "${networks}" -- "${cur}") )
            return 0
            ;;
    esac
}
complete -F _crush_completions crush
"#;

const ZSH_COMPLETION: &str = r#"#compdef crush

_crush() {
    local line state
    _arguments -C \
        "1:commands:((run\:'Run a container' stop\:'Stop a container' logs\:'Show logs' ps\:'List containers' stats\:'Show stats' watch\:'Watch files' build\:'Build image' completions\:'Generate completions' network\:'Manage networks' volume\:'Manage volumes' inspect\:'Inspect container' debug\:'AI debug' daemon\:'Run daemon' health\:'Run health checks'))" \
        "*::arg:->args"

    case $state in
        args)
            case $words[1] in
                stop|logs|inspect|debug|health)
                    local -a containers
                    containers=(${(f)"$(crush __complete containers 2>/dev/null)"})
                    _describe 'containers' containers
                    ;;
                run)
                    local -a images
                    images=(${(f)"$(crush __complete images 2>/dev/null)"})
                    _describe 'images' images
                    ;;
                volume)
                    local -a volumes
                    volumes=(${(f)"$(crush __complete volumes 2>/dev/null)"})
                    _describe 'volumes' volumes
                    ;;
                network)
                    local -a networks
                    networks=(${(f)"$(crush __complete networks 2>/dev/null)"})
                    _describe 'networks' networks
                    ;;
            esac
            ;;
    esac
}
"#;

const FISH_COMPLETION: &str = r#"complete -c crush -f
complete -c crush -n "not __fish_seen_subcommand_from run stop logs ps stats watch build completions network volume inspect debug health daemon" -a "run stop logs ps stats watch build completions network volume inspect debug health daemon"

complete -c crush -n "__fish_seen_subcommand_from stop logs inspect debug health" -f -a "(crush __complete containers 2>/dev/null)"
complete -c crush -n "__fish_seen_subcommand_from run" -f -a "(crush __complete images 2>/dev/null)"
complete -c crush -n "__fish_seen_subcommand_from volume" -f -a "(crush __complete volumes 2>/dev/null)"
complete -c crush -n "__fish_seen_subcommand_from network" -f -a "(crush __complete networks 2>/dev/null)"
"#;

const POWERSHELL_COMPLETION: &str = r#"Register-ArgumentCompleter -CommandName crush -ParameterName image -ScriptBlock {
    param($commandName, $parameterName, $wordToComplete, $commandAst, $fakeBoundParameters)
    crush __complete images 2>$null | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
        [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
    }
}
Register-ArgumentCompleter -CommandName crush -ArgumentIndex 1 -ScriptBlock {
    param($commandName, $parameterName, $wordToComplete, $commandAst, $fakeBoundParameters)
    $command = $commandAst.CommandElements[1].Value
    if ($command -in @('stop', 'logs', 'inspect', 'debug', 'health')) {
        crush __complete containers 2>$null | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
    }
}
"#;

