use std::sync::{Arc, Mutex};

use core_ipc::{ErrorCode, ProtocolError, RequestEnvelope, ResponseEnvelope, ResponseExt};
use core_session::{
    LiveSessionRegistry, PtySessionFactory, SessionHostFactory, SessionRuntimeError, SessionSpec,
    TerminalSize,
};
use core_state::{DesktopBootstrap, WorkspaceRegistry, APP_NAME, STARTER_WORKSPACE_NAME};
use serde::Serialize;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct PaneState {
    pane_id: String,
    session_id: Option<String>,
    status: String,
    output: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceState {
    id: String,
    name: String,
    root_dir: String,
    shell_profile: String,
    focused_pane_id: String,
    panes: Vec<PaneState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopState {
    protocol_version: u32,
    workspaces: Vec<WorkspaceState>,
}

impl<F> RuntimeState<F>
where
    F: SessionHostFactory,
{
    fn new(registry: WorkspaceRegistry, factory: F) -> Self {
        let mut runtime = Self {
            registry,
            sessions: LiveSessionRegistry::new(factory),
        };
        runtime.ensure_sessions_for_all_panes();
        runtime
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

    fn ensure_sessions_for_all_panes(&mut self) {
        let workspace_specs = self
            .registry
            .list()
            .iter()
            .map(|workspace| {
                (
                    workspace.id.clone(),
                    workspace.root_dir.clone(),
                    workspace.shell_profile.clone(),
                    workspace
                        .layout
                        .panes
                        .iter()
                        .map(|pane| pane.pane_id.clone())
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();

        for (workspace_id, root_dir, shell_profile, pane_ids) in workspace_specs {
            for pane_id in pane_ids {
                if self
                    .sessions
                    .session_id_for_pane(&workspace_id, &pane_id)
                    .is_none()
                {
                    let spec = SessionSpec::new(format!("{workspace_id}:{pane_id}"), &shell_profile)
                        .with_working_dir(root_dir.clone());
                    let _ = self.sessions.start(
                        &workspace_id,
                        &pane_id,
                        spec,
                        default_terminal_size(),
                    );
                }
            }
        }
    }

    fn snapshot(&mut self) -> DesktopState {
        let session_snapshots = self.sessions.snapshot();
        let workspaces = self
            .registry
            .list()
            .iter()
            .map(|workspace| {
                let panes = workspace
                    .layout
                    .panes
                    .iter()
                    .map(|pane| {
                        let session_snapshot = session_snapshots
                            .iter()
                            .rev()
                            .find(|session| {
                                session.workspace_id == workspace.id
                                    && session.pane_id == pane.pane_id
                            });

                        PaneState {
                            pane_id: pane.pane_id.clone(),
                            session_id: session_snapshot.map(|session| session.session_id.clone()),
                            status: session_snapshot
                                .map(|session| session.status.clone())
                                .unwrap_or_else(|| "none".to_string()),
                            output: session_snapshot
                                .map(|session| session.output.clone())
                                .unwrap_or_default(),
                        }
                    })
                    .collect();

                WorkspaceState {
                    id: workspace.id.clone(),
                    name: workspace.name.clone(),
                    root_dir: workspace.root_dir.clone(),
                    shell_profile: workspace.shell_profile.clone(),
                    focused_pane_id: workspace.layout.focused_pane_id.clone(),
                    panes,
                }
            })
            .collect();

        DesktopState {
            protocol_version: 1,
            workspaces,
        }
    }
}

#[tauri::command]
fn desktop_bootstrap(state: tauri::State<'_, AppState>) -> DesktopBootstrap {
    state.runtime.lock().unwrap().desktop_bootstrap()
}

#[tauri::command]
fn desktop_state(state: tauri::State<'_, AppState>) -> DesktopState {
    state.runtime.lock().unwrap().snapshot()
}

#[tauri::command]
fn pane_split(
    workspace_id: String,
    pane_id: String,
    direction: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let orientation = normalize_split_direction(&direction)?;
    let request = RequestEnvelope::new(
        "desktop-pane-split",
        "pane.split",
        json!({
            "workspaceId": workspace_id,
            "paneId": pane_id,
            "newPaneId": format!("pane-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |duration| duration.as_millis())),
            "orientation": orientation,
            "ratio": 0.5,
        }),
    );
    let response = dispatch_runtime_request(&request, &mut state.runtime.lock().unwrap());
    if response.is_ok() {
        Ok(())
    } else {
        Err(response
            .error()
            .map(|error| error.message.clone())
            .unwrap_or_else(|| "pane split failed".to_string()))
    }
}

#[tauri::command]
fn session_send_input(
    session_id: String,
    input: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let request = RequestEnvelope::new(
        "desktop-send-input",
        "session.sendInput",
        json!({
            "sessionId": session_id,
            "data": input,
        }),
    );
    let response = dispatch_runtime_request(&request, &mut state.runtime.lock().unwrap());
    if response.is_ok() {
        Ok(())
    } else {
        Err(response
            .error()
            .map(|error| error.message.clone())
            .unwrap_or_else(|| "send input failed".to_string()))
    }
}

#[tauri::command]
fn session_restart(
    session_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let request = RequestEnvelope::new(
        "desktop-restart",
        "session.restart",
        json!({
            "sessionId": session_id,
        }),
    );
    let response = dispatch_runtime_request(&request, &mut state.runtime.lock().unwrap());
    if response.is_ok() {
        Ok(())
    } else {
        Err(response
            .error()
            .map(|error| error.message.clone())
            .unwrap_or_else(|| "restart failed".to_string()))
    }
}

fn default_terminal_size() -> TerminalSize {
    TerminalSize { rows: 24, cols: 80 }
}

fn normalize_split_direction(direction: &str) -> Result<&'static str, String> {
    match direction {
        "horizontal" => Ok("horizontal"),
        "vertical" => Ok("vertical"),
        _ => Err(format!("Unsupported split direction: {direction}")),
    }
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
        SessionRuntimeError::SessionNotExited => ResponseEnvelope::error(
            id,
            ProtocolError::new(ErrorCode::Conflict, "Session has not exited"),
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
        "workspace.create" => handle_workspace_create(request, runtime),
        "pane.split" => handle_pane_split(request, runtime),
        "session.start" => handle_session_start(request, runtime),
        "session.sendInput" => handle_session_send_input(request, runtime),
        "session.resize" => handle_session_resize(request, runtime),
        "session.restart" => handle_session_restart(request, runtime),
        "session.getStatus" => handle_session_get_status(request, runtime),
        "app.getState" => handle_app_get_state(request, runtime),
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

fn handle_workspace_create<F>(
    request: &RequestEnvelope,
    runtime: &mut RuntimeState<F>,
) -> ResponseEnvelope
where
    F: SessionHostFactory,
{
    let response = core_ipc::dispatch(request, &mut runtime.registry);
    if !response.is_ok() {
        return response;
    }

    let workspace_id = response
        .result()
        .and_then(|result| result["workspaceId"].as_str())
        .unwrap_or_default()
        .to_string();
    let Some(workspace) = find_workspace(&runtime.registry, &workspace_id) else {
        return ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(ErrorCode::InternalError, "Created workspace missing from registry"),
        );
    };

    let shell_profile = workspace.shell_profile.clone();
    let root_dir = workspace.root_dir.clone();

    match runtime.sessions.start(
        &workspace_id,
        "pane-1",
        SessionSpec::new(format!("{workspace_id}:pane-1"), &shell_profile).with_working_dir(root_dir),
        default_terminal_size(),
    ) {
        Ok(session_id) => ResponseEnvelope::success(
            request.id(),
            json!({
                "workspaceId": workspace_id,
                "sessionId": session_id,
            }),
        ),
        Err(error) => runtime_error_response(request.id(), error),
    }
}

fn handle_pane_split<F>(
    request: &RequestEnvelope,
    runtime: &mut RuntimeState<F>,
) -> ResponseEnvelope
where
    F: SessionHostFactory,
{
    let response = core_ipc::dispatch(request, &mut runtime.registry);
    if !response.is_ok() {
        return response;
    }

    let workspace_id = request.payload()["workspaceId"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let pane_id = response
        .result()
        .and_then(|result| result["newPaneId"].as_str())
        .unwrap_or_default()
        .to_string();
    let Some(workspace) = find_workspace(&runtime.registry, &workspace_id) else {
        return ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(ErrorCode::InternalError, "Split workspace missing from registry"),
        );
    };

    let shell_profile = workspace.shell_profile.clone();
    let root_dir = workspace.root_dir.clone();

    match runtime.sessions.start(
        &workspace_id,
        &pane_id,
        SessionSpec::new(format!("{workspace_id}:{pane_id}"), &shell_profile)
            .with_working_dir(root_dir),
        default_terminal_size(),
    ) {
        Ok(session_id) => ResponseEnvelope::success(
            request.id(),
            json!({
                "workspaceId": workspace_id,
                "newPaneId": pane_id,
                "sessionId": session_id,
            }),
        ),
        Err(error) => runtime_error_response(request.id(), error),
    }
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

    let spec = SessionSpec::new(format!("{workspace_id}:{pane_id}"), workspace.shell_profile.clone())
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

fn handle_session_restart<F>(
    request: &RequestEnvelope,
    runtime: &mut RuntimeState<F>,
) -> ResponseEnvelope
where
    F: SessionHostFactory,
{
    let session_id = request.payload()["sessionId"].as_str().unwrap_or_default();

    match runtime.sessions.restart(session_id) {
        Ok(restarted_session_id) => match runtime.sessions.get_status(&restarted_session_id) {
            Ok(snapshot) => ResponseEnvelope::success(
                request.id(),
                json!({
                    "sessionId": restarted_session_id,
                    "workspaceId": snapshot.workspace_id,
                    "paneId": snapshot.pane_id,
                }),
            ),
            Err(error) => runtime_error_response(request.id(), error),
        },
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

fn handle_app_get_state<F>(
    request: &RequestEnvelope,
    runtime: &mut RuntimeState<F>,
) -> ResponseEnvelope
where
    F: SessionHostFactory,
{
    ResponseEnvelope::success(
        request.id(),
        serde_json::to_value(runtime.snapshot()).unwrap_or_else(|_| json!({ "workspaces": [] })),
    )
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
        .invoke_handler(tauri::generate_handler![
            desktop_bootstrap,
            desktop_state,
            pane_split,
            session_send_input,
            session_restart
        ])
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
        spawn_commands: Vec<String>,
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

    fn test_runtime() -> RuntimeState<FakeFactory> {
        RuntimeState::new(core_state::starter_workspace_registry(), FakeFactory::default())
    }

    #[test]
    fn runtime_state_new_auto_starts_session_for_starter_pane() {
        let mut runtime = test_runtime();
        let snapshot = runtime.snapshot();

        assert_eq!(runtime.sessions.session_id_for_pane("ws-inbox", "pane-1"), Some("session:1"));
        assert_eq!(snapshot.workspaces[0].panes[0].session_id, Some("session:1".into()));
        assert_eq!(snapshot.workspaces[0].panes[0].status, "running");
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
        let session_id = runtime
            .sessions
            .session_id_for_pane("ws-inbox", "pane-1")
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
        assert!(!first_response.is_ok());
        assert_eq!(first_response.error().unwrap().code, ErrorCode::Conflict);

        let second = handle_runtime_request(start_request, &mut runtime);
        let second_response: ResponseEnvelope = serde_json::from_str(&second).unwrap();
        assert!(!second_response.is_ok());
        assert_eq!(second_response.error().unwrap().code, ErrorCode::Conflict);
        assert_eq!(
            second_response.error().unwrap().message,
            "Pane already has an active session"
        );
    }

    #[test]
    fn handle_runtime_request_session_start_uses_workspace_shell_profile() {
        let mut runtime = test_runtime();
        let starter_session_id = runtime
            .sessions
            .session_id_for_pane("ws-inbox", "pane-1")
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
                        "sessionId": "{starter_session_id}",
                        "data": "exit\r\n"
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
                        "sessionId": "{starter_session_id}"
                    }}
                }}"#
            ),
            &mut runtime,
        );
        let status_response: ResponseEnvelope = serde_json::from_str(&status).unwrap();
        assert_eq!(status_response.result().unwrap()["status"], "exited");

        let restarted = handle_runtime_request(
            r#"{
                "protocolVersion": 1,
                "id": "req-start",
                "type": "command",
                "command": "session.start",
                "payload": {
                    "workspaceId": "ws-inbox",
                    "paneId": "pane-1",
                    "shellProfile": "bash",
                    "cols": 80,
                    "rows": 24
                }
            }"#,
            &mut runtime,
        );
        let restart_response: ResponseEnvelope = serde_json::from_str(&restarted).unwrap();
        assert!(restart_response.is_ok());
        assert_eq!(
            runtime.sessions.snapshot().last().unwrap().command,
            "cmd.exe"
        );
    }

    #[test]
    fn handle_runtime_request_workspace_create_auto_starts_initial_session() {
        let mut runtime = test_runtime();

        let raw = handle_runtime_request(
            r#"{
                "protocolVersion": 1,
                "id": "req-create",
                "type": "command",
                "command": "workspace.create",
                "payload": {
                    "name": "api",
                    "rootDir": "D:\\dev\\api",
                    "shellProfile": "pwsh"
                }
            }"#,
            &mut runtime,
        );

        let response: ResponseEnvelope = serde_json::from_str(&raw).unwrap();
        assert!(response.is_ok());
        let workspace_id = response.result().unwrap()["workspaceId"].as_str().unwrap();
        let session_id = response.result().unwrap()["sessionId"].as_str().unwrap();
        assert_eq!(runtime.sessions.session_id_for_pane(workspace_id, "pane-1"), Some(session_id));
        let workspace = runtime
            .registry
            .list()
            .iter()
            .find(|workspace| workspace.id == workspace_id)
            .unwrap();
        assert_eq!(workspace.shell_profile, "pwsh");
    }

    #[test]
    fn handle_runtime_request_pane_split_auto_starts_session_with_workspace_shell_profile() {
        let mut runtime = test_runtime();

        let raw = handle_runtime_request(
            r#"{
                "protocolVersion": 1,
                "id": "req-split",
                "type": "command",
                "command": "pane.split",
                "payload": {
                    "workspaceId": "ws-inbox",
                    "paneId": "pane-1",
                    "newPaneId": "pane-2",
                    "orientation": "vertical",
                    "ratio": 0.5
                }
            }"#,
            &mut runtime,
        );

        let response: ResponseEnvelope = serde_json::from_str(&raw).unwrap();
        assert!(response.is_ok());
        let session_id = response.result().unwrap()["sessionId"].as_str().unwrap();
        assert_eq!(runtime.sessions.session_id_for_pane("ws-inbox", "pane-2"), Some(session_id));
        assert_eq!(
            runtime
                .sessions
                .snapshot()
                .iter()
                .filter(|session| session.workspace_id == "ws-inbox")
                .count(),
            2
        );
    }

    #[test]
    fn handle_runtime_request_session_restart_returns_new_session_id_for_exited_session() {
        let mut runtime = test_runtime();
        let session_id = runtime
            .sessions
            .session_id_for_pane("ws-inbox", "pane-1")
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
                        "data": "exit\r\n"
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
        assert_eq!(status_response.result().unwrap()["status"], "exited");

        let restarted = handle_runtime_request(
            &format!(
                r#"{{
                    "protocolVersion": 1,
                    "id": "req-restart",
                    "type": "command",
                    "command": "session.restart",
                    "payload": {{
                        "sessionId": "{session_id}"
                    }}
                }}"#
            ),
            &mut runtime,
        );
        let restart_response: ResponseEnvelope = serde_json::from_str(&restarted).unwrap();
        assert!(restart_response.is_ok());
        assert_eq!(restart_response.result().unwrap()["sessionId"], "session:2");
        assert_eq!(restart_response.result().unwrap()["paneId"], "pane-1");
    }

    #[test]
    fn handle_runtime_request_runtime_snapshot_returns_ui_friendly_state() {
        let mut runtime = test_runtime();

        let raw = handle_runtime_request(
            r#"{
                "protocolVersion": 1,
                "id": "req-state",
                "type": "command",
                "command": "app.getState",
                "payload": {}
            }"#,
            &mut runtime,
        );

        let response: ResponseEnvelope = serde_json::from_str(&raw).unwrap();
        assert!(response.is_ok());
        let workspace = &response.result().unwrap()["workspaces"][0];
        assert_eq!(workspace["shellProfile"], "cmd.exe");
        assert_eq!(workspace["panes"][0]["paneId"], "pane-1");
        assert_eq!(workspace["panes"][0]["status"], "running");
        assert_eq!(workspace["panes"][0]["sessionId"], "session:1");
    }

    #[test]
    fn runtime_snapshot_keeps_exited_session_id_available_for_restart() {
        let mut runtime = test_runtime();
        let session_id = runtime
            .sessions
            .session_id_for_pane("ws-inbox", "pane-1")
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
                        "data": "exit\r\n"
                    }}
                }}"#
            ),
            &mut runtime,
        );
        let send_response: ResponseEnvelope = serde_json::from_str(&send).unwrap();
        assert!(send_response.is_ok());

        let snapshot = runtime.snapshot();
        assert_eq!(snapshot.workspaces[0].panes[0].status, "exited");
        assert_eq!(
            snapshot.workspaces[0].panes[0].session_id,
            Some("session:1".to_string())
        );
    }

    #[test]
    fn normalize_split_direction_rejects_unknown_values() {
        let err = normalize_split_direction("diagonal").unwrap_err();

        assert_eq!(err, "Unsupported split direction: diagonal");
    }
}
