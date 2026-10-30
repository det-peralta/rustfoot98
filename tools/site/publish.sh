#!/bin/bash
# Publishes the site to GitHub Pages (the gh-pages branch, which Pages serves): builds it with the verification gallery
# (GALLERY=1: the pictures come from your own captures in analysis/screens) and without the game's files (no SKIN), then
# force-pushes site/ as the only commit of gh-pages. Run from the repository root after tools/screens/compare.py has run.
set -e
cd "$(dirname "$0")/../.."
GALLERY=1 bash tools/site/build.sh
test ! -d site/play/skin || { echo "site/play/skin present: the game's files must not be published" >&2; exit 1; }
tmp=$(mktemp -d)
cp -r site/. "$tmp"
touch "$tmp/.nojekyll"
cd "$tmp"
git init -q -b gh-pages
git add -A
git commit -q -m "Site: docs, verification gallery and the game at /play/ (tools/site/publish.sh)"
git push -q -f "$(git -C "$OLDPWD" remote get-url origin)" gh-pages
echo "published: https://det-peralta.github.io/rustfoot98/"
