# Building Owner / House Binding Parity — Ghidra Research Report

**Address(es):** `0x0044F820` (`BuildingClass::ReadFromINI`), `0x0043B740` (`BuildingClass::Constructor`), `0x006F2B40` (`TechnoClass::Constructor`), `0x00442C40` (`BuildingClass::Init_Managers`), `0x0045E880` (`BuildingTypeClass` create-object vtable target)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** owner/house pointer storage and initialization for map-placed buildings, constructed/player-created buildings through the verified create-object path, MCV deploy building creation, and neutral/civilian buildings when they use the same map `[Structures]` path.  
**Non-Scope:** type identity, placement legality, foundation occupation, health math, tag/upgrades beyond owner argument propagation, capture/change-owner behavior after initialization, save/load persistence.  
**Confidence:** High for the owner pointer slot and verified creation paths; Medium for some caller labels where the decompiler has unnamed functions but direct constructor call semantics are clear.  
**Active in YR:** Yes / Conditional. The paths are live in `gamemd.exe`; `[Structures]` loading is conditional on house lookup and game-mode/local-player gating described below.

## 1. Overview

Buildings do not store a separate BuildingClass-local owner field. The owning house pointer is stored by `TechnoClass::Constructor` at `TechnoClass + 0x21C` (`param_1[0x87]`) from the constructor's second argument. `BuildingClass::Constructor` forwards its house argument directly to `TechnoClass::Constructor`, then `BuildingClass::Init_Managers` reads `+0x21C` to register the building with the house.

Map-placed structures are read from `[Structures]` by `BuildingClass::ReadFromINI`; field 0 is resolved with `HouseClass::FindByName`, and the resulting `HouseClass*` is passed to `BuildingClass::Constructor`. Production-created buildings use the BuildingType create-object vtable target at `0x0045E880`, which allocates a `BuildingClass` and calls the same constructor with the caller-supplied house pointer.

## 2. Class Layout / Key Offsets

| Class / field | Offset | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|
| `TechnoClass::Owner` / house pointer | `+0x21C` (`param_1[0x87]`) | Stores the owning `HouseClass*` passed to the Techno constructor. | `TechnoClass::Constructor @ 0x006F2B40` writes `param_1[0x87] = param_2`; `BuildingClass::Init_Managers @ 0x00442C40` reads `param + 0x21C`. | Yes — common Techno path for buildings. |
| Building type pointer | `BuildingClass + 0x520` (`param_1[0x148]`) | Stores `BuildingTypeClass*`; not owner, but used to identify building and initialize type-owned fields. | `BuildingClass::Constructor @ 0x0043B740` writes `param_1[0x148] = param_2`; `BuildingClass::Init_Managers @ 0x00442C40` reads `+0x520`. | Yes. |
| House tracking flag | `TechnoClass + 0x3CC` (`param_2 + 0xF3` as dword-indexed byte) | Set by `HouseClass::Add_Tracking` when registering object with owning house. | `HouseClass::Add_Tracking @ 0x004FF700` first write. | Yes, if owner pointer is non-null. |
| Building count in house | `HouseClass + 0x2F0` | Incremented for normal buildings in `HouseClass::Add_Tracking` case `WhatAmI == 6`. | `HouseClass::Add_Tracking @ 0x004FF700`. | Conditional — excludes some special/alternate cases based on object/type checks. |

## 3. Core Logic

### 3.1 Common constructor path

1. `BuildingClass::Constructor @ 0x0043B740` takes `(building_type_ptr, house_ptr)`.
2. It calls `TechnoClass::Constructor(house_ptr)`.
3. `TechnoClass::Constructor @ 0x006F2B40` stores that value at `this + 0x21C`.
4. `BuildingClass::Constructor` stores `building_type_ptr` at `this + 0x520`.
5. `BuildingClass::Constructor` calls `BuildingClass::Init_Managers`.
6. `BuildingClass::Init_Managers @ 0x00442C40` checks `this + 0x21C != 0`; if non-null, it copies a house-derived value into `this + 0x53C` and calls `HouseClass::Add_Tracking(this)`.
7. `HouseClass::Add_Tracking @ 0x004FF700` uses the `HouseClass*` as `this` and the building object as its second argument, then updates house tracking counters according to `WhatAmI`.

