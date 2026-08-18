# SetGhostCell (0x0070c610) and the state-4 refinery-exit-coord helper (0x00703590)

Session 2026-07-28, read-only Ghidra trace (gamemd.exe, project testProsjekt, image base
0x400000). Swarm slot 3. Scope: these two functions, the field they write/read, and their
callers only. Non-goals: full Mission_Harvest re-derivation (already covered by
`docs/scans/trace-swarm-20260728/mission-harvest-cadence.md`), no other miner subsystems.
Evidence-to-complete: decompile+asm for both targets, full caller enumeration, resolution of the
"is this the native [this+0x218] mechanism" question. Stop condition: both functions' write/read
contracts and caller lists established with citations, or diminishing returns on further callers.

## VERIFIED

### 1. 0x0070c610 IS the setter for TechnoClass+0x218 (same field as mission-harvest-cadence.md, not a related-but-different one)

Two-instruction `__thiscall` body: `MOV EAX,[ESP+4]` / `MOV [ECX+0x218],EAX` / `RET 4`
(verified via disassemble_function 0x0070c610; decompile_function 0x0070c610 shows the same
single statement `*(undefined4 *)(param_1 + 0x218) = param_2;`). This is byte-identical to the
field the prior lane already named "[this+0x218] archive/return target" (mission-harvest-cadence
line 44) — **Active in YR: Yes, confirmed same field, not merely related.**

The pre-existing Ghidra plate comment on this function reads *"TechnoClass::SetWarpVisualState —
Writes param to TechnoClass+0x218 (WarpState). Used to control the visual warp-in/out rendering
effect."* This name/plate is **not supported by any call site examined this session** — see
Stale-doc-replacement section below.

### 2. Stored value is a CellClass pointer (or 0), not a raw packed CELL int

At the Mission_Harvest state-1 storage-full prescan site (0x0073ea65), the value pushed into
SetGhostCell is the direct return of `MapClass::Get_CellClass` (`CALL 0x005657a0`, ECX=map
singleton 0x87f7e8), not the packed 16:16 cell value computed just before it (verified via
get_assembly_context xref_sources 0x0073ea65: `LEA ECX,[ESP+0x10] / PUSH ECX / MOV ECX,0x87f7e8 /
CALL 0x005657a0 / PUSH EAX / MOV ECX,EBP / CALL 0x0070c610`). The sentinel-hit branch
(0x0073ea7b) and the two Mission_Harvest clear-sites (0x0073e74b, 0x0073eaf2) all push the literal
`0x0` instead. **Active in YR: Yes** (verified via get_assembly_context on all four Mission_Harvest
call sites + decompile_function 0x0073e5e0).

### 3. [this+0x218] is a generic "staged/archived destination" field, not Harvest-private

`TechnoClass__Set_Destination` (0x00741970, decompile_function) calls `TechnoClass__SetGhostCell()`
(writing either 0 or a target pointer) at four different branch points unrelated to harvesting —
e.g. when redirecting to a building-target's dock-approach cell, or when a piggyback/teleport
target changes. `FootClass__OnArrival` (0x004d82b0, decompile_function) **reads** the field
directly: `if (((iVar1==0xf) && (param_1[0x86]!=0)) && (param_1[0xb7]==0)) { ... compare current
coord to the target's coord; if mismatched, Set_Destination(param_1[0x86],1) after
SetGhostCell(0) }` — i.e. on arrival during Hunt mission, if an archived destination is stashed,
resume driving to it. (`param_1` is declared `int *` in both functions' decompiled signatures, so
`param_1[0x86]` = byte offset 0x86*4 = 0x218 — confirmed against the raw offset, no ×4 ambiguity.)
`WarpAttachClass__Detach` (0x0062a4a0, decompile_function, pre-existing PROOFED plate
"chronominer-locomotion/fn-warp-attach-detach.md") also clears it (`SetGhostCell()` with implicit
0) right before re-arming a Set_Destination(0,1) when the warp target cell fails a placement
check. **Active in YR: Yes** — this is a shared TechnoClass mechanism used by mission code, the
Set_Destination pipeline, and chrono-warp detach alike; Harvest is one of many consumers.

### 4. Full caller list of 0x0070c610 (SetGhostCell) — 25 sites, all writers

