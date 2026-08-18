# Detach_From_All_Lists Listener Roster Census - Re-swarm Research Report

**Address(es):** `Detach_From_All_Lists @ 0x007258D0`, `ObjectClass::Constructor @ 0x005F3900`, `ObjectClass::~ObjectClass @ 0x005F3B80`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** Fresh census of listener vectors/classes reached by the pre-conceal `Detach_From_All_Lists` broadcast for object expiry, with roster sources, dispatch order, vtable slot identity, active class families, and Rust-visible cleanup categories.  
**Non-Scope:** Deep proof of every listener body; Bullet invalidation, aircraft cached dock, and CaptureManager internals beyond roster/table entries; save/load/runtime debugger measurement of exact live vector contents.  
**Confidence:** High for dispatch order, roster source vectors, vtable slot identities, and primary class cleanup families; Medium for exact semantic names inside large House/Techno/Foot field-clearing bodies.  
**Active in YR:** Yes for the object-expiry path. `ObjectClass::UnInit @ 0x005F65F0` calls `Detach_From_All_Lists` before virtual Conceal and before alive clear, and standard Object-derived YR runtime classes are in the roster.

## Working Notes Gate

Target question: What listener vectors/classes are reached by `Detach_From_All_Lists` before object conceal, in what order, from what roster source(s), through what vtable slot(s), and what visible cleanup categories must Rust model?

Non-goals: Do not redo each listener body deeply unless needed to classify side effects; do not re-prove Bullet invalidation, aircraft cached dock, or CaptureManager details beyond table entries.

Evidence needed to mark COMPLETE: decompile plus assembly/disassembly/byte range for `Detach_From_All_Lists`; constructor/destructor evidence for `DAT_00B0F724`; vtable memory/byte evidence for `+0x28` and `+0x2C`; decompile/byte evidence for representative active listener classes; Rust surface scan.

Stop conditions: Stop after roster sources, dispatch ordering, primary object/non-object listener classes, and visible cleanup categories are classified; defer exact runtime slot census and full body labels.

## 1. Overview

`Detach_From_All_Lists` is a removal notification dispatcher, not a bullet-only target invalidator. On object expiry, it clears two global current/UI pointers, forward-iterates `DAT_00B0F724[0..DAT_00B0F730)` and calls every entry's primary vtable `+0x28(expiring, removal_flag)`, then runs fixed post-loop cleanup helpers including spawn-retreat removal, disk-laser detach, tactical callback, and final abstract/list cleanup.

Active in YR: Yes. Evidence: `ObjectClass::UnInit @ 0x005F65F0` calls `0x007258D0` before `vtable+0xD4` Conceal and before `Object+0x90 = 0` per parent context; `Detach_From_All_Lists` decompile and byte/disassembly range `0x007258D0..0x00725ACF`.

## 2. Dispatch Order

| Order | Behavior | Evidence | Active in YR |
|---|---|---|---|
| 1 | Call target `vtable+0x2C` to get RTTI/WhatAmI. | `0x007258D0` decompile: `(**(code **)(*param_1 + 0x2c))()`; RTTI return stubs such as Unit `0x00746E20 -> B8 01`, Infantry `0x00523340 -> B8 0F`, Aircraft `0x0041C180 -> B8 02`, Building `0x00459EC0 -> B8 06`. | Yes |
| 2 | Clear `DAT_0088098C` if it equals expiring pointer. | `0x007258D0` decompile before vector loop. | Yes |
| 3 | Clear `g_UIModeLock @ 0x00880990` if it equals expiring pointer, then call `0x004A8BF0(0)`. | `0x007258D0` decompile before vector loop. | Yes |
| 4 | Special RTTI branches can return before the object-bit branch: `0x0D` House listeners, `0x04` AnimClass listeners, `0x18` singleton clear. | `0x007258D0` decompile; House branch calls `g_HouseClass_RemoveListeners` then `FUN_0055B880` and returns; RTTI `4` iterates `g_AnimClass_RemoveListeners` and returns. | Conditional on expiring RTTI |
| 5 | If `target != null` and `target+0x14` bit 1 is set, iterate `DAT_00B0F724` forward and call listener `vtable+0x28(expiring, removal_flag)`. | `0x0072593E..0x00725957` xrefs/read context; decompile loop uses `iVar2 = 0` then increments while `< DAT_00B0F730`. | Yes for object-registered targets |
| 6 | After the loop call `FUN_00439150`, `SpawnRetreat__Remove`, reverse disk-laser detach loop, optional `FUN_00413490` for RTTI `0x0F/1/2`, `FUN_00733160`, tactical `vtable+0x28(expiring,1)`, then `FUN_0055B880`. | `0x007258D0` decompile; disk-laser body `0x004A7900`; tactical vtable target verified by prior read `vtable__Tactical +0x28 -> 0x006DA560`. | Yes/Conditional per subsystem existence |

