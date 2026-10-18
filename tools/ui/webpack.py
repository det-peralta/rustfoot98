#!/usr/bin/env python3
"""Copies what the browser build of the classic interface needs into web/skin/ (git-ignored): the skin pack built by tools/ui/skin.py,
the game's country and referee lists and the team files of the user's own install. Usage: python tools/ui/webpack.py"""
import json, os, shutil

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
SKIN = os.path.join(ROOT, 'analysis', 'out', 'skin')
GAME = os.path.join(ROOT, 'original', 'elifoot98')
OUT = os.path.join(ROOT, 'web', 'skin')

os.makedirs(OUT, exist_ok=True)
for f in ('fonts.bin', 'glyphs.bin', 'forms.json'):
    shutil.copy(os.path.join(SKIN, f), os.path.join(OUT, f))
for f in ('COUNTRY.TXE', 'REFEREE.TXE'):
    shutil.copy(os.path.join(GAME, f), os.path.join(OUT, f))
teams = []
tdir = os.path.join(GAME, 'EQUIPAS')
for f in sorted(os.listdir(tdir)):
    if f.upper().endswith('.EFT'):
        shutil.copy(os.path.join(tdir, f), os.path.join(OUT, f))
        teams.append(f)
json.dump({'fonts': 'fonts.bin', 'glyphs': 'glyphs.bin', 'forms': 'forms.json', 'country': 'COUNTRY.TXE',
           'referee': 'REFEREE.TXE', 'teams': teams}, open(os.path.join(OUT, 'index.json'), 'w'))
print(f'web/skin: {len(teams)} teams')
