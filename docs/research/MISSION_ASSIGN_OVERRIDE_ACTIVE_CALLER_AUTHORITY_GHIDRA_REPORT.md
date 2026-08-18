# Mission Assign / Override Active Caller Authority — Ghidra Report

**Date:** 2026-07-22  
**Binary:** active retail Yuri's Revenge `gamemd.exe`, x86-32  
**Status:** **COMPLETE for the bounded direct/virtual caller census and receiver/argument/order authority**  
**Mode:** exhaustive slice; Ghidra read-only; no Rust, INI, contract, or plan edits

## 1. Target question

Close the active-YR production caller surface for:

- `MissionClass::Assign_Mission @ 0x005B2FD0`, virtual slot `+0x1F0`;
- `MissionClass::Override_Mission @ 0x005B3650`, virtual slot `+0x1F4`;
- the Techno override wrapper `0x007013A0` and Foot/Aircraft override layers.

For every executable call instruction, prove the receiver family, arguments, relevant
Target/NavCom order, gameplay context, and active-YR reachability. Then compare the result
with the current Rust authority surface.

### Non-goals

- Re-proving the settled Assign/Override mission-byte bodies.
- Re-investigating `+0xB8`, `+0xCC`, ReadyToCommence, Queue, Commence, Restore, or the full
  mission dispatcher.
- Exhausting the Aircraft gate semantics owned by the parallel Aircraft report. This report
  proves where the gate sits and whether a call reaches the common helper chain.
- Implementing or editing Rust.

### Stop conditions

The slice is complete when all direct xrefs and all executable `CALL [reg+0x1F0]` /
`CALL [reg+0x1F4]` encodings are classified, plausible split dispatch is excluded, leaf
vtable routing is proven, each true call has an argument/order ledger, and false-positive
slot collisions are receiver-proven.

## 2. Executive verdict

The bounded census is closed:

- **Assign:** 29 executable `+0x1F0` call instructions exist. **17 are Mission Assign** and
  **12 are BulletClass slot collisions**. The only direct code call to the base Assign body
  is from the Aircraft Assign override.
- **Override:** exactly **10 executable `+0x1F4` call instructions** exist, all true Mission
  Override calls. One SpyPlane call instruction has two predecessor argument routes, so the
  ten instructions represent eleven local argument packets.
- **Override is a three-stack-argument virtual API.** The base body returns with `RET 0xC`
  and consumes only the mission selector, but Techno/Foot/Aircraft layers consume the other
  two values as Target and NavCom/destination authority.
- Active final leaf routing is category-specific:
  - Building: archive Target -> base Override -> set Target.
  - Unit/Infantry: archive NavCom -> archive Target -> base Override -> set Target -> set
    destination.
  - Aircraft: apply the Aircraft mission gate first, then run the Unit/Infantry chain.
- Current Rust has **zero production callers** of `verb::override_mission`, cannot express
  the Target/NavCom packet or category chain, contains a production `current`-only retask
  helper, and rewrites `mission.current` from legacy machines during AI/tick projection.
  Therefore caller authority is **DRIFT**, not merely unwired parity.

## 3. Evidence and coverage ledger

### 3.1 Census method

1. `get_xrefs_to` was run on `0x005B2FD0`, `0x005B3650`, `0x007013A0`,
   `0x004D8F40`, `0x0041B9F0`, and `0x0041BB30`.
2. Every x86 ModR/M register encoding of `FF /2 [reg+disp32]` was byte-searched for
   displacements `F0 01 00 00` and `F4 01 00 00` (`search_byte_patterns`).
3. `search_instructions` independently searched `CALL` with operand `0x1f0`/`0x1f4`.
4. Program-wide `MOV` and `JMP` searches for the same offsets checked split-load/tail-call
   alternatives. There are no `JMP [reg+0x1F0]` or `JMP [reg+0x1F4]` instructions. The
   few offset-looking `MOV` loads are ordinary object/array fields and do not feed a
   register-indirect call before overwrite/control transfer (spot-checked at `0x00482F22`,
   `0x0054CD0C`, `0x00483111`, and `0x005F3ED1`).
5. Each candidate instruction was inspected in disassembly/decompile context, and receiver
   identity was proved from object construction, RTTI/COL-backed vtables, or an owner pointer
   held by a named locomotor.

Result: **no unclassified executable call instruction remains** in the bounded slot census.

### 3.2 Direct-body xrefs

