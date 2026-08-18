# GI Non-Garrison Muzzle FLH Origin Design

## Goal

Spawn non-garrison weapon muzzle `Anim=` effects and place `Report=` sound at the documented FLH fire origin when a GI shot actually fires, without changing combat damage timing or garrison muzzle behavior.

## Architecture Context

Combat already owns the authoritative "a shot fired" moment. `src/sim/combat/mod.rs` emits `SimFireEvent` when damage/projectile output is spawned; after the infantry fire-frame sync, standing, prone, and deployed GI fire events are produced on the correct animation frame.

The app layer drains `sim.fire_events` into `AppState.pending_fire_effects` in `src/app_sim_tick.rs`. Today only garrison fire consumes those events: `src/app_building_anim.rs` converts events with `garrison_muzzle_index` and `occupant_anim` into `GarrisonMuzzleFlash`, and `src/app_instances/overlays.rs` renders those flashes at building `MuzzleFlashN` pixel offsets.

Rules/art data needed for this feature already exists. `WeaponType::anim` parses weapon `Anim=`, and `ArtEntry` parses `PrimaryFireFLH`, `SecondaryFireFLH`, `ElitePrimaryFireFLH`, and `EliteSecondaryFireFLH`. `rules::flh::resolve_flh` and `util::flh_transform::flh_to_screen_offset` exist, but are currently test-only in production terms.

The boundary is important: `sim/` may emit deterministic fire facts, but must not depend on render, audio, UI, sidebar, or net. App/render/audio may resolve SHP assets, screen-space offsets, and sound positions from those facts.

## Impact Analysis

This changes the event contract between sim and app/render. `SimFireEvent` needs to carry enough immutable facts from the firing tick so the app does not infer combat state later from a possibly changed entity.

Likely touched modules:

- `src/sim/world/mod.rs` - extend `SimFireEvent` with non-garrison fire facts.
- `src/sim/combat/mod.rs` - populate those facts at the exact fire moment and keep garrison fields unchanged.
- `src/render/sprite_atlas.rs` - collect/load weapon `Anim=` SHPs in addition to `OccupantAnim`.
- `src/app.rs` / `src/sim/components.rs` - add app-owned active non-garrison muzzle flash state, parallel to garrison flashes.
- `src/app_building_anim.rs` or a new app-level effects helper - resolve FLH, select muzzle anim, spawn/advance active muzzle flashes.
- `src/app_instances/overlays.rs` - build sprite instances for active non-garrison muzzle flashes.
- `src/app_sim_tick.rs` - route weapon report sound through the same resolved fire origin when available.

Risk areas:

- Event ordering: the muzzle flash and report sound must use the same tick as the combat fire event, not a later inferred state.
- Facing mapping: weapon `AnimCount == 8` uses a documented 8-way formula that differs from FLH's 32-way position quantization.
- Layering: all FLH-to-screen math and SHP decisions must stay above `sim/`.
- Garrison regression: `OccupantAnim` fire ports must remain separate from non-garrison weapon `Anim=`.
- Asset loading: weapon `Anim=` effects must be loaded into the atlas without breaking existing warhead, damage-fire, particle, parachute, and garrison effect loading.

## Chosen Approach

Use Approach A: sim emits fire facts, and app/render/audio resolve FLH/render/audio output.

`SimFireEvent` remains the bridge from deterministic combat to presentation. Combat records stable firing-tick facts such as attacker id, attacker type id, weapon id or slot, facing, veterancy, and target. It does not choose SHP atlas entries, compute screen-space offsets, or create app sound events.

The app layer consumes each fire event once per sim tick. For non-garrison events, it resolves the firing entity's art entry, weapon entry, effective FLH, 8-way muzzle anim name, total effect frame count, and screen origin. It then spawns an app-owned one-shot muzzle flash and positions the `Report=` sound at that same origin. For garrison events, the existing `OccupantAnim` + `MuzzleFlashN` path remains the owner.

This approach avoids re-deriving combat decisions in render while preserving architecture boundaries. It also makes the fire event a stable snapshot, so the muzzle visual does not drift if the entity moves, changes weapon state, or dies after the firing tick.

## Tiny-Detail Ledger

