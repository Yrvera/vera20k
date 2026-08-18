# Building LightSource Post-Load Rehydrate Ghidra Report

Date: 2026-05-22

Status: COMPLETE for the bounded static-code slice; NEGATIVE for the prior
"exact post-load rehydrate caller" hypothesis.

Investigation mode: exhaustive-slice.

Scope:
- Resolve the open gap from
  `LIGHTSOURCE_LIFECYCLE_POWER_DAMAGE_SAVELOAD_GHIDRA_REPORT.md`: after
  `BuildingClass::Load` zeroes `BuildingClass+0x614`, what exact caller
  recreates the building `LightSourceClass*`?
- Covered: building `+0x614` allocation sites, `BuildingClass::Load` reset,
  `OnConstructionComplete` gates, `Mission_Construction` slot call, all static
  `vtable+0x4DC` call sites found by PE byte scan, and Rust-facing handoff.
- Not covered: spotlight beam rasterization for `BuildingClass+0x600`, palette
  table internals, radiation `LightSourceClass` behavior except as a caller
  discriminator.

## Executive Result

The exact general post-load rehydrate caller was not found because the static
binary evidence points the other way: building radius lights are only allocated
by `BuildingClass::Unlimbo @ 0x00440580` and
`BuildingClass::OnConstructionComplete @ 0x00445F80`. `BuildingClass::Load @
0x00453E20` explicitly clears `BuildingClass+0x614`, does not call either
allocator, and does not register `+0x614` for pointer fixup.

The previous docs' wording "recreated on-demand during the first post-load tick"
is therefore not verified by this slice. Replace it with:

> `BuildingClass+0x614` is a runtime light handle zeroed on load. No general
> post-load rehydrate caller was verified in the static call-site census. A Rust
> port should rebuild equivalent runtime light state explicitly from stable
> building type, position, and online state after snapshot load, instead of
> serializing the handle or waiting for a copied legacy lazy caller.

## Open Questions Log

Seeded:
1. Does `BuildingClass::Load` itself recreate or schedule `+0x614`? Resolved:
   no, it zeroes the pointer after fixups.
2. Does `BuildingClass::Update` lazily allocate `+0x614` on first post-load
   tick? Resolved: no direct allocation and no `vtable+0x4DC` call in the
   decompiled update body checked in this slice.
3. Does `Mission_Construction` invoke `OnConstructionComplete` in a way that
   explains all post-load buildings? Resolved: it invokes slot `+0x4DC` only in
   the construction-complete transition, not as a general load rehydrate pass.
4. Are there other building `LightSourceClass__Constructor` callers? Resolved:
   Ghidra call graph shows only `BuildingClass::Unlimbo` and
   `BuildingClass::OnConstructionComplete` for buildings; radiation callers are
   separate.
5. Is there another static `OnConstructionComplete` caller that is obviously an
   outer save/load driver? Resolved: no. All found `vtable+0x4DC` call sites are
   construction/change-owner/discovery/scenario-placement or non-building object
   boundary checks.

No un-deferred static-code questions remain for this slice. A runtime debugger
save/load experiment could still measure the player-visible result in stock YR,
but that would validate runtime outcome, not reveal a missing static allocator
site in the checked code.

## Verified Findings

### 1. `BuildingClass::Load` zeroes `+0x614` after normal pointer fixups

Active in YR: Yes.

Evidence:
- `BuildingClass__Load @ 0x00453E20` decompile.
- Assembly context at `0x00454170..0x00454174`:
  - preceding loop registers secondary anim pointers with `FUN_006CF240`;
  - `0x00454174: MOV dword ptr [EDI + 0x614],0x0`.

Tiny details:
- The zero happens after fixed-array pointer fixups for anim/upgrade slots.
- `+0x614` is not passed to `FUN_006CF240`; it is not part of the old-to-new
  pointer map.
- The write is unconditional. It does not check `LightIntensity`, power state,
  map editor mode, or whether the saved pointer was non-null.
- Active light cells are not dirtied here; `Load` only clears the handle.

Why it matters:
- `+0x614` is a runtime cache. Persisting or pointer-fixing it in Rust would
  diverge from gamemd's save/load model.

### 2. Building `LightSourceClass` allocation has only two verified building sites

Active in YR: Yes.

Evidence:
- Ghidra call graph for `LightSourceClass__Constructor @ 0x00554760`:
  - `BuildingClass__Unlimbo -> LightSourceClass__Constructor`
  - `BuildingClass__OnConstructionComplete -> LightSourceClass__Constructor`
  - radiation callers via `RadSiteClass__Activate`, separate from buildings.
