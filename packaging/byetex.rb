# Homebrew formula for ByeTex (prebuilt-binary tap).
#
# This lives in a SEPARATE tap repo for `brew` to find it:
#   zeyuyang42/homebrew-byetex  →  Formula/byetex.rb
# Then users:
#   brew install zeyuyang42/byetex/byetex
#
# Per release: bump `version` and fill each `sha256` from the release's
# SHA256SUMS (release.yml generates it). See packaging/README.md.
class Byetex < Formula
  desc "LaTeX -> Typst converter for AI agents (CLI + MCP server)"
  homepage "https://github.com/zeyuyang42/ByeTex"
  version "0.3.0"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "https://github.com/zeyuyang42/ByeTex/releases/download/v#{version}/byetex-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256_aarch64_apple_darwin"
    end
    on_intel do
      url "https://github.com/zeyuyang42/ByeTex/releases/download/v#{version}/byetex-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256_x86_64_apple_darwin"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/zeyuyang42/ByeTex/releases/download/v#{version}/byetex-v#{version}-aarch64-unknown-linux-musl.tar.gz"
      sha256 "REPLACE_WITH_SHA256_aarch64_linux_musl"
    end
    on_intel do
      url "https://github.com/zeyuyang42/ByeTex/releases/download/v#{version}/byetex-v#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "REPLACE_WITH_SHA256_x86_64_linux_musl"
    end
  end

  def install
    # Each archive extracts to byetex-v<ver>-<target>/byetex.
    bin.install Dir["byetex-*/byetex"].first => "byetex"
  end

  test do
    assert_match "byetex", shell_output("#{bin}/byetex --version")
  end
end
