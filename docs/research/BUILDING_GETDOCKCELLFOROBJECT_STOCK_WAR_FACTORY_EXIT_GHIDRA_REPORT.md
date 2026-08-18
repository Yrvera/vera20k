# Building GetDockCellForObject Stock War Factory Exit - Ghidra Research Report

**Address(es):** `0x0044EFB0` (`BuildingClass::GetDockCellForObject`), `0x00443C60` (`BuildingClass::ExitObject_Main`), `0x0044F640` (`BuildingClass::GetExitCoord`), `0x0044D880` (war-factory mission slot 26)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** stock land war factories only: `GAWEAP`, `NAWEAP`, `YAWEAP`; branch choice, blocked-cell behavior, and whether `QueueingCell` / `DockingOffset` affect land war-factory production exits.  
**Non-Scope:** naval yards, refineries, barracks, hospitals/armories, aircraft pads, full factory queue restart/refund timing, and full 5x3 foundation table byte recovery.  
**Confidence:** High for branch choice and negative facts; Medium for exact stock 5x3 `ExitList` table contents because static table bytes were not recovered in this slice.  
**Active in YR:** Yes. The stock `rulesmd.ini` and `artmd.ini` entries set `WeaponsFactory=yes`, `Factory=UnitType`, `Foundation=5x3`, `ExitCoord=512,256,0`, and no `Naval=yes` on `GAWEAP`, `NAWEAP`, or `YAWEAP`.

## 1. Overview

The main finding is negative but implementation-critical: stock land war-factory production does not use `BuildingClass::GetDockCellForObject` to choose the spawned vehicle cell. `ExitObject_Main` branches away from `GetDockCellForObject` when `WeaponsFactory=yes` and `Naval=no`, then uses `GetExitCoord` to unlimbo the unit at `ExitCoord=512,256,0`, i.e. building NW + `(2,1)` for all three stock land war factories.

`GetDockCellForObject` remains active for other building exit/dock consumers and for naval weapons factories. In the stock land war-factory flow, the 5x3 foundation `ExitList` is still read by the later war-factory mission routine for bib clearing at `ExitList[10]`, but that is not the initial production spawn-cell oracle.

## 2. Class Layout / Key Offsets

| Offset | Type | Meaning in this slice | Evidence |
|---|---:|---|---|
| `BuildingClass+0x520` (`param_1[0x148]`) | ptr | `BuildingTypeClass*` | `0x0044EFB0`, `0x00443C60` |
| `BuildingClass+0x9C/0xA0/0xA4` (`param_1[0x27..0x29]`) | int | building world coord components; `GetExitCoord` adds `ExitCoord` to these | `0x0044F640` |
| `BuildingTypeClass+0xCCE` | bool | `Naval=yes` branch gate | `0x0044EFB0`, `0x00443C60`; stock INI lacks `Naval=yes` for `GAWEAP`/`NAWEAP`/`YAWEAP` |
| `BuildingTypeClass+0xEC8/+0xECC/+0xED0` | int,int,int | `ExitCoord=X,Y,Z` lepton offsets | `0x0044F640`; `rulesmd.ini` stock WFs |
| `BuildingTypeClass+0xED4` | `short*` | foundation `ExitList` pointer, set from `0x0089D368 + foundation_id * 120` | `0x00461563..0x0046156A` |
| `BuildingTypeClass+0xEF0` | int | foundation id (`5x3` = 17) | `0x0045EC90`, `0x0045ECA0`; foundation docs |
| `BuildingTypeClass+0x16BD` | bool | `WeaponsFactory=yes` branch gate | `0x0044EFB0`, `0x00443C60`; stock INI |
| `BuildingTypeClass+0x16C1` | bool | `Hospital=yes`, forces perimeter scan in `GetDockCellForObject` | `0x0044EFB0`; stock WFs false |
| `BuildingTypeClass+0x1618/+0x161C` | int,int | `QueueingCell`, not read by `GetDockCellForObject` or stock WF exit | absence in `0x0044EFB0`; parser docs |
| `BuildingTypeClass+0x1788` | ptr | `DockingOffset%d` array, not read by `GetDockCellForObject` or stock WF exit | absence in `0x0044EFB0`; parser docs |

## 3. Core Logic

### 3.1 `GetDockCellForObject @ 0x0044EFB0`

Branch priority verified from decompilation:

1. `GDIBarracks`: test `NW+(1,2)` with `CanEnterCell(..., force=1)`.
2. `NODBarracks`: test `NW+(2,2)` with `force=1`.
3. `YuriBarracks`: test `NW+(2,1)` with `force=1`.
4. `Naval && WeaponsFactory`: compute from `GetDockCoord`, then test exactly three cells with `force=0`: `(dock.x+1,dock.y+1)`, `(dock.x+1,dock.y)`, `(dock.x,dock.y+1)`.
5. If caller supplied a non-invalid fallback cell, test that cell with `force=0`.
6. If `Type+0xED4 == null` or `Hospital=yes`, scan the foundation perimeter with `force=1`.
7. Else iterate `Type+0xED4` as `{short dx, short dy}` pairs in table order until sentinel `(0x7FFF,0x7FFF)`, testing `NW+(dx,dy)` with `force=0`.
8. If all candidates fail or are out of bounds, return `InvalidCell` (`DAT_0089C818`).

Important tiny details:

- Success is `CanEnterCell(...) == 0`; non-zero values are skipped.
- Out-of-bounds cells are skipped, not returned.
- `ExitList` iteration increments by two shorts per candidate (`psVar += 2`), so entry 10 is byte offset `+0x28`.
- The `Hospital=yes` branch ignores a non-null `ExitList` and uses the perimeter scan instead.
- `GetDockCellForObject` contains no read of `QueueingCell` (`+0x1618/+0x161C`) or `DockingOffset` (`+0x1788`).

### 3.2 Stock land war-factory branch in `ExitObject_Main @ 0x00443C60`

For `WeaponsFactory=yes` and `Naval=no`, `ExitObject_Main` does not take the standard vehicle/barracks path that calls vtable slot `+0x4D4` (`GetDockCellForObject`). It branches to the war-factory path:

1. Clears/stores the building rally/ghost-cell state.
2. Checks for sibling same-type war factories if the current factory is already in the unload mission.
3. Calls vtable slot `+0xB4`, `GetExitCoord @ 0x0044F640`.
4. Calls the produced unit's `Unlimbo` at that lepton coord with facing byte `0x40`.
5. If `Unlimbo` fails, decrements `g_MapEditorMode` and returns `0`.
6. On success, temporarily marks/unmarks, sets the unit location from `GetExitCoord`, establishes radio commands `2` and `0x18`, and queues building mission `0x10` to start the door/drive-out state machine.

### 3.3 `GetExitCoord @ 0x0044F640`

If `ExitCoord` is invalid, `GetExitCoord` returns the building center. Stock land war factories have valid `ExitCoord=512,256,0`, so the output is:

```
coord.x = building.coord.x + 512
coord.y = building.coord.y + 256
coord.z = building.coord.z + 0
```

Converted to cells, that is `NW+(2,1)` because one cell is 256 leptons and the lepton coord remains cell-centered by the standard `+128` convention downstream.

### 3.4 War-factory mission slot 26 @ `0x0044D880`

The mission routine still reads the 5x3 `ExitList`, but not for the initial spawn cell. In the `WeaponsFactory=yes` / `Naval=no` setup before the state switch, it reads `*(uint *)(Type+0xED4 + 0x28)` and combines it with the building NW cell. It then stores a lepton coord using `(entry10.x - 1, entry10.y)` as the bib/drive-out target used by the later door/drive-out logic.

This verifies the stale-doc distinction: `ExitList[10]` is relevant to bib clearing / drive-out target behavior, but `GetDockCellForObject` is not the stock land war-factory production spawn-cell selector.

## 4. INI Keys

| Key | Stock value / location | Effect in this slice | Active in YR |
|---|---|---|---|
| `Foundation=5x3` | `artmd.ini` `[GAWEAP]`, `[NAWEAP]`, `[YAWEAP]` | selects foundation id 17 and therefore `Type+0xED4 = 0x0089D368 + 17*120` | Yes |
| `WeaponsFactory=yes` | `rulesmd.ini` stock WFs | diverts `ExitObject_Main` into war-factory path; also enables naval branch inside `GetDockCellForObject` only if `Naval=yes` | Yes |
| `Factory=UnitType` | `rulesmd.ini` stock WFs | production category matching; not an exit-cell coordinate source | Yes |
| `ExitCoord=512,256,0` | `rulesmd.ini` stock WFs | actual stock land WF spawn/unlimbo coordinate source | Yes |
| `Naval=yes` | absent on `GAWEAP`/`NAWEAP`/`YAWEAP` | excludes the `Naval && WeaponsFactory` branch in `GetDockCellForObject` | No for stock land WFs |
| `QueueingCell=` | absent on stock WFs | not read by `GetDockCellForObject` or stock WF exit | No effect here |
| `DockingOffset%d=` | absent on stock WFs | not read by `GetDockCellForObject` or stock WF exit | No effect here |
| `NumberImpassableRows=1` | `rulesmd.ini` stock WFs | affects `CanEnterCell`/contact behavior, not the selected initial spawn coordinate | Yes, but outside this helper |

