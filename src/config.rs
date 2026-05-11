use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::errors::Result;
use crate::hyprland::HyprlandPlacementConfig;
use crate::scrcpy::{ScrcpyProfile, DEFAULT_PROFILE_NAME};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub mirror: MirrorConfig,
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

    pub fn to_toml_string(&self) -> Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    fn normalized(mut self) -> Self {
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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mirror: MirrorConfig::default(),
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
        profiles.insert(DEFAULT_PROFILE_NAME.to_owned(), ScrcpyProfile::default());

        Self {
            device_serial: None,
            profile: DEFAULT_PROFILE_NAME.to_owned(),
            window_title_prefix: "hypr-phone".to_owned(),
            profiles,
            hyprland: HyprlandPlacementConfig::default(),
        }
    }
}
