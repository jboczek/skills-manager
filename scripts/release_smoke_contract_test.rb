require "minitest/autorun"

class ReleaseSmokeContractTest < Minitest::Test
  WORKFLOW = File.expand_path("../.github/workflows/release-smoke.yml", __dir__)

  def setup
    assert File.file?(WORKFLOW), "release smoke workflow is missing"
    @workflow = File.read(WORKFLOW)
  end

  def test_smoke_workflow_covers_the_required_native_os_matrix
    assert_includes @workflow, "workflow_dispatch:"
    assert_includes @workflow, "runs-on: ${{ matrix.runner }}"
    assert_includes @workflow, "strategy:"
    assert_includes @workflow, "matrix:"
    {
      "macos-14" => ["arm64", "14", "aarch64-apple-darwin", "14.0"],
      "macos-15" => ["arm64", "15", "aarch64-apple-darwin", "14.0"],
      "macos-26" => ["arm64", "26", "aarch64-apple-darwin", "14.0"],
      "macos-15-intel" => ["x86_64", "15", "x86_64-apple-darwin", "15.0"],
      "macos-26-intel" => ["x86_64", "26", "x86_64-apple-darwin", "15.0"]
    }.each do |runner, (architecture, os_major, target, deployment_target)|
      assert_includes @workflow, "runner: #{runner}"
      assert_includes @workflow, "architecture: #{architecture}"
      assert_includes @workflow, "os_major: \"#{os_major}\""
      assert_includes @workflow, "target: #{target}"
      assert_includes @workflow, "deployment_target: \"#{deployment_target}\""
    end
    assert_includes @workflow, 'test "$(uname -m)" = "$ARCHITECTURE"'
    assert_includes @workflow, 'test "$(sw_vers -productVersion | cut -d. -f1)" = "$OS_MAJOR"'
    assert_includes @workflow, "gh release download"
    assert_includes @workflow, "lipo -archs"
    assert_includes @workflow, 'grep -q "Mach-O 64-bit executable $ARCHITECTURE"'
    assert_includes @workflow, "codesign --verify --strict"
    assert_includes @workflow, "--version"
  end
end
