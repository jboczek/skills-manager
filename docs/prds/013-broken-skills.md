# PRD 013 — Broken skills

Status: Complete (2026-10-31).

## Problem and outcome
Moved source directories leave dangling agent symlinks. Inventory must expose the problem and let users review a repair instead of silently skipping import.

## Approved behavior
- Global dangling symlinks are `broken`; they do not grant agent availability. Project-local exposures are excluded from detection, warnings, filtering and repair.
- Tab includes `Only broken`. A repository containing broken rows has a red `!`; each broken row is red and its connection is `broken`.
- F acts on every broken link in the selected row. Match the skill directory name within the original repository. One match proposes replacement; multiple matches require selection; no match proposes symlink removal only.
- Review shows the skill, agent, link path, old source and replacement (or removal-only notice). Only confirmation mutates links. Cancellation preserves them; source directories are never deleted.
- Refresh after changes and report failures. Refuse stale plans when the link or replacement changed.
- Import encountering broken links points users to F.

## Acceptance and evidence
Regression coverage includes inventory provenance/availability, filter cycle, all-agent repair, ambiguous source selection, no-match removal, cancellation, stale plans, shared-link deduplication, repository matching and project-local exclusion. Ratatui render tests check red warnings/rows at widths 120 and 180.

`cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are the completion checks. Browser screenshots could not run: required Docker context `colima-work-agents` is absent. See local `docs/temp/prd-broken-skills-plan.md` for execution evidence and continuation state.

Moved global skills appear once in TUI inventory: match by repository path and skill directory name, combine existing global exposures, and suppress the separate discovery row. Show the unique current source with `broken (folder changed)`, or `broken (removed)` when no replacement is found. Keep the row red while any link is broken. If multiple replacements exist, retain the old Source until the user chooses in F; never guess. Original link targets remain available in the repair preview.
