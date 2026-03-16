// core-session: session domain model plus a small live-session runtime.

use core_pty::{PtyError, PtyHost, PtySize};
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
pub enum SessionRuntimeError {
    SessionNotFound,
    PaneAlreadyBound,
    Host(String),
}

impl fmt::Display for SessionRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionRuntimeError::SessionNotFound => write!(f, "session not found"),
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
    fn collected_output(&self) -> Vec<u8>;
}

pub trait SessionHostFactory {
    fn spawn(&self, spec: &SessionSpec, size: TerminalSize)
        -> Result<Box<dyn SessionHost>, String>;
}

struct LiveSession {
    workspace_id: String,
    pane_id: String,
    record: SessionRecord,
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
                host,
            },
        );
        self.pane_bindings.insert(binding, session_id.clone());

        Ok(session_id)
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

        Ok(SessionSnapshot {
            session_id: session.record.id.to_string(),
            workspace_id: session.workspace_id.clone(),
            pane_id: session.pane_id.clone(),
            command: session.record.spec.command.clone(),
            status,
            exit_code,
            output: String::from_utf8_lossy(&session.host.collected_output()).into_owned(),
        })
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

    fn collected_output(&self) -> Vec<u8> {
        self.host.collected_output()
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
        output: Vec<u8>,
        next_wait: Option<bool>,
    }

    #[derive(Clone, Default)]
    struct FakeFactory {
        state: Arc<Mutex<FakeFactoryState>>,
    }

    struct FakeHost {
        state: Arc<Mutex<FakeFactoryState>>,
    }

    impl SessionHost for FakeHost {
        fn write_input(&mut self, data: &[u8]) -> Result<(), String> {
            let mut state = self.state.lock().unwrap();
            state.writes.push(data.to_vec());
            state.output.extend_from_slice(data);
            Ok(())
        }

        fn resize(&self, size: TerminalSize) -> Result<(), String> {
            self.state.lock().unwrap().resizes.push(size);
            Ok(())
        }

        fn try_wait(&mut self) -> Result<Option<bool>, String> {
            Ok(self.state.lock().unwrap().next_wait)
        }

        fn collected_output(&self) -> Vec<u8> {
            self.state.lock().unwrap().output.clone()
        }
    }

    impl SessionHostFactory for FakeFactory {
        fn spawn(
            &self,
            spec: &SessionSpec,
            _size: TerminalSize,
        ) -> Result<Box<dyn SessionHost>, String> {
            self.state
                .lock()
                .unwrap()
                .spawn_commands
                .push(spec.command.clone());
            Ok(Box::new(FakeHost {
                state: Arc::clone(&self.state),
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
