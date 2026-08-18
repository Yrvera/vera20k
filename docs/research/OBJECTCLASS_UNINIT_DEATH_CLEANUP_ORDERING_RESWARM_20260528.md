# ObjectClass UnInit Death Cleanup Ordering - Re-swarm Research Report

**Address(es):** `ObjectClass::UnInit @ 0x005F65F0`, `ObjectClass::Conceal @ 0x005F4D30`, `Detach_From_All_Lists @ 0x007258D0`, `FootClass::UnInit @ 0x004DE5D0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** exact ordering from `ObjectClass::UnInit` entry through immediate base cleanup, `FootClass::UnInit` pre-base wrapper cleanup, `Conceal` display/cell/list removal, pointer-expired/list invalidation dispatch, `IsAlive=0`, and pending-delete append.  
**Non-Scope:** class-specific damage formulas, death animation selection/inventory, full pending-delete drain processor, every listener body reached from `Detach_From_All_Lists`, and every non-destruction limbo caller.  
**Confidence:** High for top-level ordering and named immediate callees; Medium for semantic names of several global listener arrays not drained in this slot.  
**Active in YR:** Yes. `ObjectClass::UnInit` is in many concrete object vtables at slot `+0xF8`, is called by common live paths such as `AnimClass::Destroy @ 0x004255B0`, `FootClass::UnInit @ 0x004DE5D0`, terrain destruction reports, and bullet self-destruction reports. (corrected 2026-05-29: was `0x00425610`; that is an internal instruction offset, not the entry point; entry confirmed via `get_function_by_address 0x004255B0` — RTTI_LABEL_DRIFT)

## Working Notes Seed

Target question: What exact cleanup order does active YR use when an object is uninitialized/deleted through `ObjectClass::UnInit`, especially reference invalidation, conceal/removal, alive clear, and pending-delete append?  
Non-goals: Do not investigate full class-specific death animations, full damage formulas, full pending-delete drain, or every listener implementation.  
Evidence needed to mark COMPLETE: decompile plus disassembly for `0x005F65F0`, decompile/disassembly for immediate callees, xref/vtable evidence for active YR use, current Rust surface scan, and at least one implementation handoff.  
Stop conditions: stop at immediate base cleanup and wrapper ordering; record deeper listener bodies or class-specific death effects as Remaining Uncertainty unless directly needed for ordering.

## 1. Overview

The active YR destruction path does not first flip an object to dead and then clean it up. `ObjectClass::UnInit` first defuses an attached bomb if present, recursively uninitializes carried passengers for object-flagged instances, dispatches reference/list invalidation through `Detach_From_All_Lists`, then calls virtual `+0xD4` to conceal/remove from map and display. Only after that does it write `ObjectClass+0x90 = 0` and append the object to the deferred pending-delete vector.

This order is player-visible through selection/display/cell occupancy and mechanically visible through in-flight bullets: bullets receive pointer-expired notification before the target is concealed and before the target's alive byte is cleared.

## 2. Class Layout / Key Offsets

| Field / slot | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `Object+0x38` | attached `BombClass*` / bomb-like pointer defused first | `0x005F65F3..0x005F65FA`; `BombClass::Defuse @ 0x004389B0` | Conditional; active when object carries a bomb |
| `AbstractFlags +0x14 bit 0` | gate for `FootClass::EMPPassengers(0)` in base `UnInit` | `0x005F65FF..0x005F660D` tests `[ESI+0x14] & 1` | Conditional; active for object classes with this flag set |
| `Object+0x81` | `InLimbo`; `Conceal` early-out and final set | `0x005F4D45..0x005F4D4D`, `0x005F4E9E` | Yes |
| `Object+0x90` | alive byte cleared after conceal | `0x005F6625` | Yes |
| `Object+0x94` | display layer index, set to `-1` by `DisplayClass::RemoveFromLayer` | `0x004A9770` decompile | Conditional; active when display layer is set |
| `Object+0x98` | logic-vector membership flag cleared by `FUN_0055BAE0` inside `Conceal` under type/game-mode gate | `0x005F4DCD` call to `0x0055BAE0`; prior logic-vector remover report | Conditional; active for eligible object/type/game mode branch |
| pending delete vector | data/count/capacity at `0x00B0F69C/0x00B0F6A8/0x00B0F6A0` | `0x005F662C..0x005F667D` | Yes |
| virtual `+0x28` | pointer-expired / removal notification slot | `Detach_From_All_Lists @ 0x007258D0` calls `[listener_vtbl+0x28]` | Yes |
| virtual `+0xD4` | conceal/limbo slot | `0x005F661B..0x005F661F`; BulletClass vtable `0x007E47B8 -> 0x005F4D30` | Yes |
| virtual `+0xF8` | uninit/delete slot | `ObjectClass::UnInit @ 0x005F65F0`; xrefs from many vtables | Yes |

## 3. Core Logic

### 3.1 `ObjectClass::UnInit @ 0x005F65F0`

Verified ordered sequence:

1. Load `this` into `ESI`.
2. If `[ESI+0x38] != 0`, call `BombClass::Defuse @ 0x004389B0`.
3. If `this != null` and `AbstractFlags+0x14 bit 0` is set, call `FootClass::EMPPassengers(0) @ 0x00707CB0`.
4. Call `Detach_From_All_Lists @ 0x007258D0` with `EDX = 1`.
5. Dispatch virtual `this->vtable+0xD4`, normally `ObjectClass::Conceal @ 0x005F4D30` for the scoped base path.
6. Write byte `[ESI+0x90] = 0`.
7. Append `ESI` to the pending-delete vector if capacity permits or vector growth succeeds.
8. Return; no destructor is called inline in this function.

Evidence: decompile of `0x005F65F0`; disassembly `0x005F65F3..0x005F667D`. Active in YR: Yes.

### 3.2 Bomb Defuse Comes Before All Detach/Conceal Work

`BombClass::Defuse @ 0x004389B0` clears back-pointers on the bomb's attached object if `bomb+0x2C` is non-null, marks byte `+0x58 = 1`, clears `+0x24`, `+0x2C`, `+0x28`, stops a `VocHandle`, and clears `+0x54`.

Evidence: decompile `0x004389B0`; call site `0x005F65F3..0x005F65FA`. Active in YR: Conditional; active only when `Object+0x38` is non-null. This is also called directly by `WarheadTypeClass::Detonate`, `ObjectClass::Destructor`, and `BuildingClass::ChangeOwner`, so it is not an inert helper.

### 3.3 Passenger / EMP Hook Runs Before Reference Invalidation

Base `UnInit` calls `FootClass::EMPPassengers(0)` when object flag bit 0 is set. The callee walks the carried-object list at `+0x118`; for each passenger-like object it may detach an EMP/transport relation via `FUN_006EA870`, recurses into `FootClass::EMPPassengers`, dispatches virtual `+0xE0` with the inherited parameter, then dispatches virtual `+0xF8` on the passenger.

Evidence: `0x005F65FF..0x005F660D`; decompile `0x00707CB0`. Active in YR: Conditional; xrefs include `ObjectClass::UnInit`, `AircraftClass::Constructor`, `FootClass::ReceiveEMP`, `BuildingClass::DestructionEffects`, `UnitClass::ReceiveDamage`, and `TechnoClass::ReceiveDamage`.

Ordering implication: carried passengers can be recursively uninitialized before the carrier's own `Detach_From_All_Lists`, `Conceal`, alive clear, and pending-delete append.

### 3.4 `FootClass::UnInit @ 0x004DE5D0` Wrapper

For foot-derived classes using this vtable slot, extra cleanup happens before base `ObjectClass::UnInit`:

1. If `Foot+0x2BC != 0`, call `CaptureManagerClass::FreeAll @ 0x00472140`.
2. If `Foot+0x2AC != 0`, call `BuildingClass::DeployUnit_ChronoWarp(1) @ 0x0070FEE0`.
3. If `Foot+0x5D4 != 0`, call `FUN_006EA870(this, -1, 0)` to detach from transport/passenger state.
4. Then call `ObjectClass::UnInit @ 0x005F65F0`.

Evidence: decompile/disassembly `0x004DE5D0..0x004DE611`; data xrefs from foot-family vtables at `0x007E239C`, `0x007E8D8C`, `0x007EB150`, `0x007F5D68`. Active in YR: Yes for foot-family objects using those vtables; exact class names for each vtable were not rederived in this slot.

### 3.5 `Detach_From_All_Lists @ 0x007258D0` Runs Before Conceal

`UnInit` calls `Detach_From_All_Lists` at `0x005F6616`, before the virtual conceal call at `0x005F661F`. `Detach_From_All_Lists` first obtains `WhatAmI`/RTTI via virtual `+0x2C`, clears global current/mode pointers if they equal the expiring object, then dispatches pointer-expired notifications through listener arrays by calling each listener's virtual `+0x28` with the expiring object and the removal flag.

For ordinary object-registered classes, the branch gated by `AbstractFlags+0x14 bit 1` iterates `DAT_00B0F724`/`DAT_00B0F730`, then performs cleanup calls including `FUN_00439150`, `SpawnRetreat::Remove`, reverse `DiskLaserClass::DetachFromObject`, optional no-op `FUN_00413490` for RTTI `0x0F/1/2`, `FUN_00733160`, tactical vtable `+0x28`, and `FUN_0055B880`.

Evidence: decompile/disassembly `0x007258D0..0x00725C0D`; call order in `0x005F6612..0x005F661F`. Active in YR: Yes.

### 3.6 Bullet Target Invalidation Is Before Conceal and Alive Clear

The standard in-flight bullet invalidation path is reached as:

`ObjectClass::UnInit @ 0x005F6616` -> `Detach_From_All_Lists @ 0x007258D0` -> listener virtual `+0x28` -> `BulletClass` pointer-expired handler body `0x004684E0`.

Evidence chain: current disassembly proves `Detach_From_All_Lists` precedes conceal and alive clear; BulletClass vtable base `0x007E46E4` has slot `+0x28` at `0x007E470C`, and a live memory read returned `0x004684E0`; prior `BULLETCLASS_TARGET_INVALIDATION_AAHEATSEEKER2_GHIDRA_REPORT.md` verifies the handler writes `BulletClass+0x10C` to a `CellClass*` or null. Active in YR: Yes for normal bullet objects, including stock AAHeatSeeker2 projectile paths.

### 3.7 `ObjectClass::Conceal @ 0x005F4D30`

`Conceal` early-outs if `g_GameActive == 0` or `Object+0x81 InLimbo != 0`. Otherwise:

1. Dispatch virtual `+0x150` deselect.
2. Dispatch virtual `+0xDC(1)`.
3. Dispatch virtual `+0x124(0)`.
4. `DisplayClass::RemoveFromLayer(this)`.
5. `AnimClass::Detach`.
6. `VocHandle::Stop`.
7. If type exists and type `+0x234` is true, and game-mode/owner gate allows, call `FUN_0055BAE0(this)` to unregister from logic/object active structures.
8. If type has `+0xAC`, dirty the tactical screen rect around current coords and image size.
9. Dispatch virtual `+0x11C`.
10. Write `Object+0x81 = 1`.
11. Write `Object+0x80 = 0`.
12. Return `1`.

Evidence: decompile/disassembly `0x005F4D30..0x005F4EBD`; callees `DisplayClass::RemoveFromLayer @ 0x004A9770`, `AnimClass::Detach @ 0x00405D40`, `VocHandle::Stop @ 0x00405FD0`. Active in YR: Yes.

Ordering implication: display/layer, attached animation/audio, tactical dirtying, and limbo flag set all occur before `Object+0x90` is cleared by the caller.

### 3.8 Pending Delete Append Is Last and Can Early-Return on Capacity Failure

After alive clear, `UnInit` appends `this` to the pending-delete vector at `0x00B0F69C`. If `count >= capacity`, it only grows when the vector flags/capacity increment permit. Failure to grow returns without appending. No destructor is invoked inline.

Evidence: `0x005F662C..0x005F6680`. Active in YR: Yes; capacity failure is a low-probability allocator/vector condition, but it is live code.

## 4. INI Keys

No INI keys directly gate the scoped base `ObjectClass::UnInit` ordering. Stock-content activation examples come from existing projectile and object reports rather than a cleanup-specific INI key. Active in YR: Yes, because the vtable/call paths are engine paths, not optional INI behavior.

## 5. Integration Points

| Entry / integration | Ordering role | Evidence | Active in YR |
|---|---|---|---|
| `AnimClass::Destroy @ 0x004255B0` | detaches owner object, releases sound, optional stop sound, then calls `ObjectClass::UnInit` at `0x0042561F` | decompile/disassembly `0x004255B0` (corrected 2026-05-29: was `0x00425610`; that is an internal instruction, not the entry; entry confirmed via `get_function_by_address` — RTTI_LABEL_DRIFT) | Yes |
| `FootClass::UnInit @ 0x004DE5D0` | class wrapper cleanup before base `UnInit` | decompile/disassembly `0x004DE5D0..0x004DE611`; vtable data xrefs | Yes |
| terrain destruction paths | call vtable `+0xF8`, resolved to `ObjectClass::UnInit` for terrain/voxel anim in prior docs | `TERRAIN_CLASS_GHIDRA_REPORT.md`, `VOXELANIMCLASS_GHIDRA_REPORT.md` | Yes / Conditional by object |
| bullet detonation/self-removal | `BulletClass::AI` can dispatch `+0xF8` after detonation | `COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES_GHIDRA_REPORT.md` | Yes |
| `Detach_From_All_Lists` listener dispatch | reference invalidation before conceal/alive clear | `0x007258D0`; `0x005F6616` call site | Yes |
| `Conceal` display/cell removal | map/display/list removal before alive clear | `0x005F4D30`; `0x005F661F` call site | Yes |

## 6. Current Rust Implementation Status

| Rust surface | Current behavior | Delta |
|---|---|---|
| `src/sim/combat/mod.rs` `handle_entity_deaths` | applies damage, then calls `clear_targets_on_dead_entity`, marks animated entities `dying` and clears selected/attack/movement, or immediately removes structures/vehicles from `EntityStore` | No single `UnInit` equivalent that defuses bomb, recursively uninitializes passengers, dispatches pointer-expired notifications before conceal, then clears alive and appends pending delete. |
| `src/sim/world/mod.rs` `despawn_entity` | removes origin occupancy, clears radio contacts, removes from `EntityStore`, unregisters live object order | Physical removal is immediate and mostly ID-map based; no one-tick pending-delete object window and no ordered conceal/alive split. |
| `src/app_sim_tick.rs` death animation cleanup | after `advance_tick`, `tick_animations` returns finished dying IDs; app removes occupancy and calls `despawn_entity` | Rust animated death keeps entity in store with `health.current == 0`/`dying`, unlike gamemd `UnInit` clearing alive only after conceal/pointer invalidation and then queuing deferred delete. |
| `src/sim/movement/homing_movement.rs` | if target lookup fails, sets homing `target_id = None` and uses last-known cell | Similar high-level lost-target behavior, but not the gamemd pointer-expired mechanism; does not retarget to a `CellClass*` before target conceal/alive clear. |
| `src/sim/passenger.rs` / passenger cargo fields | transport/garrison cargo states are modeled separately; some death paths kill riders or eject garrison occupants | No recursive `FootClass::EMPPassengers -> passenger +0xF8` ordering equivalent found. |
| `src/sim/components.rs` parachute animation | app-side chute follows `target_id` and is removed when entity lands or dies | No direct native parachute cleanup ordering found in scoped base path; Rust chute cleanup remains an app-side target lookup behavior. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `ObjectClass::UnInit` exact top-level order | verified | decompile + disassembly `0x005F65F0..0x005F6681` | none |
| attached bomb defuse order | verified | call `0x005F65FA`; decompile `0x004389B0` | exact Ivan bomb class field names not rederived |
| passenger/EMP hook placement | verified | call `0x005F660D`; decompile `0x00707CB0` | exact class inventory for flag bit 0 deferred |
| `FootClass::UnInit` pre-base wrapper | verified | decompile/disassembly `0x004DE5D0..0x004DE611`; data xrefs (corrected 2026-05-29: end was `0x004DE610`; Ghidra body end is `0x004DE611` inclusive of `RET` — STALE boundary via `get_function_by_address`) | exact vtable class names deferred |
| `Detach_From_All_Lists` before conceal/alive clear | verified | `0x005F6616` before `0x005F661F` and `0x005F6625`; decompile `0x007258D0` | full listener body census deferred |
| Bullet pointer-expired ordering | verified | `0x007E470C -> 0x004684E0`; current `Detach` order; prior bullet invalidation report | no new decompile function boundary for `0x004684E0` because Ghidra lacks function boundary; body covered by prior report and disassembled bytes |
| `Conceal` display/cell/anim/audio order | verified | decompile/disassembly `0x005F4D30..0x005F4EBD` | subclass-specific `+0xDC/+0x124` effects not rederived here |
| logic unregistration through `FUN_0055BAE0` inside `Conceal` | verified for placement in `Conceal` | `0x005F4DCD`; prior logic-remover report | exact gate names inherited from prior reports |
| `Object+0x90=0` after conceal | verified | `0x005F6625` after virtual `+0xD4` call | none |
| pending-delete append after alive clear | verified | `0x005F662C..0x005F667D` | drain processor out of scope |
| direct parachute cleanup in base path | touched-not-exhausted | no direct parachute-specific callee in `UnInit`, `FootClass::UnInit`, or `Conceal` | separate paradrop/parachute native lifecycle investigation |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-001 - What is the exact `ObjectClass::UnInit` top-level order? -> Bomb defuse, passenger/EMP hook, `Detach_From_All_Lists`, virtual conceal, `Object+0x90=0`, pending-delete append.` (evidence: `0x005F65F0..0x005F667D`; Active in YR: Yes)
- `[RESOLVED] OQ-002 - Does pointer/reference invalidation occur before conceal and alive clear? -> Yes; `Detach_From_All_Lists` is called at `0x005F6616`, before virtual `+0xD4` and `[+0x90]=0`.` (evidence: `0x005F6612..0x005F6625`; Active in YR: Yes)
- `[RESOLVED] OQ-003 - Does `Conceal` happen before `IsAlive=0`? -> Yes; virtual `+0xD4` call completes before `MOV byte ptr [ESI+0x90],0`.` (evidence: `0x005F661B..0x005F6625`; Active in YR: Yes)
- `[RESOLVED] OQ-004 - What does `Conceal` remove before limbo flag set? -> selection, mark/remove calls, display layer, attached anim handle, vocal handle, optional logic membership, tactical dirty rect, then `+0x81=1` and `+0x80=0`.` (evidence: `0x005F4D30..0x005F4EBD`; Active in YR: Yes)
- `[RESOLVED] OQ-005 - Is pending delete immediate destructor execution? -> No; `UnInit` appends the object pointer to `0x00B0F69C` after alive clear and returns.` (evidence: `0x005F662C..0x005F6680`; Active in YR: Yes)
- `[RESOLVED] OQ-006 - Does `FootClass::UnInit` add pre-base cleanup? -> Yes; capture manager free, chrono/deploy detach, transport/passenger detach, then base `ObjectClass::UnInit`.` (evidence: `0x004DE5D0..0x004DE611`; Active in YR: Yes for foot-family vtables)
- `[RESOLVED] OQ-007 - Is bullet target invalidation placed before target conceal/alive clear? -> Yes; BulletClass `+0x28` is reached through pre-conceal `Detach_From_All_Lists`.` (evidence: `0x007258D0`, `0x007E470C -> 0x004684E0`, `BULLETCLASS_TARGET_INVALIDATION_AAHEATSEEKER2_GHIDRA_REPORT.md`; Active in YR: Yes)
- `[RESOLVED] OQ-008 - Is `Conceal` itself enough to notify bullets/references? -> No; scoped `Conceal` decompile has display/cell/anim/audio/logical removal but no `Detach_From_All_Lists` call.` (evidence: `0x005F4D30..0x005F4EBD`; Active in YR: Yes)
- `[RESOLVED] OQ-009 - Does bomb defuse occur after map removal? -> No; it is first in `UnInit` when `+0x38` is non-null.` (evidence: `0x005F65F3..0x005F65FA`; Active in YR: Conditional)
- `[RESOLVED] OQ-010 - Is Rust physical removal currently ordered like gamemd? -> No; current Rust immediately removes some dead entities or defers animated ones through app tick, without a central pre-conceal reference notification and post-conceal alive clear.` (evidence: `src/sim/combat/mod.rs`, `src/sim/world/mod.rs`, `src/app_sim_tick.rs`; Active in YR comparison: Yes)
- `[DEFERRED] OQ-011 - Which exact concrete classes set `AbstractFlags+0x14 bit 0` for the base `EMPPassengers` gate?` (category: requires-different-system-context; reason: top-level order is proven without full class flag inventory; next-step-if-pursued: vtable/constructor census for object flag bits)
- `[DEFERRED] OQ-012 - What is the exact pending-delete drain function and same-frame destructor order?` (category: out-of-scope; reason: parent already treats pending-delete as a one-tick window and this slot stops at append; next-step-if-pursued: trace `0x00B0F69C` consumers)
- `[DEFERRED] OQ-013 - Does native parachute cleanup have a class-specific pre-base hook outside the scoped functions?` (category: requires-different-system-context; reason: no direct parachute-specific callee appeared in base `UnInit`, `FootClass::UnInit`, or `Conceal`; next-step-if-pursued: paradrop/parachute class lifecycle trace)
- `[DEFERRED] OQ-014 - What does every listener body called by `Detach_From_All_Lists` do?` (category: bounded-cost-too-high; reason: this slot only needed ordering and bullet placement; next-step-if-pursued: listener-array census by RTTI)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `UnInit` dispatches reference/list invalidation before conceal and before alive clear. | `0x005F6616` before `0x005F661F`/`0x005F6625`; `0x007258D0` decompile | Missing central equivalent; Rust clears some targets via `clear_targets_on_dead_entity` after damage and uses ID lookup fallbacks | `src/sim/combat/mod.rs`, `src/sim/world/mod.rs`, `src/sim/movement/homing_movement.rs` | Introduce a native-order death/uninit stage: notify dependent references first, then conceal/remove map/display state, then mark not alive / pending delete | A homing bullet targeting a destroyed ground unit retargets/loses target before the target leaves map/display and before alive state becomes false | `uninit_notifies_homing_references_before_conceal_and_alive_clear` | Do not model target loss solely as `entities.get(target_id).is_none()` after removal; that is too late and loses gamemd ordering. |
| `Conceal` removes selection/display/cell/anim/audio and optional logic membership before `Object+0x81=1`, and caller clears `Object+0x90` only afterward. | `0x005F4D30..0x005F4EBD`, `0x005F6625` | Rust often clears `selected`, marks `dying`, or removes occupancy directly in combat/app paths | `src/sim/combat/mod.rs`, `src/sim/world/mod.rs`, render/app selection state | Separate conceal/map-removal state from alive/dead state so selection/display/occupancy cleanup can observe pre-alive-clear object state | Destroying a selected visible unit clears selection and display/occupancy while the object is still in the death cleanup path; later queries see alive false/pending delete | `uninit_conceal_clears_selection_and_occupancy_before_alive_false` | Do not set health/alive false as the first operation if downstream cleanup should still see a live-but-uninitializing object. |
| `FootClass::UnInit` cleans capture manager, chrono deploy relation, and transport/passenger relation before base `ObjectClass::UnInit`; base may recursively uninit passengers via `EMPPassengers(0)`. | `0x004DE5D0..0x004DE611`; `0x00707CB0` | Passenger death/despawn is split across `passenger.rs`, combat cargo handling, and app death animation cleanup | `src/sim/passenger.rs`, `src/sim/combat/mod.rs`, `src/sim/world/world_orders.rs` | Foot-derived death/despawn should detach carried/inside/passenger relations before base reference notifications and before carrier alive clear | Destroying a transport with passenger state processes passenger detach/uninit before carrier reference invalidation and pending-delete append | `foot_uninit_detaches_passengers_before_base_object_uninit` | Do not treat cargo as passive data that disappears only when the carrier entity is removed. |

