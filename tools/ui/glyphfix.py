#!/usr/bin/env python3
"""Glyph corrections taken from the captures of the original.

The port rasterises the Windows 3.1 TrueType fonts itself (tools/ui/ttraster.py); a few glyphs come out a pixel different from
Windows'. Windows 3.1 draws a TrueType glyph of a given size always with the same bitmap, so every glyph the original shows in
a capture is its exact bitmap. This tool finds them:

  harvest  renders every exact screen of tools/screens/manifest.py with the glyph log (GLYPHLOG), and wherever a glyph of the
           port differs from the original's pixels in the same place (its box and a pixel around it, nothing else drawn
           there), takes the original's bitmap; a correction is kept when one version dominates (three times the places where the port
           already agrees, and most of the differing ones).
           Writes analysis/out/skin/glyph_fix.json (merged with what is there).
  apply    writes the corrections into analysis/out/skin/fonts.bin (from the pristine copy fonts.orig.bin, made the first
           time) and copies it to web/skin/fonts.bin.

Both files are derived from the fonts and stay local (git-ignored). Run from the repository root with analysis/captures/venv."""
import json, os, shutil, struct, subprocess, sys
from collections import Counter, defaultdict
import numpy as np
from PIL import Image

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'screens'))
from manifest import SCREENS  # noqa: E402

SKIN = 'analysis/out/skin'
FIX = os.path.join(SKIN, 'glyph_fix.json')
TOOLS = {'classic': 'target/release/classic_shot', 'editor': 'target/release/editor_shot'}
TMP = 'analysis/screens/glyphs'


def load(path):
    return np.array(Image.open(path).convert('RGB')).astype(int) >> 3


def render(s):
    os.makedirs(TMP, exist_ok=True)
    bmp, log = os.path.join(TMP, s['id'] + '.bmp'), os.path.join(TMP, s['id'] + '.log')
    args = list(s['port'])
    args = args[:1] + [bmp] + args[1:] if s['tool'] == 'classic' else [bmp] + args
    r = subprocess.run([TOOLS[s['tool']]] + args, capture_output=True, env=dict(os.environ, GLYPHLOG=log))
    if r.returncode != 0:
        return None, []
    recs = []
    for line in open(log).read().splitlines():
        f = line.split('|')
        if len(f) != 11:
            continue
        clip = tuple(int(v) for v in f[10].split(','))
        recs.append({'key': '|'.join(f[:5]), 'x': int(f[5]), 'y': int(f[6]), 'w': int(f[7]), 'h': int(f[8]),
                     'c': tuple(int(v) >> 3 for v in f[9].split(',')), 'clip': clip})
    return load(bmp), recs


def original_of(s):
    if s.get('capture'):
        return os.path.join('analysis/captures', s['capture'])
    return os.path.join('analysis/screens/original', s['id'] + '.png')


