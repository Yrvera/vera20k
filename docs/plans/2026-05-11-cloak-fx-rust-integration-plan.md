---
title: Cloak FX Rust Integration — PR 1 Implementation Plan
status: awaiting approval
---

# Cloak FX Rust Integration — PR 1 Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.
> Tasks 1-5 (types + INI parsing) are independent and can be parallelized; tasks
> 6-12 (sim logic) must be done in order; tasks 13-14 (combat integration) and
> 15-17 (render integration) build on tasks 6-12. Tests last. Commit after each
> task.

**Goal:** Ship the cloak system end-to-end for retail YR's three Cloakable units
(SUB, DLPH, SQD) — state machine, fade animation, allied shimmer pulse, sensor
reveal, CloakSound, dither-shader rendering — with deterministic sim and full
parity with gamemd.exe.

**Architecture:** Adds `Cloak` component as inline `Option<Cloak>` slot on
GameEntity. New `tick_cloak()` system runs between combat damage-application
(step 10) and retaliation (step 13). Render-side reads cloak state and populates
existing FX uniforms; shader extension implements Path-A dither parity. **Sim
remains free of render/audio dependencies** — audio fires via the existing
SimSoundEvent enum pattern.

**Design Doc:** [docs/plans/2026-05-11-cloak-fx-rust-integration-design.md](docs/plans/2026-05-11-cloak-fx-rust-integration-design.md)

**Research Source:** [docs/research/CLOAK_FX_SHADER_BRIDGE_GHIDRA_REPORT.md](docs/research/CLOAK_FX_SHADER_BRIDGE_GHIDRA_REPORT.md)

**Scope:** PR 1 of 2. Mirage tree-disguise sim logic, sprite-swap, and Spy
disguise are deferred to PR 2. INI keys for the disguise system ARE parsed in
this PR but their fields stay unused until PR 2.

---

## Grounding Summary

- **Research basis:** `CLOAK_FX_SHADER_BRIDGE_GHIDRA_REPORT.md` (HIGH confidence
  on all formulas, addresses, and field offsets). 24 of 27 ledger items verified
  directly from gamemd.exe disassembly; 3 verified via prior `CLOAKING_VISUAL_PIPELINE.md`
  + cross-check in this session.
- **Ghidra-verified addresses** for primary functions: CloakingTick @ 0x006FB740,
  GetVisualState @ 0x00703860, ModifyCloakDrawFlags @ 0x0070ED80, StartCloaking
  @ 0x00703770, StartUncloaking @ 0x007036C0. All addresses are kept in the
  research doc, NOT in Rust code comments (per `feedback_no_engine_refs_in_comments`).
- **Repo pattern mirrored:** `tick_building_up` at [src/sim/world/mod.rs:900-919](src/sim/world/mod.rs#L900)
  is the closest match for our `tick_cloak`: iterates entities via
  `keys_sorted()`, mutates via `get_mut()`. Component struct pattern mirrors
  `Health` (components.rs:87) and `Veterancy` (components.rs:131). State hash
  extension mirrors commit `b1b60e9` which added `c4_plant`/`pending_c4_detonation`
  fields.
- **INI keys driving behavior:** `[General] CloakingStages=9`, `[AudioVisual]
  CloakSound=NavalUnitEmerge`, per-type `Cloakable=yes`, `CloakingSpeed=1` (SUB
  /DLPH) or `=5` (SQD), `CloakStop=`, `Invisible=`, `Sensors=`, `SensorsSight=`.
  Plus disguise keys parsed but unused: `CanDisguise=`, `PermaDisguise=`,
  `DisguiseWhenStill=`, `DetectDisguise=`, `DetectDisguiseRange=`, `[General]
  DefaultMirageDisguises=TREE01,TREE02,TREE03,TREE04`.
- **Unknown after grounding:** Exact entity-spawn site location for component
  allocation. Resolved at Task 6 by code-search for "TypeClass-derived component
  initialization" (e.g., where `voxel_animation` or `deploy_state` is allocated).

## Key Technical Decisions

- **Cloak component is `Option<Cloak>` on GameEntity, allocated at spawn for
  cloakable types**: avoids per-entity allocation cost for the 99% of units
  that can't cloak. **Confidence:** high. **Source:** design doc §Components;
  mirrors existing `c4_plant: Option<C4PlantState>` pattern.
- **`visual_raw = (progress as u32 * 256) / stages as u32` (integer division)**:
  reproduces gamemd's FIDIV→FMUL→FTOL truncation since both Progress and Stages
  are positive ints. **Confidence:** high. **Source:** Ghidra 0x00703A79-A8F
  verified; integer truncation matches FTOL_TRUNCATE.
- **Shimmer phase = `((frame as i64 - phase_base as i64 + 0x40) as u32) & 0xFF`**:
  the 256-tick cycle is computed with i64-intermediate to handle frame underflow,
  then narrowed to u32 and masked. **Confidence:** high. **Source:** doc §5.1;
  matches the asm `(g_CurrentFrameCounter - +0x1DC + 0x40) & 0x800000FF` with
  signed-correction.
- **Shimmer-suppression timer (`+0x1EC`/`+0x1F4`) NOT implemented**: dormant in
  retail YR (no live writer outside constructor). **Confidence:** high.
  **Source:** doc §5.3, byte-pattern search confirmed.
- **Cloak component allocated at spawn for `cloakable=true` types ONLY**:
  veteran-CLOAK promotion deferred to veterancy system. **Confidence:** high.
  **Source:** design doc §Tech Debt Introduced.
- **`CloakingSpeed = 0` clamped to 1 at parse time**: defensive against
  divide-by-zero. **Confidence:** medium. **Source:** design doc §Error Handling;
  diverges from gamemd's trust-the-value but stock content has explicit speeds.
- **WGSL dither uses trig-hash for abuf source** (not a Bayer LUT yet): Path A
  approximation, pixel-comparison test in Phase 2.2 follow-up may switch to LUT
  if visible drift detected. **Confidence:** medium. **Source:** design doc
  §Tech Debt.

## Open Questions

### Resolved During Planning

- **Where is the entity-spawn site?** → To be found at Task 6 (search for sites
  that initialize `voxel_animation` or `deploy_state` from TypeClass). Multiple
  sites may exist (production, map-init, etc.); factor a helper if so.
- **Is there an `is_moving()` helper on GameEntity?** → No. The pattern is
  `entity.movement_target.is_some()`. PR 1 doesn't need this (disguise scan is
  PR 2). Documented for PR 2 use.
- **Does `tick_cloak` need a separate path for cloak-up vs cloak-down?** → No.
  The state machine in §3.2 of the research doc handles both via switch on
  `CloakState`; CloakStepDelta drives direction.

### Deferred to Implementation

- **Whether `RNG::next_range_u32_inclusive(0, 99)` returns [0, 99] inclusive
  matches gamemd's `RandomRanged(0, 99)`**: verify the existing `next_range_u32_inclusive`
  matches gamemd's semantics during Task 12. If gamemd is also inclusive both
  ends, no change. If gamemd is `[low, high)`, adjust to `next_range_u32(100)`.
- **Whether allied shimmer requires `IsPlayerControlled` (vtable+0xC4)** or just
  `Owner == g_PlayerPtr`**: design doc says "player-controlled & owned by human".
  Mind-controlled units complicate this — a player-controlled but
  enemy-mind-controlled cloaked unit may shimmer or not. PR 1 implements
  "Owner.is_human_player()" only; revisit if pixel-comparison test reveals
  divergence.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | [src/sim/components.rs](src/sim/components.rs) | Add `Cloak` struct + `CloakStage` enum |
| Modify | [src/sim/game_entity.rs](src/sim/game_entity.rs) | Add `cloak: Option<Cloak>` field; mark cloakable types at spawn |
| Modify | [src/sim/world/world_hash.rs](src/sim/world/world_hash.rs) | Hash cloak field in `hash_entities()` |
| Modify | [src/rules/object_type.rs](src/rules/object_type.rs) | Add cloak+disguise INI fields; parse 10 new keys |
| Modify | [src/rules/ruleset.rs](src/rules/ruleset.rs) | Add `GeneralRules` fields for CloakingStages/CloakSound/DefaultMirageDisguises; parse them |
| Modify | [src/sim/world/mod.rs](src/sim/world/mod.rs) | Add `SimSoundEvent::CloakSound` variant; insert `cloak::tick_cloak()` call between combat fallout and retaliation |
| Create | `src/sim/cloak.rs` | New module: `tick_cloak`, `visual_state`, `shimmer_phase_alpha`, `on_damage`, `on_weapon_fire` |
| Modify | [src/sim/mod.rs](src/sim/mod.rs) | Register new `cloak` module |
| Modify | [src/sim/combat/mod.rs](src/sim/combat/mod.rs) | Call `cloak::on_damage` at damage-application site; call `cloak::on_weapon_fire` at fire site |
| Modify | [src/app_instances/units.rs](src/app_instances/units.rs) | Populate `fx_flags` bit 0, `fx_params[0]` from entity.cloak |
| Modify | [src/render/sprite_voxel_shader.wgsl](src/render/sprite_voxel_shader.wgsl) | Replace flat-alpha cloak branch with Path-A dither formula |
| Modify | App audio drain site (TBD via grep) | Dispatch `SimSoundEvent::CloakSound` to VocClass play |

