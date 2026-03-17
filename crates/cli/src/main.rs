use clap::{Parser, Subcommand};
use serde_json::{Value, json};

const PIPE_NAME: &str = r"\\.\pipe\cmux-win-v1";
const PROTOCOL_VERSION: u32 = 1;

// ── CLI argument structs ─────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "cmux-win", about = "CLI for the cmux-win terminal multiplexer")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Manage workspaces
    Workspace {
        #[command(subcommand)]
        action: WorkspaceCommand,
    },
    /// Manage panes
    Pane {
        #[command(subcommand)]
        action: PaneCommand,
    },
    /// Send a desktop notification
    Notify(NotifyArgs),
}

#[derive(Subcommand)]
enum WorkspaceCommand {
    /// Create a new workspace
    Create {
        /// Workspace name
        #[arg(long)]
        name: String,
        /// Root directory for the workspace
        #[arg(long)]
        root: String,
        /// Shell profile to use (default: cmd.exe)
        #[arg(long)]
        shell: Option<String>,
    },
    /// List all workspaces
    List,
    /// Rename an existing workspace
    Rename {
        /// Workspace ID to rename
        id: String,
        /// New name for the workspace
        #[arg(long)]
        name: String,
    },
    /// Close a workspace
    Close {
        /// Workspace ID to close
        id: String,
    },
}

#[derive(Subcommand)]
enum PaneCommand {
    /// Split an existing pane
    Split {
        /// Workspace ID
        workspace_id: String,
        /// Pane ID to split
        pane_id: String,
        /// Split direction
        #[arg(long, default_value = "vertical")]
        direction: String,
    },
    /// Close a pane
    Close {
        /// Workspace ID
        workspace_id: String,
        /// Pane ID to close
        pane_id: String,
    },
    /// Focus a pane
    Focus {
        /// Workspace ID
        workspace_id: String,
        /// Pane ID to focus
        pane_id: String,
    },
}

#[derive(Parser)]
struct NotifyArgs {
    /// Notification title
    #[arg(long)]
    title: String,
    /// Notification body
    #[arg(long)]
    body: String,
    /// Notification level (info, warning, error)
    #[arg(long, default_value = "info")]
    level: String,
}

// ── Request building ─────────────────────────────────────────────────────

fn build_request(cli: &Cli) -> (String, Value) {
    match &cli.command {
        Command::Workspace { action } => match action {
            WorkspaceCommand::Create { name, root, shell } => (
                "workspace.create".to_string(),
                json!({
                    "name": name,
                    "rootDir": root,
                    "shellProfile": shell.as_deref().unwrap_or("cmd.exe"),
                }),
            ),
            WorkspaceCommand::List => (
                "app.getState".to_string(),
                json!({}),
            ),
            WorkspaceCommand::Rename { id, name } => (
                "workspace.rename".to_string(),
                json!({
                    "workspaceId": id,
                    "name": name,
                }),
            ),
            WorkspaceCommand::Close { id } => (
                "workspace.close".to_string(),
                json!({
                    "workspaceId": id,
                }),
            ),
        },
        Command::Pane { action } => match action {
            PaneCommand::Split {
                workspace_id,
                pane_id,
                direction,
            } => {
                let new_pane_id = format!("pane-cli-{}", uuid::Uuid::new_v4());
                (
                    "pane.split".to_string(),
                    json!({
                        "workspaceId": workspace_id,
                        "paneId": pane_id,
                        "newPaneId": new_pane_id,
                        "orientation": direction,
                        "ratio": 0.5,
                    }),
                )
            }
            PaneCommand::Close {
                workspace_id,
                pane_id,
            } => (
                "pane.close".to_string(),
                json!({
                    "workspaceId": workspace_id,
                    "paneId": pane_id,
                }),
            ),
            PaneCommand::Focus {
                workspace_id,
                pane_id,
            } => (
                "pane.focus".to_string(),
                json!({
                    "workspaceId": workspace_id,
                    "paneId": pane_id,
                }),
            ),
        },
        Command::Notify(args) => (
            "notify.send".to_string(),
            json!({
                "title": args.title,
                "body": args.body,
                "level": args.level,
            }),
        ),
    }
}

fn build_envelope(command: &str, payload: Value) -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "id": format!("cli-{}", uuid::Uuid::new_v4()),
        "type": "command",
        "command": command,
        "payload": payload,
    })
}

// ── Pipe communication ───────────────────────────────────────────────────