Tiny ordering detail: the `DAT_00B0F724` loop reloads `DAT_00B0F730` in the loop condition after each callback. This proves forward index order, but not safe behavior if a callback mutates the same vector. Active in YR: Yes; mutation behavior remains deferred.

## 3. Roster Sources

`DAT_00B0F724` is a DynamicVector-like listener roster with count at `DAT_00B0F730`, capacity at `DAT_00B0F728`, vector methods at `DAT_00B0F720`, and growth step at `DAT_00B0F734`.

| Source | What appends/removes | Evidence | Active in YR |
|---|---|---|---|
| `ObjectClass::Constructor @ 0x005F3900` | Appends `this` to `DAT_00B0F724`, then appends to `DAT_00B0F674`, then `g_TagClass_RemoveListeners`, then sets `Object/Abstract flags +0x14 |= 2`. | Decompile; assembly context `0x005F3A3B..0x005F3A85` reads capacity/count and writes `DAT_00B0F724[count] = ESI`; xrefs show callers from Bullet `0x00466384`, Anim `0x00421EAE`, Terrain `0x0071BB99`, Unit `0x007493BE`, Wave `0x0075E956`, etc. | Yes |
| `ObjectClass::~ObjectClass @ 0x005F3B80` | Removes `this` from `DAT_00B0F724` by vector `+0x10` find and compacting erase. | Decompile; assembly context `0x005F3C48..0x005F3C7C`; destructor also removes object array and abstract/tag listener entries. | Yes |
| `HouseClass::Constructor @ 0x004F5F50` | Appends House to `DAT_00B0F724` in addition to abstract, factory, house-owned, tag, and other listener vectors. | Decompile; assembly context `0x004F5FB1..0x004F5FFB`; vtable `+0x2C` return stub `0x0050E360 -> 0x0D`. | Yes |
| `TeamClass::Constructor @ 0x006E8BF0` | Appends Team to `DAT_00B0F724`, `DAT_00B0F674`, tag listeners, and neuron/team listener vectors. | Decompile; assembly context `0x006E8C43..0x006E8C8D`; vtable `+0x2C` return stub `0x006F0440 -> 0x22`. | Yes |
| `FactoryClass::Constructor @ 0x004C9950` | Appends Factory to `DAT_00B0F724`. | Decompile; assembly context `0x004C99A8..0x004C99F2`; vtable `+0x2C` return stub `0x004CA750 -> 0x0C`. | Yes |
| `AlphaShapeClass` constructors `0x00420980`, `0x00420B00` | Append AlphaShape visual observers to `DAT_00B0F724`. | Decompile; assembly contexts `0x00420A1B..0x00420A68` and `0x00420B7B..0x00420BC5`; vtable `+0x2C` return stub `0x00420D80 -> 0x3E`. | Conditional: active when alpha-shape visuals are constructed |
| `ParticleSystemClass` constructor variants around `0x0062DE50/0x0062DF20` | Append ParticleSystem observers to `DAT_00B0F724` after particle-system array registration. | Decompile/assembly contexts `0x0062DEB1..0x0062DF02` and `0x0062E017..0x0062E067`; vtable `+0x2C` return stub `0x00630210 -> 0x18`. | Conditional: active for particle systems |

