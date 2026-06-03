# Skill Source Scanning

Skill source scanning discovers local skill candidates before any import or exposure plan is created.

Run:

```bash
skills-manager scan
```

The command reads configured source roots from `[skills]` in `config.toml`. It scans `central_dir` first, then each `scan_parent_dirs` entry, bounded by `max_scan_depth`.

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

## Repository Metadata

For each discovered skill, the scanner resolves the closest ancestor containing `.git` as the repository root. It reads the repository origin with:

```bash
git -C <repo_path> remote get-url origin
```

Repositories without an origin are valid scan results and show `unknown` for the origin.
