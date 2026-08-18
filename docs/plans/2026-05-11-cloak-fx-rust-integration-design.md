---
title: Cloak FX Rust Integration — Design
status: awaiting approval
---

# Cloak FX Rust Integration — Design

## Goal

Implement the gamemd.exe cloak system and Mirage tree-disguise system in Rust as
Phase 2 of the voxel GPU remap & FX work, producing player-observable parity
with gamemd for cloak fade animation, allied shimmer pulse, sensor-revealed
cloaks, and Mirage Tank tree-disguise sprite swap.

## Architecture Context

### How the existing Rust sim layer is shaped

The Rust engine uses inline `Option<Component>` slots on a unified
[`GameEntity` struct](src/sim/game_entity.rs#L48) — no separate ECS. Components
are defined in [src/sim/components.rs](src/sim/components.rs) (749 lines, no
subdirectory). New components are added as inline fields with their types
defined alongside existing structs in `components.rs`.

The sim tick orchestrator at
[src/sim/world/mod.rs:970](src/sim/world/mod.rs#L970) runs `advance_tick()`
through a fixed sequence. Key stages relevant to cloak:
- Step 6 (line 1140): `refresh_fog()` — visibility/sensor computation
- Step 7 (line 1146): `power_system::tick_power_states()`
- Step 10 (line 1180): `combat::tick_combat_with_fog()` — damage application
- Step 13 (line 1347): `combat::tick_retaliation()` then `passenger::tick_passenger_system()`

State determinism is enforced by
[src/sim/world/world_hash.rs:18](src/sim/world/world_hash.rs#L18), which calls
`hash_entities()` over `GameEntity` in stable_id order. New entity-level fields
must be hashed conditionally.

### Existing FX plumbing

[src/render/batch.rs:42](src/render/batch.rs#L42) defines `SpriteInstance` with
FX fields already wired to GPU vertex attributes 7-10: `house_color_idx`,
`fx_flags`, `fx_params: [f32;4]`, `ic_tint: [f32;4]`. The fragment shader
[src/render/sprite_voxel_shader.wgsl:96-109](src/render/sprite_voxel_shader.wgsl#L96-L109)
has a stubbed `apply_fx()` with `(flags & 1u) != 0u → c.a *= params.x` as the
cloak branch.

### Reusable mechanism: `display_type_override`

[src/sim/game_entity.rs:199](src/sim/game_entity.rs#L199) already defines
`pub display_type_override: Option<InternedId>` — used by the miner unload-class
swap. Mirage tree-disguise will reuse this exact mechanism.

### Audio dispatch

[src/sim/world/mod.rs:94-164](src/sim/world/mod.rs#L94) defines `SimSoundEvent`
enum + `Simulation.sound_events: Vec<...>` drained per frame. New variants
plug into the existing pattern.

### Deterministic RNG

[src/sim/rng.rs:8](src/sim/rng.rs#L8) exposes `SimRng` on `Simulation`, with
`next_range_u32_inclusive(low, high)` mirroring gamemd's
`Random__RandomRanged`.

### Visibility flags

[src/sim/vision/mod.rs:38](src/sim/vision/mod.rs#L38) defines per-cell flags
`FLAG_REVEALED | FLAG_VISIBLE | FLAG_GAP_COVERED`. Cloak does NOT modify cell
flags — it adds a per-entity visibility predicate that composes with these
when the entity is queried at render time.

### Existing INI parser patterns

- [`ObjectType::from_ini_section`](src/rules/object_type.rs#L673):
  `section.get_bool("Key").unwrap_or(default)`,
  `section.get_int("Key").unwrap_or(default)`.
- [`GeneralRules::from_ini`](src/rules/ruleset.rs#L733): `general.get_bool(...)`,
  `audio_visual.get(...)`, comma-list parsing via `.split(',')`.

## Impact Analysis

| File | Change |
|---|---|
| [src/sim/components.rs](src/sim/components.rs) | Add `Cloak` + `Disguise` component structs |
| [src/sim/game_entity.rs](src/sim/game_entity.rs) | Add `cloak: Option<Cloak>`, `disguise: Option<Disguise>` fields. Both `Default::default()` to None. |
| [src/sim/world/mod.rs](src/sim/world/mod.rs) | Insert `tick_cloak()` after combat (step 10.5); insert `tick_disguise()` after cloak (step 10.7). Add `SimSoundEvent::CloakSound` variant. |
| New: `src/sim/cloak.rs` | `tick_cloak()` state machine, `CloakDecloakTrigger` API for combat to fire decloak events. |
| New: `src/sim/disguise.rs` | `tick_disguise()` Mirage 8-tick scan, damage-breaks-disguise hook. |
| [src/sim/world/world_hash.rs](src/sim/world/world_hash.rs) | Hash `cloak` + `disguise` Option fields in `hash_entities()`. |
| [src/sim/combat/](src/sim/combat) | On damage application, call `cloak::on_damage()` to set decloak flag. On any damage to Mirage, call `disguise::on_damage()` to break disguise (unless PermaDisguise). On weapon-fire, call `cloak::on_fire()` if `weapon.decloak_to_fire`. |
| [src/rules/object_type.rs](src/rules/object_type.rs) | Parse new keys: `cloakable`, `cloaking_speed`, `cloak_stop`, `invisible`, `sensors`, `sensors_sight`, `disguise_when_still`, `perma_disguise`, `detect_disguise`, `detect_disguise_range`. |
| [src/rules/ruleset.rs](src/rules/ruleset.rs) | Add to `GeneralRules`: `cloaking_stages: u32`, `cloak_sound: Option<InternedId>`, `default_mirage_disguises: Vec<InternedId>`. |
| [src/app_instances/units.rs](src/app_instances/units.rs) | Per-entity: read `entity.cloak`, compute visual_state via the recipe table, compute allied shimmer alpha if applicable, populate `fx_flags` bit 0 and `fx_params[0]`. Read `entity.disguise` to decide sprite-key override (route through SHP path if disguised tree). |
| [src/render/sprite_voxel_shader.wgsl](src/render/sprite_voxel_shader.wgsl) | Extend `apply_fx` cloak branch: replace flat alpha with dither formula `val = clamp((abuf_hash * intensity * 254) / 32258, 0, 254)` where abuf_hash is a per-fragment hash. |
| [src/app_audio/](src/app_audio) | Dispatch `SimSoundEvent::CloakSound` to play `CloakSound` Voc at unit position. |
| [src/render/sprite_atlas.rs](src/render/sprite_atlas.rs) (or equivalent) | Mirage tree-disguise needs SHP frame lookup for tree OverlayType. Verify the existing OverlayType→SHP path works for disguised render. |

**Blast radius**:
- All voxel-rendered units get FX uniforms populated (zero-init for non-cloaked). Negligible perf impact.
- Combat damage path gains a cloak-decloak side-effect call. Cohesive, no API change.
- Visibility queries gain a per-entity cloak filter at render time. Render-side only — no sim impact.
- Mirage Tank rendering routes through `display_type_override` when disguised, hitting the SHP path. Existing infrastructure.

**Determinism**:
- `tick_cloak` calls `sim.rng.next_range_u32(0, 99)` for the 4% auto-cloak and 10% abort-uncloak chances. Order of entity iteration must be deterministic (BTreeMap already enforces this).
- `tick_disguise` calls `sim.rng.next_range_u32(0, count-1)` for tree picking. Same determinism.
- State hash includes new fields. Existing `hash_entities()` extension.
- No floats in sim logic — `ftol` formula computed as `(progress * 256 + half_stages) / stages` integer division. **Verify against gamemd's FIDIV → FMUL → FTOL truncation behavior.**

**Migration**: None. No on-disk state. Atlas is theater-keyed, not cloak-keyed.

## Tiny-Detail Ledger

Every implementation must reproduce these gamemd.exe observable details. Sources cited.

### Cloak state machine
- **CloakingStages = 9 default** [GHIDRA RulesClass+0x628, ini/rulesmd.ini]
- **Per-type CloakingSpeed**: SUB=1, DLPH=1, SQD=5 [ini/rulesmd.ini]
- **Default CloakingSpeed = 0 if missing** — must be clamped to ≥1 to prevent divide-by-zero [doc §9, UNKNOWN — verify clamp in gamemd or in our parser]
- **Cloak state enum: 0=Uncloaked, 1=Cloaking, 2=Cloaked, 3=Uncloaking** [doc §3.1]
- **CloakStepDelta: +1 cloaking, -1 uncloaking** [doc §3.3]
- **Uncloak starts at Progress = CloakingStages - 1**, NOT Stages [doc §3.3]
- **CloakStepTimer.duration = CloakingSpeed** [doc §3.3]
- **State 1→2 transition triggers** when visual_state hits 3 OR 5 (NOT 4) [doc §3.2]
- **State 3→0 transition triggers** when visual_state hits 0 [doc §3.2]
- **State 1 abort-uncloak chance: 10%** when visual_state hits 2 AND health < ConditionRed [doc §3.2]
- **State 0 auto-cloak chance: 4% per-tick** when CanAutoCloak fires AND health < ConditionRed [doc §3.2]
- **ReCloakDelayTimer set on state 3→0 transition** [doc §3.2] — prevents instant re-cloak after forced uncloak
- **CloakSound plays at 0→1 and 2→3 transitions** (suppressed via internal second-arg=1 path) [doc §3.3]

### Visual state mapping
- **`visual_raw = ftol(Progress / CloakingStages * 256.0)`** — division/multiplication in that order, truncate cast [doc §4.1, GHIDRA 0x00703A79]
- **Threshold ladder**: `<0x40 → 1`, `<0x80 → 2`, `<0xC0 → 3`, `<0xFF → 4`, `>=0xFF → 5` [doc §4.3]
- **Discovered-clamp**: `param_2==0 && IsDiscovered != 0 && visual_raw >= 0xC0 → return 3` [doc §4.2]
- **Progress=0 short-circuit**: returns visual_state 0 (opaque) even when CloakState!=0 [doc §4.1]
- **CloakState==0 short-circuit**: returns 0 immediately [doc §4.1]
- **Buildings always return 0** (WhatAmI()==6) [doc §4.1] — building cloak is dead in YR
- **In-editor short-circuit**: g_IsMapEditor != 0 → return 0 [doc §4.1]
- **Cloaked (state 2) viewer logic**:
  - Sensor cover (cell.sensor_count > 0 for viewer) → state 3 [doc §4.1]
  - Discovered flag (+0x41A) → state 3 [doc §4.1]
  - Otherwise enemy → state 5; allied → state 3 [doc §4.1]
- **Invisible=yes flag** (TechnoTypeClass+0xC9A): if `Invisible && !discovered && !editor` → state 5 [doc §4.1]
  - No retail YR type sets Invisible=yes. Code-live, data-dormant.

### Allied shimmer (player-owned cloaked unit pulse)
- **Phase formula**: `phase = (frame - shimmer_phase_base + 0x40) & 0xFF` [doc §5.1]
- **256-tick cycle**, deterministic via `g_CurrentFrameCounter` [doc §5.1]
- **Suppression timer (+0x1EC/+0x1F4) DORMANT** — duration=0 in retail YR [doc §5.3]
- **Pulse bands** [doc §5.2 — CORRECTION to prior reports]:
  - [0x00, 0x3F] opaque (64 frames)
  - [0x40, 0x43] shimmer (4)
  - [0x44, 0x4B] 50% blend (8)
  - [0x4C, 0x4F] shimmer (4)
  - [0x50, 0x6F] opaque (32)
  - [0x70, 0x73] shimmer (4)
  - [0x74, 0x7B] 50% blend (8)
  - [0x7C, 0x7F] shimmer (4)
  - [0x80, 0xFF] opaque (128)

### Shimmer dither (per-pixel transparency in shader)
- **Intensity formula**: `intensity = clamp((scale * 261) >> 11, 0, 254)` where scale is per-instance transparency (1000 = default) [doc §6, GHIDRA blitters]
- **LUT formula**: `val = clamp((abuf * intensity * 254) / 32258, 0, 254)` [doc §6.5, GHIDRA FUN_00420140]
- **Final palette index = `(val << 8) | source_byte`** — VPL-style 2-byte index [doc §6.5]
- **Color 0 = transparent invariant** [doc §6.5]
- **VXL state 4 brightness = 75/25 blend** (NOT 50/50!) [doc §7.3-7.5 — CORRECTION]
- **SHP state 4 = 50/50 blend** (no brightness variant for SHP) [doc §7.2]
- **State-1 blend = 75/25** [doc §6.1]
- **State-2/3 blend = 50/50** [doc §6.2]
- **For Path A shader**: per-fragment abuf_hash drives dither pattern, intensity = state-derived

### Mirage Tank tree-disguise
- **GetDisplayType branches on alliance**: allied or own → real TypeClass (+0x6C4); enemy → disguised (+0x518) [doc §8]
- **Disguise-pick interval = every 8 ticks** (`frame_counter & 7 == 0`) [doc §8]
- **Scan: 8 surrounding cells + own cell** for any enemy unit (HouseClass::Is_Ally_ByObject) [doc §8]
- **Disguise pick on no-enemy AND not-moving**: `RandomRanged(0, count-1)` from `RulesClass+0xFFC[]` (DefaultMirageDisguises) [doc §8]
- **DefaultMirageDisguises default = TREE01..TREE04** [ini/rulesmd.ini]
- **Tree TypeClass written to +0x518** (= disguised type pointer) [doc §8]
- **+0x1DC also written to g_CurrentFrameCounter** on pick (shared with cloak shimmer phase; observably moot in retail since Mirage isn't Cloakable=yes) [doc §5.4, §8]
- **Disguise-active flag +0x1D8 = 1** on pick [doc §8]
- **Enemy-found disguise break**: sets re-disguise timer to RulesClass+0x1014 ticks [doc §8]
- **Damage-breaks-disguise rule**: `CanDisguise && !PermaDisguise` → clear on TakeDamage [from DISGUISE_SYSTEM doc]

### Sim integration
- **Cloak tick must run AFTER combat damage application** so damage-decloak fires same tick [design constraint]
- **Vision in current tick reflects PREVIOUS tick's cloak state** (1-tick lag — vision runs before combat) [design constraint, acceptable]
- **RNG calls must be in stable iteration order** [determinism]
- **All cloak math uses integer/fixed-point** — no f32/f64 in sim [CLAUDE.md]
- **State hash must include cloak + disguise fields** [CLAUDE.md determinism]

## Chosen Approach

**Approach B (cloak + Mirage disguise together), split into 2 sequential PRs.**

**PR 1 — Cloak proper + INI parsing + render wiring**: ships the cloak fade
animation, allied shimmer pulse, sensor reveal, and CloakSound for SUB/DLPH/SQD.
INI keys for both cloak and disguise are parsed in this PR (cheap to do
together; the disguise fields just stay unused until PR 2).

**PR 2 — Mirage disguise sim + sprite-swap**: ships the TurretAI 8-tick scan,
RNG tree-pick, damage-breaks-disguise, and render-side `display_type_override`
swap.

This sequencing minimizes context-switching cost: PR 1 introduces the
`tick_cloak` system and the FX uniform population pattern; PR 2 reuses the
same plumbing for disguise without re-touching files.

**Shader path**: Path A (full dither parity). The dither formula is one
inline WGSL expression; no LUT upload. Path B's flat-alpha approximation is
not worth the parity gap.

**Shimmer phase computation**: CPU pre-compute. Sim provides
`cloak.shimmer_phase_base` (game tick when shimmer cycle started, default 0).
Render reads `current_game_tick - phase_base + 0x40 & 0xFF`, picks alpha from
the 4-band table, writes flat alpha into `fx_params[0]`. The dither THEN
multiplies on top in the shader. So `fx_params[0]` carries the
"transparency intent" (alpha=1.0 opaque / 0.75 shimmer / 0.5 blend); the
shader's dither stage converts that to the per-pixel pattern.

## Design

### Components (in `src/sim/components.rs`)

```rust
#[derive(Copy, Clone, Debug, Default, Hash, PartialEq, Eq)]
pub enum CloakStage {
    #[default]
    Uncloaked,    // = 0 in gamemd
    Cloaking,     // = 1
    Cloaked,      // = 2
    Uncloaking,   // = 3
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Cloak {
    pub state: CloakStage,
    pub progress: u8,                  // 0..CloakingStages-1
    pub step_delta: i8,                // +1 cloaking, -1 uncloaking
    pub step_timer: u16,               // ticks remaining before next progress step
    pub recloak_delay_timer: u16,      // ticks before re-cloak allowed (after forced uncloak)
    pub shimmer_phase_base: u32,       // game tick snapshot — shimmer cycle origin
    pub pending_decloak_trigger: bool, // flag set by combat::on_damage, consumed by tick_cloak
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Default)]
pub struct Disguise {
    pub disguised_type: Option<InternedId>,  // tree TypeClass when active
    pub active: bool,                          // mirrors gamemd's +0x1D8
    pub last_pick_frame: u32,                  // for parity-relevant timing
    pub redisguise_lockout_timer: u16,         // ticks before re-disguise after enemy-near break
    pub fire_blink_timer: u16,                 // (reserved — Spy DisguiseFireBlinkTime, currently unused)
}
```

Add to [`GameEntity` struct](src/sim/game_entity.rs#L48):
```rust
pub cloak: Option<Cloak>,
pub disguise: Option<Disguise>,
```

`cloak` is allocated on construction for any entity whose TypeClass has
`cloakable=true` OR has `VeteranAbilities[CLOAK]` granted (later, when
veterancy is implemented — for now, just `cloakable`). Otherwise `None`.

`disguise` is allocated for any entity with `disguise_when_still=true` or
`can_disguise=true`. Otherwise `None`.

### Interfaces / Contracts

```rust
// src/sim/cloak.rs
pub fn tick_cloak(world: &mut World, rules: &RulesData);

/// Called by combat::tick_combat when damage is applied.
/// Sets pending_decloak_trigger; the next tick_cloak call processes it.
pub fn on_damage(entity: &mut GameEntity);

/// Called by combat::tick_combat when a unit fires a weapon with DecloakToFire=true.
pub fn on_weapon_fire(entity: &mut GameEntity, weapon: &WeaponType);

/// Render-side helper: compute visual state 0-5 for an entity from the local
/// player's perspective. Returns None if cloak component is None.
pub fn visual_state(
    entity: &GameEntity,
    rules: &RulesData,
    viewer_house: HouseId,
    cell_sensor_count: u8,
    is_map_editor: bool,
) -> Option<u8>;

/// Render-side helper: compute the shimmer phase alpha for an allied-owned
/// cloaked unit in visual_state 3.
pub fn shimmer_phase_alpha(cloak: &Cloak, current_tick: u32) -> f32;
```

```rust
// src/sim/disguise.rs
pub fn tick_disguise(world: &mut World, rules: &RulesData);

/// Called by combat::tick_combat when damage applied.
/// Breaks disguise unless PermaDisguise.
pub fn on_damage(entity: &mut GameEntity, type_data: &ObjectType);

/// Render-side helper: returns the displayed TypeClass for an entity from a
/// viewer's perspective. Returns None for "no swap" (use real type).
pub fn display_type_for_viewer(
    entity: &GameEntity,
    viewer_house: HouseId,
    interner: &StringInterner,
) -> Option<InternedId>;
```

### Data Flow

```
Per tick:
  ... (steps 1-10 unchanged) ...
  
  step 10.5 — tick_cloak(world, rules):
    For each entity with Some(cloak):
      1. If recloak_delay_timer > 0: decrement.
      2. If pending_decloak_trigger:
           transition to state 3 (StartUncloaking semantic), clear flag
      3. Per-state machine:
         - state 0: 
             check IsCloakable + CanAutoCloak.
             if CanAutoCloak and rules.cloaking_speed > 0:
               tick step_timer; on expiry, increment progress.
             auto-cloak if health < condition_red and rng.next(0,99) < 4 → state 1.
         - state 1:
             tick step_timer; on expiry, progress += step_delta.
             compute visual_state; if 3 or 5 → state 2.
             if visual_state == 2 and health < condition_red and rng.next(0,99) <= 9
               → state 3 (abort-uncloak).
         - state 2:
             check ShouldUncloak (= near enemy sensor, or special).
             if yes → state 3.
         - state 3:
             tick step_timer; on expiry, progress += step_delta (=-1).
             compute visual_state; if 0 → state 0; reset recloak_delay_timer.
      4. On state 0→1, 2→3 transition: queue SimSoundEvent::CloakSound at unit position.

  step 10.7 — tick_disguise(world, rules):
    For each entity with Some(disguise):
      Only on (current_tick & 7 == 0):
        If is_moving: clear disguise.active, return.
        Else:
          scan 8-cells + own cell for enemy units.
          if enemy found:
            clear disguise.active
            set redisguise_lockout_timer = rules.disguise_lockout_frames
          else if !disguise.active and lockout_timer == 0:
            tree_id = rules.default_mirage_disguises[
              rng.next_range_u32_inclusive(0, len-1)
            ]
            disguise.disguised_type = Some(tree_id)
            disguise.active = true
            disguise.last_pick_frame = current_tick
      Always: decrement timers.

  ... (steps 11-18 unchanged) ...

Per frame (render-side, in app_instances/units.rs):
  For each visible entity:
    let visual_state = cloak::visual_state(entity, rules, local_viewer, ...);
    let display_type = disguise::display_type_for_viewer(entity, local_viewer, ..);

    let fx_flags = if visual_state.is_some_and(|v| v > 0) { 1 } else { 0 };
    let fx_params_0 = match visual_state {
      None | Some(0) => 1.0,
      Some(1) => 0.75,
      Some(2) | Some(3) => {
        if entity_is_player_controlled {
          shimmer_phase_alpha(cloak, current_tick)  // produces 1.0/0.75/0.5
        } else {
          0.5
        }
      }
      Some(4) => if entity.is_voxel { 0.75 } else { 0.5 },  // VXL brightness vs SHP
      Some(5) => return,  // skip draw entirely
    };

    let type_for_atlas_key = display_type.unwrap_or(entity.type_ref);
    let key = UnitSpriteKey { type_id: type_for_atlas_key, ... };
    let entry = atlas.get(&key);

    push SpriteInstance with fx_flags + fx_params[0].
```

### Shader

[src/render/sprite_voxel_shader.wgsl:96-109](src/render/sprite_voxel_shader.wgsl#L96-L109):

```wgsl
fn apply_fx(color: vec4f, flags: u32, params: vec4f, ic: vec4f, frag_pos: vec2f) -> vec4f {
    var c = color;
    if ((flags & 1u) != 0u) {
        // params.x carries the "transparency intent" in [0.0, 1.0].
        // Translate to gamemd's intensity_clamp domain [0, 254]:
        let intensity_clamp: u32 = u32(clamp(params.x * 254.0, 0.0, 254.0));
        // abuf_hash: 8-bit pseudo-random per fragment, mimicking the engine's a-buffer.
        // Use a screen-space hash that's deterministic per-pixel and stable per-frame.
        let abuf: u32 = u32(fract(sin(dot(frag_pos, vec2f(12.9898, 78.233))) * 43758.5453) * 256.0) & 0xFFu;
        // gamemd formula:
        let val_raw: u32 = (abuf * intensity_clamp * 254u) / 32258u;
        let val: u32 = min(val_raw, 254u);
        // val approximates the "dimming amount". Apply as alpha modulation:
        c.a = c.a * (f32(val) / 254.0);
    }
    // ... (other FX branches unchanged) ...
    return c;
}
```

**Alternative abuf source**: a small Bayer dither texture (8×8 or 16×16
ushort) sampled via `textureLoad`. This is more faithful to gamemd's
deterministic-pattern dither than the trigonometric hash. **Recommend
implementing both, A/B test against gamemd captures during Phase 2.1.**

### Error Handling

- **Atlas miss for disguised tree type**: render with magenta-key sentinel per
  existing `feedback_silent_render_failures` convention; log once per missing
  type.
- **CloakingSpeed=0 in INI**: clamp to 1 at parse time. Document as a parser-side
  fix that diverges from gamemd's "trust the value" — we choose safety over
  bit-perfect parity for a value that wouldn't fire in retail INI anyway.
- **No DefaultMirageDisguises in rules.ini**: skip tree-pick entirely. Mirage
  never disguises. Log warning.
- **Disguise active but disguised_type pointer is None**: treat as
  disguise.active = false. Defensive.

### Testing Strategy

- **Unit tests** for `cloak::visual_state`: matrix of (state, progress,
  viewer_relation, sensor_present, discovered, editor) → expected visual_state.
  Cover at least 30 rows including all edge cases in the ledger.
- **Unit tests** for `cloak::shimmer_phase_alpha`: matrix of phase value
  → expected band. Verify all 9 transitions (4 shimmer bands, 2 blend bands,
  3 opaque bands).
- **Unit tests** for `cloak::ftol_formula`: verify
  `visual_raw(Progress, Stages)` matches the FIDIV→FMUL→FTOL semantics for
  Progress=0..Stages, Stages=9 and Stages=4 (a non-default).
- **Unit tests** for `disguise::display_type_for_viewer`: allied/enemy/own
  viewer × active/inactive disguise → expected output.
- **Determinism test**: run the same scenario twice (cloak + disguise active),
  assert state_hash matches at each tick.
- **Integration test**: spawn a SUB, fire CloakingTick for 9 ticks, assert
  visual_state progresses 0→1→1→2→2→2→2→3 (transition to 2), then 9 more ticks
  in state 2, then force-uncloak (e.g., damage), assert 9 ticks of state-3
  transition.
- **Pixel-comparison test (manual, Phase 2.2)**: side-by-side captures vs
  gamemd of: cloaking SUB, fully-cloaked SUB seen by self, fully-cloaked SUB
  near DEST (sensor cover), Mirage Tank disguised as tree.

## Architectural Decisions

### Patterns followed

- **Inline `Option<Component>` on GameEntity**, no ECS. Cloak and Disguise are
  optional fields, allocated only on relevant TypeClasses.
- **Single-file `components.rs`**. New components added inline; no
  `components/` subdirectory introduced.
- **`tick_X(world, rules)` system function** placed in
  `src/sim/<system>.rs`, invoked from `advance_tick`. Mirrors movement, combat,
  power, vision, etc.
- **SimSoundEvent dispatch** for audio. Sim enqueues; app drains.
- **SimRng for any randomness** with deterministic per-tick order.
- **`display_type_override` for sprite swaps** — reused, not duplicated.
- **INI parser via `section.get_bool` / `get_int` / `get_f32` / `get`** —
  established pattern.
- **State hash extension** via `hash_entities()` per-entity fold.

### Patterns deviated from

- **Combat-to-cloak coupling**: `combat::tick_combat` will call a
  `cloak::on_damage` helper. This is a sim→sim cross-system call. Currently the
  sim has cross-system calls via shared mutable World, but this introduces a
  new dependency from combat → cloak. **Justification**: damage→decloak is a
  same-tick event in gamemd; deferring it to a later phase would introduce a
  1-tick lag that's player-observable for the SUB's "fire then go invisible
  next tick" mechanic. The coupling is documented in the cloak module's `//!`
  header.

### Tech debt introduced

- **The shimmer dither in WGSL uses a trig-hash for abuf** which may differ
  pixel-for-pixel from gamemd's a-buffer pattern. A faithful Bayer-LUT
  implementation is a Phase 2.2 follow-up after pixel-comparison testing
  identifies whether the divergence is visible.
- **Veteran/Elite CLOAK promotion** is not implemented in Phase 2. Cloak
  component is only allocated on units with `cloakable=true` at construction.
  A veteran SUB still cloaks the same as a rookie SUB. Veteran-granted-CLOAK
  (a moddable feature in retail) is deferred to the veterancy system's full
  implementation.
- **CloakingSpeed=0 clamp** diverges from gamemd's "trust the value"
  semantics. Documented in the parser. If a player's retail INI has 0, our
  engine clamps to 1; gamemd would divide-by-zero or instant-cloak. Edge case
  not exercised by stock content.

### Determinism considerations

- **CloakProgress / CloakingStages math**: use integer arithmetic to mimic
  ftol truncation. Formula: `visual_raw = (progress as u32 * 256 + 0) / stages as u32`
  with default integer truncation (rounds toward zero). Verify matches gamemd's
  FIDIV (FPU division then truncate-cast).
- **RNG calls in stable order**: `BTreeMap<u64, GameEntity>` already iterates by
  stable_id. `tick_cloak` iterates entities once; `tick_disguise` iterates
  once.
- **State hash adds cloak + disguise per-entity**:
  `hash_entities` is extended to hash these `Option` fields by their `Hash`
  impl. The order matters — entities iterated in stable_id order
  unconditionally.
- **No floats in sim logic**: all cloak math uses `u8` / `u16` / `u32` / `i8`.
  The `f32` only enters the render layer when packing into `fx_params[0]`.

## Alternatives Considered

- **Approach A — cloak only, disguise deferred**. Rejected: leaves Mirage Tank
  rendering as a normal VXL to enemies, a player-observable parity gap in
  every game with MGTK present.
- **Approach C — cloak + sprite-swap-only Mirage (no sim disguise tick)**.
  Rejected: would render Mirage always as TREE01 for enemies with no scan,
  no damage-break, no 8-tick interval. Violates parity on 3 distinct
  observable mechanics.
- **Shader path B (flat alpha)**. Rejected: visibly smoother than gamemd's
  dithered shimmer. Path A is one extra WGSL line.
- **GPU-computed shimmer phase**. Rejected: no determinism benefit, adds a
  uniform, adds shader complexity. CPU compute is simpler and equally
  deterministic (phase is a sim tick value).
- **Single mega-PR for cloak + disguise**. Rejected: ~1000 LOC across 10+
  files, hard to review. Splitting into 2 sequential PRs is the
  industry-standard size band (~500 LOC each).
- **Separate fields for cloak.shimmer_phase_base and disguise.last_pick_frame
  (NOT sharing storage)**. Adopted (vs gamemd's +0x1DC field aliasing). The
  field-sharing in gamemd is observably moot in retail (no Cloakable+Disguising
  unit exists). Per CLAUDE.md, internals are ours to design. Separate fields
  are cleaner and avoid the veteran-cloak-Mirage edge case where field-sharing
  would resync shimmer on disguise re-pick.
- **Cloak tick before vision (step 5.5)**. Rejected: requires damage from
  the PREVIOUS tick to be cached, complicating the combat→cloak handoff.
  Post-combat placement is cleaner and the 1-tick vision lag is acceptable
  (vision in gamemd is per-frame anyway, not per-tick).
