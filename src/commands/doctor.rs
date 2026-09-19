//! `hypr-phone doctor` — thorough environment diagnostic.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::services::scrcpy::detect_scrcpy_capabilities;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub ok: bool,
    pub entries: Vec<DoctorEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorEntry {
    pub level: String,
    pub name: String,
    pub detail: String,
    pub hint: Option<String>,
}

pub fn run(json: bool) -> Result<()> {
    let config = Config::load_default()?;
    let entries = build_report(&config);

    if json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    let mut missing_required = 0;
    for e in &entries.entries {
        let icon = match e.level.as_str() {
            "ok" => "[OK]      ",
            "info" => "[INFO]    ",
            "warn" => "[WARN]    ",
            "broken" => "[BROKEN]  ",
            "hint" => "[HINT]    ",
            _ => "[?]       ",
        };
        println!("{icon} {}: {}", e.name, e.detail);
        if let Some(h) = &e.hint {
            println!("           {h}");
        }
        if e.level == "broken" {
            missing_required += 1;
        }
    }

    println!();
    if missing_required == 0 {
        println!("Doctor passed. All required dependencies are available.");
        Ok(())
    } else {
        anyhow::bail!(
            "Doctor failed. {} required component(s) missing.",
            missing_required
        )
    }
}

fn build_report(config: &Config) -> DoctorReport {
    let mut entries: Vec<DoctorEntry> = Vec::new();

    // adb
    match which::which("adb") {
        Ok(path) => {
            let mut detail = path.display().to_string();
            if let Ok(version) = binary_version("adb", &["--version"]) {
                detail = format!("{} ({})", detail, version);
            }
            entries.push(DoctorEntry {
                level: "ok".into(),
                name: "adb".into(),
                detail,
                hint: None,
            });
        }
        Err(_) => entries.push(DoctorEntry {
            level: "broken".into(),
            name: "adb".into(),
            detail: "not in PATH".into(),
            hint: Some(
                "Install Android platform-tools: `sudo pacman -S android-tools` (Arch) or equivalent."
                    .into(),
            ),
        }),
    }

    // adb mDNS / wireless
    let adb_ok = which::which("adb").is_ok();
    if adb_ok {
        match binary_version("adb", &["mdns", "services"]) {
            Ok(_) => entries.push(DoctorEntry {
                level: "ok".into(),
                name: "adb mDNS discovery".into(),
                detail: "available".into(),
                hint: None,
            }),
            Err(_) => entries.push(DoctorEntry {
                level: "warn".into(),
                name: "adb mDNS discovery".into(),
                detail: "unavailable on this ADB version".into(),
                hint: Some(
                    "Update `android-tools` to ≥31 for mDNS-based wireless device discovery.".into(),
                ),
            }),
        }
    }

    // scrcpy
    let scrcpy_caps = detect_scrcpy_capabilities();
    match which::which("scrcpy") {
        Ok(path) => {
            let mut detail = path.display().to_string();
            if let Some(v) = &scrcpy_caps.version {
                detail.push_str(&format!(" (v{v})"));
            }
            entries.push(DoctorEntry {
                level: "ok".into(),
                name: "scrcpy".into(),
                detail,
                hint: None,
            });
            entries.push(DoctorEntry {
                level: if scrcpy_caps.virtual_display { "ok" } else { "warn" }.into(),
                name: "scrcpy virtual display".into(),
                detail: if scrcpy_caps.virtual_display {
                    "supported".into()
                } else {
                    "not supported by this version".into()
                },
                hint: if scrcpy_caps.virtual_display {
                    None
                } else {
                    Some("Update scrcpy to ≥2.0 for `--new-display` support (required for app mode).".into())
                },
            });
            entries.push(DoctorEntry {
                level: if scrcpy_caps.start_app { "ok" } else { "warn" }.into(),
                name: "scrcpy start-app".into(),
                detail: if scrcpy_caps.start_app { "supported".into() } else { "not supported".into() },
                hint: None,
            });
            entries.push(DoctorEntry {
                level: if scrcpy_caps.flex_display { "ok" } else { "info" }.into(),
                name: "scrcpy flex-display".into(),
                detail: if scrcpy_caps.flex_display { "supported".into() } else { "not supported".into() },
                hint: None,
            });
            entries.push(DoctorEntry {
                level: if scrcpy_caps.recording { "ok" } else { "warn" }.into(),
                name: "scrcpy recording".into(),
                detail: if scrcpy_caps.recording { "supported".into() } else { "not supported".into() },
                hint: None,
            });
        }
        Err(_) => entries.push(DoctorEntry {
            level: "broken".into(),
            name: "scrcpy".into(),
            detail: "not in PATH".into(),
            hint: Some("Install scrcpy: `sudo pacman -S scrcpy`".into()),
        }),
    }

    // hyprctl
    match which::which("hyprctl") {
        Ok(path) => {
            let mut detail = path.display().to_string();
            if let Ok(version) = binary_version("hyprctl", &["version"]) {
                if let Some(line) = version.lines().next() {
                    detail.push_str(&format!(" ({line})"));
                }
            }
            let instance = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok();
            let (session_level, session_detail, session_hint) = match instance {
                Some(s) => ("ok", format!("instance `{s}`"), None),
                None => (
                    "warn",
                    "$HYPRLAND_INSTANCE_SIGNATURE not set".to_string(),
                    Some("hypr-phone must be launched inside a Hyprland session.".to_string()),
                ),
            };
            entries.push(DoctorEntry {
                level: "ok".into(),
                name: "hyprctl".into(),
                detail,
                hint: None,
            });
            entries.push(DoctorEntry {
                level: session_level.into(),
                name: "Hyprland session".into(),
                detail: session_detail,
                hint: session_hint,
            });
        }
        Err(_) => entries.push(DoctorEntry {
            level: "broken".into(),
            name: "hyprctl".into(),
            detail: "not in PATH".into(),
            hint: Some("hypr-phone requires Hyprland to place windows.".into()),
        }),
    }

    // wl-clipboard
    let wl_copy = which::which("wl-copy").is_ok();
    let wl_paste = which::which("wl-paste").is_ok();
    if wl_copy && wl_paste {
        entries.push(DoctorEntry {
            level: "ok".into(),
            name: "wl-clipboard".into(),
            detail: "wl-copy and wl-paste available".into(),
            hint: None,
        });
    } else {
        entries.push(DoctorEntry {
            level: "broken".into(),
            name: "wl-clipboard".into(),
            detail: "wl-copy or wl-paste missing".into(),
            hint: Some("Install `wl-clipboard` for host clipboard sync.".into()),
        });
    }

    // notify-send
    match which::which("notify-send") {
        Ok(_) => entries.push(DoctorEntry {
            level: "ok".into(),
            name: "notify-send".into(),
            detail: "available".into(),
            hint: None,
        }),
        Err(_) => entries.push(DoctorEntry {
            level: "warn".into(),
            name: "notify-send".into(),
            detail: "not in PATH — desktop notifications disabled".into(),
            hint: Some("Install `libnotify` to enable notifications.".into()),
        }),
    }

    // rofi / wofi
    let rofi = which::which("rofi").is_ok();
    let wofi = which::which("wofi").is_ok();
    if rofi {
        entries.push(DoctorEntry {
            level: "ok".into(),
            name: "rofi".into(),
            detail: "available".into(),
            hint: None,
        });
    } else if wofi {
        entries.push(DoctorEntry {
            level: "ok".into(),
            name: "wofi".into(),
            detail: "available (fallback)".into(),
            hint: None,
        });
    } else {
        entries.push(DoctorEntry {
            level: "broken".into(),
            name: "rofi/wofi".into(),
            detail: "neither available".into(),
            hint: Some("Install either `rofi` or `wofi` for the contextual menu.".into()),
        });
    }

    // kdeconnect-cli
    match which::which("kdeconnect-cli") {
        Ok(_) => entries.push(DoctorEntry {
            level: "ok".into(),
            name: "kdeconnect-cli".into(),
            detail: "available (optional)".into(),
            hint: None,
        }),
        Err(_) => entries.push(DoctorEntry {
            level: "warn".into(),
            name: "kdeconnect-cli".into(),
            detail: "not installed — KDE Connect features disabled".into(),
            hint: Some(
                "Install `kdeconnect` to enable battery, ring, and share features.".into(),
            ),
        }),
    }

    // Config
    entries.push(DoctorEntry {
        level: "info".into(),
        name: "config".into(),
        detail: format!("version {}", config.config_version),
        hint: Some(if config.is_current_version() {
            "config is up to date".into()
        } else {
            "run `hypr-phone config migrate` to upgrade".into()
        }),
    });

    // Host name resolution sanity
    entries.push(DoctorEntry {
        level: "info".into(),
        name: "Android env".into(),
        detail: if config.devices.entries.is_empty() {
            "no paired aliases configured".into()
        } else {
            format!(
                "{} device(s) configured",
                config.devices.entries.len()
            )
        },
        hint: if config.devices.entries.is_empty() {
            Some("Run `hypr-phone setup` or `hypr-phone device pair` to add a device.".into())
        } else {
            None
        },
    });

    DoctorReport {
        ok: entries.iter().all(|e| e.level != "broken"),
        entries,
    }
}

fn binary_version(binary: &str, args: &[&str]) -> Result<String> {
    let path = which::which(binary)?;
    let output = std::process::Command::new(path).args(args).output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_is_serializable() {
        let cfg = Config::default();
        let report = build_report(&cfg);
        let json = serde_json::to_string(&report).expect("serialize");
        assert!(json.contains("entries"));
    }
}
