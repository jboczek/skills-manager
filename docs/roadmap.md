# Roadmap: Skills Manager

## Summary

This roadmap turns the Skills Manager product direction into sequenced build slices. The product should become a terminal-first skill exposure manager: one local interface for seeing which skills exist, where they came from, which agents can use them, and what filesystem changes will happen before an import or removal is applied.

V1 should prove the core local workflow before adding broader project management, remote imports, migration, or context-budget analysis.

V1 PRDs live under `docs/prds/v1/` and are linked from their roadmap items below.

## Source Context

- `docs/bigpicture.md`
- `docs/interview.md`
- `docs/skills_manager_rust_guidelines.md`
- Current repository state: Rust CLI crate with configuration, source scanning, and inventory/list slices implemented.

## Roadmap Principles

- Deliver a manually useful local workflow before automating remote or multi-project operations.
- Make filesystem state and pending changes visible before every mutation.
- Keep the product model focused on effective agent availability, not internal folder mechanics.
- Build the crate and domain model simply enough for V1 without prematurely splitting a Rust workspace.
- Prefer independently valuable slices that can each become a focused PRD or implementation plan.

## Target Outcome

Skills Manager should let a power user open a terminal UI, inspect their local skill inventory across Codex, Claude, and Copilot, import selected skills from discovered source repositories, detach exposures safely, and trust that the tool will not hide or guess destructive filesystem changes.

## V1 - Manual but useful exposure workflow

### Version Goal

Create the first usable Rust CLI/TUI that can discover skills, show effective availability across configured agents, and safely apply basic import/remove exposure changes.

### Roadmap Items

### 001 - Rust project foundation

- **Type:** Technical enabler
- **Outcome:** The project can be built, tested, linted, and evolved as a native Rust CLI/TUI application.
- **Description:** Create the initial single-crate Rust project, fixed dependency set, basic module layout, lockfile, and minimal command routing for local development.
- **Why now:** Every later roadmap item depends on having a working crate and agreed module boundaries.
- **PRD candidate:** Yes. PRD: [`docs/prds/v1/prd-001-rust-project-foundation.md`](prds/v1/prd-001-rust-project-foundation.md)
- **Dependencies:** None known.
- **Validation signal:** `cargo check --locked`, `cargo test --locked`, formatting, and clippy can run successfully on the empty foundation locally.
- **Notes / risks:** Dependency policy needs care because the guidelines require maintained crates but avoid releases newer than 14 days.

### 002 - Configuration and agent definitions

- **Type:** Capability
- **Outcome:** Users can initialize and inspect the local configuration that defines source roots and agent target paths.
- **Description:** Add config loading, opinionated default config generation, `config init`, `config show`, `config path`, and config-driven agent definitions for Codex, Claude, and Copilot. Shared target folders such as `.agents` stay hidden inside configuration.
- **Why now:** Scanning and exposure logic need explicit source and target paths before they can be safe.
- **PRD candidate:** Yes. PRD: [`docs/prds/v1/prd-002-configuration-and-agent-definitions.md`](prds/v1/prd-002-configuration-and-agent-definitions.md)
- **Dependencies:** 001.
- **Validation signal:** A fresh machine can generate a readable TOML config with usable defaults and inspect the resolved paths without mutating skill directories.
- **Notes / risks:** The UI must treat `.agents` as a technical target, not as the primary product-facing concept.

### 003 - Skill source scanning

- **Type:** Capability
- **Outcome:** Users can discover available skills in configured source locations without changing the filesystem.
- **Description:** Scan `central_dir` and configured parent directories for `SKILL.md`, preserve nested hierarchy, resolve repository roots, and derive origin from `git remote`.
- **Why now:** Import selection only works if the app can reliably find candidate skills first.
- **PRD candidate:** Yes. PRD: [`docs/prds/v1/prd-003-skill-source-scanning.md`](prds/v1/prd-003-skill-source-scanning.md)
- **Dependencies:** 001, 002.
- **Validation signal:** `skills-manager scan` finds single-skill, multi-skill, and nested-skill repositories in temp filesystem tests.
- **Notes / risks:** Recursive scanning must be bounded and must not follow symlinks.

### 004 - Inventory and exposure resolution

- **Type:** Capability
- **Outcome:** Users can see what skills are currently exposed to each configured agent and how each exposure is connected.
- **Description:** Build current inventory from config, source repositories, agent target directories, symlinks, physical copies, and current project-local paths.
- **Why now:** The core product value is visibility before mutation.
- **PRD candidate:** Yes. PRD: [`docs/prds/v1/prd-004-inventory-and-exposure-resolution.md`](prds/v1/prd-004-inventory-and-exposure-resolution.md)
- **Dependencies:** 002, 003.
- **Validation signal:** `skills-manager list` shows consolidated rows with skill, source, agent availability, scope, and connection type.
- **Notes / risks:** Duplicate names and shared target paths can confuse users unless namespace, numbered disambiguation, and effective availability rules are explicit.

### 005 - Human-readable CLI workflow

