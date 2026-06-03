---
title: Inventory and exposure resolution
summary: Build the consolidated view of discovered skills, agent availability, scope, source, and connection type.
status: planned
roadmap: v1
---

# Inventory and exposure resolution

## Context

The core value of Skills Manager is answering what skills exist, where they came from, which agents can use them, and how each exposure is connected. Source scanning alone only finds available skills. Inventory resolution must combine scan results with real agent target directories, symlinks, physical copies, and current project-local paths.

## Problem

Users do not need a dump of every filesystem path. They need one clear row per logical skill where possible, with effective availability for Codex, Claude, and Copilot. Duplicate names, shared implementation targets, symlinks, physical copies, and project-local directories can make that view misleading unless normalization and disambiguation rules are explicit.

## Goal

Build an inventory model that reflects real filesystem state without storing a separate database, consolidates exposures into readable rows, and distinguishes effective availability from low-level target mechanics.

## Non-goals

- Do not create persistent inventory storage in V1.
- Do not migrate manual installs into the central model.
- Do not track branches, commits, dirty state, or update availability.
- Do not hide physical copies or unknown provenance.
- Do not make `.agents` a product-facing agent, command target, or table column.

## Proposed experience

A user runs:

```bash
skills-manager list
```

The output shows consolidated rows with columns such as skill, source, Codex, Claude, Copilot, scope, and connection. The TUI list mode later uses the same inventory state.

## Requirements

- Build inventory from real state every time it is requested.
- Use sources of truth in this order: config, source repositories, agent target directories, symlinks and physical copies, and current project directory.
- Represent global and project-local scopes.
- Detect configured agent target directories from config, not hardcoded paths.
- Detect whether an exposure is a symlink, physical copy, missing target, or unknown connection.
- Infer symlink source repository and origin where possible.
- Render unknown provenance as `unknown`.
- Consolidate the same logical skill into one row where possible.
- Show effective availability for Codex, Claude, and Copilot.
- Treat shared `.agents` targets as config-only implementation details that can contribute to effective Codex or Copilot availability according to config.
- When multiple rows have the same display namespace, render numbered choices such as `(1)` and `(2)` with enough source path or origin context for the user to pick the intended skill.
- Rescan inventory after every mutation in later import/remove flows.

## Technical implementation notes

Implement inventory composition in `src/inventory.rs`. Keep it separate from terminal rendering in `output.rs` and TUI rendering in `tui/components/table.rs`.

Use domain types from `domain.rs`, especially `SkillId`, `AgentId`, `SkillSource`, `SkillExposure`, `ConnectionKind`, `Scope`, and `InventoryRow`.

Connection detection belongs behind small filesystem helpers. For symlinks, inspect the link itself and resolve the target only to identify source metadata; removing the link later must never delete the source.

The display identity rule for V1 should be `repo-name/skill-name` when repository name is known. If the source is unknown, use the visible skill directory name and mark provenance as unknown. Collisions must not collapse into a single row. Show duplicate display identities as numbered choices, for example `(1) repo-a/code-review` and `(2) repo-a/code-review`, with path or origin context so import/remove flows can ask the user to choose the intended item.

## Success criteria

- `skills-manager list` can show skills exposed to one or more configured agents.
- A skill exposed through multiple paths can appear as one consolidated row with multiple availability markers.
- Symlink and physical-copy exposures are distinguished.
- Project-local and global scopes are distinguished.
- Tests cover duplicate names with numbered disambiguation, hidden shared `.agents` effective availability, symlink resolution, physical copy detection, and unknown origin.

## Edge cases

- A target directory exists but is empty.
- A configured target directory does not exist.
- A symlink points to a deleted source.
- A physical copy has no Git provenance.
- Two skills share the same `repo-name/skill-name` display namespace.
- Codex and Copilot both derive availability from the same config-only shared target.

## Dependencies

- Roadmap items `002` and `003`.
- Enables roadmap items `005`, `006`, and `007`.

## Open questions

- How much path detail should be shown next to numbered namespace collision choices?
- Should broken symlinks be listed as missing exposures, warnings, or both?
