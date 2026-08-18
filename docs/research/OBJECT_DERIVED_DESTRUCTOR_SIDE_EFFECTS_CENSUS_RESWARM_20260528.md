# Object-Derived Destructor Side Effects Census - Reswarm 2026-05-28

**Address(es):** pending-delete drain `FUN_00725C70`; scalar-deleting destructor vtable slot `+0x20` for Unit, Infantry, Aircraft, Building, Bullet, Anim, VoxelAnim, Wave, Terrain, BuildingLight  
**Investigation Mode:** coverage-map  
**Claimed Scope:** bounded census of destructor targets and leaf side effects for the named high-priority `ObjectClass`-derived active gameplay classes when reached from the pending-delete drain's virtual `+0x20` call with delete flag `1`.  
**Non-Scope:** every `AbstractClass` descendant in the binary, exact semantic labels for every internal pointer released by base `MissionClass`/`FootClass` destructors, runtime debugger confirmation of rare map-editor or shutdown-only paths, and implementation changes.  
**Confidence:** High for all vtable `+0x20` targets, wrapper/free shapes, and decompiled leaf destructor effects except WaveClass; Medium for WaveClass because Ghidra has no function boundary at `0x00763200`, so evidence is raw assembly plus vtable read.  
**Active in YR:** Yes / Conditional. The pending-delete drain is active in standard YR. Each class is active when objects of that class are created; BuildingLight is conditional on `HasSpotlight=yes`, and WaveClass is stock-live for sonic/magbeam weapons per the prior direct-registration swarm.

## 1. Overview

The pending-delete drain does not reduce object destruction to `operator delete`. For common active gameplay classes, vtable `+0x20` runs class-specific cleanup before storage is freed: class arrays are compacted, listener vectors are pruned, child objects are destructed through their own virtuals, locomotor COM references are released, display/logic membership is re-cleared, sounds are stopped/detached, map cell pointers are cleared, and owner/production state can be touched.

The most important ordering correction is for Building/Unit/Infantry/Aircraft: the drain restores `Object+0x90 = 1` before the scalar destructor, but their leaf destructors later write `Object+0x90 = 0` again before chaining into base destructors. Rust must not treat the drain restore as the final alive state; it is a teardown-visible intermediate.

## 2. Vtable Targets And Key Offsets

| Class | Primary vtable | `+0x20` target | Leaf destructor body | Vtable / wrapper evidence | Active in YR |
|---|---:|---:|---:|---|---|
| `UnitClass` | `0x007F5C70` | `0x00746E80` | `0x00735780` | `read_memory 0x007F5C70`, wrapper decompile `UnitClass__ScalarDelDestructor` | Yes |
| `InfantryClass` | `0x007EB058` | `0x00523350` | `0x00517D90` | `read_memory 0x007EB058`, wrapper decompile `FUN_00523350` | Yes |
| `AircraftClass` | `0x007E22A4` | `0x0041C210` | `0x00414080` | `read_memory 0x007E22A4`, wrapper decompile `FUN_0041C210` | Yes |
| `BuildingClass` | `0x007E3EBC` | `0x00459F20` | `0x0043BCF0` | `read_memory 0x007E3EBC`, wrapper decompile `BuildingClass__ScalarDeletingDestructor` | Yes |
| `BulletClass` | `0x007E46E4` | `0x0046B5C0` | `0x00466560` | `read_memory 0x007E46E4`, wrapper decompile `FUN_0046B5C0` | Yes |
| `AnimClass` | `0x007E3354` | `0x00426590` | `0x004228E0` | `read_memory 0x007E3354`, wrapper decompile `FUN_00426590` | Yes |
| `VoxelAnimClass` | `0x007F6318` | `0x0074AB50` | `0x007499F0` | `read_memory 0x007F6318`, wrapper decompile `FUN_0074AB50` | Yes, conditional on debris paths |
| `WaveClass` | `0x007F6BF4` | `0x00763200` | combined at `0x00763200` | `search_byte_patterns 50 F8 75 00 -> 0x007F6CCC`, base inferred from reveal slot; raw assembly at `0x00763200` | Yes for `IsSonic`/`IsMagBeam` |
| `TerrainClass` | `0x007F522C` | `0x0071D350` | `0x0071B7B0` | `read_memory 0x007F522C`, wrapper decompile `FUN_0071D350` | Yes |
| `BuildingLightClass` | `0x007E3AD0` | `0x004370C0` | combined at `0x004370C0` | `search_byte_patterns 50 70 43 00 -> 0x007E3BA8`, base inferred from reveal slot; decompile `BuildingLightClass__Destructor` | Conditional, `HasSpotlight=yes` |

