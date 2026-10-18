"""Delphi 1 TPF0 binary form parser -> JSON-able tree."""
import struct, sys, os, json, glob
class R:
    def __init__(s, d): s.d, s.o = d, 0
    def u8(s): v = s.d[s.o]; s.o += 1; return v
    def ps(s): n = s.u8(); v = s.d[s.o:s.o+n].decode('latin-1'); s.o += n; return v
    def i(s, f, n): v = struct.unpack_from(f, s.d, s.o)[0]; s.o += n; return v
def value(r):
    t = r.u8()
    if t == 0: return None
    if t == 1:
        out = []
        while r.d[r.o] != 0: out.append(value(r))
        r.o += 1; return out
    if t == 2: return r.i('<b', 1)
    if t == 3: return r.i('<h', 2)
    if t == 4: return r.i('<i', 4)
    if t == 5: b = r.d[r.o:r.o+10]; r.o += 10; return {'extended': b.hex()}
    if t == 6: return r.ps()
    if t == 7: return {'ident': r.ps()}
    if t == 8: return False
    if t == 9: return True
    if t == 10: n = r.i('<I', 4); b = r.d[r.o:r.o+n]; r.o += n; return {'binary': len(b), 'hex': b.hex()}
    if t == 11:
        out = []
        while True:
            s = r.ps()
            if not s: break
            out.append(s)
        return {'set': out}
    if t == 13: return {'nil': True}
    raise ValueError('value type %d at %d' % (t, r.o))
def obj(r):
    cls = r.ps(); name = r.ps(); props = {}; kids = []
    while r.d[r.o] != 0:
        k = r.ps(); props[k] = value(r)
    r.o += 1
    while r.d[r.o] != 0: kids.append(obj(r))
    r.o += 1
    return {'class': cls, 'name': name, 'props': props, 'children': kids}
def parse(d):
    assert d[:4] == b'TPF0'; r = R(d); r.o = 4; return obj(r)
if __name__ == '__main__':
    ok = 0
    for p in sorted(glob.glob(sys.argv[1] + '/*.dfm')):
        try:
            t = parse(open(p, 'rb').read()); ok += 1
            json.dump(t, open(p[:-4] + '.json', 'w'), indent=1, ensure_ascii=False)
        except Exception as e: print('FAIL', os.path.basename(p), e)
    print('parsed', ok)
