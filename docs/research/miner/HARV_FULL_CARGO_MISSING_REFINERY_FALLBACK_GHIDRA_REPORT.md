# HARV Full-Cargo Missing-Refinery Fallback - Ghidra Research Report

**Address(es):** `0x0073E5E0`, `0x0073D630`, `0x004DF040`, `0x004593A0`, `0x0047C520`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** stock YR War Miner (`[HARV]`, not CMIN/slave miner) with full cargo when its selected refinery disappears before dock admission, during approach/dock, or during the unit-side DockUnload state-3 unload loop.  
**Non-Scope:** CMIN teleport-specific return, two-miner queue takeover timing, normal empty-cargo state-4 exit, armed-harvester targeting while docked, and exact same-frame runtime ordering between damage/sell command dispatch and the next unit mission tick.  
**Confidence:** High for the full-cargo fallback and unload-missing-building branch; Medium for exact sell/damage same-tick ordering because no runtime debugger trace was captured.  
**Active in YR:** Yes. `[HARV]` has `Harvester=yes`, `Dock=NAREFN,GAREFN`, `Storage=40`, `UnloadingClass=HORV`, and no `Teleporter=yes` in `rulesmd.ini`.

## 0. Working Notes

**Target question:** For a full stock War Miner, if the selected refinery disappears before docking or during unload, does stock YR fall into ore search, keep/refind refinery-return behavior, and what cargo/display/contact state survives?

**Non-goals:** Do not prove CMIN teleport behavior, normal state-4 unload completion, queue promotion between two miners, or Rust implementation correctness beyond naming affected surfaces and tests.

**Evidence needed to mark COMPLETE:** live Ghidra proof of `Mission_Harvest` state-0 full check ordering, state-2 no-refinery fallback, `Find_Docking_Bay` candidate semantics, `Mission_Deploy_Building` state-3 missing-building branch, and interrupt cleanup for linked docked units; INI proof that HARV reaches these active paths; Rust surface scan for current deltas.

**Stop conditions:** no Rust/INI/in-repo doc edits; do not mutate Ghidra; if live binary evidence for a branch cannot be obtained, mark the branch partial. The live Ghidra MCP exposed the needed functions, so the claimed branch is COMPLETE.

## 1. Overview

Stock YR does not let a full War Miner go mine ore just because its selected refinery vanished. `UnitClass::Mission_Harvest` state 0 checks storage percentage before ore scanning and immediately rewrites the harvest substate to return-to-refinery (`+0xBC = 2`) when storage is full.

If the refinery disappears while the miner is already in the unit-side unload loop, `Mission_Deploy_Building` state 3 re-finds the refinery by cell lookup. When `Look_up_building_in_cell()` returns null, it skips all storage removal and credit award, optionally sends radio `3`, and calls `SetMission(0x0A, queued=1)`. The cargo therefore remains on the miner, and the next Harvest pass refalls into the full-cargo return path instead of ore search.

## 2. Class Layout / Key Offsets

| Field / offset | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| Unit `+0xBC` / `param_1[0x2F]` | `Mission_Harvest` / `Mission_Deploy_Building` substate | switch at `0x0073E6D5`; state writes `0x0073E714`, `0x0073EE68`, `0x0073E51C` | Yes |
| Unit `+0x6C4` / `param_1[0x1B1]` | `UnitTypeClass*` | `UnitClass::Mission_Harvest`, `Mission_Deploy_Building` | Yes |
| UnitType `+0xE0E` | `Harvester=yes` | mission gates; `rulesmd.ini:[HARV] Harvester=yes` | Yes for HARV |
| UnitType `+0xCD4` | `Teleporter=yes` branch selector | state-2 `cVar1`/`BL` branch in `Mission_Harvest` | No for stock HARV |
| UnitType `+0x3F8` / `+0x10` on list | dock list count | `Mission_Harvest` precheck; `Find_Docking_Bay` loop | Yes |
| UnitType `+0x800` | storage capacity used by `Get_Storage_Percentage` | prior report; `[HARV] Storage=40` | Yes |
| Unit `+0x169 * 4 = +0x5A4` | current destination/contact-ish pointer tested in state 2 | `0x0073EB5A` and state-2 flow | Yes |
| Unit `+0x33C` | harvester `StorageClass` drained during unload | `Mission_Deploy_Building` state 3 | Yes |
| Unit byte `+0x6D1` | unload-active/first-entry flag | normal unload docs; abort clears current dock FSM in Rust handoff | Yes |
| Unit byte `+0x6D2` | active-harvesting presentation flag | state 0/1 writes in `Mission_Harvest`; cleared before return | Yes |
| Unit/Building `+0x2E4` | reciprocal dock link for interrupt paths, not normal zero-link DockUnload | `0x0073D63B`, `0x004593A0` | Conditional |
| `DAT_0089F6A0/2` | signed `(-1,0)` refinery rediscovery lookup from current dock cell | `0x0073E2D5`, `0x0049F2F0` prior verified | Yes |
| BuildingType `+0x16B3` | `DockUnload=yes` radio handoff | prior radio docs; `rulesmd.ini:[NAREFN]/[GAREFN]` | Yes |
| BuildingType `+0x16BB` | `Refinery=yes` | state-4 and slot-8 checks | Yes |

