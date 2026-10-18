#!/usr/bin/env python3
"""Lists the regions where a render differs from a capture of the original: regions.py golden.png ours.bmp [x y w h]"""
import sys
import numpy as np
from PIL import Image
from scipy import ndimage

g = np.array(Image.open(sys.argv[1]).convert('RGB')).astype(int)
o = np.array(Image.open(sys.argv[2]).convert('RGB')).astype(int)
x0 = y0 = 0
if len(sys.argv) >= 7:
    x0, y0, w, h = map(int, sys.argv[3:7])
    g, o = g[y0:y0 + h, x0:x0 + w], o[y0:y0 + h, x0:x0 + w]
d = (g != o).any(axis=2)
lab, n = ndimage.label(ndimage.binary_dilation(d, iterations=6))
for i, sl in enumerate(ndimage.find_objects(lab)):
    cnt = int(d[sl].sum())
    print('region x=%d y=%d w=%d h=%d  differing=%d' % (sl[1].start + x0, sl[0].start + y0, sl[1].stop - sl[1].start, sl[0].stop - sl[0].start, cnt))
print('total', int(d.sum()))
