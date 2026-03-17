use core_layout::{LayoutError, LayoutSnapshot, SplitOrientation, WorkspaceLayout};
use serde::{Deserialize, Serialize};

pub const APP_NAME: &str = "cmux-win";
pub const STARTER_WORKSPACE_NAME: &str = "inbox";

// ── Persistence / restore ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreError {
    InvalidJson,
    UnsupportedProtocolVersion,
    DuplicateWorkspaceId,
    EmptyWorkspaceLayout,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedRegistry {
    protocol_version: u32,
    workspaces: Vec<PersistedWorkspace>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedWorkspace {
    id: String,
    name: String,
    root_dir: String,
    shell_profile: String,
    layout: WorkspaceLayout,
}

// ── Bootstrap / snapshot types (unchanged) ──────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopBootstrap {
    pub app_name: String,
    pub protocol_version: u32,
    pub starter_workspace_name: String,
    pub starter_pane_count: usize,
    pub starter_split_count: usize,
    pub workspaces: Vec<WorkspaceSummary>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSummary {
    pub id: String,
    pub name: String,
    pub root_dir: String,
    pub shell_profile: String,
    pub pane_count: usize,
    pub split_count: usize,
    pub focused_pane_id: String,
}

#[must_use]
pub fn starter_bootstrap(protocol_version: u32) -> DesktopBootstrap {
    let workspaces = starter_workspace_registry().summaries();
    let layout = WorkspaceSnapshot::starter().layout;

    DesktopBootstrap {
        app_name: APP_NAME.to_string(),
        protocol_version,
        starter_workspace_name: STARTER_WORKSPACE_NAME.to_string(),
        starter_pane_count: layout.pane_count,
        starter_split_count: layout.split_count,
        workspaces,
    }
}

#[must_use]
pub fn starter_workspace_registry() -> WorkspaceRegistry {
    let mut registry = WorkspaceRegistry::new();
    registry.create(
        "ws-inbox",
        STARTER_WORKSPACE_NAME,
        "D:\\dev\\inbox",
        "cmd.exe",
        WorkspaceLayout::starter(),
    )
    .expect("starter workspace id should be unique");
    registry
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

// ── Workspace registry ───────────────────────────────────────────────────────

/// A live workspace entry held in the in-memory registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRecord {
    pub id: String,
    pub name: String,
    pub root_dir: String,
    pub shell_profile: String,
    pub layout: WorkspaceLayout,
}

/// Errors returned by registry mutation methods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceError {
    WorkspaceNotFound,
    DuplicateWorkspaceId,
    InvalidWorkspaceName,
    Layout(LayoutError),
}

impl From<LayoutError> for WorkspaceError {
    fn from(e: LayoutError) -> Self {
        WorkspaceError::Layout(e)
    }
}

/// In-memory registry of live workspaces, ordered by insertion time.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceRegistry {
    workspaces: Vec<WorkspaceRecord>,
}