- **Type:** Feature
- **Outcome:** Users can list, scan, import, remove, and check setup from scriptable commands while the TUI is still maturing.
- **Description:** Add `list`, `scan`, and `doctor` command flows with human-readable tables, plus CLI entry points for import/remove that reuse the safe planning module before any mutation.
- **Why now:** CLI flows are easier to test deeply and provide useful behavior before the full-screen UI is complete.
- **PRD candidate:** Yes. PRD: [`docs/prds/v1/prd-005-human-readable-cli-workflow.md`](prds/v1/prd-005-human-readable-cli-workflow.md)
- **Dependencies:** 002, 003, 004, 006 for mutating import/remove behavior.
- **Validation signal:** CLI snapshot tests cover list, scan, and config output; import/remove tests apply changes in temporary directories.
- **Notes / risks:** CLI must stay secondary and should not force premature JSON or automation contracts.

### 006 - Safe import and removal plans

- **Type:** Capability
- **Outcome:** Users can preview and confirm exactly which symlinks or local copies will change before applying imports or removals.
- **Description:** Introduce staged change plans for exposing skills, detaching symlinks, and deleting physical copies with explicit confirmation rules. This module is implemented before mutating CLI or TUI flows call it.
- **Why now:** Trust in mutation safety is the central V1 adoption requirement.
- **PRD candidate:** Yes. PRD: [`docs/prds/v1/prd-006-safe-import-and-removal-plans.md`](prds/v1/prd-006-safe-import-and-removal-plans.md)
- **Dependencies:** 004.
- **Validation signal:** Tests prove symlink removal never deletes source content, physical copies require explicit confirmation, and inventory is rescanned after mutation.
- **Notes / risks:** The strongest product failure mode is accidental deletion or a hidden fallback from symlink to copy.

### 007 - Assistant-style TUI shell

- **Type:** UX
- **Outcome:** Running `skills-manager` opens a full-screen terminal experience with a prompt, status feed, command hints, and complete V1 modes.
- **Description:** Implement the TUI layout, home mode, prompt command parsing, help, status feed, list mode, scan/config modes, and complete guided import/remove flows backed by the safe planning module.
- **Why now:** The intended product is terminal-first and assistant-style; V1 is incomplete if the default experience is only direct CLI commands.
- **PRD candidate:** Yes. PRD: [`docs/prds/v1/prd-007-assistant-style-tui-shell.md`](prds/v1/prd-007-assistant-style-tui-shell.md)
- **Dependencies:** 004, 005, 006.
- **Validation signal:** A user can launch the app, inspect status, run list/scan/config flows, complete import/remove with staged plan review and confirmation, and quit cleanly from the TUI.
- **Notes / risks:** The table matters, but the app should not become a raw admin grid without the broader assistant-like shell.

### Version Validation

V1 works locally if a user can initialize default source and target paths, scan discovered skills, view consolidated availability across Codex, Claude, and Copilot, import selected skills through explicit plans, remove exposures safely, and use the default TUI without needing to understand internal directory topology.

### Key Risks / Assumptions

- Agent directory conventions are stable enough to start with configurable defaults.
- The symlink-first model is acceptable for the target users.
- A single-crate architecture can carry V1 without becoming hard to reason about.
- Users will tolerate editing generated defaults in V1 if config display and diagnostics are clear.

## V2 - Broader local workspace management

### Version Goal

Reduce repeated setup across projects while keeping the same inspectable source-plus-exposure model.

### Roadmap Items

### 008 - Arbitrary project targeting

- **Type:** Feature
- **Outcome:** Users can inspect and manage project-local skills outside the current working directory.
- **Description:** Add explicit project path targeting for inventory and exposure operations.
- **Why now:** V1 proves current-directory workflows first; V2 can expand to multi-project users.
- **PRD candidate:** Yes. Suggested file: `docs/prds/008-arbitrary-project-targeting.md`
- **Dependencies:** 004, 006, 007.
- **Validation signal:** Users can point Skills Manager at a project path and see project-local availability without changing directories.
- **Notes / risks:** Path targeting increases mutation risk and needs especially clear previews.

### 009 - Git URL import into managed source directory

- **Type:** Integration
- **Outcome:** Users can add a new skill source without manually cloning it first.
- **Description:** Clone a provided Git URL into the managed source directory, scan it, and let the user select specific skills to expose.
- **Why now:** V1 intentionally avoids remote code acquisition; V2 can add it after local import semantics are trusted.
- **PRD candidate:** Yes. Suggested file: `docs/prds/009-git-url-import-into-managed-source-directory.md`
- **Dependencies:** 003, 006.
- **Validation signal:** A user can import from a test Git repository and still preview exposures before applying them.
- **Notes / risks:** Remote import must not auto-execute code and should make source location explicit.

### 010 - Manual install migration

