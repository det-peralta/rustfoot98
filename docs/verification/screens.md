# Screen by screen

Every screen of the original, captured under js-dos, against the port's render of the same state (`tools/screens/compare.py`; the pictures stay local because they are drawn with the game's own fonts and bitmaps). Counts are differing pixels of the 800x600 screen (the mouse pointer of the capture left out); what they are is under *Notes*.


## Main window

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Main window, page Jogo | `TPREPAREGAMEDLG` | 12 px (0.003 %) |  |
| Main window, page Jogador | `TPREPAREGAMEDLG` | 10 px (0.002 %) |  |
| Main window, page Finanças | `TPREPAREGAMEDLG` | 18 px (0.004 %) |  |
| Main window, page Selecção | `TPREPAREGAMEDLG` | 10 px (0.002 %) |  |
| Main window, page Adversário | `TPREPAREGAMEDLG` | 12 px (0.003 %) |  |

## Menus

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Menu Elifoot | `TPREPAREGAMEDLG` | 12 px (0.003 %) |  |
| Menu Seleccionar | `TPREPAREGAMEDLG` | 31 px (0.006 %) |  |
| Menu Equipa | `TPREPAREGAMEDLG` | 123 px (0.026 %) | the drop-down is 1 px narrower than the original's (the menus whose registered-only entries are hidden; cause not found) |
| Menu Jogador | `TPREPAREGAMEDLG` | 143 px (0.030 %) | the drop-down is 1 px narrower than the original's (the menus whose registered-only entries are hidden; cause not found) |
| Menu Campeonato | `TPREPAREGAMEDLG` | 12 px (0.003 %) |  |
| Menu Treinador | `TPREPAREGAMEDLG` | 197 px (0.041 %) | the drop-down is 1 px narrower than the original's (the menus whose registered-only entries are hidden; cause not found) |

## Menu dialogs

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Elifoot › Opções | `TOPTIONSDLG` | 82 px (0.017 %) |  |
| Elifoot › Acerca | `TABOUTDLG` | not pixel-compared (2971 px) | animated: the notice blinks every 500 ms and the banner scrolls, so the capture's moment decides |
| Equipa › Estádio | `TSTADIUMDLG` | 459 px (0.096 %) | the stadium picture is drawn from measurements (~400 px of its outline) |
| Equipa › Historial | `TPRINTLISTDLG` | 12 px (0.003 %) |  |
| Jogador › Últimas transferências | `TPRINTLISTDLG` | 12 px (0.003 %) |  |
| Campeonato › Classificação | `TCLASSIFICATIONDLG` | 10 px (0.002 %) |  |
| Campeonato › Melhores marcadores | `TPRINTLISTDLG` | 12 px (0.003 %) |  |
| Campeonato › Calendário | `TPRINTLISTDLG` | 12 px (0.003 %) |  |
| Campeonato › Últimos vencedores | `TPRINTLISTDLG` | 12 px (0.003 %) |  |
| Campeonato › Melhores marcadores de sempre | `TPRINTLISTDLG` | 12 px (0.003 %) |  |
| Treinador › História | `TPRINTLISTDLG` | 12 px (0.003 %) |  |
| Treinador › Ranking | `TPRINTLISTDLG` | 22 px (0.005 %) |  |
| Treinador › Perfil | `TCOACHOPTIONSDLG` | 181 px (0.038 %) |  |

## Start and new game

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| About box at start-up | `TABOUTDLG` | not pixel-compared (1909 px) | animated: the notice blinks and the banner scrolls |
| Choice of game | `TSELECTGAMEDLG` | 3 px (0.001 %) |  |
| New game named | `TSELECTGAMEDLG` | 183 px (0.038 %) | the edit's caret blinks |
| Countries | `TSELECTCOUNTRIESDLG` | 19 px (0.004 %) |  |
| Countries, Brasil chosen | `TSELECTCOUNTRIESDLG` | 96 px (0.020 %) |  |
| Currency | `TCURRENCYDLG` | 54 px (0.011 %) |  |
| Coaches' names | `TGETTEAMSDLG` | 4 px (0.001 %) |  |
| Clubs drawn | `TGETTEAMSDLG` | not pixel-compared (3713 px) | the club is drawn at random |
| A new game's main window | `TPLANTELDLG` | not pixel-compared (1979 px) | the club is drawn at random |