| Callee | Code xrefs | Meaning |
|---|---:|---|
| base Assign `0x005B2FD0` | `0x0041BA34` only | Aircraft override delegates to base |
| base Override `0x005B3650` | `0x007013C1` only | Techno wrapper delegates to base with all three stack args |
| Techno wrapper `0x007013A0` | `0x004D8F61` only | Foot wrapper delegates to Techno wrapper |
| Foot wrapper `0x004D8F40` | `0x0041BB7E` only | Aircraft wrapper delegates to Foot wrapper |

Data xrefs are vtable bindings and are covered below. Evidence: `get_xrefs_to` on each
address plus raw vtable reads.

### 3.3 Vtable receiver routing

| RTTI-proved table | vtable base | `+0x1F0` Assign | `+0x1F4` Override |
|---|---:|---:|---:|
| `AircraftClass` | `0x007E22A4` | `0x0041B9F0` | `0x0041BB30` |
| `BuildingClass` | `0x007E3EBC` | `0x005B2FD0` | `0x007013A0` |
| `FootClass` | `0x007E8C94` | `0x005B2FD0` | `0x004D8F40` |
| `InfantryClass` | `0x007EB058` | `0x005B2FD0` | `0x004D8F40` |
| `MissionClass` | `0x007EDCC0` | `0x005B2FD0` | `0x005B3650` |
| `RadioClass` | `0x007F0508` | `0x005B2FD0` | `0x005B3650` |
| `TechnoClass` | `0x007F4960` | `0x005B2FD0` | `0x007013A0` |
| `UnitClass` | `0x007F5C70` | `0x005B2FD0` | `0x004D8F40` |

The RTTI TypeDescriptor strings (`.?AVAircraftClass@@`, `.?AVBuildingClass@@`, etc.) were
read through each table's Complete Object Locator. Aircraft/Building/Infantry/Unit are the
active concrete stock leaves. Mission/Radio/Techno/Foot bindings explain base construction
and inheritance; they are not evidence that a standard final Techno bypasses its leaf slot.

## 4. Override ABI and category-owned ordering

### 4.1 Caller-layer signature versus base-body consumption

The virtual packet is:

```text
Override_Mission(mission, target, destination_or_navcom)
```

At `0x005B3650`, assembly ends in `RET 0xC`. The base body reads the first stack value
(mission) and does not read the target or destination values. That does **not** make the
virtual API one-argument: its wrappers and callers supply/consume all three values.

This distinction is load-bearing because decompiler prototypes often collapse the two
forwarded values. Evidence: `disassemble_function 0x005B3650`,
`disassemble_function 0x007013A0`, `disassemble_function 0x004D8F40`, and raw caller
disassembly.

### 4.2 Techno wrapper `0x007013A0`

Verified instruction order:

1. Read old Target `this+0x2B4`.
2. Write it to archived Target `this+0x2B8` (`0x7013B0..0x7013BB`).
3. Push destination, target, mission and directly call base Override (`0x7013C1`).
4. Call vtable `+0x3C8` with the target argument (`0x7013C6..0x7013CB`).
5. `RET 0xC`.

Thus the common Techno order is **archive old Target before mission mutation, then install
the new Target after the base verb**. The third argument is forwarded to the base for ABI
preservation but otherwise unused by this layer.

### 4.3 Foot wrapper `0x004D8F40`

Verified order:

1. Copy old NavCom `this+0x5A4` to suspended/archived NavCom `this+0x5A8`.
2. Forward all three values to `0x007013A0`.
3. Call vtable `+0x480` with `(destination, 1)`.
4. `RET 0xC`.

For Unit/Infantry the full observable sequence is therefore:

```text
archive NavCom -> archive Target -> base Override -> set Target -> set destination
```

### 4.4 Aircraft wrapper `0x0041BB30`

Aircraft first applies its current/new-mission gate. A passing call forwards all three
arguments to the Foot wrapper at `0x0041BB7E`; a suppressed call returns without any of the
Foot/Techno archive or setter effects. The exhaustive gate/body semantics belong to
`AIRCRAFT_MISSION_VERB_OVERRIDE_FAMILY`; this report relies only on the verified position
of that gate in front of the common chain.

### 4.5 Building versus Foot consequence

Building's concrete slot points directly to the Techno wrapper, so a Building archives and
sets Target but does **not** archive NavCom or call the Foot destination setter. Applying the
Foot chain uniformly to all Technos would be DRIFT.

## 5. Complete Mission Assign caller census

