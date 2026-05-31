---
title: ADR-001 Fixed technology stack for Skills Manager v1
status: Accepted
date: 2026-05-30
deciders: Skills Manager project team
---

# ADR-001: Fixed technology stack for Skills Manager v1

## Status
Accepted

## Date
2026-05-30

## Context
Skills Manager v1 needs a single, opinionated Rust implementation path so the team can move quickly, keep the codebase simple, and avoid design churn. The engineering guidelines require one clear stack for v1 and explicitly disallow introducing alternatives unless a separate architectural decision record is written. The same guidelines also constrain implementation style: use the local `git` CLI instead of `git2`, and do not adopt async in v1.

## Decision
Skills Manager v1 will use the following fixed technology stack, with no alternatives in v1:

| Area | Technology |
|---|---|
| Language | Rust stable |
| CLI parser | clap |
| TUI framework | ratatui |
| Terminal backend | crossterm |
| Config serialization | serde + toml |
| Config/cache/data paths | directories |
| Directory scanning | ignore |
| Git integration | `std::process::Command` calling local `git` CLI |
| Symlinks | `std::os::unix::fs` and `std::os::windows::fs` |
| Domain errors | thiserror |
| Application / CLI errors | anyhow |
| Diagnostics / logging | tracing |
| CLI testing | assert_cmd |
| Temporary filesystem tests | tempfile |
| Snapshot tests | insta |
| Dependency update automation | Later hardening after local V1 |
| Dependency auditing | Later hardening after local V1 |
| Dependency policy checks | Later hardening after local V1 |
| Release automation | Later hardening after local V1 |

Additional decisions for v1:

- Do not use async in v1; keep the application synchronous unless a concrete future need is approved by a separate ADR.
- Do not use `git2` in v1; invoke the locally installed `git` executable through `std::process::Command` instead.

## Consequences
- Engineering work for Skills Manager v1 must stay within this stack.
- New libraries, frameworks, or competing implementation approaches are out of scope for v1 by default.
- Git interactions should remain shallow and CLI-based, which reduces integration complexity but limits deeper embedded Git capabilities.
- Runtime and code structure should remain synchronous, which simplifies implementation but rules out async-first designs unless re-evaluated.
- No other technology choices should be introduced unless a separate ADR is written.
