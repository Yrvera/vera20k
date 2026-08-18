# LightSource Post-Load Rehydration Owner Reswarm

Date: 2026-05-28

Status: COMPLETE for the bounded static-code slice. Negative for a general post-load rehydration owner.

Investigation mode: exhaustive-slice.

## Scope

Single target: after `BuildingClass::Load @ 0x00453E20` zeroes `BuildingClass+0x614`, identify the exact outer caller/path that recreates or forces rehydration of the building `LightSourceClass*`, if any.

Covered only as needed:

- `BuildingClass::Load` treatment of `+0x614`.
- Building `LightSourceClass` allocation callers.
- `BuildingClass::OnConstructionComplete` gates and known virtual-call sites.
- Standard YR construction-complete, save/load, online/offline, damage/sell/destructor activity relevant to whether `+0x614` can come back.
- Rust-facing handoff for snapshot/load lighting rebuild.

Non-scope:

- Radiation and particle lights except as constructor-caller discriminators.
- `BuildingLightClass`/spotlights at `BuildingClass+0x600` except to distinguish pointer-fixup behavior.
- Radius/falloff math and LightConvert palette internals.

## Executive Result

No exact general post-load rehydration caller was found. The static binary evidence supports this narrower rule:

`BuildingClass+0x614` is a runtime cache pointer. `BuildingClass::Load` explicitly zeroes it after normal pointer fixups. The only verified building allocation sites for that pointer are:

- `BuildingClass::Unlimbo @ 0x00440580`.
- `BuildingClass::OnConstructionComplete @ 0x00445F80`.

Save/load does not call `Unlimbo`, does not directly call `OnConstructionComplete`, does not pointer-fix `+0x614`, and no scanned virtual `+0x4DC` caller is a general post-load pass over loaded buildings. Earlier wording that `+0x614` is "recreated on-demand during the first post-load tick" remains unverified and should not be used as an implementation mechanism.

The Rust-safe handoff is explicit rebuild of render/app transient building light state after snapshot load from stable building type, position, alive/on-map state, and power/online state. Do not serialize a native-like light handle, and do not wait for a copied lazy first-tick hook unless a later runtime trace proves one.

## Open Questions Log

- `[RESOLVED] LSPL-001 - Does BuildingClass::Load recreate or schedule +0x614?` No. It calls the base load/fixup machinery, registers other pointer slots, then unconditionally writes zero to `+0x614`.
- `[RESOLVED] LSPL-002 - Are there building LightSourceClass constructor callers besides Unlimbo and OnConstructionComplete?` No in Ghidra caller census. Third caller is `RadSiteClass::Activate`, not building ownership.
- `[RESOLVED] LSPL-003 - Can OnConstructionComplete's force argument alone rehydrate an already placed loaded building?` No. The light allocation block is guarded by `ActuallyPlacedOnMap == false`; force skips the top return only.
- `[RESOLVED] LSPL-004 - Does BuildingClass::Update lazily allocate +0x614 on first post-load tick?` No constructor callee and no `+0x4DC` virtual call in the checked `BuildingClass::Update` body/callee list.
- `[RESOLVED] LSPL-005 - Is there a static direct call to BuildingClass::OnConstructionComplete?` No direct `CALL 0x00445F80` hit in the local PE byte scan; Ghidra xrefs show the vtable data reference.
- `[RESOLVED] LSPL-006 - Is any scanned `CALL [reg+0x4DC]` a save/load rehydrate driver?` No. Found sites are construction, change-owner/discovery, fresh scenario placement, or non-building boundary/relocate checks.
- `[DEFERRED] LSPL-007 - What exact visual result does stock YR show after loading a save containing already placed lamp buildings?` Requires runtime/debugger or controlled in-game observation. Static code did not find a general allocator owner, but this slot did not run a live game experiment.

## Verified Binary Findings

### 1. Load zeroes `BuildingClass+0x614` and does not pointer-fix it

Active in YR: Yes, for saved-game object restore through `BuildingClass::Load @ 0x00453E20`.

Evidence:

- Decompile of `BuildingClass__Load @ 0x00453E20`.
- Assembly context around `0x00454160..0x00454183`.

Details:

- `BuildingClass::Load` calls the inherited load path, then reruns `BuildingClass::Constructor` on the already-loaded memory before restoring/registering class-specific pointers.
- It registers pointer slots such as `+0x520`-adjacent anim/upgrade fields, `+0x600` (`param_1 + 0x180`), `+0x6F4` (`param_1 + 0x1BD`), and arrays in the `+0x55C..+0x6E8` region through `FUN_006CF240(&DAT_00B0C110, slot)`.
- It does not register `param_1 + 0x185`.
- Assembly shows the final write:
  - `0x00454165: CALL 0x006CF240`
  - `0x00454170: MOV EAX,dword ptr [ESP + 0x10]`
  - `0x00454174: MOV dword ptr [EDI + 0x614],0x0`
  - epilogue follows immediately.
- The zero is unconditional. It does not test `LightIntensity`, saved pointer value, owner power state, map mode, or whether the building is already placed.
- No dirty-cell recompute or light disable wrapper is called during this zero. The stale pointer is discarded as cache state, not logically destroyed through the light lifecycle.

Why it matters:

Rust must not model a building light runtime handle as durable save identity. Native load deliberately discards it.

### 2. Building LightSource allocation has only two building owners

Active in YR: Yes for building placement/completion; radiation caller is separate.

Evidence:

- Ghidra `get_function_callers(0x00554760)` returned:
  - `BuildingClass__OnConstructionComplete @ 0x00445F80`.
  - `BuildingClass__Unlimbo @ 0x00440580`.
  - `RadSiteClass__Activate @ 0x0065B580`.
- Assembly contexts for the two building stores.

Details:

- `BuildingClass::Unlimbo` stores the constructor result:
  - `0x00440DE6: CALL 0x00554760`
  - `0x00440DEF: MOV dword ptr [ESI + 0x614],EAX`
  - `0x00440DF5: MOV ECX,dword ptr [ESI + 0x614]`
  - `0x00440DFB: PUSH 0x0`
  - `0x00440DFD: CALL 0x00554A60`
- `BuildingClass::OnConstructionComplete` stores the constructor result:
  - `0x00446750: CALL 0x00554760`
  - `0x00446759: MOV dword ptr [EBP + 0x614],EAX`
  - `0x0044675F: MOV ECX,dword ptr [EBP + 0x614]`
  - `0x00446765: PUSH 0x0`
  - `0x00446767: CALL 0x00554A60`
- Both building paths enable immediately with mode `0`.
- Both building paths use the building type light fields `+0xE30/+0xE34/+0xE38/+0xE3C/+0xE40`.
- Allocation is gated by `Type+0xE34 != 0` (`LightIntensity`), not by `LightVisibility` alone.
- `RadSiteClass::Activate` is a separate light owner and does not write `BuildingClass+0x614`.

Why it matters:

Any true post-load building rehydration path must call one of the two building allocation owners or contain a third `LightSourceClass` constructor call. The checked binary graph has no third building owner.

### 3. `OnConstructionComplete` force does not rehydrate already placed buildings

Active in YR: Yes.

Evidence:

- Decompile of `BuildingClass__OnConstructionComplete @ 0x00445F80`.
- Assembly context:
  - `0x00445F89: MOV AL,byte ptr [EBP + 0x6E4]`
  - `0x00445F8F: TEST AL,AL`
  - `0x00445F91: JZ 0x00445F9F`
  - `0x00445F93: MOV CL,byte ptr [ESP + 0x6C]`
  - `0x00445F97: TEST CL,CL`
  - `0x00445F99: JZ 0x00446FB6`

Details:

- If `ActuallyPlacedOnMap` (`+0x6E4`) is nonzero and the argument is `0`, the function returns at the top.
- If `ActuallyPlacedOnMap` is nonzero and the argument is `1`, the top return is skipped, but the main one-shot body still has `if (ActuallyPlacedOnMap == false)`.
- The `+0x614` allocation block is inside that `ActuallyPlacedOnMap == false` body.
- Tail writes include `Owner+0x1FC = 1`, `ActuallyPlacedOnMap = true`, and owner dirty bytes, so the normal body flips the fence after the one-shot work.

Why it matters:

`OnConstructionComplete(1)` is not a magic cache-rebuild switch. It can run some forced side effects for already placed buildings, but it does not recreate a missing `+0x614` when `+0x6E4` is already true.

### 4. Standard construction completion reaches the allocator, but only for construction completion

Active in YR: Yes.

Evidence:

- Decompile of `Mission_Construction @ 0x00449A50`.
- Assembly context:
  - `0x00449AC5: PUSH 0x1`
  - `0x00449AC9: CALL 0x00447780`
  - `0x00449AD0: PUSH 0x0`
  - `0x00449AD2: MOV ECX,ESI`
  - `0x00449AD4: CALL dword ptr [EDX + 0x4DC]`
  - then `Queue_Mission(5,0)` via vtable `+0x1E8`.

