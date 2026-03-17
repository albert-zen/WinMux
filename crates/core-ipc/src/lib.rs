use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use core_layout::{LayoutError, SplitOrientation, WorkspaceLayout};
use core_session::{LiveSessionRegistry, SessionHostFactory, SessionRuntimeError, TerminalSize};
use core_state::{WorkspaceError, WorkspaceRegistry};

pub const CRATE_NAME: &str = "core-ipc";
pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvelopeType {
    Command,
    Response,
    Event,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidVersion,
    InvalidPayload,
    NotFound,
    Conflict,
    TransportError,
    InternalError,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorPayload {
    pub code: ErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestEnvelope {
    protocol_version: u32,
    id: String,
    #[serde(rename = "type")]
    envelope_type: EnvelopeType,
    command: String,
    payload: Value,
}

// Internal raw type used only as the deserialization intermediary.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawResponseEnvelope {
    protocol_version: u32,
    id: String,
    #[serde(rename = "type")]
    envelope_type: EnvelopeType,
    ok: bool,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<ErrorPayload>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(try_from = "RawResponseEnvelope")]
pub struct ResponseEnvelope {
    pub protocol_version: u32,
    pub id: String,
    #[serde(rename = "type")]
    pub envelope_type: EnvelopeType,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ErrorPayload>,
}

impl TryFrom<RawResponseEnvelope> for ResponseEnvelope {
    type Error = String;

    fn try_from(raw: RawResponseEnvelope) -> Result<Self, Self::Error> {
        if raw.ok && raw.result.is_none() {
            return Err(
                "incoherent response envelope: ok=true but 'result' field is absent".into(),
            );
        }
        if raw.ok && raw.error.is_some() {
            return Err(
                "incoherent response envelope: ok=true but 'error' field must not be present alongside 'result'"
                    .into(),
            );
        }
        if !raw.ok && raw.error.is_none() {
            return Err(
                "incoherent response envelope: ok=false but 'error' field is absent".into(),
            );
        }
        if !raw.ok && raw.result.is_some() {
            return Err(
                "incoherent response envelope: ok=false but 'result' field is present; expected 'error'"
                    .into(),
            );
        }
        Ok(ResponseEnvelope {
            protocol_version: raw.protocol_version,
            id: raw.id,
            envelope_type: raw.envelope_type,
            ok: raw.ok,
            result: raw.result,
            error: raw.error,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventEnvelope {
    pub protocol_version: u32,
    #[serde(rename = "type")]
    pub envelope_type: EnvelopeType,
    pub event: String,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolError {
    pub code: ErrorCode,
    pub message: String,
}

impl ProtocolError {
    #[must_use]
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn into_error_payload(self) -> ErrorPayload {
        ErrorPayload {
            code: self.code,
            message: self.message,
        }
    }
}

impl ErrorCode {
    fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::InvalidVersion => "invalid_version",
            ErrorCode::InvalidPayload => "invalid_payload",
            ErrorCode::NotFound => "not_found",
            ErrorCode::Conflict => "conflict",
            ErrorCode::TransportError => "transport_error",
            ErrorCode::InternalError => "internal_error",
            ErrorCode::Unsupported => "unsupported",
        }
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for ProtocolError {}

#[must_use]
pub fn crate_name() -> &'static str {
    CRATE_NAME
}

impl RequestEnvelope {
    #[must_use]
    pub fn new(id: impl Into<String>, command: impl Into<String>, payload: Value) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            id: id.into(),
            envelope_type: EnvelopeType::Command,
            command: command.into(),
            payload,
        }
    }

    #[must_use]
    pub fn protocol_version(&self) -> u32 {
        self.protocol_version
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn envelope_type(&self) -> EnvelopeType {
        self.envelope_type.clone()
    }

    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }

    #[must_use]
    pub fn payload(&self) -> &Value {
        &self.payload
    }
}

impl ResponseEnvelope {
    #[must_use]
    pub fn success(id: impl Into<String>, result: Value) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            id: id.into(),
            envelope_type: EnvelopeType::Response,
            ok: true,
            result: Some(result),
            error: None,
        }
    }

    #[must_use]
    pub fn error(id: impl Into<String>, error: ProtocolError) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            id: id.into(),
            envelope_type: EnvelopeType::Response,
            ok: false,
            result: None,
            error: Some(error.into_error_payload()),
        }
    }
}

impl EventEnvelope {
    #[must_use]
    pub fn new(event: impl Into<String>, payload: Value) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            envelope_type: EnvelopeType::Event,
            event: event.into(),
            payload,
        }
    }
}

/// Read-only accessors for `ResponseEnvelope`.
///
/// Defined as a trait because `ResponseEnvelope::error` is already taken by the
/// error-response constructor and Rust forbids two inherent items with the same name.
pub trait ResponseExt {
    fn is_ok(&self) -> bool;
    fn result(&self) -> Option<&Value>;
    fn error(&self) -> Option<&ErrorPayload>;
}

impl ResponseExt for ResponseEnvelope {
    fn is_ok(&self) -> bool {
        self.ok
    }

    fn result(&self) -> Option<&Value> {
        self.result.as_ref()
    }

    fn error(&self) -> Option<&ErrorPayload> {
        self.error.as_ref()
    }
}

pub fn parse_and_validate_request(input: &str) -> Result<RequestEnvelope, ProtocolError> {
    let request: RequestEnvelope = serde_json::from_str(input).map_err(|err| {
        ProtocolError::new(
            ErrorCode::InvalidPayload,
            format!("Malformed request JSON: {err}"),
        )
    })?;

    validate_request(&request)?;
    Ok(request)
}

pub fn validate_request(request: &RequestEnvelope) -> Result<(), ProtocolError> {
    if request.protocol_version != PROTOCOL_VERSION {
        return Err(ProtocolError::new(
            ErrorCode::InvalidVersion,
            format!(
                "Unsupported protocol version: {}",
                request.protocol_version
            ),
        ));
    }

    if request.envelope_type != EnvelopeType::Command {
        return Err(ProtocolError::new(
            ErrorCode::InvalidPayload,
            "Request envelope type must be command",
        ));
    }

    if request.id.trim().is_empty() {
        return Err(ProtocolError::new(
            ErrorCode::InvalidPayload,
            "Missing required field: id",
        ));
    }

    match request.command.as_str() {
        "workspace.create" => validate_workspace_create_payload(&request.payload),
        "pane.split" => validate_pane_split_payload(&request.payload),
        "pane.close" => validate_pane_close_payload(&request.payload),
        "pane.focus" => validate_pane_focus_payload(&request.payload),
        "session.start" => validate_session_start_payload(&request.payload),
        "session.sendInput" => validate_session_send_input_payload(&request.payload),
        "session.resize" => validate_session_resize_payload(&request.payload),
        "session.restart" => validate_session_restart_payload(&request.payload),
        "workspace.close" => validate_workspace_close_payload(&request.payload),
        "workspace.rename" => validate_workspace_rename_payload(&request.payload),
        "session.getStatus" => validate_session_get_status_payload(&request.payload),
        "app.getState" => validate_app_get_state_payload(&request.payload),
        "notify.send" => validate_notify_payload(&request.payload),
        _ => Err(ProtocolError::new(
            ErrorCode::Unsupported,
            format!("Unsupported command: {}", request.command),
        )),
    }
}

fn validate_workspace_create_payload(payload: &Value) -> Result<(), ProtocolError> {
    let object = payload_object(payload)?;

    required_string_field(object, "name")?;
    required_string_field(object, "rootDir")?;
    required_string_field(object, "shellProfile")?;

    Ok(())
}

fn validate_workspace_close_payload(payload: &Value) -> Result<(), ProtocolError> {
    let object = payload_object(payload)?;
    required_string_field(object, "workspaceId")?;
    Ok(())
}

fn validate_workspace_rename_payload(payload: &Value) -> Result<(), ProtocolError> {
    let object = payload_object(payload)?;
    required_string_field(object, "workspaceId")?;
    required_string_field(object, "name")?;
    Ok(())
}

fn validate_pane_split_payload(payload: &Value) -> Result<(), ProtocolError> {
    let object = payload_object(payload)?;

    required_string_field(object, "workspaceId")?;
    required_string_field(object, "paneId")?;
    required_string_field(object, "newPaneId")?;

    let orientation = required_string_field(object, "orientation")?;
    if orientation != "vertical" && orientation != "horizontal" {
        return Err(ProtocolError::new(
            ErrorCode::InvalidPayload,
            "Invalid field orientation: expected vertical or horizontal",
        ));
    }

    let ratio = object
        .get("ratio")
        .and_then(Value::as_f64)
        .ok_or_else(|| ProtocolError::new(ErrorCode::InvalidPayload, "Missing required field: ratio"))?;
    if ratio <= 0.0 || ratio >= 1.0 {
        return Err(ProtocolError::new(
            ErrorCode::InvalidPayload,
            "Invalid field ratio: expected a number between 0 and 1",
        ));
    }

    Ok(())
}

