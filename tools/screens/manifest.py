"""The screen catalogue: every screen of Elifoot 98 that the port reproduces, how to reach it in the original (under js-dos) and how
the port renders the same screen.

Each screen is a dict:
  id        file name of the captures (analysis/screens/{original,port,diff}/<id>.png)
  group     section of the catalogue
  title     what the screen is
  form      the original's Delphi form (analysis/out/forms/<FORM>.json)
  steps     inputs that reach it in the original from the loaded game `ABCDEEEE` (the main window of joga1, FIGUEIRENSE),
            undone by `back` afterwards; None when the screen comes from a recorded session (`capture`)
  capture   a frame recorded earlier (relative to analysis/captures/) instead of `steps`
  port      arguments of `classic_shot` (or `editor_shot` when `tool` is "editor") that render the same screen in the port
  exact     True when both sides show the same state, so the pixel count is meaningful; False for a layout comparison only (the
            original's match days depend on random numbers seeded at start-up)

Steps are tuples: ("key", name) a key press, ("click", x, y) a click at a guest pixel, ("wait", ms).
"""

SAVE = "original/saves/snap2.e98"  # the port's copy of the game the js-dos image loads as ABCDEEEE
BASE = [SAVE, "human=FIGUEIRENSE"]


def menu_keys(top, nth):
    """Opens the `top`-th menu of the main window with the keyboard and highlights its `nth` selectable entry."""
    return [("key", "Alt")] + [("key", "ArrowRight")] * top + [("key", "ArrowDown")] + [("key", "ArrowDown")] * nth


# (top, title, [(entry index in the menu model, control name, label, dialog it opens or None)]): from `classic_shot dump=menus`
MENUS = [
    (0, "Elifoot", [(0, "opcoesElifoot", "Opções", "TOPTIONSDLG"), (2, "Acerca1", "Acerca", "TABOUTDLG")]),
    (1, "Seleccionar", []),
    (2, "Equipa", [(0, "Estdio1", "Estádio", "TSTADIUMDLG"), (1, "Historial1", "Historial", "TPRINTLISTDLG")]),
    (3, "Jogador", [(2, "ltimastransferncias1", "Últimas transferências", "TPRINTLISTDLG")]),
    (4, "Campeonato", [
        (0, "CampeonatoClassificao", "Classificação", "TCLASSIFICATIONDLG"),
        (1, "campeonatoMelhoresmarcadores", "Melhores marcadores", "TPRINTLISTDLG"),
        (2, "menuCalendrio", "Calendário", "TPRINTLISTDLG"),
        (4, "ltimosvencedores1", "Últimos vencedores", "TPRINTLISTDLG"),
        (5, "Melhoresmarcadoresdesempre1", "Melhores marcadores de sempre", "TPRINTLISTDLG"),
    ]),
    (5, "Treinador", [
        (0, "treinadorHistoria", "História", "TPRINTLISTDLG"),
        (1, "coachRanking", "Ranking", "TPRINTLISTDLG"),
        (3, "Opesindividuais1", "Perfil", "TCOACHOPTIONSDLG"),
    ]),
]

# the selectable entries before each one (separators are skipped by the arrow keys)
SEPARATORS = {1: [12], 3: [1], 4: [3], 5: [2]}


def nth(top, index):
    return index - sum(1 for s in SEPARATORS.get(top, []) if s < index)


# how a dialog is closed without changing anything (Escape, unless the dialog ignores it; never a default button that acts)
CLOSE = {
    "Estdio1": [("click", 588, 440), ("wait", 1000)],  # Fechar (Enter would press Construir and build stands)
    "Acerca1": [("key", "Enter"), ("wait", 1000)],
    "CampeonatoClassificao": [("click", 250, 480), ("wait", 1000)],  # a click anywhere on the standings (GoOn)
}

PAGES = [(0, "Jogo", 470), (1, "Jogador", 512), (2, "Finanças", 561), (3, "Selecção", 614), (4, "Adversário", 684)]

SCREENS = []


def slug(s):
    return s.lower().translate(str.maketrans("áãçéíóú ", "aaceiou_"))


def add(**s):
    s.setdefault("exact", True)
    s.setdefault("tool", "classic")
    s.setdefault("back", [("key", "Escape")])
    SCREENS.append(s)


