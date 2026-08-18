# AnimClass Constructor Caller Taxonomy Runtime Spawns - Ghidra Report

Date: 2026-05-28  
Target: `ANIMCLASS_CONSTRUCTOR_CALLER_TAXONOMY_RUNTIME_SPAWNS`  
Binary anchor: `AnimClass::Constructor @ 0x00421EA0`  
Mode: `/re-investigate` coverage map, read-only Ghidra

## Working Notes Gate

Target question: classify runtime callers of `AnimClass::Constructor @ 0x00421EA0` into active Yuri's Revenge spawn families, with constructor argument rows and Rust-facing ownership surfaces.

Non-goals: do not re-prove the already-settled weapon muzzle flash, warhead explosion/debris, `SetOwnerObject`, combat light, lifecycle, global registration, draw traversal, DrawIt flag, or garrison depth reports except where their facts are needed to classify caller families.

Evidence needed to mark COMPLETE: constructor argument order verified at `0x00421EA0`; xref/caller list captured; load-bearing callers sampled in decompile/disassembly context; active-YR status recorded per material family; current Rust surfaces scanned; at least one implementation handoff with concrete test-name proposal.

Stop conditions: stop after the active runtime spawn-family taxonomy and handoff are complete; record unlabeled helper liveness or exact Rules offset mapping as Remaining Uncertainty instead of expanding into unrelated system investigations.

## Summary

`AnimClass::Constructor` is not a single "visual effect" entry point. It is the common allocator/initializer for many runtime spawn families: direct weapon and warhead visuals, AnimClass self-spawned trailers/bounce/expire chains, VoxelAnim trails/impacts, building slot/damage/death overlays, infantry/unit/aircraft death and movement effects, locomotion wake/warp visuals, superweapon overlays, bridge/terrain/overlay effects, and script/action helper anims.

The parent-known muzzle/warhead families remain valid, but they are only a subset. Most active-YR spawn rows use `drawFlags=0x600`, `delay=0`, `loop=1`, `zAdjust=0`, `reverse=0`; however that is not universal. Warhead explosions and bouncer/voxel expire impacts use `drawFlags=0x2600`; building slot overlay construction uses `drawFlags=0x1600`; trailers use `delay=1`; bridge and building debris can use random delays; EMP uses a random delay; unit special reverse animation passes `reverse=1`.

Current Rust has useful narrow runtimes (`src/app_building_anim.rs` garrison runtime, `src/app_fire_effects.rs` weapon visuals, `src/sim/world/mod.rs` `world_effects`) but does not yet have a generic `AnimClass` runtime entity/spawn surface that records constructor rows, owner attachment, slot post-writes, trailer/bounce/expire spawning, or global runtime family coverage.

## Constructor Shape

Verified at `AnimClass::Constructor @ 0x00421EA0`.

Prototype shape:

```text
AnimClass::Constructor(this, AnimTypeClass* type, CoordStruct* coords,
                       int delay, int loopCount, uint drawFlags,
                       int zAdjust, char reverse)
```

Load-bearing behavior:

- Active in YR: Yes. Evidence: Ghidra xrefs from live game systems include `TechnoClassFireAtSpawnsBullet @ 0x006FDD50`, `WarheadTypeClass::Detonate @ 0x004690B0`, `BuildingClass::CreateAnimForSlot @ 0x00451890`, `AnimClass::AI @ 0x00423AC0`, `VoxelAnimClass::AI @ 0x00749F30`, `LightningStorm::GroundStrike @ 0x0053A300`, and many others.
- The constructor stores `drawFlags` into the instance draw-flag field, stores constructor `delay`, and uses the constructor `zAdjust` unless the call supplies zero and the AnimType default applies.
- Loop remaining is the byte product of AnimType loop count and constructor loop count, with constructor loop count clamped to at least one and final values below two collapsed to one.
- Constructor `reverse` and AnimType reverse start the frame at `LoopEnd - 1` with a negative step.
- `delay == 0` immediately enters `AnimClass::Middle`.
- The instance is registered in `g_AnimClass_Array`, but ordinary per-tick AI scheduling is through the live object/logic vector after reveal, not through that global array.

## Caller Taxonomy

### Already-settled direct combat visuals

