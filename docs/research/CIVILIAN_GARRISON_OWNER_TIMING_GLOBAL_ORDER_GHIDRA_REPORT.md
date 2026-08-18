# Civilian Garrison Owner Timing Global Order - Ghidra Research Report

Date: 2026-05-27

Target: `CIVILIAN_GARRISON_OWNER_TIMING_GLOBAL_ORDER`

## Target Question

Determine exact global-object/update ordering around infantry entry into a neutral/civilian `CanBeOccupied` building and the later `BuildingClass::Update -> CheckAutoSellOrCivilian @ 0x00458200` ownership transfer.

Specific answer: ownership transfer can occur later in the same global frame if the infantry's entry commit runs before the target building's `vtable+0x5C` update in the current `LogicClass::PerTickUpdate` forward object-vector pass. If the building's update already ran earlier in that pass, ownership transfer waits until the building's next eligible `BuildingClass::Update` pass, normally the next game frame. The native rule is "next building reconciliation pass after the occupant vector changes," not "always same frame" and not "always next frame."

Active in YR: Yes. `Main_Tick` calls `LogicClass::PerTickUpdate` with `ECX=0x87F778` at `0x0055DC99..0x0055DC9E`; this is the standard YR per-tick object scheduler.

## Non-goals

- Do not redo `AddGarrisonOccupant` internals beyond confirming it is called from infantry entry and does not perform the ownership transfer.
- Do not redo `CheckAutoSellOrCivilian` owner/revert selection beyond the timing-sensitive call site and transfer branch.
- Do not trace exact runtime vector indices for a particular retail map or replay.
- Do not inspect sell/destruction/ejection, garrison fire, BState/visuals, or tank bunker `+0x2E4`.
- Do not modify Rust, INI, Ghidra state, or non-research docs.

## Evidence Needed To Mark COMPLETE

- Active-YR evidence that `LogicClass::PerTickUpdate` runs from the main tick path.
- Scheduler evidence for the core object loop: forward index order, object pointer load, `vtable+0x5C` call, post-call index increment, and live count reload.
- Evidence that `BuildingClass::Update` is the building `vtable+0x5C` target and calls `CheckAutoSellOrCivilian` under the `CanBeOccupied` guard.
- Evidence that infantry garrison entry is committed from the infantry side before return from `InfantryClass::PerCellProcess`, by calling `CanDock` then `AddGarrisonOccupant`.
- Evidence that the ownership transfer itself is inside `CheckAutoSellOrCivilian`, not inside the infantry entry stack.
- Rust-facing handoff describing same-frame vs next-frame order dependency without requiring runtime vector logging.

## Stop Conditions

- Stop once the scheduler order can answer whether same-frame transfer is possible.
- Stop if remaining uncertainty is only concrete object-vector index identity for a particular scenario; that requires runtime logging and is not necessary for the implementation contract.
- Stop before entering unrelated garrison lifecycle slices.

## Findings

### 1. Main LogicClass Object Scheduler Is A Live Forward Vector Walk

Active in YR: Yes.

`Main_Tick` reaches `LogicClass::PerTickUpdate @ 0x0055AFB0` with the LogicClass singleton in `ECX`: `0x0055DC99` loads `ECX=0x87F778`, then `0x0055DC9E` calls `0x0055AFB0`.

The main object loop uses increasing index order. Assembly at `0x0055B5FB..0x0055B619`:

- `0x0055B5FF`: `XOR ESI,ESI` starts index at `0`.
- `0x0055B608`: loads item-array pointer from `[EDI+0x04]`.
- `0x0055B60B`: loads object pointer from `[items + ESI*4]`.
- `0x0055B610`: calls object `vtable+0x5C`.
- `0x0055B613`: reloads live count from `[EDI+0x10]`.
- `0x0055B616..0x0055B619`: increments `ESI`, compares to the reloaded count, and loops.

Material consequence: this is a single live forward object pass. There is no class-ordered "all infantry, then all buildings" phase and no pass-entry count snapshot for the main LogicClass object vector.

### 2. A Building's Reconciliation Runs Only When Its Own vtable+0x5C Update Is Reached

Active in YR: Yes for buildings; conditional on `CanBeOccupied=yes` for this helper call.