fn validate_pane_close_payload(payload: &Value) -> Result<(), ProtocolError> {
    let object = payload_object(payload)?;
    required_string_field(object, "workspaceId")?;
    required_string_field(object, "paneId")?;
    Ok(())
}

fn validate_pane_focus_payload(payload: &Value) -> Result<(), ProtocolError> {
    let object = payload_object(payload)?;
    required_string_field(object, "workspaceId")?;
    required_string_field(object, "paneId")?;
    Ok(())
}

fn validate_notify_payload(payload: &Value) -> Result<(), ProtocolError> {
    let object = payload_object(payload)?;

    required_string_field(object, "title")?;
    required_string_field(object, "body")?;

    let level = required_string_field(object, "level")?;
    if !matches!(level, "info" | "success" | "warning" | "error") {
        return Err(ProtocolError::new(
            ErrorCode::InvalidPayload,
            "Invalid field level: expected info, success, warning, or error",
        ));
    }

    if let Some(workspace_id) = object.get("workspaceId") {
        if !workspace_id.is_null()
            && workspace_id.as_str().is_none_or(|value| value.trim().is_empty())
        {
            return Err(ProtocolError::new(
                ErrorCode::InvalidPayload,
                "Invalid field workspaceId: expected a non-empty string",
            ));
        }
    }

    Ok(())
}

fn validate_session_start_payload(payload: &Value) -> Result<(), ProtocolError> {
    let object = payload_object(payload)?;

    required_string_field(object, "workspaceId")?;
    required_string_field(object, "paneId")?;
    required_string_field(object, "shellProfile")?;
    required_positive_u16_field(object, "cols")?;
    required_positive_u16_field(object, "rows")?;

    Ok(())
}

fn validate_session_send_input_payload(payload: &Value) -> Result<(), ProtocolError> {
    let object = payload_object(payload)?;

    required_string_field(object, "sessionId")?;
    object
        .get("data")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ProtocolError::new(
                ErrorCode::InvalidPayload,
                "Missing required field: data",
            )
        })?;

    Ok(())
}

fn validate_session_resize_payload(payload: &Value) -> Result<(), ProtocolError> {
    let object = payload_object(payload)?;

    required_string_field(object, "sessionId")?;
    required_positive_u16_field(object, "cols")?;
    required_positive_u16_field(object, "rows")?;

    Ok(())
}

fn validate_session_get_status_payload(payload: &Value) -> Result<(), ProtocolError> {
    let object = payload_object(payload)?;
    required_string_field(object, "sessionId")?;
    Ok(())
}

fn validate_session_restart_payload(payload: &Value) -> Result<(), ProtocolError> {
    let object = payload_object(payload)?;
    required_string_field(object, "sessionId")?;
    Ok(())
}

fn validate_app_get_state_payload(payload: &Value) -> Result<(), ProtocolError> {
    payload_object(payload)?;
    Ok(())
}

fn payload_object(payload: &Value) -> Result<&Map<String, Value>, ProtocolError> {
    payload.as_object().ok_or_else(|| {
        ProtocolError::new(
            ErrorCode::InvalidPayload,
            "Payload must be a JSON object",
        )
    })
}

fn required_string_field<'a>(
    payload: &'a Map<String, Value>,
    field: &str,
) -> Result<&'a str, ProtocolError> {
    let value = payload.get(field).ok_or_else(|| {
        ProtocolError::new(
            ErrorCode::InvalidPayload,
            format!("Missing required field: {field}"),
        )
    })?;

    let text = value.as_str().ok_or_else(|| {
        ProtocolError::new(
            ErrorCode::InvalidPayload,
            format!("Invalid field {field}: expected a string"),
        )
    })?;

    if text.trim().is_empty() {
        return Err(ProtocolError::new(
            ErrorCode::InvalidPayload,
            format!("Invalid field {field}: expected a non-empty string"),
        ));
    }

    Ok(text)
}

