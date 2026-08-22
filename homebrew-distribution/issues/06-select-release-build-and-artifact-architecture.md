---
status: closed
---

# Issue 06: select release build and artifact architecture

Build the first release on native GitHub-hosted macOS runners:

| Runner | Rust target | Deployment target | Archive suffix |
|---|---|---|---|
| `macos-14` | `aarch64-apple-darwin` | `14.0` | `aarch64-apple-darwin` |
| `macos-15-intel` | `x86_64-apple-darwin` | `15.0` | `x86_64-apple-darwin` |

Cross-compilation is intentionally excluded. Each runner packages one
executable plus `LICENSE` and `README.md`, checks the native Mach-O metadata,
and produces one immutable archive. Linux, Windows, bottles, casks, and
Developer ID/notarization are out of scope for this release.
