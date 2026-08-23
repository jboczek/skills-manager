require "base64"
require "digest"
require "fileutils"
require "json"
require "open3"
require "shellwords"
require "tmpdir"

require_relative "publisher_contract"

def gh_output(*args)
  stdout, stderr, status = Open3.capture3("gh", *args)
  abort "gh #{args.join(" ")} failed: #{stderr}" unless status.success?

  stdout
end

def git_output(*args)
  stdout, stderr, status = Open3.capture3("git", *args)
  abort "git #{args.join(" ")} failed: #{stderr}" unless status.success?

  stdout
end

def fail_unless(condition, message)
  abort message unless condition
end

tag = ENV.fetch("TAG")
source_repository = ENV.fetch("SOURCE_REPOSITORY", "jboczek/skills-manager")
fail_unless(tag.match?(/\Av\d+\.\d+\.\d+\z/), "invalid release tag: #{tag}")
version = tag.delete_prefix("v")
archive_names = [
  "skills-manager-#{tag}-aarch64-apple-darwin.tar.gz",
  "skills-manager-#{tag}-x86_64-apple-darwin.tar.gz",
]
asset_names = archive_names + ["SHA256SUMS"]

release = JSON.parse(gh_output("release", "view", tag, "--repo", source_repository, "--json", "isDraft,assets"))
fail_unless(!release.fetch("isDraft"), "source release #{tag} is still a draft")
fail_unless(release.fetch("assets").map { |asset| asset.fetch("name") }.sort == asset_names.sort, "source release assets are not exact")