Common base cleanup reached by several classes:

| Base body | Address | Side effects relevant to Rust |
|---|---:|---|
| `ObjectClass` destructor | `0x005F3B80` | removes from pending-delete vector idempotently, global object array, abstract/listener registries, and live logic/tag listener array; defuses attached bomb; detaches anim/sound/line-trail handles; if `Object+0x98` is still set, calls `FUN_0055BAE0`; chains to `AbstractClass__Destructor_ResetVtables`. |
| `MissionClass` / `TechnoClass` destructor | `0x006F4500` | releases several owned helper pointers by `+0x20`, stops/detaches multiple voc handles, clears owner pointer, removes registry/listener entries, destroys two object pointers through `+0xF8`, frees dynamic vectors, clears radio vector state, then calls `ObjectClass` destructor. |
| `FootClass` destructor | `0x004D3590` | clears temporal/unknown hook if active, removes from foot registries, deletes pointer at `+0x69C` through `+0x20`, clears `CellClass+0xE0` if it points to this foot object, releases sound/voc, releases locomotor COM pointer at `+0x674`, frees two nav queue buffers, then calls `MissionClass` destructor. |

## 3. Core Logic

### 3.1 Wrapper/free shape

For Unit, Infantry, Aircraft, Building, Bullet, Anim, VoxelAnim, and Terrain, the vtable `+0x20` target has the MSVC scalar-deleting shape:

1. Call the class leaf destructor body.
2. Test delete flag bit `1`.
3. If set, call `operator delete @ 0x007C8B3D(this)`.
4. Return `this`.

Evidence: wrappers `0x00746E80`, `0x00523350`, `0x0041C210`, `0x00459F20`, `0x0046B5C0`, `0x00426590`, `0x0074AB50`, `0x0071D350`.

Wave and BuildingLight differ in Ghidra presentation: their vtable entries point directly to a combined destructor/delete body. `BuildingLightClass__Destructor @ 0x004370C0` is decompiled and tests a delete flag from the stack. Wave `0x00763200` is not function-defined in read-only Ghidra; assembly shows the same vtable-reset, class-vector removal, base destructor, delete-flag test, `operator delete`, `RET 4` shape.

### 3.2 Techno-derived class destructors

`UnitClass` leaf destructor `0x00735780`:

- Reinstalls Unit vtables.
- If `Unit+0x6DC` is nonzero, calls `FUN_004C2C10` and clears it.
- When `g_GameActive != 0` and type pointer `Unit+0x6C4` is non-null:
  - `HouseClass::CanBuild(owner)` returning `-1` sets owner byte `+0x1FC = 1`.
  - If `Unit+0x5D4` is nonzero, calls `FUN_006EA870(this, -1, 0)` and clears it.
  - Calls `HouseClass::Remove_Tracking(this)`.
  - In non-map-editor mode, loops while `Unit+0x118` is nonzero, gets objects from `FUN_00473430`, and deletes them through virtual `+0x20(1)`.
  - Calls `FootClass::Limbo`.
  - If rally cell `Unit+0x6CC != -1`, calls vtable `+0x1B8` and `HouseClass::Set_Rally_Point_Cell`, then stores `-1`.
- Calls `Detach_From_All_Lists`.
- Removes self from `g_UnitClass_Array`.
- Removes its RTTI entry from the global sorted tagged-pointer heap `0x00B0E840`.
- Writes `Object+0x90 = 0`.
- Calls `FootClass` destructor `0x004D3590`.

`InfantryClass` leaf destructor `0x00517D90`:

- Reinstalls Infantry vtables.
- When game-active and type pointer `Infantry+0x6C0` is non-null:
  - `HouseClass::CanBuild(owner) == -1` sets owner byte `+0x1FC = 1`.
  - If `Infantry+0x5D4` is nonzero, calls `FUN_006EA870(this, -1, 0)` and clears it.
  - If `Infantry+0x2DC` exists and its `+0x2D8` is nonzero, calls `SlaveManagerClass__RemoveSlave`.
  - Calls `HouseClass::Remove_Tracking`.
  - Asserts if locomotor pointer `+0x674` is null, then calls locomotor vtable `+0xAC`.
  - Resets several infantry sequence/action bytes/fields and calls `FootClass::Limbo`.
