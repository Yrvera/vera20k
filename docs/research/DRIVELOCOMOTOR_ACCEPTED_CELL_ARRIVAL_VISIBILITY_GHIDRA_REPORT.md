# DriveLocomotor Accepted-Cell Arrival Visibility - Ghidra Research Report

**Address(es):** `0x004B0500`, `0x004B0F20`, `0x004AFC20`, `0x004D94B0`, `0x0065AE30`, `0x0065AD30`, `0x00737430`, `0x00739EC0`, `0x006F4AB0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** DriveLocomotion / PathType visibility order after a stock refinery `MOVE_TO_CELL(0x12)` completes at the accepted NW+(3,1) cell.
**Non-Scope:** full locomotion physics, all `PerCellProcess` branches, full tick scheduler order, refinery admission arithmetic already settled by prior reports.
**Confidence:** High for locomotor-visible fields and `0x16` gates; Medium for cross-slot "first 0x15 wins" until the other swarm slots reconcile Mission_Enter and global tick order.
**Active in YR:** Yes for stock Drive-locomotor ground movement and stock refinery docking when the unit is using DriveLocomotion/piggyback ground approach.

## Working Notes

**Target question:** When a miner finishes `MOVE_TO_CELL(0x12)` at the accepted NW+(3,1) cell, what state is visible for current cell, `Is_Moving()`, destination, and path-valid/path-track state?

**Non-goals:** Do not redo accepted-cell vs `GetDockCoord` proof; do not decode full Drive physics, all `PerCellProcess` cases, or full scheduler tick order.

**Evidence needed to mark COMPLETE:** Decompile Drive arrival branch, `Is_Moving_Now`, `PathType::Has_Valid_Steps`, destination getter/setter, `UnitClass::Receive_Radio(0x16)`, and the relevant `PerCellProcess(2)` dock consumers.

**Stop conditions:** Stop once accepted-cell arrival visibility is proven and remaining "which `0x15` source wins first" is isolated to Mission_Enter/tick-order slots.

## 1. Overview

Drive arrival does not move the unit from the accepted refinery cell NW+(3,1) to the stock `GetDockCoord` cell NW+(2,1). When a drive track finishes, the unit's object coordinates have already been advanced to the accepted cell, Drive clears its active track/head-to fields, and `DriveLocomotionClass::Is_Moving_Now` can return false while `FootClass+0x5A4` still points at the refinery destination.

The same Drive arrival path calls owner vtable `+0x504`, which resolves to `UnitClass::PerCellProcess(2)`, before returning from the locomotor tick. That pass sees the accepted cell; for stock 4x3 refinery the `GetDockCoord` equality branch fails because accepted NW+(3,1) is not `GetDockCoord` NW+(2,1). A later synchronous `UnitClass::Receive_Radio(0x16)` can therefore observe "not moving, current cell still accepted, destination building still set" without requiring a physical pad-cell move first.

## 2. Class Layout / Key Offsets

| Offset / slot | Owner | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `owner+0x9C/0xA0/0xA4` | Object/Techno | current lepton coordinates; current cell derives from these | `ObjectClass::GetOccupiedCell @ 0x005F6960`, Drive track writes via vtable `+0x1B4` | Yes |
| `Foot+0x5A4` / `param_1[0x169]` | FootClass | NavCom/destination object; remains the refinery across accepted-cell arrival | `0x004D94B0`, `0x0065AD30`, `0x00737430` | Yes |
| Path array `+0xE4`, count `+0xE8` | Foot/PathType | `PathType::Has_Valid_Steps` scans nonzero path entries | `0x0065AE30` | Yes |
| Drive `+0x34/+0x38/+0x3C` | DriveLocomotion | active destination coordinate for drive track | cleared on arrival in `0x004B0F20` | Yes |
| Drive `+0x40/+0x44/+0x48` | DriveLocomotion | active head-to/intermediate coordinate | cleared on arrival in `0x004B0F20` | Yes |
| Drive `+0x58` | DriveLocomotion | active drive track index; `-1` means no active track | set `-1` on arrival in `0x004B0F20` | Yes |
| Drive `+0x5C` | DriveLocomotion | track step index | reset to `0` on arrival in `0x004B0F20` | Yes |
| Drive `+0x63` | DriveLocomotion | head-to valid flag | cleared when head-to coord is cleared | Yes |
| `Techno+0x418` | Techno/Radio | entered/contact flag set by radio `0x18`, cleared by `0x19` | `0x006F4AB0` case `0x18/0x19` | Yes |
| Unit vtable `+0x504` | UnitClass | `PerCellProcess` dispatch from Drive arrival | call site inside `0x004B0F20`; vtable data xref `0x007F5DFC -> 0x00739EC0` | Yes |
| ILocomotion vtable `+0x10` | DriveLocomotion | `Is_Moving_Now` when active locomotor is Drive | `0x004AFC20`, called by `0x00737430` case `0x16` | Yes |

## 3. Core Logic

### 3.1 `PathType::Has_Valid_Steps` is a path-array query, not a Drive-track query

`PathType__Has_Valid_Steps @ 0x0065AE30` returns true only if `count > 0` and at least one entry in the path array at `+0xE4` is nonzero. It does not inspect Drive `+0x34`, `+0x40`, `+0x58`, or the current lepton position.

**Active in YR:** Yes. This helper is called by Foot/Unit radio and destination logic, including `UnitClass::Receive_Radio(0x0E/0x16)` paths.

### 3.2 Drive accepted-cell arrival updates current cell before it clears movement state

In `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20`, the track end branch is the zero-delta entry branch: `(track_dx == 0 && track_dy == 0 && step_index != 0)`. In that branch the function:

1. Computes whether the current object lepton cell already equals the drive head-to cell (`Drive+0x40/+0x44`).
2. Calls owner vtable `+0x1B4` and `+0x1CC` around coordinate/occupancy updates.
3. Clears Drive `+0x40/+0x44/+0x48` to null and clears `+0x63`.
4. Sets Drive `+0x58 = -1` and `+0x5C = 0`.
5. If `Foot+0x5A4` destination exists and the owner's current cell equals that destination object's coordinate cell, with Z difference `< g_DriveHeightStep * 2`, clears Drive `+0x34/+0x38/+0x3C` to null.
6. Calls owner vtable `+0x18C(2)`.
7. Calls owner vtable `+0x504()`, which for UnitClass is `UnitClass::PerCellProcess`.

For a refinery `0x12` accepted target, the destination object is the accepted `CellClass*` / destination target from the move assignment, and the unit's current cell is the accepted NW+(3,1) cell at this branch.

**Evidence:** `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20`, arrival branch in the decompile body spanning the zero-delta check through the `+0x504` call (`~0x004B15xx..0x004B1Bxx`); `ObjectClass::GetOccupiedCell @ 0x005F6960` derives occupied/current cell from `+0x9C/+0xA0/+0xA4`.

**Active in YR:** Yes. Drive-locomotor vehicles use this per-track processing every movement tick; no TS-only gate is present in the inspected branch.

### 3.3 `Is_Moving_Now` can be false immediately after arrival while destination remains non-null

`DriveLocomotionClass::Is_Moving_Now @ 0x004AFC20` returns true only if:

1. the local CDTimer still has remaining time; or
2. the locomotor's vtable `+0x10`/valid-step check is true, Drive `+0x40/+0x44/+0x48` is not null, and owner vtable `+0x538` returns positive movement budget.

It does not require `Foot+0x5A4 == 0` to return false. Therefore after `Process_Drive_Track` clears Drive head-to/track fields but leaves `Foot+0x5A4` pointing at the refinery, `Is_Moving_Now` can return false while the unit still has a destination.

**Evidence:** `DriveLocomotionClass::Is_Moving_Now @ 0x004AFC20`; `FootClass::Set_Destination_Internal @ 0x004D94B0` writes/clears `Foot+0x5A4`; the Drive arrival branch clears Drive fields, not `Foot+0x5A4`.

**Active in YR:** Yes for Drive-locomotor units. For CMIN this is active when Teleport locomotion is using Drive piggyback/ground approach.

### 3.4 Arrival-time `PerCellProcess(2)` runs before later Mission_Enter retry radio

Drive arrival calls owner vtable `+0x504()` inside `Process_Drive_Track` after clearing active Drive movement state. For a UnitClass owner this dispatches `UnitClass::PerCellProcess @ 0x00739EC0`.

The stock refinery `GetDockCoord` branch inside `PerCellProcess(2)` requires:

1. `arg == 2`;
2. current mission is `7` or `0x19`;
3. `FootClass::GetDestination(0)` is a building (`WhatAmI == 6`);
4. current unit cell, computed from vtable `+0x48`, equals destination building vtable `+0xA8` / `GetDockCoord`.

At accepted-cell arrival for a stock 4x3 refinery, current cell is NW+(3,1) and `GetDockCoord` is NW+(2,1), so this equality branch fails. The unit does not need to physically move to NW+(2,1) merely because this arrival-time `PerCellProcess` ran.

**Evidence:** `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` calls owner `+0x504`; `UnitClass::PerCellProcess @ 0x00739EC0` GetDockCoord branch; `BuildingClass::GetDockCoord @ 0x00447B20` stock 4x3 refinery result from prior reports.

**Active in YR:** Yes. The `PerCellProcess` path is live; the equality is conditional and fails for the settled stock accepted-cell vs `GetDockCoord` split.

### 3.5 `0x16` can observe stopped-at-accepted-cell state and send/prepare `0x15`

`UnitClass::Receive_Radio @ 0x00737430` case `0x16`:

1. delegates to `FootClass::Receive_Radio`, which falls through to `TechnoClass::Receive_Radio`; Techno case `0x16` transmits `0x18`, and Techno case `0x18` sets `Techno+0x418`.
2. if `Foot+0x6AF == 0` and primary facing is not `0x4000`, calls locomotor vtable `+0x4C(0x4000)` and returns `1`.
3. otherwise calls active locomotor vtable `+0x10` / `Is_Moving_Now`.
4. if not moving, `FootClass::GetDestination(0)` is non-null, `Techno+0x418 != 0`, destination `WhatAmI == 6`, and current mission is `7`, it sends radio `0x15` to the destination building.

This gate does not compare current cell to `GetDockCoord`. It is satisfied by the stopped accepted-cell state once the facing/contact/destination/mission gates are also satisfied.

**Evidence:** `UnitClass::Receive_Radio @ 0x00737430` case `0x16`; `TechnoClass::Receive_Radio @ 0x006F4AB0` cases `0x16`, `0x18`, and `0x19`; `DriveLocomotionClass::Is_Moving_Now @ 0x004AFC20`.

**Active in YR:** Yes for stock refinery `0x18/0x16` handoff. The `0x15` send is conditional on facing already being east/aligned; if not aligned, the first `0x16` only starts the turn.

### 3.6 Scoped correction: `PerCellProcess` has a second contact-flag adjacent-building `0x15` branch

While checking whether `PerCellProcess` can see accepted-cell state, this report also found a scoped correction to the shorthand "PerCellProcess sends `0x15` only via GetDockCoord equality." Later in `UnitClass::PerCellProcess @ 0x00739EC0`, if `Techno+0x418 != 0`, destination is a building, and mission is `7`, it checks the cell one row north of the current cell for the destination building and can transmit `0x15`.

That branch is not available during the first Drive-arrival `PerCellProcess` pass unless `0x18` has already set `Techno+0x418`. In the standard accepted-cell arrival flow, `0x18` is sent by the later building admission handoff, so this report does not claim the adjacent-building branch wins first. It only corrects the narrower stale wording.

**Evidence:** `UnitClass::PerCellProcess @ 0x00739EC0`, block after the GetDockCoord equality branch that gates on `param_1->field_0x418`, destination building, current mission `7`, and `current_cell.y - 1`.

**Active in YR:** Conditional. The code is live; the branch requires the contact-entered flag set by radio `0x18`.

## 4. INI Keys

No INI keys are newly decoded in this slice. Stock activity relies on already-settled INI defaults:

| Key | Stock role | Evidence | Active in YR |
|---|---|---|---|
| `[CMIN]/[HARV] Harvester=yes` | makes the miner use UnitClass harvester/dock mission paths | prior miner/refinery reports and `rulesmd.ini` | Yes |
| `[GAREFN]/[NAREFN] DockUnload=yes`, `Refinery=yes` | enables stock refinery admission and destination building gates | prior refinery flag reports and `rulesmd.ini` | Yes |

## 5. Integration Points

| Function / point | Role | Evidence | Active in YR |
|---|---|---|---|
| `FootClass::Receive_Radio(0x12) @ 0x004D8FB0` | assigns accepted-cell destination or returns already-there | prior `RADIO_0X12...` report | Yes |
| `FootClass::Set_Destination_Internal @ 0x004D94B0` | writes `Foot+0x5A4` and calls locomotor head-to-coord | decompile | Yes |
| `DriveLocomotionClass::Process @ 0x004B0500` | invokes drive movement/track processing and returns final `Is_Moving` state | decompile | Yes |
| `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` | performs accepted-cell arrival, clears Drive track/head-to, calls `PerCellProcess` | decompile | Yes |
| `DriveLocomotionClass::Is_Moving_Now @ 0x004AFC20` | visibility queried by `0x16` | decompile | Yes |
| `UnitClass::Receive_Radio(0x16) @ 0x00737430` | facing sync / direct `0x15` source when stopped | decompile | Yes |
| `UnitClass::PerCellProcess @ 0x00739EC0` | arrival-time dock consumer; GetDockCoord branch fails at accepted cell | decompile | Yes |
| `TechnoClass::Receive_Radio @ 0x006F4AB0` | `0x18` sets `Techno+0x418`; `0x19` clears it | decompile | Yes |

## 6. Current Rust Implementation Status

Current Rust surfaces scanned:

| Surface | Current behavior | Delta / risk |
|---|---|---|
| `src/sim/components.rs:196` `MovementTarget` | `movement_target.is_some()` is the Rust moving/destination proxy; `bypass_grid` supports direct refinery-footprint moves | Needs split between "Drive moving false" and "destination building still non-null" if generic radio parity is modeled |
| `src/sim/movement/movement_tick.rs:1110` | movement completion clears `entity.movement_target = None`, clears `drive_track`, and sets locomotor phase idle | Similar to Drive `Is_Moving false`, but Rust also loses generic destination identity unless miner state carries it |
| `src/sim/miner/miner_dock_sequence.rs:613` `phase_mission_enter` | if at accepted cell and not moving, marks contact entered and goes `Linked` | Directionally matches `0x12 already-there -> 0x18/0x16` timing, but compresses contact flag, facing sync, and `0x15` handoff |
| `src/sim/miner/miner_dock_sequence.rs:680` `phase_awaiting_accepted_cell` | waits until movement target clears, refreshes snapshot position, returns to `MissionEnter` | Matches the need for a later admission retry after accepted-cell arrival |
| `src/sim/miner/miner_dock_sequence.rs:700` `phase_linked` | uses `pad` for snapshot/link bookkeeping and starts display/facing handoff | Risk: do not treat this as proof that gamemd physically moved the unit to `GetDockCoord`; binary allows `0x16`/`0x15` from stopped accepted-cell state |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Drive arrival branch clears Drive track/head-to | verified | `0x004B0F20` decompile | none |
| Current cell source | verified | `0x005F6960`, owner coordinate writes in `0x004B0F20` | exact occupancy helper names remain decompiler-label dependent |
| `Is_Moving_Now` false after arrival | verified | `0x004AFC20` plus Drive arrival clear | none |
| `Foot+0x5A4` destination remains through arrival | verified | `0x004D94B0`, `0x0065AD30`, no clear in Drive arrival branch | none |
| `PathType::Has_Valid_Steps` semantics | verified | `0x0065AE30` | exact path-vector element meanings outside scope |
| Arrival-time `PerCellProcess(2)` dispatch | verified | `0x004B0F20` owner `+0x504`; `0x00739EC0` vtable data xref | none |
| `PerCellProcess` GetDockCoord branch at accepted cell | verified | `0x00739EC0` and prior GetDockCoord reports | none |
| `0x16` direct `0x15` gates | verified | `0x00737430`, `0x006F4AB0`, `0x004AFC20` | exact repeated `0x16` timing belongs to slot 1/2 |
| Global "which `0x15` wins first" | touched-not-exhausted | this report proves locomotor-visible state only | reconcile with other swarm slots |
| Current Rust surfaces | verified | files/lines in section 6 | implementation work separate |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does Drive arrival physically move the unit from accepted NW+(3,1) to GetDockCoord NW+(2,1)? -> No evidence in scoped Drive path; arrival clears Drive track at the current accepted destination cell, not at GetDockCoord.` (evidence: `0x004B0F20`; prior accepted-cell and GetDockCoord reports)
- `[RESOLVED] OQ-02 - Is current cell updated before stopped state is visible? -> Yes; Drive arrival updates object coordinates/occupancy before clearing track/head-to and returning.` (evidence: `0x004B0F20`, `0x005F6960`)
- `[RESOLVED] OQ-03 - Can `Is_Moving_Now` return false while `Foot+0x5A4` destination remains non-null? -> Yes; `Is_Moving_Now` checks timers/Drive head-to/valid movement budget, not destination non-null by itself.` (evidence: `0x004AFC20`, `0x004D94B0`)
- `[RESOLVED] OQ-04 - Is `PathType::Has_Valid_Steps` the same as Drive track/head-to validity? -> No; it scans the Foot/PathType path array only.` (evidence: `0x0065AE30`)
- `[RESOLVED] OQ-05 - Does Drive arrival call `PerCellProcess`? -> Yes, owner vtable `+0x504` is called after movement-state clearing in the arrival branch.` (evidence: `0x004B0F20`, `0x00739EC0` vtable data)
- `[RESOLVED] OQ-06 - Does arrival-time `PerCellProcess` GetDockCoord branch pass at accepted cell for stock refinery? -> No; current cell NW+(3,1) differs from stock `GetDockCoord` NW+(2,1).` (evidence: `0x00739EC0`; prior `BUILDINGCLASS_GETDOCKCOORD...` reports)
- `[RESOLVED] OQ-07 - Can `0x16` use stopped accepted-cell state? -> Yes, its direct `0x15` branch checks `Is_Moving == false`, contact flag, destination building, and mission 7; it does not check GetDockCoord equality.` (evidence: `0x00737430`)
- `[RESOLVED] OQ-08 - What sets the contact flag used by `0x16` and later PerCellProcess branches? -> Techno radio `0x18` sets `Techno+0x418`; `0x19` clears it.` (evidence: `0x006F4AB0`)
- `[RESOLVED] OQ-09 - Is the first `0x16` always a `0x15` sender? -> No; if facing is not `0x4000`, it calls locomotor `+0x4C(0x4000)` and returns first.` (evidence: `0x00737430`)
- `[RESOLVED] OQ-10 - Does Rust keep movement false on accepted-cell arrival? -> Yes, completed movement clears `movement_target` and sets locomotor idle.` (evidence: `src/sim/movement/movement_tick.rs:1110`)
- `[RESOLVED] OQ-11 - Does Rust preserve a separate generic `Foot+0x5A4` destination after movement completion? -> Not generically; miner state carries refinery/dock phase, while `movement_target` is cleared.` (evidence: `src/sim/components.rs:196`, `src/sim/miner/miner_dock_sequence.rs:613`)
- `[DEFERRED] OQ-12 - Which `0x15` source wins first in full stock tick order?` (category: requires-different-system-context; reason: slot 5 proves locomotor visibility only; Mission_Enter retry and scheduler order are owned by other swarm slots; next-step-if-pursued: reconcile slots 1-4)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Drive arrival can leave the unit stopped on accepted NW+(3,1), with current cell accepted and `Foot+0x5A4` destination still the refinery. | `0x004B0F20`, `0x004AFC20`, `0x004D94B0` | partial: Rust clears `movement_target` and keeps refinery only in miner dock state | `src/sim/movement/movement_tick.rs`; `src/sim/miner/miner_dock_sequence.rs` | Preserve a "stopped at accepted cell, destination refinery still logically active" state for dock radio handoff. | Miner reaches accepted NW+(3,1); movement is idle; next MissionEnter can perform `0x12 already-there` and `0x18/0x16` without requiring NW+(2,1). | Do not force a physical move to `GetDockCoord` just because `GetDockCoord` exists. |
| Arrival-time `PerCellProcess(2)` runs before the later retry but fails the stock GetDockCoord equality at accepted cell. | `0x004B0F20`, `0x00739EC0`, `0x00447B20` via prior report | current Rust does not model this failed `PerCellProcess` pass explicitly | miner dock tests / possible future generic per-cell hook | A stopped accepted-cell arrival should not start unload through the GetDockCoord equality path alone. | `drive_arrival_at_refinery_accepted_cell_does_not_require_getdockcoord_before_0x16`: current cell accepted, GetDockCoord pad differs, no pad-move precondition. | Do not write tests that require current cell == NW+(2,1) before any `0x16` handling. |
| `UnitClass::Receive_Radio(0x16)` can send/prepare `0x15` from stopped accepted-cell state; first call may only start facing-to-east. | `0x00737430`, `0x006F4AB0`, `0x004AFC20` | current Rust compresses contact-entered, linked, and pivot phases | `src/sim/miner/miner_dock_sequence.rs:613`, `:700` | Split/verify facing-sync vs direct `0x15`: accepted-cell stopped state is sufficient for the 0x16 gate; facing alignment may delay the actual 0x15. | `refinery_0x16_stopped_at_accepted_cell_starts_turn_then_unload`: if not east, first 0x16 starts turn; once aligned, same accepted-cell state sends/receives unload handoff. | Do not model `0x16` as a physical accepted-to-pad mover. |

