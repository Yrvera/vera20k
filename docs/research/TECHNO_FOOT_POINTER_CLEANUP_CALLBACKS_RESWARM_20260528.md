# Techno / Foot Pointer-Expired Cleanup Callbacks - Re-Swarm Research Report

**Address(es):** `Detach_From_All_Lists @ 0x007258D0`, `ObjectClass::UnInit @ 0x005F65F0`, Object callback `0x005F5230`, Radio callback `0x0065AAC0`, `TechnoClass::PointerExpired @ 0x007077C0`, `FootClass::PointerExpired @ 0x004D9960`, Building callback `0x0044E8F0`, Unit callback `0x007446E0`, Infantry callback `0x0051AA10`, Aircraft callback `0x0041B660`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** vtable `+0x28` pointer-expired callbacks reached by `Detach_From_All_Lists` from the normal `ObjectClass::UnInit` pre-conceal path, with focus on radio/contact slots, cargo/list links, passenger/transport refs, CaptureManager victim links, temporal targets, dock refs, and cell fallback writes.
**Non-Scope:** full semantics of every unlabelled Techno field, full Building damage-fire/upgrade cleanup, full radio message protocol, full cargo ejection timing, pending-delete drain, and Rust implementation.
**Confidence:** High for dispatch order, callback bindings, offsets, and active-YR liveness of the normal cleanup path; Medium for semantic names of several unlabelled Techno/Foot fields.
**Active in YR:** Yes for normal object destruction/limbo cleanup. Conditional for manager-specific branches (CaptureManager, TemporalClass, SpawnManager, AirstrikeClass, cargo, transport, and dock fields only run when those pointers exist).

## 0. Investigation Gate

**Target question:** Which Object/Radio/Techno/Foot/derived vtable `+0x28` callbacks are invoked by `Detach_From_All_Lists` before conceal/alive-clear, and which cross-object pointers do they clear or convert when a Techno/Foot/Object-derived object expires?

**Non-goals:** Do not rediscover that `UnInit` calls `Detach_From_All_Lists` before Conceal/alive clear; do not re-prove `FootClass::UnInit` pre-base capture/chrono/passenger cleanup except where it affects this callback path; do not edit Rust, INI, claims, or other docs; do not mutate Ghidra.

**Evidence needed to mark COMPLETE:**

- Decompile plus assembly for `ObjectClass::UnInit` showing `DL=1`, `ECX=this`, call `0x007258D0`, then virtual `+0xD4`, then alive clear.
- Decompile plus assembly for `Detach_From_All_Lists` object-derived branch showing `Object+0x14` bit-1 gate, `DAT_00B0F724` roster loop, and listener vtable `+0x28(expired, was_removed)`.
- Vtable memory reads for Techno/Foot/Unit/Infantry/Aircraft/Building `+0x28` bindings.
- Decompile plus assembly/context for Object, Radio, Techno, Foot callbacks, and decompile/vtable evidence for derived Unit/Infantry/Aircraft/Building callbacks.
- Focused Rust scan of entity cleanup surfaces listed by the parent.

**Stop conditions:**

- Stop after the pre-conceal object-expiry dispatch and the named callback fields are classified.
- Do not follow every unlabelled manager helper into a full subsystem unless it directly owns CaptureManager, Temporal, cargo/list, passenger/transport, or dock cleanup.
- Any field whose semantic identity is not strong from this slice is recorded as Remaining Uncertainty instead of named.

## 1. Overview

`ObjectClass::UnInit` calls `Detach_From_All_Lists` with `was_removed=1` before the object's virtual `+0xD4` limbo/conceal call and before `Object+0x90` alive is cleared. In the Object-derived branch, `Detach_From_All_Lists` walks the broad remove-listener roster at `DAT_00B0F724` and calls each listener object's vtable `+0x28(expired_object, 1)`.

The callback chain for live Techno-derived listeners is layered:

```text
UnitClass +0x28     -> Unit wrapper        -> FootClass::PointerExpired -> TechnoClass::PointerExpired -> Radio -> Object
InfantryClass +0x28 -> Infantry wrapper    -> FootClass::PointerExpired -> TechnoClass::PointerExpired -> Radio -> Object
AircraftClass +0x28 -> Aircraft wrapper    -> FootClass::PointerExpired -> TechnoClass::PointerExpired -> Radio -> Object
BuildingClass +0x28 -> Building wrapper    -> TechnoClass::PointerExpired -> Radio -> Object
TechnoClass +0x28   -> TechnoClass::PointerExpired -> Radio -> Object
FootClass +0x28     -> FootClass::PointerExpired -> TechnoClass::PointerExpired -> Radio -> Object
```

## 2. Dispatch And Binding Evidence

| Item | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| `ObjectClass::UnInit` ordering | Defuses bomb, conditionally calls `FootClass::EMPPassengers(0)`, sets `DL=1`, calls `Detach_From_All_Lists`, then virtual `+0xD4`, then clears `Object+0x90`. | decompile `0x005F65F0`; assembly `0x005F6612..0x005F6625` | Yes |
| `Detach_From_All_Lists` Object branch | For non-null Object-derived expired objects, tests `Object+0x14` bit 1, loops `DAT_00B0F724[0..DAT_00B0F730)`, and calls listener `+0x28(expired, DL)`. | decompile `0x007258D0`; assembly `0x0072592D..0x0072595F` | Yes |
| Techno binding | `TechnoClass` vtable base `0x007F4960`, slot `+0x28` points to `0x007077C0`. | `read_memory(0x007F4988,4) -> C0 77 70 00`; decompile `0x007077C0` | Yes |
| Foot binding | `FootClass` vtable base `0x007E8C94`, slot `+0x28` points to `0x004D9960`. | `read_memory(0x007E8CBC,4) -> 60 99 4D 00`; decompile `0x004D9960` | Yes |
| Unit binding | `UnitClass` vtable base `0x007F5C70`, slot `+0x28` points to `0x007446E0`, a wrapper that calls Foot then clears `+0x6C8/+0x6C4`. | `read_memory(0x007F5C98,4) -> E0 46 74 00`; decompile `0x007446E0` | Yes |
| Infantry binding | `InfantryClass` vtable base `0x007EB058`, slot `+0x28` points to `0x0051AA10`, a wrapper that calls Foot then clears `+0x6C0`. | `read_memory(0x007EB080,4) -> 10 AA 51 00`; decompile `0x0051AA10` | Yes |
| Aircraft binding | `AircraftClass` vtable base `0x007E22A4`, slot `+0x28` points to `0x0041B660`, a wrapper that calls Foot then clears `+0x6CC/+0x6C4`. | `read_memory(0x007E22CC,4) -> 60 B6 41 00`; decompile `0x0041B660` | Yes |
| Building binding | `BuildingClass` vtable base `0x007E3EBC`, slot `+0x28` points to `0x0044E8F0`, a wrapper that calls Techno then clears building-owned refs/lists. | `read_memory(0x007E3EE4,4) -> F0 E8 44 00`; decompile `0x0044E8F0` | Yes |

## 3. Callback Field Effects

### 3.1 Object and Radio layers

| Owner | Offset | Verified effect when expired matches | Evidence | Active in YR |
|---|---:|---|---|---|
| Object | `+0x34` | If non-null and equal to expired, decrements expired `+0x2C`, then clears `this+0x34`. | `0x005F5230..0x005F5247` | Yes |
| Object | `+0x30` | Only when `was_removed != 0`: if equal to expired and expired non-null, writes `this+0x30 = expired+0x30`. This is a linked-list splice/advance, not a null-only clear. | `0x005F524F..0x005F5265` | Yes in `UnInit` path because `DL=1` |
| Object | `+0x88` | Clears to null if equal to expired. | `0x005F5268..0x005F5270` | Yes |
| Radio | `+0xE4/+0xE8` | Calls Object layer first, then loops contact vector; if a slot equals expired and `was_removed != 0`, writes slot to null. | decompile `0x0065AAC0`; assembly `0x0065AACB..0x0065AAF4` | Yes in `UnInit` path |

