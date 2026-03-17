use serde::{Deserialize, Serialize};

pub const CRATE_NAME: &str = "core-notify";
pub const DEFAULT_MAX_NOTIFICATIONS: usize = 100;

#[must_use]
pub fn crate_name() -> &'static str {
    CRATE_NAME
}

/// Severity level for a notification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
}

/// Where the notification originated from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum NotificationSource {
    Cli,
    #[serde(rename_all = "camelCase")]
    Pty { pane_id: String },
    App,
}

/// A single notification entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub id: String,
    pub level: NotificationLevel,
    pub source: NotificationSource,
    pub title: String,
    pub body: String,
    pub workspace_id: Option<String>,
    pub timestamp_ms: u64,
    pub read: bool,
}

/// Error type for notification store operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotifyError {
    NotificationNotFound,
}

/// Bounded in-memory store for recent notifications.
/// Acts as a ring buffer — when full, the oldest entry is dropped.
#[derive(Debug, Clone)]
pub struct NotificationStore {
    entries: Vec<Notification>,
    max: usize,
}

impl NotificationStore {
    /// Create a new store with the default capacity.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            max: DEFAULT_MAX_NOTIFICATIONS,
        }
    }

    /// Create a new store with a custom capacity.
    #[must_use]
    pub fn with_capacity(max: usize) -> Self {
        Self {
            entries: Vec::new(),
            max,
        }
    }

    /// Push a notification into the store. If at capacity, the oldest is evicted.
    pub fn push(&mut self, notification: Notification) {
        if self.entries.len() >= self.max {
            self.entries.remove(0);
        }
        self.entries.push(notification);
    }

    /// Return the most recent `limit` notifications, newest first.
    #[must_use]
    pub fn list_recent(&self, limit: usize) -> Vec<&Notification> {
        self.entries.iter().rev().take(limit).collect()
    }

    /// Return all notifications.
    #[must_use]
    pub fn list_all(&self) -> &[Notification] {
        &self.entries
    }

    /// Mark a notification as read by ID.
    pub fn mark_read(&mut self, id: &str) -> Result<(), NotifyError> {
        let entry = self
            .entries
            .iter_mut()
            .find(|n| n.id == id)
            .ok_or(NotifyError::NotificationNotFound)?;
        entry.read = true;
        Ok(())
    }

    /// Count of unread notifications.
    #[must_use]
    pub fn unread_count(&self) -> usize {
        self.entries.iter().filter(|n| !n.read).count()
    }

    /// Count of all notifications in store.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the store is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Remove all notifications.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// List notifications filtered by workspace ID, newest first.
    #[must_use]
    pub fn list_by_workspace(&self, workspace_id: &str) -> Vec<&Notification> {
        self.entries
            .iter()
            .rev()
            .filter(|n| n.workspace_id.as_deref() == Some(workspace_id))
            .collect()
    }
}

