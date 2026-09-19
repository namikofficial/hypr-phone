//! `hypr-phone status` and module/waybar JSON output.

use crate::config::Config;
use crate::domain::status::{PhoneStatus, WaybarStatus};
use crate::services::adb::AdbDiscovery;
use crate::services::hyprland;
use crate::services::kdeconnect;
use crate::services::scrcpy::detect_scrcpy_capabilities;
use crate::services::session::bridge as session;
use crate::ui::waybar;
use anyhow::Result;

pub fn run(json: bool, waybar_flag: bool) -> Result<()> {
    let config = Config::load_default()?;
    let mut status = PhoneStatus::new();

    // Device discovery (bounded).
    let discovery = AdbDiscovery::new();
    let devices = discovery.devices_with_timeout(std::time::Duration::from_millis(750))?;
    if let Some(device) = devices.iter().find(|d| d.is_connected()) {
        status = status.with_device(device);
    }

    // Mirror session: read from session bridge (XDG_RUNTIME_DIR-based
    // cross-invocation state until hypr-phoned provides authoritative state).
    if let Some(serial) = status.transport.as_ref().and_then(|t| t.endpoint.as_ref()) {
        // Try to find session by the device serial.
        if let Ok(Some(sess)) = session::find_session_by_serial(serial) {
            if session::validate_session(&sess) {
                status.mirror = Some(crate::domain::status::MirrorStatus {
                    running: true,
                    pid: sess.pid,
                    profile: Some(sess.profile),
                    display_id: sess.display_id,
                    on_special_workspace: hyprland::is_window_visible_on_workspace(
                        &sess.window_title,
                        &config.mirror.hyprland.workspace,
                    )
                    .unwrap_or(false),
                });
            } else {
                // Session is stale - clean it up.
                let _ = session::remove_session(&sess.id);
            }
        }
    }

    // Runtime capabilities (cached for the lifetime of the process).
    populate_capabilities(&mut status);

    if json {
        if waybar_flag {
            let wb = WaybarStatus::from(&status);
            println!("{}", serde_json::to_string(&wb)?);
        } else {
            println!("{}", serde_json::to_string_pretty(&status)?);
        }
        return Ok(());
    }

    if waybar_flag {
        let wb = WaybarStatus::from(&status);
        println!("{}", serde_json::to_string(&wb)?);
        return Ok(());
    }

    // Human-readable summary.
    print_human(&status);
    let _ = &config; // currently unused; future: device alias lookup
    Ok(())
}

fn populate_capabilities(status: &mut PhoneStatus) {
    status.capabilities.scrcpy_virtual_display = detect_scrcpy_capabilities().virtual_display;
    status.capabilities.scrcpy_start_app = detect_scrcpy_capabilities().start_app;
    status.capabilities.scrcpy_flex_display = detect_scrcpy_capabilities().flex_display;
    status.capabilities.scrcpy_audio = detect_scrcpy_capabilities().audio;
    status.capabilities.scrcpy_recording = detect_scrcpy_capabilities().recording;
    status.capabilities.kdeconnect_cli = which::which("kdeconnect-cli").is_ok();
    status.capabilities.hyprland_socket = hyprland::hyprland_available();
    status.capabilities.rofi_or_wofi = which::which("rofi").is_ok() || which::which("wofi").is_ok();
    // adb mDNS support: present in modern android-tools (>=31).
    status.capabilities.adb_mdns = which::which("adb")
        .ok()
        .and_then(|_| {
            std::process::Command::new("adb")
                .args(["mdns", "services"])
                .output()
                .ok()
                .map(|o| o.status.success() || !o.stdout.is_empty())
        })
        .unwrap_or(false);
}

fn print_human(status: &PhoneStatus) {
    let header = match &status.device {
        Some(d) => format!("{} ({})", d.name, status.presence.class_name()),
        None => format!("no device ({})", status.presence.class_name()),
    };
    println!("{header}");
    if let Some(t) = &status.transport {
        println!("  transport: {}", t.kind);
        if let Some(ep) = &t.endpoint {
            println!("  endpoint: {ep}");
        }
    }
    if let Some(b) = &status.battery {
        println!(
            "  battery: {}%{}",
            b.level_percent,
            if b.charging { " (charging)" } else { "" }
        );
    }
    if let Some(kde) = &status.kdeconnect {
        println!(
            "  kdeconnect: paired={}, reachable={}",
            kde.paired, kde.reachable
        );
    }
    if let Some(m) = &status.mirror {
        if m.running {
            println!(
                "  mirror: running (profile {}, pid {:?})",
                m.profile.as_deref().unwrap_or("?"),
                m.pid
            );
        }
    }
    if !status.warnings.is_empty() {
        println!("  warnings:");
        for w in &status.warnings {
            println!("    - {w}");
        }
    }
}

#[allow(dead_code)]
pub fn quick_json() -> Result<String> {
    let status = PhoneStatus::new();
    Ok(waybar::json(&status))
}

#[allow(dead_code)]
fn _kde_optional_status(target: Option<&str>) -> Option<crate::domain::status::KdeStatus> {
    match kdeconnect::list_devices() {
        Ok(devices) => {
            let target = target.unwrap_or("");
            if let Some(d) = devices.iter().find(|d| d.id == target) {
                Some(crate::domain::status::KdeStatus {
                    paired: true,
                    reachable: true,
                    device_id: d.id.clone(),
                })
            } else {
                devices.first().map(|d| crate::domain::status::KdeStatus {
                    paired: true,
                    reachable: true,
                    device_id: d.id.clone(),
                })
            }
        }
        Err(_) => None,
    }
}
