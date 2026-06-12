# Roadmap: Skills Manager

## Summary

This roadmap turns the Skills Manager product direction into sequenced build slices. The product should become a terminal-first skill exposure manager: one local interface for seeing which skills exist, where they came from, which agents can use them, and what filesystem changes will happen before an import or removal is applied.

V1 proves the core local workflow. V2 removes current-directory-dependent behavior and adds managed remote imports and migration before any context-budget or sharing work.

V1 PRDs live under `docs/prds/v1/`, V2 PRDs live under `docs/prds/v2/`, and both are linked from their roadmap items below.

## Source Context

- `docs/bigpicture.md`
- `docs/interview.md`
- `docs/skills_manager_rust_guidelines.md`
- Current repository state: Rust CLI/TUI crate with configuration, source scanning, inventory/list, safe import/remove plans, human-readable CLI workflow, and source-grouped assistant-style TUI tables implemented.

## Roadmap Principles

- Deliver a manually useful local workflow before adding remote source acquisition or migration.
- Make filesystem state and pending changes visible before every mutation.
- Keep the product model focused on effective agent availability, not internal folder mechanics.
- Treat Skills Manager as a globally launched application whose behavior comes from configuration, not the current working directory.
- Build the crate and domain model simply enough for V1 without prematurely splitting a Rust workspace.
- Prefer independently valuable slices that can each become a focused PRD or implementation plan.

## Target Outcome

Skills Manager should let a power user launch one global terminal UI, inspect their local skill inventory across Codex, Claude, and Copilot, import selected skills from discovered source repositories, detach exposures safely, and trust that the tool will not hide or guess destructive filesystem changes.

## V1 - Manual but useful exposure workflow

### Version Goal

Create the first usable Rust CLI/TUI that can discover skills, show effective availability across configured agents, and safely apply basic import/remove exposure changes.

**Status:** Complete.

### Roadmap Items

### 001 - Rust project foundation

- **Type:** Technical enabler
- **Outcome:** The project can be built, tested, linted, and evolved as a native Rust CLI/TUI application.
- **Description:** Create the initial single-crate Rust project, fixed dependency set, basic module layout, lockfile, and minimal command routing for local development.
- **Why now:** Every later roadmap item depends on having a working crate and agreed module boundaries.
- **PRD candidate:** Yes. PRD: [`docs/prds/v1/prd-001-rust-project-foundation.md`](prds/v1/prd-001-rust-project-foundation.md)
- **Dependencies:** None known.
- **Validation signal:** `cargo check --locked`, `cargo test --locked`, formatting, and clippy can run successfully on the empty foundation locally.
- **Status:** Done.
- **Notes / risks:** Dependency policy needs care because the guidelines require maintained crates but avoid releases newer than 14 days.

### 002 - Configuration and agent definitions

- **Type:** Capability
- **Outcome:** Users can initialize and inspect the local configuration that defines source roots and agent target paths.
- **Description:** Add config loading, opinionated default config generation, `config init`, `config show`, `config path`, and config-driven agent definitions for Codex, Claude, and Copilot. Shared target folders such as `.agents` stay hidden inside configuration.
- **Why now:** Scanning and exposure logic need explicit source and target paths before they can be safe.
- **PRD candidate:** Yes. PRD: [`docs/prds/v1/prd-002-configuration-and-agent-definitions.md`](prds/v1/prd-002-configuration-and-agent-definitions.md)
- **Dependencies:** 001.
- **Validation signal:** A fresh machine can generate a readable TOML config with usable defaults and inspect the resolved paths without mutating skill directories.
- **Status:** Done.
- **Notes / risks:** The UI must treat `.agents` as a technical target, not as the primary product-facing concept.

### 003 - Skill source scanning

- **Type:** Capability
- **Outcome:** Users can discover available skills in configured source locations without changing the filesystem.
- **Description:** Scan `central_dir` and configured parent directories for `SKILL.md`, preserve nested hierarchy, resolve repository roots, and derive origin from `git remote`.
- **Why now:** Import selection only works if the app can reliably find candidate skills first.
- **PRD candidate:** Yes. PRD: [`docs/prds/v1/prd-003-skill-source-scanning.md`](prds/v1/prd-003-skill-source-scanning.md)
- **Dependencies:** 001, 002.
- **Validation signal:** `skills-manager scan` finds single-skill, multi-skill, and nested-skill repositories in temp filesystem tests.
- **Status:** Done.
- **Notes / risks:** Recursive scanning must be bounded and must not follow symlinks.

