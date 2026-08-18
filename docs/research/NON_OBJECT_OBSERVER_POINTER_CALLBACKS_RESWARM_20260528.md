# Non-Object Observer Pointer Callbacks - Reswarm 2026-05-28

**Target:** `NON_OBJECT_OBSERVER_POINTER_CALLBACKS`
**Address(es):** `Detach_From_All_Lists @ 0x007258D0`, `DAT_00B0F724` / `DAT_00B0F730`, `HouseClass +0x28 @ 0x004FB9B0`, `TeamClass +0x28 @ 0x006EAE60`, `FactoryClass +0x28 @ 0x004CA580`, `AlphaShapeClass +0x28 @ 0x00420E70`, `ParticleSystemClass +0x28 @ 0x0062FE90`
**Investigation mode:** focused reswarm slot
**Confidence:** High for registration into `DAT_00B0F724`, vtable slot addresses, Factory/AlphaShape/ParticleSystem callback bodies, and Rust-facing non-entity registry implication. Medium for exact House/Team field semantics because their `+0x28` bodies lack Ghidra function boundaries in this read-only session and were decoded from raw assembly bytes.

## Target Question

Which non-Object-derived or non-ordinary-entity observers are in the `Detach_From_All_Lists` listener mechanism, what registers them, what does their primary vtable `+0x28` callback do when an object expires, and what does that imply for Rust cleanup?

## Non-Goals

- Do not redo the settled global-roster census except where needed to anchor this slot.
- Do not investigate every `ObjectClass`/`TechnoClass`/`FootClass` listener body.
- Do not investigate runtime per-scenario roster contents or mutation safety while dispatch is in progress.
- Do not edit Rust, INI, claims files, or other research docs.

## Evidence Needed To Mark COMPLETE

- `Detach_From_All_Lists` decompile showing `DAT_00B0F724` dispatch through primary vtable `+0x28`.
- Constructor or registration-path evidence for House, Team, Factory, AlphaShape, and ParticleSystem entries.
- Vtable reads proving each class's `+0x28` callback and `+0x2C` RTTI stub.
- Callback body evidence for each scoped class, with Active in YR labels.
- Rust-facing handoff showing cleanup cannot be entity-only.

## Stop Conditions

- Stop after the scoped non-object observers and callback bodies are classified.
- Stop before broad Object/Techno/Foot callback expansion.
- Stop if Ghidra requires mutating function creation to decompile a missing callback boundary; use raw bytes/disassembly and mark uncertainty instead.

## 1. Dispatch Anchor

Verified binary finding. `Detach_From_All_Lists @ 0x007258D0` first calls the expiring target's primary vtable `+0x2C`, clears current/UI globals, handles special RTTI branches, then for object-registered targets checks `target+0x14` bit 1 and forward-iterates `DAT_00B0F724[0..DAT_00B0F730)`. Each listener is invoked through listener primary vtable `+0x28(expired, removal_flag)`. Post-loop helpers run afterward.

Active in YR: Yes. Evidence: `ObjectClass::UnInit @ 0x005F65F0` calls `Detach_From_All_Lists` on the standard object removal path before Conceal/alive-clear per existing lifecycle reports; `Detach_From_All_Lists @ 0x007258D0` decompile directly shows the object-bit branch and `DAT_00B0F724` callback loop.

This report does not claim every `DAT_00B0F724` entry is non-object. It confirms that the roster is not entity-only: ordinary object-derived listeners and several non-object manager/effect observers share the same broadcast.

## 2. Scoped Non-Object / Manager Observer Entries

