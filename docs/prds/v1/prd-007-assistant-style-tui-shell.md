---
title: Assistant-style TUI shell
summary: Make the default skills-manager experience a full-screen assistant-style terminal UI for list, scan, import, remove, config, and help flows.
status: done
roadmap: v1
---

# Assistant-style TUI shell

## Context

Skills Manager is intended to feel like a terminal-first assistant-style devtool, not a raw table or one-shot CLI. The CLI is useful, but running `skills-manager` with no arguments should open the primary product experience: a full-screen TUI with a prompt, status feed, hints, and complete guided workflows for V1 import and removal.

## Problem

A table-only interface would expose data but not create a confident management workflow. Users need to see environment status, type commands, move through list/import/remove modes, choose between ambiguous matches, review plans, apply confirmed changes, and quit cleanly without memorizing every CLI flag.

## Goal

Build the V1 TUI shell that hosts the complete core workflows and makes skill exposure management feel coherent and safe.

## Non-goals

- Do not implement advanced table cell toggling in V1.
- Do not build rich config editing inside the TUI.
- Do not add themes beyond one calm default visual style.
- Do not make the TUI responsible for business logic that belongs in config, scanning, inventory, or planning modules.
- Do not require mouse support.
- Do not route users out to CLI commands for V1 import/remove completion.

## Proposed experience

Running:

```bash
skills-manager
```

opens a full-screen interface with:

- a header showing app name and purpose,
- short environment/status feed,
- main content area,
- sticky bottom prompt,
- footer command hints.

The user can type `list`, `scan`, `import`, `remove`, `config`, or `help`. Slash-style commands such as `/list` and `/help` should also work.

Reference layout for the default shell:

```text
┌──────────────────────────────────────────────────────────────┐
│ Skills Manager v0.1                                          │
│ Manage local skills for Codex, Claude and Copilot            │
│ Type a command or use /help to get started                   │
└──────────────────────────────────────────────────────────────┘

• Environment loaded: 39 skills, 3 agents, 1 config
• Scan status: OK
• Git status: OK

[main content area]

~/projects/skills-manager [main]
> _
---------------------------------------------------------------
/ commands   ? help   esc cancel   enter apply
```

The main content area changes by mode, but the prompt and footer remain sticky so the shell always feels command-driven.

## Requirements

- Use `ratatui` with `crossterm`.
- Running the binary without subcommands must open the TUI.
- Split the screen into header, status feed, main content panel, prompt bar, and footer shortcuts.
- Support home, list, scan, import, remove, config, and help modes.
- Keep the bottom prompt visible.
- Support plain commands and slash commands.
- Support minimum keybindings: `q`, `?`, `/`, `esc`, `tab`, `enter`, `up/down`, and `left/right` where applicable.
- List mode must render inventory rows from the same inventory model used by the CLI.
- Import and remove modes must be complete in V1: selection, ambiguity resolution, target choice, staged plan preview, confirmation, apply, rescan, and result rendering all happen inside the TUI.
- Import and remove modes must use the shared safe staged plan behavior also used by CLI flows.
- Config mode must show config path and key values; advanced editing can remain manual.
- Dialogs must clearly distinguish ordinary confirmation from physical-copy deletion warning.
- Ambiguous skills or exposures must be shown as numbered choices such as `(1)` and `(2)` with enough source path or origin context to choose safely.
- Quit cleanly and restore the terminal on normal exit and recoverable errors.

## Technical implementation notes

Follow the module structure from the engineering guidelines:

- `src/tui/app.rs` owns state such as mode, input buffer, inventory, staged changes, selected row, status messages, and active dialog.
- `src/tui/layout.rs` defines the fixed screen regions.
- `src/tui/theme.rs` defines colors and text styles.
- `src/tui/events.rs` routes keyboard events.
- `src/tui/components/` contains header, status, main panel, prompt, footer, table, and dialog renderers.