### 004 - Inventory and exposure resolution

- **Type:** Capability
- **Outcome:** Users can see what skills are currently exposed to each configured agent and how each exposure is connected.
- **Description:** Build current inventory from config, source repositories, agent target directories, symlinks, physical copies, and current project-local paths.
- **Why now:** The core product value is visibility before mutation.
- **PRD candidate:** Yes. PRD: [`docs/prds/v1/prd-004-inventory-and-exposure-resolution.md`](prds/v1/prd-004-inventory-and-exposure-resolution.md)
- **Dependencies:** 002, 003.
- **Validation signal:** `skills-manager list` shows consolidated rows with skill, source, agent availability, scope, and connection type.
- **Status:** Done.
- **Notes / risks:** Duplicate names and shared target paths can confuse users unless namespace, numbered disambiguation, and effective availability rules are explicit.

### 005 - Human-readable CLI workflow

- **Type:** Feature
- **Outcome:** Users can list, scan, import, remove, and check setup from scriptable commands while the TUI is still maturing.
- **Description:** Add `list`, `scan`, and `doctor` command flows with human-readable tables, plus CLI entry points for import/remove that reuse the safe planning module before any mutation.
- **Why now:** CLI flows are easier to test deeply and provide useful behavior before the full-screen UI is complete.
- **PRD candidate:** Yes. PRD: [`docs/prds/v1/prd-005-human-readable-cli-workflow.md`](prds/v1/prd-005-human-readable-cli-workflow.md)
- **Dependencies:** 002, 003, 004, 006 for mutating import/remove behavior.
- **Validation signal:** CLI snapshot tests cover list, scan, and config output; import/remove tests apply changes in temporary directories.
- **Status:** Done.
- **Notes / risks:** CLI must stay secondary and should not force premature JSON or automation contracts.

### 006 - Safe import and removal plans

- **Type:** Capability
- **Outcome:** Users can preview and confirm exactly which symlinks or local copies will change before applying imports or removals.
- **Description:** Introduce staged change plans for exposing skills, detaching symlinks, and deleting physical copies with explicit confirmation rules. This module is implemented before mutating CLI or TUI flows call it.
- **Why now:** Trust in mutation safety is the central V1 adoption requirement.
- **PRD candidate:** Yes. PRD: [`docs/prds/v1/prd-006-safe-import-and-removal-plans.md`](prds/v1/prd-006-safe-import-and-removal-plans.md)
- **Dependencies:** 004.
- **Validation signal:** Tests prove symlink removal never deletes source content, physical copies require explicit confirmation, and inventory is rescanned after mutation.
- **Status:** Done.
- **Notes / risks:** The strongest product failure mode is accidental deletion or a hidden fallback from symlink to copy.

### 007 - Assistant-style TUI shell

- **Type:** UX
- **Outcome:** Running `skills-manager` opens a full-screen terminal experience with a prompt, status feed, command hints, and complete V1 modes.
- **Description:** Implement the TUI layout, home mode, prompt command parsing, help, status feed, list mode, scan/config modes, and complete guided import/remove flows backed by the safe planning module.
- **Why now:** The intended product is terminal-first and assistant-style; V1 is incomplete if the default experience is only direct CLI commands.
- **PRD candidate:** Yes. PRD: [`docs/prds/v1/prd-007-assistant-style-tui-shell.md`](prds/v1/prd-007-assistant-style-tui-shell.md)
- **Dependencies:** 004, 005, 006.
- **Validation signal:** A user can launch the app, inspect status, run list/scan/config flows, complete import/remove with staged plan review and confirmation, and quit cleanly from the TUI.
- **Status:** Done.
- **Notes / risks:** Table navigation and row actions were completed by follow-up item 011.

### 011 - Assistant-style TUI table actions