impl WorkspaceRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a new workspace at the end of the list.
    pub fn create(
        &mut self,
        id: impl Into<String>,
        name: impl Into<String>,
        root_dir: impl Into<String>,
        shell_profile: impl Into<String>,
        layout: WorkspaceLayout,
    ) -> Result<(), WorkspaceError> {
        let id = id.into();

        if self.workspaces.iter().any(|workspace| workspace.id == id) {
            return Err(WorkspaceError::DuplicateWorkspaceId);
        }

        self.workspaces.push(WorkspaceRecord {
            id,
            name: name.into(),
            root_dir: root_dir.into(),
            shell_profile: shell_profile.into(),
            layout,
        });

        Ok(())
    }

    /// Return all workspaces in insertion order.
    #[must_use]
    pub fn list(&self) -> &[WorkspaceRecord] {
        &self.workspaces
    }

    #[must_use]
    pub fn summaries(&self) -> Vec<WorkspaceSummary> {
        self.workspaces.iter().map(WorkspaceRecord::summary).collect()
    }

    pub fn remove_workspace(&mut self, workspace_id: &str) -> Result<(), WorkspaceError> {
        let index = self
            .workspaces
            .iter()
            .position(|workspace| workspace.id == workspace_id)
            .ok_or(WorkspaceError::WorkspaceNotFound)?;
        self.workspaces.remove(index);
        Ok(())
    }

    /// Split a pane inside the named workspace, delegating to `WorkspaceLayout`.
    pub fn split_pane(
        &mut self,
        workspace_id: &str,
        source_id: &str,
        new_id: &str,
        orientation: SplitOrientation,
        ratio: f64,
    ) -> Result<(), WorkspaceError> {
        let ws = self
            .workspaces
            .iter_mut()
            .find(|w| w.id == workspace_id)
            .ok_or(WorkspaceError::WorkspaceNotFound)?;

        ws.layout.split_pane(source_id, new_id, orientation, ratio)?;
        Ok(())
    }

    /// Close a pane inside the named workspace, delegating to `WorkspaceLayout`.
    pub fn close_pane(
        &mut self,
        workspace_id: &str,
        pane_id: &str,
    ) -> Result<(), WorkspaceError> {
        let ws = self
            .workspaces
            .iter_mut()
            .find(|w| w.id == workspace_id)
            .ok_or(WorkspaceError::WorkspaceNotFound)?;

        ws.layout.close_pane(pane_id)?;
        Ok(())
    }

    pub fn rename_workspace(
        &mut self,
        workspace_id: &str,
        new_name: &str,
    ) -> Result<(), WorkspaceError> {
        let trimmed_name = new_name.trim();
        if trimmed_name.is_empty() {
            return Err(WorkspaceError::InvalidWorkspaceName);
        }

        let ws = self
            .workspaces
            .iter_mut()
            .find(|w| w.id == workspace_id)
            .ok_or(WorkspaceError::WorkspaceNotFound)?;

        ws.name = trimmed_name.to_string();
        Ok(())
    }

    pub fn focus_pane(
        &mut self,
        workspace_id: &str,
        pane_id: &str,
    ) -> Result<(), WorkspaceError> {
        let ws = self
            .workspaces
            .iter_mut()
            .find(|w| w.id == workspace_id)
            .ok_or(WorkspaceError::WorkspaceNotFound)?;

        ws.layout.focus_pane(pane_id)?;
        Ok(())
    }

    /// Serialize the registry to JSON for persistence.
    pub fn to_persisted_json(&self) -> serde_json::Result<String> {
        let persisted = PersistedRegistry {
            protocol_version: 1,
            workspaces: self
                .workspaces
                .iter()
                .map(|ws| PersistedWorkspace {
                    id: ws.id.clone(),
                    name: ws.name.clone(),
                    root_dir: ws.root_dir.clone(),
                    shell_profile: ws.shell_profile.clone(),
                    layout: ws.layout.clone(),
                })
                .collect(),
        };
        serde_json::to_string(&persisted)
    }

    /// Restore a registry from persisted JSON.
    pub fn from_persisted_json(json: &str) -> Result<Self, RestoreError> {
        let persisted: PersistedRegistry =
            serde_json::from_str(json).map_err(|_| RestoreError::InvalidJson)?;
        if persisted.protocol_version != 1 {
            return Err(RestoreError::UnsupportedProtocolVersion);
        }

        let mut registry = Self::new();
        let mut seen_ids = std::collections::HashSet::new();

        for ws in persisted.workspaces {
            if !seen_ids.insert(ws.id.clone()) {
                return Err(RestoreError::DuplicateWorkspaceId);
            }
            if ws.layout.panes.is_empty() {
                return Err(RestoreError::EmptyWorkspaceLayout);
            }
            registry.workspaces.push(WorkspaceRecord {
                id: ws.id,
                name: ws.name,
                root_dir: ws.root_dir,
                shell_profile: ws.shell_profile,
                layout: ws.layout,
            });
        }

        Ok(registry)
    }
}

