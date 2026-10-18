#!/usr/bin/env python3
"""mouse.py X Y [click|dblclick|rclick]: moves the original's (relative, accelerated) mouse pointer to guest pixel (X, Y) by looking at the
frame, finding the arrow and correcting until it is within a pixel or two; then optionally clicks. Run from the repository root with the
local js-dos instance up (tools/capture/boot.sh)."""
import json, subprocess, sys, time
import numpy as np
from PIL import Image

ARROW = [
    "##", "#W#", "#WW#", "#WWW#", "#WWWW#", "#WWWWW#", "#WWWWWW#", "#WWWWWWW#", "#WWWWWWWW#", "#WWWWWWWWW#", "#WWWWWW#####",
    "#WWW#WW#", "#WW##WW#", "#W# ##WW#", "##   #WW#", "#     #WW#", "      #WW#", "       #WW#", "       #WW#", "        ##",
]
KEY = [(x, y, ch) for y, row in enumerate(ARROW) for x, ch in enumerate(row) if ch in '#W']


def eval_js(js):
    subprocess.run(['bash', 'tools/capture/c.sh', json.dumps({"op": "eval", "js": js})], capture_output=True)


def capture(path='/tmp/mouse_frame.png'):
    subprocess.run(['bash', 'tools/capture/c.sh', json.dumps({"op": "guest", "path": path})], capture_output=True)
    return np.array(Image.open(path).convert('RGB')).astype(int)


def find_arrow(img):
    black = img.sum(axis=2) == 0
    white = img.sum(axis=2) >= 740
    H, W = black.shape
    ok = np.ones((H - 20, W - 12), dtype=bool)
    for x, y, ch in KEY:
        m = black if ch == '#' else white
        ok &= m[y:y + H - 20, x:x + W - 12]
    ys, xs = np.nonzero(ok)
    return (int(xs[0]), int(ys[0])) if len(xs) else None


def move_rel(dx, dy):
    js = "(async()=>{const ci=await window.dosInstance.ciPromise; const w=ms=>new Promise(r=>setTimeout(r,ms)); ci.sendMouseRelativeMotion(%d,%d); await w(200); return 1})()" % (dx, dy)
    eval_js(js)


def goto(x, y):
    for _ in range(14):
        pos = find_arrow(capture())
        if pos is None:
            # the pointer is hidden or off the frame: push it towards the top-left corner in steps the PS/2 mouse accepts
            for _ in range(12):
                move_rel(-100, -100)
            continue
        dx, dy = x - pos[0], y - pos[1]
        if abs(dx) <= 1 and abs(dy) <= 1:
            return pos
        # one unit moves about two pixels; small steps stay accurate (the driver accelerates large ones)
        step = lambda d: max(-40, min(40, round(d / 2.0)))
        move_rel(step(dx), step(dy))
    return pos


def click(kind):
    n = 2 if kind == 'dblclick' else 1
    btn = 2 if kind == 'rclick' else 0
    js = "(async()=>{const ci=await window.dosInstance.ciPromise; const w=ms=>new Promise(r=>setTimeout(r,ms)); for(let i=0;i<%d;i++){ci.sendMouseButton(%d,true); await w(100); ci.sendMouseButton(%d,false); await w(250);} return 1})()" % (n, btn, btn)
    eval_js(js)


if __name__ == '__main__':
    x, y = int(sys.argv[1]), int(sys.argv[2])
    p = goto(x, y)
    print('pointer at', p)
    if len(sys.argv) > 3:
        click(sys.argv[3])
