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
# Signing identity: the first Developer ID in the keychain unless one is given.
# The hash is used rather than the name, which repeats when a certificate has
# been imported twice and then matches ambiguously.
identity="${APPLE_SIGNING_IDENTITY:-$(security find-identity -v -p codesigning 2>/dev/null |
  awk '/Developer ID Application/ { print $2; exit }')}"
# The keychain profile holding the notarization credentials; see
# scripts/publish-release.md for the one command that creates it.
notary="${BANGS_NOTARY_PROFILE:-}"
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

echo "== Bangs v$version =="

if [[ $want_mac -eq 1 ]]; then
  echo "-- macOS --"
  if [[ -n "$identity" ]]; then
    echo "签名身份 $identity"
  else
    echo "没有 Developer ID 证书，打出来的包是未签名的" >&2
  fi
  # `pnpm build` empties dist/, which is where $out lives, so it is made after.
  APPLE_SIGNING_IDENTITY="$identity" pnpm tauri build --bundles app,dmg
  bundle="$root/src-tauri/target/release/bundle"
  # Tauri's own name for the architecture; older bundles of other versions stay
  # in that directory, so the file is named, never globbed.
  mkdir -p "$out"
  arch="$([[ "$(uname -m)" == "arm64" ]] && echo aarch64 || echo x64)"
  dmg="$bundle/dmg/Bangs_${version}_${arch}.dmg"
  [[ -f "$dmg" ]] || { echo "没找到 $dmg" >&2; exit 1; }
  app="$bundle/macos/Bangs.app"
  if [[ -n "$notary" ]]; then
    # Notarize the app first, staple it, and only then wrap it up: a stapled
    # ticket has to be inside whatever people download.
    echo "-- 公证 --"
    ditto -c -k --keepParent "$app" "$bundle/macos/Bangs-notarize.zip"
    xcrun notarytool submit "$bundle/macos/Bangs-notarize.zip" \
      --keychain-profile "$notary" --wait
    xcrun stapler staple "$app"
    rm -f "$bundle/macos/Bangs-notarize.zip"
    xcrun notarytool submit "$dmg" --keychain-profile "$notary" --wait
    xcrun stapler staple "$dmg"
  else
    echo "没有设置 BANGS_NOTARY_PROFILE，跳过公证（见 scripts/publish-release.md）" >&2
  fi
  # A zipped .app is what people who dislike mounting a disk image want.
  ditto -c -k --keepParent "$app" "$out/Bangs_${version}_${arch}.app.zip"
  cp "$dmg" "$out/Bangs_${version}_${arch}.dmg"

  if [[ -n "$notary" ]]; then
    echo "-- Gatekeeper --"
    spctl --assess --type execute --verbose=2 "$app" 2>&1 | tail -2
  fi
fi

if [[ $want_windows -eq 1 ]]; then
  echo "-- Windows ($host) --"
  mkdir -p "$out"
  bundle="$root/dist/release/.bundle.$$"
  git bundle create "$bundle" HEAD >/dev/null
  scp -q "$bundle" "$host:D:/Develop/bangs-src/release.bundle"
  rm -f "$bundle"
  scp -q "$root/scripts/windows-build.ps1" "$host:D:/Develop/bangs-src/windows-build.ps1"

  set +e
  ssh "$host" "powershell -NoProfile -ExecutionPolicy Bypass -File D:\\Develop\\bangs-src\\windows-build.ps1" |
    iconv -f utf-8 -t utf-8 -c | tail -20
  built=${PIPESTATUS[0]}
  set -e
  [[ $built -eq 0 ]] || { echo "Windows 构建失败（退出码 $built），日志在 $host 的 D:\\Develop\\bangs-src\\build.log" >&2; exit 1; }

  for name in "Bangs_${version}_x64-setup.exe" "Bangs_${version}_x64_en-US.msi"; do
    scp -q "$host:D:/Develop/bangs-src/artifacts/$name" "$out/$name" ||
      { echo "缺少 $name（Windows 端没打出来）" >&2; exit 1; }
    echo "取回 $name"
  done
fi

echo
echo "产物在 $out"
ls -lh "$out" | tail -n +2 | awk '{ printf "  %-44s %s\n", $9, $5 }'
echo
echo "下一步：scripts/publish-release.md 里是发版清单"