impl WorkspaceRecord {
    #[must_use]
    pub fn summary(&self) -> WorkspaceSummary {
        let snapshot = self.layout.snapshot();

        WorkspaceSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            root_dir: self.root_dir.clone(),
            shell_profile: self.shell_profile.clone(),
            pane_count: snapshot.pane_count,
            split_count: snapshot.split_count,
            focused_pane_id: self.layout.focused_pane_id.clone(),
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use core_layout::SplitOrientation;
    use serde_json::Value;

    // ── existing snapshot/bootstrap tests (unchanged) ─────────────────────

    #[test]
    fn starter_bootstrap_uses_single_pane_layout() {
        let bootstrap = starter_bootstrap(1);

        assert_eq!(bootstrap.app_name, APP_NAME);
        assert_eq!(bootstrap.protocol_version, 1);
        assert_eq!(bootstrap.starter_workspace_name, STARTER_WORKSPACE_NAME);
        assert_eq!(bootstrap.starter_pane_count, 1);
        assert_eq!(bootstrap.starter_split_count, 0);
        assert_eq!(bootstrap.workspaces.len(), 1);
        assert_eq!(bootstrap.workspaces[0].id, "ws-inbox");
        assert_eq!(bootstrap.workspaces[0].root_dir, "D:\\dev\\inbox");
        assert_eq!(bootstrap.workspaces[0].shell_profile, "cmd.exe");
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

    // ── WorkspaceRegistry tests ───────────────────────────────────────────

    fn make_registry_with_two() -> WorkspaceRegistry {
        let mut reg = WorkspaceRegistry::new();
        reg.create("ws-a", "Alpha", "/home/alpha", "pwsh", WorkspaceLayout::starter())
            .expect("ws-a should be unique");
        reg.create("ws-b", "Beta", "/home/beta", "cmd.exe", WorkspaceLayout::starter())
            .expect("ws-b should be unique");
        reg
    }

    #[test]
    fn create_and_list_preserve_insertion_order() {
        let reg = make_registry_with_two();
        let list = reg.list();

        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, "ws-a");
        assert_eq!(list[0].name, "Alpha");
        assert_eq!(list[0].root_dir, "/home/alpha");
        assert_eq!(list[0].shell_profile, "pwsh");
        assert_eq!(list[1].id, "ws-b");
        assert_eq!(list[1].name, "Beta");
        assert_eq!(list[1].shell_profile, "cmd.exe");
    }

    #[test]
    fn empty_registry_returns_empty_list() {
        let reg = WorkspaceRegistry::new();
        assert!(reg.list().is_empty());
    }

    #[test]
    fn split_pane_delegates_to_layout_and_mutates_correct_workspace() {
        let mut reg = make_registry_with_two();

        reg.split_pane("ws-a", "pane-1", "pane-2", SplitOrientation::Vertical, 0.5)
            .expect("split should succeed");

        let list = reg.list();
        // ws-a got the split
        assert_eq!(list[0].layout.snapshot().pane_count, 2);
        assert_eq!(list[0].layout.snapshot().split_count, 1);
        assert_eq!(list[0].layout.focused_pane_id, "pane-2");
        // ws-b is untouched
        assert_eq!(list[1].layout.snapshot().pane_count, 1);
    }

    #[test]
    fn split_pane_on_unknown_workspace_returns_workspace_not_found() {
        let mut reg = make_registry_with_two();

        let err = reg
            .split_pane("ws-missing", "pane-1", "pane-2", SplitOrientation::Vertical, 0.5)
            .unwrap_err();

        assert_eq!(err, WorkspaceError::WorkspaceNotFound);
    }

    #[test]
    fn split_pane_propagates_layout_errors() {
        let mut reg = make_registry_with_two();

        let bad_ratio = reg
            .split_pane("ws-a", "pane-1", "pane-2", SplitOrientation::Vertical, 0.0)
            .unwrap_err();

        assert_eq!(bad_ratio, WorkspaceError::Layout(LayoutError::InvalidRatio));
    }

    #[test]
    fn close_pane_delegates_to_layout_and_reduces_pane_count() {
        let mut reg = make_registry_with_two();
        reg.split_pane("ws-b", "pane-1", "pane-2", SplitOrientation::Horizontal, 0.5)
            .expect("setup split");

        reg.close_pane("ws-b", "pane-2")
            .expect("close should succeed");

        let list = reg.list();
        assert_eq!(list[1].layout.snapshot().pane_count, 1);
        assert_eq!(list[1].layout.snapshot().split_count, 0);
        // ws-a unaffected
        assert_eq!(list[0].layout.snapshot().pane_count, 1);
    }

    #[test]
    fn close_pane_on_unknown_workspace_returns_workspace_not_found() {
        let mut reg = make_registry_with_two();

        let err = reg.close_pane("ws-missing", "pane-1").unwrap_err();

        assert_eq!(err, WorkspaceError::WorkspaceNotFound);
    }

    #[test]
    fn close_pane_propagates_would_empty_workspace_error() {
        let mut reg = make_registry_with_two();

        let err = reg.close_pane("ws-a", "pane-1").unwrap_err();

        assert_eq!(
            err,
            WorkspaceError::Layout(LayoutError::WouldEmptyWorkspace)
        );
    }

    #[test]
    fn focus_pane_delegates_to_layout_and_updates_summary() {
        let mut reg = make_registry_with_two();
        reg.split_pane("ws-b", "pane-1", "pane-2", SplitOrientation::Vertical, 0.5)
            .expect("split should work");

        reg.focus_pane("ws-b", "pane-1").expect("focus should work");

        let focused = reg
            .summaries()
            .into_iter()
            .find(|workspace| workspace.id == "ws-b")
            .unwrap();
        assert_eq!(focused.focused_pane_id, "pane-1");
    }

    #[test]
    fn focus_pane_on_unknown_workspace_returns_workspace_not_found() {
        let mut reg = make_registry_with_two();

        let err = reg.focus_pane("ws-missing", "pane-1").unwrap_err();

        assert_eq!(err, WorkspaceError::WorkspaceNotFound);
    }

    #[test]
    fn remove_workspace_drops_existing_entry_and_preserves_order() {
        let mut reg = make_registry_with_two();

        reg.remove_workspace("ws-a")
            .expect("existing workspace should be removable");

        let list = reg.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "ws-b");
    }

