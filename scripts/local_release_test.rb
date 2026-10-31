require "fileutils"
require "minitest/autorun"
require "open3"
require "shellwords"

class LocalReleaseTest < Minitest::Test
  SCRIPT = File.expand_path("local-release.sh", __dir__)

  def setup
    assert File.file?(SCRIPT), "local release script is missing"
  end

  def test_creates_matching_release_branch_and_tag_and_pushes_them_in_order
    with_fixture("0.2.0", "0.1.0") do |fixture|
      output = run_script(fixture)

      assert_match(/created release\/v0\.2\.0 and pushed tag v0\.2\.0/, output)
      assert_equal "release/v0.2.0", git(fixture[:checkout], "branch", "--show-current").strip
      assert_equal %w[refs/heads/release/v0.2.0 refs/tags/v0.2.0], File.readlines(fixture[:push_log], chomp: true)

      main_commit = git_remote(fixture[:remote], "rev-parse", "refs/heads/main").strip
      release_commit = git_remote(fixture[:remote], "rev-parse", "refs/heads/release/v0.2.0").strip
      tag_commit = git_remote(fixture[:remote], "rev-parse", "refs/tags/v0.2.0^{commit}").strip
      assert_equal main_commit, release_commit
      assert_equal release_commit, tag_commit
    end
  end

  def test_rejects_a_version_that_is_not_greater_than_the_latest_tag
    with_fixture("0.1.0", "0.1.0") do |fixture|
      stdout, stderr, status = Open3.capture3("bash", fixture[:script], chdir: fixture[:checkout])

      refute status.success?
      assert_match(/greater than the latest published tag/, "#{stdout}#{stderr}")
      refute remote_ref?(fixture[:remote], "refs/heads/release/v0.1.0")
    end
  end

  def test_rejects_a_stale_cargo_lock_before_creating_release_refs
    with_fixture("0.2.0", "0.1.0", stale_lock: true) do |fixture|
      stdout, stderr, status = Open3.capture3("bash", fixture[:script], chdir: fixture[:checkout])

      refute status.success?
      assert_match(/Cargo\.lock is out of date/, "#{stdout}#{stderr}")
      refute remote_ref?(fixture[:remote], "refs/heads/release/v0.2.0")
    end
  end

  private

  def with_fixture(version, published_version, stale_lock: false)
    Dir.mktmpdir("skills-manager-release-test") do |root|
      remote = File.join(root, "remote.git")
      seed = File.join(root, "seed")
      checkout = File.join(root, "checkout")
      push_log = File.join(root, "push.log")

      run_command("git", "init", "--bare", remote)
      run_command("git", "init", seed)
      git(seed, "branch", "-M", "main")
      configure_identity(seed)
      git(seed, "remote", "add", "origin", remote)
      write_project(seed, published_version)
      script_in_seed = File.join(seed, "scripts", "local-release.sh")
      FileUtils.mkdir_p(File.dirname(script_in_seed))
      FileUtils.cp(SCRIPT, script_in_seed)
      git(seed, "add", ".")
      git(seed, "commit", "-m", "initial")
      git(seed, "push", "-u", "origin", "main")
      git(seed, "tag", "-a", "v#{published_version}", "-m", "skills-manager #{published_version}")
      git(seed, "push", "origin", "v#{published_version}")

      unless version == published_version
        write_project(seed, version)
        rewrite_project_lock_version(seed, published_version) if stale_lock
        git(seed, "add", ".")
        git(seed, "commit", "-m", "prepare release")
        git(seed, "push", "origin", "main")
      end
      run_command("git", "clone", "--branch", "main", remote, checkout)
      configure_identity(checkout)

      script = File.join(checkout, "scripts", "local-release.sh")
      install_push_log_hook(remote, push_log)

      yield(remote: remote, checkout: checkout, script: script, push_log: push_log)
    end
  end

  def write_project(directory, version)
    FileUtils.mkdir_p(File.join(directory, "src"))
    File.write(
      File.join(directory, "Cargo.toml"),
      <<~TOML
        [package]
        name = "skills-manager"
        version = "#{version}"
        edition = "2024"
      TOML
    )
    File.write(File.join(directory, "src", "lib.rs"), "")
    run_command("cargo", "generate-lockfile", chdir: directory)
  end

  def rewrite_project_lock_version(directory, version)
    lock_path = File.join(directory, "Cargo.lock")
    lock = File.read(lock_path)
    updated = lock.sub(/(name = "skills-manager"\nversion = ")[^"]+/) do
      "#{Regexp.last_match(1)}#{version}"
    end
    raise "skills-manager package is missing from Cargo.lock" if updated == lock

    File.write(lock_path, updated)
  end

  def install_push_log_hook(remote, push_log)
    hook = File.join(remote, "hooks", "update")
    escaped_log = Shellwords.escape(push_log)
    File.write(hook, "#!/bin/sh\nprintf '%s\\n' \"$1\" >> #{escaped_log}\n")
    FileUtils.chmod(0o755, hook)
  end

  def configure_identity(directory)
    git(directory, "config", "user.name", "Release Test")
    git(directory, "config", "user.email", "release-test@example.com")
  end

  def run_script(fixture)
    stdout, stderr, status = Open3.capture3("bash", fixture[:script], chdir: fixture[:checkout])
    raise "script failed: #{stdout}\n#{stderr}" unless status.success?

    stdout
  end

  def git(directory, *args)
    run_command("git", "-C", directory, *args)
  end

  def git_remote(remote, *args)
    run_command("git", "--git-dir", remote, *args)
  end

  def remote_ref?(remote, ref)
    _stdout, _stderr, status = Open3.capture3("git", "--git-dir", remote, "show-ref", "--verify", "--quiet", ref)
    status.success?
  end

  def run_command(*args, chdir: nil)
    options = chdir ? {chdir: chdir} : {}
    stdout, stderr, status = Open3.capture3(*args, **options)
    raise "#{args.join(" ")} failed: #{stderr}" unless status.success?

    stdout
  end
end
