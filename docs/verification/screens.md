# Screen by screen

Every screen of the original, captured under js-dos, against the port's render of the same state (`tools/screens/compare.py`; the pictures stay local because they are drawn with the game's own fonts and bitmaps). Counts are differing pixels of the 800x600 screen (the mouse pointer of the capture left out); what they are is under *Notes*.


## Main window

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Main window, page Jogo | `TPLANTELDLG` | 55 px (0.011 %) |  |
| Main window, page Jogador | `TPLANTELDLG` | 25 px (0.005 %) |  |
| Main window, page Finanças | `TPLANTELDLG` | 71 px (0.015 %) |  |
| Main window, page Selecção | `TPLANTELDLG` | 25 px (0.005 %) |  |
| Main window, page Adversário | `TPLANTELDLG` | 33 px (0.007 %) |  |

## Menus

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Menu Elifoot | `TPLANTELDLG` | 50 px (0.010 %) |  |
| Menu Seleccionar | `TPLANTELDLG` | 70 px (0.015 %) |  |
| Menu Equipa | `TPLANTELDLG` | 166 px (0.035 %) | the drop-down is 1 px narrower than the original's (the menus whose registered-only entries are hidden; cause not found) |
| Menu Jogador | `TPLANTELDLG` | 185 px (0.039 %) | the drop-down is 1 px narrower than the original's (the menus whose registered-only entries are hidden; cause not found) |
| Menu Campeonato | `TPLANTELDLG` | 48 px (0.010 %) |  |
| Menu Treinador | `TPLANTELDLG` | 236 px (0.049 %) | the drop-down is 1 px narrower than the original's (the menus whose registered-only entries are hidden; cause not found) |

## Menu dialogs

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Elifoot › Opções | `TOPTIONSDLG` | 140 px (0.029 %) |  |
| Elifoot › Acerca | `TABOUTDLG` | not pixel-compared (3035 px) | animated: the notice blinks every 500 ms and the banner scrolls, so the capture's moment decides |
| Equipa › Estádio | `TSTADIUMDLG` | 474 px (0.099 %) | the stadium picture is drawn from measurements (~400 px of its outline) |
| Equipa › Historial | `TPRINTLISTDLG` | 51 px (0.011 %) |  |
| Jogador › Últimas transferências | `TPRINTLISTDLG` | 49 px (0.010 %) |  |
| Campeonato › Classificação | `TCLASSIFICATIONDLG` | 82 px (0.017 %) |  |
| Campeonato › Melhores marcadores | `TPRINTLISTDLG` | 286 px (0.060 %) |  |
| Campeonato › Calendário | `TPRINTLISTDLG` | 22 px (0.005 %) |  |
| Campeonato › Últimos vencedores | `TPRINTLISTDLG` | 49 px (0.010 %) |  |
| Campeonato › Melhores marcadores de sempre | `TPRINTLISTDLG` | 286 px (0.060 %) |  |
| Treinador › História | `TPRINTLISTDLG` | 51 px (0.011 %) |  |
| Treinador › Ranking | `TPRINTLISTDLG` | 222 px (0.046 %) |  |
| Treinador › Perfil | `TCOACHOPTIONSDLG` | 227 px (0.047 %) |  |
