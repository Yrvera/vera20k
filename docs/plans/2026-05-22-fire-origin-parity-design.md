# Fire Origin Parity Design

## Goal

Make non-garrison and building weapon fire use one gamemd-shaped fire-origin contract so muzzle flashes, report sounds, and projectile visuals share the same fire-tick world source.

## Architecture Context

Current fire flow is `sim/combat` -> `SimFireEvent` -> app fire presentation:

- `src/sim/combat/mod.rs` resolves firing and emits `SimFireEvent`.
- `src/sim/world/mod.rs` owns the event type and `Simulation::fire_events`.
- `src/app_sim_tick.rs` drains fire events after `advance_tick`.
- `src/app_fire_effects.rs` resolves FLH, report sound screen position, muzzle flash, and render-only projectile visuals.
- `src/app_building_anim.rs` separately consumes garrison fire events for `MuzzleFlashN` overlays.

This boundary is correct. `sim/` should keep emitting deterministic fire-tick facts; app/render/audio should keep turning those facts into presentation objects. The mismatch is that `SimFireEvent` does not carry enough fire-tick source data, so app code recomputes a screen-only offset from current entity state and leaves the source cell/elevation at the attacker cell.

Relevant source reports:

- `FLH_TURRET_AND_VISUAL_OFFSETS_GHIDRA_REPORT.md`
- `BURST_WEAPON_FIRING_GHIDRA_REPORT.md`
- `traces/FLH_MUZZLE_WORLD_REFERENCE_TRACE.md`

## Impact Analysis

Touched modules:

- `src/sim/world/mod.rs`: add a fire-origin snapshot to `SimFireEvent`.
- `src/sim/combat/mod.rs`: populate the snapshot before burst/cooldown updates.
- `src/sim/combat/combat_targeting.rs`: use existing attacker snapshot fields as the source of fire-tick position/facing/burst data.
- `src/rules/art_data.rs`: parse missing building fire-origin keys.
- `src/app_fire_effects.rs`: replace screen-only FLH resolver with a shared world-origin resolver.
- `src/app_building_anim.rs`: move garrison muzzle placement onto the same origin contract, or call the same helper.
- `src/app_instances/overlays.rs`: should keep consuming already-resolved flash positions.

Risk areas:

- Burst index must represent the pre-shot `CurrentBurstIndex` equivalent, not the post-shot updated state.
- App code must not look up current entity position when a fire-tick source is already in the event.
- Building fixed pixel offsets must be treated as isometric pixel pairs converted to world X/Y, not as final screen decorations.
- Keep `sim/` free of render/audio dependencies. Rules/art metadata can be read in the app resolver unless and until a future real projectile sim needs authoritative source coordinates.

## Chosen Approach

Use a fire-origin snapshot on `SimFireEvent`, then resolve a shared app-side `FireWorldOrigin` from fire-tick facts plus rules/art metadata.

This preserves the existing architecture: `sim/` emits facts, app layer resolves visual/audio placement. It also gives all consumers the same source point, matching gamemd's `Fire_At` pipeline where bullet launch, muzzle anim, report sound, lasers, waves, and related effects share the `GetFLH` coordinate.

## Tiny-Detail Ledger

- `TechnoClass::GetFLH` returns a world/lepton `CoordStruct`, not a screen offset. Source: `FLH_TURRET_AND_VISUAL_OFFSETS_GHIDRA_REPORT.md`, `0x006F3AD0`.
- `TechnoClass::Fire_At` computes source before bullet allocation, muzzle anim, and report sound. Source: same report, `0x006FDD50`.
- Normal FLH uses 32-way facing quantization: `((((facing >> 10) + 1) >> 1) & 0x1F) - 8`. Source: `FLH_TURRET...`.
- Burst lateral sign comes from `CurrentBurstIndex +0x3B8`: odd uses positive lateral, even uses negative lateral. Source: `FLH_TURRET...`.
- `CurrentBurstIndex` is pre-shot `0` for first shot and `1` for second shot in `Burst=2`, then wraps after `Fire_At`. Source: `BURST_WEAPON_FIRING_GHIDRA_REPORT.md`.
- `PrimaryFirePixelOffset` / `SecondaryFirePixelOffset` are isometric pixel pairs converted to world X/Y by `IsometricPixelToWorld`. Source: `FLH_TURRET...`, `0x00453840`, `0x006D2070`.
- Missing building fire pixel offset sentinel is `0xFFFF,0xFFFF`. Source: `FLH_TURRET...`, `0x0045DE40..52`.
- `PrimaryFireDualOffset` adds primary pixel offset to the generic FLH source for its building branch. Source: `FLH_TURRET...`.
- Building voxel turret/barrel origins use `GetTurretDrawPosition`, not the existing unit `TurretOffset` screen helper. Source: `FLH_TURRET...`, `0x00453BF0`.
- Garrison fire ports are isometric pixel offsets converted to world leptons and added to building render coords. Source: `FLH_TURRET...`.
- `BuildingClass::GetRenderCoords` shifts building coords by `-0x80` X/Y. Source: `FLH_TURRET...`.
- Exact final matrix/projection pixel equality remains partially unchecked. Source: `traces/FLH_MUZZLE_WORLD_REFERENCE_TRACE.md`.

## Design

### Components

Add a small sim-side snapshot type:

```rust
pub struct FireOriginSnapshot {
    pub rx: u16,
    pub ry: u16,
    pub sub_x: SimFixed,
    pub sub_y: SimFixed,
    pub z: u8,
    pub facing: u8,
    pub category: EntityCategory,
    pub burst_index: u8,
}
```

