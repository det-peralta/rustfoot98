"""Elifoot 98 data-file reference decoders (Python, for exploration / golden data).

String encoding (all files): Pascal string [len][len bytes], delta-encoded:
    enc[i] = (enc[i-1] + plain[i]) & 0xFF,  enc[-1] = len
.TXE files apply the transform twice (double delta).

.EFT team file ('EFa' magic):
    0x00  'EFa' + zeros to 0x31 (49 bytes, all zero in the 195 shipped files)
    0x32  pstr full name, pstr short name
          8 bytes: colour1 RGBx, colour2 RGBx (kit colours)
          pstr country code (3 letters)
          u8 strength/rank (1..20), u8 squad size (14..20), u8 0
          squad x N: pstr nationality(3), pstr name, u8 pos (0=GK 1=DEF 2=MID 3=ATT), u8 0
          pstr coach name   (file ends exactly here)
"""
import glob, sys, json

def dec_body(b, seed):
    prev = seed; out = bytearray()
    for c in b:
        out.append((c - prev) & 0xFF); prev = c
    return bytes(out)

def dec_pstr(d, o, twice=False):
    n = d[o]; b = dec_body(d[o+1:o+1+n], n)
    if twice: b = dec_body(b, n)
    return b.decode('latin-1'), o + 1 + n

def parse_eft(d):
    assert d[:3] == b'EFa'
    full, o = dec_pstr(d, 0x32); short, o = dec_pstr(d, o)
    c1, c2 = d[o:o+3].hex(), d[o+4:o+7].hex(); o += 8
    country, o = dec_pstr(d, o)
    strength, squad_n = d[o], d[o+1]; o += 3
    players = []
    while True:
        nat, o2 = dec_pstr(d, o)
        if not (len(nat) == 3 and nat.isalpha()): break
        name, o3 = dec_pstr(d, o2)
        players.append({'nat': nat, 'name': name, 'pos': d[o3]}); o = o3 + 2
    coach, o = dec_pstr(d, o)
    assert o == len(d)
    return dict(full=full, short=short, colors=[c1, c2], country=country,
                strength=strength, squad=squad_n, players=players, coach=coach)

def parse_txe(d):
    out = []; o = 0
    while o < len(d):
        s, o = dec_pstr(d, o, twice=True); out.append(s)
    return out

if __name__ == '__main__':
    root = sys.argv[1]
    teams = {p.split('/')[-1]: parse_eft(open(p, 'rb').read()) for p in sorted(glob.glob(root + '/EQUIPAS/*.EFT'))}
    print(json.dumps({'teams': teams}, ensure_ascii=False)[:400])
