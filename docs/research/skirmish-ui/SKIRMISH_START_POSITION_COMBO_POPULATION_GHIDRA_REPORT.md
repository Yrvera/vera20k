# Skirmish Start-Position Combo Population - Ghidra Research Report

**Address(es):** `0x006AE6E0` initialization, `0x006ACEE0` apply/message path  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** offline Skirmish dialog `0x102` start-position combo population, adjacent A-D/None ambiguity, mode gates, item data, and final start writes.  
**Non-Scope:** exact Win32 dialog template coordinates, runtime scenario start assignment after `NodeNameTag`/AI globals, multiplayer lobby behavior beyond gates that affect this helper.  
**Confidence:** High for binary-traced population/order/item data and final writes; Medium for one-character string text labels, using the prior verified string-cluster report plus pointer order.  
**Active in YR:** Yes for offline Skirmish `0x102`; Conditional for network/coop observer gates documented below.

## 1. Overview

The offline Skirmish start-position controls are combo boxes `0x6A3`, `0x6A4`, `0x6A5`, `0x6A6`, `0x6A7`, `0x6A8`, `0x6AA`, `0x6AB`. They are populated by `FUN_004E50C0`, called from `FUN_004E5310` during `FUN_006AE6E0` initialization and repopulated after start-combo changes by `FUN_004E5700`.

The actual standard offline start list is `Random`, then available numbered starts `1` through map-start-count-limited `8`, with per-item data `-2`, then `0..7`. The nearby `A-D` and `None` strings belong to the adjacent team combo `0x76D`-`0x774`, not the start-position combo in standard offline Skirmish.

**Correction, 2026-05-21:** `SKIRMISH_START_TEAM_CONTROL_DESTINATION_NAMING_GHIDRA_REPORT.md` rechecked the final Start Game write block and found that this report's original destination rows for `DAT_00A8B2FC` and `DAT_00A8B3A4` were mislabeled. Start controls still are `0x6A3..0x6AB`, but their final destinations are `DAT_00A8B2DC[slot]`, `DAT_00A8B39C`, and node `+0x5B`. Team controls are `0x76D..0x774`, and their final destinations are `DAT_00A8B2FC[slot]`, `DAT_00A8B3A4`, and node `+0x63`.

## 2. Class Layout / Key Offsets

| Field/global | Purpose | Evidence | Active in YR |
|---|---|---|---|
| `DAT_008B3F30` table string pointer column | start combo display strings, ordered `1..8`, plus an initialized-but-not-inserted `0` entry | `FUN_004E4F50` writes `0x00822BC4` down to `0x00822BA8`, then `0x00822BA4` | Yes; `0` entry not inserted by verified loop |
| `DAT_008B3F38 + n*0x0C` | start reservation owner for numbered item `n`; `-1` available, `-2` disabled/out of map count, `0..7` owning row index | `FUN_004E4F50`, `FUN_004E4FC0`, `FUN_004E50C0`, `FUN_004E5700` | Yes |
| `DAT_008B3FC0` team string table | team combo strings `A`, `B`, `C`, `D`; item data `0..3` | `FUN_004E5AC0`, `FUN_004E5B60` | Yes for team controls, not start controls |
| `DAT_008B3FC8 + n*0x0C` | team reservation owner; written by `FUN_004E5ED0` | `FUN_004E5ED0` | Yes for team controls |
| `DAT_00A8B2DC[i]` | AI slot start location written on Start Game | `FUN_006ACEE0` final apply loop calls start getter `FUN_004E5900` then writes this array; corrected by `SKIRMISH_START_TEAM_CONTROL_DESTINATION_NAMING_GHIDRA_REPORT.md` | Yes |
| `DAT_00A8B39C` and `NodeNameTag+0x5B` | local/human start location before launch | `FUN_006ACEE0` writes global then new node field `+0x5B`; corrected by `SKIRMISH_START_TEAM_CONTROL_DESTINATION_NAMING_GHIDRA_REPORT.md` | Yes |

## 3. Core Logic

### Start string table initialization

`FUN_004E4F50` initializes the start table with display pointers in descending address order that correspond, per the prior string-cluster report, to visible strings `1`, `2`, `3`, `4`, `5`, `6`, `7`, `8`, then `0`.

Tiny detail: the normal population loop in `FUN_004E50C0` iterates only while the owner-field pointer is `< 0x008B3F98`, so it inserts entries `0..7` (`1..8`) and does not insert the ninth initialized string (`0`). Active in YR: Yes, for offline Skirmish start controls; evidence `0x004E51CE` loop base and `0x004E521E` upper bound.

### Map-count limiting

`FUN_004E4FC0(count)` marks start entries before `count` as `-1` and entries from `count` through the table tail as `-2`. In `FUN_006AE6E0`, after the selected map and selected mode/category state are loaded, `count` is clamped against `ScenarioClass+0x11E4` when that value is smaller, then passed to `FUN_004E4FC0`.

