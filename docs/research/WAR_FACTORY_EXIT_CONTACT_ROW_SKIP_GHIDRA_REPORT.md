# War Factory Exit Contact Row Skip - Ghidra Research Report

**Address(es):** `0x00443C60`, `0x0044D880`, `0x00449540`, `0x0073F0A0`, `0x00458A00`, `0x0065A970`, `0x0065A820`, `0x006F4AB0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** stock land war factory produced-vehicle exit/contact passability around `GAWEAP`, `NAWEAP`, and `YAWEAP`: whether the produced vehicle enters radio contact with the factory, how `NumberImpassableRows=1` is activated, how `ClearBibArea` relates, and what must be treated as live `Can_Enter_Cell` exceptions rather than static PathGrid blockers.  
**Non-Scope:** naval yards, refineries, barracks, aircraft pads, full queue/refund handling after blocked production, exact runtime recovery of the stock 5x3 `ExitList[10]` pair, and Rust implementation.  
**Confidence:** High for contact setup, row-skip gating, `ClearBibArea` gate/formula, and Rust mismatch sizing; Medium for the exact visual drive-out route because the stock 5x3 `ExitList[10]` value remains unrecovered.  
**Active in YR:** Yes. `rulesmd.ini` stock `GAWEAP`, `NAWEAP`, and `YAWEAP` set `WeaponsFactory=yes`, `Factory=UnitType`, `Bib=yes`, `ExitCoord=512,256,0`, and `NumberImpassableRows=1`; none are `Naval=yes`.

## 1. Overview

Stock land war factories do create radio contact with the produced vehicle before the door/drive-out mission runs. That contact is the missing condition for the `NumberImpassableRows=1` passability relaxation: `UnitClass::Can_Enter_Cell` skips a contacted building occupant when the probed cell is not in the building's first impassable west column.

The important implementation consequence is negative: `NumberImpassableRows=1` must not be baked into a static PathGrid footprint for everyone. The exit cell `NW+(2,1)` and later drive-out/front-door cells are live exceptions for the produced unit while it is radio-linked to the factory; unrelated units should still see ordinary building occupancy plus the separate `HasBib` east-edge relaxation.

## 2. Class Layout / Key Offsets

| Offset | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `BuildingType+0x16BD` | `WeaponsFactory=yes` gate | `0x00443C60`, `0x0044D880`, `0x00449540`; stock INI | Yes |
| `BuildingType+0xCCE` | `Naval=yes` gate | `0x00443C60`; stock land WFs lack it | No for stock land WFs |
| `BuildingType+0xEC8/+0xECC/+0xED0` | `ExitCoord` lepton offset | `0x00443C60` calls vtable `+0xB4`; prior `GetExitCoord` report; stock INI | Yes |
| `BuildingType+0xED4` | `ExitList` pointer | `0x0044D880`, `0x00449540`; prior pointer-formula report | Yes |
| `BuildingType+0x1570` | `Bib=yes` / HasBib | `0x0073F7D3`; stock INI | Yes |
| `BuildingType+0x1620` | `NumberImpassableRows` | `0x00458A00`; stock INI | Yes for stock WFs |
| `RadioClass+0xE4/+0xE8` | contact vector data/capacity | `0x0065A970`, `0x0065A820`, `0x0065AD50` | Yes |
| `TechnoClass+0x418` | dock/contact-entered flag set by radio `0x18`, cleared by `0x19` | `0x006F4AB0` | Yes, but not the row-skip vector |

## 3. Core Logic

### 3.1 Initial land-WF exit creates radio contact

In `BuildingClass::ExitObject_Main @ 0x00443C60`, the stock land WF branch is selected by `WeaponsFactory=yes` and `Naval=no`. The branch:

1. calls building vtable `+0xB4` to obtain `GetExitCoord`;
2. calls produced unit vtable `+0xD8` (`Unlimbo`) at that coordinate with facing byte `0x40`;
3. calls building vtable `+0x278` twice: first with message `2` and target `param_2`, then with message `0x18` and target `param_2`;
4. queues building mission `0x10`.

`RadioClass::Transmit_Radio_Impl @ 0x0065A970` proves message `2` is HELLO/contact setup. It calls target `Receive_Radio(2)`; if the target returns `1`, it writes the target pointer into the sender's `Contacts[]`. `RadioClass::Receive_Radio @ 0x0065A820` also handles HELLO by writing the sender into the receiver's `Contacts[]` when ally/capacity checks pass.

**Finding:** after successful stock land WF unlimbo, the building and produced unit become reciprocal RadioClass contacts before the drive-out mission runs.  
**Active in YR:** Yes; this is the stock `WeaponsFactory=yes && !Naval` path for `GAWEAP`/`NAWEAP`/`YAWEAP`.

### 3.2 Contact is the row-skip gate in `Can_Enter_Cell`

In `UnitClass::Can_Enter_Cell @ 0x0073F0A0`, the occupant-chain branch around `0x0073F58A..0x0073F5A9` does this for each cell occupant:

1. call `DynamicVectorClass::Contains @ 0x0065AD50` on the mover's contact vector;
2. require the occupant `WhatAmI` result to be `6` (building);
3. call `FUN_00458A00`;
4. if the contact contains this building, it is a building, and `FUN_00458A00` returns false, jump to the next occupant (`LAB_0073FA87`) without blocking on this building.

`FUN_00458A00 @ 0x00458A00` first requires `Look_up_building_in_cell() == building`. If `NumberImpassableRows == -1`, it returns true, so no row-skip is created. Otherwise it compares the candidate cell x to `foundation_origin_x + NumberImpassableRows`.

For stock land WFs, `NumberImpassableRows=1`, so while the produced vehicle has radio contact with its factory:

- west column `x == rx` remains building-blocked by this helper;
- all same-building cells with `x >= rx + 1` are skipped as blockers for that contacted mover;
- the initial unlimbo cell `NW+(2,1)` is in the skipped range;
- cells around the later door/drive-out target computed from `ExitList[10]` are also live exceptions if they are same-building cells with `x >= rx+1`.

**Active in YR:** Yes for the contacted produced vehicle from stock land WFs; Conditional for other units because they need the factory in their RadioClass contact vector.

### 3.3 HasBib is separate from row skip

The HasBib branch at `0x0073F7D3` probes `cell + DAT_0089F690`; `Foundation_direction_table_init @ 0x0049F2F0` initializes `DAT_0089F690` to `(1,0)`. If the east neighbor does not contain the same building, `Can_Enter_Cell` skips this building occupant. This makes the east-edge column of a `Bib=yes` building non-blocking independently of radio contact.

For a 5x3 stock land WF, HasBib covers `(rx+4, ry..ry+2)` for any unit. `NumberImpassableRows=1` covers `(rx+1..rx+4, ry..ry+2)` only for a mover in RadioClass contact with the factory.

**Active in YR:** Yes; `Bib=yes` is set on stock land WFs and the initializer is a standard static-init table function used by active direction-offset consumers.

### 3.4 ClearBibArea scatters, but does not grant passability

`BuildingClass::ClearBibArea @ 0x00449540` is gated by `WeaponsFactory=yes`, not `Bib=yes`. It reads `*(Type+0xED4 + 0x28)` (`ExitList[10]`), computes `foundation_origin + (entry10.x - 1, entry10.y)`, finds a nearest object in that cell excluding the factory itself, and scatters objects. `FUN_0044D880 @ 0x0044D880` mission state 1 calls `ClearBibArea`; if it returns false, the mission advances to state 2.

`ClearBibArea` does not call `Can_Enter_Cell` for the produced vehicle and does not explain the row skip. It is a pre-drive-out scatter/kick routine for blockers at the fixed front-door cell.

**Active in YR:** Yes; it is called from the stock WF mission slot when `WeaponsFactory=yes`.

## 4. INI Keys

| Key | Stock values | Effect | Active in YR |
|---|---|---|---|
| `WeaponsFactory=yes` | `GAWEAP`, `NAWEAP`, `YAWEAP` | selects land-WF exit branch; gates `ClearBibArea` | Yes |
| `Factory=UnitType` | all three stock land WFs | production category; not a passability exception by itself | Yes |
| `ExitCoord=512,256,0` | all three stock land WFs in `rulesmd.ini` | initial unlimbo coordinate `NW+(2,1)` | Yes |
| `NumberImpassableRows=1` | all three stock land WFs in `rulesmd.ini` | when contact-gated, only west column remains impassable | Yes |
| `Bib=yes` | all three stock land WFs | HasBib east-edge relaxation, independent of contact row skip | Yes |
| `Naval=yes` | absent on stock land WFs | excludes naval WF path | No for stock land WFs |

## 5. Integration Points

| Function | Role | Active in YR |
|---|---|---|
| `BuildingClass::ExitObject_Main @ 0x00443C60` | successful stock WF unlimbo, HELLO, `0x18`, queue mission `0x10` | Yes |
| `RadioClass::Transmit_Radio_Impl @ 0x0065A970` | sender-side HELLO contact write | Yes |
| `RadioClass::Receive_Radio @ 0x0065A820` | receiver-side HELLO contact write | Yes |
| `UnitClass::Receive_Radio @ 0x00737430` | no case `2`, so HELLO falls through to base radio handling | Yes |
| `TechnoClass::Receive_Radio @ 0x006F4AB0` | `0x18` sets dock/contact-entered flag and propagates `0x18` | Yes |
| `UnitClass::Can_Enter_Cell @ 0x0073F0A0` | live occupant-chain passability, including contact row skip | Yes |
| `FUN_00458A00 @ 0x00458A00` | `NumberImpassableRows` helper | Yes when reached from contacted building branch |
| `BuildingClass::ClearBibArea @ 0x00449540` | front-door scatter from `ExitList[10]` | Yes |
| `FUN_0044D880 @ 0x0044D880` | WF door/drive-out state machine; calls `ClearBibArea` | Yes |

## 6. Current Rust Implementation Status

Scanned surfaces:

- `src/sim/pathfinding/core.rs::PathGrid::block_building_movement_cells` statically blocks cells returned by `building_movement_blocking_cells`.
- `src/sim/production/production_tech.rs::building_movement_blocking_cells` currently applies `NumberImpassableRows` with `number_rows_active=true`.
- `src/app_init.rs` and `src/app_sim_tick.rs` feed stock structure `number_impassable_rows` into the static PathGrid at load and every sim tick.
- `src/sim/production/production_spawn.rs::preferred_exit_offsets` converts `ExitCoord` to a primary `(2,1)` but generates neighbor fallbacks.
- Movement uses entity block sets in nearby movement code; those should not be the sole model for this contact-gated building exception.

Rust delta:

- Static `NumberImpassableRows=1` blocking is too permissive for unrelated movers because gamemd's row skip requires live radio contact with that exact building.
- Static row blocking is also too blunt for produced vehicles because the exception belongs in live `Can_Enter_Cell`/A* neighbor evaluation and movement tick checks, not in a global terrain grid.
- The stock WF exit cell `(rx+2, ry+1)` should be allowed for the produced vehicle because of reciprocal radio contact, even though it remains a building-occupied foundation cell.
- The generic neighbor fallback around `ExitCoord` remains a separate mismatch from the prior stock WF exit report.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Stock land WF branch after unlimbo | verified | `0x00443C60`, `rulesmd.ini` | none |
| HELLO creates reciprocal contact | verified | `0x0065A970`, `0x0065A820`, `0x00737430` | none |
| `0x18` flag propagation | verified | `0x00443C60`, `0x006F4AB0` | exact drive-out completion clear sender remains out-of-scope |
| Contact-vector gate inside `Can_Enter_Cell` | verified | `0x0073F58A..0x0073F5A9`, `0x0065AD50` | none |
| `NumberImpassableRows` helper | verified | `0x00458A00` | none |
| Stock WF `NumberImpassableRows=1` data | verified | `rulesmd.ini` stock WF sections | none |
| HasBib east-edge relaxation distinction | verified | `0x0073F7D3`, `0x0049F2F0`, stock `Bib=yes` | none for this slice |
| `ClearBibArea` gate and fixed cell formula | verified | `0x00449540`, `0x0044D880` | exact `ExitList[10]` pair remains unrecovered |
| Current Rust static PathGrid row blocking | verified | `src/sim/pathfinding/core.rs`, `src/sim/production/production_tech.rs`, `src/app_init.rs`, `src/app_sim_tick.rs` | implementation fix not attempted |
| A* live `Can_Enter_Cell` per-neighbor integration | touched-not-exhausted | prior `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md`; this report did not re-decompile A* | implementer should preserve as live call semantics |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Does successful stock land WF unlimbo create radio contact? -> Yes, building sends HELLO(2) to the unit; target and sender contact vectors are both updated when accepted.` (evidence: `0x00443C60`, `0x0065A970`, `0x0065A820`)
- `[RESOLVED] OQ-2 - Does UnitClass handle HELLO itself? -> No case 2 in `UnitClass::Receive_Radio`; it falls through to base radio handling.` (evidence: `0x00737430`)
- `[RESOLVED] OQ-3 - Is `NumberImpassableRows` static footprint data? -> No; it is used by `FUN_00458A00`, reached from a live `Can_Enter_Cell` branch after contact-vector and building checks.` (evidence: `0x0073F58A..0x0073F5A9`, `0x00458A00`)
- `[RESOLVED] OQ-4 - What does `NumberImpassableRows=1` mean for a contacted 5x3 WF? -> Same-building cells at x >= origin+1 are skipped as blockers; the west column remains blocking.` (evidence: `0x00458A00`, stock `rulesmd.ini`)
- `[RESOLVED] OQ-5 - Does HasBib explain the same cells? -> No. HasBib independently skips east-edge cells by probing `(x+1,y)`; it does not depend on radio contact.` (evidence: `0x0073F7D3`, `0x0049F2F0`)
- `[RESOLVED] OQ-6 - Does ClearBibArea gate on Bib? -> No; it gates on `WeaponsFactory=yes`.` (evidence: `0x00449540`)
- `[RESOLVED] OQ-7 - Does ClearBibArea grant produced-unit entry through the factory? -> No; it scatters blockers at `ExitList[10] + (-1,0)` and does not call the produced unit's `Can_Enter_Cell`.` (evidence: `0x00449540`, `0x0044D880`)
- `[RESOLVED] OQ-8 - Is stock land WF initial cell `NW+(2,1)` in the contact-skip range? -> Yes; `2 >= NumberImpassableRows(1)`.` (evidence: stock `ExitCoord=512,256,0`, `0x00458A00`)
- `[RESOLVED] OQ-9 - Should Rust statically unblock `rx+1..rx+4` for every unit? -> No; gamemd requires the mover's contact vector to contain the factory, except for the separate HasBib east edge.` (evidence: `0x0073F58A..0x0073F5A9`)
- `[DEFERRED] OQ-10 - What is the exact stock 5x3 `ExitList[10]` pair?` (category: `bounded-cost-too-high`; reason: pointer and call sites are verified, but prior table recovery did not decode the runtime-populated pair; next-step-if-pursued: narrow table-dump investigation)
- `[DEFERRED] OQ-11 - Which function sends `0x19`/break at the final end of WF drive-out?` (category: `out-of-scope`; reason: this slice only needs contact presence during exit row skip; next-step-if-pursued: trace drive locomotion/per-cell process after WF mission state 3)
- `[DEFERRED] OQ-12 - Full A* callsite matrix for live Can_Enter_Cell after Rust changes?` (category: `requires-different-system-context`; reason: A* live calls are already covered by prior report; next-step-if-pursued: verify implementation design against A* report)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `NumberImpassableRows=1` on stock land WFs is contact-gated: only a mover whose `Contacts[]` contains the factory skips same-building cells with `x >= rx+1`. | `0x0073F58A..0x0073F5A9`, `0x00458A00`, stock INI | mismatch: `PathGrid::block_building_movement_cells` statically applies rows | `src/sim/pathfinding/core.rs`, `src/sim/production/production_tech.rs`, movement live entry checks | Move row-skip semantics out of global terrain blockers and into live `Can_Enter_Cell`/entity-aware checks keyed by mover-factory contact | Produce a Grizzly from `GAWEAP`; it can leave through `(rx+2,ry+1)`, while an unrelated tank planning through `(rx+2,ry+1)` still treats the factory as occupied | `war_factory_contacted_produced_vehicle_can_enter_exitcoord_cell` | Do not globally unblock WF interior columns |
| Successful stock land WF exit establishes reciprocal radio contact before mission `0x10` drive-out. | `0x00443C60`, `0x0065A970`, `0x0065A820` | likely missing/unchecked: Rust production spawn has no explicit factory contact state | `src/sim/production/production_spawn.rs`, entity state/contact model, movement planner inputs | Represent a transient produced-unit/factory contact or equivalent per-mover exception until drive-out completes | After spawn, the produced vehicle's first path or forced drive track may evaluate factory cells with row skip; a second nearby vehicle does not inherit it | `war_factory_exit_contact_is_per_mover_not_global` | Do not model this as a static factory property |
| `ClearBibArea` is a WeaponsFactory scatter at `foundation + ExitList[10] + (-1,0)` and is separate from passability. | `0x00449540`, `0x0044D880` | missing/unchecked | future WF door/drive-out mission surface and scatter system | Scatter blockers from the front-door cell, but keep passability determined by live `Can_Enter_Cell` contact/row logic | Place a friendly unit on the front-door cell while producing a tank; factory attempts scatter before drive-out, but the produced tank's own exit permission still comes from contact row skip | `war_factory_clear_bib_area_scatters_without_static_unblock` | Do not gate scatter on `Bib=yes`; do not use scatter as the passability exception |

