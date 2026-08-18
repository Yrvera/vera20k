# UnitClass PerCellProcess Caller Tick Order - Ghidra Research Report

**Address(es):** `0x00739EC0` primary per-cell dock-arrival hook; `0x004D9290` `FootClass::Mission_Enter`; `0x006F9E50` `TechnoClass::AI_Update`; `0x004DA530` `FootClass::AI`; `0x007360C0` `UnitClass::AI`; `0x005B3060` `MissionClass::Mission_Dispatch`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** caller/tick-order placement of `UnitClass::PerCellProcess @ 0x00739EC0` relative to mission dispatch, `FootClass::Mission_Enter`, locomotor processing, and radio `0x16` after a refinery accepted-cell arrival.
**Non-Scope:** full DriveLocomotion/PathType internals, exact runtime replay frame where a stock miner crosses the stock `GetDockCoord` cell, and non-refinery per-cell consumers.
**Confidence:** High for static tick order and mission-vs-locomotor ordering; Medium for which `0x15` source wins in every runtime timing case because exact locomotor target/cell-cross frame still needs a drive-locomotor slice.
**Active in YR:** Yes. Stock `CMIN/HARV -> GAREFN/NAREFN` uses `UnitClass::AI`, `FootClass::AI`, mission `7` / Enter, refinery radio `0x0E/0x16`, and the `0x00739EC0` dock-arrival branch.

## 0. Required Working Notes

- Target question: Where and when is `UnitClass::PerCellProcess @ 0x00739EC0` reached relative to `Mission_Enter`, locomotor processing, and the repeat `0x0E/0x16` cascade?
- Non-goals: Do not re-prove accepted cell `NW+(3,1)`, `GetDockCoord` `NW+(2,1)`, or `+0x16BB/+0x16BC` flag identity unless contradicted.
- Evidence needed to mark COMPLETE: decompile and caller evidence for `UnitClass::AI -> FootClass::AI -> TechnoClass::AI_Update -> Mission_Dispatch`; decompile evidence that locomotor `Process` runs after mission dispatch; evidence for `0x00739EC0` as a per-cell hook and for its `0x15` branch.
- Stop conditions: stop once static order is proven and any remaining "which exact retail frame wins" uncertainty is narrowed to runtime locomotor cell-crossing, not mission-dispatch ordering.

## 1. Bottom Line

The due mission pass wins the start of a unit AI tick. For a live unit tick, `UnitClass::AI` calls `FootClass::AI`; `FootClass::AI` immediately calls `TechnoClass::AI_Update`; `TechnoClass::AI_Update` calls `MissionClass::Mission_Dispatch`; and only after that returns does `FootClass::AI` call the active locomotor `Process`.

Therefore:

1. A due `FootClass::Mission_Enter @ 0x004D9290` repeat `CAN_DOCK(0x0E)` check runs before that tick's locomotor processing and before any per-cell hook caused by movement in that later locomotor process.
2. `UnitClass::PerCellProcess @ 0x00739EC0` is the dock-arrival/cell-entry hook, not the mission-7 handler itself. The mission-7 handler for the stock unit path is `FootClass::Mission_Enter @ 0x004D9290`.
3. If the miner is only at accepted cell `NW+(3,1)`, `0x00739EC0` cannot send the `GetDockCoord`-gated `0x15`, because that branch requires current cell == destination `GetDockCoord` cell.
4. If locomotor processing later in the tick physically crosses the stock `GetDockCoord` cell, `0x00739EC0` can send `0x15` before the next mission-dispatch opportunity, but it did not beat the mission pass for the current tick.

This narrows the long-standing blocker: Rust should not model `0x15` as an immediate phase transition solely because the accepted `0x0E/0x12` cell was reached. It needs two distinct possible handoff sources: the repeat-radio `0x16` path on mission ticks and the cell-entry `0x00739EC0` path on actual `GetDockCoord` cell crossing.

## 2. Key Offsets / Slots