**Active in YR:** Yes. This path is reached by verified BuildingClass construction paths and is not TS-gated in the examined code.

### 3.2 Map-placed `[Structures]`

`BuildingClass::ReadFromINI @ 0x0044F820` reads section string `Structures` (`0x007E3E88`, xrefs at `0x0044F832`, `0x0044F86A`, `0x0044F879`). For each entry:

1. Reads the value string and tokenizes by comma.
2. First token is passed to `HouseClass::FindByName @ 0x0050C170`.
3. If the name is not found, the row is skipped (`FindByName` returns `-1`).
4. If found, the index loads `HouseClass*` from `g_HouseClass_Array[index]`; null pointer rows are skipped.
5. Additional game-mode gate: the row proceeds when `g_GameMode == 0` or the resolved house pointer is not `g_PlayerPtr`.
6. Second token resolves `BuildingTypeClass` via `FUN_0045E7B0`; invalid type rows are skipped.
7. After parsing fields, it allocates `0x720` bytes and calls `BuildingClass::Constructor(building_type_ptr, house_ptr)`.

**Active in YR:** Conditional. This is the active `[Structures]` map load path, but local-player rows are gated in nonzero `g_GameMode`; neutral/civilian/non-player rows proceed when their house name resolves.

### 3.3 Constructed/player structures

`HouseClass::Begin_Production @ 0x004FA350` creates production objects through a type vtable method with the producing `HouseClass*` as an argument. The BuildingType vtable entry at `0x007E45FC` points to `0x0045E880`, which:

1. Allocates `0x720` bytes.
2. Calls `BuildingClass::Constructor(building_type_ptr, house_ptr)`.
3. Returns the new object pointer or zero.

`HouseClass::Place_Production @ 0x004FB0E0` later places the factory object by calling the object's placement/unlimbo vtable path; it does not create a new owner binding for the already-created factory object in the verified branch.

**Active in YR:** Yes. This is the production create-object path used by house production code.

### 3.4 MCV deploy building creation

`UnitClass::Deploy @ 0x007393C0` allocates a `BuildingClass` and calls `BuildingClass::Constructor(building_type_ptr, unit_owner_house_ptr)` when deploying into a building. The owner argument is read from the deploying unit's existing Techno owner field (same `+0x21C` storage family).

**Active in YR:** Yes. This path is live for deployable construction vehicles; additional placement/facing branches are outside this report.

### 3.5 Neutral / civilian structures

No separate neutral/civilian special constructor was found in this slice. Neutral/civilian buildings in `[Structures]` use the same `BuildingClass::ReadFromINI` token-0 house lookup path. If `Neutral`, `Special`, `Civilian`, or another map house name exists in `g_HouseClass_Array`, the resulting `HouseClass*` is passed as the owner; if not, the structure row is skipped.

**Active in YR:** Conditional on the map/session creating the named house before `[Structures]` load.

## 4. INI Keys

No rules/art INI key controls building instance owner binding. The material data source is the map `[Structures]` value layout:

| Source | Field | Effect | Evidence | Active in YR |
|---|---:|---|---|---|
| map `[Structures]` | token 0 | Owner/house name resolved by `HouseClass::FindByName`. | `BuildingClass::ReadFromINI @ 0x0044F820`; string xrefs to `Structures`. | Yes / Conditional per game-mode and valid-house gates. |
| map `[Structures]` | token 1 | Building type id; only used to select the BuildingType pointer passed alongside owner. | `BuildingClass::ReadFromINI @ 0x0044F820`, `FUN_0045E7B0` result. | Yes. |
| rules `[BuildingType] Owner=` | N/A | Build availability/cameo ownership, not instance owner storage. | Rust scan and binary slice did not find this in instance constructor path; production passes `HouseClass*` from producer. | Not the instance owner-binding mechanism. |

## 5. Integration Points

