# Building Death Radio 0x17 Close/Helipad Gates - Re-Swarm Research Report

**Address(es):** `0x00442230` (`BuildingClass::ReceiveDamage`), focus range `0x004424A2..0x004425F4`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** The death-result contact loop in `BuildingClass::ReceiveDamage`, specifically which listed contacts receive sent radio `0x17`, which instead take close/helipad damage, and the order relative to linked undock and later destruction cleanup.  
**Non-Scope:** Full building death effects, full `BuildingClass::ChangeOwner`, computed non-immediate radio `0x17` producers, and complete receiver implementations beyond the facts needed to classify this sender.  
**Confidence:** High for the contact gates and operation order; Medium for semantic naming of `target+0x500`, which is verified as a write but not renamed here.  
**Active in YR:** Yes. The path is the ordinary building damage death case. The helipad exception is active for stock `[GAAIRC]` and `[AMRADR]` because `rulesmd.ini` sets `Helipad=yes`.

## Working Notes Gate

- Target question: In `BuildingClass::ReceiveDamage` death case, exactly which contacts get sent radio `0x17`, which contacts are routed into the close/helipad damage branch, and what state writes happen in each branch?
- Non-goals: Re-decode building destruction effects, owner-change contact retention, sell broadcast `0x17`, or every `0x17` receiver body.
- Evidence needed to mark COMPLETE: decompile plus assembly for the list source, linked `+0x2E4` removal, distance/helipad predicates, radio send, damage call, target write, INI proof for stock helipad activity, and Rust scan.
- Stop conditions: Stop at this one loop; record wider damage pipeline or receiver details as out-of-scope if they appear.

## 1. Overview

When a building dies, `BuildingClass::ReceiveDamage` snapshots the building's current RadioClass contacts into a temporary pointer list before final teardown. The linked dock occupant at `building+0x2E4` is removed from that temporary list before `UndockUnit`; every remaining listed contact is then classified by distance and by the dying building type's `Helipad` byte.

Only far contacts of non-helipad buildings receive directed sent radio `0x17`. Close contacts (`distance < 0x100`) and all contacts of a helipad building take a direct damage virtual call instead, using `Rules+0xFA8` (`C4Warhead`) and damage equal to the target type's strength multiplied by 10.

## 2. Class Layout / Key Offsets

| Offset / slot | Owner | Meaning in this slice | Evidence |
|---:|---|---|---|
| `+0xE4` | `RadioClass` base | Contacts pointer array | helper at `0x0065AD30` loads `[ECX+0xE4]`, then `[array + index*4]` |
| `+0xE8` | `RadioClass` base | Contact slot count / loop bound | `0x004422DB`, `0x0044234D..0x00442356` |
| `+0x2E4` | `BuildingClass` / docked unit peer | Linked dock occupant pointer removed from the temporary contact list before `UndockUnit` | `0x004424A2..0x004424EA` |
| `+0x520` | `BuildingClass` | Type pointer used to read building-type bytes | `0x0044258D` |
| `+0x16CB` | `BuildingTypeClass` | `Helipad=yes` byte; nonzero forces damage branch instead of sent `0x17` | `0x00442593..0x0044259B`; `rulesmd.ini:11820`, `12342` |
| `+0xA0` | contacted target's type | Strength-like integer used as close/helipad damage base | `0x004425BA..0x004425D6`; prior damage reports identify `Type+0xA0` as `Strength` |
| `+0x500` | contacted target object | Cleared only in far/non-helipad sent-radio branch | `0x004425AA` |
| vtable `+0x48` | contacted target and dying building | GetCoords for distance vector | `0x0044252E..0x00442543` |
| vtable `+0x278` | dying building radio object | Directed `Transmit_Radio(msg, target)` | `0x0044259D..0x004425A4` |
| vtable `+0x16C` | contacted target | ReceiveDamage virtual used by close/helipad branch | `0x004425B6..0x004425EE` |
| `Rules+0xFA8` | `RulesClass` | `C4Warhead` pointer | `0x004425CF..0x004425E4`; `rulesmd.ini:818`; `RULESCLASS_FIELDS.csv` |

## 3. Core Logic

### 3.1 Temporary contact list construction

The loop starts before the damage result switch. `BuildingClass::ReceiveDamage` reads the building radio slot count from `building+0xE8` and iterates slot indexes upward from `0` while `index < count`.

