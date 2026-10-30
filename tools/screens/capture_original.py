#!/usr/bin/env python3
"""Captures the screens of tools/screens/manifest.py from the original under js-dos (analysis/screens/original/<id>.png).

Needs the original booted with the game ABCDEEEE loaded (tools/capture/boot.sh, then Tab Tab Space Tab A Enter) on its main window.
Run from the repository root with analysis/captures/venv:  python tools/screens/capture_original.py [id ...]
The captures are the game's own pictures: they stay in analysis/ (git-ignored)."""
import json, os, subprocess, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'capture'))
from manifest import SCREENS  # noqa: E402
import mouse  # noqa: E402

OUT = 'analysis/screens/original'


def c(op):
    return subprocess.run(['bash', 'tools/capture/c.sh', json.dumps(op)], capture_output=True, text=True).stdout


def kind(path):
    return subprocess.run([sys.executable, 'tools/capture/classify.py', path], capture_output=True, text=True).stdout.strip()


def same_as_reference(path):
    """The frame shows the main window as captured first (page Jogo), apart from the mouse pointer."""
    import numpy as np
    from PIL import Image
    ref = os.path.join(OUT, 'main_0_jogo.png')
    a = np.array(Image.open(ref).convert('RGB')).astype(int) >> 3
    b = np.array(Image.open(path).convert('RGB')).astype(int) >> 3
    return int((a != b).any(axis=2).sum()) < 600


def run(steps):
    for s in steps:
        if s[0] == 'key':
            c({"op": "key", "key": s[1]})
        elif s[0] == 'click':
            mouse.goto(s[1], s[2])
            mouse.click('click')
        elif s[0] == 'wait':
            c({"op": "wait", "ms": s[1]})


def main():
    os.makedirs(OUT, exist_ok=True)
    want = set(sys.argv[1:])
    for s in SCREENS:
        if not s.get('steps') or (want and s['id'] not in want):
            continue
        run(s['steps'])
        c({"op": "wait", "ms": 800})
        path = os.path.abspath(os.path.join(OUT, s['id'] + '.png'))
        c({"op": "guest", "path": path})
        print(s['id'], flush=True)
        run(s['back'])
        # back on the main window as it was (the first capture of the run is the reference): stop rather than press anything else
        back = os.path.join(OUT, '..', 'back.png')
        c({"op": "wait", "ms": 600})
        c({"op": "guest", "path": os.path.abspath(back)})
        if not s.get('final') and not same_as_reference(back):
            sys.exit('not back on the main window after %s (see %s)' % (s['id'], back))

if __name__ == '__main__':
    main()
