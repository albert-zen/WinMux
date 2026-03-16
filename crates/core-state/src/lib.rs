use core_layout::{LayoutSnapshot, WorkspaceLayout};
use serde::{Deserialize, Serialize};

pub const APP_NAME: &str = "cmux-win";
pub const STARTER_WORKSPACE_NAME: &str = "inbox";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopBootstrap {
    pub app_name: String,
    pub protocol_version: u32,
    pub starter_workspace_name: String,
    pub starter_pane_count: usize,
    pub starter_split_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreSnapshot {
    pub last_focused_pane_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSnapshot {
    pub layout: LayoutSnapshot,
    pub restore: RestoreSnapshot,
    pub scrollback: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedWorkspaceRecord {
    pub workspace_id: String,
    pub name: String,
    pub root_dir: String,
    pub theme_id: Option<String>,
    pub snapshot_json: String,
}

#[must_use]
pub fn starter_bootstrap(protocol_version: u32) -> DesktopBootstrap {
    let layout = WorkspaceSnapshot::starter().layout;

    DesktopBootstrap {
        app_name: APP_NAME.to_string(),
        protocol_version,
        starter_workspace_name: STARTER_WORKSPACE_NAME.to_string(),
        starter_pane_count: layout.pane_count,
        starter_split_count: layout.split_count,
    }
}

impl WorkspaceSnapshot {
    #[must_use]
    pub fn starter() -> Self {
        let layout = WorkspaceLayout::starter();

        Self {
            layout: layout.snapshot(),
            restore: RestoreSnapshot {
                last_focused_pane_id: layout.focused_pane_id,
            },
            scrollback: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_scrollback(mut self, scrollback: Vec<String>) -> Self {
        self.scrollback = scrollback;
        self
    }

    #[must_use]
    pub fn with_scrollback_cap(mut self, cap: usize) -> Self {
        if self.scrollback.len() > cap {
            let keep_from = self.scrollback.len() - cap;
            self.scrollback = self.scrollback.split_off(keep_from);
        }

        self
    }
}

impl PersistedWorkspaceRecord {
    pub fn from_snapshot(
        workspace_id: impl Into<String>,
        name: impl Into<String>,
        root_dir: impl Into<String>,
        theme_id: Option<impl Into<String>>,
        snapshot: WorkspaceSnapshot,
    ) -> serde_json::Result<Self> {
        Ok(Self {
            workspace_id: workspace_id.into(),
            name: name.into(),
            root_dir: root_dir.into(),
            theme_id: theme_id.map(Into::into),
            snapshot_json: serde_json::to_string(&snapshot)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn starter_bootstrap_uses_single_pane_layout() {
        let bootstrap = starter_bootstrap(1);

        assert_eq!(bootstrap.app_name, APP_NAME);
        assert_eq!(bootstrap.protocol_version, 1);
        assert_eq!(bootstrap.starter_workspace_name, STARTER_WORKSPACE_NAME);
        assert_eq!(bootstrap.starter_pane_count, 1);
        assert_eq!(bootstrap.starter_split_count, 0);
    }

    #[test]
    fn persisted_workspace_snapshot_uses_queryable_columns_and_nested_json_blob() {
        let record = PersistedWorkspaceRecord::from_snapshot(
            "ws-inbox",
            "Inbox",
            "D:\\dev\\inbox",
            Some("forest"),
            WorkspaceSnapshot::starter(),
        )
        .expect("snapshot should serialize");

        assert_eq!(record.workspace_id, "ws-inbox");
        assert_eq!(record.name, "Inbox");
        assert_eq!(record.root_dir, "D:\\dev\\inbox");
        assert_eq!(record.theme_id.as_deref(), Some("forest"));

        let json: Value =
            serde_json::from_str(&record.snapshot_json).expect("json blob should be valid");

        assert_eq!(json["layout"]["paneCount"], 1);
        assert_eq!(json["layout"]["splitCount"], 0);
        assert_eq!(json["restore"]["lastFocusedPaneId"], "pane-1");
    }

    #[test]
    fn snapshot_caps_scrollback_without_breaking_restore_shape() {
        let snapshot = WorkspaceSnapshot::starter().with_scrollback(vec![
            "one".into(),
            "two".into(),
            "three".into(),
            "four".into(),
        ]);

        let capped = snapshot.with_scrollback_cap(2);

        assert_eq!(capped.scrollback, vec!["three", "four"]);
        assert_eq!(capped.layout.pane_count, 1);
        assert_eq!(capped.restore.last_focused_pane_id, "pane-1");
    }
}
