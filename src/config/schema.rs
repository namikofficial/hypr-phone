//! Configuration schema (v2).
//!
//! v1 stored devices as endpoints-as-strings. v2 stores device identity
//! (stable id + alias) separately from transport endpoints, with optional
//! adb serial, kdeconnect id, and a "preferred" flag for default selection.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::domain::device::DeviceId;
use crate::services::hyprland::HyprlandPlacementConfig;

pub const CURRENT_CONFIG_VERSION: u32 = 2;
pub const LEGACY_CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub config_version: u32,
    pub mirror: MirrorConfig,
    pub devices: DeviceConfig,
    pub reconnect: ReconnectConfig,
    pub discovery: DiscoveryConfig,
    pub hyprland: HyprlandIntegrationConfig,
    pub ui: UiConfig,
    pub paths: PathsConfig,

    // Legacy fields kept for v1 migration only.
    #[serde(skip_serializing, default)]
    pub legacy_device_serial: Option<String>,
    #[serde(skip_serializing, default)]
    pub legacy_scrcpy_args: Vec<String>,
}

impl Config {
    pub fn default_path() -> Result<PathBuf, anyhow::Error> {
        let config_dir = dirs::config_dir().context("could not resolve config directory")?;
        Ok(config_dir.join("hypr-phone").join("config.toml"))
    }