1. Weapon muzzle / `OccupantAnim`
   - Active in YR: Yes. Evidence: `TechnoClassFireAtSpawnsBullet @ 0x006FDD50` xrefs constructor and prior `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`.
   - Constructor row: `type=Weapon Anim or OccupantAnim`, `coords=&muzzleCoords`, `delay=0`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`.
   - Timing/liveness: spawned at fire resolution. Non-building shots attach to owner through `SetOwnerObject`; occupied-building flashes are post-adjusted with `ZAdjust=-200`.

2. Warhead/bomb/lightning explosion and debris basics
   - Active in YR: Yes. Evidence: `WarheadTypeClass::Detonate @ 0x004690B0`, `BombClass::Detonate @ 0x00438720`, `LightningStorm::GroundStrike @ 0x0053A300`.
   - Constructor row for explosion anim: `type=Warhead::SelectExplosionAnim(...)`, `delay=0`, `loop=1`, `drawFlags=0x2600`, `zAdjust=FUN_0048ACE0(...)`, `reverse=0`.
   - Constructor row for metallic debris/list follow-ups: commonly Rules debris list, `delay=0`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`.

### AnimClass self-spawned runtime chains

1. `TrailerAnim`
   - Active in YR: Conditional. Evidence: `AnimClass::AI @ 0x00423AC0` reads `type+0x308` and period `type+0x30C`; stock/modded art keys can provide `TrailerAnim`/separation.
   - Constructor row: `type=AnimType.TrailerAnim`, `coords=this->GetCoords()`, `delay=1`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`.
   - Timing/liveness: only while the source AnimClass remains active and not inactive; period is either every tick for separation one or `CurrentFrame % separation == 0`.

2. Bouncer `BounceAnim`
   - Active in YR: Conditional. Evidence: `AnimClass::ProcessBounceResult @ 0x00423930` reads `type+0x300` after `BounceClass::Update()` result one; stock YR uses bouncer/meteor/tiberium-like anim paths.
   - Constructor row: `type=AnimType.BounceAnim`, `coords=this->GetCoords()`, `delay=0`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`.
   - Timing/liveness: spawned on bounce result one, before damage handling from the bouncer warhead/radius fields.

3. Bouncer/meteor `ExpireAnim`
   - Active in YR: Conditional. Evidence: `AnimClass::AI @ 0x00423AC0` impact/destruction branch reads `type+0x304`.
   - Constructor row: `type=AnimType.ExpireAnim`, `coords=ftol bounce coords`, `delay=0`, `loop=1`, `drawFlags=0x2600`, `zAdjust=-30`, `reverse=0`.
   - Timing/liveness: spawned on impact/destruction in the bouncer AI path, not by normal `Destroy()`.

4. Bouncer water/smoke/list effects
   - Active in YR: Conditional. Evidence: `AnimClass::AI @ 0x00423AC0` spawns Rules pointer/list anims around bouncer/meteor movement or impact cases.
   - Constructor rows: Rules pointer/list anims, typically `delay=0`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`.
   - Timing/liveness: tied to bouncer movement over water/impact branches and optional type list at `type+0x2F0/+0x2F4`.

5. `Next=`
   - Active in YR: Yes where AnimTypes use `Next`. Evidence: constructor xrefs do not represent this; `AnimClass::AI` switches the same instance in place.
   - Constructor row: none. This is not an allocation path.

### VoxelAnim-to-AnimClass bridge

1. Voxel trailer
   - Active in YR: Conditional/Yes for voxel debris types with trailer metadata. Evidence: `VoxelAnimClass::AI @ 0x00749F30` reads `VoxelAnimType+0x2EC`.
   - Constructor row: `type=VoxelAnimType.TrailerAnim`, `coords=ftol voxel coords`, `delay=1`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`.
   - Timing/liveness: spawned while the voxel anim is alive on the parity-tested frame cadence.