There are **17 true Mission Assign call instructions**.

| Call instruction(s) | Caller/context | Receiver proof | Mission and local order | Active-YR classification |
|---|---|---|---|---|
| `0x00443ECD`, `0x00443F29`, `0x00443F49` | `BuildingClass::ExitObject_Main` | produced Foot/Unit child in `EBP`; leaf Techno vtable calls surround it | Each branch first commits a destination through `+0x480`, then Assigns `2` (Move) | Active conditional factory/building exit |
| `0x0044C910` | `BuildingClass::MissionRepairAndProduce` | produced/repaired Techno receiver | preceding virtual `+0x484` call, then Assign `5` (Guard), then later class-specific continuation | Active conditional production/repair release |
| `0x004525BE` | `FUN_00452540`, Building animation/mission transition | receiver has Building type/animation fields and Building virtuals | Assign `-1`, Queue `0x18` (Open), then Commence immediately | Active conditional Building path |
| `0x0051F4C6`, `0x0051F5B3` | Infantry routines `FUN_0051F3E0` / `FUN_0051F540` | Infantry receiver/type fields and vtable | set destination to current Target first, then Assign `8` (Capture) | Active conditional AI Infantry entry/capture paths |
| `0x0051F7CF` | Infantry routine `FUN_0051F6E0` | Infantry receiver | Assign `5` (Guard), then set destination `(0,1)` | Active conditional Infantry type path |
| `0x005D7420`, `0x005D742E` | shared MultiplayerGameMode initial-unit reassignment body beginning `0x005D70F0` | selected object is RTTI-cast to Techno (`.?AVTechnoClass@@`), changed to target House, then called through its Techno vtable | `HouseClass::IsPlayerControl(target house)` true -> Assign `5` (Guard); false -> Assign `0xB` (AreaGuard) | Active conditional multiplayer startup; shared by RTTI-proved MultiplayerBattle, MPCooperative, FreeForAll, MultiplayerManBattle, Megawealth, MultiplayerSiege, UnholyAlliance, and MultiplayerGameMode tables |
| `0x00688D89`, `0x00688D98` | `ScenarioClass::Generate_Random_Units` | newly generated Unit in `EBP` | non-player house -> Assign `0xB` (AreaGuard); player-controlled -> Assign `5` (Guard) | Active conditional random-unit generation |
| `0x007017BA` | `TechnoClass::ChangeOwner` | `this` Techno | conditional ownership-change reset Assign `5` (Guard) after owner/house state work | Active conditional owner change |
| `0x0070F87B` | shared Techno stop/guard reset helper `0x0070F850`, vtable `+0x3D0` on all concrete leaves | RTTI-backed Aircraft/Building/Foot/Infantry/Techno/Unit slot bindings | set destination `(0,1)` -> set Target `0` -> write `this+0x218=0` -> Assign `5` (Guard) | Active; 13 callers include CaptureManager, SlaveManager recall/deploy/return, deploy/undeploy, and open-transport paths |
| `0x00741A6D` | `TechnoClass::Set_Destination` | Unit/Techno receiver | branch-specific Assign `1` (Attack), then returns | Active conditional destination/occupation response |
| `0x007425AA` | `TechnoClass::Set_Destination` | Unit/Techno receiver | release locomotor interface -> Assign `-1` -> Queue `7` (Enter), then set local flags | Active conditional enter/dock path |
| `0x00710140` | `TechnoClass::PerformDeploy` | Techno receiver | release interface -> Assign `-1` -> Queue `0xD` (Stop), then common deploy continuation | Active conditional deploy path |

Evidence: exact instruction search plus per-caller disassembly/decompile. Mission names use the
verified numeric mission table; the numeric values remain authoritative.

### 5.1 Assign false-positive ledger: twelve BulletClass calls

The remaining **12** executable `CALL [reg+0x1F0]` instructions are not mission calls:

| Instructions | Caller families |
|---|---|
| `0x0044CC38`, `0x0044D47A` | `BuildingClass::Mission_Missile` |
| `0x0046A0D5`, `0x0046A28A` | `WarheadTypeClass::Detonate` |
| `0x0046A875`, `0x0046AD29` | `BulletClass::SpawnShrapnel` |
| `0x0046B52F` | `NukeMaker::SpawnDownwardNuke` |
| `0x006E35CB`, `0x006E3AFC` | House nuke-launch paths |
| `0x006CDC7F` | `SuperClass::Launch` |
| `0x006FF014`, `0x006FF86C` | `TechnoClassFireAtSpawnsBullet` |