### Negative Facts / Do Not Do

- Do not implement `NumberImpassableRows=1` as a static PathGrid shrink for stock WFs. Evidence: contact-vector check and `FUN_00458A00` call in `UnitClass::Can_Enter_Cell @ 0x0073F58A..0x0073F5A9`; Active in YR: Yes/Conditional on contact.
- Do not let an unrelated unit path through `GAWEAP/NAWEAP/YAWEAP` interior columns merely because the building has `NumberImpassableRows=1`. Evidence: `DynamicVectorClass::Contains @ 0x0065AD50` must find the building in the mover's contacts; Active in YR: Yes.
- Do not confuse HasBib east-edge relaxation with `NumberImpassableRows`. Evidence: HasBib branch at `0x0073F7D3` uses `(1,0)` probe, while row helper uses `BuildingType+0x1620`; Active in YR: Yes.
- Do not gate `ClearBibArea` on `Bib=yes`. Evidence: `0x00449540` checks `Type+0x16BD WeaponsFactory`; Active in YR: Yes.
- Do not use `ClearBibArea` as the produced-unit entry test. Evidence: it finds/scatters nearest objects and returns a scatter result; it does not dispatch produced-unit `Can_Enter_Cell`; Active in YR: Yes.

### Remaining Uncertainty

- Exact stock 5x3 `ExitList[10]` pair remains unrecovered; the verified formula is `foundation_origin + (entry10.x - 1, entry10.y)`.
- The exact sender that clears the WF exit contact at final drive-out completion was not traced; this does not affect the verified presence of contact during the exit/row-skip window.
- The Rust implementation may already have partial contact-like state elsewhere; this report only scanned the named surfaces enough to size the delta.

