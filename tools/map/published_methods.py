"""Recovers the Delphi 1 published-method names of ELIFOOT.EXE.

Delphi 1 stores, inside the code segment of each unit, a table of [far pointer][pascal string] entries for the published
methods of every form class (the ones the .dfm refers to, e.g. Timer1Timer). A hit is a far pointer to a known function start
followed by a Pascal string that looks like an identifier. Usage:
    python3 tools/map/published_methods.py analysis/out/blocks analysis/out/functions.txt > analysis/out/published.txt
Each output line: `seg:off<TAB>name`.
"""
import re, sys, glob, os, struct

def main(blocks, functions):
    starts = set()
    for line in open(functions):
        p = line.split('\t')
        if len(p) >= 3 and re.match(r'[0-9a-f]{4}:[0-9a-f]{4}$', p[0]):
            starts.add(p[0])
    seen = {}
    for path in sorted(glob.glob(os.path.join(blocks, '*_Code*.bin'))):
        seg = os.path.basename(path)[:4]
        d = open(path, 'rb').read()
        for i in range(0, len(d) - 8):
            off, tseg = struct.unpack_from('<HH', d, i)
            addr = f'{tseg:04x}:{off:04x}'
            if addr not in starts:
                continue
            n = d[i + 4]
            if not 3 <= n <= 40 or i + 5 + n > len(d):
                continue
            name = d[i + 5:i + 5 + n]
            if not re.fullmatch(rb'[A-Za-z_][A-Za-z0-9_]*', name):
                continue
            name = name.decode()
            # published handlers look like <Component><Event> (Click, Timer, Change, ...) or are plain method names
            seen.setdefault(addr, name)
    for a, n in sorted(seen.items()):
        print(f'{a}\t{n}')

if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2])