impl Default for NotificationStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_notification(id: &str, level: NotificationLevel) -> Notification {
        Notification {
            id: id.to_string(),
            level,
            source: NotificationSource::App,
            title: format!("Title {id}"),
            body: format!("Body {id}"),
            workspace_id: None,
            timestamp_ms: 1000,
            read: false,
        }
    }

    fn make_notification_for_workspace(id: &str, workspace_id: &str) -> Notification {
        Notification {
            id: id.to_string(),
            level: NotificationLevel::Info,
            source: NotificationSource::App,
            title: format!("Title {id}"),
            body: format!("Body {id}"),
            workspace_id: Some(workspace_id.to_string()),
            timestamp_ms: 1000,
            read: false,
        }
    }

    #[test]
    fn exposes_expected_name() {
        assert_eq!(crate_name(), CRATE_NAME);
    }

    #[test]
    fn new_store_is_empty() {
        let store = NotificationStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert_eq!(store.unread_count(), 0);
    }

    #[test]
    fn push_and_list_all() {
        let mut store = NotificationStore::new();
        store.push(make_notification("n1", NotificationLevel::Info));
        store.push(make_notification("n2", NotificationLevel::Warning));

        assert_eq!(store.len(), 2);
        assert_eq!(store.list_all()[0].id, "n1");
        assert_eq!(store.list_all()[1].id, "n2");
    }

    #[test]
    fn list_recent_returns_newest_first() {
        let mut store = NotificationStore::new();
        store.push(make_notification("n1", NotificationLevel::Info));
        store.push(make_notification("n2", NotificationLevel::Warning));
        store.push(make_notification("n3", NotificationLevel::Error));

        let recent = store.list_recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].id, "n3");
        assert_eq!(recent[1].id, "n2");
    }

    #[test]
    fn list_recent_with_limit_larger_than_store() {
        let mut store = NotificationStore::new();
        store.push(make_notification("n1", NotificationLevel::Info));

        let recent = store.list_recent(100);
        assert_eq!(recent.len(), 1);
    }

    #[test]
    fn bounded_cap_evicts_oldest() {
        let mut store = NotificationStore::with_capacity(3);
        store.push(make_notification("n1", NotificationLevel::Info));
        store.push(make_notification("n2", NotificationLevel::Info));
        store.push(make_notification("n3", NotificationLevel::Info));
        store.push(make_notification("n4", NotificationLevel::Info));

        assert_eq!(store.len(), 3);
        let ids: Vec<&str> = store.list_all().iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids, vec!["n2", "n3", "n4"]);
    }

    #[test]
    fn bounded_cap_at_101_keeps_100() {
        let mut store = NotificationStore::with_capacity(100);
        for i in 0..101 {
            store.push(make_notification(&format!("n{i}"), NotificationLevel::Info));
        }
        assert_eq!(store.len(), 100);
        assert_eq!(store.list_all()[0].id, "n1"); // n0 was evicted
    }

    #[test]
    fn mark_read_and_unread_count() {
        let mut store = NotificationStore::new();
        store.push(make_notification("n1", NotificationLevel::Info));
        store.push(make_notification("n2", NotificationLevel::Warning));
        store.push(make_notification("n3", NotificationLevel::Error));

        assert_eq!(store.unread_count(), 3);

        store.mark_read("n2").expect("n2 should exist");
        assert_eq!(store.unread_count(), 2);

        store.mark_read("n1").expect("n1 should exist");
        assert_eq!(store.unread_count(), 1);
    }

    #[test]
    fn mark_read_unknown_id_returns_error() {
        let mut store = NotificationStore::new();
        store.push(make_notification("n1", NotificationLevel::Info));

        let err = store.mark_read("nonexistent").unwrap_err();
        assert_eq!(err, NotifyError::NotificationNotFound);
    }

    #[test]
    fn clear_removes_all_entries() {
        let mut store = NotificationStore::new();
        store.push(make_notification("n1", NotificationLevel::Info));
        store.push(make_notification("n2", NotificationLevel::Info));

        store.clear();
        assert!(store.is_empty());
        assert_eq!(store.unread_count(), 0);
    }

    #[test]
    fn list_by_workspace_filters_correctly() {
        let mut store = NotificationStore::new();
        store.push(make_notification_for_workspace("n1", "ws-a"));
        store.push(make_notification_for_workspace("n2", "ws-b"));
        store.push(make_notification_for_workspace("n3", "ws-a"));
        store.push(make_notification("n4", NotificationLevel::Info)); // no workspace

        let ws_a = store.list_by_workspace("ws-a");
        assert_eq!(ws_a.len(), 2);
        assert_eq!(ws_a[0].id, "n3"); // newest first
        assert_eq!(ws_a[1].id, "n1");
    }

    #[test]
    fn notification_source_variants() {
        let cli = NotificationSource::Cli;
        let pty = NotificationSource::Pty { pane_id: "pane-1".to_string() };
        let app = NotificationSource::App;

        // Serde roundtrip
        for source in [cli, pty, app] {
            let json = serde_json::to_string(&source).expect("serialize");
            let restored: NotificationSource = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(restored, source);
        }
    }

    #[test]
    fn notification_level_serde_roundtrip() {
        for level in [NotificationLevel::Info, NotificationLevel::Warning, NotificationLevel::Error] {
            let json = serde_json::to_string(&level).expect("serialize");
            let restored: NotificationLevel = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(restored, level);
        }
    }

    #[test]
    fn notification_serde_roundtrip() {
        let notification = Notification {
            id: "n-test".to_string(),
            level: NotificationLevel::Warning,
            source: NotificationSource::Pty { pane_id: "pane-5".to_string() },
            title: "Alert".to_string(),
            body: "Something happened".to_string(),
            workspace_id: Some("ws-main".to_string()),
            timestamp_ms: 1234567890,
            read: false,
        };

        let json = serde_json::to_string(&notification).expect("serialize");
        let restored: Notification = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored, notification);
    }

    #[test]
    fn default_store_has_100_capacity() {
        let store = NotificationStore::default();
        // Push 101, verify only 100 remain
        let mut store_mut = store;
        for i in 0..101 {
            store_mut.push(make_notification(&format!("n{i}"), NotificationLevel::Info));
        }
        assert_eq!(store_mut.len(), 100);
    }
}
