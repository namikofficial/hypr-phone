//! `hypr-phone setup` — first-run guided setup (idempotent).

use anyhow::Result;

use crate::config::Config;
use crate::services::adb::AdbDiscovery;

pub fn run(apply: bool) -> Result<()> {
    println!("Hypr Phone — guided setup");
    println!("==========================");
    println!();

    let mut config = Config::load_default()?;
    println!("[1/6] Checking environment…");
    super::doctor::run(false)?;
    println!();

    println!("[2/6] Looking for connected devices…");
    let discovery = AdbDiscovery::new();
    let devices = discovery.devices_with_timeout(std::time::Duration::from_millis(1500))?;
    if devices.is_empty() {
        println!("  No devices detected.");
        println!("  - Plug in a USB cable with USB debugging enabled, or");
        println!("  - Run `hypr-phone device pair <ip:port> <code>` after pairing via Android settings.");
    } else {
        for d in &devices {
            println!(
                "  • {} ({}, {})",
                d.adb_serial.as_deref().unwrap_or("?"),
                d.adb_state,
                d.transport
            );
        }
    }
    println!();

    println!("[3/6] Configuring default device…");
    let aliases = config.devices.entries.clone();
    let default_alias = if aliases.len() == 1 {
        let (k, _) = aliases.iter().next().unwrap();
        println!("  Using existing alias `{k}` as default.");
        Some(k.clone())
    } else if aliases.is_empty() {
        if let Some(d) = devices.iter().find(|d| d.is_connected()) {
            let suggested = sanitize_alias(&d.display_name);
            println!(
                "  Detected connected device `{}`.",
                d.adb_serial.as_deref().unwrap_or("?")
            );
            println!("  Suggested alias: `{suggested}`");
            config.devices.entries.insert(
                suggested.clone(),
                crate::config::DeviceEntry {
                    device_id: Some(d.id.clone()),
                    alias: Some(suggested.clone()),
                    adb_serial: d.adb_serial.clone(),
                    adb_endpoint: d.transport.endpoint_string(),
                    kdeconnect_id: None,
                    notes: None,
                },
            );
            Some(suggested)
        } else {
            println!("  No connected device to seed from.");
            None
        }
    } else {
        println!("  Multiple devices already configured. Run `hypr-phone device list`.");
        config.mirror.default_alias.clone()
    };
    config.mirror.default_alias = default_alias;
    println!();

    println!("[4/6] Hyprland bindings (copy into your hyprland.conf):");
    println!();
    println!("    # Phone toggle");
    println!(
        "    bind = {}, hypr-phone toggle",
        config.hyprland.primary_bindings.toggle
    );
    println!();
    println!("    # Phone action menu");
    println!(
        "    bind = {}, hypr-phone menu",
        config.hyprland.primary_bindings.menu
    );
    println!();
    println!("    # Android apps picker");
    println!(
        "    bind = {}, hypr-phone app",
        config.hyprland.primary_bindings.apps
    );
    println!();
    println!("    # Screenshot");
    println!(
        "    bind = {}, hypr-phone screenshot",
        config.hyprland.primary_bindings.screenshot
    );
    println!();
    println!("    # Record");
    println!(
        "    bind = {}, hypr-phone record start",
        config.hyprland.primary_bindings.record
    );
    println!();

    println!("[5/6] Waybar example:");
    println!();
    println!("    \"custom/phone\": {{");
    println!("        \"exec\": \"hypr-phone module\",");
    println!(
        "        \"on-click\": \"hypr-phone toggle\","
    );
    println!(
        "        \"on-click-right\": \"hypr-phone menu\","
    );
    println!("        \"interval\": 5");
    println!("    }}");
    println!();

    println!("[6/6] Saving config…");
    if apply {
        let path = config.save_default()?;
        println!("  Wrote config to {}", path.display());
    } else {
        println!("  Dry-run. Re-run with `--apply` to write changes.");
    }
    println!();
    println!("Setup complete.");
    Ok(())
}

fn sanitize_alias(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    lower
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}
