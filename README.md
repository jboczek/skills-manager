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

PRDs live under `docs/prds/`. Durable feature documentation lives under `docs/features/`.

## Local verification

```bash
cargo fmt --check
cargo check --locked
cargo test --locked
cargo clippy --all-targets --all-features -- -D warnings
```