| Slot / field | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| vtable `+0x5C` on UnitClass | `UnitClass::AI @ 0x007360C0` | xrefs to `FootClass__AI`; `UnitClass::AI` calls `FootClass__AI` at `0x0073647B` | Yes |
| `FootClass+0x19D` / byte `+0x674` | active `ILocomotion*` | `FootClass::AI` calls `[loco_vtable+0x40]` at the locomotor process site | Yes |
| mission `7` | Enter / `Mission_Enter` | `MissionClass::Mission_Dispatch @ 0x005B3060` case `7 -> vtable+0x240` | Yes |
| vtable `+0x240` for FootClass/Unit stock path | `FootClass::Mission_Enter @ 0x004D9290` | data xrefs to `0x004D9290`; decompile sends `0x0E` and returns mission timer jitter | Yes |
| `0x00739EC0` | per-cell dock-arrival hook, historically mislabeled as UnitClass Mission_Enter in some docs | data xref `0x007F5DFC`; body contains dock-arrival `0x15`, not the generic mission-timer jitter epilogue | Yes |
| `0x007416A0` | separate unit crush/scatter per-cell handler | data xref `0x007F61A4`; decompile is crush/scatter, not refinery dock-arrival | Conditional; unit cell-entry behavior |

## 3. Static Tick Order

### 3.1 UnitClass AI calls FootClass AI

`UnitClass::AI @ 0x007360C0` calls `FootClass::AI` at `0x0073647B`. The call is after UnitClass pre-AI checks such as contained transport handling, tube movement, death explosion timers, and a small amount of unit-specific state. It is before later unit-specific turret/fire/harvest brain work.

Evidence: `decompile_function 0x007360C0`; `get_function_xrefs 0x004DA530` returns `0x0073647B in UnitClass__AI`; assembly range spot-check `0x00736430..0x007364A7`.

Active in YR: Yes. Stock miners are UnitClass objects and tick through this function.

### 3.2 FootClass AI calls TechnoClass AI_Update first

`FootClass::AI @ 0x004DA530` starts with `TechnoClass::AI_Update()` at `0x004DA539`, then checks alive state. This means mission dispatch inside `TechnoClass::AI_Update` is earlier than the FootClass locomotor process for the same unit tick.

Evidence: `decompile_function 0x004DA530`; `get_function_xrefs 0x006F9E50` returns `0x004DA539 in FootClass__AI`.

Active in YR: Yes. `UnitClass::AI` reaches this for stock harvesters unless the unit dies or is otherwise early-returned before the call.

### 3.3 TechnoClass AI_Update calls Mission_Dispatch

`TechnoClass::AI_Update @ 0x006F9E50` calls `MissionClass::Mission_Dispatch()` at `0x006FA655`. That mission dispatch happens before `TechnoClass::AI_Update` returns to `FootClass::AI`, so it is before the later FootClass locomotor `Process`.

Evidence: `decompile_function 0x006F9E50`; `get_function_xrefs 0x005B3060` returns `0x006FA655 in TechnoClass__AI_Update`.

Active in YR: Yes. This is the live mission dispatch path for active TechnoClass-derived objects, including UnitClass miners.

### 3.4 Mission_Dispatch gates Mission_Enter by timer

`MissionClass::Mission_Dispatch @ 0x005B3060` first calls `ObjectClass::AI()`, then checks mission timer fields `+0xC8/+0xD0`, and only dispatches the current mission when the timer is due. Case `7` calls vtable `+0x240`. For the stock unit path, that vtable slot resolves to `FootClass::Mission_Enter @ 0x004D9290`, not `0x00739EC0`.

Evidence: `decompile_function 0x005B3060`; `decompile_function 0x004D9290`; `get_function_xrefs 0x004D9290` shows data xrefs in vtables; `0x004D9290` sends radio `0x0E` and returns `ftol(MissionTimerEntry[Enter] * 900.0) + RandomRanged(0,2)`.

Active in YR: Yes. Standard miner return-to-refinery uses mission `7` / Enter.

### 3.5 Locomotor Process runs after Mission_Dispatch in FootClass AI

After `TechnoClass::AI_Update` returns, `FootClass::AI` later reaches the active locomotor `Process` call at the `ILocomotion` vtable `+0x40` site around `0x004DA87A`. This is the first clear per-tick movement-state processing point in FootClass AI after mission dispatch.

Evidence: `decompile_function 0x004DA530`; assembly range spot-check `0x004DA850..0x004DA8DB`; existing `FOOTCLASS_COMPLETE_GHIDRA_REPORT.md` also records `ILocomotion::Process` as called from FootClass AI.

