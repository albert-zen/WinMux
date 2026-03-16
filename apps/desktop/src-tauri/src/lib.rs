use std::sync::{Arc, Mutex};

use core_ipc::{ErrorCode, ProtocolError, RequestEnvelope, ResponseEnvelope};
use core_session::{
    LiveSessionRegistry, PtySessionFactory, SessionHostFactory, SessionRuntimeError, SessionSpec,
    TerminalSize,
};
use core_state::{DesktopBootstrap, WorkspaceRegistry, APP_NAME, STARTER_WORKSPACE_NAME};
use serde_json::{Value, json};

pub const PIPE_NAME: &str = r"\\.\pipe\cmux-win-v1";

type AppRuntime = RuntimeState<PtySessionFactory>;

pub struct AppState {
    runtime: Arc<Mutex<AppRuntime>>,
}

struct RuntimeState<F> {
    registry: WorkspaceRegistry,
    sessions: LiveSessionRegistry<F>,
}

impl<F> RuntimeState<F>
where
    F: SessionHostFactory,
{
    fn new(registry: WorkspaceRegistry, factory: F) -> Self {
        Self {
            registry,
            sessions: LiveSessionRegistry::new(factory),
        }
    }

    fn desktop_bootstrap(&self) -> DesktopBootstrap {
        let workspaces = self.registry.summaries();
        let starter = workspaces.first();

        DesktopBootstrap {
            app_name: APP_NAME.to_string(),
            protocol_version: 1,
            starter_workspace_name: STARTER_WORKSPACE_NAME.to_string(),
            starter_pane_count: starter.map_or(0, |workspace| workspace.pane_count),
            starter_split_count: starter.map_or(0, |workspace| workspace.split_count),
            workspaces,
        }
    }
}

#[tauri::command]
fn desktop_bootstrap(state: tauri::State<'_, AppState>) -> DesktopBootstrap {
    state.runtime.lock().unwrap().desktop_bootstrap()
}

fn request_id_from_input(input: &str) -> String {
    serde_json::from_str::<Value>(input)
        .ok()
        .and_then(|value| value.get("id")?.as_str().map(str::to_string))
        .unwrap_or_default()
}

fn serialize_response(response: ResponseEnvelope) -> String {
    serde_json::to_string(&response).unwrap_or_else(|_| {
        r#"{"ok":false,"error":{"code":"internal_error","message":"serialize failed"}}"#.to_string()
    })
}

fn runtime_error_response(id: &str, error: SessionRuntimeError) -> ResponseEnvelope {
    match error {
        SessionRuntimeError::SessionNotFound => ResponseEnvelope::error(
            id,
            ProtocolError::new(ErrorCode::NotFound, "Session not found"),
        ),
        SessionRuntimeError::PaneAlreadyBound => ResponseEnvelope::error(
            id,
            ProtocolError::new(ErrorCode::Conflict, "Pane already has an active session"),
        ),
        SessionRuntimeError::Host(message) => {
            ResponseEnvelope::error(id, ProtocolError::new(ErrorCode::InternalError, message))
        }
    }
}

fn dispatch_runtime_request<F>(
    request: &RequestEnvelope,
    runtime: &mut RuntimeState<F>,
) -> ResponseEnvelope
where
    F: SessionHostFactory,
{
    match request.command() {
        "session.start" => handle_session_start(request, runtime),
        "session.sendInput" => handle_session_send_input(request, runtime),
        "session.resize" => handle_session_resize(request, runtime),
        "session.getStatus" => handle_session_get_status(request, runtime),
        _ => core_ipc::dispatch(request, &mut runtime.registry),
    }
}

fn handle_runtime_request<F>(input: &str, runtime: &mut RuntimeState<F>) -> String
where
    F: SessionHostFactory,
{
    let response = match core_ipc::parse_and_validate_request(input) {
        Ok(request) => dispatch_runtime_request(&request, runtime),
        Err(error) => ResponseEnvelope::error(request_id_from_input(input), error),
    };

    serialize_response(response)
}

