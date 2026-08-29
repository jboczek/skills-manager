# Homebrew distribution

Skills Manager is distributed for macOS as two native prebuilt archives from
GitHub Releases. Users install the formula from the separate shared tap with:

```sh
brew install jboczek/tap/skills-manager
```

The source release workflow builds `aarch64-apple-darwin` on `macos-14` and
`x86_64-apple-darwin` on `macos-15-intel`. Each archive contains the
`skills-manager` executable, `LICENSE`, and `README.md` at its root. The tap
publisher validates the release tag, source commit, attestations, and hashes
before opening the one-file formula PR.

The tap formula is intentionally kept out of this application repository. Its
trusted renderer and native ARM64/Intel workflow mirror live under
`homebrew-distribution/tap/`; the public `jboczek/homebrew-tap` repository is
the active publisher and has protected `main` with both native checks required.

The published archives can also be smoke-tested directly on GitHub-hosted
macOS runners with the source repository's manually dispatched `Release smoke`
workflow. Its matrix covers ARM64 on macOS 14, 15, and 26 plus Intel on macOS
15 and 26. Each job downloads only the requested immutable tag and checks the
archive layout, architecture, deployment target, signature, and `--version`
output before succeeding.

## In-app updates

On TUI startup, Skills Manager checks `brew outdated --json=v2 skills-manager`
asynchronously. If Homebrew reports a newer formula version, the header shows
the version and directs the user to `/update`. That command runs `brew update`
followed by `brew upgrade skills-manager`, closes the TUI cleanly, and starts
the upgraded executable again.
