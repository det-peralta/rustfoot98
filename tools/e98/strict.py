#!/usr/bin/env python3
"""Strict structural walk of an .e98 save following the loader 1020:1d8d; reports where it ends."""
import struct, sys
class R:
    def __init__(s,d,o=0): s.d=d; s.o=o
    def u8(s): v=s.d[s.o]; s.o+=1; return v
    def u16(s): v=struct.unpack_from('<H',s.d,s.o)[0]; s.o+=2; return v
    def i32(s): v=struct.unpack_from('<i',s.d,s.o)[0]; s.o+=4; return v
    def raw(s,n): v=s.d[s.o:s.o+n]; s.o+=n; return v
    def pstr(s): n=s.u8(); return s.raw(n)
d=open(sys.argv[1],'rb').read()
r=R(d)
n=r.u8(); r.raw(n)               # description
r.raw(2+2+1+2+4+10+10+101)        # header fields
print('header ends at %#x'%r.o)
def lst(name,elem):
    c=r.u16(); start=r.o
    out=[elem() for _ in range(c)]
    if name[0]!=' ': print('%-8s count=%-4d %#x..%#x'%(name,c,start-2,r.o)); return out
def ev7(): r.raw(7)
def e8382():
    r.u16(); r.pstr(); r.u8(); r.u8(); r.u16(); r.raw(4+4+5)
    c=r.u16(); [ev7() for _ in range(c)]

def pn(): return r.pstr()
def e8382():
    r.u16(); pn(); r.u8(); r.u8(); r.u16(); r.raw(4+4+5)
    c=r.u16(); [r.raw(7) for _ in range(c)]
def ec770(): r.u16(); r.u8(); pn(); pn()
def player():
    r.u16(); pn(); pn(); r.raw(48)
def fixture(): r.raw(16)
def team():
    tid=r.u16()
    r.raw(50); pn(); pn(); r.raw(8); pn(); r.u8()          # EFT object 1058:15b8
    r.raw(0x37+0xe+6+2+28+28+4+4+4+10+6+4+2+2)
    lst('  squad',player)
    lst('  ba06',lambda:r.raw(7))
    lst('  bd8a',lambda:r.raw(8))
    lst('  fixt',fixture)
    r.u8(); r.raw(6)                                      # stadium
    return tid
def division():
    r.raw(4+2+21); c=r.u16(); [r.u16() for _ in range(c)]
def game():
    r.u8(); r.raw(41); r.u16(); r.u16(); r.raw(10); r.raw(4); r.u16(); r.u16(); r.u16(); r.u8()
quiet=True
lst('2702',e8382)
lst('2706',ec770)
teams=lst('26f6',team)
lst('26f2',r.u16)
lst('26fa',division)
lst('270a',game)
n=0
while True:
    t=r.u16()
    if t==0: break
    c=r.u16(); [r.u16() for _ in range(c)]; n+=1
print('1b1b groups',n,'at %#x'%r.o)
lst('270e',lambda:r.raw(10))
lst('2712',lambda:r.raw(12))
print('end %#x of %#x'%(r.o,len(d)))
