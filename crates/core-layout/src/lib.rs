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
pub enum SplitOrientation {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    PaneNotFound,
    InvalidRatio,
    WouldEmptyWorkspace,
    DuplicatePaneId,
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

    pub fn split_pane(
        &mut self,
        source_id: &str,
        new_id: &str,
        _orientation: SplitOrientation,
        ratio: f64,
    ) -> Result<(), LayoutError> {
        let source_idx = self
            .panes
            .iter()
            .position(|p| p.pane_id == source_id)
            .ok_or(LayoutError::PaneNotFound)?;

        if ratio <= 0.0 || ratio >= 1.0 {
            return Err(LayoutError::InvalidRatio);
        }

        if self.panes.iter().any(|p| p.pane_id == new_id) {
            return Err(LayoutError::DuplicatePaneId);
        }

        self.panes.insert(
            source_idx + 1,
            PaneSlot {
                pane_id: new_id.to_string(),
                session_kind: SessionKind::FreshShell,
            },
        );
        self.split_count += 1;
        self.focused_pane_id = new_id.to_string();

        Ok(())
    }

    pub fn close_pane(&mut self, pane_id: &str) -> Result<(), LayoutError> {
        if self.panes.len() <= 1 {
            return Err(LayoutError::WouldEmptyWorkspace);
        }

        let idx = self
            .panes
            .iter()
            .position(|p| p.pane_id == pane_id)
            .ok_or(LayoutError::PaneNotFound)?;

        let was_focused = self.focused_pane_id == pane_id;
        self.panes.remove(idx);
        self.split_count -= 1;

        if was_focused {
            let new_idx = if idx > 0 { idx - 1 } else { 0 };
            self.focused_pane_id = self.panes[new_idx].pane_id.clone();
        }

        Ok(())
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

    #[test]
    fn splitting_a_pane_creates_a_new_focused_terminal_slot() {
        let mut layout = WorkspaceLayout::starter();

        layout
            .split_pane("pane-1", "pane-2", SplitOrientation::Vertical, 0.5)
            .expect("starter pane should split");

        assert_eq!(layout.snapshot().pane_count, 2);
        assert_eq!(layout.snapshot().split_count, 1);
        assert_eq!(layout.focused_pane_id, "pane-2");
        assert_eq!(
            layout
                .panes
                .iter()
                .map(|pane| pane.pane_id.as_str())
                .collect::<Vec<_>>(),
            vec!["pane-1", "pane-2"]
        );
        assert_eq!(layout.panes[1].session_kind, SessionKind::FreshShell);
    }

    #[test]
    fn split_rejects_unknown_panes_and_invalid_ratios() {
        let mut layout = WorkspaceLayout::starter();

        let missing = layout.split_pane("missing", "pane-2", SplitOrientation::Vertical, 0.5);
        let zero_ratio = layout.split_pane("pane-1", "pane-2", SplitOrientation::Vertical, 0.0);
        let full_ratio = layout.split_pane("pane-1", "pane-2", SplitOrientation::Horizontal, 1.0);

        assert!(matches!(missing, Err(LayoutError::PaneNotFound)));
        assert!(matches!(zero_ratio, Err(LayoutError::InvalidRatio)));
        assert!(matches!(full_ratio, Err(LayoutError::InvalidRatio)));
    }

    #[test]
    fn closing_a_focused_pane_rebalances_and_moves_focus_to_a_neighbor() {
        let mut layout = WorkspaceLayout::starter();
        layout
            .split_pane("pane-1", "pane-2", SplitOrientation::Vertical, 0.5)
            .expect("first split should work");
        layout
            .split_pane("pane-2", "pane-3", SplitOrientation::Horizontal, 0.5)
            .expect("second split should work");

        layout.close_pane("pane-3").expect("focused pane should close");

        assert_eq!(layout.snapshot().pane_count, 2);
        assert_eq!(layout.snapshot().split_count, 1);
        assert_eq!(layout.focused_pane_id, "pane-2");
        assert_eq!(
            layout
                .panes
                .iter()
                .map(|pane| pane.pane_id.as_str())
                .collect::<Vec<_>>(),
            vec!["pane-1", "pane-2"]
        );
    }

    #[test]
    fn closing_the_last_remaining_pane_is_rejected() {
        let mut layout = WorkspaceLayout::starter();

        let closed = layout.close_pane("pane-1");

        assert!(matches!(closed, Err(LayoutError::WouldEmptyWorkspace)));
        assert_eq!(layout.snapshot().pane_count, 1);
        assert_eq!(layout.focused_pane_id, "pane-1");
    }

    #[test]
    fn split_rejects_duplicate_new_pane_ids() {
        let mut layout = WorkspaceLayout::starter();
        layout
            .split_pane("pane-1", "pane-2", SplitOrientation::Vertical, 0.5)
            .expect("first split should work");

        let duplicate = layout.split_pane("pane-1", "pane-2", SplitOrientation::Horizontal, 0.5);

        assert!(matches!(duplicate, Err(LayoutError::DuplicatePaneId)));
        assert_eq!(
            layout
                .panes
                .iter()
                .map(|pane| pane.pane_id.as_str())
                .collect::<Vec<_>>(),
            vec!["pane-1", "pane-2"]
        );
    }

    #[test]
    fn closing_a_non_focused_pane_keeps_focus_on_the_existing_pane() {
        let mut layout = WorkspaceLayout::starter();
        layout
            .split_pane("pane-1", "pane-2", SplitOrientation::Vertical, 0.5)
            .expect("first split should work");
        layout
            .split_pane("pane-2", "pane-3", SplitOrientation::Horizontal, 0.5)
            .expect("second split should work");

        layout
            .close_pane("pane-1")
            .expect("non-focused pane should close cleanly");

        assert_eq!(layout.focused_pane_id, "pane-3");
        assert_eq!(
            layout
                .panes
                .iter()
                .map(|pane| pane.pane_id.as_str())
                .collect::<Vec<_>>(),
            vec!["pane-2", "pane-3"]
        );
    }
}
