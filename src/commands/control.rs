//! `hypr-phone control` — device control actions.

use anyhow::Result;

use crate::cli::ControlCommand;
use crate::config::Config;
use crate::services::adb;

pub fn run(cmd: ControlCommand) -> Result<()> {
    let config = Config::load_default()?;
    let serial = config.resolve_target_serial(None);
    let key_code = match cmd {
        ControlCommand::Home => "KEYCODE_HOME",
        ControlCommand::Back => "KEYCODE_BACK",
        ControlCommand::Recents => "KEYCODE_APP_SWITCH",
        ControlCommand::Lock => "KEYCODE_POWER",
        ControlCommand::Wake => "KEYCODE_WAKEUP",
        ControlCommand::ScreenOff => "KEYCODE_SLEEP",
        ControlCommand::VolumeUp => "KEYCODE_VOLUME_UP",
        ControlCommand::VolumeDown => "KEYCODE_VOLUME_DOWN",
        ControlCommand::Mute => "KEYCODE_VOLUME_MUTE",
        // For notifications / quick settings, expand the notification shade
        // by sending the appropriate key events. The shade auto-expands
        // on most Android builds.
        ControlCommand::Notifications => "KEYCODE_NOTIFICATION",
        ControlCommand::QuickSettings => "KEYCODE_NOTIFICATION",
        ControlCommand::Rotate => "KEYCODE_ROTATION",
    };
    let out = adb::ops::keyevent(serial.as_deref(), key_code)?;
    if !out.is_empty() {
        println!("{out}");
    }
    Ok(())
}
