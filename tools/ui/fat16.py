#!/usr/bin/env python3
"""Minimal read-only FAT16 reader for a DOS hard-disk image (MBR with one partition), to take files out of the user's own
Windows 3.1 install: ``fat16.py image.img ls [dir]`` / ``fat16.py image.img cp path dest``."""
import struct, sys, os


class Fat16:
    def __init__(self, path):
        self.f = open(path, 'rb')
        mbr = self.f.read(512)
        lba = struct.unpack_from('<I', mbr, 446 + 8)[0]
        self.base = lba * 512
        self.f.seek(self.base)
        bs = self.f.read(512)
        self.bps, self.spc, self.reserved, self.nfats, self.rootents = struct.unpack_from('<HBHBH', bs, 11)
        total16, _, self.spf = struct.unpack_from('<HBH', bs, 19)
        self.fat_start = self.reserved * self.bps
        self.root_start = self.fat_start + self.nfats * self.spf * self.bps
        self.data_start = self.root_start + self.rootents * 32
        self.f.seek(self.base + self.fat_start)
        self.fat = self.f.read(self.spf * self.bps)

    def _read(self, off, n):
        self.f.seek(self.base + off)
        return self.f.read(n)

    def _next(self, c):
        return struct.unpack_from('<H', self.fat, c * 2)[0]

    def _chain(self, c):
        while 2 <= c < 0xfff0:
            yield c
            c = self._next(c)

    def _cluster(self, c):
        size = self.spc * self.bps
        return self._read(self.data_start + (c - 2) * size, size)

    def _entries(self, raw):
        for i in range(0, len(raw), 32):
            e = raw[i:i + 32]
            if e[0] == 0:
                break
            if e[0] == 0xe5 or e[11] == 0x0f:
                continue
            name = e[:8].decode('latin-1').rstrip()
            ext = e[8:11].decode('latin-1').rstrip()
            full = name + ('.' + ext if ext else '')
            attr = e[11]
            cl, size = struct.unpack_from('<HI', e, 26)
            yield full, attr, cl, size

    def listdir(self, cl=None):
        if cl is None:
            raw = self._read(self.root_start, self.rootents * 32)
        else:
            raw = b''.join(self._cluster(c) for c in self._chain(cl))
        return list(self._entries(raw))

    def find(self, path):
        cl, node = None, None
        parts = [p for p in path.replace('\\', '/').split('/') if p]
        for k, part in enumerate(parts):
            hit = [e for e in self.listdir(cl) if e[0].upper() == part.upper()]
            if not hit:
                raise FileNotFoundError(path)
            node = hit[0]
            cl = node[2]
        return node

    def read(self, path):
        name, attr, cl, size = self.find(path)
        return b''.join(self._cluster(c) for c in self._chain(cl))[:size]

    def walk(self, cl=None, prefix=''):
        for name, attr, c, size in self.listdir(cl):
            if name in ('.', '..'):
                continue
            if attr & 0x10:
                yield from self.walk(c, prefix + name + '/')
            else:
                yield prefix + name, size


if __name__ == '__main__':
    fs = Fat16(sys.argv[1])
    if sys.argv[2] == 'ls':
        for p, s in fs.walk():
            if len(sys.argv) < 4 or sys.argv[3].upper() in p.upper():
                print(s, p)
    elif sys.argv[2] == 'cp':
        data = fs.read(sys.argv[3])
        open(sys.argv[4], 'wb').write(data)
        print(len(data), 'bytes')
