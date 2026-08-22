# Managed Git Source Import

Managed Git source import adds repositories to the global skill library without combining source acquisition with agent exposure.

Use the CLI:

```bash
skills-manager source add https://github.com/example/skills.git
```

Or use the TUI prompt:

```text
/source add https://github.com/example/skills.git
```

## Preview And Confirmation

Skills Manager derives the repository name from the URL, removes a trailing `.git`, and previews:

- the requested Git URL
- the final `central_dir/<repository-name>` destination

No directory is created and Git is not invoked until the user confirms. Non-interactive CLI use stops after the preview because acquisition requires an interactive confirmation.

## Clone And Promotion

A new source is cloned to a unique temporary directory beneath `central_dir`. The Git command disables submodule recursion and initialization.

The temporary checkout is scanned with the existing bounded scanner, which does not follow symlinked directories. At least one `SKILL.md` must be found before the checkout is renamed to its final destination.

Clone failures and repositories without skills are cleaned up. The final destination is never partially populated by the clone workflow.

## Existing Destinations

Skills Manager reads the existing repository's `origin` and compares canonical identities. Canonical comparison:

- treats HTTPS, SSH URL, and SCP-style SSH forms as equivalent when host and repository path match
- lowercases the host
- removes trailing slash and `.git`
- preserves repository path case
- rejects embedded HTTP credentials

A matching origin is reused and scanned as-is. Reuse never runs fetch, pull, reset, or checkout, so dirty, detached, or locally modified repositories remain unchanged.

## Remote Updates

The interactive /list flow checks each visible Git source repository once per refresh by fetching origin in the background, so the list remains usable before remote checks finish. A repository group with commits beyond the local checkout shows a green one-line update notice from the first agent column and the Cmd+U shortcut. Failed background checks are ignored without showing Git stderr in the interface. Cmd+U opens a short-ID and subject preview of the missing commits. Entering y runs git pull --ff-only for that repository and refreshes the inventory; declining or a failed pull leaves the checkout unchanged.

Every unknown or conflicting destination fails:

- file or symlink
- non-Git directory
- repository without a readable `origin`
- repository with a different canonical origin

Skills Manager does not overwrite, rename, merge, or choose a suffixed destination.

## Skill Exposure

Acquisition exposes nothing automatically. After a successful clone or reuse:

- the CLI can select one or more discovered skill numbers
- the TUI can select one discovered skill
- each selection enters the existing agent selection and staged exposure plan

Every exposure still requires plan confirmation. Cancelling exposure keeps the managed source and creates no agent target.

## Authentication And Trust Boundary

Skills Manager stores no Git credentials. Git uses existing SSH configuration, credential helpers, and standard user configuration.

Repository content is treated as untrusted scan input. Skills Manager does not run repository scripts, hooks from repository content, build tools, package managers, installers, submodules, or skill code.
