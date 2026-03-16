use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutSnapshot {
    pub pane_count: usize,
    pub split_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SessionKind {
    RunningShell,
    FreshShell,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaneSlot {
    pub pane_id: String,
    pub session_kind: SessionKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseSessionResult {
    pub replaced_with_fresh_shell: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceLayout {
    pub panes: Vec<PaneSlot>,
    pub focused_pane_id: String,
    pub split_count: usize,
}

impl LayoutSnapshot {
    #[must_use]
    pub fn single_pane() -> Self {
        Self {
            pane_count: 1,
            split_count: 0,
        }
    }
}

impl WorkspaceLayout {
    #[must_use]
    pub fn starter() -> Self {
        Self {
            panes: vec![PaneSlot {
                pane_id: "pane-1".to_string(),
                session_kind: SessionKind::RunningShell,
            }],
            focused_pane_id: "pane-1".to_string(),
            split_count: 0,
        }
    }

    #[must_use]
    pub fn snapshot(&self) -> LayoutSnapshot {
        LayoutSnapshot {
            pane_count: self.panes.len(),
            split_count: self.split_count,
        }
    }

    pub fn close_session(&mut self, pane_id: &str) -> Option<CloseSessionResult> {
        let pane = self.panes.iter_mut().find(|pane| pane.pane_id == pane_id)?;

        pane.session_kind = SessionKind::FreshShell;
        self.focused_pane_id = pane.pane_id.clone();

        Some(CloseSessionResult {
            replaced_with_fresh_shell: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_with_a_single_pane() {
        let layout = LayoutSnapshot::single_pane();

        assert_eq!(layout.pane_count, 1);
        assert_eq!(layout.split_count, 0);
    }

    #[test]
    fn starter_workspace_keeps_a_terminal_slot_when_last_session_closes() {
        let mut layout = WorkspaceLayout::starter();

        let closed = layout.close_session("pane-1").expect("pane should exist");

        assert!(closed.replaced_with_fresh_shell);
        assert_eq!(layout.snapshot().pane_count, 1);
        assert_eq!(layout.snapshot().split_count, 0);
        assert_eq!(layout.panes[0].session_kind, SessionKind::FreshShell);
        assert_eq!(layout.focused_pane_id, "pane-1");
    }

    #[test]
    fn closing_a_missing_session_is_rejected() {
        let mut layout = WorkspaceLayout::starter();

        let closed = layout.close_session("missing-pane");

        assert_eq!(closed, None);
    }
}