fn required_positive_u16_field(
    payload: &Map<String, Value>,
    field: &str,
) -> Result<u16, ProtocolError> {
    let value = payload.get(field).ok_or_else(|| {
        ProtocolError::new(
            ErrorCode::InvalidPayload,
            format!("Missing required field: {field}"),
        )
    })?;

    let number = value.as_u64().ok_or_else(|| {
        ProtocolError::new(
            ErrorCode::InvalidPayload,
            format!("Invalid field {field}: expected a positive integer"),
        )
    })?;

    let size = u16::try_from(number).map_err(|_| {
        ProtocolError::new(
            ErrorCode::InvalidPayload,
            format!("Invalid field {field}: expected a positive integer"),
        )
    })?;

    if size == 0 {
        return Err(ProtocolError::new(
            ErrorCode::InvalidPayload,
            format!("Invalid field {field}: expected a positive integer"),
        ));
    }

    Ok(size)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn exposes_expected_name() {
        assert_eq!(crate_name(), CRATE_NAME);
    }

    #[test]
    fn request_envelope_round_trips_json() {
        let request = RequestEnvelope::new(
            "req_123",
            "workspace.create",
            json!({
                "name": "api",
                "rootDir": "D:\\src\\api",
                "shellProfile": "pwsh"
            }),
        );

        let serialized = serde_json::to_string(&request).expect("request should serialize");
        let parsed: RequestEnvelope =
            serde_json::from_str(&serialized).expect("request should deserialize");

        assert_eq!(parsed.protocol_version(), PROTOCOL_VERSION);
        assert_eq!(parsed.id(), "req_123");
        assert_eq!(parsed.envelope_type(), EnvelopeType::Command);
        assert_eq!(parsed.command(), "workspace.create");
        assert_eq!(parsed.payload()["rootDir"], "D:\\src\\api");
    }

    #[test]
    fn success_response_serializes_without_error_payload() {
        let response = ResponseEnvelope::success("req_123", json!({ "workspaceId": "ws_1" }));

        let serialized = serde_json::to_value(response).expect("response should serialize");

        assert_eq!(serialized["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(serialized["id"], "req_123");
        assert_eq!(serialized["type"], "response");
        assert_eq!(serialized["ok"], true);
        assert_eq!(serialized["result"]["workspaceId"], "ws_1");
        assert!(serialized.get("error").is_none());
    }

    #[test]
    fn success_response_round_trips_through_json() {
        let response = ResponseEnvelope::success("req_123", json!({ "workspaceId": "ws_1" }));
        let serialized = serde_json::to_string(&response).expect("response should serialize");
        let parsed: ResponseEnvelope =
            serde_json::from_str(&serialized).expect("response should deserialize");

        assert!(parsed.is_ok());
        assert_eq!(parsed.result(), Some(&json!({ "workspaceId": "ws_1" })));
        assert!(parsed.error().is_none());
    }

    #[test]
    fn error_response_serializes_without_result_payload() {
        let response = ResponseEnvelope::error(
            "req_123",
            ProtocolError::new(ErrorCode::InvalidPayload, "Missing required field: rootDir"),
        );

        let serialized = serde_json::to_value(response).expect("response should serialize");

        assert_eq!(serialized["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(serialized["id"], "req_123");
        assert_eq!(serialized["type"], "response");
        assert_eq!(serialized["ok"], false);
        assert_eq!(serialized["error"]["code"], "invalid_payload");
        assert_eq!(
            serialized["error"]["message"],
            "Missing required field: rootDir"
        );
        assert!(serialized.get("result").is_none());
    }

    #[test]
    fn error_response_round_trips_through_json() {
        let response = ResponseEnvelope::error(
            "req_123",
            ProtocolError::new(ErrorCode::InvalidPayload, "Missing required field: rootDir"),
        );
        let serialized = serde_json::to_string(&response).expect("response should serialize");
        let parsed: ResponseEnvelope =
            serde_json::from_str(&serialized).expect("response should deserialize");

        assert!(!parsed.is_ok());
        assert!(parsed.result().is_none());
        assert_eq!(
            parsed.error(),
            Some(&ErrorPayload {
                code: ErrorCode::InvalidPayload,
                message: "Missing required field: rootDir".into(),
            })
        );
    }

    #[test]
    fn response_envelope_round_trips_json_through_accessors() {
        let response: ResponseEnvelope = serde_json::from_str(
            r#"{
                "protocolVersion": 1,
                "id": "req_123",
                "type": "response",
                "ok": true,
                "result": {
                    "workspaceId": "ws_1"
                }
            }"#,
        )
        .expect("response should deserialize");

        assert!(response.is_ok());
        assert_eq!(response.result(), Some(&json!({ "workspaceId": "ws_1" })));
        assert!(response.error().is_none());
    }

    #[test]
    fn response_envelope_rejects_incoherent_json() {
        let err = serde_json::from_str::<ResponseEnvelope>(
            r#"{
                "protocolVersion": 1,
                "id": "req_123",
                "type": "response",
                "ok": true,
                "result": {
                    "workspaceId": "ws_1"
                },
                "error": {
                    "code": "invalid_payload",
                    "message": "should not be here"
                }
            }"#,
        )
        .expect_err("incoherent responses should fail to deserialize");

        assert!(err.to_string().contains("result"));
        assert!(err.to_string().contains("error"));
    }

    #[test]
    fn response_envelope_rejects_ok_without_result() {
        let err = serde_json::from_str::<ResponseEnvelope>(
            r#"{
                "protocolVersion": 1,
                "id": "req_123",
                "type": "response",
                "ok": true
            }"#,
        )
        .expect_err("successful responses should require a result payload");

        assert!(err.to_string().contains("result"));
    }

    #[test]
    fn response_envelope_rejects_failed_responses_without_error_payload() {
        let err = serde_json::from_str::<ResponseEnvelope>(
            r#"{
                "protocolVersion": 1,
                "id": "req_123",
                "type": "response",
                "ok": false
            }"#,
        )
        .expect_err("failed responses should require an error payload");

        assert!(err.to_string().contains("error"));
    }

    #[test]
    fn event_envelope_serializes_without_request_id() {
        let event = EventEnvelope::new(
            "workspace.updated",
            json!({
                "workspaceId": "ws_1"
            }),
        );

        let serialized = serde_json::to_value(event).expect("event should serialize");

        assert_eq!(serialized["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(serialized["type"], "event");
        assert_eq!(serialized["event"], "workspace.updated");
        assert_eq!(serialized["payload"]["workspaceId"], "ws_1");
        assert!(serialized.get("id").is_none());
    }

    #[test]
    fn event_envelope_round_trips_json() {
        let event: EventEnvelope = serde_json::from_str(
            r#"{
                "protocolVersion": 1,
                "type": "event",
                "event": "workspace.updated",
                "payload": {
                    "workspaceId": "ws_1"
                }
            }"#,
        )
        .expect("event should deserialize");

        assert_eq!(event.protocol_version, PROTOCOL_VERSION);
        assert_eq!(event.envelope_type, EnvelopeType::Event);
        assert_eq!(event.event, "workspace.updated");
        assert_eq!(event.payload["workspaceId"], "ws_1");
    }

    #[test]
    fn parse_and_validate_rejects_unsupported_protocol_version() {
        let err = parse_and_validate_request(
            r#"{
                "protocolVersion": 99,
                "id": "req_123",
                "type": "command",
                "command": "workspace.list",
                "payload": {}
            }"#,
        )
        .expect_err("unsupported protocol version should fail");

        assert_eq!(err.code, ErrorCode::InvalidVersion);
        assert!(err.message.contains("Unsupported protocol version"));
    }

    #[test]
    fn parse_and_validate_rejects_unsupported_commands() {
        let err = parse_and_validate_request(
            r#"{
                "protocolVersion": 1,
                "id": "req_123",
                "type": "command",
                "command": "workspace.destroyAll",
                "payload": {}
            }"#,
        )
        .expect_err("unknown commands should fail");

        assert_eq!(err.code, ErrorCode::Unsupported);
        assert!(err.message.contains("workspace.destroyAll"));
    }

    #[test]
    fn parse_and_validate_rejects_workspace_create_without_root_dir() {
        let err = parse_and_validate_request(
            r#"{
                "protocolVersion": 1,
                "id": "req_123",
                "type": "command",
                "command": "workspace.create",
                "payload": {
                    "name": "api",
                    "shellProfile": "pwsh"
                }
            }"#,
        )
        .expect_err("missing rootDir should fail");

        assert_eq!(err.code, ErrorCode::InvalidPayload);
        assert_eq!(err.message, "Missing required field: rootDir");
    }

    #[test]
    fn parse_and_validate_accepts_valid_workspace_create_requests() {
        let request = parse_and_validate_request(
            r#"{
                "protocolVersion": 1,
                "id": "req_123",
                "type": "command",
                "command": "workspace.create",
                "payload": {
                    "name": "api",
                    "rootDir": "D:\\src\\api",
                    "shellProfile": "pwsh"
                }
            }"#,
        )
        .expect("valid workspace.create should pass");

        assert_eq!(request.command(), "workspace.create");
        assert_eq!(request.payload()["rootDir"], "D:\\src\\api");
    }

    #[test]
    fn parse_and_validate_rejects_invalid_split_ratios() {
        let err = parse_and_validate_request(
            r#"{
                "protocolVersion": 1,
                "id": "req_123",
                "type": "command",
                "command": "pane.split",
                "payload": {
                    "workspaceId": "ws_1",
                    "paneId": "pane_1",
                    "newPaneId": "pane_2",
                    "orientation": "vertical",
                    "ratio": 1.0
                }
            }"#,
        )
        .expect_err("ratio outside bounds should fail");

        assert_eq!(err.code, ErrorCode::InvalidPayload);
        assert_eq!(
            err.message,
            "Invalid field ratio: expected a number between 0 and 1"
        );
    }

    #[test]
    fn parse_and_validate_rejects_zero_split_ratios() {
        let err = parse_and_validate_request(
            r#"{
                "protocolVersion": 1,
                "id": "req_123",
                "type": "command",
                "command": "pane.split",
                "payload": {
                    "workspaceId": "ws_1",
                    "paneId": "pane_1",
                    "newPaneId": "pane_2",
                    "orientation": "vertical",
                    "ratio": 0.0
                }
            }"#,
        )
        .expect_err("zero ratio should fail");

        assert_eq!(err.code, ErrorCode::InvalidPayload);
        assert_eq!(
            err.message,
            "Invalid field ratio: expected a number between 0 and 1"
        );
    }

    #[test]
    fn parse_and_validate_accepts_valid_split_requests() {
        let request = parse_and_validate_request(
            r#"{
                "protocolVersion": 1,
                "id": "req_123",
                "type": "command",
                "command": "pane.split",
                "payload": {
                    "workspaceId": "ws_1",
                    "paneId": "pane_1",
                    "newPaneId": "pane_2",
                    "orientation": "horizontal",
                    "ratio": 0.5
                }
            }"#,
        )
        .expect("valid pane.split should pass");

        assert_eq!(request.command(), "pane.split");
        assert_eq!(request.payload()["orientation"], "horizontal");
    }

    #[test]
    fn parse_and_validate_accepts_valid_pane_focus_requests() {
        let request = parse_and_validate_request(
            r#"{
                "protocolVersion": 1,
                "id": "req_124",
                "type": "command",
                "command": "pane.focus",
                "payload": {
                    "workspaceId": "ws_1",
                    "paneId": "pane_2"
                }
            }"#,
        )
        .expect("valid pane.focus should pass");

        assert_eq!(request.command(), "pane.focus");
    }

    #[test]
    fn parse_and_validate_accepts_valid_pane_close_requests() {
        let request = parse_and_validate_request(
            r#"{
                "protocolVersion": 1,
                "id": "req_125",
                "type": "command",
                "command": "pane.close",
                "payload": {
                    "workspaceId": "ws_1",
                    "paneId": "pane_2"
                }
            }"#,
        )
        .expect("valid pane.close should pass");

        assert_eq!(request.command(), "pane.close");
    }

    #[test]
    fn parse_and_validate_rejects_malformed_json() {
        let err = parse_and_validate_request("{ not valid json }")
            .expect_err("malformed json should fail");

        assert_eq!(err.code, ErrorCode::InvalidPayload);
        assert!(err.message.contains("Malformed request JSON"));
    }

    #[test]
    fn parse_and_validate_rejects_non_command_envelope_types() {
        let err = parse_and_validate_request(
            r#"{
                "protocolVersion": 1,
                "id": "req_123",
                "type": "response",
                "command": "workspace.list",
                "payload": {}
            }"#,
        )
        .expect_err("non-command envelopes should fail");

        assert_eq!(err.code, ErrorCode::InvalidPayload);
        assert_eq!(err.message, "Request envelope type must be command");
    }

    #[test]
    fn parse_and_validate_rejects_blank_request_ids() {
        let err = parse_and_validate_request(
            r#"{
                "protocolVersion": 1,
                "id": "   ",
                "type": "command",
                "command": "workspace.list",
                "payload": {}
            }"#,
        )
        .expect_err("blank ids should fail");

        assert_eq!(err.code, ErrorCode::InvalidPayload);
        assert_eq!(err.message, "Missing required field: id");
    }

    #[test]
    fn parse_and_validate_rejects_invalid_notify_levels() {
        let err = parse_and_validate_request(
            r#"{
                "protocolVersion": 1,
                "id": "req_123",
                "type": "command",
                "command": "notify.send",
                "payload": {
                    "title": "Build finished",
                    "body": "All tests passed",
                    "level": "loud",
                    "workspaceId": "ws_1"
                }
            }"#,
        )
        .expect_err("invalid notify level should fail");

        assert_eq!(err.code, ErrorCode::InvalidPayload);
        assert_eq!(
            err.message,
            "Invalid field level: expected info, success, warning, or error"
        );
    }

    #[test]
    fn parse_and_validate_accepts_valid_notify_requests() {
        let request = parse_and_validate_request(
            r#"{
                "protocolVersion": 1,
                "id": "req_123",
                "type": "command",
                "command": "notify.send",
                "payload": {
                    "title": "Build finished",
                    "body": "All tests passed",
                    "level": "success",
                    "workspaceId": null
                }
            }"#,
        )
        .expect("valid notify.send should pass");

        assert_eq!(request.command(), "notify.send");
        assert!(request.payload()["workspaceId"].is_null());
    }

    #[test]
    fn parse_and_validate_rejects_blank_notify_workspace_ids() {
        let err = parse_and_validate_request(
            r#"{
                "protocolVersion": 1,
                "id": "req_123",
                "type": "command",
                "command": "notify.send",
                "payload": {
                    "title": "Build finished",
                    "body": "All tests passed",
                    "level": "success",
                    "workspaceId": "   "
                }
            }"#,
        )
        .expect_err("blank workspace ids should fail");

        assert_eq!(err.code, ErrorCode::InvalidPayload);
        assert_eq!(
            err.message,
            "Invalid field workspaceId: expected a non-empty string"
        );
    }

    #[test]
    fn parse_and_validate_accepts_valid_session_start_requests() {
        let request = parse_and_validate_request(
            r#"{
                "protocolVersion": 1,
                "id": "req_123",
                "type": "command",
                "command": "session.start",
                "payload": {
                    "workspaceId": "ws_1",
                    "paneId": "pane_1",
                    "shellProfile": "cmd.exe",
                    "cols": 120,
                    "rows": 40
                }
            }"#,
        )
        .expect("valid session.start should pass");

        assert_eq!(request.command(), "session.start");
        assert_eq!(request.payload()["shellProfile"], "cmd.exe");
    }

    #[test]
    fn parse_and_validate_rejects_zero_sized_session_start_requests() {
        let err = parse_and_validate_request(
            r#"{
                "protocolVersion": 1,
                "id": "req_123",
                "type": "command",
                "command": "session.start",
                "payload": {
                    "workspaceId": "ws_1",
                    "paneId": "pane_1",
                    "shellProfile": "cmd.exe",
                    "cols": 0,
                    "rows": 40
                }
            }"#,
        )
        .expect_err("cols=0 should fail");

        assert_eq!(err.code, ErrorCode::InvalidPayload);
        assert_eq!(err.message, "Invalid field cols: expected a positive integer");
    }

    #[test]
    fn parse_and_validate_accepts_empty_session_input_payloads() {
        let request = parse_and_validate_request(
            r#"{
                "protocolVersion": 1,
                "id": "req_123",
                "type": "command",
                "command": "session.sendInput",
                "payload": {
                    "sessionId": "session:1",
                    "data": ""
                }
            }"#,
        )
        .expect("empty input should still be valid");

        assert_eq!(request.command(), "session.sendInput");
        assert_eq!(request.payload()["sessionId"], "session:1");
    }

    #[test]
    fn parse_and_validate_rejects_blank_session_status_ids() {
        let err = parse_and_validate_request(
            r#"{
                "protocolVersion": 1,
                "id": "req_123",
                "type": "command",
                "command": "session.getStatus",
                "payload": {
                    "sessionId": "   "
                }
            }"#,
        )
        .expect_err("blank session ids should fail");

        assert_eq!(err.code, ErrorCode::InvalidPayload);
        assert_eq!(err.message, "Invalid field sessionId: expected a non-empty string");
    }

    #[test]
    fn dispatch_keeps_session_commands_unsupported_until_runtime_handles_them() {
        let mut registry = WorkspaceRegistry::new();
        let request = RequestEnvelope::new(
            "r-session",
            "session.start",
            json!({
                "workspaceId": "ws_1",
                "paneId": "pane_1",
                "shellProfile": "cmd.exe",
                "cols": 80,
                "rows": 24
            }),
        );

        let response = dispatch(&request, &mut registry);

        assert!(!response.is_ok());
        assert_eq!(
            response.error(),
            Some(&ErrorPayload {
                code: ErrorCode::Unsupported,
                message: "Unsupported command: session.start".into(),
            })
        );
    }

    #[test]
    fn parse_and_validate_accepts_valid_workspace_rename_requests() {
        let request = parse_and_validate_request(
            r#"{
                "protocolVersion": 1,
                "id": "req_123",
                "type": "command",
                "command": "workspace.rename",
                "payload": {
                    "workspaceId": "ws_1",
                    "name": "api-renamed"
                }
            }"#,
        )
        .expect("workspace.rename should validate");

        assert_eq!(request.command(), "workspace.rename");
    }

    #[test]
    fn parse_and_validate_rejects_workspace_rename_without_workspace_id() {
        let err = parse_and_validate_request(
            r#"{
                "protocolVersion": 1,
                "id": "req_123",
                "type": "command",
                "command": "workspace.rename",
                "payload": {
                    "name": "api-renamed"
                }
            }"#,
        )
        .expect_err("workspace.rename should require workspaceId");

        assert_eq!(err.code, ErrorCode::InvalidPayload);
    }

    #[test]
    fn parse_and_validate_rejects_workspace_rename_with_blank_name() {
        let err = parse_and_validate_request(
            r#"{
                "protocolVersion": 1,
                "id": "req_123",
                "type": "command",
                "command": "workspace.rename",
                "payload": {
                    "workspaceId": "ws_1",
                    "name": "   "
                }
            }"#,
        )
        .expect_err("workspace.rename should reject blank names");

        assert_eq!(err.code, ErrorCode::InvalidPayload);
    }

    #[test]
    fn parse_and_validate_accepts_valid_workspace_close_requests() {
        let request = parse_and_validate_request(
            r#"{
                "protocolVersion": 1,
                "id": "req_close",
                "type": "command",
                "command": "workspace.close",
                "payload": {
                    "workspaceId": "ws-inbox"
                }
            }"#,
        )
        .expect("valid workspace.close should pass");

        assert_eq!(request.command(), "workspace.close");
        assert_eq!(request.payload()["workspaceId"], "ws-inbox");
    }

    #[test]
    fn parse_and_validate_rejects_workspace_close_without_workspace_id() {
        let err = parse_and_validate_request(
            r#"{
                "protocolVersion": 1,
                "id": "req_close",
                "type": "command",
                "command": "workspace.close",
                "payload": {}
            }"#,
        )
        .expect_err("missing workspaceId should fail");

        assert_eq!(err.code, ErrorCode::InvalidPayload);
        assert_eq!(err.message, "Missing required field: workspaceId");
    }

    #[test]
    fn parse_and_validate_rejects_workspace_close_with_blank_workspace_id() {
        let err = parse_and_validate_request(
            r#"{
                "protocolVersion": 1,
                "id": "req_close",
                "type": "command",
                "command": "workspace.close",
                "payload": {
                    "workspaceId": "  "
                }
            }"#,
        )
        .expect_err("blank workspaceId should fail");

        assert_eq!(err.code, ErrorCode::InvalidPayload);
    }
}

