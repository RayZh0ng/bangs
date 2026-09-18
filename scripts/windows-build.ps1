# Builds the Windows release on the build box. Every cache stays off C:, which
# is why CARGO_HOME, CARGO_TARGET_DIR, the pnpm store and even Tauri's own tool
# cache (it downloads NSIS and WiX into %LOCALAPPDATA%) are pointed at D:.
#
# Called by scripts/build-release.sh, which drops release.bundle next to it.
$ErrorActionPreference = 'Continue'

$src = 'D:\Develop\bangs-src'
$cache = 'D:\Develop\build-cache'
$env:CARGO_HOME = "$cache\cargo-home"
$env:CARGO_TARGET_DIR = "$cache\bangs-target"
$env:npm_config_store_dir = "$cache\pnpm-store"
$env:LOCALAPPDATA = "$cache\localappdata"
New-Item -ItemType Directory -Force -Path $env:CARGO_HOME, $env:CARGO_TARGET_DIR,
  $env:npm_config_store_dir, $env:LOCALAPPDATA, "$src\artifacts" | Out-Null

# Fresh checkout of whatever the Mac just sent.
if (Test-Path "$src\release.bundle") {
  if (Test-Path "$src\repo") { Remove-Item -Recurse -Force "$src\repo" }
  git clone -q "$src\release.bundle" "$src\repo"
}
Set-Location "$src\repo"

$log = "$src\build.log"
"windows build $(Get-Date -Format s)" | Set-Content $log
cmd /c "pnpm install --frozen-lockfile >> $log 2>&1"
cmd /c "pnpm tauri build --bundles nsis,msi >> $log 2>&1"
$code = $LASTEXITCODE
Add-Content $log "exit=$code"
Get-Content $log -Tail 12

$bundle = "$env:CARGO_TARGET_DIR\release\bundle"
Remove-Item "$src\artifacts\*" -Force -ErrorAction SilentlyContinue
Get-ChildItem -Path "$bundle\nsis\*.exe", "$bundle\msi\*.msi" -ErrorAction SilentlyContinue |
  ForEach-Object {
    Copy-Item $_.FullName "$src\artifacts\" -Force
    Write-Output ("built {0} ({1:N1} MB)" -f $_.Name, ($_.Length / 1MB))
  }