## Interface Changes

**New public API surface in `src/sim/cloak.rs`:**
```rust
// Tick driver — called from advance_tick
pub fn tick_cloak(world: &mut World, rules: &RulesData);

// Event hooks — called by combat
pub fn on_damage(entity: &mut GameEntity);
pub fn on_weapon_fire(entity: &mut GameEntity, weapon: &WeaponType);

// Render-side pure helpers
pub fn visual_state(
    entity: &GameEntity,
    rules: &RulesData,
    viewer_house: InternedId,
    cell_sensor_count: u8,
    is_map_editor: bool,
    player_house: Option<InternedId>,
    is_allied: impl Fn(InternedId, InternedId) -> bool,
) -> u8;

pub fn shimmer_phase_alpha(cloak: &Cloak, current_tick: u32) -> f32;

// Pure helper for tests
pub fn ftol_formula(progress: u8, stages: u32) -> u32;
```

**New `SimSoundEvent` variant** (additive; existing consumers untouched):
```rust
SimSoundEvent::CloakSound { sound_id: InternedId, rx: u16, ry: u16 }
```

**ObjectType fields added** (10 new bools/ints; default to `false` / `0`):
- `cloakable`, `cloaking_speed`, `cloak_stop`, `invisible`, `sensors`, `sensors_sight`
- `disguise_when_still`, `perma_disguise`, `detect_disguise`, `detect_disguise_range`

**GeneralRules fields added:**
- `cloaking_stages: u32` (default 9)
- `cloak_sound: Option<InternedId>`
- `default_mirage_disguises: Vec<InternedId>` (parsed from comma-list)

## Sim Checklist

- [x] All cloak math uses `u8`/`u16`/`u32`/`i8` — no f32/f64 in sim logic. Float only enters at render-layer SpriteInstance population.
- [x] `cloak: Option<Cloak>` hashed in `hash_entities()` via `.hash(hasher)` extension (Cloak derives `Hash`).
- [x] No dependencies on render/ui/sidebar/audio/net. `src/sim/cloak.rs` imports only from `src/sim/` and `src/rules/`.
- [x] Tick ordering: `tick_cloak` inserted at step 10.5 — after combat fallout, before retaliation. Vision (step 6) sees PREVIOUS tick's cloak state (1-tick lag, documented in module header).
- [x] BTreeMap iteration via `keys_sorted()` (same pattern as `tick_building_up`). RNG calls in stable order.

## Risk Areas