- **Type:** UX
- **Outcome:** Users can discover TUI commands, navigate list and scan tables, and start import/remove actions from selected rows.
- **Description:** Add slash command suggestions, natural list scrolling, scan-table navigation, table-driven import/remove shortcuts, active table refresh, and staged change review.
- **Why now:** This follow-up completed the high-frequency interaction model before V1 was declared finished.
- **PRD candidate:** Yes. PRD: [`docs/prds/v1/prd-007-v2-assistant-style-tui-table-actions.md`](prds/v1/prd-007-v2-assistant-style-tui-table-actions.md)
- **Dependencies:** 006, 007.
- **Validation signal:** A user can open slash suggestions, navigate list and scan rows, refresh active tables, and start import/remove from selected rows while still reviewing staged plans before applying changes.
- **Status:** Done.
- **Notes / risks:** Fast actions retain the same staged-plan safety guarantees as CLI and guided TUI flows.

### 007 v3 - Source-grouped TUI tables

- **Type:** UX
- **Outcome:** Users can browse large list and scan results as compact, privacy-safe source groups instead of flat skill tables.
- **Description:** Group list and scan rows by source, collapse every group by default, add Left/Right tree navigation, widen the first column for skill names, show repository-relative or bounded path context without exposing user-specific absolute paths, and include global and project-local `.agents/skills` inventory targets.
- **Why now:** Table actions made individual rows usable, but large repositories still dominate the initial view and duplicate or unknown source labels remain hard to distinguish.
- **PRD candidate:** Yes. PRD: [`docs/prds/v1/prd-007-v3-source-grouped-tui-tables.md`](prds/v1/prd-007-v3-source-grouped-tui-tables.md)
- **Dependencies:** 004, 007, 011.
- **Validation signal:** List and scan open as collapsed source overviews, expand and collapse consistently with arrow keys, distinguish duplicate sources with safe path context, retain skill-level actions, and show `~/.agents/skills` entries as global Codex/Copilot availability.
- **Status:** Done.
- **Notes / risks:** Group identity and fallback path shortening must remain stable without leaking home-directory prefixes. Legacy `.agents` config values must resolve to the standard `.agents/skills` layout without manual migration.

### Version Validation

V1 works locally if a user can initialize default source and target paths, scan discovered skills, view consolidated availability across Codex, Claude, and Copilot, import selected skills through explicit plans, remove exposures safely, and use the default TUI without needing to understand internal directory topology.

### Key Risks / Assumptions

- Agent directory conventions are stable enough to start with configurable defaults.
- The symlink-first model is acceptable for the target users.
- A single-crate architecture can carry V1 without becoming hard to reason about.
- Users will tolerate editing generated defaults in V1 if config display and diagnostics are clear.

## V2 - Global managed skill library

### Version Goal

Make Skills Manager independent of the launch directory and reduce manual source setup while keeping the same inspectable source-plus-exposure model.

### Roadmap Items

### 008 - Global execution context

- **Type:** Capability
- **Outcome:** Skills Manager behaves the same regardless of the directory from which it is launched.
- **Description:** Remove the current working directory as implicit product context, resolve global mutation targets from explicit configuration, and report read-only project-local exposures inside configured source repositories.
- **Why now:** The application is intended to be installed and launched globally, so shell location must not silently change inventory or mutation behavior.
- **PRD candidate:** Yes. PRD: [`docs/prds/v2/prd-008-global-execution-context.md`](prds/v2/prd-008-global-execution-context.md)
- **Dependencies:** 004, 006, 007.
- **Validation signal:** Launching the same configuration from unrelated directories produces the same scan catalog, global plans, and global plus project-local exposure inventory.
- **Status:** Done.
- **Notes / risks:** Active managed paths are absolute after leading-tilde expansion. Legacy project target config parses for compatibility but is ignored with diagnostics. List groups are repository-based: global rows use source repositories and project-local rows use containing projects. Project-local rows stay read-only, and no `--project` selector is added.

### 009 - Git URL import into managed source directory

- **Type:** Integration
- **Outcome:** Users can add a new skill source without manually cloning it first.
- **Description:** Clone a provided Git URL into the managed source directory, scan it, and let the user select specific skills to expose.
- **Why now:** V1 intentionally avoids remote code acquisition; V2 can add it after local import semantics are trusted.
- **PRD candidate:** Yes. PRD: [`docs/prds/v2/prd-009-git-url-import-into-managed-source-directory.md`](prds/v2/prd-009-git-url-import-into-managed-source-directory.md)
- **Dependencies:** 003, 006, 008.
- **Validation signal:** A user can import from a test Git repository and still preview exposures before applying them.
- **Status:** Planned.
- **Notes / risks:** Remote import must not auto-execute code, recurse into submodules, overwrite a conflicting source, or silently update an existing checkout.

