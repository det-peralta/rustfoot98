# Delphi class table (ELIFOOT.EXE)

Recovered with `tools/ghidra/ExportClasses.java` from the Delphi 1 class entries
(`[selfptr][instance size][w][0x1068][far ptrs to virtual methods][name pstr]`).
195 classes total; 44 are form classes (`w=1222`, 35 virtual methods, TForm descendants).

## Game model classes (all derive from a common base with 13-14 virtual methods)
| Class | Instance bytes | Virtual methods | Notes |
|---|---|---|---|
| `TPlayerTransfer` | 306 | 13 |  |
| `TFileFormat` | 27 | 13 |  |
| `TListOfTeams` | 12 | 6 |  |
| `TListOfDivisions` | 12 | 6 |  |
| `TListOfPlayers` | 12 | 6 |  |
| `TListOfCoaches` | 12 | 6 |  |
| `TListOfLastResults` | 12 | 6 |  |
| `TListOfGames` | 12 | 6 |  |
| `TListOfReferees` | 12 | 6 |  |
| `TCalendar` | 12 | 6 |  |
| `TCalendarEntry` | 20 | 13 | fixture calendar |
| `TTeam` | 203 | 13 | main team record |
| `TDivision` | 35 | 14 | league division |
| `TPlayer` | 56 | 13 | player |
| `TCoach` | 35 | 13 | coach |
| `TGame` | 81 | 14 | a match |
| `TLash` | 9 | 13 | ? |
| `TCoachEvent` | 11 | 13 | coach event |
| `TTeamEvent` | 11 | 13 | team event |
| `TLastResult` | 12 | 13 | last results |
| `TReferee` | 11 | 13 | referee |
| `TStadium` | 11 | 13 | stadium |
| `TGameEvent` | 13 | 13 | match event (goal, card…) |
| `TLastWinners` | 14 | 13 | champions history |
| `TLastTransfer` | 16 | 13 | transfer log |
| `TCountry` | 32 | 4 |  |
| `TListOfCountries` | 12 | 6 |  |
| `TElifootObject` | 4 | 13 |  |
| `TPlayerAttr` | 31 | 13 |  |
| `TRefereeAttr` | 49 | 13 |  |
| `TEditorCoach` | 8 | 13 |  |

The `TList*` classes are 12-byte list wrappers (6 virtual methods). Class-specific virtual
methods (the last pointers in each entry) are the candidates for stream Load/Save, which will
define the `.e98` save format.

## Forms (49)
See `tools/ne_resources.py` + `tools/dfm.py`: extract RCDATA `TPF0` resources and parse them
to JSON (control trees, geometry, captions, event handlers). Totals across forms:
227 TLabel, 91 TBitBtn, 66 TMenuItem, 30 TBevel, 29 TListBox, 19 TEdit, 16 TComboBox,
12 TTimer, 9 TCheckBox, 5 TBiGauge + 4 TGauge, 295 event-handler bindings.
