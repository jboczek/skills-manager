---
title: Global execution context
summary: Make scanning, inventory, plans, and targets independent of the directory and Git branch from which Skills Manager is launched.
status: done
roadmap: v2
---

# Global execution context

## Context

Skills Manager is a globally launched skill exposure manager. V1 established source scanning, live inventory, effective agent availability, and staged import and removal plans, but some behavior still treats the shell's current working directory as product context. Relative paths can resolve against that directory, inventory can include project-local exposures found there, and the TUI presents the current directory and Git branch.

## Problem

Launching from a different folder can silently change visible exposures and mutation targets. Displaying a Git branch reinforces the false impression that Skills Manager manages the active repository. Legacy `project_dir` configuration also keeps an obsolete project-scope model alive.

## Goal

Make explicit configuration the only discovery boundary. Scanning, inventory, diagnostics, plans, and TUI state must be independent of CWD and Git branch while still reporting read-only project-local exposures found inside configured source repositories.

## Non-goals

- Do not add a `--project` option or any other arbitrary project selector.
- Do not discover repositories outside configured source roots.
- Do not mutate repository-local skill directories.
- Do not remove the V1 scanner, live inventory, effective availability, or staged-plan safety model.
- Do not migrate physical copies or clone remote repositories in this feature.
- Do not treat Git branch, repository state, or launch directory as source identity.

## User stories

1. As a user, I want the same inventory from every launch directory, so that shell location cannot change managed state.
2. As a cautious user, I want plans limited to configured global targets, so that unrelated repositories cannot be mutated.
3. As a user with a central library, I want scanning to use explicit global sources, so that discovery is predictable.
4. As a TUI user, I want CWD and branch removed from managed context, so that the product model is clear.
5. As a config editor, I want relative managed paths rejected, so that their meaning cannot change between shells.
6. As a user, I want leading-tilde paths expanded consistently, so that global configuration stays portable.
7. As a V1 user, I want old `project_dir` values to parse, so that upgrading does not invalidate my config.
8. As a V1 user, I want diagnostics when `project_dir` is ignored, so that removed local inventory is understandable.
9. As a new user, I want generated config to contain only active fields, so that obsolete concepts are not advertised.
10. As a user, I want invalid paths reported before scanning or planning, so that partial state is not presented as valid.
11. As a CLI and TUI user, I want both interfaces to share one global context, so that their results agree.
12. As a maintainer, I want launch-directory invariance tests, so that implicit project context cannot return.

## Proposed experience

The user configures managed sources and targets with absolute paths or paths beginning with `~`. Skills Manager expands a leading tilde, validates every active path as absolute, and uses the resulting global context for all operations.

Launching list, scan, mutation flows, or the TUI from unrelated directories produces equivalent results. The scanner catalogs every skill source under configured roots. Inventory then inspects each scanned Git repository for fixed project-local exposure conventions: `.claude/skills`, `.codex/skills`, `.copilot/skills`, and `.agents/skills`.

`/scan` shows the complete source catalog. `/list` shows only skills exposed to at least one agent. Global and project-local exposures are separate rows. Project-local rows include their repository context, are read-only, and never become mutation targets. `.agents/skills` grants effective availability to Codex and Copilot.

The TUI prompt remains the command entry point but no longer shows a directory or Git branch. Status describes global configuration, sources, agents, and diagnostics.

Legacy `project_dir` values still parse but produce compatibility diagnostics and have no effect. New and normalized configuration omits them.

## Requirements

- Active V2 managed source and global target paths must be absolute after leading-tilde expansion.
- A relative `central_dir`, `scan_parent_dirs` entry, agent global target, or shared global target must fail validation; it must never resolve against CWD.
- Validation must complete before scanning, inventory construction, or plan creation.
- Scanner inputs must come from global context while preserving V1 depth, symlink, deduplication, origin, and warning behavior.
- Inventory must include only actual exposures found in configured global targets or fixed project-local targets inside scanned Git repositories.
- Source scan results without an exposure must not produce inventory rows.
- A repository contributes project-local targets only when scanning found at least one `SKILL.md` within that repository.
- Project-local target conventions are fixed: `.claude/skills` for Claude, `.codex/skills` for Codex, `.copilot/skills` for Copilot, and `.agents/skills` for Codex and Copilot.
- Global and project-local exposures of the same source must produce separate rows. Exposures in different projects must also produce separate rows.
- Nested repositories use the nearest Git root reported by source scanning.
- Project-local rows must retain actual source metadata while also carrying their containing project context.
- Project-local rows are read-only and cannot be imported into, detached, removed, or physically deleted.
- Import and removal must preserve V1 plan preview, confirmation, connection classification, rescan, and deletion safeguards.
- Every mutation target must belong to the validated configured global targets.
- Legacy agent and shared-target `project_dir` values must parse, be ignored, emit diagnostics, and be omitted from new or normalized config.
- No CLI command may accept `--project`.
- The TUI must not detect or display CWD or Git branch as managed context.
- CLI and TUI flows must consume the same resolved global context and diagnostics.
- Invalid configuration must name the field and rejected value.

