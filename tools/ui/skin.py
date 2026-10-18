#!/usr/bin/env python3
"""Builds the *skin pack* the pixel-exact Windows 3.1 UI needs, from the user's own game and Windows 3.1 install.

Nothing it writes may be committed (fonts and bitmaps are copyrighted): the output goes to ``analysis/out/skin/``.

  python3 tools/ui/skin.py            # needs freetype-py (pip install freetype-py)

Writes ``fonts.bin`` (rasterised glyphs of every font the forms use) and ``glyphs.bin`` (the BitBtn glyph bitmaps of the forms).
"""
import glob, json, os, struct, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
ROOT = os.path.abspath(os.path.join(HERE, '..', '..'))
OUT = os.path.join(ROOT, 'analysis', 'out', 'skin')
FORMS = os.path.join(ROOT, 'analysis', 'out', 'forms')
SYSTEM = os.path.join(ROOT, 'original', 'run', 'd', 's3dri')  # Windows 3.1 disk files (SZDD) next to the game
IMG = os.path.join(ROOT, 'original', 'run', 'hdd.img')

# (Delphi face, bold, italic) -> TrueType file of the Windows 3.1 install
TT = {('arial', 0, 0): 'ARIAL', ('arial', 1, 0): 'ARIALBD', ('arial', 1, 1): 'ARIALBI', ('arial', 0, 1): 'ARIALI',
      ('times new roman', 0, 0): 'TIMES', ('times new roman', 1, 0): 'TIMESBD', ('times new roman', 1, 1): 'TIMESBI',
      ('times new roman', 0, 1): 'TIMESI'}


def ttf(name):
    p = os.path.join(OUT, name + '.TTF')
    if not os.path.exists(p):
        import fat16
        os.makedirs(OUT, exist_ok=True)
        open(p, 'wb').write(fat16.Fat16(IMG).read('WINDOWS/SYSTEM/%s.TTF' % name))
    return p


def underline(path, px):
    """(offset below the baseline, thickness) of the underline GDI draws: 'post' table values scaled and rounded."""
    d = open(path, 'rb').read()
    n = struct.unpack('>H', d[4:6])[0]
    tab = {}
    for i in range(n):
        tag, cs, off, ln = struct.unpack('>4sIII', d[12 + 16 * i:28 + 16 * i])
        tab[tag] = off
    upem = struct.unpack('>H', d[tab[b'head'] + 18:tab[b'head'] + 20])[0]
    pos, th = struct.unpack('>hh', d[tab[b'post'] + 8:tab[b'post'] + 12])
    return max(1, round(-pos * px / upem)), max(1, round(th * px / upem))


def used_fonts():
    """(face, height, bold, italic) of every control of every form, following ParentFont inheritance."""
    seen = set()

    def walk(x, parent):
        pr = x['props']
        f = dict(parent)
        if 'Font.Name' in pr:
            f['name'] = pr['Font.Name']
        if 'Font.Height' in pr:
            f['h'] = pr['Font.Height']
        if 'Font.Style' in pr:
            f['style'] = pr['Font.Style']['set']
        if 'name' in f and 'h' in f:
            st = f.get('style', [])
            seen.add((f['name'].lower(), -f['h'], int('fsBold' in st), int('fsItalic' in st)))
        for y in x['children']:
            walk(y, f)

    for p in glob.glob(os.path.join(FORMS, '*.json')):
        walk(json.load(open(p)), {})
    return sorted(seen)