- Assembly:
  - `0x00440DEF: MOV [ESI+0x614],EAX` in `BuildingClass::Unlimbo`.
  - `0x00446759: MOV [EBP+0x614],EAX` in `OnConstructionComplete`.

Tiny details:
- Both building sites immediately enable the source with mode `0`:
  - `0x00440DF5..0x00440DFD`: load `[ESI+0x614]`, `PUSH 0`, call `0x00554A60`.
  - `0x0044675F..0x00446767`: load `[EBP+0x614]`, `PUSH 0`, call `0x00554A60`.
- Both use the same building type fields:
  - `Type+0xE30` visibility/radius source argument,
  - `Type+0xE34` intensity gate/argument,
  - `Type+0xE38/+0xE3C/+0xE40` RGB tint arguments.
- Allocation is gated by `Type+0xE34 != 0`, not merely by nonzero visibility.

Why it matters:
- Any post-load rehydrate path must route through `Unlimbo` or
  `OnConstructionComplete`; there is no third building light allocator in the
  checked binary graph.

### 3. `OnConstructionComplete` force flag is not a universal rehydrate switch

Active in YR: Yes.

Evidence:
- `BuildingClass__OnConstructionComplete @ 0x00445F80` decompile.

Tiny details:
- The top early-out is:
  - if `ActuallyPlacedOnMap != false` and `param_2 == 0`, return.
- The light allocation block is inside:
  - `if (param_1->ActuallyPlacedOnMap == false) { ... allocate +0x614 ... }`.
- Therefore:
  - `param_2 = 0` and already placed: returns before allocation.
  - `param_2 = 1` and already placed: skips the early return, but still skips
    the allocation block because `ActuallyPlacedOnMap` is true.
  - Either `param_2` value can allocate only when `ActuallyPlacedOnMap` is
    false.
- The function sets the placement fence at the tail:
  - `param_1->ActuallyPlacedOnMap = true`.

Why it matters:
- A hypothetical forced post-load call only works if load/constructor left
  `ActuallyPlacedOnMap` false. The force flag by itself does not rebuild a
  missing `+0x614` for an already placed building.

### 4. `Mission_Construction` calls slot `+0x4DC` only at build completion

Active in YR: Yes.

Evidence:
- `Mission_Construction @ 0x00449A50` decompile and assembly context.
- Assembly around the completion tail:
  - `0x00449AC5: PUSH 0x1`
  - `0x00449AC9: CALL 0x00447780` (`GrandOpening(1)`)
  - `0x00449AD0: PUSH 0x0`
  - `0x00449AD4: CALL dword ptr [EDX + 0x4DC]`

Tiny details:
- State `0` starts buildup animation with `GrandOpening(0)`, then sets the
  mission substate to `1`.
- State `1` waits for byte `+0x6DD != 0` before it calls:
  - radio/status slot `+0x274` with `0x0C`,
  - radio/status slot `+0x274` with `0x03`,
  - `GrandOpening(1)`,
  - `OnConstructionComplete(0)`,
  - `Queue_Mission(5,0)`.
- `Mission_Construction` is not called from `BuildingClass::Load` in the
  checked load body.

Why it matters:
- This explains fresh construction completion, and possibly buildings saved
  while still in construction animation, but it is not a general post-load pass
  over all placed buildings.

### 5. Static `vtable+0x4DC` call-site census finds no save/load rehydrate pass

Active in YR: Yes for listed code; no post-load caller verified.

Method:
- PE byte scan of `gamemd.exe` for indirect calls of the form
  `FF 90..97 DC 04 00 00`, i.e. `CALL dword ptr [reg+0x4DC]`.
- Direct-call scan for `CALL 0x00445F80` found no direct calls.

Found sites:

| Address | Function / context | Pushed arg | Rehydrate relevance |
|---|---:|---:|---|
| `0x00449AD4` | `BuildingClass::Mission_Construction` | `0` | Fresh construction completion, not load driver |
| `0x00448CEF` | `BuildingClass::ChangeOwner` | `1` | Capture/owner-change refresh, not load driver |
| `0x0044D68A` | `BuildingClass::DiscoveredBy` | `0` | Discovery hook; only reaches if several discovery/path conditions pass |
| `0x006E42BB` | Scenario/map placement helper after create/place success | `0` | Fresh object placement from type/cell; not `BuildingClass::Load` |
| `0x00414F85` | `AircraftClass::AI` boundary check | none visible | Non-building virtual slot semantics |
| `0x00414FC3` | `AircraftClass::AI` boundary check | none visible | Non-building virtual slot semantics |
| `0x004CDB9F` | `FlyLocomotionClass::Process` boundary/relocate path | none visible | Non-building virtual slot semantics |
| `0x004D8D8B` | `FootClass::PerCellProcess` boundary check | none visible | Non-building virtual slot semantics |