`BuildingClass::Update @ 0x0043FB20` is the building update body called through the scheduler's `vtable+0x5C` dispatch. In the relevant late section, assembly at `0x0044019D..0x004401AF` reads `BuildingType+0x157B`, tests it, and calls `CheckAutoSellOrCivilian @ 0x00458200` only when the byte is nonzero:

- `0x0044019D`: loads `this->Type`.
- `0x004401A3`: reads byte `[Type+0x157B]`.
- `0x004401A9..0x004401AB`: tests and skips if zero.
- `0x004401AD..0x004401AF`: calls `0x00458200`.

Material consequence: a target building cannot transfer ownership until its own scheduler turn reaches this guarded call. No earlier scheduler-side hook reconciles all buildings after every infantry entry.

### 3. Infantry Entry Commits Occupant State Before Returning From The Infantry-Side PerCellProcess Path

Active in YR: Yes for normal infantry entering `CanBeOccupied` buildings.

`InfantryClass::PerCellProcess @ 0x00519630` contains the normal garrison entry commit path. Decompile shows mission `8`, target RTTI `6` building, current-cell building equality, `BuildingClass::CanDock`, and then `BuildingClass::AddGarrisonOccupant`.

Assembly evidence:

- `0x005196C8..0x005196CF`: looks up building in current cell and compares it to the target building pointer.
- `0x005196D1..0x005196D4`: pushes infantry and calls `BuildingClass::CanDock @ 0x00457CE0` on the building.
- `0x005196D9..0x005196DB`: branches on the `CanDock` result.
- `0x0051972A..0x0051972D`: pushes the building and calls `BuildingClass::AddGarrisonOccupant @ 0x00522910`.
- `0x00519732..0x00519739`: returns from the per-cell handler after the call.

Material consequence: the occupant vector mutation occurs during the infantry object's scheduler turn, before that object returns to the main `LogicClass::PerTickUpdate` loop.

### 4. AddGarrisonOccupant Still Does Not Transfer Owner

Active in YR: Yes.

Fresh decompile of `BuildingClass::AddGarrisonOccupant @ 0x00522910` shows limbo/append/first-occupant side effects and no `ChangeOwner` call. The only owner transfer observed in this target remains in `CheckAutoSellOrCivilian`.

Material consequence: if the building has already passed its `CheckAutoSellOrCivilian` call in the current scheduler pass, there is no retroactive transfer later in the same pass unless some other path calls the building update again. No such second call was found in this scoped evidence.

### 5. CheckAutoSellOrCivilian Performs The Transfer When The Building Later Sees The New Occupant Count

Active in YR: Yes for scoped `CanBeOccupied` / `Type+0x634 == -1` buildings.

`CheckAutoSellOrCivilian @ 0x00458200` first gates on `BuildingType+0x634 == -1`. Assembly `0x00458205..0x00458212` loads `this->Type`, compares `[Type+0x634]` to `-1`, and returns on mismatch.

The occupied-civilian transfer branch re-checks occupant count and owner later in the same helper:

- `0x004582F3..0x004582F5`: calls `vtable+0x408` occupant count.
- `0x004582FD`: requires count greater than zero.
- `0x004582FF..0x00458305`: compares current owner against the resolved Civilian house pointer.
- `0x00458307..0x00458309`: calls the animation refresh helper.
- `0x0045830E..0x0045831A`: loads first occupant from `Building+0x688` and then that occupant's owner at `+0x21C`.
- `0x00458316..0x00458323`: pushes announce flag `0`, pushes new owner, and calls building `vtable+0x3D4` (`ChangeOwner`).

Material consequence: same-frame transfer is possible only if this code runs after the occupant vector append in that same `LogicClass::PerTickUpdate` pass.

### 6. Revert Uses The Resolved Civilian House, Not A Stored Original Owner

Active in YR: Yes for the same scoped `CanBeOccupied` / `Type+0x634 == -1` reconciliation path.

`CheckAutoSellOrCivilian @ 0x00458200` resolves the Civilian-side house inside the helper each time it runs:

- `0x00458230`: loads `g_HouseClass_Array_Count`.
- `0x00458236..0x0045823D`: prepares the Civilian side lookup and calls `FUN_006A46D0`.
- `0x00458248..0x00458265`: scans `g_HouseClass_Array`, comparing each house's `HouseType/Country +0xBC` side index to the Civilian side index.
- `0x00458265..0x0045826B`: stores the matched `HouseClass*` in `EBX`.
- `0x0045827E..0x004582EB`: empty, non-Civilian-owned buildings play pre-revert cues, refresh anim state, then call `ChangeOwner(EBX, 0)`.

No per-building "original owner" pointer/field is read by the empty-revert branch. The branch uses the freshly resolved Civilian house pointer, not the building's map-authored owner and not any saved owner from the boarding path.

Material consequence: Rust's `garrison_original_owner` field is not native state for this civilian-garrison revert path. It can produce drift for map-authored `Special` or custom passive-owner garrisonable buildings that native would return to the Civilian-side house.

## Lifecycle Answer

The lifecycle is order-dependent:

1. Infantry object receives its `vtable+0x5C` scheduler turn.
2. On cell arrival, its per-cell handling reaches `InfantryClass::PerCellProcess @ 0x00519630`.
3. `CanDock` succeeds, then `AddGarrisonOccupant @ 0x00522910` appends/limbos the infantry and returns. Ownership is still Civilian/neutral at this point.
4. The main `LogicClass::PerTickUpdate` loop resumes at the next object-vector index.
5. If the target building's object-vector index is still ahead in this same pass, its `BuildingClass::Update` later calls `CheckAutoSellOrCivilian`, sees the occupant count, and transfers owner in the same global frame.
6. If the target building's object-vector index was already processed earlier in the pass, it will not see the new occupant until its next `BuildingClass::Update` turn, normally next frame.

No fixed one-frame delay is proven by the binary. No guaranteed same-frame transfer is proven either. The native mechanism is "first building reconciliation pass after occupant vector mutation."

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Required Rust effect | Proposed test |
|---|---|---|---|---|
| Ownership transfer is not part of boarding; it is part of the target building's later update/reconciliation turn. Active in YR: Yes. | `0x0051972D` calls `AddGarrisonOccupant`; `0x004401AF` calls `CheckAutoSellOrCivilian`; transfer call at `0x00458323`. | `src/sim/passenger.rs::tick_boarding` transfers immediately and reports ownership changed from the boarding phase. | Remove owner mutation from boarding. Add a building-garrison reconciliation stage that can run after an occupant append only when the building's own update turn occurs. | `garrison_owner_not_changed_during_boarding_call` |
| Same-frame transfer depends on object-vector order: infantry entry before building update can transfer same frame; building before infantry waits until next building update. Active in YR: Yes. | Live forward loop `0x0055B608..0x0055B619`; building call `0x004401AF`; entry call `0x0051972D`. | `Simulation::advance_tick` uses staged phases; passenger boarding is a global phase after combat, not a live object-vector turn. | If implementing a full native scheduler, process active objects in live vector order. If not yet implementing full scheduler, model garrison reconciliation with explicit tests for "before building reconciliation" vs "after reconciliation" rather than claiming a universal tick delay. | `garrison_owner_transfers_same_frame_when_building_update_after_entry` and `garrison_owner_waits_next_frame_when_building_update_before_entry` |
| The reconciliation branch reads first occupant slot `Building+0x688[0]` owner at `+0x21C` when current building owner is the resolved Civilian house. Active in YR: Yes. | `0x0045830E..0x00458323`. | Rust stores/restores `garrison_original_owner`; this target did not re-open revert ownership except as a dependency. | Keep transfer owner sourced from occupant slot 0 during building reconciliation, not from the boarding passenger loop's current entity. | `garrison_reconciliation_uses_first_occupant_owner` |
| Empty-garrison revert resolves the Civilian-side house in `CheckAutoSellOrCivilian`; no native per-building original-owner field participates in this path. Active in YR: Yes. | `0x00458230..0x0045826B` resolves Civilian house; `0x0045827E..0x004582EB` calls `ChangeOwner(civilian_house, 0)`. | `src/sim/game_entity.rs` stores `garrison_original_owner`; `tick_unloading` restores it. | Revert empty captured civilian garrisons to the resolved Civilian-side house from the building reconciliation path, not to a saved original owner. | `empty_captured_garrison_reverts_to_civilian_house_not_original_owner` |