def harvest():
    seen = defaultdict(lambda: {'agree': 0, 'cands': []})
    for s in SCREENS:
        if not s['exact'] or not os.path.exists(original_of(s)):
            continue
        port, recs = render(s)
        if port is None:
            continue
        orig = load(original_of(s))
        H, W = port.shape[:2]
        for i, g in enumerate(recs):
            if g['w'] == 0 or g['h'] == 0:
                continue
            x0, y0, x1, y1 = g['x'] - 1, g['y'] - 1, g['x'] + g['w'] + 1, g['y'] + g['h'] + 1
            cx0, cy0, cx1, cy1 = g['clip']
            if x0 < max(cx0, 0) or y0 < max(cy0, 0) or x1 > min(cx1, W) or y1 > min(cy1, H):
                continue  # partly clipped
            # no other glyph inside the box itself; the margin leaves out the boxes of the neighbours
            others = [o for j, o in enumerate(recs) if j != i and o['x'] < x1 and o['x'] + o['w'] > x0
                      and o['y'] < y1 and o['y'] + o['h'] > y0]
            if any(o['x'] < x1 - 1 and o['x'] + o['w'] > x0 + 1 and o['y'] < y1 - 1 and o['y'] + o['h'] > y0 + 1
                   for o in others):
                continue
            own = np.ones((y1 - y0, x1 - x0), dtype=bool)
            for o in others:
                ox0, oy0 = max(o['x'], x0) - x0, max(o['y'], y0) - y0
                ox1, oy1 = min(o['x'] + o['w'], x1) - x0, min(o['y'] + o['h'], y1) - y0
                own[oy0:oy1, ox0:ox1] = False
            own[1:-1, 1:-1] = True
            c = np.array(g['c'])
            po, oo = port[y0:y1, x0:x1], orig[y0:y1, x0:x1]
            pm, om = (po == c).all(axis=2) & own, (oo == c).all(axis=2) & own
            # outside the ink both pictures must show the same background
            if not ((po == oo).all(axis=2) | pm | om | ~own).all():
                continue
            if pm.sum() == 0:
                continue  # painted over by a window drawn later
            if (pm == om).all():
                seen[g['key']]['agree'] += 1
                continue
            ys, xs = np.nonzero(om)
            if len(xs) == 0:
                continue
            bx0, by0 = xs.min(), ys.min()
            m = om[by0:ys.max() + 1, bx0:xs.max() + 1]
            seen[g['key']]['cands'].append((int(bx0) - 1, int(by0) - 1, tuple(map(tuple, m.astype(int).tolist()))))
    fixes = json.load(open(FIX)) if os.path.exists(FIX) else {}
    new = 0
    for key, v in seen.items():
        if not v['cands']:
            continue
        # the most common version of the original's glyph, if it clearly dominates the places where the port already agrees
        # (a few places can disagree: a glyph partly covered, a neighbour's pixels in its margin)
        best, n = Counter(v['cands']).most_common(1)[0]
        if n >= 3 * v['agree'] and n * 10 >= 7 * len(v['cands']):
            dx, dy, m = best
            fixes[key] = {'dx': dx, 'dy': dy, 'rows': [''.join('#' if b else '.' for b in r) for r in m], 'seen': len(v['cands'])}
            new += 1
    json.dump(fixes, open(FIX, 'w'), indent=0, sort_keys=True)
    print('corrections: %d new or renewed, %d in all (%d glyph kinds compared)' % (new, len(fixes), len(seen)))


def read_fonts(d):
    o = 4
    n, = struct.unpack_from('<H', d, o); o += 2
    fonts = []
    for _ in range(n):
        ln = d[o]; face = d[o + 1:o + 1 + ln].decode(); o += 1 + ln
        head = struct.unpack_from('<BBBBBBBBB', d, o); o += 9
        glyphs = []
        for _c in range(224):
            adv, left, top, w, nr = struct.unpack_from('<BbbBB', d, o); o += 5
            nb = (w + 7) // 8
            rows = [int.from_bytes(d[o + r * nb:o + (r + 1) * nb], 'big') >> (nb * 8 - w) if w else 0 for r in range(nr)]
            o += nr * nb
            glyphs.append([adv, left, top, w, rows])
        fonts.append([face, head, glyphs])
    return fonts


def write_fonts(fonts):
    out = bytearray(b'EFSK') + struct.pack('<H', len(fonts))
    for face, head, glyphs in fonts:
        nm = face.encode()
        out += struct.pack('<B', len(nm)) + nm + struct.pack('<BBBBBBBBB', *head)
        for adv, left, top, w, rows in glyphs:
            out += struct.pack('<BbbBB', adv, left, top, w, len(rows))
            nb = (w + 7) // 8
            for r in rows:
                out += (r << (nb * 8 - w)).to_bytes(nb, 'big') if w else b''
    return bytes(out)


def apply():
    base = os.path.join(SKIN, 'fonts.orig.bin')
    if not os.path.exists(base):
        shutil.copy(os.path.join(SKIN, 'fonts.bin'), base)
    fonts = read_fonts(open(base, 'rb').read())
    fixes = json.load(open(FIX)) if os.path.exists(FIX) else {}
    done = 0
    for key, f in fixes.items():
        face, px, bold, italic, code = key.split('|')
        for fface, head, glyphs in fonts:
            # head: px, bold, italic, ascent, descent, first, ul_pos, ul_thick, tm_height
            if fface == face and head[0] == int(px) and head[1] == int(bold) and head[2] == int(italic):
                k = int(code) - head[5]
                if 0 <= k < len(glyphs):
                    adv, left, top, w, rows = glyphs[k]
                    m = f['rows']
                    glyphs[k] = [adv, left + f['dx'], top - f['dy'], len(m[0]),
                                 [int(r.replace('#', '1').replace('.', '0'), 2) for r in m]]
                    done += 1
    data = write_fonts(fonts)
    open(os.path.join(SKIN, 'fonts.bin'), 'wb').write(data)
    if os.path.isdir('web/skin'):
        open('web/skin/fonts.bin', 'wb').write(data)
    print('applied %d corrections' % done)


if __name__ == '__main__':
    {'harvest': harvest, 'apply': apply}[sys.argv[1]]()
