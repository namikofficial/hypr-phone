//! `hypr-phone toggle` — primary daily-use command.
//!
//! 1. Resolve default device.
//! 2. If disconnected → reconnect via cached endpoint.
//! 3. If scrcpy already running → toggle special workspace visibility.
//! 4. Otherwise → launch scrcpy with default profile.

use anyhow::Result;

use crate::commands::device;
use crate::config::Config;
use crate::domain::device::PhoneDevice;
use crate::services::adb::{self, AdbDiscovery};
use crate::services::hyprland;
use crate::services::scrcpy;

#[derive(Debug, Clone, Default)]
pub struct ToggleOptions {
    pub profile: Option<String>,
    pub app: Option<String>,
    pub no_mirror: bool,
}

pub fn run(opts: ToggleOptions) -> Result<()> {
    run_with_config(&Config::load_default()?, opts)
}

pub fn run_with_config(config: &Config, opts: ToggleOptions) -> Result<()> {
    let discovery = AdbDiscovery::new();
    let timeout = std::time::Duration::from_millis(750);
    let devices = discovery.devices_with_timeout(timeout)?;

    let resolved = resolve_target_device(config, &devices)?;

    if resolved.is_connected() {
        // Device reachable. Toggle visibility vs. launch based on whether
        // a scrcpy window is already present on the special workspace.
        let prefer_toggle_visibility = config
            .mirror
            .toggle
            .prefer_toggle_visibility
            && opts.app.is_none();

        if prefer_toggle_visibility {
            // Try toggling the workspace; if no scrcpy window exists,
            // fall through to launch.
            match hyprland::toggle_special_workspace(&config.mirror.hyprland.workspace) {
                Ok(()) => {
                    println!(
                        "Toggled special workspace `{}`.",
                        config.mirror.hyprland.workspace
                    );
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("[warn] workspace toggle failed: {e}; launching mirror instead.");
                }
            }
        }

        if opts.no_mirror {
            println!("(no_mirror set, skipping scrcpy launch)");
            return Ok(());
        }

        launch_for_device(config, &resolved, &opts)
    } else {
        // Try to reconnect.
        if config.mirror.toggle.auto_reconnect {
            if let Some(endpoint) = config.resolve_wireless_endpoint(None) {
                println!("Device not connected; attempting reconnect to {endpoint}…");
                let _ = adb::connect(&endpoint);
                // Re-query after short wait.
                std::thread::sleep(std::time::Duration::from_millis(500));
                let devices = discovery.devices_with_timeout(timeout)?;
                if let Some(updated) = devices
                    .iter()
                    .find(|d| d.adb_serial.as_deref() == resolved.adb_serial.as_deref())
                    .cloned()
                {
                    if updated.is_connected() {
                        return launch_for_device(config, &updated, &opts);
                    }
                }
            }
        }
        anyhow::bail!(
            "No connected device. Run `hypr-phone device list` and `hypr-phone device pair`."
        )
    }
}

fn launch_for_device(
    config: &Config,
    device: &PhoneDevice,
    opts: &ToggleOptions,
) -> Result<()> {
    let serial = device
        .adb_serial
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("device has no ADB serial"))?;

    let profile_name = opts
        .profile
        .clone()
        .or_else(|| Some(config.mirror.profile.clone()))
        .unwrap_or_else(|| "default".to_string());

    let resolved = scrcpy::resolve_mirror_config(
        config,
        scrcpy::MirrorRequest {
            device_serial: Some(serial),
            profile: Some(&profile_name),
            app_package: opts.app.as_deref(),
            ..Default::default()
        },
    )?;

    let mut command = if let Some(pkg) = &opts.app {
        scrcpy::build_scrcpy_app_args(&resolved, pkg, false, None)
    } else {
        scrcpy::build_scrcpy_args(&resolved)
    };
    command.push("--no-vd-system-decorations".to_string());
    command.push("--no-vd-destroy-content".to_string());

    let scrcpy_bin = scrcpy::scrcpy_path()?;
    let mut cmd = std::process::Command::new(&scrcpy_bin);
    cmd.args(&command);
    let child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to launch scrcpy: {e}"))?;

    let pid = child.id();
    let session = scrcpy::describe_session(&resolved, Some(pid), None);
    println!(
        "Started scrcpy (pid {pid}) for `{serial}` with profile `{}`.",
        session.profile
    );

    if let Err(e) = hyprland::place_window(&resolved.window_title, &resolved.placement) {
        eprintln!("[warn] hyprland placement failed: {e}");
    }
    Ok(())
}

fn resolve_target_device(config: &Config, devices: &[PhoneDevice]) -> Result<PhoneDevice> {
    // 1. Explicit default alias
    if let Some(alias) = &config.mirror.default_alias {
        if let Some(entry) = config.devices.entries.get(alias) {
            if let Some(serial) = &entry.adb_serial {
                if let Some(d) = devices.iter().find(|d| d.adb_serial.as_deref() == Some(serial.as_str())) {
                    return Ok(d.clone());
                }
            }
            if let Some(endpoint) = &entry.adb_endpoint {
                if let Some(d) = devices.iter().find(|d| {
                    d.reconnect_endpoint().as_deref() == Some(endpoint.as_str())
                }) {
                    return Ok(d.clone());
                }
            }
        }
    }
    // 2. mirror.device_serial
    if let Some(serial) = &config.mirror.device_serial {
        if let Some(d) = devices.iter().find(|d| d.adb_serial.as_deref() == Some(serial.as_str())) {
            return Ok(d.clone());
        }
    }
    // 3. First connected device
    if let Some(d) = devices.iter().find(|d| d.is_connected()) {
        return Ok(d.clone());
    }
    // 4. Fall back to last-known configured device (so user can be told
    //    what is missing).
    if let Some((alias, entry)) = config.devices.entries.iter().next() {
        return Ok(PhoneDevice {
            id: alias.clone(),
            display_name: entry.alias.clone().unwrap_or_else(|| alias.clone()),
            adb_serial: entry.adb_serial.clone(),
            adb_state: crate::domain::device::AdbState::Disconnected,
            transport: entry
                .adb_endpoint
                .as_deref()
                .and_then(|s| crate::domain::device::parse_endpoint(s).map(|a| {
                    crate::domain::device::Transport::Wifi {
                        ip: a.ip(),
                        port: a.port(),
                    }
                }))
                .unwrap_or(crate::domain::device::Transport::Unknown),
            ..PhoneDevice::from_adb_listing(
                entry.adb_serial.as_deref().unwrap_or(alias.as_str()),
                "disconnected",
                None,
                None,
            )
        });
    }

    anyhow::bail!(
        "No device configured or connected. Run `hypr-phone device pair` to add one."
    )
}

// Re-export for unit testing / advanced callers.
pub use adb::discovery as _discovery;

// Suppress unused warnings for stub dispatcher.
#[allow(dead_code)]
fn _touch() {
    let _ = device::run;
}
