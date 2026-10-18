#!/usr/bin/env python3
"""Windows 3.1 style TrueType glyph rasteriser: hinted outline from FreeType (bytecode interpreter v35, mono target), filled
without dropout control by sampling pixel centres (non-zero winding), which is what the original GDI produced.
Needs freetype-py. Fonts come from the user's own Windows install (``tools/ui/fat16.py`` extracts them)."""
import math
import os
os.environ.setdefault('FREETYPE_PROPERTIES', 'truetype:interpreter-version=35')
import freetype

LOAD = freetype.FT_LOAD_TARGET_MONO | freetype.FT_LOAD_NO_BITMAP
DROPOUT = False
GROW = 0.0  # horizontal tolerance of the pixel-centre test (tuned against the original)
EPS = (0.0, 0.0)  # sampling offset, tuned against captures of the original


def _contours(o):
    pts, tags, out, s = o.points, o.tags, [], 0
    for e in o.contours:
        out.append(([(x / 64.0, y / 64.0) for x, y in pts[s:e + 1]], [t & 1 for t in tags[s:e + 1]]))
        s = e + 1
    return out


def _flatten(P, T, steps=16):
    n = len(P)
    i0 = next((i for i in range(n) if T[i]), None)
    if i0 is None:
        start = ((P[0][0] + P[-1][0]) / 2, (P[0][1] + P[-1][1]) / 2)
        seq = list(range(n))
    else:
        start = P[i0]
        seq = [(i0 + 1 + k) % n for k in range(n)]
    poly, ctrl, cur = [start], None, start

    def quad(a, c, b):
        for k in range(1, steps + 1):
            t = k / steps
            poly.append(((1 - t) ** 2 * a[0] + 2 * (1 - t) * t * c[0] + t * t * b[0],
                         (1 - t) ** 2 * a[1] + 2 * (1 - t) * t * c[1] + t * t * b[1]))

    for k in seq:
        p = P[k]
        if T[k]:
            if ctrl is None:
                poly.append(p)
            else:
                quad(cur, ctrl, p)
                ctrl = None
            cur = p
        else:
            if ctrl is not None:
                mid = ((ctrl[0] + p[0]) / 2, (ctrl[1] + p[1]) / 2)
                quad(cur, ctrl, mid)
                cur = mid
            ctrl = p
    if ctrl is not None:
        quad(cur, ctrl, start)
    return poly


def fill(polys, x0, y0, w, h, eps=EPS):
    """Rows (top to bottom) of ints: bit (w-1-c) is column c; (x0, y0) is the font-space point at the top-left pixel corner."""
    rows = []
    for r in range(h):
        y = y0 - r - 0.5 + eps[1]
        xs = []
        for poly in polys:
            m = len(poly)
            for i in range(m):
                a, b = poly[i], poly[(i + 1) % m]
                if (a[1] <= y) != (b[1] <= y):
                    t = (y - a[1]) / (b[1] - a[1])
                    xs.append((a[0] + t * (b[0] - a[0]), 1 if b[1] > a[1] else -1))
        xs.sort()
        bits, wn = 0, 0
        for k, (x, d) in enumerate(xs[:-1]):
            wn += d
            if wn:
                nxt = xs[k + 1][0]
                # pixel c is covered when its centre c + 0.5 lies in [x - GROW, nxt + GROW)
                c = max(0, int(math.ceil(x - GROW - x0 - 0.5 - eps[0])))
                first = c
                while c <= w - 1 and c + 0.5 + x0 + eps[0] < nxt + GROW:
                    bits |= 1 << (w - 1 - c)
                    c += 1
                if DROPOUT and c == first:
                    # the span [x, nxt) holds no pixel centre: light the pixel it falls in (horizontal dropout control)
                    mid = int(math.floor((x + nxt) / 2 - x0))
                    if 0 <= mid < w:
                        bits |= 1 << (w - 1 - mid)
        rows.append(bits)
    return rows


class Face:
    def __init__(self, path, px):
        self.f = freetype.Face(path)
        self.f.set_pixel_sizes(0, px)
        self.px = px
        m = self.f.size
        # GDI's tmAscent of these fonts is one more than FreeType's hinted ascender (measured against the original)
        self.ascent = (m.ascender >> 6) + 1
        self.descent = -(m.descender >> 6)
        self.height = m.height >> 6

    def glyph(self, ch):
        """(advance, left, top, width, rows) of one character, rendered like the original."""
        self.f.load_char(ch, LOAD)
        g = self.f.glyph
        adv = g.advance.x >> 6
        polys = [_flatten(*c) for c in _contours(g.outline)]
        if not polys:
            return adv, 0, 0, 0, []
        xs = [p[0] for poly in polys for p in poly]
        ys = [p[1] for poly in polys for p in poly]
        left = int(min(xs) // 1)
        right = int(-(-max(xs) // 1))
        top = int(-(-max(ys) // 1))
        bottom = int(min(ys) // 1)
        w, h = right - left, top - bottom
        return adv, left, top, w, fill(polys, left, top, w, h)
