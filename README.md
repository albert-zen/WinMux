# cmux-win

cmux-win is a Windows-first desktop terminal workspace manager inspired by cmux.

It is not a line-by-line port of the macOS project. It is a fresh implementation that keeps the useful product model while choosing a stack and behavior model that make sense on Windows.

## What We Are Building

The app should help a developer manage several active terminal work contexts inside one desktop application.

Core capabilities:

- Multiple workspaces
- Split terminal panes
- Sidebar metadata
- Terminal-triggered and CLI-triggered notifications
- Local socket API for CLI and automation
- Theme compatibility
- Session restore
- Auto-update

Not in scope for v0.1:

- Embedded browser panel
- Browser automation
- Cloud sync
- Multi-user collaboration
- Full tmux compatibility
- Restoring live process memory

## Product Intent

The point is not just "another terminal".

The point is to create a lightweight operating surface for development sessions:

- A workspace is a durable unit of context.
- Split panes let one workspace contain several related processes.
- The sidebar shows useful context without asking the user to remember it.
- Notifications let long-running tasks surface themselves.
- IPC and CLI make the app scriptable and automatable.
- Restore and themes make it feel like a real daily-use tool rather than a demo.

## Guiding Interpretation

This project should be treated as:

- A desktop productivity tool
- A terminal workspace orchestrator
- A locally scriptable app platform

This project should not be treated as:

- A shell replacement
- A browser shell
- A remote terminal server
- A VM or container manager

## Why Fresh Implementation Instead Of Port

The original cmux is tightly coupled to macOS-native implementation choices.

The useful parts to preserve are:

- Product model
- User workflows
- Surface hierarchy
- Automation hooks
- Notification ideas

The implementation should be different where Windows requires different primitives.

## Stack Summary

- Tauri 2 for the desktop shell
- Rust for core logic, PTY lifecycle, IPC, persistence, and orchestration
- React + TypeScript for the UI
- xterm.js for terminal rendering
- ConPTY for Windows pseudo-terminal support
- SQLite plus small JSON config surfaces for persistence

## Current Status

The repository is past the bootstrap stage.

What already works today:

- live ConPTY-backed terminal sessions
- named-pipe transport between desktop backend and CLI
- xterm.js terminal panes in the desktop app
- split, focus, close, resize, and restart flows for panes
- streamed terminal output plus snapshot polling fallback

What is still incomplete:

- desktop workspace create/list/switch flow
- authoritative persisted split-tree layout model
- restore and reopen behavior
- broader hardening for long-running daily use

## Quick Start

1. Install Rust and the Windows build prerequisites for Tauri.
2. Run `pnpm install` at the repository root.
3. Run `pnpm dev:web` to start the frontend only.
4. Run `pnpm dev` to start the full desktop app.
5. Run `pnpm test` to execute the current TypeScript and Rust smoke tests.

If `cargo` is not available in a fresh shell after installing Rust, reopen the terminal so the Rust toolchain path is picked up.

## Repository Layout

- `apps/desktop` contains the Tauri desktop application shell
- `crates/*` contains Rust domain crates
- `packages/protocol` contains shared TypeScript protocol types
- `packages/ui` contains shared React UI primitives
- `packages/cli` contains the future bundled CLI entrypoint
- `docs/` contains project documentation

## Success Criteria

The app is successful when a Windows developer can:

1. Create and switch between several workspaces quickly.
2. Split panes for logs, servers, shells, and test loops.
3. Restart the app and get their workspace structure back.
4. Receive useful notifications from commands and agents.
5. Control the app through a local CLI or script.
6. Use a theme that feels familiar.
7. Install updates without manual replacement workflows.

The current codebase is not at that bar yet, but it is now beyond a static demo:
it already behaves like a rough multi-pane terminal workspace prototype.

## Where To Read Next

- `docs/README.md` for the documentation index
- `docs/architecture/ARCHITECTURE.md` for module boundaries and system shape
- `docs/product/PRODUCT_SPEC.md` for user workflows and product semantics
- `docs/product/DOMAIN_MODEL.md` for exact meanings of core entities
- `docs/architecture/IPC_SPEC.md` for local protocol expectations
- `docs/process/TESTING.md` for TDD and review-loop rules
- `docs/process/TASKS.md` for the implementation split
- `docs/process/HANDOFF.md` for onboarding future contributors and agents
