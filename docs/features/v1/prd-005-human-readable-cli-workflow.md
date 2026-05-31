---
title: Human-readable CLI workflow
summary: Provide scriptable but user-first CLI commands for listing, scanning, importing, removing, config inspection, and diagnostics.
status: planned
roadmap: v1
---

# Human-readable CLI workflow

## Context

The default Skills Manager experience is the assistant-style TUI, but direct CLI commands are valuable for testing, debugging, and users who prefer one-shot operations. The CLI gives V1 a reliable way to exercise the same services that the complete TUI import/remove flows use.

## Problem

If the project waits for the TUI before exposing behavior, core workflows become harder to test. If the CLI is designed as a full automation API too early, it may lock the product into formats and flags before real usage is understood.

## Goal

Add human-readable CLI flows for core V1 operations while keeping the TUI as the primary product experience and avoiding premature stable machine-readable contracts.

## Non-goals

- Do not add stable JSON output in V1.
- Do not make CLI design more important than the TUI.
- Do not bypass confirmation for mutating commands.
- Do not expose shared implementation folders such as `.agents` as command targets.
- Do not add remote Git import.
- Do not add arbitrary project targeting in V1.

## Proposed experience

Users can run:

```bash
skills-manager list
skills-manager scan
skills-manager import repo-a/code-review --to claude,codex
skills-manager remove repo-a/code-review --from claude
skills-manager config show
skills-manager doctor
```

Read-only commands render compact tables or status lines. Mutating commands render a plan, ask for confirmation, apply only after confirmation, then rescan and show the resulting state.

## Requirements

- Implement CLI parsing for `list`, `scan`, `import`, `remove`, `config`, and `doctor`.
- Render `list`, `scan`, and `config show` as human-readable output.
- Keep command handlers thin and delegate to config, scanner, inventory, and plan modules.
- `list` must show current inventory state.
- `scan` must show discovered skills and never mutate disk.
- `import` must select a discovered skill, select target agents, prepare a change plan, confirm, apply, rescan, and render the result.
- `remove` must select an existing exposure, classify connection type, prepare a change plan, confirm, apply, rescan, and render the result.
- If an import/remove identifier is ambiguous, the CLI must print numbered options such as `(1)` and `(2)` with path or origin context and ask the user to choose one. In non-interactive mode, ambiguous mutation commands must fail rather than guessing.
- `doctor` must check config existence, configured source directories, configured target directories, local Git CLI availability, and target directory writability.
- Use snapshot tests for stable human-readable output where useful.
- Make errors actionable and include affected paths for config, scan, and mutation failures.

## Technical implementation notes

Put command handlers under `src/commands/`. The command handlers should not directly perform low-level filesystem mutation; they should call plan/apply helpers from the safe import/removal feature.

Use `assert_cmd` for command-level tests and `insta` for snapshots. Use `tempfile` to build isolated config, source, and target directory layouts.

Do not introduce table-rendering dependencies unless approved. A simple formatter in `src/output.rs` is enough for V1 human-readable tables.

Prompts should be explicit. For example, import can ask `Apply this plan? [y/N]`. Physical-copy deletion needs stronger confirmation from the safe plan feature.

## Success criteria

- CLI commands work end to end in temporary filesystem tests.
- Read-only commands are useful without opening the TUI.
- Mutating commands always show a plan before applying.
- Import/remove flows rescan and render actual post-apply state.
- CLI output remains compact enough for terminal use.

## Edge cases

- Import target already exists.
- Import skill identifier is ambiguous or unknown.
- User selects a numbered disambiguation option that is no longer valid after rescan.
- User declines confirmation.
- Standard input is non-interactive.
- `git` is not installed or not on `PATH`.
- A configured target path is not writable.

## Dependencies

- Roadmap items `002`, `003`, `004`, and `006` for mutating import/remove behavior.

## Open questions

- Should non-interactive mutation commands fail unless a future explicit `--yes` flag exists?
- How much detail should `doctor` print by default versus only on failure?
