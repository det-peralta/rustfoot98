#!/bin/bash
# usage: season_retry.sh OUTBASE STEPS [ATTEMPTS]   boots the original, loads the save "ABCDEEEE" and plays STEPS steps with season.sh into
# OUTBASE_<n>; when the original stops answering (season.sh exits 1: "stuck"), starts over from the save in the next directory.
# Stops at the first attempt that plays all its steps.
cd "$(dirname "$0")/../.."
for n in $(seq 1 ${3:-6}); do
  for p in $(pgrep -f "tools/capture/driver[.]mjs") $(pgrep -f "http[.]server 8123"); do kill $p 2>/dev/null; done
  sleep 2; : > analysis/cmd.jsonl; : > analysis/cmd.out
  bash tools/capture/boot.sh >/dev/null 2>&1
  for k in Tab Tab Space Tab A Enter; do bash tools/capture/c.sh "{\"op\":\"key\",\"key\":\"$k\"}" >/dev/null; sleep 0.7; done
  sleep 20
  echo "attempt $n -> ${1}_$n" ; bash tools/capture/season.sh ${1}_$n $2 && { echo "attempt $n played all $2 steps"; exit 0; }
done
echo "gave up after ${3:-6} attempts"; exit 1
