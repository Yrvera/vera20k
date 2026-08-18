# Civilian Garrison Ownership Transfer Timing - Ghidra Research Report

**Address(es):** `0x00458200` (`BuildingClass::CheckAutoSellOrCivilian`), `0x0043FB20` (`BuildingClass::Update`), `0x00522910` (`BuildingClass::AddGarrisonOccupant`), `0x00457DE0` (`BuildingClass::SellBuilding` occupant eject helper), `0x00449C30` (`BuildingClass::Sell`), `0x00442230` (`BuildingClass::ReceiveDamage` destruction case)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** owner-transfer/revert timing for `CanBeOccupied` civilian garrisons after `AddGarrisonOccupant`, original-owner tracking/restoration behavior, and last-occupant leave / sell / destruction ownership effects.
**Non-Scope:** `CanDock` entry gates except call flow into `AddGarrisonOccupant`; garrison fire, pips, power bonus, fire eligibility, full sell-cursor eligibility, and generic transport unload.
**Confidence:** High for the transfer/revert path and helper order; Medium for frame-level "same tick vs next tick" because the global object scheduler order was not re-decompiled in this slice.
**Active in YR:** Yes for stock `CanBeOccupied=yes` civilian garrisons; conditional on the building passing `Type+0x634 == -1` inside `CheckAutoSellOrCivilian`.

## 0. Working Notes Gate

- Target question: When does a neutral/civilian `CanBeOccupied` building transfer owner after `AddGarrisonOccupant`, how is the revert owner chosen, and what happens on last leave, sell, and destruction?
- Non-goals: Do not re-study `CanDock` gates, fire/pips/power, bunker paths, generic transports, or broad sell UI eligibility.
- Evidence needed to mark COMPLETE: decompile plus caller/disassembly evidence for transfer timing, original-owner/revert selection, last leave, sell, destruction, and YR activity.
- Stop conditions: no unresolved in-scope ownership questions; if frame scheduler ordering cannot be proven from this slice, record it precisely as Remaining Uncertainty.

## 1. Overview

`AddGarrisonOccupant @ 0x00522910` does not transfer building ownership. It limbos/appends the infantry to `Building+0x684` occupant vector and, on the first occupant, sets mission `2` and plays first-garrison sound/EVA side effects. Active in YR: Yes; `InfantryClass::PerCellProcess` reaches it after `CanDock` succeeds per the prior `GARRISON_CANDOCK_CANGARRISON_ENTRY_GATES_GHIDRA_REPORT.md`, and this function was freshly decompiled.

Actual ownership transfer and revert are lazy reconciliation in `BuildingClass::CheckAutoSellOrCivilian @ 0x00458200`, called from `BuildingClass::Update @ 0x0043FB20` only after the update code checks `Type[0x157B] != 0` (`CanBeOccupied`). Active in YR: Yes; decompile of `0x0043FB20` shows the `CanBeOccupied` guard and call, and `disassemble_bytes 0x0043FFF0..0x0044014F` covered the call region.

The binary does not preserve an arbitrary per-building "original owner" field for this revert. It recomputes the `Civilian` side index, finds the first `HouseClass` whose `CountryType+0xBC` matches that side, and uses that house for empty-building reverts. Active in YR: Yes; decompile `0x00458200` plus `FUN_006A46D0 @ 0x006A46D0`.

## 2. Class Layout / Key Offsets

| Offset / slot | Owner | Meaning in this slice | Evidence | Active in YR |
|---:|---|---|---|---|
| `BuildingType+0x157B` | building type | `CanBeOccupied`; `BuildingClass::Update` guard before calling reconciliation | `0x0043FB20` decompile; disassembly range `0x0043FFF0..0x0044014F`; INI examples in `rulesmd.ini` | Conditional on INI; Yes for stock UC buildings |
| `BuildingType+0x634` | building type | additional `CheckAutoSellOrCivilian` gate; must equal `-1` | `0x00458200` first branch | Conditional; Yes for scoped civilian/UC path |
| `Building+0x684` | building | occupant DynamicVector header | `0x00522910`, `0x00457DE0`, prior occupant report | Yes |
| `Building+0x688` | building | occupant item array pointer | `0x00458200` transfer reads `*Items[0]` | Yes |
| `Building+0x694` | building | occupant count via vtable `+0x408` and direct reads in eject helper | `0x00458200`, `0x00457DE0` | Yes |
| `Techno+0x21C` | infantry/techno | owner/house pointer; first occupant's owner is transfer target | `0x00458200` reads `Items[0]+0x21C` | Yes |
| `House+0x34 -> Country+0xBC` | house/country | side index used to identify the `Civilian` house | `0x00458200`; `0x006A46D0` side lookup | Yes |
| vtable `+0x3D4` | building | `ChangeOwner(new_house, announce_flag)`; reconciliation passes second arg `0` | `0x00458200`; `0x00448260` decompile confirms ChangeOwner signature consumes house + flag | Yes |