Material implication: the object-expiry broadcast reaches more than Object-derived things. Houses, teams, factories, alpha-shape visual handles, and particle systems can be notified when an object expires because they are in the same `DAT_00B0F724` roster. Rust must not model this as "iterate bullets/technos only."

## 4. Vtable Slot Census

Primary vtable offset `+0x28` is the listener callback slot consumed by `Detach_From_All_Lists`; primary vtable offset `+0x2C` is the RTTI/WhatAmI provider that selects early branches. Slot evidence uses raw vtable reads plus decompile/byte stubs.

| Listener class | Vtable base | `+0x28` callback | `+0x2C` RTTI stub | Cleanup family | Active in YR |
|---|---:|---:|---:|---|---|
| `ObjectClass` base/inherited | `0x007E*` family | `0x005F5230` | class-specific | owner chain/count `+0x34`, next chain `+0x30`, pointer `+0x88` clears | Yes for inherited classes |
| `UnitClass` | `0x007F5C70` | `0x007446E0` | `0x00746E20 -> 1` | Foot/Techno cleanup plus `+0x6C8/+0x6C4` clears | Yes |
| `InfantryClass` | `0x007EB058` | `0x0051AA10` | `0x00523340 -> 0x0F` | Foot/Techno cleanup plus `+0x6C0` clear | Yes |
| `AircraftClass` | `0x007E22A4` | `0x0041B660` | `0x0041C180 -> 2` | Foot/Techno cleanup plus `CachedDock +0x6CC` and `+0x6C4` clears | Yes |
| `BuildingClass` | `0x007E3EBC` | `0x0044E8F0` | `0x00459EC0 -> 6` | Techno cleanup, factory/light/anim/type/upgrade/vector clears | Yes |
| `BulletClass` | `0x007E46E4` | `0x004684E0` | `0x0046B550 -> 8` | target invalidation to cell/clear; already covered elsewhere | Yes |
| `AnimClass` | `0x007E3354` | `0x00425150` | `0x00426580 -> 4` | owner/type/attach pointer clears, remove from layer | Yes |
| `VoxelAnimClass` | `0x007F6318` | `0x005F5230` | `0x0074AB20 -> 0x29` | base Object pointer expiry | Conditional: active for voxel debris/anims |
| `TerrainClass` | `0x007F522C` | `0x0071CFD0` | `0x0071D300 -> 0x24` | base Object expiry plus terrain owner/light pointer clear at `+200` | Yes for map terrain |
| `OverlayClass` | `0x007EF3D4` | `0x005F5230` | `0x005FDF50 -> 0x14` | base Object pointer expiry | Yes for overlay objects |
| `BuildingLightClass` | `0x007E3AD0` | `0x00436A00` | `0x004370B0 -> 0x13` | base Object expiry plus target/owner `+0xE0/+0xE4` clears | Conditional: `HasSpotlight=yes`/spotlight object |
| `WaveClass` | `0x007F6BF4` | `0x0075F610` | `0x007631F0 -> 0x2B` | base Object expiry plus two wave endpoint/owner clears (`+0x1D4`, `+0xAC` from byte body) | Conditional: sonic/mag-beam wave effects |
| `SmudgeClass` | `0x007F32FC` | `0x005F5230` | `0x006B4F40 -> 0x1D` | base Object pointer expiry | Yes/Conditional: map smudges |
| `ParticleClass` | `0x007EF954` | `0x005F5230` | class-specific | base Object pointer expiry | Conditional: active when particles exist |
| `ParticleSystemClass` | `0x007EFB9C` | `0x0062FE90` | `0x00630210 -> 0x18` | base Object expiry, vector remove, owner/emitter pointer clears | Conditional: active for particle systems |
| `FactoryClass` | `0x007E88D0` | `0x004CA580` | `0x004CA750 -> 0x0C` | clears factory object pointer `+0x58` | Yes |
| `TeamClass` | `0x007F4730` | `0x006EAE60` | `0x006F0440 -> 0x22` | clears many target/member/team pointers and can relink `+0x54` from expired `+0x5D8` when removal flag set | Yes |
| `HouseClass` | `0x007EA8A0` | `0x004FB9B0` | `0x0050E360 -> 0x0D` | clears owned/selected/production/list refs; body is broad and not deeply classified here | Yes |
| `AlphaShapeClass` | `0x007E32A4` | `0x00420E70` | `0x00420D80 -> 0x3E` | marks shape dirty/dead byte `+0x3C` if owner `+0x24` expires | Conditional |