The TUI should call existing application services rather than duplicate logic. For example, list mode calls inventory resolution; scan mode calls scanner; import/remove modes build staged changes through the same planning code used by CLI commands. The TUI owns interaction state, not mutation policy.

### Theme

Use a calm dark theme with the following palette:

- background: very dark blue/black
- main text: light gray
- muted text: gray
- primary accent: purple
- secondary accent: cyan
- warning: yellow
- error: red
- success: green
- borders: subtle, not heavy

Visual rules:

- Do not overload the screen with boxes.
- Keep the prompt clearly visible at all times.
- Keep status lines short.
- Use one strong header card, not many decorative panels.
- Prefer useful content over visual noise.
- Keep list/table rendering readable without letting it dominate the product.

### Prompt behavior

The bottom prompt is the primary interaction point and should accept both plain commands and slash-style commands such as `list` and `/list`.

When possible, the prompt should show the current directory and Git branch above the input line:

```text
~/projects/skills-manager [main]
> _
```

If Git branch detection fails, show only the current directory and keep the input line unchanged.

### Keybindings

#### V1 minimum keybindings

| Key | Behavior |
| --- | --- |
| `q` | Quit the TUI. |
| `?` | Open help. |
| `/` | Focus the prompt. |
| `esc` | Cancel the current action or close the active dialog. |
| `tab` | Switch section or mode where applicable. |
| `enter` | Confirm or execute the current action. |
| `up/down` | Move through list content. |
| `left/right` | Switch panel or column where applicable. |

#### Later keybindings

| Key | Behavior |
| --- | --- |
| `space` | Toggle a staged change or exposure in advanced list interactions. |
| `a` | Apply staged changes. |
| `r` | Refresh or rescan. |
| `i` | Start import flow. |
| `x` | Start remove flow. |

### List mode behavior

List mode should render inventory rows from the shared inventory model with columns for skill, source, Claude, Codex, Copilot, scope, and connection.

V1 list behavior:

- scroll the list
- select a row
- open details for the selected row
- show duplicate display identities as numbered disambiguation choices with source path or origin context

Later list behavior:

- support cell navigation
- allow `space` to toggle exposure
- stage changes from the list view
- apply staged changes from the list view

## Success criteria

- A user can launch the app, view home status, run list and scan flows, complete import/remove/config/help flows, and quit cleanly.
- The TUI can preview, confirm, apply, rescan, and render results for import/remove plans.
- Physical-copy deletion warnings are visually distinct.
- Terminal state is restored after exit.
- TUI behavior is covered by unit tests for command parsing and state transitions where practical.

## Progress notes

- 2026-06-03: Implemented the assistant-style TUI shell with ratatui/crossterm. Running `skills-manager` with no subcommand opens the TUI in interactive terminals.
- 2026-06-03: Added home, list, scan, import, remove, config, help, and quit modes with a fixed header/status/main/prompt/footer layout.
- 2026-06-03: Added prompt command parsing for plain and slash commands, sticky prompt rendering with current directory and Git branch, and key handling for enter, escape, help, quit, list movement, slash input, and Ctrl-C.
- 2026-06-03: Added guided import/remove flows that use the shared staged plan/apply behavior, show plan previews, require confirmation, visually distinguish physical-copy deletion, apply confirmed changes, rescan inventory, and render results.
- 2026-06-03: Updated TUI inventory rendering so duplicate display identities include numbered labels and source context.

## Edge cases

- Terminal is too small for the full layout.
- Inventory scan fails while the TUI is open.
- User presses escape during a confirmation dialog.
- User enters an unknown command.
- Config is missing on startup.
- The app exits after an error and must restore terminal state.

## Dependencies

- Roadmap items `004`, `005`, and `006`.
- Uses foundation and config from roadmap items `001` and `002`.

## Open questions

- Should missing config on TUI startup route directly into config guidance or show home mode with a blocking warning?
- What is the smallest guided import/remove interaction that still feels complete without advanced table cell toggling?
