require "minitest/autorun"
require_relative "release_contract"

class ReleaseContractTest < Minitest::Test
  VERSION = "0.1.0"
  TAG = "v#{VERSION}"
  ARM_ARCHIVE = "skills-manager-v#{VERSION}-aarch64-apple-darwin.tar.gz"
  INTEL_ARCHIVE = "skills-manager-v#{VERSION}-x86_64-apple-darwin.tar.gz"

  def test_release_identity_requires_matching_version_and_release_branch
    assert ReleaseContract.valid_identity?(TAG, VERSION, "abc123", "abc123", ["release/#{TAG}"])
    refute ReleaseContract.valid_identity?("v0.2.0", VERSION, "abc123", "abc123", ["release/#{TAG}"])
    refute ReleaseContract.valid_identity?(TAG, VERSION, "abc123", "def456", ["release/#{TAG}"])
    refute ReleaseContract.valid_identity?(TAG, VERSION, "abc123", "abc123", ["main"])
  end

  def test_release_assets_are_exactly_the_two_native_archives
    names = [ARM_ARCHIVE, INTEL_ARCHIVE]
    sums = {ARM_ARCHIVE => "a" * 64, INTEL_ARCHIVE => "b" * 64}

    assert ReleaseContract.valid_assets?(VERSION, names, sums)
    refute ReleaseContract.valid_assets?(VERSION, names + ["unexpected.txt"], sums)
    refute ReleaseContract.valid_assets?(VERSION, names, sums.merge("other.tar.gz" => "c" * 64))
    refute ReleaseContract.valid_assets?(VERSION, names, sums.merge(ARM_ARCHIVE => "short"))
  end

  def test_archive_layout_requires_one_executable_and_documentation
    assert ReleaseContract.valid_archive_entries?(["skills-manager", "LICENSE", "README.md"], ["skills-manager"])
    refute ReleaseContract.valid_archive_entries?(["skills-manager", "README.md"], ["skills-manager"])
    refute ReleaseContract.valid_archive_entries?(["skills-manager", "LICENSE", "README.md", "bin/other"], ["skills-manager", "bin/other"])
  end
end
