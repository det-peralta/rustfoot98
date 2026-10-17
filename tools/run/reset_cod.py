#!/usr/bin/env python3
"""Delete C:\\WINDOWS\\eli.cod from the *run copy* of the Windows 3.1 image so Elifoot recreates it.
eli.cod is a per-install id file (the game writes strings derived from Random(20000) when it is
missing; see 1058:30ba). The FAT chain is left allocated (harmless). Usage: reset_cod.py <hdd.img>"""
import sys
p = sys.argv[1]; d = bytearray(open(p, 'rb').read()); n = 0; i = 0
while True:
    i = d.find(b'ELI     COD', i)
    if i < 0: break
    if d[i+11] == 0 and int.from_bytes(d[i+28:i+32], 'little') == 35:
        d[i] = 0xE5; n += 1
    i += 1
open(p, 'wb').write(d); print('marked deleted:', n)