## 3. Core Logic

### 3.1 State 0 full-cargo check happens before ore search

In `UnitClass::Mission_Harvest @ 0x0073E5E0`, the state switch reads `unit+0xBC` at `0x0073E6D5`. State 0 begins by checking `UnitType+0xE0F` (`Weeder`) and then calls vtable `+0x2B4` (`Get_Storage_Percentage`):

```text
0x0073E6F1  MOV CL, [UnitType+0xE0F]
0x0073E6F7  TEST CL, CL
0x0073E6F9  JNZ 0x0073E72A
0x0073E700  CALL [vtable+0x2B4]
0x0073E706  FCOMP [1.0]
0x0073E711  JNZ 0x0073E72A
0x0073E714  MOV [unit+0xBC], 0x2
0x0073E720  MOV EAX, 1
```

Only if this full-storage branch is not taken does the function continue into archive/ore-scan code. For a full HARV that re-enters Harvest after losing a refinery, the first observable state-machine result is return-to-refinery, not ore scan.

**Active in YR:** Yes. HARV has `Harvester=yes`, `Storage=40`, and `Weeder` is absent/false.

### 3.2 State 2 with no refinery stays in return logic

State 2 calls `Find_Docking_Bay` through vtable `+0x528` with the unit type's dock list:

```text
0x0073EB68  MOV ECX, [unit+0x6C4]
0x0073EB73  ADD ECX, 0x3E8
0x0073EB7E  CALL [vtable+0x528]
0x0073EB8E  TEST ESI, ESI
0x0073EB90  JZ 0x0073EC1F
```

For non-teleporter HARV (`BL == 0` from `UnitType+0xCD4`), if no dock is found in the normal lookup, the function goes to the fog-ignoring/alternate lookup at `0x0073EC1F`. If that also returns null:

```text
0x0073EC41  CALL [vtable+0x528]  ; Find_Docking_Bay(..., arg4=1)
0x0073EC50  TEST ESI, ESI
0x0073EC58  JZ 0x0073EF77       ; timer epilogue
```

There is no transition to state 0 or state 4 on this no-refinery path. The miner remains in substate 2 and retries after the mission timer jitter. If another owned refinery exists, `Find_Docking_Bay` can select it on a later state-2 tick.

**Active in YR:** Yes for HARV return behavior. The teleporter-specific branch is not active for HARV.

### 3.3 Find_Docking_Bay selects among the unit's Dock= list

`FootClass::Find_Docking_Bay @ 0x004DF040` loops over the list at the passed `TypeClass+0x3E8`/`Dock=` data and calls vtable `+0x52C` for each dock type. It retains the nearest candidate by returned distance, with a special condition allowing a candidate marked `+0x3D3` to replace the current choice:

```text
for each Dock= entry:
  candidate = vtable+0x52C(dock_type, arg3, arg4, &distance)
  if candidate and (no current candidate or distance < best or best == -1 or candidate+0x3D3):
    best = candidate
```

**Active in YR:** Yes. `[HARV] Dock=NAREFN,GAREFN`.

### 3.4 Missing refinery during state-3 unload skips storage drain

In `UnitClass::Mission_Deploy_Building @ 0x0073D630`, state 3 rediscoveres the refinery from the miner's current cell plus the `(-1,0)` lookup:

```text
0x0073E2C8  CALL [unit vtable+0x1B8]       ; current map cell
0x0073E2D5  ADD CX, [0x0089F6A0]
0x0073E2DC  ADD DX, [0x0089F6A2]
0x0073E2FF  CALL MapClass::Get_CellClass
0x0073E306  CALL Look_up_building_in_cell
0x0073E30D  CMP EDI, EBX
0x0073E30F  JNZ 0x0073E355                ; building found -> drain gate
```

When the building lookup is null, the branch is:

```text
0x0073E313  CALL 0x0065AE30               ; PathType::Has_Valid_Steps
0x0073E318  TEST AL, AL
0x0073E31A  JZ 0x0073E328
0x0073E31E  PUSH 0x3
0x0073E322  CALL [vtable+0x274]           ; radio CLEAR_LINK
0x0073E32A  PUSH 0x1
0x0073E32C  PUSH 0x0A
0x0073E330  CALL [vtable+0x1E8]           ; SetMission(Harvest, queued=1)
0x0073E338  CALL 0x005B3A00               ; timer epilogue
```

All storage/credit code is below the building-found branch at `0x0073E355+`. The missing-building path never calls `StorageClass::FindFirstNonEmptySlot`, `RemoveAmount`, or `HouseClass::Add_Tiberium_Credits`. Cargo/display/contact cleanup is therefore an abort/eject concern, not an unload credit path.

**Active in YR:** Yes. This is the live DockUnload mission handler for stock HARV.

### 3.5 Interrupt cleanup for physically linked docked units

`BuildingClass::UndockUnit @ 0x004593A0` reads `building+0x2E4`, returns immediately if null, and otherwise clears the reciprocal dock link fields and sends radio `3`. Prior and live decompilation show `BuildingClass::Sell @ 0x00449C30` calls `UndockUnit` when `field_0x2E4 != 0`; prior verified reports identify the destroyed-building caller as `BuildingClass::ReceiveDamage`.

For stock zero-link DockUnload, normal completion does not use this reciprocal link. For sell/damage while a reciprocal link exists, `UndockUnit` is still the active interrupt cleanup and preserves undumped harvester storage because it does not touch `unit+0x33C`.

**Active in YR:** Conditional. Active when a linked dock/garrison-style `+0x2E4` relation exists; stock HARV normal DockUnload mostly uses the zero-link state-3/state-4 path.

## 4. INI Keys

| INI key | Stock value | Effect in this slice | Active in YR |
|---|---|---|---|
| `rulesmd.ini:[HARV] Dock` | `NAREFN,GAREFN` | refinery candidates for state-2 return | Yes |
| `rulesmd.ini:[HARV] Harvester` | `yes` | reaches `Mission_Harvest` and DockUnload harvester paths | Yes |
| `rulesmd.ini:[HARV] Storage` | `40` | full cargo threshold via `Get_Storage_Percentage` | Yes |
| `rulesmd.ini:[HARV] UnloadingClass` | `HORV` | unloading display override; must clear on abort | Yes |
| `rulesmd.ini:[HARV] Primary` | `20mmRapid` | proves HARV is armed; combat behavior out of this slot | Yes |
| `rulesmd.ini:[HARV] Teleporter` | absent/false | selects non-teleporter return branch | Yes as false |
| `[General] HarvesterTooFarDistance` | `5` | non-teleporter close return threshold `* 0x100` | Yes |
| `[General] TiberiumShortScan` | `6` | archive/continuation scan; not entered before full state-0 return | Yes |
| `[General] TiberiumLongScan` | `48` | normal state-0 ore scan; bypassed by full-cargo check | Yes |
| `[NAREFN]/[GAREFN] DockUnload` | `yes` | radio handoff to unload mission | Yes |
| `[NAREFN]/[GAREFN] Refinery` | `yes` | refinery identity checks in unload path | Yes |

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `UnitClass::Mission_Harvest @ 0x0073E5E0` | full-cargo state-0 check; state-2 dock lookup/refind | live decompile + assembly contexts above | Yes |
| `FootClass::Find_Docking_Bay @ 0x004DF040` | selects nearest candidate from HARV `Dock=` list | live decompile | Yes |
| `UnitClass::Mission_Deploy_Building @ 0x0073D630` | unit-side DockUnload; missing-building state-3 abort | live decompile + assembly contexts above | Yes |
| `Look_up_building_in_cell @ 0x0047C520` | returns first object in `CellClass+0xE4` list whose `WhatAmI()==6` | live decompile | Yes |
| `BuildingClass::UndockUnit @ 0x004593A0` | clears reciprocal link on sell/damage/temporal interrupt | live/prior decompile | Conditional |
| `BuildingClass::Sell @ 0x00449C30` | calls `UndockUnit` if `building+0x2E4 != 0` | live decompile | Conditional |

