#!/usr/bin/env python3
"""where.py with compare.py's masks: the mouse pointer left out and, for `region="window"` screens, only the program's window.
  analysis/captures/venv/bin/python tools/screens/where_region.py <id> [min]"""
import os, sys
from collections import Counter
import numpy as np
from PIL import Image
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'capture'))
from manifest import SCREENS  # noqa: E402
from mouse import find_arrow  # noqa: E402
import compare  # noqa: E402

i = sys.argv[1]
least = int(sys.argv[2]) if len(sys.argv) > 2 else 5
s = next(x for x in SCREENS if x['id'] == i)
O = compare.load(compare.original_of(s))
P = compare.load('analysis/screens/port/%s.png' % i)
d = ((O >> 3) != (P >> 3)).any(axis=2)
arrow = compare.pointer_positions().get(i) or find_arrow(O)
if arrow:
    d[max(0, arrow[1] - 1):arrow[1] + 21, max(0, arrow[0] - 1):arrow[0] + 13] = False
if s.get('region') == 'window':
    ys, xs = np.nonzero((P != P[300, 2]).any(axis=2))
    keep = np.zeros_like(d)
    keep[ys.min():ys.max() + 1, xs.min():xs.max() + 1] = True
    keep[:20] = False
    d &= keep
ys, xs = np.nonzero(d)
print(int(d.sum()))
for (x, y), n in sorted(Counter(zip(xs // 40 * 40, ys // 20 * 20)).items(), key=lambda kv: (kv[0][1], kv[0][0])):
    if n >= least:
        print('x %3d-%3d  y %3d-%3d  %d' % (x, x + 39, y, y + 19, n))
