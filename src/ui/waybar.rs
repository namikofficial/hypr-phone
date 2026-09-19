//! Waybar / status bar output helpers.

use crate::domain::status::{PhoneStatus, WaybarStatus};

pub fn json(status: &PhoneStatus) -> String {
    let wb = WaybarStatus::from(status);
    serde_json::to_string(&wb).unwrap_or_else(|_| {
        r#"{"text":"󰄛 No phone","tooltip":"No Android device connected","class":"disconnected","alt":"disconnected"}"#.to_string()
    })
}