    #[test]
    fn remove_workspace_on_unknown_workspace_returns_workspace_not_found() {
        let mut reg = make_registry_with_two();

        let err = reg.remove_workspace("ws-missing").unwrap_err();

        assert_eq!(err, WorkspaceError::WorkspaceNotFound);
    }

    #[test]
    fn rename_workspace_updates_name_and_summaries() {
        let mut reg = make_registry_with_two();

        reg.rename_workspace("ws-a", "Alpha Prime")
            .expect("existing workspace should rename");

        assert_eq!(reg.list()[0].name, "Alpha Prime");
        assert_eq!(reg.summaries()[0].name, "Alpha Prime");
    }

    #[test]
    fn rename_workspace_on_unknown_workspace_returns_workspace_not_found() {
        let mut reg = make_registry_with_two();

        let err = reg.rename_workspace("ws-missing", "Renamed").unwrap_err();

        assert_eq!(err, WorkspaceError::WorkspaceNotFound);
    }

    #[test]
    fn rename_workspace_rejects_blank_names() {
        let mut reg = make_registry_with_two();

        let err = reg.rename_workspace("ws-a", "   ").unwrap_err();

        assert_eq!(err, WorkspaceError::InvalidWorkspaceName);
    }

    #[test]
    fn workspace_record_stores_correct_fields() {
        let layout = WorkspaceLayout::starter();
        let mut reg = WorkspaceRegistry::new();
        reg.create("ws-x", "X project", "/srv/x", "pwsh", layout.clone())
            .expect("workspace should be unique");

        let record = &reg.list()[0];
        assert_eq!(record.id, "ws-x");
        assert_eq!(record.name, "X project");
        assert_eq!(record.root_dir, "/srv/x");
        assert_eq!(record.shell_profile, "pwsh");
        assert_eq!(record.layout, layout);
    }