Receiver proof is exact:

- `BulletClass` vtable base `0x007E46E4` has `+0x1F0 = 0x00468670`.
- its COL/TypeDescriptor resolves to `.?AVBulletClass@@` (string at `0x0081AF78`).
- `0x00468670` takes two stack arguments, manipulates Bullet launch/reveal state, returns a
  boolean, and ends `RET 8`.
- each listed callsite holds the newly created/current Bullet receiver and passes launch
  coordinate/vector values; several immediately test `AL`.

Treating a raw `+0x1F0` search hit as Mission Assign without receiver proof is a confirmed
false-positive trap.

## 6. Complete Mission Override caller census

There are exactly **10 true Mission Override call instructions**.

### 6.1 Aircraft SpyPlane: one instruction, two packets

`AircraftClass::Mission_SpyPlane @ 0x00417300` reaches `CALL [vtable+0x1F4]` at
`0x00417499` through two predecessor routes:

1. `Override(7 /* Enter */, 0, EAX)` after virtual `+0x528` returns a non-null object/value
   (`0x417434..0x417451`).
2. `Override(1 /* Attack */, EAX, 0)` after virtual `+0x3C4` selects a non-null object whose
   `+0x14` flag has bit 1 (`0x417453..0x417499`).

`EBX` is zeroed at `0x00417428`. After either packet returns, the caller writes mission
substate `+0xBC = 0`. Receiver is concrete Aircraft, so the Aircraft gate runs before the
Foot/Techno chain.

The semantic name of the object returned by `+0x528` is left **UNKNOWN** here; its role as
the third Override packet value and its ordering are verified.

### 6.2 Locomotor obstruction family: eight instructions

| Call instruction(s) | Locomotor/context | Receiver | Verified packet |
|---|---|---|---|
| `0x004B3BE9` | `DriveLocomotionClass::Process_Movement` | owner Foot/Unit at `locomotor+0xC` | `Override(Attack, hostile blocking Techno or armed-overlay Cell, 0)` |
| `0x00515C2C`, `0x00515C9C` | Hover movement body `FUN_00514F70` | owner Foot/Unit at `locomotor+0xC` | same two target routes, destination zero |
| `0x005B0E88`, `0x005B0EE6` | Mech movement body `FUN_005B01C0` | owner Foot/Unit at `locomotor+0xC` | same two target routes, destination zero |
| `0x006A3238` | `ShipLocomotionClass::Process_Movement` | owner Unit at `locomotor+0xC` | same obstruction packet, destination zero |
| `0x0075BAEB`, `0x0075BB49` | `WalkLocomotionClass::ProcessMovement` | owner Foot/Infantry at `locomotor+0xC` | same two target routes, destination zero |

The two routes are:

- a non-allied blocking Techno returned by `CellClass::Find_Blocking_Object`; or
- a Cell target constructed for an armed overlay when no blocking Techno is returned.

Every route pushes the third value `0`, then target, then mission `1`. The receiver is the
locomotor's owner, not the locomotor. Therefore Unit/Infantry dispatch through the Foot
wrapper: old NavCom and old Target are archived before Attack becomes current; Target is
then installed and destination is set to zero.

### 6.3 Damage response: one instruction

At `TechnoClass::ReceiveDamage + 0x1241` (`0x00702B41`), the packet is:

```text
Override(1 /* Attack */, attacker/source Techno, 0)
```

Disassembly at `0x702B31..0x702B41` loads the source-Techno parameter, pushes `0`, source,
and `1`, then calls the victim's virtual `+0x1F4`. The source parameter identity is also
verified by `RECEIVE_DAMAGE_GHIDRA_REPORT.md`'s seven-argument signature.

The victim is polymorphic, making category routing observable:

- Building: Target archive/set only.
- Unit/Infantry: NavCom plus Target archive/set and destination zero.
- Aircraft: Aircraft gate may suppress the entire common chain; if it passes, Foot order
  applies.

This is an active conditional damage/response path. Full retaliation policy is outside this
report.

## 7. Current Rust authority comparison

### 7.1 Production call surface

Current source search (`rg`) proves:

- `src/sim/mission/verb.rs:110` defines `override_mission`, but its only callers are unit
  tests in that file. **There is no production Override caller.**
- `src/sim/mission/retask.rs:72-82` exposes `assign_mission_with_teardown` and correctly
  funnels its mission-byte write through `verb::assign_mission`, after a Rust-specific dock
  teardown selection.
