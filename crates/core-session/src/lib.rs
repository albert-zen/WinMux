// core-session: pure session domain model — no PTY, no OS specifics.

use std::fmt;

// ── SessionId ─────────────────────────────────────────────────────────────

/// Opaque, copy-friendly session identifier backed by a u64.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(u64);

impl SessionId {
    #[must_use]
    pub fn new(n: u64) -> Self {
        Self(n)
    }

    #[must_use]
    pub fn value(self) -> u64 {
        self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "session:{}", self.0)
    }
}

// ── SessionSpec ───────────────────────────────────────────────────────────

/// Declarative description of what a session should execute.
/// No PTY or OS handle lives here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSpec {
    pub name: String,
    pub command: String,
    pub working_dir: Option<String>,
}

impl SessionSpec {
    #[must_use]
    pub fn new(name: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            working_dir: None,
        }
    }

    #[must_use]
    pub fn with_working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }
}

// ── SessionStatus ─────────────────────────────────────────────────────────

/// Lifecycle state of a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionStatus {
    /// Newly created, not yet started.
    Created,
    /// Start has been requested; waiting for the process to launch.
    Starting,
    /// Process is live. `pid` is set once the OS assigns one.
    Running { pid: Option<u32> },
    /// Process has terminated.
    Exited { code: Option<i32> },
    /// User has requested a restart; a new start cycle will follow.
    RestartIntent,
}

impl SessionStatus {
    fn name(&self) -> &'static str {
        match self {
            Self::Created => "Created",
            Self::Starting => "Starting",
            Self::Running { .. } => "Running",
            Self::Exited { .. } => "Exited",
            Self::RestartIntent => "RestartIntent",
        }
    }
}

// ── TransitionError ───────────────────────────────────────────────────────

/// Returned when a lifecycle transition is not allowed from the current state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionError {
    pub from: &'static str,
    pub attempted: &'static str,
}

impl fmt::Display for TransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "cannot apply '{}' while in state '{}'",
            self.attempted, self.from
        )
    }
}

impl std::error::Error for TransitionError {}

// ── SessionRecord ─────────────────────────────────────────────────────────

/// Full session record: stable identity + declarative spec + live status.
#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub id: SessionId,
    pub spec: SessionSpec,
    pub status: SessionStatus,
}

impl SessionRecord {
    #[must_use]
    pub fn new(id: SessionId, spec: SessionSpec) -> Self {
        Self {
            id,
            spec,
            status: SessionStatus::Created,
        }
    }

    fn err(&self, attempted: &'static str) -> TransitionError {
        TransitionError {
            from: self.status.name(),
            attempted,
        }
    }

    /// `Created` → `Starting`
    pub fn start(&mut self) -> Result<(), TransitionError> {
        match self.status {
            SessionStatus::Created => {
                self.status = SessionStatus::Starting;
                Ok(())
            }
            _ => Err(self.err("start")),
        }
    }

    /// `Starting` → `Running { pid }`
    pub fn mark_running(&mut self, pid: Option<u32>) -> Result<(), TransitionError> {
        match self.status {
            SessionStatus::Starting => {
                self.status = SessionStatus::Running { pid };
                Ok(())
            }
            _ => Err(self.err("mark_running")),
        }
    }

    /// `Running` | `Starting` → `Exited { code }`
    pub fn exit(&mut self, code: Option<i32>) -> Result<(), TransitionError> {
        match self.status {
            SessionStatus::Running { .. } | SessionStatus::Starting => {
                self.status = SessionStatus::Exited { code };
                Ok(())
            }
            _ => Err(self.err("exit")),
        }
    }

    /// `Exited` → `RestartIntent`
    pub fn restart_intent(&mut self) -> Result<(), TransitionError> {
        match self.status {
            SessionStatus::Exited { .. } => {
                self.status = SessionStatus::RestartIntent;
                Ok(())
            }
            _ => Err(self.err("restart_intent")),
        }
    }

