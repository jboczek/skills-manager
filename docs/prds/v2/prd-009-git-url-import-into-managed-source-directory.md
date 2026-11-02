---
title: Git URL import into managed source directory
summary: Add Git repositories as managed skill sources, scan them safely, and expose selected skills through the existing staged workflow.
status: in_progress
roadmap: v2
---

# Git URL import into managed source directory

## Context

V1 discovers skills only from repositories already on the user's machine, requiring users to clone elsewhere before returning to scan and expose skills. V2 makes `central_dir` the global managed source library. Adding a repository changes that library; exposing a skill changes agent targets. Those mutations must remain distinct.

## Problem

Users cannot add a source from a Git URL in one inspectable workflow. Reusing `import` would blur source acquisition with skill exposure and make collisions harder to explain.

## Goal

Let users add a Git repository to global `central_dir`, verify it contains skills, then expose selected skills through the existing staged confirmation flow.

## Non-goals

- Updating, pulling, or synchronizing an existing managed source.
- Selecting a branch, tag, commit, or version.
- Storing or managing Git credentials.
- Removing managed sources.
- Initializing or updating submodules.
- Running repository scripts, hooks, build steps, installers, package managers, or skill code.
- Overwriting, renaming, suffixing, or merging destination directories.
- Turning Skills Manager into a package manager.

## User stories

1. As a user, I want `import-repo <address>`, so that source acquisition stays distinct from skill exposure.
2. As a user, I want to preview the URL and destination, so that I understand the mutation.
3. As a user, I want confirmation before cloning, so that pasted input cannot mutate disk immediately.
4. As a user, I want sources under global `central_dir`, so that launch location is irrelevant.
5. As a user, I want predictable repository-name destinations, so that sources remain recognizable.
6. As a user, I want pre-promotion scanning and failure cleanup, so that incomplete or skill-less sources are not managed.
7. As a user, I want a same-origin source reused without pulling, so that local state remains unchanged.
8. As a user, I want different or unknown collisions rejected, so that Skills Manager never guesses.
9. As a user, I want individual skill selection, so that adding a source exposes nothing automatically.
10. As a user, I want selected exposures staged and confirmed, so that V1 safety guarantees remain.
11. As a user, I want existing Git authentication used, so that Skills Manager stores no credentials.

## Proposed experience

The CLI entry point is:

```bash
skills-manager import-repo https://example.com/org/skills.git
```

The CLI requires an address. `import-repo` replaces `source add` without an alias; the existing `import <skill>` command is unchanged. The TUI provides `/import_repo [URL]`: without a URL it opens an editable address form, and with one it opens the preview. `Enter` or `Tab` continues from the command suggestion, `Esc` cancels, and invalid input or an acquisition failure returns to the URL form.

Before mutation, Skills Manager shows the normalized repository URL, destination, such as `~/skills/skills`, and scan scope, then asks for confirmation. A GitHub tree URL such as `https://github.com/jboczek/skills/tree/main/skills` resolves to the repository root. The branch and subdirectory in the pasted URL are ignored; a new clone uses the remote repository's default branch and scans the whole repository.

After confirmation, the repository is cloned to a unique temporary directory beneath `central_dir`, with submodules disabled, then scanned for `SKILL.md`. No skills means cleanup and no source. Valid clones are promoted to the previewed destination before individual skills enter existing agent selection and staged exposure confirmation.

An existing destination is reused and scanned only when its canonical `origin` matches. No fetch or pull occurs. Every other collision fails.

## Requirements

- Add `import-repo <address>` with a required address; remove `source add` without an alias and do not reinterpret skill exposure `import`.
- Add TUI `/import_repo [URL]`; remove `/source_add` without an alias. No argument opens an editable URL form; an argument opens the preview.
- Allow a GitHub `/tree/<branch>/<subdirectory>` URL to identify its repository root. Clone the remote default branch and scan the whole repository, not the copied subdirectory.
- Resolve paths from global `central_dir`, independent of current working directory.
- Derive the destination name from the URL's repository name, stripping a trailing `.git`.
- Preview URL and destination before filesystem mutation.
- Require interactive confirmation; cancellation must leave no new files.
- Keep source-acquisition confirmation separate from staged exposure confirmation. Cancelling exposure keeps the source and creates no exposures.
- Clone to a unique temporary directory under `central_dir`.
- Disable submodule initialization and recursion.
- Scan before promotion with the existing bounded, non-symlink-following scanner.
- Promote a valid clone by renaming it to the final destination only after at least one skill is found.
- Remove temporary content on any failure or when no skills are found.
- Never execute content from the repository.
- Reuse only when readable canonical `origin` matches the requested URL.
- Reuse means scan only: do not fetch, pull, reset, checkout, or modify the existing repository.
- Reusing an existing same-origin source does not update it; repository updates stay in the `/list` `u` flow.
- Fail for non-Git, unreadable-origin, or different-origin destinations.
- Never choose a suffixed destination or offer overwrite.
- Select individual skills and delegate exposures to existing staged confirmation.
- After promotion, exposure cancellation keeps the source and creates no exposures.

## Success criteria

