require "minitest/autorun"
require_relative "../scripts/publisher_contract"

class PublisherContractTest < Minitest::Test
  VERSION = "0.1.0"
  ARM_SHA = "a" * 64
  INTEL_SHA = "b" * 64

  def test_formula_is_architecture_specific_and_installs_the_binary
    formula = PublisherContract.render_formula(VERSION, ARM_SHA, INTEL_SHA)

    assert_includes formula, 'version "0.1.0"'
    assert_includes formula, "depends_on :macos"
    assert_includes formula, "on_arm do"
    assert_includes formula, "on_intel do"
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
end
