# Skill Source Scanning

Skill source scanning discovers local skill candidates before any import or exposure plan is created.

Run:

```bash
skills-manager scan
```

The command reads configured source roots from `[skills]` in `config.toml`. It scans `central_dir` first, then each `scan_parent_dirs` entry, bounded by `max_scan_depth`.

Every source path must be absolute after leading-tilde expansion. Relative paths are rejected before filesystem discovery and are never resolved against the shell's current directory.

## What Counts As A Skill

A skill is any directory containing a `SKILL.md` file. The skill root is the directory that directly contains that file.

Repositories can contain:

- one skill at the repository root
- several sibling skill directories
- nested skill directories

The scanner preserves nested paths in structured results so later import and inventory flows can distinguish candidates that share the same display namespace.

## Output

Scan output is human-readable and includes:

- skill namespace
- source marker: `[central]` or `[scan]`
- skill path
- repository name, or `unknown`
- remote origin URL, or `unknown`

When two scan results have the same display namespace, the command numbers them as `(1)`, `(2)`, and so on.

## Safety

Scanning is read-only. It never creates, deletes, links, clones, imports, or persists skill files.

The scanner does not follow symlinked directories. If configured roots overlap, identical skill roots are shown once. Missing source directories are skipped. Scan or origin lookup errors are collected as warnings so one bad path does not stop the whole scan.

Launching the same configuration from unrelated directories produces the same scan results. CWD does not define a scan root. Project-local `.claude/skills`, `.codex/skills`, `.copilot/skills`, and `.agents/skills` appear in the source catalog only when their containing repository is beneath a configured source root.

The scan catalog and exposure inventory are separate. A scanned skill can exist without appearing in `skills-manager list`; list includes it only when at least one global or project-local agent target exposes it.

## Repository Metadata

For each discovered skill, the scanner resolves the closest ancestor containing `.git` as the repository root. It reads the repository origin with:

```bash
git -C <repo_path> remote get-url origin
```

Repositories without an origin are valid scan results and show `unknown` for the origin.
