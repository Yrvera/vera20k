# SetInOpenTransport vtable +0x3D0 - Re-Swarm Research Report

**Address(es):** `0x00710470` (`TechnoClass::SetInOpenTransport`), `0x0070F850` (vtable `+0x3D0` target)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** identify the `+0x3D0` virtual called by `SetInOpenTransport`, the class bindings relevant to OpenTopped passengers, and the state ordering before LogicClass registration.  
**Non-Scope:** full OpenTopped weapon-selection math, full `Mission_Enter` state machines, cargo insertion ordering, and all non-OpenTopped uses of the same helper.  
**Confidence:** High for slot identity and ordering; Medium for human-readable names of some internal fields.  
**Active in YR:** Yes. Standard `rulesmd.ini` has `[BFRT] OpenTopped=yes`, and live Infantry/Unit `PerCellProcess` call sites gate `TechnoClass::SetInOpenTransport` on the destination transport type's OpenTopped byte.

## 1. Overview

`TechnoClass::SetInOpenTransport` is not just a flag setter. On a non-null passenger it sets `Techno+0x82`, invokes the shared `+0x3D0` virtual at `0x0070F850`, then appends the passenger to `LogicClass`'s active vector through `FUN_0055BAA0(this, 0)`.

The `+0x3D0` target is shared by the relevant Techno-derived vtables and performs the "hide but keep alive/ticking" preparation: clear destination, clear archive/attack target, clear `Techno+0x218`, and assign mission id `5`. It does not call `ObjectClass::Conceal`, does not set `Object+0x81` InLimbo, and does not remove the passenger from LogicClass; the following `FUN_0055BAA0` call explicitly registers it as active.

## 2. Class Layout / Key Offsets

| Offset | Width | Meaning in this slice | Evidence | Active in YR |
|---|---:|---|---|---|
| `Techno+0x82` | byte | `InOpenToppedTransport` / contained-open-transport flag | `0x00710470` writes `1`; `0x007104A0` and `0x007104C0` clear it | Yes for BFRT/OpenTopped boarding |
| `Techno+0x218` | dword ptr/int | multi-purpose ghost/rally/warp/archive pointer; this helper clears it | `0x0070F850` writes zero; sibling docs identify this as multi-purpose | Yes, cleared on OpenTopped entry |
| vtable `+0x3D0` | function ptr | open-transport hide/prep virtual; shared target `0x0070F850` | vtable memory reads listed below | Yes |
| vtable `+0x480` | function ptr | destination setter; called as `(0, 1)` by `0x0070F850` | `0x0070F850` bytes/decompile; Unit slot `0x00741970`, Infantry slot `0x0051AA40` | Yes |
| vtable `+0x3C8` | function ptr | target/archive-target clearer; called as `(0)` by `0x0070F850` | Infantry slot `0x0051B1F0`, Unit slot `0x006FCDB0` | Yes |
| vtable `+0x1F0` | function ptr | mission assignment virtual; called with mission id `5` | slot target `0x005B2FD0` writes mission fields | Yes |

## 3. Core Logic

### `TechnoClass::SetInOpenTransport @ 0x00710470`

Verified pseudocode:

```text
if this != null:
    byte[this + 0x82] = 1
    call this->vtable[0x3D0]()
    LogicClass_register(0x87F778, this, unique_scan_flag = 0)
return
```

Ordering evidence:

| Address / bytes | Finding |
|---|---|
| `0x00710470` decompile | non-null guard, flag write, virtual call, `FUN_0055BAA0(param_1, 0)` |
| `0x00710470` bytes `56 8B 74 24 08 85 F6 74 1E ... C6 86 82 00 00 00 01 FF 90 D0 03 00 00 ... B9 78 F7 87 00 E8 09 B6 E4 FF` | confirms `+0x82` write precedes `CALL [vtable+0x3D0]`, and `ECX=0x87F778` precedes `FUN_0055BAA0` |
| xrefs to `0x00710470` | `InfantryClass__PerCellProcess @ 0x0051A45E`, `UnitClass__PerCellProcess @ 0x0073A75D` |

### `+0x3D0` target `0x0070F850`

Verified pseudocode:

```text
this->vtable[0x480](0, 1)
this->vtable[0x3C8](0)
this->field_0x218 = 0
this->vtable[0x1F0](5)
return
```

Ordering evidence:

| Address / bytes | Finding |
|---|---|
| `0x0070F850` decompile | four operations above, no branches |
| `0x0070F850` bytes `56 8B F1 6A 01 6A 00 8B 06 FF 90 80 04 00 00 8B 16 6A 00 8B CE FF 92 C8 03 00 00 8B 06 6A 05 8B CE C7 86 18 02 00 00 00 00 00 00 FF 90 F0 01 00 00 5E C3` | exact call/write order: `+0x480(0,1)`, `+0x3C8(0)`, push `5`, clear `+0x218`, then call `+0x1F0` |
| `0x005B2FD0` decompile | mission assignment writes current mission at `+0xAC`, clears/reinitializes `+0xB4..+0xD0`, using `g_CurrentFrameCounter` |

### Vtable bindings

Relevant Techno-derived vtables bind `+0x3D0` to the same concrete function:

| Class / vtable | Slot address | Slot value | Evidence |
|---|---:|---:|---|
| `AircraftClass` vtable `0x007E22A4` | `0x007E2674` | `0x0070F850` | `read_memory 0x007E2674 = 50 F8 70 00`; xref data to `0x0070F850` |
| `BuildingClass` vtable `0x007E3EBC` | `0x007E428C` | `0x0070F850` | `read_memory 0x007E428C = 50 F8 70 00`; xref data |
| `FootClass` vtable `0x007E8C94` | `0x007E9064` | `0x0070F850` | `read_memory 0x007E9064 = 50 F8 70 00`; xref data |
| `InfantryClass` vtable `0x007EB058` | `0x007EB428` | `0x0070F850` | `read_memory 0x007EB428 = 50 F8 70 00`; xref data |
| `UnitClass` vtable `0x007F5C70` | `0x007F6040` | `0x0070F850` | `read_memory 0x007F6040 = 50 F8 70 00`; xref data |
| additional Techno-derived vtable | `0x007F4D30` | `0x0070F850` | xref data; exact class name not needed for BFRT slice |

For BFRT/OpenTopped passengers, the active relevant bindings are `InfantryClass` and `UnitClass`. Both share the same `+0x3D0` target. Their subordinate slots differ where expected: Infantry `+0x480 -> 0x0051AA40`, Unit `+0x480 -> 0x00741970`; Infantry `+0x3C8 -> 0x0051B1F0`, Unit `+0x3C8 -> 0x006FCDB0`. The shared wrapper means OpenTopped entry uses class-specific destination/target clearing through virtual dispatch, then common mission reset and active registration.

## 4. INI Keys

| Key | Scope | Default / stock value | Effect in this slice | Evidence | Active in YR |
|---|---|---|---|---|---|
| `OpenTopped=` | vehicle `ObjectType/TechnoType` data | default false; `[BFRT] OpenTopped=yes` | gates `SetInOpenTransport` call after a unit enters a transport | `rulesmd.ini:6932`; binary call sites read transport type `+0x5E4` | Yes for Battle Fortress |
| `Passengers=` | vehicle/building type | `[BFRT] Passengers=5` | permits cargo, making the OpenTopped path reachable | `rulesmd.ini:6931`; current Rust parser has `passengers` | Yes |
| `SizeLimit=` | vehicle type | `[BFRT] SizeLimit=2` | admission only; not a `+0x3D0` side effect | `rulesmd.ini:6933` | Yes |
| `OpenTransportWeapon=` | passenger TechnoType | default `-1`; stock infantry entries set `0` or `1` | later weapon selection while `+0x82` is set | `rulesmd.ini` stock entries; prior IFV/OpenTopped report | Conditional by passenger type |
| `OpenToppedRangeBonus=` / `OpenToppedDamageMultiplier=` / `OpenToppedWarpDistance=` | `[CombatDamage]` / general rules region | `2`, `1.2`, `7` in `rulesmd.ini` | not read by `SetInOpenTransport`; affects later combat/chrono behavior | `rulesmd.ini:867..869` | Conditional; outside this slice |

## 5. Integration Points

### Entry points

`SetInOpenTransport` has exactly two function xrefs in this Ghidra session:

- `InfantryClass__PerCellProcess @ 0x0051A45E`
- `UnitClass__PerCellProcess @ 0x0073A75D`

Both are on live YR boarding/contact paths. The stock BFRT path reaches these when a passenger is accepted into a transport whose type byte at `+0x5E4` is nonzero (`OpenTopped=yes`).