2. Voxel bounce/expire
   - Active in YR: Conditional/Yes for bouncy voxel debris. Evidence: `VoxelAnimClass::AI @ 0x00749F30` reads `VoxelAnimType+0x2E4` and `+0x2E8`.
   - Constructor rows: bounce uses `delay=0`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`; expire uses `delay=0`, `loop=1`, `drawFlags=0x2600`, `zAdjust=-30`; both use `reverse=0`.
   - Timing/liveness: mirrors AnimClass bouncer semantics at voxel bounce/impact decision points.

### Building runtime overlays and destruction

1. Building slot anims
   - Active in YR: Yes. Evidence: `BuildingClass::CreateAnimForSlot @ 0x00451890`.
   - Constructor row: slot-selected AnimType at computed building coords, `delay=caller-supplied`, `loop=1`, `drawFlags=0x1600`, `zAdjust=0`, `reverse=0`.
   - Timing/liveness: creates/replaces one of the building's 21 overlay slots. Post-constructor writes set building slot offsets, slot activity flags, translucency/palette fields, and current-frame preservation.

2. Building damage fire anims
   - Active in YR: Yes. Evidence: `BuildingClass::CreateDamageFireAnims @ 0x0043C0D0`.
   - Constructor row: Rules damage-fire list entry at `DamageFireOffset`-derived coords, `delay=0`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`.
   - Timing/liveness: created when damaged building slots are empty; post-constructor computes a non-positive Z adjust from offset/foundation dimensions and randomizes current frame.

3. Building destruction/debris anims
   - Active in YR: Yes. Evidence: `BuildingClass::DestructionEffects @ 0x004415F0`.
   - Constructor rows: building debris list entries at random foundation coords with random delay range in some branches; other destruction anims use `delay=0`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`.
   - Timing/liveness: created during building destruction and wall/adjacent overlay destruction branches.

### Infantry, unit, aircraft, and object damage/death visuals

1. Infantry death anims
   - Active in YR: Yes. Evidence: `InfantryClass::DoType_Sequencer @ 0x00520AE0`.
   - Constructor row: infantry death list or Rules death list entry at infantry coords, `delay=0`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`.
   - Timing/liveness: selected for death sequence cases before infantry destruction.

2. Aircraft smoke/debris/trailer
   - Active in YR: Yes. Evidence: `AircraftClass::AI @ 0x00414BB0`, `AircraftClass::ReceiveDamage @ 0x004165C0`.
   - Constructor rows: smoke/falling effects use Rules pointers and `delay=0`, `loop=1`, `drawFlags=0x600`; aircraft trailer uses `delay=1`; death debris uses type debris list and `delay=0`, `loop=1`, `drawFlags=0x600`.
   - Timing/liveness: health/falling/frame-period gated in AI; death debris gated by damage/death result and type debris metadata.

3. Unit water death and death explosion
   - Active in YR: Yes. Evidence: `UnitClass::ReceiveDamage @ 0x00737C90`, `UnitClass::Death_Explosion @ 0x00738680` xref presence.
   - Constructor rows: water death smoke/splash Rules pointers use `delay=0`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`; non-water death routes to death explosion/debris logic.
   - Timing/liveness: fired on damage result/death branch.

### Movement, wake, warp, and temporal families

1. Ground/hover/ship water wake
   - Active in YR: Yes for applicable locomotors and water cells. Evidence: `DriveLocomotionClass::Process @ 0x004B0500`, `HoverLocomotionClass::Move @ 0x00514310`, `ShipLocomotionClass::Process @ 0x0069FC10` xrefs.
   - Constructor row: Rules wake/smoke pointer at owner coords, `delay=0`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`.
   - Timing/liveness: movement-state, not-on-bridge, water-landtype, and frame-period gated.

2. Chrono/teleport/warp visuals
   - Active in YR: Yes for teleport locomotor and temporal weapon paths. Evidence: `TeleportLocomotionClass::InitiateWarp @ 0x00719400`, `TeleportLocomotionClass::ClearPendingWarpPhase @ 0x00719790`, `WarpAttachClass::SpawnWarpAnims @ 0x00629E90`, `TemporalClass::AI @ 0x006297F0`, `TemporalClass::Update @ 0x0071A760`.
   - Constructor rows: warp attach random triplet uses Rules pointer at randomized coords with `delay=0`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`; temporal erasure/final visuals use Rules pointer rows; one TemporalClass branch stores an owned AnimClass pointer and registers for remove-listener handling.
   - Timing/liveness: tied to teleport phase transitions, temporal attack state, and temporal erasure.

3. EMP attach sparks
   - Active in YR: Conditional. Evidence: `EMPulseClass::Apply @ 0x004C54E0` reads Rules EMP anim pointer and calls `SetOwnerObject`.
   - Constructor row: `type=Rules EMP anim`, `coords=object coords`, `delay=random 0..25`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`.
   - Timing/liveness: spawned for affected non-building techno objects when EMP application succeeds.

