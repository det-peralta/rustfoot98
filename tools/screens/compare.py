#!/usr/bin/env python3
"""Renders the port's side of every screen of tools/screens/manifest.py and compares it with the original's capture.

  cargo build --release -p rustfoot-ui --bin classic_shot --bin editor_shot
  analysis/captures/venv/bin/python tools/screens/compare.py [--publish]

Writes analysis/screens/{port,diff}/<id>.png, the side-by-side report analysis/screens/index.html (all git-ignored: the pictures are
drawn with the game's own fonts and bitmaps) and the table of pixel counts docs/verification/screens.md (numbers only).
It also writes docs/verification/gallery.md, whose pictures `--publish` copies to docs/verification/img/ (git-ignored: the docs site
shows them when it is built locally by tools/site/build.sh).
The captures are 6 bits per channel, so both sides are compared on the top 5 bits."""
import html, os, shutil, subprocess, sys
import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'capture'))
from manifest import SCREENS  # noqa: E402
from mouse import find_arrow  # noqa: E402

ROOT = 'analysis/screens'
TOOLS = {'classic': 'target/release/classic_shot', 'editor': 'target/release/editor_shot'}


def load(path):
    return np.array(Image.open(path).convert('RGB')).astype(int)


def render(s):
    out = os.path.join(ROOT, 'port', s['id'] + '.png')
    bmp = out[:-4] + '.bmp'
    args = list(s['port'])
    if s['tool'] == 'classic':
        args = args[:1] + [bmp] + args[1:]
    else:
        args = [bmp] + args
    r = subprocess.run([TOOLS[s['tool']]] + args, capture_output=True, text=True)
    if r.returncode != 0 or not os.path.exists(bmp):
        return None, (r.stderr or r.stdout).strip()
    Image.open(bmp).convert('RGB').save(out)
    os.remove(bmp)
    return out, ''


def pointer_positions():
    """Where the capture run (tools/screens/capture_original.py, the manifest's order) left the mouse pointer for each screen: at
    the last click before the capture."""
    at, out = None, {}
    for s in SCREENS:
        for st in s.get('steps') or []:
            if st[0] == 'click':
                at = (st[1], st[2])
        if s.get('steps') and at:
            out[s['id']] = at
        for st in s.get('back') or []:
            if st[0] == 'click':
                at = (st[1], st[2])
    return out


def original_of(s):
    if s.get('capture'):
        return os.path.join('analysis/captures', s['capture'])
    return os.path.join(ROOT, 'original', s['id'] + '.png')


def main():
    publish = 'docs/verification/img' if '--publish' in sys.argv else None
    for d in ('port', 'diff'):
        os.makedirs(os.path.join(ROOT, d), exist_ok=True)
    rows = []
    pointers = pointer_positions()
    for s in SCREENS:
        port, err = render(s)
        orig = original_of(s)
        n = None
        if port and os.path.exists(orig):
            o, p = load(orig) >> 3, load(port) >> 3
            if o.shape == p.shape:
                d = (o != p).any(axis=2)
                # the mouse pointer is in the capture only: its box is left out (where the capture run left it, else where it
                # can be seen)
                arrow = pointers.get(s['id']) or find_arrow(load(orig))
                if arrow:
                    d[max(0, arrow[1] - 1):arrow[1] + 21, max(0, arrow[0] - 1):arrow[0] + 13] = False
                n = int(d.sum())
                # the diff: the port's picture faded, the differing pixels in red
                img = (load(port) * 0.35 + 160).astype(np.uint8)
                img[d] = (255, 0, 0)
                Image.fromarray(img).save(os.path.join(ROOT, 'diff', s['id'] + '.png'))
        rows.append((s, os.path.exists(orig), port, n, err))
        print('%-36s %s' % (s['id'], err or (n if n is not None else 'no capture')), flush=True)
    write_report(rows)
    write_table(rows)
    write_gallery(rows)
    if publish:
        for s, has, port, n, err in rows:
            for kind, src in (('original', original_of(s)), ('port', port), ('diff', os.path.join(ROOT, 'diff', s['id'] + '.png'))):
                if src and os.path.exists(src):
                    os.makedirs(os.path.join(publish, kind), exist_ok=True)
                    shutil.copy(src, os.path.join(publish, kind, s['id'] + '.png'))


def verdict(s, n):
    if n is None:
        return 'no capture yet'
    if not s['exact']:
        return 'not pixel-compared (%d px)' % n
    return '%d px (%.3f %%)' % (n, 100 * n / 480000)


def write_report(rows):
    out = ['<!doctype html><meta charset=utf-8><title>Screens</title><style>body{font:14px sans-serif;background:#222;color:#eee}'
           'img{width:400px;image-rendering:pixelated;margin:2px}h2{margin:24px 0 4px}</style><h1>Elifoot 98: original / port / diff</h1>']
    group = None
    for s, has, port, n, err in rows:
        if s['group'] != group:
            group = s['group']
            out.append('<h2>%s</h2>' % html.escape(group))
        out.append('<h3>%s <small>(%s, %s)</small></h3>' % (html.escape(s['title']), s['form'], html.escape(err or verdict(s, n))))
        for kind in ('original', 'port', 'diff'):
            out.append('<img src="%s/%s.png" alt="%s">' % (kind, s['id'], kind))
    open(os.path.join(ROOT, 'index.html'), 'w').write('\n'.join(out))


def write_gallery(rows):
    out = ['# Gallery', '',
           'Each screen three times: the original under js-dos, the port\'s render of the same state, and the difference (the '
           'port faded, differing pixels in red). The counts are in [Screen by screen](screens.md).', '',
           '!!! note "Where the pictures come from"',
           '    They are drawn with the game\'s own fonts and bitmaps, so they are not stored in the repository: '
           '`tools/site/build.sh` renders and compares them from your own copy and puts them in `docs/verification/img/` '
           'before building the site.', '']
    group = None
    for s, has, port, n, err in rows:
        if s['group'] != group:
            group = s['group']
            out += ['## ' + group, '']
        out += ['### ' + s['title'], '', '%s · `%s`%s' % (verdict(s, n), s['form'], (' · ' + s['note']) if s.get('note') else ''), '',
                '| Original | Port | Difference |', '|---|---|---|',
                '| [![original](img/original/{0}.png)](img/original/{0}.png) | [![port](img/port/{0}.png)](img/port/{0}.png) | [![difference](img/diff/{0}.png)](img/diff/{0}.png) |'.format(s['id']), '']
    open('docs/verification/gallery.md', 'w').write('\n'.join(out) + '\n')


def write_table(rows):
    os.makedirs('docs/verification', exist_ok=True)
    out = ['# Screen by screen', '',
           'Every screen of the original, captured under js-dos, against the port\'s render of the same state '
           '(`tools/screens/compare.py`; the pictures stay local because they are drawn with the game\'s own fonts and bitmaps). '
           'Counts are differing pixels of the 800x600 screen (the mouse pointer of the capture left out); what they are is under *Notes*.', '']
    group = None
    for s, has, port, n, err in rows:
        if s['group'] != group:
            group = s['group']
            out += ['', '## ' + group, '', '| Screen | Form | Differing pixels | Notes |', '|---|---|---|---|']
        out.append('| %s | `%s` | %s | %s |' % (s['title'], s['form'], err or verdict(s, n), s.get('note', '')))
    open('docs/verification/screens.md', 'w').write('\n'.join(out) + '\n')


if __name__ == '__main__':
    main()