Dir.mktmpdir("skills-manager-tap") do |download_dir|
  asset_names.each do |asset|
    gh_output("release", "download", tag, "--repo", source_repository, "--pattern", asset, "--dir", download_dir)
  end

  sum_lines = File.readlines(File.join(download_dir, "SHA256SUMS"), chomp: true).reject(&:empty?)
  fail_unless(sum_lines.length == 2, "SHA256SUMS must contain exactly two entries")
  sums = sum_lines.to_h do |line|
    checksum, name = line.split
    [name, checksum]
  end
  fail_unless(sums.keys.sort == archive_names.sort, "SHA256SUMS does not list exactly both archives")
  fail_unless(sums.values.all? { |checksum| checksum.match?(/\A[0-9a-fA-F]{64}\z/) }, "SHA256SUMS contains an invalid digest")

  archive_names.each do |asset|
    path = File.join(download_dir, asset)
    checksum = Digest::SHA256.file(path).hexdigest
    fail_unless(sums.fetch(asset) == checksum, "checksum mismatch for #{asset}")
    entries = `tar -tzf #{Shellwords.escape(path)}`.lines.map { |line| line.chomp.delete_suffix("/") }.sort
    fail_unless(entries == %w[LICENSE README.md skills-manager], "invalid archive layout for #{asset}")
    system(
      "gh", "attestation", "verify", path, "-R", source_repository,
      "--signer-workflow", "#{source_repository}/.github/workflows/release.yml",
      "--source-ref", "refs/tags/#{tag}"
    ) || abort("attestation verification failed for #{asset}")
  end

  source_commit = gh_output("api", "repos/#{source_repository}/commits/#{tag}", "--jq", ".sha").strip
  release_branch_commit = gh_output("api", "repos/#{source_repository}/git/ref/heads/release/#{tag}", "--jq", ".object.sha").strip
  fail_unless(source_commit == release_branch_commit, "release tag is not the tip of release/#{tag}")

  cargo_base64 = gh_output("api", "repos/#{source_repository}/contents/Cargo.toml?ref=#{tag}", "--jq", ".content")
  cargo_version = Base64.decode64(cargo_base64).match(/^version\s*=\s*"([^"]+)"/)[1]
  fail_unless(cargo_version == version, "Cargo.toml version does not match #{tag}")

  arm_sha = sums.fetch(archive_names.fetch(0))
  intel_sha = sums.fetch(archive_names.fetch(1))
  formula = PublisherContract.render_formula(version, arm_sha, intel_sha)
  formula_path = File.join(Dir.pwd, "Formula", "skills-manager.rb")
  branch = "automation/skills-manager-#{tag}"
  expected_marker = "<!-- skills-manager-publisher tag=#{tag} source_commit=#{source_commit} -->"
  expected_state = {"tag" => tag, "source_commit" => source_commit, "diff" => ["Formula/skills-manager.rb"]}

  all_prs = JSON.parse(gh_output("pr", "list", "--repo", ENV.fetch("GITHUB_REPOSITORY"), "--base", "main", "--state", "all", "--limit", "1000", "--json", "number,state,body,url,headRefName,baseRefName"))
  tagged_prs = PublisherContract.release_prs_for_tag(all_prs, tag)
  fail_unless(tagged_prs.length <= 1, "multiple tap PRs exist for #{tag}")
  existing_pr = tagged_prs.first
  branch_exists = system("git", "ls-remote", "--exit-code", "--heads", "origin", branch, out: File::NULL, err: File::NULL)

  if existing_pr
    fail_unless(existing_pr.fetch("headRefName") == branch, "a tap PR for #{tag} uses an unexpected branch")
  end
  if existing_pr && existing_pr.fetch("state") == "MERGED"
    abort "a tap PR for #{tag} is already merged"
  end
  if existing_pr && existing_pr.fetch("state") == "CLOSED"
    abort "a tap PR for #{tag} is closed; refusing to create a duplicate"
  end

  system("git", "config", "user.name", "github-actions[bot]") || abort("could not configure git user")
  system("git", "config", "user.email", "41898282+github-actions[bot]@users.noreply.github.com") || abort("could not configure git email")

  if branch_exists
    fail_unless(existing_pr, "existing #{branch} has no matching open PR")
    fail_unless(existing_pr.fetch("body", "").include?(expected_marker), "existing tap PR metadata does not match")
    git_output("fetch", "origin", "main", branch, "--prune")
    system("git", "checkout", "-B", branch, "origin/#{branch}") || abort("could not check out existing tap branch")
    git_output("merge", "--no-edit", "origin/main")
    paths = git_output("diff", "--name-only", "origin/main...HEAD").lines.map(&:strip).reject(&:empty?)
    fail_unless(PublisherContract.allowed_diff?(paths), "existing tap PR changes more than the formula")
    fail_unless(PublisherContract.retry_matches?(expected_state, {"tag" => tag, "source_commit" => source_commit, "diff" => paths}), "existing tap PR is not an exact retry")
    fail_unless(File.binread(formula_path) == formula, "existing formula does not match release hashes")
    system("git", "push", "origin", branch) || abort("could not update tap branch")
  else
    fail_unless(existing_pr.nil?, "a tap PR exists without its expected branch")
    git_output("fetch", "origin", "main", "--prune")
    system("git", "checkout", "-B", branch, "origin/main") || abort("could not create tap branch")
    FileUtils.mkdir_p(File.dirname(formula_path))
    File.write(formula_path, formula)
    system("ruby", "-c", formula_path) || abort("rendered formula is not valid Ruby")
    working_paths = git_output("status", "--porcelain").lines.map { |line| line[3..].to_s.strip }.reject(&:empty?)
    fail_unless(PublisherContract.allowed_diff?(working_paths), "working tree changes more than the formula")
    system("git", "add", "Formula/skills-manager.rb") || abort("could not stage formula")
    cached_paths = git_output("diff", "--cached", "--name-only").lines.map(&:strip).reject(&:empty?)
    fail_unless(PublisherContract.allowed_diff?(cached_paths), "staged changes are not exactly the formula")
    system("git", "commit", "-m", "skills-manager #{version}") || abort("could not commit formula")
    system("git", "push", "--set-upstream", "origin", branch) || abort("could not push tap branch")
    pr_body = <<~BODY
      #{expected_marker}

      Updates the native macOS archives for `skills-manager #{version}`.

      Source commit: `#{source_commit}`
    BODY
    gh_output("pr", "create", "--repo", ENV.fetch("GITHUB_REPOSITORY"), "--base", "main", "--head", branch, "--title", "skills-manager #{version}", "--body", pr_body)
    created_prs = JSON.parse(gh_output("pr", "list", "--repo", ENV.fetch("GITHUB_REPOSITORY"), "--head", branch, "--base", "main", "--state", "open", "--json", "number,body"))
    existing_pr = created_prs.find { |pr| pr.fetch("body", "").include?(expected_marker) }
    fail_unless(existing_pr, "created tap PR could not be found")
  end

  pr_number = existing_pr.fetch("number").to_s
  system("gh", "pr", "checks", pr_number, "--repo", ENV.fetch("GITHUB_REPOSITORY"), "--required", "--watch") || abort("required tap checks failed")
  system("gh", "pr", "merge", pr_number, "--repo", ENV.fetch("GITHUB_REPOSITORY"), "--auto", "--squash", "--delete-branch") || abort("could not enable tap auto-merge")
end
