---
title: Safe import and removal plans
summary: Stage and confirm filesystem mutations for exposing skills, detaching symlinks, and deleting physical copies.
status: planned
roadmap: v1
---

# Safe import and removal plans

## Context

Trust is the central V1 requirement. Skills Manager will mutate local skill target directories, so users must see exactly what will change before any operation is applied. The app must make symlink removal safe, physical-copy deletion explicit, and failed symlink creation visible.

## Problem

The strongest product failure mode is accidental deletion or hidden mutation. If the app silently falls back from symlink to copy, deletes a real directory while the user expected link removal, or updates in-memory state without rescanning disk, users cannot trust it.

## Goal

Introduce a staged change plan system that all CLI and TUI import/removal flows use before filesystem mutation.

## Non-goals

- Do not implement remote cloning.
- Do not migrate manual installs into the central model.
- Do not silently create physical copies when symlinks fail.
- Do not mutate files outside configured source or target directories.
- Do not maintain a persistent transaction database.

## Proposed experience

For import, the user selects a discovered skill and target agents. The app renders a plan like:

```text
Expose repo-a/code-review to Claude
source: /Users/me/skills/repo-a/code-review
target: /Users/me/.claude/skills/code-review
connection: symlink
```

For removal, the app shows whether it will detach a symlink or delete a physical copy. Physical-copy deletion requires a distinct warning and explicit confirmation.

## Requirements

- Define staged change types for exposing skills, detaching symlinks, and deleting physical copies.
- Implement this planning/apply module before mutating CLI or TUI flows call it, so command routing never owns filesystem safety rules.
- Every mutating flow must render a plan before applying.
- Every mutating flow must ask for confirmation before applying.
- Symlink removal must remove only the link, never the source.
- Physical-copy deletion must require explicit confirmation distinct from ordinary yes/no confirmation.
- Symlink creation must use platform-specific APIs: `std::os::unix::fs::symlink` on Unix and `std::os::windows::fs::symlink_dir` on Windows.
- If symlink creation fails, show a clear error and do not fall back to a physical copy.
- Refuse to mutate outside configured source or target directories.
- After every successful mutation, rescan inventory and render actual state.
- If any apply step fails, report what was attempted and rescan before showing final state.
- Plans created from numbered disambiguation choices must revalidate the selected source/exposure against the latest inventory before applying.

## Safety contract

1. Removing a symlink removes only the symlink, never the source.
2. Physical copies require a separate warning.
3. Physical copies cannot be deleted without explicit confirmation.
4. Mutating operations must render a clear plan before execution.
5. Mutating operations must ask for confirmation before applying changes.
6. After every mutation, rescan the filesystem.
7. Do not follow symlinks during source scanning.
8. Do not silently fall back from symlink to physical copy.
9. Do not scan the entire home directory unless explicitly configured by the user.
10. Do not assume agent directories are stable forever; they must be configurable.
11. Do not auto-clone or auto-execute remote code in v1.
12. Do not mutate files outside configured source or target directories.

## Technical implementation notes

Create the plan model in a module that can be reused by CLI and TUI flows. The engineering guideline example is:

```rust
pub enum StagedChange {
    ExposeSkill { /* skill, agent, source, target, connection */ },
    DetachSkill { /* skill, agent, target */ },
    DeletePhysicalCopy { /* skill, agent, target */ },
}
```

Implement low-level link behavior in `src/symlink.rs`. Keep path classification and mutation safety there or in a nearby filesystem helper, not inside command handlers.

Use `ConnectionKind` from the domain model to decide whether removal is symlink detach, physical delete, missing, or unknown. Unknown connection types should not be deleted automatically.

The plan renderer should be shared enough that CLI and TUI show consistent information, even if they render it differently.

## Success criteria

- Tests prove symlink removal never deletes source content.
- Tests prove physical-copy deletion requires explicit confirmation.
- Tests prove failed symlink creation does not create a fallback copy.
- Tests prove import/remove rescan inventory after mutation.
- Plans include source path, target path, target agent, connection type, and destructive warning where needed.

## Edge cases

- Target path already exists before import.
- Symlink target points outside configured source directories.
- Removing a broken symlink.
- Removing a physical copy with nested files.
- User cancels after seeing the plan.
- Part of a multi-change plan succeeds and a later step fails.

## Dependencies

- Roadmap item `004`.
- Required by roadmap item `007`.
- Used by roadmap item `005` for mutating CLI import/remove behavior.

## Open questions

- Should physical-copy deletion require typing the exact skill name or the exact target path?
- Should multi-change plans apply all-or-nothing, or apply sequentially with clear partial-failure reporting in V1?