Tiny details:
- `0x006E4200` creates or retrieves a building through type-array logic and
  calls a placement-like slot `+0x490` before `+0x4DC(0)`. It does not call
  `BuildingClass::Load`, does not use the swap-map dictionary
  `DAT_00B0C110`, and is not a loaded-object rehydrate pass.
- `BuildingClass::ChangeOwner` calls `OnConstructionComplete(1)` after
  removing/re-adding the building to many owner trait lists. Because the light
  allocation block still requires `ActuallyPlacedOnMap == false`, this does not
  allocate a missing light for a normal already placed captured building.
- The non-building object calls are to the same vtable offset on other class
  layouts; they are not `BuildingClass::OnConstructionComplete` unless the
  receiver is actually a building, and the decompiled contexts are aircraft,
  foot, or locomotion boundary handling.

Why it matters:
- The checked static code does not support a broad "first post-load tick invokes
  OnConstructionComplete for all buildings" claim.

### 6. `BuildingClass::Update` does not contain the missing allocator

Active in YR: Yes.

Evidence:
- `BuildingClass__Update @ 0x0043FB20` decompile checked during this slice.
- No call to `LightSourceClass__Constructor`.
- No static `vtable+0x4DC` call in the PE scan at or inside the update body.

Tiny details:
- `Update` does call `BuildingClass__UpdateAnimation`.
- `UpdateAnimation @ 0x004509D0` manipulates animation/progress flags including
  `+0x6DD` in animation cases, but does not allocate `+0x614`.
- Online/offline helpers only enable/disable an already non-null source.

Why it matters:
- "Lazy first post-load tick" cannot currently be grounded in
  `BuildingClass::Update` allocating the light.

### 7. Destructor cleanup remains the only verified delete-and-zero path

Active in YR: Yes.

Evidence:
- `BuildingClass__Destructor @ 0x0043BCF0`.
- Assembly context:
  - `0x0043BD3A`: load `[ESI+0x614]`;
  - if non-null, `PUSH 0`, call `0x00554A80`;
  - call light object's vtable `+0x20` with delete flag `1`;
  - `0x0043BD5D: MOV [ESI+0x614],EBP` where `EBP == 0`.

Tiny details:
- Death/sell paths can disable the light before object destruction, but do not
  necessarily delete and zero the pointer at that point.
- Load zeroing is separate from destructor cleanup: it discards a stale saved
  handle without running light-object destruction.

Why it matters:
- Rust should model runtime light handles as rebuildable cache state, not as
  entity identity requiring save-file lifetime management.

## Coverage Ledger

| Area | Status | Evidence |
|---|---|---|
| `BuildingClass::Load` treatment of `+0x614` | Verified | `0x00453E20`, assembly `0x00454174` |
| Building allocation sites for `+0x614` | Verified | call graph to `0x00554760`; `0x00440DEF`, `0x00446759` |
| `OnConstructionComplete` gates | Verified | decompile `0x00445F80` |
| `Mission_Construction` completion caller | Verified | decompile `0x00449A50`; assembly `0x00449AC5..0x00449AD4` |
| Static `vtable+0x4DC` caller census | Verified | PE byte scan plus decompile/assembly context |
| General post-load rehydrate caller | Not verified; likely absent in checked static sites | no load/update/driver caller found |
| Runtime stock save/load visual measurement | Deferred | requires debugger or controlled in-game save/load observation |

## Rust Implementation Handoff

Affected surfaces:
- `src/map/lighting.rs`
  - current map-load lighting is static and computes point lights from map
    entities only.
  - It currently treats `ExtraLight` as RGB cell brightness; separate map-lighting
    synthesis says this is wrong because `ExtraLight` is draw-depth/Z adjustment,
    not ambient RGB light.
- `src/app_init.rs`
  - currently builds `lighting_grid` only during initial map load.
- `src/app_input.rs`
  - `load_save_file` calls `sim.rebuild_caches_after_load(...)` but does not
    rebuild app-level building light contributions from the loaded entity set.