- The muzzle effect is spawned only when combat actually fires, so infantry standing/prone/deployed shots inherit the fixed fire-frame timing. Source: `docs/plans/2026-05-16-infantry-fire-frame-sync-design.md`; `docs/gap-scans/2026-05-16-disparity-scan-gi-infantry-fire-sync.md`.
- GI primary standing/prone uses the selected primary weapon visual output and `PrimaryFireFLH=80,0,105`. Source: `ini/artmd.ini:281-289`; `GI_GHIDRA_REPORT.md:735-744`.
- GI deployed fire uses the `DeployedFire` visual sequence timing, but weapon choice remains target-driven; secondary Para uses `SecondaryFireFLH=80,0,90` when secondary is selected. Source: `GI_GHIDRA_REPORT.md:2507-2526`, `:2856-2865`; `ini/artmd.ini:288-289`.
- M60 and Para both define `Anim=MGUN-N,MGUN-NE,MGUN-E,MGUN-SE,MGUN-S,MGUN-SW,MGUN-W,MGUN-NW` and `OccupantAnim=UCFLASH`. Non-garrison uses `Anim=`, garrison uses `OccupantAnim`. Source: `ini/rulesmd.ini:22922-22942`; `GI_GHIDRA_REPORT.md:35-38`.
- If weapon `AnimCount == 8`, gamemd selects a directional anim from facing; otherwise it uses the first configured anim when the count is positive. Source: `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md:64-72`.
- The selected weapon muzzle anim is spawned at `muzzleCoords`, the result of `TechnoClass::GetFLH`. Source: `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md:138-150`, `:303-330`.
- `Report=` sound is played at the same `muzzleCoords`, not the unit cell origin. Source: `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md:288-301`.
- FLH selection respects primary/secondary and elite override fields. Source: `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md:380-383`; `src/rules/flh.rs`.
- FLH position uses finer facing quantization than the 8-way muzzle anim selection. Source: `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md:421-424`.
- Vanilla YR does not use per-burst FLH keys; burst lateral flip exists but is not GI-visible because GI FLH lateral values are zero. Source: `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md:395-412`; `ini/artmd.ini:288-289`.
- App-owned muzzle flashes should be one-shot effects removed when their SHP frames finish, matching current garrison flash lifecycle. Source: current pattern in `src/app_building_anim.rs`.
- Sim remains deterministic and render-agnostic; no floats or render/audio dependencies are introduced into `sim/`. Source: `AGENTS.md`.

## Design

### Components

**Fire event snapshot**

Extend `SimFireEvent` so non-garrison consumers do not need to rediscover the firing state. The event should include attacker id, attacker type id, weapon slot, selected weapon id, facing, veterancy, target, and existing garrison fields. If the existing `weapon_slot` plus attacker type is sufficient to resolve the exact weapon for all current paths, keep selected weapon id out; if garrison/open-topped/IFV paths can change the selected weapon independently, include the resolved weapon id to avoid render-side guessing.

The event is transient and drained each tick, so this is not new persistent sim state and does not require state-hash changes.

**Non-garrison muzzle flash runtime**

Add an app-owned active flash struct parallel to `GarrisonMuzzleFlash`, but world/screen anchored for non-garrison weapon fire. It should store:

- attacker id, for optional attachment/follow and lighting lookup;
- SHP/effect name as an interned id or string matching existing app patterns;
- spawn screen/world position resolved from the firing tick;
- current frame, total frames, elapsed ms, and rate ms;
- depth/sort inputs needed to draw consistently with the firing entity.

For the first implementation, the spawn position should be fixed from the firing tick. The docs note anim ownership/attachment for TechnoClass muzzle flashes, but the practical GI muzzle flash is short-lived. If visual testing shows drift during moving vehicle fire, attachment can be revisited for non-infantry. GI standing/prone/deployed validation is not blocked by that because the firing infantry is stationary during the fire frame.

**FLH origin resolver**

App-layer helper resolves a non-garrison fire event into a render/audio origin:

1. Resolve attacker entity from `attacker_id`.
2. Resolve object image/art entry from rules object `Image=` fallback pattern already used by garrison muzzle flashes.
3. Resolve weapon entry from event-selected weapon id or slot.
4. Resolve effective FLH via `rules::flh::resolve_flh`.
5. Transform FLH to screen offset using current helper or a corrected helper if write-plan review finds mismatch with the documented 32-way gamemd quantization.
6. Add offset to the firing entity's cached screen position for render/audio screen position.

This helper belongs above `sim/`, most likely app/effects code, because it uses art metadata, screen positions, and floating-point render coordinates.

**Directional weapon anim selector**

Add a pure helper for selecting a weapon muzzle anim from `WeaponType::anim` and entity facing:

- empty list: no non-garrison muzzle flash;
- one or more non-8 entries: select the first entry;
- exactly 8 entries: select the documented 8-way facing-index result.

The helper should be unit-tested independently. The write-plan should include a targeted review of Rust's `u8` facing convention against the documented gamemd facing formula before implementation; the selected formula must be centralized so future vehicle and aircraft muzzle flashes use the same rule.

**Atlas loading**

Extend `sprite_atlas` effect collection to include every weapon `Anim=` entry, not just `OccupantAnim`. Keep de-duplication case-insensitive like existing effect collection.

**Sound event handling**

Move report sound screen positioning to the same resolved fire origin. The cleanest architecture is to keep `SimSoundEvent::WeaponFired` as the authoritative sound cue, but attach enough correlation data or computed sim-space origin data so `app_sim_tick` can place it at the same origin as the corresponding `SimFireEvent`.

