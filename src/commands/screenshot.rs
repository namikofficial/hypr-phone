//! `hypr-phone screenshot` — capture screenshot to a sensible location.

use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;

use crate::config::Config;
use crate::services::adb;

pub fn run(output: Option<String>) -> Result<()> {
    let config = Config::load_default()?;
    let serial = config.resolve_target_serial(None);

    let out_path = match output {
        Some(p) => PathBuf::from(p),
        None => default_path(),
    };

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let result = adb::ops::screenshot(serial.as_deref(), &out_path)?;

    if let Some(stdout) = result.strip_prefix("Saved ") {
        let _ = stdout;
    }

    println!("{}", result);

    let _ = Command::new("notify-send")
        .arg("-a")
        .arg("hypr-phone")
        .arg("Screenshot saved")
        .arg(&out_path.display().to_string())
        .status();
    Ok(())
}

pub fn run_compat(target: Option<String>, output: Option<String>) -> Result<()> {
    let config = Config::load_default()?;
    let serial = config.resolve_target_serial(target.as_deref());

    let out_path = match output {
        Some(p) => PathBuf::from(p),
        None => {
            let hint = serial.as_deref().unwrap_or("device").replace(':', "_");
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            PathBuf::from(format!("hypr-phone-{}-{}.png", hint, stamp))
        }
    };

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let result = adb::ops::screenshot(serial.as_deref(), &out_path)?;
    println!("{}", result);
    Ok(())
}

fn default_path() -> PathBuf {
    let base = dirs::picture_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."));
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    base.join("hypr-phone").join(format!("screenshot-{ts}.png"))
}