### Negative Facts / Do Not Do

- Do not call `Conceal` alone and expect bullets/references to be invalidated. Evidence: `Conceal @ 0x005F4D30` has no `Detach_From_All_Lists`; `UnInit @ 0x005F6616` calls it before conceal.
- Do not clear `Object+0x90`/alive before display/cell removal. Evidence: `MOV [ESI+0x90],0` is at `0x005F6625`, after virtual `+0xD4`.
- Do not immediately destruct/free the object inside `ObjectClass::UnInit`. Evidence: function appends to pending-delete vector `0x00B0F69C` and returns.
- Do not make normal target destruction always null in-flight bullet targets. Evidence: BulletClass handler `0x004684E0` can write `MapClass::Get_CellClass` to `+0x10C`; prior bullet invalidation report.
- Do not move `FootClass` passenger/capture/chrono cleanup after base `ObjectClass::UnInit`. Evidence: `FootClass::UnInit @ 0x004DE5D0` performs those calls before `0x004DE60B`.

### Remaining Uncertainty

- Exact pending-delete drain function and same-frame destructor order are outside this slot; this report proves append order only.
- Exact concrete class inventory for `AbstractFlags+0x14 bit 0` and the `FootClass::UnInit` vtable data xrefs was not rederived.
- Native parachute-specific cleanup, if any, did not appear in the scoped base path and needs a separate paradrop/parachute lifecycle trace.
- Full listener-array body census for every `Detach_From_All_Lists` branch was intentionally deferred; bullet placement was verified because it is handoff-critical.