## 5. Integration Points

| Function | Relationship |
|---|---|
| `BuildingClass::ExitObject_Main @ 0x00443C60` | For non-WF standard exits, calls `GetDockCellForObject`; for stock land WFs, bypasses it and calls `GetExitCoord` |
| `BuildingClass::GetDockCellForObject @ 0x0044EFB0` | Active helper for other exits; stock land WF production does not call it |
| `BuildingClass::GetExitCoord @ 0x0044F640` | Actual stock land WF initial unlimbo coordinate source |
| `FUN_0044D880 @ 0x0044D880` | War-factory mission state machine; uses `ExitList[10]` for bib/drive-out target setup |
| `BuildingClass::ClearBibArea @ 0x00449540` | Uses the same `ExitList+0x28` convention for bib clearing |
| `UnitClass::CanEnterCell @ 0x0073F0A0` | Called by `GetDockCellForObject`; for stock WF initial `ExitCoord`, `Unlimbo` passability is the relevant gate instead |

## 6. Current Rust Implementation Status

Scanned surfaces:

- `src/sim/production/production_spawn.rs` currently selects vehicle spawn cells with `preferred_exit_offsets`, converting `ExitCoord` to `(2,1)` and then probing adjacent fallback cells before a generic nearest-walkable fallback.
- `src/sim/production/production_placement_tests.rs` has tests such as `spawn_routing_falls_back_to_next_factory_when_first_exit_is_blocked` and `spawn_routing_prefers_active_producer_when_available`.
- `src/rules/object_type.rs` parses `ExitCoord`.
- `src/rules/foundation.rs` has foundation id 17 as `5x3`.
- `src/rules/art_data.rs` parses `QueueingCell` and `DockingOffset`, but stock WFs do not need either for production exit.

Rust delta from this slice:

- The primary `(NW+2,NW+1)` stock WF cell from `ExitCoord=512,256,0` matches.
- The adjacent fallback search and generic nearest-walkable fallback do not match the verified stock land WF initial exit behavior. gamemd's stock land WF path attempts `Unlimbo` at `GetExitCoord`; if that fails, `ExitObject_Main` returns `0` from this path. It does not ask `GetDockCellForObject` for another cell.
- Rust should not use `QueueingCell`, `DockingOffset`, or a foundation-perimeter/ExitList scan for stock land WF initial production spawn.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `GetDockCellForObject @ 0x0044EFB0` branch order | verified | live decompile `0x0044EFB0` | none |
| Stock land WF branch choice in `ExitObject_Main` | verified | live decompile `0x00443C60` | none |
| `GetExitCoord @ 0x0044F640` formula | verified | live decompile `0x0044F640`; `rulesmd.ini` | none |
| Naval `WeaponsFactory` branch exclusion for stock WFs | verified | `0x0044EFB0`, `0x00443C60`; stock WFs lack `Naval=yes` | none |
| `QueueingCell` / `DockingOffset` negative fact | verified | no reads in `0x0044EFB0`, `0x00443C60` WF path, or `0x0044F640`; known parser offsets only | none |
| Stock `Foundation=5x3` id and dimensions | verified | `artmd.ini`; `FOUNDATION_PARSER_TABLE_BRACKET_EXTENTS_GHIDRA_REPORT.md`; `0x0045EC90/0x0045ECA0` | none |
| `Type+0xED4` assignment from foundation id | verified | assembly `0x00461547..0x0046156A` | none |
| Exact 5x3 `ExitList` entry sequence | touched-not-exhausted | pointer formula verified; table raw bytes not recovered into meaningful pairs in this slice | recover runtime-populated table or a trusted table dump |
| `ExitList[10]` use in WF mission/bib | verified | `0x0044D880`; prior `RALLY_POINTS_AND_UNIT_SPAWNING.md` and audit log | exact entry value remains tied to table recovery |
| House/strip refund and queue restart after blocked WF exit | deferred | out-of-scope for slot 4; parent slots 3/5 cover it | use those slot reports |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Is `GetDockCellForObject` called for stock land WF production exit? -> No. `ExitObject_Main` bypasses vtable `+0x4D4` when `WeaponsFactory=yes` and `Naval=no`, and calls `GetExitCoord` instead.` (evidence: `0x00443C60`, `0x0044F640`)
- `[RESOLVED] OQ-2 - What cell do stock WFs use for initial unlimbo? -> `ExitCoord=512,256,0`, i.e. `NW+(2,1)` for GAWEAP/NAWEAP/YAWEAP.` (evidence: `0x0044F640`, `rulesmd.ini` stock sections)
- `[RESOLVED] OQ-3 - Does the stock land WF path use the naval branch? -> No; the naval branch requires `Type+0xCCE != 0`, absent on GAWEAP/NAWEAP/YAWEAP.` (evidence: `0x0044EFB0`, `0x00443C60`, `rulesmd.ini`)
- `[RESOLVED] OQ-4 - Does `GetDockCellForObject` use `QueueingCell`? -> No reads of `+0x1618/+0x161C` in the decompiled helper or stock WF branch.` (evidence: `0x0044EFB0`, `0x00443C60`)
- `[RESOLVED] OQ-5 - Does `GetDockCellForObject` use `DockingOffset%d`? -> No reads of `+0x1788`; `DockingOffset` is a separate dock-coordinate system.` (evidence: `0x0044EFB0`; `REFINERY_DOCK_CELL_AND_ANIM_HELPERS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-6 - What happens when a candidate in `GetDockCellForObject` is blocked? -> Non-zero `CanEnterCell` result skips that candidate; if every candidate fails, returns `InvalidCell`.` (evidence: `0x0044EFB0`)
- `[RESOLVED] OQ-7 - What happens when the stock WF `ExitCoord` cell is blocked? -> `Unlimbo` failure makes `ExitObject_Main` return `0`; this is outside `GetDockCellForObject`.` (evidence: `0x00443C60`)
- `[RESOLVED] OQ-8 - Is the foundation perimeter scan relevant to stock WFs? -> Not for stock land WF production exit; it is only the `GetDockCellForObject` fallback when no `ExitList` or `Hospital=yes`.` (evidence: `0x0044EFB0`, `0x00443C60`)
- `[RESOLVED] OQ-9 - Is `ExitList` still used anywhere in the stock WF flow? -> Yes, mission slot 26 reads `Type+0xED4+0x28` (`ExitList[10]`) for bib/drive-out setup; not for initial spawn selection.` (evidence: `0x0044D880`)
- `[RESOLVED] OQ-10 - How is `Type+0xED4` assigned? -> From foundation id using `0x0089D368 + id * 120`; `5x3` is id 17.` (evidence: `0x00461547..0x0046156A`; foundation parser docs)
- `[RESOLVED] OQ-11 - Are stock WFs 5x3 in YR? -> Yes, `artmd.ini` sets `Foundation=5x3` for GAWEAP/NAWEAP/YAWEAP.` (evidence: `artmd.ini`)
- `[RESOLVED] OQ-12 - Are stock WFs `Factory=UnitType` and `WeaponsFactory=yes`? -> Yes for GAWEAP/NAWEAP/YAWEAP.` (evidence: `rulesmd.ini`)
- `[DEFERRED] OQ-13 - What are the exact runtime pairs in the 5x3 `ExitList` table?` (category: `bounded-cost-too-high`; reason: the pointer formula is verified, but the raw PE bytes at `0x0089D368 + 17*120` did not decode to meaningful `short` pairs in this static slice; next-step-if-pursued: inspect the runtime-populated Ghidra memory table or add a narrow table-dump investigation)
- `[DEFERRED] OQ-14 - What exact House/Strip queue restart happens after blocked stock WF exit?` (category: `out-of-scope`; reason: assigned to parent swarm slots 3 and 5; next-step-if-pursued: use their `Place_Production` / `StripClass::AI` reports)
- `[DEFERRED] OQ-15 - What unit-side event clears the radio tether after drive-out?` (category: `out-of-scope`; reason: drive-out completion is beyond stock exit-cell selection; next-step-if-pursued: trace `DriveLocomotion` / unit radio `8` or `0x19` sender)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Stock land WFs use `GetExitCoord`, not `GetDockCellForObject`, for initial vehicle unlimbo | `0x00443C60`, `0x0044F640`; stock INI `ExitCoord=512,256,0` | partial mismatch: Rust starts from `ExitCoord` but falls back around it | `src/sim/production/production_spawn.rs::find_spawn_cell_near_structure` | For `WeaponsFactory=yes && !Naval`, attempt the `ExitCoord` cell as the stock spawn cell and do not route through generic dock/perimeter fallback for stock parity | `war_factory_spawn_uses_exitcoord_cell_without_getdock_fallback` | Do not implement stock WF spawn as nearest-passable around the foundation |
| If stock WF `ExitCoord` unlimbo cell is blocked, this path fails instead of choosing another candidate | `0x00443C60` after failed `Unlimbo` returns `0` | mismatch: current Rust can probe neighbors and later factories | `src/sim/production/production_spawn.rs`; `src/sim/production/production_placement_tests.rs` | Blocked primary WF exit should report failure to the caller; queue/refund/retry behavior belongs to production delivery logic, not cell search | `war_factory_blocked_exit_returns_none_without_neighbor_probe` | Do not silently place the unit on adjacent cells; that changes visible spawn position |
| `QueueingCell` is not a stock WF production exit input | absence in `0x0044EFB0`, `0x00443C60`, `0x0044F640`; `artmd.ini` stock WFs omit key | none observed for WF; parser exists globally | `src/rules/art_data.rs`, `src/rules/object_type.rs`, production spawn call sites | Keep `QueueingCell` scoped to harvester/refinery behavior | `war_factory_spawn_ignores_queueing_cell_even_if_modded_art_sets_it` | Do not borrow refinery queue logic for factories |
| `DockingOffset%d` is not a stock WF production exit input | absence in `0x0044EFB0`, `0x00443C60`, `0x0044F640`; known `+0x1788` parser path | none observed for WF; parser exists globally | `src/rules/art_data.rs`, `src/sim/docking/*`, production spawn call sites | Keep `DockingOffset` for multi-pad dock/airfield/service-depot style logic, not land WF vehicle spawn | `war_factory_spawn_ignores_docking_offset_entries` | Do not treat `DockingOffset0` as an override for `ExitCoord` |
| `GetDockCellForObject` itself tests candidates in fixed branch order and returns `InvalidCell` if all fail | `0x0044EFB0` | unchecked for a future generic helper | possible future `src/sim/production` helper if non-WF exits are ported | If implementing this helper for non-WF exits, preserve branch priority and `CanEnterCell==0` success semantics | `getdockcell_exitlist_all_blocked_returns_invalid_cell` | Do not collapse `force=0` and `force=1`; they are separate passability contracts |
| Stock WF mission/bib logic reads `ExitList[10]`, not `ExitList[0]`, after spawn | `0x0044D880`; `RALLY_POINTS_AND_UNIT_SPAWNING.md` | likely missing/unchecked | future WF door/bib-clear implementation surfaces | Use the 5x3 foundation `ExitList[10]` convention for bib clearing/drive-out target, separate from initial spawn | `war_factory_bib_clear_uses_exitlist_entry_10` | Do not confuse bib clearing target with initial unlimbo cell |

Stale Docs / Follow-up Docs:

- Replace any wording that says stock land war-factory vehicle spawn uses `GetDockCellForObject` with: "Stock land war factories use `GetExitCoord` (`ExitCoord=512,256,0`) for initial unlimbo; `GetDockCellForObject` is bypassed in the non-naval WF branch."
- Replace any wording that says `QueueingCell` or `DockingOffset` controls war-factory production exit with: "Neither field is read by `GetDockCellForObject` or the stock land WF `ExitObject_Main` path; `QueueingCell` is refinery/harvester-facing, and `DockingOffset%d` is dock-coordinate/pad-facing."
- Preserve the corrected wording that war-factory bib clearing uses `ExitList[10]`, not `ExitList[0]`.

## Sources

- Ghidra live decompile: `0x0044EFB0` `BuildingClass::GetDockCellForObject`
- Ghidra live decompile: `0x00443C60` `BuildingClass::ExitObject_Main`
- Ghidra live decompile: `0x0044F640` `BuildingClass::GetExitCoord`
- Ghidra live decompile: `0x0044D880` war-factory mission slot 26
- Ghidra live assembly context: `0x00461547..0x0046156A` `Type+0xED4` assignment
- Ghidra live decompile: `0x0045EC90`, `0x0045ECA0` foundation width/height helpers
- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`
- `C:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini`
- `C:/Users/enok/Documents/ra2-rust-game-docs/FIND_NEAREST_VARIANTS_SPIRAL_COMPARISON_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/RALLY_POINTS_AND_UNIT_SPAWNING.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BUILDING_FOUNDATION_OCCUPY_MODIFIERS_PARITY_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/REFINERY_DOCK_CELL_AND_ANIM_HELPERS_GHIDRA_REPORT.md`