### Clear paths

`TechnoClass::ClearInOpenTransport @ 0x007104A0` only clears `byte[this+0x82] = 0` under a non-null guard. Its xref is `UnitClass__Mission_Deploy_Building @ 0x0073DB98`.

`CargoClass::ClearAllInOpenTransport @ 0x007104C0` gets the first passenger from `CargoClass+0x114`, then walks passenger `+0x30` links while the next object has flag bit `0x04` in `+0x14`; for each passenger it writes `+0x82 = 0`. Its xref is `UnitClass__ReceiveDamage @ 0x00737F92`.

### Tick-cycle implication

Because `SetInOpenTransport` calls `FUN_0055BAA0` after hiding/resetting the passenger, OpenTopped passengers remain in the global active vector even though their map body/destination state is cleared. Their later AI/combat logic must be allowed to tick from the active order. This is stock-live for BFRT passengers, not TS legacy.

## 6. Current Rust Implementation Status

Rust already parses much of the data needed:

- `src/rules/object_type.rs:568` parses/stores `open_topped`.
- `src/rules/object_type.rs:581` stores `open_transport_weapon`.
- `src/rules/weapon_type.rs:80` parses `OpenToppedAnim`.
- `src/rules/weapon_type.rs:138` parses `FireInTransport`.

Rust boarding currently hides the passenger by changing `PassengerRole::Inside`, clears radio contacts, and clears movement/attack/order state at `src/sim/passenger.rs:478` and `src/sim/passenger.rs:729`. It also stores an OpenTransport weapon override on the transport at `src/sim/passenger.rs:490` and `src/sim/passenger.rs:745`.