## 3. Core Logic

### 3.1 Entry adds occupant, not ownership

`BuildingClass::AddGarrisonOccupant @ 0x00522910` checks `InfantryType+0xEB4` (`Occupier`) and uses the `Assaulter` alternate branch only for the non-occupier case. For normal occupiers it calls infantry `Limbo` via vtable `+0xD4`, appends the infantry pointer to `Building+0x688[count]`, increments `Building+0x694`, recalculates threat/power via `FUN_0070F6E0`, and if the count is now `1`, sets building mission `2` and plays first-garrison sound/EVA for human player. Active in YR: Yes. Evidence: fresh decompile `0x00522910`.

No `ChangeOwner` call or `Building+Owner` write appears in this function. Active in YR: Yes. Evidence: fresh decompile `0x00522910`; prior CanDock report already established this negative fact.

### 3.2 Building update reconciliation

`BuildingClass::Update @ 0x0043FB20` calls `BuildingClass::CheckAutoSellOrCivilian()` after garrison fire/process-delayed-fire work when `this->Type[0x157B] != 0`. Active in YR: Yes. Evidence: fresh decompile `0x0043FB20`; disassembly range `0x0043FFF0..0x0044014F`; `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md` independently places the phase at `0x0044012C`.

`CheckAutoSellOrCivilian @ 0x00458200` first requires `*(int *)(Type+0x634) == -1`. If false, it returns without ownership reconciliation even when called from `CanBeOccupied` update. Active in YR: Conditional; this is the scoped civilian/UC gate. Evidence: fresh decompile and disassembly range `0x00458200..0x0045832F`.

Timing rule: transfer/revert happens on the building's next execution of this update phase after the occupant vector state changes, not inside the infantry boarding/ejection call stack. Existing docs often call this a "1-tick delay"; the stricter wording is "one `BuildingClass::Update` reconciliation pass later." Whether that lands in the same global frame as `AddGarrisonOccupant` depends on global object update order, which this slice did not re-prove. Active in YR: Yes for the reconciliation pass; frame ordering marked medium. Evidence: `0x00522910` lacks `ChangeOwner`; `0x0043FB20` calls `0x00458200`; `0x00458200` performs `ChangeOwner`.

### 3.3 Civilian house selection and revert (checked FIRST in binary)

(corrected 2026-05-29: sections 3.3 and 3.4 were numbered as transfer-then-revert, but the binary evaluates them in opposite order — revert is the FIRST conditional block in `CheckAutoSellOrCivilian`, transfer is the SECOND; ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT; verified via `decompile_function 0x00458200`)

The helper finds the revert house by loading the `Civilian` side name into `FUN_006A46D0`, which iterates the side array and returns the matching side index or `-1`. `CheckAutoSellOrCivilian` then scans `g_HouseClass_Array` for the house whose `House->CountryType+0xBC` equals that side index. Active in YR: Yes. Evidence: `0x00458200` decompile; `0x006A46D0` decompile; prior `BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md` records the string/data xref at `0x00458236` and call at `0x0045823D`.

Revert condition is exactly: `GetOccupantCount() == 0 && building.Owner != civilian_house`. The helper checks `HouseClass::IsHumanPlayer`, plays a positional sound, creates a radar event, may play EVA, calls `FUN_00458330`, then `ChangeOwner(civilian_house, 0)`. This block is evaluated **first** in the binary. Active in YR: Yes. Evidence: `0x00458200` decompile; disassembly range `0x00458200..0x0045832F`.

