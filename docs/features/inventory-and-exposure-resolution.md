# Inventory And Exposure Resolution

Inventory resolution shows which discovered skills are available to each configured agent and how each exposure is connected.

Run:

```bash
skills-manager list
```

The command requires a config file. If no config exists, it prints a `skills-manager config init` hint. With config present, inventory is rebuilt from real filesystem state every time the command runs.

## Data Sources

Inventory combines:

- configured source roots from `[skills]`, used to build the source catalog
- read-only scan results for directories containing `SKILL.md`, used to identify sources and managed repositories
- enabled configured agent target directories
- enabled shared target directories referenced by those agents
- fixed project-local target directories inside scanned Git repositories
- symlinks and physical directories found in those target paths

All source and global target paths are resolved from validated global configuration. The launch directory does not contribute inventory rows or targets. Project-local targets come only from repositories represented by configured scan results, using fixed conventions:

- `.claude/skills` exposes skills to Claude
- `.codex/skills` exposes skills to Codex
- `.copilot/skills` exposes skills to Copilot
- `.agents/skills` exposes skills to Codex and Copilot

There is no persistent inventory database.

## Output

The list output renders only skills with at least one real agent exposure. Source-only scan results remain visible through `skills-manager scan` and do not create list rows.

Columns include:

- `SKILL`: display namespace, usually `repo-name/skill-name`
- `SOURCE`: source repository name, or `unknown`
- `CLAUDE`, `CODEX`, `COPILOT`: effective availability markers
- `SCOPE`: `global` or `project-local`
- `CONNECTION`: `symlink`, `physical`, `missing`, or `unknown`

Unknown Git provenance is valid inventory state and renders as `unknown`.

## TUI Unified Browse View

The standalone `skills-manager list` and `skills-manager scan` commands keep their separate output contracts. The interactive TUI instead uses `/list` as one browser over both data sets. It refreshes the source catalog and inventory from the same scan, then offers Full, Only exposed, and Only discovered not applied views.

Full retains every inventory row and adds each discovered source whose canonical source path has no real exposure. Discovery-only rows do not become inventory rows: their agent and scope columns are `-`, their connection is `not exposed`, and they are importable but cannot be checked for batch changes or removed. Global and project-local inventory rows remain distinct even when they resolve to the same source.

## Effective Availability

Claude, Codex, and Copilot are the product-facing agent columns. Config-only shared targets such as `.agents` are never rendered as an agent column or command target.

When Codex or Copilot references the shared `.agents` target in config, skills found in global `~/.agents/skills` contribute to that agent's effective availability. If both agents reference the same shared target, a single skill exposure can mark both `CODEX` and `COPILOT`.

Legacy `project_dir` values still parse but are ignored. Project-local inventory is derived from scanned repositories and fixed conventions, not those legacy fields.

## Connections And Scope

Inventory distinguishes symlink exposures from physical copies. For symlinks, the link itself is inspected first, and the resolved target is used only to identify source metadata. Later removal flows must be able to remove a link without deleting the source skill.

Configured agent and shared target directories, including `~/.agents/skills`, render as `global`. Fixed targets inside scanned repositories render as `project-local`.

Global and project-local exposures of the same source are separate rows. The same source exposed in two projects also produces two rows. Project-local rows retain their actual source while carrying the containing project path as exposure context. Physical project-local skills use their containing repository as source; symlinks use their resolved source when available.

Project-local rows are read-only. Import, remove, detach, and physical deletion actions are available only for global rows.

## Duplicate Names

Rows with the same display namespace are not collapsed. The output numbers them as `(1)`, `(2)`, and includes source path, origin, or exposure path context in the source column so later import/remove flows can ask the user to choose the intended skill.

Example:

```text
SKILL            SOURCE                          CLAUDE   CODEX    COPILOT  SCOPE     CONNECTION
(1) repo-a/docs  repo-a/skills/docs              -        ✓        -        global          symlink
(2) repo-a/docs  repo-a/skills/docs              -        ✓        ✓        project-local   symlink
```

## Safety

`skills-manager list` is read-only. It does not create, delete, link, clone, import, remove, or persist skill files.

Configuration validation completes before source scanning or target inspection. Relative active paths fail instead of resolving against CWD.
