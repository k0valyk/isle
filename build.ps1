#requires -Version 7.0
# Build script for Isle
# Run from repo root: powershell -ExecutionPolicy Bypass -File build.ps1

param(
    [switch]$Release,
    [switch]$Sign,
    [switch]$NoInstaller,
    [string]$CertPath = $env:ISLE_CERT_PATH,
    [string]$CertPassword = $env:ISLE_CERT_PASSWORD
)

$ErrorActionPreference = "Stop"
$root = Resolve-Path $PSScriptRoot

Write-Host "[1/5] Installing frontend dependencies..." -ForegroundColor Cyan
Push-Location $root
npm ci

Write-Host "[2/5] Checking Rust toolchain..." -ForegroundColor Cyan
if (!(Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Warning "Rust (cargo) not found. Install from https://rustup.rs/ then re-run."
    Pop-Location
    exit 0
}

Write-Host "[3/5] Building Tauri app..." -ForegroundColor Cyan
if ($Release) {
    if ($NoInstaller) {
        npx.cmd tauri build --no-bundle
    } else {
        npx.cmd tauri build
    }
    $installer = Get-ChildItem "$root\src-tauri\target\release\bundle\nsis\*setup.exe" -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($installer) {
        Write-Host "[4/5] Installer: $($installer.FullName)" -ForegroundColor Green
    } elseif (-not $NoInstaller) {
        Write-Warning "Installer was not produced. Check the Tauri bundle output above."
    }
} else {
    Push-Location "$root/src-tauri"
    cargo build
    Pop-Location
}

if ($Sign) {
    Write-Host "[5/5] Signing binaries..." -ForegroundColor Cyan
    $signtool = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if (-not $signtool) { throw "signtool.exe not found. Install Windows SDK or run without -Sign." }
    if (-not $CertPath -or -not (Test-Path $CertPath)) { throw "Certificate not found. Set ISLE_CERT_PATH or pass -CertPath." }
    $targets = @("$root\src-tauri\target\release\isle.exe")
    if ($installer) { $targets += $installer.FullName }
    foreach ($target in $targets) {
        & $signtool.Source sign /fd SHA256 /f $CertPath /p $CertPassword /tr http://timestamp.digicert.com /td SHA256 $target
        if ($LASTEXITCODE -ne 0) { throw "Signing failed for $target" }
    }
}

Write-Host "Creating portable archive..." -ForegroundColor Cyan
Compress-Archive -Path "$root\src-tauri\target\release\isle.exe", "$root\README.md" -DestinationPath "$root\Isle-portable.zip" -Force

Write-Host "Build complete." -ForegroundColor Green
Write-Host "Binary: src-tauri/target/release/isle.exe" -ForegroundColor Green
Pop-Location
