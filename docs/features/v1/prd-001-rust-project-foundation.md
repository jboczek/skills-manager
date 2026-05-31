---
title: Rust project foundation
summary: Create the buildable Rust CLI/TUI foundation that all V1 Skills Manager work depends on.
status: planned
roadmap: v1
---

# Rust project foundation

## Context

Skills Manager is currently a documentation-only project. V1 needs a native Rust application that can become a terminal-first assistant-style devtool for managing local agent skills. This first feature does not deliver skill management by itself; it creates the crate, dependency policy, command routing, module boundaries, and local verification loop required for every later feature.

The project should start as one simple Rust crate. A workspace split can happen later only if the codebase clearly needs it.

## Problem

Without a working crate, later roadmap items have nowhere to place config loading, scanning, inventory resolution, mutation planning, CLI commands, or the TUI. The risk is either over-designing too early or starting with an unstructured binary that becomes hard for a junior developer to extend safely.

## Goal

Create a minimal but production-shaped Rust foundation that can be built, tested, linted, and evolved without rethinking the project structure after each V1 feature.

## Non-goals

- Do not implement full skill scanning, import, removal, or inventory behavior in this feature.
- Do not build the complete TUI experience yet.
- Do not split into multiple crates or create a Rust workspace.
- Do not introduce dependencies outside the approved stack unless a separate architectural decision is written.
- Do not create stable JSON output or automation contracts.

## Proposed experience

A developer should be able to clone the repository, run the standard Rust checks, and see a working binary skeleton:

```bash
cargo check --locked
cargo test --locked
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
skills-manager --help
```

Running `skills-manager` with no subcommand should route to the TUI entry point, even if the first implementation only renders a minimal placeholder shell. Explicit subcommands should be parsed through `clap` and routed to thin command handlers.

## Requirements

- Create a single Rust binary crate named `skills-manager`.
- Commit `Cargo.toml` and `Cargo.lock`.
- Use Rust stable.
- Add the approved V1 dependencies: `clap`, `ratatui`, `crossterm`, `serde`, `toml`, `directories`, `ignore`, `thiserror`, `anyhow`, and `tracing`.
- Add development/testing dependencies: `assert_cmd`, `tempfile`, and `insta` when tests need them.
- Create the initial source layout from the engineering guidelines, including `main.rs`, `cli.rs`, `config.rs`, `domain.rs`, `scanner.rs`, `inventory.rs`, `git.rs`, `symlink.rs`, `agent_dirs.rs`, `output.rs`, `errors.rs`, `commands/`, and `tui/`.
- Keep `main.rs` limited to diagnostics initialization, CLI parsing, and routing.
- Keep command handlers thin; business logic belongs in modules such as `config`, `scanner`, `inventory`, and `symlink`.
- Add a small smoke test proving the binary starts and `--help` works.
- Document the local verification commands for formatting, clippy, check, and tests.
- Defer GitHub Actions, Renovate, release automation, and cargo-deny wiring until after the local V1 workflow is proven.

## Technical implementation notes

Use `clap` derive for the CLI model. The initial command enum should include `list`, `scan`, `import`, `remove`, `config`, and `doctor`, but command bodies may return clear placeholder messages until their PRDs are implemented.

Use `thiserror` for domain errors and `anyhow` at application boundaries. Use `tracing` for diagnostics setup even if V1 initially logs only simple startup messages.

The first `domain.rs` should define the core types that later PRDs will extend: `SkillId`, `AgentId`, `AgentDefinition`, `Scope`, `ConnectionKind`, `SkillSource`, `SkillExposure`, and `InventoryRow`. These types should not perform filesystem IO or terminal rendering.

Do not use async. No V1 requirement needs it.

### Source layout

Keep V1 as a single crate with an explicit, junior-friendly layout:

```text
skills-manager/
  Cargo.toml                  # single-crate manifest and approved dependencies
  Cargo.lock                  # committed lockfile for repeatable local checks
  README.md                   # repository entry point
  AGENTS.md                   # repository-specific agent instructions
  LICENSE
  CHANGELOG.md

  docs/
    engineering-guidelines.md # engineering reference
    product-notes.md          # product context and notes

  src/
    main.rs                   # startup, diagnostics, parse, and route
    cli.rs                    # clap models for optional subcommands
    config.rs                 # config loading and defaults
    domain.rs                 # pure domain types
    scanner.rs                # skill discovery from source locations
    inventory.rs              # current-state assembly from config + filesystem
    git.rs                    # lightweight git metadata lookup
    symlink.rs                # symlink creation/removal and connection detection
    agent_dirs.rs             # effective target directory resolution
    output.rs                 # human-readable non-TUI formatting
    errors.rs                 # domain error definitions

    commands/
      mod.rs                  # command module exports
      list.rs                 # list command handler
      scan.rs                 # scan command handler
      import.rs               # import command handler
      remove.rs               # remove command handler
      config.rs               # config command handler
      doctor.rs               # doctor command handler
      tui.rs                  # no-subcommand TUI entry routing

    tui/
      mod.rs                  # TUI module exports
      app.rs                  # app state and update loop shell
      layout.rs               # screen layout composition
      theme.rs                # colors and style tokens
      events.rs               # keyboard and terminal event handling

      components/
        mod.rs                # component exports
        header.rs             # header component
        status.rs             # status summary component
        main_panel.rs         # central panel component
        prompt.rs             # prompt/input component
        footer.rs             # footer/help component
        table.rs              # shared table rendering
        dialog.rs             # modal/dialog rendering

  tests/
    cli_tests.rs              # lightweight CLI smoke/snapshot coverage
```

If the codebase later outgrows this layout, a workspace split can happen then; it is not a V1 requirement.

### Module responsibilities