### Superweapon, bridge, terrain, overlay, and script/action helper families

1. Lightning Storm
   - Active in YR: Yes. Evidence: `LightningStorm::CreateCloudBolt @ 0x0053A140`, `LightningStorm::GroundStrike @ 0x0053A300`, standard YR superweapon content.
   - Constructor rows: cloud-bolt and ground-strike lists use `delay=0`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`; impact explosion then uses the warhead explosion row with `drawFlags=0x2600`.
   - Timing/liveness: storm controller stores/uses spawned pointers for cloud bolt arrays and strike resolution.

2. Psychic Dominator
   - Active in YR: Yes. Evidence: `PsychicDominator::MindControlArea @ 0x0053B080`, plus adjacent dominator helper `FUN_0053AE50 @ 0x0053AE50`.
   - Constructor rows: center and per-victim mind-control anims use Rules pointers, `delay=0`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`.
   - Timing/liveness: per-victim anims attach to owners with `SetOwnerObject`; center/global pointer is stored for dominator state.

3. Bridge collapse
   - Active in YR: Yes. Evidence: `CellClass::BlowUpBridge @ 0x0047DD70`, `MapClass::CollapseBridge_*`, `FUN_00581140 @ 0x00581140`.
   - Constructor row: random bridge collapse anim types at randomized footprint coords, `delay=random 0..2`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`.
   - Timing/liveness: emitted over bridge footprint during collapse helper execution.

4. Overlay and terrain effects
   - Active in YR: Conditional/Yes depending overlay/terrain data. Evidence: `OverlayClass::Mark @ 0x005FC570`, `TerrainClass::Catch_Fire @ 0x0071C5B0`, `TerrainClass::Take_Damage @ 0x0071B920` xrefs.
   - Constructor rows: overlay-linked AnimType at cell coords generally uses `delay=0`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`, followed by palette/visual-field adjustments.
   - Timing/liveness: mark/damage/fire-cell state gated; full terrain rows were not expanded in this slot.

