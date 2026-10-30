# Screen by screen

Every screen of the original, captured under js-dos, against the port's render of the same state (`tools/screens/compare.py`; the pictures stay local because they are drawn with the game's own fonts and bitmaps). Counts are differing pixels of the 800x600 screen (the mouse pointer of the capture left out); what they are is under *Notes*.


## Main window

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Main window, page Jogo | `TPREPAREGAMEDLG` | 55 px (0.011 %) |  |
| Main window, page Jogador | `TPREPAREGAMEDLG` | 25 px (0.005 %) |  |
| Main window, page Finanças | `TPREPAREGAMEDLG` | 71 px (0.015 %) |  |
| Main window, page Selecção | `TPREPAREGAMEDLG` | 25 px (0.005 %) |  |
| Main window, page Adversário | `TPREPAREGAMEDLG` | 33 px (0.007 %) |  |

## Menus

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Menu Elifoot | `TPREPAREGAMEDLG` | 50 px (0.010 %) |  |
| Menu Seleccionar | `TPREPAREGAMEDLG` | 70 px (0.015 %) |  |
| Menu Equipa | `TPREPAREGAMEDLG` | 166 px (0.035 %) | the drop-down is 1 px narrower than the original's (the menus whose registered-only entries are hidden; cause not found) |
| Menu Jogador | `TPREPAREGAMEDLG` | 185 px (0.039 %) | the drop-down is 1 px narrower than the original's (the menus whose registered-only entries are hidden; cause not found) |
| Menu Campeonato | `TPREPAREGAMEDLG` | 48 px (0.010 %) |  |
| Menu Treinador | `TPREPAREGAMEDLG` | 236 px (0.049 %) | the drop-down is 1 px narrower than the original's (the menus whose registered-only entries are hidden; cause not found) |

## Menu dialogs

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Elifoot › Opções | `TOPTIONSDLG` | 137 px (0.029 %) |  |
| Elifoot › Acerca | `TABOUTDLG` | not pixel-compared (3031 px) | animated: the notice blinks every 500 ms and the banner scrolls, so the capture's moment decides |
| Equipa › Estádio | `TSTADIUMDLG` | 460 px (0.096 %) | the stadium picture is drawn from measurements (~400 px of its outline) |
| Equipa › Historial | `TPRINTLISTDLG` | 51 px (0.011 %) |  |
| Jogador › Últimas transferências | `TPRINTLISTDLG` | 49 px (0.010 %) |  |
| Campeonato › Classificação | `TCLASSIFICATIONDLG` | 72 px (0.015 %) |  |
| Campeonato › Melhores marcadores | `TPRINTLISTDLG` | 22 px (0.005 %) |  |
| Campeonato › Calendário | `TPRINTLISTDLG` | 22 px (0.005 %) |  |
| Campeonato › Últimos vencedores | `TPRINTLISTDLG` | 49 px (0.010 %) |  |
| Campeonato › Melhores marcadores de sempre | `TPRINTLISTDLG` | 22 px (0.005 %) |  |
| Treinador › História | `TPRINTLISTDLG` | 51 px (0.011 %) |  |
| Treinador › Ranking | `TPRINTLISTDLG` | 50 px (0.010 %) |  |
| Treinador › Perfil | `TCOACHOPTIONSDLG` | 226 px (0.047 %) |  |

## Start and new game

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| About box at start-up | `TABOUTDLG` | not pixel-compared (1967 px) | animated: the notice blinks and the banner scrolls |
| Choice of game | `TSELECTGAMEDLG` | 45 px (0.009 %) |  |
| New game named | `TSELECTGAMEDLG` | 276 px (0.058 %) | the edit's caret blinks |
| Countries | `TSELECTCOUNTRIESDLG` | 34 px (0.007 %) |  |
| Countries, Brasil chosen | `TSELECTCOUNTRIESDLG` | 111 px (0.023 %) |  |
| Currency | `TCURRENCYDLG` | 57 px (0.012 %) |  |
| Coaches' names | `TGETTEAMSDLG` | 4 px (0.001 %) |  |
| Clubs drawn | `TGETTEAMSDLG` | not pixel-compared (3713 px) | the club is drawn at random |
| A new game's main window | `TPLANTELDLG` | not pixel-compared (2012 px) | the club is drawn at random |

