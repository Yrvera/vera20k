# Guardian GI (GGI) Rust Integration — Design

**Date:** 2026-05-17
**Source research:** [`ra2-rust-game-docs/GGI_GHIDRA_REPORT.md`](../../../ra2-rust-game-docs/GGI_GHIDRA_REPORT.md) — 9 sections, all parity-critical findings resolved.
**Brainstorm context:** the §6 Rust-gap inventory in the GGI report listed 12 items; commits `e3a724f` (low-silhouette crush) and `1391629` (fire-to-anim sync) closed 5 of them. This design covers the remaining **6 gaps**.

## Goal

Wire GGI into the existing engine so a player switching between gamemd.exe and our build cannot tell them apart in a skirmish — for both walking M60 fire and deployed MissileLauncher fire, including BFRT and IFV transport cases, with veteran/elite tier swaps applied correctly.

## Architecture Context

GGI is the Allied secondary infantry. It shares the entire InfantryClass surface with E1 (covered in [`GI_GHIDRA_REPORT.md`](../../../ra2-rust-game-docs/GI_GHIDRA_REPORT.md)). **No GGI-specific code branch exists in gamemd** — every difference is value-driven: different INI keys produce different runtime behavior through the shared infantry state machine.

Touched modules (all in `src/`):

- `rules/object_type.rs` — adds two ObjectType fields (`elite_primary`, `elite_secondary`, `open_transport_weapon`) and parsing for them. `DeployedCrushable` parse already exists at line 943.
- `rules/art_data.rs` — extends per-image art entries with three sequence-Length fields (Deploy/Undeploy/DeployedFire frame counts).
- `sim/deploy.rs` — `compute_anim_ticks` becomes art-aware; defaults preserved as fallback.
- `sim/combat/combat_weapon.rs` — `select_weapon*` learn veterancy-aware elite swap and a new `WeaponOverride` enum that distinguishes IFV-slot vs OpenTransport routing.
- `sim/passenger.rs` — sets the new `WeaponOverride` variant based on whether the transport has `Gunner=yes`.
- `sim/world/world_commands.rs` — reorders DeploySound emit to fire before the state field write.
- `sim/combat/combat_targeting.rs` — verification pass for auto-deploy gating (no code change expected).
- `sim/movement/homing_movement.rs` — **new module** for AAHeatSeeker2-style homing projectiles. Sits alongside the existing ballistic `rocket_movement.rs`.

The Rust sim layer's hard invariant — `sim/` never depends on `render/`, `ui/`, `sidebar/`, `audio/`, `net/` — is preserved by every change here.

## Impact Analysis

| Gap | Files | Blast |
|-----|-------|-------|
| F  DeploySound order | `world_commands.rs:537–561` (move 12 lines) | LOW |
| C  AA cursor / auto-deploy verify | `combat_targeting.rs` + UI cursor surface | LOW (verification only) |
| G  OpenTransportWeapon + BFRT routing | `object_type.rs`, `passenger.rs:408–412`, `combat_weapon.rs:88–147` | MED |
| D  Veteran/Elite weapon swap | `object_type.rs`, `combat_weapon.rs`, 2 caller sites | MED |
| B  Deploy duration from art Length | `art_data.rs`, `ruleset.rs`, `deploy.rs:47–49`, `world_commands.rs:520,526` | MED |
| E  Homing missile module | new `homing_movement.rs` (~300 LoC), projectile-spawn dispatch | HIGH |

**Determinism:** the homing module uses SimFixed for all sim-critical math (yaw, position, vz, stall EMA) and a u32 frame counter for sidewinder phase. Sin/cos lookups for velocity rebuild are flagged for either an in-house BAM table or careful f32-with-final-SimFixed-truncation (decided at implementation).

**State hash:** new fields on RocketState/HomingState participate in `world_hash`. Adding fields requires a hash-schema bump (existing pattern in the codebase).

## Chosen Approach

Six gaps, six self-contained changes, in dependency order (small → large). No gap requires another's design; they are additive. The only architectural fork (homing vs ballistic) is settled: **parallel module**.

### F — DeploySound trigger order

