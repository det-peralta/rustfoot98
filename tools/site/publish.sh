#!/bin/bash
# Publishes the site to GitHub Pages (the gh-pages branch, which Pages serves): builds it with the verification gallery
# (GALLERY=1: the pictures come from your own captures in analysis/screens) and with the game's files (SKIN=1: web/skin/, so
# the game plays at once without asking for a copy), then force-pushes site/ as the only commit of gh-pages. Run from the repository root after tools/screens/compare.py has run.
set -e
cd "$(dirname "$0")/../.."
test -f web/skin/index.json || { echo "web/skin/ missing: build it first (tools/ui/skin.py, tools/ui/webpack.py)" >&2; exit 1; }
GALLERY=1 SKIN=1 bash tools/site/build.sh
name=$(git config user.name); email=$(git config user.email || echo "$name@users.noreply.github.com")
origin=$(git remote get-url origin)
tmp=$(mktemp -d)
cp -r site/. "$tmp"
touch "$tmp/.nojekyll"
cd "$tmp"
git init -q -b gh-pages
git add -A
git -c user.name="$name" -c user.email="$email" commit -q -m "Site: docs, verification gallery and the game at /play/ (tools/site/publish.sh)"
git push -q -f "$origin" gh-pages
echo "published: https://det-peralta.github.io/rustfoot98/"