5. Script/action helper anims
   - Active in YR: Conditional. Evidence: constructor callers `FUN_006E1CC0`, `FUN_006E2290`, `FUN_006E2390`, `FUN_006E2C40`, `FUN_006E36E0`, `FUN_006EF610`, `FUN_00739AC0`, `FUN_00739CD0`.
   - Constructor rows: most cell/action helpers use `delay=0`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`; `FUN_006E2390` uses the warhead explosion row; `FUN_006EF610` passes loop count from a packed argument; `FUN_00739CD0` passes `reverse=1`.
   - Timing/liveness: event/script/unit-special state gated. Exact user-facing feature names were not expanded; they remain a taxonomy bucket, not a final mechanism report.

## Rust Surface Check

- `src/app_building_anim.rs` contains an app-side `AnimRuntime` slice for garrison muzzle flashes and lifecycle details such as first-AI guard, `Next`, Rate, reverse, and loop count.
- `src/app_fire_effects.rs` builds non-garrison weapon muzzle flashes and projectile visuals as app presentation structures, not as native-like `AnimClass` entities with constructor rows, owner attachment, or global runtime registration.
- `src/sim/world/mod.rs` has `world_effects: Vec<WorldEffect>` for temporary world-position SHP animations, described in-code as warp effects, explosions, and similar visuals. It is not a native `AnimClass` object model.
- `src/rules/art_data.rs` parses some AnimType runtime metadata (`Next`, loop/rate, `ZAdjust`, layer, flat/tiled/translucent/shadow/reverse), but this metadata is consumed by narrow presentation slices rather than a general spawn taxonomy.

## Implementation Handoff

1. Constructor row preservation
   - Verified behavior: active YR caller families pass materially different constructor rows (`0x600`, `0x1600`, `0x2600`, random delay, `delay=1`, `zAdjust=-30`, `reverse=1`).
   - Rust delta: introduce a generic runtime anim spawn descriptor or `AnimClass` world entity that stores the constructor tuple before family-specific post-writes.
   - Affected surface: `src/sim/world/mod.rs`, `src/sim/components`/render handoff, app render consumers, existing `world_effects`.
   - Acceptance scenario: spawning a building slot anim, warhead explosion, bouncer expire, and trailer anim records distinct constructor fields rather than collapsing them to the same temporary effect defaults.
   - Proposed test name: `anim_runtime_spawn_records_constructor_arguments_by_family`.
   - Risk: high, because silently normalizing rows changes draw flags, delay, loop lifetime, reverse playback, and depth behavior.

2. Owner attachment and post-constructor mutation
   - Verified behavior: several families call `SetOwnerObject` or mutate fields after construction: weapon owner attachment, EMP sparks, Psychic Dominator per-victim anims, unit special anims, building slot offsets/translucency/palette/current-frame preservation, damage-fire ZAdjust.
   - Rust delta: model post-constructor hooks separately from constructor defaults; attached anims must track owner lifetime/position without assuming all world anims are free-floating.
   - Affected surface: fire effects, building anim overlay code, future generic anim store, remove-listener/owner cleanup handling.
   - Acceptance scenario: an attached unit-special anim follows owner-relative coordinates and can pass reverse, while a building-slot anim keeps its slot offsets and replacement-frame behavior.
   - Proposed test name: `anim_runtime_post_constructor_owner_and_slot_mutations_are_preserved`.
   - Risk: high, because blending post-writes into generic defaults loses liveness and slot ownership semantics.

3. Runtime-spawn chain separation
   - Verified behavior: `Next=` switches the same AnimClass in place, but `TrailerAnim`, `BounceAnim`, `ExpireAnim`, voxel trailer/bounce/expire, bridge collapse, lightning, and temporal helper visuals allocate new AnimClass objects through the constructor.
   - Rust delta: implement `Next` as in-place runtime type switch and implement trailer/bounce/expire/helper paths as separate spawn events with their native rows.
   - Affected surface: `src/app_building_anim.rs` lifecycle logic, generic anim AI tick, future VoxelAnim/AnimClass bridge.
   - Acceptance scenario: an anim with `Next=` does not allocate a new world anim, while a periodic `TrailerAnim` does allocate with `delay=1` and `drawFlags=0x600`.
   - Proposed test name: `anim_runtime_next_is_in_place_but_trailer_allocates`.
   - Risk: medium-high, because treating `Next` as spawn or treating trailers as in-place both break object counts, owner pointers, sounds, and draw order.

## Negative Facts / Do Not Do

- Do not implement all runtime anim spawns as `drawFlags=0x600`. Evidence: `WarheadTypeClass::Detonate @ 0x004690B0` and bouncer/voxel expire paths use `0x2600`; `BuildingClass::CreateAnimForSlot @ 0x00451890` uses `0x1600`.
- Do not implement `Next=` as a constructor spawn. Evidence: `AnimClass::AI @ 0x00423AC0` switches the existing object in place; constructor xrefs are for trailer/bounce/expire and other independent objects.
- Do not make normal `Destroy()` spawn `ExpireAnim`. Evidence: `AnimClass::AI @ 0x00423AC0` impact/destruction branch spawns `type+0x304`; the stale chaining doc's normal-destroy shortcut is not supported by the fresh xref taxonomy.
- Do not use `g_AnimClass_Array` as the per-tick AI scheduler. Evidence: constructor registration exists at `0x00421EA0`, but prior live Ghidra report verifies ordinary AI traversal through the live object/logic vector after reveal.
- Do not collapse owner-attached and free-floating anims into the same lifetime model. Evidence: `EMPulseClass::Apply @ 0x004C54E0`, `PsychicDominator::MindControlArea @ 0x0053B080`, muzzle paths, and unit-special helper paths call `SetOwnerObject` or store owner/listener pointers.

## Remaining Uncertainty

- Exact user-facing names and standard-YR trigger coverage for some unlabeled helpers remain unexpanded: `FUN_00482900`, `FUN_006622C0`, `FUN_00663030`, `FUN_00684C30`, and parts of the `FUN_006E*` action-helper cluster.
- Teleport locomotion exact Rules offsets should be rechecked in a chrono-specific slot before using those offsets as implementation constants.
- Full `TerrainClass::Catch_Fire` and `TerrainClass::Take_Damage` constructor rows were not deeply expanded; this report classifies the terrain/overlay family but is not a terrain fire implementation contract.
- Stock content coverage for every conditional modded key (`TrailerAnim`, `BounceAnim`, `ExpireAnim`, EMP anims, unit-special reverse anims) was not exhaustively enumerated against `artmd.ini`/`rulesmd.ini`.

## Stale-Doc Replacement Wording

`docs/research/ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md`:

> Replace the old bouncer/ExpireAnim shortcut wording with: Bouncer paths have two constructor families. `AnimClass::ProcessBounceResult @ 0x00423930` spawns `type+0x300 BounceAnim` at current coords with `delay=0`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0`. `AnimClass::AI @ 0x00423AC0` impact/destruction branch spawns `type+0x304 ExpireAnim` at ftol bounce coords with `delay=0`, `loop=1`, `drawFlags=0x2600`, `zAdjust=-30`, `reverse=0`. Normal `Destroy()` still does not spawn `ExpireAnim`.

