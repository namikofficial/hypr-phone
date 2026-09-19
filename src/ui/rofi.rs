//! rofi / wofi shared launcher helpers.

use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Rofi,
    Wofi,
}

impl Backend {
    pub fn binary(self) -> &'static str {
        match self {
            Backend::Rofi => "rofi",
            Backend::Wofi => "wofi",
        }
    }
}

pub fn detect_backend() -> Option<Backend> {
    if which::which("rofi").is_ok() {
        return Some(Backend::Rofi);
    }
    if which::which("wofi").is_ok() {
        return Some(Backend::Wofi);
    }
    None
}

/// Show a menu with `prompt` and return the selected label.
pub fn show_menu(
    backend: Backend,
    prompt: &str,
    entries: &[(String, String)],
) -> Result<Option<String>> {
    let mut cmd = Command::new(backend.binary());
    match backend {
        Backend::Rofi => {
            cmd.args(["-dmenu", "-i", "-p", prompt]);
        }
        Backend::Wofi => {
            cmd.args(["--dmenu", "-i", "-p", prompt]);
        }
    }
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = cmd
        .spawn()
        .with_context(|| format!("failed to launch {}", backend.binary()))?;

    if let Some(mut stdin) = child.stdin.take() {
        for (_, label) in entries {
            stdin.write_all(label.as_bytes())?;
            stdin.write_all(b"\n")?;
        }
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Ok(None);
    }
    let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if selected.is_empty() {
        Ok(None)
    } else {
        Ok(Some(selected))
    }
}

/// Show a prompt dialog (text input) and return the typed value.
pub fn prompt(backend: Backend, message: &str) -> Result<Option<String>> {
    let mut cmd = Command::new(backend.binary());
    match backend {
        Backend::Rofi => {
            cmd.args(["-dmenu", "-i", "-p", message, "-mesg", ""]);
        }
        Backend::Wofi => {
            cmd.args(["--dmenu", "-i", "-p", message]);
        }
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let output = cmd.output()?;
    if !output.status.success() {
        return Ok(None);
    }
    let typed = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if typed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(typed))
    }
}