- Calls `Detach_From_All_Lists`.
- Removes self from `g_InfantryClass_Array`.
- Removes its RTTI entry from `0x00B0E840`.
- Writes `Object+0x90 = 0`.
- Calls `FootClass` destructor.

`AircraftClass` leaf destructor `0x00414080`:

- Reinstalls Aircraft vtables, including the secondary vtable at `+0x6C0`.
- When game-active and type pointer `Aircraft+0x6C4` is non-null:
  - `HouseClass::CanBuild(owner) == -1` sets owner byte `+0x1FC = 1`.
  - Calls `FootClass__EMPPassengers(0)`.
  - Clears `+0x5D4` through `FUN_006EA870(this, -1, 0)` when nonzero.
  - Calls `HouseClass::Remove_Tracking`.
  - Calls `FootClass::Limbo`.
  - Clears type pointer `Aircraft+0x6C4`.
- Calls `Detach_From_All_Lists`.
- Removes self from `g_AircraftClass_Array`.
- Removes RTTI entry from `0x00B0E840`.
- Writes `Object+0x90 = 0`.
- Calls `FootClass` destructor.

`BuildingClass` leaf destructor `0x0043BCF0`:

- Reinstalls Building vtables.
- Releases two sound events and detaches two voc handles.
- If `Building+0x4E0 LightSource` is non-null, calls `FUN_00554A80(0)`, deletes the light source through virtual `+0x20(1)`, and nulls the pointer.
- Calls `Detach_From_All_Lists`, then `BuildingClass::Limbo`.
- Removes self from `g_BuildingClass_Array` and a secondary building vector.
- Calls `BuildingClass::ClearAnimSlot` and `FUN_00465AF0`.
- Iterates eight pointers at `Building+0x5C8..+0x5E4`; each non-null pointer is destroyed through virtual `+0xF8`, then nulled.
- If game-active, type pointer is non-null, and `BuildingType+0x16C7` is true, writes `Building+0x6EB = 0xFF`, conditionally copies `BuildingType+0x1707` into `+0x6EC`, sets `Object+0x80 = 1`, sets `+0x6EC = 1`, then calls `BuildingClass__UpdateGapGenerator_Tick(1)`.
- Uses `BuildingType+0x16F0/+0x16F4` and house instance counts to decide whether to call `HouseClass__AI_ManageProduction(owner)`.
- If game-active and health differs from `Building+0x544`, sets owner byte `+0x5778 = 1`.
- If game-active and type/owner exist, calls `HouseClass::Remove_Tracking`, then `BuildingClass::Limbo` again.
- If `Building+0x2E4 Factory` is non-null, deletes it through virtual `+0x20(1)` and clears it.
- Removes entries from listener/tagged-pointer heaps at `0x00B0F5B8`, `0x00B0F640`, and `0x00B0E840`.
- Clears `Building+0x34 Type = 0`.
- Writes `Object+0x90 = 0`.
- Destroys two dynamic-vector members at `+0x66C` and `+0x684`, freeing their buffers only when owned flags are set.
- Calls `MissionClass` destructor.

### 3.3 Projectile/effect class destructors

`BulletClass` leaf destructor `0x00466560`:

- Reinstalls Bullet vtables.
- Calls `Detach_From_All_Lists`.
- If byte `Bullet+0x158` is nonzero, removes self from `g_AnimClass_RemoveListeners`.
- If `g_GameActive != 0`, calls `ObjectClass::Conceal`.
- Clears dwords `Bullet+0xAC`, `+0xB0`, and `+0x154`.
- Removes self from `g_BulletClass_Array`.
- Calls `ObjectClass` destructor.

`AnimClass` leaf destructor `0x004228E0`:

- Reinstalls Anim vtables.
- Calls `Detach_From_All_Lists`.
- If game-active and owner pointer `Anim+0xCC` is non-null, scans `g_AnimClass_Array`; when no other anim references the same owner, calls owner vtable `+0x17C` and clears owner byte `+0x84`.
- If type pointer `Anim+0xC8` is non-null and type byte `+0x355` is set, gets the current cell and clears cell flag bit `0x20000`.
- If game-active, calls `ObjectClass::Conceal`; on success, calls `DisplayClass::RemoveFromLayer`.
- Removes self from a global abstract/listener vector `0x00B0F674/0x00B0F680`.
- If type pointer is non-null, calls `FUN_00428DE0`.
- Releases two sound events and detaches two voc handles.
- Clears owner and type pointers.
- Depending on `AbstractClass__IRTTITypeInfo_GetID`, removes self either from a secondary vector `0x00A83E04/0x00A83E10` when ID is `-2`, or from `g_AnimClass_Array`.
- Calls `ObjectClass` destructor.

`VoxelAnimClass` leaf destructor `0x007499F0`:

- Reinstalls VoxelAnim vtables.
- Calls `Detach_From_All_Lists`.
- Removes self from `g_VoxelAnimClass_Array` (`0x0088738C/0x00887398`).
- If pointer `VoxelAnim+0x108` is non-null, destroys it through virtual `+0xF8` and nulls it.
- If `g_GameActive != 0`, calls `ObjectClass::Conceal`.
- Releases two sound events and detaches two voc handles.
- Removes self from global vector `0x00B0F674/0x00B0F680`.
- Clears pointer `VoxelAnim+0x104`.
- Calls `ObjectClass` destructor.

`WaveClass` combined destructor/scalar body `0x00763200`:

- Reinstalls Wave vtables.
- Calls `Detach_From_All_Lists`.
- Removes self from global WaveClass vector `0x00A8EC3C/0x00A8EC48`.
- Destroys the embedded vector/list at `Wave+0x1F0`: calls its vtable `+0x0C`, resets its vtable to `0x007ED480`, and frees its buffer if the owned flag byte at embedded `+0x0D` is set.
- Calls `ObjectClass` destructor.
- Tests delete flag bit `1`, calls `operator delete @ 0x007C8B3D`, then returns with `RET 4`.
- No `ObjectClass::Conceal` call was visible in the `0x00763200` assembly; the destructor relies on detach/base cleanup.

### 3.4 Terrain and BuildingLight destructors

`TerrainClass` leaf destructor `0x0071B7B0`:

- Reinstalls Terrain vtables.
- Calls `Detach_From_All_Lists`.
- Removes self from global TerrainClass vector `0x00A8E98C/0x00A8E998`.
- If `g_GameActive != 0` and type pointer `Terrain+0xC8` is non-null, writes `Object+0x90 = 1` and calls `TerrainClass::Limbo`.
- Removes RTTI entry from tagged-pointer heap `0x00B0E840`.
- Calls `ObjectClass` destructor.

`BuildingLightClass` combined destructor/scalar body `0x004370C0`:

- Reinstalls BuildingLight vtables.
- Calls `ObjectClass::Conceal`.
- If conceal succeeds, explicitly calls `FUN_0055BAE0(this)` again.
- Removes self from BuildingLight global vector `0x008B4194/0x008B41A0`.
- Calls `ObjectClass` destructor.
- Tests delete flag bit `1` and calls `operator delete`.

The extra `FUN_0055BAE0` after a successful `Conceal` is load-bearing: a future Rust `BuildingLightClass` path should not treat spotlight destruction as just deleting render state; it uses object conceal/unregister plumbing plus a class-array compaction.

## 4. INI Keys

No INI key directly gates the destructor dispatch mechanism. Class activation is data-driven:

| Key / data | Effect in this slice | Default / stock activity |
|---|---|---|
| `HasSpotlight=` | Creates `BuildingLightClass`; makes its destructor relevant. | Default false; no stock repo/visible retail assignments found in the prior HasSpotlight report. Conditional for maps/mods. |
| `IsSonic=` / `IsMagBeam=` weapon flags | Create `WaveClass` paths whose destructor is relevant. | Stock-live per `DIRECT_NON_REVEAL_FUN_0055BAA0_CALLERS_RESWARM_20260528.md` / `SET_IN_OPEN_TRANSPORT` context. |
| Debris / voxel anim data | Creates `VoxelAnimClass` objects. | Conditional but common in combat/debris effects. |

## 5. Integration Points

The ordinary active path is:

1. Class-specific AI/death/caller reaches `ObjectClass::UnInit` or equivalent removal.
2. `ObjectClass::UnInit` appends the object to the pending-delete vector.
3. `Main_Tick` later calls `FUN_00725C70` after the live object-vector tick.
4. Drain removes all matching pending entries, calls COM `Release`, optionally restores `Object+0x90 = 1` for Building/Unit/Infantry/Aircraft, then calls virtual `+0x20(1)`.
5. The concrete destructor performs class cleanup and then the scalar wrapper frees storage.

The destructor bodies can call other virtual destructors (`+0x20` or `+0xF8`) recursively. Building deletes light/factory/anim-slot children; Mission destroys two object pointers at `+0x4B/+0x4C`; VoxelAnim destroys `+0x108`; Bullet and Anim unregister from listener arrays. This means Rust's future pending-delete drain needs a class-aware finalization phase, not just generic `EntityStore::remove`.

## 6. Current Rust Implementation Status

| Rust surface | Current behavior | Delta against destructor census |
|---|---|---|
| `src/sim/world/mod.rs:675` `despawn_entity` | Removes occupancy, clears radio contacts, removes `EntityStore` entry, unregisters live object. | Missing class-specific destructor finalizers, child-object destruction, listener/class registry compaction, and late delete queue semantics. |
| `src/sim/entity_store.rs:57` | `EntityStore::remove` directly removes the `GameEntity`. | No virtual `+0x20` equivalent or leaf cleanup ladder. |
| `src/sim/combat/mod.rs:804` / `:975` / `:1003` | Death handler clears targets, marks some entities `dying`, and immediately removes some classes. | No pending-delete drain, no pre-destructor `Object+0x90` restore/intermediate state, no Techno/Foot/Mission destructor side effects. |
| `src/app_sim_tick.rs:298` / `:306` | App removes entities after death animation completion. | Native object free is normally late same `Main_Tick`; death visuals are separate object/effect lifecycles. |
| `src/app_building_anim.rs:769` and `src/app_fire_effects.rs` | Many anim/projectile visuals are app-owned lists. | Native `AnimClass`, `VoxelAnimClass`, `BulletClass`, and `WaveClass` have object destructor side effects and class arrays. |
| `src/sim/components.rs:635` | Some helper comments describe cleanup on `EntityStore.remove`. | Needs future distinction between storage removal and destructor-equivalent side effects. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| UnitClass `+0x20` and leaf destructor | verified | `0x007F5C70 + 0x20 -> 0x00746E80`; decompile `0x00746E80`, `0x00735780` | exact semantic names for `+0x6DC`, `+0x5D4`, `+0x118` helpers |
| InfantryClass `+0x20` and leaf destructor | verified | `0x007EB058 + 0x20 -> 0x00523350`; decompile `0x00523350`, `0x00517D90` | exact semantic label for locomotor `+0xAC` side effect |
| AircraftClass `+0x20` and leaf destructor | verified | `0x007E22A4 + 0x20 -> 0x0041C210`; decompile `0x0041C210`, `0x00414080` | none for bounded destructor side effects |
| BuildingClass `+0x20` and leaf destructor | verified | `0x007E3EBC + 0x20 -> 0x00459F20`; decompile `0x00459F20`, `0x0043BCF0` | exact labels for some type fields `+0x16C7/+0x16F0/+0x16F4` |
| BulletClass `+0x20` and leaf destructor | verified | `0x007E46E4 + 0x20 -> 0x0046B5C0`; decompile `0x0046B5C0`, `0x00466560` | exact semantic name for `Bullet+0x158` listener flag |
| AnimClass `+0x20` and leaf destructor | verified | `0x007E3354 + 0x20 -> 0x00426590`; decompile `0x00426590`, `0x004228E0` | exact semantic name for type byte `+0x355` beyond cell flag clear path |
| VoxelAnimClass `+0x20` and leaf destructor | verified | `0x007F6318 + 0x20 -> 0x0074AB50`; decompile `0x0074AB50`, `0x007499F0` | semantic owner of `+0x108` child pointer |
| WaveClass `+0x20` destructor body | touched-not-exhausted | vtable inferred from `0x007F6CCC -> 0x0075F8B0`; assembly `0x00763200..0x007632CD` | Ghidra has no function boundary; names for embedded vector fields |
| TerrainClass `+0x20` and leaf destructor | verified | `0x007F522C + 0x20 -> 0x0071D350`; decompile `0x0071D350`, `0x0071B7B0` | none for bounded destructor side effects |
| BuildingLightClass `+0x20` and leaf destructor | verified | vtable base `0x007E3AD0`; decompile `0x004370C0`; reveal slot xref `0x007E3BA8` | exact reason for explicit post-Conceal `FUN_0055BAE0` double-remove guard |
| Shared `FootClass` destructor | verified | decompile `0x004D3590` | some pointer semantic labels |
| Shared `MissionClass` destructor | verified | decompile `0x006F4500` | some helper pointer semantic labels |
| Shared `ObjectClass` destructor | verified | decompile `0x005F3B80`; prior pending-delete report | none for bounded generic cleanup |
| Rust destructor-equivalent cleanup | touched-not-exhausted | source scans listed in Section 6 | future implementation contract/design |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-DTOR-001 - Which vtable slot does the pending-delete drain call? -> virtual `+0x20` with delete flag `1`.` (evidence: `PENDING_DELETE_DRAIN_DESTRUCTOR_TIMING_RESWARM_20260528.md`; drain `0x00725D78..0x00725D7A`)
- `[RESOLVED] OQ-DTOR-002 - What is UnitClass `+0x20`? -> `0x00746E80`, wrapper to leaf `0x00735780`, then `operator delete` when flag bit 1 is set.` (evidence: `read_memory 0x007F5C70`; decompiles)
- `[RESOLVED] OQ-DTOR-003 - What is InfantryClass `+0x20`? -> `0x00523350`, wrapper to leaf `0x00517D90`.` (evidence: `read_memory 0x007EB058`; decompiles)
- `[RESOLVED] OQ-DTOR-004 - What is AircraftClass `+0x20`? -> `0x0041C210`, wrapper to leaf `0x00414080`.` (evidence: `read_memory 0x007E22A4`; decompiles)
- `[RESOLVED] OQ-DTOR-005 - What is BuildingClass `+0x20`? -> `0x00459F20`, wrapper to leaf `0x0043BCF0`.` (evidence: `read_memory 0x007E3EBC`; decompiles)
- `[RESOLVED] OQ-DTOR-006 - What is BulletClass `+0x20`? -> `0x0046B5C0`, wrapper to leaf `0x00466560`.` (evidence: `read_memory 0x007E46E4`; decompiles)
- `[RESOLVED] OQ-DTOR-007 - What is AnimClass `+0x20`? -> `0x00426590`, wrapper to leaf `0x004228E0`.` (evidence: `read_memory 0x007E3354`; decompiles)
- `[RESOLVED] OQ-DTOR-008 - What is VoxelAnimClass `+0x20`? -> `0x0074AB50`, wrapper to leaf `0x007499F0`.` (evidence: `read_memory 0x007F6318`; decompiles)
- `[RESOLVED] OQ-DTOR-009 - What is TerrainClass `+0x20`? -> `0x0071D350`, wrapper to leaf `0x0071B7B0`.` (evidence: `read_memory 0x007F522C`; decompiles)
- `[RESOLVED] OQ-DTOR-010 - What is BuildingLightClass `+0x20`? -> `0x004370C0`, combined destructor/delete body.` (evidence: vtable base `0x007E3AD0`; decompile `0x004370C0`)
- `[RESOLVED] OQ-DTOR-011 - What is WaveClass `+0x20`? -> `0x00763200`, combined destructor/delete body; no read-only function boundary exists, but assembly proves vector removal, embedded vector destruction, ObjectClass destructor, and delete flag test.` (evidence: `read_memory 0x007F6BF4`; assembly context `0x00763200..0x007632CD`)
- `[RESOLVED] OQ-DTOR-012 - Do Techno-family destructors leave the drain-restored `Object+0x90` set? -> No; Unit/Infantry/Aircraft/Building leaf destructors write `Object+0x90 = 0` before base teardown.` (evidence: decompiles `0x00735780`, `0x00517D90`, `0x00414080`, `0x0043BCF0`)
- `[RESOLVED] OQ-DTOR-013 - Do destructors recursively destroy child objects? -> Yes; examples include Building light/factory/eight anim-slot pointers, VoxelAnim `+0x108`, Mission `+0x4B/+0x4C`, and Wave embedded vector buffer.` (evidence: decompiles/assembly cited above)
- `[RESOLVED] OQ-DTOR-014 - Does Rust currently have equivalent destructor finalizers? -> No direct class-aware pending-delete finalizer found; current cleanup centers on `despawn_entity`, `EntityStore::remove`, app visual retain lists, and combat death marking.` (evidence: `src/sim/world/mod.rs:675`, `src/sim/entity_store.rs:57`, `src/app_building_anim.rs:769`)
- `[DEFERRED] OQ-DTOR-015 - Exact names for every helper pointer released by `MissionClass` and `FootClass`.` (category: bounded-cost-too-high; reason: this census records side effects needed for finalization ordering, but exact semantic labels require separate Techno/Radio/Locomotor field audits; next-step-if-pursued: focused base-destructor field-label investigation)
- `[DEFERRED] OQ-DTOR-016 - Runtime proof for rare map-editor/shutdown paths.` (category: needs-runtime-debugger; reason: static branches are visible, but stock runtime frequency of each rare flag is outside this destructor census; next-step-if-pursued: debugger trace at destructor entries during skirmish teardown/map load)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Pending-delete `+0x20(1)` runs class leaf cleanup before storage free. | drain `0x00725D78`; wrappers listed in Section 2 | missing: `despawn_entity` removes generic storage | `src/sim/world/mod.rs:675`, future lifecycle finalizer | Add class-aware finalization at the native pending-delete drain point before entity storage removal. | Destroy one object of each modeled class; class registries/listeners/child pointers are cleared before entity disappears. | Do not replace virtual destructor effects with plain `EntityStore::remove`. |
| Unit/Infantry/Aircraft/Building destructor clears `Object+0x90` after the drain temporarily restores it. | `0x00735780`, `0x00517D90`, `0x00414080`, `0x0043BCF0` | missing: Rust has `dying`/alive booleans but no native intermediate restore/clear | `GameEntity` lifecycle state, future pending-delete queue | Model the pre-destructor alive-byte restore as intermediate and let leaf finalizers clear final alive state. | A teardown hook reading alive state before and after leaf destructor sees native transition: drain restore then leaf clear. | Do not assume the drain restore means the entity remains alive after finalization. |
| Building destructor recursively removes lights, factory, eight anim-slot objects, gap/production owner side effects, tracking, vectors, then Mission/Object bases. | `BuildingClass__Destructor @ 0x0043BCF0` | mostly missing/fragmented; building anim overlays are app/component state | `src/app_building_anim.rs`, `src/sim/components.rs`, `src/sim/world/mod.rs` | Building finalization must explicitly finalize building-owned light/anim/factory surfaces in native order. | Destroy a building with active anim slots/factory/light; children finalize once and class arrays/listeners do not retain them. | Do not let orphaned app-layer building anims survive because the entity row was removed. |
| Foot-derived destructors release locomotor COM state, clear `CellClass+0xE0` when it points to the object, free nav queues, and remove tracking/class arrays. | `FootClass__Destructor @ 0x004D3590`; Unit/Infantry/Aircraft leaves | partial: Rust clears movement/occupancy in generic despawn, but not native foot finalizer shape | `src/sim/movement/*`, `src/sim/world/mod.rs`, occupancy/cell-layer state | Foot finalization must release movement/navigation ownership and clear special cell pointers before base object removal. | Destroy a JumpJet/air/ground foot object with queued path and cell pointer; no stale cell/loco/nav queue state remains. | Do not rely only on occupancy removal; native clears more than cell occupation. |
| Bullet destructor unregisters anim-listener state, conceals if active, clears target/source fields, removes `g_BulletClass_Array`, then base object cleanup. | `0x00466560` | missing/unchecked: projectiles are mostly movement/combat/app visuals, not class-array objects | `src/sim/movement/homing_movement.rs`, `src/app_fire_effects.rs`, future projectile object model | Projectile finalization must detach listener/target state at destructor time. | Destroy an in-flight delayed/detonating projectile; target/listener references are cleared before storage free. | Do not leave projectile visuals/listeners as app-only cleanup if they affect object references. |
| Anim/VoxelAnim destructors remove class arrays, clear owner/cell flags, display layers, sounds, child pointers, and base object state. | `0x004228E0`, `0x007499F0` | partial: app-owned flashes/world effects retain/remove visually | `src/app_building_anim.rs`, `src/sim/components.rs`, future AnimClass object runtime | Anim finalization must be an object lifecycle event, not just render retain. | Expiring owner-attached anim clears owner byte/cell flag/list entries and removes display membership exactly once. | Do not equate visual expiry with native object destructor. |
| Wave destructor removes from WaveClass array and frees embedded anim-list vector before ObjectClass destructor. | assembly `0x00763200..0x007632CD` | missing: no explicit WaveClass model found | future wave/beam effect surface | Wave effects need object finalization if implemented as live logic objects. | Sonic/magbeam wave creation/destruction leaves no stale wave-array or embedded anim-list state. | Do not model wave purely as a stateless instantaneous render beam if it is live-registered. |
| BuildingLight destructor conceals/unregisters, removes BuildingLight global vector, then base object cleanup/free. | `0x004370C0` | missing: Rust has point-light ambience only, no BuildingLightClass | future spotlight object/render surface | HasSpotlight path must have a real object lifecycle and destructor cleanup. | Modded `HasSpotlight=yes` building destroyed/unlimboed removes spotlight object and live membership in native order. | Do not merge spotlight cleanup into generic lamp/point-light removal. |

### Stale Docs / Follow-up Docs

- `PENDING_DELETE_DRAIN_DESTRUCTOR_TIMING_RESWARM_20260528.md`: replace "leaf destructor body inventory deferred" with: "Follow-up `OBJECT_DERIVED_DESTRUCTOR_SIDE_EFFECTS_CENSUS_RESWARM_20260528.md` maps Unit/Infantry/Aircraft/Building/Bullet/Anim/VoxelAnim/Wave/Terrain/BuildingLight `+0x20` targets and class-specific side effects. Building/Unit/Infantry/Aircraft destructors clear `Object+0x90` again after the drain's pre-destructor restore."
- Any Rust-facing lifecycle contract should add that `ObjectClass::Destructor @ 0x005F3B80` is not only base memory cleanup: it removes pending-delete/global object/abstract/listener entries, detaches bomb/anim/voc/line-trail state, and can call `FUN_0055BAE0` if `Object+0x98` remains set.

## Sources

- Ghidra read-only memory/vtable reads: `0x007F5C70`, `0x007EB058`, `0x007E22A4`, `0x007E3EBC`, `0x007E46E4`, `0x007E3354`, `0x007F6318`, `0x007F522C`, `0x007E3AD0`, `0x007F6BF4`.
- Ghidra read-only decompiles: `0x00746E80`, `0x00735780`, `0x00523350`, `0x00517D90`, `0x0041C210`, `0x00414080`, `0x00459F20`, `0x0043BCF0`, `0x0046B5C0`, `0x00466560`, `0x00426590`, `0x004228E0`, `0x0074AB50`, `0x007499F0`, `0x0071D350`, `0x0071B7B0`, `0x004370C0`, `0x004D3590`, `0x006F4500`, `0x005F3B80`.
- Ghidra read-only assembly: `0x00763200..0x007632CD` WaveClass combined destructor/scalar body; `search_byte_patterns 50 F8 75 00 -> 0x007F6CCC`; `search_byte_patterns 50 70 43 00 -> 0x007E3BA8`.
- Prior docs referenced: `PENDING_DELETE_DRAIN_DESTRUCTOR_TIMING_RESWARM_20260528.md`, `OBJECTCLASS_UNINIT_DEATH_CLEANUP_ORDERING_RESWARM_20260528.md`, `DIRECT_NON_REVEAL_FUN_0055BAA0_CALLERS_RESWARM_20260528.md`, `BUILDINGLIGHT_HASSPOTLIGHT_REGISTRATION_RESWARM_20260528.md`, `COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES_GHIDRA_REPORT.md`, `ANIMCLASS_GLOBAL_OBJECT_REGISTRATION_LIFETIME_RESWARM_20260527.md`, `VOXELANIMCLASS_GHIDRA_REPORT.md`, `WAVECLASS_GHIDRA_REPORT.md`.
- Rust source scanned: `src/sim/world/mod.rs`, `src/sim/entity_store.rs`, `src/sim/combat/mod.rs`, `src/app_sim_tick.rs`, `src/app_building_anim.rs`, `src/app_fire_effects.rs`, `src/sim/components.rs`, `src/sim/animation.rs`.

Status: COMPLETE for the requested bounded census. WaveClass is covered by read-only assembly rather than decompile because no function boundary exists at `0x00763200` in the current Ghidra project.