### 3.2 Techno layer focus fields

| Owner | Offset / cluster | Verified effect when expired matches | Evidence | Active in YR |
|---|---:|---|---|---|
| Techno CargoClass subobject | `+0x114` | If `was_removed != 0`, calls cargo remove helper `0x004734B0` on `this+0x114`, removing expired from a `+0x30` next-linked passenger/cargo chain and decrementing count. | `0x007077D8..0x007077E9`; decompile `0x004734B0` | Conditional: yes when cargo contains expired |
| Techno manager/pointer slots | `+0x2D4`, `+0x2E0`, `+0x2CC`, `+0x278`, `+0x294`, `+0x428` | Each is compared against expired and cleared to zero. `+0x294` is strongly identified in prior docs as `AirstrikeClass*`; others remain semantically unpinned in this slice. | assembly `0x007077EE..0x00707843`; `AIRCRAFTCLASS_0XA5...` for `+0x294` | Conditional |
| Techno dock/link slot | `+0x2E4` | Cleared only if expired matches and `g_MapEditorMode != 0`; normal `UnInit` with map editor mode off does not clear it in this callback. | assembly `0x00707849..0x00707859`; prior dock docs for +0x2E4 naming caveats | Conditional; No for standard non-editor play in this callback |
| Techno unknown reciprocal cluster | `+0x1CC/+0x1D0/+0x1D4` | If expired equals `+0x1CC`, optional `+0x1D4` is finalized through vtable `+0xF8`, reciprocal fields on the `+0x1CC` object are cleared, and `this+0x1CC` is cleared. If expired equals `this+0x1D0`, callback clears reciprocal data on that object and clears `this+0x1D0`. | decompile `0x0070785F..0x007078BC` | Conditional; semantic label uncertain |
| Techno transport ref | `+0x11C` | If expired equals this field, callback clears it only when the expired object is not alive, has no health, or map editor mode is active. This matches prior transport docs that use `+0x11C` as a passenger's transport pointer, but the exact gate is callback-specific. | decompile `0x007078BC..0x007078DD`; `IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md` | Conditional |
| Techno target/archive cluster | `+0x2B4/+0x2B8` | Expired target can trigger `this->vtable+0x3C8(0)` target/archive clear, possible firing/action stop through `+0x1FC/+0x1F8`, and then `+0x2B8` clear if sensor/visibility gate allows. | decompile `0x0070799D..0x00707A86`; assembly `0x007079C0..0x00707A80` | Conditional |
| Techno owner/contact flag | `+0x21C`, byte `+0x41A` | If owner pointer equals expired, clears `+0x21C` and byte `+0x41A`. | assembly `0x00707A86..0x00707A94` | Conditional on House/object expiry |
| Techno eight-slot array | `+0x304..+0x320` | Loops eight dword slots and clears any slot equal to expired. | decompile `0x00707AC2..0x00707AE3` | Conditional |
| Techno extra pointer | `+0x324` | Clears if equal to expired. | decompile after eight-slot loop | Conditional |
| Techno was-removed-only slots | `+0x500`, `+0x218` | If `was_removed != 0`, clears each if equal to expired. | decompile `0x00707AF1..0x00707B03` | Yes in `UnInit` path when fields match |
| Techno CaptureManager | `+0x2BC` | If manager exists, calls `0x00471F90(manager, expired)`, which scans manager node vector and removes nodes whose victim equals expired. | assembly `0x00707B09..0x00707B14`; decompile `0x00471F90`; `MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md` | Conditional; active for stock mind-control units |
| Techno SpawnManager | `+0x2D0` | If manager exists, calls `SpawnManagerClass::PointerExpired(expired)`. The manager clears owner/target/spawn slots or kills/retargets spawns depending on which slot matches. | `0x00707B19..0x00707B29`; decompile `0x006B7C60` | Conditional |
| Techno TemporalClass | `+0x274` | If manager exists, calls temporal pointer-expired helper `0x0071AB60(expired)`. If expired is the temporal target, it detaches target, clears temporal link/timer fields, and may call target virtual `+0x484(0,1)`. | `0x00707B2B..0x00707B3B`; decompile `0x0071AB60`, `0x0071ABC0` | Conditional; active for temporal weapons |
| Techno AirstrikeClass | `+0x294` | If manager exists and manager `+0x4C` points back to this Techno, calls `0x0041D540(manager, expired)` to clear manager target/attached aircraft refs. | decompile `0x00707B3D..0x00707B4F`; decompile `0x0041D540` | Conditional; stock `[BORIS]` creates this manager, stock aircraft usually do not |
| Techno reciprocal links | `+0x2A8`, `+0x2AC`, `+0x2B0` | `+0x2A8` clears reciprocal peer `+0x2A8`; `+0x2AC/+0x2B0` call chrono/deploy cleanup, clear reciprocal `+0x2B0/+0x2AC`, then clear self. | decompile `0x00707B50..0x00707B96` | Conditional; semantic names need follow-up |
| Techno vectors | `+0x444/+0x450/+0x45C/+0x468` and `+0x474/+0x480` | Removes expired from two pointer vectors by find-index then left-shift compaction; first vector pair removes from two related arrays/counts when a `+0x45C` entry matches. | decompile `0x00707BB0..0x00707C64`; assembly `0x00707C20..0x00707C3B` | Conditional |