**Change:** in [`src/sim/world/world_commands.rs`](../../src/sim/world/world_commands.rs), reorder the deploy/undeploy command handler so the `SimSoundEvent::EntityDeployed` / `EntityUndeployed` emit happens **before** `entity.deploy_state = new_phase;` on line 537.

Today (paraphrased):

```rust
// line 517:    match entity.deploy_state { ... compute new_phase ... }
// line 537:    entity.deploy_state = new_phase;
// line 539:    if entering Deploy → emit EntityDeployed { ... }
// line 550:    if entering Undeploy → emit EntityUndeployed { ... }
```

After:

```rust
// match entity.deploy_state { ... compute new_phase ... }
// if entering Deploy → emit EntityDeployed { ... }
// if entering Undeploy → emit EntityUndeployed { ... }
// entity.deploy_state = new_phase;  ← LAST
```

Six lines moved; no struct changes; tests already verify emission so they continue to pass.

### C — AA cursor / no auto-deploy (verification only)

**Inspection checklist:**

1. `combat_targeting.rs::scan_targets_for_entity` — confirm no path issues `DeployPhase::Deploying { … }` when an air target is acquired.
2. Cursor surface (likely under `src/ui/` or `src/sidebar/` — locate it during execution) — confirm no "deploy-on-air-target" cursor branch exists.
3. Confirm `select_weapon`'s AA gate at `combat_weapon.rs:251` returns `None` for the walking GGI + air target case — already true per the impact-analysis read.

If the inspection finds an auto-deploy branch, document it as a follow-up; do not silently add it under this design's umbrella.

**Expected outcome:** no code change.

### G — OpenTransportWeapon + BFRT routing

**Field additions to `ObjectType`** in [`src/rules/object_type.rs`](../../src/rules/object_type.rs):

```rust
/// `OpenTransportWeapon=` from rules.ini. Index meaning:
/// - 0 → fire passenger's Primary
/// - 1 → fire passenger's Secondary
/// - -1 → no override (default)
///
/// Consumed only when passenger is inside an open-topped transport
/// that does NOT have `Gunner=yes`. For Gunner transports, the
/// passenger's IFVMode + transport's weapon_list[] take over instead.
#[serde(default = "default_neg_one")]
pub open_transport_weapon: i32,

fn default_neg_one() -> i32 { -1 }
```

Parse `OpenTransportWeapon=` alongside the existing `IFVMode=` block.

**Replace `Option<u32> ifv_weapon_index`** on `GameEntity` (or wherever it lives — confirm during implementation) with:

```rust
// src/sim/combat/combat_weapon.rs (new top-of-file enum)
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WeaponOverride {
    /// IFV (Gunner=yes) — fire transport's weapon_list[idx] using the
    /// gattling/multi-weapon path. idx is the passenger's IFVMode.
    IfvSlot(u32),
    /// BFRT-style (OpenTopped=yes, no Gunner) — fire passenger's own
    /// Primary (0) or Secondary (1) per OpenTransportWeapon.
    OpenTransport(u32),
}
```

**`passenger.rs:408–412` change:**

```rust
let override_ = if transport_gunner {
    Some(WeaponOverride::IfvSlot(pax_ifv_mode))
} else if transport_open_topped && pax_open_transport_weapon >= 0 {
    Some(WeaponOverride::OpenTransport(pax_open_transport_weapon as u32))
} else {
    None
};
attacker.weapon_override = override_;
```

**`select_weapon_with_ifv` becomes `select_weapon_with_override`:**

```rust
pub fn select_weapon_with_override<'a>(
    rules: &'a RuleSet,
    attacker_obj: &'a ObjectType,
    target_category: EntityCategory,
    target_armor: &str,
    veterancy: u16,                       // ← from Gap D below
    override_: Option<WeaponOverride>,
) -> Option<SelectedWeapon<'a>> {
    match override_ {
        Some(WeaponOverride::IfvSlot(idx)) => {
            // existing IFV path — read attacker_obj.weapon_list[idx]
            // (here attacker_obj is the transport, not the passenger)
        }
        Some(WeaponOverride::OpenTransport(0)) => {
            // fire passenger's Primary
            try_weapon(rules, attacker_obj.primary.as_deref()?, …)
        }
        Some(WeaponOverride::OpenTransport(1)) => {
            // fire passenger's Secondary
            try_weapon(rules, attacker_obj.secondary.as_deref()?, …)
        }
        Some(WeaponOverride::OpenTransport(_)) | None => {
            // fall through to base Primary→Secondary selection
        }
    }
    // existing primary/secondary fallback
}
```

