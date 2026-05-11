use std::process::ExitStatus;

use anyhow::anyhow;

pub type Result<T> = anyhow::Result<T>;

pub fn missing_dependency(binary: &str, hint: &str) -> anyhow::Error {
    anyhow!("Missing dependency `{binary}` in PATH. {hint}")
}

pub fn command_spawn_error(program: &str, args: &[&str], source: std::io::Error) -> anyhow::Error {
    anyhow!(
        "Failed to execute `{}`: {source}. Ensure `{program}` is installed and executable.",
        format_command(program, args)
    )
}

pub fn command_failed_error(
    program: &str,
    args: &[&str],
    status: ExitStatus,
    stdout: &str,
    stderr: &str,
) -> anyhow::Error {
    let status_text = status
        .code()
        .map(|code| code.to_string())
        .unwrap_or_else(|| "terminated by signal".to_string());
    let output = if !stderr.trim().is_empty() {
        stderr.trim()
    } else if !stdout.trim().is_empty() {
        stdout.trim()
    } else {
        "No output from command."
    };

    anyhow!(
        "Command `{}` failed with exit status {status_text}. {output}",
        format_command(program, args)
    )
}

fn format_command(program: &str, args: &[&str]) -> String {
    if args.is_empty() {
        program.to_string()
    } else {
        format!("{program} {}", args.join(" "))
    }
}
