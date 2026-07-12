# Changelog

## 2026-07-12

| Time | Change | Docs |
|---|---|---|
| 2026-07-12 20:44 | Completed PRD-007 v4: unified the interactive list and discovery views, added filter cycling and safe discovery imports, and removed interactive `/scan`. | `README.md`, `features/assistant-style-tui-shell.md`, `features/inventory-and-exposure-resolution.md`, `prds/v1/done/prd-007-v4-unified-list-view.md`, `roadmap.md` |

## 2026-06-21

| Time | Change | Docs |
|---|---|---|
| 2026-06-21 20:10 | Removed stale warnings for ignored legacy `project_dir` values while keeping legacy configs parseable and normalized output free of those fields. | `README.md`, `features/human-readable-cli-workflow.md`, `features/inventory-and-exposure-resolution.md`, `features/assistant-style-tui-shell.md`, `prds/v2/prd-008-global-execution-context.md`, `roadmap.md` |

## 2026-06-13

| Time | Change | Docs |
|---|---|---|
| 2026-06-13 00:06 | Implemented PRD-009 managed Git source import with preview-before-mutation, temporary clone scanning and cleanup, strict same-origin reuse, CLI/TUI skill selection, and staged exposure confirmation. | `README.md`, `features/git-source-import.md`, `prds/v2/prd-009-git-url-import-into-managed-source-directory.md`, `roadmap.md` |

## 2026-06-12

| Time | Change | Docs |
|---|---|---|
| 2026-06-12 23:38 | Split global TUI inventory rows into separate source-repository groups while preserving project grouping and per-row scope. | `README.md`, `features/assistant-style-tui-shell.md`, `prds/v2/prd-008-global-execution-context.md`, `roadmap.md` |
| 2026-06-12 23:28 | Corrected PRD-008 inventory semantics: list now shows only actual global and read-only project-local exposures, uses fixed project conventions, preserves source paths and project context, and blocks local mutation. | `README.md`, `features/skill-source-scanning.md`, `features/inventory-and-exposure-resolution.md`, `features/human-readable-cli-workflow.md`, `features/assistant-style-tui-shell.md`, `prds/v2/prd-008-global-execution-context.md`, `roadmap.md` |
| 2026-06-12 15:17 | Implemented PRD-008 global execution context with absolute path validation, ignored legacy project targets, CWD-invariant inventory and plans, shared CLI/TUI diagnostics, and no TUI directory or branch context. | `README.md`, `features/skill-source-scanning.md`, `features/inventory-and-exposure-resolution.md`, `features/human-readable-cli-workflow.md`, `features/assistant-style-tui-shell.md`, `prds/v2/prd-008-global-execution-context.md`, `roadmap.md` |
| 2026-06-12 13:40 | Added the complete V2 PRD set for global execution, managed Git source import, and conservative manual-install migration, with roadmap links and implementation order. | `README.md`, `roadmap.md`, `prds/v2/prd-008-global-execution-context.md`, `prds/v2/prd-009-git-url-import-into-managed-source-directory.md`, `prds/v2/prd-010-manual-install-migration.md` |
| 2026-06-12 12:24 | Reframed V2 around globally launched, current-directory-independent operation, removed arbitrary project targeting, moved completed TUI follow-ups into V1, and aligned completed V1 statuses. | `AGENTS.md`, `bigpicture.md`, `roadmap.md`, `prds/v1/prd-002-configuration-and-agent-definitions.md`, `prds/v1/prd-006-safe-import-and-removal-plans.md`, `prds/v1/prd-007-v2-assistant-style-tui-table-actions.md`, `prds/v1/prd-007-v3-source-grouped-tui-tables.md` |

## 2026-06-11

| Time | Change | Docs |
|---|---|---|
| 2026-06-11 22:59 | Included global and project-local `.agents/skills` entries in list inventory with legacy config migration and Codex/Copilot mapping. | `README.md`, `features/inventory-and-exposure-resolution.md`, `prds/v1/prd-007-v3-source-grouped-tui-tables.md`, `roadmap.md` |
| 2026-06-11 22:04 | Widened the PRD-007 v3 list and scan skill-name columns by approximately 25%. | `README.md`, `features/assistant-style-tui-shell.md`, `prds/v1/prd-007-v3-source-grouped-tui-tables.md`, `roadmap.md` |
| 2026-06-11 00:23 | Implemented PRD-007 v3 source-grouped TUI tables with privacy-safe labels, collapsed navigation, refresh preservation, and skill-only actions. | `README.md`, `features/assistant-style-tui-shell.md`, `prds/v1/prd-007-v3-source-grouped-tui-tables.md`, `roadmap.md` |

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
