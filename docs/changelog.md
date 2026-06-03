# Changelog

## 2026-06-03

| Time | Change | Docs |
|---|---|---|
| 2026-06-03 23:04 | Implemented PRD-007 v2 TUI table actions with slash suggestions, shared table navigation, scan selection, import/remove shortcuts, refresh, and staged-plan safety tests. | `README.md`, `features/assistant-style-tui-shell.md`, `prds/v1/prd-007-v2-assistant-style-tui-table-actions.md`, `roadmap.md` |
| 2026-06-03 22:18 | Created the PRD-007 v2 follow-up for slash command suggestions, natural TUI table scrolling, scan-table navigation, and table-driven import/remove shortcuts. | `prds/v1/prd-007-v2-assistant-style-tui-table-actions.md`, `roadmap.md` |
| 2026-06-03 21:46 | Documented the assistant-style TUI shell, updated README default-run guidance, and aligned TUI duplicate/context and global help/quit key behavior. | `AGENTS.md`, `README.md`, `features/assistant-style-tui-shell.md`, `prds/v1/prd-007-assistant-style-tui-shell.md`, `roadmap.md` |
| 2026-06-03 15:15 | Documented the human-readable CLI workflow, updated README command guidance, and required exact `yes` confirmation before physical-copy removal. | `README.md`, `features/human-readable-cli-workflow.md`, `prds/v1/prd-005-human-readable-cli-workflow.md`, `roadmap.md` |
| 2026-06-03 14:48 | Documented inventory and exposure resolution, updated `list` README guidance, and added duplicate-context/shared `.agents` verification. | `README.md`, `features/inventory-and-exposure-resolution.md`, `prds/v1/prd-004-inventory-and-exposure-resolution.md`, `roadmap.md` |
| 2026-06-03 14:27 | Added read-only skill source scanning output with skill paths, moved PRDs under `docs/prds/`, and documented the scan feature. | `README.md`, `features/skill-source-scanning.md`, `prds/v1/prd-003-skill-source-scanning.md`, `roadmap.md` |

## 2026-06-01

| Time | Change | Docs |
|---|---|---|
| 2026-06-01 00:19 | Implemented the human-readable CLI workflow for import, remove, and doctor, plus shared command helpers and CLI coverage for ambiguous and missing skills. | `prds/v1/prd-005-human-readable-cli-workflow.md`, `roadmap.md` |

## 2026-05-31

| Time | Change | Docs |
|---|---|---|
| 2026-05-31 09:30 | Implemented inventory building and `list`, including exposure resolution for symlinks and physical copies, duplicate disambiguation, and CLI coverage for missing config and empty targets. | `features/inventory-and-exposure-resolution.md`, `prds/v1/prd-004-inventory-and-exposure-resolution.md` |
| 2026-05-31 08:38 | Added the Rust CLI/TUI foundation crate, placeholder command routing, and the initial verification test suite. | `README.md`, `features/v1/prd-001-rust-project-foundation.md` |

## 2026-05-30

| Time | Change | Docs |
|---|---|---|
| 2026-05-30 22:21 | Flattened the V1 PRDs into a single version folder and updated roadmap links to the new flat paths. | `prds/v1/prd-001-rust-project-foundation.md`, `prds/v1/prd-002-configuration-and-agent-definitions.md`, `prds/v1/prd-003-skill-source-scanning.md`, `prds/v1/prd-004-inventory-and-exposure-resolution.md`, `prds/v1/prd-005-human-readable-cli-workflow.md`, `prds/v1/prd-006-safe-import-and-removal-plans.md`, `prds/v1/prd-007-assistant-style-tui-shell.md`, `roadmap.md` |
| 2026-05-30 22:05 | Created ADR-001 and ADR-002 from the engineering guidelines, enriched PRDs 001/002/006/007 with the missing technical, safety, and UI details, and added ADR cross-reference links back into the guidelines. | `adr/adr-001-technology-stack.md`, `adr/adr-002-dependency-policy.md`, `adr/README.md`, `prds/v1/prd-001-rust-project-foundation.md`, `prds/v1/prd-002-configuration-and-agent-definitions.md`, `prds/v1/prd-006-safe-import-and-removal-plans.md`, `prds/v1/prd-007-assistant-style-tui-shell.md`, `skills_manager_rust_guidelines.md` |
