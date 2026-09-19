//! `hypr-phone record` — start / stop / status for screen recording.

use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;

use crate::config::Config;
use crate::domain::session::{RecordingInfo, ScrcpySession};
use crate::services::{adb, scrcpy};

pub fn run(action: crate::cli::RecordAction) -> Result<()> {
    match action {
        crate::cli::RecordAction::Start { output } => start(output),
        crate::cli::RecordAction::Stop => stop(),
        crate::cli::RecordAction::Status => status(),
    }
}

fn default_output_dir() -> PathBuf {
    dirs::video_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hypr-phone")
}

fn timestamped_path(serial_hint: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    default_output_dir().join(format!("record-{serial_hint}-{ts}.mp4"))
}

pub fn start(output: Option<String>) -> Result<()> {
    let config = Config::load_default()?;
    let discovery = adb::AdbDiscovery::new();
    let devices = discovery.devices_with_timeout(std::time::Duration::from_millis(750))?;
    let device = devices
        .iter()
        .find(|d| d.is_connected())
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("No connected device."))?;
    let serial = device
        .adb_serial
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Device has no ADB serial."))?;

    let out_path = match output {
        Some(p) => PathBuf::from(p),
        None => timestamped_path(&serial.replace(':', "_")),
    };

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let resolved = scrcpy::resolve_mirror_config(
        &config,
        scrcpy::MirrorRequest {
            device_serial: Some(&serial),
            profile: Some("record"),
            ..Default::default()
        },
    )?;
    let args = scrcpy::build_scrcpy_record_args(&resolved, &out_path.to_string_lossy());

    let bin = scrcpy::scrcpy_path()?;
    let mut cmd = Command::new(bin);
    cmd.args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let child = cmd.spawn().map_err(|e| anyhow::anyhow!("failed to spawn scrcpy: {e}"))?;

    // Persist the session to the state file (P2 daemon will replace this).
    let session = scrcpy::describe_session(&resolved, Some(child.id()), None);
    let mut session = session;
    session.recording = Some(RecordingInfo {
        output_path: out_path.to_string_lossy().to_string(),
        started_at_unix_secs: ScrcpySession::now_secs(),
    });
    persist_session(&session)?;

    println!(
        "Recording started (pid {}). Output: {}",
        session.pid.unwrap_or(0),
        out_path.display()
    );
    Ok(())
}

pub fn stop() -> Result<()> {
    let path = session_path()?;
    if !path.exists() {
        println!("No active recording.");
        return Ok(());
    }
    let raw = fs::read_to_string(&path)?;
    let session: ScrcpySession = serde_json::from_str(&raw)?;
    let pid = session.pid.ok_or_else(|| anyhow::anyhow!("session has no pid"))?;
    let out = session
        .recording
        .as_ref()
        .map(|r| r.output_path.clone())
        .unwrap_or_default();

    // Send SIGINT to scrcpy so it can finalize the MP4 cleanly.
    let _ = std::process::Command::new("kill")
        .args(["-INT", &pid.to_string()])
        .status();

    fs::remove_file(&path)?;
    println!("Recording stopped. Output: {out}");
    Ok(())
}

pub fn status() -> Result<()> {
    let path = session_path()?;
    if !path.exists() {
        println!("No active recording.");
        return Ok(());
    }
    let raw = fs::read_to_string(&path)?;
    let session: ScrcpySession = serde_json::from_str(&raw)?;
    let pid = session.pid.unwrap_or(0);
    let out = session
        .recording
        .as_ref()
        .map(|r| r.output_path.clone())
        .unwrap_or_default();
    println!("Recording (pid {pid}) → {out}");
    Ok(())
}

fn session_path() -> Result<PathBuf> {
    let dir = dirs::state_dir()
        .or_else(dirs::config_dir)
        .ok_or_else(|| anyhow::anyhow!("could not resolve state dir"))?;
    Ok(dir.join("hypr-phone").join("active-recording.json"))
}

fn persist_session(session: &ScrcpySession) -> Result<()> {
    let path = session_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, serde_json::to_string_pretty(session)?)?;
    Ok(())
}