For `WeaponOverride::IfvSlot`, the attacker_obj passed in is the **transport's** ObjectType (since it's the transport's weapon firing). For `WeaponOverride::OpenTransport`, the attacker_obj is the **passenger's** ObjectType. Caller sites must pass the right one — document this contract clearly.

### D — Veteran/Elite weapon swap

**Field additions to `ObjectType`:**

```rust
#[serde(default)]
pub elite_primary: Option<String>,

#[serde(default)]
pub elite_secondary: Option<String>,
```

Parse `ElitePrimary=` and `EliteSecondary=` keys.

**Veteran tier:** between Rookie (0) and Veteran (100..200), no weapon swap occurs in gamemd — the `VeteranAbilities` line applies damage/ROF/sight multipliers but the weapon ID is unchanged. The swap only happens at **Elite** (`veterancy >= 200`).

**`select_weapon` signature change:**

```rust
pub fn select_weapon<'a>(
    rules: &'a RuleSet,
    attacker_obj: &'a ObjectType,
    target_category: EntityCategory,
    target_armor: &str,
    veterancy: u16,        // ← NEW
) -> Option<SelectedWeapon<'a>>
```

**Resolution helpers** (mirror `select_garrison_weapon`):

```rust
fn primary_for_tier(obj: &ObjectType, veterancy: u16) -> Option<&str> {
    let is_elite = veterancy >= 200;
    if is_elite {
        obj.elite_primary.as_deref().or(obj.primary.as_deref())
    } else {
        obj.primary.as_deref()
    }
}

fn secondary_for_tier(obj: &ObjectType, veterancy: u16) -> Option<&str> {
    let is_elite = veterancy >= 200;
    if is_elite {
        obj.elite_secondary.as_deref().or(obj.secondary.as_deref())
    } else {
        obj.secondary.as_deref()
    }
}
```

**Callers** at `combat_targeting.rs:204` and `combat/mod.rs:1561`: pass `attacker.veterancy`.

**Out of scope for this design (deferred):** the actual `STRONGER`/`FIREPOWER`/`ROF`/`SIGHT`/`FASTER` multipliers from `VeteranAbilities` / `EliteAbilities`. Those affect damage parity but are a separate system. File as follow-up.

### B — Deploy duration from art-INI sequence Length

**New fields on the art-entry struct** in [`src/rules/art_data.rs`](../../src/rules/art_data.rs) (commit `1391629` already established this registry):

```rust
/// Sequence frame counts from artmd.ini. None when the sequence is undefined
/// for this image — falls back to `DEPLOY_DEFAULT_TICKS`.
pub deploy_frames: Option<u16>,
pub undeploy_frames: Option<u16>,
pub deployed_fire_frames: Option<u16>,
```

These come from the middle integer of `Deploy=300,15,0` etc. in the artmd.ini sequence section.

**Frame-to-tick conversion** in `compute_anim_ticks`:

```rust
/// Convert SHP frames to sim ticks. gamemd's animation engine runs at
/// roughly 80ms/frame on a 22ms sim tick → ratio ≈ 3.64.
///
/// For GGI Deploy=15 frames: 15 * 80 / 22 = 54.5 → 54 ticks (matches
/// existing 55 fallback within rounding).
fn frames_to_ticks(frames: u16) -> u16 {
    ((frames as u32) * 80 / 22) as u16
}
```

This is an approximation — gamemd advances animations on per-tick basis using a Rate field we don't fully model. Document this clearly; it's a known parity drift bounded by ±1 tick.

**`compute_anim_ticks` signature change:**

```rust
pub(crate) fn compute_anim_ticks(
    art: Option<&ArtEntry>,
    phase: DeployPhaseKind,
) -> u16 {
    let frames = art.and_then(|a| match phase {
        DeployPhaseKind::Deploying => a.deploy_frames,
        DeployPhaseKind::Undeploying => a.undeploy_frames,
    });
    frames.map(frames_to_ticks).unwrap_or(DEPLOY_DEFAULT_TICKS)
}
```