// ── Command dispatch ─────────────────────────────────────────────────────────

static WORKSPACE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

fn new_workspace_id() -> String {
    let n = WORKSPACE_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("ws-{n:016x}")
}

/// Dispatch a validated `RequestEnvelope` against the provided registry and
/// return the corresponding `ResponseEnvelope`.
pub fn dispatch(request: &RequestEnvelope, registry: &mut WorkspaceRegistry) -> ResponseEnvelope {
    if let Err(err) = validate_request(request) {
        return ResponseEnvelope::error(request.id(), err);
    }

    match request.command() {
        "workspace.create" => handle_workspace_create(request, registry),
        "workspace.close" => handle_workspace_close(request, registry),
        "workspace.rename" => handle_workspace_rename(request, registry),
        "pane.split" => handle_pane_split(request, registry),
        "pane.close" => handle_pane_close(request, registry),
        "pane.focus" => handle_pane_focus(request, registry),
        "notify.send" => handle_notify_send(request),
        cmd => ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(ErrorCode::Unsupported, format!("Unsupported command: {cmd}")),
        ),
    }
}

/// Parse, validate, dispatch, and serialize in one call.
/// Always returns well-formed JSON. Best-effort request-id echo on error.
pub fn dispatch_raw(input: &str, registry: &mut WorkspaceRegistry) -> String {
    let response = match parse_and_validate_request(input) {
        Ok(request) => dispatch(&request, registry),
        Err(err) => {
            let id = serde_json::from_str::<Value>(input)
                .ok()
                .and_then(|v| v.get("id")?.as_str().map(String::from))
                .unwrap_or_default();
            ResponseEnvelope::error(id, err)
        }
    };
    serde_json::to_string(&response)
        .unwrap_or_else(|_| r#"{"ok":false,"error":{"code":"internal_error","message":"serialize failed"}}"#.to_string())
}

// ── Unified runtime dispatch ──────────────────────────────────────────────

/// Unified runtime context for dispatching all command types.
/// Holds both the workspace registry and the live session registry.
pub struct RuntimeContext<'a, F: SessionHostFactory> {
    pub registry: &'a mut WorkspaceRegistry,
    pub sessions: &'a mut LiveSessionRegistry<F>,
}

