use serde::{Deserialize, Serialize};

pub const CRATE_NAME: &str = "core-events";

#[must_use]
pub fn crate_name() -> &'static str {
    CRATE_NAME
}

/// Domain events emitted by mutations in the cmux-win system.
/// These are designed for IPC broadcast and can be serialized to JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DomainEvent {
    #[serde(rename_all = "camelCase")]
    WorkspaceCreated {
        workspace_id: String,
    },
    #[serde(rename_all = "camelCase")]
    WorkspaceRenamed {
        workspace_id: String,
        name: String,
    },
    #[serde(rename_all = "camelCase")]
    WorkspaceClosed {
        workspace_id: String,
    },
    #[serde(rename_all = "camelCase")]
    PaneSplit {
        workspace_id: String,
        pane_id: String,
        new_pane_id: String,
    },
    #[serde(rename_all = "camelCase")]
    PaneClosed {
        workspace_id: String,
        pane_id: String,
    },
    #[serde(rename_all = "camelCase")]
    PaneFocused {
        workspace_id: String,
        pane_id: String,
    },
    #[serde(rename_all = "camelCase")]
    SessionStarted {
        session_id: String,
        workspace_id: String,
        pane_id: String,
    },
    #[serde(rename_all = "camelCase")]
    SessionExited {
        session_id: String,
    },
    #[serde(rename_all = "camelCase")]
    NotificationCreated {
        notification_id: String,
    },
}

/// Convenience constructors for DomainEvent variants.
impl DomainEvent {
    #[must_use]
    pub fn workspace_created(workspace_id: impl Into<String>) -> Self {
        Self::WorkspaceCreated {
            workspace_id: workspace_id.into(),
        }
    }

    #[must_use]
    pub fn workspace_renamed(workspace_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self::WorkspaceRenamed {
            workspace_id: workspace_id.into(),
            name: name.into(),
        }
    }

    #[must_use]
    pub fn workspace_closed(workspace_id: impl Into<String>) -> Self {
        Self::WorkspaceClosed {
            workspace_id: workspace_id.into(),
        }
    }

    #[must_use]
    pub fn pane_split(
        workspace_id: impl Into<String>,
        pane_id: impl Into<String>,
        new_pane_id: impl Into<String>,
    ) -> Self {
        Self::PaneSplit {
            workspace_id: workspace_id.into(),
            pane_id: pane_id.into(),
            new_pane_id: new_pane_id.into(),
        }
    }

    #[must_use]
    pub fn pane_closed(workspace_id: impl Into<String>, pane_id: impl Into<String>) -> Self {
        Self::PaneClosed {
            workspace_id: workspace_id.into(),
            pane_id: pane_id.into(),
        }
    }

    #[must_use]
    pub fn pane_focused(workspace_id: impl Into<String>, pane_id: impl Into<String>) -> Self {
        Self::PaneFocused {
            workspace_id: workspace_id.into(),
            pane_id: pane_id.into(),
        }
    }

    #[must_use]
    pub fn session_started(
        session_id: impl Into<String>,
        workspace_id: impl Into<String>,
        pane_id: impl Into<String>,
    ) -> Self {
        Self::SessionStarted {
            session_id: session_id.into(),
            workspace_id: workspace_id.into(),
            pane_id: pane_id.into(),
        }
    }

    #[must_use]
    pub fn session_exited(session_id: impl Into<String>) -> Self {
        Self::SessionExited {
            session_id: session_id.into(),
        }
    }

    #[must_use]
    pub fn notification_created(notification_id: impl Into<String>) -> Self {
        Self::NotificationCreated {
            notification_id: notification_id.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_expected_name() {
        assert_eq!(crate_name(), CRATE_NAME);
    }

    #[test]
    fn workspace_created_serde_roundtrip() {
        let event = DomainEvent::workspace_created("ws-1");
        let json = serde_json::to_string(&event).unwrap();
        let restored: DomainEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, event);
        assert!(json.contains("\"type\":\"workspaceCreated\""));
    }

    #[test]
    fn workspace_renamed_serde_roundtrip() {
        let event = DomainEvent::workspace_renamed("ws-1", "New Name");
        let json = serde_json::to_string(&event).unwrap();
        let restored: DomainEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, event);
        assert!(json.contains("\"type\":\"workspaceRenamed\""));
        assert!(json.contains("\"name\":\"New Name\""));
    }

