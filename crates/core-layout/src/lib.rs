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

// ---------------------------------------------------------------------------
// LayoutNode — recursive split tree
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum LayoutNode {
    Pane(PaneSlot),
    Split {
        orientation: SplitOrientation,
        ratio: f64,
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
    },
}

impl LayoutNode {
    /// In-order (left-to-right) leaf traversal.
    fn collect_panes(&self) -> Vec<&PaneSlot> {
        match self {
            LayoutNode::Pane(slot) => vec![slot],
            LayoutNode::Split { first, second, .. } => {
                let mut panes = first.collect_panes();
                panes.extend(second.collect_panes());
                panes
            }
        }
    }

    /// Mutable in-order leaf traversal.
    fn collect_panes_mut(&mut self) -> Vec<&mut PaneSlot> {
        match self {
            LayoutNode::Pane(slot) => vec![slot],
            LayoutNode::Split { first, second, .. } => {
                let mut panes = first.collect_panes_mut();
                panes.extend(second.collect_panes_mut());
                panes
            }
        }
    }

    /// Count leaf (Pane) nodes.
    fn pane_count(&self) -> usize {
        match self {
            LayoutNode::Pane(_) => 1,
            LayoutNode::Split { first, second, .. } => first.pane_count() + second.pane_count(),
        }
    }

    /// Count internal Split nodes.
    fn split_count(&self) -> usize {
        match self {
            LayoutNode::Pane(_) => 0,
            LayoutNode::Split { first, second, .. } => {
                1 + first.split_count() + second.split_count()
            }
        }
    }

    /// Check whether any leaf has the given pane_id.
    fn contains_pane(&self, pane_id: &str) -> bool {
        match self {
            LayoutNode::Pane(slot) => slot.pane_id == pane_id,
            LayoutNode::Split { first, second, .. } => {
                first.contains_pane(pane_id) || second.contains_pane(pane_id)
            }
        }
    }

    /// Find a mutable reference to a leaf by pane_id.
    fn find_pane_mut(&mut self, pane_id: &str) -> Option<&mut PaneSlot> {
        match self {
            LayoutNode::Pane(slot) => {
                if slot.pane_id == pane_id {
                    Some(slot)
                } else {
                    None
                }
            }
            LayoutNode::Split { first, second, .. } => first
                .find_pane_mut(pane_id)
                .or_else(|| second.find_pane_mut(pane_id)),
        }
    }

    /// Return the leftmost leaf.
    fn first_pane(&self) -> &PaneSlot {
        match self {
            LayoutNode::Pane(slot) => slot,
            LayoutNode::Split { first, .. } => first.first_pane(),
        }
    }

    /// Replace the leaf with `source_id` by a Split containing the original
    /// pane as `first` and `new_pane` as `second`.
    /// Returns true if the replacement was made, false if source_id not found.
    fn split_pane(
        &mut self,
        source_id: &str,
        new_pane: PaneSlot,
        orientation: SplitOrientation,
        ratio: f64,
    ) -> bool {
        match self {
            LayoutNode::Pane(slot) => {
                if slot.pane_id == source_id {
                    let original = LayoutNode::Pane(slot.clone());
                    *self = LayoutNode::Split {
                        orientation,
                        ratio,
                        first: Box::new(original),
                        second: Box::new(LayoutNode::Pane(new_pane)),
                    };
                    true
                } else {
                    false
                }
            }
            LayoutNode::Split { first, second, .. } => {
                first.split_pane(source_id, new_pane.clone(), orientation.clone(), ratio)
                    || second.split_pane(source_id, new_pane, orientation, ratio)
            }
        }
    }

