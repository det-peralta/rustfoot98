# Gallery

Each screen three times: the original under js-dos, the port's render of the same state, and the difference (the port faded, differing pixels in red). The counts are in [Screen by screen](screens.md).

!!! note "Where the pictures come from"
    They are drawn with the game's own fonts and bitmaps, so they are not stored in the repository: `tools/site/build.sh` renders and compares them from your own copy and puts them in `docs/verification/img/` before building the site.

## Main window

### Main window, page Jogo

55 px (0.011 %) · `TPREPAREGAMEDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/main_0_jogo.png)](img/original/main_0_jogo.png) | [![port](img/port/main_0_jogo.png)](img/port/main_0_jogo.png) | [![difference](img/diff/main_0_jogo.png)](img/diff/main_0_jogo.png) |

### Main window, page Jogador

25 px (0.005 %) · `TPREPAREGAMEDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/main_1_jogador.png)](img/original/main_1_jogador.png) | [![port](img/port/main_1_jogador.png)](img/port/main_1_jogador.png) | [![difference](img/diff/main_1_jogador.png)](img/diff/main_1_jogador.png) |

### Main window, page Finanças

71 px (0.015 %) · `TPREPAREGAMEDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/main_2_financas.png)](img/original/main_2_financas.png) | [![port](img/port/main_2_financas.png)](img/port/main_2_financas.png) | [![difference](img/diff/main_2_financas.png)](img/diff/main_2_financas.png) |

### Main window, page Selecção

25 px (0.005 %) · `TPREPAREGAMEDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/main_3_seleccao.png)](img/original/main_3_seleccao.png) | [![port](img/port/main_3_seleccao.png)](img/port/main_3_seleccao.png) | [![difference](img/diff/main_3_seleccao.png)](img/diff/main_3_seleccao.png) |

### Main window, page Adversário

33 px (0.007 %) · `TPREPAREGAMEDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/main_4_adversario.png)](img/original/main_4_adversario.png) | [![port](img/port/main_4_adversario.png)](img/port/main_4_adversario.png) | [![difference](img/diff/main_4_adversario.png)](img/diff/main_4_adversario.png) |

## Menus

### Menu Elifoot

50 px (0.010 %) · `TPREPAREGAMEDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/menu_0_elifoot.png)](img/original/menu_0_elifoot.png) | [![port](img/port/menu_0_elifoot.png)](img/port/menu_0_elifoot.png) | [![difference](img/diff/menu_0_elifoot.png)](img/diff/menu_0_elifoot.png) |

### Menu Seleccionar

70 px (0.015 %) · `TPREPAREGAMEDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/menu_1_seleccionar.png)](img/original/menu_1_seleccionar.png) | [![port](img/port/menu_1_seleccionar.png)](img/port/menu_1_seleccionar.png) | [![difference](img/diff/menu_1_seleccionar.png)](img/diff/menu_1_seleccionar.png) |

### Menu Equipa

166 px (0.035 %) · `TPREPAREGAMEDLG` · the drop-down is 1 px narrower than the original's (the menus whose registered-only entries are hidden; cause not found)

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/menu_2_equipa.png)](img/original/menu_2_equipa.png) | [![port](img/port/menu_2_equipa.png)](img/port/menu_2_equipa.png) | [![difference](img/diff/menu_2_equipa.png)](img/diff/menu_2_equipa.png) |

### Menu Jogador

185 px (0.039 %) · `TPREPAREGAMEDLG` · the drop-down is 1 px narrower than the original's (the menus whose registered-only entries are hidden; cause not found)

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/menu_3_jogador.png)](img/original/menu_3_jogador.png) | [![port](img/port/menu_3_jogador.png)](img/port/menu_3_jogador.png) | [![difference](img/diff/menu_3_jogador.png)](img/diff/menu_3_jogador.png) |

### Menu Campeonato

48 px (0.010 %) · `TPREPAREGAMEDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/menu_4_campeonato.png)](img/original/menu_4_campeonato.png) | [![port](img/port/menu_4_campeonato.png)](img/port/menu_4_campeonato.png) | [![difference](img/diff/menu_4_campeonato.png)](img/diff/menu_4_campeonato.png) |

