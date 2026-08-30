class Linkmap < Formula
  desc "Git for understanding code. Fast, local, static code structure analysis."
  homepage "https://github.com/AshleyImmanuel/Link_Tool"
  license "MIT"
  version "0.1.2"

  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/AshleyImmanuel/Link_Tool/releases/download/v0.1.2/linkmap-0.1.2-macos-x86_64.tar.gz"
      sha256 "dcf64b178a25c33c840675da5bdf42b211373b6d7b445cd0882a471344b99826"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/AshleyImmanuel/Link_Tool/releases/download/v0.1.2/linkmap-0.1.2-linux-x86_64.tar.gz"
      sha256 "254628a89c61ee789c492e99dbf1f888be9a52d6b1003e5e39d16f3f9a01bad7"
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

