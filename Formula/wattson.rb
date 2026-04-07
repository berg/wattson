# This file is a template — placeholders are substituted by the release workflow.
# The rendered version lives in https://github.com/berg/homebrew-wattson
class Wattson < Formula
  desc "RF Power Meter TUI for RPM-series RF power meters"
  homepage "https://github.com/berg/wattson"
  version "__VERSION__"
  license "MIT"

  on_macos do
    on_intel do
      url "https://github.com/berg/wattson/releases/download/v__VERSION__/wattson-v__VERSION__-x86_64-apple-darwin.tar.gz"
      sha256 "__SHA256_X86_MACOS__"
    end
    on_arm do
      url "https://github.com/berg/wattson/releases/download/v__VERSION__/wattson-v__VERSION__-aarch64-apple-darwin.tar.gz"
      sha256 "__SHA256_ARM_MACOS__"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/berg/wattson/releases/download/v__VERSION__/wattson-v__VERSION__-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "__SHA256_X86_LINUX__"
    end
    on_arm do
      url "https://github.com/berg/wattson/releases/download/v__VERSION__/wattson-v__VERSION__-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "__SHA256_ARM_LINUX__"
    end
  end

  def install
    bin.install "wattson"
  end

  test do
    assert_match "wattson", shell_output("#{bin}/wattson --help 2>&1")
  end
end
