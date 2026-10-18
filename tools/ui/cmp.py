#!/usr/bin/env python3
"""Pixel comparison of a render against a capture of the original (800x600).
  cmp.py golden.png ours.bmp [x y w h]     prints the number of differing pixels and, with a region, both as ASCII."""
import sys
import numpy as np
from PIL import Image

g = np.array(Image.open(sys.argv[1]).convert('RGB')).astype(int)
o = np.array(Image.open(sys.argv[2]).convert('RGB')).astype(int)
if len(sys.argv) >= 7:
    x, y, w, h = map(int, sys.argv[3:7])
    g, o = g[y:y + h, x:x + w], o[y:y + h, x:x + w]
diff = (g != o).any(axis=2)
print('differing pixels:', int(diff.sum()), 'of', diff.size)
if len(sys.argv) >= 7 and diff.sum():
    leg = {}
    def sym(c):
        c = tuple(int(v) for v in c)
        return leg.setdefault(c, chr(ord('a') + len(leg) % 26) if len(leg) < 26 else '?')
    for r in range(min(h, int(__import__("os").environ.get("ROWS", "40")))):
        print(''.join(sym(g[r, c]) for c in range(min(w, 90))), '  ', ''.join(sym(o[r, c]) for c in range(min(w, 90))))
    print({v: k for k, v in leg.items()})