/// Dispatch any command through the unified runtime context.
/// Handles workspace, pane, session, and app commands.
pub fn dispatch_runtime<F: SessionHostFactory>(
    request: &RequestEnvelope,
    ctx: &mut RuntimeContext<'_, F>,
) -> ResponseEnvelope {
    if let Err(err) = validate_request(request) {
        return ResponseEnvelope::error(request.id(), err);
    }
    match request.command() {
        // Workspace and pane commands delegate to existing dispatch
        "workspace.create" | "workspace.close" | "workspace.rename"
        | "pane.split" | "pane.close" | "pane.focus"
        | "notify.send" => dispatch(request, ctx.registry),

        // Session commands
        "session.sendInput" => handle_session_send_input(request, ctx),
        "session.resize" => handle_session_resize(request, ctx),
        "session.restart" => handle_session_restart(request, ctx),
        "session.getStatus" => handle_session_get_status(request, ctx),

        _ => ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(ErrorCode::Unsupported, format!("Unknown command: {}", request.command())),
        ),
    }
}

/// Parse, validate, dispatch via the unified runtime, and serialize in one call.
/// Always returns well-formed JSON. Best-effort request-id echo on error.
pub fn dispatch_runtime_raw<F: SessionHostFactory>(
    input: &str,
    ctx: &mut RuntimeContext<'_, F>,
) -> String {
    let response = match parse_and_validate_request(input) {
        Ok(request) => dispatch_runtime(&request, ctx),
        Err(err) => {
            let id = serde_json::from_str::<Value>(input)
                .ok()
                .and_then(|v| v.get("id")?.as_str().map(String::from))
                .unwrap_or_default();
            ResponseEnvelope::error(id, err)
        }
    };
    serde_json::to_string(&response)
        .unwrap_or_else(|_| r#"{"ok":false,"error":{"code":"internal_error","message":"serialize failed"}}"#.to_string())
}

fn session_runtime_error_response(id: &str, error: SessionRuntimeError) -> ResponseEnvelope {
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

fn handle_session_send_input<F: SessionHostFactory>(
    request: &RequestEnvelope,
    ctx: &mut RuntimeContext<'_, F>,
) -> ResponseEnvelope {
    let payload = request.payload();
    let session_id = payload["sessionId"].as_str().unwrap_or_default();
    let data = payload["data"].as_str().unwrap_or_default();

    match ctx.sessions.send_input(session_id, data.as_bytes()) {
        Ok(()) => ResponseEnvelope::success(
            request.id(),
            serde_json::json!({
                "delivered": true,
                "sessionId": session_id,
            }),
        ),
        Err(error) => session_runtime_error_response(request.id(), error),
    }
}

fn handle_session_resize<F: SessionHostFactory>(
    request: &RequestEnvelope,
    ctx: &mut RuntimeContext<'_, F>,
) -> ResponseEnvelope {
    let payload = request.payload();
    let session_id = payload["sessionId"].as_str().unwrap_or_default();
    let cols = payload["cols"].as_u64().unwrap_or(80) as u16;
    let rows = payload["rows"].as_u64().unwrap_or(24) as u16;

    match ctx.sessions.resize(session_id, TerminalSize { rows, cols }) {
        Ok(()) => ResponseEnvelope::success(
            request.id(),
            serde_json::json!({
                "resized": true,
                "sessionId": session_id,
            }),
        ),
        Err(error) => session_runtime_error_response(request.id(), error),
    }
}

fn handle_session_restart<F: SessionHostFactory>(
    request: &RequestEnvelope,
    ctx: &mut RuntimeContext<'_, F>,
) -> ResponseEnvelope {
    let session_id = request.payload()["sessionId"].as_str().unwrap_or_default();

    match ctx.sessions.restart(session_id) {
        Ok(restarted_session_id) => match ctx.sessions.get_status(&restarted_session_id) {
            Ok(snapshot) => ResponseEnvelope::success(
                request.id(),
                serde_json::json!({
                    "sessionId": restarted_session_id,
                    "workspaceId": snapshot.workspace_id,
                    "paneId": snapshot.pane_id,
                }),
            ),
            Err(error) => session_runtime_error_response(request.id(), error),
        },
        Err(error) => session_runtime_error_response(request.id(), error),
    }
}

fn handle_session_get_status<F: SessionHostFactory>(
    request: &RequestEnvelope,
    ctx: &mut RuntimeContext<'_, F>,
) -> ResponseEnvelope {
    let session_id = request.payload()["sessionId"].as_str().unwrap_or_default();

    match ctx.sessions.get_status(session_id) {
        Ok(snapshot) => ResponseEnvelope::success(
            request.id(),
            serde_json::json!({
                "sessionId": snapshot.session_id,
                "workspaceId": snapshot.workspace_id,
                "paneId": snapshot.pane_id,
                "command": snapshot.command,
                "status": snapshot.status,
                "exitCode": snapshot.exit_code,
                "output": snapshot.output,
            }),
        ),
        Err(error) => session_runtime_error_response(request.id(), error),
    }
}

// ── Workspace / pane / notify dispatch handlers ──────────────────────────

fn handle_workspace_create(
    request: &RequestEnvelope,
    registry: &mut WorkspaceRegistry,
) -> ResponseEnvelope {
    let p = request.payload();
    let name = p["name"].as_str().unwrap_or_default();
    let root_dir = p["rootDir"].as_str().unwrap_or_default();
    let shell_profile = p["shellProfile"].as_str().unwrap_or_default();

    let id = new_workspace_id();
    match registry.create(&id, name, root_dir, shell_profile, WorkspaceLayout::starter()) {
        Ok(()) => ResponseEnvelope::success(request.id(), serde_json::json!({ "workspaceId": id })),
        Err(WorkspaceError::DuplicateWorkspaceId) => ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(ErrorCode::Conflict, "Workspace ID conflict; try again"),
        ),
        Err(e) => ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(ErrorCode::InternalError, format!("{e:?}")),
        ),
    }
}

fn handle_workspace_close(
    request: &RequestEnvelope,
    registry: &mut WorkspaceRegistry,
) -> ResponseEnvelope {
    let p = request.payload();
    let workspace_id = p["workspaceId"].as_str().unwrap_or_default();

    match registry.remove_workspace(workspace_id) {
        Ok(()) => ResponseEnvelope::success(
            request.id(),
            serde_json::json!({ "workspaceId": workspace_id }),
        ),
        Err(WorkspaceError::WorkspaceNotFound) => ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(
                ErrorCode::NotFound,
                format!("Workspace not found: {workspace_id}"),
            ),
        ),
        Err(e) => ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(ErrorCode::InternalError, format!("{e:?}")),
        ),
    }
}

fn handle_workspace_rename(
    request: &RequestEnvelope,
    registry: &mut WorkspaceRegistry,
) -> ResponseEnvelope {
    let p = request.payload();
    let workspace_id = p["workspaceId"].as_str().unwrap_or_default();
    let name = p["name"].as_str().unwrap_or_default();

    match registry.rename_workspace(workspace_id, name) {
        Ok(()) => ResponseEnvelope::success(
            request.id(),
            serde_json::json!({ "workspaceId": workspace_id }),
        ),
        Err(WorkspaceError::WorkspaceNotFound) => ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(
                ErrorCode::NotFound,
                format!("Workspace not found: {workspace_id}"),
            ),
        ),
        Err(WorkspaceError::InvalidWorkspaceName) => ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(ErrorCode::InvalidPayload, "Invalid workspace name"),
        ),
        Err(e) => ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(ErrorCode::InternalError, format!("{e:?}")),
        ),
    }
}

fn handle_pane_split(
    request: &RequestEnvelope,
    registry: &mut WorkspaceRegistry,
) -> ResponseEnvelope {
    let p = request.payload();
    let workspace_id = p["workspaceId"].as_str().unwrap_or_default();
    let pane_id = p["paneId"].as_str().unwrap_or_default();
    let new_pane_id = p["newPaneId"].as_str().unwrap_or_default();
    let orientation = match p["orientation"].as_str().unwrap_or("horizontal") {
        "vertical" => SplitOrientation::Vertical,
        _ => SplitOrientation::Horizontal,
    };
    let ratio = p["ratio"].as_f64().unwrap_or(0.5);

    match registry.split_pane(workspace_id, pane_id, new_pane_id, orientation, ratio) {
        Ok(()) => ResponseEnvelope::success(
            request.id(),
            serde_json::json!({ "workspaceId": workspace_id, "newPaneId": new_pane_id }),
        ),
        Err(WorkspaceError::WorkspaceNotFound) => ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(
                ErrorCode::NotFound,
                format!("Workspace not found: {workspace_id}"),
            ),
        ),
        Err(WorkspaceError::Layout(LayoutError::PaneNotFound)) => ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(ErrorCode::NotFound, format!("Pane not found: {pane_id}")),
        ),
        Err(WorkspaceError::Layout(LayoutError::DuplicatePaneId)) => ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(
                ErrorCode::Conflict,
                format!("Pane ID already exists: {new_pane_id}"),
            ),
        ),
        Err(e) => ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(ErrorCode::InternalError, format!("{e:?}")),
        ),
    }
}

