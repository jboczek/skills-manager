---
title: Configuration and agent definitions
summary: Add persistent TOML configuration for source roots, scan settings, and configurable agent target paths.
status: done
roadmap: v1
---

# Configuration and agent definitions

## Context

Skills Manager must not guess where skills live or where agents read them from. V1 needs an explicit configuration file that defines source roots, scan depth, supported agents, and target directories. This makes later scanning, inventory resolution, import, and removal behavior inspectable and safe.

The product-facing model is agent availability: Codex, Claude, and Copilot. Shared folders such as `.agents` are implementation targets in config, not the main product concept.

## Problem

Users currently lose track of which skill directories are active for each tool. If Skills Manager hardcodes paths or exposes implementation targets as product concepts, it will repeat the same confusion it is meant to solve. If first-run defaults are absent, users must already know the directory topology before the product can help them.

## Goal

Provide a simple, editable TOML configuration that users can initialize, inspect, and use as the source of truth for all V1 scan and exposure operations.

## Non-goals

- Do not build a rich config editor inside the TUI in V1.
- Do not auto-discover every possible agent directory convention.
- Do not treat `.agents` as a product-facing agent.
- Do not mutate skill directories during config initialization.
- Do not add cloud sync, profiles, or team config sharing.

## Proposed experience

A user can run:

```bash
skills-manager config init
skills-manager config path
skills-manager config show
```

`config init` creates `~/.config/skills-manager/config.toml` with opinionated but editable defaults if it does not already exist. `config path` prints the resolved config path. `config show` prints normalized, human-readable key values so the user can verify which source roots and agent target paths Skills Manager will use.

## Requirements

- Store V1 config at `~/.config/skills-manager/config.toml` using platform-correct resolution through `directories`.
- Use TOML serialization through `serde` and `toml`.
- Include `[skills]` settings for `central_dir`, `scan_parent_dirs`, and `max_scan_depth`.
- Default `central_dir` to `~/skills` and `scan_parent_dirs` to an empty list so first-run config is useful without broad filesystem traversal.
- Default `max_scan_depth` to `10`.
- Include config-driven `[agents.<id>]` sections for Claude, Codex, and Copilot.
- Include shared implementation targets such as `.agents` in a separate config-only structure, not as user-facing agents.
- Default agent paths should include Claude `~/.claude/skills`, Codex `~/.codex/skills` and `.codex/skills`, Copilot `~/.copilot/skills` and `.copilot/skills`, plus a config-only `.agents` shared target referenced by Codex and Copilot.
- Each agent definition should support `display_name`, `global_dir`, `project_dir`, `enabled`, and any config-only shared target references needed for effective availability.
- Include `[preferences]` with `default_connection = "symlink"` and `confirm_physical_delete = true`.
- Expand user-relative paths such as `~` without adding an extra dependency unless an ADR approves it.
- Validate required config shape before scan or mutation commands run.
- Make config parse errors actionable by showing the file path and failing key.
- Ensure `config init` does not overwrite an existing config unless the user explicitly confirms or passes a future force flag.
- Provided defaults must be safe to inspect immediately. Missing default directories are warnings for `doctor`, not a reason to generate empty values.

## Technical implementation notes

Implement configuration in `src/config.rs`. Keep path resolution separate from deserialization so tests can parse TOML without touching the real user config directory.

Use `directories::ProjectDirs` or equivalent from the `directories` crate to resolve config locations. For `~` expansion inside configured paths, use the home directory available from the same crate or standard environment APIs through a small helper. Do not introduce `shellexpand` unless the dependency policy is revisited.

Represent agents as a map keyed by stable string IDs rather than a hardcoded enum. This keeps future agent support mostly config-driven.

The default config should include Codex, Claude, and Copilot as product-facing agents. If a shared technical target such as `.agents` is needed, keep it under a config-only shared target section and map it into effective Codex/Copilot availability later in inventory resolution. It should not appear as an agent, command target, or product-facing table column.

### Example config

```toml
[skills]
central_dir = "~/skills"
scan_parent_dirs = []
max_scan_depth = 10

[agents.claude]
display_name = "Claude"
global_dir = "~/.claude/skills"
enabled = true
shared_target_ids = []

[agents.codex]
display_name = "Codex"
global_dir = "~/.codex/skills"
project_dir = ".codex/skills"
enabled = true
shared_target_ids = ["agents"]

[agents.copilot]
display_name = "Copilot"
global_dir = "~/.copilot/skills"
project_dir = ".copilot/skills"
enabled = true
shared_target_ids = ["agents"]

[shared_targets.agents]
# Config-only implementation target used by agents that reference this id.
# This must not appear as a product-facing agent or command target.
display_name = ".agents"
project_dir = ".agents"
enabled = true

[preferences]
default_connection = "symlink"
confirm_physical_delete = true
```

## Success criteria

- A fresh machine can create a readable config file with provided defaults.
- `config show` prints resolved source roots and agent targets without mutating skill directories.
- Invalid TOML produces a clear error message.
- Tests cover default config generation, config parsing, path expansion, and agent map parsing.

## Edge cases

- Config file exists but is missing required sections.
- `central_dir` is absent or empty.
- Agent target paths use defaults that do not exist yet.
- The config directory itself does not exist before `config init`.
- A path contains `~` in the middle of a string, which should not be expanded as a home prefix.

## Dependencies

- Roadmap item `001`.
- Enables roadmap items `003`, `004`, `005`, `006`, and `007`.

## Decisions

- `config init` writes configuration only; `doctor` reports missing target directories.
- Missing target directories are warnings because import creates them when needed.
