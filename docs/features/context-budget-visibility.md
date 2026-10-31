# Global Context Budget Visibility

Skills Manager shows a directional estimate of the discovery metadata exposed
globally to each enabled configured agent in the TUI status feed:

```text
• Estimated tokens (global): Claude Code ~776 (36 skills) · Codex ~6394 (201 skills) · GitHub Copilot ~1568 (72 skills)
```

The values are calculated after inventory load or refresh with the
`o200k_base` tokenizer. Each contributing count is the number of unique global
inventory rows exposed to that agent; one row is counted at most once even if
it has duplicate exposures for the same agent. The same shared global row is
counted independently for every agent that receives it.

For Claude Code and GitHub Copilot, the serialized estimate is the metadata
`name` plus its `description`. For Codex, the selected exposed skill path plus
`SKILL.md` is appended because its discovery catalog includes the location.
The name falls back to the inventory skill name when frontmatter is missing or
invalid, and a non-string description is ignored.

Project-local exposures never contribute. The estimate excludes the skill
body, referenced files, scripts, assets, tool schemas, plugin metadata,
provider prompt wrappers, and hidden system context. It is not a billing,
cache, context-window, or exact runtime-prompt measurement. Initial loading is
shown as `(loading)` and a failed initial load as `unavailable`; a failed later
refresh leaves the last successful estimate visible alongside the existing
refresh error.

The standalone `skills-manager list` and `skills-manager scan` commands do not
include this estimate.
