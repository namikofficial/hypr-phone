//! Host clipboard via wl-copy / wl-paste.

use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};

pub fn read() -> Result<String> {
    let output = Command::new("wl-paste")
        .args(["--no-newline"])
        .output()
        .map_err(|_| {
            anyhow::anyhow!("`wl-paste` is not available. Install `wl-clipboard`.")
        })?;
    if !output.status.success() {
        bail!(
            "`wl-paste` exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn write(text: &str) -> Result<()> {
    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|_| anyhow::anyhow!("`wl-copy` is not available. Install `wl-clipboard`."))?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(text.as_bytes())
            .context("failed writing to wl-copy stdin")?;
    }
    let status = child.wait().context("failed waiting for wl-copy")?;
    if !status.success() {
        bail!("`wl-copy` exited {status}");
    }
    Ok(())
}

pub fn wl_copy_available() -> bool {
    which::which("wl-copy").is_ok() && which::which("wl-paste").is_ok()
}