`AircraftClass__Set_Destination`(0x0041aa80), `BuildingClass__ExitObject_Main`(0x00443c60),
`BuildingClass__MissionRepairAndProduce`(0x0044b780), `EventClass__Execute`(0x004c6cb0),
`FUN_00455d50`, `FUN_0050c920`, `FUN_006ed200`, `FUN_006ed7e0`, `FUN_006ef110`,
`FootClass__Mission_AreaGuard`(0x004d6aa0), `FootClass__Mission_Patrol`(0x004d4280),
`FootClass__Mission_Rescue`(0x004ddf90), `FootClass__OnArrival`(0x004d82b0),
`FootClass__PointerExpired`(0x004d9960), `InfantryClass__PerCellProcess`(0x00519630),
`InfantryClass__Set_Destination`(0x0051aa40), `SlaveManagerClass__AI_Update`(0x006af6c0),
`SlaveManagerClass__HandleReturnedSlaves`(0x006b0db0), `TeamClass__Recruit_Or_Add`(0x006e9380),
`TechnoClass__Set_Destination`(0x00741970), `TeleportLocomotionClass__StateMachineTick`(0x007192f0),
`UnitClass__AI`(0x007360c0), `UnitClass__Mission_Deploy`(0x006afd60),
`UnitClass__Mission_Harvest`(0x0073e5e0, 4 call sites), `UnitClass__PerCellProcess`(0x00739ec0),
`WarpAttachClass__Detach`(0x0062a4a0). (verified via get_function_callers 0x0070c610 /
get_xrefs_to 0x0070c610). Only Set_Destination, OnArrival, WarpAttachClass__Detach and
Mission_Harvest were decompiled this session (item 3 above); the remaining ~19 callers were not
individually inspected — see Remaining Uncertainty.

### 5. FUN_00703590 (state-4 exit-coord helper) is a generic "find nearby passable cell around a reference object" wrapper — no bib or facing math of its own

