# Skirmish Start/Team Control Destination Naming - Ghidra Research Report

**Address(es):** `0x006ACEE0`, `0x004E50C0`, `0x004E5700`, `0x004E5940`, `0x004E5B60`, `0x004E5900`, `0x004E6030`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Standard offline Skirmish dialog `0x102` naming and final write destinations for start-position controls versus team controls.
**Non-Scope:** Gameplay spawn assignment after shell exit, network/WOL lobby variants except where helper gates are visible, and exact string resource text.
**Confidence:** High
**Active in YR:** Yes, for standard offline Skirmish Start Game flow.

## 1. Overview

This slice resolves the start/team naming contradiction in the recent Skirmish reports. The start-position control family is `0x6A3..0x6AB`, and the team control family is `0x76D..0x774`.

On Start Game, `FUN_006ACEE0` writes selected start item data to `DAT_00A8B2DC[slot]` for AI rows and `DAT_00A8B39C` for the local row. It writes selected team item data to `DAT_00A8B2FC[slot]` for AI rows and `DAT_00A8B3A4` for the local row.

## 2. Key Controls And Globals

| ID / global | Meaning | Evidence | Active in YR? |
|-------------|---------|----------|---------------|
| `0x6A3` | local start-position combo | `FUN_004E4E60(0)`, `FUN_004E50C0`, `FUN_004E5700`, final read at `0x006AD60C..0x006AD617` | Yes |
| `0x6A4..0x6AB` | AI start-position combos for rows 1..7 | `FUN_004E4E60(1..7)`, `FUN_004E50C0`, `FUN_004E5700`, AI read at `0x006AD4C7..0x006AD4D4` | Yes |
| `0x76D` | local team combo | `FUN_004E5940(0)`, `FUN_004E5B60`, final read at `0x006AD61C..0x006AD627` | Yes |
| `0x76E..0x774` | AI team combos for rows 1..7 | `FUN_004E5940(1..7)`, `FUN_004E5B60`, AI read at `0x006AD4DB..0x006AD4E6` | Yes |
| `DAT_00A8B2DC[slot]` | AI selected start item data | `0x006AD4CD` calls `FUN_004E5900`, `0x006AD4D4` stores result | Yes |
| `DAT_00A8B2FC[slot]` | AI selected team item data | `0x006AD4E1` calls `FUN_004E6030`, `0x006AD4E6` stores result | Yes |
| `DAT_00A8B39C` | local selected start item data | `0x006AD60C..0x006AD617`, store at `0x006AD63B` | Yes |
| `DAT_00A8B3A4` | local selected team item data | `0x006AD61C..0x006AD627`, store at `0x006AD641` | Yes |
| new node `+0x5B` | copied local start value | `FUN_006ACEE0` copies `DAT_00A8B39C` into allocated node | Yes |
| new node `+0x63` | copied local team value | `FUN_006ACEE0` copies `DAT_00A8B3A4` into allocated node | Yes |

## 3. Core Logic

### Start Control Family

`FUN_004E4E60(index)` maps row indices to start-position combo IDs:

```text
0 -> 0x6A3
1 -> 0x6A4
2 -> 0x6A5
3 -> 0x6A6
4 -> 0x6A7
5 -> 0x6A8
6 -> 0x6AA
7 -> 0x6AB
```

`FUN_004E50C0(hwnd, control)` populates those controls. It inserts `Random` with item data `-2`, then numbered start entries whose item data is `0..7` when the reservation table says that number is available or already owned by the same row.

Tiny detail: `FUN_004E50C0` maps `0x6A3..0x6AB` back to row index `0..7` before filtering the reservation table. That makes start-number uniqueness a property of the global reservation table, not a per-combo local list.

`FUN_004E5700(hwnd, control)` handles selection changes only for the `0x6A3..0x6AB` family. It clears the old reservation owned by that row, reads the new selected item data, writes the row index into the reservation table unless the item data is `-2`, and rebuilds all eight start controls.

### Team Control Family

`FUN_004E5940(index)` maps row indices to team combo IDs:

```text
0 -> 0x76D
1 -> 0x76E
2 -> 0x76F
3 -> 0x770
4 -> 0x771
5 -> 0x772
6 -> 0x773
7 -> 0x774
```

`FUN_004E5AC0` initializes the team string table with A, B, C, D string pointers. `FUN_004E5B60(hwnd, control)` optionally inserts `None` with item data `-2`, then inserts A-D with item data `0..3`.

Tiny detail: `FUN_004E5B60` inserts `None` only when the selected `MPModes` mode object's vtable `+0x2C` returns a negative value. Follow-up `SKIRMISH_TEAM_NONE_INSERTION_VTABLE_0X2C_GHIDRA_REPORT.md` resolves that method as `MustAlly`: false returns `-2` and inserts `None`; true returns `0` and suppresses it. The item-data contract remains `-2` for no team and `0..3` for A-D.

`FUN_004E5ED0(hwnd, control, value)` is the team selector helper. It writes the owning row into `DAT_008B3FC8 + value*0x0C` when selecting A-D, but does not reserve the `-2` None item.

### Final Start Game Writes

In `FUN_006ACEE0`, the AI active-row packing order is:

```text
row kind  -> DAT_00A8B27C[slot]
country   -> DAT_00A8B29C[slot]
color     -> DAT_00A8B2BC[slot]
start     -> DAT_00A8B2DC[slot]
team      -> DAT_00A8B2FC[slot]
```

The key assembly-level call order in the active AI row block is:

| Address | Operation |
|---------|-----------|
| `0x006AD4C7..0x006AD4CD` | loads start-control ID argument, calls `FUN_004E5900` |
| `0x006AD4D4` | stores returned start item data to `DAT_00A8B2DC[slot]` |
| `0x006AD4DB..0x006AD4E1` | loads team-control ID argument, calls `FUN_004E6030` |
| `0x006AD4E6` | stores returned team item data to `DAT_00A8B2FC[slot]` |

