# Build Windows MSI installer for Rustforged
# Run this script on Windows with PowerShell

param(
    [Parameter()]
    [ValidateSet("x64", "arm64", "both")]
    [string]$Architecture = "x64"
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir
$CargoToml = Get-Content "$ProjectRoot\Cargo.toml" -Raw
$Version = [regex]::Match($CargoToml, 'version\s*=\s*"([^"]+)"').Groups[1].Value

Set-Location $ProjectRoot

Write-Host "Building Rustforged v$Version for Windows" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# Check prerequisites
Write-Host "`nChecking prerequisites..."

$CargoPackager = Get-Command cargo-packager -ErrorAction SilentlyContinue
if (-not $CargoPackager) {
    Write-Host "Error: cargo-packager not found" -ForegroundColor Red
    Write-Host "Install with: cargo install cargo-packager --locked"
    exit 1
}

if (-not (Test-Path "packaging\icons\icon.ico")) {
    Write-Host "Error: Windows icon not found at packaging\icons\icon.ico" -ForegroundColor Red
    Write-Host "See packaging\icons\README.md for icon generation instructions"
    exit 1
}

function Build-Target {
    param([string]$Target, [string]$ArchName)

    Write-Host "`nBuilding for $ArchName ($Target)..." -ForegroundColor Yellow

    # Check if target is installed
    $InstalledTargets = rustup target list --installed
    if ($InstalledTargets -notcontains $Target) {
        Write-Host "Installing Rust target: $Target"
        rustup target add $Target
    }

    # Build release binary
    Write-Host "Building release binary..."
    cargo build --release --target $Target
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    # Create MSI installer
    Write-Host "Creating MSI installer..."
    cargo packager --release --target $Target --binaries-dir "target\$Target\release" --formats msi
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    # Find output
    $MsiDir = "target\release\packager"
    if ((Test-Path $MsiDir) -and (Get-ChildItem "$MsiDir\*.msi" -ErrorAction SilentlyContinue)) {
        $MsiFiles = Get-ChildItem "$MsiDir\*.msi"
        Write-Host "`nOutput:" -ForegroundColor Green
        foreach ($msi in $MsiFiles) {
            Write-Host "  $($msi.Name) ($([math]::Round($msi.Length / 1MB, 2)) MB)"
        }

        # Copy to releases directory
        $ReleasesDir = "$ProjectRoot\releases"
        if (-not (Test-Path $ReleasesDir)) {
            New-Item -ItemType Directory -Path $ReleasesDir | Out-Null
        }
        Copy-Item "$MsiDir\*.msi" $ReleasesDir -Force
        Write-Host "Copied to releases\"
    } else {
        Write-Host "Error: MSI output not found in $MsiDir" -ForegroundColor Red
        exit 1
    }
}

# Build requested architectures
$Targets = @{
    "x64" = @{ Target = "x86_64-pc-windows-msvc"; ArchName = "x64" }
    "arm64" = @{ Target = "aarch64-pc-windows-msvc"; ArchName = "ARM64" }
}

if ($Architecture -eq "both") {
    foreach ($arch in @("x64", "arm64")) {
        Build-Target -Target $Targets[$arch].Target -ArchName $Targets[$arch].ArchName
    }
} else {
    Build-Target -Target $Targets[$Architecture].Target -ArchName $Targets[$Architecture].ArchName
}

Write-Host "`nBuild complete!" -ForegroundColor Green
