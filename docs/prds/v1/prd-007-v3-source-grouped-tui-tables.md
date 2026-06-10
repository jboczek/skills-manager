---
title: Source-grouped TUI tables
summary: Group list and scan rows by privacy-safe source identity with collapsed-by-default keyboard navigation.
status: planned
roadmap: v2
---

# Source-grouped TUI tables

## Context

PRD-007 delivered the assistant-style TUI shell, and PRD-007 v2 added shared list/scan navigation and table-driven actions. The TUI can now display and act on roughly 150 discovered skills, but both list and scan views still render every skill as one flat row.

Large repositories often contribute many skills. Some repositories expose the same skill twice from different internal folders, such as `.agents/skills/<name>` and `skills/<name>`. Other rows have no resolved Git repository and therefore show `unknown`. The current source column may also fall back to an absolute path containing machine-specific directories and a user name.

## Problem

A flat table makes the initial view noisy and forces users to scroll through long runs of skills from the same source. The source label is not always enough to explain where a row came from:

- many rows repeat the same repository name,
- two different repositories can have the same folder name,
- duplicate skills inside one repository need their internal paths to be distinguishable,
- unresolved sources collapse into the generic `unknown` label,
- absolute path context exposes irrelevant machine-specific prefixes.

List and scan must present the same grouping and keyboard behavior so users do not have to learn two table models.

## Goal

Make source repositories and source folders the primary browsing level in TUI list and scan views. Both views should open with every source group collapsed, let users expand with Right and collapse with Left, and show privacy-safe path context that distinguishes duplicate or unresolved sources without displaying a home directory or user name.

## Non-goals

- Do not change inventory, scanning, exposure resolution, import, or removal semantics.
- Do not add group-level bulk import or removal.
- Do not add persistent expansion preferences across application restarts.
- Do not add mouse interaction, filtering, search, or arbitrary tree depth.
- Do not change the non-interactive `skills-manager list` and `skills-manager scan` CLI output in this slice.
- Do not replace numbered duplicate disambiguation where it is still needed.

## User stories

1. As a user with many skills, I want the initial table grouped and collapsed by source, so that I can understand my sources before browsing individual skills.
2. As a keyboard user, I want Right to expand a selected source group, so that I can inspect its skills without opening a dialog.
3. As a keyboard user, I want Left to collapse an expanded source group, so that I can quickly return to the source overview.
4. As a user moving between list and scan, I want identical grouping and navigation rules, so that both views feel like one interface.
5. As a user with duplicate repository names, I want a short path suffix beside the source name, so that I can tell the groups apart.
6. As a user with duplicate skill names in one repository, I want each child row to show its repository-relative path, so that I can select the intended skill.
7. As a privacy-conscious user, I want displayed paths to omit home directories and user names, so that screenshots and terminal output do not leak local machine details.
8. As a user with unresolved sources, I want `unknown` groups to include useful path context, so that they are not merged into one misleading group.
9. As a user performing import or removal, I want actions to apply only to selected skill rows, so that expanding a group never implies a bulk mutation.

## Proposed experience

List and scan render a two-level tree inside their existing tables. Source groups are the top level and skill rows are children. On first load, only group rows are visible:

```text
SKILL                                      SOURCE
> marketingskills · pgit/marketingskills  72 skills
> skills · external/skills                28 skills
> skills · pgit/skills                     6 skills
> unknown · .codex/skills                  4 skills
```

The group marker is `>` when collapsed and `v` when expanded. The selected group expands with Right:

```text
v marketingskills · pgit/marketingskills  72 skills
    marketing-psychology                  .agents/skills/marketing-psychology
    marketing-psychology                  skills/marketing-psychology
    programmatic-seo                      .agents/skills/programmatic-seo
```

For a skill discovered inside a Git repository, the child path is relative to the repository root. For a source without a repository root, the displayed path is a stable suffix containing at most the final two parent folders plus the skill folder, for example `.codex/skills/pdf`. No displayed path includes `/Users/<name>`, `/home/<name>`, or another absolute root prefix.

