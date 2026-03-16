use std::sync::{Arc, Mutex};

use core_state::{DesktopBootstrap, WorkspaceRegistry, APP_NAME, STARTER_WORKSPACE_NAME};

pub const PIPE_NAME: &str = r"\\.\pipe\cmux-win-v1";

pub struct AppState {
    pub registry: Arc<Mutex<WorkspaceRegistry>>,
}

#[tauri::command]
fn desktop_bootstrap(state: tauri::State<'_, AppState>) -> DesktopBootstrap {
    let registry = state.registry.lock().unwrap();
    let workspaces = registry.summaries();
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

#[cfg(target_os = "windows")]
fn start_named_pipe_server(registry: Arc<Mutex<WorkspaceRegistry>>) {
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio pipe runtime")
            .block_on(run_pipe_server(registry));
    });
}

#[cfg(target_os = "windows")]
async fn run_pipe_server(registry: Arc<Mutex<WorkspaceRegistry>>) {
    use tokio::net::windows::named_pipe::ServerOptions;

    let mut first = true;
    loop {
        let server = {
            let mut opts = ServerOptions::new();
            if first {
                opts.first_pipe_instance(true);
                first = false;
            }
            match opts.create(PIPE_NAME) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[cmux-pipe] failed to create pipe instance: {e}");
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                    continue;
                }
            }
        };

        match server.connect().await {
            Ok(()) => {
                let reg = Arc::clone(&registry);
                tokio::spawn(handle_pipe_connection(server, reg));
            }
            Err(e) => {
                eprintln!("[cmux-pipe] client connect error: {e}");
            }
        }
    }
}

#[cfg(target_os = "windows")]
async fn handle_pipe_connection(
    server: tokio::net::windows::named_pipe::NamedPipeServer,
    registry: Arc<Mutex<WorkspaceRegistry>>,
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
                    let mut reg = registry.lock().unwrap();
                    core_ipc::dispatch_raw(line.trim_end(), &mut reg)
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
    let registry = Arc::new(Mutex::new(core_state::starter_workspace_registry()));

    #[cfg(target_os = "windows")]
    start_named_pipe_server(Arc::clone(&registry));

    tauri::Builder::default()
        .manage(AppState { registry })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![desktop_bootstrap])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
