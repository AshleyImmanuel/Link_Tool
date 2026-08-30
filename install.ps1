param(
  [string]$Repo = $env:LINKMAP_REPO,
  [string]$Version = $env:LINKMAP_VERSION,
  [string]$InstallDir = $env:LINKMAP_INSTALL_DIR
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($Repo)) { $Repo = "AshleyImmanuel/Link_Tool" }
if ([string]::IsNullOrWhiteSpace($Version)) { $Version = "latest" }
if ([string]::IsNullOrWhiteSpace($InstallDir)) { $InstallDir = Join-Path $env:USERPROFILE ".local\bin" }

Write-Warning "linkmap is an experimental hobby project and is still under review. Use at your own risk."
Write-Warning "If you find issues, contact Ashley via LinkedIn: https://www.linkedin.com/in/ashley-immanuel-81609731b/"

$asset = "linkmap-windows-x86_64.zip"
$base = "https://github.com/$Repo/releases"

if ($Version -eq "latest") {
  $url = "$base/latest/download/$asset"
  $sumsUrl = "$base/latest/download/SHA256SUMS"
} else {
  $url = "$base/download/v$Version/$asset"
  $sumsUrl = "$base/download/v$Version/SHA256SUMS"
}

$tmp = Join-Path $env:TEMP ("linkmap-install-" + [Guid]::NewGuid().ToString("n"))
New-Item -ItemType Directory -Force $tmp | Out-Null
try {
  $zipPath = Join-Path $tmp $asset
  $sumsPath = Join-Path $tmp "SHA256SUMS"

  Write-Host "Downloading $url"
  Invoke-WebRequest -UseBasicParsing $url -OutFile $zipPath
  Invoke-WebRequest -UseBasicParsing $sumsUrl -OutFile $sumsPath

  $sumsBytes = [IO.File]::ReadAllBytes($sumsPath)
  $sumsText = [Text.Encoding]::UTF8.GetString($sumsBytes)
  $line = ($sumsText -split "`n" | Where-Object { $_ -match " $([regex]::Escape($asset))$" } | Select-Object -First 1)
  if (-not $line) { throw "Could not find checksum for $asset in SHA256SUMS" }
  $expected = ($line -split "\s+")[0].Trim()

  $actual = (Get-FileHash -Algorithm SHA256 $zipPath).Hash.ToLowerInvariant()
  if ($actual -ne $expected.ToLowerInvariant()) {
    throw "Checksum mismatch for $asset`nExpected: $expected`nActual:   $actual"
  }

  New-Item -ItemType Directory -Force $InstallDir | Out-Null
  $extract = Join-Path $tmp "extract"
  Expand-Archive -Path $zipPath -DestinationPath $extract -Force

  $exe = Join-Path $extract "linkmap.exe"
  if (-not (Test-Path $exe)) { throw "linkmap.exe not found in archive" }

  $dest = Join-Path $InstallDir "linkmap.exe"
  Copy-Item $exe $dest -Force

  Write-Host "Installed linkmap to $dest"
  Write-Host "Try: linkmap --version"
  if (-not ($env:PATH -split ";" | Where-Object { $_ -eq $InstallDir })) {
    Write-Host ""
    Write-Host "Note: $InstallDir is not on PATH yet."
    Write-Host "Add it, or run with the full path."
  }
} finally {
  Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

