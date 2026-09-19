//! `hypr-phone clipboard` — host ↔ phone clipboard sync.

use anyhow::Result;

use crate::cli::ClipboardAction;
use crate::config::Config;
use crate::services::{adb, clipboard};

pub fn run(action: ClipboardAction) -> Result<()> {
    let config = Config::load_default()?;
    let serial = config.resolve_target_serial(None);
    match action {
        ClipboardAction::Send { text } => {
            let content = match text {
                Some(t) => t,
                None => clipboard::read()?,
            };
            let out = adb::ops::send_clipboard(serial.as_deref(), &content)?;
            println!("Sent {} chars.", content.chars().count());
            if !out.is_empty() {
                println!("{out}");
            }
            Ok(())
        }
        ClipboardAction::Receive => {
            let out = adb::ops::receive_clipboard(serial.as_deref())?;
            clipboard::write(&out)?;
            println!("{out}");
            Ok(())
        }
    }
}
