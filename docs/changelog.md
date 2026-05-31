# Changelog

## 2026-05-31

| Time | Change | Docs |
|---|---|---|
| 2026-05-31 09:30 | Implemented inventory building and `list`, including exposure resolution for symlinks and physical copies, duplicate disambiguation, and CLI coverage for missing config and empty targets. | `docs/features/v1/prd-004-inventory-and-exposure-resolution.md` |
| 2026-05-31 08:38 | Added the Rust CLI/TUI foundation crate, placeholder command routing, and the initial verification test suite. | `README.md`, `features/v1/prd-001-rust-project-foundation.md` |

## 2026-05-30

| Time | Change | Docs |
|---|---|---|
| 2026-05-30 22:21 | Flattened the V1 PRDs into a single version folder and updated roadmap links to the new flat paths. | `features/v1/prd-001-rust-project-foundation.md`, `features/v1/prd-002-configuration-and-agent-definitions.md`, `features/v1/prd-003-skill-source-scanning.md`, `features/v1/prd-004-inventory-and-exposure-resolution.md`, `features/v1/prd-005-human-readable-cli-workflow.md`, `features/v1/prd-006-safe-import-and-removal-plans.md`, `features/v1/prd-007-assistant-style-tui-shell.md`, `roadmap.md` |
| 2026-05-30 22:05 | Created ADR-001 and ADR-002 from the engineering guidelines, enriched PRDs 001/002/006/007 with the missing technical, safety, and UI details, and added ADR cross-reference links back into the guidelines. | `adr/adr-001-technology-stack.md`, `adr/adr-002-dependency-policy.md`, `adr/README.md`, `features/v1/prd-001-rust-project-foundation.md`, `features/v1/prd-002-configuration-and-agent-definitions.md`, `features/v1/prd-006-safe-import-and-removal-plans.md`, `features/v1/prd-007-assistant-style-tui-shell.md`, `skills_manager_rust_guidelines.md` |