fn handle_notify_send(request: &RequestEnvelope) -> ResponseEnvelope {
    let p = request.payload();
    let title = p["title"].as_str().unwrap_or_default();
    let body = p["body"].as_str().unwrap_or_default();
    let level = p["level"].as_str().unwrap_or("info");
    ResponseEnvelope::success(
        request.id(),
        serde_json::json!({
            "queued": true,
            "title": title,
            "body": body,
            "level": level,
        }),
    )
}

fn handle_pane_close(
    request: &RequestEnvelope,
    registry: &mut WorkspaceRegistry,
) -> ResponseEnvelope {
    let p = request.payload();
    let workspace_id = p["workspaceId"].as_str().unwrap_or_default();
    let pane_id = p["paneId"].as_str().unwrap_or_default();

    match registry.close_pane(workspace_id, pane_id) {
        Ok(()) => ResponseEnvelope::success(
            request.id(),
            serde_json::json!({ "workspaceId": workspace_id, "paneId": pane_id }),
        ),
        Err(WorkspaceError::WorkspaceNotFound) => ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(
                ErrorCode::NotFound,
                format!("Workspace not found: {workspace_id}"),
            ),
        ),
        Err(WorkspaceError::Layout(LayoutError::PaneNotFound)) => ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(ErrorCode::NotFound, format!("Pane not found: {pane_id}")),
        ),
        Err(WorkspaceError::Layout(LayoutError::WouldEmptyWorkspace)) => ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(ErrorCode::Conflict, "Cannot close the last pane"),
        ),
        Err(e) => ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(ErrorCode::InternalError, format!("{e:?}")),
        ),
    }
}

fn handle_pane_focus(
    request: &RequestEnvelope,
    registry: &mut WorkspaceRegistry,
) -> ResponseEnvelope {
    let p = request.payload();
    let workspace_id = p["workspaceId"].as_str().unwrap_or_default();
    let pane_id = p["paneId"].as_str().unwrap_or_default();

    match registry.focus_pane(workspace_id, pane_id) {
        Ok(()) => ResponseEnvelope::success(
            request.id(),
            serde_json::json!({ "workspaceId": workspace_id, "paneId": pane_id }),
        ),
        Err(WorkspaceError::WorkspaceNotFound) => ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(
                ErrorCode::NotFound,
                format!("Workspace not found: {workspace_id}"),
            ),
        ),
        Err(WorkspaceError::Layout(LayoutError::PaneNotFound)) => ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(ErrorCode::NotFound, format!("Pane not found: {pane_id}")),
        ),
        Err(e) => ResponseEnvelope::error(
            request.id(),
            ProtocolError::new(ErrorCode::InternalError, format!("{e:?}")),
        ),
    }
}

#[cfg(test)]
mod handler_tests {
    use super::*;
    use serde_json::json;

    fn inbox_registry() -> WorkspaceRegistry {
        core_state::starter_workspace_registry()
    }

    // ── workspace.create ──────────────────────────────────────────────────

    #[test]
    fn workspace_create_inserts_workspace_and_returns_id() {
        let mut reg = WorkspaceRegistry::new();
        let req = RequestEnvelope::new(
            "r1",
            "workspace.create",
            json!({ "name": "myws", "rootDir": "/tmp/myws", "shellProfile": "pwsh" }),
        );
        let resp = dispatch(&req, &mut reg);

        assert!(resp.is_ok());
        let id = resp.result().unwrap()["workspaceId"].as_str().unwrap();
        assert!(!id.is_empty());
        assert_eq!(reg.list().len(), 1);
        assert_eq!(reg.list()[0].name, "myws");
        assert_eq!(reg.list()[0].root_dir, "/tmp/myws");
    }

    #[test]
    fn workspace_create_returned_id_matches_registry_entry() {
        let mut reg = WorkspaceRegistry::new();
        let req = RequestEnvelope::new(
            "r2",
            "workspace.create",
            json!({ "name": "proj", "rootDir": "D:\\src\\proj", "shellProfile": "bash" }),
        );
        let resp = dispatch(&req, &mut reg);

        let id = resp.result().unwrap()["workspaceId"].as_str().unwrap();
        assert_eq!(reg.list()[0].id, id);
    }

    #[test]
    fn workspace_create_new_workspace_starts_with_single_pane_layout() {
        let mut reg = WorkspaceRegistry::new();
        let req = RequestEnvelope::new(
            "r3",
            "workspace.create",
            json!({ "name": "ws", "rootDir": "/", "shellProfile": "sh" }),
        );
        dispatch(&req, &mut reg);

        let snapshot = reg.list()[0].layout.snapshot();
        assert_eq!(snapshot.pane_count, 1);
        assert_eq!(snapshot.split_count, 0);
    }

    // ── pane.split ────────────────────────────────────────────────────────

    #[test]
    fn pane_split_mutates_correct_workspace() {
        let mut reg = inbox_registry();
        let req = RequestEnvelope::new(
            "r4",
            "pane.split",
            json!({
                "workspaceId": "ws-inbox",
                "paneId": "pane-1",
                "newPaneId": "pane-2",
                "orientation": "vertical",
                "ratio": 0.5
            }),
        );
        let resp = dispatch(&req, &mut reg);

        assert!(resp.is_ok());
        assert_eq!(resp.result().unwrap()["newPaneId"], "pane-2");
        assert_eq!(reg.list()[0].layout.snapshot().pane_count, 2);
        assert_eq!(reg.list()[0].layout.snapshot().split_count, 1);
    }

    #[test]
    fn pane_split_returns_not_found_for_unknown_workspace() {
        let mut reg = WorkspaceRegistry::new();
        let req = RequestEnvelope::new(
            "r5",
            "pane.split",
            json!({
                "workspaceId": "ws-nope",
                "paneId": "pane-1",
                "newPaneId": "pane-2",
                "orientation": "horizontal",
                "ratio": 0.5
            }),
        );
        let resp = dispatch(&req, &mut reg);

        assert!(!resp.is_ok());
        assert_eq!(resp.error().unwrap().code, ErrorCode::NotFound);
        assert!(resp.error().unwrap().message.contains("ws-nope"));
    }

    #[test]
    fn pane_split_returns_not_found_for_unknown_pane() {
        let mut reg = inbox_registry();
        let req = RequestEnvelope::new(
            "r6",
            "pane.split",
            json!({
                "workspaceId": "ws-inbox",
                "paneId": "pane-missing",
                "newPaneId": "pane-new",
                "orientation": "vertical",
                "ratio": 0.4
            }),
        );
        let resp = dispatch(&req, &mut reg);

        assert!(!resp.is_ok());
        assert_eq!(resp.error().unwrap().code, ErrorCode::NotFound);
    }

    #[test]
    fn pane_split_returns_conflict_for_duplicate_new_pane_id() {
        let mut reg = inbox_registry();
        // First split succeeds
        let req1 = RequestEnvelope::new(
            "r7a",
            "pane.split",
            json!({
                "workspaceId": "ws-inbox",
                "paneId": "pane-1",
                "newPaneId": "pane-2",
                "orientation": "vertical",
                "ratio": 0.5
            }),
        );
        dispatch(&req1, &mut reg);

        // Second split with same new pane ID fails
        let req2 = RequestEnvelope::new(
            "r7b",
            "pane.split",
            json!({
                "workspaceId": "ws-inbox",
                "paneId": "pane-1",
                "newPaneId": "pane-2",
                "orientation": "horizontal",
                "ratio": 0.5
            }),
        );
        let resp = dispatch(&req2, &mut reg);

        assert!(!resp.is_ok());
        assert_eq!(resp.error().unwrap().code, ErrorCode::Conflict);
    }

    #[test]
    fn pane_focus_updates_the_workspace_focus_target() {
        let mut reg = inbox_registry();
        let split = RequestEnvelope::new(
            "r7c",
            "pane.split",
            json!({
                "workspaceId": "ws-inbox",
                "paneId": "pane-1",
                "newPaneId": "pane-2",
                "orientation": "vertical",
                "ratio": 0.5
            }),
        );
        dispatch(&split, &mut reg);

        let req = RequestEnvelope::new(
            "r7d",
            "pane.focus",
            json!({
                "workspaceId": "ws-inbox",
                "paneId": "pane-1"
            }),
        );
        let resp = dispatch(&req, &mut reg);

        assert!(resp.is_ok());
        assert_eq!(reg.list()[0].layout.focused_pane_id(), "pane-1");
    }

    #[test]
    fn pane_focus_returns_not_found_for_unknown_pane() {
        let mut reg = inbox_registry();

        let req = RequestEnvelope::new(
            "r7d-missing",
            "pane.focus",
            json!({
                "workspaceId": "ws-inbox",
                "paneId": "pane-missing"
            }),
        );
        let resp = dispatch(&req, &mut reg);

        assert!(!resp.is_ok());
        assert_eq!(resp.error().unwrap().code, ErrorCode::NotFound);
    }

