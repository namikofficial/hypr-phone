//! Typed errors used internally where it adds clarity.
//!
//! At the application boundary we use `anyhow::Error` so end users get
//! friendly messages. Internally we surface context with these typed
//! variants via `From` conversions.

use std::fmt;

#[derive(Debug)]
pub enum HyprPhoneError {
    NoDevice,
    DeviceNotConnected { serial: String, state: String },
    AmbiguousDevice(String),
    InvalidEndpoint { endpoint: String, reason: String },
    UnknownProfile(String),
    AppNotInstalled(String),
    HyprlandUnavailable(String),
    CommandFailed(String),
    Config(String),
    Io(std::io::Error),
}

impl fmt::Display for HyprPhoneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HyprPhoneError::NoDevice => write!(f, "no connected device found"),
            HyprPhoneError::DeviceNotConnected { serial, state } => {
                write!(f, "device `{serial}` is not connected (state: {state})")
            }
            HyprPhoneError::AmbiguousDevice(s) => {
                write!(f, "multiple devices connected and no default set: {s}")
            }
            HyprPhoneError::InvalidEndpoint { endpoint, reason } => {
                write!(f, "invalid endpoint `{endpoint}`: {reason}")
            }
            HyprPhoneError::UnknownProfile(s) => {
                write!(f, "scrcpy profile `{s}` is not defined")
            }
            HyprPhoneError::AppNotInstalled(s) => {
                write!(f, "android app `{s}` is not installed on the device")
            }
            HyprPhoneError::HyprlandUnavailable(s) => {
                write!(f, "Hyprland IPC unavailable: {s}")
            }
            HyprPhoneError::CommandFailed(s) => write!(f, "command execution failed: {s}"),
            HyprPhoneError::Config(s) => write!(f, "config error: {s}"),
            HyprPhoneError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for HyprPhoneError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HyprPhoneError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for HyprPhoneError {
    fn from(e: std::io::Error) -> Self {
        HyprPhoneError::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, HyprPhoneError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_includes_context() {
        let err = HyprPhoneError::DeviceNotConnected {
            serial: "abc".into(),
            state: "offline".into(),
        };
        assert!(err.to_string().contains("abc"));
        assert!(err.to_string().contains("offline"));
    }
}