Effect: maps with fewer start positions keep the extra numbered strings out of the available list because `FUN_004E50C0` only inserts owner `-1` or the control's own row index. Active in YR: Yes; evidence `0x006AEBF9`-`0x006AEC0B`, `FUN_004E4FC0`.

### Offline start combo population

`FUN_004E50C0(hwnd, control_id)` performs this exact sequence:

1. Gets the control and hides it while rebuilding.
2. Clears the combo with message `0x14B`.
3. Sets owner-draw flags/heights via `0x4DD` and `0x4DE`.
4. Adds `GUI:RandomAsSymbols` first and sets item data to `-2`.
5. Maps control ID to row index: `0x6A3..0x6A8,0x6AA,0x6AB` => `0..7`.
6. Iterates start table entries for visible starts `1..8`.
7. Inserts an entry when the owner is `-1` or already equals this row index.
8. Sets inserted numbered item data to table index `0..7`.
9. Selects the item whose owner equals this row, otherwise keeps selection index `0` (`Random`).
10. Enables the combo with `0x4F1`.
11. Restores visibility only if it was visible before rebuild.

Active in YR: Yes for standard offline Skirmish; evidence `FUN_006AE6E0` calls `FUN_004E5310`, which calls `FUN_004E50C0` when `g_GameMode` is not `3` or `4`.

### Start-combo change handling

`FUN_006ACEE0` routes `0x6A3`, `0x6A4`, `0x6A5`, `0x6A6`, `0x6A7`, `0x6A8`, `0x6AA`, `0x6AB` to `FUN_004E5700` only when `param_4 == 1` (combo selection-change notification).

`FUN_004E5700`:

1. Computes this control's row index.
2. Finds any start table entry currently owned by that row and resets it to `-1`.
3. Reads current selection via `0x147`, then item data via `0x150`.
4. If item data is not `-2`, writes the row index into `DAT_008B3F38 + item_data*0x0C`.
5. Rebuilds every start combo using `FUN_004E5310`, preserving unique numbered-start reservations across rows.

Active in YR: Yes; evidence `FUN_006ACEE0` switch cases and `FUN_004E5700`.

### Team combo and A-D / None ambiguity

`FUN_004E5AC0` initializes team strings `A`, `B`, `C`, `D` from `LETTER_A` through `LETTER_D`, in display order A, B, C, D, and also writes an unused trailing `0` pointer. `FUN_004E5B60` adds optional `GUI:NoneAsSymbols` first with item data `-2`, then adds A-D with item data `0..3`.

Active in YR: Yes for the adjacent team controls `0x76D`-`0x774`; not active as start-position entries. Evidence `FUN_006040B0` maps `0x6A3`-`0x6AB` to `STT:HostComboStart` and `0x76D`-`0x774` to `STT:HostComboTeam`; `FUN_004E5940` maps indices to `0x76D`-`0x774`; `FUN_004E5B60` populates that control family.

## 4. INI Keys

No INI keys directly populate these shell combo entries. The list is binary/string-table driven and limited by the selected map start-count path plus selected mode/category gates. Active in YR: Yes; evidence no related key reads in `FUN_006AE6E0`, `FUN_004E4F50`, `FUN_004E50C0`, `FUN_004E5700`, `FUN_004E5B60`, or `FUN_006ACEE0`.

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Init path | `FUN_006AE6E0` initializes AI-player combos, country/color, start table, team table, then repopulates after map load | `0x006AE870`-`0x006AEC19` | Yes |
| Start tooltip/control family | Skirmish dialog `0x102` start controls are `0x6A3`-`0x6AB` | `FUN_006040B0` | Yes |
| Selection-change path | start controls call `FUN_004E5700` only on notification `param_4 == 1` | `FUN_006ACEE0` | Yes |
| Launch/apply path | Start Game `0x617` reads active rows and writes start values to AI array/local node | `FUN_006ACEE0` | Yes |
| Network/observer path | in `g_GameMode == 3 || 4`, certain local/observer rows use disabled one-item start/team controls | `FUN_004E5310`, `FUN_004E5260`, `FUN_004E5D60`, `FUN_004E5CB0` | Conditional; not standard offline Skirmish |

## 6. Current Rust Implementation Status

Rust has a single `StartPosition` setting with `Auto` and `Position(u8)`, plus shell state fields for player/opponent starts. It does not yet model the original binary's global per-row reservation table that removes already-chosen numbered starts from other rows.

