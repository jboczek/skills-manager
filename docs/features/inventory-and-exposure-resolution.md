# Inventory And Exposure Resolution

Inventory resolution shows which discovered skills are available to each configured agent and how each exposure is connected.

Run:

```bash
skills-manager list
```

The command requires a config file. If no config exists, it prints a `skills-manager config init` hint. With config present, inventory is rebuilt from real filesystem state every time the command runs.

## Data Sources

Inventory combines:

- configured source roots from `[skills]`
- read-only scan results for directories containing `SKILL.md`
- enabled configured agent target directories
- enabled shared target directories referenced by those agents
- symlinks and physical directories found in those target paths
- current project-local target paths

There is no persistent inventory database in V1.

## Output

The list output renders one row per logical skill where possible.

Columns include:

- `SKILL`: display namespace, usually `repo-name/skill-name`
- `SOURCE`: source repository name, or `unknown`
- `CLAUDE`, `CODEX`, `COPILOT`: effective availability markers
- `SCOPE`: `global`, `local`, or `unknown`
- `CONNECTION`: `symlink`, `physical`, `missing`, or `unknown`

Unknown Git provenance is valid inventory state and renders as `unknown`.

## Effective Availability

Claude, Codex, and Copilot are the product-facing agent columns. Config-only shared targets such as `.agents` are never rendered as an agent column or command target.

When Codex or Copilot references the shared `.agents` target in config, skills found in `.agents` contribute to that agent's effective availability. If both agents reference the same shared target, a single skill exposure can mark both `CODEX` and `COPILOT`.

## Connections And Scope

Inventory distinguishes symlink exposures from physical copies. For symlinks, the link itself is inspected first, and the resolved target is used only to identify source metadata. Later removal flows must be able to remove a link without deleting the source skill.

Global target directories render as `global`; project target directories and shared project targets render as `local`.

## Duplicate Names

Rows with the same display namespace are not collapsed. The output numbers them as `(1)`, `(2)`, and includes source path, origin, or exposure path context in the source column so later import/remove flows can ask the user to choose the intended skill.

Example:

```text
SKILL            SOURCE                          CLAUDE   CODEX    COPILOT  SCOPE     CONNECTION
(1) repo-a/docs  repo-a (/Users/me/repo-a-one)   -        ✓        -        local     symlink
(2) repo-a/docs  repo-a (/Users/me/repo-a-two)   -        -        ✓        local     symlink
```

## Safety

`skills-manager list` is read-only. It does not create, delete, link, clone, import, remove, or persist skill files.
