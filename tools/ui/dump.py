#!/usr/bin/env python3
"""ASCII dump of a region of a capture: dump.py image x y w h   (one letter per distinct colour)"""
import sys
import numpy as np
from PIL import Image
g = np.array(Image.open(sys.argv[1]).convert('RGB')).astype(int)
x, y, w, h = map(int, sys.argv[2:6])
fixed = {(0, 0, 0): '#', (248, 252, 248): 'W', (192, 192, 192): '.', (128, 128, 128): 'g', (0, 128, 0): ' ', (0, 0, 128): 'n', (248, 252, 0): 'y'}
leg = {}
for r in range(y, y + h):
    row = ''
    for c in range(x, x + w):
        t = tuple(int(v) for v in g[r, c])
        row += fixed.get(t) or leg.setdefault(t, chr(97 + len(leg)) if len(leg) < 20 else '?')
    print('%3d %s' % (r, row))
print(leg)
