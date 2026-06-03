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
- sticky prompt with current directory and Git branch when available
- footer with mode-specific shortcuts

The prompt remains visible across modes. Pressing `/` from an empty prompt opens command suggestions with short descriptions.

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

List mode refreshes inventory from the shared inventory service and renders rows with skill, source, Claude, Codex, Copilot, scope, and connection columns. Duplicate display identities include numbered labels such as `(1)` and `(2)` with path, origin, or exposure context. Selection starts at the first row when rows exist, and the viewport scrolls only after selection moves past the visible top or bottom.

Scan mode runs the shared scanner and shows discovered skills with source type, repository, and origin context. Scan rows use the same selection and scrolling model as list rows.

Config mode shows the resolved config path and current TOML. Rich config editing remains outside V1.

Help mode lists prompt commands and key behavior.

Import mode is reached from table shortcuts or existing flow state. It guides the user through ambiguity resolution, target-agent selection, staged plan preview, confirmation, apply, rescan, and result rendering.

Remove mode is reached from table shortcuts or existing flow state. It guides the user through exposure selection when needed, staged plan preview, confirmation, apply, rescan, and result rendering.

## Key Behavior

The implemented V1 key behavior is:

- `Enter`: submit prompt input or confirm the current guided step
- `Esc`: cancel the current action and return home
- `?`: open help when the prompt is empty
- `q`: quit when the prompt is empty
- `Up` / `Down`: move through command suggestions, list rows, or scan rows
- `/`: open slash command suggestions from an empty prompt
- `i`: import the selected scan row, or import missing enabled-agent exposures from the selected list row
- `x`: remove the selected list exposure, or choose which exposure to remove when a row has multiple
- `r`: refresh the active list or scan table
- `Ctrl-C`: quit

Tab, left/right panel switching, and multi-cell exposure toggling are reserved for future modes with multiple active sections or editable table columns.

## Safety

Import and remove shortcuts use the same shared staged plan and apply modules as the CLI. Filesystem changes are not applied until the user confirms the rendered plan.

Imports create symlink exposures for selected enabled agents and skip existing target paths rather than overwriting them.

Removals detach symlinks without deleting source skills. Physical-copy removals are visually distinguished as destructive and require a second exact `yes` confirmation before deletion.

After a plan is applied, the TUI refreshes inventory and renders the resulting state.

## Boundaries

This shell does not include advanced table cell toggling, space-to-stage exposure edits, rich config editing, remote Git imports, arbitrary project targeting, or stable machine-readable output.