## 10. Negative Facts / Do Not Do

- Do not make Drive arrival move the unit from accepted NW+(3,1) to `GetDockCoord` NW+(2,1); no scoped Drive path does that. Evidence: `0x004B0F20`.
- Do not equate `Is_Moving == false` with `Foot+0x5A4 == 0`; destination can remain non-null after Drive head-to/track clears. Evidence: `0x004AFC20`, `0x004D94B0`.
- Do not treat `PathType::Has_Valid_Steps` as Drive track validity; it scans a separate path array. Evidence: `0x0065AE30`.
- Do not claim `0x16` itself checks `GetDockCoord`; it checks `Is_Moving`, contact flag, destination type, and mission. Evidence: `0x00737430`.
- Do not claim `PerCellProcess` has only one possible `0x15` branch; the contact-flag adjacent-building branch also exists, though its first-win timing is outside this slot. Evidence: `0x00739EC0`.

## 11. Remaining Uncertainty

- Full "which `0x15` source wins first" remains a parent reconciliation item across slots 1-4. This slot proves the Drive/PathType state that those sources can observe.
- Exact runtime facing timer cadence between first `0x16` turn-start and later `0x16` direct `0x15` is owned by the `0x16` timing slot.
- Exact natural replay tick/frame where Mission_Enter retries after accepted-cell arrival is owned by the Mission_Enter slot.

