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

## Start and new game

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| About box at start-up | `TABOUTDLG` | not pixel-compared (1971 px) | animated: the notice blinks and the banner scrolls |
| Choice of game | `TSELECTGAMEDLG` | 66 px (0.014 %) |  |
| New game named | `TSELECTGAMEDLG` | 364 px (0.076 %) | the edit's caret blinks |
| Countries | `TSELECTCOUNTRIESDLG` | 85 px (0.018 %) |  |
| Countries, Brasil chosen | `TSELECTCOUNTRIESDLG` | 153 px (0.032 %) |  |
| Currency | `TCURRENCYDLG` | 182 px (0.038 %) |  |
| Coaches' names | `TGETTEAMSDLG` | 4 px (0.001 %) |  |
| Clubs drawn | `TGETTEAMSDLG` | not pixel-compared (3713 px) | the club is drawn at random |
| A new game's main window | `TPLANTELDLG` | not pixel-compared (1959 px) | the club is drawn at random |

## Team editor

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Editor without a team | `TEDITOR_MAIN` | 688 px (0.143 %) |  |
| Editor with a team | `TEDITOR_MAIN` | 915 px (0.191 %) | the capture shows the flag's hint (the country, set by the program) under the pointer |
| Editor, a player selected | `TEDITOR_MAIN` | 224 px (0.047 %) |  |
| Editor menu Ficheiro | `TEDITOR_MAIN` | 151 px (0.031 %) |  |
| Editor menu Jogador | `TEDITOR_MAIN` | 437 px (0.091 %) |  |
| Editor menu Equipa | `TEDITOR_MAIN` | 494 px (0.103 %) |  |
| Editor menu Elifoot | `TEDITOR_MAIN` | 168 px (0.035 %) |  |
| Abrir | `TOPENTEAMDLG` | 34 px (0.007 %) |  |
| Equipa › Nome | `TSETTEAMNAMEDLG` | 121 px (0.025 %) |  |
| Equipa › Cores | `TTEAMCOLORDLG` | 348 px (0.072 %) |  |
| Equipa › País | `TSETCOUNTRYDLG` | 186 px (0.039 %) |  |
| Equipa › Nível inicial | `TTEAMSKILLDLG` | 118 px (0.025 %) |  |
| Equipa › Treinador | `TCOACHNAMEDLG` | 95 px (0.020 %) |  |
| Jogador › Editar | `TEDITPLAYERDLG` | 205 px (0.043 %) |  |
| Jogador › Transferir | `TTRANSFERPLAYERDLG` | 189 px (0.039 %) |  |
| Elifoot › Acerca | `TABOUTDLG` | not pixel-compared (1882 px) | animated: blinking notice, scrolling banner |

## Match day

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Half-time | `TSUBSTITUTIONDLG` | not pixel-compared (114357 px) | the match differs (the original's random numbers): layout only |
| Standings after a round | `TCLASSIFICATIONDLG` | not pixel-compared (112815 px) | the match differs (the original's random numbers): layout only |
| Cup draw | `TSORTEIODLG` | not pixel-compared (16504 px) | the match differs (the original's random numbers): layout only |
| Penalty shoot-out | `TPENALTIESDLG` | not pixel-compared (122634 px) | the match differs (the original's random numbers): layout only |
| A human's penalty | `TPENALTYDLG` | not pixel-compared (195729 px) | the match differs (the original's random numbers): layout only |
| Coaches sacked | `TPRINTLISTDLG` | not pixel-compared (1476 px) | the match differs (the original's random numbers): layout only |
| End of season | `TENDOFSEASONDLG` | not pixel-compared (10517 px) | the match differs (the original's random numbers): layout only |
| Auction | `TSELLPLAYERDLG` | not pixel-compared (2789 px) | the match differs (the original's random numbers): layout only |
| Invitation to coach another club | `TINVITECOACHDLG` | 9 px (0.002 %) |  |

## Menu dialogs

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Balance sheet (a click on the Finanças list) | `TBALANCEDLG` | 74 px (0.015 %) |  |

## Start and new game

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Hint of the file name | `TSELECTGAMEDLG` | 93 px (0.019 %) | the capture's list shows ABCDEEEE |

## Menu dialogs

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Elifoot › Opções, page Jogo | `TOPTIONSDLG` | 78 px (0.016 %) |  |

## Main window

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| A player selected, page Jogo | `TPLANTELDLG` | 64 px (0.013 %) |  |
| A player selected, page Jogador | `TPLANTELDLG` | 64 px (0.013 %) |  |
| Renovar contrato (the player's page) | `TPLANTELDLG` | 342 px (0.071 %) |  |
| Jogador › Vender | `TPLANTELDLG` | 42 px (0.009 %) |  |
