#!/bin/bash
# Builds the whole site into site/: the documentation (mkdocs-material) at the root and the game at play/.
#   tools/site/build.sh            docs + game, no third-party files (players are asked for their own copy of the game)
#   GALLERY=1 tools/site/build.sh  also the verification gallery: renders and compares every screen (needs the captures of the
#                                  original in analysis/screens/original, see tools/screens/) and puts the pictures in the docs
#   SKIN=1 tools/site/build.sh     also copies web/skin/ (the game's data and the Windows 3.1 fonts) so the game plays at once.
#                                  Only for a site you may publish them on: see NOTICE.md.
# Needs mkdocs-material (pip install mkdocs-material) and the wasm32 target (rustup target add wasm32-unknown-unknown).
set -e
cd "$(dirname "$0")/../.."
MKDOCS=${MKDOCS:-mkdocs}
command -v "$MKDOCS" >/dev/null || MKDOCS=analysis/captures/venv/bin/mkdocs
bash web/build.sh
if [ "${GALLERY:-0}" = 1 ]; then
  cargo build --release -p rustfoot-ui --bin classic_shot --bin editor_shot
  PY=${PY:-analysis/captures/venv/bin/python}
  "$PY" tools/screens/compare.py --publish
fi
"$MKDOCS" build --clean
mkdir -p site/play
cp web/*.html web/*.js web/*.css web/*.wasm web/*.png web/*.svg web/manifest.webmanifest site/play/
if [ "${SKIN:-0}" = 1 ]; then
  cp -r web/skin site/play/skin
  echo "site/play/skin: the game's data and fonts are included"
fi
echo "site/ ready: docs at site/index.html, the game at site/play/ (serve with: python3 -m http.server -d site 8000)"
