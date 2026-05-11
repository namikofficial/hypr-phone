use std::{
    net::IpAddr,
    path::PathBuf,
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
    list_devices()
}

pub fn list_devices() -> Result<Vec<AdbDevice>> {
    list_devices_with_timeout(Duration::from_secs(2))
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

fn run_adb_command(args: &[&str]) -> Result<String> {
    let adb = adb_path()?;
    let output = Command::new(&adb)
        .args(args)
        .output()
        .map_err(|err| command_spawn_error("adb", args, err))?;

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
            "adb",
            args,
            output.status,
            &stdout,
            &stderr,
        ))
    }
}
