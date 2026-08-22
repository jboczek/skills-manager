# Homebrew distribution map

Status: approved for the first macOS release.

## Repository model

- Source: `jboczek/skills-manager`.
- Shared public tap: `jboczek/homebrew-tap`, exposed to Homebrew as
  `jboczek/tap`.
- The formula and tap publisher belong to the tap repository. The source
  repository contains release/build automation and the reproducible tap
  template used to initialize that repository; it does not place a formula in
  the application root.
- The supported direct install is:

  ```sh
  brew install jboczek/tap/skills-manager
  ```

The tap can contain future CLI formulae, but this release adds only
`skills-manager`.

## Release contract

- The executable name is `skills-manager`, from the Cargo binary target.
- The version is read from `Cargo.toml` and must match `vX.Y.Z`, the commit on
  `release/vX.Y.Z`, and the GitHub Release tag.
- Native build jobs use `macos-14`/`aarch64-apple-darwin` with
  `MACOSX_DEPLOYMENT_TARGET=14.0` and `macos-15-intel`/`x86_64-apple-darwin`
  with `MACOSX_DEPLOYMENT_TARGET=15.0`.
- The immutable assets are exactly:

  ```text
  skills-manager-vX.Y.Z-aarch64-apple-darwin.tar.gz
  skills-manager-vX.Y.Z-x86_64-apple-darwin.tar.gz
  SHA256SUMS
  ```

- Each archive has `skills-manager`, `LICENSE`, and `README.md` at its root;
  the executable bit, Mach-O architecture, deployment target, version output,
  and ad-hoc signature are checked before packaging.
- The release is created as a draft with GitHub-generated notes. It becomes
  public only after its checks pass. Published tags and assets are never
  overwritten.

## Formula contract

`Formula/skills-manager.rb` is the only formula in the first tap change. It
contains an explicit `version`, `depends_on :macos`, architecture-specific
`on_arm` and `on_intel` URL/SHA-256 pairs, `bin.install "skills-manager"`, and
a `skills-manager --version` test. It has no bottles, `latest`, `:no_check`, or
`Hardware::CPU` logic.

## Tap publication contract

The tap-side publisher accepts a release tag, downloads the release assets,
verifies the source commit, GitHub artifact attestations, and SHA-256 values,
renders the formula from trusted code in the tap, and opens or updates one
`automation/skills-manager-vX.Y.Z` PR. Before pushing, it rejects every diff
other than the single formula file. The PR requires native ARM64 and Intel
audit/install/test checks; auto-merge is enabled only after both pass.

See [issue 03](issues/03-use-shared-public-homebrew-tap.md), [issue 06](issues/06-select-release-build-and-artifact-architecture.md),
[issue 07](issues/07-select-homebrew-formula-and-tap-workflow.md),
[issue 08](issues/08-design-release-authentication-and-supply-chain-controls.md),
and [issue 10](issues/10-decide-release-state-machine-and-recovery-policy.md).
