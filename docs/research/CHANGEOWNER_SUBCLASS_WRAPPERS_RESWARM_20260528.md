# ChangeOwner Subclass Wrappers - Reswarm Ghidra Research Report

**Address(es):** `0x007014A0` (`TechnoClass::ChangeOwner`), `0x004DBED0` (`FootClass::ChangeOwner`), `0x007463A0` (`UnitClass` slot `+0x3D4` wrapper), `0x00448260` (`BuildingClass::ChangeOwner`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** subclass/wrapper order around `TechnoClass::ChangeOwner`, focused on `FootClass`, `UnitClass` convoy wrapper discovered during the slice, `BuildingClass`, and standard YR capture/garrison/sell entry points that call or bypass them.
**Non-Scope:** full `TechnoClass::ChangeOwner` body already covered by `TECHNOCLASS_CHANGEOWNER_LIFECYCLE_ORDER_RESWARM_20260528.md`; full campaign/team-script ownership actions outside capture/garrison/sell; exact names of every `BuildingType+0x16xx` typed-list flag.
**Confidence:** High for wrapper ordering and named entry points; Medium for semantic names of several building type-list flags.
**Active in YR:** Yes. Engineer capture, civilian-garrison reconciliation, mind control, Psychic Dominator, UnitClass crush hijack/convoy handling, and player sell are live standard YR systems; team/script ownership actions are conditional on map/script data and are not fully exhausted here.

## 1. Overview

`ChangeOwner` is not a single direct owner write. Runtime virtual slot `+0x3D4` dispatches to different wrappers before the shared `TechnoClass::ChangeOwner` two-phase transfer:

- buildings dispatch to `BuildingClass::ChangeOwner @ 0x00448260`;
- ordinary foot-derived objects dispatch to `FootClass::ChangeOwner @ 0x004DBED0`;
- units dispatch first to `UnitClass__Transfer_Convoy_On_Owner_Change @ 0x007463A0`, then to `FootClass::ChangeOwner`;
- `TechnoClass::ChangeOwner @ 0x007014A0` is the base transfer and is directly referenced by the base techno vtable only.

Player sell is a bypass for owner transfer: `BuildingClass::Sell @ 0x00449C30` can call `SellBuilding @ 0x00457DE0` to eject garrison occupants, then removes/sells the building; it does not use `ChangeOwner` for captured-civilian preservation.

## 2. Class Layout / Key Offsets

| Offset / slot | Owner | Meaning in this slice | Evidence | Active in YR |
|---:|---|---|---|---|
| vtable `+0x3D4` | techno hierarchy | virtual ownership-transfer slot | data xrefs `0x007E4290`, `0x007E2678`, `0x007E9068`, `0x007EB42C`, `0x007F6044`, `0x007F4D34` | Yes |
| `Techno+0x21C` | all techno | current owner pointer | all wrapper no-op checks / base body | Yes |
| `Techno+0x41A` | all techno | local-player-owned byte written by base after owner pointer write | prior report; base decompile `0x007014A0` | Yes |
| `Foot+0x55C` (`param_1[0x157]`) | foot | wrapper-updated coordinate / nav value when object is not in the vtable `+0x54` state | `0x004DBF37..0x004DBF4A` decompile | Conditional |
| `Foot+0x5D4` (`param_1[0x175]`) | foot | non-null team/member link used for `FUN_006EA870(this,-1,0)` after successful base owner change | `0x004DBF13..0x004DBF34` | Conditional |
| `Unit+0x6C8` | unit | convoy/linked unit pointer transferred before the parent unit | `0x007463B0..0x007463D5` | Conditional; live for convoy-linked units |
| `Unit+0x6D0` | unit | convoy membership byte toggled around linked-unit transfer | `0x007463B9`, `0x007463D5` | Conditional |
| `Building+0x38` | building | bomb pointer; defused on owner change when not `CanBeOccupied` | `0x00448277..`; `BombClass__Defuse` callee | Conditional |
| `Building+0x6E3` | building | ownership-changed / captured byte set before base transfer | `0x00448723` area in decompile | Yes for successful building transfer |
| `Building+0x6E8` | building | cleared after base transfer before vtable `+0x4DC(1)` | `0x00448CDE..0x00448CEF` area in decompile | Yes for successful building transfer |
| `BuildingType+0x157B` | building type | `CanBeOccupied`; gates bomb defuse skip and garrison reconciliation | `0x0044827A`; `0x0043FB20`; INI `CanBeOccupied=` | Conditional; Yes for stock UC buildings |
| `BuildingType+0x1552` | building type | `NeedsEngineer`; gates capture/EVA/online-effects branch in building wrapper | `0x004483B5..0x004484D3`; CAHOSP audit | Conditional |
| `Building+0x684/0x688/0x694` | building | garrison occupant vector/count | `0x00458200`, `0x00457DE0` | Conditional; Yes for UC buildings |

## 3. Core Logic

### 3.1 Vtable slot map

| Concrete class | Slot `+0x3D4` target | Evidence |
|---|---|---|
| `TechnoClass` base | `0x007014A0` | data xref `0x007F4D34`; vtable base `0x007F4960` referenced by `TechnoClass__Constructor` |
| `AircraftClass` | `0x004DBED0` (`FootClass::ChangeOwner`) | data xref `0x007E2678`; vtable base `0x007E22A4` referenced by `AircraftClass__Constructor` |
| `FootClass` | `0x004DBED0` | data xref `0x007E9068`; vtable base `0x007E8C94` referenced by `FootClass__Constructor` |
| `InfantryClass` | `0x004DBED0` | data xref `0x007EB42C`; vtable base `0x007EB058` referenced by `InfantryClass__Constructor` |
| `UnitClass` | `0x007463A0` (`UnitClass__Transfer_Convoy_On_Owner_Change`) | data xref `0x007F6044`; vtable base `0x007F5C70` referenced by `UnitClass__Constructor` |
| `BuildingClass` | `0x00448260` (`BuildingClass::ChangeOwner`) | data xref `0x007E4290`; vtable base `0x007E3EBC` referenced by `BuildingClass__Constructor` |

### 3.2 FootClass wrapper order

`FootClass::ChangeOwner @ 0x004DBED0` is a narrow wrapper around the base transfer.

1. Reads the type via vtable `+0x84`.
2. If `Type+0x5F0 != 0`, computes a coordinate/value through vtable `+0x1B8` and calls vtable `+0x4EC` before the base transfer. Evidence: `0x004DBED5..0x004DBF01`.
3. Calls `TechnoClass::ChangeOwner` with `ECX=this`, pushes a constant `1`, and pushes the new owner. Evidence: assembly `0x004DBF01..0x004DBF0A`.
4. If the base returns false, it re-runs the `Type+0x5F0` check and calls vtable `+0x4E8` with the new-owner-derived vtable `+0x1B8` value, then returns `0`. Evidence: decompile failure branch `0x004DBF8F..0x004DBFB6`.
5. On success, if the new owner has byte `House+0x1EC != 0` and `Foot+0x5D4 != 0`, it calls `FUN_006EA870(this, -1, 0)`. Evidence: `0x004DBF13..0x004DBF34`.
6. On success, if virtual `+0x54` returns `0`, it stores a vtable `+0x1B8` result into `Foot+0x55C`. Evidence: `0x004DBF37..0x004DBF4A`.
7. On success, if `Type+0x5F0 != 0`, it calls vtable `+0x4E8` after the base transfer and returns `1`. Evidence: `0x004DBF4F..0x004DBF85`.

Material consequence: aircraft and infantry inherit this wrapper directly, while units do not start here; units use the UnitClass convoy wrapper first.

### 3.3 UnitClass wrapper order

`UnitClass` slot `+0x3D4` points to `UnitClass__Transfer_Convoy_On_Owner_Change @ 0x007463A0`.

1. If `newOwner == this->Owner`, returns false/no-op. Evidence: `0x007463A8..0x007463AE`.
2. Reads `Unit+0x6C8` convoy/linked unit. If non-null:
   - clears parent `Unit+0x6D0 = 0`;
   - calls the linked unit's own vtable `+0x3D4(newOwner, 1)`;
   - restores `this->0x6C8 = linked`;
   - sets linked `+0x6D0 = 1`.
   Evidence: assembly `0x007463B0..0x007463D5`; call at `0x007463C9`.
3. Calls `FootClass::ChangeOwner(this, newOwner, 1)` and returns its result. Evidence: direct call at `0x007463E1`.

Material consequence: a future Rust owner-transfer API cannot dispatch every non-building object directly to a foot/base helper; units need the convoy pre-transfer first.

### 3.4 BuildingClass wrapper order

`BuildingClass::ChangeOwner @ 0x00448260` is a large building-specific pre/base/post wrapper. The exact base-owner pointer write still happens only inside `TechnoClass::ChangeOwner`, reached at `0x00448BE8`.

Pre-base order verified from decompile and assembly:

1. No-op guard: if `newOwner == this->Owner`, return `0`. Evidence: `0x0044826B..0x00448271`.
2. If `Building+0x38 != 0` and `Type+0x157B == 0`, defuse bomb before any owner write. Evidence: `BombClass__Defuse` callee; decompile top branch.
3. If `Type+0x16C7 != 0`, set `newOwner+0x56F8 = 1`.
4. Old-owner capture/economy branch can add credits from `Type+0x1558` and writes building `+0x6D0/+0x6D4/+0x6D8` timing/value fields before owner changes.
5. Old-owner mask/list recalculation for certain type flags runs while `this->Owner` is still the old owner.
6. Clears `Building+0x3D3 = 0`; if wall/fence flag `Type+0x16BE != 0`, calls `BuildingClass__ConnectWalls(this, 0)`.
7. Human/EVA/radar notification branch runs before base transfer and is gated by the caller's announce stack arg and `NeedsEngineer`/related type fields. Evidence: decompile around `0x00448340..0x004483B0`.
8. Marks redraw/dirty state (`field_0x80 = 1`) and sets `HasEngineer = true`.
9. If `Type+0x1552 != 0`, enables building-specific post-capture systems before base transfer: `StuffEnabled = true`, production/light anim if light source exists, wall recalculation, weapon anim attachment. Evidence: decompile around `0x004483B5..0x004484D3`.
10. Iterates docked/associated occupants/units from `Building+0xE8` and calls their own virtual `+0x3D4(newOwner, 1)` before the building base transfer when relationship/range checks pass. Evidence: call at `0x00448663`.
11. Abandons factory production if `Building+Factory != null`.
12. Calls `HouseClass__Recount(this)` and `FUN_006E6AB0(this)`, then sets `Building+0x6E3 = 1`.
13. Removes this building from old-owner typed tracking vectors and subtracts old-owner contributions, including self-heal/power-style counters with old-owner clamps where applicable.
14. Runs pre-base feature-off hooks, including vtable `+0x410(1)` for one type flag, vtable `+0x418`, and vtable `+0x48C(0,0,0,0)`.
15. Calls `TechnoClass::ChangeOwner` with `ECX=this`, pushed constant `1`, and pushed new owner. Evidence: assembly `0x00448BD3..0x00448BE8`.

Post-base order verified from decompile:

1. Re-enables selected feature state for the new owner, including `Type+0x16C7` and `Type+0xCD1` branches.
2. Calls `FUN_00448070(0)`.
3. Recalculates base centers for old and new owners and fixes house `+0x54E0` pointers.
4. If the old owner was human and type flag `+0x16B9` is set, resets sidebar/UI mode globals.
5. Calls `FUN_004FFA50(this)`, clears `Building+0x6E8 = 0`, then calls vtable `+0x4DC(1)`.
6. If wall/fence and not map editor, extends/recalculates neighboring wall directions and can reset `LaserFenceFrame`.
7. If human, calls vtable `+0x488(...)` and `FUN_004F42F0(1)`.
8. Clears the new owner's country bit from building `+0x210` when present.
9. Recomputes animation facing/direction/remap: vtable `+0x1E4`, `+0x1BC`, `+0x464`, optional `FUN_0070E360`, writes `unknown_short_700`, then calls `BuildingClass__UpdateAnimFacingAndDirection` and `BuildingClass__SetAnimRemap`.
10. Adds this building into the new owner's typed tracking vectors and increments new-owner contributions.
11. Handles radar overlay update when `Type+0xEB8 != 0`.
12. Replays relationship messages for units collected in the pre-base associated-unit loop.
13. Optional AI upgrade refund/removal branch can run late.
14. Sets old/new owner dirty bytes: old owner `+0x1FC = 1` and new owner byte at vector context `+0x1FC = 1` in the decompile.

Material consequence: `BuildingClass::ChangeOwner` is not "base owner write plus count rebuild." It does old-owner cleanup before `Techno+0x21C` changes, then new-owner feature/list/UI/animation work after the base transfer.

### 3.5 Entry-point selection and bypasses

| Entry point / gameplay path | Uses which wrapper? | Verified call / bypass | Active in standard YR |
|---|---|---|---|
| Engineer capture, close-range `InfantryClass::Mission_Capture @ 0x005202F0` | target building virtual `+0x3D4` -> `BuildingClass::ChangeOwner` | `0x0052044C PUSH 1`, `0x0052044F ECX=target`, `0x00520451 CALL [EDX+0x3D4]`; then writes `Building+0x338` from engineer type and destroys engineer | Yes for `Engineer=yes` infantry and `Capturable/NeedsEngineer` buildings |
| Older/adjacent infantry per-cell capture branches | target virtual `+0x3D4` -> `BuildingClass::ChangeOwner` | calls at `0x00519A2C` and `0x00519F7E`, both push `1` and new owner before `CALL [vtable+0x3D4]` | Yes; same infantry mission/per-cell family |
| Civilian garrison transfer | building virtual `+0x3D4(firstOccupant.Owner, 0)` -> `BuildingClass::ChangeOwner` | `0x00458316..0x00458323` | Yes for `CanBeOccupied=yes`, `Type+0x634 == -1` UC buildings |
| Civilian garrison empty revert | building virtual `+0x3D4(civilianHouse, 0)` -> `BuildingClass::ChangeOwner` | `0x004582E6 PUSH 0`, `0x004582E8 PUSH EBX`, `0x004582EB CALL [EDX+0x3D4]` | Yes for empty captured UC buildings |
| Player sell of captured civilian garrison | bypasses `ChangeOwner`; uses `BuildingClass::Sell` -> `SellBuilding` ejection -> final sell/removal | `BuildingClass__Sell @ 0x00449C30`; `SellBuilding @ 0x00457DE0`; no `ChangeOwner` branch in sell outcome | Yes after current owner is the player |
| Red-HP garrison ejection | `CheckAutoSellOrCivilian` first calls `SellBuilding`, then may call `ChangeOwner(civilian,0)` in same helper | `0x00458218..0x004582xx`, then empty branch `0x004582EB` | Yes for occupied UC building at red HP |
| Mind-control capture | victim virtual `+0x3D4(controllerOwner, 1)` -> class-specific wrapper | `CaptureManagerClass__CaptureUnit @ 0x00471D40`, call at `0x00471DB8` | Yes for mind-control weapons/systems |
| Mind-control release | victim virtual `+0x3D4(originalOwner, 1)` -> class-specific wrapper | `CaptureManagerClass__FreeUnit @ 0x00471FF0`, call at `0x004720DA` | Yes |
| Psychic Dominator area mind control | victim virtual `+0x3D4(DAT_00A9FAC8, 1)` -> class-specific wrapper | `PsychicDominator__MindControlArea`, call at `0x0053B298` | Yes when superweapon fires |
| Unit crush/hijack-style branch in `UnitClass::PerCellProcess` | unit virtual `+0x3D4(victim.Owner, 1)` -> `UnitClass` wrapper -> `FootClass` | `0x00741876 PUSH 1`, `0x0074187B CALL [EDX+0x3D4]` | Conditional; active code path |
| Slave/liberation helper `FUN_006B0AE0` | passenger/slave virtual `+0x3D4(newOwner,1)` -> class-specific wrapper | call at `0x006B0BBB` | Conditional; active for slave manager / chrono-death slave liberation contexts |
| Team/script action case `0x14` | object virtual `+0x3D4(houseByCountry, ...)` | `TeamClass__Recruit_Or_Add` decompile case `0x14` | Conditional on map script; touched-not-exhausted |

## 4. INI Keys

| Key | Default / stock evidence | Effect in this slice | Active in YR |
|---|---|---|---|
| `Engineer=` | stock engineers set yes; Mission_Capture checks infantry type byte at `+0xEC5` in current decompile | enables close-range engineer capture path that calls target `+0x3D4(...,1)` | Yes |
| `Capturable=` / `NeedsEngineer=` | stock tech/player buildings vary; existing audits distinguish the fields | decides cursor/mission eligibility before Mission_Capture; building wrapper reads `NeedsEngineer`-related byte `Type+0x1552` for capture notification branch | Conditional |
| `CanBeOccupied=` | many `rulesmd.ini` civilian structures set yes | gates garrison reconciliation and bomb-defuse skip; transfer/revert calls `+0x3D4(...,0)` | Yes for UC buildings |
| `MaxNumberOccupants=` | stock UC capacity values | affects occupant vector; first occupant owner is read at `Items[0]+0x21C` for transfer | Yes for UC buildings |
| `Unsellable=` | tech buildings often use it; ordinary captured UC player-sell path has no captured-origin exception | sell eligibility, not a `ChangeOwner` wrapper path | Conditional |
| `MindControl=yes` warheads/weapons | stock Yuri systems use it | routes into `CaptureManagerClass`, which calls victim `+0x3D4` | Yes |

## 5. Integration Points

`BuildingClass::ChangeOwner` and `FootClass::ChangeOwner` are not normally found by direct caller lists because gameplay calls the virtual slot. Direct function xrefs still prove the wrapper chain:

- `FootClass::ChangeOwner @ 0x004DBED0`: data xrefs from `AircraftClass`, `FootClass`, and `InfantryClass` vtables plus direct call from `UnitClass__Transfer_Convoy_On_Owner_Change @ 0x007463E1`.
- `UnitClass__Transfer_Convoy_On_Owner_Change @ 0x007463A0`: data xref from `UnitClass` vtable at `0x007F6044`.
- `BuildingClass::ChangeOwner @ 0x00448260`: data xref from `BuildingClass` vtable at `0x007E4290`.
- `TechnoClass::ChangeOwner @ 0x007014A0`: direct calls from `FootClass::ChangeOwner @ 0x004DBF0A` and `BuildingClass::ChangeOwner @ 0x00448BE8`, plus base Techno vtable data xref `0x007F4D34`.

## 6. Current Rust Implementation Status

Current Rust does not have an owner-transfer API that models virtual `+0x3D4` dispatch or the subclass pre/base/post sequence.

| Rust surface | Current behavior observed | Delta |
|---|---|---|
| `src/sim/world/world_orders.rs:229..244` | engineer capture directly writes `b.owner = engineer_owner`, adjusts owned counts, then despawns engineer | mismatch: bypasses `BuildingClass::ChangeOwner` pre/base/post order |
| `src/sim/passenger.rs:534..610` | garrison reconciliation directly writes `building.owner = new_owner` / `civilian_owner`; event ordering partly modeled | mismatch: owner write bypasses building wrapper, typed tracking, base transfer order, animation/remap/radar hooks |
| `src/sim/production/production_sell.rs:684..733` | player sell ejects garrison occupants then removes/refunds building | matches the verified sell bypass at the broad owner-transfer level; exact full sell state machine remains separate |
| `src/sim/entity_store.rs:140..149` and `src/sim/world/mod.rs:1201..1204` | owner index is rebuilt once per tick from direct owner fields | mismatch risk: native old-owner removal/new-owner add happen inside `ChangeOwner`, not via later global rebuild |
| mind-control surfaces | no complete native-equivalent owner-transfer wrapper found in this scan | missing/unchecked for future MC implementation |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TechnoClass::ChangeOwner @ 0x007014A0` | verified-from-prior, spot-checked | prior slot-5 report; decompile and call-site assembly here | full body not repeated |
| `FootClass::ChangeOwner @ 0x004DBED0` | verified | decompile; xrefs; assembly `0x004DBEF0..0x004DBF55` | names of vtable `+0x4E8/+0x4EC` side effects |
| `UnitClass` slot `+0x3D4 @ 0x007463A0` | verified | data xref `0x007F6044`; decompile; assembly `0x007463B0..0x007463EF` | exact gameplay scenarios that populate `+0x6C8` beyond convoy link |
| `BuildingClass::ChangeOwner @ 0x00448260` | verified for wrapper order | decompile; xrefs; assembly `0x00448260..0x0044864F`, `0x00448BC0..0x00448C05` | exact semantic names for every typed house vector flag |
| engineer capture call sites | verified | decompile `0x005202F0`; calls `0x00520451`, `0x00519A2C`, `0x00519F7E` | cursor eligibility already covered by other docs |
| garrison transfer/revert call sites | verified | decompile `0x00458200`; calls `0x004582EB`, `0x00458323` | none for owner-transfer wrapper selection |
| player sell bypass | verified | decompile `0x00449C30`, `0x00457DE0`; no `ChangeOwner` preservation branch | exact full sell animation/refund details outside this slot |
| mind-control capture/release | verified for wrapper dispatch | decompile `0x00471D40`, `0x00471FF0`; calls `0x00471DB8`, `0x004720DA` | full CaptureManager fate/RNG policies outside this slot |
| Psychic Dominator owner change | touched-not-exhausted | decompile `0x0053B260`; call `0x0053B298` | full superweapon target filtering outside this slot |
| Team/script owner-change actions | touched-not-exhausted | pattern hits/decompile in `TeamClass__Recruit_Or_Add`, case `0x14` | enumerate all map/script uses if implementation targets scripts |
| Rust comparison | verified for named touchpoints | `rg` and line reads of `world_orders.rs`, `passenger.rs`, `production_sell.rs`, `entity_store.rs`, `world/mod.rs` | no Rust edited |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Which classes override vtable +0x3D4? -> Building uses `0x00448260`; Unit uses `0x007463A0`; Aircraft/Foot/Infantry use `0x004DBED0`; base Techno uses `0x007014A0`.` (evidence: data xrefs `0x007E4290`, `0x007F6044`, `0x007E2678`, `0x007E9068`, `0x007EB42C`, `0x007F4D34`)
- `[RESOLVED] OQ-02 - Does FootClass add pre/post behavior around base ChangeOwner? -> Yes: `Type+0x5F0` pre/post vtable calls, success-only team/link cleanup, and conditional coordinate/nav write.` (evidence: `0x004DBED0`)
- `[RESOLVED] OQ-03 - Do Units go directly to FootClass? -> No; UnitClass has its own `+0x3D4` wrapper that transfers a linked convoy unit first, then calls FootClass.` (evidence: `0x007F6044`, `0x007463A0`)
- `[RESOLVED] OQ-04 - Does BuildingClass call base before or after old-owner cleanup? -> After extensive building-specific old-owner cleanup/removal and feature-off hooks; base call is at `0x00448BE8`.` (evidence: `0x00448260`, `0x00448BE8`)
- `[RESOLVED] OQ-05 - Does BuildingClass perform post-base new-owner work? -> Yes: feature re-enable, base-center/UI, vtable `+0x4DC`, walls, animation/remap, new-owner typed vectors and counters.` (evidence: `0x00448260` decompile after `0x00448BE8`)
- `[RESOLVED] OQ-06 - Which wrapper does engineer capture use? -> Target building virtual `+0x3D4`, dispatching to BuildingClass for building targets, with stack arg `1`.` (evidence: `0x00520451`; sibling calls `0x00519A2C`, `0x00519F7E`)
- `[RESOLVED] OQ-07 - Which wrapper does civilian garrison reconciliation use? -> Building virtual `+0x3D4`, stack arg `0`, dispatching to BuildingClass.` (evidence: `0x004582EB`, `0x00458323`)
- `[RESOLVED] OQ-08 - Does player sell use ChangeOwner to preserve/revert captured garrisons? -> No; sell uses `BuildingClass::Sell` and `SellBuilding` ejection, then final sell/removal if reached.` (evidence: `0x00449C30`, `0x00457DE0`)
- `[RESOLVED] OQ-09 - Which wrapper does mind control use? -> Victim virtual `+0x3D4`, so units/buildings/infantry dispatch by concrete class; CaptureManager itself does not choose a direct base helper.` (evidence: `0x00471DB8`, `0x004720DA`)
- `[RESOLVED] OQ-10 - Is Psychic Dominator a standard YR owner-transfer path? -> Yes, it calls victim virtual `+0x3D4(DAT_00A9FAC8,1)` after freeing existing capture state.` (evidence: `0x0053B298`)
- `[RESOLVED] OQ-11 - Are direct Rust owner writes equivalent? -> No; Rust direct writes bypass wrapper order and native old/new owner side effects.` (evidence: `src/sim/world/world_orders.rs:229`, `src/sim/passenger.rs:597`, `src/sim/passenger.rs:608`)
- `[RESOLVED] OQ-12 - Are sell-garrison Rust comments aligned with current binary at owner-transfer level? -> Broadly yes for player sell: sell ejects occupants then removes/refunds; no ChangeOwner preserve branch.` (evidence: `src/sim/production/production_sell.rs:681`, `0x00449C30`)
- `[DEFERRED] OQ-13 - What exact semantic names correspond to every BuildingType `+0x16xx` typed-list branch?` (category: `bounded-cost-too-high`; reason: this slot needed wrapper ordering, not a full BuildingType parser audit; next-step-if-pursued: verify each field through `BuildingTypeClass::ReadINI` string xrefs)
- `[DEFERRED] OQ-14 - Enumerate every scenario/team script opcode that can trigger virtual +0x3D4 in standard campaign maps.` (category: `requires-different-system-context`; reason: script system is outside capture/garrison/sell target; next-step-if-pursued: `/re-swarm` map script owner-change opcodes)
- `[DEFERRED] OQ-15 - Exact runtime population rules for Unit+0x6C8 convoy link.` (category: `requires-different-system-context`; reason: Unit wrapper order is verified, but convoy setup is a movement/team subsystem; next-step-if-pursued: investigate convoy link lifecycle)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Building ownership transfer must dispatch through `BuildingClass::ChangeOwner`: pre-base old-owner cleanup/list removal and associated-unit transfer, base owner write, then new-owner list/UI/anim/radar hooks. | `0x00448260`; base call `0x00448BE8`; entry calls `0x00520451`, `0x004582EB`, `0x00458323` | mismatch: direct `owner = ...` writes | `src/sim/world/world_orders.rs`; `src/sim/passenger.rs`; future owner-transfer API | Replace direct capture/garrison owner mutations with a building-owner-transfer operation that preserves native old-before-base/new-after-base ordering as implemented features come online. | Engineer captures a powered/tech/garrison building: old owner loses building before base owner pointer write, new owner receives building-side effects after; events and indices observe the same order. | Do not update `EntityStore` by late rebuild as the only side effect; native changes tracking inside the transfer. |
| Unit ownership transfer must run the UnitClass convoy wrapper before FootClass/Base when `Unit+0x6C8` is populated. | `0x007463A0`; data xref `0x007F6044`; call `0x007463C9` then `0x007463E1` | missing/unchecked; no owner-transfer API found | future unit mind-control/capture owner-transfer surface | Dispatch by entity category/concrete class; for vehicles, handle convoy-linked transfer before common foot/base effects. | Mind-control or scripted owner change on a convoy-linked unit transfers the linked unit first and toggles `+0x6D0` equivalent before parent transfer. | Do not treat all mobile objects as plain FootClass. |
| Infantry and aircraft owner transfer use `FootClass::ChangeOwner`, not direct base. | vtable data xrefs `0x007E2678`, `0x007E9068`, `0x007EB42C`; direct base call `0x004DBF0A` | missing/unchecked for future mind-control/release paths | future mind-control / Psychic Dominator implementation | Preserve Foot wrapper pre/post side effects around base owner transfer. | Mind-controlling infantry with a team link performs wrapper cleanup before returning to guard/fate logic. | Do not call only a base `set_owner` helper for infantry/aircraft. |
| Engineer capture calls building virtual `+0x3D4(newOwner,1)`, then writes building `+0x338` from engineer type and destroys the engineer. | `0x0052044C..0x0052046D` | mismatch: direct owner write, owned-count mutation, despawn | `src/sim/world/world_orders.rs:229..244` | Capture should call the building transfer operation, then apply the engineer type tag equivalent and consume engineer in native order. | Capture a tech building with an engineer: captured state/visual/tag/engineer consumption all occur in the same order as native. | Do not mutate house counts manually around direct owner write; wrapper/base own that sequencing. |
| Civilian garrison transfer/revert calls building virtual `+0x3D4(...,0)` from `CheckAutoSellOrCivilian`; boarding itself still does not transfer owner. | `0x00522910`; `0x004582EB`; `0x00458323` | partial: Rust models delayed reconciliation but still direct-writes owner | `src/sim/passenger.rs:367..390`, `:534..610` | Keep live-object-order reconciliation, but route the owner mutation through the building transfer operation with announce flag/equivalent false. | Occupier enters neutral UC building before building update: transfer occurs on building turn through wrapper, not during boarding. | Do not regress to immediate boarding-time owner transfer. |
| Player sell is a ChangeOwner bypass: occupied captured garrison sells by ejecting occupants and removing/refunding the building, not by reverting owner. | `0x00449C30`; `0x00457DE0`; no `ChangeOwner` in sell outcome | broad owner-transfer behavior now appears aligned | `src/sim/production/production_sell.rs:681..733` | Keep player-sell distinct from empty/revert reconciliation. | Captured UC building owned by player is sold: occupants eject, building is removed/refunded; no owner-revert preservation branch. | Do not merge `SellBuilding` helper semantics with `CheckAutoSellOrCivilian` revert semantics. |
| Mind control and Psychic Dominator call victim virtual `+0x3D4`; concrete class decides Building/Unit/Foot wrapper. | `0x00471DB8`, `0x004720DA`, `0x0053B298` | missing/unchecked | future mind-control/superweapon owner-transfer surfaces | Owner transfer API must dispatch by concrete entity class and support release to original owner through the same wrapper. | Yuri mind-controls a vehicle, then releases it: capture and release both run class-specific wrappers with base owner change in the middle. | Do not implement mind control as a raw `entity.owner = controller` plus flag. |

### Stale Docs / Follow-up Docs

- `TECHNOCLASS_SYSTEMS_GHIDRA_REPORT.md` section 5.2 should not describe ownership change as only the base `TechnoClass` operation. Replacement framing: "Virtual `+0x3D4` dispatches through concrete wrappers. Building, Unit, and Foot/Infantry/Aircraft wrappers perform pre/post work around the base `TechnoClass::ChangeOwner`; callers such as engineer capture, garrison reconciliation, mind control, and Psychic Dominator must use the concrete dispatch path."
- `BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md` implication text that says garrison-only ownership can be simplified to setting `entity.owner` plus vision rebuild is stale for exact parity. Replacement framing: "Even garrison reconciliation reaches full `BuildingClass::ChangeOwner(...,0)`. Current Rust may stage implementation, but direct owner assignment is not mechanism-equivalent to native wrapper order."

## Sources

- Ghidra read-only decompile: `0x007014A0`, `0x004DBED0`, `0x007463A0`, `0x00448260`, `0x005202F0`, `0x00458200`, `0x00449C30`, `0x00457DE0`, `0x00471D40`, `0x00471FF0`, `0x0053B260`, `0x006B0AE0`, `0x007416A0`.
- Ghidra read-only xrefs/data: `get_function_xrefs` / `get_xrefs_to` for `0x004DBED0`, `0x00448260`, `0x007014A0`, `0x007463A0`; vtable entries `0x007E2678`, `0x007E9068`, `0x007EB42C`, `0x007F6044`, `0x007E4290`, `0x007F4D34`.
- Ghidra read-only assembly/context: `0x004DBEF0..0x004DBF55`, `0x007463B0..0x007463EF`, `0x00448BC0..0x00448C05`, `0x005203F0..0x0052042F`, `0x00471D9F..0x00471DCF`, `0x00458200..0x0045832F`, `0x00449C30..0x0044A9FF`.
- Prior reports used as maps/checks: `TECHNOCLASS_CHANGEOWNER_LIFECYCLE_ORDER_RESWARM_20260528.md`, `BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md`, `CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md`, `CAPTURED_CIVILIAN_GARRISON_SELL_OUTCOME_GHIDRA_REPORT.md`, `SELECTION_LIFECYCLE_GHIDRA_REPORT.md`, `FOOTCLASS_VTABLE_COMPLETE.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust scanned/read: `src/sim/world/world_orders.rs`, `src/sim/passenger.rs`, `src/sim/production/production_sell.rs`, `src/sim/entity_store.rs`, `src/sim/world/mod.rs`, `src/sim/game_entity.rs`.

## Status

COMPLETE for the requested subclass/wrapper ordering and standard YR capture/garrison/sell entry-point selection. Broader scenario script owner-change opcodes and convoy-link population are explicitly deferred because they are separate systems and do not change the verified wrapper dispatch contract.
