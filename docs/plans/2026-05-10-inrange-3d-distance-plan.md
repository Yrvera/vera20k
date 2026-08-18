# InRange 3D Distance — Stage 1 Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Replace the 2D `lepton_distance_sq_raw` + `is_within_range_leptons` pair at the four targeting/cursor sites with a single `compute_in_range` function that mirrors gamemd `TechnoClass::InRange (0x006F7220)` for the distance-side parity ledger: 3D Euclidean distance, IsLowFlying ground-snap, AirRange bonus, arcing-weapon early gate, foundation bonus, bridge LOS gate, and verified boundary semantics.

**Architecture:** New `src/sim/combat/in_range.rs` houses `compute_in_range` and a small set of private helpers. The existing 2D `lepton_distance_sq_raw` survives untouched (AOE keeps using it). Four call sites — three combat, one cursor — converge on the new function. Stages 2-N add the remaining range-VALUE chain (Bunker / OpenTopped / Veteran) as match arms inside one helper, with no further call site rewrites.

**Design Doc:** [docs/plans/2026-05-10-inrange-3d-distance-design.md](2026-05-10-inrange-3d-distance-design.md)

---

## Grounding Summary

**ra2-rust-game-docs/ (R1):** [TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md) is the primary source. Includes the §0 corrections (2026-05-07) clarifying that:
- `weapon.Projectile.Arcing=yes` (not target.Type) gates Branch B
- `weapon.Projectile.SubjectToElevation=yes` gates height-fire bonus
- WhatAmI()==3 is provably dead in YR (only TypeClass templates inherit)
- Bridge Z gate is LOS occlusion (active every match with bridges)

[BUNKER_SYSTEM_GHIDRA_REPORT.md §5](../../../ra2-rust-game-docs/BUNKER_SYSTEM_GHIDRA_REPORT.md) covers the range-VALUE chain (Bunker / OpenTopped / Garrison REPLACES — Stage 2 work, referenced here for completeness).

[COORDINATE_SYSTEM_GAMEMD.md:127-131](../../../ra2-rust-game-docs/COORDINATE_SYSTEM_GAMEMD.md) verifies `LevelHeight = HeightFactor = 104` at gamemd `0x89DDB8`.

**Ghidra verification (R2):** All decompilations referenced in the research doc were re-verified during the brainstorm and `/review-plan` passes — addresses `0x006F7220` (InRange), `0x005F65A0` (ObjectClass::GetCoords), `0x005F6B60`/`0x005F6B90` (IsLowFlying / IsHighFlying), `0x006F6F60` (height-fire helper), and the WhatAmI returns for Unit (1), Aircraft (2), Building (6), Infantry (0xF). **`FUN_006F6F60` gate is verified** as `attacker.IsLowFlying() && target.IsLowFlying()` (disasm 0x006F6F69-0x006F6F81: two `JZ exit` after IsLowFlying calls). The corrections doc's "high-ground bonus for ground units" interpretation was wrong — this helper is a low-flying-vs-low-flying height-difference bonus, **not** a high-ground tank bonus. Stage 1 stub-returns-0 is therefore correct for 99%+ of engagements.

**Repo pattern (R3):** Combat module structure follows `src/sim/combat/{combat_targeting,combat_aoe,combat_weapon,combat_fire_gate}.rs` — each subsystem in its own file, `pub(crate)` exports, re-exported from `combat/mod.rs`. New `in_range.rs` follows this exact pattern. Existing helpers `pursuit_weapon_range` (`combat/mod.rs:279`) and `resolve_target_coords` (`combat/mod.rs:255`) — both added 2026-05-08 — provide reusable patterns for weapon-selection and target-coord resolution; **`compute_in_range` is a separate concern** (range checking) and does not subsume them. **Existing `TargetKind` enum at `combat/mod.rs:144`** with `Entity(u64)` and `Cell(u16, u16)` variants is reused — the design doc's proposed `RangeTarget` is dropped.