### Menu Treinador

236 px (0.049 %) · `TPREPAREGAMEDLG` · the drop-down is 1 px narrower than the original's (the menus whose registered-only entries are hidden; cause not found)

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/menu_5_treinador.png)](img/original/menu_5_treinador.png) | [![port](img/port/menu_5_treinador.png)](img/port/menu_5_treinador.png) | [![difference](img/diff/menu_5_treinador.png)](img/diff/menu_5_treinador.png) |

## Menu dialogs

### Elifoot › Opções

140 px (0.029 %) · `TOPTIONSDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/dlg_opcoeselifoot.png)](img/original/dlg_opcoeselifoot.png) | [![port](img/port/dlg_opcoeselifoot.png)](img/port/dlg_opcoeselifoot.png) | [![difference](img/diff/dlg_opcoeselifoot.png)](img/diff/dlg_opcoeselifoot.png) |

### Elifoot › Acerca

not pixel-compared (3035 px) · `TABOUTDLG` · animated: the notice blinks every 500 ms and the banner scrolls, so the capture's moment decides

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/dlg_acerca1.png)](img/original/dlg_acerca1.png) | [![port](img/port/dlg_acerca1.png)](img/port/dlg_acerca1.png) | [![difference](img/diff/dlg_acerca1.png)](img/diff/dlg_acerca1.png) |

### Equipa › Estádio

474 px (0.099 %) · `TSTADIUMDLG` · the stadium picture is drawn from measurements (~400 px of its outline)

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/dlg_estdio1.png)](img/original/dlg_estdio1.png) | [![port](img/port/dlg_estdio1.png)](img/port/dlg_estdio1.png) | [![difference](img/diff/dlg_estdio1.png)](img/diff/dlg_estdio1.png) |

### Equipa › Historial

51 px (0.011 %) · `TPRINTLISTDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/dlg_historial1.png)](img/original/dlg_historial1.png) | [![port](img/port/dlg_historial1.png)](img/port/dlg_historial1.png) | [![difference](img/diff/dlg_historial1.png)](img/diff/dlg_historial1.png) |

### Jogador › Últimas transferências

49 px (0.010 %) · `TPRINTLISTDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/dlg_ltimastransferncias1.png)](img/original/dlg_ltimastransferncias1.png) | [![port](img/port/dlg_ltimastransferncias1.png)](img/port/dlg_ltimastransferncias1.png) | [![difference](img/diff/dlg_ltimastransferncias1.png)](img/diff/dlg_ltimastransferncias1.png) |

### Campeonato › Classificação

82 px (0.017 %) · `TCLASSIFICATIONDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/dlg_campeonatoclassificao.png)](img/original/dlg_campeonatoclassificao.png) | [![port](img/port/dlg_campeonatoclassificao.png)](img/port/dlg_campeonatoclassificao.png) | [![difference](img/diff/dlg_campeonatoclassificao.png)](img/diff/dlg_campeonatoclassificao.png) |

### Campeonato › Melhores marcadores

286 px (0.060 %) · `TPRINTLISTDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/dlg_campeonatomelhoresmarcadores.png)](img/original/dlg_campeonatomelhoresmarcadores.png) | [![port](img/port/dlg_campeonatomelhoresmarcadores.png)](img/port/dlg_campeonatomelhoresmarcadores.png) | [![difference](img/diff/dlg_campeonatomelhoresmarcadores.png)](img/diff/dlg_campeonatomelhoresmarcadores.png) |

### Campeonato › Calendário

22 px (0.005 %) · `TPRINTLISTDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/dlg_menucalendrio.png)](img/original/dlg_menucalendrio.png) | [![port](img/port/dlg_menucalendrio.png)](img/port/dlg_menucalendrio.png) | [![difference](img/diff/dlg_menucalendrio.png)](img/diff/dlg_menucalendrio.png) |

