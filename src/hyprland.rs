use std::{
    process::Command,
    thread,
    time::{Duration, Instant},
};

use anyhow::{anyhow, bail, Context};
use serde::{Deserialize, Serialize};

use crate::errors::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HyprlandPlacementConfig {
    pub enabled: bool,
    pub workspace: String,
    pub width: u16,
    pub height: u16,
    pub retry_timeout_ms: u64,
    pub retry_interval_ms: u64,
}

impl Default for HyprlandPlacementConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            workspace: "special:phone".to_owned(),
            width: 420,
            height: 900,
            retry_timeout_ms: 3000,
            retry_interval_ms: 200,
        }
    }
}

pub fn hyprctl_dispatch(args: &[&str]) -> Result<()> {
    let output = Command::new("hyprctl")
        .arg("dispatch")
        .args(args)
        .output()
        .with_context(|| format!("failed to execute hyprctl dispatch {:?}", args))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    bail!(
        "hyprctl dispatch {:?} failed with status {}: {}",
        args,
        output.status,
        stderr.trim()
    );
}

pub fn place_window(window_title: &str, config: &HyprlandPlacementConfig) -> Result<()> {
    if !config.enabled {
        return Ok(());
    }

    let retry_timeout = Duration::from_millis(config.retry_timeout_ms.max(1));
    let retry_interval = Duration::from_millis(config.retry_interval_ms.max(1));
    let deadline = Instant::now() + retry_timeout;
    let mut last_error: Option<anyhow::Error> = None;

    while Instant::now() <= deadline {
        match place_window_once(window_title, config) {
            Ok(()) => return Ok(()),
            Err(error) => {
                last_error = Some(error);
                thread::sleep(retry_interval);
            }
        }
    }

    Err(anyhow!(
        "failed to place scrcpy window in hyprland after {}ms: {}",
        config.retry_timeout_ms,
        last_error
            .map(|error| error.to_string())
            .unwrap_or_else(|| "unknown hyprland error".to_owned())
    ))
}

fn place_window_once(window_title: &str, config: &HyprlandPlacementConfig) -> Result<()> {
    let title_selector = format!("title:^{}$", escape_regex(window_title));
    hyprctl_dispatch(&["focuswindow", title_selector.as_str()])?;
    hyprctl_dispatch(&["movetoworkspace", config.workspace.as_str()])?;
    hyprctl_dispatch(&["togglefloating"])?;

    let width = config.width.to_string();
    let height = config.height.to_string();
    hyprctl_dispatch(&["resizeactive", "exact", width.as_str(), height.as_str()])?;
    hyprctl_dispatch(&["centerwindow"])?;
    Ok(())
}

fn escape_regex(raw: &str) -> String {
    let mut escaped = String::with_capacity(raw.len());
    for character in raw.chars() {
        if matches!(
            character,
            '\\' | '.' | '+' | '*' | '?' | '^' | '$' | '(' | ')' | '[' | ']' | '{' | '}' | '|'
        ) {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}