| Path | Verified owner source | Constructor call | Active in YR |
|---|---|---|---|
| Map `[Structures]` | `HouseClass::FindByName(token0)` then `g_HouseClass_Array[index]` | `0x0044F820 -> 0x0043B740` | Conditional; valid house and game-mode gate. |
| Production create-object | Producing `HouseClass*` passed to BuildingType create vtable method | `0x004FA350 -> vtable @ 0x007E45FC -> 0x0045E880 -> 0x0043B740` | Yes. |
| Production placement | Existing factory object | `0x004FB0E0` places existing object; no new owner constructor in verified placement branch. | Yes. |
| Unit deploy / MCV | Deploying unit's owner house pointer | `0x007393C0 -> 0x0043B740` | Yes. |
| House tracking | `this + 0x21C` owner pointer | `0x00442C40 -> 0x004FF700` | Yes if owner non-null. |

## 6. Current Rust Implementation Status

| Area | Status | Rust evidence | Binary parity notes |
|---|---|---|---|
| Map parser preserves owner token | MATCH | `src/map/entities.rs:188-210`, `src/map/entities.rs:240-279` read `[Structures]` field 0 into `MapEntity.owner`. | Matches the binary's token-0 owner source. |
| Map spawn stores owner on building entity | MATCH | `src/sim/world/world_spawn.rs:111-129` interns `map_ent.owner` and passes it to `GameEntity::new`; `src/sim/game_entity.rs:82-83`, `src/sim/game_entity.rs:283-323` store it. | Equivalent to `TechnoClass +0x21C` as the instance owner field, using interned ID instead of pointer. |
| Invalid house handling for map structures | MISMATCH / UNKNOWN | `src/sim/world/world_spawn.rs:111-129` interns any owner string; `increment_owned_count` only updates if a `HouseState` exists (`src/sim/world/mod.rs:501-512`). | gamemd skips `[Structures]` rows whose owner name is not found or whose house pointer is null. Rust may spawn an entity with an owner that has no house state. |
| Nonzero game-mode local-player `[Structures]` gate | UNKNOWN | No equivalent gate found in the relevant Rust map spawn path. | gamemd proceeds only if `g_GameMode == 0 || house != g_PlayerPtr`. Need session-mode mapping before judging. |
| Neutral/civilian map structures | MATCH when house exists; MISMATCH if missing house is still spawned | Same map owner-token path as above. | gamemd uses same path and requires `HouseClass::FindByName` success. |
| Production-created final placed building owner | MATCH | `src/sim/production/production_placement.rs:180-248` calls `sim.spawn_object(type_id, owner, ...)`; `src/sim/world/world_spawn.rs:280-337` stores `owner_iid`. | Equivalent final owner binding for placed buildings. |
| Ready building object lifetime | MISMATCH / UNKNOWN | `src/sim/production/production_queue.rs:479-487` records ready building type in `ready_by_owner`; object is spawned only during `place_ready_building`. | gamemd creates a `BuildingClass` production object with owner before placement via `0x0045E880`; observable impact outside owner binding was not investigated. |
| House registration / owned count | PARTIAL MATCH | `src/sim/world/world_spawn.rs:253-254`, `src/sim/world/world_spawn.rs:425-426` call `increment_owned_count`; `src/sim/world/mod.rs:501-512` increments building/unit counts. | gamemd calls `HouseClass::Add_Tracking` from `BuildingClass::Init_Managers`; its building count path has type/virtual gates not mirrored in this narrow comparison. |
| Per-owner entity index | MATCH after tick rebuild / UNKNOWN same-tick | `src/sim/world/mod.rs:1023-1026` rebuilds `EntityStore` owner index each tick; `EntityStore::insert` itself does not update `by_owner` (`src/sim/entity_store.rs:49-54`, `128-137`). | gamemd registration happens during constructor/init. Same-tick `ids_for_owner` freshness before tick rebuild was not audited. |
| Capture/change-owner | DEFERRED | Rust owner mutation at `src/sim/world/world_orders.rs:228-241`; binary `BuildingClass::ChangeOwner @ 0x00448260` was not analyzed for this report. | Out of scope; not initialization. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TechnoClass::Constructor` owner storage | verified | `0x006F2B40` writes owner argument to `+0x21C` | none |
| `BuildingClass::Constructor` forwarding | verified | `0x0043B740` calls Techno ctor with house arg; stores type at `+0x520` | none |
| `BuildingClass::Init_Managers` owner read | verified | `0x00442C40` reads `+0x21C`, calls `HouseClass::Add_Tracking` | none |
| `HouseClass::Add_Tracking` building registration | touched-not-exhausted | `0x004FF700` case `WhatAmI==6` | exact meaning of special gates is outside owner-binding slice |
| `[Structures]` map load owner lookup | verified | `0x0044F820`, `0x0050C170`, string xrefs `0x007E3E88` | none for owner binding |
| Production create-object owner path | verified | `0x004FA350`, vtable data xref `0x007E45FC -> 0x0045E880`, `0x0045E880` ctor call | none |
| Production placement owner mutation | verified-negative | `0x004FB0E0` places existing object in examined branch | none for owner binding |
| MCV deploy owner path | verified | `0x007393C0` allocates building and passes deploying unit owner to ctor | none |
| Neutral/civilian special path | touched-not-exhausted | same `[Structures]` path verified | separate scripted/trigger-spawn neutral creation, if any, deferred |
| Rust map spawn comparison | verified | `src/map/entities.rs`, `src/sim/world/world_spawn.rs`, `src/sim/game_entity.rs` | none |
| Rust production placement comparison | verified | `src/sim/production/production_placement.rs`, `src/sim/world/world_spawn.rs`, `src/sim/production/production_queue.rs` | ready-object lifetime impact deferred |

## 8. Open Questions - Final State

[RESOLVED] OQ1 - Where is the building owner stored? Answer: common Techno owner slot `+0x21C`; evidence `0x006F2B40`, `0x00442C40`.

[RESOLVED] OQ2 - Do map `[Structures]` rows pass owner names through type data or house lookup? Answer: house lookup from token 0 with `HouseClass::FindByName`, then `g_HouseClass_Array[index]`; evidence `0x0044F820`, `0x0050C170`.

[RESOLVED] OQ3 - Are invalid map owner names tolerated? Answer: no, rows are skipped if lookup returns `-1` or house pointer is null; evidence `0x0044F820`, `0x0050C170`.

[RESOLVED] OQ4 - Do neutral/civilian structures need a special owner binding path? Answer: not for `[Structures]`; they use the same token-0 house lookup if the named house exists; evidence `0x0044F820`.

[RESOLVED] OQ5 - How are production-created buildings initially bound to a house? Answer: BuildingType create-object vtable target `0x0045E880` calls `BuildingClass::Constructor(type, house)`; evidence `0x004FA350`, data xref `0x007E45FC`, `0x0045E880`.

[RESOLVED] OQ6 - Does production placement create a new owner binding? Answer: no in the verified path; it places the existing factory object; evidence `0x004FB0E0`.

[RESOLVED] OQ7 - Does MCV deploy use the same owner slot? Answer: yes; deploy creates a BuildingClass with the unit's owner house pointer; evidence `0x007393C0`, `0x0043B740`, `0x006F2B40`.

[DEFERRED] OQ8 - Exact owner behavior for post-initialization `BuildingClass::ChangeOwner`. Reason: out-of-scope, not initialization. Category: out-of-scope. Next step: dedicated change-owner parity investigation.

[DEFERRED] OQ9 - Scripted trigger spawns or map editor RMG neutral building generation beyond same constructor call inventory. Reason: bounded-cost-too-high and not required for map `[Structures]`/production owner binding. Category: requires-different-system-context.

## Sources

- Ghidra: `0x0044F820`, `0x0050C170`, `0x0043B740`, `0x006F2B40`, `0x00442C40`, `0x004FF700`, `0x004FA350`, `0x0045E880`, `0x004FB0E0`, `0x007393C0`.
- Ghidra data/xrefs: `0x007E3E88` (`Structures` string), `0x007E45FC -> 0x0045E880` BuildingType create-object vtable entry.
- Rust files read: `src/map/entities.rs`, `src/sim/world/world_spawn.rs`, `src/sim/game_entity.rs`, `src/sim/production/production_placement.rs`, `src/sim/production/production_queue.rs`, `src/sim/world/mod.rs`, `src/sim/entity_store.rs`, `src/sim/world/world_orders.rs`, `src/sim/house_state.rs`.
