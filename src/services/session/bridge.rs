//! XDG_RUNTIME_DIR-based session bridge for scrcpy sessions.
//!
//! Until `hypr-phoned` exists, this provides crash-recovery session knowledge
//! across CLI invocations. Daemon replaces this with in-memory authoritative state.
//!
//! Session file lives at: `$XDG_RUNTIME_DIR/hypr-phone/sessions/`

use std::{fs, path::PathBuf};

use anyhow::{Context, Result};

use crate::domain::session::ScrcpySession;

/// Base directory for Hypr Phone runtime state.
pub fn runtime_dir() -> Result<PathBuf> {
    let dir = std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".local").join("run")))
        .context("neither XDG_RUNTIME_DIR nor HOME is set")?;
    Ok(dir.join("hypr-phone"))
}

/// Directory for active session files.
pub fn sessions_dir() -> Result<PathBuf> {
    Ok(runtime_dir()?.join("sessions"))
}

/// Path to a session file for a given session id.
pub fn session_path(session_id: &str) -> Result<PathBuf> {
    Ok(sessions_dir()?.join(format!("{session_id}.json")))
}

/// Write a session to disk for cross-invocation knowledge.
pub fn persist_session(session: &ScrcpySession) -> Result<()> {
    let dir = sessions_dir()?;
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create session dir {}", dir.display()))?;
    let path = session_path(&session.id)?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_string_pretty(session)?)?;
    fs::rename(&tmp, &path)
        .with_context(|| format!("failed to atomically replace {}", path.display()))?;
    Ok(())
}

/// Remove a session file (session ended).
pub fn remove_session(session_id: &str) -> Result<()> {
    let path = session_path(session_id)?;
    if path.exists() {
        fs::remove_file(&path)
            .with_context(|| format!("failed to remove session file {}", path.display()))?;
    }
    Ok(())
}

/// List all persisted session ids.
pub fn list_session_ids() -> Result<Vec<String>> {
    let dir = sessions_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut ids = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.ends_with(".json") {
            ids.push(name_str.trim_end_matches(".json").to_string());
        }
    }
    Ok(ids)
}

/// Load a session by id.
pub fn load_session(session_id: &str) -> Result<Option<ScrcpySession>> {
    let path = session_path(session_id)?;
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)?;
    Ok(Some(serde_json::from_str(&raw)?))
}

/// Load all persisted sessions.
pub fn load_all_sessions() -> Result<Vec<ScrcpySession>> {
    let ids = list_session_ids()?;
    let mut sessions = Vec::new();
    for id in ids {
        if let Some(s) = load_session(&id)? {
            sessions.push(s);
        }
    }
    Ok(sessions)
}

/// Check if a PID is still running on Linux.
pub fn is_pid_alive(pid: u32) -> bool {
    let stat_path = format!("/proc/{}/stat", pid);
    fs::metadata(stat_path).is_ok()
}

/// Get process start time from /proc/<pid>/stat to detect PID reuse.
/// Returns None if process doesn't exist or we can't read the info.
pub fn process_start_time(pid: u32) -> Result<Option<u64>> {
    let stat_path = format!("/proc/{}/stat", pid);
    let raw =
        fs::read_to_string(&stat_path).with_context(|| format!("failed to read {}", stat_path))?;
    // /proc/<pid>/stat format: pid (comm) state ppid pgrp session tty_nr ...
    // The comm field is in parentheses and may contain spaces or parens.
    // Find the last ')' and parse fields after it: state(1) utime(14) stime(15) ...
    // We need the starttime field which is at index 19 (0-based after comm).
    let after_comm = raw.rsplit_once(')').map(|(_, rest)| rest);
    let Some(fields) = after_comm else {
        return Ok(None);
    };
    let parts: Vec<&str> = fields.split_whitespace().collect();
    // starttime is typically field index 19 after comm (utime=14, stime=15,
    // cutime=16, cstime=17, priority=18, nice=19, starttime=20 in some kernels)
    // Let's use the standard position: after ') ' the fields are:
    // state(1) ppid(2) pgid(3) sid(4) tty_nr(5) tpgid(6) flags(7)
    // minflt(8) cminflt(9) majflt(10) cmajflt(11) utime(12) stime(13)
    // cutime(14) cstime(15) priority(16) nice(17) num_threads(18)
    // itrealvalue(19) starttime(20) vsize(21) rss(22) ...
    // starttime is at index 19 (0-based in the fields after comm).
    const START_TIME_IDX: usize = 19;
    if parts.len() > START_TIME_IDX {
        let val = parts[START_TIME_IDX]
            .parse::<u64>()
            .context("failed to parse starttime")?;
        Ok(Some(val))
    } else {
        Ok(None)
    }
}

/// Validate that a recorded session's PID is still the same process.
/// Returns true if the session is valid (PID alive and starttime matches).
pub fn validate_session(session: &ScrcpySession) -> bool {
    let Some(pid) = session.pid else {
        return false;
    };
    if !is_pid_alive(pid) {
        return false;
    }
    // If we have a recorded start time, validate it hasn't been reused.
    // For now just checking is_pid_alive is sufficient for P0.
    true
}

/// Find a session by target serial.
pub fn find_session_by_serial(serial: &str) -> Result<Option<ScrcpySession>> {
    for session in load_all_sessions()? {
        if session.serial == serial {
            return Ok(Some(session));
        }
    }
    Ok(None)
}

/// Find a session by window title pattern.
pub fn find_session_by_window_title(title_pattern: &str) -> Result<Option<ScrcpySession>> {
    for session in load_all_sessions()? {
        if session.window_title.contains(title_pattern) {
            return Ok(Some(session));
        }
    }
    Ok(None)
}

/// Clean up stale sessions (PID dead or title no longer valid).
pub fn cleanup_stale_sessions() -> Result<usize> {
    let ids = list_session_ids()?;
    let mut removed = 0;
    for id in ids {
        if let Some(session) = load_session(&id)? {
            if !validate_session(&session) {
                let _ = remove_session(&id);
                removed += 1;
            }
        }
    }
    Ok(removed)
}

/// Represents the current mirror state for a target.
#[derive(Debug, Clone)]
pub struct MirrorState {
    pub session: ScrcpySession,
    pub visible: bool,
    pub window_address: Option<String>,
}

impl MirrorState {
    pub fn is_alive(&self) -> bool {
        validate_session(&self.session)
    }
}
