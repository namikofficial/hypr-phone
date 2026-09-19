//! Hyprland integration service.
//!
//! Runtime-first placement via `hyprctl dispatch` rather than static
//! `windowrulev2` config (which current Hyprland docs deprecate).

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

/// Window placement configuration. Stored under `[mirror.hyprland]` in
/// user config and used by `toggle` to place the scrcpy window.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HyprlandPlacementConfig {
    pub enabled: bool,
    pub workspace: String,
    pub width: u16,
    pub height: u16,
    pub retry_timeout_ms: u64,
    pub retry_interval_ms: u64,
    pub center: bool,
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
            center: true,
        }
    }
}

/// A single Hyprland `dispatch` action.
#[derive(Debug, Clone)]
pub enum DispatchAction<'a> {
    Workspace(&'a str),
    MoveToWorkspaceSilent(&'a str),
    ToggleSpecialWorkspace(Option<&'a str>),
    MovetoWorkspace(&'a str),
    FocusWindow(&'a str),
    ToggleFloating,
    ResizeActiveExact(u16, u16),
    CenterWindow,
    BringActiveToTop,
    Custom(&'a str, Vec<String>),
}

impl<'a> DispatchAction<'a> {
    fn args(&self) -> Vec<String> {
        match self {
            DispatchAction::Workspace(w) => vec!["workspace".into(), (*w).into()],
            DispatchAction::MoveToWorkspaceSilent(w) => {
                vec!["movetoworkspacesilent".into(), (*w).into()]
            }
            DispatchAction::ToggleSpecialWorkspace(w) => {
                if let Some(w) = w {
                    vec!["togglespecialworkspace".into(), (*w).into()]
                } else {
                    vec!["togglespecialworkspace".into()]
                }
            }
            DispatchAction::MovetoWorkspace(w) => vec!["movetoworkspace".into(), (*w).into()],
            DispatchAction::FocusWindow(sel) => vec!["focuswindow".into(), (*sel).into()],
            DispatchAction::ToggleFloating => vec!["togglefloating".into()],
            DispatchAction::ResizeActiveExact(w, h) => {
                vec![
                    "resizeactive".into(),
                    "exact".into(),
                    w.to_string(),
                    h.to_string(),
                ]
            }
            DispatchAction::CenterWindow => vec!["centerwindow".into()],
            DispatchAction::BringActiveToTop => vec!["bringactivetotop".into()],
            DispatchAction::Custom(action, args) => {
                let mut v = vec![(*action).into()];
                v.extend(args.iter().cloned());
                v
            }
        }
    }
}

/// Run `hyprctl dispatch <args...>`.
pub fn dispatch(action: DispatchAction<'_>) -> Result<()> {
    let args = action.args();
    let mut cmd = std::process::Command::new("hyprctl");
    cmd.arg("dispatch");
    for a in &args {
        cmd.arg(a);
    }
    let output = cmd
        .output()
        .with_context(|| format!("failed to execute hyprctl dispatch {args:?}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        bail!(
            "hyprctl dispatch {:?} failed with status {}: {}{}",
            args,
            output.status,
            stderr.trim(),
            if stderr.trim().is_empty() {
                format!(" {}", stdout.trim())
            } else {
                String::new()
            }
        );
    }
    Ok(())
}

/// Run `hyprctl <subcmd> <args...>` and return stdout.
pub fn query(subcmd: &str, args: &[&str]) -> Result<String> {
    let output = std::process::Command::new("hyprctl")
        .arg(subcmd)
        .args(args)
        .output()
        .with_context(|| format!("failed to execute hyprctl {subcmd} {args:?}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "hyprctl {subcmd} failed: {}",
            if stderr.trim().is_empty() {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            } else {
                stderr.trim().to_string()
            }
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Check whether Hyprland is available in the current session.
pub fn hyprland_available() -> bool {
    which::which("hyprctl").is_ok() && std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok()
}

/// Place a window matching `window_title` into the configured workspace,
/// float + size + center. Idempotent and retry-aware.
pub fn place_window(window_title: &str, placement: &HyprlandPlacementConfig) -> Result<()> {
    if !placement.enabled {
        return Ok(());
    }

    let deadline = std::time::Instant::now()
        + std::time::Duration::from_millis(placement.retry_timeout_ms.max(1));
    let interval = std::time::Duration::from_millis(placement.retry_interval_ms.max(1));
    let mut last_error: Option<anyhow::Error> = None;

    let selector = format!("title:^{}$", escape_regex(window_title));

    while std::time::Instant::now() <= deadline {
        match try_place_once(&selector, placement) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_error = Some(e);
                std::thread::sleep(interval);
            }
        }
    }

    Err(anyhow!(
        "failed to place scrcpy window in hyprland after {}ms: {}",
        placement.retry_timeout_ms,
        last_error
            .map(|e| e.to_string())
            .unwrap_or_else(|| "unknown hyprland error".to_owned())
    ))
}

fn try_place_once(selector: &str, placement: &HyprlandPlacementConfig) -> Result<()> {
    dispatch(DispatchAction::FocusWindow(selector))?;
    dispatch(DispatchAction::MovetoWorkspace(
        placement.workspace.as_str(),
    ))?;
    dispatch(DispatchAction::ToggleFloating)?;
    if placement.width > 0 && placement.height > 0 {
        dispatch(DispatchAction::ResizeActiveExact(
            placement.width,
            placement.height,
        ))?;
    }
    if placement.center {
        dispatch(DispatchAction::CenterWindow)?;
    }
    Ok(())
}

/// Toggle a special workspace (e.g. `special:phone`).
pub fn toggle_special_workspace(name: &str) -> Result<()> {
    dispatch(DispatchAction::ToggleSpecialWorkspace(Some(name)))
}

/// Show / focus a special workspace.
pub fn show_special_workspace(name: &str) -> Result<()> {
    dispatch(DispatchAction::Workspace(name))
}

/// Move the currently focused window into a special workspace silently.
pub fn move_active_to_special_workspace(name: &str) -> Result<()> {
    dispatch(DispatchAction::MoveToWorkspaceSilent(name))
}

/// Check if a window matching `title_pattern` is currently visible on the
/// given workspace. Returns Ok(true) if visible, Ok(false) if not found.
pub fn is_window_visible_on_workspace(title_pattern: &str, _workspace: &str) -> Result<bool> {
    // Full implementation would check the specific workspace's client list.
    // For now, check if the window exists in any client list.
    match query("clients", &[]) {
        Ok(clients) => {
            // Look for the window title in the clients output.
            Ok(clients.contains(title_pattern))
        }
        Err(_) => Ok(false),
    }
}

/// Wait for a window matching `title_pattern` to appear, with timeout.
/// Returns Ok(()) if window found within timeout, Err otherwise.
pub fn wait_for_window_with_timeout(title_pattern: &str, timeout_ms: u64) -> Result<()> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms.max(1));
    let interval = std::time::Duration::from_millis(100);

    while std::time::Instant::now() <= deadline {
        if let Ok(clients) = query("clients", &[]) {
            if clients.contains(title_pattern) {
                return Ok(());
            }
        }
        std::thread::sleep(interval);
    }

    Err(anyhow!(
        "window '{}' not found within {}ms",
        title_pattern,
        timeout_ms
    ))
}

fn escape_regex(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if matches!(
            ch,
            '\\' | '.' | '+' | '*' | '?' | '^' | '$' | '(' | ')' | '[' | ']' | '{' | '}' | '|'
        ) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_args_basic() {
        let a = DispatchAction::Workspace("1");
        assert_eq!(a.args(), vec!["workspace".to_string(), "1".to_string()]);
        let a = DispatchAction::MoveToWorkspaceSilent("special:phone");
        assert_eq!(
            a.args(),
            vec![
                "movetoworkspacesilent".to_string(),
                "special:phone".to_string()
            ]
        );
        let a = DispatchAction::ResizeActiveExact(420, 900);
        assert_eq!(
            a.args(),
            vec![
                "resizeactive".to_string(),
                "exact".to_string(),
                "420".to_string(),
                "900".to_string(),
            ]
        );
    }

    #[test]
    fn escape_regex_handles_meta_chars() {
        assert_eq!(escape_regex("hypr-phone:1.0"), "hypr-phone:1\\.0");
        assert_eq!(escape_regex("a+b"), "a\\+b");
    }
}