async fn send_request(envelope: &str) -> Result<String, Box<dyn std::error::Error>> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::windows::named_pipe::ClientOptions;

    let client = ClientOptions::new().open(PIPE_NAME)?;
    let (read_half, mut write_half) = tokio::io::split(client);

    let mut message = envelope.to_string();
    message.push('\n');
    write_half.write_all(message.as_bytes()).await?;

    let mut reader = BufReader::new(read_half);
    let mut response = String::new();
    reader.read_line(&mut response).await?;

    Ok(response)
}

// ── Response formatting ──────────────────────────────────────────────────

fn format_response(command: &str, response: &Value) -> String {
    let ok = response.get("ok").and_then(Value::as_bool).unwrap_or(false);

    if !ok {
        let error_msg = response
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("Unknown error");
        let error_code = response
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        return format!("Error [{}]: {}", error_code, error_msg);
    }

    let result = response.get("result").unwrap_or(&Value::Null);

    match command {
        "app.getState" => format_workspace_list(result),
        "workspace.create" => {
            let id = result
                .get("workspaceId")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            format!("Created workspace {id}")
        }
        "workspace.rename" => {
            let id = result
                .get("workspaceId")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            format!("Renamed workspace {id}")
        }
        "workspace.close" => {
            let id = result
                .get("workspaceId")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            format!("Closed workspace {id}")
        }
        "pane.split" => {
            let pane_id = result
                .get("newPaneId")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            format!("Split pane, created {pane_id}")
        }
        "pane.close" => "Pane closed".to_string(),
        "pane.focus" => "Pane focused".to_string(),
        "notify.send" => "Notification sent".to_string(),
        _ => format!("OK: {}", serde_json::to_string_pretty(result).unwrap_or_default()),
    }
}

fn format_workspace_list(result: &Value) -> String {
    let workspaces = match result.get("workspaces").and_then(Value::as_array) {
        Some(ws) => ws,
        None => return "No workspaces found.".to_string(),
    };

    if workspaces.is_empty() {
        return "No workspaces found.".to_string();
    }

    let active_id = result
        .get("activeWorkspaceId")
        .and_then(Value::as_str)
        .unwrap_or("");

    let mut lines = vec![format!(
        "{:<40} {:<20} {}",
        "ID", "NAME", "PANES"
    )];

    for ws in workspaces {
        let id = ws.get("id").and_then(Value::as_str).unwrap_or("?");
        let name = ws.get("name").and_then(Value::as_str).unwrap_or("?");
        let pane_count = ws
            .get("panes")
            .and_then(Value::as_array)
            .map(|p| p.len())
            .unwrap_or(0);

        let marker = if id == active_id { " *" } else { "" };
        lines.push(format!(
            "{:<40} {:<20} {}{}",
            id, name, pane_count, marker
        ));
    }

    lines.join("\n")
}