`docs/research/ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`:

> Replace any wording that implies `g_AnimClass_Array` is the ordinary tick scheduler with: AnimClass constructor appends to `g_AnimClass_Array` for class registry/maintenance, but ordinary per-tick AI for revealed anim objects is through the live `LogicClass` object vector (`ObjectClass::Reveal -> DynamicVector::Insert`; `LogicClass::PerTickUpdate @ 0x0055B608..0x0055B619` calls the virtual AI slot and reloads live count).

## Evidence Log

- Ghidra caller list for `AnimClass::Constructor @ 0x00421EA0` captured 70+ direct callers including combat, building, locomotion, superweapon, bridge, overlay, terrain, and action helpers.
- Decompile/assembly context checked for load-bearing callers: `AnimClass::AI @ 0x00423AC0`, `AnimClass::ProcessBounceResult @ 0x00423930`, `VoxelAnimClass::AI @ 0x00749F30`, `BuildingClass::CreateAnimForSlot @ 0x00451890`, `BuildingClass::CreateDamageFireAnims @ 0x0043C0D0`, `BuildingClass::DestructionEffects @ 0x004415F0`, `AircraftClass::AI @ 0x00414BB0`, `AircraftClass::ReceiveDamage @ 0x004165C0`, `InfantryClass::DoType_Sequencer @ 0x00520AE0`, `EMPulseClass::Apply @ 0x004C54E0`, `LightningStorm::CreateCloudBolt @ 0x0053A140`, `LightningStorm::GroundStrike @ 0x0053A300`, `PsychicDominator::MindControlArea @ 0x0053B080`, `TemporalClass::AI @ 0x006297F0`, `TemporalClass::Update @ 0x0071A760`, `WarpAttachClass::SpawnWarpAnims @ 0x00629E90`, `DriveLocomotionClass::Process @ 0x004B0500`, `HoverLocomotionClass::Move @ 0x00514310`, `FUN_00581140 @ 0x00581140`, `FUN_006EF610 @ 0x006EF610`, `FUN_00739AC0 @ 0x00739AC0`, and `FUN_00739CD0 @ 0x00739CD0`.
- Prior verified reports used as scoped evidence, not re-investigated: `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`, `ANIMCLASS_AI_LIFECYCLE_EXACT_SUBSET_RESWARM_20260527.md`, `ANIMCLASS_GLOBAL_OBJECT_REGISTRATION_LIFETIME_RESWARM_20260527.md`, `OCCUPANTANIM_ANIMCLASS_LIFECYCLE_DRAWIT_DEPTH_GHIDRA_REPORT.md`, `TIBTRE_ANIMCLASS_ORE_SPAWN_TICK_GHIDRA_REPORT.md`, `VOXELANIMCLASS_GHIDRA_REPORT.md`.
- Rust touchpoints read: `src/app_building_anim.rs`, `src/app_fire_effects.rs`, `src/sim/world/mod.rs`, `src/rules/art_data.rs`.
