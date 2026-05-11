use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::adb;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModuleOutput {
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleStatus {
    pub text: String,
    pub tooltip: String,
    pub class: String,
}

impl ModuleOutput {
    pub fn placeholder(module_name: &str) -> Self {
        Self {
            lines: vec![format!("{module_name}: not implemented")],
        }
    }
}

impl ModuleStatus {
    fn connected(name: &str, serial: &str) -> Self {
        Self {
            text: format!("\u{f011c} {name}"),
            tooltip: format!("Android device connected: {name}\\nADB: {serial}"),
            class: "connected".to_string(),
        }
    }

    fn disconnected() -> Self {
        Self {
            text: "\u{f011b} No phone".to_string(),
            tooltip: "No Android device connected".to_string(),
            class: "disconnected".to_string(),
        }
    }
}

pub fn status_with_timeout(timeout: Duration) -> ModuleStatus {
    match adb::list_devices_with_timeout(timeout) {
        Ok(devices) => devices
            .into_iter()
            .find(|device| device.is_connected())
            .map(|device| ModuleStatus::connected(device.display_name(), &device.serial))
            .unwrap_or_else(ModuleStatus::disconnected),
        Err(_) => ModuleStatus::disconnected(),
    }
}

pub fn fast_status() -> ModuleStatus {
    status_with_timeout(Duration::from_millis(750))
}

pub fn status_json_with_timeout(timeout: Duration) -> String {
    let status = status_with_timeout(timeout);
    serde_json::to_string(&status).unwrap_or_else(|_| {
        r#"{"text":"󰄛 No phone","tooltip":"No Android device connected","class":"disconnected"}"#
            .to_string()
    })
}

pub fn fast_status_json() -> String {
    status_json_with_timeout(Duration::from_millis(750))
}
