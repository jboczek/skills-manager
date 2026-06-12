---
title: Assistant-style TUI table actions
summary: Add slash command suggestions, natural table scrolling, scan-table navigation, and table-driven import/remove shortcuts to the assistant-style TUI.
status: done
roadmap: v1
---

# Assistant-style TUI table actions

## Context

PRD-007 delivered the first assistant-style TUI shell for Skills Manager. Running `skills-manager` opens a full-screen terminal interface with a sticky prompt, list and scan views, config/help modes, and guided import/remove flows backed by safe staged plans.

That V1 shell is usable, but the highest-frequency interactions still feel more like typed commands than an interactive management surface. The user can already move through the list table, but the selected-row background does not behave like a standard scrolling table. The scan table renders useful rows but does not support row navigation. Import and remove are still explicit prompt commands even though the roadmap expects future exposure changes to happen from table context.

## Problem

The TUI should help users discover commands and act on visible rows without memorizing exact command syntax. Today:

- pressing `/` starts text input but does not show available commands or what they do,
- list navigation keeps the active-row background pinned too high until the selection reaches the end,
- scan results cannot be navigated with up/down keys,
- import and remove are separate typed workflows instead of actions on selected skills or exposures.

This makes the shell feel less like an assistant-style interface and slows down repeated local skill management.

## Goal

Make the TUI's primary workflows discoverable and table-driven. Users should be able to open a slash command menu, choose commands with the keyboard, navigate both list and scan tables naturally, and start import/remove actions from the currently selected table row while preserving staged-plan safety.

## Non-goals

- Do not remove or change non-TUI CLI subcommands such as `skills-manager import` or `skills-manager remove`.
- Do not add mouse support.
- Do not implement remote Git URL import.
- Do not add rich config editing.
- Do not add a theme system or visual redesign beyond the table and command-menu behavior needed here.
- Do not bypass staged plan preview, confirmation, or physical-copy deletion warnings.

## User stories

1. As a TUI user, I want `/` to show available commands, so that I do not have to remember command names.
2. As a TUI user, I want each slash command to include a short description, so that I can understand what the command will do before running it.
3. As a TUI user, I want to move through slash command suggestions with up/down keys, so that command selection is keyboard-first.
4. As a TUI user, I want list-table selection to start at the first row and move down row by row, so that navigation behaves like a normal terminal table.
5. As a TUI user, I want the list viewport to scroll only after the selected row reaches the bottom of the visible table, so that I keep context while browsing.
6. As a TUI user, I want scan results to support the same up/down row navigation as list results, so that scan mode is not read-only.
7. As a TUI user, I want to import a selected scanned skill with a shortcut, so that I can act on the row I am looking at.
8. As a TUI user, I want to remove selected exposed skills or exposures from the list table with a shortcut, so that removal is tied to visible inventory state.
9. As a cautious user, I want every table shortcut that mutates files to show the same staged plan preview and confirmation as the existing flows, so that faster navigation does not reduce safety.

## Proposed experience

When the prompt is empty and the user presses `/`, the TUI opens a compact command suggestion menu above the prompt. Each row contains the command and a short description:

```text
/list    Show exposed skills and availability
/scan    Discover skills from configured sources
/config  Show current configuration
/help    Show commands and keybindings
/quit    Exit Skills Manager
```

The highlighted suggestion starts at the first command. Up/down moves the highlight. Enter runs the highlighted command. Esc closes the menu and returns to the previous mode. If the user types after `/`, the menu filters to matching commands. Import and remove should no longer appear as primary prompt commands in this menu because they move to table actions.

List mode starts with the first visible row selected. Up/down moves the selected row and the active-row background together. The table viewport remains at the top while the selection moves through visible rows. When the selection reaches the last visible row and the user presses down again, the viewport scrolls down by one row and the selection remains on the last visible row. Moving upward mirrors the behavior: the viewport scrolls up only after the selection reaches the first visible row.

Scan mode uses the same table navigation model. It starts at the first discovered skill when rows exist, supports up/down movement, and preserves a stable empty-state message when no rows are available.

Table actions replace explicit TUI import/remove commands:

- In scan mode, pressing `i` starts import for the selected discovered skill.
- In list mode, pressing `i` starts import for the selected skill when there are missing enabled-agent exposures to create.
- In list mode, pressing `x` starts removal for the selected exposed skill or exposure.
- If the selected row maps to multiple possible agents or exposures, the TUI shows a small keyboard picker before creating the staged plan.
- Enter opens or expands row details without mutating state.
- `r` refreshes the current table by rerunning the relevant list or scan data load.

