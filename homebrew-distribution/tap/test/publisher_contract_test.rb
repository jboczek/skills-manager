require "minitest/autorun"
require_relative "../scripts/publisher_contract"

class PublisherContractTest < Minitest::Test
  VERSION = "0.1.0"
  ARM_SHA = "a" * 64
  INTEL_SHA = "b" * 64

  def test_formula_is_architecture_specific_and_installs_the_binary
    formula = PublisherContract.render_formula(VERSION, ARM_SHA, INTEL_SHA)

    assert_includes formula, "version \"0.1.0\"\n  stable.version Version.new(\"0.1.0\", detected_from_url: true)"
    assert_includes formula, "depends_on :macos"
    assert_includes formula, "def self.configure_architecture\n    on_arm do\n      url \"https://github.com/jboczek/skills-manager/releases/download/v0.1.0/skills-manager-v0.1.0-aarch64-apple-darwin.tar.gz\"\n      sha256 \"#{ARM_SHA}\""
    assert_includes formula, "on_intel do\n      url \"https://github.com/jboczek/skills-manager/releases/download/v0.1.0/skills-manager-v0.1.0-x86_64-apple-darwin.tar.gz\"\n      sha256 \"#{INTEL_SHA}\""
    assert_includes formula, "configure_architecture\n  version \"0.1.0\""
    refute_includes formula, "stable do"
    assert_includes formula, ARM_SHA
    assert_includes formula, INTEL_SHA
    assert_includes formula, 'bin.install "skills-manager"'
    assert_includes formula, 'skills-manager --version'
    refute_includes formula, "latest"
    refute_includes formula, ":no_check"
    refute_includes formula, "Hardware::CPU"
  end

  def test_formula_inputs_require_literal_sha256_values
    assert PublisherContract.valid_formula_inputs?(VERSION, ARM_SHA, INTEL_SHA)
    refute PublisherContract.valid_formula_inputs?(VERSION, "not-a-hash", INTEL_SHA)
    refute PublisherContract.valid_formula_inputs?("1.0", ARM_SHA, INTEL_SHA)
  end

  def test_only_the_formula_file_is_an_allowed_tap_diff
    assert PublisherContract.allowed_diff?(["Formula/skills-manager.rb"])
    refute PublisherContract.allowed_diff?(["Formula/skills-manager.rb", "README.md"])
    refute PublisherContract.allowed_diff?([])
    refute PublisherContract.allowed_diff?(["Formula/other.rb"])
  end

  def test_retry_requires_the_same_tag_commit_and_exact_diff
    expected = {"tag" => "v0.1.0", "source_commit" => "abc123", "diff" => ["Formula/skills-manager.rb"]}
    assert PublisherContract.retry_matches?(expected, expected.dup)
    refute PublisherContract.retry_matches?(expected, expected.merge("source_commit" => "def456"))
    refute PublisherContract.retry_matches?(expected, expected.merge("diff" => ["Formula/skills-manager.rb", "README.md"]))
  end

  def test_publisher_sets_a_noninteractive_git_identity_before_committing
    publisher = File.read(File.expand_path("../scripts/publish.rb", __dir__))

    assert_includes publisher, '"config", "user.name", "github-actions[bot]"'
    assert_includes publisher, '"config", "user.email", "41898282+github-actions[bot]@users.noreply.github.com"'
  end

  def test_publisher_verifies_the_release_workflow_and_tag_provenance
    publisher = File.read(File.expand_path("../scripts/publish.rb", __dir__))

    assert_includes publisher, '"--signer-workflow", "#{source_repository}/.github/workflows/release.yml"'
    assert_includes publisher, '"--source-ref", "refs/tags/#{tag}"'
  end

  def test_existing_pr_retry_refreshes_its_branch_from_current_main
    publisher = File.read(File.expand_path("../scripts/publish.rb", __dir__))

    assert_includes publisher, 'git_output("merge", "--no-edit", "origin/main")'
    assert_includes publisher, 'system("git", "push", "origin", branch)'
  end
end
