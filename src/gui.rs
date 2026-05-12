use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuiScaffoldStatus {
    pub config_version: u32,
    pub tray_enabled: bool,
    pub pairing_wizard_enabled: bool,
    pub profile_editor_enabled: bool,
    pub device_cards_enabled: bool,
    pub notes: String,
}

impl GuiScaffoldStatus {
    pub fn from_config(config: &Config) -> Self {
        Self {
            config_version: config.config_version,
            tray_enabled: config.gui.tray_enabled,
            pairing_wizard_enabled: config.gui.pairing_wizard_enabled,
            profile_editor_enabled: config.gui.profile_editor_enabled,
            device_cards_enabled: config.gui.device_cards_enabled,
            notes: "CLI foundation ready; Tauri frontend can bind to this stable config schema."
                .to_string(),
        }
    }
}

pub fn generate_hyprland_rules(config: &Config) -> String {
    format!(
        "# hypr-phone generated rules\nwindowrulev2 = float,title:^(\\Q{}\\E:.*)$\nwindowrulev2 = workspace {},title:^(\\Q{}\\E:.*)$\nwindowrulev2 = size {} {},title:^(\\Q{}\\E:.*)$\nwindowrulev2 = center,title:^(\\Q{}\\E:.*)$",
        config.mirror.window_title_prefix,
        config.mirror.hyprland.workspace,
        config.mirror.window_title_prefix,
        config.mirror.hyprland.width,
        config.mirror.hyprland.height,
        config.mirror.window_title_prefix,
        config.mirror.window_title_prefix
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn generates_rules_with_prefix() {
        let config = Config::default();
        let rules = generate_hyprland_rules(&config);
        assert!(rules.contains("hypr-phone generated rules"));
        assert!(rules.contains("windowrulev2 = workspace special:phone"));
    }

    #[test]
    fn scaffold_status_reflects_config() {
        let mut config = Config::default();
        config.gui.tray_enabled = false;
        let status = GuiScaffoldStatus::from_config(&config);
        assert_eq!(status.config_version, 1);
        assert!(!status.tray_enabled);
        assert!(status.device_cards_enabled);
    }
}
