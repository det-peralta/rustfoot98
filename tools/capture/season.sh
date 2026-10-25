#!/bin/bash
# usage: season.sh OUTDIR N   plays the original day after day (A (Automático: F5 is the formation 4-4-2, greyed out when the squad cannot play it) + Enter on main windows, Enter on auctions/halftime,
# ArrowDown+Enter on penalty dialogs); keeps every frame (about 25 KB each).
# Needs the original booted and loaded (tools/capture/boot.sh) and analysis/captures/venv (pillow, numpy).
cd "$(dirname "$0")/../.."
D=$1; mkdir -p $D; : > $D/log.txt; prev=""; same=0; ph=""
key() { bash tools/capture/c.sh "{\"op\":\"key\",\"key\":\"$1\"}" >/dev/null; }
for i in $(seq -w 1 $2); do
  bash tools/capture/c.sh "{\"op\":\"guest\",\"path\":\"$PWD/$D/cur.png\"}" >/dev/null
  k=$(analysis/captures/venv/bin/python tools/capture/classify.py $D/cur.png)
  echo "$i $k $(md5sum $D/cur.png | cut -c1-8)" >> $D/log.txt
  cp $D/cur.png $D/${i}_$k.png   # every frame is kept (the season-end dialogs look like round views to the classifier)
  case $k in
    roundview) ;;
    auction) key 1; key Enter; sleep 2;;
    halftime) key Enter;;
    penalty|dialog_over_roundview) key ArrowDown; key Enter;;
    main|main_inactive) key a; key Enter;;
    *) key Enter;;
  esac
  h=$(md5sum $D/cur.png | cut -c1-8); if [ "$h" = "$ph" ]; then same=$((same+1)); else same=0; fi; ph=$h
  if [ $same -ge 40 ]; then echo "stuck at $i ($k)" | tee -a $D/log.txt; exit 1; fi
  prev=$k; sleep 0.8
done
