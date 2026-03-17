# cmux-win IPC Spec v0 Draft

## Goals

- Local-only control surface for CLI and automation
- Stable versioned protocol
- Clear request/response model
- Event subscription for UI and tools

## Transport

Windows transport:

- Named pipe
- Proposed endpoint: `\\\\.\\pipe\\cmux-win-v1`

Future mapping:

- Unix domain socket on macOS/Linux

## Trust Model

v0.1 assumes same-user local access.

Rules:

- Bind only to a local transport
- Reject unsupported protocol versions
- Validate all command payloads

## Envelope

All messages are UTF-8 JSON objects.

Request envelope:

```json
{
  "protocolVersion": 1,
  "id": "req_123",
  "type": "command",
  "command": "workspace.create",
  "payload": {}
}
```

Response envelope:

```json
{
  "protocolVersion": 1,
  "id": "req_123",
  "type": "response",
  "ok": true,
  "result": {}
}
```

Error envelope:

```json
{
  "protocolVersion": 1,
  "id": "req_123",
  "type": "response",
  "ok": false,
  "error": {
    "code": "invalid_payload",
    "message": "Missing required field: rootDir"
  }
}
```

Event envelope:

```json
{
  "protocolVersion": 1,
  "type": "event",
  "event": "workspace.updated",
  "payload": {}
}
```

## Implemented Commands

These commands are validated and dispatched by `core-ipc`:

### Workspace Commands

- `workspace.create`
- `workspace.close`
- `workspace.rename`

Example create payload:

```json
{
  "name": "api",
  "rootDir": "D:\\\\src\\\\api",
  "shellProfile": "pwsh"
}
```

### Pane Commands

- `pane.split`
- `pane.close`
- `pane.focus`

Example split payload:

```json
{
  "workspaceId": "ws_1",
  "paneId": "pane_1",
  "orientation": "vertical",
  "ratio": 0.5
}
```

### Session Commands

These are validated by `core-ipc` but dispatched at the desktop runtime layer, not in the core dispatch loop:

- `session.start`
- `session.sendInput`
- `session.resize`
- `session.restart`
- `session.getStatus`

### Notification Commands

- `notify.send` (validated and accepted; no-op in core dispatch)

Example notify payload:

```json
{
  "title": "Build finished",
  "body": "All tests passed",
  "level": "info",
  "workspaceId": "ws_1"
}
```

### App Commands

- `app.getState` (validated; handled at desktop runtime layer)

## Planned Commands (Not Yet Implemented)

### Workspace

- `workspace.list`
- `workspace.get`
- `workspace.focus`

### Pane

- `pane.resize`

### Theme

- `theme.list`
- `theme.apply`
- `theme.import`

### Notification

- `notify.listRecent`

### App

- `app.subscribe`
- `app.unsubscribe`
- `app.checkForUpdates`

## Event Set v0.1

- `workspace.created`
- `workspace.updated`
- `workspace.closed`
- `pane.created`
- `pane.updated`
- `pane.closed`
- `session.started`
- `session.output`
- `session.exited`
- `sidebar.updated`
- `notification.created`
- `theme.applied`
- `app.updateAvailable`

## Output Streaming

Terminal output is evented through `session.output`.

Desktop bridge note:

- the logical IPC event name remains `session.output`
- the Tauri desktop bridge emits the same stream as `session-output` because Tauri event names cannot contain `.`

Payload:

```json
{
  "workspaceId": "ws_1",
  "paneId": "pane_1",
  "sessionId": "term_1",
  "chunk": "hello\\r\\n"
}
```

Rules:

- Preserve ordering per session
- Permit chunking
- Do not guarantee line boundaries

## Session Snapshot Shape

Used for restore and `app.getState`.

```json
{
  "workspaces": [
    {
      "id": "ws_1",
      "name": "api",
      "rootDir": "D:\\\\src\\\\api",
      "activeTheme": "night-owl",
      "splitTree": {},
      "sidebar": {},
      "restore": {
        "shellProfile": "pwsh",
        "lastFocusedPaneId": "pane_2"
      }
    }
  ]
}
```

## CLI Mapping

Examples:

- `cmux-win workspace create --name api --root D:\\src\\api`
- `cmux-win workspace list`
- `cmux-win pane split --pane pane_1 --orientation vertical`
- `cmux-win notify --title "Build finished" --body "All tests passed"`

CLI should be a thin mapper over IPC.

## Error Codes

- `invalid_version`
- `invalid_payload`
- `not_found`
- `conflict`
- `transport_error`
- `internal_error`
- `unsupported`

## Compatibility Rules

1. Additive fields are allowed in minor revisions.
2. Removing or renaming commands requires a protocol version bump.
3. Clients must ignore unknown event fields.
4. Server must reject unknown commands with `unsupported`.