`DeployPhaseKind` is a small enum local to `deploy.rs` (Deploying / Undeploying — Deployed has no duration).

**Caller change** in `world_commands.rs:520,526`: look up the entity's art entry from the rules registry, pass it in.

### E — Homing missile module

**New file:** [`src/sim/movement/homing_movement.rs`](../../src/sim/movement/homing_movement.rs).

Sits alongside the existing ballistic [`rocket_movement.rs`](../../src/sim/movement/rocket_movement.rs); the latter is unchanged.

#### Components

```rust
//! Homing missile flight — per-tick yaw correction toward a tracked target.
//!
//! Used for projectiles with `Ranged=yes` (BulletType+0x2A0) such as
//! AAHeatSeeker2 (MissileLauncher's projectile). Distinct from rocket_movement.rs
//! which handles ballistic-arc projectiles (V3, dumb-fire).
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/entity_store, sim/game_entity, map/terrain.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::sim::entity_store::EntityStore;
use crate::util::fixed_math::{SimFixed, SIM_ONE, SIM_ZERO, /* ... */};

/// Phase within the homing missile state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HomingPhase {
    /// Arming: per-tick decrement until ready to detonate on impact.
    Arming,
    /// Cruise: tracking target with sidewinder yaw + cruise altitude control.
    Cruise,
    /// Stall failsafe: target unreachable, self-detonate next tick.
    SelfDestruct,
    /// Impact: caller despawns this tick.
    Detonation,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HomingState {
    pub phase: HomingPhase,

    // Target tracking
    pub target_id: Option<u64>,
    pub last_known_pos: (u16, u16),

    // Flight
    pub yaw_bam: u16,
    pub pitch_bam: u16,
    pub speed: SimFixed,
    pub altitude: SimFixed,
    pub vz: SimFixed,

    // Per-projectile parameters from BulletType / WeaponType / Rules
    pub rot_ini: u16,
    pub missile_rot_var: SimFixed,
    pub floater: bool,
    pub very_high: bool,
    pub arm_ticks_remaining: u16,

    // Sidewinder phase + stall detection
    pub frame_counter: u32,
    pub stall_counter: u8,
    pub stall_ema: SimFixed,
    pub last_distance_to_target: SimFixed,

    // Render-only
    pub pitch: f32,
}

pub fn attach_homing_state(
    entities: &mut EntityStore,
    bullet_id: u64,
    origin: (u16, u16),
    target_id: u64,
    weapon_speed: SimFixed,
    rot_ini: u16,
    arm_frames: u16,
    floater: bool,
    very_high: bool,
    missile_rot_var: SimFixed,
) -> bool;

pub fn tick_homing_movement(
    entities: &mut EntityStore,
    tick_ms: u32,
    sim_tick: u64,
) -> Vec<u64>;
```

#### Per-tick logic (in execution order, matching §8.1 + §9.4)

```
for each entity with HomingState:
    1. Refresh last_known_pos from target if alive (else keep last value).
    2. Compute desired_yaw = atan2_bam(target_pos - self_pos).
    3. sidewinder = cos(2π * (frame_counter % 15) / 15) * missile_rot_var
                  + missile_rot_var + 1.0
    4. delta_far = ftol(sidewinder * rot_ini)
    5. close_range = distance_to_target < 256 leptons (1 cell)
    6. delta = close_range ? ftol((frame_counter % 15) * 1.5) : delta_far
    7. rot_bam_per_tick = (delta as u8) << 8
    8. yaw_bam = step_toward_bam_inclusive(yaw_bam, desired_yaw, rot_bam_per_tick)
    9. Rebuild velocity (vx, vy) from yaw_bam + speed; preserve horizontal magnitude.
   10. pos += (ftol(vx), ftol(vy))     // truncation
   11. vz = if floater { vz } else { (vz + signum(vz) * 3) / 4 }
   12. Cruise altitude controller:
         cruise_alt_cells = if floater || very_high { 10 } else { min(target_alt / 256, 5) }
         dz = self_z - cruise_alt_cells * 64 - ground_height
         if |dz| > 20: snap z by ±18
         pitch_target = if dz < -32 { 0x2000 }
                        else if dz > 32 { 0x4800 }
                        else { 0x4000 }
         pitch_step = rot_bam_per_tick / 2
         pitch_bam = step_toward_bam_inclusive(pitch_bam, pitch_target, pitch_step)
   13. Apply pitch to velocity → updates vz.
   14. arm_ticks_remaining = arm_ticks_remaining.saturating_sub(1)
   15. Stall detect:
         dist_now = distance_to(target_or_last_known)
         delta_dist = last_distance_to_target - dist_now
         last_distance_to_target = dist_now
         if stall_counter < 60:
             stall_counter += 1
             stall_ema += delta_dist
         else:
             stall_ema = stall_ema * 0.9 + delta_dist * 0.1
             if stall_ema <= small_threshold && !floater:
                 phase = SelfDestruct
   16. if dist_now <= detonation_threshold && arm_ticks_remaining == 0:
            phase = Detonation
            detonated.push(id)
   17. frame_counter = frame_counter.wrapping_add(1)
```

