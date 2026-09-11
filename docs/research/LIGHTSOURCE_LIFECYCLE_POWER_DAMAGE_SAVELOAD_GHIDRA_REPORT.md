# LightSource Lifecycle, Power, Damage, Save/Load Ghidra Report

Date: 2026-05-22

2026-09-11 correction against active retail `gamemd.exe` SHA-256
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`:
ordinary House low power does **not** establish a lamp-disable callback.
The explicit online/offline entries are `0x00452260` / `0x00452360`;
`0x00452250` is a return-zero stub and `0x00452380` is an interior address.
The power-based implementation recommendation below has been corrected.

Status: PARTIAL. Building LightSource allocation, enable/disable, death/sell teardown, destructor cleanup, and load zeroing are verified. The remaining bounded gap is the exact outer post-load caller that rehydrates `BuildingClass+0x614` after `BuildingClass__Load` zeroes it.

## Target question

How does the building `LightSourceClass*` at `BuildingClass+0x614` live across placement/unlimbo, construction complete, online/offline power transitions, owner change, damage/destruction/sell, destructor, and save/load, and what Rust lifecycle hooks need to mirror it?

## Non-goals

- Do not investigate radiation `LightSourceClass` except to distinguish wrapper semantics.
- Do not investigate `BuildingLightClass`/spotlights at `BuildingClass+0x600`.
- Do not re-derive point-light falloff, LightConvert cache behavior, map `[Lighting]`, or `ExtraLight=`.
- Do not mutate Ghidra labels/comments, Rust, INI, existing docs, or `.swarm-claims.md`.

## Evidence needed to mark COMPLETE

- Verify the building allocation sites that store a `LightSourceClass*` into `+0x614`.
- Verify the allocation gate and constructor arguments for building lights.
- Verify wrapper semantics for `0x00554A60`, `0x00554A80`, and dirty recompute mode used by building callers.
- Verify online/offline, owner-change, sell, damage/destruction, and destructor call sites against `+0x614`.
- Verify save/load treatment of `+0x614`.
- Identify the exact post-load rehydration caller, or record it as remaining uncertainty.

## Stop conditions

- Stop if the scope expands into radius-light math, radiation, spotlights, or full building save/load.
- Stop once every requested lifecycle surface has either a verified call site or a named uncertainty.
- Stop rather than editing existing stale docs.

## Verified Findings

### 1. Constructor state and wrapper semantics

- Active in YR: Yes. `LightSourceClass__Constructor @ 0x00554760` inserts the source into the global light-source vector and initializes the source inactive: `LightSource+0x48 = 0`, with detail threshold `+0x34 = 2`. Evidence: decompile `0x00554760`; corroborated by `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`.
- Active in YR: Yes. `0x00554A60` is the enable wrapper: if `+0x48 == 0`, it sets `+0x48 = 1` and calls `0x00554AF0(mode)`. Evidence: decompile `0x00554A60`.
- Active in YR: Yes. `0x00554A80` is the disable wrapper: if `+0x48 != 0`, it sets `+0x48 = 0` and calls `0x00554AF0(mode)`. Evidence: decompile `0x00554A80`.
- Active in YR: Yes. Standard building lifecycle callers checked pass immediate recompute mode `0`; this report spot-checked assembly call sites and agrees with `LIGHTSOURCE_QUEUED_MODE_CALLER_CENSUS_GHIDRA_REPORT.md`.

### 2. Placement / unlimbo allocation

- Active in YR: Yes. `BuildingClass__Unlimbo @ 0x00440580` allocates the building radius light only when `BuildingTypeClass+0xE34 != 0`, i.e. nonzero `LightIntensity`, and only if `BuildingClass+0x614` is currently null. Evidence: decompile `0x00440580`; assembly `0x00440D80..0x00440DFD`.
- Active in YR: Yes. The constructor arguments are copied from `BuildingTypeClass+0xE30/+0xE34/+0xE38/+0xE3C/+0xE40` plus the building coordinate returned by vtable `+0x48`; the return pointer is stored at `BuildingClass+0x614`. Evidence: assembly `0x00440DA0..0x00440DEF`, constructor call `0x00440DE6`, store `0x00440DEF`.
- Active in YR: Yes. Immediately after storing `+0x614`, Unlimbo loads that pointer, pushes `0`, and calls `0x00554A60`, so the new source is enabled and affected cells recompute immediately. Evidence: assembly `0x00440DF5..0x00440DFD`.

### 3. Construction-complete allocation

- Active in YR: Yes. `BuildingClass__OnConstructionComplete @ 0x00445F80` repeats the same null-check/allocation pattern when `Type+0xE34 != 0`. Evidence: decompile `0x00445F80`, assembly constructor call `0x00446750`, store `0x00446759`.
- Active in YR: Yes. The construction-complete path immediately enables the newly stored pointer with `PUSH 0; CALL 0x00554A60`. Evidence: assembly `0x0044675F..0x00446767`.
- Active in YR: Conditional. The entry gate is `if (ActuallyPlacedOnMap && !force_flag) return`; the normal one-shot construction-complete path is active for freshly completed buildings, and forced calls are the likely post-load rehydrate route, but this slot did not prove the outer force caller. Evidence: decompile `0x00445F80`; existing `BUILDINGCLASS_RESIDUAL_Q_R4.md` notes the force flag.

### 4. Explicit online/offline effects and ordinary power

- Active in YR: Yes. `BuildingClass__GoOnline @ 0x00452260` checks `BuildingClass+0x614`; if non-null, it pushes `0` and calls `0x00554A60`. Evidence: assembly `0x004522B1..0x004522C3`; active Event caller `0x004C6D3D` and Trigger caller `0x006DE02E`.
- Active in YR: Yes. `BuildingClass__RestoreOnlineEffects @ 0x00452410` also enables a non-null light source with mode `0`. Evidence: assembly `0x00452415..0x00452428`.
- Active in YR: Yes. `BuildingClass__GoOffline @ 0x00452360` calls `BuildingClass__ApplyOfflineEffects`; that routine disables non-null `+0x614` with mode `0`. Evidence: entry `0x00452360`, decompile `0x00452480`, assembly `0x00452485..0x00452498`; callers `0x0044FD23` (map INI), `0x004C6D9A` (Event), `0x006DDFB9` (Trigger).
- Ordinary House power walk `0x00508C30 -> 0x00454CE0` reaches a return stub. Building power effects `0x0043FB20 -> 0x004549B0 -> 0x004545D0/0x004547C0` change animations, without disabling `+0x614`. Stock GALITE has `Power=0` and also falls outside the strict-positive type-power animation branch. Recomputing lamp activity from `is_building_powered` is therefore not an equivalent producer.
- `0x00452410` is also reached by EMP expiry at `0x006FAF54`; this does not establish ordinary House power recovery as its caller. Explicit offline/EMP state must have its own admitted producers.

### 5. Owner change / capture

- Active in YR: Conditional. `BuildingClass__ChangeOwner @ 0x00448170` enables a non-null `+0x614` with mode `0` inside its engineer/capture online-effects branch (`Type+0x1552 != 0`). Evidence: decompile `0x00448170`; assembly `0x004484C0..0x004484D3`.
- Active in YR: Yes for captured buildings that enter that branch. The light's color/radius data remains type-derived, not owner-derived; owner change matters because online/power state and enabled effects are re-evaluated. Evidence: no owner argument is passed to `0x00554A60`; constructor args remain type fields at the allocation sites.

### 6. Damage/destruction and sell

- Active in YR: Yes. `BuildingClass__ReceiveDamage @ 0x00442230`, damage result case `4`, disables `+0x614` before destruction effects (`vtable+0x4EC`). Evidence: decompile `0x00442230`, assembly `0x00442640..0x0044264C`.
- Active in YR: Yes. `BuildingClass__Sell @ 0x00449830` disables a non-null `+0x614` during deploy/sell conversion handling and again in the later sell/removal branch. Evidence: decompile `0x00449830`, assembly calls `0x00449F1E` and `0x0044A20B`.
- Active in YR: Yes. These death/sell paths remove the emitted light immediately but do not themselves prove pointer deletion/zeroing at the call site; final object cleanup happens in the destructor path. Evidence: decompile `0x00442230` and `0x00449830` show `0x00554A80(0)` but not a store of zero to `+0x614`.

### 7. Destructor cleanup

- Active in YR: Yes. `BuildingClass__Destructor @ 0x0043BCF0` checks `+0x614`, disables it with mode `0`, calls the light object's vtable `+0x20` destructor with delete flag `1`, then stores zero to the building pointer. Evidence: decompile `0x0043BCF0`; assembly `0x0043BD3A..0x0043BD5D`.
- Active in YR: Yes. This is the only verified building path in this slice that both destroys the `LightSourceClass` object and zeroes `BuildingClass+0x614`. Death/sell earlier disable the effect so cells stop being lit before the building object is finally freed.

### 8. Save/load treatment

- Active in YR: Yes. `BuildingClass__Load @ 0x00453E20` performs normal pointer fixups for several fields including `+0x600`, then explicitly writes `0` to `+0x614`. Evidence: decompile `0x00453E20`; assembly `0x00454170..0x00454174`.
- Active in YR: Yes. `+0x614` is therefore a runtime cache, not a stable save pointer. It must not be serialized/fixed up as the building's durable light identity. Evidence: `BuildingClass__Load` zero plus absence of `+0x614` in the surrounding fixup list; sibling save/load report corroborates this.
- The exact general native source-rehydration producer remains unproved; see [the later post-load census](BUILDING_LIGHTSOURCE_POST_LOAD_REHYDRATE_GHIDRA_REPORT.md). Rust's eager reconstruction from alive/on-map buildings and type parameters is existing compatibility behavior, not native save/load parity. Aggregate House power does not define retained source activity. Exact Cell/Convert restoration and explicit source-active restoration remain open.

## Implementation Handoff

1. Verified behavior -> Building lamp source is allocated from type light fields only when `LightIntensity != 0`, then enabled immediately on Unlimbo/construction complete.
   Rust delta -> Add a runtime building-light-source state/registry instead of relying only on startup `collect_building_lights`.
   Affected surface -> `src/map/lighting.rs`, `src/sim/world/world_spawn.rs`, `src/sim/production/production_placement.rs`, placement/construction completion hooks.
   Acceptance scenario -> Placing or completing a lamp-post building immediately changes nearby cell light profiles using immediate recompute semantics.
   Proposed test name -> `test_building_lightsource_created_and_enabled_on_placement`.
   Risk -> High screenshot visibility on maps with lamp posts.

2. Verified behavior -> Online/offline/capture transitions call `0x00554A60`/`0x00554A80` with mode `0` for non-null `+0x614`.
   Rust delta -> Retain the nullable source and its explicit active state. Publish affected cells from admitted online/offline and conditional capture callbacks; do not derive activity from aggregate House power.
   Affected surface -> `src/sim/power_system.rs`, `src/sim/world/mod.rs`, capture/owner-change command handling, render-facing lighting cache.
   Acceptance scenario -> Ordinary low power preserves an already active lamp. Explicit online/offline callbacks toggle it, and repeated allocation does not reactivate an existing inactive source.
   Proposed test name -> `gsi_04_20_building_lamp_tracks_explicit_activity_and_detail`.
   Risk -> Medium/high; visible on lamp-heavy maps and any modded powered lamp building.

3. Verified behavior -> Damage case 4 and sell disable the light before destruction/sell effects; destructor later deletes and zeros the handle.
   Rust delta -> Remove or deactivate building light contribution at the beginning of death/sell/removal, not only after entity deletion.
   Affected surface -> `src/sim/production/production_sell.rs`, building damage/destruction handling, lighting invalidation.
   Acceptance scenario -> Destroying or selling a lamp building removes its lighting before subsequent debris/survivor/removal effects are processed.
   Proposed test name -> `test_sell_and_death_disable_lightsource_before_entity_removal`.
   Risk -> Medium; prevents one-frame stale lighting and stale cache entries.

4. Verified behavior -> `BuildingClass__Load` zeroes `+0x614`; the runtime light handle is not a durable save pointer.
   Rust delta -> Discard outgoing source operations and handles when loading. Preserve the current eager reconstruction only as explicitly limited compatibility; prove exact native source-active and Cell/Convert restoration separately.
   Affected surface -> save/load snapshot code once implemented, `src/map/lighting.rs`, building rehydrate path.
   Acceptance scenario -> A saved game with a lamp building reloads with the same visible lighting, but no stale runtime light ID is required in the save data.
   Proposed test name -> `test_building_lightsource_rehydrated_after_load_without_serialized_handle`.
   Risk -> Medium; save/load parity and determinism.

## Negative Facts / Do Not Do

- Do not allocate a building `LightSourceClass` just because `LightVisibility` is nonzero or defaults to `5000`; checked building allocation gates on `Type+0xE34` (`LightIntensity`) at `0x00440580` and `0x00445F80`. Active in YR: Yes.
- Do not treat constructor allocation as visible light; constructor starts inactive at `+0x48 = 0`, and the building lifecycle must call the enable wrapper. Active in YR: Yes.
- Do not merge `+0x614` radius lights with `+0x600` `BuildingLightClass` spotlights; they have separate allocation gates, behavior, and lifecycle. Active in YR: Yes/Conditional depending on `HasSpotlight=`.
- Do not persist or pointer-fix up `+0x614` as durable building state; `BuildingClass__Load` explicitly writes zero at `0x00454174`. Active in YR: Yes.
- Do not implement standard building light invalidation through queued mode; verified building callers pass mode `0`. Active in YR: Yes.
- Do not leave a lamp visually active until final entity deletion on sell/death; damage case 4 and sell disable the source before later destruction/removal work. Active in YR: Yes.

## Historical Rust Delta (2026-05-22)

The following scan predates the current source owner and ordered frame events.
See the [2026-09-11 delivery addendum](LIGHTSOURCE_DIRTY_SCHEDULING_00554AF0_00554D50_GHIDRA_REPORT.md#2026-09-11-production-correction).

- `src/map/lighting.rs` currently builds a startup `LightingGrid`, collects point lights from initial map entities, and accumulates them directly into a `HashMap<(u16,u16), [f32;3]>`.
- `src/app_init.rs` constructs that lighting grid during app initialization and does not model building light lifecycle after startup.
- `src/sim/power_system.rs` tracks owner power transitions and `is_building_powered`, but there is no render-facing lamp-source enable/disable hook.
- `src/sim/production/production_placement.rs`, `src/sim/world/world_spawn.rs`, and `src/sim/production/production_sell.rs` are the likely Rust ownership boundaries for placement/spawn/sell hooks.

## Stale Doc Notes

- `BUILDINGCLASS_FIELD_VERIFICATION_ROUND_2.md` and older master reports say `+0x614` is allocated when any `Type+0xE30..0xE40` light field is nonzero or describe it as default/all-building ambient light. Replacement wording: "`BuildingClass+0x614` is a nullable `LightSourceClass*` allocated in checked building paths when `BuildingTypeClass+0xE34` (`LightIntensity`) is nonzero; `LightVisibility` default alone does not allocate or emit a building light."
- `BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md` calls `Type+0xE34` "has `LightVisibility=`". Replacement wording: "`Type+0xE34` is `LightIntensity`; `Type+0xE30` is `LightVisibility`."
- `BUILDINGCLASS_ON_DESTROYED_GHIDRA_REPORT.md` wording that says `+0x614` is "destroyed" in ReceiveDamage should be narrowed. Replacement wording: "ReceiveDamage case 4 disables the active `LightSourceClass` via `0x00554A80(0)` before destruction effects; object delete/zeroing is verified in `BuildingClass__Destructor`."

## Remaining Uncertainty

- The exact outer post-load caller that recreates or forces rehydration of `+0x614` after `BuildingClass__Load` zeroes it was not proven. Existing docs claim first post-load lazy rebuild, and forced `OnConstructionComplete` can allocate when called, but this slot did not drain the caller chain.
- This report did not prove all possible direct calls to `LightSourceClass__Constructor`; it verified the two building allocation sites in scope and excluded radiation/particle light paths by non-goal.
- This report did not inspect `BuildingClass__PointerExpired`; older docs say it clears `+0x614` when the pointed object expires, but normal building lifecycle handoff does not depend on that path.

## Sources

- Ghidra decompiles: `0x00440580`, `0x00445F80`, `0x00452250`, `0x00452380`, `0x00452410`, `0x00452480`, `0x00448170`, `0x00449830`, `0x00442230`, `0x0043BCF0`, `0x00453E20`, `0x00554760`, `0x00554A60`, `0x00554A80`, `0x00554AF0`.
- Ghidra assembly spot-checks: `0x00440DA0..0x00440DFD`, `0x00446750..0x00446767`, `0x004522B1..0x004522C3`, `0x00452415..0x00452428`, `0x00452485..0x00452498`, `0x004484C0..0x004484D3`, `0x00449F1E`, `0x0044A20B`, `0x00442640..0x0044264C`, `0x0043BD3A..0x0043BD5D`, `0x00454170..0x00454174`.
- Local Rust scan: `src/map/lighting.rs`, `src/app_init.rs`, `src/sim/power_system.rs`, `src/sim/production/production_placement.rs`, `src/sim/world/world_spawn.rs`, `src/sim/production/production_sell.rs`.
