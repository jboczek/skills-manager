#!/usr/bin/env bash
set -euo pipefail

tag="${1:?release tag is required}"
case "$tag" in
  v[0-9]*.[0-9]*.[0-9]*) ;;
  *) echo "invalid release tag: $tag" >&2; exit 1 ;;
esac

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
repo_root="$(CDPATH= cd -- "$script_dir/.." && pwd)"
version="${tag#v}"
metadata="$(cd "$repo_root" && cargo metadata --no-deps --format-version 1)"
cargo_version="$(printf '%s' "$metadata" | ruby -rjson -e 'puts JSON.parse(STDIN.read).fetch("packages").fetch(0).fetch("version")')"
tag_commit="$(cd "$repo_root" && git rev-parse --verify "$tag^{commit}")"
release_ref="refs/remotes/origin/release/$tag"
release_commit="$(cd "$repo_root" && git rev-parse --verify "$release_ref^{commit}")"

cd "$repo_root"
ruby -I"$script_dir" -rrelease_contract -e '
  tag, version, tag_commit, release_commit = ARGV
  abort "release identity mismatch" unless ReleaseContract.valid_identity?(tag, version, tag_commit, release_commit, ["release/#{tag}"])
' "$tag" "$cargo_version" "$tag_commit" "$release_commit"

echo "validated $tag at $tag_commit from release/$tag"