- **Type:** Capability
- **Outcome:** Users can convert older copied installs into the central source-plus-symlink model.
- **Description:** Detect physical copies, propose a migration plan, preserve source content, and replace target copies with links only after confirmation.
- **Why now:** Migration is useful only after V1 can accurately classify existing exposures.
- **PRD candidate:** Yes. Suggested file: `docs/prds/010-manual-install-migration.md`
- **Dependencies:** 004, 006.
- **Validation signal:** A dry migration preview explains exactly what will be moved, linked, or left untouched.
- **Notes / risks:** Migration is destructive-adjacent and should be conservative by default.

### 011 - Stronger TUI exposure editing

- **Type:** UX
- **Outcome:** Users can stage exposure changes directly from the inventory table.
- **Description:** Add table cell navigation, space-to-toggle exposure, staged change review, and apply shortcuts.
- **Why now:** V1 can rely on guided flows; V2 should make repeated management faster.
- **PRD candidate:** Yes. Suggested file: `docs/prds/011-stronger-tui-exposure-editing.md`
- **Dependencies:** 006, 007.
- **Validation signal:** A user can toggle multiple exposures in one session and apply them after reviewing the staged plan.
- **Notes / risks:** Fast toggles must not reduce safety or make effective availability harder to understand.

### Version Validation

V2 works if users can manage multiple local projects and newly cloned sources without falling back to manual copying or losing confidence in what will change.

### Key Risks / Assumptions

- Users have enough repeated project setup pain to justify project targeting.
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
| 001 | Rust project foundation | V1 | [`docs/prds/v1/prd-001-rust-project-foundation.md`](prds/v1/prd-001-rust-project-foundation.md) | High | Drafted | None |
| 002 | Configuration and agent definitions | V1 | [`docs/prds/v1/prd-002-configuration-and-agent-definitions.md`](prds/v1/prd-002-configuration-and-agent-definitions.md) | High | Drafted | 001 |
| 003 | Skill source scanning | V1 | [`docs/prds/v1/prd-003-skill-source-scanning.md`](prds/v1/prd-003-skill-source-scanning.md) | High | Done | 001, 002 |
| 004 | Inventory and exposure resolution | V1 | [`docs/prds/v1/prd-004-inventory-and-exposure-resolution.md`](prds/v1/prd-004-inventory-and-exposure-resolution.md) | High | Done | 002, 003 |
| 005 | Human-readable CLI workflow | V1 | [`docs/prds/v1/prd-005-human-readable-cli-workflow.md`](prds/v1/prd-005-human-readable-cli-workflow.md) | Medium | Drafted | 002, 003, 004, 006 for mutations |
| 006 | Safe import and removal plans | V1 | [`docs/prds/v1/prd-006-safe-import-and-removal-plans.md`](prds/v1/prd-006-safe-import-and-removal-plans.md) | High | Drafted | 004 |
| 007 | Assistant-style TUI shell | V1 | [`docs/prds/v1/prd-007-assistant-style-tui-shell.md`](prds/v1/prd-007-assistant-style-tui-shell.md) | High | Drafted | 004, 005, 006 |
| 008 | Arbitrary project targeting | V2 | `docs/prds/008-arbitrary-project-targeting.md` | Medium | Later | 004, 006, 007 |
| 009 | Git URL import into managed source directory | V2 | `docs/prds/009-git-url-import-into-managed-source-directory.md` | Medium | Later | 003, 006 |
| 010 | Manual install migration | V2 | `docs/prds/010-manual-install-migration.md` | Medium | Later | 004, 006 |
| 011 | Stronger TUI exposure editing | V2 | `docs/prds/011-stronger-tui-exposure-editing.md` | Medium | Later | 006, 007 |
| 012 | Context budget visibility | V3 | `docs/prds/012-context-budget-visibility.md` | Low | Parking lot | 004, 007 |
| 013 | Shareable setup profiles | V3 | `docs/prds/013-shareable-setup-profiles.md` | Low | Parking lot | 008, 009, 010 |
| 014 | Repository health and update awareness | V3 | `docs/prds/014-repository-health-and-update-awareness.md` | Low | Parking lot | 009 |

## Open Questions

- How much first-run guidance is necessary after provided defaults are generated?
- Should V1 include project-local skill management in the TUI, or only detection and display?
- What extra path or origin detail should be shown alongside numbered options when duplicate skill names collide?
- Which provided agent target path defaults are acceptable for the first local V1, and which should only be warnings from `doctor`?
- How strong should confirmation be for physical-copy deletion: yes/no prompt, typed path, or typed phrase?

## Decisions Needed

- Confirm that V1 should start with a single Rust crate, not a workspace.
- Confirm whether `skills-manager scan` should be part of the first implementation slice or follow config initialization.
- Confirm the first TUI implementation includes complete guided import/remove flows backed by the shared plan/apply module.
- Decide the minimum local dependency age verification process before the first `Cargo.toml` is committed.

## Change Log

- 2026-05-19: Added V1 PRD links for roadmap items 001-007 and marked those PRDs as drafted.
- 2026-05-19: Updated PRD Candidates table to include version assignment and ordering dependencies.
- 2026-05-19: Initial roadmap created from Big Picture, interview notes, and Rust engineering guidelines.
