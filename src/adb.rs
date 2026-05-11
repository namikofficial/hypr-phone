use std::{
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::Context;

use crate::errors::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdbDevice {
    pub serial: String,
    pub state: String,
    pub model: Option<String>,
}

impl AdbDevice {
    pub fn is_connected(&self) -> bool {
        self.state == "device"
    }

    pub fn display_name(&self) -> &str {
        self.model.as_deref().unwrap_or(&self.serial)
    }
}

pub fn adb_path() -> Result<PathBuf> {
    Ok(which::which("adb")?)
}

pub fn parse_devices_output(output: &str) -> Vec<AdbDevice> {
    output
        .lines()
        .skip(1)
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }

            let mut parts = trimmed.split_whitespace();
            let serial = parts.next()?.to_string();
            let state = parts.next().unwrap_or("unknown").to_string();
            let model = parts
                .find_map(|part| part.strip_prefix("model:"))
                .map(|model| model.replace('_', " "))
                .filter(|model| !model.is_empty());

            Some(AdbDevice {
                serial,
                state,
                model,
            })
        })
        .collect()
}

pub fn list_devices() -> Result<Vec<AdbDevice>> {
    list_devices_with_timeout(Duration::from_secs(2))
}

pub fn list_devices_with_timeout(timeout: Duration) -> Result<Vec<AdbDevice>> {
    let adb = adb_path()?;
    let mut child = Command::new(adb)
        .args(["devices", "-l"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("failed to start `adb devices -l`")?;

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
                return Ok(parse_devices_output(&String::from_utf8_lossy(&output.stdout)));
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