### Stale Docs / Follow-up Docs

- `docs/research/BIB_SYSTEM_GHIDRA_REPORT.md`: replace wording that says `DAT_0089F690` magnitude/east edge still needs runtime verification with: "`Foundation_direction_table_init @ 0x0049F2F0` initializes `DAT_0089F690` to `(1,0)`, so the HasBib branch probes the east neighbor; this is separate from `NumberImpassableRows`."
- `docs/fidelity-checks/refinery-placement-bib.md`: replace the final "does not cover" line "`The Can_Enter_Cell HasBib edge-relaxation (see N1 above - needs runtime data)`" with: "`The HasBib east-edge relaxation is covered in N1; war-factory `NumberImpassableRows` contact row-skip is a separate live `Can_Enter_Cell` exception.`"
- `docs/research/BUILDING_PATH_BLOCKING_PASSABILITY_DISCREPANCY_GHIDRA_REPORT.md`: replace wording that says row-count reachability is not exhausted for war factories with: "`For stock land war-factory produced vehicles, row-count reachability is confirmed: successful exit creates reciprocal RadioClass contact, then `UnitClass::Can_Enter_Cell` uses `NumberImpassableRows=1` to skip same-building cells at `x >= origin+1` for that contacted mover.`"

## Sources

- Ghidra decompile: `0x00443C60` `BuildingClass::ExitObject_Main`
- Ghidra decompile: `0x0044D880` BuildingClass mission slot 26 / WF vehicle eject
- Ghidra decompile: `0x00449540` `BuildingClass::ClearBibArea`
- Ghidra decompile: `0x0073F0A0` `UnitClass::Can_Enter_Cell`
- Ghidra assembly context: `0x0073F58A..0x0073F5A9`, `0x0073F7D3`, `0x0073FA87`
- Ghidra decompile: `0x00458A00` `NumberImpassableRows` helper
- Ghidra decompile: `0x0065AD50` `DynamicVectorClass::Contains`
- Ghidra decompile: `0x0065A970`, `0x0065AAA0`, `0x0065A820` RadioClass helpers
- Ghidra decompile: `0x006F4AB0` `TechnoClass::Receive_Radio`
- Ghidra decompile: `0x00737430` `UnitClass::Receive_Radio`
- Ghidra decompile: `0x0049F2F0` `Foundation_direction_table_init`
- `ini/rulesmd.ini`
- `ini/artmd.ini`
- `docs/research/BUILDING_GETDOCKCELLFOROBJECT_STOCK_WAR_FACTORY_EXIT_GHIDRA_REPORT.md`
- `docs/research/BUILDING_PATH_BLOCKING_PASSABILITY_DISCREPANCY_GHIDRA_REPORT.md`
- `docs/research/ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md`
