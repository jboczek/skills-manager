---
title: Unified TUI list view
summary: Replace the separate TUI scan view with one filterable list that shows both exposed and discovered skills.
status: done
---

# Unified TUI list view

## tldr;

Make `/list` the sole TUI command for browsing skills. Its default **Full** view combines the existing exposure inventory with discovered skills that have not been exposed; Tab cycles through **Full**, **Only exposed**, and **Only discovered not applied**. The existing list columns remain unchanged. The direct `skills-manager list` and `skills-manager scan` CLI commands remain separate.

## Context

The full-screen TUI currently has two nearly identical grouped tables:

- `/list` renders real global and project-local exposures in seven columns and provides import/remove actions.
- `/scan` renders discovered source skills in four different columns and provides import only.

Both views are populated from the same configured source catalog, use the same collapsed source groups, and have the same arrow-key navigation. Moving between them makes users choose between two representations of one skill ecosystem.

## Problem

Users cannot see source discovery and agent exposure together. They must switch commands to answer whether a discovered skill is already exposed, while learning two table layouts and separate command entry points.

## Scope

| Area | Change |
|---|---|
| TUI command | Remove `/scan` from interactive command parsing, suggestions, help, and footer guidance. `/list` is the single browsing command. |
| Default view | `/list` refreshes the scan catalog and exposure inventory and opens **Full**. Full shows every existing inventory row plus every discovered source skill with no real agent exposure. |
| Tab views | With an empty prompt and no command menu open, Tab cycles `Full → Only exposed → Only discovered not applied → Full`. |
| Row meaning | **Only exposed** is the current inventory unchanged. **Only discovered not applied** contains source skills whose canonical source identity has no real inventory exposure. Full is their union. Existing global/project-local exposure rows stay separate; no availability or scope is inferred from a scan result. |
| Columns | Keep exactly `SKILL`, `SOURCE`, `CLAUDE`, `CODEX`, `COPILOT`, `SCOPE`, and `CONNECTION`, including their current sizing and source-group presentation. A discovery-only row shows `-` for each agent and scope, and `not exposed` as its connection. |
| Actions | `i` imports a discovery-only row using the former `/scan` flow. Existing exposed-row import behavior remains. `x` and Space remain available only for applicable exposure rows; discovery-only rows cannot be removed or selected for batch exposure changes. Project-local rows remain read-only. |
| Refresh and navigation | `r` reloads both data sets while keeping the current filter. Source grouping, privacy-safe paths, expansion retention, selection safety, and arrow navigation continue to apply to the filtered rows. |

## Non-goals

- Do not change the standalone `skills-manager list` or `skills-manager scan` CLI commands or their text output.
- Do not change scanning, inventory building, exposure resolution, source matching, import plans, removal plans, or project-local read-only rules.
- Do not add free-text filtering, search, sorting controls, new columns, mouse controls, bulk import of a source group, or persistent view preferences.
- Do not merge distinct global or project-local inventory rows merely because they resolve to one source skill.

## Acceptance criteria

- [x] Entering `/list` loads one coherent source catalog and inventory, opens **Full**, and shows exposed and discovery-only skills in one grouped seven-column table.
- [x] Full includes every existing inventory row and each discovered source skill that has no real agent exposure; a discovered source already exposed globally or project-locally is not repeated as discovery-only.
- [x] **Only exposed** reproduces the existing list rows, headers, grouping, paths, availability markers, scope, and connection values.
- [x] **Only discovered not applied** shows only unexposed scan results, with `-` agent/scope values and `not exposed` in `CONNECTION`.
- [x] With the prompt empty and command suggestions closed, Tab cycles the three views in the documented order. Tab still completes the selected command whenever command suggestions are open.
- [x] `/scan` is no longer offered or accepted by the TUI; user guidance for imports points to `/list`.
- [x] `i` opens the existing safe import flow for an unexposed discovered row. `x` and Space do not stage changes for an unexposed discovered row; existing exposed and project-local action restrictions remain intact.
- [x] `r` preserves the active filter and safely restores selection and expanded groups when their rows still exist.
- [x] `skills-manager list` and `skills-manager scan` remain available as separate non-interactive CLI commands with their current output contracts.

## Implementation notes

- Start with TDD: add the smallest focused tests for the unified row projection and filter cycle before changing production code.
- Model discovered source state and agent exposure state separately. Match a scan result to exposure state by canonical source identity; do not infer exposure scope or agent availability from its source path.
- Build a typed TUI presentation row for inventory and discovery-only items rather than overloading an inventory row. This keeps action eligibility explicit and prevents a discovery item from being passed to remove or batch-selection code.
- Generate the inventory from the same fresh source catalog used to build the unified list so filter membership cannot disagree with the displayed scan data.
- Keep the current `SourceTable` grouping and privacy-safe labels. The filtered row set, not a new grouping model, drives each Tab state.
- Preserve Tab completion precedence while the command menu is open or the prompt contains text.
- Keep commits small and split implementation into slices that modify no more than three files when practical. Preserve behavior outside this TUI scope and prefer direct edits over new abstractions or dependencies.

## Progress notes

- 2026-07-12: Added a typed unified TUI row projection with canonical-source de-duplication and focused Full, exposed-only, discovery-only, and unmatched-exposure tests.
- 2026-07-12: Replaced interactive `/scan` with the filterable `/list` view while preserving the separate non-interactive CLI commands.
- 2026-07-12: Verified renderer headers and discovery-only values, action restrictions, filter/refresh behavior, selection and expansion retention, `rtk cargo test --locked`, and `cargo fmt --check`.

## Tasks

- [x] Add projection tests for Full, Only exposed, Only discovered not applied, canonical-source de-duplication, and exposure rows that have no matching scan result.
- [x] Implement the typed unified list projection and have `/list` refresh scan and inventory state from one source catalog.
- [x] Add renderer tests for all seven existing headers and discovery-only row values; remove the separate scan-table renderer only after those tests pass.
- [x] Add event tests for the empty-prompt Tab cycle, command-menu Tab completion precedence, refresh retention, and action eligibility by row type.
- [x] Remove the interactive `/scan` mode, pending load, suggestion, help/footer references, and obsolete TUI-only tests while retaining scanner state for import flows.
- [x] Run focused TUI tests, `rtk cargo test --locked`, and `cargo fmt --check`.
- [x] Update the README, inventory feature documentation, PRD progress notes, roadmap status, and changelog after implementation; keep the local `docs/temp/prd-007-v4-unified-list-plan.md` out of commits.