    #[test]
    fn workspace_closed_serde_roundtrip() {
        let event = DomainEvent::workspace_closed("ws-1");
        let json = serde_json::to_string(&event).unwrap();
        let restored: DomainEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, event);
    }

    #[test]
    fn pane_split_serde_roundtrip() {
        let event = DomainEvent::pane_split("ws-1", "pane-1", "pane-2");
        let json = serde_json::to_string(&event).unwrap();
        let restored: DomainEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, event);
        assert!(json.contains("\"newPaneId\":\"pane-2\""));
    }

    #[test]
    fn pane_closed_serde_roundtrip() {
        let event = DomainEvent::pane_closed("ws-1", "pane-1");
        let json = serde_json::to_string(&event).unwrap();
        let restored: DomainEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, event);
    }

    #[test]
    fn pane_focused_serde_roundtrip() {
        let event = DomainEvent::pane_focused("ws-1", "pane-1");
        let json = serde_json::to_string(&event).unwrap();
        let restored: DomainEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, event);
    }

    #[test]
    fn session_started_serde_roundtrip() {
        let event = DomainEvent::session_started("sess-1", "ws-1", "pane-1");
        let json = serde_json::to_string(&event).unwrap();
        let restored: DomainEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, event);
        assert!(json.contains("\"sessionId\":\"sess-1\""));
    }

    #[test]
    fn session_exited_serde_roundtrip() {
        let event = DomainEvent::session_exited("sess-1");
        let json = serde_json::to_string(&event).unwrap();
        let restored: DomainEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, event);
    }

    #[test]
    fn notification_created_serde_roundtrip() {
        let event = DomainEvent::notification_created("notif-1");
        let json = serde_json::to_string(&event).unwrap();
        let restored: DomainEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, event);
    }

    #[test]
    fn all_variants_use_camel_case_type_tag() {
        let events = vec![
            DomainEvent::workspace_created("ws-1"),
            DomainEvent::workspace_renamed("ws-1", "Name"),
            DomainEvent::workspace_closed("ws-1"),
            DomainEvent::pane_split("ws-1", "p-1", "p-2"),
            DomainEvent::pane_closed("ws-1", "p-1"),
            DomainEvent::pane_focused("ws-1", "p-1"),
            DomainEvent::session_started("s-1", "ws-1", "p-1"),
            DomainEvent::session_exited("s-1"),
            DomainEvent::notification_created("n-1"),
        ];

        let expected_types = vec![
            "workspaceCreated",
            "workspaceRenamed",
            "workspaceClosed",
            "paneSplit",
            "paneClosed",
            "paneFocused",
            "sessionStarted",
            "sessionExited",
            "notificationCreated",
        ];

        for (event, expected_type) in events.iter().zip(expected_types.iter()) {
            let json = serde_json::to_string(event).unwrap();
            let value: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(
                value["type"].as_str().unwrap(),
                *expected_type,
                "event {:?} should have type tag {}",
                event,
                expected_type
            );
        }
    }

    #[test]
    fn event_helpers_accept_string_and_str() {
        // Verify impl Into<String> works with both &str and String
        let _e1 = DomainEvent::workspace_created("ws-1");
        let _e2 = DomainEvent::workspace_created(String::from("ws-2"));
        let _e3 = DomainEvent::pane_split("ws-1", "p-1", String::from("p-2"));
    }

    #[test]
    fn deserialize_from_json_string() {
        let json = r#"{"type":"workspaceCreated","workspaceId":"ws-test"}"#;
        let event: DomainEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event, DomainEvent::workspace_created("ws-test"));
    }
}
