# SPDX-License-Identifier: AGPL-3.0-only

param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("aarch64", "x86_64")]
    [string]$Architecture
)

$ErrorActionPreference = "Stop"

$target = switch ($Architecture) {
    "aarch64" { "aarch64-pc-windows-msvc" }
    "x86_64"  { "x86_64-pc-windows-msvc" }
}

rustup target add $target
cargo build --release --target $target

$binary = "target/$target/release/beacon.exe"
if (-not (Test-Path $binary)) {
    Write-Error "binary not found: $binary"
    exit 1
}

$distDir = "target/dist"
$bundleDir = "$distDir/beacon"
if (Test-Path $distDir) { Remove-Item -Recurse -Force $distDir }
New-Item -ItemType Directory -Force -Path "$bundleDir/resources" | Out-Null

Copy-Item $binary "$bundleDir/beacon.exe"
Copy-Item -Recurse "assets" "$bundleDir/resources/assets"

Compress-Archive -Path "$bundleDir/*" -DestinationPath "$distDir/BEACON-windows-$Architecture.zip"

Write-Host "created $distDir/BEACON-windows-$Architecture.zip"
