//! ADB discovery — listing devices, parsing output, mDNS hooks.

use std::{
    net::{IpAddr, Ipv4Addr},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::domain::device::{
    parse_endpoint, AdbState, DeviceCapabilities, PhoneDevice, Transport,
};

/// Wrapper that runs `adb` and captures results. Implemented for
/// `RealAdb` (subprocess) and used in tests via mocking at the PATH level.
pub trait AdbRunner {
    fn run(&self, args: &[&str]) -> Result<String>;
}

pub struct RealAdb;

impl AdbRunner for RealAdb {
    fn run(&self, args: &[&str]) -> Result<String> {
        let adb = which::which("adb").map_err(|_| {
            anyhow!(
                "Missing dependency `adb` in PATH. Install Android platform-tools and ensure `adb` is on PATH."
            )
        })?;
        let output = Command::new(adb)
            .args(args)
            .output()
            .with_context(|| format!("failed to execute `adb {}`", args.join(" ")))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if !output.status.success() {
            bail!(
                "`adb {}` exited with status {}: {}{}",
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
        if stdout.trim().is_empty() && stderr.trim().is_empty() {
            Ok(String::new())
        } else if stdout.trim().is_empty() {
            Ok(stderr)
        } else {
            Ok(stdout)
        }
    }
}

/// Parse `adb devices -l` output into `PhoneDevice`s.
pub fn parse_devices_l_output(output: &str) -> Vec<PhoneDevice> {
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
                        .map(|v| v.replace('_', " "))
                        .filter(|v| !v.is_empty());
                }
                if product.is_none() {
                    product = part
                        .strip_prefix("product:")
                        .map(|v| v.replace('_', " "))
                        .filter(|v| !v.is_empty());
                }
            }
            Some(PhoneDevice::from_adb_listing(&serial, &state, model, product))
        })
        .collect()
}

#[derive(Debug, Clone, Default)]
pub struct DiscoveryOptions {
    pub timeout: Duration,
    pub include_mdns: bool,
}

pub struct AdbDiscovery {
    runner: Box<dyn AdbRunner + Send + Sync>,
}

impl AdbDiscovery {
    pub fn new() -> Self {
        Self {
            runner: Box::new(RealAdb),
        }
    }

    pub fn with_runner(runner: Box<dyn AdbRunner + Send + Sync>) -> Self {
        Self { runner }
    }

    /// Run `adb devices -l` and return parsed devices.
    pub fn devices_l(&self) -> Result<Vec<PhoneDevice>> {
        let raw = self.runner.run(&["devices", "-l"])?;
        Ok(parse_devices_l_output(&raw))
    }