For each slot:

1. `0x004422F9..0x00442301`: pushes the slot index and calls `0x0065AD30`.
2. `0x0065AD30..0x0065AD3D`: the helper reads `Contacts[index]` from `this+0xE4`.
3. `0x00442303..0x00442305`: null contacts are skipped.
4. `0x0044230F..0x0044234A`: non-null contacts are appended to a temporary DynamicVector-like pointer list, growing the vector in chunks of `10` when needed.

Active in YR: Yes. This runs at entry to `BuildingClass::ReceiveDamage`, before the result switch, for ordinary building damage.

Tiny details:

- The source is the RadioClass contact slot array, not a refinery-specific waiting queue.
- Iteration is ascending contact-slot order.
- Null slots are skipped and do not occupy entries in the temporary list.
- The temporary list is built before `TechnoClass::ReceiveDamage` result handling, so it snapshots contacts as they existed at function entry.
- The temporary vector grows with a literal `10` capacity increment (`0x004422EB`, `0x0044231F..0x00442334`).

### 3.2 Linked `+0x2E4` occupant removal happens before the loop

In death result case `4`, if `building+0x2E4` is non-null, the code first removes that exact pointer from the temporary contact list:

1. `0x004424A2`: reads `building+0x2E4`.
2. `0x004424A8..0x004424BB`: searches the temp list for the address of `building+0x2E4`.
3. `0x004424BE..0x004424E8`: if found and in range, decrements the temp count and shifts later entries left by one.
4. `0x004424EA`: calls `BuildingClass::UndockUnit`.

Active in YR: Yes when a building dies with a linked dock occupant. This includes refinery/service/airfield-style linked cases that use this field.

Tiny details:

- The linked unit is removed from the temporary list before `UndockUnit`.
- If the pointer search returns `-1` or an out-of-range index, the temp list is not compacted.
- Only one matching entry is removed by this code shape.
- The linked unit therefore does not receive this later far `0x17` send or close/helipad damage from the loop. Its effects come from `UndockUnit` and receiver paths outside this loop.

### 3.3 Per-contact gate

For each remaining listed contact:

1. `0x0044251F..0x00442543`: loads target pointer, calls target vtable `+0x48` for target coords, then building vtable `+0x48` for building coords.
2. `0x00442546..0x00442564`: computes `building_coord - target_coord`.
3. `0x00442564`: calls `CoordStruct::Set` into a stack coord.
4. `0x00442581`: calls `CoordStruct::Distance3D`, which uses `sqrt(dx*dx + dy*dy + dz*dz)` then `Math::ftol`.
5. `0x00442586..0x0044258B`: signed-compare distance with `0x100`; `JL` enters the damage branch.
6. `0x0044258D..0x0044259B`: only when distance is not close, reads `building.Type+0x16CB`; nonzero enters the damage branch.
7. `0x0044259D..0x004425AA`: otherwise sends directed radio `0x17` and clears `target+0x500`.

Equivalent predicate:

```text
if distance < 0x100:
    close_damage_branch(target)
else if dying_building.Type.Helipad != 0:
    close_damage_branch(target)
else:
    send_radio_0x17_to_target(target)
    target+0x500 = 0
```

Tiny details:

- The distance boundary is strict: exactly `0x100` leptons is not close and proceeds to the helipad test.
- The helipad byte is read only after the contact fails the close-distance test.
- Helipad status belongs to the dying building, not the contacted target.
- A helipad building routes every listed remaining contact to damage, even if the contact is far away.
- A non-helipad building routes far contacts to sent radio `0x17`.
- The far branch clears `target+0x500` after the radio call returns.
- The close/helipad branch does not perform the `target+0x500 = 0` write in this caller.

### 3.4 Far/non-helipad branch

Assembly:

- `0x0044259D`: loads dying building vtable.
- `0x0044259F`: pushes target pointer.
- `0x004425A0`: pushes literal `0x17`.
- `0x004425A2`: sets `ECX = dying building`.
- `0x004425A4`: calls vtable `+0x278`.
- `0x004425AA`: clears `target+0x500`.
- `0x004425B4`: jumps to loop increment.

Active in YR: Yes for a dying contacted non-helipad building when the remaining contact's 3D coord distance is `>= 0x100`.

