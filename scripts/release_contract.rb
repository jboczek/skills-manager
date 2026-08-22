module ReleaseContract
  ARCHIVE_TARGETS = %w[aarch64-apple-darwin x86_64-apple-darwin].freeze

  def self.valid_version?(version)
    version.match?(/\A\d+\.\d+\.\d+\z/)
  end

  def self.archive_names(version)
    return [] unless valid_version?(version)

    ARCHIVE_TARGETS.map { |target| "skills-manager-v#{version}-#{target}.tar.gz" }
  end

  def self.valid_identity?(tag, cargo_version, source_commit, release_commit, containing_branches)
    valid_version?(cargo_version) &&
      tag == "v#{cargo_version}" &&
      source_commit == release_commit &&
      containing_branches.include?("release/#{tag}")
  end

  def self.valid_assets?(version, names, checksums)
    expected = archive_names(version)
    names.sort == expected.sort &&
      checksums.keys.sort == expected.sort &&
      checksums.values.all? { |checksum| checksum.match?(/\A[0-9a-fA-F]{64}\z/) }
  end

  def self.valid_archive_entries?(entries, executable_paths)
    entries.sort == %w[LICENSE README.md skills-manager] && executable_paths == ["skills-manager"]
  end
end