### 3.4 Transfer (checked SECOND in binary)

Transfer condition is exactly: `GetOccupantCount() > 0 && building.Owner == civilian_house`. It calls `FUN_00458330` first, then `ChangeOwner(first_occupant->Owner, 0)` where first occupant is `*(*(building+0x688))` and owner is at `+0x21C`. This block is evaluated **after** the revert block. Active in YR: Yes. Evidence: `0x00458200`; `0x00458330` decompile; `0x00448260` ChangeOwner decompile.

This means multi-occupant ownership follows occupant slot `0`, not the newest occupant. Active in YR: Yes. Evidence: `0x00458200` transfer expression reads the first item only.

### 3.4a Empty revert and red-HP ejection (detail)

`FUN_00458330` is not original-owner tracking. It refreshes building animation variants based on health and whether occupant count is zero/nonzero, selecting among healthy-empty, damaged, and healthy-garrisoned anim names for five slots. Active in YR: Yes. Evidence: fresh decompile `0x00458330`; disassembly range `0x00458330..0x0045844E`.

At the start of the same helper, if `ObjectClass::IsRedHP(this)` is true, it calls `BuildingClass::SellBuilding @ 0x00457DE0`. That helper ejects and clears the occupant vector. Because `CheckAutoSellOrCivilian` then continues and re-reads the occupant count, a red-HP eject can also take the empty-revert branch during the same reconciliation invocation. Active in YR: Yes. Evidence: `0x00458200` decompile; `0x00457DE0` decompile.

### 3.5 Sell and destruction

`BuildingClass::Sell @ 0x00449C30` is a state machine. In state `1`, after survivor setup, it calls `GetOccupantCount`; if count is positive it calls `BuildingClass::SellBuilding @ 0x00457DE0`. State `2` later destroys/removes the building and handles refund/storage logic for the actual sell transaction. Active in YR: Conditional on the sell command reaching this state; the state machine is active. Evidence: fresh decompile `0x00449C30`; disassembly `0x00449C30..0x0044A9FF`.

`SellBuilding @ 0x00457DE0` is occupant ejection only. It resets `Building+0x69C`, finds an exit coordinate, processes occupants high-to-low, calls each occupant `Unlimbo(exit, 0)`, destroys only occupants whose `Unlimbo` fails, queues Guard/Scatter for successful exits, clears/resizes the occupant vector, and recalculates threat/power. It does not call `ChangeOwner`. Active in YR: Yes. Evidence: fresh decompile `0x00457DE0`.

`BuildingClass::ReceiveDamage @ 0x00442230` destruction case `4` calls `SellBuilding` when `Type[0x157B] != 0`, before final destruction handling. That means destruction ejects occupants but does not revert the now-destroyed building to civilian. Active in YR: Yes. Evidence: fresh decompile `0x00442230`.

## 4. INI Keys

| Key / source | Default / stock evidence | Binary role | Active in YR |
|---|---|---|---|
| `CanBeOccupied=` | many stock `rulesmd.ini` civilian structures, e.g. `rulesmd.ini:13002`, `14108` | `BuildingType+0x157B`; gates the update call and destruction ejection | Conditional; Yes for stock UC buildings |
| `MaxNumberOccupants=` | stock values under UC buildings | not ownership timing; only affects entry capacity and occupant vector size expectations | Conditional |
| `[Special] MultiplayPassive=true` | `rulesmd.ini:3343` | entry permission in `CanDock`, not the ownership revert target here | Yes for entry; not transfer target |
| `[Neutral] MultiplayPassive=true` | `rulesmd.ini:3351` | entry permission in `CanDock`, not the ownership revert target here | Yes for entry; not transfer target |
| `Civilian` side name | side data, not an INI key in this repo scan | `FUN_006A46D0` side-name lookup used to find revert house | Yes |

## 5. Integration Points

