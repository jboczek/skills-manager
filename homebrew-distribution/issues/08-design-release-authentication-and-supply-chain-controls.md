---
status: closed
decision: least-privilege-split-ownership
---

# Issue 08: release authentication and supply-chain controls

## Ownership and permissions

The source workflow defaults to `contents: read`. Build jobs additionally use
only `id-token: write` and `attestations: write` to create provenance for the
final archives. The job that creates or publishes a source release is isolated
behind a protected `release-publish` environment and receives the minimum
source-repository `contents: write` permission.

The source repository never receives tap `contents: write`, pull-request, or
ref-update permission. If automatic dispatch is enabled, the protected
`tap-dispatch` environment contains a fine-grained token scoped to
`jboczek/homebrew-tap` with Actions/workflow-dispatch write and metadata read
only. It cannot edit files, branches, releases, or pull requests. A manual
dispatch from the tap is the safe fallback when that token is not configured.

The tap publisher uses its own job-scoped `GITHUB_TOKEN` with only `contents:
write`, `pull-requests: write`, and the read permissions needed for checks. Its
`main` branch requires review and the native ARM64 and Intel checks. No
workflow runs with `pull_request_target` against untrusted code.

## Immutable and pinned inputs

- Every `uses:` reference is pinned to a full commit SHA and its human-readable
  release is kept in a nearby comment.
- Release tags are verified against the exact source commit and Cargo version.
- Existing published releases or assets are never replaced. A retry may reuse
  a matching draft only after byte/hash validation.
- The publisher verifies both entries in `SHA256SUMS` against the downloaded
  archives and verifies GitHub artifact attestations for both archives against
  `jboczek/skills-manager`.
- Release and tap tokens are stored as environment secrets, are not printed,
  and are never passed to untrusted pull-request code.
- The formula renderer validates tag, version, filenames, and hexadecimal
  hashes before writing the formula. It runs from trusted tap code, not from
  release-provided scripts.

## Recovery boundary

Any failed build, attestation, checksum, release, or tap validation stops
before the next write. In particular, a failed tap publication cannot change
tap `main`; only a protected PR merge can do that. The complete retry policy is
recorded in [issue 10](10-decide-release-state-machine-and-recovery-policy.md).
