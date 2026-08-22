# Skills Manager

Skills Manager is a terminal-first tool for discovering skill sources and managing agent exposures. Scanning, inventory, diagnostics, and mutation plans are independent of the directory and Git branch from which the application is launched.

## Assistant-style TUI

Run without subcommands to open the full-screen assistant-style terminal UI:

```bash
skills-manager
```

The TUI shows a header, status feed, main content area, sticky prompt, and footer hints. `/list` is the sole skill browser: it opens a collapsed, privacy-safe source-group view that combines real exposures with discovered-but-unexposed skills in the existing seven columns. Press `/` from an empty prompt to open command suggestions, or type plain commands such as `list` and `help`.

Core prompt commands:

- `/list`: refresh and show the Full unified view.
- `/source add <git-url>`: add or reuse a managed Git source, then optionally expose one discovered skill.
- `/config`: show the config path and current TOML.
- `/help`: show commands and key hints.
- `/quit`: exit.

Key behavior:

- `Enter` submits the prompt or confirms the current guided step.
- `Esc` cancels the current action and returns home.
- `/` opens command suggestions with descriptions.
- `?` opens help when the prompt is empty.
- `q` quits when the prompt is empty.
- `Up` / `Down` move through command suggestions or currently visible source/skill rows.
- `Right` expands a source group, then moves from an expanded group to its first skill.
- `Left` moves from a skill to its source group, then collapses the group.
- `Tab` cycles Full, Only exposed, and Only discovered not applied while the prompt is empty.
- `i` imports a selected discovered skill, or imports missing enabled-agent exposures from a selected global list skill.
- `x` removes a selected global list skill exposure or prompts for which exposure to remove.
- `r` refreshes the current list while retaining the active view and matching selection/group expansion.
- `Cmd+U` reviews missing commits and updates the selected repository after confirmation.

Import and remove actions run only from expanded skill rows; group rows never imply a bulk action. Discovery-only rows show `-` for agents and scope and `not exposed` for connection; they can be imported with `i`, but cannot be checked or removed. Typed `/import` and `/remove` commands guide users to these shortcuts instead of starting standalone prompt workflows. Table shortcuts use the same staged plan behavior as the CLI, show the plan before applying, rescan after apply, and require exact `yes` confirmation before deleting physical copies.

List groups represent repositories. Global rows are grouped by source repository, while project-local rows are grouped by their containing project. Scope remains visible per row. Project-local rows are read-only, so `i` and `x` report that they cannot mutate those rows.

When a global repository has commits on its origin that are not in the local checkout, `/list` checks for them in the background and shows a green one-line repository-update notice from the `CLAUDE` column. Failed checks stay out of the interface. `Cmd+U` opens the missing commit subjects; entering `y` runs a fast-forward-only `git pull` and refreshes the list.

## Configuration

Create the default config:

```bash
skills-manager config init
```

Inspect the config path or current TOML:

```bash
skills-manager config path
skills-manager config show
```

The scan source settings live under `[skills]`:

- `central_dir`: scanned first.
- `scan_parent_dirs`: scanned after `central_dir`.
- `max_scan_depth`: bounded scan depth, defaulting to `10`.

Managed source and target paths must be absolute or begin with `~`. Leading tildes are expanded before validation. Relative active paths are rejected before scanning, inventory construction, or plan creation.

Legacy `project_dir` fields still parse for upgrade compatibility, but they are ignored and omitted from generated or normalized config.

## Skill Scanning

Run a read-only source scan:

```bash
skills-manager scan
```

The scan command finds directories containing `SKILL.md` in configured source roots. It does not create, delete, link, clone, or import files. It skips symlinked directories, deduplicates overlapping scan roots, and prints each discovered skill with:

- skill namespace
- source marker (`[central]` or `[scan]`)
- skill path
- repository name, or `unknown`
- remote origin URL, or `unknown`

Example:

```text
skills/code-review  [central]  /Users/me/skills/code-review  skills  git@github.com:me/skills.git
```

## Skill Inventory

Show the current effective skill availability:

```bash
skills-manager list
```

The list command rebuilds inventory from configured global targets and fixed project-local targets inside repositories found by the configured source scan. If no config exists, it prints a `config init` hint instead of guessing target paths. Scan results without an agent exposure remain in `scan` output and do not create list rows.

Inventory output includes:

- skill namespace
- actual source path, resolving symlinks when possible
- effective availability for Claude, Codex, and Copilot
- scope: `global` or `project-local`
- connection: `symlink`, `physical`, `missing`, or `unknown`