| Point | Role | Evidence | Active in YR |
|---|---|---|---|
| `InfantryClass::PerCellProcess` -> `AddGarrisonOccupant` | entry commit after `CanDock`; no transfer in target body | prior CanDock report plus fresh `0x00522910` | Yes |
| `BuildingClass::Update` -> `CheckAutoSellOrCivilian` | actual transfer/revert owner path | `0x0043FB20` decompile; `0x0043FFF0..0x0044014F` disassembly | Yes |
| `BuildingClass::Sell` -> `SellBuilding` | sell ejects occupants before sell destruction flow | `0x00449C30` decompile | Conditional on sell command |
| `ReceiveDamage` destruction case -> `SellBuilding` | destruction ejects occupants before final death handling | `0x00442230` decompile | Yes |
| `ChangeOwner` vtable `+0x3D4` | ownership mutation with second arg `0` from reconciliation | `0x00458200`; `0x00448260` | Yes |

## 6. Current Rust Implementation Status

Rust currently transfers ownership immediately after boarding in `src/sim/passenger.rs` instead of waiting for the building update reconciliation phase. It also records `GameEntity::garrison_original_owner` and restores that stored owner when cargo empties.

Native gamemd does not store a per-building original owner for this path; it recomputes the `Civilian` side house and always reverts empty captured civilian garrisons to that house. This can differ from Rust if a map-authored garrisonable structure started as `Special`, a nonstandard passive house, or another custom owner accepted by entry logic.

