"""Extracts, from the disassembly of the save/load routines, the sequence of raw file operations of each function.

Input: analysis/out/io_funcs.lst (tools/ghidra/ListFunctions.java on the functions reachable from 1020:1796 / 1020:1d8d).
Output: for every function, lines like `R 2 field+0x4` (BlockRead of 2 bytes into object field +4), `W 55 field+0x16`,
`R 2 local-6`, `call 1010:2031`, `str` (a Pascal string reader) — enough to read the .e98 layout off the code.
"""
import re, sys

def main(path):
    funcs = {}
    cur = None
    for line in open(path):
        if line.startswith('### '):
            cur = line[4:].strip(); funcs[cur] = []
        elif cur:
            funcs[cur].append(line.rstrip('\n'))
    names = {}
    for l in open('tools/map/known_names.tsv'):
        a, n = l.rstrip('\n').split('\t'); names[a] = n
    for f, lines in funcs.items():
        print(f'== {f} {names.get(f, "")}')
        ops = []
        for i, l in enumerate(lines):
            m = re.search(r'CALLF 0x10c0:0f(60|67)', l)
            if m:
                kind = 'R' if m.group(1) == '60' else 'W'
                # size = the PUSH imm before "XOR AX,AX / PUSH AX / PUSH AX"
                size = None
                j = i - 1
                while j > 0 and size is None and i - j < 8:
                    mm = re.search(r'PUSH (-?0x[0-9a-f]+)\s*$', lines[j])
                    if mm and 'XOR AX,AX' in lines[j + 1]:
                        size = int(mm.group(1), 16)
                    j -= 1
                ctx = ' | '.join(x.split('  ', 2)[-1].strip()[:44] for x in lines[max(0, i - 14):i] if re.search(r'LEA DI|ADD DI|LES DI,ES:\[DI \+|MOV DI', x))
                ops.append(f'{kind} {size} ({ctx[-90:]})')
                continue
            m = re.search(r'CALLF 0x(1[0-9a-f]{3}):0x([0-9a-f]{4})', l)
            if m:
                a = f'{m.group(1)}:{m.group(2)}'
                if a in funcs and a != f:
                    ops.append(f'call {a} {names.get(a, "")}')
                elif a in ('1058:044f', '1058:0401'):
                    ops.append(f'call {a}')
        print('\n'.join('   ' + o for o in ops))

if __name__ == '__main__':
    main(sys.argv[1])
