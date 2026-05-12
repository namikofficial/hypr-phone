use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::errors::Result;
use crate::hyprland::HyprlandPlacementConfig;
use crate::scrcpy::{
    ScrcpyProfile, DEFAULT_PROFILE_NAME, LOW_LATENCY_PROFILE_NAME, PRESENTATION_PROFILE_NAME,
};

pub const STABLE_CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub config_version: u32,
    pub mirror: MirrorConfig,
    pub devices: DeviceConfig,
    pub reconnect: ReconnectConfig,
    pub gui: GuiConfig,
    // Legacy fields kept for compatibility.
    pub device_serial: Option<String>,
    pub scrcpy_args: Vec<String>,
}

impl Config {
    pub fn default_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("could not resolve config directory")?;
        Ok(config_dir.join("hypr-phone").join("config.toml"))
    }

    pub fn load_from_path(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file at {}", path.display()))?;
        let config = toml::from_str::<Self>(&raw)
            .with_context(|| format!("failed to parse TOML config at {}", path.display()))?;
        Ok(config.normalized())
    }

    pub fn load_default() -> Result<Self> {
        let path = Self::default_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }

        Self::load_from_path(&path)
    }

    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create config directory at {}", parent.display())
            })?;
        }
        fs::write(path, self.to_toml_string()?)
            .with_context(|| format!("failed to write config to {}", path.display()))?;
        Ok(())
    }

    pub fn save_default(&self) -> Result<PathBuf> {
        let path = Self::default_path()?;
        self.save_to_path(&path)?;
        Ok(path)
    }

    pub fn to_toml_string(&self) -> Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    pub fn is_stable_version(&self) -> bool {
        self.config_version == STABLE_CONFIG_VERSION
    }

    fn normalized(mut self) -> Self {
        if self.config_version == 0 {
            self.config_version = STABLE_CONFIG_VERSION;
        }

        if self.mirror.device_serial.is_none() {
            self.mirror.device_serial = self.device_serial.clone();
        }

        if !self.scrcpy_args.is_empty() {
            self.mirror
                .profiles
                .entry(self.mirror.profile.clone())
                .or_default()
                .args
                .extend(self.scrcpy_args.clone());
        }

        self
    }

    pub fn resolve_alias(&self, alias_or_serial: &str) -> Option<&DeviceAlias> {
        self.devices.aliases.get(alias_or_serial)
    }

    pub fn resolve_target_serial(&self, alias_or_serial: Option<&str>) -> Option<String> {
        alias_or_serial
            .and_then(|name| {
                self.resolve_alias(name)
                    .and_then(|alias| alias.adb_serial.clone().or(alias.adb_endpoint.clone()))
                    .or_else(|| Some(name.to_string()))
            })
            .or_else(|| self.mirror.device_serial.clone())
            .or_else(|| self.device_serial.clone())
    }

    pub fn resolve_wireless_endpoint(&self, target: Option<&str>) -> Option<String> {
        if let Some(value) = target {
            if value.contains(':') {
                return Some(value.to_string());
            }
            if let Some(alias) = self.resolve_alias(value) {
                if let Some(endpoint) = alias.adb_endpoint.clone() {
                    return Some(endpoint);
                }
            }
            return None;
        }

        self.reconnect
            .recent_endpoints
            .iter()
            .find(|entry| !entry.trim().is_empty())
            .cloned()
    }

    pub fn remember_endpoint(&mut self, endpoint: &str) {
        if endpoint.trim().is_empty() {
            return;
        }

        self.reconnect
            .recent_endpoints
            .retain(|saved| saved != endpoint);
        self.reconnect
            .recent_endpoints
            .insert(0, endpoint.to_string());
        self.reconnect
            .recent_endpoints
            .truncate(self.reconnect.max_history.max(1) as usize);
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            config_version: STABLE_CONFIG_VERSION,
            mirror: MirrorConfig::default(),
            devices: DeviceConfig::default(),
            reconnect: ReconnectConfig::default(),
            gui: GuiConfig::default(),
            device_serial: None,
            scrcpy_args: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MirrorConfig {
    pub device_serial: Option<String>,
    pub profile: String,
    pub window_title_prefix: String,
    pub profiles: BTreeMap<String, ScrcpyProfile>,
    pub hyprland: HyprlandPlacementConfig,
}

impl Default for MirrorConfig {
    fn default() -> Self {
        let mut profiles = BTreeMap::new();
        profiles.insert(DEFAULT_PROFILE_NAME.to_owned(), default_profile());
        profiles.insert(LOW_LATENCY_PROFILE_NAME.to_owned(), low_latency_profile());
        profiles.insert(PRESENTATION_PROFILE_NAME.to_owned(), presentation_profile());

        Self {
            device_serial: None,
            profile: DEFAULT_PROFILE_NAME.to_owned(),
            window_title_prefix: "hypr-phone".to_owned(),
            profiles,
            hyprland: HyprlandPlacementConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DeviceConfig {
    pub aliases: BTreeMap<String, DeviceAlias>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DeviceAlias {
    pub adb_serial: Option<String>,
    pub adb_endpoint: Option<String>,
    pub kdeconnect_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReconnectConfig {
    pub auto_save_history: bool,
    pub max_history: u8,
    pub recent_endpoints: Vec<String>,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            auto_save_history: true,
            max_history: 10,
            recent_endpoints: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GuiConfig {
    pub tray_enabled: bool,
    pub pairing_wizard_enabled: bool,
    pub profile_editor_enabled: bool,
    pub device_cards_enabled: bool,
}

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            tray_enabled: true,
            pairing_wizard_enabled: true,
            profile_editor_enabled: true,
            device_cards_enabled: true,
        }
    }
}

fn default_profile() -> ScrcpyProfile {
    ScrcpyProfile {
        video_bit_rate: Some("8M".to_owned()),
        max_size: Some(1080),
        max_fps: Some(60),
        audio: true,
        ..ScrcpyProfile::default()
    }
}

fn low_latency_profile() -> ScrcpyProfile {
    ScrcpyProfile {
        video_bit_rate: Some("4M".to_owned()),
        max_size: Some(720),
        max_fps: Some(60),
        turn_screen_off: true,
        ..ScrcpyProfile::default()
    }
}

fn presentation_profile() -> ScrcpyProfile {
    ScrcpyProfile {
        video_bit_rate: Some("12M".to_owned()),
        max_size: Some(1080),
        max_fps: Some(30),
        audio: true,
        ..ScrcpyProfile::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_stable_version() {
        let config = Config::default();
        assert_eq!(config.config_version, STABLE_CONFIG_VERSION);
        assert!(config.is_stable_version());
    }

    #[test]
    fn migrates_legacy_version_and_fields() {
        let raw = r#"
config_version = 0
device_serial = "legacy-device"
scrcpy_args = ["--max-fps", "30"]

[mirror]
profile = "default"
"#;

        let parsed: Config = toml::from_str(raw).expect("parse config");
        let normalized = parsed.normalized();

        assert_eq!(normalized.config_version, 1);
        assert_eq!(
            normalized.mirror.device_serial.as_deref(),
            Some("legacy-device")
        );
        let profile = normalized
            .mirror
            .profiles
            .get("default")
            .expect("default profile exists");
        assert!(profile.args.iter().any(|arg| arg == "--max-fps"));
    }

    #[test]
    fn resolves_alias_and_reconnect_endpoint() {
        let mut config = Config::default();
        config.devices.aliases.insert(
            "pixel".to_string(),
            DeviceAlias {
                adb_serial: Some("pixel-serial".to_string()),
                adb_endpoint: Some("192.168.1.22:5555".to_string()),
                kdeconnect_id: Some("kde-pixel".to_string()),
            },
        );

        assert_eq!(
            config.resolve_target_serial(Some("pixel")).as_deref(),
            Some("pixel-serial")
        );
        assert_eq!(
            config.resolve_wireless_endpoint(Some("pixel")).as_deref(),
            Some("192.168.1.22:5555")
        );

        config.remember_endpoint("10.0.0.2:5555");
        config.remember_endpoint("10.0.0.3:5555");
        assert_eq!(
            config.resolve_wireless_endpoint(None).as_deref(),
            Some("10.0.0.3:5555")
        );
        assert_eq!(config.resolve_wireless_endpoint(Some("unknown")), None);
        assert_eq!(
            config.resolve_wireless_endpoint(Some("172.16.0.5:5555"))
                .as_deref(),
            Some("172.16.0.5:5555")
        );
    }

    #[test]
    fn remembers_endpoints_with_dedup_and_history_limit() {
        let mut config = Config::default();
        config.reconnect.max_history = 2;

        config.remember_endpoint("10.0.0.1:5555");
        config.remember_endpoint("10.0.0.2:5555");
        config.remember_endpoint("10.0.0.1:5555");
        config.remember_endpoint("10.0.0.3:5555");

        assert_eq!(
            config.reconnect.recent_endpoints,
            vec!["10.0.0.3:5555".to_string(), "10.0.0.1:5555".to_string()]
        );
    }
}
