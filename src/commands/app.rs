//! `hypr-phone app` — launch an Android app on the phone in its own scrcpy
//! virtual-display window.

use anyhow::Result;

use crate::config::Config;
use crate::services::{adb, hyprland, scrcpy};
use crate::ui::rofi;

pub fn run(package: Option<String>) -> Result<()> {
    let config = Config::load_default()?;
    let discovery = adb::AdbDiscovery::new();
    let devices = discovery.devices_with_timeout(std::time::Duration::from_millis(750))?;
    let device = devices
        .iter()
        .find(|d| d.is_connected())
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("No connected device. Connect a phone first."))?;
    let serial = device
        .adb_serial
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Device has no ADB serial"))?;

    let pkg = match package {
        Some(p) if p.contains('.') => p,
        Some(query) => {
            // Treat as fuzzy query via rofi.
            let apps = list_apps(Some(&serial))?;
            let backend = rofi::detect_backend().ok_or_else(|| {
                anyhow::anyhow!("Provide an exact package name or install rofi/wofi.")
            })?;
            let entries: Vec<(String, String)> = apps
                .iter()
                .filter(|a| a.contains(&query))
                .map(|a| (a.clone(), a.clone()))
                .collect();
            let Some(selected) = rofi::show_menu(backend, "Apps", &entries)? else {
                return Ok(());
            };
            selected
        }
        None => {
            // Show full picker.
            let apps = list_apps(Some(&serial))?;
            let backend = rofi::detect_backend().ok_or_else(|| {
                anyhow::anyhow!("Provide an exact package name or install rofi/wofi.")
            })?;
            let entries: Vec<(String, String)> =
                apps.iter().map(|a| (a.clone(), a.clone())).collect();
            let Some(selected) = rofi::show_menu(backend, "Apps", &entries)? else {
                return Ok(());
            };
            selected
        }
    };

    if !is_safe_package(&pkg) {
        anyhow::bail!(
            "Refusing to launch `{pkg}` — package name must match `<reverse.dns>` shape."
        );
    }

    launch_app(&config, &serial, &pkg)?;
    Ok(())
}

fn is_safe_package(p: &str) -> bool {
    !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '.') && p.contains('.')
}

fn list_apps(serial: Option<&str>) -> Result<Vec<String>> {
    let raw = adb::ops::run_adb_shell(serial, &["pm", "list", "packages", "-3"])?;
    Ok(raw
        .lines()
        .filter_map(|l| l.strip_prefix("package:").map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
        .collect())
}

pub fn launch_app(config: &Config, serial: &str, package: &str) -> Result<()> {
    let resolved = scrcpy::resolve_mirror_config(
        config,
        scrcpy::MirrorRequest {
            device_serial: Some(serial),
            profile: Some("app"),
            app_package: Some(package),
            ..Default::default()
        },
    )?;
    let args = scrcpy::build_scrcpy_app_args(&resolved, package, false, None);
    let bin = scrcpy::scrcpy_path()?;
    let mut cmd = std::process::Command::new(&bin);
    cmd.args(&args);
    let child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to launch scrcpy: {e}"))?;
    println!("Launched `{package}` on `{serial}` (pid {}).", child.id());
    if let Err(e) = hyprland::place_window(&resolved.window_title, &resolved.placement) {
        eprintln!("[warn] hyprland placement failed: {e}");
    }
    Ok(())
}