#### Helper functions

```rust
/// Inclusive snap-to-target: returns true if cur can reach tgt this tick.
/// Matches gamemd Facing__IsWithinROT (uses <=).
fn within_rot_bam(cur: u16, tgt: u16, cap: u16) -> bool {
    let delta = (cur.wrapping_sub(tgt)) as i16;
    (delta.unsigned_abs() as u16) <= cap
}

/// Step current BAM angle toward target by at most cap; snap if within.
fn step_toward_bam_inclusive(cur: u16, tgt: u16, cap: u16) -> u16 {
    if within_rot_bam(cur, tgt, cap) {
        tgt
    } else {
        let delta = (tgt.wrapping_sub(cur)) as i16;
        // Pick shortest-arc direction
        if delta > 0 {
            cur.wrapping_add(cap)
        } else {
            cur.wrapping_sub(cap)
        }
    }
}

fn atan2_bam(dy: SimFixed, dx: SimFixed) -> u16 {
    // f32::atan2 is acceptable here because:
    //   - the result is immediately truncated to u16 BAM
    //   - we only use it to compare against current yaw via inclusive <=
    //   - the comparison is monotonic, so float jitter ≤ ±1 BAM is invisible
    let angle_rad = sim_to_f32(dy).atan2(sim_to_f32(dx));
    let bam_f = angle_rad * (32768.0 / std::f32::consts::PI);
    (bam_f as i32).rem_euclid(65536) as u16
}
```

#### Dispatch at projectile spawn

The spawn pipeline isn't fully traced (impact analysis flagged it as unverified). During implementation:

1. Locate where projectiles are spawned in response to a successful `select_weapon` + bullet allocation.
2. Branch on the projectile's `ranged` flag (BulletType `Ranged=yes` → `+0x2A0`):
   - `ranged=true` AND has `Image=` set → `attach_homing_state(target_id=target_entity_id, …)`
   - otherwise → existing `attach_rocket_state(target_pos, …)` for ballistic arcs
3. Pass through:
   - `rot_ini` from `BulletType.rot`
   - `arm_frames` from `BulletType.arm`
   - `floater` from `BulletType.floater`
   - `very_high` from `BulletType.very_high`
   - `missile_rot_var` from `RuleSet.general.missile_rot_var` (parse from `[General]MissileROTVar=`, default `SimFixed::from_num(1)`)
   - `weapon_speed` from `WeaponType.speed`

#### Tick-order integration

`World::advance_tick`, "air + special movement" phase: add `tick_homing_movement(…)` after `tick_rocket_movement(…)`. Both produce detonation lists handled by the same downstream damage dispatcher.

#### Determinism

- All sim-critical math uses SimFixed: `speed`, `altitude`, `vz`, `stall_ema`, `last_distance_to_target`.
- BAM angles are integer (`u16`) — wrap arithmetic is exact.
- Sidewinder cosine: the formula uses `cos(2π * (frame_counter % 15) / 15)` over 15 discrete values. Compute these into a `const SIDEWINDER_TABLE: [SimFixed; 15]` at compile time (or as a `OnceLock`) so the cosine never runs at runtime. **This is the single most important determinism step in the module.**
- `atan2_bam` uses f32 but its result is truncated and used only in a monotonic `<=` comparison — float jitter ≤ ±1 BAM cannot flip the comparison's outcome (cap is always ≥ 256 BAM). Document this contract; revisit if a stricter lockstep need surfaces.

