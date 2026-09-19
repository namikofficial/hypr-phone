//! `hypr-phone toggle` — primary daily-use command.
//!
//! Algorithm:
//! 1. Resolve selected target.
//! 2. If disconnected → try reconnect.
//! 3. Resolve presentation session:
//!    - If recorded PID dead → clean stale session.
//!    - If window exists and visible → hide.
//!    - If window exists and hidden → show.
//!    - If no window → launch scrcpy → wait for window → show.
//! 4. Never claim success just because Hyprland accepted a dispatcher.

use anyhow::{anyhow, Result};

use crate::config::Config;
use crate::domain::device::PhoneDevice;
use crate::services::adb::{self, AdbDiscovery};
use crate::services::hyprland;
use crate::services::scrcpy;
use crate::services::session::bridge as session;

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

    if !resolved.is_connected() {
        // Try to reconnect wireless device.
        if config.mirror.toggle.auto_reconnect {
            if let Some(endpoint) = config.resolve_wireless_endpoint(None) {
                println!("Device not connected; attempting reconnect to {endpoint}…");
                let _ = adb::connect(&endpoint);
                std::thread::sleep(std::time::Duration::from_millis(500));
                let devices = discovery.devices_with_timeout(timeout)?;
                if let Some(updated) = devices
                    .iter()
                    .find(|d| d.adb_serial.as_deref() == resolved.adb_serial.as_deref())
                    .cloned()
                {
                    if updated.is_connected() {
                        return launch_or_toggle(config, &updated, &opts);
                    }
                }
            }
        }
        anyhow::bail!(
            "No connected device. Run `hypr-phone device list` and `hypr-phone device pair`."
        );
    }

    launch_or_toggle(config, &resolved, &opts)
}

/// Main toggle logic: check existing session → decide show/hide/launch.
fn launch_or_toggle(config: &Config, device: &PhoneDevice, opts: &ToggleOptions) -> Result<()> {
    if opts.no_mirror {
        println!("(no_mirror set, skipping mirror)");
        return Ok(());
    }

    let serial = device
        .adb_serial
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("device has no ADB serial"))?;

    // Clean up any stale sessions first.
    let _ = session::cleanup_stale_sessions();

    // Look for an existing session for this device.
    let existing = session::find_session_by_serial(serial)?;

    let prefer_toggle = config.mirror.toggle.prefer_toggle_visibility && opts.app.is_none();

    if let Some(existing_session) = existing {
        // Session exists. Validate PID is still alive.
        if !session::validate_session(&existing_session) {
            // Stale session - clean it up.
            let _ = session::remove_session(&existing_session.id);
        } else if prefer_toggle {
            // Validate the window is actually visible before toggling.
            if hyprland::is_window_visible_on_workspace(
                &existing_session.window_title,
                &config.mirror.hyprland.workspace,
            )? {
                // Window is visible → hide it.
                return hide_mirror(config, &existing_session);
            } else {
                // Window exists but is hidden → show it.
                return show_mirror(config, &existing_session);
            }
        }
    }

    // No valid session or not doing visibility toggle → launch fresh.
    launch_for_device(config, device, opts)
}

/// Hide the mirror by toggling the special workspace.
fn hide_mirror(config: &Config, _session: &crate::domain::session::ScrcpySession) -> Result<()> {
    match hyprland::toggle_special_workspace(&config.mirror.hyprland.workspace) {
        Ok(()) => {
            println!("Hidden Android mirror.");
            Ok(())
        }
        Err(e) => Err(anyhow!("failed to hide mirror: {}", e)),
    }
}

/// Show/hide the mirror by toggling the special workspace.
fn show_mirror(config: &Config, session: &crate::domain::session::ScrcpySession) -> Result<()> {
    // First ensure the window is on the workspace.
    if let Err(e) = hyprland::place_window(&session.window_title, &config.mirror.hyprland) {
        eprintln!("[warn] placement failed: {e}");
    }
    match hyprland::show_special_workspace(&config.mirror.hyprland.workspace) {
        Ok(()) => {
            println!("Revealed Android mirror.");
            Ok(())
        }
        Err(e) => Err(anyhow!("failed to reveal mirror: {}", e)),
    }
}

/// Launch scrcpy for a device and persist the session.
fn launch_for_device(config: &Config, device: &PhoneDevice, opts: &ToggleOptions) -> Result<()> {
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
    let session_obj = scrcpy::describe_session(&resolved, Some(pid), None);

    // Persist session for cross-invocation knowledge.
    if let Err(e) = session::persist_session(&session_obj) {
        eprintln!("[warn] failed to persist session: {e}");
    }

    println!(
        "Started scrcpy (pid {pid}) for `{serial}` with profile `{}`.",
        session_obj.profile
    );

    // Place window on the configured workspace.
    if let Err(e) = hyprland::place_window(&resolved.window_title, &config.mirror.hyprland) {
        eprintln!("[warn] hyprland placement failed: {e}");
    }

    // Prove the window exists before claiming success.
    // Retry briefly to give scrcpy time to create the window.
    let window_found = hyprland::wait_for_window_with_timeout(
        &resolved.window_title,
        config.mirror.hyprland.retry_timeout_ms,
    )
    .is_ok();

    if window_found {
        println!("Android mirror ready.");
    } else {
        eprintln!("[warn] scrcpy launched but window not detected within timeout.");
    }

    Ok(())
}

fn resolve_target_device(config: &Config, devices: &[PhoneDevice]) -> Result<PhoneDevice> {
    // 1. Explicit default alias
    if let Some(alias) = &config.mirror.default_alias {
        if let Some(entry) = config.devices.entries.get(alias) {
            if let Some(serial) = &entry.adb_serial {
                if let Some(d) = devices
                    .iter()
                    .find(|d| d.adb_serial.as_deref() == Some(serial.as_str()))
                {
                    return Ok(d.clone());
                }
            }
            if let Some(endpoint) = &entry.adb_endpoint {
                if let Some(d) = devices
                    .iter()
                    .find(|d| d.reconnect_endpoint().as_deref() == Some(endpoint.as_str()))
                {
                    return Ok(d.clone());
                }
            }
        }
    }
    // 2. mirror.device_serial
    if let Some(serial) = &config.mirror.device_serial {
        if let Some(d) = devices
            .iter()
            .find(|d| d.adb_serial.as_deref() == Some(serial.as_str()))
        {
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
                .and_then(|s| {
                    crate::domain::device::parse_endpoint(s).map(|a| {
                        crate::domain::device::Transport::Wifi {
                            ip: a.ip(),
                            port: a.port(),
                        }
                    })
                })
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
        "No device configured or connected. Run `hypr-phone device list` and `hypr-phone device pair`."
    )
}

// Re-export for unit testing / advanced callers.
#[allow(dead_code)]
pub use adb::discovery as _discovery;