Preferred design: include `report_sound_id` on `SimFireEvent` and let app consume the sound cue while resolving the non-garrison muzzle origin. That removes the current duplicate cell-origin `fire_sounds` path for weapon reports and ensures visual/audio use one origin. If a later implementation needs nonvisual report-only weapons, the event still carries report id even when `Anim=` is empty.

Garrison report sound placement should be evaluated separately: this design may keep existing garrison sound behavior initially if changing it would require building fire-port world origin semantics. Non-garrison GI report sound must use FLH origin.

### Interfaces / Contracts

- `SimFireEvent` becomes the single transient event for weapon fire presentation: muzzle visuals and weapon report cues.
- Existing garrison consumers keep using `garrison_muzzle_index` and `occupant_anim`.
- Non-garrison consumers ignore events with `garrison_muzzle_index.is_some()`.
- App-owned active flash state is presentation-only and not serialized into sim.
- `sprite_atlas` loads weapon `Anim=` names as effect SHPs.

### Data Flow

1. Combat reaches the exact fire moment.
2. Combat applies damage/projectile output as it does today.
3. Combat emits one `SimFireEvent` containing firing-tick facts and optional `report_sound_id`.
4. `app_sim_tick` drains fire events.
5. Existing garrison path spawns `OccupantAnim` flashes for garrison events.
6. New non-garrison path resolves FLH and weapon `Anim=`.
7. New path spawns one active muzzle flash if a muzzle anim exists.
8. New path emits/queues `GameSoundEvent::WeaponFired` at the resolved FLH origin if `report_sound_id` exists.
9. Overlay instance builder renders active non-garrison muzzle flashes until their frames complete.

### Error Handling

Missing attacker, missing art entry, missing weapon, missing SHP atlas entry, or empty `Anim=` should skip the visual without panicking. Missing visual data must not suppress damage. Missing report sound id means no sound event. If FLH data is absent in INI, parsed defaults already provide `0,0,0`.

### Testing Strategy

Unit tests:

- Weapon muzzle anim selector: empty list, one entry, non-8 multi-entry, 8-way directional selection.
- FLH resolver integration: primary vs secondary and elite override selection.
- Combat event emission: standing GI primary event carries primary slot/weapon/facing/veterancy/report; deployed GI secondary event carries secondary slot/weapon while preserving deployed visual timing from the previous fire-frame tests.
- Garrison event regression: garrison fire still carries `occupant_anim` and does not use non-garrison weapon `Anim=` path.

App/render tests where feasible:

- Atlas collection includes `M60` / `Para` `Anim=MGUN-*` entries.
- Non-garrison fire event with GI art/rules resolves to the expected primary or secondary FLH.
- Report sound conversion uses resolved FLH origin for non-garrison events.

Manual/live verification:

- Standing GI rifle shot shows `MGUN-*` on the rifle fire frame.
- Prone GI rifle shot shows `MGUN-*` on the prone fire frame.
- Deployed GI shot shows `MGUN-*` on `DeployedFire`, with target-driven weapon choice unchanged.
- Report sound remains synchronized with the muzzle flash.

## Architectural Decisions

- **Sim emits facts, not render output.** This follows the existing `SimFireEvent` pattern and preserves the sim/render boundary.
- **Non-garrison and garrison muzzle flashes stay separate.** Garrison uses building fire-port pixel offsets and `OccupantAnim`; non-garrison uses weapon `Anim=` and FLH.
- **Report sound rides the fire event.** This avoids two independent origin-resolution paths for the same shot.
- **No GI-specific hardcoding.** GI remains the validation case, but all behavior comes from weapon/art/rules data.
- **No new persistent sim state.** Fire events are transient; state hashing is not affected unless implementation later adds a pending or persistent fire-output state.

Tech debt to avoid during implementation:

- Do not use current entity weapon state in render to choose which weapon fired; use event facts.
- Do not compute screen-space FLH in `sim/`.
- Do not load only `MGUN-*`; collect all weapon `Anim=` entries generically.

## Alternatives Considered

### B. Sim resolves muzzle anim and FLH output

This would make combat emit a more complete muzzle event. It was rejected because it pushes art/render/audio concerns toward `sim/` and increases the risk of floating-point or asset decisions in gameplay logic.

### C. Render infers everything from current entity state

This would keep `SimFireEvent` minimal. It was rejected because render could observe the entity after it moved, died, retargeted, changed weapon state, or changed facing. That creates visible drift and repeats combat decision logic outside combat.

### Visual-only pass

This would implement the muzzle flash but keep report sound at unit cell origin. It was rejected because the docs place both visual muzzle anim and `Report=` sound at `muzzleCoords`; one origin resolver should serve both outputs.
