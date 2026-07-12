# Assistant-style TUI Shell

The assistant-style TUI shell is the default interactive Skills Manager experience. Running `skills-manager` without a subcommand opens a full-screen terminal interface for browsing unified skill state, importing skills, removing exposures, viewing config, and reading help.

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
config
/config
help
/help
quit
/quit
q
```

Unknown commands keep the user in the shell and show an error message with a help hint.

The slash suggestion menu lists `/list`, `/source_add`, `/config`, `/help`, and `/quit`. Import and remove are table actions in the TUI. If a user types `/import` or `/remove`, the TUI guides them to the row shortcuts instead of starting standalone prompt workflows.

## Modes

Home mode shows loaded skill and enabled-agent counts with suggested commands.

List mode is the sole TUI browser. `/list` refreshes one source catalog and its exposure inventory, then opens Full with one collapsed group per repository. Full contains every real global or project-local inventory row plus each discovered source skill with no real exposure. A discovered source is de-duplicated only by canonical source identity; exposure scope and agent availability are never inferred from its path.

Global exposure rows are grouped by source repository; project-local rows are grouped by their containing project. Expanding a group reveals privacy-safe source paths. A discovery-only row retains the same grouping and seven-column layout, but renders `-` for Claude, Codex, Copilot, and scope, and `not exposed` for connection. Tab cycles Full, Only exposed, and Only discovered not applied. The existing list column sizes and duplicate labels remain unchanged.

Repository-backed groups use the canonical repository root as identity. Repositories with the same folder name remain separate and receive distinguishing safe path context. Unresolved sources use their privacy-safe source container as identity, so unrelated `unknown` rows are not merged. Display labels omit standard home-directory prefixes and user names.

Config mode shows the resolved config path and normalized TOML. Ignored legacy fields are omitted from the normalized output. Rich config editing remains outside the current scope.

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
- `Tab`: cycle Full, Only exposed, and Only discovered not applied when the prompt is empty and the command menu is closed
- `i`: import the selected discovery skill, or import missing enabled-agent exposures from a selected global list skill
- `x`: remove a selected global list skill exposure, or choose which exposure to remove when a row has multiple
- `r`: refresh the active list while retaining its filter and matching selection/group expansion
- `Ctrl-C`: quit

All source groups start collapsed. Refresh preserves expansion for source identities that still exist, and resize events keep the selected visible row within the viewport. Pressing `i` or `x` on a group row asks the user to select a skill inside the group.

Space checks only exposed rows for existing batch actions. Discovery-only rows cannot be checked or removed; `i` sends them through the existing staged import flow.

## Safety

Import and remove shortcuts use the same shared staged plan and apply modules as the CLI. Filesystem changes are not applied until the user confirms the rendered plan.

Imports create symlink exposures for selected enabled agents and skip existing target paths rather than overwriting them.

Removals detach symlinks without deleting source skills. Physical-copy removals are visually distinguished as destructive and require a second exact `yes` confirmation before deletion.

Project-local list rows are read-only. Import and remove shortcuts report the restriction without entering a mutation flow or building a change plan.

Discovery-only rows are also protected from removal and batch selection. They expose no implied availability or scope; importing one still uses the normal plan preview and confirmation flow.

After a plan is applied, the TUI refreshes inventory and renders the resulting state.

The TUI resolves and validates the same global context as the CLI before scanning, inventory construction, or plan creation.

## Boundaries

This shell does not include arbitrary table-cell editing, rich config editing, remote Git imports, arbitrary project targeting, or stable machine-readable output.