The local row mirrors that same meaning:

| Address | Operation |
|---------|-----------|
| `0x006AD60C..0x006AD617` | reads `0x6A3` via `FUN_004E5900`; returned value later stored as local start |
| `0x006AD61C..0x006AD627` | reads `0x76D` via `FUN_004E6030`; returned value later stored as local team |
| `0x006AD63B` | stores local start to `DAT_00A8B39C` |
| `0x006AD641` | stores local team to `DAT_00A8B3A4` |

## 4. INI Keys

No INI key directly defines these start/team combo IDs, item data, or final destination globals. The list content is binary/string-table driven and constrained by the selected multiplayer mode/category object. Relevant map start capacity comes from scenario waypoints or random-map player count, but that is outside this naming slice.

## 5. Integration Points

`FUN_006ACEE0` is the active command handler for Skirmish dialog `0x102`. It routes `0x6A3..0x6AB` selection-change notifications to `FUN_004E5700`, and on Start Game `0x617` it reads both the start and team families before shell exit.

The older start-position population report is correct about the control families and item data, but wrong in its global destination rows. The newer side/country/team final-writes report is correct for the destination names. The Start Game handoff report has verified write addresses but one prose line swaps the labels for `DAT_00A8B2DC` and `DAT_00A8B2FC`.

## 6. Current Rust Implementation Status

Rust currently has abstract fields for one player start position and per-opponent start/team data in `src/ui/skirmish_shell/state.rs`, but the launch handoff only exports the player start position and one AI country. It does not yet model the stock eight-row reservation table, team A-D/None item-data contract, or the separate destination families verified here.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|--------------------------|--------|----------|--------------|
| `FUN_006ACEE0` Start Game final writes | verified | decompile plus assembly context at `0x006AD4C7..0x006AD4E6`, `0x006AD60C..0x006AD641` | none for naming |
| `FUN_004E4E60` start control mapper | verified | decompile: maps `0..7` to `0x6A3..0x6AB` | none |
| `FUN_004E50C0` start population | verified | decompile: Random `-2`, numbered starts `0..7`, start reservation table | none for naming |
| `FUN_004E5700` start change handler | verified | decompile: only `0x6A3..0x6AB` reservation updates | none |
| `FUN_004E5940` team control mapper | verified | decompile: maps `0..7` to `0x76D..0x774` | none |
| `FUN_004E5AC0` team string table init | verified | decompile: A-D strings | none |
| `FUN_004E5B60` team population | verified | decompile: optional None `-2`, A-D `0..3`; condition resolved by `SKIRMISH_TEAM_NONE_INSERTION_VTABLE_0X2C_GHIDRA_REPORT.md` | none for offline Team None condition |
| `FUN_004E5900` / `FUN_004E6030` getter wrappers | verified | decompile: both read current selection when passed `-1`, then return item data | semantic difference comes from caller-supplied control family |
| Prior doc conflict | conflict-needs-resolution | this report versus stale lines in `SKIRMISH_START_POSITION_COMBO_POPULATION_GHIDRA_REPORT.md` and `SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md` | patch stale prose/tables to reference this report |

## 8. Open Questions - Final State

[RESOLVED] OQ-STN-001 - Which controls are start-position controls? `0x6A3..0x6AB`. Evidence: `FUN_004E4E60`, `FUN_004E50C0`, `FUN_004E5700`, `FUN_006ACEE0` switch.

[RESOLVED] OQ-STN-002 - Which controls are team controls? `0x76D..0x774`. Evidence: `FUN_004E5940`, `FUN_004E5AC0`, `FUN_004E5B60`, final reads in `FUN_006ACEE0`.

[RESOLVED] OQ-STN-003 - Where do AI start values write? `DAT_00A8B2DC[slot]`. Evidence: `0x006AD4C7..0x006AD4D4`.

[RESOLVED] OQ-STN-004 - Where do AI team values write? `DAT_00A8B2FC[slot]`. Evidence: `0x006AD4DB..0x006AD4E6`.

[RESOLVED] OQ-STN-005 - Where do local start/team values write? Start writes `DAT_00A8B39C` and node `+0x5B`; team writes `DAT_00A8B3A4` and node `+0x63`. Evidence: `0x006AD60C..0x006AD641` and node field stores in `FUN_006ACEE0`.

[RESOLVED] OQ-STN-006 - Exact selected-mode vtable method controlling whether team None is inserted. The selected `MPModes` mode object's vtable `+0x2C` method at `0x005D5DC0` reads `MustAlly` at object `+0x3F`: false returns `-2` and inserts Team `None`; true returns `0` and suppresses it. Evidence: `SKIRMISH_TEAM_NONE_INSERTION_VTABLE_0X2C_GHIDRA_REPORT.md`.

## Sources

- Ghidra decompiled: `0x006ACEE0`, `0x004E50C0`, `0x004E5700`, `0x004E5940`, `0x004E5AC0`, `0x004E5B60`, `0x004E5ED0`, `0x004E5900`, `0x004E6030`, `0x004E4E60`, `0x004E4170`, `0x004E5310`, `0x004E5D60`.
- Ghidra assembly context checked: `0x006AD4C7..0x006AD4E6`, `0x006AD60C..0x006AD641`.
- Prior reports checked: `SKIRMISH_START_POSITION_COMBO_POPULATION_GHIDRA_REPORT.md`, `SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`, `SKIRMISH_SIDE_COUNTRY_TEAM_FINAL_WRITES_GHIDRA_REPORT.md`.
- Rust scan: `src/ui/skirmish_shell/state.rs`.