Group identity is based on the resolved repository root when available. The display label combines repository name with a short path suffix so repositories with the same name remain distinct. When no repository root is available, grouping uses the privacy-safe source-container suffix rather than merging all `unknown` rows.

Up and Down move through visible group and child rows only. Right expands a collapsed group. Left collapses an expanded group. Left on a child selects its parent group; a second Left collapses it. Right on an expanded group selects its first child when one exists. Groups with one skill still use the same collapsed-by-default behavior.

Import and remove shortcuts remain skill-level actions. Pressing `i` or `x` on a group row shows a short message to select a skill inside the group. Refresh preserves expansion for source identities that still exist; new groups start collapsed. Enter and existing staged-plan safety behavior remain unchanged.

## Requirements

- List and scan TUI views must use one shared source-grouping and visible-row navigation model.
- Every source group must be collapsed on first load.
- Group rows must render `>` when collapsed and `v` when expanded.
- Right must expand a selected collapsed group.
- Left must collapse a selected expanded group.
- Left on a child row must select its parent group.
- Right on an expanded group must select its first child when present.
- Up and Down must navigate only rows currently visible after expansion is applied.
- Group rows must show a source name, privacy-safe path suffix, and skill count.
- Git-backed skills must show repository-relative child paths.
- Non-Git or unresolved skills must show at most the final two parent folders plus the skill folder.
- Displayed source context must never include an absolute home path or user name.
- Distinct repository roots with the same repository name must remain separate groups.
- Unresolved rows from distinct source containers must remain separate groups.
- List `i`, list `x`, and scan `i` must operate only on child skill rows.
- Refresh must preserve expansion for surviving group identities and reset invalid selection safely.
- Empty list and scan results must retain their current empty-state behavior.

## Success criteria

- Opening list or scan with many skills shows a compact source overview rather than all skill rows.
- A user can expand, browse, and collapse any source using only arrow keys.
- The same key sequence behaves the same way in list and scan.
- Duplicate skills from `.agents/skills` and `skills` can be distinguished without absolute paths.
- No source or child label displays the current user's home-directory prefix.
- Existing skill-level import, remove, refresh, and staged-plan behavior continues to work.

## Edge cases

- A repository contains only one skill.
- Two repository roots have the same final folder name.
- A scanned skill has a repository name but no remote URL.
- A physical-copy exposure has no matching scan result or repository root.
- Refresh removes the selected child or its entire group.
- Terminal height changes while an expanded group is selected.
- A long relative path must be truncated by the existing table layout.

## Implementation decisions

- Build a shared TUI presentation projection that groups inventory rows and scan results without changing their domain identity or mutation behavior.
- Use canonical repository root as the stable group key when available.
- Use a normalized privacy-safe source-container suffix as the fallback group key for unresolved sources.
- Derive Git-backed child labels from existing repository-relative scan metadata. Derive unresolved labels from a bounded path suffix.
- Keep expansion state keyed by source-group identity, separate from selected visible-row index and viewport offset.

## Testing decisions

- Add unit tests for grouping by repository root, same-name repositories, unresolved source containers, and stable sort order.
- Add path-formatting tests for repository-relative paths, bounded fallback suffixes, root-level skills, and home-prefix removal.
- Add shared navigation tests for initial collapse, Right expansion, Left collapse, child-to-parent movement, visible-row scrolling, refresh, resize, and empty data.
- Add list and scan state tests proving both modes use the same grouping behavior.
- Add action-routing tests proving group rows cannot start import or removal while child rows retain existing staged-plan routing.

## Tasks

- [ ] Add tests for shared source grouping and privacy-safe path labels.
- [ ] Add tests for collapsed visible-row navigation in list and scan.
- [ ] Introduce the shared source-group table projection.
- [ ] Render collapsed and expanded group rows in both tables.
- [ ] Route Left and Right keys through shared group navigation.
- [ ] Preserve skill-level import/remove behavior and refresh expansion state.
- [ ] Update TUI help, footer hints, and durable feature documentation.