#### Tiny-detail coverage (per the ledger)

- ✅ Sidewinder formula via `SIDEWINDER_TABLE`
- ✅ Close-range linear ramp at `< 256` leptons
- ✅ Inclusive `<=` snap via `within_rot_bam`
- ✅ `vz >>= 2` damper (Floater gates it)
- ✅ Cruise dead-band ±20 / snap ±18 / half-threshold ±32 leptons
- ✅ Stall-detect 60-frame window + EMA
- ✅ Target relocation each tick; fly-to-last-known on death
- ✅ Arm decrement (approximation: per-tick instead of AnimRate-tick — flagged)
- ✅ Truncation rounding via `as i32` casts (matches Math__ftol per §8.3)

## Tiny-Detail Ledger

(Carried forward from the brainstorm — see the §4.5 ledger above. Each item maps to either an implementation in this design or is explicitly flagged as approximation.)

| Detail | Source | Where it lives in this design |
|--------|--------|--------------------------------|
| DeploySound plays BEFORE state write | §3.1 | Gap F reorder in world_commands.rs |
| Deploy Length from art middle int | §3.1 + §9.5 | Gap B `art_data.deploy_frames` |
| End-of-sequence strict `<` test | §3.1 | already in `tick_deploy_state` (counter > 1 keeps going, ==1 promotes) |
| DeployedCrushable default TRUE | §9.5 | already done — `default_true` serde |
| Deploy is player-initiated (no auto-deploy) | §3.10 | Gap C verification |
| Fire-frame anchor strict `==` | §3.3 | already done (commit `1391629`) |
| SecondaryFire anchor when ProneFire defined | §9.5 | already done — verify against the recent combat/mod.rs |
| Damage order: falloff → zero-floor → Verses → MaxDamage | §3.6 | already done in combat_weapon.rs / combat/mod.rs |
| Truncation rounding (Rust `as i32` matches) | §8.3 | already done implicitly |
| No ProneDamage multiplier | §9.1 | already done — verify no later regression |
| Sidewinder ROT formula | §9.4 | Gap E `SIDEWINDER_TABLE` |
| Close-range linear ramp | §9.4 | Gap E `close_range` branch |
| Inclusive `<=` IsWithinROT | §8.1 | Gap E `within_rot_bam` |
| `vz >>= 2` damper | §8.1 | Gap E step 11 |
| Cruise dead-band ±20 / snap ±18 / half ±32 | §8.1 | Gap E step 12 constants |
| Stall-detect 60-frame + EMA | §8.1 | Gap E step 15 |
| Target relocation per tick; last-known on death | §8.1 | Gap E step 1 |
| Veteran/Elite swap threshold 200 | §4.1 (matches existing garrison) | Gap D `is_elite = veterancy >= 200` |
| OpenTransportWeapon default -1 | §4.1 | Gap G `default_neg_one()` |
| IFV vs BFRT routing distinction | §3.7 | Gap G `WeaponOverride` enum |

**Known approximations explicitly accepted in this design:**

- Frames-to-ticks conversion uses `frames * 80 / 22` — ≤1-tick drift per deploy. Acceptable because gamemd's animation rate isn't fully modeled and the visual deploy duration is dominated by SHP framerate, not sim tick rate.
- Arm decrement is per-tick rather than gamemd's AnimRate-tick — for AAHeatSeeker2 (`Arm=2`) and Rate=1, identical; for projectiles with non-1 Rate, ≤2-tick arm-window drift. Acceptable until a non-Rate=1 projectile surfaces.
- `atan2_bam` uses f32 internally — float jitter is bounded ≤ ±1 BAM, never flips the `<=` comparison outcome. Replace with a SimFixed BAM table only if lockstep desync surfaces (low risk).

## Design

### Components

