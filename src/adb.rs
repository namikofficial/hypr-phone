use std::{
    net::IpAddr,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{anyhow, bail, Context};
use serde::{Deserialize, Serialize};

use crate::errors::{command_failed_error, command_spawn_error, missing_dependency, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdbDevice {
    pub serial: String,
    pub state: String,
    pub model: Option<String>,
    pub product: Option<String>,
}

impl AdbDevice {
    pub fn is_connected(&self) -> bool {
        self.state == "device"
    }

    pub fn display_name(&self) -> &str {
        self.model
            .as_deref()
            .or(self.product.as_deref())
            .unwrap_or(&self.serial)
    }
}

pub fn adb_path() -> Result<PathBuf> {
    which::which("adb").map_err(|_| {
        missing_dependency(
            "adb",
            "Install Android platform-tools and ensure `adb` is on PATH.",
        )
    })
}

pub fn parse_devices_output(output: &str) -> Vec<AdbDevice> {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty()
                || trimmed.starts_with("List of devices attached")
                || trimmed.starts_with('*')
            {
                return None;
            }

            let mut parts = trimmed.split_whitespace();
            let serial = parts.next()?.to_string();
            let state = parts.next().unwrap_or("unknown").to_string();

            let mut model = None;
            let mut product = None;
            for part in parts {
                if model.is_none() {
                    model = part
                        .strip_prefix("model:")
                        .map(|value| value.replace('_', " "))
                        .filter(|value| !value.is_empty());
                }
                if product.is_none() {
                    product = part
                        .strip_prefix("product:")
                        .map(|value| value.replace('_', " "))
                        .filter(|value| !value.is_empty());
                }
            }

            Some(AdbDevice {
                serial,
                state,
                model,
                product,
            })
        })
        .collect()
}

pub fn devices() -> Result<Vec<AdbDevice>> {
    list_devices_blocking()
}

pub fn list_devices_blocking() -> Result<Vec<AdbDevice>> {
    let adb = adb_path()?;
    let args = ["devices", "-l"];
    let output = Command::new(&adb)
        .args(args)
        .output()
        .map_err(|err| command_spawn_error("adb", &args, err))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        return Err(command_failed_error(
            "adb",
            &args,
            output.status,
            &stdout,
            &stderr,
        ));
    }

    Ok(parse_devices_output(&stdout))
}

pub fn list_devices_with_timeout(timeout: Duration) -> Result<Vec<AdbDevice>> {
    let adb = adb_path()?;
    let mut child = Command::new(adb)
        .args(["devices", "-l"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| command_spawn_error("adb", &["devices", "-l"], err))?;

    let deadline = Instant::now() + timeout;
    loop {
        match child
            .try_wait()
            .context("failed to poll `adb devices -l`")?
        {
            Some(_) => {
                let output = child
                    .wait_with_output()
                    .context("failed to collect `adb devices -l` output")?;
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                if !output.status.success() {
                    return Err(command_failed_error(
                        "adb",
                        &["devices", "-l"],
                        output.status,
                        &stdout,
                        &stderr,
                    ));
                }
                return Ok(parse_devices_output(&stdout));
            }
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(Vec::new());
            }
            None => thread::sleep(Duration::from_millis(20)),
        }
    }
}

pub fn pair(endpoint: &str, pairing_code: &str) -> Result<String> {
    validate_ip_port(endpoint)?;
    if pairing_code.trim().is_empty() {
        bail!("Pairing code cannot be empty.");
    }
    run_adb_command(&["pair", endpoint, pairing_code])
}

pub fn connect(endpoint: &str) -> Result<String> {
    validate_ip_port(endpoint)?;
    run_adb_command(&["connect", endpoint])
}

pub fn disconnect(serial: &str) -> Result<String> {
    if serial.trim().is_empty() {
        bail!("Device serial cannot be empty.");
    }
    run_adb_command(&["disconnect", serial])
}

pub fn screenshot(serial: Option<&str>, output_path: &Path) -> Result<String> {
    let adb = adb_path()?;
    let mut command = Command::new(adb);
    if let Some(serial) = serial {
        command.args(["-s", serial]);
    }
    let args = ["exec-out", "screencap", "-p"];
    command.args(args);

    let output = command
        .output()
        .map_err(|err| command_spawn_error("adb", &args, err))?;

    if !output.status.success() {
        return Err(command_failed_error(
            "adb",
            &args,
            output.status,
            &String::from_utf8_lossy(&output.stdout),
            &String::from_utf8_lossy(&output.stderr),
        ));
    }

    std::fs::write(output_path, &output.stdout)
        .with_context(|| format!("failed to write screenshot to {}", output_path.display()))?;

    Ok(format!("Saved screenshot to {}", output_path.display()))
}

pub fn push_file(serial: Option<&str>, local_path: &str, remote_path: &str) -> Result<String> {
    if local_path.trim().is_empty() || remote_path.trim().is_empty() {
        bail!("local_path and remote_path must not be empty");
    }
    run_adb_command_with_optional_serial(serial, &["push", local_path, remote_path])
}

pub fn pull_file(serial: Option<&str>, remote_path: &str, local_path: &str) -> Result<String> {
    if local_path.trim().is_empty() || remote_path.trim().is_empty() {
        bail!("remote_path and local_path must not be empty");
    }
    run_adb_command_with_optional_serial(serial, &["pull", remote_path, local_path])
}