`SimFireEvent` gets `source: FireOriginSnapshot`.

Add an app-side resolved origin type:

```rust
struct FireWorldOrigin {
    rx: u16,
    ry: u16,
    sub_x: SimFixed,
    sub_y: SimFixed,
    z: u8,
    screen_x: f32,
    screen_y: f32,
}
```

Add resolver functions in `app_fire_effects.rs`, or a small `app_fire_origin.rs` if the file grows:

- `resolve_fire_world_origin(sim, rules, art_registry, event) -> Option<FireWorldOrigin>`
- `resolve_techno_flh_origin(...)`
- `resolve_building_flh_origin(...)`
- `iso_pixel_to_world_delta(...)`
- `project_fire_origin(...)`

### Interfaces / Contracts

`SimFireEvent` must carry all data that can change after the fire tick:

- source cell and sub-cell
- source elevation
- facing
- burst index
- weapon slot
- target
- garrison muzzle index

App resolution may read static metadata:

- rules object type
- art entry
- weapon/projectile entries
- effect frame counts

It must not use the current entity's position to derive source coordinates except as a fallback for legacy saves/tests that construct events manually.

### Data Flow

1. Combat snapshot already has `pos_rx`, `pos_ry`, `sub_x`, `sub_y`, `facing`, `category`, `burst_remaining`.
2. At fire emission, compute `burst_index_at_fire`.
3. Store `FireOriginSnapshot` on `SimFireEvent`.
4. App drains event and resolves one `FireWorldOrigin`.
5. Muzzle flash, report sound, and projectile visual all consume that one origin.
6. Garrison path either reuses the resolver or is changed to create a `GarrisonMuzzleFlash` from the resolved origin instead of raw screen pixel offsets.

### Burst Index Mapping

Current Rust stores `burst_remaining`, not `CurrentBurstIndex`. For `weapon.burst = N`:

- If `snap.burst_remaining == 0`, this is the first shot, so `burst_index = 0`.
- Otherwise, `burst_index = N - snap.burst_remaining`.

This matches the pre-shot index sequence for ordinary burst fire. It must be tested with `Burst=2` and `Burst=3`.

### Origin Resolution Rules

Normal unit/techno:

- Select FLH by weapon slot and veterancy using existing `rules::flh::resolve_flh`.
- Apply 32-way quantization.
- Flip lateral sign from `burst_index`.
- Produce a world delta and add to source render coords.
- Project to screen once.

Building:

- Parse and store `PrimaryFirePixelOffset`, `SecondaryFirePixelOffset`, and `PrimaryFireDualOffset`.
- If garrison muzzle index is present, convert `MuzzleFlashN` isometric pixel pair to world delta and add to building render coords.
- If fixed fire pixel offset exists, convert the selected pixel pair to world delta and add per the verified `BuildingClass::GetFLH` branch.
- If building voxel turret/barrel branch applies, use a dedicated building turret origin helper. If required metadata is not yet parsed, mark that branch `UNKNOWN` in code comments/tests rather than falling back silently.
- Otherwise fall back to generic techno FLH.

Projection:

- `FireWorldOrigin` should preserve sub-cell source for projectile visual start/duration/frame calculations.
- Screen projection should be derived from the same world source used for sound and muzzle flash.

### Error Handling

- If metadata is missing, skip the specific presentation effect rather than inventing a source.
- For tests that construct old minimal `SimFireEvent` values, prefer fixture helper constructors rather than production fallbacks.
- Log at debug/trace level for missing art metadata, not warn spam during normal play.

### Testing Strategy

Focused unit tests:

- `burst_index_first_and_second_shot_for_burst2`
- `fire_event_snapshots_fire_tick_position_not_post_tick_position`
- `normal_flh_origin_uses_world_source_not_attacker_cell`
- `normal_flh_burst2_alternates_lateral_side`
- `muzzle_flash_sound_projectile_share_same_fire_origin`
- `primary_fire_pixel_offset_sentinel_absent`
- `building_primary_fire_pixel_offset_converts_iso_pixels_to_world`
- `garrison_muzzle_flash_uses_shared_fire_origin`
- `projectile_visual_start_rx_ry_subcell_comes_from_fire_origin`

Regression tests should update existing `SimFireEvent` fixtures in `combat_tests` and `app_fire_effects` to assert the new snapshot fields.

Manual parity check:

- APOC/MTNK primary fire facing east: two-shot burst alternates muzzle side.
- GI standing fire: muzzle and report sound use same projected FLH point.
- Garrisoned building: fire ports remain at the correct building offsets.
- Fixed-origin building weapon, if stock fixture exists: projectile starts from fixed pixel origin.

## Architectural Decisions

- Keep fire-origin resolution app-side for now. This avoids making `sim/` depend on art/render metadata.
- Put fire-tick mutable data in `SimFireEvent`; app metadata lookup is allowed only for static rules/art fields.
- Use one origin object for all presentation consumers to avoid muzzle/sound/projectile drift.
- Do not implement a real `BulletClass` in this change. The render-only projectile visual can be improved without expanding simulation ownership.

## Alternatives Considered

### Compute complete fire origin in sim

This has the strongest fire-tick correctness and would prepare for sim-owned projectile flight. Rejected for this step because it would require threading more art/building presentation metadata into `sim/`, increasing the layering risk.

### App-only visual patch

This would keep the event small and recompute from current entity state. Rejected because it preserves the core parity bug: the attacker can move, rotate, die, or update burst state before app presentation resolves the source.

### Separate fixes for muzzle, sound, projectile, and garrison

Rejected because gamemd uses the same source coordinate for all of them. Separate fixes would invite future drift.