## 6. Current Rust Implementation Status

Current Rust already reflects the desired high-level behavior in the uncommitted miner work:

- `src/sim/miner/miner_system.rs::handle_search_ore` checks `miner.is_full()` before ore scan and sends full miners to `ReturnToRefinery`.
- `src/sim/miner/miner_system.rs::handle_return` clears an invalid `reserved_refinery`; if the miner is full it clears `target_ore_cell` and remains `ReturnToRefinery`, otherwise it uses `SearchOre`.
- `src/sim/miner/miner_dock_sequence.rs::abort_invalid_refinery` clears the unload display override, movement/facing/dock fields, preserves cargo, and chooses `ReturnToRefinery` for full cargo through `dock_abort_state`.
- `src/sim/miner/miner_dock_sequence.rs::interrupt_refinery_docked_miners` models the interrupt cleanup shape for contacted/on-pad miners when the refinery is removed or invalidated.
- `src/sim/miner/miner_tests.rs` already contains targeted tests named `full_miner_losing_dying_refinery_keeps_returning` and `dying_refinery_aborts_unload_without_credit_or_stuck_visual`.

No Rust files were modified by this investigation.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Existing trace gap | verified | `miner/traces/MINER_REFINERY_UNAVAILABLE_MID_CYCLE_TRACE.md` | stale line paths need replacement because Rust has changed |
| HARV INI identity | verified | `rulesmd.ini:[HARV]` | none |
| State-0 full check before ore scan | verified | `0x0073E700..0x0073E720` | none |
| State-0 no-full ore scan branch | touched-not-exhausted | `0x0073E72A+`; prior harvest reports | exact archive details out of scope |
| State-2 no-refinery fallback | verified | `0x0073EB7E..0x0073EC58`; timer epilogue `0x0073EF77` | runtime wait length not measured |
| State-2 alternate refinery selection | verified | `FootClass::Find_Docking_Bay @ 0x004DF040` | exact same-distance tie beyond decompile not needed for this slot |
| State-3 missing-building unload abort | verified | `0x0073E2C8..0x0073E338` | none |
| Storage/credit bypass on missing building | verified | drain starts only under found branch `0x0073E355+` | none |
| Conditional radio `3` on missing building | verified | `0x0073E313..0x0073E322` | exact `PathType` state at each runtime moment out of scope |
| `UndockUnit` sell/damage cleanup | verified/touched | `0x004593A0`, `BuildingClass::Sell @ 0x00449C30`; prior ReceiveDamage report | exact same-frame order needs runtime debugger |
| Normal state-4 completion | deferred | sibling report `STOCK_MISSION_DEPLOY_BUILDING...` | out of scope for missing-refinery abort |
| Two-miner queue promotion | deferred | sibling slot | out of scope |

## 8. Open Questions - Final State Of The Investigation Log