The key mismatch is in combat ticking: `src/sim/combat/mod.rs:1370` skips all entities whose `PassengerRole` is inside a transport, with the OpenTopped case explicitly deferred. That contradicts the verified active-vector registration in gamemd. Rust also appears to put the OpenTransport weapon override on the transport, whereas gamemd's stock OpenTopped architecture keeps passengers active and lets their own weapon-selection path observe `Techno+0x82`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TechnoClass::SetInOpenTransport @ 0x00710470` | verified | decompile; bytes at `0x00710470`; xrefs to Infantry/Unit per-cell paths | none |
| vtable `+0x3D0` target identity | verified | slot reads `0x007EB428`, `0x007F6040`, `0x007E9064`, `0x007E428C`, `0x007E2674`; xrefs to `0x0070F850` | none for BFRT slice |
| `0x0070F850` operation order | verified | decompile; bytes at `0x0070F850` | exact friendly name of the helper remains inferred |
| Infantry subclass subordinate slots | verified | `0x007EB4D8 -> 0x0051AA40`; `0x007EB420 -> 0x0051B1F0`; `0x007EB248 -> 0x005B2FD0` | full bodies outside scope |
| Unit subclass subordinate slots | verified | `0x007F60F0 -> 0x00741970`; `0x007F6038 -> 0x006FCDB0`; `0x007F5E60 -> 0x005B2FD0` | full bodies outside scope |
| `ClearInOpenTransport @ 0x007104A0` | verified | decompile; bytes; xref `0x0073DB98` | none |
| `CargoClass::ClearAllInOpenTransport @ 0x007104C0` | verified | decompile; bytes; xref `0x00737F92` | whether damage caller only fires on destruction is outside scope |
| Rust parser surfaces | verified | `object_type.rs`, `weapon_type.rs` line scans | no code edits in this slot |
| Rust combat tick behavior | verified-as-current-shape | `combat/mod.rs:1370` skip inside transport | exact Rust fix design deferred to implementation |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - What concrete function does `SetInOpenTransport` call through vtable `+0x3D0`? -> `0x0070F850`.` (evidence: `0x00710470` decompile; vtable reads `0x007EB428`, `0x007F6040`)`
- `[RESOLVED] OQ-02 - Is the target class-specific for BFRT passengers? -> No for the wrapper; Infantry and Unit vtables both bind `+0x3D0` to `0x0070F850`, though subordinate virtuals are class-specific.` (evidence: vtable memory reads above)`
- `[RESOLVED] OQ-03 - Does the helper call `ObjectClass::Conceal` or set `Object+0x81` InLimbo? -> No; `0x0070F850` only calls `+0x480`, `+0x3C8`, clears `+0x218`, and calls `+0x1F0(5)`.` (evidence: `0x0070F850` decompile/bytes)`
- `[RESOLVED] OQ-04 - Does active registration occur before or after hide/prep? -> After; `FUN_0055BAA0(this,0)` follows the `+0x3D0` call.` (evidence: `0x00710470` bytes)`
- `[RESOLVED] OQ-05 - Is the path active in standard YR? -> Yes for BFRT: `[BFRT] OpenTopped=yes`, and Infantry/Unit per-cell call sites gate on transport type `+0x5E4`.` (evidence: `rulesmd.ini:6932`; `0x0051A45E`; `0x0073A75D`)`
- `[RESOLVED] OQ-06 - What clears `+0x82`? -> single-object clear at `0x007104A0`, cargo-list clear at `0x007104C0`.` (evidence: decompiles and xrefs)`
- `[RESOLVED] OQ-07 - Does this imply hidden passengers should still tick? -> Yes; the path explicitly registers the passenger in LogicClass after hiding/prep.` (evidence: `0x00710470`; `FUN_0055BAA0` prior report)`
- `[RESOLVED] OQ-08 - Does Rust currently allow inside-transport passengers to combat tick? -> No; combat skips `PassengerRole::Inside` at `combat/mod.rs:1370`.` (evidence: Rust scan)`
- `[DEFERRED] OQ-09 - What is the best canonical name for `0x0070F850`?` (category: `out-of-scope`; reason: behavior is fully identified, but naming needs broader vtable taxonomy; next-step-if-pursued: audit nearby TechnoClass vtable docs and assign a non-misleading local name.)`
- `[DEFERRED] OQ-10 - Is `+0x218` best named ghost cell, warp target, archive target, or rally target in this exact path?` (category: `requires-different-system-context`; reason: existing docs prove it is multi-purpose; this slice only proves OpenTopped entry clears it; next-step-if-pursued: synthesize all `+0x218` uses.)`
- `[DEFERRED] OQ-11 - Full damage/destruction semantics of `CargoClass::ClearAllInOpenTransport`.` (category: `requires-different-system-context`; reason: this slot only needed the clear helper and xref; next-step-if-pursued: trace `UnitClass::ReceiveDamage @ 0x00737F92` around transport death.)`

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| OpenTopped boarding sets passenger contained flag, clears destination/target/ghost state through `+0x3D0`, then registers the passenger in LogicClass active order. | `0x00710470` decompile/bytes; `0x0070F850` decompile/bytes; vtable reads | partial/mismatch: Rust hides passenger and clears some state, but active combat tick is skipped | `src/sim/passenger.rs`; `src/sim/world/mod.rs`; `src/sim/combat/mod.rs` | Represent OpenTopped-contained passengers as hidden/contained but still live-active; clear movement target, attack/archive target, ghost/rally pointer equivalent, and mission/order to guard-id-equivalent before active tick continues | BFRT with one GI passenger boards; next combat tick still considers the passenger from active order while body remains absent from cell occupancy | Do not model OpenTopped as transport-owned weapon fire only; gamemd keeps passengers active |
| The `+0x3D0` wrapper does not call `ObjectClass::Conceal` and does not set `Object+0x81` InLimbo. | `0x0070F850` body lacks Conceal call and `+0x81` write; compare `ObjectClass::Conceal @ 0x005F4D30` | unchecked/likely mismatch if Rust equates all `PassengerRole::Inside` with limbo/despawn semantics | `src/sim/passenger.rs`; occupancy/render filtering surfaces | Keep separate state for contained-open-transport vs true limbo/despawn; hidden body should not imply removal from active-order or death cleanup | Save/load or tick snapshot with BFRT passenger preserves entity and active-order membership while occupancy excludes passenger body | Do not unregister live object when entering an OpenTopped transport |
| Clear helpers only reset `+0x82`; they do not undo destination/mission state by themselves. | `0x007104A0` and `0x007104C0` decompile/bytes | unchecked: Rust unload restores `PassengerRole::None` and occupancy, but exact clear ordering vs mission/destination is not proven | `src/sim/passenger.rs:1002` and unload/death paths | Unload/death clear of OpenTopped contained flag should be explicit and not assumed to restore all order fields unless separately verified | BFRT unload clears contained flag and occupancy is restored; active-order position is preserved or changed only by a verified native path | Do not use `ClearInOpenTransport` as a full unhide/unlimbo operation |
| Weapon selection for OpenTopped should be passenger-owned while `+0x82` is set, using passenger `OpenTransportWeapon`; the `SetInOpenTransport` path prepares the passenger for that by keeping it active. | `0x00710470`; prior IFV/OpenTopped report; Rust scan shows transport override | mismatch risk: Rust stores `WeaponOverride::OpenTransport` on the transport, not necessarily on/passenger-through the passenger | `src/sim/combat/combat_weapon.rs`; `src/sim/combat/mod.rs` | Combat should let eligible contained passengers select/fire their own OpenTransport weapon from active order, subject to later verified range/ROF rules | BFRT with two different passenger weapon slots fires per-passenger ROF/weapon selection, not a single transport override | Do not collapse multiple passengers into one transport weapon override |