### Campeonato › Últimos vencedores

49 px (0.010 %) · `TPRINTLISTDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/dlg_ltimosvencedores1.png)](img/original/dlg_ltimosvencedores1.png) | [![port](img/port/dlg_ltimosvencedores1.png)](img/port/dlg_ltimosvencedores1.png) | [![difference](img/diff/dlg_ltimosvencedores1.png)](img/diff/dlg_ltimosvencedores1.png) |

### Campeonato › Melhores marcadores de sempre

286 px (0.060 %) · `TPRINTLISTDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/dlg_melhoresmarcadoresdesempre1.png)](img/original/dlg_melhoresmarcadoresdesempre1.png) | [![port](img/port/dlg_melhoresmarcadoresdesempre1.png)](img/port/dlg_melhoresmarcadoresdesempre1.png) | [![difference](img/diff/dlg_melhoresmarcadoresdesempre1.png)](img/diff/dlg_melhoresmarcadoresdesempre1.png) |

### Treinador › História

51 px (0.011 %) · `TPRINTLISTDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/dlg_treinadorhistoria.png)](img/original/dlg_treinadorhistoria.png) | [![port](img/port/dlg_treinadorhistoria.png)](img/port/dlg_treinadorhistoria.png) | [![difference](img/diff/dlg_treinadorhistoria.png)](img/diff/dlg_treinadorhistoria.png) |

### Treinador › Ranking

222 px (0.046 %) · `TPRINTLISTDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/dlg_coachranking.png)](img/original/dlg_coachranking.png) | [![port](img/port/dlg_coachranking.png)](img/port/dlg_coachranking.png) | [![difference](img/diff/dlg_coachranking.png)](img/diff/dlg_coachranking.png) |

### Treinador › Perfil

227 px (0.047 %) · `TCOACHOPTIONSDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/dlg_opesindividuais1.png)](img/original/dlg_opesindividuais1.png) | [![port](img/port/dlg_opesindividuais1.png)](img/port/dlg_opesindividuais1.png) | [![difference](img/diff/dlg_opesindividuais1.png)](img/diff/dlg_opesindividuais1.png) |

## Start and new game

### About box at start-up

not pixel-compared (1971 px) · `TABOUTDLG` · animated: the notice blinks and the banner scrolls

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/start_0_about.png)](img/original/start_0_about.png) | [![port](img/port/start_0_about.png)](img/port/start_0_about.png) | [![difference](img/diff/start_0_about.png)](img/diff/start_0_about.png) |

### Choice of game

66 px (0.014 %) · `TSELECTGAMEDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/start_1_select.png)](img/original/start_1_select.png) | [![port](img/port/start_1_select.png)](img/port/start_1_select.png) | [![difference](img/diff/start_1_select.png)](img/diff/start_1_select.png) |

### New game named

364 px (0.076 %) · `TSELECTGAMEDLG` · the edit's caret blinks

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/start_3_named.png)](img/original/start_3_named.png) | [![port](img/port/start_3_named.png)](img/port/start_3_named.png) | [![difference](img/diff/start_3_named.png)](img/diff/start_3_named.png) |

### Countries

85 px (0.018 %) · `TSELECTCOUNTRIESDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/start_4_next.png)](img/original/start_4_next.png) | [![port](img/port/start_4_next.png)](img/port/start_4_next.png) | [![difference](img/diff/start_4_next.png)](img/diff/start_4_next.png) |

### Countries, Brasil chosen

153 px (0.032 %) · `TSELECTCOUNTRIESDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/start_5_brasil.png)](img/original/start_5_brasil.png) | [![port](img/port/start_5_brasil.png)](img/port/start_5_brasil.png) | [![difference](img/diff/start_5_brasil.png)](img/diff/start_5_brasil.png) |

### Currency

