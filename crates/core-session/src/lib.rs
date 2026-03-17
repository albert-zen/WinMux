// core-session: session domain model plus a small live-session runtime.

pub use core_pty::OutputBuffer;
use core_pty::{DEFAULT_OUTPUT_CAP, PtyError, PtyHost, PtySize};

/// Maximum retained output bytes per session.
pub const SESSION_OUTPUT_CAP_BYTES: usize = DEFAULT_OUTPUT_CAP;
use std::collections::HashMap;
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
    Created,
    Starting,
    Running { pid: Option<u32> },
    Exited { code: Option<i32> },
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

    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Starting => "starting",
            Self::Running { .. } => "running",
            Self::Exited { .. } => "exited",
            Self::RestartIntent => "restart_intent",
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

    pub fn start(&mut self) -> Result<(), TransitionError> {
        match self.status {
            SessionStatus::Created => {
                self.status = SessionStatus::Starting;
                Ok(())
            }
            _ => Err(self.err("start")),
        }
    }

    pub fn mark_running(&mut self, pid: Option<u32>) -> Result<(), TransitionError> {
        match self.status {
            SessionStatus::Starting => {
                self.status = SessionStatus::Running { pid };
                Ok(())
            }
            _ => Err(self.err("mark_running")),
        }
    }

    pub fn exit(&mut self, code: Option<i32>) -> Result<(), TransitionError> {
        match self.status {
            SessionStatus::Running { .. } | SessionStatus::Starting => {
                self.status = SessionStatus::Exited { code };
                Ok(())
            }
            _ => Err(self.err("exit")),
        }
    }

    pub fn restart_intent(&mut self) -> Result<(), TransitionError> {
        match self.status {
            SessionStatus::Exited { .. } => {
                self.status = SessionStatus::RestartIntent;
                Ok(())
            }
            _ => Err(self.err("restart_intent")),
        }
    }

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