- `src/sim/mission/retask.rs:89-98` exposes `assign_mission_keep_fields`, which writes only
  `mission.current`. It bypasses every native verb and preserves all other fields by fiat.
- Four live player-command sites use that current-only path:
  `world_commands.rs:336`, `:365`, `:391`, and `:428` (Attack, ForceAttack,
  ForceAttackCell, AttackMove).
- Seven command families use `assign_mission_with_teardown` at `world_commands.rs:149`,
  `:291`, `:812`, `:916`, `:1077`, `:1185`, and `:1387`.
- Bunker install/release call `verb::assign_mission` directly at
  `docking/bunker_link.rs:57` and `:138`.

### 7.2 Authority conflict, not only missing wiring

`src/sim/world/techno_ai.rs:544-552` and `src/sim/world/mod.rs:980-1003` describe MissionCom
as a projection of legacy `Option<T>` machines and overwrite `mission.current`/`substate`
during object AI or tick-tail refresh. Consequently, even a future verb call can be
clobbered by the projection layer unless the authority boundary changes.

The current `override_mission(com, mission, now)` API also lacks:

- target argument;
- destination/NavCom argument;
- archived Target/NavCom ownership;
- category-specific Building/Foot/Aircraft dispatch;
- the Aircraft pre-gate.

This caller-layer mismatch is independently sufficient for **DRIFT**. It is separate from
the already-documented byte-body discrepancies inside `verb::override_mission`.

### 7.3 What the census does and does not authorize

The census authorizes Rust wrappers for the verified native caller families (locomotor
obstruction, ReceiveDamage response, SpyPlane, ownership/spawn/reset/deploy paths). It does
**not** prove that every current Rust player command should mechanically change from
`assign_mission_keep_fields` to Override. Those command-to-native-entry mappings need their
own action trace; replacing them wholesale would be an unsupported inference.

## 8. Implementation handoff

1. Add a Simulation/entity-level three-value Override authority surface that owns the exact
   category chain: Aircraft gate; optional Foot NavCom archive/destination setter; Techno
   Target archive/setter; base mission-byte verb. Keep base-byte logic separate from
   caller-layer Target/NavCom effects.
2. Cut over MissionCom from tail/AI projection to authoritative state before wiring live
   Override callers. Remove or strictly bound current-only writes such as
   `assign_mission_keep_fields`; map player command sites only after a native action trace.
3. Wire the closed native caller families incrementally, starting with locomotor obstruction
   and ReceiveDamage packets, and preserve per-call order. Add a receiver-class regression
   matrix so Building, Unit/Infantry, and Aircraft cannot collapse into one sequence.

Suggested executable test names:

- `override_routes_target_and_navcom_in_native_category_order`
- `aircraft_override_gate_suppresses_all_archive_and_setter_effects`
- `locomotor_blocker_override_attacks_target_with_zero_destination`
- `receive_damage_override_uses_source_and_leaf_receiver_chain`
- `mission_shadow_does_not_clobber_live_verb_state`
- `slot_1f0_census_rejects_bullet_receiver_collisions`

## 9. Do-not-dos

- Do not model Override as a one-argument API merely because the base body reads only
  mission; `RET 0xC` and the wrapper chain prove three-value authority.
- Do not set the new Target/NavCom before archiving the old values.
- Do not apply Foot NavCom archive/destination effects to Building.
- Do not bypass the Aircraft pre-gate or perform partial archive/setter effects when it
  suppresses a call.
- Do not classify raw `+0x1F0` call hits as Assign without receiver proof; twelve active
  Bullet calls use the same numeric slot.

## 10. Adversarial and cold checks

1. **Slot collision:** all 29 `+0x1F0` instructions were reclassified by receiver; the 12
   non-mission calls converge on RTTI-proved Bullet slot `0x00468670`.
2. **Decompiler arity loss:** raw pushes and `RET 0xC` were used instead of collapsed
   pseudocode for Override.
3. **Split dispatch:** program-wide MOV/JMP offset searches found no alternate Mission slot
   call route.
4. **Intermediate-vtable trap:** final Aircraft/Building/Infantry/Unit slots were read
   directly; base Mission/Radio/Techno/Foot tables were not treated as final-leaf behavior.
5. **Unnamed-owner trap:** `0x005D70F0` was not left as an anonymous callsite. RTTI/COL data
   proves the eight multiplayer game-mode tables sharing it, and the selected receiver is
   RTTI-cast to Techno before Assign.

