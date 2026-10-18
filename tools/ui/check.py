#!/usr/bin/env python3
"""Renders every case of tools/ui/cases.json with win31shot and reports how many pixels differ from the capture of the original.
Captures live in analysis/shots (git-ignored); cases whose capture is missing are skipped. Run from the repository root:
  cargo build -p elifoot-ui --release --bin win31shot && python3 tools/ui/check.py"""
import json, os, subprocess, sys
import numpy as np
from PIL import Image

root = os.getcwd()
data = os.environ.get('ELIFOOT_ROOT', root)
cases = json.load(open('tools/ui/cases.json'))
total_bad = 0
for c in cases:
    if not os.path.exists(os.path.join(data, c['golden'])):
        print('%-16s skipped (no capture)' % c['form'])
        continue
    out = '/tmp/win31_%s.bmp' % c['form']
    subprocess.run([os.path.join(root, 'target/release/win31shot'), c['form'], out] + c['args'], check=True, capture_output=True)
    g = np.array(Image.open(os.path.join(data, c['golden'])).convert('RGB')).astype(int)
    o = np.array(Image.open(out).convert('RGB')).astype(int)
    x, y, w, h = c['region']
    d = (g[y:y + h, x:x + w] != o[y:y + h, x:x + w]).any(axis=2)
    total_bad += int(d.sum())
    print('%-16s %6d of %6d pixels differ (%.3f%%)  %s' % (c['form'], d.sum(), d.size, 100 * d.sum() / d.size, c.get('note', '')))
sys.exit(0)