fn find_workspace<'a>(
    registry: &'a WorkspaceRegistry,
    workspace_id: &str,
) -> Option<&'a core_state::WorkspaceRecord> {
    registry.list().iter().find(|workspace| workspace.id == workspace_id)
}

fn handle_session_start<F>(
    request: &RequestEnvelope,
    runtime: &mut RuntimeState<F>,
) -> ResponseEnvelope
where
    F: SessionHostFactory,
{
    let payload = request.payload();
    let workspace_id = payload["workspaceId"].as_str().unwrap_or_default();
    let pane_id = payload["paneId"].as_str().unwrap_or_default();
    let shell_profile = payload["shellProfile"].as_str().unwrap_or_default();
    let cols = payload["cols"].as_u64().unwrap_or(80) as u16;
    let rows = payload["rows"].as_u64().unwrap_or(24) as u16;

    let Some(workspace) = find_workspace(&runtime.registry, workspace_id) else {
        return ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(
                ErrorCode::NotFound,
                format!("Workspace not found: {workspace_id}"),
            ),
        );
    };

    if !workspace
        .layout
        .panes
        .iter()
        .any(|pane| pane.pane_id == pane_id)
    {
        return ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(ErrorCode::NotFound, format!("Pane not found: {pane_id}")),
        );
    }

    let spec = SessionSpec::new(format!("{workspace_id}:{pane_id}"), shell_profile)
        .with_working_dir(workspace.root_dir.clone());

    match runtime.sessions.start(
        workspace_id,
        pane_id,
        spec,
        TerminalSize { rows, cols },
    ) {
        Ok(session_id) => ResponseEnvelope::success(
            request.id(),
            json!({
                "sessionId": session_id,
                "workspaceId": workspace_id,
                "paneId": pane_id,
            }),
        ),
        Err(error) => runtime_error_response(request.id(), error),
    }
}

fn handle_session_send_input<F>(
    request: &RequestEnvelope,
    runtime: &mut RuntimeState<F>,
) -> ResponseEnvelope
where
    F: SessionHostFactory,
{
    let payload = request.payload();
    let session_id = payload["sessionId"].as_str().unwrap_or_default();
    let data = payload["data"].as_str().unwrap_or_default();

    match runtime.sessions.send_input(session_id, data.as_bytes()) {
        Ok(()) => ResponseEnvelope::success(
            request.id(),
            json!({
                "delivered": true,
                "sessionId": session_id,
            }),
        ),
        Err(error) => runtime_error_response(request.id(), error),
    }
}

fn handle_session_resize<F>(
    request: &RequestEnvelope,
    runtime: &mut RuntimeState<F>,
) -> ResponseEnvelope
where
    F: SessionHostFactory,
{
    let payload = request.payload();
    let session_id = payload["sessionId"].as_str().unwrap_or_default();
    let cols = payload["cols"].as_u64().unwrap_or(80) as u16;
    let rows = payload["rows"].as_u64().unwrap_or(24) as u16;

    match runtime
        .sessions
        .resize(session_id, TerminalSize { rows, cols })
    {
        Ok(()) => ResponseEnvelope::success(
            request.id(),
            json!({
                "resized": true,
                "sessionId": session_id,
            }),
        ),
        Err(error) => runtime_error_response(request.id(), error),
    }
}

fn handle_session_get_status<F>(
    request: &RequestEnvelope,
    runtime: &mut RuntimeState<F>,
) -> ResponseEnvelope
where
    F: SessionHostFactory,
{
    let session_id = request.payload()["sessionId"].as_str().unwrap_or_default();

    match runtime.sessions.get_status(session_id) {
        Ok(snapshot) => ResponseEnvelope::success(
            request.id(),
            json!({
                "sessionId": snapshot.session_id,
                "workspaceId": snapshot.workspace_id,
                "paneId": snapshot.pane_id,
                "command": snapshot.command,
                "status": snapshot.status,
                "exitCode": snapshot.exit_code,
                "output": snapshot.output,
            }),
        ),
        Err(error) => runtime_error_response(request.id(), error),
    }
}

#[cfg(target_os = "windows")]
fn start_named_pipe_server(runtime: Arc<Mutex<AppRuntime>>) {
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio pipe runtime")
            .block_on(run_pipe_server(runtime));
    });
}