Rust `production_sell.rs` has a captured-civilian sell branch using `garrison_original_owner.is_some()` to eject/revert without removing the building. Fresh binary evidence here verifies `SellBuilding` itself is eject-only, but `BuildingClass::Sell` state `2` is the actual sell destruction path when that state machine runs. Sell-command eligibility for captured civilians was not re-proven in this slice.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `AddGarrisonOccupant @ 0x00522910` | verified | fresh decompile | none for ownership |
| `BuildingClass::Update @ 0x0043FB20` call into reconciliation | verified | fresh decompile; disassembly `0x0043FFF0..0x0044014F` | global object scheduler frame order |
| `CheckAutoSellOrCivilian @ 0x00458200` | verified | fresh decompile; disassembly `0x00458200..0x0045832F` | none for ownership |
| `FUN_006A46D0` side lookup | verified | fresh decompile; prior data xref evidence | none |
| `FUN_00458330` anim refresh helper | verified | fresh decompile; disassembly `0x00458330..0x0045844E` | exact anim names per stock building out-of-scope |
| `SellBuilding @ 0x00457DE0` ownership effect | verified | fresh decompile | exit-cell exact order covered by separate ejection target |
| `BuildingClass::Sell @ 0x00449C30` ownership effect | touched-not-exhausted | fresh decompile; disassembly `0x00449C30..0x0044A9FF` | sell-command eligibility/UI for captured civilians |
| destruction case `ReceiveDamage @ 0x00442230` | verified | fresh decompile | none for ownership |
| power bonus / pips / fire eligibility | deferred | user non-scope | separate investigations |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does AddGarrisonOccupant transfer owner? -> No; it appends occupant and first-occupant side effects only.` (evidence: `0x00522910`)
- `[RESOLVED] OQ-02 - Where is ownership transfer performed? -> `CheckAutoSellOrCivilian` called from `BuildingClass::Update` after `CanBeOccupied` guard.` (evidence: `0x0043FB20`, `0x00458200`)
- `[RESOLVED] OQ-03 - What is the transfer condition? -> `count > 0 && Owner == civilian_house`.` (evidence: `0x00458200`)
- `[RESOLVED] OQ-04 - Which occupant supplies the new owner? -> Items[0]'s `+0x21C` owner pointer.` (evidence: `0x00458200`)
- `[RESOLVED] OQ-05 - How is original owner tracked? -> It is not stored per building in this path; the helper recomputes the `Civilian` side house.` (evidence: `0x00458200`, `0x006A46D0`)
- `[RESOLVED] OQ-06 - What is the revert condition? -> `count == 0 && Owner != civilian_house`, then ChangeOwner(civilian_house, 0).` (evidence: `0x00458200`)
- `[RESOLVED] OQ-07 - Are StructureAbandoned cues pre- or post-revert? -> Pre-revert; human/sound/radar branch runs before ChangeOwner.` (evidence: `0x00458200`)
- `[RESOLVED] OQ-08 - What happens at red HP? -> The helper calls SellBuilding to eject, then can immediately see count zero and revert in the same call.` (evidence: `0x00458200`, `0x00457DE0`)
- `[RESOLVED] OQ-09 - What happens on destruction? -> ReceiveDamage case 4 calls SellBuilding for `CanBeOccupied`, then destruction continues; no revert path for the destroyed building.` (evidence: `0x00442230`)
- `[RESOLVED] OQ-10 - What happens when SellBuilding runs? -> Occupants eject and vector clears; no ChangeOwner inside SellBuilding.` (evidence: `0x00457DE0`)
- `[RESOLVED] OQ-11 - Does BuildingClass::Sell call SellBuilding? -> Yes in state 1 when occupant count is positive; state 2 is the actual sell destruction path.` (evidence: `0x00449C30`)
- `[RESOLVED] OQ-12 - Is this TS legacy only? -> No; `rulesmd.ini` has active `CanBeOccupied=yes` civilian buildings and `BuildingClass::Update` reads the live flag.` (evidence: `rulesmd.ini`, `0x0043FB20`)
- `[RESOLVED] OQ-13 - Can transfer occur in the same global frame as entry if the building has not yet updated this frame? -> Yes, when the infantry entry runs before the target building update in the live `LogicClass` forward vector pass; otherwise transfer waits until the building's next update.` (evidence: `CIVILIAN_GARRISON_OWNER_TIMING_GLOBAL_ORDER_GHIDRA_REPORT.md`, `0x0055B5FB..0x0055B619`, `0x004401AF`)
- `[DEFERRED] OQ-14 - Is player sell command always available for captured civilian garrisons in standard UI?` (category: `requires-different-system-context`; reason: this slice verified `BuildingClass::Sell` if reached, not sidebar command gating; next-step-if-pursued: trace sell command availability/cursor for captured UC buildings)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Ownership transfer is deferred to the building update reconciliation pass: `count > 0 && owner == CivilianHouse` -> `ChangeOwner(first_occupant.owner, 0)`. | `0x00522910`, `0x0043FB20`, `0x00458200` | Rust transfers immediately in boarding | `src/sim/passenger.rs::tick_boarding`; possible building update/passenger reconciliation surface | Do not recolor/re-index ownership in the same boarding call; reconcile at a deterministic building-update phase or explicitly document if Rust chooses an intentional immediate approximation. | GI enters neutral CAGAS; immediately after boarding but before the building reconciliation phase, owner is still civilian; after reconciliation, owner is GI owner. Proposed test: `garrison_owner_transfers_on_building_reconciliation_not_boarding` | Immediate transfer can reveal/control/fire/sound from the wrong owner one phase early. |
| Empty revert does not restore arbitrary stored original owner; it recomputes the `Civilian` side house and calls `ChangeOwner(civilian_house, 0)` after pre-revert sound/radar/EVA. | `0x00458200`, `0x006A46D0` | Rust stores `garrison_original_owner` and restores it | `src/sim/passenger.rs::tick_unloading`, `GameEntity::garrison_original_owner`, `production_sell.rs` captured-civilian paths | Revert to the resolved Civilian house for native parity; keep pre-revert owner only for sound/event gating. | Map-authored `Special` garrisonable accepted by entry empties after ownership transfer; binary-style result is Civilian-side owner, not necessarily `Special`. Proposed test: `garrison_empty_reverts_to_civilian_side_not_stored_original_owner` | Storing arbitrary original owner can preserve map-specific passive owners that gamemd would not restore. |
| Red-HP reconciliation ejects occupants with `SellBuilding`, then can immediately take the empty-revert branch in the same helper invocation. | `0x00458200`, `0x00457DE0` | Rust red-HP/ejection timing unchecked in this slice | combat/health transition plus passenger/ejection surfaces | Route ConditionRed crossing for occupied UC buildings through occupant ejection and ownership reconciliation, not destruction-only logic. | Damage occupied civilian garrison to red but not dead; occupants eject alive, building remains, owner reverts during the reconciliation phase. Proposed test: `red_hp_garrison_ejects_and_reverts_without_destroying_building` | Do not wait for building death to eject red-HP garrisons. |
| If `BuildingClass::Sell` state machine is reached, it calls `SellBuilding` for occupants and then proceeds to sell destruction; `SellBuilding` itself never changes owner. | `0x00449C30`, `0x00457DE0` | Rust captured-civilian sell uses `garrison_original_owner` to eject/revert/preserve building | `src/sim/production/production_sell.rs::sell_building` | Re-check captured-civilian sell UX before preserving or destroying; do not cite `SellBuilding` alone as proof that player sell preserves the building. | Captured civilian sell command, if allowed, should be tested against native: eject order, ownership after state machine, and whether the building remains. Proposed test: `captured_civilian_sell_matches_native_state_machine_outcome` | Existing docs conflict; command eligibility remains a blocker before changing this path. |