## 12. Stale Docs / Follow-up Docs

Replace any wording that says:

> `PerCellProcess` can send `0x15` only after current cell equals `GetDockCoord`.

with:

> `UnitClass::PerCellProcess(2)` has a `GetDockCoord` equality branch that sends `0x15` when current cell equals the destination building dock coordinate. It also has a later contact-flag branch that can send `0x15` when `Techno+0x418` is set and the destination building is found in the cell one row north of the unit. The stock accepted-cell arrival pass occurs before `0x18` sets `Techno+0x418`, so first-win timing must be resolved against Mission_Enter/0x16 order rather than by the GetDockCoord branch alone.

Replace any wording that implies:

> `Is_Moving == false` means no destination remains.

with:

> Drive `Is_Moving_Now` can be false after track/head-to clear while `Foot+0x5A4` still points at the refinery destination.

## Sources

- Ghidra decompile:
  - `DriveLocomotionClass::Process @ 0x004B0500`
  - `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20`
  - `DriveLocomotionClass::Is_Moving_Now @ 0x004AFC20`
  - `FootClass::Set_Destination_Internal @ 0x004D94B0`
  - `PathType::Has_Valid_Steps @ 0x0065AE30`
  - `FootClass::GetDestination @ 0x0065AD30`
  - `UnitClass::Receive_Radio @ 0x00737430`
  - `UnitClass::PerCellProcess @ 0x00739EC0`
  - `TechnoClass::Receive_Radio @ 0x006F4AB0`
  - `ObjectClass::GetOccupiedCell @ 0x005F6960`
- Prior docs:
  - `docs/research/RADIO_0X12_MOVE_TO_CELL_PAYLOAD_AND_TIMESTAMPS_GHIDRA_REPORT.md`
  - `docs/research/BUILDINGCLASS_GETDOCKCOORD_STOCK_REFINERY_BRANCH_GHIDRA_REPORT.md`
  - `docs/research/UNITCLASS_PERCELLPROCESS_DOCK_ARRIVAL_GETDOCKCOORD_GHIDRA_REPORT.md`
- Rust scanned:
  - `src/sim/components.rs`
  - `src/sim/movement/movement_tick.rs`
  - `src/sim/miner/miner_dock_sequence.rs`