- **`rules/object_type.rs`** — owns parse of `ElitePrimary=`, `EliteSecondary=`, `OpenTransportWeapon=`.
- **`rules/art_data.rs`** — owns parse of sequence Length fields.
- **`sim/combat/combat_weapon.rs`** — owns `WeaponOverride` enum and the veterancy-aware selection logic.
- **`sim/movement/homing_movement.rs`** — owns the full homing flight loop and `SIDEWINDER_TABLE`.
- **`sim/deploy.rs`** — owns `compute_anim_ticks(art, phase)`.
- **`sim/world/world_commands.rs`** — owns the sound-emit ordering.
- **`sim/passenger.rs`** — owns the `WeaponOverride` selection at transport-load time.

### Interfaces / Contracts

- `select_weapon*` API gains a `veterancy: u16` parameter and replaces `Option<u32>` with `Option<WeaponOverride>`. All call sites updated atomically.
- `compute_anim_ticks` API gains `art: Option<&ArtEntry>` and `phase: DeployPhaseKind`. Call sites updated atomically.
- `HomingState` is a new struct on `GameEntity` — additive (does not conflict with `RocketState`; a given entity has at most one of the two).
- `WeaponOverride` replaces `ifv_weapon_index` on `GameEntity` (rename + type swap). Serde compatibility: write a migration shim if save-files exist for this field; otherwise plain rename.

### Data Flow

**Deploy command path:**

```
player presses D
  → world_commands::handle_deploy_command(entity_id)
  → look up entity.object_type + entity.art_alias
  → ticks = compute_anim_ticks(rules.art(alias), DeployPhaseKind::Deploying)
  → emit SimSoundEvent::EntityDeployed(deploy_sound)   ← BEFORE state write
  → entity.deploy_state = Some(DeployPhase::Deploying { ticks_remaining: ticks })
```

**Veterancy-aware weapon selection:**

```
combat tick (passive acquire OR active fire)
  → snapshot attacker.veterancy
  → snapshot attacker.weapon_override (set at transport-load time)
  → select_weapon_with_override(rules, obj, target_cat, armor, veterancy, override_)
    ├── if Override::IfvSlot(idx) → transport.weapon_list[idx]
    ├── if Override::OpenTransport(0|1) → passenger's primary_for_tier / secondary_for_tier
    └── else → primary_for_tier → secondary_for_tier
```

**Homing missile flight:**

```
fire path produces a bullet
  → if projectile.ranged → attach_homing_state(target_id = target_entity, …)
  → else → attach_rocket_state(target_pos, …)

each tick (world.advance_tick, "air + special movement" phase):
  → detonated = tick_rocket_movement(…)
  → detonated.extend(tick_homing_movement(…))
  → for each id in detonated: dispatch damage + despawn
```

### Error Handling

- INI parse failures for new fields default to the documented gamemd defaults (`-1` for OpenTransportWeapon, `None` for elite weapons, `None` for art-sequence frames). No errors propagated; missing keys are normal.
- `select_weapon_with_override` returns `None` if no path produces a valid weapon — caller already handles this as "cannot engage."
- `attach_homing_state` returns `false` if entity is missing (matches `attach_rocket_state` contract).
- Stall-detect self-destruct is informational; treated identically to a normal detonation by the despawn dispatcher.

### Testing Strategy

Each gap gets unit + integration tests in the existing pattern:

- **Gap F** — extend `deploy_tests.rs` to assert sound emit precedes state mutation. Existing tests verify presence; new tests verify ordering by inspecting the (entity_state, events_emitted) tuple at the point of emit.
- **Gap C** — no code change → no new tests. Document the inspection result in the implementation PR description.
- **Gap G** — `combat_weapon_tests.rs`: GGI inside BFRT (no Gunner) fires MissileLauncher; GGI inside IFV (Gunner) fires CRMissileLauncher (IFV.Weapon17). `OpenTransportWeapon=-1` falls through to base primary/secondary.
- **Gap D** — `combat_weapon_tests.rs`: Rookie GGI uses M60+MissileLauncher; Veteran GGI uses same; Elite GGI uses M60E+MissileLauncherE. Threshold tests at 199 (still Veteran) and 200 (Elite).
- **Gap B** — `deploy_tests.rs`: with art entry providing `deploy_frames=15`, `compute_anim_ticks` returns 54 (=15*80/22). Without art entry, returns 55 (fallback).
- **Gap E** — new `homing_movement_tests.rs`:
  - Smoke: missile launches, tracks moving target, detonates within N ticks.
  - Sidewinder: per-tick BAM step varies over the 15-frame phase.
  - Close range: ROT formula switches inside 1 cell.
  - Target death: missile continues to last-known position.
  - Stall detect: stationary target out of reach → self-destruct after the EMA threshold.
  - Inclusive snap: at exact ROT boundary, `step_toward_bam_inclusive` snaps to target.
  - vz damper: with Floater=false, vz halves repeatedly; with Floater=true, it doesn't.
  - Cruise dead-band: dz=±20 → no clamp; dz=±21 → snap ±18.

