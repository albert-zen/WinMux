# Redesign: cmux-style Terminal Multiplexer UI

## Current Problems

The current UI looks like a web dashboard, not a terminal multiplexer:
- Too many forms and buttons in sidebar
- Pane cards with headers/borders look like web components
- No notification indicators
- Workspace switching is clunky
- Doesn't feel native or terminal-like

## cmux Reference Design

cmux (https://github.com/manaflow-ai/cmux) key features:
1. **Vertical tabs sidebar** - Minimal, shows workspace metadata
2. **Notification rings** - Blue ring on panes needing attention
3. **Terminal-centric** - Panes fill the space, no card chrome
4. **Dark by default** - Native terminal aesthetic
5. **Keyboard-first** - Cmd+N, Cmd+1-9, Cmd+D for everything

## New UI Design

### Layout

```
┌──────────────────────────────────────────────────────────────┐
│ ┌─────────┐ ┌────────────────────────────────────────────────┤
│ │ workspace│ │                                                │
│ │ tabs    │ │                                                │
│ │         │ │              Terminal Panes                    │
│ │ ▸ inbox │ │                                                │
│ │   main  │ │         (no card borders, fill space)          │
│ │ ▸ feat  │ │                                                │
│ │         │ │                                                │
│ │ ─────── │ │                                                │
│ │ + New   │ │                                                │
│ └─────────┘ └────────────────────────────────────────────────┤
│ ┌─────────────────────────────────────────────────────────────┤
│ │ Status bar: workspace name | branch | notification count   │
│ └─────────────────────────────────────────────────────────────┘
└──────────────────────────────────────────────────────────────┘
```

### Sidebar Changes

**Remove:**
- Workspace create form (move to modal/shortcut)
- Rename form (move to modal)
- Theme selector dropdown (move to settings)
- Root directory input
- Shell profile input
- Pane ID labels
- "Split Right/Down" buttons (use keyboard shortcuts)

**Add:**
- Minimal workspace tabs with:
  - Workspace name
  - Git branch (if detected)
  - Notification indicator dot
  - Active state highlight
- "+" button at bottom for new workspace
- Settings gear icon

### Pane Changes

**Remove:**
- Card borders and shadows
- Pane header with ID/session ID
- Status badges
- Close button (use Cmd+Shift+W)
- Restart button overlay (use Cmd+R or context menu)

**Add:**
- Blue ring/border when notification pending
- Subtle focus indicator (border color change)
- Split handles remain but more subtle

### Color Scheme

Match terminal aesthetic:
- Background: #1a1a1a (terminal black)
- Foreground: #f0f0f0
- Accent: #3b82f6 (blue for notifications)
- Focused border: subtle highlight
- Split handles: barely visible, highlight on hover

### Workspace Creation Flow

Instead of form always visible:
1. Click "+" or Cmd+N → Modal appears
2. Enter workspace name (required)
3. Optional: directory, shell profile (pre-filled)
4. Enter to create, Escape to cancel

### Files to Modify

1. `App.tsx` - Remove forms, add modal state, simplify layout
2. `App.css` - Terminal-ify colors, remove card styles
3. `WorkspaceSplitView.tsx` - Remove pane headers, add notification ring
4. `components/WorkspaceSidebar.tsx` - NEW: Minimal vertical tabs
5. `components/CreateWorkspaceModal.tsx` - NEW: Modal for workspace creation
6. `components/StatusBar.tsx` - NEW: Bottom status bar

### Keyboard Shortcuts (keep existing, add)

| Shortcut | Action |
|----------|--------|
| Cmd/Ctrl+N | New workspace modal |
| Cmd/Ctrl+1-9 | Jump to workspace |
| Cmd/Ctrl+B | Toggle sidebar |
| Cmd/Ctrl+D | Split right |
| Cmd/Ctrl+Shift+D | Split down |

## Implementation Order

1. Simplify sidebar - remove forms, keep workspace list only
2. Create workspace modal
3. Update colors to terminal aesthetic
4. Remove pane chrome (headers, borders)
5. Add notification ring styling
6. Add status bar
7. Wire up keyboard shortcuts for workspace switching
