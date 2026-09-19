//! Canonical phone status model.
//!
//! One status struct consumed by CLI output, rofi menu, Waybar, and any
//! future UI. Built from device discovery + scrcpy session + KDE Connect.

use serde::{Deserialize, Serialize};

use super::device::{DeviceSummary, PhoneDevice, TransportKind};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PresenceStatus {
    #[default]
    Disconnected,
    Connected,
    Connecting,
    Unauthorized,
    Error,
}

impl PresenceStatus {
    pub fn from_device(d: Option<&PhoneDevice>) -> Self {
        match d {
            None => PresenceStatus::Disconnected,
            Some(d) => match d.adb_state {
                super::device::AdbState::Connected => PresenceStatus::Connected,
                super::device::AdbState::Unauthorized => PresenceStatus::Unauthorized,
                _ => PresenceStatus::Disconnected,
            },
        }
    }

    pub fn class_name(&self) -> &'static str {
        match self {
            PresenceStatus::Connected => "connected",
            PresenceStatus::Disconnected => "disconnected",
            PresenceStatus::Connecting => "connecting",
            PresenceStatus::Unauthorized => "unauthorized",
            PresenceStatus::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportInfo {
    pub kind: TransportKind,
    pub endpoint: Option<String>,
}

impl From<&PhoneDevice> for TransportInfo {
    fn from(d: &PhoneDevice) -> Self {
        Self {
            kind: d.transport.kind(),
            endpoint: d.reconnect_endpoint(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryInfo {
    pub level_percent: u8,
    pub charging: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdeStatus {
    pub paired: bool,
    pub reachable: bool,
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub profile: Option<String>,
    pub display_id: Option<u32>,
    pub on_special_workspace: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuntimeCapabilities {
    pub scrcpy_virtual_display: bool,
    pub scrcpy_start_app: bool,
    pub scrcpy_flex_display: bool,
    pub scrcpy_audio: bool,
    pub scrcpy_recording: bool,
    pub kdeconnect_cli: bool,
    pub adb_mdns: bool,
    pub hyprland_socket: bool,
    pub rofi_or_wofi: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneStatus {
    #[serde(default)]
    pub device: Option<DeviceSummary>,
    #[serde(default)]
    pub presence: PresenceStatus,
    #[serde(default)]
    pub transport: Option<TransportInfo>,
    #[serde(default)]
    pub battery: Option<BatteryInfo>,
    #[serde(default)]
    pub kdeconnect: Option<KdeStatus>,
    #[serde(default)]
    pub mirror: Option<MirrorStatus>,
    #[serde(default)]
    pub capabilities: RuntimeCapabilities,
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl PhoneStatus {
    pub fn new() -> Self {
        Self {
            device: None,
            presence: PresenceStatus::Disconnected,
            transport: None,
            battery: None,
            kdeconnect: None,
            mirror: None,
            capabilities: RuntimeCapabilities::default(),
            warnings: Vec::new(),
        }
    }

    pub fn with_device(mut self, device: &PhoneDevice) -> Self {
        self.device = Some(DeviceSummary::from(device));
        self.presence = PresenceStatus::from_device(Some(device));
        self.transport = Some(TransportInfo::from(device));
        self
    }

    pub fn add_warning(&mut self, msg: impl Into<String>) {
        self.warnings.push(msg.into());
    }

    pub fn summary_class(&self) -> &'static str {
        if let Some(mirror) = &self.mirror {
            if mirror.running {
                return "mirroring";
            }
        }
        self.presence.class_name()
    }
}

impl Default for PhoneStatus {
    fn default() -> Self {
        Self::new()
    }
}

/// Waybar-compatible JSON shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaybarStatus {
    pub text: String,
    pub tooltip: String,
    pub class: String,
    pub alt: String,
}

impl From<&PhoneStatus> for WaybarStatus {
    fn from(s: &PhoneStatus) -> Self {
        let (text, _icon, alt) = match (&s.device, s.presence) {
            (Some(d), _) => (
                format!("\u{e1f8} {}", truncate(&d.name, 18)),
                "\u{e1f8}",
                "connected",
            ),
            (None, PresenceStatus::Connecting) => (
                "\u{f0213} Connecting…".to_string(),
                "\u{f0213}",
                "connecting",
            ),
            (None, _) => ("\u{e1f7} No phone".to_string(), "\u{e1f7}", "disconnected"),
        };

        let mut tooltip = String::new();
        if let Some(d) = &s.device {
            tooltip.push_str(&d.name);
            tooltip.push('\n');
        }
        if let Some(t) = &s.transport {
            tooltip.push_str(&format!("Transport: {}\n", t.kind));
        }
        if let Some(b) = &s.battery {
            tooltip.push_str(&format!(
                "Battery: {}%{}\n",
                b.level_percent,
                if b.charging { " (charging)" } else { "" }
            ));
        }
        if let Some(kde) = &s.kdeconnect {
            tooltip.push_str(&format!(
                "KDE Connect: {}{}\n",
                if kde.reachable {
                    "reachable"
                } else {
                    "unreachable"
                },
                if kde.paired { " (paired)" } else { "" }
            ));
        }
        if let Some(m) = &s.mirror {
            if m.running {
                tooltip.push_str(&format!(
                    "Mirror: running (profile {})\n",
                    m.profile.as_deref().unwrap_or("?")
                ));
            }
        }
        if !s.warnings.is_empty() {
            tooltip.push_str("\nWarnings:\n");
            for w in &s.warnings {
                tooltip.push_str(&format!("  • {w}\n"));
            }
        }
        if tooltip.is_empty() {
            tooltip.push_str("No Android device connected");
        }
        let tooltip = tooltip.trim_end().to_string();

        let class = s.summary_class().to_string();

        Self {
            text,
            tooltip,
            class,
            alt: alt.to_string(),
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{truncated}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_device() -> PhoneDevice {
        PhoneDevice::from_adb_listing("192.168.1.5:5555", "device", Some("Pixel 8".into()), None)
    }

    #[test]
    fn status_with_device_is_connected() {
        let s = PhoneStatus::new().with_device(&sample_device());
        assert_eq!(s.presence, PresenceStatus::Connected);
        assert_eq!(s.summary_class(), "connected");
    }

    #[test]
    fn status_without_device_is_disconnected() {
        let s = PhoneStatus::new();
        assert_eq!(s.summary_class(), "disconnected");
    }

    #[test]
    fn waybar_includes_battery_and_kde_when_present() {
        let mut s = PhoneStatus::new().with_device(&sample_device());
        s.battery = Some(BatteryInfo {
            level_percent: 74,
            charging: false,
        });
        s.kdeconnect = Some(KdeStatus {
            paired: true,
            reachable: true,
            device_id: "kde-pixel".into(),
        });
        s.mirror = Some(MirrorStatus {
            running: true,
            pid: Some(1234),
            profile: Some("default".into()),
            display_id: None,
            on_special_workspace: true,
        });

        let w = WaybarStatus::from(&s);
        assert_eq!(w.class, "mirroring");
        assert!(w.tooltip.contains("Pixel 8"));
        assert!(w.tooltip.contains("74%"));
        assert!(w.tooltip.contains("KDE Connect"));
        assert!(w.tooltip.contains("Mirror"));
    }

    #[test]
    fn waybar_text_includes_device_name() {
        let s = PhoneStatus::new().with_device(&sample_device());
        let w = WaybarStatus::from(&s);
        assert!(w.text.contains("Pixel 8"));
    }
}