Signature (verified via decompile_function + disassemble_function 0x00703590):
`__thiscall FUN_00703590(this=<searching object, ECX>, hidden_out_ptr, reference_object_or_0)`
(RET 0x8 confirms exactly 2 stack dwords beyond the implicit this; the caller-side push order at
the Mission_Harvest callsite — `PUSH building_ptr` then `PUSH &local_buffer` then `MOV ECX,EBP` —
confirms the hidden output pointer is pushed last/closest to the call, and the reference-object is
pushed first, matching the callee's `MOV ECX,[ESP+0x2c]` read of the reference-object argument).
Body:
1. `iVar1 = (**(this->vtable+0x84))()` then reads `+0x67c` off the result, remapping value `4`→`1`.
   Vtable slot 0x84 resolves (UnitClass vtable 0x007f5c70+0x84 = 0x007f5cf4, `read_memory` →
   0x006f3270) to a thunk that tail-calls slot 0x88 (`decompile_function 0x006f3270`,
   Ghidra-labeled `TechnoClass__GetTechnoType_Trampoline`) — i.e. a type-accessor whose `+0x67c`
   field feeds the "zone/movement type" argument to the cell search, clamped so type-value 4 becomes 1.
2. Gets a reference coordinate: if `reference_object_or_0 == 0`, calls `this->vtable+0x48` (own
   Get_Coord); otherwise calls `reference_object->vtable+0x48` (the passed object's Get_Coord).
   Both results (X,Y leptons) are shifted right 8 (÷256, matching the project's verified
   leptons-per-cell=256 constant) and packed into a 32-bit cell value.
3. `iVar3 = (**(this->vtable+0x84))()` again, reads `+0x5b4` off the result; if that's `-1`,
   zone id = `-1`, else `MapClass__GetZoneID` (0x0056d230-family) is called with
   `this+0x8c` (byte; `param_1[0x23]` in `int*` indexing = offset 0x8c) as the movement-zone arg.
4. Calls the generic map-side cell search (`CALL 0x0056dc20`, invoked with **ECX = the fixed
   MapClass singleton address 0x87f7e8**, not a FootClass `this` — the Ghidra display name
   `FootClass__Find_Nearby_Passable_Cell` is a navigation label only; the call shape shows a
   MapClass-singleton-owned method) with the reference cell, the zone/type value from step 1, the
   zone id from step 3, the movement-zone byte from `this+0x8c`, and several fixed flags
   (`1,1,0,0,0,1`), writing the resulting cell through the hidden output pointer.

No bib offset, no facing/direction computation, and no Refinery/Weeder-type-specific field reads
occur inside 0x00703590 itself — the "drive off the pad" effect comes entirely from
Find_Nearby_Passable_Cell's own passability filtering around the building's footprint (occupied
building cells are simply not passable), not from any bib-aware code in this helper. **Active in
YR: Conditional** — the mechanism itself is definitely active, but "how it avoids the bib
specifically" is an emergent property of the shared passability search, not a Harvest/Refinery
special case; the underlying passability-vs-bib-cell semantics were not independently
re-investigated this session (out of scope — see project_force_track_bib_step memory).

### 6. Mission_Harvest state-4 call: reference object is the Refinery/Weeder building, not the unit itself

`iVar8 = Look_up_building_in_cell()` (building under the unit's own cell) is passed as
`reference_object_or_0`; `this` (ECX) at the call is the harvester unit itself (`MOV ECX,EBP`)
(verified via get_assembly_context xref_sources 0x0073ef4f + decompile_function 0x0073e5e0). So
the search is centered on the **building's** coordinate, using the **unit's** movement-zone/type,
then the resulting cell is converted via `MapClass__Get_CellClass` and fed to
`Set_Destination(cell, 1)` — exactly matching the existing doc's characterization.

### 7. Other callers of 0x00703590 confirm it is a general building-exit / warp-fallback helper, not harvest-specific

`WarpAttachClass__Detach` (0x0062a4a0) calls it with no reference object shown in the high-level
decompile but the surrounding code (`ppiStack_48 = &piStack_20` where `piStack_20 = param_1[9]`,
the attached TechnoClass) as a fallback when `CellClass__CheckCellPassability` on the direct warp
target fails. `BuildingClass__ExitObject_Main` (0x00443c60, 2 call sites: 0x00443ee3, 0x00443f83)
calls it with the same push shape (`PUSH ESI` reference-object, `PUSH &buffer` output) — this is
the generic "any unit exits any building" path (verified via get_assembly_context xref_sources
0x00443ee3). Other callers: `AircraftClass__Carryall_Pickup`(0x00416af0), `FUN_0070d8f0`,
`FUN_00739cd0`, `FootClass__Mission_Attack`(0x004d4dc0), `FootClass__Mission_Guard`(0x004d5070),
`UnitClass__AI`(0x007360c0) — verified via get_function_callers 0x00703590 / get_xrefs_to
0x00703590; these were not individually decompiled this session.

## Implementation Handoff

- **Verified behavior**: `[this+0x218]` (TechnoClass "staged destination") is written to either 0
  or a `CellClass*` by the single 2-instruction setter 0x0070c610, and is read/consumed generically
  by `Set_Destination`, `OnArrival`, and `WarpAttachClass__Detach`, not just Mission_Harvest.
  **Rust delta**: `miner.last_harvest_cell` (src/sim/miner/miner_system.rs) currently models this
  as a miner-local field; native has it as a general TechnoClass field reused across missions.
  **Affected surface**: any future non-Harvest mission logic that reads/writes the archived
  destination (Hunt-mission resume-on-arrival, warp-detach re-target) will not currently have
  a home in the Rust model. **Acceptance scenario**: a harvester interrupted mid-return (e.g. by a
  Hunt order) that later re-enters Harvest should not spuriously retain a stale archive target from
  the interrupting mission unless native does the same. **Proposed test name**:
  `miner_archive_cell_is_technoclass_generic_not_harvest_local`. **Risk**: low for current Harvest-only
  scope; medium if/when Hunt/OnArrival or chrono-warp-detach porting begins and reuses the same
  storage slot.
- **Verified behavior**: state-4 exit cell search centers on the Refinery/Weeder building's
  coordinate (not the unit's), using the unit's own movement-zone byte (`this+0x8c`) and type field
  (`typeaccessor()+0x67c`, clamped 4→1) as search filters, via the same generic
  Find_Nearby_Passable_Cell used by building-exit and warp-fallback code elsewhere.
  **Rust delta**: `exit_cell`/`find_nearby_passable_cell_with_index` in
  src/sim/miner/miner_dock_sequence.rs should center the search on the refinery/weeder building
  cell, not the unit's current cell, and should not need any bib-specific offset math — passability
  filtering alone should be responsible for excluding the building's own footprint.
  **Affected surface**: miner_dock_sequence.rs exit-cell computation. **Acceptance scenario**: a
  harvester queued at state 4 standing on a refinery pad computes an exit cell near the refinery's
  footprint (not near its own current position, which is inside/adjacent to that same footprint,
  so in practice these usually coincide but would diverge for an oddly-shaped Weeder/Refinery
  footprint). **Proposed test name**: `miner_state4_exit_cell_centers_on_refinery_not_unit`.
  **Risk**: low — likely already produces the same result in the common case since the unit is on
  the building's cell; the distinction matters mainly for large building footprints.
- **Verified behavior**: the Find_Nearby_Passable_Cell call inside 0x00703590 is invoked as a
  MapClass-singleton method (ECX=0x87f7e8), not a FootClass instance method, despite the Ghidra
  display name `FootClass__Find_Nearby_Passable_Cell`. **Rust delta**: none required (Rust already
  models this as a free/map-level helper); this is purely a naming-trust note for future RE work.
  **Affected surface**: documentation hygiene only. **Acceptance scenario**: n/a. **Proposed test
  name**: n/a. **Risk**: none (informational).

## Negative Facts / Do Not Do

- Do not treat 0x0070c610's Ghidra plate comment ("SetWarpVisualState" / "WarpState" / "visual
  warp-in/out rendering effect") as ground truth — no caller examined this session (including the
  chrono-warp-adjacent `WarpAttachClass__Detach`) does anything render/animation-related with this
  field; every observed use is "stash or clear a pending/archived destination pointer" (verified via
  decompile_function 0x00741970, 0x004d82b0, 0x0062a4a0 + disassemble_function 0x0070c610).
- Do not assume 0x00703590 contains bib-specific or facing-specific exit logic — it contains
  neither; it is a thin wrapper around the generic passable-cell search (verified via
  decompile_function + disassemble_function 0x00703590, no BuildingType field reads present in the
  function body).
- Do not assume the state-4 exit search is centered on the harvester's own position — it is
  centered on the Refinery/Weeder building's coordinate (verified via get_assembly_context
  xref_sources 0x0073ef4f showing `Look_up_building_in_cell()` result passed as the
  reference-object argument, not `this`).
- Do not assume SetGhostCell's argument is a raw packed CELL(x,y) integer — in the one branch
  traced end-to-end (0x0073ea65) it is a `CellClass*` pointer from `MapClass::Get_CellClass`
  (verified via get_assembly_context xref_sources 0x0073ea65).
- Do not rename/relabel `FootClass__Find_Nearby_Passable_Cell` (0x0056dc20) based on this report —
  the ECX-singleton-vs-FootClass-instance discrepancy noted in item 5 was observed only at this
  helper's two call sites and was not independently re-verified against the function's own
  prologue this session (no mutating Ghidra calls were made; nothing was renamed).

## Remaining Uncertainty

- ~19 of 0x0070c610's 25 callers (e.g. `AircraftClass__Set_Destination`,
  `BuildingClass__MissionRepairAndProduce`, `EventClass__Execute`, the slave-manager and
  team-recruit call sites, `TeleportLocomotionClass__StateMachineTick`, `UnitClass__AI`,
  `UnitClass__PerCellProcess`, several `FUN_*` addresses) were enumerated but not individually
  decompiled this session; their exact write conditions (0 vs. pointer) are unconfirmed.
- 6 of 0x00703590's 9 callers (`AircraftClass__Carryall_Pickup`, `FUN_0070d8f0`, `FUN_00739cd0`,
  `FootClass__Mission_Attack`, `FootClass__Mission_Guard`, `UnitClass__AI`) were enumerated but not
  decompiled; their reference-object argument (self vs. some other object) is unconfirmed.
- The exact semantics of the type-accessor's `+0x67c` field (remapped 4→1) and the `this+0x8c` byte
  passed as the movement-zone argument were not cross-referenced against TechnoTypeClass/rulesmd.ini
  field names this session — treated as opaque "zone/type" values sufficient to describe the call
  shape, not named definitively.
- Whether `FootClass__Find_Nearby_Passable_Cell`'s real identity is a MapClass method (per the
  ECX=0x87f7e8 singleton pattern observed at its call sites in this report) versus a mislabeled
  thunk was not independently verified by decompiling 0x0056dc20 itself — flagged as an open
  question for whoever next touches that function's label.