    #[test]
    fn pane_close_removes_the_target_pane() {
        let mut reg = inbox_registry();
        let split = RequestEnvelope::new(
            "r7e",
            "pane.split",
            json!({
                "workspaceId": "ws-inbox",
                "paneId": "pane-1",
                "newPaneId": "pane-2",
                "orientation": "vertical",
                "ratio": 0.5
            }),
        );
        dispatch(&split, &mut reg);

        let req = RequestEnvelope::new(
            "r7f",
            "pane.close",
            json!({
                "workspaceId": "ws-inbox",
                "paneId": "pane-2"
            }),
        );
        let resp = dispatch(&req, &mut reg);

        assert!(resp.is_ok());
        assert_eq!(reg.list()[0].layout.panes().len(), 1);
        assert_eq!(reg.list()[0].layout.panes()[0].pane_id, "pane-1");
    }

    // ── workspace.close ─────────────────────────────────────────────────

    #[test]
    fn workspace_close_removes_workspace_from_registry() {
        let mut reg = inbox_registry();
        let req = RequestEnvelope::new(
            "rc1",
            "workspace.close",
            json!({ "workspaceId": "ws-inbox" }),
        );
        let resp = dispatch(&req, &mut reg);

        assert!(resp.is_ok());
        assert_eq!(resp.result().unwrap()["workspaceId"], "ws-inbox");
        assert!(reg.list().is_empty());
    }

    #[test]
    fn workspace_close_returns_not_found_for_unknown_workspace() {
        let mut reg = inbox_registry();
        let req = RequestEnvelope::new(
            "rc2",
            "workspace.close",
            json!({ "workspaceId": "ws-missing" }),
        );
        let resp = dispatch(&req, &mut reg);

        assert!(!resp.is_ok());
        assert_eq!(resp.error().unwrap().code, ErrorCode::NotFound);
        assert!(resp.error().unwrap().message.contains("ws-missing"));
    }

    #[test]
    fn workspace_close_preserves_remaining_workspaces() {
        let mut reg = inbox_registry();
        // Create a second workspace
        let create = RequestEnvelope::new(
            "rc3a",
            "workspace.create",
            json!({ "name": "api", "rootDir": "/tmp/api", "shellProfile": "bash" }),
        );
        let create_resp = dispatch(&create, &mut reg);
        assert!(create_resp.is_ok());
        let api_id = create_resp.result().unwrap()["workspaceId"]
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(reg.list().len(), 2);

        // Close the inbox workspace
        let close = RequestEnvelope::new(
            "rc3b",
            "workspace.close",
            json!({ "workspaceId": "ws-inbox" }),
        );
        let resp = dispatch(&close, &mut reg);

        assert!(resp.is_ok());
        assert_eq!(reg.list().len(), 1);
        assert_eq!(reg.list()[0].id, api_id);
    }

    #[test]
    fn workspace_close_is_idempotent_returns_not_found_on_second_call() {
        let mut reg = inbox_registry();
        let req = RequestEnvelope::new(
            "rc4",
            "workspace.close",
            json!({ "workspaceId": "ws-inbox" }),
        );
        let first = dispatch(&req, &mut reg);
        assert!(first.is_ok());

        let second = dispatch(&req, &mut reg);
        assert!(!second.is_ok());
        assert_eq!(second.error().unwrap().code, ErrorCode::NotFound);
    }

    #[test]
    fn workspace_rename_updates_name_and_returns_workspace_id() {
        let mut reg = inbox_registry();
        let req = RequestEnvelope::new(
            "rr1",
            "workspace.rename",
            json!({ "workspaceId": "ws-inbox", "name": "Inbox Prime" }),
        );

        let resp = dispatch(&req, &mut reg);

        assert!(resp.is_ok());
        assert_eq!(resp.result().unwrap()["workspaceId"], "ws-inbox");
        assert_eq!(reg.list()[0].name, "Inbox Prime");
    }

    #[test]
    fn workspace_rename_returns_not_found_for_unknown_workspace() {
        let mut reg = inbox_registry();
        let req = RequestEnvelope::new(
            "rr2",
            "workspace.rename",
            json!({ "workspaceId": "ws-missing", "name": "Inbox Prime" }),
        );

        let resp = dispatch(&req, &mut reg);

        assert!(!resp.is_ok());
        assert_eq!(resp.error().unwrap().code, ErrorCode::NotFound);
    }

    // ── notify.send ───────────────────────────────────────────────────────

    #[test]
    fn notify_send_returns_queued_true_with_echoed_fields() {
        let mut reg = WorkspaceRegistry::new();
        let req = RequestEnvelope::new(
            "r8",
            "notify.send",
            json!({ "title": "Build done", "body": "All green", "level": "success" }),
        );
        let resp = dispatch(&req, &mut reg);

        assert!(resp.is_ok());
        let result = resp.result().unwrap();
        assert_eq!(result["queued"], true);
        assert_eq!(result["title"], "Build done");
        assert_eq!(result["body"], "All green");
        assert_eq!(result["level"], "success");
    }

    #[test]
    fn notify_send_does_not_mutate_registry() {
        let mut reg = inbox_registry();
        let initial_len = reg.list().len();
        let req = RequestEnvelope::new(
            "r9",
            "notify.send",
            json!({ "title": "Hi", "body": "there", "level": "info" }),
        );
        dispatch(&req, &mut reg);

        assert_eq!(reg.list().len(), initial_len);
    }

    // ── dispatch_raw ──────────────────────────────────────────────────────

    #[test]
    fn dispatch_raw_handles_invalid_json() {
        let mut reg = WorkspaceRegistry::new();
        let out = dispatch_raw("not json at all", &mut reg);
        let v: Value = serde_json::from_str(&out).expect("output should be valid JSON");
        assert_eq!(v["ok"], false);
        assert_eq!(v["error"]["code"], "invalid_payload");
    }

    #[test]
    fn dispatch_raw_echoes_request_id_on_success() {
        let mut reg = WorkspaceRegistry::new();
        let input = r#"{"protocolVersion":1,"id":"pipe-req-1","type":"command","command":"notify.send","payload":{"title":"T","body":"B","level":"info"}}"#;
        let out = dispatch_raw(input, &mut reg);
        let v: Value = serde_json::from_str(&out).expect("output should be valid JSON");
        assert_eq!(v["ok"], true);
        assert_eq!(v["id"], "pipe-req-1");
    }

    #[test]
    fn dispatch_raw_echoes_request_id_on_validation_error() {
        let mut reg = WorkspaceRegistry::new();
        let input = r#"{"protocolVersion":1,"id":"bad-req","type":"command","command":"workspace.create","payload":{}}"#;
        let out = dispatch_raw(input, &mut reg);
        let v: Value = serde_json::from_str(&out).expect("output should be valid JSON");
        assert_eq!(v["ok"], false);
        assert_eq!(v["id"], "bad-req");
    }