- [RESOLVED] OQ-01 - What mode is this investigation? -> exhaustive-slice for full HARV cargo plus missing selected refinery before/during DockUnload. (evidence: user scope)
- [RESOLVED] OQ-02 - Does prior research exist? -> Yes; the May 20 trace is partial and this report verifies its binary assumptions against live Ghidra. (evidence: `MINER_REFINERY_UNAVAILABLE_MID_CYCLE_TRACE.md`)
- [RESOLVED] OQ-03 - Is HARV active on this path in YR? -> Yes, `Harvester=yes`, `Dock=NAREFN,GAREFN`, `Storage=40`; no Teleporter. (evidence: `rulesmd.ini:[HARV]`)
- [RESOLVED] OQ-04 - Does state 0 scan ore before checking full cargo? -> No; full storage writes substate 2 and returns before archive/search logic. (evidence: `0x0073E700..0x0073E720`)
- [RESOLVED] OQ-05 - Does a full HARV that re-enters Harvest fall into ore search? -> No; it immediately refalls into return-to-refinery substate 2. (evidence: `0x0073E714`)
- [RESOLVED] OQ-06 - What happens if state 2 finds no refinery? -> It reaches the mission timer epilogue while staying in state 2; no ore-search transition is written. (evidence: `0x0073EC50..0x0073EF77`)
- [RESOLVED] OQ-07 - Can state 2 select another refinery? -> Yes; it calls `Find_Docking_Bay` over the unit type's `Dock=` list and keeps the best candidate. (evidence: `0x004DF040`)
- [RESOLVED] OQ-08 - Does non-teleporter HARV use `HarvesterTooFarDistance`? -> Yes, distance is compared with `Rules+0xD78 * 0x100`; stock value is `5`. (evidence: `0x0073EBFB..0x0073EC19`; `rulesmd.ini`)
- [RESOLVED] OQ-09 - Does missing refinery during state-3 unload drain cargo? -> No; null lookup branches before the drain gate and storage calls. (evidence: `0x0073E30D..0x0073E338`, drain branch `0x0073E355+`)
- [RESOLVED] OQ-10 - What mission is set after state-3 missing-building abort? -> `SetMission(0x0A, queued=1)`. (evidence: `0x0073E32A..0x0073E330`)
- [RESOLVED] OQ-11 - Is radio clear sent unconditionally on the missing-building abort? -> No, it is conditional on `PathType::Has_Valid_Steps`. (evidence: `0x0073E313..0x0073E322`)
- [RESOLVED] OQ-12 - What does `Look_up_building_in_cell` consider a refinery/building object? -> It scans `CellClass+0xE4` and returns the first object whose `WhatAmI()==6`; it does not inspect dead/health flags itself. (evidence: live decompile `0x0047C520`)
- [RESOLVED] OQ-13 - Does stock normal DockUnload rely on reciprocal `+0x2E4`? -> No for normal zero-link completion; reciprocal link is conditional/interrupt context. (evidence: `STANDARD_REFINERY_0X2E4_WRITER_INVENTORY_GHIDRA_REPORT.md`, `0x0073D63B`)
- [RESOLVED] OQ-14 - Does sell call interrupt cleanup when a reciprocal link exists? -> Yes, `BuildingClass::Sell` checks `field_0x2E4` and calls `UndockUnit`. (evidence: live decompile `0x00449C30`)
- [RESOLVED] OQ-15 - Is cargo preserved by the interrupt/missing-building branch? -> Yes for the scoped binary branches because neither `UndockUnit` nor the state-3 missing-building branch removes `unit+0x33C` storage. (evidence: `0x004593A0`, `0x0073E311..0x0073E338`)
- [RESOLVED] OQ-16 - What current Rust surfaces implement the intended fallback? -> `handle_search_ore`, `handle_return`, `abort_invalid_refinery`, `interrupt_refinery_docked_miners`, and miner tests. (evidence: source scan)
- [DEFERRED] OQ-17 - Exactly which same-frame order occurs when a refinery is killed during a deposit tick? (category: `needs-runtime-debugger`; reason: static decompile proves branch behavior but not command-vs-mission scheduling on a live frame; next-step-if-pursued: non-breaking trace `BuildingClass::ReceiveDamage`, `UndockUnit`, and `Mission_Deploy_Building` during a controlled kill-on-pad scenario)
- [DEFERRED] OQ-18 - How does queue promotion behave when another full HARV is waiting and the refinery disappears? (category: `out-of-scope`; reason: sibling queue/dock slots own this; next-step-if-pursued: two-miner missing-refinery trace)
- [DEFERRED] OQ-19 - Exact unloading display restoration mechanism in vanilla after a zero-link missing-building abort. (category: `requires-different-system-context`; reason: this slot proves abort leaves unload path and no further credit, but display class swap lifecycle is slot 7; next-step-if-pursued: HARV unloading class display timing investigation)