- A Git URL is added to `central_dir/<repo-name>`.
- A GitHub tree URL adds the whole repository from its default branch, regardless of the branch and subdirectory in the URL.
- Declining confirmation creates no destination or temporary clone.
- Repositories without skills are not promoted.
- Multi-skill repositories allow individual selection and do not expose unselected skills.
- Same-origin repositories are scanned without network update.
- Different or unknown collisions fail without changes.
- Every exposure requires existing staged confirmation.

## Edge cases

- URL variants include trailing slash, query fragment, `.git`, or SCP-style SSH syntax.
- The URL yields no safe repository name.
- `central_dir` does not exist yet or is not writable.
- Git, authentication, remote access, scanning, or cleanup fails.
- The destination appears between preview and promotion.
- The destination is a file, symlink, non-Git directory, or repository without `origin`.
- Canonically equivalent HTTPS and SSH forms refer to the same host and repository path.
- A reused repository is dirty, detached, or modified; scan it as-is.

## Dependencies

- PRD 003 skill source scanning.
- PRD 005 human-readable CLI workflow and PRD 007 assistant-style TUI shell.
- PRD 006 staged exposure plans and confirmations.
- PRD 008 global execution context.
- The local Git CLI and the configured global `central_dir`.

## Implementation decisions

- Identity is canonical origin plus repository-name destination; no registry or lockfile is added.
- Canonical comparison normalizes transport syntax, lowercase host, trailing slash, and `.git` while preserving repository path case. Reject embedded HTTP credentials; use existing Git configuration, credential helpers, or SSH.
- Temporary clones use `central_dir` so same-filesystem rename avoids partial destinations.
- Git operations are limited to clone and origin inspection. Repository content is untrusted scan input.
- Acquisition and exposure remain separate services within the guided flow.
- A GitHub tree URL is reduced to its repository root before destination and origin handling; a new clone uses the remote default branch.

## Testing decisions

- Test that `/import_repo [URL]` and required-argument `import-repo <address>` replace the old source-import commands without aliases; keep `import <skill>` unchanged.
- Cover the editable TUI URL form, cancellation, invalid-input correction, acquisition retry, and preview of whole-repository scope for GitHub tree URLs.
- Unit-test destination and canonical URL handling across HTTPS, SSH, SCP-style, suffix, invalid-name, and credential cases.
- Prove preview and confirmation precede directory creation or Git invocation.
- Use temporary Git repositories for promotion, multiple skills, no skills, failure, and cleanup tests.
- Test collision behavior for same origin, different origin, missing origin, non-Git directory, file, and symlink destinations.
- Record Git commands to prove submodules are disabled and reuse performs no fetch or pull.
- Assert only selected skills produce staged changes and exposure cancellation leaves the source without exposures.

## Progress notes

- 2026-06-13: Added canonical Git URL handling for HTTPS, SSH URL, SCP-style SSH, and `file://` forms, including safe destination-name derivation and embedded HTTP credential rejection.
- 2026-06-13: Added temporary clone acquisition beneath global `central_dir`, disabled submodule recursion, scanned before promotion, and cleaned failed or skill-less clones.
- 2026-06-13: Added strict destination collision behavior with same-origin scan-only reuse and rejection of files, symlinks, non-Git directories, missing origins, and different origins.
- 2026-06-13: Added interactive `skills-manager source add <git-url>` with preview-before-mutation, optional multi-skill selection, and delegation to existing staged CLI exposure plans.
- 2026-06-13: Added `/source add <git-url>` to the TUI with preview, confirmation, skill selection, and delegation to the existing agent-selection and exposure-plan state.
- 2026-06-13: Added regression coverage proving no mutation before confirmation, cleanup on failure, no updates during reuse, selective exposure, and source retention after exposure cancellation.
- 2026-11-02: The follow-up replaces the TUI `/source_add` and CLI `source add` entry points with `/import_repo [URL]` and required-argument `import-repo <address>`, without aliases. GitHub tree URLs resolve to the repository root and preview the whole-repository scan; the clone uses the default branch. TUI URL correction and cancellation keep source acquisition behind confirmation, and exposure keeps its separate confirmation. Stage C reports 272 unit tests, 30 CLI integration tests, formatting, and diff checks passing. Final functional and code-review verification remains for stage E.
- 2026-11-02: Updated the README, feature documentation, roadmap, and changelog for the follow-up. The PRD remains in progress until stage E completes.

## Tasks

- [x] Add Git URL canonicalization and repository-name tests.
- [x] Implement temporary clone, bounded pre-promotion scan, promotion, and cleanup.
- [x] Implement strict same-origin reuse and collision rejection.
- [x] Add the interactive CLI repository-import workflow, now exposed as required-argument `import-repo` without the former `source add` command.
- [x] Add the TUI repository-import workflow, now exposed as `/import_repo [URL]` without `/source_add`.
- [x] Delegate selected skills to existing staged exposure plans.
- [x] Add CLI, TUI, and temporary Git repository regression coverage.
- [x] Update README, durable feature documentation, roadmap, and changelog.
- [x] Resolve GitHub tree URLs to the repository root and preview the whole-repository scan.
- [x] Keep URL acquisition and skill exposure as separately confirmed steps.
- [x] Update follow-up documentation for the implemented behavior.
- [ ] Complete stage E functional verification and independent code review.
