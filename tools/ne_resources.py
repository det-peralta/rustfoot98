"""Extract resources from a 16-bit NE executable; save RCDATA forms (TPF0) to a dir."""
import struct, sys, os
def ne_resources(d):
    ne = struct.unpack_from('<H', d, 0x3C)[0]
    assert d[ne:ne+2] == b'NE'
    rt = ne + struct.unpack_from('<H', d, ne+0x24)[0]
    shift = struct.unpack_from('<H', d, rt)[0]; p = rt + 2; out = []
    while True:
        tid = struct.unpack_from('<H', d, p)[0]
        if tid == 0: break
        cnt = struct.unpack_from('<H', d, p+2)[0]; p += 8
        for _ in range(cnt):
            off, ln, flags, rid = struct.unpack_from('<HHHH', d, p); p += 12
            name = None
            if rid & 0x8000: rid &= 0x7FFF
            else:
                o = rt + rid; name = d[o+1:o+1+d[o]].decode('latin-1')
            out.append((tid, rid, name, d[off<<shift:(off<<shift)+(ln<<shift)]))
        # type-name strings handled by caller if tid&0x8000==0
    return out
def type_name(d, rt, tid):
    o = rt + (tid); return d[o+1:o+1+d[o]].decode('latin-1')
if __name__ == '__main__':
    exe, outdir = sys.argv[1], sys.argv[2]; os.makedirs(outdir, exist_ok=True)
    d = open(exe, 'rb').read()
    ne = struct.unpack_from('<H', d, 0x3C)[0]; rt = ne + struct.unpack_from('<H', d, ne+0x24)[0]
    shift = struct.unpack_from('<H', d, rt)[0]; p = rt + 2; n = 0
    while True:
        tid = struct.unpack_from('<H', d, p)[0]
        if tid == 0: break
        cnt = struct.unpack_from('<H', d, p+2)[0]; p += 8
        tname = ('%d' % (tid & 0x7FFF)) if tid & 0x8000 else d[rt+tid+1:rt+tid+1+d[rt+tid]].decode('latin-1')
        for _ in range(cnt):
            off, ln, fl, rid = struct.unpack_from('<HHHH', d, p); p += 12
            if rid & 0x8000: nm = str(rid & 0x7FFF)
            else: o = rt + rid; nm = d[o+1:o+1+d[o]].decode('latin-1')
            data = d[off<<shift:(off<<shift)+(ln<<shift)]
            print(tname, nm, len(data), data[:8])
            if tname == '10' and data[:4] == b'TPF0':
                open(os.path.join(outdir, nm + '.dfm'), 'wb').write(data); n += 1
    print('forms saved', n)