Cold spot checks:

- Drive obstruction at `0x004B3BE9`: raw disassembly proves third arg zero, target from the
  blocker/cell route, mission `1`, receiver from locomotor owner.
- SpyPlane at `0x00417499`: predecessor-stack reconstruction proves both the Enter and Attack
  packets; treating the instruction as only Attack would be incomplete.
- Bullet vtable `0x007E46E4 + 0x1F0`: raw pointer is `0x00468670`, and TypeDescriptor string
  is `.?AVBulletClass@@`.

## 11. Negative facts

- No active concrete standard-YR leaf directly dispatches its `+0x1F4` slot to the base
  `0x005B3650`; final leaves reach it through Techno/Foot/Aircraft wrappers.
- No true Mission Override caller supplies a nonzero third argument except the SpyPlane
  Enter route.
- No locomotor obstruction Override assigns Guard, Move, or Enter; all eight instructions
  assign Attack (`1`).
- No production Rust caller invokes `verb::override_mission`.
- No complete caller census can be obtained by xrefs to `0x005B2FD0`/`0x005B3650` alone;
  virtual calls dominate.

## 12. Uncertainty and deferrals

The bounded census, receiver identities, argument packets, helper order, and active code
reachability are complete. The following names/policies are deliberately not upgraded:

- the semantic name of SpyPlane virtual `+0x528`'s returned third packet value is UNKNOWN;
- the semantic name of the preceding `+0x484` call at `0x0044C910` is not required for the
  Assign order and remains UNCHECKED here;
- exact Aircraft gate truth-table semantics are delegated to the parallel Aircraft report;
- exact mapping from current Rust player commands to original input/action entry points is
  deferred to an action trace.

None of these deferrals leaves a call instruction, receiver, argument packet, or relevant
Target/NavCom order unclassified.

## 13. Stale wording discovered

`docs/research/MISSIONCLASS_VERB_API_GUARDS_OVERRIDE_RESTORE_SEMANTICS_GHIDRA_REPORT.md`
lines 511-514 says all five verbs are called on every active Techno whenever a mission
transition occurs and summarizes callers only as “throughout the codebase.” That is too
broad and is not a caller census. Replace it with the per-verb active caller reports; for
Assign/Override the authoritative counts are **17 true Assign instructions**, **10 true
Override instructions**, and **12 excluded Bullet slot collisions**.

`src/sim/mission/retask.rs` lines 85-88 also states, without native caller evidence, that
four combat commands “should not wipe a pending restore.” The current-only write may be a
temporary migration policy, but it must not be cited as gamemd authority.

No stale document was edited because this task owns exactly one new report.

## 14. Sources

Primary binary evidence used this session:

- `search_byte_patterns` for every `FF /2 [reg+0x1F0]` and `[reg+0x1F4]` encoding.
- `search_instructions` for CALL/MOV/JMP operands `0x1f0` and `0x1f4`.
- `get_xrefs_to` for `0x005B2FD0`, `0x005B3650`, `0x007013A0`, `0x004D8F40`,
  `0x0041B9F0`, `0x0041BB30`, `0x00468670`, and relevant data/vtable addresses.
- `read_memory` / `inspect_memory_content` for concrete vtables, COLs, and RTTI strings.
- `disassemble_bytes`, `disassemble_function`, and `decompile_function` for every listed
  caller family and helper chain.

Corroborating research read before/while investigating:

- `MISSIONCLASS_VERB_API_GUARDS_OVERRIDE_RESTORE_SEMANTICS_GHIDRA_REPORT.md`
- `MISSION_RAW_BYTES_0XB8_0XCC_FULL_CENSUS_GHIDRA_REPORT.md`
- `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md`
- `FOOTCLASS_FIELD_0xAC_LOCOMOTOR_AND_BASE_FIELD_AUTHORITY_GHIDRA_REPORT.md`
- `RECEIVE_DAMAGE_GHIDRA_REPORT.md`

## 15. Final status

**COMPLETE** for `MISSION_ASSIGN_OVERRIDE_ACTIVE_CALLER_AUTHORITY`.

All bounded production call instructions are classified with receiver proof; the
three-argument Override ABI and concrete leaf ordering are closed; false slot collisions
are explicitly excluded; Rust authority drift and safe implementation boundaries are
recorded. No Ghidra mutations and no Rust/INI/contract/plan changes were made.