## Team editor

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Editor without a team | `TEDITOR_MAIN` | 0 px (0.000 %) |  |
| Editor with a team (the flag's hint) | `TEDITOR_MAIN` | 164 px (0.034 %) |  |
| Editor, a player selected | `TEDITOR_MAIN` | 223 px (0.046 %) |  |
| Editor menu Ficheiro | `TEDITOR_MAIN` | 151 px (0.031 %) |  |
| Editor menu Jogador | `TEDITOR_MAIN` | 436 px (0.091 %) |  |
| Editor menu Equipa | `TEDITOR_MAIN` | 493 px (0.103 %) |  |
| Editor menu Elifoot | `TEDITOR_MAIN` | 167 px (0.035 %) |  |
| Abrir | `TOPENTEAMDLG` | 35 px (0.007 %) |  |
| Equipa › Nome | `TSETTEAMNAMEDLG` | 120 px (0.025 %) |  |
| Equipa › Cores | `TTEAMCOLORDLG` | 347 px (0.072 %) |  |
| Equipa › País | `TSETCOUNTRYDLG` | 185 px (0.039 %) |  |
| Equipa › Nível inicial | `TTEAMSKILLDLG` | 115 px (0.024 %) |  |
| Equipa › Treinador | `TCOACHNAMEDLG` | 94 px (0.020 %) |  |
| Jogador › Editar | `TEDITPLAYERDLG` | 203 px (0.042 %) |  |
| Jogador › Transferir | `TTRANSFERPLAYERDLG` | 182 px (0.038 %) |  |
| Elifoot › Acerca | `TABOUTDLG` | not pixel-compared (1878 px) | animated: blinking notice, scrolling banner |

## Match day

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Half-time | `TSUBSTITUTIONDLG` | not pixel-compared (113854 px) | the match differs (the original's random numbers): layout only |
| Standings after a round | `TCLASSIFICATIONDLG` | not pixel-compared (123944 px) | the match differs (the original's random numbers): layout only |
| Cup draw | `TSORTEIODLG` | not pixel-compared (16506 px) | the match differs (the original's random numbers): layout only |
| Penalty shoot-out | `TPENALTIESDLG` | not pixel-compared (122602 px) | the match differs (the original's random numbers): layout only |
| A human's penalty | `TPENALTYDLG` | not pixel-compared (190980 px) | the match differs (the original's random numbers): layout only |
| Coaches sacked | `TPRINTLISTDLG` | not pixel-compared (1543 px) | the match differs (the original's random numbers): layout only |
| End of season | `TENDOFSEASONDLG` | not pixel-compared (19396 px) | the match differs (the original's random numbers): layout only |
| Auction | `TSELLPLAYERDLG` | not pixel-compared (2811 px) | the match differs (the original's random numbers): layout only |
| Invitation to coach another club | `TINVITECOACHDLG` | 8 px (0.002 %) |  |

## Menu dialogs

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Balance sheet (a click on the Finanças list) | `TBALANCEDLG` | 68 px (0.014 %) |  |

## Start and new game

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Hint of the file name | `TSELECTGAMEDLG` | 72 px (0.015 %) | the capture's list shows ABCDEEEE |

## Menu dialogs

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Elifoot › Opções, page Jogo | `TOPTIONSDLG` | 76 px (0.016 %) |  |

## Main window

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| A player selected, page Jogo | `TPREPAREGAMEDLG` | 62 px (0.013 %) |  |
| A player selected, page Jogador | `TPREPAREGAMEDLG` | 62 px (0.013 %) |  |
| Renovar contrato (the player's page) | `TPREPAREGAMEDLG` | 239 px (0.050 %) |  |
| Jogador › Vender | `TPREPAREGAMEDLG` | 42 px (0.009 %) |  |

## Match day

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Round view during the games | `TJOURNEYDLG` | not pixel-compared (3890 px) | the match differs (the original's random numbers): layout only |

## Team editor

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| New team: name (the hint of the full name) | `TSETTEAMNAMEDLG` | 56 px (0.012 %) |  |
| New team: colours | `TTEAMCOLORDLG` | 94 px (0.020 %) |  |
| New team: country | `TSETCOUNTRYDLG` | 316 px (0.066 %) |  |
| New team: starting level | `TTEAMSKILLDLG` | 50 px (0.010 %) |  |
| New team: coach | `TCOACHNAMEDLG` | 57 px (0.012 %) |  |
| New team: the first player | `TEDITPLAYERDLG` | 175 px (0.036 %) |  |
| New team made (the flag's hint) | `TEDITOR_MAIN` | 248 px (0.052 %) |  |
| Gravar equipa | `TSAVETEAMDLG` | 290 px (0.060 %) |  |
| The team cannot be saved | `TCANTSAVETEAMDLG` | 231 px (0.048 %) |  |

## Main window

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Renovar contrato, a low offer typed | `TPREPAREGAMEDLG` | 247 px (0.051 %) |  |
| The player refuses the offer (Actualização de Salário) | `TASKMORESALARYDLG` | 183 px (0.038 %) |  |

## Match day

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| An injured player must be replaced | `TFORCEDSUBSTITUTIONDLG` | not pixel-compared (131006 px) | the match differs (the original's random numbers): layout only |

## Coverage

78 screens, 63 of them compared pixel for pixel (median 72 differing pixels); 34 of the 49 forms of `ELIFOOT.EXE` and `EDITEQ.EXE` appear in them. The others:

| Form | Why it has no screen |
|---|---|
| `TADDINSDLG` | "Extras": not shown by the unregistered game in any recorded session; the port does not use it |
| `TASSIGNNACDLG` | "Abreviatura Inexistente": asks for a country when a team's code is unknown; no team of the packs has one |
| `TCHECKPLAYERDLG` | registered copy only (Jogador › Procurar jogadores); hidden in the unregistered game and the port |
| `TCLOCKDLG` | "Relógio" (a gauge): not shown in any recorded session; the port does not use it |
| `TCOACHMANAGERDLG` | registered copy only (Treinador › Gestão de treinadores) |
| `TCODEDLG` | the registration code dialog: no key check is implemented, by decision |
| `TFORM1` | an empty form left in the program; never shown |
| `TINPUTDLG` | not shown in any recorded session; the port does not use it |
| `TJANELACOR` | not shown in any recorded session (the editor's colours use TTeamColorDlg) |
| `TLOANDLG` | offered only to a club in debt or with a loan (Equipa › Pedir / pagar empréstimo); not reached in the recordings |
| `TMAINWINDOW` | an empty form captioned "Elifoot 97", left in the program; never shown |
| `TOBSERVEDPLAYERSDLG` | registered copy only (Jogadores sob observação) |
| `TPROGRESSDLG` | the start-up gauge (captured as start_7_next, animated): not compared |
| `TREFEREEAIDDLG` | registered copy only: the referee's name opens it only with features 0x23 / 0x0a (1008:2427, 1018:3416); checked: a click at half-time does nothing |
| `TWAITDLG` | not shown in any recorded session; the port does not use it |
