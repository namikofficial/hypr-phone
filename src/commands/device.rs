//! `hypr-phone device` subcommands.

use anyhow::Result;

use crate::cli::DeviceAction;
use crate::config::Config;
use crate::services::adb;

pub fn run(action: DeviceAction) -> Result<()> {
    match action {
        DeviceAction::List { json } => list(json),
        DeviceAction::Pair { endpoint, code } => {
            let out = adb::pair(&endpoint, &code)?;
            println!("{out}");
            Ok(())
        }
        DeviceAction::Connect { endpoint } => {
            let endpoint = endpoint.ok_or_else(|| {
                anyhow::anyhow!("Provide an endpoint or run `hypr-phone device pair` first.")
            })?;
            let mut config = Config::load_default()?;
            let normalized = adb::normalize_endpoint(&endpoint, 5555);
            let out = adb::connect(&normalized)?;
            config.remember_endpoint(&normalized, None);
            let _ = config.save_default();
            println!("{out}");
            Ok(())
        }
        DeviceAction::Reconnect { target } => {
            let mut config = Config::load_default()?;
            let endpoint = config
                .resolve_wireless_endpoint(target.as_deref())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "No known wireless endpoint. Run `hypr-phone device pair` first."
                    )
                })?;
            let normalized = adb::normalize_endpoint(&endpoint, 5555);
            let out = adb::connect(&normalized)?;
            config.remember_endpoint(&normalized, None);
            let _ = config.save_default();
            println!("{out}");
            Ok(())
        }
        DeviceAction::Disconnect { serial } => {
            let out = adb::disconnect(&serial)?;
            println!("{out}");
            Ok(())
        }
    }
}

fn list(as_json: bool) -> Result<()> {
    let discovery = adb::AdbDiscovery::new();
    let devices = discovery.devices_l()?;
    if as_json {
        println!("{}", serde_json::to_string_pretty(&devices)?);
        return Ok(());
    }
    if devices.is_empty() {
        println!("No adb devices detected.");
        return Ok(());
    }
    for d in devices {
        let serial = d.adb_serial.as_deref().unwrap_or("?");
        println!("{serial} ({}) — {}", d.adb_state, d.transport);
        if let Some(model) = &d.model {
            println!("    model: {model}");
        }
    }
    Ok(())
}