Skills exposed through shared implementation targets such as `~/.agents/skills` are shown as effective Codex or Copilot availability according to config. `.agents` is not rendered as an agent column or command target.

For every Git repository represented by configured scan results, Skills Manager checks fixed project-local conventions:

- `.claude/skills` for Claude
- `.codex/skills` for Codex
- `.copilot/skills` for Copilot
- `.agents/skills` for Codex and Copilot

These rows are grouped under project context such as `analystloop · pgit/analystloop`, render as `project-local`, and are read-only. The same source exposed globally and in a project is shown as separate rows. The same source exposed in two projects is also shown separately.

When multiple rows share the same display namespace, the list output numbers them as `(1)`, `(2)`, and includes source path or origin context so the intended skill can be identified.

Example:

```text
SKILL                     SOURCE                                  CLAUDE   CODEX    COPILOT  SCOPE          CONNECTION
skills/code-review        /Users/me/skills/code-review            -        ✓        ✓        global         symlink
analystloop/adx-intake    /Users/me/pgit/analystloop/.agents/...  -        ✓        ✓        project-local  physical
```

PRDs live under `docs/prds/`. Durable feature documentation lives under `docs/features/`.

## Managed Git Sources

Add a Git repository to the configured global `central_dir`:

```bash
skills-manager source add https://github.com/example/skills.git
```

The CLI and `/source add <git-url>` TUI flow show the URL and derived destination before mutation and require confirmation. The destination is `central_dir/<repository-name>`, with a trailing `.git` removed from the repository name.

New repositories are cloned to a unique temporary directory beneath `central_dir` with submodule recursion disabled. Skills Manager scans the temporary clone with the existing bounded, non-symlink-following scanner. A repository with no `SKILL.md` is removed instead of promoted. A valid clone is renamed to its final destination only after the scan succeeds.

An existing destination is reused only when its readable canonical `origin` matches the requested URL. Reuse scans the local checkout as-is and never fetches, pulls, resets, or checks out content. Files, symlinks, non-Git directories, missing origins, and different origins are rejected without overwrite or suffix guessing.

After acquisition, selecting skills is optional. The CLI accepts comma-separated skill numbers; the TUI accepts one skill number. Selected skills enter the existing staged exposure workflow. Declining exposure keeps the managed source and creates no agent targets.

Skills Manager stores no credentials. Git uses the user's existing SSH configuration, credential helpers, and other normal Git authentication.

## V2 status

V2 is specified as three ordered slices:

1. [Global execution context](docs/prds/v2/prd-008-global-execution-context.md): completed; removes launch-directory dependence while reporting read-only project-local exposures inside configured source repositories.
2. [Git URL import into the managed source directory](docs/prds/v2/prd-009-git-url-import-into-managed-source-directory.md): completed; safely acquires or reuses a managed source before optional staged exposure.
3. [Manual install migration](docs/prds/v2/prd-010-manual-install-migration.md): conservatively convert physical global installs into managed sources and symlink exposures.

The remaining V2 migration slice is planned.

## Importing And Removing Skills

Mutating CLI commands always print a change plan before applying filesystem changes:

```bash
skills-manager import repo-a/code-review --to claude,codex
skills-manager remove repo-a/code-review --from claude
```

`import` scans configured source roots, selects the requested discovered skill, prepares symlink exposures for the requested agents, asks `Apply this plan? [y/N]`, applies only after confirmation, then rescans and prints the resulting inventory.

`remove` reads current inventory, selects the requested exposed skill, and refuses project-local rows as read-only. For global rows it prepares a removal plan, asks `Apply this plan? [y/N]`, applies only after confirmation, then rescans and prints the resulting inventory. Symlink removals detach the link without deleting the source skill. Physical-copy removals print a permanent-delete warning and require a second exact `yes` confirmation.

If a skill identifier is ambiguous, interactive commands print numbered options with path or origin context and ask which one to use. Non-interactive ambiguous mutation commands fail instead of guessing.

## Diagnostics

Check local setup:

```bash
skills-manager doctor
```

`doctor` validates the global execution context, checks configured source directories, checks global agent and shared target directories for writability when they exist, and verifies that the local `git` CLI is available for origin detection. It prints `PASS`, `WARN`, or `FAIL` status lines with affected paths.

## Local verification

```bash
cargo fmt --check
cargo check --locked
cargo test --locked
cargo clippy --all-targets --all-features -- -D warnings
```
