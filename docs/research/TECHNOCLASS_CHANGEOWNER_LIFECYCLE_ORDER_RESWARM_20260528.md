# TechnoClass ChangeOwner Lifecycle Order -- Re-Swarm Ghidra Research Report

**Address(es):** `0x007014A0` (`TechnoClass::ChangeOwner`, vtable slot `+0x3D4`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** exact lifecycle order inside the `TechnoClass::ChangeOwner` body only: old-owner visual/removal gates, target/order clears, spawned/slave/mind-control cleanup directly in body, house tracking remove/add, `Owner` field write timing, local-discovery field, target reacquisition, dock/building preservation gates.
**Non-Scope:** full `BuildingClass::ChangeOwner`, full capture callers, full mind-control/slave-manager systems, vtable layout reconstruction beyond slots touched by this body.
**Confidence:** High for the primary function order; Medium for semantic names of several inherited vtable slots where the called function was identified from existing docs rather than direct vtable-table recovery in this pass.
**Active in YR:** Yes. This is the common Techno vtable ownership-transfer method reached by active building capture, garrison reconciliation, mind control, slave liberation, and related active YR paths; caller activation is cited from existing verified reports and the body was freshly decompiled at `0x007014A0`.

## Working Notes

Target question: What exact order does `TechnoClass::ChangeOwner @ 0x007014A0` use for owner-transfer lifecycle side effects?
Non-goals: Do not expand into every capture caller, complete mind-control/hijacker behavior, or full building-specific wrapper semantics.
Evidence needed to mark COMPLETE: fresh decompile of `0x007014A0`, assembly evidence around the old-owner reads and `+0x21C` owner write, helper decompiles for house remove/add and target cleanup, existing caller docs proving active YR reachability, and Rust surface scan for owner-index/order deltas.
Stop conditions: stop after the primary body and immediate top-level helper semantics needed for ordering are resolved; list unrelated caller/body systems as remaining uncertainty instead of following them.

## 1. Overview

`TechnoClass::ChangeOwner` performs a two-phase owner transfer. It first runs old-owner and old-map side effects while `TechnoClass+0x21C` still points at the old owner, then writes `+0x21C = new_owner`, then runs local-owner, reveal/re-mark, add-to-game, target reacquisition, radar, and path/order reset hooks under the new owner.

This matters because a Rust implementation that writes `entity.owner` first and then "fixes indexes" cannot reproduce native side effects that deliberately read the old owner between `0x007014AB` and `0x00701734`.

## 2. Class Layout / Key Offsets

| Offset | Type | Meaning in this slice | Active in YR | Evidence |
|---|---:|---|---|---|
| `Techno+0x21C` | `HouseClass*` | owner pointer; old owner read at entry and not overwritten until late body | Yes | decompile `param_1[0x87]`; asm `0x007014AB MOV [ESI+0x21C]`, `0x00701735 MOV [ESI+0x21C], EBP` |
| `Object/Techno+0x81` | byte | in-limbo / not-placed gate; many map/house side effects skipped when nonzero | Yes | asm `0x0070158A`, `0x00701664`, `0x00701697`, `0x007017C0` |
| `Object+0x83` | byte | selected flag checked only for local-player old owner | Yes | asm `0x007014C1..0x007014CF` |
| `Techno+0x218` | pointer/int | cleared early unless object is specific RTTI-1 + helper-approved case | Conditional | decompile branch `0x007014EF..0x00701524`; prior docs note this field is multi-purpose |
| `Techno+0x2D0` | pointer | spawn manager pointer; if non-null, kills all spawns before owner write | Conditional | asm `0x00701574..0x0070157E`; helper `0x006B7100` |
| `Techno+0x508` | int | occupation marker/cache cleared before owner write, recomputed after owner write | Yes for placed objects | asm `0x00701719`, post-write recompute `0x00701765..0x0070177E` |
| `Techno+0x41A` | byte | local-player-owned flag, set immediately after owner write | Yes | decompile `*(byte*)(this+0x41A) = (new_owner == g_PlayerPtr && g_PlayerPtr != 0)`; asm `0x0070173B..0x00701755` |
| `Techno+0x2E0` | pointer/int | cleared after add-to-game; used as a target/order state slot | Yes | asm `0x007017A0 MOV [ESI+0x2E0],0` |
| `Techno+0x423` | byte | radar unregister/register refresh gate after order/target reset | Conditional | asm `0x0070184F..0x00701868` |

## 3. Core Logic

The exact order in the body is:

1. Read old owner from `this+0x21C`; if `new_owner == old_owner`, return before any visible or bookkeeping side effect. Active in YR: Yes. Evidence: asm `0x007014AB..0x007014B3`.
2. If old owner is `g_PlayerPtr` and `this+0x83` selected, call vtable `+0x150` before any owner write. Active in YR: Conditional; only when the losing owner is local player and object is selected. Evidence: asm `0x007014B9..0x007014CF`; `SELECTION_LIFECYCLE_GHIDRA_REPORT.md` resolves this as the mind-control/capture auto-deselect path.
3. Clear target/archive-like state via vtable `+0x3C8(0)`, then call vtable `+0x480(0,1)`. Active in YR: Yes; exact semantic name varies by concrete class. Evidence: asm `0x007014D5..0x007014E9`.
4. Clear `+0x218` unless the object passes the RTTI-1 helper path. Active in YR: Conditional; the default path clears. Evidence: decompile/asm `0x007014EF..0x00701524`.
5. Unless mission/state gates match the special preserved case (`+0xAC == 0x13`, or RTTI-1 with Type `+0xE13` and `+0xAC == 0x10`), call vtable `+0x1E8(5,1)`. Active in YR: Conditional. Evidence: asm `0x00701524..0x0070156E`.
6. If `+0x2D0` spawn manager exists, call `SpawnManagerClass::Kill_All_Spawns @ 0x006B7100`. Active in YR: Conditional; live for spawned-aircraft/carrier owners. Evidence: asm `0x00701574..0x0070157E`; helper decompile kills/removes spawn entries and timestamps them with `g_CurrentFrameCounter`.
7. Call `FUN_00638840`. Active in YR: Conditional; helper uses current `Owner` before the owner write and clears a manager/object vector. Evidence: call at `0x00701583..0x00701585`, helper decompile reads `local_4+0x21C` and clears fields `+0x8C..+0x99`; exact system name deferred.
8. If not in limbo (`+0x81 == 0`), call `HouseClass::Removed_From_Game(old_owner, this, 0)`. Active in YR: Yes for placed objects. Evidence: asm `0x0070158A..0x0070159D`; helper `0x005025F0` uses old-owner `ECX` and for buildings sets old owner `+0x5778 = 1`.
9. Call vtable `+0xE0(0)`. Active in YR: Yes; semantic name deferred. Evidence: asm `0x007015A2..0x007015A8`.
10. Add this object's type cost to `new_owner+0x54E8` using the old owner as cost context, before the owner pointer changes. Active in YR: Yes. Evidence: asm `0x007015AE..0x007015D2`.
11. Call `HouseClass::Remove_Tracking(old_owner, this)` and then `HouseClass::Add_Tracking(new_owner, this)` before `this+0x21C` changes. Active in YR: Yes. Evidence: asm `0x007015D8..0x007015E6`; helpers `0x004FF550`/`0x004FF700` mutate house counters and per-type indexes using the passed house, not `this+0x21C`.
12. Increment new owner's per-house kill/score arrays for RTTI 1, 2, 15, or 6, unless `TechnoType+0xC9F` DontScore is set; write old owner's last-killed-house value from `new_owner+0x34+0xB8`. Active in YR: Conditional by RTTI and DontScore. Evidence: asm `0x007015EB..0x0070165E`.
13. If placed, call vtable `+0x124(0)`, `TechnoClass::StopAllTargeting @ 0x0070D4A0`, then vtable `+0x124(3)`. Active in YR: Yes for placed objects. Evidence: asm `0x00701664..0x00701691`; helper decompile clears other Technos and TeamClass pointers targeting this object.
14. If placed, find current cell, remove existing occupation marker by negating `+0x508` and calling `FUN_00481870`, clear `+0x508`, and call vtable `+0x48C(0,0,0,0)`. Active in YR: Yes for placed objects. Evidence: asm `0x00701697..0x0070172F`.
15. Write `this+0x21C = new_owner`. Active in YR: Yes. Evidence: asm `0x00701735 MOV dword ptr [ESI+0x21C], EBP`.
16. Set `this+0x41A = 1` iff `new_owner == g_PlayerPtr && g_PlayerPtr != 0`; otherwise zero. Active in YR: Yes. Evidence: decompile and asm `0x0070173B..0x00701755`.
17. If placed and a cell was found, recompute `+0x508` via vtable `+0x2C0`, call `FUN_00481870` again, then call `HouseClass::Added_To_Game(new_owner, this, 1)`. Active in YR: Yes for placed objects. Evidence: asm `0x00701757..0x00701785`; helper `0x00502A80` increments new-owner tracking/cost/power/storage.
18. Clear `+0x2E0`; if either `+0xAC` or `+0xB4` equals `0x15`, call vtable `+0x1F0(5)`. Active in YR: Conditional. Evidence: asm `0x00701794..0x007017BA`.
19. If in limbo, skip the remaining target/order refresh and return success. Active in YR: Conditional. Evidence: asm `0x007017C0..0x007017C8`.
20. Dock/building preservation gate: if this object is a building (`RTTI == 6`), current mission/state `+0xAC == 0x10`, and `BuildingType+0x16BD` is set, skip target/destination clearing. Otherwise check contact slot 0 via `FootClass::GetDestination(0)`; if the destination object is a building with Type `+0x16BD`, also skip. Active in YR: Conditional; `+0x16BD` identity remains deferred. Evidence: asm `0x007017D0..0x00701825`; `FootClass__GetDestination @ 0x0065AD30` decompile returns contact/destination slot.
21. If not preserved, call vtable `+0x480(0,1)`, vtable `+0x3C8(0)`, and vtable `+0x484(0,1)` after owner change. Active in YR: Yes when preservation gate fails. Evidence: asm `0x00701827..0x00701849`.
22. If `+0x423` set, call vtable `+0x498` then `+0x494`, then always call vtable `+0x488(0,0,0,0,0)`. Active in YR: Conditional for radar refresh, Yes for final reveal/update call on placed objects. Evidence: asm `0x0070184F..0x00701881`; `BUILDINGCLASS_VTABLE_COMPLETE.md` names building `+0x494/+0x498` radar register/unregister and inherited `+0x488` as `TechnoClass::UpdateReveal @ 0x0070AF50`.

## 4. INI Keys

No INI key is read directly by `TechnoClass::ChangeOwner`. Type fields already parsed from INI gate several branches:

| INI / field | Default / stock relevance | Effect in this body | Active in YR | Evidence |
|---|---|---|---|---|
| `DontScore=` -> `TechnoType+0xC9F` | type-dependent | suppresses per-house score/kill array increment | Conditional | body reads `+0xC9F` at `0x00701611` and type docs verify parser |
| Unknown `BuildingType+0x16BD` | field identity deferred; stock meaning not established here | preserves dock/building state by skipping post-owner target/order clears | Conditional | body reads `+0x16BD` at `0x007017E3..0x007017F1`, destination variant at `0x00701817..0x00701825` |
| `[GAREFN]/[NAREFN] DockUnload=yes` | stock active refineries | not read by this function; listed only because dock preservation may be relevant to owner-transfer handoff around harvesters | Yes for stock refineries, but not direct evidence for `+0x16BD` | `ini/rulesmd.ini:11726`, `12519`; separate dock reports |
| `[SMIN]/[YAREFN] Enslaves=SLAV`, `SlavesNumber=5` | stock Yuri slave miner/refinery | not read directly by this function; slave liberation can call vtable `+0x3D4` from `SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md` | Conditional | `ini/rulesmd.ini:9099-9100`, `13279-13280`; sibling report |

## 5. Integration Points

Active YR reachability:

| Integration point | Status | Evidence |
|---|---|---|
| Building capture/garrison wrapper | verified active | `BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md` identifies `BuildingClass::ChangeOwner @ 0x00448260` as active vtable `+0x3D4`, and garrison reconciliation/capture call it |
| Mind control capture/free | verified active | `MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md` says `CaptureManagerClass::CaptureUnit @ 0x00471D40` calls target vtable `+0x3D4`, and `FreeUnit @ 0x00471FF0` restores through the same slot |
| Slave liberation | verified active conditional | `SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md` shows slave vtable `+0x3D4(new_owner,1)` before `+0x3D0`/`+0x388`; stock `SMIN/YAREFN` have slave fields |
| Target cleanup consumers | verified | `TechnoClass::StopAllTargeting @ 0x0070D4A0` scans `g_TechnoClass_Array` and `g_TeamClass_Array`, clearing targeting pointers to this object |
| House tracking | verified | `HouseClass::Remove_Tracking @ 0x004FF550`, `Add_Tracking @ 0x004FF700`, `Removed_From_Game @ 0x005025F0`, `Added_To_Game @ 0x00502A80` decompiled in this pass |

## 6. Current Rust Implementation Status

Rust currently has no central `TechnoClass::ChangeOwner` equivalent. Engineer capture in `src/sim/world/world_orders.rs:177` directly writes `b.owner = engineer_owner` at line `233`, then adjusts owned counts at lines `240` and `242`, and consumes the engineer. That is not the native order for a generic Techno owner transfer: native removes old owner/game/map state, clears targeters, unmarks occupation, writes `+0x21C`, then re-adds/reveals/reacquires.

`src/sim/entity_store.rs` has a `by_owner` secondary index and `ids_for_owner`, but `insert`, `remove`, and direct owner mutation do not synchronously update that index. Tests explicitly rebuild after owner mutation (`entity_store.rs:467..471`), so any future owner-transfer API must decide exactly when old-owner and new-owner indexed views become visible.

Rust has fields that correspond to affected state (`GameEntity.owner`, `movement_target`, `attack_target`, `capture_target`, `mind_controlled`, `slave_harvester`), but no scanned function currently clears/rebuilds these in native `ChangeOwner` order.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TechnoClass::ChangeOwner @ 0x007014A0` owner-write order | verified | decompile plus asm `0x007014AB..0x00701881` | none for body order |
| Same-owner early return | verified | asm `0x007014AB..0x007014B3` | return high bits not meaningful for Rust |
| Local-player deselect gate | verified | asm `0x007014B9..0x007014CF`; selection doc | exact vtable `+0x150` body not re-decompiled here |
| Spawn manager cleanup | verified | call `0x00701574..0x0070157E`; helper `0x006B7100` | exact spawn state enum names deferred |
| `FUN_00638840` cleanup | touched-not-exhausted | call `0x00701583..0x00701585`; helper decompile | semantic owner/system name deferred |
| House Removed/Add/Tracking helpers | verified | decompile `0x005025F0`, `0x00502A80`, `0x004FF550`, `0x004FF700` | full counter taxonomy not needed here |
| Old-owner vs new-owner field visibility | verified | old-owner reads before `0x00701735`; write at `0x00701735` | none |
| Targeting cleanup | verified | body calls `0x0070D4A0` at `0x0070167C`; helper decompile | exact vtable names for object target slots deferred |
| Occupation unmark/re-mark | verified | asm `0x00701697..0x0070172F`, `0x00701757..0x00701785` | `FUN_00481870` semantic details deferred |
| Dock/building preservation gate | touched-not-exhausted | asm `0x007017D0..0x00701825` | identity/defaults of `BuildingType+0x16BD` unresolved |
| Rust engineer capture owner mutation | verified current shape | `src/sim/world/world_orders.rs:177..242` | exact owner-transfer API missing |
| Rust owner index timing | verified current shape | `src/sim/entity_store.rs:50..57`, `136..145`, tests `467..471` | no parity API currently exists |

## 8. Open Questions -- Final State

- `[RESOLVED] OQ-01 -- Where is owner written? -> late in body at `0x00701735`, after house remove/add, targeting cleanup, and occupation unmark.` (evidence: `0x00701735`; Active in YR: Yes)
- `[RESOLVED] OQ-02 -- Are old-owner side effects before the owner write? -> yes: deselect, Removed_From_Game, Remove_Tracking, old-owner last-killed write, and occupation unmark all run before `+0x21C` changes.` (evidence: `0x007014B9..0x00701734`; Active in YR: Yes)
- `[RESOLVED] OQ-03 -- Does Add_Tracking run before or after owner write? -> before owner write, using the new owner as explicit `ECX/this` rather than reading `this+0x21C`.` (evidence: `0x007015E3..0x007015E6`, write `0x00701735`; Active in YR: Yes)
- `[RESOLVED] OQ-04 -- Is local-player flag set before or after owner write? -> after owner write, based on new owner equality to `g_PlayerPtr`.` (evidence: `0x00701735..0x00701755`; Active in YR: Yes)
- `[RESOLVED] OQ-05 -- Are spawned units handled directly in this body? -> yes, if `+0x2D0` is non-null the body calls `SpawnManagerClass::Kill_All_Spawns`.` (evidence: `0x00701574..0x0070157E`, `0x006B7100`; Active in YR: Conditional)
- `[RESOLVED] OQ-06 -- Does this body directly handle mind-control node allocation/freeing? -> no; mind-control callers use vtable `+0x3D4`, but node fields are managed in CaptureManager functions outside this body.` (evidence: `MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md` §5.3/5.4; Active in YR: Conditional)
- `[RESOLVED] OQ-07 -- Does this body directly handle slave-manager arrays? -> no slave-manager array walk appears in this body; slave manager can call vtable `+0x3D4` externally.` (evidence: body decompile; `SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md`; Active in YR: Conditional)
- `[RESOLVED] OQ-08 -- Does StopAllTargeting happen under old or new owner? -> old-owner phase, before `+0x21C` write.` (evidence: call at `0x0070167C`, owner write `0x00701735`; Active in YR: Yes)
- `[RESOLVED] OQ-09 -- Is map occupation unmarked before owner write and re-marked after? -> yes, old marker is removed/cleared before `0x00701735`, then recomputed after.` (evidence: `0x00701697..0x0070172F`, `0x00701757..0x0070177E`; Active in YR: Yes for placed objects)
- `[RESOLVED] OQ-10 -- Does the function skip map/house placed-object side effects for limbo objects? -> yes, multiple `+0x81` gates skip Removed_From_Game, mission/target occupation, Added_To_Game, and late target/order refresh.` (evidence: `0x0070158A`, `0x00701664`, `0x00701697`, `0x007017C0`; Active in YR: Conditional)
- `[RESOLVED] OQ-11 -- Are target/order clears performed both before and after owner write? -> yes: early `+0x3C8/+0x480` before old-owner removal, and late `+0x480/+0x3C8/+0x484` after owner write unless preserved.` (evidence: `0x007014D5..0x007014E9`, `0x00701827..0x00701849`; Active in YR: Conditional)
- `[DEFERRED] OQ-12 -- What is exact `BuildingType+0x16BD` identity/default?` (category: requires-different-system-context; reason: body proves it gates preservation, but parser/default attribution is outside this slot; next-step-if-pursued: targeted field parse/use investigation)
- `[DEFERRED] OQ-13 -- What exact system does `FUN_00638840` own?` (category: requires-different-system-context; reason: direct helper body was decompiled enough to prove pre-owner-write cleanup, but full manager taxonomy is outside this slot; next-step-if-pursued: identify `FUN_00705D20` owner and struct layout)
- `[DEFERRED] OQ-14 -- Exact vtable names for `+0xE0`, `+0x124`, `+0x48C`, `+0x1F0`, `+0x484` on every subclass.` (category: bounded-cost-too-high; reason: not needed to prove body order; next-step-if-pursued: vtable-table audit for Techno/Foot/Building/Unit/Aircraft)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Owner field write occurs late, after old-owner removal/tracking/target cleanup/occupation unmark | asm `0x007014AB..0x00701735` | missing; capture writes owner first | `src/sim/world/world_orders.rs`, future shared owner-transfer helper, `src/sim/entity_store.rs` | model a two-phase owner transfer where old-owner queries still see old owner until the native write point | engineer captures a selected player building: old owner count/selection/targeters are cleared before new owner visibility; proposed test `change_owner_writes_owner_after_old_owner_cleanup` | Do not implement as `entity.owner = new; rebuild all indexes; clear targets` |
| `Remove_Tracking(old)`, `Add_Tracking(new)` run before `Techno+0x21C` changes and use explicit house arguments | asm `0x007015D8..0x007015E6`, helper decompiles `0x004FF550/0x004FF700` | missing/unchecked; `by_owner` is rebuild-only around direct mutation | `src/sim/entity_store.rs`, owned-count functions in `src/sim/world/mod.rs` | owner-index API must expose native transition order or avoid index reads mid-transfer until after the correct point | instrumented transfer where an observer in old-cleanup phase queries owner index: object remains old-owned until write point, then appears new-owned after add/reveal; proposed test `change_owner_owner_index_transition_matches_write_point` | Do not rely on `rebuild_owner_index()` after arbitrary direct mutation as a parity model |
| Placed objects unmark occupation before owner write and re-mark/update reveal after owner write | asm `0x00701697..0x0070172F`, `0x00701757..0x00701881`; `TechnoClass::UpdateReveal @ 0x0070AF50` | missing/partial; current capture only changes owner/counts | occupancy/vision/fog update surfaces, `src/sim/passenger.rs`, `src/sim/world/world_orders.rs` | transfer must separate unmark-old, write-owner/local flag, re-mark/add-to-game/reveal/update passes | captured/garrison-reverted building changes remap and reveal only after old occupation has been removed; proposed test `change_owner_reveals_after_owner_write_and_remark` | Do not recolor/reveal before the old occupation marker is cleared |

### Negative Facts / Do Not Do

- Do not treat `TechnoClass::ChangeOwner` as a simple owner assignment plus count repair. Evidence: `+0x21C` is not written until `0x00701735`, after dozens of side effects.
- Do not update per-owner indexes solely by reading the object's current owner during `Remove_Tracking`/`Add_Tracking`; native passes old and new houses explicitly before the owner field changes. Evidence: `0x007015D8..0x007015E6`.
- Do not collapse the early and late target/order clears into one call. Evidence: early `+0x3C8/+0x480` at `0x007014D5..0x007014E9`, late `+0x480/+0x3C8/+0x484` at `0x00701827..0x00701849`, with a preservation branch between them.
- Do not assume mind-control/slave bookkeeping is inside `TechnoClass::ChangeOwner`; this body only handles common owner-transfer mechanics. Evidence: primary body lacks node/vector allocation, while sibling reports place MC/slave arrays in CaptureManager/SlaveManager.
- Do not mark the dock/building preservation gate complete until `BuildingType+0x16BD` is identified from parser/default evidence. Evidence: body reads `+0x16BD`, but this slot did not trace its INI source.

### Stale Docs / Follow-up Docs

- `docs/research/TECHNOCLASS_SYSTEMS_GHIDRA_REPORT.md` §5.2 should replace its current high-level list with: "Ownership transfer is two-phase. `TechnoClass::ChangeOwner @ 0x007014A0` reads old `Owner` at entry and does not write `Techno+0x21C = new_owner` until `0x00701735`; before that it performs old-owner deselect/removal/tracking, target cleanup, spawn cleanup, stop-targeting, and occupation unmark. Only after the owner write does it set `+0x41A`, re-mark/add-to-game, run late target/order cleanup unless the preservation gate applies, refresh radar, and call `UpdateReveal`."
- `docs/research/BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md` §8 says garrison-only transfer can set `entity.owner` immediately because complex house lists are absent. Replacement: "Even for garrison-only Rust paths, direct immediate owner assignment is not mechanism-equivalent unless old-owner cleanup, targeter cleanup, occupation unmark, owner write, add/reveal, and duplicate-suppression timing are modeled in the same relative order."

## Sources

- Fresh Ghidra decompile: `TechnoClass::ChangeOwner @ 0x007014A0`.
- Fresh Ghidra assembly contexts: `0x007014AB..0x00701881`, especially `0x00701735`.
- Fresh Ghidra helper decompiles: `HouseClass::Remove_Tracking @ 0x004FF550`, `HouseClass::Add_Tracking @ 0x004FF700`, `HouseClass::Removed_From_Game @ 0x005025F0`, `HouseClass::Added_To_Game @ 0x00502A80`, `SpawnManagerClass::Kill_All_Spawns @ 0x006B7100`, `TechnoClass::StopAllTargeting @ 0x0070D4A0`, `FootClass::GetDestination @ 0x0065AD30`, `TechnoClass::UpdateReveal @ 0x0070AF50`.
- Existing docs: `BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md`, `MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md`, `SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md`, `BUILDING_OWNER_HOUSE_BINDING_PARITY_GHIDRA_REPORT.md`, `TECHNOCLASS_SYSTEMS_GHIDRA_REPORT.md`.
- Rust scan: `src/sim/world/world_orders.rs`, `src/sim/entity_store.rs`, `src/sim/world/mod.rs`, `src/sim/game_entity.rs`.
- INI scan: `ini/rulesmd.ini` for stock `SMIN`, `GAREFN`, `NAREFN`, `YAREFN` activation data.
