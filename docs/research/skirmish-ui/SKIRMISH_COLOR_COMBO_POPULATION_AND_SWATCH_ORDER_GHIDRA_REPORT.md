# Skirmish Color Combo Population And Swatch Order - Ghidra Research Report

**Address(es):** `0x004E43C0`, `0x004E45A0`, `0x004E4770`, `0x004E4820`, `0x004E49A0`, `0x004E4C20`, `0x004E4E20`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Skirmish dialog `0x102` color combo table initialization, combo population, selection update, item-data lookup, and the adjacent `0x102` init/command callers that write skirmish player/AI color state.
**Non-Scope:** generic combo owner-draw paint internals, dropdown hit-testing internals, online/lobby callers of the same helpers, and side/start/team combo internals except where needed to identify state-write layout.
**Confidence:** High for the scoped `0x102` path.
**Active in YR:** Yes. `FUN_006AE2C0` passes dialog id `0x102` and proc address `0x006AE3F0` to `FUN_00622650`; `FUN_006AE3F0` dispatches custom init `0x497` to `FUN_006AE6E0` and `WM_COMMAND` `0x111` to `FUN_006ACEE0`.

## 1. Overview

Skirmish dialog `0x102` uses eight color combo controls: the local player row plus seven AI rows. The binary maintains a global color table with hardcoded swatch DWORDs and owner-slot markers; combo population adds only colors whose owner is this slot or unowned, so taken colors disappear from other enabled rows after every selection change.

The static table contains nine initialized swatches, but the normal population loop in `FUN_004E45A0` emits only item data `0..7`. Row `8` is initialized and addressable by helper logic, but it is not inserted into normal color combo dropdowns by this function.

## 2. Control IDs, Slots, And Table Layout

| Concept | Verified value | Evidence | Active in YR |
|---|---:|---|---|
| Dialog | `0x102` | `0x006AE321` sets `ECX=0x102`; `0x006AE31C` sets proc `0x006AE3F0` | Yes |
| Color controls | slot 0 `0x6A2`; slots 1..7 `0x522..0x528` | repeated maps in `0x004E4826..0x004E4888`, `0x004E4626..0x004E46B7`, `0x004E4C26..0x004E4C99` | Yes |
| Color table base | row base `0x008B4038`; swatch `+0x04`; owner `+0x08` | `FUN_004E43C0`, `FUN_004E45A0`, `FUN_004E4C20` | Yes |
| Owner sentinel | `-1` means unowned/available | `FUN_004E43C0` writes `0xFFFFFFFF` to each owner | Yes |
| Combo sentinel item data | `-2` means no actual color selection | `FUN_004E45A0`, `FUN_004E4770`, `FUN_004E4C20` | Yes |

Color combo ID to slot mapping is exact:

| Slot | Control ID |
|---:|---:|
| `0` | `0x6A2` |
| `1` | `0x522` |
| `2` | `0x523` |
| `3` | `0x524` |
| `4` | `0x525` |
| `5` | `0x526` |
| `6` | `0x527` |
| `7` | `0x528` |

The default/fallback expression used for non-matching IDs maps any non-`0x528` fallthrough to slot `-1` in some helpers or an invalid control id in refresh helpers. In the active `0x102` caller, only the eight listed control IDs are routed to these functions.

## 3. Swatch Values And Ordering

`FUN_004E43C0` loads nine color-name string pointers with string IDs `0x1DB..0x1E3`, then copies nine DWORD swatches from `0x008316A8` into `0x008B403C + row*12`, and initializes `0x008B4040 + row*12` to `-1`.

| Row / item data | Static swatch DWORD | Bytes in memory | Interpreted low/mid/high channels | Normal combo insertion |
|---:|---:|---|---|---|
| `0` | `0x000DE2DD` | `DD E2 0D 00` | `221,226,13` | Yes |
| `1` | `0x001919FF` | `FF 19 19 00` | `255,25,25` | Yes |
| `2` | `0x00E2742A` | `2A 74 E2 00` | `42,116,226` | Yes |
| `3` | `0x002ED13E` | `3E D1 2E 00` | `62,209,46` | Yes |
| `4` | `0x0019A0FF` | `FF A0 19 00` | `255,160,25` | Yes |
| `5` | `0x00E6D732` | `32 D7 E6 00` | `50,215,230` | Yes |
| `6` | `0x00BD2895` | `95 28 BD 00` | `149,40,189` | Yes |
| `7` | `0x00EB9AFF` | `FF 9A EB 00` | `255,154,235` | Yes |
| `8` | `0x00606060` | `60 60 60 00` | `96,96,96` | No in `FUN_004E45A0` |

