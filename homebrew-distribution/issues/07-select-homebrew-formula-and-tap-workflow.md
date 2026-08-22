---
status: closed
---

# Issue 07: select the formula and tap workflow

Use one Ruby formula in `Formula/skills-manager.rb`. The publisher renders
literal release URLs and SHA-256 values into `on_arm` and `on_intel` blocks,
installs the root executable with `bin.install`, and tests the installed
version. The formula is macOS-only and has no bottle or source-build logic.

The tap workflow is the only publisher. It runs from a protected `main`, uses
trusted scripts checked into the tap, validates the release before changing a
branch, permits only a one-file formula diff, and opens or updates the
version-specific automation PR. Native ARM64 and Intel checks are required
before auto-merge.