182 px (0.038 %) · `TCURRENCYDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/start_6_next.png)](img/original/start_6_next.png) | [![port](img/port/start_6_next.png)](img/port/start_6_next.png) | [![difference](img/diff/start_6_next.png)](img/diff/start_6_next.png) |

### Coaches' names

4 px (0.001 %) · `TGETTEAMSDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/start_8_next.png)](img/original/start_8_next.png) | [![port](img/port/start_8_next.png)](img/port/start_8_next.png) | [![difference](img/diff/start_8_next.png)](img/diff/start_8_next.png) |

### Clubs drawn

not pixel-compared (3713 px) · `TGETTEAMSDLG` · the club is drawn at random

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/start_12_drawn.png)](img/original/start_12_drawn.png) | [![port](img/port/start_12_drawn.png)](img/port/start_12_drawn.png) | [![difference](img/diff/start_12_drawn.png)](img/diff/start_12_drawn.png) |

### A new game's main window

not pixel-compared (2012 px) · `TPLANTELDLG` · the club is drawn at random

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/start_13_main.png)](img/original/start_13_main.png) | [![port](img/port/start_13_main.png)](img/port/start_13_main.png) | [![difference](img/diff/start_13_main.png)](img/diff/start_13_main.png) |

## Team editor

### Editor without a team

688 px (0.143 %) · `TEDITOR_MAIN`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_empty.png)](img/original/ed_empty.png) | [![port](img/port/ed_empty.png)](img/port/ed_empty.png) | [![difference](img/diff/ed_empty.png)](img/diff/ed_empty.png) |

### Editor with a team (the flag's hint)

165 px (0.034 %) · `TEDITOR_MAIN`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_team.png)](img/original/ed_team.png) | [![port](img/port/ed_team.png)](img/port/ed_team.png) | [![difference](img/diff/ed_team.png)](img/diff/ed_team.png) |

### Editor, a player selected

224 px (0.047 %) · `TEDITOR_MAIN`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_team_selected.png)](img/original/ed_team_selected.png) | [![port](img/port/ed_team_selected.png)](img/port/ed_team_selected.png) | [![difference](img/diff/ed_team_selected.png)](img/diff/ed_team_selected.png) |

### Editor menu Ficheiro

151 px (0.031 %) · `TEDITOR_MAIN`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_menu_ficheiro.png)](img/original/ed_menu_ficheiro.png) | [![port](img/port/ed_menu_ficheiro.png)](img/port/ed_menu_ficheiro.png) | [![difference](img/diff/ed_menu_ficheiro.png)](img/diff/ed_menu_ficheiro.png) |

### Editor menu Jogador

437 px (0.091 %) · `TEDITOR_MAIN`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_menu_jogador.png)](img/original/ed_menu_jogador.png) | [![port](img/port/ed_menu_jogador.png)](img/port/ed_menu_jogador.png) | [![difference](img/diff/ed_menu_jogador.png)](img/diff/ed_menu_jogador.png) |

### Editor menu Equipa

494 px (0.103 %) · `TEDITOR_MAIN`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_menu_equipa.png)](img/original/ed_menu_equipa.png) | [![port](img/port/ed_menu_equipa.png)](img/port/ed_menu_equipa.png) | [![difference](img/diff/ed_menu_equipa.png)](img/diff/ed_menu_equipa.png) |

### Editor menu Elifoot

168 px (0.035 %) · `TEDITOR_MAIN`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_menu_elifoot.png)](img/original/ed_menu_elifoot.png) | [![port](img/port/ed_menu_elifoot.png)](img/port/ed_menu_elifoot.png) | [![difference](img/diff/ed_menu_elifoot.png)](img/diff/ed_menu_elifoot.png) |

### Abrir

34 px (0.007 %) · `TOPENTEAMDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_open.png)](img/original/ed_open.png) | [![port](img/port/ed_open.png)](img/port/ed_open.png) | [![difference](img/diff/ed_open.png)](img/diff/ed_open.png) |

### Equipa › Nome

121 px (0.025 %) · `TSETTEAMNAMEDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_name.png)](img/original/ed_name.png) | [![port](img/port/ed_name.png)](img/port/ed_name.png) | [![difference](img/diff/ed_name.png)](img/diff/ed_name.png) |