// ── Live runtime ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub rows: u16,
    pub cols: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub session_id: String,
    pub workspace_id: String,
    pub pane_id: String,
    pub command: String,
    pub status: String,
    pub exit_code: Option<i32>,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionOutputEvent {
    pub session_id: String,
    pub workspace_id: String,
    pub pane_id: String,
    pub chunk: String,
    pub reset_terminal: bool,
    cursor_start: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionRuntimeError {
    SessionNotFound,
    SessionNotExited,
    PaneAlreadyBound,
    Host(String),
}

impl fmt::Display for SessionRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionRuntimeError::SessionNotFound => write!(f, "session not found"),
            SessionRuntimeError::SessionNotExited => write!(f, "session has not exited"),
            SessionRuntimeError::PaneAlreadyBound => {
                write!(f, "pane already has an active session")
            }
            SessionRuntimeError::Host(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for SessionRuntimeError {}

pub trait SessionHost: Send {
    fn write_input(&mut self, data: &[u8]) -> Result<(), String>;
    fn resize(&self, size: TerminalSize) -> Result<(), String>;
    fn try_wait(&mut self) -> Result<Option<bool>, String>;
    fn collected_output(&self) -> OutputBuffer;
}

pub trait SessionHostFactory {
    fn spawn(&self, spec: &SessionSpec, size: TerminalSize)
        -> Result<Box<dyn SessionHost>, String>;
}

struct LiveSession {
    workspace_id: String,
    pane_id: String,
    record: SessionRecord,
    size: TerminalSize,
    emitted_output_bytes: usize,
    host: Box<dyn SessionHost>,
}

pub struct LiveSessionRegistry<F> {
    factory: F,
    next_id: u64,
    pane_bindings: HashMap<(String, String), String>,
    sessions: HashMap<String, LiveSession>,
}

impl<F> LiveSessionRegistry<F>
where
    F: SessionHostFactory,
{
    fn refresh_exit_state(session: &mut LiveSession) -> Result<(), SessionRuntimeError> {
        if matches!(session.record.status, SessionStatus::Exited { .. }) {
            return Ok(());
        }

        match session.host.try_wait().map_err(SessionRuntimeError::Host)? {
            Some(success) => {
                let exit_code = if success { Some(0) } else { Some(1) };
                let _ = session.record.exit(exit_code);
            }
            None => {}
        }

        Ok(())
    }

    #[must_use]
    pub fn new(factory: F) -> Self {
        Self {
            factory,
            next_id: 1,
            pane_bindings: HashMap::new(),
            sessions: HashMap::new(),
        }
    }

    pub fn start(
        &mut self,
        workspace_id: &str,
        pane_id: &str,
        spec: SessionSpec,
        size: TerminalSize,
    ) -> Result<String, SessionRuntimeError> {
        let binding = (workspace_id.to_string(), pane_id.to_string());
        self.prune_exited_sessions_for_pane(workspace_id, pane_id)?;
        if let Some(existing_session_id) = self.pane_bindings.get(&binding).cloned() {
            if let Some(existing_session) = self.sessions.get_mut(&existing_session_id) {
                Self::refresh_exit_state(existing_session)?;
                if matches!(existing_session.record.status, SessionStatus::Exited { .. }) {
                    self.pane_bindings.remove(&binding);
                } else {
                    return Err(SessionRuntimeError::PaneAlreadyBound);
                }
            } else {
                self.pane_bindings.remove(&binding);
            }
        }

        let id = SessionId::new(self.next_id);
        self.next_id += 1;

        let mut record = SessionRecord::new(id, spec.clone());
        record
            .start()
            .map_err(|err| SessionRuntimeError::Host(err.to_string()))?;

        let host = self
            .factory
            .spawn(&spec, size)
            .map_err(SessionRuntimeError::Host)?;

        record
            .mark_running(None)
            .map_err(|err| SessionRuntimeError::Host(err.to_string()))?;

        let session_id = id.to_string();
        self.sessions.insert(
            session_id.clone(),
            LiveSession {
                workspace_id: workspace_id.to_string(),
                pane_id: pane_id.to_string(),
                record,
                size,
                emitted_output_bytes: 0,
                host,
            },
        );
        self.pane_bindings.insert(binding, session_id.clone());

        Ok(session_id)
    }

    fn prune_exited_sessions_for_pane(
        &mut self,
        workspace_id: &str,
        pane_id: &str,
    ) -> Result<(), SessionRuntimeError> {
        let stale_ids = self
            .sessions
            .iter_mut()
            .filter_map(|(session_id, session)| {
                if session.workspace_id == workspace_id && session.pane_id == pane_id {
                    Some(session_id.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for session_id in stale_ids {
            let should_remove = if let Some(session) = self.sessions.get_mut(&session_id) {
                Self::refresh_exit_state(session)?;
                matches!(session.record.status, SessionStatus::Exited { .. })
            } else {
                false
            };

            if should_remove {
                if self.pane_bindings.get(&(workspace_id.to_string(), pane_id.to_string()))
                    == Some(&session_id)
                {
                    self.pane_bindings
                        .remove(&(workspace_id.to_string(), pane_id.to_string()));
                }
                self.sessions.remove(&session_id);
            }
        }

        Ok(())
    }

    pub fn send_input(
        &mut self,
        session_id: &str,
        data: &[u8],
    ) -> Result<(), SessionRuntimeError> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or(SessionRuntimeError::SessionNotFound)?;
        session
            .host
            .write_input(data)
            .map_err(SessionRuntimeError::Host)
    }

    pub fn resize(
        &mut self,
        session_id: &str,
        size: TerminalSize,
    ) -> Result<(), SessionRuntimeError> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or(SessionRuntimeError::SessionNotFound)?;
        session
            .host
            .resize(size)
            .map_err(SessionRuntimeError::Host)
    }

    pub fn get_status(&mut self, session_id: &str) -> Result<SessionSnapshot, SessionRuntimeError> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or(SessionRuntimeError::SessionNotFound)?;

        Self::refresh_exit_state(session)?;

        if matches!(session.record.status, SessionStatus::Exited { .. }) {
            self.pane_bindings
                .remove(&(session.workspace_id.clone(), session.pane_id.clone()));
        }

        let (status, exit_code) = match &session.record.status {
            SessionStatus::Exited { code } => (session.record.status.label().to_string(), *code),
            _ => (session.record.status.label().to_string(), None),
        };

        let raw_output = session.host.collected_output();
        let capped = if raw_output.bytes.len() > SESSION_OUTPUT_CAP_BYTES {
            &raw_output.bytes[raw_output.bytes.len() - SESSION_OUTPUT_CAP_BYTES..]
        } else {
            &raw_output.bytes
        };

        Ok(SessionSnapshot {
            session_id: session.record.id.to_string(),
            workspace_id: session.workspace_id.clone(),
            pane_id: session.pane_id.clone(),
            command: session.record.spec.command.clone(),
            status,
            exit_code,
            output: String::from_utf8_lossy(capped).into_owned(),
        })
    }

    pub fn restart(&mut self, session_id: &str) -> Result<String, SessionRuntimeError> {
        let (workspace_id, pane_id, spec, size) = {
            let session = self
                .sessions
                .get_mut(session_id)
                .ok_or(SessionRuntimeError::SessionNotFound)?;

            Self::refresh_exit_state(session)?;
            if !matches!(session.record.status, SessionStatus::Exited { .. }) {
                return Err(SessionRuntimeError::SessionNotExited);
            }

            (
                session.workspace_id.clone(),
                session.pane_id.clone(),
                session.record.spec.clone(),
                session.size,
            )
        };

        self.sessions.remove(session_id);
        self.pane_bindings
            .remove(&(workspace_id.clone(), pane_id.clone()));

        self.start(&workspace_id, &pane_id, spec, size)
    }

    pub fn drain_output_events(&mut self) -> Result<Vec<SessionOutputEvent>, SessionRuntimeError> {
        let mut session_ids = self.sessions.keys().cloned().collect::<Vec<_>>();
        session_ids.sort_by_key(|session_id| {
            session_id
                .split(':')
                .nth(1)
                .and_then(|suffix| suffix.parse::<u64>().ok())
                .unwrap_or(0)
        });

        let mut events = Vec::new();
        let mut cursor_updates = Vec::<(String, usize)>::new();

        for session_id in session_ids {
            let Some(session) = self.sessions.get_mut(&session_id) else {
                continue;
            };

            Self::refresh_exit_state(session)?;
            let output = session.host.collected_output();
            let logical_len = output.dropped_prefix_bytes + output.bytes.len();

            if logical_len < session.emitted_output_bytes {
                // Host shrank in an unexpected way; reset cursor.
                cursor_updates.push((session_id, logical_len));
                continue;
            }

            if logical_len == session.emitted_output_bytes {
                continue;
            }

            // How far into output.bytes does the new data start?
            let new_start = if session.emitted_output_bytes >= output.dropped_prefix_bytes {
                session.emitted_output_bytes - output.dropped_prefix_bytes
            } else {
                // Some already-emitted bytes were dropped; start from the beginning
                // of retained bytes (they are after the drop boundary).
                0
            };
            let reset_terminal = session.emitted_output_bytes < output.dropped_prefix_bytes;

            let chunk =
                String::from_utf8_lossy(&output.bytes[new_start..]).into_owned();
            cursor_updates.push((session_id.clone(), logical_len));

            if chunk.is_empty() {
                continue;
            }

            events.push(SessionOutputEvent {
                session_id: session.record.id.to_string(),
                workspace_id: session.workspace_id.clone(),
                pane_id: session.pane_id.clone(),
                chunk,
                reset_terminal,
                cursor_start: session.emitted_output_bytes,
            });
        }

        for (session_id, emitted_output_bytes) in cursor_updates {
            if let Some(session) = self.sessions.get_mut(&session_id) {
                session.emitted_output_bytes = emitted_output_bytes;
            }
        }

        Ok(events)
    }

    pub fn rollback_output_events(&mut self, events: &[SessionOutputEvent]) {
        for event in events {
            if let Some(session) = self.sessions.get_mut(&event.session_id) {
                session.emitted_output_bytes = event.cursor_start;
            }
        }
    }

    pub fn remove_pane(&mut self, workspace_id: &str, pane_id: &str) {
        let binding = (workspace_id.to_string(), pane_id.to_string());
        self.pane_bindings.remove(&binding);

        let session_ids = self
            .sessions
            .iter()
            .filter_map(|(session_id, session)| {
                if session.workspace_id == workspace_id && session.pane_id == pane_id {
                    Some(session_id.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for session_id in session_ids {
            self.sessions.remove(&session_id);
        }
    }

    #[must_use]
    pub fn snapshot(&mut self) -> Vec<SessionSnapshot> {
        let mut snapshots = self
            .sessions
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .filter_map(|session_id| self.get_status(&session_id).ok())
            .collect::<Vec<_>>();
        snapshots.sort_by_key(|snapshot| {
            snapshot
                .session_id
                .split(':')
                .nth(1)
                .and_then(|suffix| suffix.parse::<u64>().ok())
                .unwrap_or(0)
        });
        snapshots
    }

    #[must_use]
    pub fn session_id_for_pane(&self, workspace_id: &str, pane_id: &str) -> Option<&str> {
        self.pane_bindings
            .get(&(workspace_id.to_string(), pane_id.to_string()))
            .map(String::as_str)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PtySessionFactory;

struct PtySessionHost {
    host: PtyHost,
}

impl SessionHost for PtySessionHost {
    fn write_input(&mut self, data: &[u8]) -> Result<(), String> {
        self.host.write_input(data).map_err(|err| err.to_string())
    }

    fn resize(&self, size: TerminalSize) -> Result<(), String> {
        self.host
            .resize(PtySize {
                rows: size.rows,
                cols: size.cols,
            })
            .map_err(|err| err.to_string())
    }

    fn try_wait(&mut self) -> Result<Option<bool>, String> {
        self.host.try_wait().map_err(|err| err.to_string())
    }

    fn collected_output(&self) -> OutputBuffer {
        OutputBuffer {
            bytes: self.host.collected_output(),
            dropped_prefix_bytes: self.host.dropped_prefix_bytes(),
        }
    }
}

impl SessionHostFactory for PtySessionFactory {
    fn spawn(
        &self,
        spec: &SessionSpec,
        size: TerminalSize,
    ) -> Result<Box<dyn SessionHost>, String> {
        let host = PtyHost::spawn_in_dir(
            &spec.command,
            &[],
            PtySize {
                rows: size.rows,
                cols: size.cols,
            },
            spec.working_dir.as_deref(),
        )
        .map_err(|err: PtyError| err.to_string())?;
        Ok(Box::new(PtySessionHost { host }))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    fn make_record() -> SessionRecord {
        SessionRecord::new(SessionId::new(1), SessionSpec::new("bash", "bash"))
    }

    #[derive(Default)]
    struct FakeFactoryState {
        spawn_commands: Vec<String>,
        writes: Vec<Vec<u8>>,
        resizes: Vec<TerminalSize>,
        outputs: Vec<Arc<Mutex<OutputBuffer>>>,
        next_wait: Option<bool>,
    }

    #[derive(Clone, Default)]
    struct FakeFactory {
        state: Arc<Mutex<FakeFactoryState>>,
    }

    struct FakeHost {
        state: Arc<Mutex<FakeFactoryState>>,
        output: Arc<Mutex<OutputBuffer>>,
    }

    impl SessionHost for FakeHost {
        fn write_input(&mut self, data: &[u8]) -> Result<(), String> {
            self.state.lock().unwrap().writes.push(data.to_vec());
            self.output.lock().unwrap().bytes.extend_from_slice(data);
            Ok(())
        }

        fn resize(&self, size: TerminalSize) -> Result<(), String> {
            self.state.lock().unwrap().resizes.push(size);
            Ok(())
        }

        fn try_wait(&mut self) -> Result<Option<bool>, String> {
            Ok(self.state.lock().unwrap().next_wait)
        }

        fn collected_output(&self) -> OutputBuffer {
            let guard = self.output.lock().unwrap();
            OutputBuffer {
                bytes: guard.bytes.clone(),
                dropped_prefix_bytes: guard.dropped_prefix_bytes,
            }
        }
    }

    impl SessionHostFactory for FakeFactory {
        fn spawn(
            &self,
            spec: &SessionSpec,
            _size: TerminalSize,
        ) -> Result<Box<dyn SessionHost>, String> {
            let output = Arc::new(Mutex::new(OutputBuffer::default()));
            let mut state = self.state.lock().unwrap();
            state.outputs.push(Arc::clone(&output));
            state.spawn_commands.push(spec.command.clone());
            Ok(Box::new(FakeHost {
                state: Arc::clone(&self.state),
                output,
            }))
        }
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
    fn exit_from_running_or_starting_is_allowed() {
        let mut running = make_record();
        running.start().unwrap();
        running.mark_running(Some(9)).unwrap();
        running.exit(Some(0)).unwrap();
        assert_eq!(running.status, SessionStatus::Exited { code: Some(0) });

        let mut starting = make_record();
        starting.start().unwrap();
        starting.exit(None).unwrap();
        assert_eq!(starting.status, SessionStatus::Exited { code: None });
    }

    #[test]
    fn restart_flow_roundtrips() {
        let mut r = make_record();
        r.start().unwrap();
        r.mark_running(Some(1)).unwrap();
        r.exit(Some(42)).unwrap();
        r.restart_intent().unwrap();
        r.restart().unwrap();
        assert_eq!(r.status, SessionStatus::Starting);
    }

    // ── Invalid transitions ──────────────────────────────────────────────

    #[test]
    fn invalid_transitions_return_meaningful_errors() {
        let mut r = make_record();

        let err = r.mark_running(None).unwrap_err();
        assert_eq!(err.attempted, "mark_running");
        assert_eq!(err.from, "Created");

        let err = r.exit(None).unwrap_err();
        assert_eq!(err.attempted, "exit");
        assert_eq!(err.from, "Created");

        r.start().unwrap();
        r.mark_running(Some(1)).unwrap();
        let err = r.start().unwrap_err();
        assert_eq!(err.attempted, "start");
        assert_eq!(err.from, "Running");

        let err = r.restart().unwrap_err();
        assert_eq!(err.attempted, "restart");
        assert_eq!(err.from, "Running");
    }

    // ── Live session registry ────────────────────────────────────────────

    #[test]
    fn live_registry_starts_session_and_assigns_id() {
        let factory = FakeFactory::default();
        let mut registry = LiveSessionRegistry::new(factory.clone());
        let session_id = registry
            .start(
                "ws-1",
                "pane-1",
                SessionSpec::new("shell", "pwsh"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap();

        assert_eq!(session_id, "session:1");
        assert_eq!(
            factory.state.lock().unwrap().spawn_commands,
            vec!["pwsh".to_string()]
        );
        assert_eq!(registry.session_id_for_pane("ws-1", "pane-1"), Some("session:1"));
    }

    #[test]
    fn live_registry_rejects_second_session_for_same_pane() {
        let factory = FakeFactory::default();
        let mut registry = LiveSessionRegistry::new(factory);
        registry
            .start(
                "ws-1",
                "pane-1",
                SessionSpec::new("shell", "pwsh"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap();

        let err = registry
            .start(
                "ws-1",
                "pane-1",
                SessionSpec::new("shell", "cmd.exe"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap_err();

        assert_eq!(err, SessionRuntimeError::PaneAlreadyBound);
    }

    #[test]
    fn live_registry_send_input_and_resize_delegate_to_host() {
        let factory = FakeFactory::default();
        let mut registry = LiveSessionRegistry::new(factory.clone());
        let session_id = registry
            .start(
                "ws-1",
                "pane-1",
                SessionSpec::new("shell", "pwsh"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap();

        registry.send_input(&session_id, b"dir\r\n").unwrap();
        registry
            .resize(&session_id, TerminalSize { rows: 40, cols: 120 })
            .unwrap();

        let state = factory.state.lock().unwrap();
        assert_eq!(state.writes, vec![b"dir\r\n".to_vec()]);
        assert_eq!(state.resizes, vec![TerminalSize { rows: 40, cols: 120 }]);
    }

    #[test]
    fn live_registry_get_status_updates_exit_and_returns_output() {
        let factory = FakeFactory::default();
        factory.state.lock().unwrap().next_wait = Some(true);
        let mut registry = LiveSessionRegistry::new(factory.clone());
        let session_id = registry
            .start(
                "ws-1",
                "pane-1",
                SessionSpec::new("shell", "pwsh"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap();
        registry.send_input(&session_id, b"echo hello\r\n").unwrap();

        let snapshot = registry.get_status(&session_id).unwrap();

        assert_eq!(snapshot.workspace_id, "ws-1");
        assert_eq!(snapshot.pane_id, "pane-1");
        assert_eq!(snapshot.command, "pwsh");
        assert_eq!(snapshot.status, "exited");
        assert_eq!(snapshot.exit_code, Some(0));
        assert!(snapshot.output.contains("echo hello"));
    }

    #[test]
    fn live_registry_returns_not_found_for_unknown_session() {
        let factory = FakeFactory::default();
        let mut registry = LiveSessionRegistry::new(factory);

        let err = registry.get_status("session:404").unwrap_err();

        assert_eq!(err, SessionRuntimeError::SessionNotFound);
    }

    #[test]
    fn live_registry_releases_pane_binding_after_exit() {
        let factory = FakeFactory::default();
        factory.state.lock().unwrap().next_wait = Some(true);
        let mut registry = LiveSessionRegistry::new(factory);
        let first_session = registry
            .start(
                "ws-1",
                "pane-1",
                SessionSpec::new("shell", "pwsh"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap();

        let snapshot = registry.get_status(&first_session).unwrap();
        assert_eq!(snapshot.status, "exited");

        let second_session = registry
            .start(
                "ws-1",
                "pane-1",
                SessionSpec::new("shell", "cmd.exe"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap();

        assert_eq!(second_session, "session:2");
    }

    #[test]
    fn live_registry_restart_spawns_new_host_and_returns_new_session_id() {
        let factory = FakeFactory::default();
        factory.state.lock().unwrap().next_wait = Some(true);
        let mut registry = LiveSessionRegistry::new(factory.clone());
        let first_session = registry
            .start(
                "ws-1",
                "pane-1",
                SessionSpec::new("shell", "pwsh"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap();

        let snapshot = registry.get_status(&first_session).unwrap();
        assert_eq!(snapshot.status, "exited");

        let restarted = registry
            .restart(&first_session)
            .expect("exited session should restart");

        assert_eq!(restarted, "session:2");
        assert_eq!(registry.session_id_for_pane("ws-1", "pane-1"), Some("session:2"));
        assert_eq!(
            factory.state.lock().unwrap().spawn_commands,
            vec!["pwsh".to_string(), "pwsh".to_string()]
        );
    }

    #[test]
    fn live_registry_restart_rejects_running_sessions() {
        let factory = FakeFactory::default();
        let mut registry = LiveSessionRegistry::new(factory);
        let session_id = registry
            .start(
                "ws-1",
                "pane-1",
                SessionSpec::new("shell", "pwsh"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap();

        let err = registry.restart(&session_id).unwrap_err();

        assert_eq!(err, SessionRuntimeError::SessionNotExited);
    }

    #[test]
    fn live_registry_start_prunes_exited_sessions_for_the_same_pane() {
        let factory = FakeFactory::default();
        factory.state.lock().unwrap().next_wait = Some(true);
        let mut registry = LiveSessionRegistry::new(factory);
        let first_session = registry
            .start(
                "ws-1",
                "pane-1",
                SessionSpec::new("shell", "pwsh"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap();

        let snapshot = registry.get_status(&first_session).unwrap();
        assert_eq!(snapshot.status, "exited");

        let second_session = registry
            .start(
                "ws-1",
                "pane-1",
                SessionSpec::new("shell", "cmd.exe"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap();

        let all_sessions = registry.snapshot();
        assert_eq!(second_session, "session:2");
        assert_eq!(all_sessions.len(), 1);
        assert_eq!(all_sessions[0].session_id, "session:2");
    }

    #[test]
    fn live_registry_remove_pane_clears_binding_and_sessions() {
        let factory = FakeFactory::default();
        let mut registry = LiveSessionRegistry::new(factory);
        registry
            .start(
                "ws-1",
                "pane-1",
                SessionSpec::new("shell", "pwsh"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap();
        registry
            .start(
                "ws-1",
                "pane-2",
                SessionSpec::new("shell", "cmd.exe"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap();

        registry.remove_pane("ws-1", "pane-2");

        assert_eq!(registry.session_id_for_pane("ws-1", "pane-2"), None);
        let sessions = registry.snapshot();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].pane_id, "pane-1");
    }

    #[test]
    fn live_registry_remove_pane_ignores_missing_bindings() {
        let factory = FakeFactory::default();
        let mut registry = LiveSessionRegistry::new(factory);
        let session_id = registry
            .start(
                "ws-1",
                "pane-1",
                SessionSpec::new("shell", "pwsh"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap();

        registry.remove_pane("ws-1", "pane-missing");

        assert_eq!(registry.session_id_for_pane("ws-1", "pane-1"), Some("session:1"));
        assert_eq!(registry.snapshot()[0].session_id, session_id);
    }

    #[test]
    fn live_registry_drain_output_events_emits_only_new_chunks() {
        let factory = FakeFactory::default();
        let mut registry = LiveSessionRegistry::new(factory);
        let session_id = registry
            .start(
                "ws-1",
                "pane-1",
                SessionSpec::new("shell", "pwsh"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap();

        registry.send_input(&session_id, b"echo one\r\n").unwrap();
        let first = registry.drain_output_events().unwrap();
        let second = registry.drain_output_events().unwrap();

        assert_eq!(
            first,
            vec![SessionOutputEvent {
                session_id: "session:1".to_string(),
                workspace_id: "ws-1".to_string(),
                pane_id: "pane-1".to_string(),
                chunk: "echo one\r\n".to_string(),
                reset_terminal: false,
                cursor_start: 0,
            }]
        );
        assert!(second.is_empty());

        registry.send_input(&session_id, b"echo two\r\n").unwrap();
        let third = registry.drain_output_events().unwrap();
        assert_eq!(third.len(), 1);
        assert_eq!(third[0].chunk, "echo two\r\n");
    }

    #[test]
    fn live_registry_drain_output_events_preserves_session_order() {
        let factory = FakeFactory::default();
        let mut registry = LiveSessionRegistry::new(factory);
        let first_session = registry
            .start(
                "ws-1",
                "pane-1",
                SessionSpec::new("shell", "pwsh"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap();
        let second_session = registry
            .start(
                "ws-1",
                "pane-2",
                SessionSpec::new("shell", "cmd.exe"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap();

        registry.send_input(&second_session, b"second\r\n").unwrap();
        registry.send_input(&first_session, b"first\r\n").unwrap();

        let events = registry.drain_output_events().unwrap();
        let session_ids = events
            .iter()
            .map(|event| event.session_id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(session_ids, vec!["session:1", "session:2"]);
        assert_eq!(events[0].chunk, "first\r\n");
        assert_eq!(events[1].chunk, "second\r\n");
    }

    #[test]
    fn live_registry_drain_output_events_does_not_reemit_after_output_shrinks() {
        let factory = FakeFactory::default();
        let mut registry = LiveSessionRegistry::new(factory.clone());
        let session_id = registry
            .start(
                "ws-1",
                "pane-1",
                SessionSpec::new("shell", "pwsh"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap();

        registry.send_input(&session_id, b"first\r\n").unwrap();
        let first = registry.drain_output_events().unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].chunk, "first\r\n");

        {
            let output = factory.state.lock().unwrap().outputs[0].clone();
            output.lock().unwrap().bytes.clear();
        }

        assert!(registry.drain_output_events().unwrap().is_empty());

        registry.send_input(&session_id, b"second\r\n").unwrap();
        let second = registry.drain_output_events().unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].chunk, "second\r\n");
    }

    #[test]
    fn live_registry_can_rollback_undelivered_output_events() {
        let factory = FakeFactory::default();
        let mut registry = LiveSessionRegistry::new(factory);
        let first_session = registry
            .start(
                "ws-1",
                "pane-1",
                SessionSpec::new("shell", "pwsh"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap();
        let second_session = registry
            .start(
                "ws-1",
                "pane-2",
                SessionSpec::new("shell", "cmd.exe"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap();

        registry.send_input(&first_session, b"first\r\n").unwrap();
        registry.send_input(&second_session, b"second\r\n").unwrap();

        let events = registry.drain_output_events().unwrap();
        registry.rollback_output_events(&events[1..]);

        let replayed = registry.drain_output_events().unwrap();
        assert_eq!(replayed.len(), 1);
        assert_eq!(replayed[0].session_id, second_session);
        assert_eq!(replayed[0].chunk, "second\r\n");
    }

    #[test]
    fn live_registry_get_status_returns_only_capped_recent_output() {
        let factory = FakeFactory::default();
        let mut registry = LiveSessionRegistry::new(factory.clone());
        let session_id = registry
            .start(
                "ws-1",
                "pane-1",
                SessionSpec::new("shell", "pwsh"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap();

        {
            let output = factory.state.lock().unwrap().outputs[0].clone();
            let mut output = output.lock().unwrap();
            output.bytes = vec![b'a'; SESSION_OUTPUT_CAP_BYTES - 4];
            output.bytes.extend_from_slice(b"tail");
        }

        let snapshot = registry.get_status(&session_id).unwrap();

        assert_eq!(snapshot.output.len(), SESSION_OUTPUT_CAP_BYTES);
        assert!(snapshot.output.ends_with("tail"));
    }

    #[test]
    fn live_registry_drain_output_events_preserves_new_bytes_after_prefix_trim() {
        let factory = FakeFactory::default();
        let mut registry = LiveSessionRegistry::new(factory.clone());
        let session_id = registry
            .start(
                "ws-1",
                "pane-1",
                SessionSpec::new("shell", "pwsh"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap();

        registry.send_input(&session_id, b"abcdef").unwrap();
        let first = registry.drain_output_events().unwrap();
        assert_eq!(first[0].chunk, "abcdef");

        {
            let output = factory.state.lock().unwrap().outputs[0].clone();
            let mut output = output.lock().unwrap();
            output.bytes.drain(0..4);
            output.dropped_prefix_bytes += 4;
            output.bytes.extend_from_slice(b"ghij");
        }

        let second = registry.drain_output_events().unwrap();

        assert_eq!(second.len(), 1);
        assert_eq!(second[0].chunk, "ghij");
        assert!(!second[0].reset_terminal);
    }

    #[test]
    fn live_registry_drain_output_events_requests_terminal_reset_after_gap() {
        let factory = FakeFactory::default();
        let mut registry = LiveSessionRegistry::new(factory.clone());
        let _session_id = registry
            .start(
                "ws-1",
                "pane-1",
                SessionSpec::new("shell", "pwsh"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap();

        {
            let output = factory.state.lock().unwrap().outputs[0].clone();
            let mut output = output.lock().unwrap();
            output.bytes.extend_from_slice(b"abcdefghij");
            output.dropped_prefix_bytes = 6;
        }

        let events = registry.drain_output_events().unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].chunk, "abcdefghij");
        assert!(events[0].reset_terminal);
    }

    #[test]
    fn live_registry_drain_output_events_can_request_multiple_terminal_resets() {
        let factory = FakeFactory::default();
        let mut registry = LiveSessionRegistry::new(factory.clone());
        let _session_id = registry
            .start(
                "ws-1",
                "pane-1",
                SessionSpec::new("shell", "pwsh"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .unwrap();

        {
            let output = factory.state.lock().unwrap().outputs[0].clone();
            let mut output = output.lock().unwrap();
            output.bytes.extend_from_slice(b"first-reset");
            output.dropped_prefix_bytes = 4;
        }
        let first = registry.drain_output_events().unwrap();
        assert_eq!(first.len(), 1);
        assert!(first[0].reset_terminal);

        {
            let output = factory.state.lock().unwrap().outputs[0].clone();
            let mut output = output.lock().unwrap();
            output.bytes.clear();
            output.bytes.extend_from_slice(b"second-reset");
            output.dropped_prefix_bytes = 32;
        }
        let second = registry.drain_output_events().unwrap();

        assert_eq!(second.len(), 1);
        assert!(second[0].reset_terminal);
        assert_eq!(second[0].chunk, "second-reset");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn pty_session_factory_runs_cmd_and_captures_output() {
        let mut registry = LiveSessionRegistry::new(PtySessionFactory);
        let session_id = registry
            .start(
                "ws-live",
                "pane-1",
                SessionSpec::new("shell", "cmd.exe"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .expect("cmd.exe should spawn");

        registry
            .send_input(&session_id, b"echo hello_live\r\nexit\r\n")
            .expect("input should write");

        let deadline = Instant::now() + Duration::from_secs(6);
        loop {
            let snapshot = registry.get_status(&session_id).expect("status should resolve");
            if snapshot.output.contains("hello_live") && snapshot.status == "exited" {
                break;
            }

            assert!(
                Instant::now() < deadline,
                "expected cmd.exe output and exit, got status={} output={:?}",
                snapshot.status,
                snapshot.output
            );
            thread::sleep(Duration::from_millis(50));
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn pty_session_factory_honors_working_directory() {
        let working_dir = std::env::temp_dir();
        let expected_dir = working_dir
            .to_string_lossy()
            .trim_end_matches(['\\', '/'])
            .to_string();
        let mut registry = LiveSessionRegistry::new(PtySessionFactory);
        let session_id = registry
            .start(
                "ws-live",
                "pane-1",
                SessionSpec::new("shell", "cmd.exe")
                    .with_working_dir(working_dir.to_string_lossy().to_string()),
                TerminalSize { rows: 24, cols: 80 },
            )
            .expect("cmd.exe should spawn");

        registry
            .send_input(&session_id, b"cd\r\nexit\r\n")
            .expect("input should write");

        let deadline = Instant::now() + Duration::from_secs(6);
        loop {
            let snapshot = registry.get_status(&session_id).expect("status should resolve");
            if snapshot.output.contains(&expected_dir) && snapshot.status == "exited" {
                break;
            }

            assert!(
                Instant::now() < deadline,
                "expected working directory {:?} in output, got status={} output={:?}",
                expected_dir,
                snapshot.status,
                snapshot.output
            );
            thread::sleep(Duration::from_millis(50));
        }
    }
}
