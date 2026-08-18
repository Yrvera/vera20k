# Infantry Gate CanGarrison Result Contract - Ghidra Research Report

**Address(es):** `0x004525F0` (`BuildingClass::CanGarrison`), `0x004A51B0` (`Building+0x350` helper), `0x0051BF90` / branch `0x0051C4EB..0x0051C549` (`InfantryClass::Can_Enter_Cell`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** verify the gate-style `BuildingClass::CanGarrison` passability contract as consumed by `InfantryClass::Can_Enter_Cell`, including `Gate=yes` vs non-`Gate`, mission `0x18`, `Building+0x350` helper bytes, result codes `3/5/7`, and player-visible pathing/cursor implication.
**Non-Scope:** civilian `CanDock` entry validation, bunker `UnitRepair` row-helper, full gate animation/state machine, gate mission writers beyond the read-side `0x18` predicate.
**Confidence:** High for read-side contract and result-code mapping; Medium for stock content activation because this report did not runtime-trace a stock gate opening/closing.
**Active in YR:** Conditional. The code path is live in YR pathfinding and walk locomotion; the gate branch requires a building type with `Gate=yes` (`BuildingType+0x16B7`) and a live gate state.

## 0. Working Notes Gate

- Target question: What exact `BuildingClass::CanGarrison` passability contract does `InfantryClass::Can_Enter_Cell` consume for gate-style buildings, and which native result codes does it produce?
- Non-goals: Do not re-open civilian `CanDock` entry, bunker/UnitRepair row-helper, or full gate animation writers except negative separation.
- Evidence needed to mark COMPLETE: decompile plus assembly context for `CanGarrison`, helper `0x004A51B0`, infantry caller branch, live vtable/caller evidence, INI/default evidence for `Gate=yes`, and Rust implementation handoff.
- Stop conditions: report written at this exact path, `.swarm-claims.md` updated only, all open questions resolved or explicitly deferred, and no Rust/INI/in-repo docs modified.

## 1. Overview

`BuildingClass::CanGarrison @ 0x004525F0` is a misleadingly named gate passability helper. It returns true for non-`Gate=` buildings, but for `Gate=yes` buildings it requires current mission `0x18` and helper `0x004A51B0(Building+0x350)` true. Active in YR: Yes for the helper body; player-visible effect is conditional on a `Gate=yes` building.

`InfantryClass::Can_Enter_Cell @ 0x0051BF90` reaches `CanGarrison` only in the building-object branch after checking `BuildingType+0x16B7`. A failed gate passability check maps to code `3` for allied gates, code `5` for enemy gates when the infantry can take an action, and code `7` for enemy gates when it cannot. Active in YR: Yes; `AStar_main_loop @ 0x00429A90` and `WalkLocomotionClass::ProcessMovement @ 0x0075B650` dispatch through vtable `+0x1AC`.

## 2. Class Layout / Key Offsets

| Offset / slot | Owner | Meaning in this slice | Evidence | Active in YR |
|---:|---|---|---|---|
| `+0x520` | `BuildingClass` | pointer to `BuildingTypeClass`; typed as `param_1[0x148]` by decompiler | `CanGarrison` decompile reads `param_1[0x148] + 0x16B7`; assembly `0x004525F3..0x00452601` | Yes |
| `+0x16B7` | `BuildingTypeClass` | `Gate=` flag used by `CanGarrison` and infantry caller | `0x004525F3..0x00452601`; infantry read `0x0051C4EB..0x0051C4F3`; field map in `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md:336`; `rulesmd.ini:17186..17204` | Conditional on INI/mod data |
| vtable `+0x184` | mission-bearing object | `MissionClass::GetCurrentMission`; returns `+0xAC` unless `-1`, else `+0xB4` | `MissionClass__GetCurrentMission @ 0x005B3040`; assembly `0x005B3040..0x005B3051`; call in `CanGarrison @ 0x00452607..0x00452614` | Yes |
| `+0x350` | `BuildingClass` | gate-state helper object passed to `0x004A51B0` | `LEA ECX,[ESI+0x350]` at `0x00452616`; helper reads `+0x18/+0x19` | Conditional on gate runtime state |
| `+0x18` | helper at `Building+0x350` | must be byte `0` for passability | helper decompile/assembly `0x004A51B0..0x004A51C2` | Conditional |
| `+0x19` | helper at `Building+0x350` | must be byte `1` for passability | helper decompile/assembly `0x004A51B7..0x004A51BE` | Conditional |
| `Techno+0x21C` | infantry and building | owner/house pointer used for allied check | infantry assembly `0x0051C504..0x0051C516` | Yes |
| vtable `+0x2AC` | infantry | action/capability predicate used for hostile failed gate branch | infantry decompile and assembly `0x0051C52D..0x0051C53A` | Yes |
| vtable `+0x1AC` | techno class virtual | `Can_Enter_Cell` dispatch slot | `AStar_main_loop @ 0x00429A90` calls `(*param_4->vtable+0x1AC)`; walk runtime assembly `0x0075B68C..0x0075B690` | Yes |

## 3. Core Logic

### 3.1 `BuildingClass::CanGarrison @ 0x004525F0`

Pseudocode verified from decompile plus assembly:

```text
if (!this.Type.Gate) return true
if (this.GetCurrentMission() == 0x18) {
    if (gate_helper(this + 0x350)) return true
}
return false
```

Material findings:

- Non-`Gate=` buildings return true immediately. Evidence: `MOV CL,[Type+0x16B7]`, `TEST`, `JNZ`, then `MOV AL,1; RET` at `0x004525F3..0x00452606`. Active in YR: Yes as helper code; conditional effect because the infantry caller only invokes it after its own `Gate` branch.
- `Gate=yes` buildings require current mission `0x18`. Evidence: vtable `+0x184` call and `CMP EAX,0x18` at `0x00452607..0x00452614`; `MissionClass__GetCurrentMission @ 0x005B3040` reads `+0xAC`, falling back to `+0xB4`. Active in YR: Conditional on gate mission state.
- `Gate=yes` buildings require helper `0x004A51B0` true on `Building+0x350`. Evidence: `LEA ECX,[ESI+0x350]`, `CALL 0x004A51B0`, `TEST AL`, false return at `0x00452616..0x0045262C`. Active in YR: Conditional on gate runtime state.

### 3.2 `Building+0x350` Helper `0x004A51B0`

The helper is a two-byte predicate:

```text
return byte[+0x18] == 0 && byte[+0x19] == 1
```

Evidence: decompile of `0x004A51B0`; assembly reads `MOV AL,[ECX+0x18]`, rejects nonzero at `0x004A51B0..0x004A51B5`, reads `MOV DL,[ECX+0x19]`, compares to `1`, and returns `1` only on equality at `0x004A51B7..0x004A51C2`. Active in YR: Conditional on gate state object.

This report deliberately does not name the full semantic states behind `+0x18/+0x19`; only the read-side predicate is verified. The adjacent helper at `0x004A51D0` checks a different byte combination and is out-of-scope.

### 3.3 Infantry Caller Result Codes

`InfantryClass::Can_Enter_Cell` has the gate path inside its `WhatAmI()==6` building-object branch:

```text
if (building.Type.Gate) {
    if (!building.CanGarrison()) {
        if (building.Owner allied with infantry.Owner) result = max(result, 3)
        else if (!infantry.can_take_action()) return 7
        else result = max(result, 5)
    }
}
```

Evidence: decompile `0x0051BF90`; assembly `0x0051C4EB..0x0051C549`:

- Reads `BuildingType+0x16B7` at `0x0051C4EB`; if zero, jumps to generic building/object handling at `0x0051C553`. Active in YR: Conditional on `Gate=yes`.
- Calls `BuildingClass::CanGarrison` at `0x0051C4F5..0x0051C4F7`; if true, jumps to shared continuation `0x0051C70F` without raising the result. Active in YR: Conditional.
- If `CanGarrison` false, calls allied check `0x004F9A50` with owner pointers from `Techno+0x21C` at `0x0051C504..0x0051C516`. Active in YR: Yes.
- Allied failed gates set `EBX = 3` only if current result is below `3` (`CMP EBX,0x3`, `JGE`, `MOV EBX,0x3` at `0x0051C51A..0x0051C528`). Active in YR: Yes.
- Enemy failed gates call infantry vtable `+0x2AC`; false returns code `7` via jump to `0x0051C7D0` (`0x0051C52D..0x0051C53A`). Active in YR: Yes.
- Enemy failed gates with action capability set `EBX = 5` only if below `5` (`0x0051C540..0x0051C549`). Active in YR: Yes.

### 3.4 Live Pathing / Player-Visible Implication

`AStar_main_loop @ 0x00429A90` calls the mover's vtable `+0x1AC` for neighbor eligibility. Its decompile contains `iVar17 = (**(code **)(*param_4 + 0x1ac))(...); if (iVar17 < 7) ...`, so result codes below `7` remain traversable candidates with costs, while `7` blocks the neighbor. Active in YR: Yes; no TS-only gate was observed on this dispatch.

`WalkLocomotionClass::ProcessMovement @ 0x0075B650` also calls the owner's vtable `+0x1AC` at runtime (`0x0075B68C..0x0075B690`). It reacts differently to codes: code `6` scatters friendly blockers, code `5/4` can identify a blocking object and attack/handle it, code `2` schedules blocked-path retry, code `7` aborts/clears movement in the runtime blocked path. Active in YR: Yes for infantry walk locomotion.

Player-visible implication: a closed/ineligible allied `Gate=yes` building behaves like a soft scatter/wait blocker (`3`); a closed/ineligible enemy gate is an attackable blocker (`5`) only if the infantry can take an action, otherwise it is hard impassable (`7`). This affects A* route selection and runtime cursor/order feedback indirectly through whether the path/cell can be treated as attackable vs impossible. Active in YR: Conditional on gate state and ownership.

## 4. INI Keys / Stock Data

| Key | Stock evidence | Binary read/effect in this slice | Active in YR |
|---|---|---|---|
| `Gate=` | `[GAGATE_A] Gate=yes` at `ini/rulesmd.ini:17186..17204`; base `rules.ini:9394..9412` also has `GAGATE_A Gate=yes` | consumed as `BuildingType+0x16B7` by `CanGarrison` and infantry branch; field mapping documented in `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md:336` | Conditional; `rulesmd.ini` also changes general gate lists to `GADUMY`, so stock map/use activation was not runtime-traced here |
| `GateStages=` | prior visual report says parsed to `BuildingType+0x16F8`; not used by this passability helper | not read by `CanGarrison` or the scoped infantry branch | No for this contract; visual animation only |
| `GateCloseDelay=` / `DeployTime=` | present on `[GAGATE_A]` near `Gate=yes` | no read observed in `CanGarrison` or `0x0051C4EB..0x0051C549` | No for this read-side contract |

## 5. Integration Points

| Integration point | Evidence | Active in YR |
|---|---|---|
| A* pathfinding dispatches through mover vtable `+0x1AC`, consuming result codes as pass/block classes | `AStar_main_loop @ 0x00429A90` decompile calls `(*param_4->vtable+0x1AC)` and gates neighbor expansion on `< 7` | Yes |
| Walk locomotion performs a runtime `Can_Enter_Cell` check before final subcell movement | assembly `0x0075B68C..0x0075B690` calls `dword ptr [ESI+0x1AC]`; subsequent code switches on `2/3/4/5/6/7` behavior | Yes |
| `InfantryClass::Can_Enter_Cell` has the scoped gate branch | decompile `0x0051BF90`; assembly `0x0051C4EB..0x0051C549` | Yes |
| Civilian garrison entry uses `CanDock`, not this helper | prior settled report `GARRISON_CANDOCK_CANGARRISON_ENTRY_GATES_GHIDRA_REPORT.md`; decompiles `0x0051E3B0` and `0x00519630` call `BuildingClass::CanDock` for action/arrival | Yes, but out-of-scope except negative separation |

## 6. Current Rust Implementation Status

Rust scan used Codegraph first, then file reads/`rg`. Current relevant surfaces:

- `src/sim/pathfinding/cell_entry.rs`: `CellEntryResult` already maps native codes `0..7`; no gate-specific infantry branch exists in the scanned classifier.
- `src/sim/pathfinding/cell_entry.rs`: vehicle row-helper logic is now explicitly unit-only (`LiveVehicleBuildingEntry.mover_category != EntityCategory::Unit` keeps blocker), which is correct negative separation for this scope.
- `src/sim/movement/movement_occupancy.rs`: dispatch distinguishes infantry vs vehicle deferred checks but ultimately uses the shared classification surface.
- `src/rules/object_type.rs`: no Rust building `Gate=` parser surfaced in `rg`; only overlay gates are parsed in `src/map/overlay_types.rs` and terrain overlays in `src/map/resolved_terrain.rs`.

Current delta: Rust has native result-code names but lacks the separate infantry `Gate=yes` building passability branch and building gate-state model needed to produce `3/5/7` from this contract.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingClass::CanGarrison @ 0x004525F0` | verified | decompile; assembly `0x004525F3..0x0045262C` | none for read-side passability |
| Non-`Gate=` true branch | verified | `0x004525F3..0x00452606` | none |
| `Gate=yes` mission `0x18` branch | verified | `0x00452607..0x00452614`; `MissionClass__GetCurrentMission @ 0x005B3040` | mission name/writers deferred |
| `Building+0x350` helper predicate | verified | decompile/assembly `0x004A51B0..0x004A51C2`; call at `0x00452616` | semantic state names deferred |
| Infantry caller `Gate` branch | verified | decompile `0x0051BF90`; assembly `0x0051C4EB..0x0051C549` | none for result mapping |
| Result code `3` allied path | verified | `0x0051C504..0x0051C528` | none |
| Result code `5` enemy/action path | verified | `0x0051C52D..0x0051C549` | exact action predicate semantics at vtable `+0x2AC` not expanded |
| Result code `7` enemy/no-action path | verified | `0x0051C532..0x0051C53A` branch to hard-block return | none for code mapping |
| A* / walk liveness | verified | `AStar_main_loop @ 0x00429A90`; walk assembly `0x0075B68C..0x0075B690` | vtable table bytes not re-read in this slot |
| Civilian `CanDock` entry | deferred | out-of-scope; prior report already settled | separate validator, not this report |
| Bunker/UnitRepair row-helper | deferred | out-of-scope; negative separation only | separate bunker reports |
| Full gate animation state machine | deferred | out-of-scope | trace writers to mission `0x18` and `Building+0x350` if needed |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-001 - Is this an exhaustive or coverage investigation? -> exhaustive-slice for the read-side `CanGarrison`/infantry result-code contract.` (evidence: user target and this report scope)
- `[RESOLVED] OQ-002 - Does non-`Gate=` return true? -> Yes; `CanGarrison` returns `1` before mission/helper checks.` (evidence: `0x004525F3..0x00452606`)
- `[RESOLVED] OQ-003 - Does `Gate=yes` require mission `0x18`? -> Yes; vtable `+0x184` result is compared against literal `0x18`.` (evidence: `0x00452607..0x00452614`)
- `[RESOLVED] OQ-004 - What does vtable `+0x184` read? -> `MissionClass__GetCurrentMission` returns `+0xAC` unless `-1`, else `+0xB4`.` (evidence: `0x005B3040..0x005B3051`)
- `[RESOLVED] OQ-005 - What is the `Building+0x350` helper predicate? -> byte `+0x18 == 0` and byte `+0x19 == 1`.` (evidence: `0x004A51B0..0x004A51C2`)
- `[RESOLVED] OQ-006 - Does infantry call this helper from its building branch? -> Yes, after reading `BuildingType+0x16B7`.` (evidence: `0x0051C4EB..0x0051C4F7`)
- `[RESOLVED] OQ-007 - What does a true `CanGarrison` do to infantry result code? -> It jumps to continuation without increasing `EBX` result in the gate branch.` (evidence: `0x0051C4FC..0x0051C4FE -> 0x0051C70F`)
- `[RESOLVED] OQ-008 - What code does failed allied gate passability produce? -> `max(current, 3)`.` (evidence: `0x0051C504..0x0051C528`)
- `[RESOLVED] OQ-009 - What code does failed enemy gate passability with action capability produce? -> `max(current, 5)`.` (evidence: `0x0051C52D..0x0051C549`)
- `[RESOLVED] OQ-010 - What code does failed enemy gate passability with no action capability produce? -> hard block `7`.` (evidence: `0x0051C532..0x0051C53A`, target branch `0x0051C7D0`)
- `[RESOLVED] OQ-011 - Is the path live in YR A*? -> Yes, A* dispatches through vtable `+0x1AC` and consumes codes below `7` as expandable.` (evidence: `AStar_main_loop @ 0x00429A90`)
- `[RESOLVED] OQ-012 - Is the path live in YR walk runtime? -> Yes, walk locomotion calls owner vtable `+0x1AC` at runtime.` (evidence: `0x0075B68C..0x0075B690`)
- `[RESOLVED] OQ-013 - Is `CanDock` part of this branch? -> No; action/arrival `CanDock` callers are separate.` (evidence: `0x0051E3B0`, `0x00519630`; prior report)
- `[RESOLVED] OQ-014 - Is bunker/UnitRepair row-helper part of this infantry gate branch? -> No; branch reads `+0x16B7` and calls `CanGarrison`, not row helper fields.` (evidence: `0x0051C4EB..0x0051C549`; prior infantry report)
- `[RESOLVED] OQ-015 - Does current Rust already model the native code enum? -> Yes, `CellEntryResult::yr_code` maps `0..7`; gate branch is missing.` (evidence: `src/sim/pathfinding/cell_entry.rs`)
- `[DEFERRED] OQ-016 - What exact writer names/transition labels set mission `0x18` and helper bytes?` (category: out-of-scope; reason: target is read-side passability contract; next-step-if-pursued: trace gate mission state machine and `Building+0x350` writers)
- `[DEFERRED] OQ-017 - Does a stock campaign/skirmish map exercise `[GAGATE_A]` after `rulesmd.ini` dummy gate list changes?` (category: needs-runtime-debugger; reason: INI has `Gate=yes`, but stock placement/use was not runtime-traced; next-step-if-pursued: set breakpoint on `0x004525F0` with stock gate fixture)

Zero-add pass: re-read `0x004525F0`, `0x004A51B0`, and branch `0x0051C4EB..0x0051C549` after drafting; no new in-scope branch or constant was added.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `CanGarrison` read-side contract is gate passability: non-`Gate=` true; `Gate=yes` true only when mission `0x18` and helper `Building+0x350` bytes are `+0x18==0`, `+0x19==1`. | `0x004525F0`; assembly `0x004525F3..0x0045262C`; helper `0x004A51B0..0x004A51C2`; `0x005B3040` | missing | building rules/runtime state plus `src/sim/pathfinding/cell_entry.rs` | Add a building gate passability predicate separate from civilian garrison entry. | Closed `Gate=yes` building is not treated like ordinary non-gate building; open/enterable mission/helper state permits infantry gate branch continuation; proposed test `infantry_gate_can_garrison_open_state_allows_gate_branch_continuation` | Do not use `CanDock`, `CanBeOccupied`, or cargo capacity to model this helper. |
| Failed allied `Gate=yes` passability in `InfantryClass::Can_Enter_Cell` upgrades result to at least code `3`. | `0x0051C504..0x0051C528` | missing | `src/sim/pathfinding/cell_entry.rs`, movement occupancy classifier | Return `CellEntryResult::ScatterRequired` / YR code `3` for allied closed/ineligible gates. | Allied infantry pathing into a closed allied gate cell yields code `3`, causing scatter/wait-style behavior rather than hard abort; proposed test `infantry_allied_closed_gate_returns_scatter_required_code_3` | Do not collapse allied closed gates to generic friendly stationary code `6`. |
| Failed enemy `Gate=yes` passability returns code `5` if infantry can take an action, otherwise code `7`. | `0x0051C52D..0x0051C549` | missing | `src/sim/pathfinding/cell_entry.rs`, action/weapon capability surface | Classify enemy closed gates as attackable blockers only when the mover can act; otherwise hard-block. | Armed infantry vs closed enemy gate yields code `5`; no-action/unarmed infantry yields code `7`; proposed test `infantry_enemy_closed_gate_action_capability_selects_code_5_or_7` | Do not always return `OccupiedEnemy`; code `7` is required for no-action infantry. |

## Negative Facts / Do Not Do

- Do not use `BuildingClass::CanGarrison` as the civilian building entry validator. Evidence: `CanGarrison` only checks `Gate`, mission `0x18`, and `Building+0x350`; `CanDock` is called by `InfantryClass::What_Action_OnObject @ 0x0051E3B0` and `PerCellProcess @ 0x00519630`. Active in YR: Yes.
- Do not apply bunker/UnitRepair `NumberImpassableRows` row-helper behavior to this infantry gate branch. Evidence: scoped branch reads `BuildingType+0x16B7` and calls `0x004525F0`; no `+0x16A9/+0x16AB/+0x1620` read or row helper call appears in `0x0051C4EB..0x0051C549`. Active in YR: No for this infantry branch.
- Do not treat `GateStages`, `GateCloseDelay`, or `DeployTime` as part of the `CanGarrison` passability predicate based on this report. Evidence: `0x004525F0` reads only `Gate`, mission, and helper bytes. Active in YR: No for this helper.
- Do not make all enemy gates attackable. Evidence: enemy failed passability first calls vtable `+0x2AC`; false returns `7` before the `5` upgrade. Active in YR: Yes.
- Do not globally reinterpret `BuildingType+0x16BF` as `Gate=`. Evidence: current verified gate branch reads `+0x16B7`; `INFANTRY_BUILDING_OCCUPANT_PATHING_GHIDRA_REPORT.md` already corrected stale `UNIT_CAN_ENTER_CELL` offset naming. Active in YR: Yes.

## Remaining Uncertainty

- Full gate writer/state-machine contract for mission `0x18` and `Building+0x350` byte transitions remains out-of-scope.
- Stock-map runtime activation of `[GAGATE_A] Gate=yes` after YR's `GDIGateOne/GDIGateTwo/NodGate* = GADUMY` general-list changes was not traced.
- The exact semantic name for infantry vtable `+0x2AC` was not expanded; only its true/false effect on the enemy gate result code is verified.

## Stale Docs / Follow-up Docs

- `docs/research/UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` replacement wording: "`BuildingType+0x16B7` is the `Gate=` flag used by `InfantryClass::Can_Enter_Cell` and `BuildingClass::CanGarrison`. `BuildingType+0x16BF` is not the gate flag in this branch. Replace 'CanBeGarrisoned/HasActiveAnim_0x16B7' and 'IsGate_0x16BF' with 'Gate_0x16B7' for the scoped infantry gate branch."
- `docs/research/UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` replacement wording for helper role: "`BuildingClass::CanGarrison @ 0x004525F0` is gate passability, not civilian garrison entry. For `Gate=yes`, it requires mission `0x18` and helper `0x004A51B0(Building+0x350)` true; failed infantry caller mapping is allied `3`, enemy action `5`, enemy no-action `7`."
- `docs/research/GARRISON_SYSTEM_GHIDRA_REPORT.md` replacement wording for stale confidence/warp wording: "`CanGarrison` is now HIGH-confidence gate passability by direct decompile and infantry caller evidence. `CanDock` calls `TechnoClass::IsMindControlled @ 0x007105E0`; do not label that scoped call as an IsBeingWarped/chrono gate without separate evidence."

## Sources

- Ghidra decompile/read-only: `0x004525F0`, `0x004A51B0`, `0x0051BF90`, `0x005B3040`, `0x00429A90`, `0x0075B650`, `0x0051E3B0`, `0x00519630`.
- Ghidra assembly context/read-only: `0x004525F3..0x0045262C`, `0x004A51B0..0x004A51C2`, `0x0051C4EB..0x0051C549`, `0x005B3040..0x005B3051`, `0x0075B68C..0x0075B690`.
- Prior docs referenced for duplication/conflict only: `GARRISON_CANDOCK_CANGARRISON_ENTRY_GATES_GHIDRA_REPORT.md`, `INFANTRY_BUILDING_OCCUPANT_PATHING_GHIDRA_REPORT.md`, `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md`, `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md`, `GARRISON_SYSTEM_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scan: `src/sim/pathfinding/cell_entry.rs`, `src/sim/movement/movement_occupancy.rs`, `src/rules/object_type.rs`, `src/map/overlay_types.rs`, `src/map/resolved_terrain.rs`.