#[cfg(target_os = "windows")]
async fn run_pipe_server(runtime: Arc<Mutex<AppRuntime>>) {
    use tokio::net::windows::named_pipe::ServerOptions;

    let mut first = true;
    loop {
        let server = {
            let mut options = ServerOptions::new();
            if first {
                options.first_pipe_instance(true);
                first = false;
            }
            match options.create(PIPE_NAME) {
                Ok(server) => server,
                Err(error) => {
                    eprintln!("[cmux-pipe] failed to create pipe instance: {error}");
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                    continue;
                }
            }
        };

        match server.connect().await {
            Ok(()) => {
                let runtime = Arc::clone(&runtime);
                tokio::spawn(handle_pipe_connection(server, runtime));
            }
            Err(error) => {
                eprintln!("[cmux-pipe] client connect error: {error}");
            }
        }
    }
}

#[cfg(target_os = "windows")]
async fn handle_pipe_connection(
    server: tokio::net::windows::named_pipe::NamedPipeServer,
    runtime: Arc<Mutex<AppRuntime>>,
) {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let (read_half, mut write_half) = tokio::io::split(server);
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                let response = {
                    let mut runtime = runtime.lock().unwrap();
                    handle_runtime_request(line.trim_end(), &mut runtime)
                };
                if write_half.write_all(response.as_bytes()).await.is_err() {
                    break;
                }
                if write_half.write_all(b"\n").await.is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let runtime = Arc::new(Mutex::new(RuntimeState::new(
        core_state::starter_workspace_registry(),
        PtySessionFactory,
    )));

    #[cfg(target_os = "windows")]
    start_named_pipe_server(Arc::clone(&runtime));

    tauri::Builder::default()
        .manage(AppState { runtime })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![desktop_bootstrap])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_ipc::ResponseExt;
    use core_session::SessionHost;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct FakeFactoryState {
        writes: Vec<Vec<u8>>,
        output: Vec<u8>,
        exited: bool,
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
            if data.windows(4).any(|window| window == b"exit") {
                state.exited = true;
            }
            Ok(())
        }

        fn resize(&self, _size: TerminalSize) -> Result<(), String> {
            Ok(())
        }

        fn try_wait(&mut self) -> Result<Option<bool>, String> {
            let exited = self.state.lock().unwrap().exited;
            Ok(exited.then_some(true))
        }

        fn collected_output(&self) -> Vec<u8> {
            self.state.lock().unwrap().output.clone()
        }
    }

    impl SessionHostFactory for FakeFactory {
        fn spawn(
            &self,
            _spec: &SessionSpec,
            _size: TerminalSize,
        ) -> Result<Box<dyn SessionHost>, String> {
            Ok(Box::new(FakeHost {
                state: Arc::clone(&self.state),
            }))
        }
    }

    fn test_runtime() -> RuntimeState<FakeFactory> {
        RuntimeState::new(core_state::starter_workspace_registry(), FakeFactory::default())
    }

    #[test]
    fn handle_runtime_request_starts_session_for_known_pane() {
        let mut runtime = test_runtime();

        let raw = handle_runtime_request(
            r#"{
                "protocolVersion": 1,
                "id": "req-start",
                "type": "command",
                "command": "session.start",
                "payload": {
                    "workspaceId": "ws-inbox",
                    "paneId": "pane-1",
                    "shellProfile": "cmd.exe",
                    "cols": 80,
                    "rows": 24
                }
            }"#,
            &mut runtime,
        );

        let response: ResponseEnvelope = serde_json::from_str(&raw).unwrap();
        assert!(response.is_ok());
        assert_eq!(response.result().unwrap()["sessionId"], "session:1");
    }

    #[test]
    fn handle_runtime_request_rejects_unknown_panes() {
        let mut runtime = test_runtime();

        let raw = handle_runtime_request(
            r#"{
                "protocolVersion": 1,
                "id": "req-start",
                "type": "command",
                "command": "session.start",
                "payload": {
                    "workspaceId": "ws-inbox",
                    "paneId": "pane-missing",
                    "shellProfile": "cmd.exe",
                    "cols": 80,
                    "rows": 24
                }
            }"#,
            &mut runtime,
        );

        let response: ResponseEnvelope = serde_json::from_str(&raw).unwrap();
        assert!(!response.is_ok());
        assert_eq!(response.error().unwrap().code, ErrorCode::NotFound);
    }

    #[test]
    fn handle_runtime_request_rejects_unknown_workspaces() {
        let mut runtime = test_runtime();

        let raw = handle_runtime_request(
            r#"{
                "protocolVersion": 1,
                "id": "req-start",
                "type": "command",
                "command": "session.start",
                "payload": {
                    "workspaceId": "ws-missing",
                    "paneId": "pane-1",
                    "shellProfile": "cmd.exe",
                    "cols": 80,
                    "rows": 24
                }
            }"#,
            &mut runtime,
        );

        let response: ResponseEnvelope = serde_json::from_str(&raw).unwrap();
        assert!(!response.is_ok());
        assert_eq!(response.error().unwrap().code, ErrorCode::NotFound);
        assert_eq!(
            response.error().unwrap().message,
            "Workspace not found: ws-missing"
        );
    }

    #[test]
    fn handle_runtime_request_roundtrips_session_io_and_status() {
        let mut runtime = test_runtime();
        let start = handle_runtime_request(
            r#"{
                "protocolVersion": 1,
                "id": "req-start",
                "type": "command",
                "command": "session.start",
                "payload": {
                    "workspaceId": "ws-inbox",
                    "paneId": "pane-1",
                    "shellProfile": "cmd.exe",
                    "cols": 80,
                    "rows": 24
                }
            }"#,
            &mut runtime,
        );
        let start_response: ResponseEnvelope = serde_json::from_str(&start).unwrap();
        let session_id = start_response.result().unwrap()["sessionId"]
            .as_str()
            .unwrap()
            .to_string();

        let send = handle_runtime_request(
            &format!(
                r#"{{
                    "protocolVersion": 1,
                    "id": "req-send",
                    "type": "command",
                    "command": "session.sendInput",
                    "payload": {{
                        "sessionId": "{session_id}",
                        "data": "echo hi\r\nexit\r\n"
                    }}
                }}"#
            ),
            &mut runtime,
        );
        let send_response: ResponseEnvelope = serde_json::from_str(&send).unwrap();
        assert!(send_response.is_ok());

        let status = handle_runtime_request(
            &format!(
                r#"{{
                    "protocolVersion": 1,
                    "id": "req-status",
                    "type": "command",
                    "command": "session.getStatus",
                    "payload": {{
                        "sessionId": "{session_id}"
                    }}
                }}"#
            ),
            &mut runtime,
        );
        let status_response: ResponseEnvelope = serde_json::from_str(&status).unwrap();
        assert!(status_response.is_ok());
        let result = status_response.result().unwrap();
        assert_eq!(result["status"], "exited");
        assert!(result["output"].as_str().unwrap().contains("echo hi"));
    }

    #[test]
    fn handle_runtime_request_rejects_second_start_for_active_pane() {
        let mut runtime = test_runtime();
        let start_request = r#"{
            "protocolVersion": 1,
            "id": "req-start",
            "type": "command",
            "command": "session.start",
            "payload": {
                "workspaceId": "ws-inbox",
                "paneId": "pane-1",
                "shellProfile": "cmd.exe",
                "cols": 80,
                "rows": 24
            }
        }"#;

        let first = handle_runtime_request(start_request, &mut runtime);
        let first_response: ResponseEnvelope = serde_json::from_str(&first).unwrap();
        assert!(first_response.is_ok());

        let second = handle_runtime_request(start_request, &mut runtime);
        let second_response: ResponseEnvelope = serde_json::from_str(&second).unwrap();
        assert!(!second_response.is_ok());
        assert_eq!(second_response.error().unwrap().code, ErrorCode::Conflict);
        assert_eq!(
            second_response.error().unwrap().message,
            "Pane already has an active session"
        );
    }
}
