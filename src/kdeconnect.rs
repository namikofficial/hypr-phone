use std::process::Command;

use anyhow::{bail, Context};

use crate::{
    cli::MediaAction,
    errors::{command_failed_error, command_spawn_error, missing_dependency, Result},
};

pub fn kdeconnect_path() -> Result<std::path::PathBuf> {
    which::which("kdeconnect-cli").map_err(|_| {
        missing_dependency(
            "kdeconnect-cli",
            "Install `kdeconnect` package to enable KDE Connect bridge commands.",
        )
    })
}

pub fn list_devices() -> Result<String> {
    run_kdeconnect_command(&["--list-devices", "--id-name-only"])
}

pub fn battery(target: Option<&str>) -> Result<String> {
    let mut args = Vec::new();
    if let Some(device) = target {
        args.push("--device");
        args.push(device);
    }
    args.push("--battery");
    run_kdeconnect_command(&args)
}

pub fn ring(target: Option<&str>) -> Result<String> {
    let mut args = Vec::new();
    if let Some(device) = target {
        args.push("--device");
        args.push(device);
    }
    args.push("--ring");
    run_kdeconnect_command(&args)
}

pub fn notify(target: Option<&str>, title: &str, body: &str) -> Result<String> {
    if title.trim().is_empty() {
        bail!("notification title cannot be empty");
    }

    let mut args = Vec::new();
    if let Some(device) = target {
        args.push("--device");
        args.push(device);
    }
    args.push("--send-notification");
    args.push(title);
    args.push(body);
    run_kdeconnect_command(&args)
}

pub fn media_action(target: Option<&str>, action: MediaAction) -> Result<String> {
    let action_name = match action {
        MediaAction::Play => "play",
        MediaAction::Pause => "pause",
        MediaAction::PlayPause => "playpause",
        MediaAction::Stop => "stop",
        MediaAction::Next => "next",
        MediaAction::Previous => "previous",
    };

    let mut args = Vec::new();
    if let Some(device) = target {
        args.push("--device");
        args.push(device);
    }
    args.push("--mpris");
    args.push(action_name);
    run_kdeconnect_command(&args)
}

fn run_kdeconnect_command(args: &[&str]) -> Result<String> {
    let kdeconnect = kdeconnect_path()?;
    let output = Command::new(&kdeconnect)
        .args(args)
        .output()
        .map_err(|err| command_spawn_error("kdeconnect-cli", args, err))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        if stdout.is_empty() {
            Ok(stderr)
        } else {
            Ok(stdout)
        }
    } else {
        Err(command_failed_error(
            "kdeconnect-cli",
            args,
            output.status,
            &stdout,
            &stderr,
        ))
    }
}

pub fn resolve_kde_target(
    alias_target: Option<&str>,
    alias_lookup: Option<&str>,
) -> Option<String> {
    alias_lookup
        .map(str::to_string)
        .or_else(|| alias_target.map(str::to_string))
}

pub fn validate_kde_presence() -> Result<()> {
    kdeconnect_path().context("KDE Connect bridge is unavailable")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_alias_then_direct_target() {
        assert_eq!(
            resolve_kde_target(Some("pixel"), Some("kde-id")),
            Some("kde-id".to_string())
        );
        assert_eq!(
            resolve_kde_target(Some("kde-id"), None),
            Some("kde-id".to_string())
        );
        assert_eq!(resolve_kde_target(None, None), None);
    }
}