### Equipa › Cores

348 px (0.072 %) · `TTEAMCOLORDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_colors.png)](img/original/ed_colors.png) | [![port](img/port/ed_colors.png)](img/port/ed_colors.png) | [![difference](img/diff/ed_colors.png)](img/diff/ed_colors.png) |

### Equipa › País

186 px (0.039 %) · `TSETCOUNTRYDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_country.png)](img/original/ed_country.png) | [![port](img/port/ed_country.png)](img/port/ed_country.png) | [![difference](img/diff/ed_country.png)](img/diff/ed_country.png) |

### Equipa › Nível inicial

118 px (0.025 %) · `TTEAMSKILLDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_level.png)](img/original/ed_level.png) | [![port](img/port/ed_level.png)](img/port/ed_level.png) | [![difference](img/diff/ed_level.png)](img/diff/ed_level.png) |

### Equipa › Treinador

95 px (0.020 %) · `TCOACHNAMEDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_coach.png)](img/original/ed_coach.png) | [![port](img/port/ed_coach.png)](img/port/ed_coach.png) | [![difference](img/diff/ed_coach.png)](img/diff/ed_coach.png) |

### Jogador › Editar

205 px (0.043 %) · `TEDITPLAYERDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_player.png)](img/original/ed_player.png) | [![port](img/port/ed_player.png)](img/port/ed_player.png) | [![difference](img/diff/ed_player.png)](img/diff/ed_player.png) |

### Jogador › Transferir

189 px (0.039 %) · `TTRANSFERPLAYERDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_transfer.png)](img/original/ed_transfer.png) | [![port](img/port/ed_transfer.png)](img/port/ed_transfer.png) | [![difference](img/diff/ed_transfer.png)](img/diff/ed_transfer.png) |

### Elifoot › Acerca

not pixel-compared (1882 px) · `TABOUTDLG` · animated: blinking notice, scrolling banner

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_about.png)](img/original/ed_about.png) | [![port](img/port/ed_about.png)](img/port/ed_about.png) | [![difference](img/diff/ed_about.png)](img/diff/ed_about.png) |

## Match day

### Half-time

not pixel-compared (113850 px) · `TSUBSTITUTIONDLG` · the match differs (the original's random numbers): layout only

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/md_halftime.png)](img/original/md_halftime.png) | [![port](img/port/md_halftime.png)](img/port/md_halftime.png) | [![difference](img/diff/md_halftime.png)](img/diff/md_halftime.png) |

### Standings after a round

not pixel-compared (123945 px) · `TCLASSIFICATIONDLG` · the match differs (the original's random numbers): layout only

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/md_standings.png)](img/original/md_standings.png) | [![port](img/port/md_standings.png)](img/port/md_standings.png) | [![difference](img/diff/md_standings.png)](img/diff/md_standings.png) |

### Cup draw

not pixel-compared (16504 px) · `TSORTEIODLG` · the match differs (the original's random numbers): layout only

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/md_cupdraw.png)](img/original/md_cupdraw.png) | [![port](img/port/md_cupdraw.png)](img/port/md_cupdraw.png) | [![difference](img/diff/md_cupdraw.png)](img/diff/md_cupdraw.png) |

### Penalty shoot-out

not pixel-compared (122602 px) · `TPENALTIESDLG` · the match differs (the original's random numbers): layout only

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/md_shootout.png)](img/original/md_shootout.png) | [![port](img/port/md_shootout.png)](img/port/md_shootout.png) | [![difference](img/diff/md_shootout.png)](img/diff/md_shootout.png) |

### A human's penalty

not pixel-compared (190978 px) · `TPENALTYDLG` · the match differs (the original's random numbers): layout only

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/md_penalty.png)](img/original/md_penalty.png) | [![port](img/port/md_penalty.png)](img/port/md_penalty.png) | [![difference](img/diff/md_penalty.png)](img/diff/md_penalty.png) |