## 5. Visible Cleanup Categories Rust Must Model

| Category | Verified native behavior | Evidence | Active in YR |
|---|---|---|---|
| Pre-conceal reference invalidation stage | Listener callbacks run before Conceal and alive clear, so callbacks can inspect an object still present/alive/unconcealed. | `ObjectClass::UnInit` parent evidence plus `0x007258D0` dispatch order. | Yes |
| Object/Techno/Foot target and movement refs | `TechnoClass::PointerExpired @ 0x007077C0` clears many object refs, radio/cargo helpers, vectors, CaptureManager victim node, SpawnManager/temporal/chrono relations; `FootClass::PointerExpired @ 0x004D9960` clears transport/nav refs and may replace an object target with a cell pointer. | Decompile `0x007077C0`, `0x004D9960`; vtable reads listed above. | Yes |
| Class-specific techno refs | Unit/Infantry/Aircraft/Building callbacks chain and add extra field clears; aircraft cached dock is in this roster. | Decompile `0x007446E0`, `0x0051AA10`, `0x0041B660`, `0x0044E8F0`; aircraft dock prior report. | Yes |
| Visual/effect owner cleanup | Anim, AlphaShape, BuildingLight, Wave, ParticleSystem, Terrain, VoxelAnim/Overlay/Smudge callbacks clear owners/endpoints or mark visuals for removal/refresh. | Decompile/bytes `0x00425150`, `0x00420E70`, `0x00436A00`, `0x0075F610`, `0x0062FE90`, `0x0071CFD0`. | Yes/Conditional by effect existence |
| Global post-loop effect cleanup | Disk laser source/target expiry sets state `-1` and queues delete; spawn-retreat remove and tactical `+0x28` occur after listener loop. | `0x007258D0`; `DiskLaserClass::DetachFromObject @ 0x004A7900`; tactical call target verified but body deferred. | Yes/Conditional |
| Non-object manager/listener cleanup | House, Team, Factory, AlphaShape, ParticleSystem entries are in `DAT_00B0F724`, so object expiry can clear non-object manager references. | Constructor xrefs and vtable callbacks in Sections 3-4. | Yes/Conditional |

## 6. Current Rust Implementation Status

