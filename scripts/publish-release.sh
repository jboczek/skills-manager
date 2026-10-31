#!/usr/bin/env bash
set -euo pipefail

tag="${1:?release tag is required}"
download_dir="$(mktemp -d "${TMPDIR:-/tmp}/skills-manager-release.XXXXXX")"
trap 'rm -rf "$download_dir"' EXIT

release_json="$(gh release view "$tag" --json isDraft)"
test "$(printf '%s' "$release_json" | jq -r '.isDraft')" = true
asset_names="$(gh release view "$tag" --json assets --jq '.assets[].name' | sort)"
expected_assets="$(printf '%s\n' "skills-manager-$tag-aarch64-apple-darwin.tar.gz" "skills-manager-$tag-x86_64-apple-darwin.tar.gz" SHA256SUMS | sort)"
test "$asset_names" = "$expected_assets"
gh release download "$tag" --pattern "skills-manager-$tag-aarch64-apple-darwin.tar.gz" --pattern "skills-manager-$tag-x86_64-apple-darwin.tar.gz" --pattern SHA256SUMS --dir "$download_dir"
scripts_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
"$scripts_dir/verify-release-assets.sh" "$tag" "$download_dir"
gh release edit "$tag" --draft=false
echo "published release $tag"
