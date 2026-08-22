require "minitest/autorun"

TAP_TEMPLATE_ROOT = File.expand_path("..", __dir__)

class TapWorkflowContractTest < Minitest::Test
  def test_publisher_is_a_pinned_manual_dispatch_workflow
    workflow = File.read(File.join(TAP_TEMPLATE_ROOT, ".github/workflows/publish-skills-manager.yml"))

    assert_includes workflow, "workflow_dispatch:"
    assert_includes workflow, "      tag:\n"
    assert_includes workflow, "ruby scripts/publish.rb"
    assert_includes workflow, "contents: write"
    assert_includes workflow, "pull-requests: write"
    assert_includes workflow, "attestations: read"
    refute_includes workflow, "pull_request_target"
    assert workflow.scan(/uses:\s+[^\s]+@([0-9a-f]{40})/).length >= 1
    refute_match(/uses:\s+[^\s]+@(?![0-9a-f]{40})/, workflow)
  end

  def test_native_formula_checks_have_the_required_commands_and_names
    workflow = File.read(File.join(TAP_TEMPLATE_ROOT, ".github/workflows/formula-checks.yml"))

    assert_includes workflow, "name: formula (${{ matrix.name }})"
    assert_includes workflow, "runner: macos-14"
    assert_includes workflow, "runner: macos-15-intel"
    assert_includes workflow, "      - scripts/publish.rb\n"
    assert_includes workflow, "      - scripts/publisher_contract.rb\n"
    assert_includes workflow, "      - .github/workflows/formula-checks.yml\n"
    assert_includes workflow, "Homebrew/actions/setup-homebrew@"
    assert_includes workflow, "if [[ -f Formula/skills-manager.rb ]]"
    assert_includes workflow, "ruby -I scripts test/publisher_contract_test.rb"
    assert_includes workflow, "ruby test/tap_workflow_contract_test.rb"
    assert_includes workflow, "brew test-bot --only-formulae --build-from-source"
    assert_includes workflow, "brew audit --new --formula skills-manager"
    assert_includes workflow, "brew style --formula skills-manager"
    assert_includes workflow, "brew audit --strict --online --os=all --arch=all --formula skills-manager"
    assert_includes workflow, "HOMEBREW_NO_INSTALL_FROM_API=1 brew install --build-from-source"
    assert_includes workflow, "brew test skills-manager"
    assert_includes workflow, "skills-manager --version"
  end

  def test_tap_template_does_not_include_a_bottle_publisher
    refute File.exist?(File.join(TAP_TEMPLATE_ROOT, ".github/workflows/publish.yml"))
  end
end