Details:

- The caller is the mission-0 construction flow, not the load-game flow.
- It calls the vtable slot with argument `0`.
- It is gated by construction mission state and construction-completion byte logic, not by save/load.
- Because it is an ordinary virtual slot call, it resolves to `BuildingClass::OnConstructionComplete` only when the receiver is a building.

Why it matters:

Freshly built buildings can allocate/enable `+0x614` through normal construction completion. This does not explain already placed buildings restored from save.

### 5. Static `+0x4DC` virtual-call census finds no save/load rehydrate pass

Active in YR: Yes for the listed code sites; no save/load owner verified.

Method:

- Local PE byte scan of `<ra2-install>/gamemd.exe`.
- Scanned executable sections for `FF 90..97 DC 04 00 00` (`CALL [reg+0x4DC]`).
- Scanned for direct `E8 rel32` calls targeting `0x00445F80`.
- Ghidra assembly/decompile was used to classify the found call sites.

Results:

| Address | Context | Argument / receiver | Rehydration relevance |
|---|---|---|---|
| `0x00449AD4` | `Mission_Construction` | pushes `0` on building receiver | Fresh construction completion only |
| `0x00448CEF` | `BuildingClass` change-owner/capture region | pushes `1` | Not save/load; does not allocate if `+0x6E4` already true |
| `0x0044D68A` | building discovery hook | pushes `0` | Discovery side effect; early-outs for already placed buildings with arg `0` |
| `0x006E42BB` | scenario/map placement helper | pushes `0` after placement-like slot `+0x490` succeeds | Fresh object placement, not `BuildingClass::Load` |
| `0x00414F85` | aircraft/boundary check | receiver from aircraft path | Non-building vtable slot use |
| `0x00414FC3` | aircraft/boundary check | receiver from aircraft path | Non-building vtable slot use |
| `0x004CD050` | `FlyLocomotionClass::Emergency_Relocate` | receiver `param_1[2]` | Non-save/load boundary/relocate check |
| `0x004CDB9F` | `FlyLocomotionClass::Process` | receiver `param_1[0xC]` | Non-save/load movement relocation |
| `0x004D8D8B` | `FootClass::PerCellProcess` | foot receiver | Non-building boundary check |

Direct `CALL 0x00445F80` hits: none.

Additional Ghidra evidence:

- `get_function_callers(0x00445F80)` returned no callers because building calls are virtual.
- `get_function_xrefs(0x00445F80)` returned the building vtable data reference at `0x007E4398`.

Why it matters:

If a post-load pass rehydrated all buildings through the building vtable slot, it should appear as a `CALL [reg+0x4DC]` static site or a direct call. The drained static call-site set has no such save/load driver.

### 6. `BuildingClass::Update` is not the lazy post-load allocator

Active in YR: Yes.

Evidence:

- Decompile of `BuildingClass__Update @ 0x0043FB20`.
- Ghidra callee list for `0x0043FB20`.

Details:

- The callee list includes animation, repair/power, gap/special effects, damage fire animation, survivals/destruction handling, `TechnoClass__AI_Update`, and other building runtime helpers.
- The callee list does not include `LightSourceClass__Constructor @ 0x00554760`.
- The decompiled body does not contain a virtual `+0x4DC` call.
- Online/offline helpers reached by power logic only enable/disable an existing non-null source; they do not allocate `+0x614`.

Why it matters:

The specific "first post-load tick lazily allocates building lights" hypothesis is not supported by `BuildingClass::Update`.

### 7. Online/offline, damage/sell, and destructor paths do not create post-load lights

Active in YR: Yes for their standard lifecycle conditions.

Evidence:

- Prior lifecycle report plus current targeted decompile/call checks for `BuildingClass::Update`, `BuildingClass::Load`, and allocation callers.
- Existing verified assembly in `LIGHTSOURCE_LIFECYCLE_POWER_DAMAGE_SAVELOAD_GHIDRA_REPORT.md`:
  - `GoOnline @ 0x00452250` enables non-null `+0x614`.
  - `RestoreOnlineEffects @ 0x00452410` enables non-null `+0x614`.
  - `ApplyOfflineEffects @ 0x00452480` disables non-null `+0x614`.
  - `ReceiveDamage` result case `4` disables non-null `+0x614`.
  - `Sell` disables non-null `+0x614`.
  - `Destructor @ 0x0043BCF0` disables, deletes, then zeroes non-null `+0x614`.