## Team editor

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Editor without a team | `TEDITOR_MAIN` | 0 px (0.000 %) |  |
| Editor with a team (the flag's hint) | `TEDITOR_MAIN` | 122 px (0.025 %) |  |
| Editor, a player selected | `TEDITOR_MAIN` | 181 px (0.038 %) |  |
| Editor menu Ficheiro | `TEDITOR_MAIN` | 123 px (0.026 %) |  |
| Editor menu Jogador | `TEDITOR_MAIN` | 400 px (0.083 %) |  |
| Editor menu Equipa | `TEDITOR_MAIN` | 452 px (0.094 %) |  |
| Editor menu Elifoot | `TEDITOR_MAIN` | 125 px (0.026 %) |  |
| Abrir | `TOPENTEAMDLG` | 30 px (0.006 %) |  |
| Equipa › Nome | `TSETTEAMNAMEDLG` | 100 px (0.021 %) |  |
| Equipa › Cores | `TTEAMCOLORDLG` | 321 px (0.067 %) |  |
| Equipa › País | `TSETCOUNTRYDLG` | 167 px (0.035 %) |  |
| Equipa › Nível inicial | `TTEAMSKILLDLG` | 74 px (0.015 %) |  |
| Equipa › Treinador | `TCOACHNAMEDLG` | 74 px (0.015 %) |  |
| Jogador › Editar | `TEDITPLAYERDLG` | 188 px (0.039 %) |  |
| Jogador › Transferir | `TTRANSFERPLAYERDLG` | 88 px (0.018 %) |  |
| Elifoot › Acerca | `TABOUTDLG` | not pixel-compared (1820 px) | animated: blinking notice, scrolling banner |

## Match day

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Half-time | `TSUBSTITUTIONDLG` | not pixel-compared (113854 px) | the match differs (the original's random numbers): layout only |
| Standings after a round | `TCLASSIFICATIONDLG` | not pixel-compared (123925 px) | the match differs (the original's random numbers): layout only |
| Cup draw | `TSORTEIODLG` | not pixel-compared (16565 px) | the match differs (the original's random numbers): layout only |
| Penalty shoot-out | `TPENALTIESDLG` | not pixel-compared (122602 px) | the match differs (the original's random numbers): layout only |
| A human's penalty | `TPENALTYDLG` | not pixel-compared (190985 px) | the match differs (the original's random numbers): layout only |
| Coaches sacked | `TPRINTLISTDLG` | not pixel-compared (1543 px) | the match differs (the original's random numbers): layout only |
| End of season | `TENDOFSEASONDLG` | not pixel-compared (19383 px) | the match differs (the original's random numbers): layout only |
| Auction | `TSELLPLAYERDLG` | not pixel-compared (2778 px) | the match differs (the original's random numbers): layout only |
| Invitation to coach another club | `TINVITECOACHDLG` | 2 px (0.000 %) |  |

## Menu dialogs

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Balance sheet (a click on the Finanças list) | `TBALANCEDLG` | 17 px (0.004 %) |  |

## Start and new game

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Hint of the file name | `TSELECTGAMEDLG` | 30 px (0.006 %) | the capture's list shows ABCDEEEE |

## Menu dialogs

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Elifoot › Opções, page Jogo | `TOPTIONSDLG` | 11 px (0.002 %) |  |

## Main window

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| A player selected, page Jogo | `TPREPAREGAMEDLG` | 18 px (0.004 %) |  |
| A player selected, page Jogador | `TPREPAREGAMEDLG` | 18 px (0.004 %) |  |
| Renovar contrato (the player's page) | `TPREPAREGAMEDLG` | 218 px (0.045 %) |  |
| Jogador › Vender | `TPREPAREGAMEDLG` | 12 px (0.003 %) |  |

## Match day

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Round view during the games | `TJOURNEYDLG` | not pixel-compared (3892 px) | the match differs (the original's random numbers): layout only |

## Team editor

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| New team: name (the hint of the full name) | `TSETTEAMNAMEDLG` | 56 px (0.012 %) |  |
| New team: colours | `TTEAMCOLORDLG` | 94 px (0.020 %) |  |
| New team: country | `TSETCOUNTRYDLG` | 316 px (0.066 %) |  |
| New team: starting level | `TTEAMSKILLDLG` | 29 px (0.006 %) |  |
| New team: coach | `TCOACHNAMEDLG` | 57 px (0.012 %) |  |
| New team: the first player | `TEDITPLAYERDLG` | 176 px (0.037 %) |  |
| New team made (the flag's hint) | `TEDITOR_MAIN` | 242 px (0.050 %) |  |
| Gravar equipa | `TSAVETEAMDLG` | 235 px (0.049 %) |  |
| The team cannot be saved | `TCANTSAVETEAMDLG` | 214 px (0.045 %) |  |

## Main window

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| Renovar contrato, a low offer typed | `TPREPAREGAMEDLG` | 226 px (0.047 %) |  |
| The player refuses the offer (Actualização de Salário) | `TASKMORESALARYDLG` | 184 px (0.038 %) |  |

## Match day

| Screen | Form | Differing pixels | Notes |
|---|---|---|---|
| An injured player must be replaced | `TFORCEDSUBSTITUTIONDLG` | not pixel-compared (131008 px) | the match differs (the original's random numbers): layout only |

## Coverage

78 screens, 63 of them compared pixel for pixel (median 56 differing pixels); 34 of the 49 forms of `ELIFOOT.EXE` and `EDITEQ.EXE` appear in them. The others:

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
| `TLOANDLG` | registered copy only: its menu entry (Equipa › Pedir / pagar empréstimo) and the stadium's loan button are hidden in the unregistered game (1018:11f3; the stadium capture shows no loan button) |
| `TMAINWINDOW` | an empty form captioned "Elifoot 97", left in the program; never shown |
| `TOBSERVEDPLAYERSDLG` | registered copy only (Jogadores sob observação) |
| `TPROGRESSDLG` | the start-up gauge (captured as start_7_next, animated): not compared |
| `TREFEREEAIDDLG` | registered copy only: the referee's name opens it only with features 0x23 / 0x0a (1008:2427, 1018:3416); checked: a click at half-time does nothing |
| `TWAITDLG` | not shown in any recorded session; the port does not use it |
