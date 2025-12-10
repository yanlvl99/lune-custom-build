#!/usr/bin/env pwsh
# Lune Custom Build - Auto Installer
# Usage: irm https://raw.githubusercontent.com/yanlvl99/lune-custom-build/main/installer/install.ps1 | iex

$ErrorActionPreference = 'Stop'

Write-Host ""
Write-Host "  ╔═══════════════════════════════════════════════════════════╗" -ForegroundColor Magenta
Write-Host "  ║               LUNE CUSTOM BUILD INSTALLER                 ║" -ForegroundColor Magenta
Write-Host "  ╚═══════════════════════════════════════════════════════════╝" -ForegroundColor Magenta
Write-Host ""

# Detect architecture
$arch = if ([Environment]::Is64BitOperatingSystem) {
    if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64" -or $env:PROCESSOR_IDENTIFIER -match "ARM") {
        "aarch64"
    } else {
        "x86_64"
    }
} else {
    Write-Host "[ERROR] 32-bit systems are not supported." -ForegroundColor Red
    exit 1
}

Write-Host "[INFO] Detected architecture: $arch" -ForegroundColor Cyan

# Get latest release
Write-Host "[INFO] Fetching latest release..." -ForegroundColor Cyan
try {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/yanlvl99/lune-custom-build/releases/latest"
    $version = $release.tag_name
    Write-Host "[INFO] Latest version: $version" -ForegroundColor Green
} catch {
    Write-Host "[ERROR] Failed to fetch release info: $_" -ForegroundColor Red
    exit 1
}

# Find the correct asset
$assetName = "lune-windows-$arch.zip"
$asset = $release.assets | Where-Object { $_.name -eq $assetName }

if (-not $asset) {
    Write-Host "[ERROR] Asset '$assetName' not found in release." -ForegroundColor Red
    Write-Host "[INFO] Available assets:" -ForegroundColor Yellow
    $release.assets | ForEach-Object { Write-Host "  - $($_.name)" }
    exit 1
}

$downloadUrl = $asset.browser_download_url
Write-Host "[INFO] Downloading $assetName..." -ForegroundColor Cyan

# Create install directory
$installDir = "$env:LOCALAPPDATA\Lune"
if (-not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}

# Download and extract
$zipPath = "$env:TEMP\lune-download.zip"
try {
    Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath -UseBasicParsing
    Write-Host "[INFO] Extracting..." -ForegroundColor Cyan
    Expand-Archive -Path $zipPath -DestinationPath $installDir -Force
    Remove-Item $zipPath -Force
} catch {
    Write-Host "[ERROR] Download failed: $_" -ForegroundColor Red
    exit 1
}

# Add to PATH if not already there
$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($userPath -notlike "*$installDir*") {
    Write-Host "[INFO] Adding to PATH..." -ForegroundColor Cyan
    [Environment]::SetEnvironmentVariable("PATH", "$userPath;$installDir", "User")
    $env:PATH = "$env:PATH;$installDir"
}

# Verify installation
$lunePath = Join-Path $installDir "lune.exe"
if (Test-Path $lunePath) {
    Write-Host ""
    Write-Host "  ╔═══════════════════════════════════════════════════════════╗" -ForegroundColor Green
    Write-Host "  ║              INSTALLATION COMPLETE!                       ║" -ForegroundColor Green
    Write-Host "  ╚═══════════════════════════════════════════════════════════╝" -ForegroundColor Green
    Write-Host ""
    Write-Host "  Version:  $version" -ForegroundColor White
    Write-Host "  Arch:     $arch" -ForegroundColor White
    Write-Host "  Location: $installDir" -ForegroundColor White
    Write-Host ""
    Write-Host "  Run 'lune --help' to get started!" -ForegroundColor Yellow
    Write-Host "  (You may need to restart your terminal for PATH changes)" -ForegroundColor DarkGray
    Write-Host ""
} else {
    Write-Host "[ERROR] Installation verification failed." -ForegroundColor Red
    exit 1
}
