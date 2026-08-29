#!/usr/bin/env bash
set -euo pipefail

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
repo_root="$(CDPATH= cd -- "$script_dir/.." && pwd)"
cd "$repo_root"

git rev-parse --show-toplevel >/dev/null 2>&1 || die "run this script inside a Git repository"

current_branch="$(git symbolic-ref --quiet --short HEAD || true)"
[ "$current_branch" = "main" ] || die "run this script from the local main branch"

[ -z "$(git status --porcelain --untracked-files=all)" ] || die "main has uncommitted or untracked changes"

git fetch origin main --tags

local_main="$(git rev-parse refs/heads/main)"
remote_main="$(git rev-parse refs/remotes/origin/main)"
[ "$local_main" = "$remote_main" ] || die "local main is not synchronized with origin/main"

cargo_version="$(
  cargo metadata --no-deps --locked --format-version 1 |
    ruby -rjson -e '
      metadata = JSON.parse(STDIN.read)
      package = metadata.fetch("packages").find { |candidate| candidate.fetch("name") == "skills-manager" }
      abort "skills-manager package is missing" unless package
      puts package.fetch("version")
    '
)"

ruby -rrubygems -e 'exit(ARGV.fetch(0).match?(/\A\d+\.\d+\.\d+\z/) ? 0 : 1)' "$cargo_version" ||
  die "Cargo version must be X.Y.Z: $cargo_version"

latest_tag="$(
  git ls-remote --tags --refs origin 'v*' |
    ruby -rrubygems -e '
      tags = STDIN.each_line.map { |line| line.split("\t", 2).last.to_s.strip.sub(%r{\Arefs/tags/}, "") }
      tags.select! { |tag| tag.match?(/\Av\d+\.\d+\.\d+\z/) }
      puts tags.max_by { |tag| Gem::Version.new(tag[1..-1]) } if tags.any?
    '
)"

if [ -n "$latest_tag" ]; then
  latest_version="${latest_tag#v}"
  ruby -rrubygems -e \
    'exit(Gem::Version.new(ARGV.fetch(0)) > Gem::Version.new(ARGV.fetch(1)) ? 0 : 1)' \
    "$cargo_version" "$latest_version" ||
    die "Cargo version $cargo_version must be greater than the latest published tag $latest_tag"
fi

release_branch="release/v$cargo_version"
release_tag="v$cargo_version"

if git show-ref --verify --quiet "refs/heads/$release_branch" ||
  git ls-remote --exit-code --heads origin "refs/heads/$release_branch" >/dev/null 2>&1; then
  die "release branch already exists: $release_branch"
fi

if git show-ref --verify --quiet "refs/tags/$release_tag" ||
  git ls-remote --exit-code --tags origin "refs/tags/$release_tag" >/dev/null 2>&1; then
  die "release tag already exists: $release_tag"
fi

git switch --create "$release_branch" refs/remotes/origin/main
git push --set-upstream origin "$release_branch"
git tag --annotate "$release_tag" --message "skills-manager $cargo_version"

if ! git push origin "$release_tag"; then
  printf 'error: branch was pushed but tag %s was not; retry with: git push origin %s\n' \
    "$release_tag" "$release_tag" >&2
  exit 1
fi

printf 'created %s and pushed tag %s\n' "$release_branch" "$release_tag"