### Determinism considerations

- `tick_homing_movement` is deterministic given identical entity state and identical Rules. SimFixed math + integer BAM + precomputed `SIDEWINDER_TABLE`.
- `atan2_bam` uses f32; its result is truncated and only used in `<=` comparisons against integer caps. Bounded jitter cannot flip the comparison; lockstep safe.
- New `HomingState` fields participate in `world_hash`. Schema version bumped (existing pattern from other state additions — e.g. when `deployed_crushable` was added).

## Architectural Decisions

| Decision | Rationale |
|----------|-----------|
| **Parallel homing module** (not generalize rocket_movement) | User-chosen. Keeps V3 ballistic code untouched. Two structs share shape but their tick logic differs enough that merging would obscure both. |
| **`WeaponOverride` enum replaces `Option<u32>`** | The single-Option type collapsed two incompatible override semantics (IFV slot vs passenger primary/secondary). A typed enum makes the routing explicit and caller-readable. |
| **Veteran tier does NOT swap weapons** | gamemd parity: VeteranAbilities only applies multipliers, not weapon swaps. Confirmed by reading the existing `select_garrison_weapon` model. |
| **Frames-to-ticks approximation accepted** | Gamemd's animation Rate field isn't fully modeled; the 80/22 ratio matches the empirical deploy timing within ±1 tick. Revisit when SHP animation rates are wired end-to-end. |
| **`atan2_bam` uses f32** | The `<=` comparison is monotonic and cap ≥ 256 BAM ≫ float jitter. Replace with SimFixed table only if lockstep desync surfaces. |
| **`SIDEWINDER_TABLE` precomputed** | Single most important determinism step. 15-entry table replaces runtime cosine entirely. |

**No new patterns introduced** — every change follows an existing model:

- Veteran weapon swap mirrors `select_garrison_weapon`.
- Art sequence Length mirrors the recent `art_data.rs` registry from commit `1391629`.
- `WeaponOverride` enum mirrors other enum-based dispatch in `sim/combat/`.
- `homing_movement.rs` mirrors `rocket_movement.rs` structure.

**No tech debt introduced.** Approximations are documented and bounded.

## Alternatives Considered

- **Generalize rocket_movement into projectile_movement** — rejected by user choice; user prefers V3 ballistic code untouched. Architecturally cleaner but bigger blast radius.
- **Extend RocketState with optional homing fields** — rejected: mixes concerns; bloats RocketState for non-homing projectiles; less type-safe than two distinct structs.
- **Use a SimFixed BAM-lookup sin/cos table for `atan2_bam`** — deferred; current f32-with-truncation contract is safe for the monotonic comparison and avoids 65K-entry table allocation. Promote to lookup table only if lockstep issue surfaces.
- **Apply ProneDamage multiplier "for completeness"** — rejected per §9.1 (dead data in YR; would introduce a 30–50% damage drift). Documented prominently.
- **Auto-deploy GGI on air target acquisition** — rejected per §3.10 (gamemd does not do this; the player must deploy explicitly).
- **Single design pass for veteran-ability multipliers (STRONGER/FIREPOWER/ROF/SIGHT/FASTER)** — deferred. Out of GGI scope; a separate system that affects many units. Will be a follow-up brainstorm.

---

## Hand-off

Approved. Natural next step: `/write-plan 2026-05-17-ggi-rust-integration-design` to break this into bite-sized implementation tasks in dependency order (F → C → G → D → B → E). Each gap is independently testable; E is the only one large enough to warrant phasing within itself.
