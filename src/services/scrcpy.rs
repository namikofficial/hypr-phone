//! Scrcpy integration: profile resolution, argument construction,
//! and process launching.

use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};

use crate::config::{Config, ScrcpyProfile};
use crate::domain::session::ScrcpySession;
// use crate::services::adb; // not currently used in this module

pub const DEFAULT_PROFILE: &str = "default";
pub const LOW_LATENCY_PROFILE: &str = "low_latency";
pub const PRESENTATION_PROFILE: &str = "presentation";
pub const DESK_PROFILE: &str = "desk";
pub const APP_PROFILE: &str = "app";
pub const RECORD_PROFILE: &str = "record";

/// Outcome of resolving a mirror request against config + current state.
#[derive(Debug, Clone)]
pub struct ResolvedMirror {
    pub device_serial: String,
    pub device_id: Option<String>,
    pub profile_name: String,
    pub profile: ScrcpyProfile,
    pub window_title: String,
    pub placement: crate::services::hyprland::HyprlandPlacementConfig,
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MirrorRequest<'a> {
    pub device_serial: Option<&'a str>,
    pub profile: Option<&'a str>,
    /// Explicit `--new-display` size override (e.g. `"900x1600"`).
    pub display_size: Option<&'a str>,
    /// Append raw scrcpy args (escape hatch for power users).
    pub extra_args: Vec<String>,
    /// Launching for app mode (e.g. `hypr-phone app <pkg>`).
    pub app_package: Option<&'a str>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolveOptions {
    pub require_connected: bool,
}

pub fn resolve_mirror_config(
    config: &Config,
    request: MirrorRequest<'_>,
) -> Result<ResolvedMirror> {
    let profile_name = request
        .profile
        .unwrap_or(config.mirror.profile.as_str())
        .to_owned();

    let profile = config
        .mirror
        .profiles
        .get(&profile_name)
        .cloned()
        .ok_or_else(|| anyhow!("scrcpy profile '{profile_name}' is not defined"))?;

    let device_serial = request
        .device_serial
        .map(str::to_string)
        .or_else(|| config.resolve_target_serial(None))
        .or_else(|| config.mirror.device_serial.clone())
        .ok_or_else(|| {
            anyhow!(
                "No target device. Connect your phone or configure a device alias (see `hypr-phone device list`)."
            )
        })?;

    if !is_likely_serial(&device_serial) {
        bail!("device serial `{device_serial}` is not a valid adb identifier");
    }

    let window_title = build_window_title(
        &config.mirror.window_title_prefix,
        &profile_name,
        &device_serial,
        request.app_package,
    );

    Ok(ResolvedMirror {
        device_serial,
        device_id: None,
        profile_name,
        profile,
        window_title,
        placement: config.mirror.hyprland.clone(),
        extra_args: request.extra_args,
    })
}

fn is_likely_serial(s: &str) -> bool {
    !s.trim().is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '-' | '_' | '.' | '[' | ']'))
}

/// Build a sensible window title for Hyprland matching.
pub fn build_window_title(
    prefix: &str,
    profile_name: &str,
    serial: &str,
    app: Option<&str>,
) -> String {
    let normalized_prefix = if prefix.trim().is_empty() {
        "hypr-phone"
    } else {
        prefix.trim()
    };
    let sanitized_serial = serial.replace([':', ' '], "_");
    match app {
        Some(pkg) => format!("{normalized_prefix}:app:{pkg}:{sanitized_serial}"),
        None => format!("{normalized_prefix}:{sanitized_serial}:{profile_name}"),
    }
}

/// Build the scrcpy argument list for a resolved mirror.
pub fn build_scrcpy_args(resolved: &ResolvedMirror) -> Vec<String> {
    let mut args =
        resolved
            .profile
            .to_args(Some(&resolved.device_serial), &resolved.window_title, None);
    args.extend(resolved.extra_args.iter().cloned());
    args
}

/// Build the scrcpy argument list for an app-mode launch.
pub fn build_scrcpy_app_args(
    resolved: &ResolvedMirror,
    package: &str,
    force_stop: bool,
    display_size: Option<&str>,
) -> Vec<String> {
    let mut args = resolved.profile.to_args(
        Some(&resolved.device_serial),
        &resolved.window_title,
        display_size,
    );
    let mut start = String::from("--start-app=");
    if force_stop {
        start.push('+');
    }
    start.push_str(package);
    args.push(start);
    args.extend(resolved.extra_args.iter().cloned());
    args
}

/// Build the scrcpy argument list for a recording launch.
pub fn build_scrcpy_record_args(resolved: &ResolvedMirror, output: &str) -> Vec<String> {
    let mut args =
        resolved
            .profile
            .to_args(Some(&resolved.device_serial), &resolved.window_title, None);
    args.push(format!("--record={output}"));
    args.extend(resolved.extra_args.iter().cloned());
    args
}

pub fn scrcpy_path() -> Result<std::path::PathBuf> {
    which::which("scrcpy").map_err(|_| {
        anyhow!(
            "Missing dependency `scrcpy` in PATH. Install `scrcpy` to mirror your Android device."
        )
    })
}

/// Probe installed scrcpy for capability detection (used by doctor).
/// Results are cached for the lifetime of the process to avoid repeated subprocess calls.
pub fn detect_scrcpy_capabilities() -> ScrcpyCapabilities {
    static CACHED: std::sync::OnceLock<ScrcpyCapabilities> = std::sync::OnceLock::new();
    CACHED
        .get_or_init(|| {
            let mut caps = ScrcpyCapabilities::default();
            let Ok(help) = scrcpy_help() else {
                return caps;
            };
            caps.version = extract_version(&help);
            caps.virtual_display = help.contains("--new-display");
            caps.start_app = help.contains("--start-app=");
            caps.flex_display = help.contains("--flex-display");
            caps.recording = help.contains("--record=");
            caps.audio = help.contains("--audio-source=");
            caps.camera = help.contains("--video-source=camera") || help.contains("--camera-size=");
            caps.otg = help.contains("--otg");
            caps.hid = help.contains("--hid-keyboard") || help.contains("--hid-mouse");
            caps
        })
        .clone()
}