Proposed test names:

- `open_topped_passenger_remains_live_registered_after_boarding`
- `open_topped_inside_passenger_can_fire_from_active_order`
- `open_topped_boarding_clears_destination_target_and_ghost_state`
- `clear_in_open_transport_does_not_unhide_or_unregister_passenger`

### Negative Facts / Do Not Do

- Do not treat `SetInOpenTransport` as a normal `Reveal`/`Conceal` pair. Evidence: `0x0070F850` does not call `ObjectClass::Conceal`; `0x00710470` immediately calls `FUN_0055BAA0`.
- Do not unregister OpenTopped passengers from active order on boarding. Evidence: `FUN_0055BAA0(this, 0)` is called after `+0x3D0`.
- Do not store OpenTopped fire only as a transport-level weapon override. Evidence: gamemd sets state on the passenger and keeps that passenger active.
- Do not assume `ClearInOpenTransport` restores map occupancy. Evidence: `0x007104A0` only writes `+0x82 = 0`; `0x007104C0` only loops and writes the same byte.
- Do not infer that `+0x82` means general limbo or airstrike-only state. Evidence: the sole open-transport writer is `0x00710470`, active from Infantry/Unit per-cell boarding paths.

### Stale Docs / Follow-up Docs

- `docs/research/IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md`: replace "`unit.vtable[0x3D0](unit) // some virtual notification`" with "`unit.vtable[0x3D0]()` resolves to shared `0x0070F850`; it calls `+0x480(0,1)`, `+0x3C8(0)`, clears `Techno+0x218`, and calls `+0x1F0(5)` before `SetInOpenTransport` registers the passenger in LogicClass."
- `docs/research/BULLETCLASS_LIFECYCLE_AND_TIER1_VERIFICATIONS_GHIDRA_REPORT.md`: replace "`vtable+0x3D0 (Hide / RemoveFromMap-ish)`" with "`vtable+0x3D0 -> 0x0070F850, a hide/prep helper that clears destination/target/ghost state and assigns mission id 5; it is not `ObjectClass::Conceal` and does not itself remove LogicClass membership."

## Sources

- Ghidra decompile/read-only:
  - `0x00710470` `TechnoClass::SetInOpenTransport`
  - `0x007104A0` `TechnoClass::ClearInOpenTransport`
  - `0x007104C0` `CargoClass::ClearAllInOpenTransport`
  - `0x0070F850` vtable `+0x3D0` target
  - `0x005B2FD0` `MissionClass::Assign_Mission`
  - `0x0051B1F0`, `0x006FCDB0`, `0x0051AA40`, `0x00741970` subordinate vtable targets
  - `0x0051A430` `InfantryClass::PerCellProcess`
  - `0x0073A720` `UnitClass::PerCellProcess`
- Ghidra memory/vtable reads:
  - `0x007EB428`, `0x007F6040`, `0x007E9064`, `0x007E428C`, `0x007E2674`, `0x007F4D30`
  - `0x007EB4D8`, `0x007F60F0`, `0x007EB420`, `0x007F6038`, `0x007EB248`, `0x007F5E60`
- Research docs referenced:
  - `docs/research/IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md`
  - `docs/research/BULLETCLASS_LIFECYCLE_AND_TIER1_VERIFICATIONS_GHIDRA_REPORT.md`
  - `docs/research/LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`
- INI checked:
  - `ini/rulesmd.ini`
- Rust scanned:
  - `src/sim/passenger.rs`
  - `src/sim/combat/mod.rs`
  - `src/sim/world/mod.rs`
  - `src/rules/object_type.rs`
  - `src/rules/weapon_type.rs`