- `src/sim/world/mod.rs`
  - `Simulation::rebuild_caches_after_load` rebuilds deterministic sim caches
    like screen coords and occupancy, but lighting is app/render-side state.

Implementation guidance:
- Do not serialize a building light handle equivalent to gamemd `+0x614`.
- Do not wait for a copied "first tick lazy" hook unless a later runtime trace
  proves one. The static binary slice did not find it.
- Rebuild render/app-side point lights after load from stable data:
  building type, cell/coord, `LightIntensity`, `LightVisibility`, tint fields,
  and current online/alive/placed state once those exist in Rust.
- Allocation gate should be `LightIntensity != 0`, not `LightVisibility > 0`
  alone.
- Use the later map-lighting implementation spec for byte-exact falloff and
  tint math. This report only resolves the save/load lifecycle question.

Acceptance scenarios:
- Save/load a map containing a lamp building with nonzero `LightIntensity`; after
  loading, nearby cell/sprite tint matches the pre-save visible light without
  serializing a runtime light object.
- Save/load the same map with a building whose `LightVisibility` is default
  nonzero but `LightIntensity == 0`; it must not emit a point light.
- Save/load an offline/depowered light-emitting building once Rust has power
  state hooked to lights; it must not relight until the online transition enables
  it.
- Destroy/sell a lit building, then save/load; the light must stay absent.

Suggested focused test names:
- `building_point_light_rebuilt_after_snapshot_load`
- `light_visibility_without_intensity_does_not_rehydrate_light`
- `destroyed_building_light_not_recreated_after_load`
- `offline_building_light_rehydrate_respects_power_state`

## Corrections To Prior Docs

Recommended edits for existing docs:
- In `BUILDINGCLASS_SAVE_LOAD_GHIDRA_REPORT.md`, replace:
  - "`LightSourceClass* -- recreated on-demand during the first post-load tick,
    not persisted.`"
  - with:
  - "`LightSourceClass* -- zeroed on load and not pointer-fixed. This slice did
    not verify a general post-load rehydrate caller; Rust should rebuild runtime
    light state explicitly after snapshot load.`"
- In `BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md`, replace:
  - "`runtime cached fields (+0x614, +0x644, etc.) are rebuilt lazily on the
    first post-load tick.`"
  - with:
  - "`runtime cached fields are not restored through Unlimbo. For +0x614
    specifically, static analysis verifies load zeroing and normal allocation
    sites but not a general post-load lazy rehydrate caller.`"
- In `LIGHTSOURCE_LIFECYCLE_POWER_DAMAGE_SAVELOAD_GHIDRA_REPORT.md`, close the
  open gap as:
  - "`No exact general post-load rehydrate caller was found in the static
    call-site census. Treat earlier lazy-rebuild wording as unverified.`"

No docs were patched by this investigation; this report records the correction
bundle for a later doc cleanup pass.

## Sources

- `BuildingClass::Load @ 0x00453E20`
- `BuildingClass::Constructor @ 0x0043B740`
- `BuildingClass::Unlimbo @ 0x00440580`
- `BuildingClass::Update @ 0x0043FB20`
- `BuildingClass::UpdateAnimation @ 0x004509D0`
- `BuildingClass::OnConstructionComplete @ 0x00445F80`
- `BuildingClass::GrandOpening @ 0x00447780`
- `BuildingClass::Mission_Construction @ 0x00449A50`
- `BuildingClass::ChangeOwner @ 0x00448170`
- `BuildingClass::DiscoveredBy @ 0x0044D5D0`
- Scenario/map placement helper `FUN_006E4200`
- `LightSourceClass::Constructor @ 0x00554760`
- `LightSourceClass` enable helper `0x00554A60`
- `LightSourceClass` disable helper `0x00554A80`
- PE call-site scan of `gamemd.exe` for indirect `vtable+0x4DC` calls and
  direct `CALL 0x00445F80`
- Existing reports:
  - `LIGHTSOURCE_LIFECYCLE_POWER_DAMAGE_SAVELOAD_GHIDRA_REPORT.md`
  - `BUILDINGCLASS_SAVE_LOAD_GHIDRA_REPORT.md`
  - `BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md`
  - `BUILDINGCLASS_MISSION_GUARD_AND_CONSTRUCTION.md`
  - `MAP_LIGHTING_FINAL_SYSTEM_MODEL_SYNTHESIS.md`