Evidence: memory at `0x008316A8`; `FUN_004E43C0`; `FUN_004E45A0` loop starts at owner pointer `0x008B4040`, increments by `0x0C`, and continues only while the next owner pointer is `< 0x008B40A0`, so it processes rows `0..7` and stops before row `8` owner `0x008B40A0`. Active in YR: Yes, through `FUN_006AE6E0 -> FUN_004E43C0 -> FUN_004E4820`.

## 4. Population Flow

`FUN_006AE6E0` initializes the color system in this order: side helper `FUN_004E3B90`, color table init `FUN_004E43C0`, all-color-combo refresh `FUN_004E4820`, then start/team helpers. Active in YR: Yes, called from `FUN_006AE3F0` on custom init message `0x497`.

`FUN_004E4820` loops slots `0..7`, maps each slot to the control ID above, and chooses one of two population paths:

| Path | Condition | Effect | Evidence | Active in YR |
|---|---|---|---|---|
| Restricted grey path `FUN_004E4770` | `g_GameMode == 3 || g_GameMode == 4`, and row player pointer equals `DAT_00AC11B4` or row player `+0x6B == -1` | one disabled/grey row only | `0x004E4893..0x004E48BF` | Conditional; active only in modes `3`/`4` with local/closed row condition |
| Normal path `FUN_004E45A0` | all other cases | sentinel row plus available/taken-by-self color rows | `0x004E48B4` | Yes |

`FUN_004E45A0` hides the combo, resets content (`0x14B`), enables swatch mode (`0x4DD` lParam `1`), sets dropdown row cap (`0x4DE` lParam `9`), adds a `GUI:RandomAsSymbols` sentinel row with item data `-2` and row swatch `-1`, then iterates color rows `0..7`. Assembly at `0x004E45EF` loads key pointer `0x00822B7C`; the adjacent `0x20A` immediate is the `GDlgSupp.cpp` source-line argument, not a string ID. A row is added only when owner equals this slot or `-1`. Added color rows use display text pointer `0x00822B78` (`"ab"`), receive per-item swatch data via custom message `0x498`, and store item data equal to the color row number via `CB_SETITEMDATA` `0x151`. Active in YR: Yes.

`FUN_004E4770` is the restricted/grey path: it resets the combo, enables swatch mode, sets max rows to `9`, adds one string ID `0x237` row, assigns item data `-2`, writes swatch `-1`, selects row `0`, and sends custom message `0x4F1` lParam `1`. It does not add any real color row. Active in YR: Conditional as above.

## 5. Selection Update And Disabled/Taken Handling

`FUN_006ACEE0` routes `WM_COMMAND` for color controls `0x6A2` and `0x522..0x528` to `FUN_004E4C20` only when notification code is `1`. Active in YR: Yes, from `FUN_006AE3F0` handling `0x111`.

`FUN_004E4C20` performs the live selection update:

1. Map control ID to slot.
2. Scan all nine owner fields `0x008B4040..0x008B40A0`; the first row whose owner equals this slot is cleared to `-1`.
3. Read current selection with `CB_GETCURSEL` `0x147`.
4. Read selected item data with `CB_GETITEMDATA` `0x150`.
5. If item data is not `-2`, write this slot to `0x008B4040 + itemData*12`.
6. Refresh all eight color combos, which removes newly claimed colors from other normal combos.

Taken colors are not disabled in place; they are omitted from other normal combos on the all-combo refresh. Closed/restricted rows do not expose the taken color list at all; they display the single `-2` grey row. Active in YR: Yes for normal row color changes; Conditional for restricted rows.

`FUN_004E49A0` is the programmatic selection equivalent used by init and row-state changes. It clears old ownership for the target slot, scans existing combo rows until an item data equals the requested color, selects that row, writes owner if item data is not `-2`, restores visibility, then refreshes all eight color combos. If the requested color item data is absent from the combo, no owner is written before the refresh. Active in YR: Yes, used by `FUN_006AE6E0` initialization and `FUN_006ADC20`-adjacent row changes.

`FUN_004E4E20` is a getter, not a population function. If row argument is `-1`, it first reads current selection (`0x147`); then it returns item data through `0x150`. Its decompiler signature is misleadingly `void`, but the assembly returns the `SendDlgItemMessageA` result in `EAX`. Active in YR: Yes, used by tooltip flow and final state write flow.

## 6. Player/AI State Writes

Color combo selection changes do not directly write the final player/AI session arrays; they update the owner table and visible combos. Final state collection occurs in `FUN_006ACEE0` on Start/Back handling.

For AI rows, `FUN_006ACEE0` loops seven player/AI combo rows and, for rows whose AI-player item data is `0`, `1`, or `2`, writes:

| Destination | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `(&DAT_00A8B2BC)[row+1]` | selected color item data for AI row | `0x006AD4B3..0x006AD4C0` calls `FUN_004E4E20(-1)` and stores `EAX` | Yes |
| `DAT_00A8B3F4 + row*12` | persisted/snapshot color item data for AI row | `0x006AD5A5..0x006AD5E8`; color getter return stored at `[ECX+0x4]` | Yes |

