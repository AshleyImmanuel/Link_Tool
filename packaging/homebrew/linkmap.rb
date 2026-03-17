class Linkmap < Formula
  desc "Git for understanding code. Fast, local, static code structure analysis."
  homepage "https://github.com/AshleyImmanuel/Link_Tool"
  license "MIT"
  version "0.1.1"

  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/AshleyImmanuel/Link_Tool/releases/download/v0.1.1/linkmap-0.1.1-macos-x86_64.tar.gz"
      sha256 "c05627cc34df4b48f6eb8984eae5f9da39cad7fb0bac1f799c91bda0d9267db7"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/AshleyImmanuel/Link_Tool/releases/download/v0.1.1/linkmap-0.1.1-linux-x86_64.tar.gz"
      sha256 "085f29a46431ef2363cf6be66586d17d9d7a2ba3b0e33311a34cdc8026577b85"
    end
  end

  def install
    bin.install "linkmap"
  end

  def caveats
    <<~EOS
      Linkmap is an experimental hobby project and is still under review. Use at your own risk.

      If you find issues, contact Ashley via LinkedIn:
        https://www.linkedin.com/in/ashley-immanuel-81609731b/
    EOS
  end

  test do
    system "#{bin}/linkmap", "--version"
  end
end

