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

# the team editor (EDITEQ.EXE), captured over Program Manager in an earlier session (analysis/captures/editor/): compared inside the
# editor's window only (`region`), with the emulator's team files and ARSENAL.EFT open
ED = ["dir=analysis/screens/jsdos/d/eli98/equipas"]
ED_NOTES = {
    "ed_about": "animated: blinking notice, scrolling banner",
}
ARS = ED + ["team=ARSENAL.EFT"]
for sid, cap, title, form, port in [
    ("ed_empty", "empty", "Editor without a team", "TEDITOR_MAIN", ED),
    ("ed_team", "team", "Editor with a team (the flag's hint)", "TEDITOR_MAIN", ARS + ["do=%545,228"]),
    ("ed_team_selected", "team_selected", "Editor, a player selected", "TEDITOR_MAIN", ARS + ["sel=2"]),
    ("ed_menu_ficheiro", "menu_ficheiro", "Editor menu Ficheiro", "TEDITOR_MAIN", ARS + ["menu=0", "hl=0"]),
    ("ed_menu_jogador", "menu_jogador", "Editor menu Jogador", "TEDITOR_MAIN", ARS + ["menu=1", "hl=0"]),
    ("ed_menu_equipa", "menu_equipa", "Editor menu Equipa", "TEDITOR_MAIN", ARS + ["sel=2", "menu=2", "hl=0"]),
    ("ed_menu_elifoot", "menu_elifoot", "Editor menu Elifoot", "TEDITOR_MAIN", ARS + ["menu=3", "hl=0"]),
    ("ed_open", "open", "Abrir", "TOPENTEAMDLG", ED + ["do=Abrir1"]),
    ("ed_name", "dlg_nome", "Equipa › Nome", "TSETTEAMNAMEDLG", ARS + ["sel=2", "do=Nome1"]),
    ("ed_colors", "dlg_cores", "Equipa › Cores", "TTEAMCOLORDLG", ARS + ["sel=2", "do=TeamColor"]),
    ("ed_country", "dlg_pais", "Equipa › País", "TSETCOUNTRYDLG", ARS + ["sel=2", "do=EquipaPais"]),
    ("ed_level", "dlg_nivel", "Equipa › Nível inicial", "TTEAMSKILLDLG", ARS + ["do=EquipaNivelInicial"]),
    ("ed_coach", "dlg_treinador", "Equipa › Treinador", "TCOACHNAMEDLG", ARS + ["do=EquipaTreinador"]),
    ("ed_player", "edit_player", "Jogador › Editar", "TEDITPLAYERDLG", ARS + ["sel=2", "do=JogadorEditar"]),
    ("ed_transfer", "dlg_transfer", "Jogador › Transferir", "TTRANSFERPLAYERDLG", ARS + ["sel=2", "do=JogadorTransferir"]),
    ("ed_about", "about", "Elifoot › Acerca", "TABOUTDLG", ARS + ["do=Acerca1"]),
]:
    add(id=sid, group="Team editor", title=title, form=form, steps=None, capture="editor/%s.png" % cap, tool="editor",
        region="window", port=port, exact=sid != "ed_about",
        note=ED_NOTES.get(sid, ""))

# match days: the original's come from recorded seasons (tools/capture/season.sh, analysis/captures/probe*), the port plays from the
# same save until the dialog opens (`ev=season:N:Kind`). The original seeds its random numbers at start-up, so the games differ:
# these compare the layout, except the invitation, whose texts are the same.
MD = "the match differs (the original's random numbers): layout only"
for sid, cap, title, form, port, exact, note in [
    ("md_halftime", "probe2/0032_halftime.png", "Half-time", "TSUBSTITUTIONDLG", BASE + ["ev=season:1:Halftime"], False, MD),
    ("md_standings", "probe3/0012_dialog_over_roundview.png", "Standings after a round", "TCLASSIFICATIONDLG",
     BASE + ["ev=season:1:Standings"], False, MD),
    ("md_cupdraw", "probe3/0015_dialog_over_roundview.png", "Cup draw", "TSORTEIODLG", BASE + ["ev=season:2:CupDraw"], False, MD),
    ("md_shootout", "probe3/0092_dialog_over_roundview.png", "Penalty shoot-out", "TPENALTIESDLG",
     BASE + ["ev=season:40:Shootout"], False, MD),
    ("md_penalty", "probe3/0560_dialog_over_roundview.png", "A human's penalty", "TPENALTYDLG", BASE + ["ev=season:40:Penalty"],
     False, MD),
    ("md_lashes", "probe3/0570_dialog_over_roundview.png", "Coaches sacked", "TPRINTLISTDLG", BASE + ["ev=season:40:Lashes"],
     False, MD),
    ("md_endofseason", "probe3/1019_dialog_over_roundview.png", "End of season", "TENDOFSEASONDLG",
     BASE + ["ev=season:40:EndOfSeason"], False, MD),
    ("md_auction", "probe2/0001_auction.png", "Auction", "TSELLPLAYERDLG", BASE + ["ev=season:1:Auction"], False, MD),
    ("md_invitation", "probe3/0225_dialog_over_roundview.png", "Invitation to coach another club", "TINVITECOACHDLG",
     ["-", "coach=joga2", "mdopen=invite:GREMIO"], True, ""),
]:
    add(id=sid, group="Match day", title=title, form=form, steps=None, capture=cap, port=port, exact=exact, note=note)

