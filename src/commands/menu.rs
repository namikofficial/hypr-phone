//! Contextual launcher menu (rofi/wofi).

use anyhow::Result;

use crate::config::Config;
use crate::services::adb::AdbDiscovery;
use crate::services::kdeconnect;
use crate::ui::rofi;

/// Run the contextual launcher.
pub fn run() -> Result<()> {
    let config = Config::load_default()?;
    let discovery = AdbDiscovery::new();
    let devices = discovery.devices_with_timeout(std::time::Duration::from_millis(500))?;
    let connected = devices.iter().find(|d| d.is_connected()).cloned();

    let backend = rofi::detect_backend();
    if backend.is_none() {
        anyhow::bail!("neither `rofi` nor `wofi` is available");
    }

    let actions = build_menu_actions(connected.is_some());

    if let Some(backend) = backend {
        let entries: Vec<(String, String)> = actions
            .iter()
            .map(|a| (a.id().to_string(), a.label().to_string()))
            .collect();
        let Some(selected) = rofi::show_menu(backend, "Hypr Phone", &entries)? else {
            return Ok(());
        };

        // Map selection back to action id.
        let Some((action_id, _)) = entries.into_iter().find(|(_, l)| l == &selected) else {
            return Ok(());
        };

        dispatch(&action_id, &config, connected.as_ref())?;
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct MenuAction {
    pub id: &'static str,
    pub label: String,
}

impl MenuAction {
    pub fn id(&self) -> &'static str {
        self.id
    }
    pub fn label(&self) -> &str {
        &self.label
    }
}

pub fn build_menu_actions(connected: bool) -> Vec<MenuAction> {
    if connected {
        vec![
            MenuAction {
                id: "toggle",
                label: "Toggle phone".into(),
            },
            MenuAction {
                id: "screenshot",
                label: "Screenshot".into(),
            },
            MenuAction {
                id: "send",
                label: "Send (file / url / text)".into(),
            },
            MenuAction {
                id: "app",
                label: "Open Android app…".into(),
            },
            MenuAction {
                id: "record",
                label: "Record screen".into(),
            },
            MenuAction {
                id: "control",
                label: "Device controls…".into(),
            },
            MenuAction {
                id: "clipboard",
                label: "Clipboard sync".into(),
            },
            MenuAction {
                id: "kde_battery",
                label: "Battery (KDE Connect)".into(),
            },
            MenuAction {
                id: "kde_ring",
                label: "Find / ring phone".into(),
            },
            MenuAction {
                id: "doctor",
                label: "Doctor / diagnostics".into(),
            },
            MenuAction {
                id: "setup",
                label: "Setup / preferences".into(),
            },
        ]
    } else {
        vec![
            MenuAction {
                id: "connect",
                label: "Connect phone".into(),
            },
            MenuAction {
                id: "pair",
                label: "Pair new phone".into(),
            },
            MenuAction {
                id: "reconnect",
                label: "Reconnect last phone".into(),
            },
            MenuAction {
                id: "doctor",
                label: "Doctor / diagnostics".into(),
            },
            MenuAction {
                id: "setup",
                label: "Setup".into(),
            },
        ]
    }
}

fn dispatch(
    action_id: &str,
    _config: &Config,
    _device: Option<&crate::domain::device::PhoneDevice>,
) -> Result<()> {
    match action_id {
        "toggle" => {
            // Delegate to the real toggle command.
            super::toggle::run(super::toggle::ToggleOptions::default())
        }
        "screenshot" => {
            // Use default output path.
            super::screenshot::run(None)
        }
        "send" => {
            // Send requires user input - delegate to the send command with stdin read.
            // Since rofi doesn't provide text input, we show a message.
            println!("Use `hypr-phone send <file|url|text>` to send content.");
            Ok(())
        }
        "app" => {
            // App picker requires rofi for interactive selection.
            super::app::run(None)
        }
        "record" => {
            // Start recording with default output.
            super::record::run(crate::cli::RecordAction::Start { output: None })
        }
        "control" => {
            // Control requires a sub-action - show available actions.
            println!("Use `hypr-phone control <home|back|volume-up|...>` for device controls.");
            Ok(())
        }
        "clipboard" => {
            // Sync clipboard from host to phone.
            super::clipboard::run(crate::cli::ClipboardAction::Send { text: None })
        }
        "kde_battery" => {
            // Show battery via KDE Connect.
            match kdeconnect::battery(None) {
                Ok(info) => {
                    println!(
                        "Battery: {}%{}{}",
                        info.level_percent,
                        if info.charging { " (charging)" } else { "" },
                        if info.level_percent < 20 {
                            " ⚠️"
                        } else {
                            ""
                        }
                    );
                    Ok(())
                }
                Err(e) => {
                    println!("Battery: unavailable ({})", e);
                    Ok(())
                }
            }
        }
        "kde_ring" => {
            // Ring the phone via KDE Connect.
            match kdeconnect::ring(None) {
                Ok(()) => {
                    println!("Ringing your phone…");
                    Ok(())
                }
                Err(e) => {
                    println!("Ring: unavailable ({})", e);
                    Ok(())
                }
            }
        }
        "doctor" => super::doctor::run(false),
        "setup" => super::setup::run(false),
        "connect" | "pair" | "reconnect" => {
            // These require additional input (endpoint, pairing code) that
            // can't be provided through a simple menu. Guide user to CLI.
            match action_id {
                "connect" => println!(
                    "Use `hypr-phone device connect [endpoint]` to connect a wireless device."
                ),
                "pair" => {
                    println!("Use `hypr-phone device pair <endpoint> <code>` to pair a device.")
                }
                "reconnect" => {
                    println!("Use `hypr-phone device reconnect` to reconnect to the last endpoint.")
                }
                _ => {}
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