#[derive(Debug, Clone, Default)]
pub struct ScrcpyCapabilities {
    pub version: Option<String>,
    pub virtual_display: bool,
    pub start_app: bool,
    pub flex_display: bool,
    pub recording: bool,
    pub audio: bool,
    pub camera: bool,
    pub otg: bool,
    pub hid: bool,
}

impl ScrcpyCapabilities {
    pub fn as_toml_table(&self) -> String {
        let mut s = String::new();
        let push = |s: &mut String, k: &str, v: &str| {
            s.push_str(&format!("{k} = {v}\n"));
        };
        if let Some(v) = &self.version {
            push(&mut s, "version", &toml_quote(v));
        }
        push(&mut s, "virtual_display", &self.virtual_display.to_string());
        push(&mut s, "start_app", &self.start_app.to_string());
        push(&mut s, "flex_display", &self.flex_display.to_string());
        push(&mut s, "recording", &self.recording.to_string());
        push(&mut s, "audio", &self.audio.to_string());
        push(&mut s, "camera", &self.camera.to_string());
        push(&mut s, "otg", &self.otg.to_string());
        push(&mut s, "hid", &self.hid.to_string());
        s
    }
}

fn toml_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn extract_version(help: &str) -> Option<String> {
    // First non-empty line often starts with "scrcpy X.Y <URL>".
    for line in help.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("scrcpy ") {
            if let Some(version) = rest.split_whitespace().next() {
                return Some(version.to_string());
            }
        }
    }
    None
}

fn scrcpy_help() -> Result<String> {
    let bin = scrcpy_path()?;
    let output = Command::new(bin).arg("--help").output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Spawn the scrcpy process for a resolved mirror.
pub fn spawn_mirror(resolved: &ResolvedMirror) -> Result<std::process::Child> {
    let args = build_scrcpy_args(resolved);
    let bin = scrcpy_path()?;
    let mut cmd = Command::new(bin);
    cmd.args(&args);
    let child = cmd
        .spawn()
        .with_context(|| format!("failed to launch scrcpy with args: {args:?}"))?;
    Ok(child)
}

/// Construct a `ScrcpySession` description from a launched process.
pub fn describe_session(
    resolved: &ResolvedMirror,
    pid: Option<u32>,
    display_id: Option<u32>,
) -> ScrcpySession {
    ScrcpySession {
        id: format!("sess-{:x}", fast_hash(&resolved.window_title)),
        pid,
        serial: resolved.device_serial.clone(),
        profile: resolved.profile_name.clone(),
        window_title: resolved.window_title.clone(),
        display_id,
        started_at_unix_secs: ScrcpySession::now_secs(),
        recording: None,
    }
}

fn fast_hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn build_window_title_app_mode() {
        let t = build_window_title("hypr-phone", "app", "ABC:5555", Some("com.example.app"));
        assert_eq!(t, "hypr-phone:app:com.example.app:ABC_5555");
    }

    #[test]
    fn build_window_title_normal_mode() {
        let t = build_window_title("hypr-phone", "default", "192.168.1.5:5555", None);
        assert_eq!(t, "hypr-phone:192.168.1.5_5555:default");
    }

    #[test]
    fn app_args_include_start_app() {
        let config = Config::default();
        let resolved = resolve_mirror_config(
            &config,
            MirrorRequest {
                device_serial: Some("ABC"),
                profile: Some("app"),
                ..Default::default()
            },
        )
        .expect("resolve");
        let args = build_scrcpy_app_args(&resolved, "com.example.app", true, Some("720x1280"));
        assert!(args.iter().any(|a| a == "--start-app=+com.example.app"));
        assert!(args.windows(2).any(|p| p == ["--new-display", "720x1280"]));
    }

    #[test]
    fn record_args_include_record_flag() {
        let config = Config::default();
        let resolved = resolve_mirror_config(
            &config,
            MirrorRequest {
                device_serial: Some("ABC"),
                profile: Some("record"),
                ..Default::default()
            },
        )
        .expect("resolve");
        let args = build_scrcpy_record_args(&resolved, "/tmp/rec.mp4");
        assert!(args.iter().any(|a| a == "--record=/tmp/rec.mp4"));
    }

    #[test]
    fn failing_profile() {
        let config = Config::default();
        let result = resolve_mirror_config(
            &config,
            MirrorRequest {
                device_serial: Some("ABC"),
                profile: Some("missing"),
                ..Default::default()
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn no_target_device_fails() {
        let mut config = Config::default();
        config.mirror.device_serial = None;
        let result = resolve_mirror_config(&config, MirrorRequest::default());
        assert!(result.is_err());
    }

    #[test]
    fn to_args_honors_profile_settings() {
        let profile = ScrcpyProfile {
            audio: false,
            turn_screen_off: true,
            max_fps: Some(30),
            ..ScrcpyProfile::default()
        };
        let args = profile.to_args(Some("ABC"), "title", None);
        assert!(args.windows(2).any(|p| p == ["--serial", "ABC"]));
        assert!(args.contains(&"--no-audio".to_string()));
        assert!(args.contains(&"--turn-screen-off".to_string()));
        assert!(args.contains(&"--max-fps".to_string()));
    }
}
