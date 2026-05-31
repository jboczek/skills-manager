# Documentation Maintenance Guide

This guide is for AI coding agents working in this repository. Use it to find the right product documentation and keep `/docs` aligned with implementation changes.

## Documentation Map

- `docs/big_picture.md`: high-level application context. It should explain what the application does, what problem it solves, and the broad product direction. If this file is missing or outdated, suggest creating or updating it with the `big-picture` skill.
- `docs/roadmap.md`: planned versions and the PRDs expected in each version. Link to PRDs when they exist. If this file is missing or outdated, suggest creating or updating it with the `roadmap` skill.
- `docs/prds/`: PRDs for planned, active, or completed product work. Follow the local PRD format or use the `to-prd` skill when creating a new PRD.
- `docs/features/`: durable feature documentation for implemented behavior. A feature doc may describe behavior delivered across several PRDs.
- `docs/changelog.md`: short reverse-chronological log of changes.
- `docs/adrs/`: architectural decision records for important technical trade-offs. Use when a decision is too detailed for a PRD but important enough to document.

## Maintenance Rules

After completing implementation and docs change:

- Update the relevant PRD status or progress notes in `docs/prds/` when a PRD exists.
- Update the relevant feature doc in `docs/features/` so it reflects the implemented behavior.
- Update the relevant ADR in `docs/adrs/` if a significant architectural decision has been made.
- If no matching feature doc exists, suggest creating one and create it when the user asks or the task clearly requires it.
- Add a short entry near the top of `docs/changelog.md`.

## Changelog Format

Group entries by day. Each chunk of changes should get its own row, even when multiple people or agents update the repository on the same day.

```markdown
## 2026-05-30

| Time | Change | Docs |
|---|---|---|
| 2026-05-30 14:35 | Added export flow validation. | `features/export-flow.md`, `prds/export-validation.md` |
```

Use the repository's local timezone if known. Keep entries short and factual.

## Missing Docs

Do not block implementation only because one of these documents is missing. When documentation is missing, mention it clearly and suggest the smallest useful next document to create.

Keep this guide short. Detailed PRD, roadmap, big-picture, and feature-document structure belongs in the relevant skills or local project conventions.