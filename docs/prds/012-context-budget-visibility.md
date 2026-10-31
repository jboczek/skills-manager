---
title: Global skill token estimates
summary: Show an o200k_base estimate of discovery metadata tokens and the contributing global skill count per enabled agent in the TUI status area.
status: done
roadmap: v3
---

# Global skill token estimates

## Goal

Show one compact TUI status line with the estimated number of tokens occupied by globally exposed skill discovery metadata and the number of contributing skills for each enabled agent: Claude Code, Codex, and GitHub Copilot.

Use the `o200k_base` encoding for every agent in this first iteration. The result is directional, not a claim about an agent's exact runtime prompt.

## What is happening today

The TUI status area shows:

- the total inventory skill count and enabled agent count;
- scan health;
- the imported and discovered-not-imported summary.

The inventory already distinguishes `Scope::Global` from `Scope::ProjectLocal` and records the agents and paths for each real exposure. Skills Manager does not parse the exposed `SKILL.md` frontmatter or estimate its discovery-time token footprint.

## What should happen instead

After inventory loads, insert this line between `Global context` and `Scan`:

```text
• Estimated tokens (global): Claude ~776 (36 skills) · Codex ~6394 (201 skills) · Copilot ~1568 (72 skills)
```

The values above are illustrative. The application must calculate them from the current global exposure inventory.

- Include only rows whose scope is exactly `Scope::Global`.
- Show one estimate for each enabled configured agent, in configured agent order and using its display name.
- After each token estimate, show in parentheses how many unique global inventory rows contributed to that agent's result. Render `1 skill` for one row and `<N> skills` for every other count, including zero.
- Exclude every project-local exposure, even when the same skill is also available globally. The global row is counted; the project-local row is not.
- During initial inventory loading, render `• Estimated tokens (global): (loading)`.
- If the initial inventory load fails, render `• Estimated tokens (global): unavailable` and keep the existing error message.
- A failed later refresh keeps the last successful estimate and reports the existing refresh error.
- Recalculate after every successful inventory refresh, including refreshes caused by import or removal.

The status label and the `~` prefix must make clear that the values are estimates.

## Why

Skill discovery metadata consumes model context before a skill is selected. A per-agent estimate makes excessive global exposure or unusually verbose descriptions visible without pretending that Skills Manager can reproduce each provider's complete hidden prompt.

The first iteration deliberately uses one tokenizer so the numbers are comparable and the implementation remains small. Agent-specific text serialization still reflects the documented discovery metadata known to differ between hosts.

## Key decisions

- Use `o200k_base` from `tiktoken-rs` for Claude Code, Codex, and Copilot. Do not map agents to different tokenizers in this iteration.
- Estimate discovery metadata only. Do not read or count the `SKILL.md` body, referenced files, scripts, assets, tool schemas, plugin metadata, system prompts, or content loaded after skill activation.
- Read the YAML frontmatter from the `SKILL.md` belonging to the actual global exposure path. Do not infer exposure scope from a source path.
- Parse YAML with a maintained serde-compatible YAML crate; do not implement YAML by splitting `name:` and `description:` lines. This must support quoted values and folded or literal multiline descriptions.
- Add `tiktoken-rs = "0.12.0"` and `yaml_serde = "0.10.7"` only after rechecking the repository dependency-age policy at implementation time. Commit the resulting `Cargo.lock` update.
- For metadata, use a non-empty frontmatter `name`; otherwise fall back to `InventoryRow.skill_id.name`. Use a string `description`; otherwise use an empty description.
- Missing, unreadable, unterminated, or invalid frontmatter is non-fatal. Count the fallback name only and continue calculating the remaining skills.
- Count a global inventory row at most once per agent. If the row has multiple exposures for the same agent, sort their `SKILL.md` paths lexicographically, use the first path as the metadata source, and do not add a second copy.
- Derive the displayed skill count from exactly the same filtered and de-duplicated rows used for that agent's token sum. Do not run a separate inventory count with different inclusion rules.
- Count the same shared global skill separately for every agent to which it is exposed because each agent has its own context budget.
- Serialize the text passed to `o200k_base` exactly as follows, with no surrounding JSON, XML, labels, or estimated wrapper overhead:

  - Claude Code and Copilot: `name` when the description is empty; otherwise `name + "\n" + description`.
  - Codex: `name + "\n" + skill_md_path` when the description is empty; otherwise `name + "\n" + description + "\n" + skill_md_path`.