## Success criteria

- The same config produces the same scans, inventory, availability, and plans from unrelated directories.
- CWD-local skill folders do not appear merely because of launch location.
- Project-local skills appear only when their repository is represented in configured scan results.
- Every list row has at least one effective agent exposure.
- Global and project-local copies of the same skill remain distinguishable.
- Every active relative managed path fails before filesystem discovery or mutation.
- Legacy `project_dir` configs load with diagnostics and do not define project-local inventory targets.
- New configuration contains no `project_dir` fields.
- The TUI contains no CWD or Git branch context.
- V1 scan, inventory, disambiguation, and mutation safety remains intact for global paths.

## Edge cases

- A configured path is exactly `~`, contains `~` after the first component, or expands to a missing directory.
- The same global target is referenced directly and through a shared target.
- A legacy config contains several `project_dir` values.
- CWD is unavailable, deleted after launch, or inside a Git worktree.
- A relative path exists beneath CWD and would have been valid under V1.
- A symlink in a global target points to a source outside configured source roots.
- Config validation reports both ignored legacy fields and fatal relative active paths.

## Dependencies

- V1 configuration and agent definitions, scanning, inventory, staged plans, and TUI table actions.
- Enables V2 Git URL import and manual install migration.

## Implementation decisions

- Use one resolved global context for CLI and TUI containing validated sources, enabled global targets, shared targets, and diagnostics.
- Keep raw configuration parsing separate from active configuration resolution so legacy fields can be accepted without influencing behavior.
- Treat path validation as a configuration boundary, not a scanner or planner fallback.
- Preserve V1 scan, availability, connection, and plan concepts while separating source catalog entries from exposure inventory rows.
- Derive read-only project-local targets from scanned repository roots and fixed conventions, not from CWD or legacy `project_dir` configuration.
- Render compatibility diagnostics consistently in configuration inspection, doctor output, and TUI status without blocking otherwise valid global behavior.

## Testing decisions

- Add configuration tests for absolute paths, leading-tilde expansion, rejected relative paths, ignored legacy `project_dir` values, diagnostics, and serialization without legacy fields.
- Add launch-directory invariance tests for list, scan, import planning, and removal planning.
- Add inventory tests proving source-only rows are excluded, configured global/shared targets still produce effective availability, project-local conventions are discovered, `.agents` maps to Codex and Copilot, and contexts do not collapse.
- Add plan tests proving every generated target belongs to validated global targets and relative configuration cannot reach plan creation.
- Add TUI state and rendering tests proving prompt and status content contain no CWD or Git branch and still show useful global configuration diagnostics.
- Retain V1 regression coverage for scan boundaries, disambiguation, detach safety, physical deletion, partial apply, and rescanning.

## Progress notes

- 2026-06-12: Added a resolved global context that expands leading tildes, validates every active source and global target as absolute, and aggregates compatibility diagnostics.
- 2026-06-12: Removed CWD resolution and project-local targets from scan, inventory, doctor, import, remove, and TUI flows.
- 2026-06-12: Kept legacy `project_dir` parsing while ignoring those values, reporting diagnostics, and omitting them from generated or normalized config.
- 2026-06-12: Removed TUI launch-directory and Git-branch state and replaced it with a global prompt and global-context status.
- 2026-06-12: Added CLI and TUI regression coverage for invalid relative paths, ignored local targets, global shared targets, diagnostics, and launch-directory-invariant inventory and plans.
- 2026-06-12: Reopened the PRD after correcting the model: source scanning remains global and complete, while list inventory contains only real global or read-only project-local exposures discovered inside configured source repositories.
- 2026-06-12: Separated source catalog entries from inventory rows, added fixed project-local target discovery, grouped TUI rows by exposure context, rendered actual source paths, and blocked local mutation in CLI and TUI.
- 2026-06-12: Corrected TUI grouping so global rows are separated by source repository instead of sharing one scope bucket; project-local rows remain grouped by containing project.

## Tasks

- [x] Add failing configuration tests for path validation, tilde expansion, legacy fields, diagnostics, and serialization.
- [x] Implement one resolved global context shared by CLI and TUI.
- [x] Migrate scan, list, doctor, import, and remove to validated global sources and targets.
- [x] Remove CWD and Git branch from TUI state and rendering.
- [x] Add launch-directory invariance tests for inventory and mutation plans.
- [x] Separate source catalog rows from exposure inventory rows.
- [x] Discover fixed project-local exposure targets inside scanned repositories.
- [x] Render project context without replacing actual source information.
- [x] Block project-local mutation in CLI and TUI flows.
- [x] Add corrected CLI and TUI regression coverage.
- [x] Update durable feature documentation, README, roadmap, and changelog.