Deferred items are non-material to the main answer: full HARV should not enter ore search, cargo is preserved, and missing-building unload abort does not credit more ore.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Full HARV state 0 checks storage before ore scan and writes return substate 2 | `0x0073E700..0x0073E720`; `[HARV] Storage=40` | appears fixed | `src/sim/miner/miner_system.rs::handle_search_ore` | Full miners that lost a refinery must not drive to ore or wait-no-ore before retrying return | Full War Miner with stale `target_ore_cell`, invalid refinery, and another live refinery remains `ReturnToRefinery` then selects the live refinery | Do not set `SearchOre` first and rely on a later correction |
| State 2 no-refinery path stays in return/retry, not ore search | `0x0073EC50..0x0073EF77` | appears fixed | `handle_return`, `find_nearest_refinery` | If no refinery exists, full miner waits/retries as return logic; if another exists, selects it on return tick | Remove selected refinery while full HARV is en route; next tick clears reservation; following tick picks second refinery | Do not clear cargo or stale into `WaitNoOre` via `SearchOre` |
| Missing refinery during state-3 unload skips storage drain and calls `SetMission(0x0A,1)` | `0x0073E311..0x0073E338` | appears fixed | `src/sim/miner/miner_dock_sequence.rs::abort_invalid_refinery`, `phase_unloading` | Abort dock sequence, preserve all cargo, and return full miner to refinery selection | Dying/sold refinery with HARV in `Unloading` and timer ready grants zero credits, keeps cargo, clears dock phase | Do not continue crediting a dying/despawned refinery |
| Missing-building abort conditionally sends radio `3` only if path steps/contact state require it | `0x0073E313..0x0073E322` | abstracted | `RefineryDockContacts`, `abort_invalid_refinery` | Clear Rust contact/queue/on-pad bookkeeping for the invalid refinery without inventing a normal completion exit | Invalid refinery leaves no occupied/contact state and queued miners are not blocked by stale occupant | Do not model this as normal state-4 completion or `ReleaseDockedHarvester` |
| Interrupt cleanup preserves undumped storage and clears dock visuals/links | `0x004593A0`; `0x00449C30`; state-3 no-drain branch | appears fixed | `interrupt_refinery_docked_miners`, `abort_invalid_refinery`, entity display override | Clear `HORV` display override, pivot/facing target, movement/dock caches, and preserve cargo | War Miner visibly stops unloading after refinery destruction and can seek another refinery with full cargo | Do not leave `display_type_override=HORV` after abort |

### Proposed Rust Test Names

- `war_miner_full_searchore_after_missing_refinery_refalls_to_return`
- `war_miner_full_return_invalid_refinery_selects_second_refinery`
- `war_miner_unload_missing_refinery_preserves_cargo_and_credits_none`
- `war_miner_unload_abort_clears_horv_display_override`
- Existing coverage names that match this report: `full_miner_losing_dying_refinery_keeps_returning`, `dying_refinery_aborts_unload_without_credit_or_stuck_visual`

### Stale Docs / Follow-up Docs

Replace the May 20 trace lines that say Rust "sets `SearchOre`" in the missing-refinery paths with:

> Current Rust now mirrors the verified stock-YR full-cargo guard: a full miner whose selected refinery becomes invalid clears the stale reservation and target ore cell, preserves cargo, and remains/returns to `ReturnToRefinery` so the next return tick can select another live owned refinery. Non-full miners may still fall back to ore search.

Replace the dying-refinery crediting claim with:

> Current Rust treats `dying` refineries as invalid for miner dock resolution; dock/unload abort clears contact/visual state and does not credit additional cargo to the dying refinery.

Retain the binary behavior wording:

> Stock `Mission_Deploy_Building` state 3 handles a missing refinery by optionally transmitting radio `3`, then `SetMission(Harvest=0x0A, queued=1)`, without draining storage or awarding credits. Full-cargo re-entry into `Mission_Harvest` state 0 immediately writes return substate 2 before ore scan.

## Sources

- Ghidra live decompile / context: `UnitClass::Mission_Harvest @ 0x0073E5E0`, `UnitClass::Mission_Deploy_Building @ 0x0073D630`, `FootClass::Find_Docking_Bay @ 0x004DF040`, `Look_up_building_in_cell @ 0x0047C520`, `BuildingClass::UndockUnit @ 0x004593A0`, `BuildingClass::Sell @ 0x00449C30`.
- Prior docs read: `miner/traces/MINER_REFINERY_UNAVAILABLE_MID_CYCLE_TRACE.md`, `HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md`, `MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md`, `STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_PATHTYPE_STATE4_GHIDRA_REPORT.md`, `BUILDING_UNDOCKUNIT_0x4593A0_CHRONO_MINER_GHIDRA_REPORT.md`, `STANDARD_REFINERY_0X2E4_WRITER_INVENTORY_GHIDRA_REPORT.md`.
- INI checked: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`, `rules.ini`, `artmd.ini`, `art.ini`.
- Rust scanned: `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_tests.rs`.

**Status:** COMPLETE