Receiver implications are inherited from verified receiver docs: Foot/Unit/Aircraft `Receive_Radio(0x17)` may mutate path, mission, destination, unload/deploy latch, and aircraft airfield target. This sender must therefore not be represented as contact deletion only.

### 3.5 Close/helipad branch

Assembly:

- `0x004425B6..0x004425C0`: target `GetType` via vtable `+0x84`, then reads `type+0xA0`.
- `0x004425CC..0x004425D6`: computes `damage = type+0xA0 * 10` by `EAX + EAX*4`, then shift-left one.
- `0x004425DA`: reads `Rules+0xFA8`.
- `0x004425E0..0x004425EB`: pushes receive-damage arguments.
- `0x004425EE`: calls target vtable `+0x16C`.

The pushed argument values, in callee order, are:

```text
&damage, 0, Rules+0xFA8, 1, 0
```

`Rules+0xFA8` resolves to `[CombatDamage] C4Warhead=Super` in stock YR (`rulesmd.ini:818`). Prior rules reports and `RULESCLASS_FIELDS.csv` also identify `+0xFA8` as `C4Warhead`.

Active in YR: Yes for close contacts of any dying building, and all listed remaining contacts of dying `Helipad=yes` buildings.

Tiny details:

- Damage amount is based on the contacted target's own type strength, not the dying building's strength.
- The multiplier is exactly `10`.
- The damage virtual is called on the contact, not on the dying building.
- This branch does not send radio `0x17`.
- This branch does not clear `target+0x500` in the caller.
- Because it goes through the full target `ReceiveDamage` virtual, target class-specific death/immunity/animation behavior still applies.

### 3.6 Ordering relative to later destruction cleanup

After the per-contact loop ends:

1. `0x00442608..0x0044261E`: tears down the temporary vector/list.
2. Later in death case: if `Type+0x157B` is set, calls `BuildingClass::SellBuilding`.
3. If `LightSource` is non-null, calls `FUN_00554A80(0)`.
4. Calls building vtable `+0x4EC`, the destruction-effects routine.
5. Optional later uninit/place-occupy-map branch runs after destruction effects.

Therefore the contact classification loop runs after linked-undock/capture-manager/chrono-deploy cleanup, but before garrison SellBuilding ejection, light source teardown, and destruction effects.

## 4. INI Keys

| INI key / section | Stock YR value | Binary field / effect | Active in standard YR? |
|---|---|---|---|
| `[CombatDamage] C4Warhead` | `Super` | `Rules+0xFA8`, used by close/helipad damage call | Yes (`rulesmd.ini:818`) |
| `[GAAIRC] Helipad` | `yes` | `BuildingType+0x16CB`, forces damage branch for all listed contacts | Yes (`rulesmd.ini:11820`) |
| `[GAAIRC] NumberOfDocks` | `4` | sizes airfield contact/pad behavior elsewhere; not read in this branch | Yes (`rulesmd.ini:11838`) |
| `[AMRADR] Helipad` | `yes` | same field; American airfield variant | Conditional stock content, yes in INI (`rulesmd.ini:12342`) |
| `[AMRADR] NumberOfDocks` | `4` | same dock-capacity context, not read in this branch | Conditional stock content, yes in INI (`rulesmd.ini:12358`) |

## 5. Integration Points

- `BuildingClass::ReceiveDamage @ 0x00442230`: owner of this entire slice.
- `RadioClass` contact helper `0x0065AD30`: returns `Contacts[index]` from `+0xE4`.
- `BuildingClass::UndockUnit`: called before the remaining-contact loop when `building+0x2E4 != 0`.
- `CoordStruct::Distance3D`: produces the integer compared against `0x100`.
- Dying building radio vtable `+0x278`: directed sent radio branch.
- Contact target damage vtable `+0x16C`: close/helipad branch.
- Later death chain: `SellBuilding`, light teardown, and vtable `+0x4EC` destruction effects all occur after this loop.

## 6. Current Rust Implementation Status

Rust scan only, no edits:

- `src/sim/game_entity.rs:187` stores `radio_contacts: Vec<u64>`.
- `src/sim/entity_store.rs:64` and `src/sim/world/mod.rs:700` clear contact links generically with `clear_radio_contacts_for`.
- `src/sim/combat/mod.rs:832..1007` snapshots dead entities, handles building survivor/garrison data, clears targets, and for non-animated corpses calls `entities.clear_radio_contacts_for(dead_id)` before removal.
- `src/sim/world/mod.rs:675..701` despawn also clears radio contacts and unregisters the live object.
- `src/sim/aircraft/mod.rs:553..564` handles a destroyed airfield by releasing the dock and setting the aircraft to Idle; it does not model the native helipad death damage call.
- `src/rules/object_type.rs:691..695` exposes `helipad` and `number_of_docks`; `src/rules/object_type.rs:1136..1137` parses `Helipad` and `NumberOfDocks`.

Current Rust can remove links, abort refinery/dock state in some specialized systems, and release airfield dock state. It does not have a generic building-death contact loop that snapshots RadioClass contacts, removes only `+0x2E4`, gates strict-distance/helipad, sends receiver `0x17` only to far non-helipad contacts, and applies C4Warhead strength*10 damage to close/helipad contacts.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Contact list source | verified | `0x004422DB..0x00442356`, `0x0065AD30..0x0065AD3D` | none |
| Dynamic temp-list growth | verified | `0x004422EB`, `0x0044231F..0x00442334` | exact allocator failure side effect not runtime-tested |
| Linked `+0x2E4` removal | verified | `0x004424A2..0x004424EA` | none |
| Distance vector and 3D distance call | verified | `0x0044251F..0x00442586`; decompile `CoordStruct::Distance3D` | none |
| Strict close threshold | verified | `CMP EAX,0x100; JL 0x004425B6` at `0x00442586..0x0044258B` | none |
| Helipad gate | verified | `0x0044258D..0x0044259B`; `rulesmd.ini:11820`, `12342` | none |
| Far sent radio `0x17` | verified | `0x0044259D..0x004425A4` | receiver body details are sibling-doc scope |
| Far `target+0x500` clear | verified | `0x004425AA` | semantic field name remains medium confidence |
| Close/helipad damage amount | verified | `0x004425B6..0x004425D6` | none for formula |
| Close/helipad damage warhead and args | verified | `0x004425DA..0x004425EE`; `rulesmd.ini:818` | exact meaning of the pushed `1,0` flags remains inherited from damage pipeline docs |
| Rust building death contact parity | touched-not-exhausted | Rust scan paths in section 6 | future implementation and tests |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is this path active in standard YR? -> Yes, `BuildingClass::ReceiveDamage` death case is ordinary building destruction; helipad exception is active for stock `GAAIRC` and INI-present `AMRADR`.` (evidence: `0x00442230`; `rulesmd.ini:11820`, `12342`)
- `[RESOLVED] OQ-02 - What objects enter the death contact loop? -> Non-null RadioClass contact slots copied before death cleanup, minus the exact linked `building+0x2E4` pointer if present.` (evidence: `0x004422DB..0x00442356`; `0x004424A2..0x004424EA`)
- `[RESOLVED] OQ-03 - Does the loop include empty contact slots? -> No, null return from `0x0065AD30` skips append.` (evidence: `0x00442303..0x00442305`)
- `[RESOLVED] OQ-04 - What is the distance cutoff? -> Strict signed `distance < 0x100`; exactly `0x100` is not close.` (evidence: `0x00442586..0x0044258B`)
- `[RESOLVED] OQ-05 - Is the helipad byte read on the target or dying building? -> Dying building type byte `building.Type+0x16CB`.` (evidence: `0x0044258D..0x00442593`)
- `[RESOLVED] OQ-06 - Which contacts receive sent `0x17`? -> Remaining listed contacts of a non-helipad dying building whose 3D distance is `>= 0x100`.` (evidence: `0x00442586..0x004425A4`)
- `[RESOLVED] OQ-07 - Which contacts do not receive sent `0x17`? -> The removed `+0x2E4` linked occupant, close contacts, and every listed contact when the dying building is a helipad.` (evidence: `0x004424A2..0x004425EE`)
- `[RESOLVED] OQ-08 - What happens in the close/helipad branch? -> Calls target damage virtual with damage `target.Type+0xA0 * 10`, distance `0`, warhead `Rules+0xFA8`, plus flags `1,0`.` (evidence: `0x004425B6..0x004425EE`)
- `[RESOLVED] OQ-09 - Does close/helipad branch clear `target+0x500` here? -> No; the only caller-side `+0x500` clear is after the far `0x17` send.` (evidence: `0x004425AA`; no corresponding write in `0x004425B6..0x004425EE`)
- `[RESOLVED] OQ-10 - Does far branch damage the contact? -> No direct damage call in this branch; it sends radio `0x17` and clears `+0x500`.` (evidence: `0x0044259D..0x004425B4`)
- `[RESOLVED] OQ-11 - Does linked `+0x2E4` occupant get this branch after UndockUnit? -> No when found in the temp list; it is removed before `UndockUnit` and before the loop.` (evidence: `0x004424A2..0x004424EA`)
- `[RESOLVED] OQ-12 - What Rust surfaces are affected? -> Generic contact clearing/despawn and specialized aircraft/refinery dock cleanup lack this exact loop.` (evidence: `src/sim/combat/mod.rs:832`, `src/sim/entity_store.rs:64`, `src/sim/aircraft/mod.rs:553`)
- `[DEFERRED] OQ-13 - Exact semantic name and downstream readers of target `+0x500`.` (category: `requires-different-system-context`; reason: this slice proves the write but not a complete field census; next-step-if-pursued: TechnoClass field `+0x500` writer/reader sweep)
- `[DEFERRED] OQ-14 - Exact class-specific result if the close/helipad target survives C4Warhead strength*10 damage due to immunity or force flags.` (category: `requires-different-system-context`; reason: this belongs to the full target `ReceiveDamage` pipeline; next-step-if-pursued: receiver-specific close-damage runtime trace)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Building death snapshots non-null RadioClass contacts, removes only the linked `+0x2E4` occupant, then classifies remaining contacts. | `0x004422DB..0x00442356`; `0x004424A2..0x004424EA` | Missing generic snapshot/classify loop; Rust clears links directly | `src/sim/combat/mod.rs`, `src/sim/entity_store.rs`, future radio-contact helper | Preserve contact-slot order, skip nulls, and remove only the linked dock occupant before classifying contacts. | Destroy a contacted non-helipad building with one linked dock unit and one far waiting contact; linked unit is not processed by the loop, far contact is. Proposed test: `building_death_contact_loop_removes_only_linked_dock_occupant` | Do not call `clear_radio_contacts_for` before running receiver effects; it destroys the evidence needed to classify contacts. |
| Far non-helipad contacts receive directed radio `0x17`, then `target+0x500` is cleared. | `0x00442586..0x004425AA` | Missing generic sent-`0x17` receiver side effects; only link cleanup exists | `src/sim/combat/mod.rs`, `src/sim/game_entity.rs`, future radio receiver model | For contacts with distance `>= 0x100` and dying building `helipad == false`, run sent `0x17` receiver behavior and then clear the Rust equivalent of `Techno+0x500` if/when modeled. | Destroy a service depot/refinery with a contacted mover at exactly one cell or farther; mover receives `0x17` cleanup rather than damage. Proposed test: `building_death_far_non_helipad_contact_gets_radio_0x17` | Do not treat returned `0x17` from BuildingClass radio `0x08` as this sent message. |
| Close contacts use damage branch for `distance < 0x100`; exactly `0x100` is far unless the building is a helipad. | `CMP EAX,0x100; JL` at `0x00442586..0x0044258B` | Unimplemented | combat contact cleanup / fixed-point distance helper | Use the same strict boundary after 3D distance integer conversion; do not use `<= 0x100`. | Place a contact at exactly `256` leptons from a dying non-helipad building; it receives radio `0x17`, not close damage. Proposed test: `building_death_contact_distance_256_is_far_branch` | Off-by-one changes whether a unit is killed or only evicted. |
| Dying helipad buildings route all remaining listed contacts to the damage branch regardless of distance. | `0x0044258D..0x0044259B`; `rulesmd.ini:11820`, `12342` | Rust airfield loss releases dock/sets Idle, but does not apply this native contact damage branch | `src/sim/aircraft/mod.rs`, `src/sim/docking/aircraft_dock`, `src/sim/combat/mod.rs` | When a `Helipad=yes` building dies, remaining listed contacts should receive target damage with `Rules.C4Warhead` and target strength*10, not sent radio `0x17`. | Destroy an Allied Airforce Command with a contacted aircraft far from the building; aircraft takes close/helipad death damage instead of only releasing dock. Proposed test: `helipad_death_far_contact_uses_c4_damage_not_radio_0x17` | Do not generalize the non-helipad far radio path to airfields. |
| Close/helipad damage amount is `target.Type+0xA0 * 10` with `Rules+0xFA8` (`C4Warhead=Super`). | `0x004425B6..0x004425EE`; `rulesmd.ini:818`; `RULESCLASS_FIELDS.csv` | C4Warhead exists in rules support for some systems, but not wired to this building-death contact branch | `src/rules/ruleset.rs`, `src/sim/combat/mod.rs`, target ReceiveDamage model | Apply the damage through the target's normal damage pipeline with the resolved C4 warhead; preserve target-class side effects. | Close tank/aircraft contact near a dying building receives target-strength*10 C4Warhead damage and downstream death behavior. Proposed test: `building_death_close_contact_damage_is_target_strength_times_ten_c4warhead` | Do not simply set health to zero unless target ReceiveDamage flags prove exact equivalence for the target class. |

## 10. Negative Facts / Do Not Do

- Do not send radio `0x17` to every contact on building death. Close contacts and all helipad contacts take damage instead. Evidence: `0x00442586..0x004425EE`.
- Do not use `<= 0x100` for the close gate. The binary uses signed `JL` after `CMP EAX,0x100`, so the close branch is strictly `< 0x100`. Evidence: `0x00442586..0x0044258B`.
- Do not base close/helipad damage on the dying building's strength. The branch reads the contacted target's type through target vtable `+0x84` and then reads `type+0xA0`. Evidence: `0x004425B6..0x004425D6`.
- Do not clear `target+0x500` in the close/helipad branch unless a separate target damage path proves it. The caller clear is only in the far radio branch. Evidence: `0x004425AA` and absence in `0x004425B6..0x004425EE`.
- Do not process the linked `building+0x2E4` occupant again in the remaining-contact loop when it is found in the temp list. The binary removes it before `UndockUnit`. Evidence: `0x004424A2..0x004424EA`.
- Do not treat helipad destruction as ordinary far-contact radio cleanup. `Helipad=yes` is a force-damage override for this branch. Evidence: `0x0044258D..0x0044259B`, `rulesmd.ini:11820`.

## 11. Remaining Uncertainty

- `target+0x500` is verified as a write but not semantically named in this slot. A separate TechnoClass field census should identify readers/writers before implementing that exact field.
- The pushed close-damage flags `1,0` are verified as literal argument values here, but their full semantic labels belong to the shared ReceiveDamage pipeline docs.
- The exact result for unusual targets that survive `C4Warhead` strength*10 damage through immunity or modded flags was not runtime-tested.

## Sources

- Ghidra read-only decompile: `BuildingClass::ReceiveDamage @ 0x00442230`; `BuildingClass::UndockUnit`; `RadioClass::Transmit_Radio`; `UnitClass::Receive_Radio`; `FootClass::Receive_Radio`; `AircraftClass::Receive_Radio`; `CoordStruct::Set`; `CoordStruct::Distance3D`.
- Ghidra read-only assembly contexts: `0x004422DB..0x00442356`, `0x0065AD30..0x0065AD3D`, `0x004424A2..0x004424EA`, `0x0044251F..0x004425F4`.
- Docs referenced: `SENT_RADIO_0X17_CALLER_SWEEP_RESWARM_20260528.md`; `miner/REFINERY_DESTROYED_OR_SOLD_MID_UNLOAD_CONTACTS_DISPLAY_CREDITS_GHIDRA_REPORT.md`; `BUILDINGCLASS_ON_DESTROYED_GHIDRA_REPORT.md`; `RULESCLASS_GHIDRA_REPORT.md`; `AIRFIELD_RADIO_CACHEDDOCK_CONTACT_LIFETIME_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, especially `[CombatDamage] C4Warhead=Super`, `[GAAIRC] Helipad=yes`, `[AMRADR] Helipad=yes`.
- Rust scan: `src/sim/combat/mod.rs`, `src/sim/entity_store.rs`, `src/sim/world/mod.rs`, `src/sim/game_entity.rs`, `src/sim/aircraft/mod.rs`, `src/rules/object_type.rs`.

## Status

COMPLETE for the bounded `BuildingClass::ReceiveDamage` death contact close/helipad gate slice.