Active in YR: Yes. Stock miner movement/arrival depends on the active locomotor object.

## 4. PerCellProcess Placement And The `0x15` Race

`UnitClass::PerCellProcess @ 0x00739EC0` is not invoked by `FootClass::Mission_Enter @ 0x004D9290`; `0x004D9290` is self-contained for the repeat `CAN_DOCK(0x0E)` mission pass. Prior miner docs also note `0x004D9290` does not call `0x00739EC0`.

The dock-arrival branch inside `0x00739EC0` requires mission `7` or `0x19`, destination object `WhatAmI()==6`, current unit cell converted from current coords, destination `GetDockCoord` converted to cell, and equality of X/Y cells. Only then it calls `FootClass::PerCellProcess(2)`, sends radio `0x15`, and calls locomotor vtable `+0x5C`.

Evidence for branch: `decompile_function 0x00739EC0`; assembly/context ranges from prior report `0x0073A391..0x0073A3B1` for destination vtable `+0xA8`, `0x0073A417..0x0073A437` for cell compare, `0x0073A4F7..0x0073A507` for `FootClass::PerCellProcess(2)` then radio `0x15`, and `0x0073A521..0x0073A52B` for locomotor `+0x5C`.

Active in YR: Yes. This is the live stock unit dock-arrival path, but the `0x15` send is conditional on the cell equality gate.

### Practical Race Answer

If both are considered inside the same unit AI tick, the due mission pass is earlier:

`UnitClass::AI -> FootClass::AI -> TechnoClass::AI_Update -> Mission_Dispatch -> FootClass::Mission_Enter -> radio 0x0E/0x16 path -> return -> FootClass::AI locomotor Process -> possible per-cell callback`.

So `UnitClass::PerCellProcess` cannot fire before a due repeat `0x0E/0x16` cascade in that same unit tick. It can only fire later in the tick if locomotor processing actually crosses or reports the dock cell. If it does fire during that movement process, it happens before the next tick's mission dispatch.

Active in YR: Yes / Conditional. The ordering is active; the cell-entry callback is conditional on movement reaching the `GetDockCoord` cell.

## 5. Current Rust Implementation Status

Current Rust has a dock phase split in `src/sim/miner/miner_dock_sequence.rs`:

| Rust surface | Current behavior relevant to this slice | Status |
|---|---|---|
| `phase_mission_enter` | accepted cell reached and not moving can mark contact entered and transition directly to `Linked` | drift-risk |
| `phase_awaiting_accepted_cell` | movement completion returns to `MissionEnter` for the next CAN_DOCK pass | broadly aligned |
| `phase_linked` | unconditionally snaps snapshot `rx/ry` to pad, marks pad occupied, starts pivot/display/sound | drift-risk because it conflates `0x15` handoff with a Rust phase transition rather than modeling source/tick order |
| `RefineryDockPhase::Linked` docs | described as `0x15 pad-arrival handoff` | stale-risk unless it distinguishes repeat-radio `0x16 -> 0x15` from cell-entry `0x00739EC0 -> 0x15` |

