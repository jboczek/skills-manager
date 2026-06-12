---
title: Global execution context
summary: Make scanning, inventory, plans, and targets independent of the directory and Git branch from which Skills Manager is launched.
status: planned
roadmap: v2
---

# Global execution context

## Context

Skills Manager is a globally launched skill exposure manager. V1 established source scanning, live inventory, effective agent availability, and staged import and removal plans, but some behavior still treats the shell's current working directory as product context. Relative paths can resolve against that directory, inventory can include project-local exposures found there, and the TUI presents the current directory and Git branch.

## Problem

Launching from a different folder can silently change visible exposures and mutation targets. Displaying a Git branch reinforces the false impression that Skills Manager manages the active repository. Legacy `project_dir` configuration also keeps an obsolete project-scope model alive.

## Goal

Make explicit global configuration the only managed execution context. Scanning, inventory, diagnostics, plans, and TUI state must be independent of CWD and Git branch.

## Non-goals

- Do not add a `--project` option or any other arbitrary project selector.
- Do not discover or manage repository-local skill directories implicitly.
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

Launching list, scan, mutation flows, or the TUI from unrelated directories produces equivalent results. `.codex/skills`, `.copilot/skills`, and `.agents/skills` beneath CWD are ignored unless explicitly configured by absolute or tilde-expanded path.

The TUI prompt remains the command entry point but no longer shows a directory or Git branch. Status describes global configuration, sources, agents, and diagnostics.

Legacy `project_dir` values still parse but produce compatibility diagnostics and have no effect. New and normalized configuration omits them.

## Requirements

- Active V2 managed source and global target paths must be absolute after leading-tilde expansion.
- A relative `central_dir`, `scan_parent_dirs` entry, agent global target, or shared global target must fail validation; it must never resolve against CWD.
- Validation must complete before scanning, inventory construction, or plan creation.
- Scanner inputs must come from global context while preserving V1 depth, symlink, deduplication, origin, and warning behavior.
- Inventory must use configured sources and global targets only; project-local targets must not contribute rows or availability.
- Import and removal must preserve V1 plan preview, confirmation, connection classification, rescan, and deletion safeguards.
- Every mutation target must belong to the validated configured global targets.
- Legacy agent and shared-target `project_dir` values must parse, be ignored, emit diagnostics, and be omitted from new or normalized config.
- No CLI command may accept `--project`.
- The TUI must not detect or display CWD or Git branch as managed context.
- CLI and TUI flows must consume the same resolved global context and diagnostics.
- Invalid configuration must name the field and rejected value.

## Success criteria

- The same config produces the same scans, inventory, availability, and plans from unrelated directories.
- CWD-local skill folders do not appear unless explicitly configured as global paths.
- Every active relative managed path fails before filesystem discovery or mutation.
- Legacy `project_dir` configs load with diagnostics but create no project-local inventory or targets.
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
- Preserve V1 scan, inventory, availability, connection, and plan concepts. Active V2 config no longer produces project-local scope.
- Render compatibility diagnostics consistently in configuration inspection, doctor output, and TUI status without blocking otherwise valid global behavior.

## Testing decisions

- Add configuration tests for absolute paths, leading-tilde expansion, rejected relative paths, ignored legacy `project_dir` values, diagnostics, and serialization without legacy fields.
- Add launch-directory invariance tests for list, scan, import planning, and removal planning.
- Add inventory tests proving CWD-local agent folders are ignored and configured global/shared targets still produce effective availability.
- Add plan tests proving every generated target belongs to validated global targets and relative configuration cannot reach plan creation.
- Add TUI state and rendering tests proving prompt and status content contain no CWD or Git branch and still show useful global configuration diagnostics.
- Retain V1 regression coverage for scan boundaries, disambiguation, detach safety, physical deletion, partial apply, and rescanning.