### 3.3 Foot and concrete Foot-derived additions

| Owner | Offset | Verified effect when expired matches | Evidence | Active in YR |
|---|---:|---|---|---|
| Foot | `+0x5D4` | Clears if equal to expired. Prior docs call this a RadioLink/current team/transport-style link depending on context; this slice only proves pointer clear. | assembly `0x004D9973..0x004D9988`; `UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md` for RadioLink label | Conditional |
| Foot | `+0x694` | If equal to expired and expired appears invalid/dead or map editor mode is active, clears. If the manager/object at `+0x694` is still non-null and its count at `+0x6C` is positive, callback dispatches that object's `+0x28(expired,1)`. Existing unit docs identify victim `Foot+0x694` as a parasite attacker link for terror drones. | decompile `0x004D998C..0x004D99CD`; `units/soviet/DRON.md` | Conditional |
| Foot passenger/list link | `+0x5D8` | If equal to expired and `was_removed != 0`, writes `this+0x5D8 = expired+0x5D8`; this is a linked-list advance rather than a null-only clear. | decompile `0x004D99D3..0x004D99F0` | Yes in `UnInit` path when field matches |
| Foot ghost/cell slot | `+0x218` | If equal to expired, calls `TechnoClass::SetGhostCell(0)` rather than a plain null write. | decompile `0x004D99F3..0x004D9A03` | Conditional |
| Foot target/list slots | `+0x5A8`, `+0x5A4` | Clears each if equal to expired. | decompile `0x004D9A08..0x004D9A1A` | Conditional |
| Foot destination fallback | `+0x5CC -> +0x5C8` | If object pointer `+0x5CC` equals expired, writes `+0x5C8` to the `CellClass*` at expired object's coordinates via `GetCoords -> MapClass::Get_CellClass`, then clears `+0x5CC`. If `+0x5C8` itself equals expired, clears `+0x5C8`. | decompile `0x004D9B80..0x004D9BC5`; cell fallback arithmetic context `0x004D9C77..0x004D9C99` | Conditional; active when a Foot object's object target expires |
| Foot vectors | `+0x5B0/+0x5BC` and `+0x58C/+0x598` | Removes every matching expired pointer from two compacted arrays by left-shifting and decrementing counts; loop index is decremented after removal so duplicate matches are removed. | decompile `0x004D9BD0..0x004D9C5C` | Conditional |
| Unit wrapper | `+0x6C8`, `+0x6C4` | Unit wrapper clears both after Foot layer. Existing `UNIT_0X6C8_CONVOY_LINK_LIFECYCLE...` covers convoy semantics for `+0x6C8`. | decompile `0x007446E0` | Conditional |
| Infantry wrapper | `+0x6C0` | Infantry wrapper clears after Foot layer. Semantic identity not established in this slice. | decompile `0x0051AA10` | Conditional |
| Aircraft wrapper | `+0x6CC`, `+0x6C4` | Aircraft wrapper clears both after Foot layer. | decompile `0x0041B660` | Conditional |