- For Codex, `skill_md_path` is the selected exposure path joined with `SKILL.md`, rendered as stored rather than canonicalized. The path is included because Codex exposes skill name, description, and location in its discovery catalog.
- Sum the per-skill token counts as unsigned integers. Render plain base-10 integers without locale separators and prefix each token value with `~`; render the exact contributing row count as `(1 skill)` or `(<N> skills)` without `~`.
- Keep the estimate out of standalone CLI `list` and `scan` output. This slice changes only the TUI status block shown in the attached reference.
- Calculate estimates when inventory is loaded or refreshed, never during each render pass.

## Changes

### `src/context_estimate.rs`

**Change**

- Add a focused module that extracts frontmatter metadata, builds the agent-specific discovery text, counts it with `o200k_base`, and aggregates global estimates.
- Expose a small result type containing `agent_id`, `display_name`, `token_count`, and `skill_count` and a function accepting `&[InventoryRow]` and `&[AgentTarget]`.
- Filter disabled agents and non-global rows before reading metadata.
- De-duplicate repeated exposures for the same inventory row and agent according to the deterministic path rule above.

**Reason**

Token calculation is domain logic shared by initial load and refresh. It does not belong in filesystem scanning or terminal rendering.

**Expected behavior**

Given the same inventory, agent configuration, exposed files, and paths, the module returns the same ordered estimates and contributing skill counts independently of the current working directory.

### `src/lib.rs`, `Cargo.toml`, and `Cargo.lock`

**Change**

- Export the new module.
- Add the pinned tokenizer and YAML dependencies selected above after confirming they still satisfy `docs/adr/adr-002-dependency-policy.md`.

**Reason**

The feature needs standards-compliant YAML parsing and the explicitly selected encoding.

**Expected behavior**

Locked builds have no runtime download or network dependency and use `o200k_base` for all three agents.

### `src/tui/app.rs`

**Change**

- Store the last successful estimates in `App`.
- Extend `InitialLoad` so the background load calculates estimates after building inventory and returns both together.
- Recalculate estimates in `refresh_inventory()` after a successful inventory build.
- Add the estimated-token line between `Global context` and `Scan` in `rebuild_status_messages()`.
- Render loading, ready, and initial-load-failure states exactly as specified above.

**Reason**

Reading hundreds of `SKILL.md` files and tokenizing their metadata must not happen on every frame. The line must also remain current after exposure changes.

**Expected behavior**

The status area updates once data is available and reflects every successful import, removal, or manual list refresh without restarting the application.

### `src/tui/layout.rs` and status rendering tests

**Change**

- Increase the fixed status region from three to four rows.
- Keep the existing wrapping and last-lines behavior in `src/tui/components/status.rs` unless a failing test proves a minimal adjustment is required.

**Reason**

The new line must not displace the current `Global context`, `Scan`, or imported/discovered summary at the reference terminal width.

**Expected behavior**

All four status lines remain visible at the documented screenshot width. Existing narrow-terminal truncation behavior remains unchanged.

### Tests

**Change**

- Start implementation with failing unit tests for the estimator and TUI status contract.
- Cover global-only filtering, enabled-agent filtering, shared exposures, duplicate same-agent exposures, deterministic path selection, contributing skill counts, singular/plural count labels, metadata fallbacks, quoted and multiline YAML, Unicode descriptions, Codex-only path inclusion, output ordering, loading, initial failure, refresh, and the four-row status layout.
- Add a render-level regression at the reference width proving all four lines are visible and contain the exact label, `~` markers, and per-agent skill counts.

**Reason**

The main failure risks are silently counting project-local rows, double-counting shared targets, parsing YAML incorrectly, or clipping the new status line.