    /// Remove a leaf by pane_id, collapsing its parent Split by returning
    /// the sibling. Returns `None` if the root itself IS that pane (caller
    /// handles). Returns `Some(new_tree)` with the pane removed.
    fn remove_pane(&self, pane_id: &str) -> Option<LayoutNode> {
        match self {
            LayoutNode::Pane(slot) => {
                if slot.pane_id == pane_id {
                    // Root is the pane itself — caller must handle
                    None
                } else {
                    // This pane is not the target; return unchanged
                    Some(self.clone())
                }
            }
            LayoutNode::Split {
                first, second, orientation, ratio,
            } => {
                // Check if either direct child is the target pane
                if let LayoutNode::Pane(slot) = first.as_ref() {
                    if slot.pane_id == pane_id {
                        // Remove first child, collapse to second
                        return Some(second.as_ref().clone());
                    }
                }
                if let LayoutNode::Pane(slot) = second.as_ref() {
                    if slot.pane_id == pane_id {
                        // Remove second child, collapse to first
                        return Some(first.as_ref().clone());
                    }
                }

                // Recurse into children
                let new_first = first.remove_pane(pane_id);
                if let Some(new_first) = new_first {
                    if &new_first != first.as_ref() {
                        // Removal happened in first subtree
                        return Some(LayoutNode::Split {
                            orientation: orientation.clone(),
                            ratio: *ratio,
                            first: Box::new(new_first),
                            second: second.clone(),
                        });
                    }
                } else {
                    // first was the pane itself (shouldn't reach here since
                    // we checked Pane case above), but handle gracefully
                    return Some(second.as_ref().clone());
                }

                let new_second = second.remove_pane(pane_id);
                if let Some(new_second) = new_second {
                    if &new_second != second.as_ref() {
                        return Some(LayoutNode::Split {
                            orientation: orientation.clone(),
                            ratio: *ratio,
                            first: first.clone(),
                            second: Box::new(new_second),
                        });
                    }
                } else {
                    return Some(first.as_ref().clone());
                }

                // Pane not found in this subtree — return unchanged
                Some(self.clone())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// WorkspaceLayout — tree-backed
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceLayout {
    root: LayoutNode,
    focused_pane_id: String,
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
    // --- Public accessors ---------------------------------------------------

    /// Ordered leaves (left-to-right traversal), replaces the old `pub panes` field.
    #[must_use]
    pub fn panes(&self) -> Vec<&PaneSlot> {
        self.root.collect_panes()
    }

    /// Current focused pane id, replaces the old `pub focused_pane_id` field.
    #[must_use]
    pub fn focused_pane_id(&self) -> &str {
        &self.focused_pane_id
    }

    /// Direct access to the tree root.
    #[must_use]
    pub fn root(&self) -> &LayoutNode {
        &self.root
    }

    /// True when the tree has no panes (for restore validation).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.root.pane_count() == 0
    }

    // --- Constructors -------------------------------------------------------

    #[must_use]
    pub fn starter() -> Self {
        Self {
            root: LayoutNode::Pane(PaneSlot {
                pane_id: "pane-1".to_string(),
                session_kind: SessionKind::RunningShell,
            }),
            focused_pane_id: "pane-1".to_string(),
        }
    }

    // --- Queries ------------------------------------------------------------

    #[must_use]
    pub fn snapshot(&self) -> LayoutSnapshot {
        LayoutSnapshot {
            pane_count: self.root.pane_count(),
            split_count: self.root.split_count(),
        }
    }

    // --- Mutations ----------------------------------------------------------

    pub fn close_session(&mut self, pane_id: &str) -> Option<CloseSessionResult> {
        let pane = self.root.find_pane_mut(pane_id)?;
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
        orientation: SplitOrientation,
        ratio: f64,
    ) -> Result<(), LayoutError> {
        // Validate source exists
        if !self.root.contains_pane(source_id) {
            return Err(LayoutError::PaneNotFound);
        }

        // Validate ratio
        if ratio <= 0.0 || ratio >= 1.0 {
            return Err(LayoutError::InvalidRatio);
        }

        // Validate no duplicate
        if self.root.contains_pane(new_id) {
            return Err(LayoutError::DuplicatePaneId);
        }

        let new_pane = PaneSlot {
            pane_id: new_id.to_string(),
            session_kind: SessionKind::FreshShell,
        };

        self.root.split_pane(source_id, new_pane, orientation, ratio);
        self.focused_pane_id = new_id.to_string();

        Ok(())
    }

    pub fn focus_pane(&mut self, pane_id: &str) -> Result<(), LayoutError> {
        if !self.root.contains_pane(pane_id) {
            return Err(LayoutError::PaneNotFound);
        }
        self.focused_pane_id = pane_id.to_string();
        Ok(())
    }

    pub fn close_pane(&mut self, pane_id: &str) -> Result<(), LayoutError> {
        if self.root.pane_count() <= 1 {
            return Err(LayoutError::WouldEmptyWorkspace);
        }

        if !self.root.contains_pane(pane_id) {
            return Err(LayoutError::PaneNotFound);
        }

        // Build pre-removal pane ID list for focus recovery
        let pre_ids: Vec<String> = self
            .root
            .collect_panes()
            .iter()
            .map(|p| p.pane_id.clone())
            .collect();

        let was_focused = self.focused_pane_id == pane_id;

        // Find index of closing pane
        let idx = pre_ids.iter().position(|id| id == pane_id).unwrap();

        // Remove pane from tree
        let new_root = self.root.remove_pane(pane_id);
        match new_root {
            Some(tree) => self.root = tree,
            None => {
                // Root was the pane itself — but we already checked pane_count > 1
                // so this shouldn't happen. Return error defensively.
                return Err(LayoutError::WouldEmptyWorkspace);
            }
        }

        // Update focus if needed
        if was_focused {
            let remaining: Vec<String> = self
                .root
                .collect_panes()
                .iter()
                .map(|p| p.pane_id.clone())
                .collect();
            let new_idx = if idx > 0 { idx - 1 } else { 0 };
            // Clamp to remaining length
            let clamped = new_idx.min(remaining.len() - 1);
            self.focused_pane_id = remaining[clamped].clone();
        }

        Ok(())
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Original 12 tests (updated for new API)
    // -----------------------------------------------------------------------

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
        assert_eq!(layout.panes()[0].session_kind, SessionKind::FreshShell);
        assert_eq!(layout.focused_pane_id(), "pane-1");
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
        assert_eq!(layout.focused_pane_id(), "pane-2");
        assert_eq!(
            layout
                .panes()
                .iter()
                .map(|pane| pane.pane_id.as_str())
                .collect::<Vec<_>>(),
            vec!["pane-1", "pane-2"]
        );
        assert_eq!(layout.panes()[1].session_kind, SessionKind::FreshShell);
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
    fn focusing_an_existing_pane_updates_focus() {
        let mut layout = WorkspaceLayout::starter();
        layout
            .split_pane("pane-1", "pane-2", SplitOrientation::Vertical, 0.5)
            .expect("split should work");

        layout.focus_pane("pane-1").expect("existing pane should focus");

        assert_eq!(layout.focused_pane_id(), "pane-1");
    }

    #[test]
    fn focusing_a_missing_pane_is_rejected() {
        let mut layout = WorkspaceLayout::starter();

        let err = layout.focus_pane("pane-missing");

        assert!(matches!(err, Err(LayoutError::PaneNotFound)));
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
        assert_eq!(layout.focused_pane_id(), "pane-2");
        assert_eq!(
            layout
                .panes()
                .iter()
                .map(|pane| pane.pane_id.as_str())
                .collect::<Vec<_>>(),
            vec!["pane-1", "pane-2"]
        );
    }

    #[test]
    fn closing_a_non_focused_pane_keeps_current_focus() {
        let mut layout = WorkspaceLayout::starter();
        layout
            .split_pane("pane-1", "pane-2", SplitOrientation::Vertical, 0.5)
            .expect("first split should work");
        layout
            .split_pane("pane-2", "pane-3", SplitOrientation::Horizontal, 0.5)
            .expect("second split should work");

        layout.focus_pane("pane-3").expect("pane-3 should focus");
        layout
            .close_pane("pane-1")
            .expect("non-focused pane should close");

        assert_eq!(layout.focused_pane_id(), "pane-3");
        assert_eq!(
            layout
                .panes()
                .iter()
                .map(|pane| pane.pane_id.as_str())
                .collect::<Vec<_>>(),
            vec!["pane-2", "pane-3"]
        );
    }

    #[test]
    fn closing_the_last_remaining_pane_is_rejected() {
        let mut layout = WorkspaceLayout::starter();

        let closed = layout.close_pane("pane-1");

        assert!(matches!(closed, Err(LayoutError::WouldEmptyWorkspace)));
        assert_eq!(layout.snapshot().pane_count, 1);
        assert_eq!(layout.focused_pane_id(), "pane-1");
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
                .panes()
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

        assert_eq!(layout.focused_pane_id(), "pane-3");
        assert_eq!(
            layout
                .panes()
                .iter()
                .map(|pane| pane.pane_id.as_str())
                .collect::<Vec<_>>(),
            vec!["pane-2", "pane-3"]
        );
    }

    // -----------------------------------------------------------------------
    // 7 new tree-specific tests
    // -----------------------------------------------------------------------

    #[test]
    fn split_pane_records_orientation_in_tree() {
        let mut layout = WorkspaceLayout::starter();
        layout
            .split_pane("pane-1", "pane-2", SplitOrientation::Vertical, 0.5)
            .expect("split should work");

        match layout.root() {
            LayoutNode::Split { orientation, .. } => {
                assert_eq!(*orientation, SplitOrientation::Vertical);
            }
            _ => panic!("root should be a Split node after splitting"),
        }
    }

    #[test]
    fn nested_splits_produce_correct_tree_structure() {
        let mut layout = WorkspaceLayout::starter();
        layout
            .split_pane("pane-1", "pane-2", SplitOrientation::Vertical, 0.5)
            .expect("first split");
        layout
            .split_pane("pane-2", "pane-3", SplitOrientation::Horizontal, 0.5)
            .expect("second split");

        assert_eq!(layout.snapshot().pane_count, 3);
        assert_eq!(layout.snapshot().split_count, 2);

        // Verify tree shape: root Split, first child is Pane("pane-1"),
        // second child is Split containing pane-2 and pane-3
        match layout.root() {
            LayoutNode::Split { first, second, .. } => {
                match first.as_ref() {
                    LayoutNode::Pane(slot) => assert_eq!(slot.pane_id, "pane-1"),
                    _ => panic!("first child should be Pane(pane-1)"),
                }
                match second.as_ref() {
                    LayoutNode::Split {
                        first: inner_first,
                        second: inner_second,
                        orientation,
                        ..
                    } => {
                        assert_eq!(*orientation, SplitOrientation::Horizontal);
                        match inner_first.as_ref() {
                            LayoutNode::Pane(slot) => assert_eq!(slot.pane_id, "pane-2"),
                            _ => panic!("inner first should be Pane(pane-2)"),
                        }
                        match inner_second.as_ref() {
                            LayoutNode::Pane(slot) => assert_eq!(slot.pane_id, "pane-3"),
                            _ => panic!("inner second should be Pane(pane-3)"),
                        }
                    }
                    _ => panic!("second child should be a Split"),
                }
            }
            _ => panic!("root should be a Split"),
        }
    }

    #[test]
    fn panes_returns_leaves_in_left_to_right_order() {
        let mut layout = WorkspaceLayout::starter();
        // Split pane-1 -> pane-1 | pane-2
        layout
            .split_pane("pane-1", "pane-2", SplitOrientation::Vertical, 0.5)
            .expect("split 1");
        // Split pane-1 -> pane-1 | pane-3  (nested inside the first child)
        layout
            .split_pane("pane-1", "pane-3", SplitOrientation::Horizontal, 0.5)
            .expect("split 2");
        // Split pane-2 -> pane-2 | pane-4
        layout
            .split_pane("pane-2", "pane-4", SplitOrientation::Vertical, 0.5)
            .expect("split 3");

        let ids: Vec<&str> = layout.panes().iter().map(|p| p.pane_id.as_str()).collect();
        assert_eq!(ids, vec!["pane-1", "pane-3", "pane-2", "pane-4"]);
    }

    #[test]
    fn closing_first_child_of_split_collapses_correctly() {
        let mut layout = WorkspaceLayout::starter();
        layout
            .split_pane("pane-1", "pane-2", SplitOrientation::Vertical, 0.5)
            .expect("split");

        // Focus pane-2 so closing pane-1 doesn't trigger focus recovery issues
        layout.focus_pane("pane-2").unwrap();

        layout.close_pane("pane-1").expect("close first child");

        assert_eq!(layout.snapshot().pane_count, 1);
        assert_eq!(layout.snapshot().split_count, 0);
        // Root should collapse to a single Pane
        match layout.root() {
            LayoutNode::Pane(slot) => assert_eq!(slot.pane_id, "pane-2"),
            _ => panic!("root should collapse to Pane(pane-2)"),
        }
    }

    #[test]
    fn closing_pane_in_deep_tree_collapses_parent_split() {
        let mut layout = WorkspaceLayout::starter();
        // Build 3 levels: pane-1 | (pane-2 | (pane-3 | pane-4))
        layout
            .split_pane("pane-1", "pane-2", SplitOrientation::Vertical, 0.5)
            .expect("split 1");
        layout
            .split_pane("pane-2", "pane-3", SplitOrientation::Horizontal, 0.5)
            .expect("split 2");
        layout
            .split_pane("pane-3", "pane-4", SplitOrientation::Vertical, 0.5)
            .expect("split 3");

        assert_eq!(layout.snapshot().pane_count, 4);
        assert_eq!(layout.snapshot().split_count, 3);

        // Close pane-3 (one of the deepest leaves). Its parent Split
        // (pane-3 | pane-4) should collapse to just pane-4.
        layout.close_pane("pane-3").expect("close deep pane");

        assert_eq!(layout.snapshot().pane_count, 3);
        assert_eq!(layout.snapshot().split_count, 2);

        let ids: Vec<&str> = layout.panes().iter().map(|p| p.pane_id.as_str()).collect();
        assert_eq!(ids, vec!["pane-1", "pane-2", "pane-4"]);
    }

    #[test]
    fn split_preserves_ratio_in_tree() {
        let mut layout = WorkspaceLayout::starter();
        layout
            .split_pane("pane-1", "pane-2", SplitOrientation::Vertical, 0.3)
            .expect("split with 0.3 ratio");

        match layout.root() {
            LayoutNode::Split { ratio, .. } => {
                assert!(
                    (*ratio - 0.3).abs() < f64::EPSILON,
                    "ratio should be 0.3, got {}",
                    ratio
                );
            }
            _ => panic!("root should be a Split"),
        }
    }

    #[test]
    fn serde_round_trip_multi_level_tree() {
        let mut layout = WorkspaceLayout::starter();
        layout
            .split_pane("pane-1", "pane-2", SplitOrientation::Vertical, 0.5)
            .expect("split 1");
        layout
            .split_pane("pane-2", "pane-3", SplitOrientation::Horizontal, 0.7)
            .expect("split 2");

        let json = serde_json::to_string(&layout).expect("serialize");
        let deserialized: WorkspaceLayout =
            serde_json::from_str(&json).expect("deserialize");

        assert_eq!(layout, deserialized);

        // Verify structure survived
        assert_eq!(deserialized.snapshot().pane_count, 3);
        assert_eq!(deserialized.snapshot().split_count, 2);
        assert_eq!(deserialized.focused_pane_id(), "pane-3");
    }
}