For the local player row, `FUN_006ACEE0` calls `FUN_004E4E20(-1)` on color control `0x6A2`, then passes that item data to `FUN_0069B7E0(..., 1)`. Evidence: `0x006AD5FE..0x006AD636`. Active in YR: Yes.

During dialog initialization, `FUN_006AE6E0` reads saved AI row triples from `DAT_00A8B3EC..` / `DAT_00A8B3F0..` / `DAT_00A8B3F4..`; if the row type maps to closed (`local_14 == -1`), it programmatically sets side/color/start/team to `-2` and disables those controls. Otherwise it calls `FUN_004E49A0` with the saved color value at the row triple color field. Evidence: `0x006AE9F4..0x006AEAA1`. Active in YR: Yes.

## 7. INI Keys

No INI key is read by the scoped color combo functions. `rules.ini` / `rulesmd.ini` contain gameplay `[Colors]` color schemes, but this UI combo slice uses the hardcoded binary table at `0x008316A8` and string-table IDs in `GDlgSupp.cpp`. Active in YR: Yes, verified by decompilation of the scoped functions showing no INI parser calls in this path.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Dialog `0x102` activation | verified | `FUN_006AE2C0` instructions `0x006AE31C..0x006AE328` | none |
| Init dispatch | verified | `FUN_006AE3F0` message `0x497 -> FUN_006AE6E0` | none |
| Command dispatch | verified | `FUN_006AE3F0` `0x111 -> FUN_006ACEE0`; `FUN_006ACEE0` color controls notification `1 -> FUN_004E4C20` | none |
| Static color table init | verified | `FUN_004E43C0`; memory `0x008316A8` | none |
| Normal combo population | verified | `FUN_004E45A0` | none |
| Restricted grey combo population | verified | `FUN_004E4770`; gate in `FUN_004E4820` | none |
| All-combo refresh | verified | `FUN_004E4820`; duplicated loops in `FUN_004E49A0` and `FUN_004E4C20` | none |
| Programmatic color selection | verified | `FUN_004E49A0` | none |
| User color selection update | verified | `FUN_004E4C20` | none |
| Selected item-data getter | verified | `FUN_004E4E20` assembly return in `EAX` | none |
| Final player/AI color state writes | verified | `FUN_006ACEE0` store sites `0x006AD4B3..0x006AD4C0`, `0x006AD5A5..0x006AD5E8`, `0x006AD5FE..0x006AD636` | none for color fields |
| Generic owner-draw callback visuals | deferred | user scope assigns visuals to slot 1 | out-of-scope |
| Non-`0x102` callers (`0x005E...`, WOL/lobby-like paths) | deferred | `get_function_callers` results for shared helpers | out-of-scope for this Skirmish dialog slice |

## 9. Open Questions - Final State

[RESOLVED] OQ1 - Which controls are color combos? Slot `0` is `0x6A2`; slots `1..7` are `0x522..0x528`. Evidence: `FUN_004E4820`, `FUN_004E45A0`, `FUN_004E4C20`.

[RESOLVED] OQ2 - Does the combo populate all nine initialized swatches? No. `FUN_004E43C0` initializes nine rows, but `FUN_004E45A0` adds only rows `0..7`; row `8` at owner pointer `0x008B40A0` is not inserted by the normal loop. Evidence: `0x004E46BD..0x004E472C`.

[RESOLVED] OQ3 - How are taken colors handled? The selected slot's previous owner row is cleared; the new selected item data owns the row unless it is `-2`; all combos are then rebuilt so other normal rows omit that color. Evidence: `FUN_004E4C20`.

[RESOLVED] OQ4 - What does disabled/restricted handling do? It does not list colors; it creates one `-2` row from string ID `0x237`, sets grey flag `0x4F1=1`, and selects row `0`. Evidence: `FUN_004E4770`.

[RESOLVED] OQ5 - Does `FUN_004E4E20` write state? No. It returns selected item data; later callers store that value. Evidence: `0x004E4E20..0x004E4E52`, `FUN_006ACEE0`.

## Sources

- Ghidra decompile/disassembly: `FUN_006AE2C0`, `FUN_006AE3F0`, `FUN_006AE6E0`, `FUN_006ACEE0`.
- Ghidra decompile/disassembly: `FUN_004E43C0`, `FUN_004E45A0`, `FUN_004E4770`, `FUN_004E4820`, `FUN_004E49A0`, `FUN_004E4C20`, `FUN_004E4E20`.
- Ghidra memory: `0x008316A8` swatch table.
- Prior docs cross-checked: `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`, `traces/SKIRMISH_PLAYER_AI_COMBOS_FLAGS_TRACE.md`, `SKIRMISH_SHELL_RETAIL_ASSETS_GHIDRA_REPORT.md`.