| Class | Registration path | Vtable evidence | Callback body | Active in YR |
|---|---|---|---|---|
| `HouseClass` | `HouseClass::Constructor @ 0x004F54A0` appends `this` to `DAT_00B0F724` after abstract/factory/tag-style listener vectors and before `g_HouseClass_Array`. | `vtable__HouseClass 0x007EA8A0`: `+0x28 -> 0x004FB9B0`; `+0x2C -> 0x0050E360`; RTTI bytes `B8 0D 00 00 00 C3`. | Clears/removes expired object references from House-owned vectors and factory/building pointer slots; exact labels below. | Yes. Houses are created by `ScenarioClass::Create_Houses @ 0x00687F10` and standard scenario init callers. |
| `TeamClass` | `TeamClass::Constructor @ 0x006E8A90` appends to `g_TeamClass_Array`, `DAT_00B0F674`, tag listener vector, `DAT_00B0F724`, then neuron/team listener vector. | `vtable__TeamClass 0x007F4730`: `+0x28 -> 0x006EAE60`; `+0x2C -> 0x006F0440`; RTTI bytes `B8 22 00 00 00 C3`. | Clears many team object pointers; if removal flag is nonzero and expired pointer equals `Team+0x54`, relinks `Team+0x54` from expired `+0x5D8`. | Yes. Constructor has active callers including `TeamClass::Recruit_Or_Add @ 0x006E9380` and scenario/team creation helpers. |
| `FactoryClass` | `FactoryClass::Constructor @ 0x004C98B0` appends to `g_FactoryClass_Array`, then appends `this` to `DAT_00B0F724`. | `vtable_FactoryClass 0x007E88D0`: `+0x28 -> 0x004CA580`; `+0x2C -> 0x004CA750`; RTTI bytes `B8 0C 00 00 00 C3`. | If expired pointer equals `Factory+0x58`, clears `Factory+0x58`. | Yes. Constructor callers include `HouseClass::Begin_Production @ 0x004FA350`; factories are standard production runtime objects. |
| `AlphaShapeClass` | Constructors `0x00420960` and `0x00420AF0` append to alpha-shape array `DAT_0088A0F4/DAT_0088A100`, then append `this` to `DAT_00B0F724`. | `vtable__AlphaShapeClass 0x007E32A4`: `+0x28 -> 0x00420E70`; `+0x2C -> 0x00420D80`; RTTI bytes `B8 3E 00 00 00 C3`. | If expired pointer equals `AlphaShape+0x24` source object, writes disabled byte `AlphaShape+0x3C = 1`. | Conditional. Constructor `0x00420960` is called from `ObjectClass::Reveal @ 0x005F4EC0` when alpha/fog ghost creation conditions pass. |
| `ParticleSystemClass` | Constructors `0x0062DC50` and `0x0062DF20` call `ObjectClass::Constructor`, register in particle-system array `DAT_00A8020C/DAT_00A80218`, then append to `DAT_00B0F724`. | `vtable__ParticleSystemClass 0x007EFB9C`: `+0x28 -> 0x0062FE90`; `+0x2C -> 0x00630210`; RTTI bytes `B8 18 00 00 00 C3`. | Chains base object pointer-expiry, removes expired pointer from internal vector `+0xC0/+0xCC`, clears type/owner/emitter-like refs at `+0xAC`, `+0xE4`, `+0xE0`, and sets `+0xF8 = 1` for the `+0xE0` match. | Conditional. Constructor `0x0062DC50` has active stock callers from area damage, destruction effects, gap generator, `TechnoClass::FireAt`, `TechnoClass::AI_Update`, `UnitClass::AI`, voxel anims, and warp attach. |

## 3. Callback Body Notes

### HouseClass `+0x28 @ 0x004FB9B0`

Verified binary finding from vtable read plus raw byte disassembly. Ghidra had no function boundary at `0x004FB9B0`, so this callback was not decompiled in this read-only session.

Active in YR: Yes. The vtable is installed by `HouseClass::Constructor @ 0x004F54A0`, House objects are created from standard scenario setup, and `HouseClass::Constructor` appends House to `DAT_00B0F724`.

Observed behavior:

- If expired pointer equals `House+0x54E0`, clear `House+0x54E0` (`0x004FB9BB..0x004FB9C9`).
- Calls RTTI/cast helper `0x007CAAE4` against the expired pointer; if it returns non-null, removes that pointer from vector-like storage at `House+0x38` by find-index plus left-compaction (`0x004FB9CA..0x004FBA1D`).
- Calls expired `vtable+0x2C`; only if RTTI is `6` (BuildingClass) and removal flag is nonzero does it remove the expired building from a series of House vectors at offsets including `+0x50`, `+0x80`, `+0x98`, `+0xB0`, `+0xC8`, `+0xE0`, `+0xF8`, `+0x110`, `+0x128`, `+0x140`, and `+0x68`, with a `HouseClass::RecalcBonuses @ 0x0050BF60` call after these building-list removals (`0x004FBA1D..0x004FBC58`).
- Clears matching pointers in a 12-entry block starting at `House+0x210` and in production/factory pointer slots `House+0x53AC..0x53CC` (`0x004FBC5D..0x004FBCF6`).

Rust-facing inference: House cleanup has both broad abstract/reference cleanup and building/factory production-list cleanup. This is not equivalent to removing an entity from `EntityStore` and later scanning only live entities.

### TeamClass `+0x28 @ 0x006EAE60`

Verified binary finding from vtable read plus raw byte disassembly. Ghidra had no function boundary at `0x006EAE60`, so this callback was not decompiled in this read-only session.

Active in YR: Yes. `TeamClass::Constructor @ 0x006E8A90` registers Team objects into `DAT_00B0F724`, and Team construction/AI paths are standard YR.

Observed behavior:

- Clears `Team+0x70` if it equals the expired pointer (`0x006EAE64..0x006EAE71`).
- If expired pointer equals `Team+0x54` and removal flag is nonzero, replaces `Team+0x54` with `expired+0x5D8` instead of blindly nulling it (`0x006EAE71..0x006EAE87`).
- Clears matching object pointers at `Team+0x2C`, `+0x28`, `+0x40`, `+0x3C`, `+0x34`, `+0x24`, `+0x38`, `+0x44`, and `+0x30`, then returns with `RET 8` (`0x006EAE87..0x006EAED0`).

Rust-facing inference: Team cleanup is role-specific pointer invalidation and one relink/fallback path, not uniform `Option::None` behavior.

### FactoryClass `+0x28 @ 0x004CA580`

Verified binary finding from decompile.

Active in YR: Yes. Factories are created by production paths and appended to `DAT_00B0F724`.

Body:

```text
if expired == Factory+0x58:
    Factory+0x58 = 0
```

Evidence: `FactoryClass__vtable_10 @ 0x004CA580` decompile; vtable read `0x007E88D0 + 0x28 -> 0x004CA580`; constructor `0x004C98B0` appends to `DAT_00B0F724`.

### AlphaShapeClass `+0x28 @ 0x00420E70`

Verified binary finding from decompile.

Active in YR: Conditional. Active when alpha-shape fog ghost visuals are constructed from `ObjectClass::Reveal @ 0x005F4EC0`.

Body:

```text
if expired == AlphaShape+0x24 source_object:
    AlphaShape+0x3C = 1
```

The later `AlphaShapeClass::PurgeDisabled @ 0x00420E90` destroys disabled shapes during the per-tick cleanup pass.

Evidence: `AlphaShapeClass__Notification @ 0x00420E70` decompile; vtable read `0x007E32A4 + 0x28 -> 0x00420E70`; constructors `0x00420960` and `0x00420AF0` append to `DAT_00B0F724`; caller `ObjectClass::Reveal @ 0x005F4EC0`.

### ParticleSystemClass `+0x28 @ 0x0062FE90`

Verified binary finding from decompile.

Active in YR: Conditional. Particle systems are stock-live effects when spawned by weapons, damage, building destruction/gap generators, voxel anims, and related paths.

Body summary:

- Chains the base Object pointer-expiry body.
- Finds expired pointer in an internal vector whose vtable/find method is at `ParticleSystem+0xBC`, data at `+0xC0`, count at `+0xCC`; if found and in range, decrements count and left-compacts.
- Clears `ParticleSystem+0xAC` if it equals expired.
- Clears `ParticleSystem+0xE4` if it equals expired.
- If `ParticleSystem+0xE0` equals expired, writes `ParticleSystem+0xF8 = 1` and clears `+0xE0`.