No Rust files were modified in this investigation.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `UnitClass::AI @ 0x007360C0` call to `FootClass::AI` | verified | decompile; xref `0x0073647B` | none |
| `FootClass::AI @ 0x004DA530` first-call ordering | verified | decompile; xref `0x004DA539` to `TechnoClass::AI_Update` | none |
| `TechnoClass::AI_Update @ 0x006F9E50` mission dispatch placement | verified | decompile; xref `0x006FA655` to `MissionClass::Mission_Dispatch` | none |
| `MissionClass::Mission_Dispatch @ 0x005B3060` timer and mission 7 dispatch | verified | decompile; case `7 -> vtable+0x240` | none |
| `FootClass::Mission_Enter @ 0x004D9290` repeat `0x0E` source | verified | decompile; data xrefs in vtables | exact MissionTimerEntry address/name not needed for this slice |
| `FootClass::AI` locomotor process after mission dispatch | verified | decompile; process call around `0x004DA87A` | exact locomotor callback internals deferred |
| `UnitClass::PerCellProcess @ 0x00739EC0` dock-arrival `0x15` | verified | decompile; prior assembly context `0x0073A391..0x0073A52B` | runtime cell-cross frame deferred |
| `UnitClass::PerCellProcess @ 0x007416A0` | touched-not-exhausted | decompile shows crush/scatter per-cell handler | not the refinery target; no further work needed here |
| Which stock source sends first in all replay timings | deferred | static order proves mission-pass-before-locomotor; exact drive cell-cross frame not proven | drive-locomotor arrival/cell-crossing runtime slice |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is 0x00739EC0 called by FootClass::Mission_Enter? -> No evidence of a call; 0x004D9290 is self-contained and prior miner docs explicitly separate the per-cell hook.` (evidence: `decompile_function 0x004D9290`; `miner/REFINERY_DOCK_EXIT_CHAIN_VERIFIED_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-2 - Does Mission_Dispatch run before locomotor Process in the same FootClass::AI tick? -> Yes. FootClass::AI calls TechnoClass::AI_Update first; TechnoClass::AI_Update calls Mission_Dispatch; FootClass::AI later calls locomotor vtable +0x40.` (evidence: `0x004DA539`, `0x006FA655`, `0x004DA87A`)
- `[RESOLVED] OQ-3 - Can PerCellProcess beat a due Mission_Enter repeat in the same tick? -> No. The mission pass is earlier than locomotor/per-cell processing in that unit tick.` (evidence: `0x004DA530`, `0x006F9E50`, `0x005B3060`)
- `[RESOLVED] OQ-4 - Can PerCellProcess still beat the next repeat after a physical cell crossing? -> Yes, conditionally. If locomotor processing crosses the `GetDockCoord` cell later in the current tick, `0x00739EC0` can send `0x15` before the next tick's mission dispatch.` (evidence: `0x004DA87A` ordering plus `0x00739EC0` dock-arrival branch)
- `[RESOLVED] OQ-5 - Is accepted `NW+(3,1)` enough for the 0x00739EC0 0x15 branch? -> No. That branch compares current cell to destination `GetDockCoord`, which is a separate gate.` (evidence: `0x0073A391..0x0073A437`; prior `REFINERY_DOCK_0X16_BRIDGE_VERIFICATION_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-6 - Is 0x00739EC0 the mission-7 handler? -> No for this static slice. Mission dispatch case 7 calls vtable +0x240; stock unit vtable data also references 0x004D9290 for that slot. 0x00739EC0 is a per-cell dock-arrival hook in current verified miner docs.` (evidence: `0x005B3060`, xrefs to `0x004D9290`, docs listed below)
- `[DEFERRED] OQ-7 - What exact drive-locomotor call reports the stock GetDockCoord cell crossing?` (category: `requires-different-system-context`; reason: this slot only proved ordering around Mission_Enter and FootClass AI; next-step-if-pursued: drive-locomotor process/cell-cross callback slice)
- `[DEFERRED] OQ-8 - In a retail replay, is the miner ever physically moved from accepted `NW+(3,1)` to stock GetDockCoord `NW+(2,1)` before the second synchronized 0x16?` (category: `needs-runtime-debugger`; reason: static evidence proves both possible sources and their tick order but not the concrete retail frame for every timing case; next-step-if-pursued: runtime trace of current cell, mission timer, `0x16` timer, and per-cell callback)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Mission dispatch and `FootClass::Mission_Enter` run before locomotor process/per-cell callback in the same unit AI tick | `0x004DA539`, `0x006FA655`, `0x004DA87A`, `0x005B3060`, `0x004D9290` | Rust has phase ordering but `Linked` can be entered directly from accepted-cell state | `src/sim/miner/miner_dock_sequence.rs::phase_mission_enter`, `phase_awaiting_accepted_cell` | keep the accepted-cell "already there" retry as a mission-timer/event boundary before any `0x15`/unload handoff | accepted-cell arrival with next mission pass due runs CAN_DOCK/0x16 logic before any simulated per-cell pad arrival | `miner_dock_mission_enter_repeats_before_cell_entry_handoff`; do not start unload merely because movement_target cleared at accepted cell |
| `0x00739EC0` `0x15` is a cell-entry/GetDockCoord source, conditional on current cell == destination dock cell | `0x0073A391..0x0073A52B` | Rust `phase_linked` snaps snapshot to pad and marks pad occupied without source identity | `phase_linked`, dock reservation pad occupancy bookkeeping | represent cell-entry `0x15` separately from repeat-radio `0x16 -> 0x15`; never hide physical-vs-snapshot cell drift | miner at accepted cell but not GetDockCoord does not trigger cell-entry `0x15`; miner that actually enters GetDockCoord can trigger it before next mission repeat | `miner_dock_getdockcoord_cell_entry_sends_0x15`; do not fake by setting only `snap.rx/snap.ry` |
| If locomotor crosses GetDockCoord later in the current tick, per-cell `0x15` can happen before the next mission pass, but not before the current due mission pass | combined static order above plus `0x00739EC0` branch | Rust currently does not model the two-source race explicitly | miner dock FSM and future locomotor/per-cell event integration | add source-aware tests and state for "0x16 path handoff" vs "per-cell GetDockCoord handoff" before changing coordinates | one test where due mission tick wins; one test where actual GetDockCoord cell crossing wins before next mission tick | `miner_dock_two_0x15_sources_ordered_by_tick_phase`; do not force every stock handoff through NW+(2,1) |

## 9. Negative Facts / Do Not Do

- Do not keep treating `0x00739EC0` as the ordinary mission-7 dispatch handler. The live mission-dispatch function case `7` calls vtable `+0x240`, and the stock unit path has `0x004D9290` in that mission slot.
- Do not let "per-cell hook happens on movement" imply it precedes mission execution. In FootClass AI, mission dispatch is inside `TechnoClass::AI_Update` before locomotor `Process`.
- Do not start Rust unload from accepted `NW+(3,1)` solely because the miner stopped there. The `0x00739EC0` cell-entry path is gated by destination `GetDockCoord`.
- Do not collapse `0x16` and `0x00739EC0` into one generic "linked" event. They are separate `0x15` sources with different gates and tick placement.
- Do not use `0x007416A0` as the refinery dock-arrival body. It is a separate unit crush/scatter per-cell handler.

## 10. Stale Docs / Follow-up Docs

Stale replacement wording found:

- In `MISSIONCLASS_STATE_MACHINE.md`, replace the row `UnitClass | Mission_Enter | 0x00739EC0` with: `UnitClass | Mission_Enter | inherits/uses FootClass::Mission_Enter @ 0x004D9290 for the mission-dispatch case-7 repeat radio path; UnitClass dock-arrival choreography lives in per-cell hook 0x00739EC0 and should not be labeled as the mission-dispatch handler.`
- In docs that say `UnitClass::Mission_Enter (= UnitClass__PerCellProcess)`, replace with: `FootClass::Mission_Enter @ 0x004D9290 performs the mission-7 repeat CAN_DOCK pass. UnitClass::PerCellProcess @ 0x00739EC0 is the cell-entry/dock-arrival hook that can send 0x15 after current cell equals destination GetDockCoord.`

## Sources

- Ghidra `decompile_function 0x007360C0`
- Ghidra `decompile_function 0x004DA530`
- Ghidra `decompile_function 0x006F9E50`
- Ghidra `decompile_function 0x005B3060`
- Ghidra `decompile_function 0x004D9290`
- Ghidra `decompile_function 0x00739EC0`
- Ghidra `decompile_function 0x007416A0`
- Ghidra `decompile_function 0x00737430`
- Ghidra `get_function_xrefs 0x004DA530`, `0x006F9E50`, `0x005B3060`, `0x004D9290`, `0x00739EC0`, `0x007416A0`
- Ghidra disassembly spot-check ranges `0x00736430..0x007364A7`, `0x004DA850..0x004DA8DB`, `0x005B3060..0x005B3357`, `0x00739EC0..0x0073B0B7`
- `docs/research/REFINERY_DOCK_0X16_BRIDGE_VERIFICATION_GHIDRA_REPORT.md`
- `docs/research/UNITCLASS_PERCELLPROCESS_DOCK_ARRIVAL_GETDOCKCOORD_GHIDRA_REPORT.md`
- `docs/research/miner/REFINERY_DOCK_EXIT_CHAIN_VERIFIED_GHIDRA_REPORT.md`
- `docs/research/miner/HARVESTER_DOCK_UNLOAD.md`
- Current Rust scan: `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/mod.rs`
