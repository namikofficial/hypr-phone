//! Command-line interface for hypr-phone.

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "hypr-phone",
    version,
    about = "Hyprland-native Android presence layer",
    long_about = "hypr-phone turns your Android phone into a Hyprland-native device.\n\n\
                  Press Super+P to toggle the phone workspace, or run `hypr-phone menu` for\n\
                  a contextual control surface."
)]
pub struct Cli {
    /// Override the config file path.
    #[arg(long, global = true)]
    pub config: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Open the contextual launcher menu (default).
    Menu,

    /// Toggle the phone workspace / mirror.
    Toggle {
        /// Optional scrcpy profile name.
        #[arg(long)]
        profile: Option<String>,
        /// Launch this Android app on toggle (uses app mode profile).
        #[arg(long)]
        app: Option<String>,
        /// Skip launching scrcpy; only toggle the workspace.
        #[arg(long)]
        no_mirror: bool,
    },

    /// Show canonical phone status.
    Status {
        /// Print JSON instead of human output.
        #[arg(long)]
        json: bool,
        /// Output Waybar-shaped JSON.
        #[arg(long, conflicts_with = "json")]
        waybar: bool,
    },

    /// Open an Android app on the phone as a Hyprland window.
    App {
        /// Package name (e.g. `com.whatsapp`). If omitted, show picker.
        package: Option<String>,
    },

    /// Intelligent send: file → push/KDE; URL → open; text → clipboard.
    Send {
        /// File path, URL, or text. Use `-` to read from stdin.
        thing: String,
    },

    /// Start, stop, or check screen recording.
    Record {
        #[command(subcommand)]
        action: RecordAction,
    },

    /// Capture a screenshot.
    Screenshot {
        /// Override the default output path.
        #[arg(long)]
        output: Option<String>,
    },

    /// Device control actions (home, back, volume, etc.).
    Control {
        #[command(subcommand)]
        action: ControlCommand,
    },

    /// Clipboard sync.
    Clipboard {
        #[command(subcommand)]
        action: ClipboardAction,
    },

    /// Manage devices (alias of pair/connect/...).
    Device {
        #[command(subcommand)]
        action: DeviceAction,
    },

    /// Run a thorough environment diagnostic.
    Doctor {
        /// Emit JSON for tooling / Waybar.
        #[arg(long)]
        json: bool,
    },

    /// First-run guided setup.
    Setup {
        /// Apply changes (idempotent) instead of dry-run.
        #[arg(long)]
        apply: bool,
    },

    /// Manage configuration files.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Waybar module JSON (alias for `status --waybar`).
    #[command(name = "module")]
    Module,

    /// Backwards-compatible subcommands kept for v0.3 callers.
    #[command(subcommand)]
    Compat(CompatCommand),
}

#[derive(Debug, Clone, Subcommand)]
pub enum DeviceAction {
    /// List currently visible devices.
    List {
        /// Print JSON.
        #[arg(long)]
        json: bool,
    },
    /// Pair a new wireless device.
    Pair {
        /// Endpoint `ip:port` from "Wireless debugging".
        endpoint: String,
        /// Pairing code shown on the device.
        code: String,
    },
    /// Connect to a wireless endpoint.
    Connect {
        /// Endpoint `ip:port`. If omitted, runs the guided flow.
        endpoint: Option<String>,
    },
    /// Reconnect to the last-known endpoint.
    Reconnect {
        /// Optional alias or endpoint override.
        target: Option<String>,
    },
    /// Disconnect a serial or endpoint.
    Disconnect {
        /// Serial or endpoint to disconnect.
        serial: String,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ConfigAction {
    /// Print the resolved config path.
    Path,
    /// Write a default config if one does not exist.
    Init,
    /// Show config version + migration status.
    Status,
    /// Migrate to the current config version.
    Migrate,
}

#[derive(Debug, Clone, Subcommand)]
pub enum RecordAction {
    /// Start recording. Returns immediately; recording runs in foreground.
    Start {
        /// Output file path.
        #[arg(long)]
        output: Option<String>,
    },
    /// Stop the active recording.
    Stop,
    /// Print recording status.
    Status,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum ControlAction {
    Home,
    Back,
    Recents,
    Lock,
    Wake,
    ScreenOff,
    VolumeUp,
    VolumeDown,
    Mute,
    Notifications,
    QuickSettings,
    Rotate,
}

#[derive(Debug, Clone, Subcommand)]
pub enum ControlCommand {
    Home,
    Back,
    Recents,
    Lock,
    Wake,
    ScreenOff,
    VolumeUp,
    VolumeDown,
    Mute,
    Notifications,
    QuickSettings,
    Rotate,
}

#[derive(Debug, Clone, Subcommand)]
pub enum ClipboardAction {
    /// Send host clipboard (or text) to the phone.
    Send {
        /// Explicit text; reads from wl-paste if omitted.
        #[arg(long)]
        text: Option<String>,
    },
    /// Read phone clipboard into host clipboard.
    Receive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum CompatMediaAction {
    Play,
    Pause,
    PlayPause,
    Stop,
    Next,
    Previous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum MediaAction {
    Play,
    Pause,
    PlayPause,
    Stop,
    Next,
    Previous,
}

/// Backwards-compatible command surface for v0.3 callers.
#[derive(Debug, Clone, Subcommand)]
pub enum CompatCommand {
    /// Legacy `mirror` command.
    Mirror {
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        profile: Option<String>,
    },
    /// Legacy `screenshot` shortcut.
    Screenshot {
        target: Option<String>,
        output: Option<String>,
    },
    /// Legacy `push`.
    Push {
        local_path: String,
        remote_path: String,
        #[arg(long)]
        target: Option<String>,
    },
    /// Legacy `pull`.
    Pull {
        remote_path: String,
        local_path: String,
        #[arg(long)]
        target: Option<String>,
    },
    /// Legacy `install-apk`.
    InstallApk {
        apk_path: String,
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        reinstall: bool,
    },
    /// Legacy `shell`.
    Shell {
        #[arg(long)]
        target: Option<String>,
        #[arg(required = true, trailing_var_arg = true)]
        command: Vec<String>,
    },
    /// Legacy `clipboard` (also reachable via top-level `clipboard`).
    Clipboard {
        #[command(subcommand)]
        action: CompatClipboardAction,
    },
    /// Legacy `kde`.
    Kde {
        #[command(subcommand)]
        action: CompatKdeAction,
    },
    /// Legacy `menu` shortcut.
    Menu,
    /// Legacy `gui status`.
    GuiStatus,
    /// Legacy `gui generate-rules`.
    GuiRules,
    /// Legacy `devices`.
    Devices {
        #[arg(long)]
        json: bool,
    },
    /// Legacy `pair`.
    Pair {
        endpoint: String,
        pairing_code: String,
    },
    /// Legacy `connect`.
    Connect { endpoint: Option<String> },
    /// Legacy `reconnect`.
    Reconnect { target: Option<String> },
    /// Legacy `disconnect`.
    Disconnect { serial: String },
}

#[derive(Debug, Clone, Subcommand)]
pub enum CompatClipboardAction {
    Send {
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        text: Option<String>,
    },
    Receive {
        #[arg(long)]
        target: Option<String>,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum CompatKdeAction {
    Devices,
    Battery {
        #[arg(long)]
        target: Option<String>,
    },
    Ring {
        #[arg(long)]
        target: Option<String>,
    },
    Notify {
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        title: String,
        #[arg(long)]
        body: String,
    },
    Media {
        #[arg(long)]
        target: Option<String>,
        action: CompatMediaAction,
    },
}
