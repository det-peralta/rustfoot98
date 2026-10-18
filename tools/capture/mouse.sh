#!/bin/bash
# mouse.sh X Y [click|dblclick|down|up]: moves the guest's mouse to guest pixel (X, Y) (the pointer is relative and doubled, so
# it is first pushed into the top-left corner) and optionally clicks.
cd "$(dirname "$0")/../.."
X=$1; Y=$2; ACT=${3:-none}
bash tools/capture/c.sh "{\"op\":\"eval\",\"js\":\"(async()=>{const ci=await window.dosInstance.ciPromise; const w=ms=>new Promise(r=>setTimeout(r,ms)); ci.sendMouseRelativeMotion(-3000,-3000); await w(250); ci.sendMouseRelativeMotion($((X/2)),$((Y/2))); await w(250); const act='$ACT'; const click=async()=>{ci.sendMouseButton(0,true); await w(120); ci.sendMouseButton(0,false); await w(300);}; if(act==='click') await click(); if(act==='dblclick'){await click(); await click();} return 1})()\"}" >/dev/null
