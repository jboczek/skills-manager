# `jboczek/homebrew-tap`

This directory is the reproducible template for the separate public tap. Copy
its contents into `jboczek/homebrew-tap` after `brew tap-new jboczek/tap`.

The tap-side publisher accepts a released `vX.Y.Z` tag, verifies the two
archives, checksums, source commit, and artifact attestations, and creates or
reuses one `automation/skills-manager-vX.Y.Z` pull request. It never changes
`main` directly. Configure `main` branch protection with these required checks:

- `formula (ARM64)`
- `formula (Intel)`

Keep auto-merge enabled only for pull requests that pass both checks. The
publisher needs a tap-scoped `GITHUB_TOKEN` with contents and pull-request
write permissions; the source repository only dispatches this workflow with a
fine-grained Actions/workflow-dispatch token.
