# Human-readable CLI Workflow

The human-readable CLI workflow gives V1 direct commands for inspecting and changing local skill exposure state while the TUI remains the primary product experience.

Run:

```bash
skills-manager list
skills-manager scan
skills-manager import repo-a/code-review --to claude,codex
skills-manager remove repo-a/code-review --from claude
skills-manager config show
skills-manager doctor
```

## Read-only Commands

`skills-manager scan` reads configured skill source roots and prints discovered `SKILL.md` directories. It does not mutate disk.

`skills-manager list` rebuilds inventory from scan results and configured target directories. It prints effective availability for Claude, Codex, and Copilot, plus scope and connection type.

`skills-manager config show` prints the current TOML config. If the config file is missing, it prints a `config init` hint.

`skills-manager doctor` checks config existence and parsing, source directories, target directory writability, and local Git CLI availability. Output is compact status lines with affected paths so failures are actionable.

## Import Flow

`skills-manager import <skill> --to <agents>` selects a discovered skill from the current scan results. The `--to` value is a comma-separated list of configured agent ids such as `claude,codex`. If `--to` is omitted, the command targets all enabled agents.

Before applying anything, import renders a plan showing the source path, target path, and connection type. It only applies after `Apply this plan? [y/N]` is confirmed. After applying, it rescans and prints the resulting inventory.

V1 imports use symlink exposures. Existing target paths are skipped instead of overwritten.

## Remove Flow

`skills-manager remove <skill> --from <agents>` selects an existing exposure from the current inventory. The `--from` value is a comma-separated list of configured agent ids. If `--from` is omitted, the command removes matching exposures from all agents.

Before applying anything, remove renders a plan. Symlink plans detach the link without deleting source content. Physical-copy plans warn that the target directory will be permanently deleted. Physical-copy deletion requires the normal `Apply this plan? [y/N]` confirmation and a second exact `yes` confirmation before applying.

After applying, remove rescans and prints the resulting inventory.

## Ambiguous Identifiers

Skill identifiers can be exact, such as `repo-a/code-review`, or name-only, such as `code-review`. If a name-only identifier matches multiple skills, interactive mutation commands print numbered options such as `(1)` and `(2)` with path or origin context.

Non-interactive ambiguous mutation commands fail instead of guessing. Users should rerun with a more specific identifier.

## V1 Boundaries

The CLI is intentionally human-readable. V1 does not provide stable JSON output, remote Git import, arbitrary project targeting, or mutation without confirmation.
