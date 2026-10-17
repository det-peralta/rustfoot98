#!/bin/bash
# Builds the WebAssembly core into web/ and (optionally) serves it: ./build.sh [serve]
set -e
cd "$(dirname "$0")/.."
cargo build --release --target wasm32-unknown-unknown -p elifoot-wasm
cp target/wasm32-unknown-unknown/release/elifoot_wasm.wasm web/
echo "built web/elifoot_wasm.wasm ($(wc -c < web/elifoot_wasm.wasm) bytes)"
if [ "$1" = "serve" ]; then cd web && echo "http://localhost:8080" && python3 -m http.server 8080; fi
