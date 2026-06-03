# Skills Manager

Skills Manager is a terminal-first tool for discovering and managing local agent skills. V1 is focused on explicit local configuration, read-only source scanning, and safe exposure workflows.

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

## Local verification

```bash
cargo fmt --check
cargo check --locked
cargo test --locked
cargo clippy --all-targets --all-features -- -D warnings
```
