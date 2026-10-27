#!/bin/bash
# usage: days.sh OUTDIR N   plays the original like season.sh and, on every main window, pulls the game's saves out of the emulator
# (ci.persist) and keeps each new version of the save as OUTDIR/NNN_<name> (the original saves the game at the start of every day):
# consecutive days of the original's own state, for checking the rules (wages, gates, prizes) against the port.
cd "$(dirname "$0")/../.."
D=$1; mkdir -p $D/tmp; : > $D/log.txt; ph=""; same=0; n=0
key() { bash tools/capture/c.sh "{\"op\":\"key\",\"key\":\"$1\"}" >/dev/null; }
pull() {
  bash tools/capture/c.sh '{"op":"eval","js":"window.dosInstance.ciPromise.then(ci => ci.persist()).then(b => { let s = \"\"; for (let i = 0; i < b.length; i += 8192) s += String.fromCharCode.apply(null, b.subarray(i, i + 8192)); return btoa(s); })"}' > $D/tmp/persist.txt
  python3 - $D <<'P'
import base64, json, zipfile, io, sys, os, hashlib, glob
d = sys.argv[1]
t = open(d + '/tmp/persist.txt').read()
z = zipfile.ZipFile(io.BytesIO(base64.b64decode(json.loads(t[t.index(' ') + 1:].strip()))))
for i in z.infolist():
    if i.filename.lower().startswith('d/eli98/jogos/') and i.filename.lower().endswith('.e98'):
        data = z.read(i)
        name = os.path.basename(i.filename)
        h = hashlib.md5(data).hexdigest()
        if any(hashlib.md5(open(p, 'rb').read()).hexdigest() == h for p in glob.glob(d + '/*_' + name)):
            continue
        k = len(glob.glob(d + '/*_' + name)) + 1
        open('%s/%03d_%s' % (d, k, name), 'wb').write(data)
        print('new save %03d_%s' % (k, name))
P
}
for i in $(seq -w 1 $2); do
  bash tools/capture/c.sh "{\"op\":\"guest\",\"path\":\"$PWD/$D/cur.png\"}" >/dev/null
  k=$(analysis/captures/venv/bin/python tools/capture/classify.py $D/cur.png)
  echo "$i $k" >> $D/log.txt
  case $k in
    roundview) ;;
    auction) key 1; key Enter; sleep 2;;
    halftime) key Enter;;
    penalty|dialog_over_roundview) key ArrowDown; key Enter;;
    main|main_inactive) pull >> $D/log.txt; key a; key Enter;;
    *) key Enter;;
  esac
  h=$(md5sum $D/cur.png | cut -c1-8); if [ "$h" = "$ph" ]; then same=$((same+1)); else same=0; fi; ph=$h
  if [ $same -ge 40 ]; then echo "stuck at $i ($k)" | tee -a $D/log.txt; exit 1; fi
  sleep 0.8
done