| Surface | Current shape | Delta |
|---|---|---|
| `src/sim/entity_store.rs::clear_radio_contacts_for` | Clears radio contacts by stable id. | Covers one cleanup family only; not a full `DAT_00B0F724` listener broadcast. |
| `src/sim/world/mod.rs::despawn_entity` / app death flow | Removes entity/occupancy/order state after death handling. | Missing native pre-conceal broadcast while the expiring object is still present and alive. |
| Combat/movement target refs | Targets are scattered across `attack_target`, `movement_target`, capture/cargo/docking fields. | Missing native-order centralized invalidation, including object-to-cell fallback cases. |
| Aircraft docking | Has slot/queue abstractions. | Must keep cached dock pointer-expiry distinct from queue/slot cancellation. |
| Mind control | Simple `mind_controlled` style fields. | Missing CaptureManager-style victim-node removal vs controller FreeAll split. |
| Visual/effect objects | Effects are not all native listener objects. | DiskLaser/Wave/AlphaShape/ParticleSystem/Anim ownership cleanup cannot match until represented as listener-aware lifecycle surfaces. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Detach_From_All_Lists` object branch order | verified | `0x007258D0` decompile; range `0x007258D0..0x00725ACF` | vector mutation behavior during callbacks |
| `DAT_00B0F724` vector layout/count/capacity | verified | xrefs to `0x00B0F720/724/728/730/734`; constructor/destructor contexts | runtime slot contents in a specific mission |
| ObjectClass constructor/destructor source | verified | `0x005F3900`, `0x005F3B80`; contexts `0x005F3A3B..0x005F3A85`, `0x005F3C48..0x005F3C7C` | none |
| Non-object roster sources | verified | House/Team/Factory/AlphaShape/ParticleSystem constructors and vtable reads | exact runtime presence per scenario |
| Vtable `+0x28` identity for primary classes | verified | raw vtable reads in Section 4; decompile/byte bodies | exact labels for some body fields |
| Vtable `+0x2C` RTTI stubs | verified | byte stubs `B8 <rtti> 00 00 00 C3` for listed classes | none for listed classes |
| Listener body cleanup categories | verified/touched | decompile/bytes listed in Section 5 | full field naming for House/Team/Techno/Foot |
| Tactical `+0x28` body | touched-not-exhausted | `0x007258D0` call; prior vtable target `0x006DA560` | function boundary/body decode |
| SpawnRetreat remove body | touched-not-exhausted | `0x007258D0` fixed call placement | exact storage/body semantics |
| Bullet callback body | deferred | prior bullet invalidation report | non-goal |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - What is the target question? -> See Working Notes Gate; roster/order/vtable/classes/categories only.` (evidence: user scope; Active in YR: Yes for object expiry)
- `[RESOLVED] OQ-02 - Which vector is the main pre-conceal object-expiry roster? -> `DAT_00B0F724` with count `DAT_00B0F730`.` (evidence: `0x007258D0`, `0x005F3900`; Active in YR: Yes)
- `[RESOLVED] OQ-03 - What creates the Object-derived roster entries? -> `ObjectClass::Constructor` appends and `ObjectClass::~ObjectClass` removes by compacting erase.` (evidence: `0x005F3900`, `0x005F3B80`; Active in YR: Yes)
- `[RESOLVED] OQ-04 - Are non-object listeners also in the roster? -> Yes: House, Team, Factory, AlphaShape, ParticleSystem constructors append to `DAT_00B0F724`.` (evidence: `0x004F5F50`, `0x006E8BF0`, `0x004C9950`, `0x00420980`, `0x00420B00`, particle constructor contexts; Active in YR: Yes/Conditional)
- `[RESOLVED] OQ-05 - What vtable slot is dispatched? -> Primary vtable `+0x28` with `(expired, removal_flag)` arguments.` (evidence: `0x007258D0` callsites and vtable reads; Active in YR: Yes)
- `[RESOLVED] OQ-06 - What slot selects RTTI branches? -> Primary vtable `+0x2C`; listed classes return constant RTTI stubs.` (evidence: stubs in Section 4; Active in YR: Yes)
- `[RESOLVED] OQ-07 - Is iteration ordered? -> Yes, forward from index 0; later helpers run after the loop.` (evidence: `0x007258D0`; Active in YR: Yes)
- `[RESOLVED] OQ-08 - Which visible cleanup categories are load-bearing? -> pointer refs, radio/contact/cargo, movement/target cell fallback, cached dock, CaptureManager nodes, visual/effect ownership, House/Team/Factory refs, disk laser/spawn/tactical cleanup.` (evidence: Sections 4-5; Active in YR: Yes/Conditional)
- `[RESOLVED] OQ-09 - Are Bullet, aircraft cached dock, and CaptureManager reopened? -> No; only roster entries/table effects are recorded.` (evidence: user non-goals; prior reports; Active in YR: Yes)
- `[DEFERRED] OQ-10 - What exact objects occupy `DAT_00B0F724` in a specific stock mission tick?` (category: `needs-runtime-debugger`; reason: static constructors prove sources, not live per-scenario census; next-step-if-pursued: runtime watch vector appends/removes)
- `[DEFERRED] OQ-11 - What exact body runs at Tactical `+0x28 @ 0x006DA560`?` (category: `bounded-cost-too-high`; reason: dispatch target known but Ghidra has no function boundary in read-only session; next-step-if-pursued: dedicated tactical pointer-expiry body report)
- `[DEFERRED] OQ-12 - What exact body/storage does `SpawnRetreat__Remove` mutate?` (category: `bounded-cost-too-high`; reason: placement/order verified; body not needed for roster census; next-step-if-pursued: spawn-retreat storage census)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Object expiry forward-broadcasts `DAT_00B0F724` listeners through `+0x28` before Conceal/alive clear, then runs fixed post-loop helpers. | `0x007258D0`; `0x005F3900`; `0x005F3B80`; Active in YR: Yes | Missing | `src/sim/world/mod.rs`, `src/sim/entity_store.rs`, death/despawn flow | Add a native-order pre-conceal listener stage separate from entity deletion. | Destroy an object targeted by attacker, team/factory/visual refs, and disk laser; all dependent refs clear before occupancy/display removal. Proposed test: `detach_from_all_lists_broadcasts_forward_before_conceal_and_alive_clear`. | Do not rely on post-remove `get(id).is_none()` cleanup. |
| The roster contains Object-derived classes plus non-object observers such as House, Team, Factory, AlphaShape, and ParticleSystem. | Constructor xrefs in Section 3; Active in YR: Yes/Conditional | Missing | future listener registry / lifecycle surfaces | Listener registry must support manager/effect observers, not just entities or bullets. | Expiring object clears a factory/team/house visual pointer even though the listener is not the expiring entity. Proposed test: `detach_listener_registry_notifies_non_entity_observers`. | Do not model the broadcast as "iterate all combat entities." |
| `FootClass::PointerExpired` can replace an object target with a cell fallback instead of blindly nulling it. | `0x004D9960`; Active in YR: Yes | Missing/scattered | movement, targeting, navigation refs | Preserve native per-field behavior: some object refs clear, some chains relink, some cache a cell. | Foot unit with object destination sees target expire; object pointer clears but cell fallback remains where native stores it. Proposed test: `foot_pointer_expiry_preserves_cell_fallback_for_expired_object_target`. | Do not null every pointer role uniformly. |
| Visual/effect listener classes clear owner/endpoints or mark themselves dirty/dead through the same roster. | `0x00425150`, `0x00420E70`, `0x00436A00`, `0x0075F610`, `0x0062FE90`; Active in YR: Yes/Conditional | Missing for several native effects | render/effects/anim ownership, disk laser/wave/particle systems | Effects with owner/target pointers need listener callbacks and pre-conceal invalidation. | Destroy owner of attached anim/alpha/particle/wave; effect marks/removes in the same expiry broadcast. Proposed test: `effect_owner_expiry_detaches_visual_listener_before_object_conceal`. | Do not make owner-attached visuals poll missing owners later. |
| Post-loop disk-laser/spawn/tactical cleanup is ordered after the roster broadcast. | `0x007258D0`; `0x004A7900`; Active in YR: Yes/Conditional | Missing/partial | future disk laser, spawn manager, tactical selection/render refs | Preserve fixed order: listeners first, then spawn retreat, reverse disk laser detach, tactical callback, final cleanup. | Floating Disc beam and a listener both reference a dying target; listener callback observes pre-disk-laser state, laser delete queues afterward. Proposed test: `disk_laser_detach_runs_after_listener_roster_broadcast`. | Do not interleave disk-laser cleanup inside each listener. |

