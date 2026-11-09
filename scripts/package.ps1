# Builds Claude House and puts everything needed to run it in one folder (and a zip).
#
#   powershell -ExecutionPolicy Bypass -File scripts\package.ps1
#
# Result: dist\ClaudeHouse\  (claude-house.exe, ch-link.exe, ch.exe, README.md)
#         dist\ClaudeHouse.zip
#
# The three programs must stay in the same folder: the app registers ch-link.exe from its own folder
# as the claude:// handler, and ch.exe looks for ch-link.exe next to itself.

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

cargo build --release -p claude-house-cli -p claude-house-app
if ($LASTEXITCODE -ne 0) {
    throw 'cargo build failed. If Claude House is running from target\release, quit it (tray icon > Quit) and run this again.'
}

$out = Join-Path $root 'dist\ClaudeHouse'
if (Test-Path $out) { Remove-Item -Recurse -Force $out }
New-Item -ItemType Directory -Force $out | Out-Null

foreach ($exe in 'claude-house.exe', 'ch-link.exe', 'ch.exe') {
    Copy-Item (Join-Path $root "target\release\$exe") $out
}
Copy-Item (Join-Path $root 'README.md'), (Join-Path $root 'LICENSE') $out

$zip = Join-Path $root 'dist\ClaudeHouse.zip'
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path $out -DestinationPath $zip

Write-Host "Ready: $out"
Write-Host "Zip:   $zip"
