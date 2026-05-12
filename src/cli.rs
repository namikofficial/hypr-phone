use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "hypr-phone", version, about = "Hyprland Android companion CLI")]
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
        /// Endpoint in the form `ip:port`. If omitted, starts guided pairing + connect flow.
        endpoint: Option<String>,
    },
    /// Reconnect to a known wireless endpoint or alias.
    Reconnect {
        /// Alias or endpoint.
        target: Option<String>,
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
    /// Capture a screenshot from an Android device.
    Screenshot {
        /// Alias or serial.
        target: Option<String>,
        /// Output path (default: ./hypr-phone-<serial>-<timestamp>.png).
        output: Option<String>,
    },
    /// Push a file to device.
    Push {
        local_path: String,
        remote_path: String,
        /// Alias or serial.
        target: Option<String>,
    },
    /// Pull a file from device.
    Pull {
        remote_path: String,
        local_path: String,
        /// Alias or serial.
        target: Option<String>,
    },
    /// Install an APK via adb install.
    InstallApk {
        apk_path: String,
        /// Alias or serial.
        target: Option<String>,
        /// Pass -r to reinstall over existing app.
        #[arg(long)]
        reinstall: bool,
    },
    /// Run adb shell command.
    Shell {
        /// Alias or serial.
        #[arg(long)]
        target: Option<String>,
        /// Shell command to run.
        #[arg(required = true, trailing_var_arg = true)]
        command: Vec<String>,
    },
    /// Clipboard helpers.
    Clipboard {
        #[command(subcommand)]
        command: ClipboardCommand,
    },
    /// KDE Connect bridge helpers.
    Kde {
        #[command(subcommand)]
        command: KdeCommand,
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
    /// GUI/tray roadmap scaffolding commands.
    Gui {
        #[command(subcommand)]
        command: GuiCommand,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ClipboardCommand {
    /// Send host clipboard (or provided text) to phone clipboard.
    Send {
        /// Alias or serial.
        #[arg(long)]
        target: Option<String>,
        /// Text to send. If omitted, reads from wl-paste.
        #[arg(long)]
        text: Option<String>,
    },
    /// Receive phone clipboard and copy to host clipboard.
    Receive {
        /// Alias or serial.
        #[arg(long)]
        target: Option<String>,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum KdeCommand {
    /// List KDE Connect devices.
    Devices,
    /// Query battery status.
    Battery {
        /// KDE Connect device id or configured alias.
        #[arg(long)]
        target: Option<String>,
    },
    /// Ring/find phone.
    Ring {
        /// KDE Connect device id or configured alias.
        #[arg(long)]
        target: Option<String>,
    },
    /// Send notification message.
    Notify {
        /// KDE Connect device id or configured alias.
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        title: String,
        #[arg(long)]
        body: String,
    },
    /// Media control command.
    Media {
        /// KDE Connect device id or configured alias.
        #[arg(long)]
        target: Option<String>,
        action: MediaAction,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum MediaAction {
    Play,
    Pause,
    PlayPause,
    Stop,
    Next,
    Previous,
}

#[derive(Debug, Clone, Subcommand)]
pub enum ConfigCommand {
    /// Print the default config file path.
    Path,
    /// Initialize a default config file if it does not exist.
    Init,
    /// Show config format version and migration status.
    Status,
    /// Rewrite config to the stable v1 format.
    Migrate,
}

#[derive(Debug, Clone, Subcommand)]
pub enum GuiCommand {
    /// Print current GUI/tray scaffold state as JSON.
    Status,
    /// Generate a starter Hyprland rule snippet from config.
    GenerateRules,
}