Concrete proposed Rust test names:

- `detach_from_all_lists_broadcasts_forward_before_conceal_and_alive_clear`
- `detach_listener_registry_notifies_non_entity_observers`
- `foot_pointer_expiry_preserves_cell_fallback_for_expired_object_target`
- `effect_owner_expiry_detaches_visual_listener_before_object_conceal`
- `disk_laser_detach_runs_after_listener_roster_broadcast`

## Negative Facts / Do Not Do

- Do not call `DAT_00B0F724` a bullet-target list. It contains broad listener/observer classes. Active in YR: Yes; evidence: constructor census.
- Do not restrict the broadcast to Object-derived runtime entities. House, Team, Factory, AlphaShape, and ParticleSystem are roster sources. Active in YR: Yes/Conditional.
- Do not clear Rust `alive` or remove storage before the broadcast. Native callback order is pre-Conceal and pre-alive-clear. Active in YR: Yes.
- Do not null every expiring object reference uniformly; some listener bodies relink chains or cache cells. Active in YR: Yes.
- Do not fold disk-laser/spawn/tactical cleanup into the listener loop; native fixed helpers run after the forward roster loop. Active in YR: Yes/Conditional.

## Remaining Uncertainty

- Runtime per-scenario contents and mutation behavior of `DAT_00B0F724` require a debugger/watchpoint pass.
- Tactical `+0x28 @ 0x006DA560` body and `SpawnRetreat__Remove` storage remain body-level follow-ups.
- Exact semantic names for some fields in `HouseClass`, `TeamClass`, `TechnoClass`, and `FootClass` callbacks are not fully assigned here; the cleanup families and ordering are verified.