## Negative Facts / Do Not Do

- Do not write "ownership transfers one tick later" as an unconditional native rule. Active in YR: Yes; scheduler order permits same-frame transfer if the building has not updated yet.
- Do not write "ownership transfers same tick" as an unconditional native rule. Active in YR: Yes; if the building already updated earlier in the live vector pass, the transfer waits.
- Do not transfer ownership in `AddGarrisonOccupant` or Rust's boarding commit. Active in YR: Yes; `0x00522910` has no `ChangeOwner`, while `0x00458200` has the transfer.
- Do not use a per-building original-owner field as native evidence for civilian garrison revert. Active in YR: Yes; `0x00458200` resolves the Civilian house and reverts to it.
- Do not model this as an infantry-class phase followed by a building-class phase. Active in YR: Yes; the scheduler is a single live forward object vector calling each object's `vtable+0x5C`.
- Do not rely on `EntityStore` sorted IDs or a pre-collected snapshot as if it were gamemd's active LogicClass vector. Active in YR: Yes; native uses `LogicClass+0x04/+0x10`, live count reload, and vector-order side effects.

## Remaining Uncertainty

- Concrete retail map/replay object-vector indices for a specific infantry/building pair are runtime state, not statically determined here. A runtime debugger trace at `0x0055B608` would be needed to say which stock scenario instances transfer same-frame.
- Save/load reconstruction of the LogicClass vector was not traced in this target. It can affect concrete ordering after load, but not the mechanism.
- This report does not design a full Rust LogicClass scheduler migration. It only states the garrison owner timing contract that such a scheduler or interim reconciliation phase must preserve.

## Proposed Rust Test Names

- `garrison_owner_not_changed_during_boarding_call`
- `garrison_owner_transfers_same_frame_when_building_update_after_entry`
- `garrison_owner_waits_next_frame_when_building_update_before_entry`
- `garrison_reconciliation_uses_first_occupant_owner`
- `empty_captured_garrison_reverts_to_civilian_house_not_original_owner`
- `garrison_reconciliation_runs_once_per_building_update_turn`

## Stale Doc Wording

Replace wording that says "ownership transfers one tick later" with:

> Ownership transfer is deferred until the target building's next `BuildingClass::Update -> CheckAutoSellOrCivilian` reconciliation pass after `AddGarrisonOccupant` mutates the occupant vector. Because `LogicClass::PerTickUpdate` walks a live forward object vector, that reconciliation can occur later in the same global frame if the building has not updated yet; otherwise it occurs on a later frame.

Replace wording that says Rust should "delay by one tick" with:

> Rust should avoid owner mutation in boarding and perform garrison owner transfer from a building reconciliation/update stage. Exact same-frame vs next-frame behavior depends on whether the building reconciliation stage is still ahead of the infantry entry in the modeled object order.

## Sources

- Ghidra read-only decompile: `InfantryClass__PerCellProcess @ 0x00519630`, `BuildingClass__Update @ 0x0043FB20`, `BuildingClass__CheckAutoSellOrCivilian @ 0x00458200`, `BuildingClass__AddGarrisonOccupant @ 0x00522910`, `InfantryClass__AI @ 0x0051BAB0`, `FootClass__AI`, `FootClass__Locomotion_AI`.
- Ghidra read-only assembly context: `0x0055DC99..0x0055DC9E`, `0x0055B5FB..0x0055B619`, `0x005196C8..0x00519739`, `0x0044019D..0x004401AF`, `0x00458200..0x00458329`.
- Existing research used as navigation/corroboration: `LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`, `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`, `CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md`, `GARRISON_CANDOCK_CANGARRISON_ENTRY_GATES_GHIDRA_REPORT.md`.
- Rust read-only surfaces: `src/sim/passenger.rs::tick_boarding`, `src/sim/world/mod.rs::Simulation::advance_tick`, `src/sim/game_entity.rs::garrison_original_owner`.

## Status

COMPLETE for the target scheduling question and implementation handoff. Remaining uncertainty is limited to concrete runtime vector indices for a specific scenario, which requires debugger logging and does not change the resolved lifecycle rule.