Evidence: `src/ui/main_menu.rs:109`, `src/ui/main_menu.rs:139`, `src/ui/skirmish_shell/state.rs:30`, `src/ui/skirmish_shell/state.rs:39`, `src/app_skirmish.rs:45`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_006AE6E0` init around start/team calls | verified | `0x006AE870`-`0x006AEC19` | none for combo population |
| `FUN_006ACEE0` start-control switch and final writes | verified | `0x006ACEE0`, `0x006AD207`-`0x006AD225`, final apply loop | none for start writes |
| `FUN_004E4F50` start string table init | verified | `0x004E4F50` | one-char values rely on prior string-cluster report |
| `FUN_004E4FC0` start-count availability marking | verified | `FUN_004E4FC0`, `0x006AEC0B` caller | exact source method returning initial count not expanded |
| `FUN_004E50C0` standard start combo populate | verified | `0x004E50C0`, `0x004E51CE`-`0x004E5228` | none |
| `FUN_004E5700` start change handler | verified | `FUN_004E5700` | none |
| `FUN_004E5260` disabled one-item random start combo | verified | `FUN_004E5260`, xref to `GUI:RandomAsSymbols` | conditional network/observer only |
| `FUN_004E5AC0` A-D team table init | verified | `FUN_004E5AC0`, xrefs to `LETTER_A`-`LETTER_D` | none for ambiguity |
| `FUN_004E5B60` standard team combo populate | verified | `FUN_004E5B60`, xref to `GUI:NoneAsSymbols`; condition resolved by `SKIRMISH_TEAM_NONE_INSERTION_VTABLE_0X2C_GHIDRA_REPORT.md` | none for offline Team None condition |
| `FUN_004E5D60` / `FUN_004E5CB0` disabled team path | verified | `FUN_004E5D60`, `FUN_004E5CB0` | conditional network/observer only |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Does the offline start combo include A-D? No. A-D is populated by `FUN_004E5B60` for team controls `0x76D`-`0x774`, not start controls `0x6A3`-`0x6AB` (evidence: `FUN_004E5AC0`, `FUN_004E5B60`, `FUN_004E5940`, `FUN_006040B0`).

[RESOLVED] OQ-2 - Does the offline start combo include None? No for the verified standard start populate helper. `None` is inserted by team helper `FUN_004E5B60` and disabled-team helper `FUN_004E5CB0`; disabled start helper `FUN_004E5260` inserts only Random (evidence: xrefs from `GUI:NoneAsSymbols` at `0x00822BF8`; `FUN_004E5260`).

[RESOLVED] OQ-3 - Does the offline start combo include `0`? No in the verified standard population loop. `FUN_004E4F50` initializes a trailing `0` string pointer, but `FUN_004E50C0` stops before that entry (evidence: `0x004E4FA0`, `0x004E51CE`, `0x004E521E`).

[RESOLVED] OQ-4 - What item data values are attached? Random/disabled sentinel uses `-2`; numbered start entries use `0..7`; team None uses `-2`; team A-D use `0..3` (evidence: `FUN_004E50C0`, `FUN_004E5260`, `FUN_004E5B60`, `FUN_004E5CB0`).

[RESOLVED] OQ-5 - What writes happen if the start combo changes? Selection-change notification `param_4 == 1` updates only the reservation table and rebuilds all start combos; final Start Game reads item data and writes AI starts/local start globals and node field (evidence: `FUN_004E5700`, `FUN_006ACEE0`).

[RESOLVED] OQ-6 - Exact selected-mode vtable method semantics for when team `None` appears. Team `None` is inserted by the team combo helper, not the start combo; the selected `MPModes` mode object's vtable `+0x2C` returns `-2` when `MustAlly` is false and `0` when true. Evidence: `SKIRMISH_TEAM_NONE_INSERTION_VTABLE_0X2C_GHIDRA_REPORT.md`.

## Sources

- Ghidra decompiled/rechecked: `FUN_006AE6E0`, `FUN_006ACEE0`, `FUN_006040B0`, `FUN_004E4F30`, `FUN_004E4F50`, `FUN_004E4FC0`, `FUN_004E50C0`, `FUN_004E5260`, `FUN_004E5310`, `FUN_004E5480`, `FUN_004E5700`, `FUN_004E5900`, `FUN_004E5940`, `FUN_004E5AC0`, `FUN_004E5B60`, `FUN_004E5CB0`, `FUN_004E5D60`, `FUN_004E5E20`, `FUN_004E5ED0`, `FUN_004E6030`.
- String evidence: `GUI:RandomAsSymbols` at `0x00822B7C`, `STT:HostComboStart` at `0x00822B90`, `LETTER_A`-`LETTER_D` at `0x00822BEC`..`0x00822BC8`, `GUI:NoneAsSymbols` at `0x00822BF8`.
- Prior doc used: `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_START_POSITION_UX_GHIDRA_REPORT.md`.
- Rust status scan: `src/ui/main_menu.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish.rs`.