    #[test]
    fn summaries_project_live_layout_fields() {
        let mut reg = WorkspaceRegistry::new();
        reg.create("ws-x", "X project", "/srv/x", "pwsh", WorkspaceLayout::starter())
            .expect("workspace should be unique");
        reg.split_pane("ws-x", "pane-1", "pane-2", SplitOrientation::Vertical, 0.5)
            .expect("split should succeed");

        let summaries = reg.summaries();

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, "ws-x");
        assert_eq!(summaries[0].name, "X project");
        assert_eq!(summaries[0].root_dir, "/srv/x");
        assert_eq!(summaries[0].shell_profile, "pwsh");
        assert_eq!(summaries[0].pane_count, 2);
        assert_eq!(summaries[0].split_count, 1);
        assert_eq!(summaries[0].focused_pane_id, "pane-2");
    }

    #[test]
    fn starter_workspace_registry_creates_demo_workspace() {
        let registry = starter_workspace_registry();
        let workspaces = registry.list();

        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0].id, "ws-inbox");
        assert_eq!(workspaces[0].name, STARTER_WORKSPACE_NAME);
        assert_eq!(workspaces[0].root_dir, "D:\\dev\\inbox");
        assert_eq!(workspaces[0].shell_profile, "cmd.exe");
        assert_eq!(workspaces[0].layout.snapshot().pane_count, 1);
    }

    #[test]
    fn create_rejects_duplicate_workspace_ids() {
        let mut reg = WorkspaceRegistry::new();
        reg.create("ws-x", "X project", "/srv/x", "pwsh", WorkspaceLayout::starter())
            .expect("first insert should work");

        let duplicate = reg.create(
            "ws-x",
            "Another",
            "/srv/other",
            "cmd.exe",
            WorkspaceLayout::starter(),
        );

        assert_eq!(duplicate, Err(WorkspaceError::DuplicateWorkspaceId));
        assert_eq!(reg.list().len(), 1);
    }

    #[test]
    fn persisted_state_roundtrip_preserves_workspace_layout_and_focus() {
        let mut reg = make_registry_with_two();
        reg.split_pane("ws-a", "pane-1", "pane-2", SplitOrientation::Vertical, 0.5)
            .expect("split should succeed");
        reg.focus_pane("ws-a", "pane-1").expect("focus should succeed");
        reg.rename_workspace("ws-b", "Beta Prime")
            .expect("rename should succeed");

        let json = reg
            .to_persisted_json()
            .expect("registry should serialize to persisted json");
        let restored =
            WorkspaceRegistry::from_persisted_json(&json).expect("persisted json should restore");

        assert_eq!(restored.list().len(), 2);
        assert_eq!(restored.list()[0].layout.snapshot().pane_count, 2);
        assert_eq!(restored.list()[0].layout.focused_pane_id, "pane-1");
        assert_eq!(restored.list()[1].name, "Beta Prime");
    }

    #[test]
    fn persisted_state_rejects_duplicate_workspace_ids() {
        let json = r#"{
            "protocolVersion": 1,
            "workspaces": [
                {
                    "id": "ws-a",
                    "name": "Alpha",
                    "rootDir": "/alpha",
                    "shellProfile": "pwsh",
                    "layout": {
                        "panes": [{ "paneId": "pane-1", "sessionKind": "runningShell" }],
                        "focusedPaneId": "pane-1",
                        "splitCount": 0
                    }
                },
                {
                    "id": "ws-a",
                    "name": "Beta",
                    "rootDir": "/beta",
                    "shellProfile": "cmd.exe",
                    "layout": {
                        "panes": [{ "paneId": "pane-7", "sessionKind": "freshShell" }],
                        "focusedPaneId": "pane-7",
                        "splitCount": 0
                    }
                }
            ]
        }"#;

        let err = WorkspaceRegistry::from_persisted_json(json).unwrap_err();

        assert_eq!(err, RestoreError::DuplicateWorkspaceId);
    }

    #[test]
    fn persisted_state_rejects_empty_workspace_layouts() {
        let json = r#"{
            "protocolVersion": 1,
            "workspaces": [
                {
                    "id": "ws-a",
                    "name": "Alpha",
                    "rootDir": "/alpha",
                    "shellProfile": "pwsh",
                    "layout": {
                        "panes": [],
                        "focusedPaneId": "pane-1",
                        "splitCount": 0
                    }
                }
            ]
        }"#;

        let err = WorkspaceRegistry::from_persisted_json(json).unwrap_err();

        assert_eq!(err, RestoreError::EmptyWorkspaceLayout);
    }

    #[test]
    fn persisted_state_rejects_invalid_json() {
        let err = WorkspaceRegistry::from_persisted_json("{not-json").unwrap_err();

        assert_eq!(err, RestoreError::InvalidJson);
    }

    #[test]
    fn persisted_state_rejects_unsupported_protocol_versions() {
        let json = r#"{
            "protocolVersion": 7,
            "workspaces": []
        }"#;

        let err = WorkspaceRegistry::from_persisted_json(json).unwrap_err();

        assert_eq!(err, RestoreError::UnsupportedProtocolVersion);
    }
}