### Negative Facts / Do Not Do

- Do not transfer ownership inside `AddGarrisonOccupant`; Active in YR: Yes. Evidence: `0x00522910` decompile lacks `ChangeOwner`.
- Do not model `garrison_original_owner` as a native per-building stored field for this path; Active in YR: Yes. Evidence: `0x00458200` recomputes Civilian house via side/house arrays.
- Do not revert to `Neutral` or `Special` by string name as the binary rule; Active in YR: Yes. Evidence: `0x006A46D0` side lookup plus `House->CountryType+0xBC` match.
- Do not describe `FUN_00458330` as ownership cleanup; Active in YR: Yes. Evidence: `0x00458330` selects anim variants.
- Do not use `SellBuilding @ 0x00457DE0` alone to prove the building survives player sell; Active in YR: Conditional. Evidence: `BuildingClass::Sell @ 0x00449C30` is the actual sell state machine and continues after eject.

### Stale Docs / Follow-up Docs

- `docs/research/BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md` replacement wording for "1-tick delay": "Ownership transfer is deferred until the building's next `BuildingClass::Update` reconciliation pass after the occupant vector changes. It is not in `AddGarrisonOccupant`; same-frame vs next-frame timing depends on the live `LogicClass` object-vector order."
- `docs/research/GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md` replacement wording for `CheckAutoSellOrCivilian`: "Empty + non-civilian owner -> pre-revert sound/radar/EVA, anim refresh, then `ChangeOwner(CivilianHouse, 0)`. Occupied + Civilian owner -> anim refresh, then `ChangeOwner(first_occupant->Owner, 0)`. Red HP -> `SellBuilding` ejection first, no destruction by that helper."
- `docs/plans/2026-05-04-unified-garrison-eject-plan.md` replacement wording: "`SellBuilding @ 0x00457DE0` proves only occupant ejection and vector clearing. It does not itself prove captured civilian player-sell preserves the building; `BuildingClass::Sell @ 0x00449C30` must be considered for the actual sell transaction outcome."

## 10. Remaining Uncertainty

- Exact retail map/replay object-vector indices for a specific infantry/building pair still require runtime logging. The binary rule is now verified as "next target-building reconciliation pass," which can be same-frame or next-frame depending on live object order.
- Sell command availability/UX for captured civilian garrisons was not re-proven; `BuildingClass::Sell` behavior is verified only if the command reaches that state machine.

## Sources

- Ghidra read-only decompile: `0x00458200`, `0x0043FB20`, `0x00522910`, `0x00457DE0`, `0x00449C30`, `0x00442230`, `0x00458330`, `0x006A46D0`, `0x00448260`.
- Ghidra read-only disassembly ranges: `0x00458200..0x0045832F`, `0x0043FFF0..0x0044014F`, `0x00458330..0x0045844E`, `0x00449C30..0x0044A9FF`.
- Prior docs used as maps/checks: `GARRISON_CANDOCK_CANGARRISON_ENTRY_GATES_GHIDRA_REPORT.md`, `GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md`, `BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md`, `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md`, `GARRISON_SELL_DESTRUCTION_EJECTION_PATH_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scanned only: `src/sim/passenger.rs`, `src/sim/production/production_sell.rs`, `src/sim/combat/mod.rs`, `src/sim/world/mod.rs`, `src/sim/components.rs`.

## Status

COMPLETE for the scoped transfer/revert path and Rust-facing handoff. Two scheduler/UI-adjacent questions are deferred because they require different system contexts and do not change the verified owner-transfer owner/revert logic.
