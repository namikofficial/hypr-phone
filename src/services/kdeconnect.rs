//! KDE Connect bridge. `kdeconnect-cli` is treated as an optional
//! capability; absence is not a fatal error.

use anyhow::{anyhow, bail, Context, Result};

use crate::cli::MediaAction;

pub fn kdeconnect_path() -> Result<std::path::PathBuf> {
    which::which("kdeconnect-cli").map_err(|_| {
        anyhow!(
            "KDE Connect CLI is not installed. Install `kdeconnect` to enable battery/ring/notify features."
        )
    })
}

pub fn list_devices() -> Result<Vec<KdeDevice>> {
    let raw = run(&["--list-devices", "--id-name-only"])?;
    let mut out = Vec::new();
    for line in raw.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            out.push(KdeDevice {
                id: parts[0].to_string(),
                name: parts[1..].join(" "),
            });
        }
    }
    Ok(out)
}

#[derive(Debug, Clone)]
pub struct KdeDevice {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct BatteryInfo {
    pub level_percent: u8,
    pub charging: bool,
}

pub fn battery(target: Option<&str>) -> Result<BatteryInfo> {
    let raw = run_with_target(target, &["--battery"])?;
    parse_battery(&raw)
}

fn parse_battery(raw: &str) -> Result<BatteryInfo> {
    // Output looks like:
    //   Battery: 92%
    // or:
    //   Battery: 92% (charging)
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Battery:") {
            let rest = rest.trim();
            let mut parts = rest.split_whitespace();
            let level_str = parts.next().unwrap_or("0").trim_end_matches('%');
            let level: u8 = level_str.parse().unwrap_or(0);
            let charging = trimmed.contains("(charging)")
                || trimmed.contains("Charging")
                || trimmed.contains("Plugged in");
            return Ok(BatteryInfo {
                level_percent: level,
                charging,
            });
        }
    }
    Ok(BatteryInfo {
        level_percent: 0,
        charging: false,
    })
}

pub fn ring(target: Option<&str>) -> Result<()> {
    run_with_target(target, &["--ring"]).map(|_| ())
}

pub fn notify(target: Option<&str>, title: &str, body: &str) -> Result<()> {
    if title.trim().is_empty() {
        bail!("notification title cannot be empty");
    }
    run_with_target(target, &["--send-notification", title, body]).map(|_| ())
}

pub fn media_action(target: Option<&str>, action: MediaAction) -> Result<()> {
    let name = match action {
        MediaAction::Play => "play",
        MediaAction::Pause => "pause",
        MediaAction::PlayPause => "playpause",
        MediaAction::Stop => "stop",
        MediaAction::Next => "next",
        MediaAction::Previous => "previous",
    };
    run_with_target(target, &["--mpris", name]).map(|_| ())
}

/// Send a local file to a paired KDE Connect device via `kdeconnect-cli --share`.
pub fn send_file(target: &str, path: &std::path::Path) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("file not found: {}", path.display());
    }
    let path_str = path.to_string_lossy().to_string();
    run_with_target(Some(target), &["--share", &path_str]).map(|_| ())
}

/// Raw invocation: pass argv directly to `kdeconnect-cli`.
pub fn run_raw(args: &[&str]) -> Result<String> {
    run(args)
}

fn run_with_target(target: Option<&str>, args: &[&str]) -> Result<String> {
    let mut full_args = Vec::new();
    if let Some(t) = target {
        full_args.push("--device".to_string());
        full_args.push(t.to_string());
    }
    full_args.extend(args.iter().map(|s| s.to_string()));
    run_owned(&full_args)
}

fn run(args: &[&str]) -> Result<String> {
    let owned: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
    run_owned(&owned)
}

fn run_owned(args: &[String]) -> Result<String> {
    let bin = kdeconnect_path()?;
    let output = std::process::Command::new(bin)
        .args(args)
        .output()
        .with_context(|| format!("failed to execute `kdeconnect-cli {}`", args.join(" ")))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        bail!(
            "`kdeconnect-cli {}` exited {}: {}{}",
            args.join(" "),
            output.status,
            stderr.trim(),
            if stderr.trim().is_empty() {
                format!(" {}", stdout.trim())
            } else {
                String::new()
            }
        );
    }
    Ok(if stdout.is_empty() { stderr } else { stdout })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_battery_basic() {
        let info = parse_battery("Battery: 92%").unwrap();
        assert_eq!(info.level_percent, 92);
        assert!(!info.charging);
    }

    #[test]
    fn parses_battery_with_charging() {
        let info = parse_battery("Battery: 92% (charging)").unwrap();
        assert_eq!(info.level_percent, 92);
        assert!(info.charging);
    }

    #[test]
    fn parses_battery_missing_returns_zero() {
        let info = parse_battery("no battery info").unwrap();
        assert_eq!(info.level_percent, 0);
    }
}
