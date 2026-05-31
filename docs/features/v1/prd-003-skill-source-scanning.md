---
title: Skill source scanning
summary: Discover skills from configured source locations by finding SKILL.md files without mutating the filesystem.
status: planned
roadmap: v1
---

# Skill source scanning

## Context

Skills Manager imports skills from repositories the user already has locally in V1. A skill is recognized by a `SKILL.md` file, and a repository can contain one skill, multiple skills, or nested skill directories. The app must scan configured source locations and present candidates before any import plan is created.

## Problem

Manual skill management breaks down when users cannot tell which skills exist inside a repository or which repository they came from. A naive recursive scan can also be slow, noisy, or unsafe if it follows symlinks or scans too broadly.

## Goal

Build a bounded, read-only scanner that finds candidate skills in configured source roots, preserves useful hierarchy, resolves repository origin when possible, and produces normalized scan results for CLI, inventory, and import flows.

## Non-goals

- Do not clone remote repositories in V1.
- Do not scan the entire home directory unless the user explicitly configured it.
- Do not follow symlinks during source scanning.
- Do not persist scan results as a database.
- Do not validate full skill metadata beyond the presence of `SKILL.md`.

## Proposed experience

A user runs:

```bash
skills-manager scan
```

The command scans configured `central_dir` and `scan_parent_dirs`, then prints discovered skills with namespace, path, repository name, and remote origin when known. The command never creates, deletes, or links files.

## Requirements

- Scan `central_dir` first, then each configured `scan_parent_dirs` entry.
- Use the `ignore` crate for directory walking.
- Respect `max_scan_depth`, defaulting to `10`.
- Detect a skill when a directory contains `SKILL.md`.
- Treat the skill root as the directory containing `SKILL.md`.
- Preserve nested hierarchy in scan results so users can distinguish multi-skill repositories.
- Resolve the repository root as the closest ancestor containing `.git`.
- Resolve repository origin through `git -C <repo_path> remote get-url origin` using `std::process::Command`.
- Render unknown origin as `unknown`.
- Preserve enough path and origin context for later numbered disambiguation when two scan results have the same display namespace.
- Do not follow symlinks.
- Deduplicate identical skill roots if they are reached from overlapping configured scan paths.
- Return structured scan results that can be reused by import and inventory logic.

## Technical implementation notes

Implement filesystem traversal in `src/scanner.rs`. Implement Git origin resolution in `src/git.rs`. Keep `git.rs` small and command-based; do not use `git2` in V1.

The scanner should produce a data structure containing at least:

- `skill_id`, using `repo-name/skill-name` where possible as a display namespace,
- `skill_path`,
- `skill_relative_path` inside the repository when nested,
- `repo_name`,
- `repo_path`,
- `remote_url`,
- `source_kind` or equivalent marker showing whether the skill came from `central_dir` or a scanned parent.
- a stable per-result selector or ordering key that lets CLI/TUI render duplicate namespaces as `(1)`, `(2)`, and so on.

The scanner should be callable from tests with explicit config values and temporary directories. Avoid reading the real user config in scanner unit tests.

## Success criteria

- `skills-manager scan` finds single-skill, multi-skill, and nested-skill repositories.
- Scan results include origin when `git remote origin` exists.
- Unknown origin is represented clearly and does not fail the whole scan.
- Tests prove symlinked directories are not followed.
- Tests prove overlapping scan roots do not duplicate the same skill root.

## Edge cases

- A skill directory is inside a repository with no `origin` remote.
- A `SKILL.md` exists outside any Git repository.
- Two repositories have the same repository folder name and same skill name; scan output keeps both and lets later flows present numbered choices.
- Permission errors occur while walking a configured directory.
- A configured scan directory does not exist.

## Dependencies

- Roadmap items `001` and `002`.
- Feeds roadmap items `004`, `005`, `006`, and `007`.

## Open questions

- Should permission errors fail the scan or be collected as warnings? V1 should likely continue scanning and show warnings.
- How much source path should scan output show by default before it becomes noisy?