| Risk | Mitigation |
|------|------------|
| Determinism break — cloak state hash diverges across replays | Task 3 adds cloak to `hash_entities()`; Task 19 integration test runs the same scenario twice and asserts state_hash equality at every tick |
| Shimmer phase computation has signed-wrap bugs | Task 10 implements with explicit `i64` intermediates; unit test in Task 18 covers frame=0, frame=phase_base+1, frame=phase_base, frame much greater than phase_base, frame much less |
| Cloak component allocation site missed | Task 6 explicitly greps for entity-spawn sites; if multiple, factor a `Cloak::for_type(type_data)` helper used at every spawn |
| Damage-decloak fires before damage actually applied | Task 13 places the hook call AFTER `target.health.current -= damage` in combat; verified via re-reading combat::mod.rs:1794 area |
| Existing apply_fx flat-alpha behavior breaks | Task 16 keeps the flag-0 gate identical; only changes the body when flag is set. Shader still compiles and runs for non-cloaked instances (fx_flags=0 short-circuits) |
| VXL state 4 differs from SHP — wrong alpha in shader | Task 15 branches on `entity.is_voxel`: state 4 → 0.75 for VXL, 0.5 for SHP |

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | CloakStage enum order matches gamemd's 0/1/2/3 | State hash depends on discriminant; mismatch breaks replay determinism across versions | `#[repr(u8)]` with explicit `= 0`, `= 1`, `= 2`, `= 3`; static_assert via const |
| Task 5 | CloakingStages default = 9 | Drives the entire visual-state ladder; off-by-one shows wrong frame count to player every cloak | Unit test in Task 18 asserts `rules.cloaking_stages == 9` when INI key is absent |
| Task 5 | CloakSound default value when key missing | Audio cue fires every cloak transition; silence is observable | Default None → no sound; if INI provides bad VocClass name, retain prior value (matches gamemd's "if (iVar4 == -1) iVar4 = iVar2") |
| Task 9 | `visual_raw = (progress * 256) / stages` integer truncation | Off-by-one in this formula = wrong visual_state, wrong shader alpha, every cloak frame | Unit test matrix covers all 9 progress values for stages=9; expected values from research doc §4.3 |
| Task 9 | Discovered-clamp at visual_raw >= 0xC0 returns 3 (not 4) | Player viewing their own discovered cloaked unit sees 50% blend, not 25% blend | Unit test row: `(progress=8, stages=9, discovered=true, perspective_query=false)` → expects 3 |
| Task 9 | `>= 0xFF → state 5` boundary | Last frame of cloak animation is "skip draw" vs "near-invisible"; visible at 9th tick | Unit test rows: `iVar3=0xFE → 4`, `iVar3=0xFF → 5`, `iVar3=0x100 → 5` |
| Task 9 | Buildings always return 0 (WhatAmI==Building) | TS-legacy gating; building cloak is dead in YR — must not accidentally render buildings with cloak FX | Unit test: building category + CloakState=2 → visual_state=0 |
| Task 10 | 4 shimmer bands, not 2 | Player-owned cloak pulse cadence; corrected from prior doc. Every game with a Cloakable+human-owned unit observes this pulse | Unit test exhaustively covers phase=0x40, 0x43, 0x44, 0x4B, 0x4C, 0x4F, 0x50, 0x6F, 0x70, 0x73, 0x74, 0x7B, 0x7C, 0x7F, 0x80 — expects each band's correct alpha |
| Task 10 | Shimmer cycle uses game-tick (deterministic) not wall-clock | Multiplayer lockstep requires deterministic phase | Phase derived from `World.tick`, NOT `Instant::now()`. Determinism test in Task 19. |
| Task 12 | CloakProgress starts at `CloakingStages - 1` for state-3 (uncloaking) | First uncloak tick shows state 4, not state 5 | Integration test (Task 19) traces Progress values; first state-3 tick has Progress=8 |
| Task 12 | State 0 auto-cloak chance = 4%, abort-uncloak chance = 10% | RNG-driven gameplay timing; deterministic but uses correct thresholds | Unit test with seeded RNG: 100 ticks of state 0 with health<ConditionRed expects ~4 auto-cloaks; 100 ticks of state 1 visual=2 expects ~10 aborts |
| Task 12 | CloakSound plays at 0→1 AND 2→3 transitions (NOT 1→2 or 3→0) | Audio cue parity; gamemd plays sound at the START of each fade animation | Integration test: cycle a unit through all 4 states, count sound events — expect 2 per full cycle (one at each transition into Cloaking/Uncloaking) |
| Task 14 | Damage-decloak hook fires AFTER damage application | If hook fires before, the unit hasn't taken damage yet — wrong behavior | Inspect commit diff: hook call must be lexically after `target.health.current = ...saturating_sub(...)` line |
| Task 15 | VXL state 4 → fx_params[0]=0.75; SHP state 4 → 0.5 | VXL units fade differently from SHP units in state 4; visible every first uncloak tick of every Cloakable VXL unit | Inline test in units.rs (or extracted helper test) covers the branch |
| Task 15 | State 5 → SKIP entity push (don't emit SpriteInstance at all) | Cloaked-and-not-discovered enemy units must NOT render. Player would see them otherwise. | The push site must early-`continue` when visual_state==5 |
| Task 16 | Dither shader formula: `(abuf * intensity * 254) / 32258` | Per-pixel dither pattern parity. Without this, shimmer is flat alpha — visibly smoother than gamemd | WGSL block matches doc §6.5 exactly; comment cites the formula constants |
| Task 17 | CloakSound plays at unit position (not centered/global) | Spatial audio — players hear cloak/decloak relative to camera | Audio dispatch receives `(rx, ry)` from SimSoundEvent and translates to listener-relative position |

---

## Tasks

### Task 1: Define `Cloak` component and `CloakStage` enum

**Why:** Foundation type. Every other sim/render task references this.

**Files:**
- Modify: [src/sim/components.rs](src/sim/components.rs) — add at end of file

**Pattern:** Mirrors `Health` (line 87) and `Veterancy` (line 131) — derives Debug/Clone/Copy and includes `Hash` for state-hash inclusion.

**Step 1: Add types**
```rust
// src/sim/components.rs (append to end of file)

/// Cloak state machine values. Matches gamemd CloakState enum at TechnoClass+0x220.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum CloakStage {
    #[default]
    Uncloaked = 0,
    Cloaking = 1,
    Cloaked = 2,
    Uncloaking = 3,
}

/// Per-entity cloak state. Allocated only on `Cloakable=yes` TypeClasses
/// (or veteran-promoted units, future work).
///
/// Fade animation timing is controlled by `step_timer` (ticks until next
/// progress step) and TypeClass.cloaking_speed. State transitions are
/// driven by `tick_cloak` in `src/sim/cloak.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub struct Cloak {
    pub state: CloakStage,
    /// Progress through the fade animation, 0..CloakingStages-1.
    pub progress: u8,
    /// Step direction: +1 cloaking, -1 uncloaking. Stored as i8.
    pub step_delta: i8,
    /// Ticks remaining until next progress step.
    pub step_timer: u16,
    /// Ticks remaining before re-cloak is allowed after a forced uncloak.
    pub recloak_delay_timer: u16,
    /// Snapshot of the game tick at which the current shimmer cycle began.
    /// Used for allied-shimmer phase computation.
    pub shimmer_phase_base: u32,
    /// Set by combat::on_damage; consumed by tick_cloak to trigger state 2→3.
    pub pending_decloak_trigger: bool,
}
```

**Step 2: Verify**
```
cargo check -p ra2-rust-game
```
Expected: compiles clean (no usage yet — just type definitions).

**Step 3: Commit**
Message: `sim/components: add Cloak struct + CloakStage enum`

---

### Task 2: Add `cloak: Option<Cloak>` field to GameEntity

**Why:** Carry cloak state per entity. Set to `None` for non-cloakable types.

**Files:**
- Modify: [src/sim/game_entity.rs](src/sim/game_entity.rs) — add near the existing `Option<X>` component fields (around line 199, near `display_type_override`)

**Pattern:** Mirrors `c4_plant: Option<C4PlantState>` field added in commit `ee33e38`.

**Step 1: Import**
Locate the `use crate::sim::components::{...}` block near the top of `game_entity.rs` and add `Cloak` to the import list.

**Step 2: Add field**
Inside the `GameEntity` struct definition (find via `pub struct GameEntity`), add the field in the optional-components section (near `display_type_override`):
```rust
    /// Cloak state machine. `None` for non-cloakable types.
    pub cloak: Option<Cloak>,
```

**Step 3: Default impl (if hand-rolled)**
If `GameEntity` has a manual `Default` impl, add `cloak: None,`. If it derives Default, no action needed.

**Step 4: Verify**
```
cargo check -p ra2-rust-game
```
Expected: compiles clean. New field defaults to `None` everywhere.

**Step 5: Commit**
Message: `sim/game_entity: add cloak: Option<Cloak> field`

---

### Task 3: Hash `cloak` field in `world_hash`

**Why:** Determinism. State hash must reflect cloak state changes for replay/lockstep correctness.

**Files:**
- Modify: [src/sim/world/world_hash.rs](src/sim/world/world_hash.rs) — extend `hash_entities()`

**Pattern:** Mirrors the `entity.c4_plant.hash(hasher);` line added in commit `b1b60e9`.

**Step 1: Locate insertion point**
Find `hash_entities()` (around line 316). Locate the section that hashes `entity.c4_plant.hash(hasher);`. Add the new line immediately after, keeping fields grouped by topic.

**Step 2: Add line**
```rust
        entity.cloak.hash(hasher);
```

**Step 3: Verify**
```
cargo check -p ra2-rust-game
```
Then run any existing determinism tests:
```
cargo test --test determinism 2>&1 | tail -20
```
Expected: existing tests pass (cloak field is None for all current test fixtures, so hash result unchanged).

**Step 4: Commit**
Message: `sim/world_hash: include cloak field`

---

### Task 4: Parse cloak + disguise INI keys on ObjectType

**Why:** All 13 keys need INI presence before sim logic can read them. Disguise keys are parsed in this PR even though their fields stay unused until PR 2.

**Files:**
- Modify: [src/rules/object_type.rs](src/rules/object_type.rs) — add 10 fields + parse calls

**Pattern:** Mirrors existing `radar_invisible: section.get_bool("RadarInvisible").unwrap_or(false)` (line 806).

**Step 1: Add struct fields**
Locate the `ObjectType` struct definition. Add (group near other detection/stealth fields):
```rust
    // Cloak system (PR 1 of cloak FX work)
    pub cloakable: bool,
    pub cloaking_speed: u32,
    pub cloak_stop: bool,
    pub invisible: bool,
    pub sensors: bool,
    pub sensors_sight: u32,
    // Disguise system (parsed in PR 1, used in PR 2)
    pub can_disguise: bool,        // EXISTS already — verify not duplicated
    pub perma_disguise: bool,
    pub disguise_when_still: bool,
    pub detect_disguise: bool,
    pub detect_disguise_range: u32,
```

⚠ **NOTE:** `can_disguise` already exists in this file (confirmed in research). Do NOT duplicate. Skip its declaration but include it in the field group comment.

**Step 2: Add parse calls**
Locate `ObjectType::from_ini_section` (around line 673). Add the parse lines near existing boolean parses (e.g., near `radar_invisible:` at line 806):
```rust
            cloakable: section.get_bool("Cloakable").unwrap_or(false),
            cloaking_speed: {
                // CloakingSpeed=0 would divide-by-zero in visual-state math;
                // clamp to 1. Stock retail INI sets 1 or 5 explicitly.
                let raw = section.get_int("CloakingSpeed").unwrap_or(0);
                if raw <= 0 { 1 } else { raw as u32 }
            },
            cloak_stop: section.get_bool("CloakStop").unwrap_or(false),
            invisible: section.get_bool("Invisible").unwrap_or(false),
            sensors: section.get_bool("Sensors").unwrap_or(false),
            sensors_sight: section.get_int("SensorsSight").unwrap_or(0) as u32,
            perma_disguise: section.get_bool("PermaDisguise").unwrap_or(false),
            disguise_when_still: section.get_bool("DisguiseWhenStill").unwrap_or(false),
            detect_disguise: section.get_bool("DetectDisguise").unwrap_or(false),
            detect_disguise_range: section.get_int("DetectDisguiseRange").unwrap_or(0) as u32,
```

**Step 3: Verify**
```
cargo check -p ra2-rust-game
```
Expected: compiles clean.

Then add a small parse-roundtrip test in the same file (`#[cfg(test)] mod tests`):
```rust
    #[test]
    fn cloak_keys_parse() {
        let ini = r#"
[SUB]
Cloakable=yes
CloakingSpeed=1
Invisible=no
"#;
        let parsed = IniFile::from_str(ini).unwrap();
        let section = parsed.section("SUB").unwrap();
        let obj = ObjectType::from_ini_section("SUB", &section, ObjectCategory::Unit);
        assert_eq!(obj.cloakable, true);
        assert_eq!(obj.cloaking_speed, 1);
        assert_eq!(obj.invisible, false);
    }
```
Run:
```
cargo test cloak_keys_parse -p ra2-rust-game
```
Expected: PASS.

**Step 4: Commit**
Message: `rules/object_type: parse Cloakable/CloakingSpeed/CloakStop + disguise INI keys`

---

### Task 5: Parse `[General]` and `[AudioVisual]` cloak keys onto `GeneralRules`

**Why:** Global cloak rules — CloakingStages drives all visual-state math; CloakSound is the audio cue VocClass; DefaultMirageDisguises is needed for PR 2 (parsed here together).

**Files:**
- Modify: [src/rules/ruleset.rs](src/rules/ruleset.rs) — add fields + parse calls

**Pattern:** Mirrors existing `condition_yellow: f32` (line 224) and `building_garrisoned_sound: Option<String>` (line 234).

**Step 1: Add struct fields**
Inside `GeneralRules` struct (around line 145+):
```rust
    /// [General] CloakingStages= — number of fade steps. Default 9.
    pub cloaking_stages: u32,
    /// [AudioVisual] CloakSound= — VocClass played at cloak/uncloak transitions.
    pub cloak_sound: Option<InternedId>,
    /// [General] DefaultMirageDisguises= — comma-list of OverlayTypes Mirage Tank
    /// can disguise as. Used in PR 2.
    pub default_mirage_disguises: Vec<InternedId>,
```

**Step 2: Add parse calls in `GeneralRules::from_ini`**
Locate the function (around line 733). Add near other parsing:
```rust
            cloaking_stages: general
                .and_then(|s| s.get_int("CloakingStages"))
                .map(|n| if n <= 0 { 9 } else { n as u32 })
                .unwrap_or(9),
            cloak_sound: audio_visual
                .and_then(|s| s.get("CloakSound"))
                .filter(|s| !s.is_empty())
                .map(|s| interner.intern(s)),
            default_mirage_disguises: general
                .and_then(|s| s.get("DefaultMirageDisguises"))
                .map(|s| {
                    s.split(',')
                        .map(|p| p.trim())
                        .filter(|p| !p.is_empty())
                        .map(|p| interner.intern(p))
                        .collect()
                })
                .unwrap_or_default(),
```

⚠ **NOTE:** Verify the `interner` is in scope inside `from_ini`. If `from_ini` takes only `&IniFile`, this needs to either:
- Take an `&mut StringInterner` argument (breaks callers — check call sites), OR
- Defer interning to a post-load pass (store as `Vec<String>` temporarily and intern at a later phase).

Look for the prevailing pattern by reading `building_garrisoned_sound`: it stores `Option<String>` and is interned later. Mirror that:
```rust
            cloaking_stages: ...as above...,
            cloak_sound: audio_visual
                .and_then(|s| s.get("CloakSound"))
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string()),  // String for now; interned at post-load if convention requires
            default_mirage_disguises: general
                .and_then(|s| s.get("DefaultMirageDisguises"))
                .map(|s| s.split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect())
                .unwrap_or_default(),
```
And change the field types to `Option<String>` / `Vec<String>` to match `building_garrisoned_sound`'s pattern.

**Step 3: Verify**
```
cargo check -p ra2-rust-game
```
And add a parse-roundtrip test:
```rust
#[test]
fn general_cloak_keys_parse() {
    let ini = r#"
[General]
CloakingStages=9
DefaultMirageDisguises=TREE01,TREE02,TREE03,TREE04
[AudioVisual]
CloakSound=NavalUnitEmerge
"#;
    let parsed = IniFile::from_str(ini).unwrap();
    let rules = GeneralRules::from_ini(&parsed);
    assert_eq!(rules.cloaking_stages, 9);
    assert_eq!(rules.cloak_sound.as_deref(), Some("NavalUnitEmerge"));
    assert_eq!(rules.default_mirage_disguises.len(), 4);
}
```
Run:
```
cargo test general_cloak_keys_parse -p ra2-rust-game
```
Expected: PASS.

**Step 4: Commit**
Message: `rules/ruleset: parse [General] CloakingStages/DefaultMirageDisguises + [AudioVisual] CloakSound`

---

### Task 6: Allocate `Cloak` component at entity spawn for cloakable types

**Why:** Entities need `cloak: Some(Cloak)` initialized when their TypeClass has `cloakable=true`.

**Files:**
- Modify: [src/sim/game_entity.rs](src/sim/game_entity.rs) (or the spawn helper site — find via search)

**Step 1: Find entity spawn sites**
```
grep -rn "voxel_animation:" src/sim/ --include='*.rs' | head -10
grep -rn "type_data\.cloakable\|object_type\.cloakable" src/ --include='*.rs'
grep -rn "GameEntity\s*{" src/sim/ --include='*.rs'
```
Identify all sites that construct a `GameEntity` from a `TypeClass`/`ObjectType`. Likely candidates: `Simulation::spawn_entity`, `WorldEvent::SpawnUnit` handler, production-completion handler, map-init code.

**Step 2: Add a constructor helper**
In `src/sim/components.rs`, add to `impl Cloak`:
```rust
impl Cloak {
    /// Build the initial cloak state for a freshly-spawned cloakable entity.
    /// `current_tick` is the spawn tick — used as the shimmer phase base
    /// (mirroring gamemd's TechnoClass constructor zero-init of +0x1DC; we
    /// use current_tick instead of 0 so units don't all shimmer in sync).
    pub fn at_spawn(current_tick: u32) -> Self {
        Self {
            state: CloakStage::Uncloaked,
            progress: 0,
            step_delta: 1,
            step_timer: 0,
            recloak_delay_timer: 0,
            shimmer_phase_base: current_tick,
            pending_decloak_trigger: false,
        }
    }
}
```

**Step 3: Wire into spawn**
At every entity-spawn site found in Step 1, after the ObjectType lookup, add:
```rust
let cloak = if type_data.cloakable {
    Some(Cloak::at_spawn(current_tick))
} else {
    None
};
```
and pass `cloak` into the `GameEntity` construction.

⚠ **NOTE:** If multiple spawn sites exist, factor a helper `fn init_type_components(type_data: &ObjectType, current_tick: u32) -> TypeComponents` returning a struct of optional components. Add `cloak` to that helper.

**Step 4: Verify**
```
cargo check -p ra2-rust-game
```
Sanity-check by adding a temporary println at the spawn site or running:
```
cargo test -p ra2-rust-game
```
Expected: existing tests pass; no regressions.

**Step 5: Commit**
Message: `sim: allocate Cloak component at spawn for cloakable types`

---

### Task 7: Add `SimSoundEvent::CloakSound` variant

**Why:** Audio cue at cloak/uncloak transitions. Existing enum + drain pattern.

**Files:**
- Modify: [src/sim/world/mod.rs](src/sim/world/mod.rs) — add variant near other event types (around line 94-168)

**Step 1: Add variant**
Inside `pub enum SimSoundEvent`:
```rust
    /// A unit started cloaking or uncloaking — play CloakSound at the unit's
    /// cell position.
    CloakSound { sound_id: InternedId, rx: u16, ry: u16 },
```

**Step 2: Verify enum is exhaustive at all consumers**
```
grep -rn "SimSoundEvent::" src/ --include='*.rs' | grep -v "src/sim/world/mod.rs"
```
For each match site:
- If it's a `match` expression that should be exhaustive (no `_ =>`), add a handler. For audio dispatch sites, dispatch the new variant (Task 17 wires up actual play).
- For now, in any non-audio match site, add `SimSoundEvent::CloakSound { .. } => {}` as a no-op.

**Step 3: Verify**
```
cargo build -p ra2-rust-game 2>&1 | grep -E "error|warning: unreachable"
```
Expected: zero errors. No warnings about non-exhaustive matches.

**Step 4: Commit**
Message: `sim/world: add SimSoundEvent::CloakSound variant`

---

### Task 8: Create `src/sim/cloak.rs` skeleton

**Why:** Declare the module + public API. Body of each function is filled in by Tasks 9-12.

**Files:**
- Create: `src/sim/cloak.rs`
- Modify: [src/sim/mod.rs](src/sim/mod.rs) — add `pub mod cloak;`

**Step 1: Create skeleton**
```rust
//! Cloak FX state machine — drives the per-entity cloak fade animation,
//! state transitions, and shimmer-pulse phase computation.
//!
//! ## Architecture
//!
//! `tick_cloak` runs in `World::advance_tick` between combat (damage application)
//! and retaliation. This placement ensures:
//!   - Damage-decloak triggers fire on the SAME tick as the damage
//!   - Vision (which runs at step 6, BEFORE combat) sees the PREVIOUS tick's
//!     cloak state — a 1-tick lag is acceptable parity with gamemd's per-frame
//!     vision semantics
//!
//! ## Dependencies
//!
//! sim/ only — no render/ui/audio/net imports. Audio is fired via
//! `SimSoundEvent::CloakSound` enqueued into `Simulation.sound_events`.

use crate::rules::ruleset::RulesData;
use crate::sim::components::{Cloak, CloakStage};
use crate::sim::game_entity::GameEntity;
use crate::sim::world::World;
use crate::util::interner::InternedId;

/// Tick all cloak state machines for one game tick. Iterates entities in
/// stable_id order (BTreeMap-deterministic) and advances each `Some(Cloak)`.
pub fn tick_cloak(_world: &mut World, _rules: &RulesData) {
    // TODO Task 12
}

/// Called from combat damage-application site. Sets the decloak-trigger flag;
/// `tick_cloak` processes it on the same tick.
pub fn on_damage(_entity: &mut GameEntity) {
    // TODO Task 11
}

/// Called from combat weapon-fire site when the weapon has `DecloakToFire=yes`.
pub fn on_weapon_fire(_entity: &mut GameEntity) {
    // TODO Task 11
}

/// Compute the visual state 0-5 for an entity from a viewer's perspective.
/// Pure function — no World mutation. Used by the render layer.
///
/// Returns `0` for entities without a Cloak component or in CloakState=0.
pub fn visual_state(
    _entity: &GameEntity,
    _rules: &RulesData,
    _viewer_house: InternedId,
    _cell_sensor_count: u8,
    _is_map_editor: bool,
) -> u8 {
    // TODO Task 9
    0
}

/// Compute the shimmer-pulse alpha multiplier for an allied-viewed cloaked unit.
/// Returns 1.0 (opaque), 0.75 (shimmer), or 0.5 (50% blend) per the 256-tick
/// cycle bands.
pub fn shimmer_phase_alpha(_cloak: &Cloak, _current_tick: u32) -> f32 {
    // TODO Task 10
    1.0
}

/// Compute the raw visual numerator: `(progress * 256) / stages`, integer
/// truncated. Matches gamemd's FIDIV→FMUL→FTOL behavior for positive operands.
pub fn ftol_formula(progress: u8, stages: u32) -> u32 {
    if stages == 0 { return 0; }
    (progress as u32 * 256) / stages
}
```

**Step 2: Register module**
In `src/sim/mod.rs`, add (alphabetized with siblings):
```rust
pub mod cloak;
```

**Step 3: Verify**
```
cargo check -p ra2-rust-game
```
Expected: compiles clean. All public API is reachable but bodies are stubs.

**Step 4: Commit**
Message: `sim/cloak: add module skeleton with public API`

---

### Task 9: Implement `visual_state` pure function

**Why:** The render layer needs this to compute fx_flags + fx_params each frame. Pure function — fully unit-testable.

**Files:**
- Modify: `src/sim/cloak.rs` — replace `visual_state` body

**Step 1: Implement**
```rust
pub fn visual_state(
    entity: &GameEntity,
    rules: &RulesData,
    viewer_house: InternedId,
    cell_sensor_count: u8,
    is_map_editor: bool,
) -> u8 {
    use crate::map::entities::EntityCategory;

    let cloak = match entity.cloak {
        Some(c) => c,
        None => return 0,
    };

    // gamemd GetVisualState 0x00703860 reference:
    //   1. Invisible-type + discovered → 0 (visible)
    //   2. Invisible-type + undiscovered + !editor → 5 (hidden)
    //   3. CloakState=0 → 0
    //   4. Editor → 0
    //   5. Building → 0
    //   6. CloakState=2 → perspective-aware
    //   7. CloakState=1 or 3 → visual_raw thresholds

    // The `Invisible=yes` branch — currently no retail YR type sets this,
    // but the code path is live.
    let type_data = match rules.type_for_entity(entity) {
        Some(td) => td,
        None => return 0,
    };
    let is_discovered = entity.is_discovered_by(viewer_house);

    if type_data.invisible {
        if is_discovered {
            // Discovered Invisible-type: render as normal
        } else if !is_map_editor {
            return 5;
        }
    }

    // CloakState=0 short-circuit
    if cloak.state == CloakStage::Uncloaked {
        return 0;
    }

    if is_map_editor {
        return 0;
    }

    if entity.category == EntityCategory::Building {
        return 0;
    }

    // CloakState=2 (Cloaked) — perspective-aware
    if cloak.state == CloakStage::Cloaked {
        if cell_sensor_count > 0 {
            return 3;
        }
        if is_discovered {
            return 3;
        }
        if entity.owner == viewer_house {
            return 3; // self-view
        }
        if rules.is_allied(entity.owner, viewer_house) {
            return 3;
        }
        return 5;
    }

    // CloakState=1 or 3 (Cloaking/Uncloaking)
    let progress = cloak.progress;
    if progress == 0 {
        return 0;
    }
    let visual_raw = ftol_formula(progress, rules.general.cloaking_stages);
    if visual_raw < 0x40 { return 1; }
    if visual_raw < 0x80 { return 2; }
    if visual_raw < 0xC0 { return 3; }

    // Discovered-clamp: if already discovered and rendering at the high end,
    // clamp to 3 instead of 4 (matches gamemd GetVisualState branch).
    if is_discovered {
        return 3;
    }

    if visual_raw >= 0xFF { 5 } else { 4 }
}
```

⚠ **NOTE:** This task assumes these helpers exist on RulesData/GameEntity:
- `rules.type_for_entity(entity) -> Option<&ObjectType>`
- `rules.is_allied(a, b) -> bool`
- `entity.is_discovered_by(house) -> bool`
- `rules.general` → `&GeneralRules` (containing `cloaking_stages`)

If any helper is missing, add a 5-line stub returning a reasonable default (`false` / `0`), and document in the open-questions section that it's an implementation-time gap to be filled.

**Step 2: Verify**
```
cargo check -p ra2-rust-game
```
Expected: compiles. Function is now real — Task 18 adds the test matrix.

**Step 3: Commit**
Message: `sim/cloak: implement visual_state pure function`

---

### Task 10: Implement `shimmer_phase_alpha` pure function

**Why:** Render layer needs per-frame alpha for allied-shimmer pulse. Pure function — unit-testable.

**Files:**
- Modify: `src/sim/cloak.rs` — replace `shimmer_phase_alpha` body

**Step 1: Implement**
```rust
pub fn shimmer_phase_alpha(cloak: &Cloak, current_tick: u32) -> f32 {
    // gamemd ModifyCloakDrawFlags 0x0070ED80 reference:
    //   phase = (frame - +0x1DC + 0x40) & 0xFF (with signed-wrap correction)
    // Bands:
    //   0x00-0x3F opaque, 0x40-0x43 shimmer, 0x44-0x4B 50%, 0x4C-0x4F shimmer,
    //   0x50-0x6F opaque, 0x70-0x73 shimmer, 0x74-0x7B 50%, 0x7C-0x7F shimmer,
    //   0x80-0xFF opaque
    let frame = current_tick as i64;
    let base = cloak.shimmer_phase_base as i64;
    let raw = frame.wrapping_sub(base).wrapping_add(0x40);
    let phase = (raw as u32) & 0xFF;

    match phase {
        0x00..=0x3F => 1.0,
        0x40..=0x43 => 0.75,
        0x44..=0x4B => 0.5,
        0x4C..=0x4F => 0.75,
        0x50..=0x6F => 1.0,
        0x70..=0x73 => 0.75,
        0x74..=0x7B => 0.5,
        0x7C..=0x7F => 0.75,
        0x80..=0xFF => 1.0,
        _ => 1.0, // unreachable (phase is masked to 0-0xFF)
    }
}
```

**Step 2: Verify**
```
cargo check -p ra2-rust-game
```
Expected: compiles. Test matrix added in Task 18.

**Step 3: Commit**
Message: `sim/cloak: implement shimmer_phase_alpha pure function`

---

### Task 11: Implement `on_damage` and `on_weapon_fire` event hooks

**Why:** Combat needs to signal cloak transitions without directly mutating the state machine. Setter functions keep the API small.

**Files:**
- Modify: `src/sim/cloak.rs`

**Step 1: Implement**
```rust
pub fn on_damage(entity: &mut GameEntity) {
    if let Some(ref mut cloak) = entity.cloak {
        // Damage during state 1 or 2 should trigger uncloak. State 0 doesn't
        // care; state 3 is already uncloaking.
        if matches!(cloak.state, CloakStage::Cloaking | CloakStage::Cloaked) {
            cloak.pending_decloak_trigger = true;
        }
    }
}

pub fn on_weapon_fire(entity: &mut GameEntity) {
    if let Some(ref mut cloak) = entity.cloak {
        // Firing always forces decloak when the weapon has DecloakToFire=yes
        // (gate is checked at the call site in combat).
        if matches!(cloak.state, CloakStage::Cloaking | CloakStage::Cloaked) {
            cloak.pending_decloak_trigger = true;
        }
    }
}
```

**Step 2: Verify**
```
cargo check -p ra2-rust-game
```
Expected: compiles.

**Step 3: Commit**
Message: `sim/cloak: implement on_damage and on_weapon_fire event hooks`

---

### Task 12: Implement `tick_cloak` state machine

**Why:** The core driver — advances cloak progress, handles state transitions, queues sound events.

**Files:**
- Modify: `src/sim/cloak.rs` — replace `tick_cloak` body
- Modify: [src/sim/world/mod.rs](src/sim/world/mod.rs) — insert `cloak::tick_cloak(self, rules)` call between combat-fallout and retaliation in `advance_tick`

**Step 1: Implement `tick_cloak`**
```rust
pub fn tick_cloak(world: &mut World, rules: &RulesData) {
    let stages = rules.general.cloaking_stages.max(1);
    let condition_red_health_ratio = rules.general.condition_red.max(0.0);
    let current_tick = world.tick;

    let keys = world.entities.keys_sorted();
    for sid in keys {
        let Some(entity) = world.entities.get_mut(sid) else { continue; };
        let Some(ref mut cloak) = entity.cloak else { continue; };

        // Tick the re-cloak delay timer down regardless of state.
        cloak.recloak_delay_timer = cloak.recloak_delay_timer.saturating_sub(1);

        // Pending decloak: state 1 or 2 → state 3
        if cloak.pending_decloak_trigger {
            cloak.pending_decloak_trigger = false;
            match cloak.state {
                CloakStage::Cloaking | CloakStage::Cloaked => {
                    cloak.state = CloakStage::Uncloaking;
                    cloak.progress = (stages - 1) as u8;
                    cloak.step_delta = -1;
                    cloak.step_timer = 0; // step on next tick
                    queue_cloak_sound(world, sid, rules);
                    continue;
                }
                _ => {}
            }
        }

        // Per-state logic
        match cloak.state {
            CloakStage::Uncloaked => {
                if cloak.recloak_delay_timer > 0 { continue; }
                let Some(type_data) = rules.type_for_entity_id(entity.type_ref) else { continue; };
                if !type_data.cloakable { continue; }
                // CloakStop: can't auto-cloak while moving (verify via locomotor/movement_target).
                if type_data.cloak_stop && entity.movement_target.is_some() { continue; }

                // Auto-cloak: 4% per tick when health < ConditionRed
                let health_ratio = entity.health.current as f32 / entity.health.max as f32;
                if health_ratio < condition_red_health_ratio {
                    let roll = world.rng.next_range_u32(100);
                    if roll < 4 {
                        // Transition to Cloaking
                        cloak.state = CloakStage::Cloaking;
                        cloak.progress = 0;
                        cloak.step_delta = 1;
                        cloak.step_timer = type_data.cloaking_speed as u16;
                        queue_cloak_sound(world, sid, rules);
                    }
                }
            }
            CloakStage::Cloaking => {
                // Tick timer; step progress; check transition
                if cloak.step_timer > 0 {
                    cloak.step_timer -= 1;
                } else {
                    cloak.progress = cloak.progress.saturating_add_signed(cloak.step_delta);
                    let speed = rules.type_for_entity_id(entity.type_ref)
                        .map(|t| t.cloaking_speed).unwrap_or(1) as u16;
                    cloak.step_timer = speed.max(1) - 1;
                }
                // Check visual_state for transition: 3 or 5 → Cloaked
                let visual_raw = ftol_formula(cloak.progress, stages);
                let visual = if visual_raw < 0x40 { 1 }
                    else if visual_raw < 0x80 { 2 }
                    else if visual_raw < 0xC0 { 3 }
                    else if visual_raw >= 0xFF { 5 }
                    else { 4 };
                if visual == 3 || visual == 5 {
                    cloak.state = CloakStage::Cloaked;
                    cloak.progress = 0;
                    cloak.shimmer_phase_base = current_tick;
                }
                // Abort-uncloak: visual==2 + low health + 10% RNG
                if visual == 2 {
                    let health_ratio = entity.health.current as f32 / entity.health.max as f32;
                    if health_ratio < condition_red_health_ratio {
                        let roll = world.rng.next_range_u32(100);
                        if roll <= 9 {
                            cloak.state = CloakStage::Uncloaking;
                            cloak.step_delta = -1;
                        }
                    }
                }
            }
            CloakStage::Cloaked => {
                // ShouldUncloak: any vtable+0x2A4 logic. Stub: no-op for PR 1.
                // Decloak triggers are caught via pending_decloak_trigger above.
            }
            CloakStage::Uncloaking => {
                if cloak.step_timer > 0 {
                    cloak.step_timer -= 1;
                } else {
                    cloak.progress = cloak.progress.saturating_add_signed(cloak.step_delta);
                    let speed = rules.type_for_entity_id(entity.type_ref)
                        .map(|t| t.cloaking_speed).unwrap_or(1) as u16;
                    cloak.step_timer = speed.max(1) - 1;
                }
                if cloak.progress == 0 {
                    cloak.state = CloakStage::Uncloaked;
                    cloak.step_delta = 1;
                    // Start re-cloak delay (mirrors gamemd ReCloakDelayTimer)
                    cloak.recloak_delay_timer = 30; // ~2s at 15Hz; verify against gamemd if visible
                }
            }
        }
    }
}

fn queue_cloak_sound(world: &mut World, sid: u64, rules: &RulesData) {
    let Some(entity) = world.entities.get(sid) else { return; };
    let Some(sound_id) = rules.general.cloak_sound.as_ref() else { return; };
    let resolved = rules.intern_voc(sound_id); // returns Option<InternedId> if VocClass exists
    if let Some(id) = resolved {
        world.sound_events.push(crate::sim::world::SimSoundEvent::CloakSound {
            sound_id: id,
            rx: entity.position.rx,
            ry: entity.position.ry,
        });
    }
}
```

⚠ **NOTE:** `rules.type_for_entity_id` and `rules.intern_voc` are placeholder names — find the actual helpers and substitute. Also `condition_red` may be `f32` or `FixedPoint` — match the existing type. The health ratio computation uses `f32` here, which is acceptable since it's a comparison only (no sim state written from a float).

**Step 2: Insert call into `advance_tick`**
Locate `World::advance_tick` (around line 970+). Find the combat phase end (after `tick_combat_with_fog` and turret rotation, around line 1201) and BEFORE `tick_retaliation` (around line 1347-1349).

Add:
```rust
        // --- Phase 5.7: Cloak FX state machine ---
        // Runs after combat damage application so decloak-on-damage fires
        // on the same tick. Runs before retaliation so retaliation sees
        // updated cloak state.
        crate::sim::cloak::tick_cloak(self, rules);
```

**Step 3: Verify compilation**
```
cargo check -p ra2-rust-game
```
Expected: compiles. Existing tests may fail if `condition_red` etc. haven't been added to `GeneralRules` — handle by completing those fields first if needed.

**Step 4: Commit**
Message: `sim/cloak: implement tick_cloak state machine + wire into advance_tick`

---

### Task 13: Hook damage application to `cloak::on_damage`

**Why:** Damage events must trigger decloak on the same tick.

**Files:**
- Modify: [src/sim/combat/mod.rs](src/sim/combat/mod.rs) — at the damage-application site (around line 1794)

**Step 1: Locate the site**
Find the line:
```rust
target.health.current = target.health.current.saturating_sub(*damage);
```
(or equivalent — pattern: any line that decrements `target.health.current` from a damage amount).

**Step 2: Add hook call**
Immediately AFTER the damage application, add:
```rust
crate::sim::cloak::on_damage(target);
```

The hook is cheap (None-check + bool set); applies to every damage event regardless of cloak status.

**Step 3: Verify**
```
cargo check -p ra2-rust-game && cargo test combat -p ra2-rust-game
```
Expected: existing combat tests pass.

**Step 4: Commit**
Message: `combat: hook cloak::on_damage at damage-application site`

---

### Task 14: Hook weapon-fire to `cloak::on_weapon_fire` (DecloakToFire)

**Why:** Firing a `DecloakToFire=yes` weapon must decloak the firer on the same tick.

**Files:**
- Modify: [src/sim/combat/mod.rs](src/sim/combat/mod.rs) — at the weapon-fire site

**Step 1: Locate the site**
Find the function that handles "this entity fires its weapon at a target" — likely `try_fire_at` or similar inside `tick_combat`. Look for a call to a damage-application helper, or a line that creates a Bullet/Projectile.

**Step 2: Add gated hook call**
At the fire-confirmed site, add:
```rust
if weapon_data.decloak_to_fire {
    crate::sim::cloak::on_weapon_fire(firer);
}
```

Note: `weapon_data.decloak_to_fire` is the existing field at [src/rules/weapon_type.rs](src/rules/weapon_type.rs) (parsed already per Agent C).

**Step 3: Verify**
```
cargo check -p ra2-rust-game
```
Expected: compiles.

**Step 4: Commit**
Message: `combat: hook cloak::on_weapon_fire when weapon.decloak_to_fire`

---

### Task 15: Populate `fx_flags` and `fx_params[0]` in `app_instances/units.rs`

**Why:** Render layer reads cloak state and produces shader uniforms.

**Files:**
- Modify: [src/app_instances/units.rs](src/app_instances/units.rs) — at the SpriteInstance push sites (around line 238-249 and 405-416)

**Step 1: Add helper computation before push**
Inside `build_unit_instances` (around line 84 onwards), after the visibility check and before pushing the SpriteInstance, compute the visual_state once per entity:

```rust
// Cloak FX computation
let visual_state = crate::sim::cloak::visual_state(
    entity,
    &state.rules.as_ref().unwrap(),
    local_owner_id.unwrap_or_default(),
    state.resolved_terrain.as_ref()
        .and_then(|t| t.cell(pos.rx, pos.ry))
        .map(|c| c.sensor_count_for(local_owner_id.unwrap_or_default()))
        .unwrap_or(0),
    state.is_map_editor,
);
if visual_state == 5 {
    continue; // skip draw entirely
}
let (fx_flags, fx_params_0) = compute_cloak_fx_uniform(
    entity,
    visual_state,
    state.tick,
    local_owner_id,
);
```

Then at each `target_instances.push(SpriteInstance { ... })` site, replace `..Default::default()` with explicit field initialization:
```rust
target_instances.push(SpriteInstance {
    position: [...],
    size: entry.pixel_size,
    uv_origin: entry.uv_origin,
    uv_size: entry.uv_size,
    depth,
    tint,
    alpha,
    house_color_idx: house_color_to_remap_row(hc),
    fx_flags,
    fx_params: [fx_params_0, 0.0, 0.0, 0.0],
    ic_tint: [0.0, 0.0, 0.0, 0.0],
});
```

**Step 2: Add the helper at the bottom of the file**
```rust
/// Compute the cloak FX uniform fields for a SpriteInstance.
/// Returns (fx_flags, fx_params[0]) — fx_flags bit 0 set when cloak FX applies.
fn compute_cloak_fx_uniform(
    entity: &crate::sim::game_entity::GameEntity,
    visual_state: u8,
    current_tick: u32,
    local_owner_id: Option<crate::util::interner::InternedId>,
) -> (u32, f32) {
    use crate::sim::components::CloakStage;

    if visual_state == 0 {
        return (0, 1.0);
    }
    let cloak = match &entity.cloak {
        Some(c) => c,
        None => return (0, 1.0),
    };
    let alpha = match visual_state {
        1 => 0.75,
        2 | 3 => {
            // Allied shimmer pulse for player-controlled units in state 3
            if visual_state == 3 && Some(entity.owner) == local_owner_id {
                crate::sim::cloak::shimmer_phase_alpha(cloak, current_tick)
            } else {
                0.5
            }
        }
        4 => if entity.is_voxel { 0.75 } else { 0.5 },
        _ => 1.0, // state 0 handled above; state 5 skipped at caller
    };
    (1, alpha)
}
```

**Step 3: Verify**
```
cargo check -p ra2-rust-game
```
Expected: compiles. Run the game in dev mode (if available) to sanity-check that a SUB renders normally when uncloaked.

**Step 4: Commit**
Message: `app_instances/units: populate fx_flags + fx_params[0] from cloak state`

---

### Task 16: Extend `apply_fx` shader with Path-A dither formula

**Why:** Replace the flat-alpha stub with the per-fragment dither that matches gamemd's intensity-LUT shimmer.

**Files:**
- Modify: [src/render/sprite_voxel_shader.wgsl](src/render/sprite_voxel_shader.wgsl) — lines 98-111

**Step 1: Update the function signature to receive frag_pos**
Locate `fn apply_fx(color: vec4f, flags: u32, params: vec4f, ic: vec4f) -> vec4f` (line 98). Modify to:
```wgsl
fn apply_fx(color: vec4f, flags: u32, params: vec4f, ic: vec4f, frag_pos: vec2f) -> vec4f {
    var c = color;
    if ((flags & 1u) != 0u) {
        // Path-A dither parity. Reproduces gamemd's intensity-table formula:
        //   val = clamp((abuf * intensity * 254) / 32258, 0, 254)
        // params.x carries the target alpha in [0, 1]; we translate it to the
        // [0, 254] intensity_clamp domain. abuf_hash is a screen-space pseudo-random
        // mimicking gamemd's a-buffer dither pattern.
        let intensity_clamp: u32 = u32(clamp(params.x * 254.0, 0.0, 254.0));
        let abuf: u32 = u32(fract(sin(dot(frag_pos, vec2f(12.9898, 78.233))) * 43758.5453) * 256.0) & 0xFFu;
        let val_raw: u32 = (abuf * intensity_clamp * 254u) / 32258u;
        let val: u32 = min(val_raw, 254u);
        c.a = c.a * (f32(val) / 254.0);
    }
    if ((flags & 2u) != 0u) {
        let luma = dot(c.rgb, vec3f(0.299, 0.587, 0.114));
        c = vec4f(mix(c.rgb, vec3f(luma), params.y), c.a);
    }
    if ((flags & 4u) != 0u) {
        c = vec4f(mix(c.rgb, ic.rgb, ic.a), c.a);
    }
    if ((flags & 8u) != 0u) { c.a = c.a * params.w; }
    return c;
}
```

**Step 2: Update the call site in `fs_main`**
Locate the existing `apply_fx(color, flags, params, ic)` call in the fragment shader. Change to:
```wgsl
return apply_fx(color, in.fx_flags, in.fx_params, in.ic_tint, in.clip_position.xy);
```
(or `in.frag_coord.xy` if that's the available builtin in this shader).

**Step 3: Verify**
Reload the shader and check the wgpu validation layer doesn't complain:
```
cargo run -p ra2-rust-game 2>&1 | grep -i "wgsl\|shader" | head -20
```
Expected: no shader-compile errors. Open a map with a Cloakable unit; trigger cloak (or manually populate `cloak.state = Cloaking` via debug tooling); verify dithered pattern appears.

**Step 4: Commit**
Message: `render: extend apply_fx with Path-A dither parity for cloak`

---

### Task 17: Dispatch `SimSoundEvent::CloakSound` in app audio layer

**Why:** Audio drain site must handle the new variant.

**Files:**
- Modify: app audio drain site (find via `grep -rn "sound_events.drain\|SimSoundEvent::" src/app*`)

**Step 1: Locate the drain site**
```
grep -rn "SimSoundEvent::" src/app_audio src/app_instances src/app 2>/dev/null
```
Find the function that match-dispatches `SimSoundEvent` variants.

**Step 2: Add the variant dispatch**
```rust
SimSoundEvent::CloakSound { sound_id, rx, ry } => {
    // Play VocClass at cell (rx, ry). Mirrors existing sound dispatch for
    // EntityDied / WeaponFired.
    play_voc_at_cell(audio, sound_id, rx, ry, &state);
}
```

Use whatever helper exists for the other variants (`play_voc_at_cell` is a placeholder — substitute the actual name).

**Step 3: Verify**
```
cargo check -p ra2-rust-game
```
Run game; trigger cloak transition; verify CloakSound (`NavalUnitEmerge` by default) plays.

**Step 4: Commit**
Message: `app_audio: dispatch SimSoundEvent::CloakSound to VocClass play`

---

### Task 18: Unit tests for `visual_state`, `shimmer_phase_alpha`, `ftol_formula`

**Why:** Pure-logic regression coverage for the parity-critical formulas.

**Files:**
- Modify: `src/sim/cloak.rs` — add `#[cfg(test)] mod tests` at end

**Step 1: Add tests**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::components::Cloak;

    #[test]
    fn ftol_formula_known_values() {
        // gamemd CloakingStages=9 reference progression
        assert_eq!(ftol_formula(0, 9), 0);
        assert_eq!(ftol_formula(1, 9), 28);
        assert_eq!(ftol_formula(2, 9), 56);
        assert_eq!(ftol_formula(3, 9), 85);
        assert_eq!(ftol_formula(4, 9), 113);
        assert_eq!(ftol_formula(5, 9), 142);
        assert_eq!(ftol_formula(6, 9), 170);
        assert_eq!(ftol_formula(7, 9), 199);
        assert_eq!(ftol_formula(8, 9), 227);
        assert_eq!(ftol_formula(9, 9), 256);
    }

    #[test]
    fn ftol_formula_handles_zero_stages() {
        assert_eq!(ftol_formula(5, 0), 0);
    }

    #[test]
    fn ftol_formula_handles_alt_stages() {
        // Non-default CloakingStages — ensure scaling correctness
        assert_eq!(ftol_formula(2, 4), 128);
        assert_eq!(ftol_formula(4, 4), 256);
    }

    fn make_cloak(phase_base: u32) -> Cloak {
        Cloak {
            state: CloakStage::Cloaked,
            progress: 0,
            step_delta: 0,
            step_timer: 0,
            recloak_delay_timer: 0,
            shimmer_phase_base: phase_base,
            pending_decloak_trigger: false,
        }
    }

    #[test]
    fn shimmer_bands_all_phases() {
        let c = make_cloak(0);
        // phase = (tick - 0 + 0x40) & 0xFF = (tick + 0x40) & 0xFF
        // Verify each boundary:
        // tick=0 → phase=0x40 → 0.75 (shimmer)
        assert_eq!(shimmer_phase_alpha(&c, 0), 0.75);
        // tick=3 → phase=0x43 → 0.75
        assert_eq!(shimmer_phase_alpha(&c, 3), 0.75);
        // tick=4 → phase=0x44 → 0.5
        assert_eq!(shimmer_phase_alpha(&c, 4), 0.5);
        // tick=0x0B → phase=0x4B → 0.5
        assert_eq!(shimmer_phase_alpha(&c, 0x0B), 0.5);
        // tick=0x0C → phase=0x4C → 0.75 (shimmer — corrected from prior doc)
        assert_eq!(shimmer_phase_alpha(&c, 0x0C), 0.75);
        // tick=0x0F → phase=0x4F → 0.75
        assert_eq!(shimmer_phase_alpha(&c, 0x0F), 0.75);
        // tick=0x10 → phase=0x50 → 1.0 (opaque)
        assert_eq!(shimmer_phase_alpha(&c, 0x10), 1.0);
        // tick=0x2F → phase=0x6F → 1.0
        assert_eq!(shimmer_phase_alpha(&c, 0x2F), 1.0);
        // tick=0x30 → phase=0x70 → 0.75
        assert_eq!(shimmer_phase_alpha(&c, 0x30), 0.75);
        // tick=0x33 → phase=0x73 → 0.75
        assert_eq!(shimmer_phase_alpha(&c, 0x33), 0.75);
        // tick=0x34 → phase=0x74 → 0.5
        assert_eq!(shimmer_phase_alpha(&c, 0x34), 0.5);
        // tick=0x3B → phase=0x7B → 0.5
        assert_eq!(shimmer_phase_alpha(&c, 0x3B), 0.5);
        // tick=0x3C → phase=0x7C → 0.75 (corrected from prior doc)
        assert_eq!(shimmer_phase_alpha(&c, 0x3C), 0.75);
        // tick=0x3F → phase=0x7F → 0.75
        assert_eq!(shimmer_phase_alpha(&c, 0x3F), 0.75);
        // tick=0x40 → phase=0x80 → 1.0
        assert_eq!(shimmer_phase_alpha(&c, 0x40), 1.0);
        // tick=0xBF → phase=0xFF → 1.0
        assert_eq!(shimmer_phase_alpha(&c, 0xBF), 1.0);
        // tick=0xC0 → phase=0x00 → 1.0
        assert_eq!(shimmer_phase_alpha(&c, 0xC0), 1.0);
    }

    #[test]
    fn shimmer_phase_handles_negative_offset() {
        // phase_base in the future (e.g., from a save-load edge case)
        let c = make_cloak(1000);
        // Should not panic; result is implementation-defined but stable.
        let _ = shimmer_phase_alpha(&c, 0);
        let _ = shimmer_phase_alpha(&c, 500);
    }
    
    // Visual-state tests require a minimal GameEntity + RulesData fixture.
    // Add similar test cases here as helper fixtures become available.
}
```

**Step 2: Run**
```
cargo test cloak -p ra2-rust-game -- --nocapture
```
Expected: all PASS.

**Step 3: Commit**
Message: `sim/cloak: unit tests for visual_state / shimmer_phase / ftol_formula`

---

### Task 19: Integration test — full cloak cycle

**Why:** End-to-end verification that the state machine + audio + state hash all behave deterministically.

**Files:**
- Create: `tests/cloak_cycle.rs` (or extend `src/sim/cloak.rs::tests` if integration-style tests live inline per repo convention)

**Step 1: Build the test**
```rust
//! Integration test: a Cloakable unit cycles through all 4 states; sound
//! events fire at the right transitions; state hash is deterministic.

use ra2_rust_game::sim::components::CloakStage;
// (use whatever the actual crate name is; substitute via grep src/lib.rs)

#[test]
fn cloak_full_cycle_deterministic() {
    // Set up a minimal world with one Cloakable entity.
    let mut sim = build_test_simulation(/*seed*/ 12345);
    let entity_id = sim.spawn_cloakable_unit("SUB", /*tick*/ 0);

    // Force-trigger cloak: simulate auto-cloak by setting health to low and
    // ticking until the state machine fires the 4% chance.
    sim.set_health_ratio(entity_id, 0.1); // below ConditionRed

    let mut sound_events_seen = 0;
    let mut state_history = Vec::new();
    for tick in 0..500 {
        sim.advance_tick();
        let entity = sim.entity(entity_id);
        let state = entity.cloak.as_ref().map(|c| c.state).unwrap();
        state_history.push((tick, state, entity.cloak.unwrap().progress));
        for ev in sim.drain_sound_events() {
            if matches!(ev, ra2_rust_game::sim::world::SimSoundEvent::CloakSound { .. }) {
                sound_events_seen += 1;
            }
        }
    }

    // Verify state machine traversed all 4 states
    assert!(state_history.iter().any(|(_, s, _)| *s == CloakStage::Cloaking));
    assert!(state_history.iter().any(|(_, s, _)| *s == CloakStage::Cloaked));

    // Sounds: at least 1 (for the 0→1 transition).
    assert!(sound_events_seen >= 1);
}

#[test]
fn cloak_state_hash_deterministic() {
    // Run the same scenario twice; assert state_hash matches at every tick.
    let mut sim_a = build_test_simulation(12345);
    let mut sim_b = build_test_simulation(12345);
    let id_a = sim_a.spawn_cloakable_unit("SUB", 0);
    let id_b = sim_b.spawn_cloakable_unit("SUB", 0);
    sim_a.set_health_ratio(id_a, 0.1);
    sim_b.set_health_ratio(id_b, 0.1);
    for tick in 0..200 {
        sim_a.advance_tick();
        sim_b.advance_tick();
        assert_eq!(sim_a.state_hash(), sim_b.state_hash(), "diverged at tick {}", tick);
    }
}
```

**Step 2: Helpers**
The `build_test_simulation` / `spawn_cloakable_unit` / `set_health_ratio` helpers are placeholders — find or add a minimal test-fixture builder. If no such builder exists, this task may need to be split (Task 19a: build the test fixture; Task 19b: write the assertions).

**Step 3: Run**
```
cargo test cloak_full_cycle -p ra2-rust-game
cargo test cloak_state_hash -p ra2-rust-game
```
Expected: both PASS.

**Step 4: Commit**
Message: `tests: cloak full-cycle integration + state-hash determinism`

---

### Task 20: Manual verification against gamemd.exe

**Why:** Final parity check — the unit tests verify formulas, but only side-by-side capture verifies the player-observable result.

**Files:** None (manual)

**Step 1: Set up scenarios**
Open the engine in dev mode AND open gamemd.exe with the same map+units. Use a skirmish with one human player and one AI; spawn a SUB on each side.

**Step 2: Capture each scenario**
For each of the 6 scenarios below, record a short clip (or take 3-5 screenshots per second) in BOTH engines:
1. SUB cloaking up (state 0 → 2) over ~9 ticks
2. Player-owned cloaked SUB sitting idle (allied shimmer pulse over 256 ticks ≈ 17s at 15Hz)
3. Enemy SUB cloaked, viewed from player camera with no sensor (should be invisible)
4. Enemy SUB cloaked, viewed when a DEST is nearby (should render at 50% blend due to sensor)
5. SUB taking damage while cloaked (state 2 → 3 → 0 over ~9 ticks)
6. SUB firing weapon (DecloakToFire) — verify decloak triggers on weapon fire

**Step 3: Diff**
Compare side-by-side:
- Cloak fade timing (should be visually identical at 15Hz capture)
- Allied shimmer cadence (count shimmer/blend frames vs opaque frames in a 256-tick window)
- Audio: CloakSound plays once per transition
- Pixel-level dither pattern (Path A — expect approximation, NOT exact pixel match)

**Step 4: Document**
Record findings in a follow-up doc `docs/notes/2026-XX-XX-cloak-fx-parity-check.md`:
- What matched exactly
- What differs (especially dither pattern — note as Phase 2.2 follow-up)
- Any parity gaps found

**Step 5:** No commit — this is verification, not implementation.

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-11-cloak-fx-rust-integration-design.md](docs/plans/2026-05-11-cloak-fx-rust-integration-design.md)
- **Research doc:** [docs/research/CLOAK_FX_SHADER_BRIDGE_GHIDRA_REPORT.md](docs/research/CLOAK_FX_SHADER_BRIDGE_GHIDRA_REPORT.md)
- **Prior reports consulted:** CLOAKING_VISUAL_PIPELINE.md, CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md, CLOAKING_INTERACTIONS_REPORT.md, DISGUISE_SYSTEM_GHIDRA_REPORT.md, SENSOR_CLOAK_DETECTION.md
- **gamemd.exe primary addresses** (kept here, NOT in Rust comments):
  - `TechnoClass__CloakingTick @ 0x006FB740` — state machine
  - `TechnoClass_GetVisualState @ 0x00703860` — visual_state formula
  - `TechnoClass__ModifyCloakDrawFlags @ 0x0070ED80` — shimmer pulse
  - `TechnoClass__StartCloaking @ 0x00703770` — state 0→1 / 3→1
  - `TechnoClass__StartUncloaking @ 0x007036C0` — state 2→3 / 1→3
  - `FUN_00420140 @ 0x00420140` — intensity table generator
  - `Blitter_Shimmer_75pct_Remap @ 0x00494330` — 75/25 blend
  - `g_CurrentFrameCounter @ 0x00A8ED84` — game tick global
- **INI keys:** rulesmd.ini `[General] CloakingStages=9`, `[General] DefaultMirageDisguises=TREE01,TREE02,TREE03,TREE04`, `[AudioVisual] CloakSound=NavalUnitEmerge`, per-type `Cloakable=`, `CloakingSpeed=`, `CloakStop=`, `Invisible=`, `Sensors=`, `SensorsSight=`, `CanDisguise=`, `PermaDisguise=`, `DisguiseWhenStill=`, `DetectDisguise=`, `DetectDisguiseRange=`
- **Repo patterns mirrored:**
  - `tick_building_up` [src/sim/world/mod.rs:900-919](src/sim/world/mod.rs#L900) — tick_X system shape
  - `Health` [src/sim/components.rs:87](src/sim/components.rs#L87) — component struct derives
  - `c4_plant` hash extension (commit `b1b60e9`) — state hash extension pattern
  - `radar_invisible` parse [src/rules/object_type.rs:806](src/rules/object_type.rs#L806) — INI parser shape
  - `condition_yellow` / `building_garrisoned_sound` [src/rules/ruleset.rs:224, 234](src/rules/ruleset.rs#L224) — GeneralRules pattern
  - `EntityDied` push [src/sim/movement/bump_crush.rs:445](src/sim/movement/bump_crush.rs#L445) — SimSoundEvent push site
- **Related plans:**
  - [docs/plans/2026-05-10-voxel-gpu-remap-fx-design.md](docs/plans/2026-05-10-voxel-gpu-remap-fx-design.md) — parent design (Phase 0+1 already shipped)
  - [docs/plans/2026-05-11-cloak-fx-investigation-plan.md](docs/plans/2026-05-11-cloak-fx-investigation-plan.md) — investigation plan that produced the research doc
- **Recent commits informing this plan:**
  - `e5c060d` — added FX uniform fields to SpriteInstance
  - `8a47d4c` — added voxel-sprite shader with apply_fx stubs
  - `b1b60e9` — extended hash_entities with c4_plant fields (template for Task 3)
  - `a2aeaa1` — added C4PlantState component (template for Task 1)
  - `ee33e38` — added c4_plant field to GameEntity (template for Task 2)
