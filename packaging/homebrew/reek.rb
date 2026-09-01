class Reek < Formula
  desc "REEK Ultimate Uninstaller - the uninstaller that actually uninstalls"
  homepage "https://github.com/greek/greek-uninstaller"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/greek/greek-uninstaller/releases/download/v#{version}/reek-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256_AARCH64_DARWIN"
    else
      url "https://github.com/greek/greek-uninstaller/releases/download/v#{version}/reek-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256_X86_64_DARWIN"
    end
  end

  on_linux do
    url "https://github.com/greek/greek-uninstaller/releases/download/v#{version}/reek-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "REPLACE_WITH_SHA256_X86_64_LINUX"
  end

  def install
    bin.install "reek"
    bin.install "reek-tui" if File.exist?("reek-tui")
    generate_completions_from_executable(bin/"reek", "completions", shells: [:bash, :zsh, :fish])
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/reek --version")
  end
end