### 3.4 Building additions

`BuildingClass +0x28` calls `TechnoClass::PointerExpired` first, then clears building-local refs:

- `+0x540`, `+0x548`, `Factory`, `+0x600`, and `LightSource` if they equal expired.
- Damage/fire anim slots `+0x57C` and `+0x584` can trigger `BuildingClass::CreateAnimForSlot` instead of just clearing, depending on health ratio and type anim strings.
- If expired is an Anim (`WhatAmI == 4`), it scans eight `+0x5C8` slots and clears the matching slot, or calls `0x00451B40` when the expired anim has byte `+0x118`.
- Clears Type and three upgrade refs if they match expired.
- Removes expired from building vectors around `+0x66C/+0x670/+0x67C`, and in map editor mode from a second vector around `+0x684/+0x688/+0x694`.

Evidence: decompile `0x0044E8F0`; vtable binding `0x007E3EE4 -> 0x0044E8F0`.
Active in YR: Yes for buildings; individual branches are conditional on the referenced pointers existing.

## 4. Current Rust Implementation Status

Current Rust has a generic reciprocal radio-contact cleanup, so the older broad statement "no generic radio BREAK cleanup on despawn" is stale for the current workspace:

- `src/sim/entity_store.rs:64..70` implements `EntityStore::clear_radio_contacts_for`, clearing peer contacts and the removed entity's own vector.
- `src/sim/world/mod.rs:675..698` calls `self.clear_radio_contacts_for(stable_id)` before removing the entity and unregistering live object membership.
- `src/sim/entity_store.rs:231..285` tests one-sided, reciprocal, missing-id, and order-preserving radio contact cleanup.

Remaining gaps are not the generic radio vector. The pointer-expired callback clears many non-radio stable-ID refs and component refs: cargo/passenger chains, transport refs, attack/archive targets, CaptureManager victim lists, temporal target links, spawn/airstrike manager refs, Foot destination object-to-cell fallback, and several compacted pointer vectors. Current focused scan found `GameEntity` has generic fields such as `attack_target`, `movement_target`, `navigation`, `passenger_role`, `miner`, `dock_state`, `mind_controlled`, `capture_target`, and `bunker_occupant` (`src/sim/game_entity.rs:169..230`, `445..504`), but no generic "pointer expired" pass that applies the full native per-class callback order.

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `ObjectClass::UnInit -> Detach -> Limbo -> alive clear` | verified | `0x005F65F0`; assembly `0x005F6612..0x005F6625` | none |
| Object-derived broad roster `DAT_00B0F724` | verified | `0x007258D0`; assembly `0x0072592D..0x0072595F` | exact roster ownership/name outside scope |
| Object callback `0x005F5230` | verified | decompile + assembly `0x005F5230..0x005F527A` | semantic names for `+0x30/+0x34/+0x88` |
| Radio callback `0x0065AAC0` | verified | decompile + assembly `0x0065AACB..0x0065AAF4` | none for contact clear |
| Techno callback `0x007077C0` | verified for focus branches | decompile + selected assembly contexts listed above | semantic names for unlabelled fields |
| Foot callback `0x004D9960` | verified for focus branches | decompile + assembly `0x004D9973..0x004D99C6`, `0x004D9B80..0x004D9C5C` | semantic names for several fields |
| Unit/Infantry/Aircraft/Building wrappers | verified | vtable reads and decompiles | full branch effects for Building damage-fire outside scope |
| CaptureManager victim removal from callback | verified | `0x00707B09..0x00707B14`; `0x00471F90`; MC doc | full owner/anim fate inside `FreeUnit` is prior-doc scope |
| Temporal target cleanup from callback | verified | `0x00707B2B..0x00707B3B`; `0x0071AB60`; `0x0071ABC0` | full temporal visual/damage state outside scope |
| Rust focused scan | verified-by-scan | files/lines in section 4 | implementation not performed |