**INI keys (R4):**
- [`[ElevationModel]`](../../ini/rules.ini#L757) `ElevationIncrement=4`, `ElevationIncrementBonus=2` — referenced for height-fire bonus (Stage 1 stub; activation deferred per open question).
- `AirRangeBonus=` on TechnoTypeClasses (rules.ini:6690) — **NOT yet parsed**; Task 1 adds it.
- `MinimumRange=` on weapons — already parsed as `Weapon.minimum_range: SimFixed`.
- `Arcing=`, `SubjectToElevation=` on projectiles — already parsed on `ProjectileType`.
- `[General] FlightLevel=1500` — already parsed as `GeneralRules.flight_level: i32`. Used to derive the Stage 1 placeholder for `HIGH_FLIGHT_THRESHOLD_LEPTONS`.

**Existing infrastructure to reuse:**
- `isqrt_i64` at `src/util/fixed_math.rs:246` (already used by AOE and movement code).
- `SimFixed` fixed-point math throughout sim.
- `ResolvedTerrainGrid` at `src/map/resolved_terrain.rs:151` — has `cells: Vec<ResolvedTerrainCell>` (line 154), `width()/height()` accessors (lines 166/170), `build_height_map()` and `build_bridge_height_map()` for cell-elevation lookups (lines 903/913). Note: the type is `ResolvedTerrainGrid`, not `ResolvedTerrain` — already used elsewhere as `terrain: &ResolvedTerrainGrid` (e.g., `src/sim/bridge_state/walker.rs:30`).
- `ResolvedTerrainCell` fields: `pub level: u8` (line 75 — cell elevation level), `pub has_bridge_deck: bool` (line 121), `pub bridge_deck_level: u8` (line 124). **Note:** the field is `level`, not `height`.
- `crate::sim::production::foundation_dimensions` for Building-target foundation lookup (already public, used cross-module in 8+ files).
- `GameEntity` lives in `crate::sim::game_entity` (line 88 has `pub locomotor: Option<LocomotorState>`); `EntityCategory` lives in `crate::map::entities`.
- `LocomotorState.altitude: SimFixed` at `src/sim/movement/locomotor.rs:140`.
- `SNAPSHOT_VERSION = 5` constant at `src/sim/snapshot.rs:16` — Stage 1 bumps to 6.

**Git re-verify:** Recent combat/ commits (2026-05-08 → 2026-05-09) added `TargetKind`, `resolve_target_coords`, `pursuit_weapon_range`, and cell-target attack support. None conflict with this plan; the new code uses these as building blocks.

**Still unknown after grounding:**
- Exact runtime values of `DAT_00AC13C8` (HighFlightLevel) and `DAT_00B0EB24` (BridgeHeightDelta) — Stage 1 ships placeholders (1000 and 416 leptons respectively) per design doc OQ-5.
- `FUN_006F6F60` formula constant `DAT_00B0EB34` (ballistic-term scale) — needed for Stage 2+ height-fire activation, not Stage 1. The full formula is `sqrt((levels × 256)² + (DAT_00B0EB34 × dh)²)` where `levels = dh / Rules.ElevationIncrement`. Stage 1 stubs the function.

## Key Technical Decisions

- **Reuse existing `TargetKind` enum** (added 2026-05-08) instead of creating a new `RangeTarget` enum from the design doc — **Confidence:** high
  - **Source:** repo pattern src/sim/combat/mod.rs:144

- **Integer sqrt comparison via `isqrt_i64` instead of squared-leptons comparison** — deliberate parity tightening: matches gamemd's `(int)Sqrt_Approx(...) <= range` semantics within ±1 lepton; squared comparison can diverge by 1 lepton at boundaries — **Confidence:** high
  - **Source:** Ghidra disasm 0x006F75F2 + COORDINATE_SYSTEM_GAMEMD.md sqrt notes

- **Position struct unchanged** (`pos.z: u8` + `loco.altitude: SimFixed` stays separate) — `effective_z_leptons` helper computes per call. Avoids large refactor — **Confidence:** high
  - **Source:** design doc § Architectural Decisions

- **Height-fire bonus shipped as stub returning 0** with framework in place. Gate verified during `/review-plan`: `attacker.IsLowFlying() && target.IsLowFlying()`. The bonus only fires when both attacker AND target are low-flying aircraft AND weapon's projectile has `SubjectToElevation=yes` — a rare combination in standard YR play. Activating it later just means filling in the formula; gate is known. — **Confidence:** high (gate); high (rare-case decision)
  - **Source:** Ghidra disasm 0x006F6F69-0x006F6F81 (verified `JZ exit` after both IsLowFlying calls)

- **`HIGH_FLIGHT_THRESHOLD_LEPTONS = 1000` placeholder** — anchored between low-cruise (≈500 lep) and high-cruise (≈1500 lep) per air_movement.rs constants — **Confidence:** medium
  - **Source:** inferred from src/sim/movement/air_movement.rs:675 (1500) and 706 (500); awaits resolution of OQ-5
  - **Flag for /review-plan:** confirm splitting at 1000 lep correctly classifies real YR aircraft (Harrier, Black Eagle, Rocketeer, Kirov, paratroopers).

- **`BRIDGE_HEIGHT_DELTA_LEPTONS = 416`** = `4 × LEPTONS_PER_LEVEL` (matches `BridgeHeight=4` Rules default) — **Confidence:** medium
  - **Source:** rules.ini default + LEPTONS_PER_LEVEL=104; awaits runtime confirmation

- **Bridge LOS gate reads `ResolvedTerrainCell.has_bridge_deck` and `level` directly via `terrain.cells[ry × width + rx]`** — Stage 1 doesn't need the pre-built `bridge_height_map`; per-call field access is simpler and the map is computed at terrain-resolve time anyway — **Confidence:** high
  - **Source:** repo pattern src/sim/bridge_state/walker.rs:30 uses `terrain: &ResolvedTerrainGrid` directly; `ResolvedTerrainCell` fields verified at resolved_terrain.rs:75, 121, 124

## Open Questions

### Resolved During Planning / Review

- **Should we reuse `TargetKind` or create `RangeTarget`?** → Reuse `TargetKind` (already exists with the right shape).
- **Where does `isqrt_i64` live?** → `src/util/fixed_math.rs:246`, already imported by combat code.
- **Is `MinimumRange` parsed?** → Yes, as `Weapon.minimum_range: SimFixed`.
- **Is `air_range_bonus` parsed?** → No (Task 1 adds it).
- **What's the actual cell-elevation accessor?** → `ResolvedTerrainGrid.cells[idx]` where `idx = ry * width + rx`. Field is `cell.level: u8` (NOT `height`).
- **Where does `GameEntity` live?** → `crate::sim::game_entity` (NOT `crate::map::entities`); `EntityCategory` IS in `crate::map::entities`. Split imports accordingly.
- **Where does `foundation_dimensions` live?** → `crate::sim::production::foundation_dimensions` (NOT `crate::sim::combat::`). Already public; no visibility upgrade needed.
- **What's the `weapon.projectile` accessor?** → `Weapon.projectile: Option<String>` is the field. Resolve via `rules.projectile(name)` (verify exact `RuleSet` accessor) — there is no `weapon.projectile_ref()` shorthand.
- **`FUN_006F6F60` gate semantics?** → Verified `attacker.IsLowFlying() && target.IsLowFlying()` (disasm 0x006F6F69-0x006F6F81, two `JZ exit` after IsLowFlying calls). Stub-returns-0 is correct for 99%+ of engagements; bonus only fires for low-flying-vs-low-flying weapons with `SubjectToElevation=yes`. Stage 2+ activation just means implementing the formula `sqrt((levels × 256)² + (DAT_00B0EB34 × dh)²)`.

### Deferred to Implementation

- **Exact `HIGH_FLIGHT_THRESHOLD_LEPTONS` value** — placeholder 1000 lep; refine when DAT_00AC13C8 traced from gamemd at runtime or from rules.ini parser.
- **Exact `BRIDGE_HEIGHT_DELTA_LEPTONS`** — placeholder 416 lep; same.
- **`DAT_00B0EB34` ballistic-term scale** — needed for Stage 2+ height-fire activation, not Stage 1.
- **`weapon+0xA0` field identity** — disasm shows `[weapon+0xA0].byte+0x297` gates the height-fire helper call. Whether `weapon+0xA0` is `Projectile*` or `WarheadType*` matters for Stage 2 activation; doesn't affect Stage 1 stub.
- **Cursor site `&ResolvedTerrainGrid` access** — Task 10 verifies the cursor scope can reach terrain (likely via `&Simulation` accessor); if not, may require a small accessor addition.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/rules/object_type.rs` | Add `air_range_bonus: Option<SimFixed>` field + parser |
| Modify | `src/util/lepton.rs` | Add 4 constants for InRange |
| Create | `src/sim/combat/in_range.rs` | `compute_in_range` + private helpers |
| Modify | `src/sim/combat/mod.rs` | `pub mod in_range;` + re-exports |
| Modify | `src/sim/combat/combat_targeting.rs:193` | Call site migration |
| Modify | `src/sim/combat/mod.rs:1049, :1381` | Two call site migrations |
| Modify | `src/app_cursor.rs:346` | Cursor call site migration |
| Modify | `src/sim/snapshot.rs:16` | Bump `SNAPSHOT_VERSION` 5 → 6 |

## Interface Changes

- **`ObjectType.air_range_bonus: Option<SimFixed>`** — new public field. Read-only, defaults to `None`. No existing code reads it before Task 4 wires it through `compute_in_range`.
- **`compute_in_range(...) -> bool`** — new `pub(crate)` API in `combat/in_range.rs`, re-exported from `combat/mod.rs`. Consumers: 4 call sites listed above.
- **`SNAPSHOT_VERSION` 5 → 6** — incompatible save/replay change. Old saves cannot be loaded after this PR. Documented in commit message.

## Sim Checklist

- [x] All math uses `fixed`-point — no f32/f64 in game logic. `compute_in_range` uses `i64` and `SimFixed` only.
- [x] New state included in deterministic state hash. Targeting reads now include `loco.altitude` (SimFixed); state hash already includes targeting decisions.
- [x] No dependencies on render/ui/sidebar/audio/net. Verify by grep after Task 5: `cd src/sim && grep -rn "use crate::\(render\|ui\|sidebar\|audio\|net\)" combat/in_range.rs` — must return empty.
- [x] Tick ordering impact: none. The function is a query, not a tick step.
- [x] BTreeMap iteration order: unaffected. EntityStore iteration is unchanged; lookups by stable_id are deterministic.

## Risk Areas

From design doc Impact Analysis:

- **Determinism (replay break)** — accepted, Task 11 bumps SNAPSHOT_VERSION. Task 12 verifies determinism preserved across two runs of the same seed.
- **Sqrt approximation drift** — accepted parity tightening; ±1 lepton possible at extreme distances.
- **Height-fire stub** — explicit known parity gap, narrowly scoped. Per gate verification, the bonus only fires for low-flying-vs-low-flying engagements with `SubjectToElevation=yes` weapons (rare). Stage 1 ships stub-returns-0; same as current 2D behavior — no regression. Stage 2+ activation just fills in the formula.
- **`HIGH_FLIGHT_THRESHOLD_LEPTONS` placeholder** — wrong value would mis-classify aircraft (LowFlying vs HighFlying). Mitigation: Task 13 manual verification with multiple aircraft types.
- **Cell-target paths** — `TargetKind::Cell(rx, ry)` lacks z_level. Cell targets in the new function read ground Z from `terrain.cells[ry × width + rx].level * LEPTONS_PER_LEVEL`. Verify in Task 5 the lookup is plumbed.
- **Terrain parameter threading** — `acquire_best_target` (combat_targeting.rs:128) and `acquire_best_target_for_entity` (line 69) do NOT currently take `&ResolvedTerrainGrid`. Task 6.5 threads it through their signatures and updates callers BEFORE the call-site migrations begin. Underestimating this in original draft caused the call-site tasks to look simpler than they are.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 5 | 3D distance for ground-vs-air | Aircraft at altitude must require longer range to hit, not the same range as ground units. Visible every air-vs-ground engagement. | Unit test (Task 6 #4); manual verify Task 13 (Kirov vs SAM at varying altitudes). |
| Task 5 | LowFlying ground-snap | Harrier descending to attack must remain shootable at horizontal range, not escape due to altitude penalty. Visible every Harrier strike. | Unit test (Task 6 #5); manual verify Task 13 (Harrier strike). |
| Task 5 | Arcing-weapon 2D fallthrough | V3 / Prism / Dreadnought / Apocalypse Rocket parity must not regress; arcing weapons should behave EXACTLY as today (no Z penalty). Visible every long-range artillery shot. | Unit test (Task 6 #10); manual verify Task 13 (V3 Rocket fire across cliff). |
| Task 5 | Foundation bonus on building targets | Range to attack a 4×2 ConYard differs from range to attack a 2×2 Power Plant. Visible every building shot. | Unit test (Task 6 #7); manual verify Task 13 (compare engage range on different building sizes). |
| Task 5 | Bridge LOS gate | Tank under bridge cannot fire up at infantry on deck. Visible every match with bridges and units routing under them. | Unit test (Task 6 #11); manual verify Task 13 (under-bridge scenario). |
| Task 5 | Boundary inclusive max + strict min | Off-by-one at exact range = unit fires when it shouldn't or doesn't fire when it should. Subtle but visible at range edges. | Unit tests (Task 6 #2, #3). |
| Task 5 | Sentinel `weapon.range == -512 lep` returns true | Weapons with always-in-range sentinel (e.g., some specials) must always pass. Visible if any such weapon exists in YR. | Unit test (Task 6 #1). |

---

## Tasks

### Task 1: Add `air_range_bonus` to ObjectType + parser

**Why:** `compute_in_range` (Task 5) needs `attacker.type.air_range_bonus` for the `AirRange` bonus when target is high-flying. INI key exists but isn't parsed.

**Files:**
- Modify: `src/rules/object_type.rs` (struct field + parser)

**Pattern:** Mirror existing `guard_range: Option<SimFixed>` at field declaration (line 259) and parser (line 776).

**Step 1: Add struct field**

Insert in `ObjectType` struct after `guard_range`:

```rust
    /// Range bonus added to the weapon's max range when firing at airborne
    /// (high-flying) targets. Read from `AirRangeBonus=` in [TechnoTypeClass]
    /// section. None means no bonus.
    ///
    /// In gamemd this is `TechnoTypeClass+0x68C`. Stored in **leptons** (not
    /// cells) per gamemd convention — added directly to the leptons-space
    /// effective range without ×256 scaling.
    pub air_range_bonus: Option<SimFixed>,
```

**Step 2: Add parser**

Insert in the `from_ini`-style parser after `guard_range:` line 776:

```rust
            air_range_bonus: section.get_f32("AirRangeBonus").map(sim_from_f32),
```

**Step 3: Add unit test**

Add to the existing test module in `object_type.rs`:

```rust
    #[test]
    fn parses_air_range_bonus() {
        let ini = IniFile::from_str("[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nAirRangeBonus=4\n");
        let section = ini.section("MTNK").expect("section");
        let obj = ObjectType::from_ini_section("MTNK", section).expect("parse");
        assert_eq!(obj.air_range_bonus, Some(crate::util::fixed_math::sim_from_f32(4.0)));
    }

    #[test]
    fn air_range_bonus_default_none() {
        let ini = IniFile::from_str("[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\n");
        let section = ini.section("MTNK").expect("section");
        let obj = ObjectType::from_ini_section("MTNK", section).expect("parse");
        assert_eq!(obj.air_range_bonus, None);
    }
```

**Step 4: Verify**

Run: `cargo test -p ra2-rust-game --lib object_type::tests::parses_air_range_bonus air_range_bonus_default_none`
Expected: 2 passed.

**Step 5: Commit**

Commit message: `rules: add ObjectType.air_range_bonus field for InRange Stage 1`

---

### Task 2: Add InRange constants to `src/util/lepton.rs`

**Why:** `compute_in_range` needs gameplay-grade Z constants (separate from rendering's HEIGHT_STEP=15.0). One file, four constants, no dependencies.

**Files:**
- Modify: `src/util/lepton.rs` (append constants)

**Pattern:** Follows existing constant declarations in `util/lepton.rs` and `util/fixed_math.rs` (top-of-module `pub const X: T = ...`).

**Step 1: Append constants**

At the end of `src/util/lepton.rs` (or near other lepton constants if grouped):

```rust
// ─── InRange (3D distance) constants ────────────────────────────────────

/// Leptons per cell-elevation level. The gameplay-grade Z conversion factor.
///
/// Verified at gamemd `0x89DDB8` (= LevelHeight = HeightFactor = cot(60°) ×
/// 256√2 × 0.5 ≈ 104). Distinct from rendering's `HEIGHT_STEP = 15.0` pixels —
/// THIS is the value combat uses for 3D distance.
///
/// See ra2-rust-game-docs/COORDINATE_SYSTEM_GAMEMD.md:127-131.
pub const LEPTONS_PER_LEVEL: i64 = 104;

/// Sentinel weapon range meaning "always in range". gamemd checks
/// `weapon->Range == -0x200` (= -512 leptons) before any other gate; if matched,
/// returns true unconditionally. Used by some special weapons.
pub const WEAPON_RANGE_ALWAYS_IN_RANGE_LEPTONS: i64 = -512;

/// Lepton threshold dividing low-flying from high-flying aircraft for InRange
/// gating. gamemd uses `HighFlightLevel × 2` (DAT_00AC13C8 × 2). Exact value
/// pending OQ-5 in TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md.
///
/// Initial value chosen so that cruise altitude (≈1500 lep, per
/// air_movement.rs:675) classifies as high-flying and dive altitude (≈500 lep,
/// per air_movement.rs:706) classifies as low-flying.
pub const HIGH_FLIGHT_THRESHOLD_LEPTONS: i64 = 1000;

/// Z bump in leptons added to a cell's ground height when a bridge deck is
/// present on the cell. gamemd uses DAT_00B0EB24 (runtime-init); placeholder
/// = 4 × LEPTONS_PER_LEVEL = 416 lep, matching the Rules.ini `BridgeHeight=4`
/// default.
///
/// See OQ-5 in TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md for refinement.
pub const BRIDGE_HEIGHT_DELTA_LEPTONS: i64 = 416;
```

**Step 2: Verify compilation**

Run: `cargo check -p ra2-rust-game --lib`
Expected: PASS, no warnings about unused constants (they'll be used in Task 3+).

If "unused" warnings appear, that's OK at this stage — Task 3 imports them.

**Step 3: Commit**

Commit message: `util/lepton: add LEPTONS_PER_LEVEL + InRange Stage 1 constants`

---

### Task 3: Create `in_range.rs` with helper functions

**Why:** Set up the module skeleton with private helpers (`effective_z_leptons`, `is_low_flying`, `is_high_flying`, `ground_z_with_bridge_offset`) that the main `compute_in_range` (Task 5) and bonus helper (Task 4) will use. Done first because everything else depends on these.

**Files:**
- Create: `src/sim/combat/in_range.rs`
- Modify: `src/sim/combat/mod.rs` (add `pub mod in_range;` line)

**Pattern:** Module structure mirrors existing `src/sim/combat/combat_targeting.rs` and `combat_aoe.rs` — `//!` doc comment header, `use` block, `pub(crate)` exports.

**Step 1: Create `src/sim/combat/in_range.rs`**

```rust
//! 3D weapon range check matching gamemd.exe TechnoClass::InRange (0x006F7220).
//!
//! Replaces the 2D `lepton_distance_sq_raw` + `is_within_range_leptons` pair
//! at the four targeting/cursor sites. Stage 1 implements 3D distance,
//! IsLowFlying ground-snap, AirRange bonus, arcing-weapon 2D fallthrough,
//! foundation bonus, bridge LOS gate, and the verified boundary semantics
//! (<= max inclusive, < min strict, -512 lep sentinel).
//!
//! Stages 2-N add the remaining range-VALUE chain (Bunker / OpenTopped /
//! Veteran). Stage Arcing adds the full Branch B slope-arc check.
//!
//! See ra2-rust-game-docs/TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md for
//! the parity ledger and verified Ghidra evidence.
//!
//! Depends on: rules (ObjectType, Weapon, ProjectileType), map (terrain
//! height + bridge), util/lepton (constants), util/fixed_math (isqrt_i64).
//! Does NOT depend on render/ui/sidebar/audio/net.

use crate::map::entities::EntityCategory;
use crate::sim::game_entity::GameEntity;
use crate::util::lepton::{HIGH_FLIGHT_THRESHOLD_LEPTONS, LEPTONS_PER_LEVEL};

/// Combined absolute Z of an entity in leptons (cell elevation × 104 +
/// locomotor altitude for airborne entities). Matches gamemd's
/// `ObjectClass+0xA4` (Coords.Z) — a single absolute-leptons value rather
/// than separate level + altitude fields.
///
/// Droppod and parachute altitudes are intentionally NOT added — those
/// entities are always IsLowFlying-equivalent during descent and get
/// ground-snapped by the InRange caller.
pub(crate) fn effective_z_leptons(entity: &GameEntity) -> i64 {
    let base = entity.position.z as i64 * LEPTONS_PER_LEVEL;
    match entity.locomotor.as_ref() {
        Some(loco) => base + loco.altitude.to_num::<i64>(),
        None => base,
    }
}

/// Entity is currently airborne and below the high-flight threshold.
/// gamemd `ObjectClass::IsLowFlying` (0x005F6B60) — gate by airborne flag
/// (byte@0x74 != 0) AND `Get_Height() < HighFlightLevel × 2`.
///
/// Used by the InRange caller to decide whether to ground-snap the target's
/// Z before the distance computation (low-flying targets are ranged at the
/// ground beneath them, not at their actual altitude).
pub(crate) fn is_low_flying(entity: &GameEntity) -> bool {
    if entity.category != EntityCategory::Aircraft {
        return false;
    }
    let alt = entity
        .locomotor
        .as_ref()
        .map(|l| l.altitude.to_num::<i64>())
        .unwrap_or(0);
    alt > 0 && alt < HIGH_FLIGHT_THRESHOLD_LEPTONS
}

/// Entity is currently airborne and at or above the high-flight threshold.
/// gamemd `ObjectClass::IsHighFlying` (0x005F6B90). Mutually exclusive with
/// `is_low_flying` for airborne units.
///
/// Used by the InRange caller to enable the AirRange bonus on the attacker's
/// weapon when the target is high-flying.
pub(crate) fn is_high_flying(entity: &GameEntity) -> bool {
    if entity.category != EntityCategory::Aircraft {
        return false;
    }
    let alt = entity
        .locomotor
        .as_ref()
        .map(|l| l.altitude.to_num::<i64>())
        .unwrap_or(0);
    alt >= HIGH_FLIGHT_THRESHOLD_LEPTONS
}
```

**Step 2: Hook up the module**

In `src/sim/combat/mod.rs`, near the other `mod` declarations (find an existing `mod combat_targeting;` or similar line), insert:

```rust
mod in_range;
pub(crate) use self::in_range::{
    compute_in_range, effective_z_leptons, is_high_flying, is_low_flying,
};
```

**Note:** `compute_in_range` is referenced here even though it doesn't exist yet — Task 5 adds it. If `cargo check` complains, comment out `compute_in_range,` from the `use` line until Task 5; Tasks 4 and 5 add it back.

Alternative: only export `effective_z_leptons, is_high_flying, is_low_flying` for now; add `compute_in_range` to the export in Task 5.

**Step 3: Verify compilation**

Run: `cargo check -p ra2-rust-game --lib`
Expected: PASS. If `compute_in_range` referenced in the export breaks the build, use the alternative noted in Step 2.

**Step 4: Add baseline unit tests**

Append to `in_range.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    // Tests assume helper constructors `make_entity_at_level` and
    // `make_aircraft_with_altitude` from combat_tests.rs are reused or
    // inlined here; if not available, write minimal local fixtures.

    #[test]
    fn effective_z_ground_unit() {
        // Ground unit on level 5 should return 5 * 104 = 520 lep.
        // (Test implementation details depend on entity fixture API;
        //  use whatever combat_tests.rs uses to construct GameEntity.)
        // Pseudocode shape:
        //   let entity = ground_entity_at_level(5);
        //   assert_eq!(effective_z_leptons(&entity), 520);
    }

    #[test]
    fn effective_z_airborne_aircraft_adds_altitude() {
        // Aircraft on level 0 with altitude 1500 lep should return 1500.
        // On level 2 with altitude 800 should return 2*104 + 800 = 1008.
    }

    #[test]
    fn is_low_flying_only_for_airborne_aircraft() {
        // Ground UnitClass at level 5 → false.
        // Aircraft at altitude 0 → false (not in air).
        // Aircraft at altitude 500 → true (below 1000 threshold).
        // Aircraft at altitude 1500 → false (above threshold; high-flying).
    }

    #[test]
    fn is_high_flying_inverse_threshold() {
        // Aircraft at altitude 999 → false.
        // Aircraft at altitude 1000 → true (>= threshold).
        // Aircraft at altitude 1500 → true.
        // Ground unit at any level → false.
    }
}
```

Fill in the test bodies based on the entity-fixture pattern used in
`src/sim/combat/combat_tests.rs` (functions like `make_entity`, etc.).

**Step 5: Verify tests pass**

Run: `cargo test -p ra2-rust-game --lib in_range::tests`
Expected: 4 passed.

**Step 6: Sim-boundary check**

Run: `grep -rn "use crate::\(render\|ui\|sidebar\|audio\|net\)" src/sim/combat/in_range.rs`
Expected: empty output. (If anything matches, the sim/ boundary rule is broken — fix before commit.)

**Step 7: Commit**

Commit message: `combat/in_range: scaffold module + altitude/flight helpers`

---

### Task 4: Add `compute_effective_max_range_leptons` helper

**Why:** Centralizes the bonus chain. Stage 1 implements AirRange + Foundation; height-fire ships as a stub returning 0 with TODO; Stage 2 adds Bunker/OpenTopped/Veteran by extending this one helper without touching call sites.

**Files:**
- Modify: `src/sim/combat/in_range.rs` (append helper)

**Pattern:** Pure function, no side effects, takes references, returns `i64` leptons. Mirrors the design doc § "Internal flow."

**Step 1: Add the helper after the existing flight-gate helpers**

```rust
use crate::rules::ruleset::RuleSet;
use crate::rules::weapon_type::Weapon;       // verify exact module path
use crate::sim::combat::TargetKind;
use crate::sim::production::foundation_dimensions;
use crate::sim::world::EntityStore;          // verify exact module path
use crate::util::interner::StringInterner;

/// Stage 1: weapon base range (in leptons) plus AirRange bonus (if target is
/// high-flying) plus foundation bonus (if target is a building) plus
/// height-fire bonus (Stage 1 stub returns 0 — see height_fire_bonus_leptons).
///
/// Stages 2-N add: Garrison REPLACES, Bunker, OpenTopped, Veteran. Each is a
/// match arm or branch added to this function — call sites stay unchanged.
pub(crate) fn compute_effective_max_range_leptons(
    attacker: &GameEntity,
    target: &TargetKind,
    weapon: &Weapon,
    rules: &RuleSet,
    interner: &StringInterner,
    entities: &EntityStore,
) -> i64 {
    let mut range_lep: i64 = weapon.range.to_num::<i64>() * 256;

    // L13: AirRange bonus when target is high-flying.
    if let TargetKind::Entity(target_id) = *target {
        if let Some(target_entity) = entities.get(target_id) {
            if is_high_flying(target_entity) {
                if let Some(attacker_obj) = rules.object(interner.resolve(attacker.type_ref)) {
                    if let Some(air_bonus) = attacker_obj.air_range_bonus {
                        // gamemd stores AirRange in leptons (not cells). Add raw.
                        range_lep += air_bonus.to_num::<i64>() * 256;
                    }
                }
            }
        }
    }

    // L16: Foundation bonus when target is a building.
    // gamemd: (FoundationW + FoundationH) * 0x40 leptons.
    if let TargetKind::Entity(target_id) = *target {
        if let Some(target_entity) = entities.get(target_id) {
            if target_entity.category == EntityCategory::Structure {
                if let Some(target_obj) = rules.object(interner.resolve(target_entity.type_ref)) {
                    let (fw, fh) = foundation_dimensions(&target_obj.foundation);
                    range_lep += (fw as i64 + fh as i64) * 0x40;
                }
            }
        }
    }

    // L17: Height-fire bonus (gated by weapon.projectile.subject_to_elevation).
    // STAGE 1 STUB: returns 0. Gate verified during /review-plan as
    // `attacker.IsLowFlying() && target.IsLowFlying()` (gamemd disasm
    // 0x006F6F69-0x006F6F81). The bonus only fires when both attacker and
    // target are low-flying aircraft AND weapon's projectile sets
    // SubjectToElevation=yes — rare in standard YR play. Stage 2+ activates
    // by filling in the formula:
    //   sqrt((levels × 256)² + (DAT_00B0EB34 × dh)²)
    //   where levels = dh / Rules.ElevationIncrement (Rules+0x1838).
    //
    // Resolve weapon.projectile (Option<String>) via rules.projectile():
    let subject_to_elevation = weapon
        .projectile
        .as_deref()
        .and_then(|name| rules.projectile(name))
        .map(|p| p.subject_to_elevation)
        .unwrap_or(false);
    if subject_to_elevation {
        range_lep += height_fire_bonus_leptons(attacker, target, entities, rules);
    }

    range_lep
}

/// STAGE 1 STUB — returns 0.
///
/// gamemd `FUN_006F6F60` (height-fire bonus helper). Gate verified
/// (disasm 0x006F6F69-0x006F6F81, two `JZ exit` after IsLowFlying calls):
/// fires only when `attacker.IsLowFlying() && target.IsLowFlying()`. For
/// 99%+ of engagements (any ground attacker, any ground target, any
/// HighFlying-target case), this function would return 0 anyway — the stub
/// matches gamemd output for those cases.
///
/// The "wrong" cases (low-flying-vs-low-flying with SubjectToElevation
/// weapon) are rare; Stage 2+ implements the formula:
///   levels = max(0, target_cell_level − attacker_cell_level) / Rules.ElevationIncrement
///   bonus = sqrt((levels × 256)² + (DAT_00B0EB34 × dh)²)
fn height_fire_bonus_leptons(
    _attacker: &GameEntity,
    _target: &TargetKind,
    _entities: &EntityStore,
    _rules: &RuleSet,
) -> i64 {
    0
}
```

**Note on `weapon.projectile`:** the field is `Option<String>` ([weapon_type.rs:47](src/rules/weapon_type.rs#L47)). Resolve via `rules.projectile(name)` — confirm the exact accessor name on `RuleSet` before writing. There is no `weapon.projectile_ref()` shorthand.

**Note on `foundation_dimensions`:** lives in `crate::sim::production` (already public, used cross-module in 8+ files). No visibility upgrade needed.

**Step 2: Verify compilation**

Run: `cargo check -p ra2-rust-game --lib`
Expected: PASS.

**Step 3: Add unit tests**

Append to the `in_range::tests` module:

```rust
    #[test]
    fn effective_range_air_range_bonus_for_high_flying_target() {
        // Setup: attacker with AirRangeBonus=2 cells (= 512 lep), target = high-flying aircraft.
        // weapon.range = 4 cells (= 1024 lep). Expect effective_range = 1024 + 512 = 1536 lep.
    }

    #[test]
    fn effective_range_no_air_range_bonus_for_low_flying() {
        // Same setup but target altitude = 500 lep (low-flying). Expect 1024 lep (no bonus).
    }

    #[test]
    fn effective_range_foundation_bonus_for_building_target() {
        // Target = 4x2 building, attacker weapon.range = 4 cells. Foundation bonus = (4+2)*64 = 384 lep.
        // Expect effective_range = 1024 + 384 = 1408 lep.
    }

    #[test]
    fn effective_range_height_fire_stub_returns_zero() {
        // Weapon with subject_to_elevation=true, attacker on level 0, target on level 10.
        // Stage 1 stub: bonus = 0. Expect effective_range = base only.
    }
```

**Step 4: Verify**

Run: `cargo test -p ra2-rust-game --lib in_range::tests`
Expected: 4 new tests pass + 4 from Task 3 = 8 total.

**Step 5: Commit**

Commit message: `combat/in_range: effective max range helper (AirRange + Foundation)`

---

### Task 5: Add `compute_in_range` main function

**Why:** The single entry point that all four call sites converge on. Implements the verified gamemd flow: sentinel → arcing gate → max range with bonuses → 3D distance with LowFlying snap → min/max range gates → bridge LOS gate.

**Files:**
- Modify: `src/sim/combat/in_range.rs` (append main function)
- Modify: `src/sim/combat/mod.rs` (re-export `compute_in_range` if not already done in Task 3)

**Pattern:** Pure-function range query. Caller-builds-source contract per design doc.

**Step 1: Add `compute_in_range` and `compute_in_range_arcing_2d` helper**

```rust
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::util::fixed_math::{SIM_ZERO, SimFixed, isqrt_i64};
use crate::util::lepton::{
    BRIDGE_HEIGHT_DELTA_LEPTONS, WEAPON_RANGE_ALWAYS_IN_RANGE_LEPTONS,
};

/// Full 3D range check. Returns true if `attacker` (firing from `src`) can
/// hit `target` with `weapon`, accounting for all Stage 1 gates.
///
/// `src` is caller-supplied (typically `(attacker_x_lep, attacker_y_lep,
/// effective_z_leptons(attacker))`). AntiAircraft cell-snap deferred to
/// Stage Arcing.
pub(crate) fn compute_in_range(
    attacker: &GameEntity,
    src: (i64, i64, i64),
    target: &TargetKind,
    weapon: &Weapon,
    rules: &RuleSet,
    interner: &StringInterner,
    entities: &EntityStore,
    terrain: &ResolvedTerrainGrid,
) -> bool {
    let weapon_range_lep: i64 = weapon.range.to_num::<i64>() * 256;

    // L10: Sentinel — always-in-range short-circuit.
    if weapon_range_lep == WEAPON_RANGE_ALWAYS_IN_RANGE_LEPTONS {
        return true;
    }

    // L20: Arcing-weapon 2D fallthrough — preserves V3/Prism/etc current behavior.
    // Stage Arcing brainstorm replaces this with the full slope-arc check.
    // weapon.projectile is Option<String>; resolve via rules.projectile().
    let arcing = weapon
        .projectile
        .as_deref()
        .and_then(|name| rules.projectile(name))
        .map(|p| p.arcing)
        .unwrap_or(false);
    if arcing {
        return compute_in_range_arcing_2d(attacker, src, target, weapon, rules, interner, entities);
    }

    // L11–L17: Effective max range (AirRange + Foundation + height-fire stub).
    let max_range_lep = compute_effective_max_range_leptons(
        attacker, target, weapon, rules, interner, entities,
    );

    // L3, L4, L6: Resolve target coords with LowFlying ground-snap.
    let (tx, ty, tz) = resolve_target_coords_3d(target, entities, rules, interner, terrain);

    let (sx, sy, sz) = src;
    let dx = sx - tx;
    let dy = sy - ty;
    let dz = sz - tz;
    let dist_sq: i64 = dx * dx + dy * dy + dz * dz;
    let dist_lep = isqrt_i64(dist_sq);

    // L9: Min range — strict <.
    if weapon.minimum_range > SIM_ZERO {
        let min_range_lep = weapon.minimum_range.to_num::<i64>() * 256;
        if dist_lep < min_range_lep {
            return false;
        }
    }

    // L8: Max range — inclusive <=.
    if dist_lep > max_range_lep {
        return false;
    }

    // L22: Bridge LOS gate — attacker on bridge cell beneath, target on deck above.
    if attacker_under_bridge_targeting_above(src, tz, terrain) {
        return false;
    }

    true
}

/// Stage 1 arcing-weapon path: 2D distance only (preserves current behavior
/// for V3 / Prism / Dreadnought / Apocalypse Rocket / etc). Stage Arcing
/// adds the full Branch B slope-arc check.
fn compute_in_range_arcing_2d(
    attacker: &GameEntity,
    src: (i64, i64, i64),
    target: &TargetKind,
    weapon: &Weapon,
    rules: &RuleSet,
    interner: &StringInterner,
    entities: &EntityStore,
) -> bool {
    let weapon_range_lep: i64 = weapon.range.to_num::<i64>() * 256;
    if weapon_range_lep == WEAPON_RANGE_ALWAYS_IN_RANGE_LEPTONS {
        return true;
    }
    // Arcing weapons skip AirRange/Foundation/HeightFire in gamemd Branch B.
    // For Stage 1 we just check 2D distance vs base range, matching current
    // existing 2D behavior so arcing weapons don't regress.
    let max_range_lep = weapon_range_lep;

    let (tx_full, ty_full, _tz) = resolve_target_coords_3d_simple(target, entities, rules, interner);

    let (sx, sy, _sz) = src;
    let dx = sx - tx_full;
    let dy = sy - ty_full;
    let dist_sq: i64 = dx * dx + dy * dy;
    let dist_lep = isqrt_i64(dist_sq);

    if weapon.minimum_range > SIM_ZERO {
        let min_range_lep = weapon.minimum_range.to_num::<i64>() * 256;
        if dist_lep < min_range_lep {
            return false;
        }
    }
    dist_lep <= max_range_lep
}

/// Resolve target coords for the InRange 3D path. Applies LowFlying ground-snap
/// (with bridge offset) on entity targets; cell targets get cell-center XY and
/// `z_level × LEPTONS_PER_LEVEL` Z.
fn resolve_target_coords_3d(
    target: &TargetKind,
    entities: &EntityStore,
    rules: &RuleSet,
    interner: &StringInterner,
    terrain: &ResolvedTerrainGrid,
) -> (i64, i64, i64) {
    match *target {
        TargetKind::Entity(id) => {
            let Some(t) = entities.get(id) else {
                // Stale target — return absurd coords so the dist check rejects.
                return (i64::MAX / 4, i64::MAX / 4, 0);
            };
            // For buildings: shift to foundation center (matches existing
            // target_coords() in combat/mod.rs:206).
            let (rx, ry, sub_x, sub_y) = if t.category == EntityCategory::Structure {
                if let Some(obj) = rules.object(interner.resolve(t.type_ref)) {
                    let (fw, fh) = crate::sim::production::foundation_dimensions(&obj.foundation);
                    let offset_x = (fw.saturating_sub(1) as i32) * 128;
                    let offset_y = (fh.saturating_sub(1) as i32) * 128;
                    let full_x: i32 = t.position.rx as i32 * 256
                        + t.position.sub_x.to_num::<i32>()
                        + offset_x;
                    let full_y: i32 = t.position.ry as i32 * 256
                        + t.position.sub_y.to_num::<i32>()
                        + offset_y;
                    (
                        (full_x / 256) as u16,
                        (full_y / 256) as u16,
                        SimFixed::from_num(full_x % 256),
                        SimFixed::from_num(full_y % 256),
                    )
                } else {
                    (t.position.rx, t.position.ry, t.position.sub_x, t.position.sub_y)
                }
            } else {
                (t.position.rx, t.position.ry, t.position.sub_x, t.position.sub_y)
            };
            let tx = rx as i64 * 256 + sub_x.to_num::<i64>();
            let ty = ry as i64 * 256 + sub_y.to_num::<i64>();
            let tz = if is_low_flying(t) {
                ground_z_with_bridge_offset(rx, ry, terrain)
            } else {
                effective_z_leptons(t)
            };
            (tx, ty, tz)
        }
        TargetKind::Cell(rx, ry) => {
            let tx = rx as i64 * 256 + 128;
            let ty = ry as i64 * 256 + 128;
            let tz = ground_z_with_bridge_offset(rx, ry, terrain);
            (tx, ty, tz)
        }
    }
}

/// 2D-only target coords resolution for arcing weapons (no Z snap, no terrain
/// lookup). Mirrors the foundation-center adjustment for buildings.
fn resolve_target_coords_3d_simple(
    target: &TargetKind,
    entities: &EntityStore,
    rules: &RuleSet,
    interner: &StringInterner,
) -> (i64, i64, i64) {
    match *target {
        TargetKind::Entity(id) => {
            // Reuse existing 2D resolution.
            if let Some((rx, ry, sub_x, sub_y)) =
                crate::sim::combat::resolve_target_coords(target, entities, Some(rules), interner)
            {
                let tx = rx as i64 * 256 + sub_x.to_num::<i64>();
                let ty = ry as i64 * 256 + sub_y.to_num::<i64>();
                let tz = entities
                    .get(id)
                    .map(|t| effective_z_leptons(t))
                    .unwrap_or(0);
                (tx, ty, tz)
            } else {
                (i64::MAX / 4, i64::MAX / 4, 0)
            }
        }
        TargetKind::Cell(rx, ry) => (rx as i64 * 256 + 128, ry as i64 * 256 + 128, 0),
    }
}

/// Ground Z in leptons for a cell, plus bridge deck offset if the cell has
/// a bridge deck on it.
fn ground_z_with_bridge_offset(rx: u16, ry: u16, terrain: &ResolvedTerrainGrid) -> i64 {
    let cell_idx = ry as usize * terrain.width() as usize + rx as usize;
    let cell = match terrain.cells.get(cell_idx) {
        Some(c) => c,
        None => return 0,
    };
    let mut z = cell.level as i64 * LEPTONS_PER_LEVEL;
    if cell.has_bridge_deck {
        z += BRIDGE_HEIGHT_DELTA_LEPTONS;
    }
    z
}

/// Bridge LOS gate: returns true when attacker is in a bridge cell, at a Z
/// below the bridge deck top, and the target Z is at or above the deck top
/// — meaning the attacker would have to fire through the deck.
///
/// Verified at gamemd 0x006F75FB-0x006F762F (corrected interpretation per
/// TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md §0).
fn attacker_under_bridge_targeting_above(
    src: (i64, i64, i64),
    target_z_lep: i64,
    terrain: &ResolvedTerrainGrid,
) -> bool {
    let (sx, sy, sz) = src;
    let rx = (sx / 256) as u16;
    let ry = (sy / 256) as u16;
    let cell_idx = ry as usize * terrain.width() as usize + rx as usize;
    let cell = match terrain.cells.get(cell_idx) {
        Some(c) => c,
        None => return false,
    };
    if !cell.has_bridge_deck {
        return false;
    }
    let bridge_top = cell.level as i64 * LEPTONS_PER_LEVEL + BRIDGE_HEIGHT_DELTA_LEPTONS;
    sz < bridge_top && target_z_lep >= bridge_top
}
```

**Notes on field accessors (verified during /review-plan):**
- `weapon.projectile: Option<String>` ([weapon_type.rs:47](src/rules/weapon_type.rs#L47)) → resolve via `rules.projectile(name)`. Confirm exact accessor on `RuleSet` if needed.
- `ResolvedTerrainCell` fields: `pub level: u8` (line 75), `pub has_bridge_deck: bool` (line 121), `pub bridge_deck_level: u8` (line 124).
- `ResolvedTerrainGrid.width()` and `.height()` accessors at [resolved_terrain.rs:166, 170](src/map/resolved_terrain.rs#L166-L170).
- `crate::sim::production::foundation_dimensions` — already public; no visibility upgrade needed.
- `crate::sim::combat::resolve_target_coords` — already `pub(crate)` per existing code at combat/mod.rs:255.

**Step 2: Hook up the export**

If Task 3 already exported `compute_in_range`, this step is done. Otherwise, in `combat/mod.rs`:

```rust
pub(crate) use self::in_range::{compute_in_range, ...};
```

**Step 3: Verify compilation**

Run: `cargo check -p ra2-rust-game --lib`
Expected: PASS. Field/method name mismatches noted above are likely; fix as you go.

**Step 4: Sim-boundary check**

Run: `grep -rn "use crate::\(render\|ui\|sidebar\|audio\|net\)" src/sim/combat/in_range.rs`
Expected: empty.

**Step 5: Commit**

Commit message: `combat/in_range: compute_in_range main function (3D + bridge LOS + arcing gate)`

---

### Task 6: Add `compute_in_range` unit tests

**Why:** Catch regressions on the parity-critical items (sentinel, boundary, 3D vs 2D divergence, LowFlying snap, AirRange, Foundation, arcing fallthrough, bridge LOS).

**Files:**
- Modify: `src/sim/combat/in_range.rs` (append to `mod tests`)

**Pattern:** Existing combat test fixture pattern from `src/sim/combat/combat_tests.rs` — `make_entity`, `test_rules`, `test_interner`. Reuse where possible; add minimal local fixtures only if needed.

**Step 1: Add the 11 tests**

```rust
    use crate::sim::combat::TargetKind;
    use crate::util::fixed_math::SimFixed;

    // Test 1: Sentinel always-in-range
    #[test]
    fn sentinel_always_in_range() {
        // weapon.range = -2 cells (= -512 lep) → returns true regardless of distance.
    }

    // Test 2: Boundary inclusive max
    #[test]
    fn max_range_inclusive_at_exact_boundary() {
        // distance == range → true. distance == range + 1 lep → false.
    }

    // Test 3: Boundary strict min
    #[test]
    fn min_range_strict_at_exact_boundary() {
        // weapon.minimum_range = 2 cells. distance == 2 cells (= 512 lep) → true (inclusive).
        // distance == 2 cells - 1 lep (= 511 lep) → false (inside min, strict <).
    }

    // Test 4: 3D vs 2D divergence
    #[test]
    fn three_d_distance_rejects_high_z_delta() {
        // dx=dy=0, dz = 10 levels (= 1040 lep). weapon.range = 4 cells (= 1024 lep). → false.
        // Same setup with weapon.range = 5 cells (= 1280 lep). → true.
        // Documents that 3D-aware InRange is stricter than 2D in the elevation direction.
    }

    // Test 5: LowFlying ground-snap
    #[test]
    fn low_flying_target_z_snapped_to_ground() {
        // Aircraft target at altitude 500 lep on cell level 0 (low-flying).
        // Attacker at level 0, horizontal distance 4 cells. weapon.range = 4 cells.
        // Without snap: dist² = (4*256)² + 500² > 4*256² → false.
        // With snap (Stage 1): tz = 0, dz = 0, dist = 1024 lep == range → true.
    }

    // Test 6: HighFlying does NOT snap, AirRange bonus applies
    #[test]
    fn high_flying_target_uses_actual_z_with_air_range_bonus() {
        // Aircraft target at altitude 1500 lep on level 0.
        // Attacker at level 0, horizontal 4 cells. weapon.range=4, AirRangeBonus=2.
        // Effective max = 4+2 = 6 cells (= 1536 lep).
        // dist = sqrt(1024² + 1500²) ≈ 1816 lep.
        // 1816 > 1536 → false (Z penalty dominates even with AirRange).
    }

    // Test 7: Foundation bonus on building target
    #[test]
    fn foundation_bonus_extends_range_for_building_target() {
        // Target = 4x2 building. weapon.range = 4 cells.
        // Foundation bonus = (4+2)*64 = 384 lep = 1.5 cells.
        // Attacker at horizontal 5 cells (= 1280 lep) from building NW corner.
        // Without bonus: 1280 > 1024 → false.
        // With bonus: 1024 + 384 = 1408 lep, 1280 < 1408 → true.
    }

    // Test 8: Sentinel beats min-range
    #[test]
    fn sentinel_overrides_min_range() {
        // weapon.range = -2 cells (sentinel), MinimumRange = 10 cells.
        // distance = 0 → true (sentinel wins, min not checked).
    }

    // Test 9: Cell target (no LowFlying snap, no bonuses)
    #[test]
    fn cell_target_uses_3d_distance_no_bonuses() {
        // TargetKind::Cell on level 0. Attacker on level 5 (= 520 lep up).
        // weapon.range = 2 cells (= 512 lep). Horizontal distance = 0.
        // dist = 520 lep > 512 → false (3D-aware).
    }

    // Test 10: Arcing weapon falls through to 2D
    #[test]
    fn arcing_weapon_uses_2d_distance() {
        // Weapon with projectile.arcing = true. Attacker on level 0,
        // target on level 5 (dz = 520 lep), horizontal 4 cells (= 1024 lep).
        // Arcing: dist_2d = 1024 lep == range → true.
        // (Without arcing fallthrough, the 3D path would reject due to dz.)
    }

    // Test 11: Bridge LOS gate
    #[test]
    fn bridge_los_gate_blocks_under_bridge_to_deck() {
        // Setup: terrain with a bridge cell at (5, 5), bridge_deck=true,
        // ground level 0 (deck top = 0 + 416 = 416 lep).
        // Attacker at (5, 5) on level 0, attacker.Z = 0 (under bridge).
        // Target on the deck at Z = 416.
        // Should reject.
        //
        // Same setup but attacker.Z = 416 (on the deck) → should NOT trigger
        // the gate (attacker.Z >= bridge_top).
    }
```

Fill test bodies with specific entity construction — follow the patterns in
`src/sim/combat/combat_tests.rs`. For tests requiring `ResolvedTerrainGrid`,
look at `src/sim/world/world_tests.rs:69, 119, 433` for existing fixture
patterns that build a `ResolvedTerrainGrid` from a vec of `ResolvedTerrainCell`s.

**Step 2: Verify**

Run: `cargo test -p ra2-rust-game --lib in_range::tests`
Expected: 11 + previous 8 = 19 passed.

**Step 3: Commit**

Commit message: `combat/in_range: 11 unit tests covering parity-critical items`

---

### Task 6.5: Thread `&ResolvedTerrainGrid` through targeting + cursor signatures

**Why:** `compute_in_range` needs `&ResolvedTerrainGrid` for the bridge LOS gate and the LowFlying ground-snap. Current targeting / cursor functions don't carry it. This task does the parameter plumbing **before** the call-site migrations begin (Tasks 7-10), so each migration is a clean local edit.

**Files:**
- Modify: `src/sim/combat/combat_targeting.rs:69, 128` (`acquire_best_target_for_entity`, `acquire_best_target` signatures)
- Modify: `src/sim/combat/mod.rs` (callers of the above; specifically search for `acquire_best_target` and `acquire_best_target_for_entity`)
- Modify: `src/app_cursor.rs:311` (`any_selected_unit_in_range` signature) — verify cursor scope can reach terrain via `&Simulation` accessor; if not, add a `pub fn terrain(&self) -> &ResolvedTerrainGrid` accessor on `Simulation`
- Possibly modify: `src/sim/world/mod.rs` (where the targeting tick is called from `advance_tick`) — confirm `&ResolvedTerrainGrid` is available in that scope

**Pattern:** Existing example to mirror: [bridge_state/walker.rs:30](src/sim/bridge_state/walker.rs#L30) takes `terrain: &ResolvedTerrainGrid` directly. Match that convention.

**Step 1: Add `terrain: &ResolvedTerrainGrid` parameter**

To both `acquire_best_target_for_entity` (currently at line 69) and `acquire_best_target` (currently at line 128). The parameter is unused at this step — it's plumbing only.

**Step 2: Update all callers**

```
grep -rn "acquire_best_target\b" src/
```

For each caller, pass `terrain` through. Start from `combat/mod.rs` and walk up to `World::advance_tick`. The terrain is available on `Simulation` (verify exact accessor — likely `sim.terrain` or `sim.world.terrain`; if the field is private, the accessor needs to be added).

**Step 3: Update cursor**

`any_selected_unit_in_range` at [app_cursor.rs:311](src/app_cursor.rs#L311) takes `sim: &Simulation`. Add the `&ResolvedTerrainGrid` parameter (prefer explicit over reaching into `sim`), then update its caller in the cursor-update flow.

**Step 4: Verify compilation**

Run: `cargo check -p ra2-rust-game --lib`
Expected: PASS. Compiler will guide you to any caller missed.

**Step 5: Run combat tests**

Run: `cargo test -p ra2-rust-game --lib combat`
Expected: PASS. No behavioral change yet — just parameter plumbing.

**Step 6: Commit**

Commit message: `combat/cursor: thread &ResolvedTerrainGrid through targeting signatures (no behavior change)`

---

### Task 7: Migrate `combat_targeting.rs:193` to `compute_in_range`

**Why:** First call site swap. Smallest and most isolated of the four — proves the API works end-to-end before touching the more complex sites.

**Files:**
- Modify: `src/sim/combat/combat_targeting.rs:193`

**Pattern:** Replace `lepton_distance_sq_raw + is_within_range_leptons` with `compute_in_range`. Build `src` from attacker fields + `effective_z_leptons`. Wrap candidate as `TargetKind::Entity(candidate.stable_id)`.

**Step 1: Edit lines 193-205**

Read current:

```rust
        let dist_sq = lepton_distance_sq_raw(
            attacker.pos_rx,
            attacker.pos_ry,
            attacker.sub_x,
            attacker.sub_y,
            candidate.position.rx,
            candidate.position.ry,
            candidate.position.sub_x,
            candidate.position.sub_y,
        );
        if !is_within_range_leptons(dist_sq, scan_range) {
            continue;
        }
```

Replace with:

```rust
        // 3D range check via gamemd-parity InRange. attacker is a snapshot
        // (no GameEntity reference), so look up the entity for altitude/category.
        let attacker_entity = match entities.get(attacker.stable_id) {
            Some(e) => e,
            None => continue, // attacker despawned mid-scan
        };
        let src = (
            attacker.pos_rx as i64 * 256 + attacker.sub_x.to_num::<i64>(),
            attacker.pos_ry as i64 * 256 + attacker.sub_y.to_num::<i64>(),
            crate::sim::combat::effective_z_leptons(attacker_entity),
        );
        // selected.weapon already gives the resolved &Weapon. scan_range is the
        // overridden range (for guard_range / garrison) — a temporary weapon
        // ref with overridden range matches the existing pattern; for now
        // pass the resolved weapon and let compute_in_range handle bonuses.
        // SCAN RANGE OVERRIDE: pursuit/garrison may pass a different range
        // than weapon.range. Stage 1 uses weapon.range directly via compute_in_range;
        // if scan_range differs from weapon.range, log a warning and fall back
        // to the existing 2D path for parity safety.
        let in_range = if scan_range == selected.weapon.range {
            crate::sim::combat::compute_in_range(
                attacker_entity,
                src,
                &TargetKind::Entity(candidate.stable_id),
                selected.weapon,
                rules,
                interner,
                entities,
                terrain,
            )
        } else {
            // Override path — preserve current 2D behavior until Stage 2 refines
            // scan_range_override threading through compute_in_range.
            let dist_sq = lepton_distance_sq_raw(
                attacker.pos_rx,
                attacker.pos_ry,
                attacker.sub_x,
                attacker.sub_y,
                candidate.position.rx,
                candidate.position.ry,
                candidate.position.sub_x,
                candidate.position.sub_y,
            );
            is_within_range_leptons(dist_sq, scan_range)
        };
        if !in_range {
            continue;
        }
```

**Note:** Task 6.5 already threaded `terrain: &ResolvedTerrainGrid` through this function's signature. It's in scope here.

**Step 2: Verify compilation**

Run: `cargo check -p ra2-rust-game --lib`
Expected: PASS.

**Step 3: Run existing combat tests**

Run: `cargo test -p ra2-rust-game --lib combat`
Expected: All existing tests pass (no behavioral change for ground-vs-ground at level 0).

**Step 4: Commit**

Commit message: `combat/targeting: migrate passive acquisition to compute_in_range`

---

### Task 8: Migrate `combat/mod.rs:1049` (garrison passive acquisition)

**Why:** Second call site. Same pattern as Task 7. Garrison-specific scan range override is preserved via the override-path fallback (same pattern as Task 7).

**Files:**
- Modify: `src/sim/combat/mod.rs:1049`

**Pattern:** Identical to Task 7.

**Step 1: Read current code at line 1049-1061** (already shown in the design analysis).

**Step 2: Replace with the same pattern as Task 7**

Build `src` from `pos_rx/ry/sub_x/sub_y`, look up attacker entity from `entities`, call `compute_in_range` with `TargetKind::Entity(candidate.stable_id)`. Preserve the garrison `scan_range` override path with the 2D fallback if it differs from `selected.weapon.range`.

The detailed code shape is the same as Task 7 — copy and adapt to local variable names.

**Step 3: Verify compilation + tests**

```
cargo check -p ra2-rust-game --lib
cargo test -p ra2-rust-game --lib combat
```
Expected: PASS, all tests pass.

**Step 4: Commit**

Commit message: `combat: migrate garrison passive acquisition to compute_in_range`

---

### Task 9: Migrate `combat/mod.rs:1381` (combat fire gate)

**Why:** Third call site — the active fire gate. Has the most context (attacker snapshot, weapon, target may be Entity or Cell).

**Files:**
- Modify: `src/sim/combat/mod.rs:1381-1413`

**Pattern:** Same as Tasks 7-8. Additionally handles `TargetKind::Cell` (force-fire on terrain) — pass through directly rather than synthesizing entity.

**Step 1: Identify the target kind**

The existing code at line 1381 uses destructured `target_rx, target_ry, target_sub_x, target_sub_y`. Upstream there's a check `is_cell_target` distinguishing Entity vs Cell. Use that to construct the correct `TargetKind` variant:

```rust
let target_kind = if is_cell_target {
    TargetKind::Cell(target_rx, target_ry)
} else {
    TargetKind::Entity(snap.attack_target_id)
};
```

**Step 2: Replace the distance check**

Use the same pattern as Task 7. The garrison-specific `effective_range` override branch (lines 1392-1397) needs the same fallback treatment as Task 7's scan-range override.

**Step 3: Verify**

```
cargo check -p ra2-rust-game --lib
cargo test -p ra2-rust-game --lib combat
```
Expected: PASS.

**Step 4: Commit**

Commit message: `combat: migrate combat fire gate to compute_in_range`

---

### Task 10: Migrate `app_cursor.rs:346` (cursor in-range)

**Why:** Fourth and final call site. Cursor is outside `sim/` — it's the user-facing "can I shoot this?" indicator. Must agree with combat sites.

**Files:**
- Modify: `src/app_cursor.rs:346-358`

**Pattern:** Same as Tasks 7-9. Cursor scope already has `&Simulation` and `&RuleSet`; needs to access `&ResolvedTerrainGrid` (likely via `sim.world.terrain` or similar).

**Step 1: Verify cursor has terrain access**

Read `src/app_cursor.rs` around line 311 (function `any_selected_unit_in_range`). Check if `&ResolvedTerrainGrid` is accessible — likely via `sim.world.terrain()` or similar. If not directly accessible, either:
- Add a public accessor on Simulation: `pub fn terrain(&self) -> &ResolvedTerrainGrid`
- Pass terrain in from the cursor-update call site

**Step 2: Replace the distance check at line 346**

```rust
        let in_range = combat::compute_in_range(
            entity,
            (
                entity.position.rx as i64 * 256 + entity.position.sub_x.to_num::<i64>(),
                entity.position.ry as i64 * 256 + entity.position.sub_y.to_num::<i64>(),
                combat::effective_z_leptons(entity),
            ),
            &TargetKind::Entity(target_id),
            // Need a &Weapon, not just weapon_range. Fetch via rules.weapon(name).
            match obj.primary.as_ref().and_then(|w| rules.weapon(w)) {
                Some(w) => w,
                None => continue,
            },
            rules,
            &sim.interner,
            &sim.entities,
            &sim.world.terrain,  // confirm exact accessor
        );
        if in_range {
            return true;
        }
```

(The current cursor code computes `weapon_range` separately and calls
`is_within_range_leptons`. The new code passes the full `&Weapon` and lets
`compute_in_range` handle range derivation.)

**Step 3: Verify cursor behavior unchanged in flat tests**

Run: `cargo test -p ra2-rust-game --lib app_cursor`
Expected: PASS (if such tests exist; otherwise rely on Task 13 manual verification).

Run: `cargo build -p ra2-rust-game`
Expected: PASS.

**Step 4: Commit**

Commit message: `app_cursor: migrate in-range check to compute_in_range`

---

### Task 11: Bump SNAPSHOT_VERSION

**Why:** Stage 1 changes targeting reads (now consult `loco.altitude`), which changes the state hash whenever an aircraft is mid-flight in a targeting calculation. Old saves/replays cannot be loaded.

**Files:**
- Modify: `src/sim/snapshot.rs:16`

**Step 1: Edit the constant**

Before:
```rust
const SNAPSHOT_VERSION: u32 = 5;
```

After:
```rust
const SNAPSHOT_VERSION: u32 = 6;
```

**Step 2: Update any related test fixtures**

Run: `grep -rn "SNAPSHOT_VERSION\|version: 5" src/sim/`
Verify no test hardcodes `5` for the version field; if any do, update to `6`.

**Step 3: Verify**

```
cargo build -p ra2-rust-game
cargo test -p ra2-rust-game --lib snapshot
```
Expected: PASS.

**Step 4: Commit**

Commit message: `sim/snapshot: bump SNAPSHOT_VERSION 5 → 6 for InRange 3D Stage 1

Targeting now reads aircraft altitude, changing state hash for any in-flight
air-vs-ground engagement. One-time replay break.`

---

### Task 12: Full test suite + state-hash determinism check

**Why:** Verify Stage 1 doesn't regress existing combat behavior and preserves determinism.

**Files:** none (test-only)

**Step 1: Run full test suite**

```
cargo test -p ra2-rust-game --lib
```
Expected: All tests pass. Failing tests are either (a) a Stage 1 bug or (b) a test that needs updating because it depended on 2D semantics. Categorize each failure before fixing.

**Step 2: Determinism regression**

If a state-hash-roundtrip test exists (`src/sim/snapshot.rs:146`
`round_trip_preserves_state_hash`), confirm it still passes. If not, run the
existing `deploy_tests.rs:486` pattern that compares two simulation runs
with the same seed and asserts identical state hashes.

**Step 3: Profile-time check (optional)**

Run a representative skirmish for ~60 seconds. Verify framerate is unchanged
within noise (compute_in_range is per-targeting-tick; should be cheap).

**Step 4: No commit unless tests need fixing**

If everything passes, no commit. If a test needed updating, commit:

`tests: update <test_name> for InRange 3D Stage 1 expectations`

---

### Task 13: Manual in-game verification

**Why:** Confirm parity-critical items behave correctly against the original game (or at least intuitive 3D-aware behavior).

**Files:** none (manual)

**Step 1: Kirov vs SAM at altitude**

Spawn a Kirov at cruise altitude over a Patriot/SAM site. Verify the SAM engages at the expected range — should match gamemd within ±1 cell. Compare to current 2D behavior (Kirov gettable from anywhere within 2D range without altitude penalty).

**Step 2: Building target range**

Aim at a 4x2 building (e.g., Construction Yard) with a tank. Move the tank
to the edge of effective range. Compare to a 2x2 building (Power Plant) —
the CY should be hittable at slightly longer range due to foundation bonus.

**Step 3: V3 / arcing weapon parity**

Fire a V3 Rocket at a target across a cliff. Verify the V3 hits at the same
horizontal range as before — no regression from arcing-fallthrough.

**Step 4: Bridge LOS scenario**

Place a tank under a bridge. Place infantry on the bridge deck above. Order
the tank to attack — should be REJECTED (cannot fire through deck). Order
the same tank to attack ground-level infantry on the same horizontal cell
as the bridge — should fire normally.

**Step 5: Boundary precision**

Use any tank with weapon range = 6 cells. Move target unit to exactly 6
cells away (cell-distance). Should fire (inclusive). Move to 6 cells + 1
sub_lepton offset — should still fire (at the boundary). Move further —
should stop firing.

**Step 6: If anything diverges from gamemd, log it and decide:**
- Bug in Stage 1 → fix and re-test.
- Open question revealed (e.g., HIGH_FLIGHT_THRESHOLD wrong) → record in
  research doc, note as Stage 1 follow-up.

**No commit** unless code changes are needed.

---

## Stage 2+ Follow-Ups (Not in this Plan)

- **Bunker / OpenTopped / Veteran range bonuses** — extend `compute_effective_max_range_leptons` with these match arms when Stage 2 brainstorm covers them. No call site rewrites needed.
- **Garrison REPLACES** — Stage 2 brainstorm decides whether to integrate the existing garrison range logic into `compute_in_range` or keep it parallel.
- **Height-fire activation** — gate is verified (`attacker.IsLowFlying() && target.IsLowFlying()`); formula known. Stage 2+ implementation only needs `DAT_00B0EB34` runtime value (one Ghidra read) and replacing the stub function body. No structural change needed.
- **Stage Arcing brainstorm** — full Branch B with slope arc check, replacing the Stage 1 2D fallthrough. Separate brainstorm.

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-10-inrange-3d-distance-design.md](2026-05-10-inrange-3d-distance-design.md)
- **Primary Ghidra report:** [ra2-rust-game-docs/TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md) — covers function signature, distance metric, boundary semantics, branch flavors, range bonus chain, sources of source/target coords, sqrt approximation note, TS-vs-YR audit, open questions
- **Bonus chain reference:** [ra2-rust-game-docs/BUNKER_SYSTEM_GHIDRA_REPORT.md §5](../../../ra2-rust-game-docs/BUNKER_SYSTEM_GHIDRA_REPORT.md#L200-L232) — Garrison REPLACES, Bunker, OpenTopped (Stage 2 referent)
- **Coordinate system:** [ra2-rust-game-docs/COORDINATE_SYSTEM_GAMEMD.md:127-131](../../../ra2-rust-game-docs/COORDINATE_SYSTEM_GAMEMD.md#L127-L131) — LevelHeight = 104 verification
- **gamemd.exe addresses:**
  - `0x006F7220` — TechnoClass::InRange (primary function)
  - `0x004CAC40` — Sqrt_Approx (float32-LUT approximation)
  - `0x007C5F00` — Math::ftol
  - `0x005F65A0` — ObjectClass::GetCoords
  - `0x005F6B60` / `0x005F6B90` — IsLowFlying / IsHighFlying
  - `0x006F6F60` / `0x006F70E0` — Height-fire bonus helpers (Stage 1 stubbed)
  - `0x89DDB8` — LevelHeight constant (= 104)
  - `g_RulesClass + 0x1838` — ElevationIncrement
- **INI keys:**
  - `[General] FlightLevel=` (already parsed)
  - `[ElevationModel] ElevationIncrement=`, `ElevationIncrementBonus=`, `ElevationBonusCap=` (Stage 2 — used when height-fire is activated)
  - WeaponType `MinimumRange=`, `Range=` (already parsed)
  - ProjectileType `Arcing=`, `SubjectToElevation=` (already parsed)
  - TechnoTypeClass `AirRangeBonus=` (Task 1 adds parsing)
- **Related repo code:**
  - `src/sim/combat/mod.rs:144` — `TargetKind` enum (reused)
  - `src/sim/combat/mod.rs:255` — `resolve_target_coords` (reused for arcing 2D path)
  - `src/sim/combat/mod.rs:206` — `target_coords` (foundation-center adjustment pattern, replicated in `resolve_target_coords_3d`)
  - `src/sim/combat/mod.rs:1750` — `lepton_distance_sq_raw` (kept for AOE; not deleted)
  - `src/util/fixed_math.rs:246` — `isqrt_i64` (reused)
  - `src/rules/object_type.rs:259, 776` — `guard_range` (parser pattern mirrored for `air_range_bonus`)
  - `src/rules/projectile_type.rs:42, 59` — `arcing`, `subject_to_elevation` (already parsed; consumed in Task 5)
  - `src/map/resolved_terrain.rs:68-180` — `ResolvedTerrainCell` + `ResolvedTerrainGrid` (cell-elevation lookups for bridge LOS)
  - `src/sim/snapshot.rs:16` — `SNAPSHOT_VERSION` (Task 11 bumps)
- **Prior commits relevant to this work:**
  - `5c66164` combat: add pursuit_weapon_range helper + promote resolve_target_coords (2026-05-08)
  - `d0f1605` combat: TargetKind enum widens AttackTarget for cell-target attacks
  - `86af0cd` combat: Ctrl-click force-fire on empty terrain + Alt+Ctrl override
  - `3c7e38b` combat: range failure preserves attack_target (gamemd parity)