// ── Entry point ──────────────────────────────────────────────────────────

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cli = Cli::parse();
    let (command, payload) = build_request(&cli);
    let envelope = build_envelope(&command, payload);

    let envelope_str = serde_json::to_string(&envelope).unwrap_or_else(|err| {
        eprintln!("Failed to serialize request: {err}");
        std::process::exit(1);
    });

    match send_request(&envelope_str).await {
        Ok(response_str) => {
            let response: Value = match serde_json::from_str(response_str.trim()) {
                Ok(v) => v,
                Err(err) => {
                    eprintln!("Failed to parse response: {err}");
                    std::process::exit(1);
                }
            };

            let output = format_response(&command, &response);
            let ok = response.get("ok").and_then(Value::as_bool).unwrap_or(false);

            if ok {
                println!("{output}");
            } else {
                eprintln!("{output}");
                std::process::exit(1);
            }
        }
        Err(err) => {
            eprintln!("Failed to connect to cmux-win desktop app: {err}");
            eprintln!("Is the desktop app running?");
            std::process::exit(1);
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    /// Helper: parse CLI args and return (command, payload) tuple.
    fn parse_and_build(args: &[&str]) -> (String, Value) {
        let cli = Cli::parse_from(args);
        build_request(&cli)
    }

    // ── build_request: workspace commands ────────────────────────────────

    #[test]
    fn workspace_create_builds_correct_request() {
        let (cmd, payload) = parse_and_build(&[
            "cmux-win",
            "workspace",
            "create",
            "--name",
            "my-ws",
            "--root",
            "C:\\projects",
            "--shell",
            "powershell.exe",
        ]);
        assert_eq!(cmd, "workspace.create");
        assert_eq!(payload["name"], "my-ws");
        assert_eq!(payload["rootDir"], "C:\\projects");
        assert_eq!(payload["shellProfile"], "powershell.exe");
    }

    #[test]
    fn workspace_create_defaults_shell_to_cmd() {
        let (cmd, payload) = parse_and_build(&[
            "cmux-win",
            "workspace",
            "create",
            "--name",
            "ws",
            "--root",
            "/tmp",
        ]);
        assert_eq!(cmd, "workspace.create");
        assert_eq!(payload["shellProfile"], "cmd.exe");
    }

    #[test]
    fn workspace_list_uses_app_get_state() {
        let (cmd, payload) = parse_and_build(&["cmux-win", "workspace", "list"]);
        assert_eq!(cmd, "app.getState");
        assert_eq!(payload, json!({}));
    }

    #[test]
    fn workspace_rename_builds_correct_request() {
        let (cmd, payload) = parse_and_build(&[
            "cmux-win",
            "workspace",
            "rename",
            "ws-123",
            "--name",
            "new-name",
        ]);
        assert_eq!(cmd, "workspace.rename");
        assert_eq!(payload["workspaceId"], "ws-123");
        assert_eq!(payload["name"], "new-name");
    }

    #[test]
    fn workspace_close_builds_correct_request() {
        let (cmd, payload) = parse_and_build(&["cmux-win", "workspace", "close", "ws-456"]);
        assert_eq!(cmd, "workspace.close");
        assert_eq!(payload["workspaceId"], "ws-456");
    }

    // ── build_request: pane commands ─────────────────────────────────────

    #[test]
    fn pane_split_builds_correct_request() {
        let (cmd, payload) = parse_and_build(&[
            "cmux-win",
            "pane",
            "split",
            "ws-1",
            "pane-1",
            "--direction",
            "horizontal",
        ]);
        assert_eq!(cmd, "pane.split");
        assert_eq!(payload["workspaceId"], "ws-1");
        assert_eq!(payload["paneId"], "pane-1");
        assert_eq!(payload["orientation"], "horizontal");
        assert_eq!(payload["ratio"], 0.5);
        // newPaneId should start with "pane-cli-"
        let new_pane_id = payload["newPaneId"].as_str().unwrap();
        assert!(
            new_pane_id.starts_with("pane-cli-"),
            "newPaneId should start with 'pane-cli-', got: {new_pane_id}"
        );
    }

    #[test]
    fn pane_split_defaults_direction_to_vertical() {
        let (cmd, payload) =
            parse_and_build(&["cmux-win", "pane", "split", "ws-1", "pane-1"]);
        assert_eq!(cmd, "pane.split");
        assert_eq!(payload["orientation"], "vertical");
    }

    #[test]
    fn pane_close_builds_correct_request() {
        let (cmd, payload) =
            parse_and_build(&["cmux-win", "pane", "close", "ws-1", "pane-1"]);
        assert_eq!(cmd, "pane.close");
        assert_eq!(payload["workspaceId"], "ws-1");
        assert_eq!(payload["paneId"], "pane-1");
    }

    #[test]
    fn pane_focus_builds_correct_request() {
        let (cmd, payload) =
            parse_and_build(&["cmux-win", "pane", "focus", "ws-1", "pane-1"]);
        assert_eq!(cmd, "pane.focus");
        assert_eq!(payload["workspaceId"], "ws-1");
        assert_eq!(payload["paneId"], "pane-1");
    }

    // ── build_request: notify ────────────────────────────────────────────

    #[test]
    fn notify_builds_correct_request() {
        let (cmd, payload) = parse_and_build(&[
            "cmux-win",
            "notify",
            "--title",
            "Alert",
            "--body",
            "Something happened",
            "--level",
            "warning",
        ]);
        assert_eq!(cmd, "notify.send");
        assert_eq!(payload["title"], "Alert");
        assert_eq!(payload["body"], "Something happened");
        assert_eq!(payload["level"], "warning");
    }

    #[test]
    fn notify_defaults_level_to_info() {
        let (cmd, payload) = parse_and_build(&[
            "cmux-win",
            "notify",
            "--title",
            "Hello",
            "--body",
            "World",
        ]);
        assert_eq!(cmd, "notify.send");
        assert_eq!(payload["level"], "info");
    }

    // ── build_envelope ───────────────────────────────────────────────────

    #[test]
    fn build_envelope_has_required_fields() {
        let envelope = build_envelope("workspace.create", json!({"name": "test"}));
        assert_eq!(envelope["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(envelope["type"], "command");
        assert_eq!(envelope["command"], "workspace.create");
        assert_eq!(envelope["payload"]["name"], "test");
        // id should start with "cli-"
        let id = envelope["id"].as_str().unwrap();
        assert!(
            id.starts_with("cli-"),
            "envelope id should start with 'cli-', got: {id}"
        );
    }

    #[test]
    fn build_envelope_generates_unique_ids() {
        let e1 = build_envelope("notify.send", json!({}));
        let e2 = build_envelope("notify.send", json!({}));
        assert_ne!(e1["id"], e2["id"]);
    }

    // ── format_response ──────────────────────────────────────────────────

    #[test]
    fn format_response_workspace_create_success() {
        let response = json!({
            "ok": true,
            "result": { "workspaceId": "ws-abc" }
        });
        let output = format_response("workspace.create", &response);
        assert_eq!(output, "Created workspace ws-abc");
    }

    #[test]
    fn format_response_workspace_rename_success() {
        let response = json!({
            "ok": true,
            "result": { "workspaceId": "ws-abc" }
        });
        let output = format_response("workspace.rename", &response);
        assert_eq!(output, "Renamed workspace ws-abc");
    }

    #[test]
    fn format_response_workspace_close_success() {
        let response = json!({
            "ok": true,
            "result": { "workspaceId": "ws-abc" }
        });
        let output = format_response("workspace.close", &response);
        assert_eq!(output, "Closed workspace ws-abc");
    }

    #[test]
    fn format_response_pane_split_success() {
        let response = json!({
            "ok": true,
            "result": { "newPaneId": "pane-cli-xyz" }
        });
        let output = format_response("pane.split", &response);
        assert_eq!(output, "Split pane, created pane-cli-xyz");
    }

    #[test]
    fn format_response_pane_close_success() {
        let response = json!({ "ok": true, "result": {} });
        let output = format_response("pane.close", &response);
        assert_eq!(output, "Pane closed");
    }

    #[test]
    fn format_response_pane_focus_success() {
        let response = json!({ "ok": true, "result": {} });
        let output = format_response("pane.focus", &response);
        assert_eq!(output, "Pane focused");
    }

    #[test]
    fn format_response_notify_success() {
        let response = json!({ "ok": true, "result": {} });
        let output = format_response("notify.send", &response);
        assert_eq!(output, "Notification sent");
    }

    #[test]
    fn format_response_error() {
        let response = json!({
            "ok": false,
            "error": { "code": "not_found", "message": "Workspace not found" }
        });
        let output = format_response("workspace.close", &response);
        assert_eq!(output, "Error [not_found]: Workspace not found");
    }

    #[test]
    fn format_response_workspace_list_with_workspaces() {
        let response = json!({
            "ok": true,
            "result": {
                "protocolVersion": 1,
                "workspaces": [
                    {
                        "id": "ws-1",
                        "name": "project-a",
                        "rootDir": "C:\\dev\\a",
                        "shellProfile": "cmd.exe",
                        "focusedPaneId": "pane-1",
                        "panes": [
                            { "paneId": "pane-1", "status": "running" }
                        ]
                    },
                    {
                        "id": "ws-2",
                        "name": "project-b",
                        "rootDir": "C:\\dev\\b",
                        "shellProfile": "powershell.exe",
                        "focusedPaneId": "pane-2",
                        "panes": [
                            { "paneId": "pane-2", "status": "running" },
                            { "paneId": "pane-3", "status": "running" }
                        ]
                    }
                ],
                "activeWorkspaceId": "ws-1"
            }
        });
        let output = format_response("app.getState", &response);
        assert!(output.contains("ID"));
        assert!(output.contains("NAME"));
        assert!(output.contains("PANES"));
        assert!(output.contains("ws-1"));
        assert!(output.contains("project-a"));
        assert!(output.contains("ws-2"));
        assert!(output.contains("project-b"));
        // Active workspace should have a marker
        assert!(output.contains("*"));
    }

    #[test]
    fn format_response_workspace_list_empty() {
        let response = json!({
            "ok": true,
            "result": {
                "protocolVersion": 1,
                "workspaces": [],
                "activeWorkspaceId": null
            }
        });
        let output = format_response("app.getState", &response);
        assert_eq!(output, "No workspaces found.");
    }
}