## 6. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does `ObjectClass::UnInit` call callbacks before conceal/alive clear? -> Yes, `DL=1; ECX=this; CALL 0x007258D0` precedes virtual `+0xD4` and `Object+0x90=0`.` (evidence: `0x005F6612..0x005F6625`)
- `[RESOLVED] OQ-02 - Does `Detach_From_All_Lists` use the broad roster for Object-derived expired pointers? -> Yes, it tests `Object+0x14` bit 1 and loops `DAT_00B0F724` count `DAT_00B0F730`.` (evidence: `0x0072592D..0x0072595F`)
- `[RESOLVED] OQ-03 - What argument reaches callbacks in the normal UnInit path? -> `was_removed=1` via `DL=1` from `ObjectClass::UnInit`.` (evidence: `0x005F6612`)
- `[RESOLVED] OQ-04 - Which callbacks bind to Techno/Foot/Unit/Infantry/Aircraft/Building `+0x28`? -> bindings listed in section 2.` (evidence: vtable memory reads)
- `[RESOLVED] OQ-05 - Are RadioClass contact slots cleared here? -> Yes, listener `+0xE4` slots equal to expired are nulled when `was_removed != 0`.` (evidence: `0x0065AAC0`, `0x0065AAF4`)
- `[RESOLVED] OQ-06 - Does Techno remove expired passengers/cargo list nodes? -> Yes, `was_removed` calls `0x004734B0` on `Techno+0x114`.` (evidence: `0x007077E2..0x007077E9`, `0x004734B0`)
- `[RESOLVED] OQ-07 - Does pointer expiry clear CaptureManager victim refs? -> Yes, if `Techno+0x2BC` exists it calls `0x00471F90(manager, expired)`, which removes matching victim nodes.` (evidence: `0x00707B09..0x00707B14`, `0x00471F90`)
- `[RESOLVED] OQ-08 - Does pointer expiry clear Temporal target refs? -> Yes, if `Techno+0x274` exists it calls `0x0071AB60`, which handles expired target/source cases and clears temporal fields.` (evidence: `0x00707B2B..0x00707B3B`, `0x0071AB60`)
- `[RESOLVED] OQ-09 - Is there a Foot object-to-cell fallback write? -> Yes, when `Foot+0x5CC` equals expired, it writes `Foot+0x5C8 = MapClass::Get_CellClass(expired.GetCoords())` before clearing `+0x5CC`.` (evidence: `0x004D9B80..0x004D9BC5`)
- `[RESOLVED] OQ-10 - Does current Rust already clear generic radio contacts on despawn? -> Yes, `Simulation::despawn_entity` calls `clear_radio_contacts_for`, with EntityStore tests.` (evidence: `src/sim/world/mod.rs:675..698`; `src/sim/entity_store.rs:64..70`, `231..285`)
- `[DEFERRED] OQ-11 - Exact semantic names for all Techno unlabelled fields in `0x007077C0`.` (category: `requires-different-system-context`; reason: this slot classified effects but several names need constructor/load/save/writer triangulation; next-step-if-pursued: field-cluster reports for `+0x1CC/+0x1D0/+0x1D4`, `+0x2A8..+0x2B0`, and `+0x444..+0x480`)
- `[DEFERRED] OQ-12 - Exact semantic name for Infantry `+0x6C0` and Aircraft `+0x6CC`.` (category: `requires-different-system-context`; reason: vtable wrapper effect is proven but names require class-specific writer scans; next-step-if-pursued: class-local field lifecycle scan)