    pub fn load_from_path(path: &Path) -> Result<Self, anyhow::Error> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file at {}", path.display()))?;
        let parsed: Config = toml::from_str(&raw)
            .with_context(|| format!("failed to parse TOML config at {}", path.display()))?;
        Ok(parsed.migrated())
    }

    pub fn load_default() -> Result<Self, anyhow::Error> {
        let path = Self::default_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        Self::load_from_path(&path)
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), anyhow::Error> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create config directory at {}", path.display())
            })?;
        }
        fs::write(path, self.to_toml_string()?)
            .with_context(|| format!("failed to write config to {}", path.display()))?;
        Ok(())
    }

    pub fn save_default(&self) -> Result<PathBuf, anyhow::Error> {
        let path = Self::default_path()?;
        self.save_to_path(&path)?;
        Ok(path)
    }

    pub fn to_toml_string(&self) -> Result<String, anyhow::Error> {
        Ok(toml::to_string_pretty(self)?)
    }

    pub fn is_current_version(&self) -> bool {
        self.config_version == CURRENT_CONFIG_VERSION
    }

    /// Migrate in-memory from v1 or unversioned to v2.
    pub fn migrated(mut self) -> Self {
        // Bump version (0 → 2, 1 → 2).
        if self.config_version < CURRENT_CONFIG_VERSION {
            // Pull legacy top-level fields into the new mirror config.
            if self.mirror.device_serial.is_none() {
                if let Some(s) = self.legacy_device_serial.take() {
                    self.mirror.device_serial = Some(s);
                }
            }
            if !self.legacy_scrcpy_args.is_empty() {
                self.mirror
                    .profiles
                    .entry(self.mirror.profile.clone())
                    .or_default()
                    .args
                    .extend(self.legacy_scrcpy_args.iter().cloned());
            }
            // v1 devices.aliases was keyed by alias → DeviceAlias (adb_serial,
            // adb_endpoint, kdeconnect_id). v2 stores DeviceEntry with stable
            // device_id. We rekey by alias for now and let discovery match it
            // back once a device is observed.
            for (alias, v1_alias) in std::mem::take(&mut self.devices.v1_aliases) {
                if !self.devices.entries.contains_key(&alias) {
                    self.devices.entries.insert(
                        alias.clone(),
                        DeviceEntry {
                            device_id: None,
                            alias: Some(alias),
                            adb_serial: v1_alias.adb_serial,
                            adb_endpoint: v1_alias.adb_endpoint,
                            kdeconnect_id: v1_alias.kdeconnect_id,
                            notes: None,
                        },
                    );
                }
            }
            // v1 reconnect_history → v2 known_endpoints (additive, both kept).
            for endpoint in &self.reconnect.legacy_recent_endpoints {
                if !self
                    .reconnect
                    .known_endpoints
                    .iter()
                    .any(|e| &e.endpoint == endpoint)
                {
                    self.reconnect.known_endpoints.push(KnownEndpoint {
                        endpoint: endpoint.clone(),
                        last_seen_unix_secs: 0,
                        device_id: None,
                    });
                }
            }
            self.reconnect.legacy_recent_endpoints.clear();

            self.config_version = CURRENT_CONFIG_VERSION;
        }
        self
    }

    pub fn resolve_alias(&self, alias_or_serial: &str) -> Option<&DeviceEntry> {
        self.devices.entries.get(alias_or_serial)
    }

    /// Resolve a target string (alias, serial, or endpoint) to a serial
    /// suitable for `adb -s`.
    pub fn resolve_target_serial(&self, alias_or_serial: Option<&str>) -> Option<String> {
        if let Some(s) = alias_or_serial {
            if let Some(entry) = self.resolve_alias(s) {
                if let Some(serial) = entry.adb_serial.clone() {
                    return Some(serial);
                }
                if let Some(endpoint) = entry.adb_endpoint.clone() {
                    return Some(endpoint);
                }
            }
            return Some(s.to_string());
        }
        self.mirror.device_serial.clone().or_else(|| {
            // If we have exactly one device configured, use it.
            if self.devices.entries.len() == 1 {
                let only = self.devices.entries.values().next()?;
                only.adb_serial
                    .clone()
                    .or_else(|| only.adb_endpoint.clone())
            } else {
                None
            }
        })
    }

    /// Resolve a wireless endpoint from alias or recent history.
    pub fn resolve_wireless_endpoint(&self, target: Option<&str>) -> Option<String> {
        if let Some(value) = target {
            if value.contains(':') {
                return Some(value.to_string());
            }
            if let Some(entry) = self.resolve_alias(value) {
                if let Some(endpoint) = entry.adb_endpoint.clone() {
                    return Some(endpoint);
                }
            }
            return None;
        }
        self.reconnect
            .known_endpoints
            .iter()
            .find(|e| !e.endpoint.trim().is_empty())
            .map(|e| e.endpoint.clone())
    }

    /// Resolve the user-facing default device id (alias) if one is configured.
    pub fn default_alias(&self) -> Option<&str> {
        self.mirror.default_alias.as_deref()
    }

    pub fn remember_endpoint(&mut self, endpoint: &str, device_id: Option<&str>) {
        if endpoint.trim().is_empty() {
            return;
        }
        let max = self.reconnect.max_history.max(1) as usize;
        self.reconnect
            .known_endpoints
            .retain(|e| e.endpoint != endpoint);
        self.reconnect.known_endpoints.insert(
            0,
            KnownEndpoint {
                endpoint: endpoint.to_string(),
                last_seen_unix_secs: now_secs(),
                device_id: device_id.map(str::to_string),
            },
        );
        self.reconnect.known_endpoints.truncate(max);
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl Default for Config {
    fn default() -> Self {
        Self {
            config_version: CURRENT_CONFIG_VERSION,
            mirror: MirrorConfig::default(),
            devices: DeviceConfig::default(),
            reconnect: ReconnectConfig::default(),
            discovery: DiscoveryConfig::default(),
            hyprland: HyprlandIntegrationConfig::default(),
            ui: UiConfig::default(),
            paths: PathsConfig::default(),
            legacy_device_serial: None,
            legacy_scrcpy_args: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MirrorConfig {
    /// Default alias to use when no target is provided.
    pub default_alias: Option<String>,
    pub device_serial: Option<String>,
    pub profile: String,
    pub window_title_prefix: String,
    pub profiles: BTreeMap<String, ScrcpyProfile>,
    pub hyprland: HyprlandPlacementConfig,
    pub toggle: ToggleBehavior,
}

impl Default for MirrorConfig {
    fn default() -> Self {
        let mut profiles = BTreeMap::new();
        profiles.insert("default".to_owned(), default_profile());
        profiles.insert("low_latency".to_owned(), low_latency_profile());
        profiles.insert("presentation".to_owned(), presentation_profile());
        profiles.insert("desk".to_owned(), desk_profile());
        profiles.insert("app".to_owned(), app_profile());
        profiles.insert("record".to_owned(), record_profile());

        Self {
            default_alias: None,
            device_serial: None,
            profile: "default".to_owned(),
            window_title_prefix: "hypr-phone".to_owned(),
            profiles,
            hyprland: HyprlandPlacementConfig::default(),
            toggle: ToggleBehavior::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ToggleBehavior {
    /// Allow toggle to auto-launch mirror when device reachable.
    pub auto_mirror: bool,
    /// Attempt reconnect before launch if device is not currently online.
    pub auto_reconnect: bool,
    /// Maximum reconnect attempts before giving up.
    pub max_reconnect_attempts: u8,
    /// Per-attempt timeout in milliseconds.
    pub reconnect_timeout_ms: u64,
    /// When true, always toggle special workspace visibility (don't re-mirror).
    pub prefer_toggle_visibility: bool,
}

impl Default for ToggleBehavior {
    fn default() -> Self {
        Self {
            auto_mirror: true,
            auto_reconnect: true,
            max_reconnect_attempts: 2,
            reconnect_timeout_ms: 3000,
            prefer_toggle_visibility: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScrcpyProfile {
    pub video_bit_rate: Option<String>,
    pub max_size: Option<u32>,
    pub max_fps: Option<u32>,
    pub audio: bool,
    pub stay_awake: bool,
    pub turn_screen_off: bool,
    pub fullscreen: bool,
    pub window_borderless: bool,
    pub always_on_top: bool,
    /// Virtual display size for app mode (`--new-display=WxH`).
    pub virtual_display_size: Option<String>,
    /// Virtual display density.
    pub virtual_display_dpi: Option<u32>,
    /// Use flex-display for app mode.
    pub flex_display: bool,
    pub args: Vec<String>,
}

impl Default for ScrcpyProfile {
    fn default() -> Self {
        Self {
            video_bit_rate: None,
            max_size: None,
            max_fps: None,
            audio: false,
            stay_awake: true,
            turn_screen_off: false,
            fullscreen: false,
            window_borderless: false,
            always_on_top: false,
            virtual_display_size: None,
            virtual_display_dpi: None,
            flex_display: false,
            args: Vec::new(),
        }
    }
}

impl ScrcpyProfile {
    /// Build the scrcpy command-line arguments.
    ///
    /// `display_size` lets callers override virtual display dimensions for
    /// app-mode launches that target a specific window size.
    pub fn to_args(
        &self,
        serial: Option<&str>,
        window_title: &str,
        display_size: Option<&str>,
    ) -> Vec<String> {
        let mut args = Vec::new();

        if let Some(serial) = serial {
            args.push("--serial".to_owned());
            args.push(serial.to_owned());
        }

        args.push("--window-title".to_owned());
        args.push(window_title.to_owned());

        if let Some(bit_rate) = &self.video_bit_rate {
            args.push("--video-bit-rate".to_owned());
            args.push(bit_rate.clone());
        }
        if let Some(max_size) = self.max_size {
            args.push("--max-size".to_owned());
            args.push(max_size.to_string());
        }
        if let Some(max_fps) = self.max_fps {
            args.push("--max-fps".to_owned());
            args.push(max_fps.to_string());
        }
        if !self.audio {
            args.push("--no-audio".to_owned());
        }
        if self.stay_awake {
            args.push("--stay-awake".to_owned());
        }
        if self.turn_screen_off {
            args.push("--turn-screen-off".to_owned());
        }
        if self.fullscreen {
            args.push("--fullscreen".to_owned());
        }
        if self.window_borderless {
            args.push("--window-borderless".to_owned());
        }
        if self.always_on_top {
            args.push("--always-on-top".to_owned());
        }

        if let Some(size) = display_size.or(self.virtual_display_size.as_deref()) {
            args.push("--new-display".to_owned());
            args.push(size.to_owned());
        } else if self.flex_display {
            args.push("--new-display".to_owned());
        }
        if self.flex_display {
            args.push("--flex-display".to_owned());
        }

        args.extend(self.args.clone());
        args
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DeviceConfig {
    /// Alias → DeviceEntry map. Alias is the user-chosen short name like
    /// "pixel", "work-phone".
    pub entries: BTreeMap<String, DeviceEntry>,

    /// Default device when no alias is given. Resolved against `entries`.
    pub default: Option<String>,

    /// Migration-only: v1 aliases parsed from `[devices.aliases.<name>]`.
    #[serde(rename = "aliases", default, skip_serializing)]
    pub v1_aliases: BTreeMap<String, LegacyDeviceAlias>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DeviceEntry {
    /// Stable device id (filled in after first observation).
    pub device_id: Option<DeviceId>,
    /// User-chosen alias name (mirrors the map key for clarity).
    pub alias: Option<String>,
    /// Most recently seen ADB serial.
    pub adb_serial: Option<String>,
    /// Most recently seen wireless endpoint.
    pub adb_endpoint: Option<String>,
    /// Paired KDE Connect device id.
    pub kdeconnect_id: Option<String>,
    /// Optional user notes.
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LegacyDeviceAlias {
    pub adb_serial: Option<String>,
    pub adb_endpoint: Option<String>,
    pub kdeconnect_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReconnectConfig {
    pub auto_save_history: bool,
    pub max_history: u8,
    pub known_endpoints: Vec<KnownEndpoint>,
    pub auto_reconnect: bool,

    /// Migration-only: alias for v1's `recent_endpoints`.
    #[serde(rename = "recent_endpoints", skip_serializing, default)]
    pub legacy_recent_endpoints: Vec<String>,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            auto_save_history: true,
            max_history: 10,
            known_endpoints: Vec::new(),
            auto_reconnect: true,
            legacy_recent_endpoints: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct KnownEndpoint {
    pub endpoint: String,
    pub last_seen_unix_secs: u64,
    pub device_id: Option<DeviceId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DiscoveryConfig {
    /// Try to discover devices on the network via ADB mDNS.
    pub use_mdns: bool,
    /// mDNS scan timeout in milliseconds.
    pub mdns_timeout_ms: u64,
    /// Periodically run `adb devices` to refresh known devices (used when
    /// no daemon is running).
    pub live_discovery: bool,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            use_mdns: true,
            mdns_timeout_ms: 2000,
            live_discovery: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HyprlandIntegrationConfig {
    pub enabled: bool,
    pub generate_static_rules: bool,
    pub rules_path: Option<PathBuf>,
    pub primary_bindings: HyprlandBindings,
}

impl Default for HyprlandIntegrationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            generate_static_rules: false,
            rules_path: None,
            primary_bindings: HyprlandBindings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HyprlandBindings {
    pub toggle: String,
    pub menu: String,
    pub screenshot: String,
    pub record: String,
    pub apps: String,
}

impl Default for HyprlandBindings {
    fn default() -> Self {
        Self {
            toggle: "SUPER, P".into(),
            menu: "SUPER SHIFT, P".into(),
            screenshot: "SUPER SHIFT, S".into(),
            record: "SUPER SHIFT, R".into(),
            apps: "SUPER, A".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub menu_backend: String,
    pub rofi_theme: Option<String>,
    pub tray_enabled: bool,
    pub show_battery_in_status: bool,
    pub show_kde_in_status: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            menu_backend: "auto".into(),
            rofi_theme: None,
            tray_enabled: false,
            show_battery_in_status: true,
            show_kde_in_status: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PathsConfig {
    pub screenshots_dir: Option<PathBuf>,
    pub recordings_dir: Option<PathBuf>,
}

// Profile factories (mirror what v1 had; new profiles added: desk, app, record).

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

fn desk_profile() -> ScrcpyProfile {
    ScrcpyProfile {
        video_bit_rate: Some("6M".to_owned()),
        max_size: Some(1024),
        max_fps: Some(60),
        turn_screen_off: true,
        always_on_top: false,
        ..ScrcpyProfile::default()
    }
}

fn app_profile() -> ScrcpyProfile {
    ScrcpyProfile {
        video_bit_rate: Some("6M".to_owned()),
        max_size: Some(720),
        max_fps: Some(45),
        virtual_display_size: Some("720x1280".to_owned()),
        virtual_display_dpi: Some(320),
        flex_display: true,
        ..ScrcpyProfile::default()
    }
}

fn record_profile() -> ScrcpyProfile {
    ScrcpyProfile {
        video_bit_rate: Some("8M".to_owned()),
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
    fn defaults_to_current_version() {
        let cfg = Config::default();
        assert_eq!(cfg.config_version, CURRENT_CONFIG_VERSION);
        assert!(cfg.is_current_version());
    }

    #[test]
    fn migrates_legacy_v1_config() {
        let raw = r#"
config_version = 1
legacy_device_serial = "legacy-device"

[mirror]
profile = "default"

[devices.aliases.pixel]
adb_serial = "ABC"
adb_endpoint = "192.168.1.20:5555"
kdeconnect_id = "kde-pixel"

[reconnect]
auto_save_history = true
max_history = 10
recent_endpoints = ["192.168.1.20:5555", "192.168.1.99:5555"]
"#;

        let parsed: Config = toml::from_str(raw).expect("parse v1");
        let migrated = parsed.migrated();
        assert_eq!(migrated.config_version, CURRENT_CONFIG_VERSION);
        assert_eq!(
            migrated.mirror.device_serial.as_deref(),
            Some("legacy-device")
        );
        let entry = migrated
            .devices
            .entries
            .get("pixel")
            .expect("pixel alias migrated");
        assert_eq!(entry.adb_serial.as_deref(), Some("ABC"));
        assert_eq!(entry.adb_endpoint.as_deref(), Some("192.168.1.20:5555"));
        assert_eq!(entry.kdeconnect_id.as_deref(), Some("kde-pixel"));
        assert_eq!(
            migrated.reconnect.known_endpoints[0].endpoint,
            "192.168.1.20:5555"
        );
        assert_eq!(migrated.reconnect.known_endpoints.len(), 2);
    }

    #[test]
    fn resolve_alias_prefers_serial_over_endpoint() {
        let mut cfg = Config::default();
        cfg.devices.entries.insert(
            "pixel".into(),
            DeviceEntry {
                adb_serial: Some("ABC123".into()),
                adb_endpoint: Some("192.168.1.20:5555".into()),
                ..DeviceEntry::default()
            },
        );
        assert_eq!(
            cfg.resolve_target_serial(Some("pixel")).as_deref(),
            Some("ABC123")
        );
        assert_eq!(
            cfg.resolve_wireless_endpoint(Some("pixel")).as_deref(),
            Some("192.168.1.20:5555")
        );
    }

    #[test]
    fn remember_endpoint_dedups_and_limits() {
        let mut cfg = Config::default();
        cfg.reconnect.max_history = 2;
        cfg.remember_endpoint("10.0.0.1:5555", None);
        cfg.remember_endpoint("10.0.0.2:5555", None);
        cfg.remember_endpoint("10.0.0.1:5555", None);
        cfg.remember_endpoint("10.0.0.3:5555", None);
        let eps: Vec<&str> = cfg
            .reconnect
            .known_endpoints
            .iter()
            .map(|e| e.endpoint.as_str())
            .collect();
        assert_eq!(eps, vec!["10.0.0.3:5555", "10.0.0.1:5555"]);
    }

    #[test]
    fn resolve_wireless_endpoint_returns_recent() {
        let mut cfg = Config::default();
        cfg.remember_endpoint("10.0.0.5:5555", None);
        cfg.remember_endpoint("10.0.0.6:5555", None);
        assert_eq!(
            cfg.resolve_wireless_endpoint(None).as_deref(),
            Some("10.0.0.6:5555")
        );
        assert!(cfg
            .resolve_wireless_endpoint(Some("unknown-alias"))
            .is_none());
        assert_eq!(
            cfg.resolve_wireless_endpoint(Some("172.16.0.5:5555"))
                .as_deref(),
            Some("172.16.0.5:5555")
        );
    }

    #[test]
    fn profile_args_include_known_flags() {
        let profile = ScrcpyProfile {
            audio: false,
            stay_awake: true,
            turn_screen_off: true,
            max_fps: Some(30),
            flex_display: true,
            virtual_display_size: Some("720x1280".into()),
            ..ScrcpyProfile::default()
        };
        let args = profile.to_args(Some("serial"), "title", None);
        assert!(args.windows(2).any(|p| p == ["--serial", "serial"]));
        assert!(args.contains(&"--no-audio".to_string()));
        assert!(args.contains(&"--turn-screen-off".to_string()));
        assert!(args.contains(&"--new-display".to_string()));
        assert!(args.contains(&"720x1280".to_string()));
        assert!(args.contains(&"--flex-display".to_string()));
    }

    #[test]
    fn display_size_override_supersedes_profile_default() {
        let profile = ScrcpyProfile::default();
        let args = profile.to_args(None, "title", Some("900x1600"));
        // Override should be present.
        assert!(args.windows(2).any(|p| p == ["--new-display", "900x1600"]));
    }
}