    /// List devices with a hard timeout; returns empty on timeout.
    pub fn devices_with_timeout(&self, timeout: Duration) -> Result<Vec<PhoneDevice>> {
        let adb = which::which("adb").map_err(|_| anyhow!("adb not in PATH"))?;
        let mut child = Command::new(adb)
            .args(["devices", "-l"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| "failed to spawn `adb devices -l`")?;
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
                        bail!(
                            "`adb devices -l` exited {}: {}",
                            output.status,
                            stderr.trim()
                        );
                    }
                    return Ok(parse_devices_l_output(&stdout));
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

    /// Best-effort mDNS discovery: `adb mdns services` (Android 11+/ADB
    /// Wi-Fi 2.0).
    pub fn mdns_discover(&self, timeout: Duration) -> Result<Vec<PhoneDevice>> {
        let raw = self.runner.run(&["mdns", "services"])?;
        let mut devices = Vec::new();
        for line in raw.lines().skip(1) {
            // Lines look like:  _adb._tcp  192.168.1.5:5555  pixel-8
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 2 {
                continue;
            }
            let endpoint = cols[1];
            let model_hint = cols.get(2).map(|s| s.replace('_', " "));
            if let Some(addr) = parse_endpoint(endpoint) {
                let transport = Transport::Wifi {
                    ip: addr.ip(),
                    port: addr.port(),
                };
                let model = model_hint.clone();
                let device = PhoneDevice::from_adb_listing(
                    &addr.to_string(),
                    "unknown",
                    model,
                    None,
                );
                let mut device = device;
                device.transport = transport;
                device.capabilities.mdns = true;
                device.capabilities.wireless_adb = true;
                devices.push(device);
            }
        }
        // Cap timeout to callers via a sentinel; current `runner.run` does
        // not have one, so we just return the result. The `timeout` param
        // exists for API symmetry with `devices_with_timeout`.
        let _ = timeout;
        Ok(devices)
    }

    /// Combined discovery: `adb devices -l` ∪ mDNS, deduped by identity.
    pub fn discover(&self, opts: DiscoveryOptions) -> Result<Vec<PhoneDevice>> {
        let mut combined = self.devices_with_timeout(opts.timeout)?;
        if opts.include_mdns {
            if let Ok(mdns) = self.mdns_discover(opts.timeout) {
                combined.extend(mdns);
            }
        }
        Ok(crate::domain::device::dedupe_by_identity(combined))
    }
}

impl Default for AdbDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// Pair over wireless ADB.
pub fn pair(endpoint: &str, pairing_code: &str) -> Result<String> {
    validate_ip_port(endpoint)?;
    if pairing_code.trim().is_empty() {
        bail!("Pairing code cannot be empty.");
    }
    RealAdb.run(&["pair", endpoint, pairing_code])
}

/// Connect to a wireless endpoint.
pub fn connect(endpoint: &str) -> Result<String> {
    validate_ip_port(endpoint)?;
    RealAdb.run(&["connect", endpoint])
}

/// Disconnect by serial or endpoint.
pub fn disconnect(serial: &str) -> Result<String> {
    if serial.trim().is_empty() {
        bail!("Device serial cannot be empty.");
    }
    RealAdb.run(&["disconnect", serial])
}

/// Validate `ip:port` format.
pub fn validate_ip_port(endpoint: &str) -> Result<()> {
    let (ip, port) = endpoint.split_once(':').ok_or_else(|| {
        anyhow!("Invalid endpoint `{endpoint}`. Expected format `<ip:port>`.")
    })?;
    let _ip: IpAddr = ip
        .parse()
        .with_context(|| format!("Invalid IP address `{ip}` in endpoint `{endpoint}`."))?;
    let port: u16 = port
        .parse()
        .with_context(|| format!("Invalid port `{port}` in endpoint `{endpoint}`."))?;
    if port == 0 {
        bail!("Port in endpoint `{endpoint}` must be between 1 and 65535.");
    }
    Ok(())
}

/// Normalize a partial endpoint (no port → add default).
pub fn normalize_endpoint(raw: &str, default_port: u16) -> String {
    let trimmed = raw.trim();
    if trimmed.contains(':') {
        trimmed.to_string()
    } else {
        format!("{trimmed}:{default_port}")
    }
}

/// Best-effort attempt to detect a remote device's identity (model, serial).
pub fn detect_remote_identity(serial: Option<&str>) -> Result<PhoneDevice> {
    let args = if let Some(s) = serial {
        vec!["-s", s, "shell", "getprop", "ro.product.model"]
    } else {
        vec!["shell", "getprop", "ro.product.model"]
    };
    let model = RealAdb
        .run(&args)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let serial_args = if let Some(s) = serial {
        vec!["-s", s, "get-serialno"]
    } else {
        vec!["get-serialno"]
    };
    let android_serial = RealAdb
        .run(&serial_args)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let serial_str = serial.unwrap_or("device").to_string();
    let state = if serial.is_some() { "device" } else { "unknown" };
    let mut device = PhoneDevice::from_adb_listing(&serial_str, state, model.clone(), None);
    if let Some(model) = model {
        device.model = Some(model);
    }
    device.android_serial = android_serial;
    Ok(device)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_devices_l_output() {
        let raw = "List of devices attached\n\
                   192.168.1.10:5555 device product:oriole model:Pixel_6\n\
                   ABCDEF unauthorized product:foo model:Bar\n";
        let devices = parse_devices_l_output(raw);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].adb_serial.as_deref(), Some("192.168.1.10:5555"));
        assert_eq!(devices[0].model.as_deref(), Some("Pixel 6"));
        assert_eq!(devices[0].adb_state, AdbState::Connected);
        assert_eq!(devices[1].adb_state, AdbState::Unauthorized);
    }

    #[test]
    fn parses_usb_serial() {
        let raw = "List of devices attached\nusb:1-1.4 device product:panther model:Pixel_7\n";
        let devices = parse_devices_l_output(raw);
        assert_eq!(devices[0].transport.kind(), crate::domain::device::TransportKind::Usb);
    }

    #[test]
    fn normalizes_endpoint() {
        assert_eq!(normalize_endpoint("192.168.1.20", 5555), "192.168.1.20:5555");
        assert_eq!(normalize_endpoint("192.168.1.20:1234", 5555), "192.168.1.20:1234");
    }

    #[test]
    fn validates_ip_port() {
        assert!(validate_ip_port("192.168.0.2:5555").is_ok());
        assert!(validate_ip_port("bad").is_err());
        assert!(validate_ip_port("192.168.0.2:0").is_err());
    }
}
