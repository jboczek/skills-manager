# AGENTS.md

## Instructions

1. Start implementation with TDD: start with a unit test to cover what should be implemented.
2. After writing code, list what could break and suggest tests to cover it.
3. When there is a bug, start by writing a test that reproduces it, then fix it until the test passes.
4. Every time the user corrects the agent, add a new rule to this file so it does not happen again.
5. Minimum code that solves the problem. Nothing speculative. Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.
    No features beyond what was asked.
    No abstractions for single-use code.
    No "flexibility" or "configurability" that wasn't requested.
    No error handling for impossible scenarios.
    If you write 200 lines and it could be 50, rewrite it.
6. Touch only what you must. Clean up only your own mess.
7. After implementation, ask yourself: "Am I proud of this code? Would I want to show it to a senior engineer?" If not, rewrite it until you are.
8. `/docs/AGENTS.md` is a set of rules for maintaining the documentation in this repository. Follow these rules when updating docs or suggesting doc changes. 
9. When completing PRD work, update the README, matching `docs/features/` document, PRD status/progress notes, roadmap status, and changelog before considering the branch complete.

Before implementation, make a plan and add it to temp file named after the PRD `/docs/temp/prd-xxx-plan.md`. The plan should be a simple step-by-step outline of how. Add the plan file but do not commit it.

## Feature branches
- Create a new branch for each feature or bugfix, named after the PRD or issue it addresses (e.g., `prd-001-{feature-name}` or `issue-123`).
- Use simple, descriptive commit messages that explain the "what" and "why" of each change.
- Avoid large, monolithic commits. Instead, break your work into smaller, logical commits that are easier to review and understand.
- Never commit files you did not create or change. Always run `git diff --cached --stat` before committing to verify only intended files are staged.
- Never commit files under `docs/temp/` or `docs/preview/` — these are ignored by `.gitignore` and must stay local only.
