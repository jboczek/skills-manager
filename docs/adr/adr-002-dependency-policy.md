---
title: ADR-002 Dependency policy for Skills Manager v1
status: Accepted
date: 2026-05-30
deciders: Skills Manager project team
---

# ADR-002: Dependency policy for Skills Manager v1

## Status
Accepted

## Date
2026-05-30

## Context
Skills Manager should use modern, maintained Rust dependencies, but the project also needs to reduce supply-chain risk during early development. The engineering guidelines therefore require a minimum release-age gate for dependencies, committed lockfiles, and a manual workflow in local v1. Automated dependency hardening is intentionally deferred until the local v1 workflow has been proven.

## Decision
The project adopts the following dependency policy for v1:

- Use current, maintained dependencies.
- Do not add or upgrade to a crate version released less than **14 days ago**.
- Commit `Cargo.lock` to the repository.
- Check dependency release dates manually before adding or upgrading crates.
- Do not manually bypass the dependency policy unless there is a documented security or compatibility reason.
- Do not use Renovate, `cargo-audit`, or `cargo-deny` in v1; defer them to post-v1 hardening.

Later hardening will be added **after the local v1 workflow is proven**. At that point, add Renovate, `cargo-audit`, `cargo-deny`, and GitHub Actions, using a Renovate configuration like:

```json
{
  "extends": ["config:recommended"],
  "minimumReleaseAge": "14 days",
  "packageRules": [
    {
      "matchManagers": ["cargo"],
      "minimumReleaseAge": "14 days"
    }
  ]
}
```

When CI dependency checks are added, GitHub Actions should run:

```bash
cargo check --locked
cargo test --locked
cargo audit
cargo deny check
```

For manual dependency updates before that hardening exists, use:

```bash
cargo update
cargo check --locked
cargo test --locked
```

Before merging a manual dependency update, confirm that no updated crate version was published less than 14 days ago.

## Consequences
- Dependency selection stays intentionally conservative in v1.
- `Cargo.lock` becomes part of the reviewed source of truth and should remain committed.
- Dependency freshness checks remain manual until post-v1 hardening is introduced.
- Renovate, `cargo-audit`, `cargo-deny`, and CI-based dependency enforcement are explicitly out of scope for local v1, not forgotten.
- Once the local v1 workflow is proven, the project should add the documented Renovate configuration and CI dependency checks exactly as described in this ADR.
- Any exception to this policy must be documented with its security or compatibility rationale.
