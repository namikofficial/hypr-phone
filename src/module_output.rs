use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::adb;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleStatus {
    pub text: String,
    pub tooltip: String,
    pub class: String,
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

pub fn status_from_devices(devices: &[adb::AdbDevice]) -> ModuleStatus {
    devices
        .iter()
        .find(|device| device.is_connected())
        .map(|device| ModuleStatus::connected(device.display_name(), &device.serial))
        .unwrap_or_else(ModuleStatus::disconnected)
}

pub fn status_with_timeout(timeout: Duration) -> ModuleStatus {
    match adb::list_devices_with_timeout(timeout) {
        Ok(devices) => status_from_devices(&devices),
        Err(_) => ModuleStatus::disconnected(),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_connected_status() {
        let devices = vec![adb::AdbDevice {
            serial: "192.168.1.10:5555".to_string(),
            state: "device".to_string(),
            model: Some("Pixel 6".to_string()),
            product: None,
        }];
        let status = status_from_devices(&devices);
        assert_eq!(status.class, "connected");
        assert!(status.tooltip.contains("Pixel 6"));
    }

    #[test]
    fn builds_disconnected_status() {
        let status = status_from_devices(&[]);
        assert_eq!(status.class, "disconnected");
        assert!(status.text.contains("No phone"));
    }
}
