# Assistant-style TUI Shell

The assistant-style TUI shell is the default interactive Skills Manager experience. Running `skills-manager` without a subcommand opens a full-screen terminal interface for inspecting inventory, scanning source roots, importing skills, removing exposures, viewing config, and reading help.

## Launch

Run:

```bash
skills-manager
```

In an interactive terminal, the app enters a ratatui/crossterm alternate-screen UI. In non-interactive contexts, it exits successfully instead of trying to take over stdin/stdout.

## Layout

The shell uses fixed regions:

- header with app name and purpose
- status feed with loaded skill and agent counts
- main content panel for the current mode
- sticky prompt labeled `Skills`
- footer with mode-specific shortcuts

The prompt remains visible across modes. It does not inspect or display the launch directory or Git branch. Pressing `/` from an empty prompt opens command suggestions with short descriptions.

## Commands

Supported prompt commands:

```text
list
/list
scan
/scan
config
/config
help
/help
quit
/quit
q
```

Unknown commands keep the user in the shell and show an error message with a help hint.

The slash suggestion menu lists `/list`, `/scan`, `/config`, `/help`, and `/quit`. Import and remove are table actions in the TUI. If a user types `/import` or `/remove`, the TUI guides them to the row shortcuts instead of starting standalone prompt workflows.

## Modes

Home mode shows loaded skill and enabled-agent counts with suggested commands.

List mode refreshes inventory from the shared configured context and opens with one collapsed group per repository. Global rows are grouped by source repository; project-local rows are grouped by their containing project. Expanding a group reveals skill children whose source column uses the resolved source path, shortened to a privacy-safe suffix when needed. Project physical skills show paths such as `.agents/skills/adx-intake`; project symlinks show their resolved source. Child rows retain the Claude, Codex, Copilot, scope, and connection details. Duplicate display identities keep numbered labels such as `(1)` and `(2)`. The first column reserves 30 cells for source and skill labels.

Scan mode runs the shared scanner and groups the complete source catalog by source repository. Its first column reserves 35 cells for source and skill labels. Global list rows and scan rows use the same source-repository identity; project-local list rows use their containing project identity.

Repository-backed groups use the canonical repository root as identity. Repositories with the same folder name remain separate and receive distinguishing safe path context. Unresolved sources use their privacy-safe source container as identity, so unrelated `unknown` rows are not merged. Display labels omit standard home-directory prefixes and user names.

Config mode shows the resolved config path and normalized TOML. Compatibility diagnostics for ignored legacy fields appear in the status feed. Rich config editing remains outside the current scope.

Help mode lists prompt commands and key behavior.

Import mode is reached from table shortcuts or existing flow state. It guides the user through ambiguity resolution, target-agent selection, staged plan preview, confirmation, apply, rescan, and result rendering.

Remove mode is reached from table shortcuts or existing flow state. It guides the user through exposure selection when needed, staged plan preview, confirmation, apply, rescan, and result rendering.

## Key Behavior

The implemented V1 key behavior is:

- `Enter`: submit prompt input or confirm the current guided step
- `Esc`: cancel the current action and return home
- `?`: open help when the prompt is empty
- `q`: quit when the prompt is empty
- `Up` / `Down`: move through command suggestions or visible source/skill rows
- `Right`: expand a collapsed source group, or select the first child of an expanded group
- `Left`: select a skill's parent source group, or collapse an expanded group
- `/`: open slash command suggestions from an empty prompt
- `i`: import the selected scan skill, or import missing enabled-agent exposures from a selected global list skill
- `x`: remove a selected global list skill exposure, or choose which exposure to remove when a row has multiple
- `r`: refresh the active list or scan table
- `Ctrl-C`: quit

All source groups start collapsed. Refresh preserves expansion for source identities that still exist, and resize events keep the selected visible row within the viewport. Pressing `i` or `x` on a group row asks the user to select a skill inside the group.

Tab, panel switching, and multi-cell exposure toggling are reserved for future modes with multiple active sections or editable table columns.

## Safety

Import and remove shortcuts use the same shared staged plan and apply modules as the CLI. Filesystem changes are not applied until the user confirms the rendered plan.

Imports create symlink exposures for selected enabled agents and skip existing target paths rather than overwriting them.

Removals detach symlinks without deleting source skills. Physical-copy removals are visually distinguished as destructive and require a second exact `yes` confirmation before deletion.

Project-local list rows are read-only. Import and remove shortcuts report the restriction without entering a mutation flow or building a change plan.

After a plan is applied, the TUI refreshes inventory and renders the resulting state.

The TUI resolves and validates the same global context as the CLI before scanning, inventory construction, or plan creation.

## Boundaries

This shell does not include advanced table cell toggling, space-to-stage exposure edits, rich config editing, remote Git imports, arbitrary project targeting, or stable machine-readable output.
