#!/usr/bin/env bash
set -euo pipefail

tag="${1:?release tag is required}"
target="${2:?Rust target is required}"
deployment_target="${3:?deployment target is required}"
dist_dir="${4:-dist}"
version="${tag#v}"

case "$target" in
  aarch64-apple-darwin) expected_arch="arm64" ;;
  x86_64-apple-darwin) expected_arch="x86_64" ;;
  *) echo "unsupported Rust target: $target" >&2; exit 1 ;;
esac

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
repo_root="$(CDPATH= cd -- "$script_dir/.." && pwd)"
binary="$repo_root/target/$target/release/skills-manager"
archive="$dist_dir/skills-manager-$tag-$target.tar.gz"

cd "$repo_root"
rustc_path="$(rustup which rustc)"
MACOSX_DEPLOYMENT_TARGET="$deployment_target" RUSTC="$rustc_path" cargo build --locked --release --target "$target"
test -f "$binary"
codesign --force --sign - --timestamp=none "$binary"

version_output="$($binary --version)"
case "$version_output" in
  "skills-manager $version"*) ;;
  *) echo "unexpected version output: $version_output" >&2; exit 1 ;;
esac

test "$(lipo -archs "$binary")" = "$expected_arch"
actual_deployment="$(otool -l "$binary" | awk '
  /LC_BUILD_VERSION/ { in_build=1; next }
  in_build && $1 == "minos" { print $2; exit }
  /LC_VERSION_MIN_MACOSX/ { in_min=1; next }
  in_min && $1 == "version" { print $2; exit }
')"
test "$actual_deployment" = "$deployment_target"
codesign --verify --strict --verbose=2 "$binary"
signature_info="$(codesign --display --verbose=4 "$binary" 2>&1)"
printf '%s\n' "$signature_info" | grep -q '^Signature=adhoc$'

package_dir="$(mktemp -d "${TMPDIR:-/tmp}/skills-manager-package.XXXXXX")"
trap 'rm -rf "$package_dir"' EXIT
mkdir -p "$dist_dir"
install -m 755 "$binary" "$package_dir/skills-manager"
cp "$repo_root/LICENSE" "$package_dir/LICENSE"
cp "$repo_root/README.md" "$package_dir/README.md"

tar -czf "$archive" -C "$package_dir" skills-manager LICENSE README.md
actual_entries="$(tar -tzf "$archive" | sed 's#/$##' | sort)"
expected_entries="$(printf '%s\n' LICENSE README.md skills-manager)"
test "$actual_entries" = "$expected_entries"
test "$(tar -tvzf "$archive" | awk '$NF == "skills-manager" {print substr($1, 1, 10)}')" = "-rwxr-xr-x"

echo "created $archive"