def pack_fonts():
    import ttraster
    ttraster.GROW = 0.0
    ttraster.DROPOUT = bool(int(os.environ.get('DROPOUT', '0')))
    out = bytearray(b'EFSK')
    entries = []
    for face, px, bold, italic in used_fonts():
        key = (face, bold, italic)
        if face == 'arial' and px == 9:
            continue  # raster font, see below
        if key not in TT:
            print('skip (bitmap or unknown font):', face, px, bold, italic)
            continue
        f = ttraster.Face(ttf(TT[key]), px)
        glyphs = []
        for code in range(32, 256):
            ch = bytes([code]).decode('cp1252', 'replace')
            adv, left, top, w, rows = f.glyph(ch)
            glyphs.append((adv, left, top, w, rows))
        entries.append((face, px, bold, italic, f.ascent, f.descent, glyphs, underline(ttf(TT[key]), px), f.tm_height))
    # bitmap fonts of the Windows 3.1 install: "MS Sans Serif" (8, 12, 24 pt) and "System"; bold is GDI's emulation (smear by one pixel)
    import fon
    def bitmap(path, face, wanted, only=None, as_face=None, as_px=None):
        for f in fon.read_fon(path):
            if (f['points'] in wanted or wanted is None) and (only is None or f['face'] == only):
                for bold in (0, 1):
                    glyphs = []
                    for w, rows in f['glyphs']:
                        if bold:
                            rows = [r | (r << 1) for r in rows]
                            w = w + 1 if w else w
                            adv = w
                        else:
                            adv = w
                        # bitmap fonts draw from the cell origin: left 0, top = ascent
                        glyphs.append((adv, 0, f['ascent'], w, [r << 0 for r in rows]))
                    # the window's px size in Delphi terms: 8 pt -> 11, 12 pt -> 16, 24 pt -> 32; System -> 13
                    px = {8: 11, 12: 16, 24: 32}.get(f['points'], 13) if face != 'system' else 13
                    entries.append((as_face or face, as_px or px, bold, 0, f['ascent'], f['height'] - f['ascent'], glyphs, (f['ascent'] + 1, 1), f['height']))
    bitmap(os.path.join(SYSTEM, 'SSERIFE.FON'), 'ms sans serif', {8, 12, 24})
    bitmap(os.path.join(SYSTEM, 'VGASYS.FON'), 'system', {10})
    # the Windows 3.1 font mapper draws Arial at 9 pixels with the raster font MS Serif 7 pt (measured against the original)
    bitmap(os.path.join(SYSTEM, 'SMALLE.FON'), 'ms serif', {7}, only='MS Serif', as_face='arial', as_px=9)
    out += struct.pack('<H', len(entries))
    for face, px, bold, italic, asc, desc, glyphs, ul, tmh in entries:
        nm = face.encode()
        out += struct.pack('<B', len(nm)) + nm + struct.pack('<BBBBBBBBB', px, bold, italic, asc, desc, 32, ul[0], ul[1], tmh)
        for adv, left, top, w, rows in glyphs:
            out += struct.pack('<BbbBB', adv, left, top, w, len(rows))
            nb = (w + 7) // 8
            for r in rows:
                out += (r << (nb * 8 - w)).to_bytes(nb, 'big') if w else b''
    path = os.path.join(OUT, 'fonts.bin')
    open(path, 'wb').write(out)
    print('fonts.bin', len(entries), 'fonts', len(out), 'bytes')


def pack_glyphs():
    """BitBtn glyph bitmaps: form, control, BMP bytes."""
    out = bytearray(b'EFGL')
    items = []
    for p in sorted(glob.glob(os.path.join(FORMS, '*.json'))):
        form = os.path.basename(p)[:-5]

        def walk(x):
            g = x['props'].get('Glyph.Data') or x['props'].get('Picture.Data')
            if isinstance(g, dict) and 'hex' in g:
                raw = bytes.fromhex(g['hex'])
                i = raw.find(b'BM')
                if i >= 0:
                    items.append((form, x['name'], raw[i:]))
            for y in x['children']:
                walk(y)

        walk(json.load(open(p)))
    # the bitmaps inside ELIFOOT.EXE itself (BBOK, BBCANCEL, ... the standard BitBtn glyphs, scroll arrows)
    exe = open(os.path.join(ROOT, 'original', 'elifoot98', 'ELIFOOT.EXE'), 'rb').read()
    ne = struct.unpack_from('<H', exe, 0x3C)[0]
    rt = ne + struct.unpack_from('<H', exe, ne + 0x24)[0]
    shift = struct.unpack_from('<H', exe, rt)[0]
    p = rt + 2
    while struct.unpack_from('<H', exe, p)[0]:
        tid, cnt = struct.unpack_from('<HH', exe, p)
        p += 8
        for _ in range(cnt):
            off, ln, fl, rid = struct.unpack_from('<HHHH', exe, p)
            p += 12
            if tid == 0x8002 and not rid & 0x8000:
                o = rt + rid
                nm = exe[o + 1:o + 1 + exe[o]].decode('latin-1')
                dib = exe[off << shift:(off << shift) + (ln << shift)]
                hs, w, h, planes, bpp = struct.unpack_from('<IiiHH', dib, 0)
                ncol = struct.unpack_from('<I', dib, 32)[0] or (1 << bpp)
                hdr = b'BM' + struct.pack('<IHHI', 14 + len(dib), 0, 0, 14 + hs + 4 * ncol)
                items.append(('*', nm, hdr + dib))
    out += struct.pack('<H', len(items))
    for form, name, bmp in items:
        a, b = form.encode(), name.encode()
        out += struct.pack('<B', len(a)) + a + struct.pack('<B', len(b)) + b + struct.pack('<I', len(bmp)) + bmp
    open(os.path.join(OUT, 'glyphs.bin'), 'wb').write(out)
    print('glyphs.bin', len(items), 'bitmaps', len(out), 'bytes')


if __name__ == '__main__':
    os.makedirs(OUT, exist_ok=True)
    for name in ('TIMES', 'TIMESBD', 'TIMESBI', 'TIMESI'):
        try:
            ttf(name)
        except FileNotFoundError:
            print('missing', name)
    pack_fonts()
    pack_glyphs()
