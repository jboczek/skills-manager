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