## 7. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Pre-conceal object expiry broadcasts to all `DAT_00B0F724` listeners and invokes class-specific pointer cleanup before limbo/conceal and alive clear. | `0x005F6612..0x005F6625`; `0x0072592D..0x0072595F` | partial: radio contacts clear on despawn, but no general per-class pointer-expired pass | `src/sim/world/mod.rs::despawn_entity`, `src/sim/entity_store.rs`, future component cleanup helper | Before entity removal, apply deterministic cleanup for every cross-entity stable-ID field that can point at the expired entity, preserving native order: generic Object/Radio-like refs, then Techno, then Foot/derived wrappers. | Despawn a target referenced by movement, attack/archive, cargo, transport, temporal/mind-control-like components; all refs clear/fallback before removal. Proposed test: `pointer_expired_cleanup_runs_before_entity_remove_for_non_radio_refs` | Do not assume reciprocal `radio_contacts` cleanup covers non-radio pointers. |
| Radio contacts are already represented as a generic vector and current Rust clears them reciprocally on despawn. | binary `0x0065AAC0`; Rust `entity_store.rs:64..70`, `world/mod.rs:695`, tests `231..285` | no broad radio-vector delta observed | `src/sim/entity_store.rs`, `src/sim/world/mod.rs` | Preserve existing order-preserving `clear_radio_contacts_for`; layer additional pointer-expiry cleanup around it rather than replacing it. | Existing radio-contact tests plus a despawn integration fixture. Proposed test: `despawn_preserves_radio_contact_order_while_clearing_expired_refs` | Do not reintroduce stale "missing generic radio cleanup" wording. |
| Foot object-target expiry can convert an object pointer to a `CellClass*` fallback at `+0x5C8` using expired object's coordinates, then clear the object pointer `+0x5CC`. | `0x004D9B80..0x004D9BC5` | likely missing/unchecked: Rust movement/navigation targets use IDs/cells but no native object-to-cell fallback on despawn found | `src/sim/game_entity.rs` movement/navigation fields, `src/sim/world/world_orders.rs`, movement target resolution | When a movement/nav target entity expires in the matching native role, preserve a cell fallback target from the expired entity's last coordinates instead of just nulling all movement state. | Unit pursuing an object target should keep/convert to last target cell after target removal when native `+0x5CC` role applies. Proposed test: `foot_pointer_expired_object_target_converts_to_last_cell_fallback` | Do not clear all destination state blindly on target despawn. |
| CaptureManager and Temporal managers receive pointer-expired callbacks from Techno when present. | `0x00707B09..0x00707B3B`; `0x00471F90`; `0x0071AB60` | missing/unchecked: Rust has `mind_controlled` bool and combat/teleport state, but no CaptureManager/Temporal manager cleanup equivalent in focused scan | future mind-control manager, temporal/chrono combat effect components | Manager-backed links must remove expired victims/targets through manager-specific cleanup, not only by clearing a victim bool or target ID. | Destroy a mind-controlled victim and controller in separate scenarios; destroy temporal target/source; manager lists and reciprocal fields clear deterministically. Proposed tests: `capture_manager_pointer_expired_removes_victim_node`, `temporal_pointer_expired_detaches_target_and_clears_reciprocal_refs` | Do not model mind control or temporal links as one-sided flags. |
| Stock non-editor pointer-expired callback does not clear Techno `+0x2E4`; it only clears when `g_MapEditorMode != 0`. | `0x00707849..0x00707859` | Rust dock cleanup may clear reservations on despawn, but should not claim this exact callback as the source for normal stock `+0x2E4` cleanup | miner/dock/contact code, `src/sim/miner`, `src/sim/docking` | Keep normal dock/reservation cleanup sourced to verified radio/dock mission paths, not to Techno pointer-expired `+0x2E4` unless map-editor mode is modeled. | A stock refinery/miner despawn cleanup test should not assert a normal-play `+0x2E4` clear from this callback. Proposed test: `stock_dock_cleanup_does_not_depend_on_map_editor_only_2e4_pointer_expired_branch` | Do not treat every matching pointer field as cleared in standard play; some are editor-gated. |