### Coaches sacked

not pixel-compared (1543 px) · `TPRINTLISTDLG` · the match differs (the original's random numbers): layout only

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/md_lashes.png)](img/original/md_lashes.png) | [![port](img/port/md_lashes.png)](img/port/md_lashes.png) | [![difference](img/diff/md_lashes.png)](img/diff/md_lashes.png) |

### End of season

not pixel-compared (19401 px) · `TENDOFSEASONDLG` · the match differs (the original's random numbers): layout only

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/md_endofseason.png)](img/original/md_endofseason.png) | [![port](img/port/md_endofseason.png)](img/port/md_endofseason.png) | [![difference](img/diff/md_endofseason.png)](img/diff/md_endofseason.png) |

### Auction

not pixel-compared (2819 px) · `TSELLPLAYERDLG` · the match differs (the original's random numbers): layout only

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/md_auction.png)](img/original/md_auction.png) | [![port](img/port/md_auction.png)](img/port/md_auction.png) | [![difference](img/diff/md_auction.png)](img/diff/md_auction.png) |

### Invitation to coach another club

9 px (0.002 %) · `TINVITECOACHDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/md_invitation.png)](img/original/md_invitation.png) | [![port](img/port/md_invitation.png)](img/port/md_invitation.png) | [![difference](img/diff/md_invitation.png)](img/diff/md_invitation.png) |

## Menu dialogs

### Balance sheet (a click on the Finanças list)

74 px (0.015 %) · `TBALANCEDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/dlg_balance.png)](img/original/dlg_balance.png) | [![port](img/port/dlg_balance.png)](img/port/dlg_balance.png) | [![difference](img/diff/dlg_balance.png)](img/diff/dlg_balance.png) |

## Start and new game

### Hint of the file name

93 px (0.019 %) · `TSELECTGAMEDLG` · the capture's list shows ABCDEEEE

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/hint_gamename.png)](img/original/hint_gamename.png) | [![port](img/port/hint_gamename.png)](img/port/hint_gamename.png) | [![difference](img/diff/hint_gamename.png)](img/diff/hint_gamename.png) |

## Menu dialogs

### Elifoot › Opções, page Jogo

78 px (0.016 %) · `TOPTIONSDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/dlg_opcoes_jogo.png)](img/original/dlg_opcoes_jogo.png) | [![port](img/port/dlg_opcoes_jogo.png)](img/port/dlg_opcoes_jogo.png) | [![difference](img/diff/dlg_opcoes_jogo.png)](img/diff/dlg_opcoes_jogo.png) |

## Main window

### A player selected, page Jogo

64 px (0.013 %) · `TPREPAREGAMEDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/main_sel_jogo.png)](img/original/main_sel_jogo.png) | [![port](img/port/main_sel_jogo.png)](img/port/main_sel_jogo.png) | [![difference](img/diff/main_sel_jogo.png)](img/diff/main_sel_jogo.png) |

### A player selected, page Jogador

64 px (0.013 %) · `TPREPAREGAMEDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/main_sel_jogador.png)](img/original/main_sel_jogador.png) | [![port](img/port/main_sel_jogador.png)](img/port/main_sel_jogador.png) | [![difference](img/diff/main_sel_jogador.png)](img/diff/main_sel_jogador.png) |

### Renovar contrato (the player's page)

342 px (0.071 %) · `TPREPAREGAMEDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/main_contract.png)](img/original/main_contract.png) | [![port](img/port/main_contract.png)](img/port/main_contract.png) | [![difference](img/diff/main_contract.png)](img/diff/main_contract.png) |

### Jogador › Vender

