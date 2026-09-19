//! `hypr-phone send` — intelligent file / URL / text routing.

use std::{
    fs,
    io::{self, Read},
    path::Path,
};

use anyhow::Result;

use crate::config::Config;
use crate::services::adb;
use crate::services::kdeconnect;

pub fn run(thing: String) -> Result<()> {
    let config = Config::load_default()?;
    let serial = config.resolve_target_serial(None);

    // stdin support
    let payload = if thing == "-" {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        buf.trim_end().to_string()
    } else {
        thing
    };

    if is_url(&payload) {
        return send_url(serial.as_deref(), &payload);
    }

    if is_existing_file(&payload) {
        return send_file(serial.as_deref(), Path::new(&payload), &config);
    }

    // Default: text → clipboard send
    send_text(serial.as_deref(), &payload)
}

fn is_url(s: &str) -> bool {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix("http://") {
        return !rest.is_empty();
    }
    if let Some(rest) = s.strip_prefix("https://") {
        return !rest.is_empty();
    }
    false
}

fn is_existing_file(s: &str) -> bool {
    !s.contains('\n') && Path::new(s).is_file()
}

fn send_url(serial: Option<&str>, url: &str) -> Result<()> {
    println!("Opening URL on phone: {url}");
    let out = adb::ops::open_url(serial, url)?;
    if !out.is_empty() {
        println!("{out}");
    }
    Ok(())
}

fn send_file(serial: Option<&str>, path: &Path, config: &Config) -> Result<()> {
    // Prefer KDE Connect if paired.
    if let Some(alias) = config.mirror.default_alias.as_deref().or(Some("default")) {
        if let Some(entry) = config.devices.entries.get(alias) {
            if let Some(kde_id) = &entry.kdeconnect_id {
                if kdeconnect::kdeconnect_path().is_ok() {
                    println!(
                        "Routing through KDE Connect to `{}`.",
                        kde_id
                    );
                    kdeconnect::send_file(kde_id, path)?;
                    return Ok(());
                }
            }
        }
    }
    // Fallback to ADB push.
    let remote = format!("/sdcard/Download/{}", path.file_name().and_then(|n| n.to_str()).unwrap_or("file"));
    let out = adb::ops::push_file(serial, &path.to_string_lossy(), &remote)?;
    println!("Pushed {} → {remote}", path.display());
    println!("{out}");
    Ok(())
}

fn send_text(serial: Option<&str>, text: &str) -> Result<()> {
    let out = adb::ops::send_clipboard(serial, text)?;
    println!("Sent {} characters to phone clipboard.", text.chars().count());
    if !out.is_empty() {
        println!("{out}");
    }
    Ok(())
}
