"""Lists, for each serializer of the .e98 save, the sequence of raw writes and nested writer calls (from the decompiled C)."""
import re, sys, glob
bodies = {}
for path in glob.glob('analysis/out/corpus/*.c'):
    cur = None
    for line in open(path, errors='replace'):
        m = re.match(r'// ===== ([0-9a-f]{4}:[0-9a-f]{4})', line)
        if m: cur = m.group(1); bodies[cur] = []
        elif cur: bodies[cur].append(line)
def show(addr, indent=0, seen=None):
    seen = seen if seen is not None else set()
    if addr in seen or addr not in bodies: return
    seen.add(addr)
    text = ''.join(bodies[addr])
    print(' ' * indent + f'== {addr}')
    for m in re.finditer(r'(BlockWrite_10c0_0f67|FUN_10c0_0f67)\(0,0,([^,]+),([^;]*?)\);|(\w+_1[0-9a-f]{3}_[0-9a-f]{4})\(', text):
        if m.group(1):
            print(' ' * indent + f'  write {m.group(2).strip()} bytes of {m.group(3).strip()[:60]}')
        else:
            name = m.group(4)
            a = name[-9:].replace('_', ':')
            if a in bodies and a != addr and not name.startswith(('StackCheck','IOCheck','IOResult','ListGet','Max','Min')):
                print(' ' * indent + f'  call {name}')
for a in sys.argv[1:]:
    show(a)