## 8. Negative Facts / Do Not Do

- Do not treat `Object+0x98` as this pointer-expiry mechanism. This path is gated by `Object+0x14` bit 1 and `DAT_00B0F724`; `Object+0x98` belongs to active logic membership, not callback dispatch. Evidence: `0x0072592D..0x0072595F`.
- Do not clear only radio contacts and call pointer cleanup complete. Techno/Foot callbacks also clear cargo, manager, target/archive, temporal, transport, and compacted vector refs. Evidence: `0x007077C0`, `0x004D9960`.
- Do not null Foot destination object state unconditionally; `Foot+0x5CC` expiry writes a `CellClass*` fallback to `+0x5C8` from the expired object's coordinates. Evidence: `0x004D9B80..0x004D9BC5`.
- Do not implement CaptureManager or Temporal cleanup as one-sided flags. The callback calls manager helpers (`0x00471F90`, `0x0071AB60`) that remove/clear reciprocal state. Evidence: `0x00707B09..0x00707B3B`.
- Do not claim Techno `+0x2E4` is cleared by pointer-expired cleanup in standard non-editor play. That clear is gated by `g_MapEditorMode != 0`. Evidence: `0x00707849..0x00707859`.

## 9. Remaining Uncertainty

- Exact semantic names for Techno fields `+0x1CC/+0x1D0/+0x1D4`, `+0x2A8/+0x2AC/+0x2B0`, `+0x428`, `+0x434`, `+0x444..+0x480`, and some `+0x2CC/+0x2D4/+0x2E0` slots remain unproven in this slice. Effects are recorded; names need writer/load/save triangulation.
- Foot `+0x694` is consistent with parasite/terror-drone links in existing unit docs, but this report did not run a full ParasiteClass field lifecycle audit.
- Infantry `+0x6C0`, Aircraft `+0x6CC`, and shared `+0x6C4` wrapper fields were verified as cleared but not semantically named here.
- Building callback has many additional building-local refs/lists; this report only classified the wrapper and major clear behavior, not every building damage-fire/upgrade side effect.

## 10. Stale Docs / Follow-up Wording

- `docs/research/BROADCAST_RADIO_TO_ALL_LIMBO_BREAK_CLEANUP_GHIDRA_REPORT.md`: replace any current-Rust wording that says there is no generic reciprocal radio-contact cleanup on despawn with: "Current Rust now calls `Simulation::clear_radio_contacts_for` from `despawn_entity`, backed by `EntityStore::clear_radio_contacts_for` tests. Remaining pointer-expiry drift is in non-radio refs: cargo/passenger chains, transport refs, target/archive refs, manager-backed CaptureManager/Temporal/Spawn/Airstrike links, Foot object-to-cell fallback, and derived-class wrapper fields."

No docs were edited besides this report.

## Sources

- Ghidra read-only decompile/assembly: `0x005F65F0`, `0x007258D0`, `0x005F5230`, `0x0065AAC0`, `0x007077C0`, `0x004D9960`, `0x0044E8F0`, `0x007446E0`, `0x0051AA10`, `0x0041B660`, `0x004734B0`, `0x00471F90`, `0x006B7C60`, `0x0071AB60`, `0x0071ABC0`, `0x0041D540`.
- Ghidra vtable memory reads: `0x007F4988`, `0x007E8CBC`, `0x007F5C98`, `0x007EB080`, `0x007E22CC`, `0x007E3EE4`.
- Existing research referenced: `MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md`, `IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md`, `TECHNO_EJECT_PASSENGER_VIRTUAL_SLOTS_RESWARM_20260528.md`, `AIRCRAFTCLASS_0XA5_RADIO_GATE_WRITERS_GHIDRA_REPORT.md`, `UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md`.
- Rust focused scan: `src/sim/game_entity.rs`, `src/sim/entity_store.rs`, `src/sim/world/mod.rs`, `src/sim/world/world_orders.rs`, `src/sim/passenger.rs`.
