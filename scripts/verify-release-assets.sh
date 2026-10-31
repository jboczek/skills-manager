#!/usr/bin/env bash
set -euo pipefail

tag="${1:?release tag is required}"
dist_dir="${2:?asset directory is required}"
arm_archive="skills-manager-$tag-aarch64-apple-darwin.tar.gz"
intel_archive="skills-manager-$tag-x86_64-apple-darwin.tar.gz"
sums_file="$dist_dir/SHA256SUMS"

test -d "$dist_dir"
test -f "$sums_file"
actual_files="$(cd "$dist_dir" && find . -maxdepth 1 -type f -print | sed 's#^./##' | sort)"
expected_files="$(printf '%s\n' "$arm_archive" "$intel_archive" SHA256SUMS | sort)"
test "$actual_files" = "$expected_files"
test "$(awk 'NF { count += 1 } END { print count + 0 }' "$sums_file")" -eq 2
test "$(awk 'NF && NF != 2 { count += 1 } END { print count + 0 }' "$sums_file")" -eq 0

for archive in "$arm_archive" "$intel_archive"; do
  archive_path="$dist_dir/$archive"
  test -f "$archive_path"
  expected_count="$(awk -v name="$archive" '$2 == name {count += 1} END {print count + 0}' "$sums_file")"
  test "$expected_count" -eq 1
  expected_sum="$(awk -v name="$archive" '$2 == name {print $1}' "$sums_file")"
  actual_sum="$(shasum -a 256 "$archive_path" | awk '{print $1}')"
  test "$actual_sum" = "$expected_sum"
  entries="$(tar -tzf "$archive_path" | sed 's#/$##' | sort)"
  test "$entries" = "$(printf '%s\n' LICENSE README.md skills-manager)"
done

echo "validated release assets for $tag"