Evidence: `FUN_0062FE90` decompile; vtable read `0x007EFB9C + 0x28 -> 0x0062FE90`; constructors `0x0062DC50` and `0x0062DF20` append to `DAT_00B0F724`; caller list for `0x0062DC50` includes stock weapon/damage/effect paths.

## 4. Active YR Status Summary

- House observer callback: Active in YR: Yes. Standard scenario houses are constructed and registered.
- Team observer callback: Active in YR: Yes. Team objects are constructed by standard team/AI paths and registered.
- Factory observer callback: Active in YR: Yes. Production creates factories and registers them.
- AlphaShape observer callback: Active in YR: Conditional. Requires alpha/fog ghost shape construction from object reveal/fog state.
- ParticleSystem observer callback: Active in YR: Conditional. Requires a spawned particle system; stock weapons/effects do spawn them.
- Object-expiry `DAT_00B0F724` dispatch: Active in YR: Yes for object-registered target removal.

## 5. Implementation Handoff

| Verified behavior | Evidence | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|---|
| `DAT_00B0F724` dispatch notifies non-entity manager/effect observers through `+0x28` before post-loop helpers. | `Detach_From_All_Lists @ 0x007258D0` decompile; House/Team/Factory/AlphaShape/ParticleSystem constructor/vtable evidence above. | Rust cleanup cannot be implemented as entity-only target scans or post-remove missing-ID polling. | `src/sim/world/mod.rs`, future listener registry, production/team/effect lifecycles. | Destroy one object referenced by a factory, team, house building list, alpha shape, and particle system; every registered non-entity observer receives pre-remove cleanup. | `detach_listener_registry_notifies_non_entity_observers_before_entity_removal` | High if native death/despawn is implemented without manager/effect callbacks. |
| Factory cleanup clears current production object pointer `+0x58` when that object expires. | `FactoryClass +0x28 @ 0x004CA580` decompile plus constructor `0x004C98B0`. | Production state must subscribe to object expiry, not only validate at completion time. | `src/sim/production/*`, house production state. | Produced/queued object expires or is invalidated before placement/completion; factory current object handle clears in the same detach broadcast. | `factory_pointer_expiry_clears_current_object_handle` | Medium; stale production handles affect sidebar/placement readiness. |
| Team cleanup clears many object roles and can relink `Team+0x54` from `expired+0x5D8` when removal flag is set. | Team vtable read and disassembly `0x006EAE60..0x006EAED0`; constructor `0x006E8A90`. | Future TeamClass runtime cannot treat every expired object role as uniform nulling. | future `TeamClass`/AI script runtime. | Team target/member pointer expires with removal flag; `+0x54` equivalent follows native fallback while other role pointers clear. | `team_pointer_expiry_preserves_native_relink_for_role_54` | Medium; exact role names need a dedicated Team report before implementation. |
| AlphaShape and ParticleSystem visual/effect observers clean themselves through the same object-expiry broadcast. | `AlphaShape +0x28 @ 0x00420E70`; `ParticleSystem +0x28 @ 0x0062FE90`; constructor/caller evidence. | Effects need listener-aware owner invalidation rather than polling for missing owners after render/sim storage removal. | render/effects/particles/alpha-shape lifecycles. | Owner object expires; alpha shape marks disabled and particle system clears owner/emitter refs before the owner is concealed/deleted. | `effect_observers_handle_owner_expiry_during_detach_broadcast` | High for visual/effect parity once these systems exist. |

## 6. Negative Facts / Do Not Do

- Do not restrict `Detach_From_All_Lists` cleanup to Object-derived `GameEntity` records. Active in YR: Yes/Conditional; House, Team, Factory, AlphaShape, and ParticleSystem register in `DAT_00B0F724`.
- Do not treat `DAT_00B0F724` as a bullet-target list or combat-only target list. Active in YR: Yes; it is the broad object-expiry listener roster.
- Do not null all observer references uniformly. Active in YR: Yes; Team `+0x54` can be relinked from `expired+0x5D8`, House removes from vectors and clears factory slots, ParticleSystem marks one path dirty at `+0xF8`.
- Do not make AlphaShape owner expiry immediately free the shape. Active in YR: Conditional; callback marks `+0x3C = 1`, and purge later destroys disabled shapes.
- Do not fold Factory cleanup into House-only production scanning. Active in YR: Yes; Factory is its own registered observer and clears `Factory+0x58` directly.

