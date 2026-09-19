//! Scrcpy session tracking and lifecycle.

use serde::{Deserialize, Serialize};

/// A single scrcpy process launched by Hypr Phone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrcpySession {
    /// Unique session id (uuid-ish, just a v4-style random string).
    pub id: String,
    /// Process id, if still running.
    pub pid: Option<u32>,
    /// ADB serial this session targets.
    pub serial: String,
    /// Profile used to launch (e.g. `default`, `low_latency`, `app`).
    pub profile: String,
    /// Window title pattern, used for Hyprland matching.
    pub window_title: String,
    /// Virtual display id (for app mode with `--new-display`).
    pub display_id: Option<u32>,
    /// When the session was launched.
    pub started_at_unix_secs: u64,
    /// If `Some`, an active recording is in progress on this session.
    pub recording: Option<RecordingInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingInfo {
    pub output_path: String,
    pub started_at_unix_secs: u64,
}

impl ScrcpySession {
    pub fn now_secs() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    pub fn is_record_running(&self) -> bool {
        self.recording.is_some()
    }

    pub fn age(&self) -> std::time::Duration {
        let started =
            std::time::UNIX_EPOCH + std::time::Duration::from_secs(self.started_at_unix_secs);
        let now = std::time::SystemTime::now();
        now.duration_since(started).unwrap_or_default()
    }
}

/// In-memory registry of active scrcpy sessions.
#[derive(Debug, Default, Clone)]
pub struct ScrcpySessionManager {
    sessions: Vec<ScrcpySession>,
}

impl ScrcpySessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, session: ScrcpySession) {
        self.sessions.push(session);
    }

    pub fn unregister(&mut self, id: &str) -> Option<ScrcpySession> {
        let pos = self.sessions.iter().position(|s| s.id == id)?;
        Some(self.sessions.remove(pos))
    }

    pub fn find_by_serial(&self, serial: &str) -> Option<&ScrcpySession> {
        self.sessions.iter().find(|s| s.serial == serial)
    }

    pub fn find_by_title(&self, title: &str) -> Option<&ScrcpySession> {
        self.sessions.iter().find(|s| s.window_title == title)
    }

    pub fn sessions(&self) -> &[ScrcpySession] {
        &self.sessions
    }

    pub fn sessions_for_serial(&self, serial: &str) -> Vec<&ScrcpySession> {
        self.sessions
            .iter()
            .filter(|s| s.serial == serial)
            .collect()
    }

    pub fn any_running_for_serial(&self, serial: &str) -> bool {
        self.sessions
            .iter()
            .any(|s| s.serial == serial && s.pid.is_some())
    }

    pub fn active_recording_for_serial(&self, serial: &str) -> Option<&RecordingInfo> {
        self.sessions
            .iter()
            .find(|s| s.serial == serial && s.recording.is_some())
            .and_then(|s| s.recording.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(serial: &str, title: &str) -> ScrcpySession {
        ScrcpySession {
            id: format!("sess-{serial}"),
            pid: Some(1000),
            serial: serial.to_string(),
            profile: "default".to_string(),
            window_title: title.to_string(),
            display_id: None,
            started_at_unix_secs: ScrcpySession::now_secs(),
            recording: None,
        }
    }

    #[test]
    fn register_and_find() {
        let mut mgr = ScrcpySessionManager::new();
        mgr.register(sample("ABC", "hypr-phone:ABC"));
        assert!(mgr.find_by_serial("ABC").is_some());
        assert!(mgr.find_by_title("hypr-phone:ABC").is_some());
    }

    #[test]
    fn unregister_removes_session() {
        let mut mgr = ScrcpySessionManager::new();
        mgr.register(sample("ABC", "hypr-phone:ABC"));
        let removed = mgr.unregister("sess-ABC");
        assert!(removed.is_some());
        assert!(mgr.find_by_serial("ABC").is_none());
    }

    #[test]
    fn any_running_reports_active_sessions() {
        let mut mgr = ScrcpySessionManager::new();
        mgr.register(sample("A", "t:A"));
        assert!(mgr.any_running_for_serial("A"));
        assert!(!mgr.any_running_for_serial("B"));
    }
}
