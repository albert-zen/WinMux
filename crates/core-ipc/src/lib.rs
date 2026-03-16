use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

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
    fn parse_and_validate_rejects_unimplemented_command_payloads_as_unsupported() {
        let err = parse_and_validate_request(
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
        .expect_err("commands without validation should stay unsupported for now");

        assert_eq!(err.code, ErrorCode::Unsupported);
        assert_eq!(err.message, "Unsupported command: workspace.rename");
    }
}
