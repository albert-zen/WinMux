use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashSet;
use std::path::PathBuf;

use core_ipc::{ErrorCode, ProtocolError, RequestEnvelope, ResponseEnvelope, ResponseExt};
use core_session::{
    LiveSessionRegistry, PtySessionFactory, SessionHostFactory, SessionRuntimeError, SessionSpec,
    TerminalSize,
};
use core_state::{DesktopBootstrap, WorkspaceRegistry, APP_NAME, STARTER_WORKSPACE_NAME};
use serde::Serialize;
use serde_json::{Value, json};
use tauri::Emitter;

pub const PIPE_NAME: &str = r"\\.\pipe\cmux-win-v1";
pub const SESSION_OUTPUT_EVENT_NAME: &str = "session.output";

type AppRuntime = RuntimeState<PtySessionFactory>;

static NEXT_PANE_ID_SUFFIX: AtomicU64 = AtomicU64::new(1);

pub struct AppState {
    runtime: Arc<Mutex<AppRuntime>>,
}

struct RuntimeState<F> {
    registry: WorkspaceRegistry,
    sessions: LiveSessionRegistry<F>,
    state_path: Option<PathBuf>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionOutputPayload {
    workspace_id: String,
    pane_id: String,
    session_id: String,
    chunk: String,
    reset_terminal: bool,
}

impl<F> RuntimeState<F>
where
    F: SessionHostFactory,
{
    #[cfg(test)]
    fn new(registry: WorkspaceRegistry, factory: F) -> Self {
        let mut runtime = Self {
            registry,
            sessions: LiveSessionRegistry::new(factory),
            state_path: None,
        };
        runtime.ensure_sessions_for_all_panes();
        runtime
    }

    fn new_with_state_path(
        registry: WorkspaceRegistry,
        factory: F,
        state_path: PathBuf,
    ) -> Self {
        let mut runtime = Self {
            registry,
            sessions: LiveSessionRegistry::new(factory),
            state_path: Some(state_path),
        };
        runtime.ensure_sessions_for_all_panes();
        runtime
    }

    fn save_registry(&self) {
        if let Some(ref path) = self.state_path {
            if let Ok(json) = self.registry.to_persisted_json() {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let temp_path = path.with_extension("json.tmp");
                if std::fs::write(&temp_path, json).is_ok() {
                    let rename_result = std::fs::rename(&temp_path, path).or_else(|_| {
                        let _ = std::fs::remove_file(path);
                        std::fs::rename(&temp_path, path)
                    });
                    if rename_result.is_err() {
                        let _ = std::fs::remove_file(&temp_path);
                    }
                }
            }
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

    #[cfg(test)]
    fn drain_output_events(&mut self) -> Result<Vec<SessionOutputPayload>, SessionRuntimeError> {
        let events = self.sessions.drain_output_events()?;
        Ok(events
            .into_iter()
            .map(|event| SessionOutputPayload {
                workspace_id: event.workspace_id,
                pane_id: event.pane_id,
                session_id: event.session_id,
                chunk: event.chunk,
                reset_terminal: event.reset_terminal,
            })
            .collect())
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
fn workspace_create(
    name: String,
    root_dir: String,
    shell_profile: String,
    state: tauri::State<'_, AppState>,
) -> Result<Value, String> {
    let request = RequestEnvelope::new(
        "desktop-workspace-create",
        "workspace.create",
        json!({
            "name": name,
            "rootDir": root_dir,
            "shellProfile": shell_profile,
        }),
    );
    let response = dispatch_runtime_request(&request, &mut state.runtime.lock().unwrap());
    if response.is_ok() {
        Ok(response.result().cloned().unwrap_or_else(|| json!({})))
    } else {
        Err(response
            .error()
            .map(|error| error.message.clone())
            .unwrap_or_else(|| "workspace create failed".to_string()))
    }
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
            "newPaneId": next_pane_id(),
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
fn pane_focus(
    workspace_id: String,
    pane_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let request = RequestEnvelope::new(
        "desktop-pane-focus",
        "pane.focus",
        json!({
            "workspaceId": workspace_id,
            "paneId": pane_id,
        }),
    );
    let response = dispatch_runtime_request(&request, &mut state.runtime.lock().unwrap());
    if response.is_ok() {
        Ok(())
    } else {
        Err(response
            .error()
            .map(|error| error.message.clone())
            .unwrap_or_else(|| "pane focus failed".to_string()))
    }
}

#[tauri::command]
fn pane_close(
    workspace_id: String,
    pane_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let request = RequestEnvelope::new(
        "desktop-pane-close",
        "pane.close",
        json!({
            "workspaceId": workspace_id,
            "paneId": pane_id,
        }),
    );
    let response = dispatch_runtime_request(&request, &mut state.runtime.lock().unwrap());
    if response.is_ok() {
        Ok(())
    } else {
        Err(response
            .error()
            .map(|error| error.message.clone())
            .unwrap_or_else(|| "pane close failed".to_string()))
    }
}

#[tauri::command]
fn workspace_close(
    workspace_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let request = RequestEnvelope::new(
        "desktop-workspace-close",
        "workspace.close",
        json!({
            "workspaceId": workspace_id,
        }),
    );
    let response = dispatch_runtime_request(&request, &mut state.runtime.lock().unwrap());
    if response.is_ok() {
        Ok(())
    } else {
        Err(response
            .error()
            .map(|error| error.message.clone())
            .unwrap_or_else(|| "workspace close failed".to_string()))
    }
}

#[tauri::command]
fn workspace_rename(
    workspace_id: String,
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let request = RequestEnvelope::new(
        "desktop-workspace-rename",
        "workspace.rename",
        json!({
            "workspaceId": workspace_id,
            "name": name,
        }),
    );
    let response = dispatch_runtime_request(&request, &mut state.runtime.lock().unwrap());
    if response.is_ok() {
        Ok(())
    } else {
        Err(response
            .error()
            .map(|error| error.message.clone())
            .unwrap_or_else(|| "workspace rename failed".to_string()))
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

#[tauri::command]
fn session_resize(
    session_id: String,
    rows: u16,
    cols: u16,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let request = RequestEnvelope::new(
        "desktop-resize",
        "session.resize",
        json!({
            "sessionId": session_id,
            "rows": rows,
            "cols": cols,
        }),
    );
    let response = dispatch_runtime_request(&request, &mut state.runtime.lock().unwrap());
    if response.is_ok() {
        Ok(())
    } else {
        Err(response
            .error()
            .map(|error| error.message.clone())
            .unwrap_or_else(|| "resize failed".to_string()))
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

fn next_pane_id() -> String {
    let suffix = NEXT_PANE_ID_SUFFIX.fetch_add(1, Ordering::Relaxed);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!("pane-{timestamp}-{suffix}")
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
        "workspace.close" => handle_workspace_close(request, runtime),
        "workspace.rename" => handle_workspace_rename(request, runtime),
        "pane.split" => handle_pane_split(request, runtime),
        "pane.focus" => handle_pane_focus(request, runtime),
        "pane.close" => handle_pane_close(request, runtime),
        "session.start" => handle_session_start(request, runtime),
        "session.sendInput" => handle_session_send_input(request, runtime),
        "session.resize" => handle_session_resize(request, runtime),
        "session.restart" => handle_session_restart(request, runtime),
        "session.getStatus" => handle_session_get_status(request, runtime),
        "app.getState" => handle_app_get_state(request, runtime),
        _ => core_ipc::dispatch(request, &mut runtime.registry),
    }
}

fn handle_pane_focus<F>(
    request: &RequestEnvelope,
    runtime: &mut RuntimeState<F>,
) -> ResponseEnvelope
where
    F: SessionHostFactory,
{
    let response = core_ipc::dispatch(request, &mut runtime.registry);
    if response.is_ok() {
        runtime.save_registry();
    }
    response
}

fn handle_workspace_rename<F>(
    request: &RequestEnvelope,
    runtime: &mut RuntimeState<F>,
) -> ResponseEnvelope
where
    F: SessionHostFactory,
{
    let response = core_ipc::dispatch(request, &mut runtime.registry);
    if response.is_ok() {
        runtime.save_registry();
    }
    response
}

fn handle_pane_close<F>(
    request: &RequestEnvelope,
    runtime: &mut RuntimeState<F>,
) -> ResponseEnvelope
where
    F: SessionHostFactory,
{
    let payload = request.payload();
    let workspace_id = payload["workspaceId"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let pane_id = payload["paneId"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    let response = core_ipc::dispatch(request, &mut runtime.registry);
    if response.is_ok() {
        runtime.sessions.remove_pane(&workspace_id, &pane_id);
        runtime.save_registry();
    }
    response
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

fn result_field(response: &ResponseEnvelope, field: &str) -> Option<String> {
    response
        .result()
        .and_then(|result| result[field].as_str())
        .map(str::to_string)
}

fn rollback_created_workspaces(registry: &mut WorkspaceRegistry, existing_ids: &HashSet<String>) {
    let created_ids = registry
        .list()
        .iter()
        .filter(|workspace| !existing_ids.contains(&workspace.id))
        .map(|workspace| workspace.id.clone())
        .collect::<Vec<_>>();

    for workspace_id in created_ids {
        let _ = registry.remove_workspace(&workspace_id);
    }
}

fn rollback_split_panes(
    registry: &mut WorkspaceRegistry,
    workspace_id: &str,
    existing_pane_ids: &HashSet<String>,
) {
    let new_pane_ids = find_workspace(registry, workspace_id)
        .map(|workspace| {
            workspace
                .layout
                .panes
                .iter()
                .filter(|pane| !existing_pane_ids.contains(&pane.pane_id))
                .map(|pane| pane.pane_id.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    for pane_id in new_pane_ids {
        let _ = registry.close_pane(workspace_id, &pane_id);
    }
}

fn handle_workspace_create<F>(
    request: &RequestEnvelope,
    runtime: &mut RuntimeState<F>,
) -> ResponseEnvelope
where
    F: SessionHostFactory,
{
    let existing_workspace_ids = runtime
        .registry
        .list()
        .iter()
        .map(|workspace| workspace.id.clone())
        .collect::<HashSet<_>>();
    let response = core_ipc::dispatch(request, &mut runtime.registry);
    if !response.is_ok() {
        return response;
    }

    let Some(workspace_id) = result_field(&response, "workspaceId") else {
        rollback_created_workspaces(&mut runtime.registry, &existing_workspace_ids);
        return ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(
                ErrorCode::InternalError,
                "workspace.create response missing workspaceId",
            ),
        );
    };
    let Some(workspace) = find_workspace(&runtime.registry, &workspace_id) else {
        rollback_created_workspaces(&mut runtime.registry, &existing_workspace_ids);
        return ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(ErrorCode::InternalError, "Created workspace missing from registry"),
        );
    };

    let shell_profile = workspace.shell_profile.clone();
    let root_dir = workspace.root_dir.clone();
    let Some(pane_id) = workspace.layout.panes.first().map(|pane| pane.pane_id.clone()) else {
        rollback_created_workspaces(&mut runtime.registry, &existing_workspace_ids);
        return ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(ErrorCode::InternalError, "Created workspace has no panes"),
        );
    };

    match runtime.sessions.start(
        &workspace_id,
        &pane_id,
        SessionSpec::new(format!("{workspace_id}:{pane_id}"), &shell_profile)
            .with_working_dir(root_dir),
        default_terminal_size(),
    ) {
        Ok(session_id) => {
            runtime.save_registry();
            ResponseEnvelope::success(
                request.id(),
                json!({
                    "workspaceId": workspace_id,
                    "paneId": pane_id,
                    "sessionId": session_id,
                }),
            )
        }
        Err(error) => {
            let _ = runtime.registry.remove_workspace(&workspace_id);
            runtime_error_response(request.id(), error)
        }
    }
}

fn handle_workspace_close<F>(
    request: &RequestEnvelope,
    runtime: &mut RuntimeState<F>,
) -> ResponseEnvelope
where
    F: SessionHostFactory,
{
    let workspace_id = request.payload()["workspaceId"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    // Collect pane IDs before removing the workspace so we can clean up sessions.
    let pane_ids = find_workspace(&runtime.registry, &workspace_id)
        .map(|workspace| {
            workspace
                .layout
                .panes
                .iter()
                .map(|pane| pane.pane_id.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let response = core_ipc::dispatch(request, &mut runtime.registry);
    if response.is_ok() {
        for pane_id in &pane_ids {
            runtime.sessions.remove_pane(&workspace_id, pane_id);
        }
        runtime.save_registry();
    }
    response
}

fn handle_pane_split<F>(
    request: &RequestEnvelope,
    runtime: &mut RuntimeState<F>,
) -> ResponseEnvelope
where
    F: SessionHostFactory,
{
    let workspace_id = request.payload()["workspaceId"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let existing_pane_ids = find_workspace(&runtime.registry, &workspace_id)
        .map(|workspace| {
            workspace
                .layout
                .panes
                .iter()
                .map(|pane| pane.pane_id.clone())
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();
    let response = core_ipc::dispatch(request, &mut runtime.registry);
    if !response.is_ok() {
        return response;
    }

    let Some(pane_id) = result_field(&response, "newPaneId") else {
        rollback_split_panes(&mut runtime.registry, &workspace_id, &existing_pane_ids);
        return ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(
                ErrorCode::InternalError,
                "pane.split response missing newPaneId",
            ),
        );
    };
    let Some(workspace) = find_workspace(&runtime.registry, &workspace_id) else {
        rollback_split_panes(&mut runtime.registry, &workspace_id, &existing_pane_ids);
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
        Ok(session_id) => {
            runtime.save_registry();
            ResponseEnvelope::success(
                request.id(),
                json!({
                    "workspaceId": workspace_id,
                    "newPaneId": pane_id,
                    "sessionId": session_id,
                }),
            )
        }
        Err(error) => {
            rollback_split_panes(&mut runtime.registry, &workspace_id, &existing_pane_ids);
            runtime_error_response(request.id(), error)
        }
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

fn drain_and_emit_output_events<F, Emit>(
    runtime: &Arc<Mutex<RuntimeState<F>>>,
    mut emit: Emit,
) -> Result<(), String>
where
    F: SessionHostFactory,
    Emit: FnMut(&str, &SessionOutputPayload) -> Result<(), String>,
{
    let events = {
        let mut runtime = runtime
            .lock()
            .map_err(|_| "runtime lock poisoned".to_string())?;
        runtime
            .sessions
            .drain_output_events()
            .map_err(|error| error.to_string())?
    };

    for (index, event) in events.iter().enumerate() {
        let payload = SessionOutputPayload {
            workspace_id: event.workspace_id.clone(),
            pane_id: event.pane_id.clone(),
            session_id: event.session_id.clone(),
            chunk: event.chunk.clone(),
            reset_terminal: event.reset_terminal,
        };

        if let Err(error) = emit(SESSION_OUTPUT_EVENT_NAME, &payload) {
            let mut runtime = runtime
                .lock()
                .map_err(|_| "runtime lock poisoned".to_string())?;
            runtime.sessions.rollback_output_events(&events[index..]);
            return Err(error);
        }
    }

    Ok(())
}

fn start_output_event_pump<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    runtime: Arc<Mutex<AppRuntime>>,
) {
    std::thread::spawn(move || loop {
        if let Err(error) = drain_and_emit_output_events(&runtime, |event_name, payload| {
            app_handle
                .emit(event_name, payload.clone())
                .map_err(|error| error.to_string())
        }) {
            eprintln!("[cmux-events] output pump error: {error}");
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    });
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

fn load_registry_from_state_path(path: &std::path::Path) -> WorkspaceRegistry {
    match std::fs::read_to_string(path) {
        Ok(json) => WorkspaceRegistry::from_persisted_json(&json)
            .unwrap_or_else(|_| core_state::starter_workspace_registry()),
        Err(_) => core_state::starter_workspace_registry(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state_path = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("cmux-win")
        .join("state.json");

    let registry = load_registry_from_state_path(&state_path);
    let runtime = Arc::new(Mutex::new(RuntimeState::new_with_state_path(
        registry,
        PtySessionFactory,
        state_path,
    )));
    let event_runtime = Arc::clone(&runtime);

    #[cfg(target_os = "windows")]
    start_named_pipe_server(Arc::clone(&runtime));

    tauri::Builder::default()
        .setup(move |app| {
            start_output_event_pump(app.handle().clone(), Arc::clone(&event_runtime));
            Ok(())
        })
        .manage(AppState { runtime })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            desktop_bootstrap,
            desktop_state,
            workspace_create,
            workspace_close,
            workspace_rename,
            pane_split,
            pane_focus,
            pane_close,
            session_send_input,
            session_restart,
            session_resize
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_ipc::ResponseExt;
    use core_session::{OutputBuffer, SessionHost};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Default)]
    struct FakeFactoryState {
        spawn_commands: Vec<String>,
        writes: Vec<Vec<u8>>,
        outputs: Vec<Arc<Mutex<OutputBuffer>>>,
        exited: bool,
        fail_next_spawn: bool,
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
            let mut state = self.state.lock().unwrap();
            state.writes.push(data.to_vec());
            self.output.lock().unwrap().bytes.extend_from_slice(data);
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

        fn collected_output(&self) -> OutputBuffer {
            self.output.lock().unwrap().clone()
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
            if state.fail_next_spawn {
                state.fail_next_spawn = false;
                return Err(format!("spawn failed for {}", spec.command));
            }
            state.outputs.push(Arc::clone(&output));
            state.spawn_commands.push(spec.command.clone());
            Ok(Box::new(FakeHost {
                state: Arc::clone(&self.state),
                output,
            }))
        }
    }

    fn test_runtime() -> RuntimeState<FakeFactory> {
        RuntimeState::new(core_state::starter_workspace_registry(), FakeFactory::default())
    }

    fn test_runtime_with_factory(factory: FakeFactory) -> RuntimeState<FakeFactory> {
        RuntimeState::new(core_state::starter_workspace_registry(), factory)
    }

    fn unique_state_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join(format!("cmux-win-{label}-{nanos}"))
            .join("state.json")
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
        let pane_id = response.result().unwrap()["paneId"].as_str().unwrap();
        let session_id = response.result().unwrap()["sessionId"].as_str().unwrap();
        assert_eq!(pane_id, "pane-1");
        assert_eq!(runtime.sessions.session_id_for_pane(workspace_id, pane_id), Some(session_id));
        let workspace = runtime
            .registry
            .list()
            .iter()
            .find(|workspace| workspace.id == workspace_id)
            .unwrap();
        assert_eq!(workspace.shell_profile, "pwsh");
    }

    #[test]
    fn handle_runtime_request_workspace_create_rolls_back_registry_when_session_start_fails() {
        let factory = FakeFactory::default();
        let mut runtime = test_runtime_with_factory(factory.clone());
        factory.state.lock().unwrap().fail_next_spawn = true;

        let raw = handle_runtime_request(
            r#"{
                "protocolVersion": 1,
                "id": "req-create-fail",
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
        assert!(!response.is_ok());
        assert_eq!(runtime.registry.list().len(), 1);
        assert_eq!(runtime.registry.list()[0].id, "ws-inbox");
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
    fn handle_runtime_request_pane_split_rolls_back_layout_when_session_start_fails() {
        let factory = FakeFactory::default();
        let mut runtime = test_runtime_with_factory(factory.clone());
        factory.state.lock().unwrap().fail_next_spawn = true;

        let raw = handle_runtime_request(
            r#"{
                "protocolVersion": 1,
                "id": "req-split-fail",
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
        assert!(!response.is_ok());

        let snapshot = runtime.snapshot();
        assert_eq!(snapshot.workspaces[0].panes.len(), 1);
        assert_eq!(snapshot.workspaces[0].panes[0].pane_id, "pane-1");
        assert_eq!(runtime.sessions.session_id_for_pane("ws-inbox", "pane-2"), None);
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
    fn handle_runtime_request_pane_focus_updates_snapshot_focus() {
        let mut runtime = test_runtime();
        let split = handle_runtime_request(
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
        let split_response: ResponseEnvelope = serde_json::from_str(&split).unwrap();
        assert!(split_response.is_ok());

        let focused = handle_runtime_request(
            r#"{
                "protocolVersion": 1,
                "id": "req-focus",
                "type": "command",
                "command": "pane.focus",
                "payload": {
                    "workspaceId": "ws-inbox",
                    "paneId": "pane-1"
                }
            }"#,
            &mut runtime,
        );
        let focus_response: ResponseEnvelope = serde_json::from_str(&focused).unwrap();
        assert!(focus_response.is_ok());
        assert_eq!(runtime.snapshot().workspaces[0].focused_pane_id, "pane-1");
    }

    #[test]
    fn handle_runtime_request_pane_close_removes_live_session_binding() {
        let mut runtime = test_runtime();
        let split = handle_runtime_request(
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
        let split_response: ResponseEnvelope = serde_json::from_str(&split).unwrap();
        assert!(split_response.is_ok());
        assert_eq!(runtime.sessions.session_id_for_pane("ws-inbox", "pane-2"), Some("session:2"));

        let closed = handle_runtime_request(
            r#"{
                "protocolVersion": 1,
                "id": "req-close",
                "type": "command",
                "command": "pane.close",
                "payload": {
                    "workspaceId": "ws-inbox",
                    "paneId": "pane-2"
                }
            }"#,
            &mut runtime,
        );
        let close_response: ResponseEnvelope = serde_json::from_str(&closed).unwrap();
        assert!(close_response.is_ok());
        assert_eq!(runtime.sessions.session_id_for_pane("ws-inbox", "pane-2"), None);
        assert_eq!(runtime.snapshot().workspaces[0].panes.len(), 1);
    }

    #[test]
    fn handle_runtime_request_pane_close_keeps_session_binding_when_last_pane_close_is_rejected() {
        let mut runtime = test_runtime();

        let closed = handle_runtime_request(
            r#"{
                "protocolVersion": 1,
                "id": "req-close-last",
                "type": "command",
                "command": "pane.close",
                "payload": {
                    "workspaceId": "ws-inbox",
                    "paneId": "pane-1"
                }
            }"#,
            &mut runtime,
        );
        let close_response: ResponseEnvelope = serde_json::from_str(&closed).unwrap();
        assert!(!close_response.is_ok());
        assert_eq!(
            runtime.sessions.session_id_for_pane("ws-inbox", "pane-1"),
            Some("session:1")
        );
    }

    #[test]
    fn handle_runtime_request_closing_focused_pane_moves_focus_to_neighbor() {
        let mut runtime = test_runtime();
        let split = handle_runtime_request(
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
        let split_response: ResponseEnvelope = serde_json::from_str(&split).unwrap();
        assert!(split_response.is_ok());

        let closed = handle_runtime_request(
            r#"{
                "protocolVersion": 1,
                "id": "req-close",
                "type": "command",
                "command": "pane.close",
                "payload": {
                    "workspaceId": "ws-inbox",
                    "paneId": "pane-2"
                }
            }"#,
            &mut runtime,
        );
        let close_response: ResponseEnvelope = serde_json::from_str(&closed).unwrap();
        assert!(close_response.is_ok());

        let snapshot = runtime.snapshot();
        assert_eq!(snapshot.workspaces[0].focused_pane_id, "pane-1");
        assert_eq!(snapshot.workspaces[0].panes[0].pane_id, "pane-1");
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
    fn runtime_drain_output_events_returns_session_output_payloads() {
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
                        "data": "echo stream\r\n"
                    }}
                }}"#
            ),
            &mut runtime,
        );
        let send_response: ResponseEnvelope = serde_json::from_str(&send).unwrap();
        assert!(send_response.is_ok());

        let events = runtime.drain_output_events().unwrap();
        assert_eq!(
            events,
            vec![SessionOutputPayload {
                workspace_id: "ws-inbox".to_string(),
                pane_id: "pane-1".to_string(),
                session_id: "session:1".to_string(),
                chunk: "echo stream\r\n".to_string(),
                reset_terminal: false,
            }]
        );
        assert!(runtime.drain_output_events().unwrap().is_empty());
    }

    #[test]
    fn drain_and_emit_output_events_uses_session_output_event_name() {
        let runtime = Arc::new(Mutex::new(test_runtime()));
        let session_id = runtime
            .lock()
            .unwrap()
            .sessions
            .session_id_for_pane("ws-inbox", "pane-1")
            .unwrap()
            .to_string();

        {
            let mut runtime = runtime.lock().unwrap();
            let send = handle_runtime_request(
                &format!(
                    r#"{{
                        "protocolVersion": 1,
                        "id": "req-send",
                        "type": "command",
                        "command": "session.sendInput",
                        "payload": {{
                            "sessionId": "{session_id}",
                            "data": "echo emit\r\n"
                        }}
                    }}"#
                ),
                &mut runtime,
            );
            let send_response: ResponseEnvelope = serde_json::from_str(&send).unwrap();
            assert!(send_response.is_ok());
        }

        let mut emitted = Vec::<(String, SessionOutputPayload)>::new();
        drain_and_emit_output_events(&runtime, |event_name, payload| {
            emitted.push((event_name.to_string(), payload.clone()));
            Ok(())
        })
        .unwrap();

        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0].0, SESSION_OUTPUT_EVENT_NAME);
        assert_eq!(emitted[0].1.chunk, "echo emit\r\n");
        assert!(!emitted[0].1.reset_terminal);
    }

    #[test]
    fn drain_and_emit_output_events_rolls_back_undelivered_events_after_emit_failure() {
        let runtime = Arc::new(Mutex::new(test_runtime()));

        {
            let mut runtime = runtime.lock().unwrap();
            let split = handle_runtime_request(
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
            let split_response: ResponseEnvelope = serde_json::from_str(&split).unwrap();
            assert!(split_response.is_ok());

            let first_session = runtime
                .sessions
                .session_id_for_pane("ws-inbox", "pane-1")
                .unwrap()
                .to_string();
            let second_session = runtime
                .sessions
                .session_id_for_pane("ws-inbox", "pane-2")
                .unwrap()
                .to_string();

            for (session_id, data) in
                [(first_session, "first\\r\\n"), (second_session, "second\\r\\n")]
            {
                let send = handle_runtime_request(
                    &format!(
                        r#"{{
                            "protocolVersion": 1,
                            "id": "req-send",
                            "type": "command",
                            "command": "session.sendInput",
                            "payload": {{
                                "sessionId": "{session_id}",
                                "data": "{data}"
                            }}
                        }}"#
                    ),
                    &mut runtime,
                );
                let send_response: ResponseEnvelope = serde_json::from_str(&send).unwrap();
                assert!(send_response.is_ok());
            }
        }

        let mut first_pass = 0;
        let err = drain_and_emit_output_events(&runtime, |_event_name, payload| {
            first_pass += 1;
            if first_pass == 2 {
                return Err("emit failed".to_string());
            }

            assert_eq!(payload.chunk, "first\r\n");
            Ok(())
        })
        .unwrap_err();
        assert_eq!(err, "emit failed");

        let mut replay = Vec::<String>::new();
        drain_and_emit_output_events(&runtime, |_event_name, payload| {
            replay.push(payload.chunk.clone());
            Ok(())
        })
        .unwrap();

        assert_eq!(replay, vec!["second\r\n".to_string()]);
    }

    #[test]
    fn normalize_split_direction_rejects_unknown_values() {
        let err = normalize_split_direction("diagonal").unwrap_err();

        assert_eq!(err, "Unsupported split direction: diagonal");
    }

    #[test]
    fn next_pane_id_generates_distinct_values() {
        let first = next_pane_id();
        let second = next_pane_id();

        assert_ne!(first, second);
        assert!(first.starts_with("pane-"));
        assert!(second.starts_with("pane-"));
    }

    #[test]
    fn result_field_reads_string_values_from_success_payloads() {
        let response = ResponseEnvelope::success(
            "req-result",
            json!({
                "workspaceId": "ws-test",
                "newPaneId": "pane-2"
            }),
        );

        assert_eq!(result_field(&response, "workspaceId").as_deref(), Some("ws-test"));
        assert_eq!(result_field(&response, "newPaneId").as_deref(), Some("pane-2"));
        assert_eq!(result_field(&response, "missing"), None);
    }

    #[test]
    fn rollback_created_workspaces_removes_entries_not_in_existing_set() {
        let mut registry = core_state::starter_workspace_registry();
        registry
            .create(
                "ws-extra",
                "Extra",
                "D:\\dev\\extra",
                "pwsh",
                core_layout::WorkspaceLayout::starter(),
            )
            .unwrap();
        let existing_ids = HashSet::from([String::from("ws-inbox")]);

        rollback_created_workspaces(&mut registry, &existing_ids);

        assert_eq!(registry.list().len(), 1);
        assert_eq!(registry.list()[0].id, "ws-inbox");
    }

    #[test]
    fn rollback_split_panes_closes_new_panes_not_in_existing_set() {
        let mut registry = core_state::starter_workspace_registry();
        registry
            .split_pane(
                "ws-inbox",
                "pane-1",
                "pane-2",
                core_layout::SplitOrientation::Vertical,
                0.5,
            )
            .unwrap();
        let existing_panes = HashSet::from([String::from("pane-1")]);

        rollback_split_panes(&mut registry, "ws-inbox", &existing_panes);

        let workspace = &registry.list()[0];
        assert_eq!(workspace.layout.panes.len(), 1);
        assert_eq!(workspace.layout.panes[0].pane_id, "pane-1");
    }

    #[test]
    fn handle_runtime_request_workspace_close_removes_workspace_and_cleans_up_sessions() {
        let mut runtime = test_runtime();
        assert_eq!(runtime.sessions.session_id_for_pane("ws-inbox", "pane-1"), Some("session:1"));

        let raw = handle_runtime_request(
            r#"{
                "protocolVersion": 1,
                "id": "req-ws-close",
                "type": "command",
                "command": "workspace.close",
                "payload": {
                    "workspaceId": "ws-inbox"
                }
            }"#,
            &mut runtime,
        );

        let response: ResponseEnvelope = serde_json::from_str(&raw).unwrap();
        assert!(response.is_ok());
        assert_eq!(response.result().unwrap()["workspaceId"], "ws-inbox");
        assert!(runtime.registry.list().is_empty());
        assert_eq!(runtime.sessions.session_id_for_pane("ws-inbox", "pane-1"), None);
    }

    #[test]
    fn handle_runtime_request_workspace_close_cleans_up_all_pane_sessions() {
        let mut runtime = test_runtime();

        // Split to create a second pane with its own session
        let split = handle_runtime_request(
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
        let split_response: ResponseEnvelope = serde_json::from_str(&split).unwrap();
        assert!(split_response.is_ok());
        assert_eq!(runtime.sessions.session_id_for_pane("ws-inbox", "pane-2"), Some("session:2"));

        let raw = handle_runtime_request(
            r#"{
                "protocolVersion": 1,
                "id": "req-ws-close",
                "type": "command",
                "command": "workspace.close",
                "payload": {
                    "workspaceId": "ws-inbox"
                }
            }"#,
            &mut runtime,
        );

        let response: ResponseEnvelope = serde_json::from_str(&raw).unwrap();
        assert!(response.is_ok());
        assert!(runtime.registry.list().is_empty());
        assert_eq!(runtime.sessions.session_id_for_pane("ws-inbox", "pane-1"), None);
        assert_eq!(runtime.sessions.session_id_for_pane("ws-inbox", "pane-2"), None);
    }

    #[test]
    fn handle_runtime_request_workspace_close_returns_not_found_for_unknown_workspace() {
        let mut runtime = test_runtime();

        let raw = handle_runtime_request(
            r#"{
                "protocolVersion": 1,
                "id": "req-ws-close-missing",
                "type": "command",
                "command": "workspace.close",
                "payload": {
                    "workspaceId": "ws-missing"
                }
            }"#,
            &mut runtime,
        );

        let response: ResponseEnvelope = serde_json::from_str(&raw).unwrap();
        assert!(!response.is_ok());
        assert_eq!(response.error().unwrap().code, ErrorCode::NotFound);
        // Starter workspace should be untouched
        assert_eq!(runtime.registry.list().len(), 1);
        assert_eq!(runtime.registry.list()[0].id, "ws-inbox");
    }

    #[test]
    fn handle_runtime_request_workspace_close_preserves_other_workspaces() {
        let mut runtime = test_runtime();

        // Create a second workspace
        let create = handle_runtime_request(
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
        let create_response: ResponseEnvelope = serde_json::from_str(&create).unwrap();
        assert!(create_response.is_ok());
        let api_workspace_id = create_response.result().unwrap()["workspaceId"]
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(runtime.registry.list().len(), 2);

        // Close only the inbox workspace
        let raw = handle_runtime_request(
            r#"{
                "protocolVersion": 1,
                "id": "req-ws-close",
                "type": "command",
                "command": "workspace.close",
                "payload": {
                    "workspaceId": "ws-inbox"
                }
            }"#,
            &mut runtime,
        );

        let response: ResponseEnvelope = serde_json::from_str(&raw).unwrap();
        assert!(response.is_ok());
        assert_eq!(runtime.registry.list().len(), 1);
        assert_eq!(runtime.registry.list()[0].id, api_workspace_id);
        // API workspace session should still be intact
        assert!(runtime.sessions.session_id_for_pane(&api_workspace_id, "pane-1").is_some());
    }

    #[test]
    fn handle_runtime_request_workspace_close_snapshot_excludes_closed_workspace() {
        let mut runtime = test_runtime();

        let raw = handle_runtime_request(
            r#"{
                "protocolVersion": 1,
                "id": "req-ws-close",
                "type": "command",
                "command": "workspace.close",
                "payload": {
                    "workspaceId": "ws-inbox"
                }
            }"#,
            &mut runtime,
        );
        let response: ResponseEnvelope = serde_json::from_str(&raw).unwrap();
        assert!(response.is_ok());

        let snapshot = runtime.snapshot();
        assert!(snapshot.workspaces.is_empty());
    }

    #[test]
    fn handle_runtime_request_workspace_rename_updates_snapshot_name() {
        let mut runtime = test_runtime();

        let raw = handle_runtime_request(
            r#"{
                "protocolVersion": 1,
                "id": "req-ws-rename",
                "type": "command",
                "command": "workspace.rename",
                "payload": {
                    "workspaceId": "ws-inbox",
                    "name": "Inbox Prime"
                }
            }"#,
            &mut runtime,
        );

        let response: ResponseEnvelope = serde_json::from_str(&raw).unwrap();
        assert!(response.is_ok());
        assert_eq!(runtime.snapshot().workspaces[0].name, "Inbox Prime");
    }

    #[test]
    fn handle_runtime_request_workspace_rename_returns_not_found_for_unknown_workspace() {
        let mut runtime = test_runtime();

        let raw = handle_runtime_request(
            r#"{
                "protocolVersion": 1,
                "id": "req-ws-rename-missing",
                "type": "command",
                "command": "workspace.rename",
                "payload": {
                    "workspaceId": "ws-missing",
                    "name": "Inbox Prime"
                }
            }"#,
            &mut runtime,
        );

        let response: ResponseEnvelope = serde_json::from_str(&raw).unwrap();
        assert!(!response.is_ok());
        assert_eq!(response.error().unwrap().code, ErrorCode::NotFound);
    }

    #[test]
    fn save_and_load_registry_roundtrip_restores_workspaces_and_sessions() {
        let state_path = unique_state_path("restore");
        let mut runtime = RuntimeState::new_with_state_path(
            core_state::starter_workspace_registry(),
            FakeFactory::default(),
            state_path.clone(),
        );

        let split = RequestEnvelope::new(
            "persist-split",
            "pane.split",
            json!({
                "workspaceId": "ws-inbox",
                "paneId": "pane-1",
                "newPaneId": "pane-2",
                "orientation": "vertical",
                "ratio": 0.5
            }),
        );
        let rename = RequestEnvelope::new(
            "persist-rename",
            "workspace.rename",
            json!({
                "workspaceId": "ws-inbox",
                "name": "Inbox Prime"
            }),
        );

        let split_response = handle_pane_split(&split, &mut runtime);
        assert!(split_response.is_ok());
        let rename_response = handle_workspace_rename(&rename, &mut runtime);
        assert!(rename_response.is_ok());
        assert!(state_path.exists());

        let restored_registry = load_registry_from_state_path(&state_path);
        let mut restored_runtime = RuntimeState::new(restored_registry, FakeFactory::default());
        let snapshot = restored_runtime.snapshot();

        assert_eq!(snapshot.workspaces.len(), 1);
        assert_eq!(snapshot.workspaces[0].name, "Inbox Prime");
        assert_eq!(snapshot.workspaces[0].panes.len(), 2);
        assert!(snapshot.workspaces[0].panes.iter().all(|pane| pane.session_id.is_some()));

        let _ = fs::remove_file(state_path);
    }

    #[test]
    fn load_registry_from_state_path_falls_back_to_starter_on_invalid_json() {
        let state_path = unique_state_path("invalid");
        fs::create_dir_all(state_path.parent().unwrap()).expect("state dir should exist");
        fs::write(&state_path, "{invalid").expect("invalid state should be written");

        let restored_registry = load_registry_from_state_path(&state_path);

        assert_eq!(restored_registry.list().len(), 1);
        assert_eq!(restored_registry.list()[0].id, "ws-inbox");

        let _ = fs::remove_file(state_path);
    }
}
