#!/bin/bash
# usage: season.sh OUTDIR N   plays the original day after day (F5+Enter on main windows, Enter on auctions/halftime,
# ArrowDown+Enter on penalty dialogs); keeps a frame whenever the kind of screen changes (and every 15th step).
# Needs the original booted and loaded (tools/capture/boot.sh) and analysis/captures/venv (pillow, numpy).
cd "$(dirname "$0")/../.."
D=$1; mkdir -p $D; : > $D/log.txt; prev=""
key() { bash tools/capture/c.sh "{\"op\":\"key\",\"key\":\"$1\"}" >/dev/null; }
for i in $(seq -w 1 $2); do
  bash tools/capture/c.sh "{\"op\":\"guest\",\"path\":\"$PWD/$D/cur.png\"}" >/dev/null
  k=$(analysis/captures/venv/bin/python tools/capture/classify.py $D/cur.png)
  echo "$i $k $(md5sum $D/cur.png | cut -c1-8)" >> $D/log.txt
  if [ "$k" != "$prev" ] || [ $((10#$i % 15)) = 0 ]; then cp $D/cur.png $D/${i}_$k.png; fi
  case $k in
    roundview) ;;
    auction|halftime) key Enter;;
    penalty|dialog_over_roundview) key ArrowDown; key Enter;;
    main|main_inactive) key F5; key Enter;;
    *) key Enter;;
  esac
  prev=$k; sleep 0.8
done