## 7. Remaining Uncertainty

- Ghidra had no function boundary for `HouseClass +0x28 @ 0x004FB9B0` or `TeamClass +0x28 @ 0x006EAE60`; body claims for those two are assembly/byte-decoded, not decompiler output.
- Exact semantic names for House vectors at `+0x50/+0x80/+0x98/+0xB0/+0xC8/+0xE0/+0xF8/+0x110/+0x128/+0x140/+0x68`, the 12 slots at `+0x210`, and Team fields remain unresolved.
- Runtime mutation behavior if a listener edits `DAT_00B0F724` during dispatch remains unresolved.
- Exact stock-map frequency of AlphaShape entries depends on fog/shroud conditions and alpha-image availability.

## 8. Stale Docs / Follow-Up Wording

No new stale-doc wording beyond the existing roster-census replacement was found. The existing wording remains correct:

> `DAT_00B0F724` is a broad removal-listener roster. `ObjectClass::Constructor` registers Object-derived instances, but HouseClass, TeamClass, FactoryClass, AlphaShapeClass, and ParticleSystemClass constructors also append listener entries. Object expiry dispatches this roster forward through primary vtable `+0x28` before post-loop spawn/disk-laser/tactical cleanup.

## Sources

- Ghidra read-only decompile: `Detach_From_All_Lists @ 0x007258D0`; `HouseClass::Constructor @ 0x004F54A0`; `TeamClass::Constructor @ 0x006E8A90`; `FactoryClass::Constructor @ 0x004C98B0`; `AlphaShapeClass::Constructor @ 0x00420960`; `AlphaShapeClass::Constructor @ 0x00420AF0`; `ParticleSystemClass::Constructor @ 0x0062DC50`; `ParticleSystemClass::Constructor @ 0x0062DF20`; `FactoryClass +0x28 @ 0x004CA580`; `AlphaShapeClass +0x28 @ 0x00420E70`; `ParticleSystemClass +0x28 @ 0x0062FE90`.
- Ghidra read-only vtable/byte reads: `House 0x007EA8A0`, `Team 0x007F4730`, `Factory 0x007E88D0`, `AlphaShape 0x007E32A4`, `ParticleSystem 0x007EFB9C`; RTTI stubs `0x0050E360`, `0x006F0440`, `0x004CA750`, `0x00420D80`, `0x00630210`.
- Ghidra read-only caller evidence: `ObjectClass::Reveal -> AlphaShapeClass::Constructor`; particle constructor callers including `Apply_area_damage`, `BuildingClass::DestructionEffects`, `TechnoClassFireAtSpawnsBullet`, `TechnoClass::AI_Update`, `UnitClass::AI`, `VoxelAnimClass::Constructor`, `WarpAttachClass::UpdateAttack`; House/Team/Factory constructor callers listed in the body.
- Raw byte disassembly decoded from Ghidra `read_memory`: `HouseClass +0x28` ranges `0x004FB9B0..0x004FBCF6`; `TeamClass +0x28` range `0x006EAE60..0x006EAED0`.
- Existing cross-check docs: `DETACH_FROM_ALL_LISTS_LISTENER_ROSTER_CENSUS_RESWARM_20260528.md`, `DETACH_FROM_ALL_LISTS_LISTENER_EFFECTS_RESWARM_20260528.md`, `OBJECT_LOGIC_LIFECYCLE_ACTIVE_MEMBERSHIP_SYSTEM_MODEL_SYNTHESIS.md`, `ALPHA_SHAPE_CLASS_LIFECYCLE.md`, `PERTICKUPDATE_NON_OBJECT_GLOBAL_LOOPS_GHIDRA_REPORT.md`.