- `main.rs`: Application entry point only. It initializes diagnostics, parses CLI arguments, opens the TUI when no subcommand is provided, and routes explicit commands to thin handlers.
- `cli.rs`: Defines the `clap` model for `list`, `scan`, `import`, `remove`, `config`, and `doctor`. The CLI is a convenience surface for scripting and debugging, not the primary product experience.
- `domain.rs`: Holds pure, extensible core types used across config, scanning, inventory, and mutation planning. It must stay free of filesystem mutation, process execution, and terminal rendering.
- `config.rs`: Loads and validates explicit, user-editable configuration from the platform-correct config directory via `directories`. `config init` should write meaningful defaults without creating source or target directories.
- `scanner.rs`: Discovers skills by finding directories containing `SKILL.md` in configured source locations. It should use `ignore`, respect `max_scan_depth`, resolve the nearest `.git` root, and never follow symlinks while scanning sources.
- `git.rs`: Retrieves lightweight repository metadata, especially `origin`, through local `git` commands such as `git -C <repo_path> remote get-url origin`. V1 does not need `git2`.
- `symlink.rs`: Owns symlink creation/removal plus connection-kind detection. It must distinguish symlinks from physical copies and refuse physical-copy deletion unless the user explicitly confirms it.
- `agent_dirs.rs`: Resolves effective target directories for configured agents such as Claude, Codex, and Copilot. It should stay config-driven and treat `.agents` as an internal shared target rather than a product-facing agent.
- `inventory.rs`: Builds the live inventory view from config, source repositories, target directories, and the current project directory. V1 inventory is computed from real state on demand, not persisted as a database.
- `output.rs`: Formats non-TUI output for humans, primarily tables. Stable JSON or other automation-oriented output formats are intentionally deferred.
- `errors.rs`: Defines reusable project/domain errors with `thiserror`, while leaving application-boundary context wrapping to `anyhow`.
- `commands/`: Contains thin handlers for each explicit CLI command. These modules should translate CLI intent into calls into config, scanner, inventory, symlink, and related modules instead of housing business logic themselves.
- `tui/`: Contains the TUI shell, layout, event loop, theme, and reusable components. V1 can start as a placeholder screen, but running without arguments should already enter this module boundary.

### Domain types

Use stable identifier-based types so later agent additions remain mostly configuration work rather than enum rewrites:

```rust
use std::path::PathBuf;

pub struct SkillId {
    pub namespace: String,
    pub name: String,
}

pub struct AgentId(pub String);

pub struct AgentDefinition {
    pub id: AgentId,
    pub display_name: String,
    pub global_dir: Option<PathBuf>,
    pub project_dir: Option<PathBuf>,
    pub shared_target_ids: Vec<String>,
    pub enabled: bool,
}

pub enum Scope {
    Global,
    ProjectLocal,
}

pub enum ConnectionKind {
    Symlink,
    PhysicalCopy,
    Missing,
    Unknown,
}

pub struct SkillSource {
    pub repo_name: Option<String>,
    pub repo_path: Option<PathBuf>,
    pub remote_url: Option<String>,
}

pub struct SkillExposure {
    pub agent_id: AgentId,
    pub path: PathBuf,
    pub connection: ConnectionKind,
}

pub struct InventoryRow {
    pub skill_id: SkillId,
    pub source: SkillSource,
    pub scope: Scope,
    pub exposures: Vec<SkillExposure>,
}
```

These types are intentionally pure data structures. Agent-specific special cases should stay minimal so that adding a new agent mainly means adding config and reusing the same scan/exposure flow.

### Error handling

Use a split error model:

- `thiserror`: reusable domain errors from modules such as config, scanner, inventory, and symlink.
- `anyhow`: application and CLI boundaries where errors are wrapped with context before being returned or rendered.

Example domain error shape:

```rust
use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum SkillsError {
    #[error("skill not found: {0}")]
    SkillNotFound(String),

    #[error("target already exists: {0}")]
    TargetAlreadyExists(PathBuf),

    #[error("refusing to delete physical copy without explicit confirmation: {0}")]
    PhysicalDeleteRequiresConfirmation(PathBuf),

    #[error("unknown skill origin")]
    UnknownOrigin,
}
```

## Test strategy

### Unit tests

Cover the core pure logic first:

- `SkillId` normalization and namespace generation.
- Duplicate skill name handling and scan result normalization.
- Exposure resolution and connection kind detection.
- Staged change generation for planned mutations.
- Config parsing, prompt command parsing, and agent config parsing.
- Hidden shared `.agents` effective availability behavior.

### CLI tests

Keep CLI coverage lightweight in V1 using `assert_cmd`, `predicates`, `insta`, and `tempfile`.

- Add smoke coverage for binary startup and `--help`.
- Snapshot human-readable output for `skills-manager list`, `skills-manager scan`, and `skills-manager config show`.
- Use temporary filesystem fixtures only where needed for source/target layout scenarios; avoid a large integration suite at this stage.

## Success criteria

- The repository builds as a Rust application with a committed lockfile.
- `skills-manager --help` prints all intended V1 command names.
- Running without arguments enters the TUI entry point.
- Local verification commands can run the agreed checks.
- A junior developer can find the intended home for config, scanning, inventory, symlink, CLI, and TUI work without guessing.

## Edge cases

- If tooling such as `cargo audit` or `cargo deny` is not installed locally, document those checks as later hardening rather than making them a foundation blocker.
- If a dependency has a version released less than 14 days ago, choose an older maintained version or delay the upgrade.

## Dependencies

- Roadmap item `001`.
- Source guidance: `docs/skills_manager_rust_guidelines.md`.

## Open questions

- Should the placeholder no-argument TUI render a minimal screen immediately, or can it print a temporary message until the TUI PRD begins?
- Which dependency age verification process is enough before CI/release hardening exists?