**Expected behavior**

Tests fail before the feature exists and pass only when filtering, counting, refresh, and rendering satisfy this specification.

### Documentation after implementation

**Change**

- Add `docs/features/context-budget-visibility.md` with the user-visible formula and limitations.
- Update `README.md` and the relevant TUI screenshot or textual walkthrough.
- Mark this PRD and roadmap item complete and add an entry to `docs/changelog.md`.

**Reason**

Users must understand that the displayed values cover global discovery metadata and are estimates based on `o200k_base`.

**Expected behavior**

Product documentation, roadmap status, and the implemented TUI use the same terminology and scope.

## Verification

- Run `cargo fmt --check`.
- Run `cargo check --locked`.
- Run `cargo test --locked`.
- Run `cargo clippy --locked --all-targets --all-features -- -D warnings`.
- Run the repository's visual TUI fixture check at the reference width and inspect the rendered status block.
- Manually verify with a mixed inventory that adding or removing only a project-local exposure does not change any estimate.
- Manually verify that adding or removing a global exposure changes only the agents receiving that exposure.
- Compare at least one fixture's module result with a direct `o200k_base` count over the exact serialized text to guard against accidental wrapper text.

## Done when

- The TUI displays `Estimated tokens (global)` between `Global context` and `Scan`.
- Each enabled agent has one `~N` value calculated with `o200k_base`.
- Each token value is followed by the exact number of unique global skills contributing to it, rendered as `(1 skill)` or `(<N> skills)`.
- Only real `Scope::Global` exposure rows contribute to the values.
- Project-local rows never affect the result.
- Claude Code and Copilot count name plus description; Codex counts name, description, and exposed `SKILL.md` path.
- Repeated exposure records do not double-count a skill for one agent.
- Loading and initial-load failure states are explicit, and failed refreshes preserve the last valid estimate.
- Successful import, removal, and refresh operations update the values.
- The four status lines fit at the reference width and all required automated and visual checks pass.
- README, feature documentation, PRD status, roadmap status, screenshot or walkthrough, and changelog are updated with the implementation.

## Out of scope

- Project-local token estimates.
- Counting skill bodies or content loaded after activation.
- Reproducing provider-owned prompt wrappers or hidden system context.
- Different tokenizers per agent or model detection.
- Exact billing, cache, context-window, or percentage calculations.
- Per-skill breakdowns, sorting, warnings, thresholds, recommendations, or removal actions.
- CLI output, machine-readable output, configuration switches, or a new TUI view.
- Skills not represented by real inventory exposures, including undiscovered provider built-ins or plugin-provided catalog entries.

## Implementation notes

- `src/context_estimate.rs` owns frontmatter extraction, YAML parsing,
  agent-specific serialization, `o200k_base` counting, and global-row
  aggregation so rendering never performs filesystem reads or tokenization.
- Inventory rows are filtered by exact global scope and enabled configured
  agents before metadata is read. Duplicate exposures for one row and agent
  use the lexicographically first stored `SKILL.md` path.
- The TUI stores successful estimates in `App`, calculates them during initial
  load and refresh, and renders explicit loading/unavailable states while
  preserving the last successful result across a later refresh failure.

## Progress notes

- 2026-09-13: Implemented the global discovery-metadata estimator, TUI status
  integration, four-row status layout, YAML/tokenization tests, and refresh
  coverage.
- 2026-09-13: Updated the README, TUI feature notes, durable feature doc,
  roadmap, and changelog; verified the locked Rust checks and reference-width
  status rendering.

## Tasks

- [x] Add deterministic global-only aggregation with YAML frontmatter
  fallbacks and `o200k_base` serialization rules.
- [x] Calculate estimates during initial inventory load and every successful
  refresh, including import/removal refreshes.
- [x] Render loading, unavailable, ready, singular/plural counts, and the
  four-line status region without changing CLI output.
- [x] Add focused estimator, TUI status, refresh, and reference-width tests.
- [x] Update user-facing documentation and mark the roadmap item complete.
