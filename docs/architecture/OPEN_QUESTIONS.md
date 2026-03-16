# cmux-win Open Questions

## Purpose

This file tracks deliberate unknowns that should not be silently guessed during implementation.

Unknowns are not bugs. They are pending product or engineering choices.

## Product Questions

Current product-level decisions for pane closing, theme import, scrollback direction, and notification policy have been resolved and moved into `DECISIONS.md`.

## Engineering Questions

Current engineering-level decisions for PTY encapsulation, metadata refresh, session snapshot storage, and CLI packaging have been resolved and moved into `DECISIONS.md`.

## Process Rule

When resolving an item here:

1. Move the conclusion into `DECISIONS.md`
2. Update any affected spec files
3. Add tests if runtime behavior changes
