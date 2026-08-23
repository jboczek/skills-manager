module PublisherContract
  def self.valid_version?(version)
    version.match?(/\A\d+\.\d+\.\d+\z/)
  end

  def self.valid_formula_inputs?(version, arm_sha256, intel_sha256)
    valid_version?(version) &&
      arm_sha256.match?(/\A[0-9a-fA-F]{64}\z/) &&
      intel_sha256.match?(/\A[0-9a-fA-F]{64}\z/)
  end

  def self.render_formula(version, arm_sha256, intel_sha256)
    raise ArgumentError, "invalid formula inputs" unless valid_formula_inputs?(version, arm_sha256, intel_sha256)

    base_url = "https://github.com/jboczek/skills-manager/releases/download/v#{version}"
    arm_archive = "skills-manager-v#{version}-aarch64-apple-darwin.tar.gz"
    intel_archive = "skills-manager-v#{version}-x86_64-apple-darwin.tar.gz"

    <<~RUBY
      class SkillsManager < Formula
        desc "Terminal-first skill exposure manager"
        homepage "https://github.com/jboczek/skills-manager"

        def self.configure_architecture
          on_arm do
            url "#{base_url}/#{arm_archive}"
            sha256 "#{arm_sha256}"
          end

          on_intel do
            url "#{base_url}/#{intel_archive}"
            sha256 "#{intel_sha256}"
          end
        end

        configure_architecture
        version "#{version}"
        stable.version Version.new("#{version}", detected_from_url: true)
        license "MIT"
        depends_on :macos

        def install
          bin.install "skills-manager"
        end

        test do
          assert_match version.to_s, shell_output("\#{bin}/skills-manager --version")
        end
      end
    RUBY
  end

  def self.allowed_diff?(paths)
    paths == ["Formula/skills-manager.rb"]
  end

  def self.retry_matches?(expected, existing)
    expected == existing
  end
end