### Stale Docs / Follow-up Docs

- `docs/research/LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md`: replace any wording that frames Rust death/limbo differences as "different mechanism but not wrong" with: "Rust currently differs from gamemd `ObjectClass::UnInit` ordering unless it proves pre-conceal reference invalidation, conceal/display/cell cleanup before alive clear, and deferred pending-delete semantics produce byte/pixel-identical results for the full input space. Under current parity rules this is DRIFT or UNCHECKED, not an internal-only difference."

## Sources

- Ghidra read-only decompile/disassembly:
  - `ObjectClass::UnInit @ 0x005F65F0`
  - `ObjectClass::Conceal @ 0x005F4D30`
  - `Detach_From_All_Lists @ 0x007258D0`
  - `FootClass::UnInit @ 0x004DE5D0`
  - `BombClass::Defuse @ 0x004389B0`
  - `FootClass::EMPPassengers @ 0x00707CB0`
  - `CaptureManagerClass::FreeAll @ 0x00472140`
  - `BuildingClass::DeployUnit_ChronoWarp @ 0x0070FEE0`
  - `FUN_006EA870`
  - `DisplayClass::RemoveFromLayer @ 0x004A9770`
  - `AnimClass::Detach @ 0x00405D40`
  - `VocHandle::Stop @ 0x00405FD0`
  - `DiskLaserClass::DetachFromObject @ 0x004A7900`
  - BulletClass vtable read `0x007E470C -> 0x004684E0`
- Prior reports referenced:
  - `BULLETCLASS_TARGET_INVALIDATION_AAHEATSEEKER2_GHIDRA_REPORT.md`
  - `AAHEATSEEKER2_CELLCLASS_RETARGET_AI_BEHAVIOR_GHIDRA_REPORT.md`
  - `COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES_GHIDRA_REPORT.md`
  - `LOGICCLASS_LIVE_VECTOR_VS_RUST_ENTITY_PASSES_GHIDRA_REPORT.md`
  - `LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md`
  - `VOXELANIMCLASS_GHIDRA_REPORT.md`
  - `TERRAIN_CLASS_GHIDRA_REPORT.md`
- Rust surfaces scanned:
  - `src/sim/combat/mod.rs`
  - `src/sim/world/mod.rs`
  - `src/app_sim_tick.rs`
  - `src/sim/animation.rs`
  - `src/sim/movement/homing_movement.rs`
  - `src/sim/passenger.rs`
  - `src/sim/components.rs`
