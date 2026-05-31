# Big Picture: Skills Manager

## Summary

Skills Manager is a terminal-first devtool for people who use multiple agentic coding tools and want one reliable view of their local skill ecosystem. It should show which skills exist, where they came from, where they are exposed, which agents can use them, and what will change before the user applies an import or removal.

The strategic direction is not a classic package manager. The stronger product shape is a **skill exposure manager**: source repositories remain understandable and centralized, while configured Codex, Claude, and Copilot target paths expose selected skills through controlled links or local copies.

## Problem / Opportunity

Power users increasingly work across Codex, Claude, Copilot, and repository-local agent workflows. Each tool has its own conventions for skill directories, global scope, and project scope. Today, users have to remember this topology manually, copy files into the right places, and infer whether a skill is actually available to the intended agent.

Skills Manager addresses the visibility and control gap:

- what skills are installed,
- which repository or folder they came from,
- which global or project-local targets expose them,
- whether each exposure is a symlink or physical copy,
- and what disk changes will happen before applying an action.

The opportunity is to make skill management feel like an intentional workflow instead of a set of scattered filesystem habits.

## Users / Stakeholders

Primary users are power users and developers who already work with Git, local repositories, and agent-based development tools. The first stakeholder is the project author, with a broader audience of developers who use more than one agent ecosystem and want local control without maintaining mental maps of every skill directory.

Affected stakeholders include future contributors, because the project needs a simple Rust architecture and clear dependency policy, and users of each supported agent directory, because Skills Manager must respect native conventions rather than hiding them.

## Direction

### V1 — First useful version

V1 should prove that a user can open a terminal UI and answer the core question:

```text
What skills do I have, where do they come from, where are they exposed, and what exactly will change after apply?
```

The first useful local version should:

- scan configured source roots and supported agent directories,
- discover skills by `SKILL.md`,
- show one consolidated row per logical skill where possible,
- distinguish global and project-local scope,
- show effective availability for Codex, Claude, and Copilot,
- show connection type such as symlink or physical copy,
- support selecting specific discovered skills from repositories that contain one or many skills,
- stage import, exposure, detach, and removal changes before applying them,
- after confirmation, actually create the selected exposure in each configured tool target path,
- warn clearly before deleting a physical copy,
- provide editable defaults so a new user can inspect a config immediately instead of starting from empty values,
- keep shared implementation folders such as `.agents` hidden inside configuration rather than presenting them as agents or command targets.

Implementation details such as shared or tool-specific skill folders should stay inside configuration. The product-facing model should remain focused on which tools can use each skill.

The UI should be a full-screen assistant-style TUI, not just a raw table. The table is the control surface, but the product experience should include a command prompt, status feed, hints, and complete interactive flows for `list`, `scan`, `import`, `remove`, `config`, and `help`.

### V2 — Expansion

Once V1 proves the local inventory and exposure model, V2 can expand from current-directory workflows into broader local workspace management. Likely directions include operating against arbitrary project paths, importing directly from Git URLs into a managed source directory, and migrating older manual installs into the central source-plus-symlink model.

The important V2 move is to reduce manual setup across projects without hiding what the tool is doing. Users should still be able to inspect every source, target, and pending filesystem change.

### V3 — Long-term potential

The long-term opportunity is a local skill control plane for agentic development environments. Skills Manager could become the place where users curate reusable skills, expose them consistently across tools, audit what each project can access, and share repeatable setups across machines or teams.

One valuable later capability is context-budget visibility. Because skills can inject descriptions or metadata into an agent's context window, Skills Manager could estimate how much context is consumed by the currently enabled skills for each tool. That would help users avoid overloading an agent with skills that reduce the useful working context.

This is speculative and depends on V1 proving that users trust the inventory and apply model. Without that trust, higher-level features such as migration, synchronization, or team conventions would create more risk than value.

## Key Bets

- Multi-agent users have enough skill-directory friction that a dedicated exposure manager is worth using.
- A source-repository-plus-symlink model is easier to reason about than repeated manual copies.
- Users will trust the tool if it shows staged filesystem changes before applying them.
- A terminal-first assistant-style interface with complete import/remove flows is the right default for the target audience.
- Codex, Claude, and Copilot conventions are stable enough to support a useful V1 through configurable target paths.
- A simple single-crate Rust application can carry V1 without premature workspace complexity.

## Risks / Open Questions

- The supported agent directory conventions may diverge or change, so target path configuration must stay flexible.
- Duplicate names, nested skills, and multi-skill repositories may make the inventory model confusing unless collisions are shown as numbered choices with enough path or origin context.
- Physical-copy removal is risky; the product must make destructive actions explicit and hard to misread.
- Startup scanning could become slow if recursive repository discovery is too broad or poorly bounded.
- The boundary between effective availability and technical exposure may confuse users unless the UI explains it through layout and labels.
- V1 intentionally avoids full package-manager concerns such as versions, updates, branches, and dirty repository state; this may be acceptable for exposure management but could become a common user expectation.
- Context-window usage estimates may be approximate because each agent can decide differently which skill fields are injected into context.

## Source Inputs

- `docs/interview.md`
- `docs/skills_manager_rust_guidelines.md`