    /// `RestartIntent` → `Starting`
    pub fn restart(&mut self) -> Result<(), TransitionError> {
        match self.status {
            SessionStatus::RestartIntent => {
                self.status = SessionStatus::Starting;
                Ok(())
            }
            _ => Err(self.err("restart")),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record() -> SessionRecord {
        SessionRecord::new(
            SessionId::new(1),
            SessionSpec::new("bash", "bash"),
        )
    }

    // ── SessionSpec ──────────────────────────────────────────────────────

    #[test]
    fn spec_stores_fields() {
        let s = SessionSpec::new("shell", "bash").with_working_dir("/tmp");
        assert_eq!(s.name, "shell");
        assert_eq!(s.command, "bash");
        assert_eq!(s.working_dir.as_deref(), Some("/tmp"));
    }

    #[test]
    fn spec_working_dir_defaults_to_none() {
        let s = SessionSpec::new("x", "cmd");
        assert!(s.working_dir.is_none());
    }

    // ── SessionId ────────────────────────────────────────────────────────

    #[test]
    fn session_id_roundtrips() {
        let id = SessionId::new(42);
        assert_eq!(id.value(), 42);
        assert_eq!(id.to_string(), "session:42");
    }

    // ── Initial state ────────────────────────────────────────────────────

    #[test]
    fn new_record_is_created() {
        let r = make_record();
        assert_eq!(r.status, SessionStatus::Created);
    }

    // ── Happy-path lifecycle ─────────────────────────────────────────────

    #[test]
    fn start_transitions_to_starting() {
        let mut r = make_record();
        r.start().unwrap();
        assert_eq!(r.status, SessionStatus::Starting);
    }

    #[test]
    fn mark_running_carries_pid() {
        let mut r = make_record();
        r.start().unwrap();
        r.mark_running(Some(1234)).unwrap();
        assert_eq!(r.status, SessionStatus::Running { pid: Some(1234) });
    }

    #[test]
    fn mark_running_accepts_no_pid() {
        let mut r = make_record();
        r.start().unwrap();
        r.mark_running(None).unwrap();
        assert_eq!(r.status, SessionStatus::Running { pid: None });
    }

    #[test]
    fn exit_from_running_carries_code() {
        let mut r = make_record();
        r.start().unwrap();
        r.mark_running(Some(99)).unwrap();
        r.exit(Some(0)).unwrap();
        assert_eq!(r.status, SessionStatus::Exited { code: Some(0) });
    }

    #[test]
    fn exit_from_starting_is_allowed() {
        let mut r = make_record();
        r.start().unwrap();
        r.exit(Some(1)).unwrap();
        assert_eq!(r.status, SessionStatus::Exited { code: Some(1) });
    }

    #[test]
    fn restart_intent_follows_exited() {
        let mut r = make_record();
        r.start().unwrap();
        r.mark_running(None).unwrap();
        r.exit(None).unwrap();
        r.restart_intent().unwrap();
        assert_eq!(r.status, SessionStatus::RestartIntent);
    }

    #[test]
    fn restart_moves_back_to_starting() {
        let mut r = make_record();
        r.start().unwrap();
        r.mark_running(None).unwrap();
        r.exit(None).unwrap();
        r.restart_intent().unwrap();

        r.restart().unwrap();

        assert_eq!(r.status, SessionStatus::Starting);
    }

    // ── Guard rails: invalid transitions ────────────────────────────────

    #[test]
    fn start_from_starting_is_rejected() {
        let mut r = make_record();
        r.start().unwrap();
        let err = r.start().unwrap_err();
        assert_eq!(err.from, "Starting");
        assert_eq!(err.attempted, "start");
    }

    #[test]
    fn mark_running_from_created_is_rejected() {
        let mut r = make_record();
        let err = r.mark_running(None).unwrap_err();
        assert_eq!(err.from, "Created");
    }

    #[test]
    fn exit_from_created_is_rejected() {
        let mut r = make_record();
        let err = r.exit(Some(0)).unwrap_err();
        assert_eq!(err.from, "Created");
    }

    #[test]
    fn exit_from_exited_is_rejected() {
        let mut r = make_record();
        r.start().unwrap();
        r.mark_running(None).unwrap();
        r.exit(Some(0)).unwrap();
        let err = r.exit(Some(0)).unwrap_err();
        assert_eq!(err.from, "Exited");
    }

    #[test]
    fn restart_intent_from_running_is_rejected() {
        let mut r = make_record();
        r.start().unwrap();
        r.mark_running(Some(7)).unwrap();
        let err = r.restart_intent().unwrap_err();
        assert_eq!(err.from, "Running");
    }

    #[test]
    fn restart_intent_from_restart_intent_is_rejected() {
        let mut r = make_record();
        r.start().unwrap();
        r.mark_running(None).unwrap();
        r.exit(None).unwrap();
        r.restart_intent().unwrap();
        let err = r.restart_intent().unwrap_err();
        assert_eq!(err.from, "RestartIntent");
    }

    #[test]
    fn restart_from_exited_is_rejected() {
        let mut r = make_record();
        r.start().unwrap();
        r.mark_running(None).unwrap();
        r.exit(None).unwrap();

        let err = r.restart().unwrap_err();

        assert_eq!(err.from, "Exited");
        assert_eq!(err.attempted, "restart");
    }

    // ── TransitionError display ──────────────────────────────────────────

    #[test]
    fn transition_error_displays_readable_message() {
        let e = TransitionError {
            from: "Created",
            attempted: "mark_running",
        };
        let msg = e.to_string();
        assert!(msg.contains("mark_running"), "msg: {msg}");
        assert!(msg.contains("Created"), "msg: {msg}");
    }
}
