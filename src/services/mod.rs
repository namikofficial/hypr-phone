//! External-facing services: ADB discovery + ops, scrcpy, Hyprland,
//! KDE Connect, host clipboard, notifications.

pub mod adb;
pub mod clipboard;
pub mod hyprland;
pub mod kdeconnect;
pub mod notifications;
pub mod scrcpy;
pub mod session;