pub fn install_apk(serial: Option<&str>, apk_path: &str, reinstall: bool) -> Result<String> {
    if apk_path.trim().is_empty() {
        bail!("apk_path must not be empty");
    }

    let mut args = vec!["install"];
    if reinstall {
        args.push("-r");
    }
    args.push(apk_path);
    run_adb_command_with_optional_serial(serial, &args)
}

pub fn shell(serial: Option<&str>, command: &[String]) -> Result<String> {
    if command.is_empty() {
        bail!("shell command cannot be empty");
    }

    let mut args = vec!["shell".to_string()];
    args.extend(command.iter().cloned());
    run_adb_command_with_optional_serial_owned(serial, &args)
}

pub fn send_clipboard(serial: Option<&str>, text: &str) -> Result<String> {
    if text.trim().is_empty() {
        bail!("clipboard text cannot be empty");
    }

    run_adb_command_with_optional_serial_owned(
        serial,
        &[
            "shell".to_string(),
            "cmd".to_string(),
            "clipboard".to_string(),
            "set".to_string(),
            "text".to_string(),
            text.to_string(),
        ],
    )
}

pub fn receive_clipboard(serial: Option<&str>) -> Result<String> {
    run_adb_command_with_optional_serial(serial, &["shell", "cmd", "clipboard", "get", "text"])
}

pub fn validate_ip_port(endpoint: &str) -> Result<()> {
    let (ip, port) = endpoint
        .split_once(':')
        .ok_or_else(|| anyhow!("Invalid endpoint `{endpoint}`. Expected format `<ip:port>`."))?;

    ip.parse::<IpAddr>()
        .with_context(|| format!("Invalid IP address `{ip}` in endpoint `{endpoint}`."))?;

    let port = port
        .parse::<u16>()
        .with_context(|| format!("Invalid port `{port}` in endpoint `{endpoint}`."))?;
    if port == 0 {
        bail!("Port in endpoint `{endpoint}` must be between 1 and 65535.");
    }

    Ok(())
}

pub fn normalize_endpoint(raw: &str, default_port: u16) -> String {
    let trimmed = raw.trim();
    if trimmed.contains(':') {
        trimmed.to_string()
    } else {
        format!("{trimmed}:{default_port}")
    }
}

pub fn pick_connected_serial(preferred: Option<String>, devices: &[AdbDevice]) -> Option<String> {
    preferred.or_else(|| {
        devices
            .iter()
            .find(|device| device.is_connected())
            .map(|device| device.serial.clone())
    })
}

fn run_adb_command_with_optional_serial(serial: Option<&str>, args: &[&str]) -> Result<String> {
    let owned = args
        .iter()
        .map(|arg| (*arg).to_string())
        .collect::<Vec<_>>();
    run_adb_command_with_optional_serial_owned(serial, &owned)
}

fn run_adb_command_with_optional_serial_owned(
    serial: Option<&str>,
    args: &[String],
) -> Result<String> {
    let adb = adb_path()?;
    let mut command = Command::new(&adb);
    let mut label_args = Vec::new();
    if let Some(serial) = serial {
        command.args(["-s", serial]);
        label_args.push("-s".to_string());
        label_args.push(serial.to_string());
    }
    command.args(args);
    label_args.extend(args.iter().cloned());

    let output = command
        .output()
        .map_err(|err| command_spawn_error("adb", &[], err))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        if stdout.is_empty() {
            Ok(stderr)
        } else {
            Ok(stdout)
        }
    } else {
        let borrowed = label_args.iter().map(String::as_str).collect::<Vec<_>>();
        Err(command_failed_error(
            "adb",
            &borrowed,
            output.status,
            &stdout,
            &stderr,
        ))
    }
}

fn run_adb_command(args: &[&str]) -> Result<String> {
    run_adb_command_with_optional_serial(None, args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_adb_devices_output() {
        let output = "List of devices attached\n192.168.1.10:5555 device product:oriole model:Pixel_6\nABCDEF unauthorized product:foo model:Bar\n\n";
        let devices = parse_devices_output(output);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].serial, "192.168.1.10:5555");
        assert_eq!(devices[0].model.as_deref(), Some("Pixel 6"));
        assert_eq!(devices[1].state, "unauthorized");
    }

    #[test]
    fn normalizes_endpoint_with_default_port() {
        assert_eq!(
            normalize_endpoint("192.168.1.20", 5555),
            "192.168.1.20:5555"
        );
        assert_eq!(
            normalize_endpoint("192.168.1.20:1234", 5555),
            "192.168.1.20:1234"
        );
    }

    #[test]
    fn validates_ip_port_endpoints() {
        assert!(validate_ip_port("192.168.0.2:5555").is_ok());
        assert!(validate_ip_port("bad-endpoint").is_err());
        assert!(validate_ip_port("192.168.0.2:0").is_err());
    }

    #[test]
    fn picks_preferred_or_first_connected_serial() {
        let devices = vec![
            AdbDevice {
                serial: "x".to_string(),
                state: "offline".to_string(),
                model: None,
                product: None,
            },
            AdbDevice {
                serial: "y".to_string(),
                state: "device".to_string(),
                model: None,
                product: None,
            },
        ];

        assert_eq!(
            pick_connected_serial(Some("manual".to_string()), &devices).as_deref(),
            Some("manual")
        );
        assert_eq!(pick_connected_serial(None, &devices).as_deref(), Some("y"));
    }
}
