require "minitest/autorun"

class ReleaseSmokeContractTest < Minitest::Test
  WORKFLOW = File.expand_path("../.github/workflows/release-smoke.yml", __dir__)

  def setup
    assert File.file?(WORKFLOW), "release smoke workflow is missing"
    @workflow = File.read(WORKFLOW)
  end

  def test_smoke_workflow_runs_the_arm64_release_on_macos_15
    assert_includes @workflow, "workflow_dispatch:"
    assert_includes @workflow, "runs-on: macos-15"
    assert_includes @workflow, 'test "$(uname -m)" = "arm64"'
    assert_includes @workflow, 'test "$(sw_vers -productVersion | cut -d. -f1)" = "15"'
    assert_includes @workflow, "aarch64-apple-darwin"
    assert_includes @workflow, "gh release download"
    assert_includes @workflow, "lipo -archs"
    assert_includes @workflow, "codesign --verify --strict"
    assert_includes @workflow, "--version"
  end
end
