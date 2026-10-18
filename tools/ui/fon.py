#!/usr/bin/env python3
"""Windows 3.x .FON raster font reader (NE container, RT_FONT resources, FNT v2/v3).

The fonts come from the user's own Windows 3.1 install (e.g. ``original/run/d/s3dri/SSERIFE.FON``); nothing is shipped.
``read_fon(path)`` returns a list of fonts: dict(face, points, height, ascent, weight, italic, first, last, default, glyphs=[(width, rows)])
where ``rows`` is a list of ints, one per pixel row, each holding the glyph's ``width`` bits (MSB = leftmost pixel).
"""
import struct, sys, json


def szdd(d):
    """Expands a Microsoft COMPRESS.EXE (SZDD) file, as found on the Windows 3.1 disks; other data is returned unchanged."""
    if d[:8] != b'SZDD\x88\xf0\x27\x33':
        return d
    size = struct.unpack_from('<I', d, 10)[0]
    win = bytearray(b' ' * 4096)
    pos, out, i = 4096 - 16, bytearray(), 14
    while len(out) < size and i < len(d):
        flags = d[i]
        i += 1
        for bit in range(8):
            if len(out) >= size or i >= len(d):
                break
            if flags >> bit & 1:
                c = d[i]
                i += 1
                out.append(c)
                win[pos] = c
                pos = (pos + 1) & 4095
            else:
                b1, b2 = d[i], d[i + 1]
                i += 2
                m = b1 | (b2 & 0xf0) << 4
                for k in range((b2 & 0x0f) + 3):
                    c = win[(m + k) & 4095]
                    out.append(c)
                    win[pos] = c
                    pos = (pos + 1) & 4095
    return bytes(out)


def _ne_resources(d):
    ne = struct.unpack_from('<I', d, 0x3c)[0]
    assert d[ne:ne + 2] == b'NE', 'not a NE file'
    rt = ne + struct.unpack_from('<H', d, ne + 0x24)[0]
    shift = struct.unpack_from('<H', d, rt)[0]
    o = rt + 2
    out = []
    while True:
        t = struct.unpack_from('<H', d, o)[0]
        if t == 0:
            break
        n = struct.unpack_from('<H', d, o + 2)[0]
        o += 8
        for _ in range(n):
            off, ln, fl, rid = struct.unpack_from('<HHHH', d, o)
            o += 12
            out.append((t, d[off << shift:(off << shift) + (ln << shift)]))
    return out


def parse_fnt(b):
    ver = struct.unpack_from('<H', b, 0)[0]
    (typ, pts, vres, hres, asc, intl, extl) = struct.unpack_from('<7H', b, 0x42)
    italic, underline, strike = b[0x50], b[0x51], b[0x52]
    weight, charset, pixw, pixh, pitch, avgw, maxw = struct.unpack_from('<HBHHBHH', b, 0x53)
    first, last, default, brk = b[0x5f], b[0x60], b[0x61], b[0x62]
    widthbytes = struct.unpack_from('<H', b, 0x63)[0]
    face_off = struct.unpack_from('<I', b, 0x69)[0]
    face = b[face_off:b.index(b'\0', face_off)].decode('latin-1') if face_off else ''
    n = last - first + 2  # includes the sentinel glyph
    table = 0x76 if ver == 0x200 else 0x94
    glyphs = []
    for i in range(last - first + 1):
        if ver == 0x200:
            w, off = struct.unpack_from('<HH', b, table + 4 * i)
        else:
            w, off = struct.unpack_from('<HI', b, table + 6 * i)
        cols = (w + 7) // 8
        rows = [0] * pixh
        for c in range(cols):
            for r in range(pixh):
                byte = b[off + c * pixh + r]
                rows[r] |= byte << (8 * (cols - 1 - c))
        # keep only the glyph's width bits, MSB first
        rows = [(v >> (cols * 8 - w)) & ((1 << w) - 1) if w else 0 for v in rows]
        glyphs.append((w, rows))
    return dict(face=face, points=pts, height=pixh, ascent=asc, weight=weight, italic=bool(italic), first=first,
                last=last, default=default, vres=vres, hres=hres, glyphs=glyphs)


def read_fon(path):
    d = szdd(open(path, 'rb').read())
    return [parse_fnt(b) for t, b in _ne_resources(d) if t == 0x8008]


if __name__ == '__main__':
    for p in sys.argv[1:]:
        for f in read_fon(p):
            print(p.split('/')[-1], f['face'], f['points'], 'pt h=%d asc=%d w=%d' % (f['height'], f['ascent'], f['weight']),
                  'chars %d-%d' % (f['first'], f['last']), 'dpi %d/%d' % (f['hres'], f['vres']))