## Stale Docs / Follow-up Docs

- `docs/research/DETACH_FROM_ALL_LISTS_LISTENER_EFFECTS_RESWARM_20260528.md`: replace any wording implying `DAT_00B0F724` is only object-derived/Unit-Infantry-Aircraft-Building/Bullet/Anim listeners with: "`DAT_00B0F724` is a broad removal-listener roster. `ObjectClass::Constructor` registers Object-derived instances, but HouseClass, TeamClass, FactoryClass, AlphaShapeClass, and ParticleSystemClass constructors also append listener entries. Object expiry dispatches this roster forward through primary vtable `+0x28` before post-loop spawn/disk-laser/tactical cleanup."
- `docs/research/BULLETCLASS_TARGET_INVALIDATION_AAHEATSEEKER2_GHIDRA_REPORT.md`: optional wording addendum after the bullet path: "The same `Detach_From_All_Lists` broadcast is broader than bullets; bullet target invalidation is one roster callback among House/Team/Factory/Object/effect listeners."

## Sources

- Ghidra read-only decompile/bytes: `Detach_From_All_Lists @ 0x007258D0`, `ObjectClass::Constructor @ 0x005F3900`, `ObjectClass::~ObjectClass @ 0x005F3B80`, `TechnoClass::PointerExpired @ 0x007077C0`, `FootClass::PointerExpired @ 0x004D9960`, `UnitClass +0x28 @ 0x007446E0`, `InfantryClass +0x28 @ 0x0051AA10`, `AircraftClass +0x28 @ 0x0041B660`, `BuildingClass +0x28 @ 0x0044E8F0`, `AnimClass +0x28 @ 0x00425150`, `DiskLaserClass::DetachFromObject @ 0x004A7900`, `SpawnManagerClass::PointerExpired @ 0x006B7C60`, `FactoryClass +0x28 @ 0x004CA580`, `AlphaShapeClass +0x28 @ 0x00420E70`, `ParticleSystemClass +0x28 @ 0x0062FE90`, `BuildingLightClass +0x28 @ 0x00436A00`, `TerrainClass +0x28 @ 0x0071CFD0`.
- Raw vtable/RTTI bytes read: Unit `0x007F5C70`, Infantry `0x007EB058`, Aircraft `0x007E22A4`, Building `0x007E3EBC`, Anim `0x007E3354`, Bullet `0x007E46E4`, VoxelAnim `0x007F6318`, Terrain `0x007F522C`, Overlay `0x007EF3D4`, BuildingLight `0x007E3AD0`, Wave `0x007F6BF4`, Smudge `0x007F32FC`, House `0x007EA8A0`, Team `0x007F4730`, Factory `0x007E88D0`, AlphaShape `0x007E32A4`, ParticleSystem `0x007EFB9C`, Particle `0x007EF954`.
- Prior reports used only as cross-checks: `DETACH_FROM_ALL_LISTS_LISTENER_EFFECTS_RESWARM_20260528.md`, `AIRFIELD_RADIO_CACHEDDOCK_CONTACT_LIFETIME_GHIDRA_REPORT.md`, bullet invalidation reports, CaptureManager/mind-control reports, disk laser reports.
- Rust scan: `src/sim/entity_store.rs`, `src/sim/world/mod.rs`, `src/app_sim_tick.rs`, target/movement/docking/mind-control/effect surfaces by `rg`.
