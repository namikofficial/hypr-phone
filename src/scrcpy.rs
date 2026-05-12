use std::{
    path::{Path, PathBuf},
    process::{Child, Command},
    time::Duration,
};

use anyhow::{anyhow, bail, Context};
use serde::{Deserialize, Serialize};

use crate::adb;
use crate::config::Config;
use crate::errors::{missing_dependency, Result};
use crate::hyprland::{self, HyprlandPlacementConfig};

pub const DEFAULT_PROFILE_NAME: &str = "default";
pub const LOW_LATENCY_PROFILE_NAME: &str = "low_latency";
pub const PRESENTATION_PROFILE_NAME: &str = "presentation";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScrcpyProfile {
    pub video_bit_rate: Option<String>,
    pub max_size: Option<u32>,
    pub max_fps: Option<u32>,
    pub audio: bool,
    pub stay_awake: bool,
    pub turn_screen_off: bool,
    pub fullscreen: bool,
    pub window_borderless: bool,
    pub always_on_top: bool,
    pub args: Vec<String>,
}

impl Default for ScrcpyProfile {
    fn default() -> Self {
        Self {
            video_bit_rate: None,
            max_size: None,
            max_fps: None,
            audio: false,
            stay_awake: true,
            turn_screen_off: false,
            fullscreen: false,
            window_borderless: false,
            always_on_top: false,
            args: Vec::new(),
        }
    }
}

impl ScrcpyProfile {
    pub fn to_args(&self, serial: Option<&str>, window_title: &str) -> Vec<String> {
        let mut args = Vec::new();

        if let Some(serial) = serial {
            args.push("--serial".to_owned());
            args.push(serial.to_owned());
        }

        args.push("--window-title".to_owned());
        args.push(window_title.to_owned());

        if let Some(video_bit_rate) = &self.video_bit_rate {
            args.push("--video-bit-rate".to_owned());
            args.push(video_bit_rate.clone());
        }

        if let Some(max_size) = self.max_size {
            args.push("--max-size".to_owned());
            args.push(max_size.to_string());
        }

        if let Some(max_fps) = self.max_fps {
            args.push("--max-fps".to_owned());
            args.push(max_fps.to_string());
        }

        if !self.audio {
            args.push("--no-audio".to_owned());
        }

        if self.stay_awake {
            args.push("--stay-awake".to_owned());
        }

        if self.turn_screen_off {
            args.push("--turn-screen-off".to_owned());
        }

        if self.fullscreen {
            args.push("--fullscreen".to_owned());
        }

        if self.window_borderless {
            args.push("--window-borderless".to_owned());
        }

        if self.always_on_top {
            args.push("--always-on-top".to_owned());
        }

        args.extend(self.args.clone());
        args
    }
}

#[derive(Debug, Clone, Default)]
pub struct MirrorRequest<'a> {
    pub device_serial: Option<&'a str>,
    pub profile: Option<&'a str>,
}

#[derive(Debug, Clone)]
pub struct ResolvedMirrorConfig {
    pub device_serial: Option<String>,
    pub profile_name: String,
    pub profile: ScrcpyProfile,
    pub window_title: String,
    pub hyprland: HyprlandPlacementConfig,
}

#[derive(Debug)]
pub struct MirrorLaunch {
    pub child: Child,
    pub resolved: ResolvedMirrorConfig,
    pub hyprland_error: Option<String>,
}

pub fn resolve_mirror_config(
    config: &Config,
    request: MirrorRequest<'_>,
) -> Result<ResolvedMirrorConfig> {
    let profile_name = request
        .profile
        .unwrap_or(config.mirror.profile.as_str())
        .to_owned();

    let profile = config
        .mirror
        .profiles
        .get(&profile_name)
        .cloned()
        .ok_or_else(|| anyhow!("scrcpy profile '{profile_name}' is not defined in config"))?;

    let devices = adb::list_devices_with_timeout(Duration::from_millis(750))?;

    let mut device_serial = request
        .device_serial
        .map(ToOwned::to_owned)
        .or_else(|| config.mirror.device_serial.clone())
        .or_else(|| config.device_serial.clone());

    if device_serial.is_none() {
        device_serial = devices
            .iter()
            .find(|device| device.is_connected())
            .map(|device| device.serial.clone());
    }

    let Some(serial) = device_serial.as_deref() else {
        bail!(
            "No connected adb device found. Run `hypr-phone devices` first or pass a serial: `hypr-phone mirror <serial>`."
        );
    };

    if !devices
        .iter()
        .any(|device| device.serial == serial && device.is_connected())
    {
        bail!(
            "ADB device `{serial}` is not connected. Run `hypr-phone devices` and reconnect before mirroring."
        );
    }

    let window_title = build_window_title(
        config.mirror.window_title_prefix.as_str(),
        &profile_name,
        device_serial.as_deref(),
    );

    Ok(ResolvedMirrorConfig {
        device_serial,
        profile_name,
        profile,
        window_title,
        hyprland: config.mirror.hyprland.clone(),
    })
}

pub fn build_window_title(prefix: &str, profile_name: &str, serial: Option<&str>) -> String {
    let normalized_prefix = if prefix.trim().is_empty() {
        "hypr-phone"
    } else {
        prefix.trim()
    };

    match serial {
        Some(serial) => format!("{normalized_prefix}:{serial}"),
        None => format!("{normalized_prefix}:{profile_name}"),
    }
}

pub fn build_scrcpy_args(resolved: &ResolvedMirrorConfig) -> Vec<String> {
    resolved.profile.to_args(
        resolved.device_serial.as_deref(),
        resolved.window_title.as_str(),
    )
}

pub fn build_scrcpy_command(scrcpy_binary: &Path, args: &[String]) -> Command {
    let mut command = Command::new(scrcpy_binary);
    command.args(args);
    command
}

pub fn launch_mirror(config: &Config, request: MirrorRequest<'_>) -> Result<MirrorLaunch> {
    let resolved = resolve_mirror_config(config, request)?;
    let scrcpy_binary = scrcpy_path()?;
    let args = build_scrcpy_args(&resolved);
    let mut command = build_scrcpy_command(scrcpy_binary.as_path(), &args);
    let child = command
        .spawn()
        .with_context(|| format!("failed to launch scrcpy with args: {:?}", args))?;

    let hyprland_error = hyprland::place_window(&resolved.window_title, &resolved.hyprland)
        .err()
        .map(|error| error.to_string());

    Ok(MirrorLaunch {
        child,
        resolved,
        hyprland_error,
    })
}

pub fn scrcpy_path() -> Result<PathBuf> {
    which::which("scrcpy").map_err(|_| {
        missing_dependency(
            "scrcpy",
            "Install `scrcpy` and ensure it is available in PATH.",
        )
    })
}
