#!/usr/bin/env bash
# Publishes site/ to the gh-pages branch, which GitHub Pages serves at
# https://gxlself.github.io/bangs/.
#
# The branch holds the site and nothing else, so it is built in a throwaway
# repository and force-pushed; the working tree here is never touched.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

remote_name="${BANGS_REMOTE:-origin}"
remote_url="$(git remote get-url "$remote_name")"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

cp -R site/. "$work/"
# Without this, Pages runs Jekyll and drops anything starting with _.
touch "$work/.nojekyll"

git -C "$work" init -q -b gh-pages
git -C "$work" add -A
git -C "$work" \
  -c user.name="$(git config user.name)" \
  -c user.email="$(git config user.email)" \
  commit -q -m "Publish the site — $(node -p "require('$root/package.json').version") — $(date +%F)"
git -C "$work" push -q -f "$remote_url" gh-pages

echo "站点已推到 $remote_name/gh-pages"
echo "首次发布后去 GitHub → Settings → Pages 选 gh-pages / (root)，之后跑这个脚本就够了"
