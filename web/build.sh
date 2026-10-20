#!/bin/bash
# Builds the WebAssembly core into web/ and (optionally) serves it: ./build.sh [serve]
set -e
cd "$(dirname "$0")/.."
cargo build --release --target wasm32-unknown-unknown -p rustfoot-wasm
cp target/wasm32-unknown-unknown/release/rustfoot_wasm.wasm web/
echo "built web/rustfoot_wasm.wasm ($(wc -c < web/rustfoot_wasm.wasm) bytes)"
cargo build --release --target wasm32-unknown-unknown -p rustfoot-ui-web
cp target/wasm32-unknown-unknown/release/rustfoot_ui_web.wasm web/rustfoot_ui.wasm
echo "built web/rustfoot_ui.wasm ($(wc -c < web/rustfoot_ui.wasm) bytes)"
if [ "$1" = "serve" ]; then cd web && echo "http://localhost:8080" && python3 -m http.server 8080; fi
