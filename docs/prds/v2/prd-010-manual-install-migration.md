---
title: Manual install migration
summary: Convert eligible global physical skill installs into verified managed sources with symlink exposures.
status: planned
roadmap: v2
---

# Manual install migration

## Context

Skills Manager classifies target entries as symlinks, physical copies, missing paths, or unknown connections. Manual installs often appear as physical copies without managed sources. V2 needs an explicit way to move them into the central source-plus-symlink model without guessing or losing content.

## Problem

Physical installs are hard to maintain and easy to mistake for managed exposures. Silent replacement would undermine visible, reversible filesystem changes.

## Goal

Provide a conservative `migrate` flow that discovers eligible global physical installs, explains which source will be created or reused, replaces confirmed copies with verified symlinks, and leaves every ambiguous or unsafe item untouched.

## Non-goals

- Do not silently bulk-migrate inventory during startup, scan, import, or list.
- Do not migrate project-local targets or accept an arbitrary project path.
- Do not migrate symlinks, files, directories without `SKILL.md`, unknown entry types, or paths outside configured global targets.
- Do not deduplicate or reorganize existing managed sources.
- Do not update, merge, overwrite, or delete an existing managed source.
- Do not add remote acquisition, versioning, or source updates.

## User stories

1. As a user, I want to run migration explicitly, so that inventory operations remain read-only.
2. As a user, I want discovery limited to configured global agent targets, so that my launch directory cannot expand the migration scope.
3. As a user, I want only physical directories containing `SKILL.md` proposed, so that unrelated entries are ignored.
4. As a user, I want unsupported entries reported as untouched, so that the tool never guesses.
5. As a user, I want an exact existing managed source reused when it is the only content match, so that the resulting link points to content I already manage.
6. As a user, I want an unmatched copy adopted under `central_dir`, so that its content becomes a managed source before its exposure changes.
7. As a user, I want deterministic destinations and collision refusal, so that adoption never overwrites or renames another source.
8. As a user, I want ambiguous or changed paths left untouched with reasons, so that I can resolve them.
9. As a user, I want the plan to show source, backup, link, and untouched actions, so that confirmation is informed.
10. As a user, I want the original preserved until verification and restored on failure, so that migration cannot lose my skill.
11. As a user, I want inventory rescanned after apply, so that displayed state matches disk.

## Proposed experience

The user runs `skills-manager migrate` or `/migrate` in the TUI. Skills Manager scans configured global agent and shared targets, then renders a plan without changing disk.

Each candidate is shown as one of:

- **Reuse source:** link the target to one unique exact content match already under `central_dir`.
- **Create source:** copy the candidate to `central_dir/<skill-name>` when that path is free, then replace the target with a link.
- **Untouched:** explain the symlink, unsupported entry, outside-target path, ambiguity, collision, or state change that prevents migration.

The plan shows the target, source creation or reuse, temporary backup, replacement symlink, and untouched reason. Applying requires exact `yes`. After completion or failure, Skills Manager rescans and renders inventory.

## Requirements

- Discovery must inspect only enabled configured global agent directories and enabled referenced global shared-target directories.
- A candidate must be a physical directory directly inside a target and contain `SKILL.md`.
- The same configured filesystem target must be inspected once even when shared by multiple agents.
- Exact matching must compare the complete tree and file bytes without following symlinks. Unsupported nested entries remain untouched.
- Reuse is allowed only when exactly one managed skill directory under `central_dir` has identical content.
- Multiple exact matches are ambiguous and must remain untouched.
- An unmatched candidate may be copied to `central_dir/<skill-name>` only when that path does not exist.
- If the adoption destination already exists and is not the unique exact source match, the candidate must remain untouched; do not overwrite, merge, rename, or add a suffix.
- Apply must revalidate candidate type, containment, content, source match, and destination availability.
- Apply must stop on the first failure, restore the affected target, and report completed, restored, and untouched items.
- No original physical copy may be removed before the managed source exists, matches the candidate, and the replacement symlink resolves to it.
- Temporary backup removal may happen only after source and link verification succeeds.
- A successful or failed apply must trigger a fresh inventory scan.

## Success criteria

- A dry plan classifies every inspected entry as reusable, adoptable, or untouched with a reason.
- A uniquely matching managed source is reused without modification.
- An unmatched copy is preserved in a new managed source and exposed through a verified symlink.
- Ambiguous, unsupported, colliding, changed, or outside-target items remain unchanged.
- Injected failures after backup or link creation restore the original physical install.
- Launching the same configured migration from unrelated directories produces the same candidates and plan.

## Edge cases

- Multiple agents reference the same global shared target.
- `central_dir` is missing, unwritable, inside a target, or resolves outside its configured path.
- A target entry changes between discovery, confirmation, and apply.
- A candidate contains nested symlinks, sockets, devices, unreadable files, or permission-restricted directories.
- A matching source exists more than once or disappears before apply.
- A free adoption destination selected during planning is created by another process before apply.
- Backup creation, source copy, verification, symlink creation, rollback, or rescan fails.

## Dependencies

- PRD 004 inventory and exposure resolution for physical-copy classification.
- PRD 006 safe plans for shared confirmation, rendering, and post-mutation rescan behavior.
- PRD 008 global execution context so migration scope never depends on the current directory.

## Implementation decisions

- Add shared migration discovery and planning services for CLI and TUI; command handlers own interaction only.
- Extend staged plans with migration-specific source-create/source-reuse, backup, replacement-link, and untouched records rather than expressing migration as physical deletion.
- Use a deterministic tree manifest and file-byte hashes. Paths, entry kinds, and bytes must match; timestamps and ownership do not.
- For adoption, copy to a temporary path under `central_dir`, verify it, then finalize the selected free destination. Reused sources are never modified.
- Before linking, rename the target to a temporary sibling backup. On failure, remove an incomplete replacement, restore the backup, and remove new migration artifacts when safe. Delete the backup only after verification.

## Testing decisions

- Unit-test target discovery, containment checks, candidate classification, deterministic content identity, unique-match selection, deterministic destination naming, collision refusal, plan rendering, and revalidation.
- Use temporary filesystem integration tests for unique reuse, unmatched adoption, ambiguous matches, shared targets, nested unsupported entries, concurrent state changes, and launch-directory independence.
- Add failure-injection tests at source copy, backup, link, verification, and rescan boundaries; assert that the original physical directory and its contents remain available.
- Add CLI and TUI flow tests proving no mutation occurs before exact confirmation and that refreshed inventory reports the final connection type.