42 px (0.009 %) · `TPREPAREGAMEDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/main_sell.png)](img/original/main_sell.png) | [![port](img/port/main_sell.png)](img/port/main_sell.png) | [![difference](img/diff/main_sell.png)](img/diff/main_sell.png) |

## Match day

### Round view during the games

not pixel-compared (3917 px) · `TJOURNEYDLG` · the match differs (the original's random numbers): layout only

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/md_roundview.png)](img/original/md_roundview.png) | [![port](img/port/md_roundview.png)](img/port/md_roundview.png) | [![difference](img/diff/md_roundview.png)](img/diff/md_roundview.png) |

## Team editor

### New team: name (the hint of the full name)

55 px (0.011 %) · `TSETTEAMNAMEDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_new_name.png)](img/original/ed_new_name.png) | [![port](img/port/ed_new_name.png)](img/port/ed_new_name.png) | [![difference](img/diff/ed_new_name.png)](img/diff/ed_new_name.png) |

### New team: colours

93 px (0.019 %) · `TTEAMCOLORDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_new_color.png)](img/original/ed_new_color.png) | [![port](img/port/ed_new_color.png)](img/port/ed_new_color.png) | [![difference](img/diff/ed_new_color.png)](img/diff/ed_new_color.png) |

### New team: country

315 px (0.066 %) · `TSETCOUNTRYDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_new_country.png)](img/original/ed_new_country.png) | [![port](img/port/ed_new_country.png)](img/port/ed_new_country.png) | [![difference](img/diff/ed_new_country.png)](img/diff/ed_new_country.png) |

### New team: starting level

51 px (0.011 %) · `TTEAMSKILLDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_new_level.png)](img/original/ed_new_level.png) | [![port](img/port/ed_new_level.png)](img/port/ed_new_level.png) | [![difference](img/diff/ed_new_level.png)](img/diff/ed_new_level.png) |

### New team: coach

56 px (0.012 %) · `TCOACHNAMEDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_new_coach.png)](img/original/ed_new_coach.png) | [![port](img/port/ed_new_coach.png)](img/port/ed_new_coach.png) | [![difference](img/diff/ed_new_coach.png)](img/diff/ed_new_coach.png) |

### New team: the first player

176 px (0.037 %) · `TEDITPLAYERDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_new_player.png)](img/original/ed_new_player.png) | [![port](img/port/ed_new_player.png)](img/port/ed_new_player.png) | [![difference](img/diff/ed_new_player.png)](img/diff/ed_new_player.png) |

### New team made (the flag's hint)

249 px (0.052 %) · `TEDITOR_MAIN`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_new_team.png)](img/original/ed_new_team.png) | [![port](img/port/ed_new_team.png)](img/port/ed_new_team.png) | [![difference](img/diff/ed_new_team.png)](img/diff/ed_new_team.png) |

### Gravar equipa

291 px (0.061 %) · `TSAVETEAMDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_save.png)](img/original/ed_save.png) | [![port](img/port/ed_save.png)](img/port/ed_save.png) | [![difference](img/diff/ed_save.png)](img/diff/ed_save.png) |

### The team cannot be saved

232 px (0.048 %) · `TCANTSAVETEAMDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/ed_cantsave.png)](img/original/ed_cantsave.png) | [![port](img/port/ed_cantsave.png)](img/port/ed_cantsave.png) | [![difference](img/diff/ed_cantsave.png)](img/diff/ed_cantsave.png) |

## Main window

### Renovar contrato, a low offer typed

335 px (0.070 %) · `TPREPAREGAMEDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/main_contract_low.png)](img/original/main_contract_low.png) | [![port](img/port/main_contract_low.png)](img/port/main_contract_low.png) | [![difference](img/diff/main_contract_low.png)](img/diff/main_contract_low.png) |

### The player refuses the offer (Actualização de Salário)

274 px (0.057 %) · `TASKMORESALARYDLG`

| Original | Port | Difference |
|---|---|---|
| [![original](img/original/dlg_askmore.png)](img/original/dlg_askmore.png) | [![port](img/port/dlg_askmore.png)](img/port/dlg_askmore.png) | [![difference](img/diff/dlg_askmore.png)](img/diff/dlg_askmore.png) |

