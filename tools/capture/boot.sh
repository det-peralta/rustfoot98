#!/bin/bash
# (re)starts the local js-dos instance of the original and brings it to the Elifoot "select game" dialog.
cd "$(dirname "$0")/../.."
bash tools/capture/c.sh '{"op":"quit"}' >/dev/null 2>&1; sleep 1
(cd analysis/jsdos && (python3 -m http.server 8123 --bind 127.0.0.1 >/dev/null 2>&1 &)) 2>/dev/null
(ELI_URL=http://127.0.0.1:8123/index.html nohup node tools/capture/driver.mjs > /tmp/driver.log 2>&1 &)
sleep 25
bash tools/capture/c.sh '{"op":"wait","ms":14000}' >/dev/null
bash tools/capture/c.sh '{"op":"click","x":580,"y":196}' >/dev/null     # js-dos "mouse lock" notice
bash tools/capture/c.sh '{"op":"wait","ms":40000}' >/dev/null           # Windows starts, the game shows its About box
bash tools/capture/c.sh '{"op":"key","key":"Enter"}' >/dev/null
bash tools/capture/c.sh '{"op":"wait","ms":6000}' >/dev/null
