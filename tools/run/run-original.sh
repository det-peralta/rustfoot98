#!/bin/bash
# Run the original Elifoot 98 locally in DOSBox + Windows 3.1.
# Game files live in run/d (host folder) so saves appear on disk at run/d/eli98/jogos/*.e98
set -e
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SRC="$ROOT/original/bundle/x"
RUN="$ROOT/original/run"
if [ ! -f "$SRC/hdd.img" ]; then
  mkdir -p "$ROOT/original/bundle" && curl -L -o "$ROOT/original/bundle/elifoot98.jsdos" https://www.elifoot98.com.br/assets/elifoot/elifoot98.jsdos
  unzip -o -q "$ROOT/original/bundle/elifoot98.jsdos" -d "$SRC"
fi
if [ ! -d "$RUN" ]; then mkdir -p "$RUN" && cp -r "$SRC/hdd.img" "$SRC/d" "$RUN/"; fi   # pristine copy stays in bundle/
cd "$RUN" && exec dosbox -conf "$ROOT/tools/run/dosbox.conf" -noconsole