    #[test]
    fn dispatch_raw_full_workspace_create_roundtrip() {
        let mut reg = WorkspaceRegistry::new();
        let input = r#"{"protocolVersion":1,"id":"x1","type":"command","command":"workspace.create","payload":{"name":"alpha","rootDir":"/home/alpha","shellProfile":"bash"}}"#;
        let out = dispatch_raw(input, &mut reg);
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["ok"], true);
        assert!(v["result"]["workspaceId"].as_str().unwrap().starts_with("ws-"));
        assert_eq!(reg.list().len(), 1);
    }

    #[test]
    fn dispatch_rejects_invalid_requests_when_called_directly() {
        let mut reg = WorkspaceRegistry::new();
        let req = RequestEnvelope::new(
            "direct-invalid",
            "workspace.create",
            json!({}),
        );

        let resp = dispatch(&req, &mut reg);

        assert!(!resp.is_ok());
        assert_eq!(resp.error().unwrap().code, ErrorCode::InvalidPayload);
        assert!(reg.list().is_empty());
    }

    // ── dispatch_runtime tests ──────────────────────────────────────────

    // Minimal mock for testing session dispatch without real PTY processes.
    mod fake_session {
        use core_session::{OutputBuffer, SessionHost, SessionHostFactory, SessionSpec, TerminalSize};
        use std::sync::{Arc, Mutex};

        #[derive(Default)]
        pub struct FakeState {
            pub writes: Vec<Vec<u8>>,
            pub resizes: Vec<TerminalSize>,
            pub next_wait: Option<bool>,
            pub outputs: Vec<Arc<Mutex<OutputBuffer>>>,
        }

        #[derive(Clone, Default)]
        pub struct FakeFactory {
            pub state: Arc<Mutex<FakeState>>,
        }

        struct FakeHost {
            state: Arc<Mutex<FakeState>>,
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
                _spec: &SessionSpec,
                _size: TerminalSize,
            ) -> Result<Box<dyn SessionHost>, String> {
                let output = Arc::new(Mutex::new(OutputBuffer::default()));
                let mut state = self.state.lock().unwrap();
                state.outputs.push(Arc::clone(&output));
                Ok(Box::new(FakeHost {
                    state: Arc::clone(&self.state),
                    output,
                }))
            }
        }
    }

    use fake_session::FakeFactory;
    use core_session::{LiveSessionRegistry, SessionSpec, TerminalSize};

    fn make_runtime_ctx<'a>(
        registry: &'a mut WorkspaceRegistry,
        sessions: &'a mut LiveSessionRegistry<FakeFactory>,
    ) -> RuntimeContext<'a, FakeFactory> {
        RuntimeContext { registry, sessions }
    }

    /// Helper: start a session bound to workspace_id + pane_id, return its ID.
    fn start_test_session(
        sessions: &mut LiveSessionRegistry<FakeFactory>,
        workspace_id: &str,
        pane_id: &str,
    ) -> String {
        sessions
            .start(
                workspace_id,
                pane_id,
                SessionSpec::new(format!("{workspace_id}:{pane_id}"), "cmd.exe"),
                TerminalSize { rows: 24, cols: 80 },
            )
            .expect("session should start")
    }

    #[test]
    fn dispatch_runtime_routes_workspace_commands_through_existing_dispatch() {
        let mut reg = WorkspaceRegistry::new();
        let factory = FakeFactory::default();
        let mut sessions = LiveSessionRegistry::new(factory);
        let mut ctx = make_runtime_ctx(&mut reg, &mut sessions);

        let request = RequestEnvelope::new(
            "rt-ws-1",
            "workspace.create",
            json!({
                "name": "test-ws",
                "rootDir": "/tmp/test",
                "shellProfile": "bash"
            }),
        );

        let response = dispatch_runtime(&request, &mut ctx);

        assert!(response.is_ok());
        let result = response.result().unwrap();
        assert!(result["workspaceId"].as_str().unwrap().starts_with("ws-"));
        assert_eq!(ctx.registry.list().len(), 1);
    }

    #[test]
    fn dispatch_runtime_handles_session_send_input() {
        let mut reg = WorkspaceRegistry::new();
        let factory = FakeFactory::default();
        let mut sessions = LiveSessionRegistry::new(factory.clone());

        // Create a workspace and pane first so the session can bind.
        reg.create("ws-test", "test", "/tmp", "bash", WorkspaceLayout::starter())
            .unwrap();

        let session_id = start_test_session(&mut sessions, "ws-test", "pane-1");

        let mut ctx = make_runtime_ctx(&mut reg, &mut sessions);
        let request = RequestEnvelope::new(
            "rt-input-1",
            "session.sendInput",
            json!({
                "sessionId": session_id,
                "data": "ls -la\n"
            }),
        );

        let response = dispatch_runtime(&request, &mut ctx);

        assert!(response.is_ok());
        let result = response.result().unwrap();
        assert_eq!(result["delivered"], true);
        assert_eq!(result["sessionId"], session_id);

        // Verify the input was actually written.
        let state = factory.state.lock().unwrap();
        assert_eq!(state.writes, vec![b"ls -la\n".to_vec()]);
    }

    #[test]
    fn dispatch_runtime_handles_session_resize() {
        let mut reg = WorkspaceRegistry::new();
        let factory = FakeFactory::default();
        let mut sessions = LiveSessionRegistry::new(factory.clone());

        reg.create("ws-rsz", "resize-ws", "/tmp", "bash", WorkspaceLayout::starter())
            .unwrap();

        let session_id = start_test_session(&mut sessions, "ws-rsz", "pane-1");

        let mut ctx = make_runtime_ctx(&mut reg, &mut sessions);
        let request = RequestEnvelope::new(
            "rt-resize-1",
            "session.resize",
            json!({
                "sessionId": session_id,
                "cols": 120,
                "rows": 40
            }),
        );

        let response = dispatch_runtime(&request, &mut ctx);

        assert!(response.is_ok());
        let result = response.result().unwrap();
        assert_eq!(result["resized"], true);
        assert_eq!(result["sessionId"], session_id);

        // Verify the resize was actually applied.
        let state = factory.state.lock().unwrap();
        assert_eq!(state.resizes, vec![TerminalSize { rows: 40, cols: 120 }]);
    }

    #[test]
    fn dispatch_runtime_handles_session_restart() {
        let mut reg = WorkspaceRegistry::new();
        let factory = FakeFactory::default();
        let mut sessions = LiveSessionRegistry::new(factory.clone());

        reg.create("ws-restart", "restart-ws", "/tmp", "bash", WorkspaceLayout::starter())
            .unwrap();

        let session_id = start_test_session(&mut sessions, "ws-restart", "pane-1");

        // Mark the session as exited so restart is allowed.
        factory.state.lock().unwrap().next_wait = Some(true);
        // Trigger exit detection via get_status.
        let _ = sessions.get_status(&session_id);

        let mut ctx = make_runtime_ctx(&mut reg, &mut sessions);
        let request = RequestEnvelope::new(
            "rt-restart-1",
            "session.restart",
            json!({ "sessionId": session_id }),
        );

        let response = dispatch_runtime(&request, &mut ctx);

        assert!(response.is_ok());
        let result = response.result().unwrap();
        // After restart, a new session ID is returned.
        assert!(result["sessionId"].as_str().unwrap().starts_with("session:"));
        assert_eq!(result["workspaceId"], "ws-restart");
        assert_eq!(result["paneId"], "pane-1");
    }

    #[test]
    fn dispatch_runtime_handles_session_get_status() {
        let mut reg = WorkspaceRegistry::new();
        let factory = FakeFactory::default();
        let mut sessions = LiveSessionRegistry::new(factory);

        reg.create("ws-status", "status-ws", "/tmp", "bash", WorkspaceLayout::starter())
            .unwrap();

        let session_id = start_test_session(&mut sessions, "ws-status", "pane-1");

        let mut ctx = make_runtime_ctx(&mut reg, &mut sessions);
        let request = RequestEnvelope::new(
            "rt-status-1",
            "session.getStatus",
            json!({ "sessionId": session_id }),
        );

        let response = dispatch_runtime(&request, &mut ctx);

        assert!(response.is_ok());
        let result = response.result().unwrap();
        assert_eq!(result["sessionId"], session_id);
        assert_eq!(result["workspaceId"], "ws-status");
        assert_eq!(result["paneId"], "pane-1");
        assert_eq!(result["status"], "running");
        assert!(result["command"].as_str().unwrap().contains("cmd.exe"));
    }

    #[test]
    fn dispatch_runtime_rejects_unknown_commands() {
        let mut reg = WorkspaceRegistry::new();
        let factory = FakeFactory::default();
        let mut sessions = LiveSessionRegistry::new(factory);
        let mut ctx = make_runtime_ctx(&mut reg, &mut sessions);

        let request = RequestEnvelope::new(
            "rt-unknown-1",
            "bogus.command",
            json!({}),
        );

        let response = dispatch_runtime(&request, &mut ctx);

        assert!(!response.is_ok());
        let err = response.error().unwrap();
        assert_eq!(err.code, ErrorCode::Unsupported);
        assert!(err.message.contains("bogus.command"));
    }

    #[test]
    fn dispatch_runtime_session_send_input_returns_not_found_for_missing_session() {
        let mut reg = WorkspaceRegistry::new();
        let factory = FakeFactory::default();
        let mut sessions = LiveSessionRegistry::new(factory);
        let mut ctx = make_runtime_ctx(&mut reg, &mut sessions);

        let request = RequestEnvelope::new(
            "rt-miss-1",
            "session.sendInput",
            json!({
                "sessionId": "session:9999",
                "data": "hello"
            }),
        );

        let response = dispatch_runtime(&request, &mut ctx);

        assert!(!response.is_ok());
        assert_eq!(response.error().unwrap().code, ErrorCode::NotFound);
    }

    #[test]
    fn dispatch_runtime_raw_routes_session_commands() {
        let mut reg = WorkspaceRegistry::new();
        let factory = FakeFactory::default();
        let mut sessions = LiveSessionRegistry::new(factory);

        reg.create("ws-raw", "raw-ws", "/tmp", "bash", WorkspaceLayout::starter())
            .unwrap();
        let session_id = start_test_session(&mut sessions, "ws-raw", "pane-1");

        let mut ctx = make_runtime_ctx(&mut reg, &mut sessions);
        let input = format!(
            r#"{{"protocolVersion":1,"id":"raw-rt-1","type":"command","command":"session.getStatus","payload":{{"sessionId":"{}"}}}}"#,
            session_id
        );

        let out = dispatch_runtime_raw(&input, &mut ctx);
        let v: Value = serde_json::from_str(&out).expect("output should be valid JSON");
        assert_eq!(v["ok"], true);
        assert_eq!(v["id"], "raw-rt-1");
        assert_eq!(v["result"]["sessionId"], session_id);
    }
}
