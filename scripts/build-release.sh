#!/usr/bin/env bash
# Builds what a release needs and collects it in dist/release/v<version>/.
#
#   scripts/build-release.sh            # both platforms
#   scripts/build-release.sh --mac      # this Mac only
#   scripts/build-release.sh --windows  # the Windows box only
#
# macOS is built here; Windows is built over ssh on the box named by
# BANGS_WIN_HOST (default: win-gxl), whose caches all live on D: — see
# scripts/windows-build.ps1.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

host="${BANGS_WIN_HOST:-win-gxl}"
remote_repo='D:\Develop\bangs-src\repo'
version="$(node -p "require('./package.json').version")"
out="$root/dist/release/v$version"

want_mac=1
want_windows=1
case "${1:-}" in
  --mac) want_windows=0 ;;
  --windows) want_mac=0 ;;
  "") ;;
  *) echo "用法: $0 [--mac|--windows]" >&2; exit 1 ;;
esac

mkdir -p "$out"
echo "== Bangs v$version =="

if [[ $want_mac -eq 1 ]]; then
  echo "-- macOS --"
  pnpm tauri build --bundles app,dmg
  bundle="$root/src-tauri/target/release/bundle"
  arch="$(uname -m)"
  # A zipped .app is what people who dislike mounting a disk image want.
  ditto -c -k --keepParent "$bundle/macos/Bangs.app" "$out/Bangs_${version}_${arch}.app.zip"
  cp "$bundle"/dmg/Bangs_*.dmg "$out/Bangs_${version}_${arch}.dmg"
fi

if [[ $want_windows -eq 1 ]]; then
  echo "-- Windows ($host) --"
  bundle="$root/dist/release/.bundle.$$"
  git bundle create "$bundle" HEAD >/dev/null
  scp -q "$bundle" "$host:D:/Develop/bangs-src/release.bundle"
  rm -f "$bundle"
  scp -q "$root/scripts/windows-build.ps1" "$host:D:/Develop/bangs-src/windows-build.ps1"

  ssh "$host" "powershell -NoProfile -ExecutionPolicy Bypass -File D:\\Develop\\bangs-src\\windows-build.ps1" |
    iconv -f utf-8 -t utf-8 -c | tail -20

  for name in "Bangs_${version}_x64-setup.exe" "Bangs_${version}_x64_en-US.msi"; do
    if scp -q "$host:D:/Develop/bangs-src/artifacts/$name" "$out/$name" 2>/dev/null; then
      echo "取回 $name"
    else
      echo "缺少 $name（Windows 端没打出来）" >&2
    fi
  done
fi

echo
echo "产物在 $out"
ls -lh "$out" | tail -n +2 | awk '{ printf "  %-44s %s\n", $9, $5 }'
echo
echo "下一步：scripts/publish-release.md 里是发版清单"
