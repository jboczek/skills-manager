# `jboczek/homebrew-tap`

This directory mirrors the separate public tap `jboczek/homebrew-tap`. It is
kept with the source repository so the tap's trusted automation remains
reviewable and reproducible.

The tap-side publisher accepts a released `vX.Y.Z` tag, verifies the two
archives, checksums, source commit, and artifact attestations, and creates or
reuses one `automation/skills-manager-vX.Y.Z` pull request. It never changes
`main` directly. Configure `main` branch protection with these required checks:

- `formula (ARM64)`
- `formula (Intel)`

Keep auto-merge enabled only for pull requests that pass both checks. The
publisher uses the tap workflow's job-scoped `GITHUB_TOKEN` with contents,
pull-request, check, status, Actions, and attestation read permissions. The
source repository has no tap contents or pull-request write access; optional
dispatch uses a protected environment and a fine-grained
Actions/workflow-dispatch token. Manual dispatch from the tap is the fallback.
