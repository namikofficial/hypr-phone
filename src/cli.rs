use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "hypr-phone", version, about = "MVP scaffold for hypr-phone")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Run the default flow.
    Run,
    /// Check required dependencies and environment setup.
    Doctor,
    /// List adb devices.
    Devices {
        /// Print device list as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Pair over Wi-Fi using `adb pair`.
    Pair {
        /// Endpoint in the form `ip:port`.
        endpoint: String,
        /// Pairing code shown on the device.
        pairing_code: String,
    },
    /// Connect over Wi-Fi using `adb connect`.
    Connect {
        /// Endpoint in the form `ip:port`.
        endpoint: String,
    },
    /// Disconnect an adb device by serial.
    Disconnect {
        /// Device serial or endpoint.
        serial: String,
    },
    /// Launch scrcpy mirroring with config profile support.
    Mirror {
        /// Device serial or endpoint (`adb` serial format).
        device: Option<String>,
        /// Scrcpy profile name from config.
        #[arg(long)]
        profile: Option<String>,
    },
    /// Print Waybar/Wayle module JSON status.
    Module,
    /// Open the interactive rofi/wofi menu.
    Menu,
    /// Manage configuration.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ConfigCommand {
    /// Print the default config file path.
    Path,
    /// Initialize a default config file if it does not exist.
    Init,
}