After a table action creates a staged plan, the existing plan preview, confirmation, apply, rescan, and result rendering behavior remains in force. Physical-copy deletion warnings stay visually distinct and still require exact confirmation.

## Requirements

- Pressing `/` with an empty prompt opens a command suggestion menu.
- Command suggestions must show a command label and a short description.
- Up/down must move through command suggestions.
- Enter must execute the selected command suggestion.
- Esc must close the command suggestion menu without changing mode.
- Typing after `/` must filter command suggestions by command text.
- Prompt command suggestions must include list, scan, config, help, and quit.
- TUI import and remove must be removed from primary prompt suggestions and help as standalone workflows.
- If a user types `/import` or `/remove` directly, the TUI should guide them to use table shortcuts instead of starting the old standalone TUI flow.
- List and scan tables must use the same navigation behavior.
- Table selection must begin at the first row when rows exist.
- The selected-row background must stay attached to the selected row.
- The viewport must scroll only when the selection moves past the top or bottom visible row.
- Scan mode must support up/down row navigation.
- `i` in scan mode must start import from the selected discovered skill.
- `i` in list mode must start import for missing enabled-agent exposures when applicable.
- `x` in list mode must start removal for selected exposed state when applicable.
- Ambiguous selected rows must show a keyboard picker before staging changes.
- All mutating shortcuts must use the shared staged plan and apply behavior already used by CLI and V1 TUI flows.
- `r` must refresh the active table.
- Empty tables must not crash or leave stale selection state.

## Success criteria

- A user can discover available TUI commands by pressing `/`.
- A user can choose list or scan from the slash menu without typing the full command.
- List navigation behaves like a standard scrolling table from the first row to the last row.
- Scan results support the same row navigation as list results.
- A user can import from a selected scan row without typing an import command.
- A user can remove from a selected list row without typing a remove command.
- Mutating table shortcuts still show staged plan review and confirmation before filesystem changes.

## Edge cases

- Command suggestion filtering leaves no matches.
- The user presses table shortcuts while no row is selected.
- The selected list row already has all enabled-agent exposures.
- The selected list row has multiple exposures that could be removed.
- A scan result has duplicate display identity with another result.
- The table height changes after terminal resize.
- Refreshing the table removes the previously selected row.

## Implementation decisions

- Use one shared table navigation model for list and scan views. It should track selected row index and viewport offset separately so visual selection and scrolling are predictable.
- Represent command suggestions as command descriptors with a label, description, and action. This keeps prompt help, slash suggestions, and command execution aligned.
- Keep table actions as TUI interaction state only. Import/remove mutation policy continues to live in the shared staged plan and apply modules.
- Keep import/remove CLI commands intact. This PRD only changes how the full-screen TUI exposes those actions.

## Testing decisions

- Add unit tests for slash command menu state transitions: open, filter, move selection, execute, and escape.
- Add unit tests for shared table navigation: initial selection, down movement before scrolling, down movement after viewport bottom, up movement before scrolling, up movement after viewport top, empty rows, and resized viewport.
- Add TUI state tests proving scan mode handles up/down navigation.
- Add state tests proving scan `i`, list `i`, and list `x` route through staged plan creation rather than direct filesystem mutation.
- Add tests for typed `/import` and `/remove` guidance so removed TUI prompt commands do not silently regress.

## Progress notes

- 2026-06-03: Implemented slash command suggestions, shared list/scan table navigation, natural list scrolling, scan row selection, table-driven import/remove shortcuts, active-table refresh, and staged-plan safety tests.
- 2026-06-03: Verified with `cargo fmt --check` and `cargo test --locked`.

## Tasks

- [x] Add slash command suggestion state and rendering.
- [x] Replace standalone TUI import/remove help entries with table action guidance.
- [x] Introduce shared table navigation state for list and scan.
- [x] Fix list viewport scrolling behavior.
- [x] Add scan table navigation.
- [x] Add import shortcut from scan/list table context.
- [x] Add remove shortcut from list table context.
- [x] Preserve staged plan preview and confirmation for all mutating table actions.
- [x] Add focused unit tests for command suggestions, table navigation, scan navigation, and table action routing.
