---
status: closed
decision: separate-shared-public-tap
---

# Issue 03: use a shared public Homebrew tap

## Decision

Use `jboczek/homebrew-tap` for formulae and tap automation. Keep application
source and release builds in `jboczek/skills-manager`.

The repository name follows Homebrew's `homebrew-<name>` convention, so users
can address it as `jboczek/tap`. The first tap PR contains only
`Formula/skills-manager.rb`; future CLI formulae may be added independently.

## Consequences

- A formula is not added to `skills-manager` itself. The oMLX same-repository
  example is informative only; its source-build formula does not match this
  project's prebuilt-archive contract.
- The source repository can create and attest release assets without gaining
  contents write access to the tap.
- The tap owns its formula renderer, PR branch, checks, and protected `main`.
- The public repository is initialized and protected. The source repository
  keeps the publisher and native-check files under `homebrew-distribution/tap/`
  as a reviewable mirror; it does not receive tap contents or pull-request
  write access.