# single captures made while working on a screen (analysis/captures/)
add(id="dlg_balance", group="Menu dialogs", title="Balance sheet (a click on the Finanças list)", form="TBALANCEDLG", steps=None,
    capture="bal_dlg.png", port=BASE + ["page=2", "ev=click:560,300"])
add(id="hint_gamename", group="Start and new game", title="Hint of the file name", form="TSELECTGAMEDLG", steps=None,
    capture="hint4.png", port=["-", START_TEAMS, "saves=ABCDEEEE", "ev=key:Enter;click:196,183;move:440,221;tick:1000"],
    note="the capture's list shows ABCDEEEE")

# more screens of the loaded game, captured after the menus' (the last ones change the window for good: `final`)
add(id="dlg_opcoes_jogo", group="Menu dialogs", title="Elifoot › Opções, page Jogo", form="TOPTIONSDLG",
    steps=menu_keys(0, 0) + [("key", "Enter"), ("wait", 2500), ("click", 337, 183), ("wait", 1500)],
    back=[("key", "Escape"), ("wait", 800)], port=BASE + ["dialog=opcoesElifoot", "ev=click:337,183"])
add(id="main_sel_jogo", group="Main window", title="A player selected, page Jogo", form="TPLANTELDLG",
    steps=[("click", 200, 218), ("wait", 1500)], back=[], final=True, port=BASE + ["ev=click:200,218"])
add(id="main_sel_jogador", group="Main window", title="A player selected, page Jogador", form="TPLANTELDLG",
    steps=[("click", 512, 511), ("wait", 1500)], back=[], final=True, port=BASE + ["select=2", "page=1"])
add(id="main_contract", group="Main window", title="Renovar contrato (the player's page)", form="TPLANTELDLG", steps=None,
    port=BASE + ["ev=click:200,218;click:570,479"])
add(id="main_sell", group="Main window", title="Jogador › Vender", form="TPLANTELDLG", steps=None,
    port=BASE + ["ev=click:200,218", "page=sell"])
add(id="md_roundview", group="Match day", title="Round view during the games", form="TJOURNEYDLG", steps=None,
    capture="probe1/0030_roundview.png", port=BASE + ["ev=turn:;tick:20000"], exact=False, note=MD)

# the team editor's new team, step by step (typed TESTE FC / TESTE, Áustria, coach "treinador x"), and saving a team without
# goalkeepers (the two removed, then Steve Morrow)
NEW_TEAM = "Novaequipa;#T;#E;#S;#T;#E;# ;#F;#C;#Tab;#T;#E;#S;#T;#E"
AFTER_LEVEL = NEW_TEAM + ";!OKBtn;!OKBtn;#Down;#Down;!OKBtn;!OKBtn"
NO_KEEPERS = "JogadorRemover;JogadorRemover;@250,293;JogadorRemover;FicheiroSair"
for sid, cap, title, form, port in [
    ("ed_new_name", "new_name", "New team: name (the hint of the full name)", "TSETTEAMNAMEDLG", ED + ["do=" + NEW_TEAM + ";%272,277"]),
    ("ed_new_color", "new_color", "New team: colours", "TTEAMCOLORDLG", ED + ["do=" + NEW_TEAM + ";!OKBtn"]),
    ("ed_new_country", "new_country", "New team: country", "TSETCOUNTRYDLG", ED + ["do=" + NEW_TEAM + ";!OKBtn;!OKBtn"]),
    ("ed_new_level", "new_level", "New team: starting level", "TTEAMSKILLDLG",
     ED + ["do=" + NEW_TEAM + ";!OKBtn;!OKBtn;#Down;#Down;!OKBtn"]),
    ("ed_new_coach", "new_coach", "New team: coach", "TCOACHNAMEDLG", ED + ["do=" + AFTER_LEVEL]),
    ("ed_new_player", "new_player", "New team: the first player", "TEDITPLAYERDLG",
     ED + ["do=" + AFTER_LEVEL + ";#t;#r;#e;#i;#n;#a;#d;#o;#r;# ;#x;!OKBtn"]),
    ("ed_new_team", "new_team", "New team made (the flag's hint)", "TEDITOR_MAIN",
     ED + ["do=" + AFTER_LEVEL + ";#t;#r;#e;#i;#n;#a;#d;#o;#r;# ;#x;!OKBtn;!CancelBtn;%545,245"]),
    ("ed_save", "dlg_save", "Gravar equipa", "TSAVETEAMDLG", ARS + ["sel=0", "do=" + NO_KEEPERS]),
    ("ed_cantsave", "dlg_cantsave", "The team cannot be saved", "TCANTSAVETEAMDLG", ARS + ["sel=0", "do=" + NO_KEEPERS + ";!OKBtn"]),
]:
    add(id=sid, group="Team editor", title=title, form=form, steps=None, capture="editor/%s.png" % cap, tool="editor",
        region="window", port=port)