# the main window, page by page (the tabs at the bottom right)
for i, name, x in PAGES:
    add(id="main_%d_%s" % (i, slug(name)), group="Main window", title="Main window, page %s" % name,
        form="TPLANTELDLG", steps=[("click", x, 511), ("wait", 1500)], back=[("click", 470, 511), ("wait", 1000)],
        port=BASE + (["page=%d" % i] if i else []))

MENU_NOTES = {n: "the drop-down is 1 px narrower than the original's (the menus whose registered-only entries are hidden; cause not "
               "found)" for n in (2, 3, 5)}

# the menus, open
for top, title, entries in MENUS:
    add(id="menu_%d_%s" % (top, title.lower()), group="Menus", title="Menu %s" % title, form="TPLANTELDLG",
        note=MENU_NOTES.get(top, ""),
        steps=menu_keys(top, 0), back=[("key", "Escape"), ("key", "Escape")],
        port=BASE + ["menu=%d" % top, "hl=0"])

# what the counts of a screen leave out
NOTES = {
    "Acerca1": "animated: the notice blinks every 500 ms and the banner scrolls, so the capture's moment decides",
    "Estdio1": "the stadium picture is drawn from measurements (~400 px of its outline)",
}
EXACT = {"Acerca1": False}

# the dialogs the menus open
for top, title, entries in MENUS:
    for index, control, label, form in entries:
        add(note=NOTES.get(control, ""), exact=EXACT.get(control, True),id="dlg_%s" % control.lower(), group="Menu dialogs", title="%s › %s" % (title, label), form=form,
            steps=menu_keys(top, nth(top, index)) + [("key", "Enter"), ("wait", 2500)],
            back=CLOSE.get(control, [("key", "Enter" if form in ("TPRINTLISTDLG", "TCLASSIFICATIONDLG") else "Escape"), ("wait", 800)]),
            port=BASE + ["dialog=%s" % control])

# the start of the program and a new game, captured once from a fresh boot (About, then: the new-game radio, the name NOVO, Brasil,
# the currency, the gauge, the coach "joga", the draw). The port gets the emulator's own team files and saved games.
START_TEAMS = "teams=analysis/screens/jsdos/d/eli98/equipas"  # unzip d/eli98/equipas/* of analysis/jsdos/game.jsdos there
NEW = "key:Enter;click:196,183;key:n;key:o;key:v;key:o"
for sid, title, form, ev, exact, note in [
    ("start_0_about", "About box at start-up", "TABOUTDLG", "", False,
     "animated: the notice blinks and the banner scrolls"),
    ("start_1_select", "Choice of game", "TSELECTGAMEDLG", "key:Enter", True, ""),
    ("start_3_named", "New game named", "TSELECTGAMEDLG", NEW, True, "the edit's caret blinks"),
    ("start_4_next", "Countries", "TSELECTCOUNTRIESDLG", NEW + ";key:Enter", True, ""),
    ("start_5_brasil", "Countries, Brasil chosen", "TSELECTCOUNTRIESDLG", NEW + ";key:Enter;click:165,269", True, ""),
    ("start_6_next", "Currency", "TCURRENCYDLG", NEW + ";key:Enter;click:165,269;click:600,130", True, ""),
    ("start_8_next", "Coaches' names", "TGETTEAMSDLG",
     NEW + ";key:Enter;click:165,269;click:600,130;key:Enter;tick:20000", True, ""),
    ("start_12_drawn", "Clubs drawn", "TGETTEAMSDLG",
     NEW + ";key:Enter;click:165,269;click:600,130;key:Enter;tick:20000;key:j;key:o;key:g;key:a;click:400,418", False,
     "the club is drawn at random"),
    ("start_13_main", "A new game's main window", "TPLANTELDLG",
     NEW + ";key:Enter;click:165,269;click:600,130;key:Enter;tick:20000;key:j;key:o;key:g;key:a;click:400,418;key:Enter", False,
     "the club is drawn at random"),
]:
    add(id=sid, group="Start and new game", title=title, form=form, steps=None, exact=exact, note=note,
        port=["-", START_TEAMS, "saves=ABCDEEEE"] + (["ev=" + ev] if ev else []))
