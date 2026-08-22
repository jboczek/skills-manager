---
status: closed
decision: guarded-idempotent-release-to-tap-pr
---

# Issue 10: release state machine and recovery policy

## States and guarded transitions

```text
Prepared
  -> Tagged
  -> Built
  -> DraftRelease
  -> PublishedRelease
  -> TapDispatchPending
  -> TapPROpen
  -> TapChecksPassed
  -> TapMerged
```

`Prepared` requires `Cargo.toml` version `X.Y.Z` and branch
`release/vX.Y.Z`. `Tagged` requires an exact `vX.Y.Z` tag on the release
commit. `Built` requires both native jobs and all archive checks. `DraftRelease`
requires exactly two archives, `SHA256SUMS`, generated notes, and attestations.
`PublishedRelease` is allowed only after the draft checks pass. `TapPROpen`
requires the expected source tag, commit, hashes, and one-file formula diff.
`TapMerged` requires both native tap checks and protected-branch approval.

The tap publisher never writes `main` directly. Auto-merge is enabled only
after the required ARM64 and Intel checks report success. The short-lived
`automation/skills-manager-vX.Y.Z` branch is deleted after merge; the source
tag remains unchanged.

## Failure and retry policy

- A build failure leaves the source repository and tap unchanged.
- A failed draft creation may be retried only when no release exists, or when
  an existing draft has the same tag/target commit and its existing assets are
  byte-identical. Mismatched or published releases fail closed; no asset is
  overwritten and no second release is created.
- A failed tap dispatch leaves the published source release intact and can be
  retried from the tap with the same tag.
- A retry may reuse an existing tap PR only when its branch name, PR marker
  fields, source commit, tag, and exact one-file diff all match the expected
  result. Any mismatch is a hard conflict requiring a new version or explicit
  human cleanup; the publisher does not force-push or close unrelated work.
- A failed audit, install, test, provenance, or allowlist check leaves tap
  `main` at the previous formula. The PR may be retried after the cause is
  corrected, but it cannot bypass required checks.

## Invariants

1. Cargo version, `release/vX.Y.Z`, `vX.Y.Z`, and release commit agree.
2. A tag maps to at most one release and one matching tap PR.
3. Tap `main` never references a new version before checks pass.
4. Tags and published assets are immutable.
5. A tap PR changes exactly `Formula/skills-manager.rb`, and its version, two
   URLs, and two SHA-256 values are updated as one unit.

## Required validation matrix

| Stage | ARM64 | Intel |
|---|---|---|
| Native release build | `macos-14`, deployment target 14.0 | `macos-15-intel`, deployment target 15.0 |
| Tap checks | `brew test-bot --only-formulae --build-from-source`, audit/install/test | same commands and formula |
| Manual release smoke test | macOS 14, 15, 26 | macOS 15, 26 |

The manual OS-version checks remain release gates even though they are not all
available in this task workspace.

The shared tap is configured with protected `main`; its publisher PR can
auto-merge only after the native ARM64 and Intel required checks and the
configured review requirement pass.