Details:

- Enable/disable wrappers require a non-null pointer in the building field.
- A loaded building whose `+0x614` was zeroed will not be allocated by simply going online/offline.
- Destruction/sell cannot be the rehydrate owner; they remove or disable light contribution.
- Destructor remains the only verified building path that both destroys the light object and writes zero as part of real lifetime teardown. Load zeroing is separate and does not call the light destructor.

Why it matters:

Post-load light state must be rebuilt by a restore/rebuild step outside these ordinary toggle paths if Rust wants loaded lamp lighting to be visible.

## Active In Standard YR

- Save/load path: Active in standard YR saved-game load. `BuildingClass::Load` zeroing is live for loaded building objects.
- Fresh placement/unlimbo: Active for scenario placement, deploy, and similar placement paths that call building `Unlimbo`.
- Fresh construction completion: Active for buildings completing construction through `Mission_Construction`.
- Owner-change/discovery `+0x4DC` calls: Active conditionally, but not a general save/load rehydrate path and not sufficient to allocate when `+0x6E4` is already true.
- Online/offline, damage, sell, destructor: Active under their ordinary gameplay conditions, but none allocates `+0x614`.
- Radiation `LightSourceClass` caller: Active for radiation site logic, but separate owner and no `BuildingClass+0x614` write.

## Implementation Handoff

Verified behavior -> `BuildingClass::Load` zeroes `+0x614` and does not pointer-fix it.
Rust delta -> Treat building light handles/contributions as transient rebuildable render/app state, not authoritative save state.
Affected surfaces -> [src/sim/snapshot.rs](src/sim/snapshot.rs), [src/sim/world/mod.rs](src/sim/world/mod.rs), [src/app_input.rs](src/app_input.rs), [src/app_init.rs](src/app_init.rs), [src/map/lighting.rs](src/map/lighting.rs).
Acceptance scenario -> Save/load a map with a live lamp building; after load, app/render lighting is rebuilt from stable loaded building state, not from a serialized light handle.
Suggested test -> `building_point_light_rebuilt_after_snapshot_load_without_serialized_handle`.

Verified behavior -> Only `Unlimbo` and `OnConstructionComplete` allocate building `+0x614`; both gate on `LightIntensity != 0` and enable with mode `0`.
Rust delta -> Keep the point-light collection/allocation gate as `LightIntensity != 0`, and eventually model native enable/disable timing separately from type parsing.
Affected surfaces -> [src/map/lighting.rs](src/map/lighting.rs), [src/app_init.rs](src/app_init.rs), future building placement/completion hooks.
Acceptance scenario -> A building with default/nonzero `LightVisibility` but zero `LightIntensity` emits no light before or after load.
Suggested test -> `light_visibility_without_intensity_does_not_rehydrate_light`.

Verified behavior -> No general static post-load `OnConstructionComplete` caller exists, and force argument `1` does not allocate for already placed buildings.
Rust delta -> Do not copy a fake "first loaded tick calls construction complete for all buildings" mechanism. Rebuild render-side lights explicitly after snapshot load.
Affected surfaces -> [src/app_input.rs](src/app_input.rs), [src/app_init.rs](src/app_init.rs).
Acceptance scenario -> Loading a save should not rerun construction-complete side effects such as free-unit spawn, owner bonuses, EVA, wall connection, or superweapon events merely to restore lighting.
Suggested test -> `snapshot_load_rebuilds_lighting_without_construction_complete_side_effects`.

Verified behavior -> Online/offline/damage/sell/destructor paths only toggle or remove existing light pointers.
Rust delta -> When native light handles exist, power/death/sell hooks should toggle/remove contributions only when an owned runtime source exists; snapshot load should create/rebuild equivalent runtime source state before such toggles matter.
Affected surfaces -> future power/building lifecycle code, [src/app_init.rs](src/app_init.rs), [src/map/lighting.rs](src/map/lighting.rs).
Acceptance scenario -> Save/load an offline or destroyed lamp building; it must not become lit solely because the type has `LightIntensity`.
Suggested tests -> `offline_building_light_stays_inactive_after_snapshot_load`, `destroyed_building_light_not_recreated_after_load`.

Current Rust note:

- [src/app_input.rs](src/app_input.rs) already rebuilds `state.lighting_grid` after snapshot load via `rebuild_lighting_grid_from_sim`.
- [src/app_init.rs](src/app_init.rs) collects live structures with `!dying` and `health.current > 0`, then derives point lights from rules.
- This is directionally aligned with the static-code finding that the native handle is transient. It is still not full gamemd mechanism parity because native has `LightSourceClass` active/detail state and immediate dirty-cell recompute semantics.

## Negative Facts / Do Not Do

- Do not serialize or pointer-fix a building `LightSourceClass*` equivalent as authoritative state. `BuildingClass::Load` explicitly writes zero to `+0x614`.
- Do not claim a verified first-post-load-tick lazy allocator. This slot found no static load/update/save-driver caller that allocates `+0x614` for all loaded buildings.
- Do not call or emulate `OnConstructionComplete` for all loaded buildings as a lighting restore shortcut. It has many unrelated one-shot side effects and its light allocation body still requires `ActuallyPlacedOnMap == false`.
- Do not allocate a building point light from `LightVisibility` alone. The checked native allocation gate is `BuildingTypeClass+0xE34` (`LightIntensity`) nonzero.
- Do not merge `BuildingClass+0x614` with `BuildingClass+0x600` spotlight state. `+0x600` is pointer-fixed in load; `+0x614` is zeroed.
- Do not treat `GoOnline`, `RestoreOnlineEffects`, `GoOffline`, damage, sell, or destructor as rehydration owners. They require or remove an existing pointer; they do not allocate it.
- Do not broaden this result to radiation or particle lights. `RadSiteClass::Activate` is a separate `LightSourceClass` constructor caller and not a building pointer rehydrate path.

## Remaining Uncertainty

- Runtime visual outcome after loading a stock YR save containing already placed lamp buildings remains unmeasured in this slot. Static evidence says no general rehydrate owner was found; a live debugger/watchpoint or controlled game observation could prove whether stock save/load leaves such lamps dark, or whether an unanalyzed dynamic/OLE path outside the static call-site slice changes the outcome.
- Exact complete OLE save/load object ordering is outside this slot. It is not needed for the negative static caller result, but would be needed for a full save-system model.
- This report does not patch older docs that still say "lazy first post-load tick". It records the correction bundle for later doc cleanup.

## Sources

- Ghidra read-only decompiles:
  - `BuildingClass::Load @ 0x00453E20`
  - `BuildingClass::OnConstructionComplete @ 0x00445F80`
  - `BuildingClass::Mission_Construction @ 0x00449A50`
  - `BuildingClass::Update @ 0x0043FB20`
  - `BuildingClass::Unlimbo @ 0x00440580`
  - `BuildingClass::DiscoveredBy-like hook @ 0x0044D5D0`
  - scenario/map placement helper `0x006E4200`
  - `FlyLocomotionClass::Emergency_Relocate @ 0x004CD000`
- Ghidra caller/callee queries:
  - callers of `LightSourceClass::Constructor @ 0x00554760`
  - xrefs/callers of `BuildingClass::OnConstructionComplete @ 0x00445F80`
  - callees of `BuildingClass::Update @ 0x0043FB20`
- Assembly contexts:
  - `0x00454160..0x00454183`
  - `0x00445F89..0x00445F99`
  - `0x00440DE6..0x00440DFD`
  - `0x00446750..0x00446767`
  - `0x00449AC5..0x00449AD4`
  - `0x00448CEF`
  - `0x0044D68A`
  - `0x006E42BB`
  - `0x004CD050`
- Local PE byte scan:
  - `CALL [reg+0x4DC]` hits: `0x00414F85`, `0x00414FC3`, `0x00448CEF`, `0x00449AD4`, `0x0044D68A`, `0x004CD050`, `0x004CDB9F`, `0x004D8D8B`, `0x006E42BB`.
  - Direct `CALL 0x00445F80`: none.
- Local Rust scan:
  - [src/map/lighting.rs](src/map/lighting.rs)
  - [src/app_init.rs](src/app_init.rs)
  - [src/app_input.rs](src/app_input.rs)
  - [src/sim/snapshot.rs](src/sim/snapshot.rs)
- Prior research map:
  - `LIGHTSOURCE_LIFECYCLE_POWER_DAMAGE_SAVELOAD_GHIDRA_REPORT.md`
  - `BUILDING_LIGHTSOURCE_POST_LOAD_REHYDRATE_GHIDRA_REPORT.md`
  - `LIGHTSOURCE_QUEUED_MODE_CALLER_CENSUS_GHIDRA_REPORT.md`
