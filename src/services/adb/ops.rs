//! ADB operations: command execution, file transfer, clipboard, shell.
//!
//! The discovery module (in `adb_discovery.rs`) builds `PhoneDevice`s;
//! this module wraps the actual `adb` calls.

use std::{
    fs,
    path::Path,
    process::Command,
};

use anyhow::{bail, Context, Result};

// (no unused imports)

pub fn adb_path() -> Result<std::path::PathBuf> {
    which::which("adb").map_err(|_| {
        anyhow::anyhow!("Missing dependency `adb` in PATH. Install Android platform-tools.")
    })
}

/// Capture a screenshot to a file.
pub fn screenshot(serial: Option<&str>, output_path: &Path) -> Result<String> {
    let adb = adb_path()?;
    let mut command = Command::new(adb);
    if let Some(serial) = serial {
        command.args(["-s", serial]);
    }
    command.args(["exec-out", "screencap", "-p"]);
    let output = command
        .output()
        .with_context(|| "failed to execute `adb exec-out screencap -p`")?;
    if !output.status.success() {
        bail!(
            "`adb screencap` exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(output_path, &output.stdout)
        .with_context(|| format!("failed to write screenshot to {}", output_path.display()))?;
    Ok(format!("Saved screenshot to {}", output_path.display()))
}

/// Push a file to the device.
pub fn push_file(serial: Option<&str>, local: &str, remote: &str) -> Result<String> {
    if local.trim().is_empty() || remote.trim().is_empty() {
        bail!("local and remote paths must not be empty");
    }
    run_adb_with_serial(serial, &["push", local, remote])
}

/// Pull a file from the device.
pub fn pull_file(serial: Option<&str>, remote: &str, local: &str) -> Result<String> {
    if local.trim().is_empty() || remote.trim().is_empty() {
        bail!("local and remote paths must not be empty");
    }
    run_adb_with_serial(serial, &["pull", remote, local])
}

/// Install an APK.
pub fn install_apk(serial: Option<&str>, apk_path: &str, reinstall: bool) -> Result<String> {
    if apk_path.trim().is_empty() {
        bail!("apk path must not be empty");
    }
    let mut args = vec!["install".to_string()];
    if reinstall {
        args.push("-r".into());
    }
    args.push(apk_path.into());
    run_adb_with_serial_owned(serial, &args)
}

/// Run an arbitrary `adb shell` command. Args are passed verbatim — no
/// shell interpolation on the local side.
pub fn shell(serial: Option<&str>, command: &[String]) -> Result<String> {
    if command.is_empty() {
        bail!("shell command cannot be empty");
    }
    let mut args = vec!["shell".to_string()];
    args.extend(command.iter().cloned());
    run_adb_with_serial_owned(serial, &args)
}

/// Set the device clipboard via `cmd clipboard set-text <text>`.
/// The text is single-quoted for the remote shell, escaping any single
/// quotes.
pub fn send_clipboard(serial: Option<&str>, text: &str) -> Result<String> {
    if text.is_empty() {
        bail!("clipboard text cannot be empty");
    }
    let quoted = quote_posix(text);
    run_adb_with_serial_owned(
        serial,
        &[
            "shell".into(),
            "cmd".into(),
            "clipboard".into(),
            "set-text".into(),
            quoted,
        ],
    )
}

/// Read the device clipboard via `cmd clipboard get-text`.
pub fn receive_clipboard(serial: Option<&str>) -> Result<String> {
    run_adb_with_serial(serial, &["shell", "cmd", "clipboard", "get-text"])
}

/// Open a URL on the device (Android `VIEW` intent).
pub fn open_url(serial: Option<&str>, url: &str) -> Result<String> {
    if url.trim().is_empty() {
        bail!("URL cannot be empty");
    }
    run_adb_with_serial(
        serial,
        &[
            "shell",
            "am",
            "start",
            "-a",
            "android.intent.action.VIEW",
            "-d",
            url,
        ],
    )
}

/// Start an Android app by package name.
pub fn start_app(serial: Option<&str>, package: &str, force_stop: bool) -> Result<String> {
    if package.trim().is_empty() {
        bail!("package cannot be empty");
    }
    let mut args = vec!["shell".to_string(), "am".to_string()];
    if force_stop {
        args.push("force-stop".into());
        args.push(package.into());
        return run_adb_with_serial_owned(serial, &args);
    }
    args.push("start".into());
    args.push(package.into());
    run_adb_with_serial_owned(serial, &args)
}

/// Send a keyevent (Home/Back/Volume/etc).
pub fn keyevent(serial: Option<&str>, key_code: &str) -> Result<String> {
    if key_code.trim().is_empty() {
        bail!("keyevent cannot be empty");
    }
    run_adb_with_serial(serial, &["shell", "input", "keyevent", key_code])
}

/// Tap a screen coordinate.
pub fn tap(serial: Option<&str>, x: u32, y: u32) -> Result<String> {
    run_adb_with_serial(
        serial,
        &["shell", "input", "tap", &x.to_string(), &y.to_string()],
    )
}

/// List installed Android apps by package name via `pm list packages`.
pub fn list_packages(serial: Option<&str>) -> Result<Vec<String>> {
    let raw = run_adb_with_serial(serial, &["shell", "pm", "list", "packages"])?;
    Ok(raw
        .lines()
        .filter_map(|l| l.strip_prefix("package:").map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
        .collect())
}

fn quote_posix(text: &str) -> String {
    if text.is_empty() {
        return "''".to_string();
    }
    let mut out = String::with_capacity(text.len() + 2);
    out.push('\'');
    for ch in text.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

fn run_adb_with_serial(serial: Option<&str>, args: &[&str]) -> Result<String> {
    let owned: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
    run_adb_with_serial_owned(serial, &owned)
}

pub fn run_adb_shell(serial: Option<&str>, args: &[&str]) -> Result<String> {
    let mut full = vec!["shell".to_string()];
    full.extend(args.iter().map(|s| s.to_string()));
    run_adb_with_serial_owned(serial, &full)
}

fn run_adb_with_serial_owned(serial: Option<&str>, args: &[String]) -> Result<String> {
    let adb = adb_path()?;
    let mut command = Command::new(adb);
    if let Some(serial) = serial {
        command.args(["-s", serial]);
    }
    command.args(args);
    let output = command
        .output()
        .with_context(|| format!("failed to execute `adb {}`", args.join(" ")))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.status.success() {
        Ok(if stdout.is_empty() { stderr } else { stdout })
    } else {
        bail!(
            "`adb {}` exited {}: {}{}",
            args.join(" "),
            output.status,
            stderr,
            if stderr.is_empty() {
                format!(" {}", stdout)
            } else {
                String::new()
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_posix_basic() {
        assert_eq!(quote_posix("hello"), "'hello'");
        assert_eq!(quote_posix("can't"), "'can'\\''t'");
        assert_eq!(quote_posix(""), "''");
    }
}