### 010 - Manual install migration

- **Type:** Capability
- **Outcome:** Users can convert older copied installs into the central source-plus-symlink model.
- **Description:** Detect physical copies, propose a migration plan, preserve source content, and replace target copies with links only after confirmation.
- **Why now:** Migration is useful only after V1 can accurately classify existing exposures.
- **PRD candidate:** Yes. PRD: [`docs/prds/v2/prd-010-manual-install-migration.md`](prds/v2/prd-010-manual-install-migration.md)
- **Dependencies:** 004, 006, 008.
- **Validation signal:** A dry migration preview explains exactly what will be moved, linked, or left untouched.
- **Status:** Planned.
- **Notes / risks:** Migration is destructive-adjacent, so ambiguous matches, destination collisions, and unknown layouts remain untouched by default.

### Version Validation

V2 works if users can launch Skills Manager from anywhere, import remote sources without manual cloning, and migrate older installs without losing confidence in what will change.

### Key Risks / Assumptions

- Removing current-directory context does not hide any still-useful global inventory behavior.
- Remote import does not turn the product into a full package manager too early.
- Migration plans can be made understandable enough to trust.

## V3 - Local skill control plane

### Version Goal

Turn Skills Manager from a local exposure utility into a durable control plane for curating, auditing, and sharing agent skill setups.

### Roadmap Items

### 012 - Context budget visibility

- **Type:** Research
- **Outcome:** Users can see a practical estimate of how much context enabled skills consume for each agent.
- **Description:** Estimate context impact from skill descriptions or relevant metadata and show it per configured agent.
- **Why now:** This depends on trusted inventory and real user need; it should not distract from V1/V2 safety and workflow foundations.
- **PRD candidate:** Later. Suggested file: `docs/prds/012-context-budget-visibility.md`
- **Dependencies:** 004, 007.
- **Validation signal:** Users can identify obviously overexposed or overly verbose skill setups and make better enablement decisions.
- **Notes / risks:** Estimates may be approximate because each agent may inject skill context differently.

### 013 - Shareable setup profiles

- **Type:** Capability
- **Outcome:** Users or teams can reproduce a curated skill setup across machines or projects.
- **Description:** Define exportable profiles describing sources, agent targets, and intended exposures without bundling private local paths blindly.
- **Why now:** Sharing only makes sense after the local model is stable and trusted.
- **PRD candidate:** Later. Suggested file: `docs/prds/013-shareable-setup-profiles.md`
- **Dependencies:** 008, 009, 010.
- **Validation signal:** A profile can recreate intended exposures on another machine after path review and confirmation.
- **Notes / risks:** Profiles may expose sensitive repository names or local paths if not designed carefully.

### 014 - Repository health and update awareness

- **Type:** Capability
- **Outcome:** Users can understand whether managed skill sources are stale, dirty, or disconnected from their upstreams.
- **Description:** Add optional repository status visibility such as current branch, dirty state, and available upstream changes.
- **Why now:** V1 explicitly avoids package-manager behavior; this becomes useful only after users rely on central source repositories.
- **PRD candidate:** Later. Suggested file: `docs/prds/014-repository-health-and-update-awareness.md`
- **Dependencies:** 009.
- **Validation signal:** Users can identify stale or locally modified skill sources before sharing or migrating setups.
- **Notes / risks:** This can pull the product toward full version management; keep it informational until demand is proven.

### Version Validation

V3 works if Skills Manager becomes the trusted local place to curate, audit, and reproduce agent skill environments rather than just a safer symlink helper.

### Key Risks / Assumptions

- Advanced visibility is only valuable if users already manage enough skills to need it.
- Team or cross-machine sharing may require security and privacy decisions that are not relevant to V1.
- Repository health can become a product distraction if it turns into a package manager.

## Later / Parking Lot

- Stable machine-readable JSON output for automation use cases.
- Support for additional agents beyond Codex, Claude, and Copilot.
- CI, release automation, Homebrew tap, WinGet, and installer scripts after the local V1 workflow is proven.
- Rich config editing inside the TUI.
- Workspace split into multiple crates if the single crate becomes a maintenance problem.

