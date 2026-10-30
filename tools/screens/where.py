#!/usr/bin/env python3
"""Where a screen of the catalogue differs: boxes of 40x20 pixels with their counts (after tools/screens/compare.py).
  analysis/captures/venv/bin/python tools/screens/where.py <id> [min]"""
import sys
from collections import Counter
import numpy as np
from PIL import Image

i = sys.argv[1]
least = int(sys.argv[2]) if len(sys.argv) > 2 else 5
import os
src = 'analysis/screens/original/%s.png' % i
if not os.path.exists(src):
    sys.path.insert(0, 'tools/screens'); from manifest import SCREENS
    src = 'analysis/captures/' + next(s['capture'] for s in SCREENS if s['id'] == i)
o = np.array(Image.open(src).convert('RGB')).astype(int) >> 3
p = np.array(Image.open('analysis/screens/port/%s.png' % i).convert('RGB')).astype(int) >> 3
ys, xs = np.nonzero((o != p).any(axis=2))
for (x, y), n in sorted(Counter(zip(xs // 40 * 40, ys // 20 * 20)).items(), key=lambda kv: (kv[0][1], kv[0][0])):
    if n >= least:
        print('x %3d-%3d  y %3d-%3d  %d' % (x, x + 39, y, y + 19, n))
