//! Desktop notifications via notify-send (libnotify).

use anyhow::Result;

pub fn notify(title: &str, body: &str, icon: Option<&str>) -> Result<()> {
    if which::which("notify-send").is_err() {
        return Ok(()); // silently no-op
    }
    let mut args: Vec<String> = Vec::new();
    if let Some(icon) = icon {
        args.push("-i".into());
        args.push(icon.to_string());
    }
    if !title.is_empty() {
        args.push(title.to_string());
    }
    if !body.is_empty() {
        args.push(body.to_string());
    }
    let _ = std::process::Command::new("notify-send")
        .args(&args)
        .status();
    Ok(())
}