## PRD Candidates

| ID | Roadmap Item | Version | Suggested PRD File | Priority | Status | Dependencies |
|---|---|---|---|---|---|---|
| 001 | Rust project foundation | V1 | [`docs/prds/v1/prd-001-rust-project-foundation.md`](prds/v1/prd-001-rust-project-foundation.md) | High | Done | None |
| 002 | Configuration and agent definitions | V1 | [`docs/prds/v1/prd-002-configuration-and-agent-definitions.md`](prds/v1/prd-002-configuration-and-agent-definitions.md) | High | Done | 001 |
| 003 | Skill source scanning | V1 | [`docs/prds/v1/prd-003-skill-source-scanning.md`](prds/v1/prd-003-skill-source-scanning.md) | High | Done | 001, 002 |
| 004 | Inventory and exposure resolution | V1 | [`docs/prds/v1/prd-004-inventory-and-exposure-resolution.md`](prds/v1/prd-004-inventory-and-exposure-resolution.md) | High | Done | 002, 003 |
| 005 | Human-readable CLI workflow | V1 | [`docs/prds/v1/prd-005-human-readable-cli-workflow.md`](prds/v1/prd-005-human-readable-cli-workflow.md) | Medium | Done | 002, 003, 004, 006 for mutations |
| 006 | Safe import and removal plans | V1 | [`docs/prds/v1/prd-006-safe-import-and-removal-plans.md`](prds/v1/prd-006-safe-import-and-removal-plans.md) | High | Done | 004 |
| 007 | Assistant-style TUI shell | V1 | [`docs/prds/v1/prd-007-assistant-style-tui-shell.md`](prds/v1/prd-007-assistant-style-tui-shell.md) | High | Done | 004, 005, 006 |
| 008 | Global execution context | V2 | [`docs/prds/v2/prd-008-global-execution-context.md`](prds/v2/prd-008-global-execution-context.md) | High | Planned | 004, 006, 007 |
| 009 | Git URL import into managed source directory | V2 | [`docs/prds/v2/prd-009-git-url-import-into-managed-source-directory.md`](prds/v2/prd-009-git-url-import-into-managed-source-directory.md) | Medium | Planned | 003, 006, 008 |
| 010 | Manual install migration | V2 | [`docs/prds/v2/prd-010-manual-install-migration.md`](prds/v2/prd-010-manual-install-migration.md) | Medium | Planned | 004, 006, 008 |
| 011 | Assistant-style TUI table actions | V1 | [`docs/prds/v1/prd-007-v2-assistant-style-tui-table-actions.md`](prds/v1/prd-007-v2-assistant-style-tui-table-actions.md) | Medium | Done | 006, 007 |
| 012 | Context budget visibility | V3 | `docs/prds/012-context-budget-visibility.md` | Low | Parking lot | 004, 007 |
| 013 | Shareable setup profiles | V3 | `docs/prds/013-shareable-setup-profiles.md` | Low | Parking lot | 008, 009, 010 |
| 014 | Repository health and update awareness | V3 | `docs/prds/014-repository-health-and-update-awareness.md` | Low | Parking lot | 009 |

## V2 Decisions

- Managed sources and mutation targets are global, explicit configuration paths; the launch directory is not product context.
- Relative managed source and global target paths are invalid in V2. Legacy project target fields may be read for compatibility but are not active targets.
- Git source destinations use the repository name. Matching canonical origins are reused without pulling; conflicting or unknown destinations fail without overwrite or suffix guessing.
- Migration candidates are physical skill directories containing `SKILL.md` inside configured global agent targets. Ambiguous or conflicting candidates remain untouched.
- Implementation order is `008`, then `009`, then `010`.

## Change Log

- 2026-06-12: Added and linked the complete V2 PRD set for global execution, managed Git source import, and conservative manual-install migration.
- 2026-06-12: Reframed V2 around globally launched, current-directory-independent operation; removed arbitrary project targeting, moved completed TUI actions into V1, and aligned V1 statuses.
- 2026-05-19: Added V1 PRD links for roadmap items 001-007 and marked those PRDs as drafted.
- 2026-05-19: Updated PRD Candidates table to include version assignment and ordering dependencies.
- 2026-05-19: Initial roadmap created from Big Picture, interview notes, and Rust engineering guidelines.
