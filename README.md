# Skills Manager

Skills Manager is a terminal-first tool for discovering and managing local agent skills. V1 is focused on explicit local configuration, read-only source scanning, and safe exposure workflows.

## Assistant-style TUI

Run without subcommands to open the full-screen assistant-style terminal UI:

```bash
skills-manager
```

The TUI shows a header, status feed, main content area, sticky prompt, and footer hints. List and scan open as collapsed source-group overviews, with privacy-safe repository-relative or shortened child paths and extra width for skill names. Press `/` from an empty prompt to open command suggestions, or type plain commands such as `list` and `help`.

Core prompt commands:

- `/list`: refresh and show current inventory.
- `/scan`: scan configured source roots.
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
- `i` imports the selected scan skill, or imports missing enabled-agent exposures from the selected list skill.
- `x` removes the selected list skill exposure or prompts for which exposure to remove.
- `r` refreshes the active list or scan table.

Import and remove actions run only from expanded skill rows; group rows never imply a bulk action. Typed `/import` and `/remove` commands guide users to these shortcuts instead of starting standalone prompt workflows. Table shortcuts use the same staged plan behavior as the CLI, show the plan before applying, rescan after apply, and require exact `yes` confirmation before deleting physical copies.

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

The list command rebuilds inventory from the configured source scan roots and configured agent target directories each time it runs. If no config exists, it prints a `config init` hint instead of guessing target paths.

Inventory output includes:

- skill namespace
- source repository, or `unknown`
- effective availability for Claude, Codex, and Copilot
- scope: `global`, `local`, or `unknown`
- connection: `symlink`, `physical`, `missing`, or `unknown`

Skills exposed through shared implementation targets such as `.agents` are shown as effective Codex or Copilot availability according to config. `.agents` is not rendered as an agent column or command target.

When multiple rows share the same display namespace, the list output numbers them as `(1)`, `(2)`, and includes source path or origin context so the intended skill can be identified.

Example:

```text
SKILL                     SOURCE                          CLAUDE   CODEX    COPILOT  SCOPE     CONNECTION
skills/code-review        skills                          -        ✓        ✓        local     symlink
(1) repo-a/docs           repo-a (/Users/me/repo-a-one)   -        ✓        -        local     symlink
(2) repo-a/docs           repo-a (/Users/me/repo-a-two)   -        -        ✓        local     symlink
```

PRDs live under `docs/prds/`. Durable feature documentation lives under `docs/features/`.

## Importing And Removing Skills

Mutating CLI commands always print a change plan before applying filesystem changes:

```bash
skills-manager import repo-a/code-review --to claude,codex
skills-manager remove repo-a/code-review --from claude
```

`import` scans configured source roots, selects the requested discovered skill, prepares symlink exposures for the requested agents, asks `Apply this plan? [y/N]`, applies only after confirmation, then rescans and prints the resulting inventory.

`remove` reads current inventory, selects the requested exposed skill, prepares a removal plan, asks `Apply this plan? [y/N]`, applies only after confirmation, then rescans and prints the resulting inventory. Symlink removals detach the link without deleting the source skill. Physical-copy removals print a permanent-delete warning and require a second exact `yes` confirmation.

If a skill identifier is ambiguous, interactive commands print numbered options with path or origin context and ask which one to use. Non-interactive ambiguous mutation commands fail instead of guessing.

## Diagnostics

Check local setup:

```bash
skills-manager doctor
```

`doctor` checks whether config exists and parses, configured source directories are present, configured agent target directories are writable when they exist, and the local `git` CLI is available for origin detection. It prints `PASS`, `WARN`, or `FAIL` status lines with affected paths.

## Local verification

```bash
cargo fmt --check
cargo check --locked
cargo test --locked
cargo clippy --all-targets --all-features -- -D warnings
```
