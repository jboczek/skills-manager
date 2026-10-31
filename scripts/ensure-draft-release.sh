#!/usr/bin/env bash
set -euo pipefail

tag="${1:?release tag is required}"
dist_dir="${2:?asset directory is required}"
version="${tag#v}"
expected_assets="$(printf '%s\n' "skills-manager-$tag-aarch64-apple-darwin.tar.gz" "skills-manager-$tag-x86_64-apple-darwin.tar.gz" SHA256SUMS | sort)"

if release_json="$(gh release view "$tag" --json isDraft,assets 2>/dev/null)"; then
  test "$(printf '%s' "$release_json" | jq -r '.isDraft')" = true
  asset_names="$(printf '%s' "$release_json" | jq -r '.assets[].name' | sort)"
  test "$asset_names" = "$expected_assets"

  download_dir="$(mktemp -d "${TMPDIR:-/tmp}/skills-manager-release.XXXXXX")"
  trap 'rm -rf "$download_dir"' EXIT
  gh release download "$tag" --pattern "skills-manager-$tag-aarch64-apple-darwin.tar.gz" --pattern "skills-manager-$tag-x86_64-apple-darwin.tar.gz" --pattern SHA256SUMS --dir "$download_dir"
  for asset in $expected_assets; do
    cmp -s "$dist_dir/$asset" "$download_dir/$asset"
  done
  echo "matching immutable draft already exists for $tag"
else
  gh release create "$tag" \
    "$dist_dir/skills-manager-$tag-aarch64-apple-darwin.tar.gz" \
    "$dist_dir/skills-manager-$tag-x86_64-apple-darwin.tar.gz" \
    "$dist_dir/SHA256SUMS" \
    --draft --generate-notes --verify-tag --title "skills-manager $version"
  echo "created draft release for $tag"
fi
