# InRange 3D Distance — Stage 1 Design

## Goal

Replace the 2D `lepton_distance_sq_raw` + `is_within_range_leptons` pair at the four
targeting/cursor sites with a single `compute_in_range` function that mirrors gamemd
`TechnoClass::InRange (0x006F7220)` for the distance-side parity ledger: 3D Euclidean
distance, IsLowFlying ground-snap, AirRange bonus, height-fire bonus, bridge LOS gate,
foundation bonus, arcing-weapon early gate, and the verified boundary semantics
(`<=` max, `<` min, `-512` sentinel).

## Architecture Context

**Current state.** `src/sim/combat/mod.rs:1750` defines `lepton_distance_sq_raw(ax_cell,
ay_cell, ax_sub, ay_sub, bx_cell, by_cell, bx_sub, by_sub) -> i64` returning squared
distance in lepton² space. Paired with `is_within_range_leptons(dist_sq, range_cells)
-> bool` that does `dist_sq <= range_lep²`. Five call sites:

- [combat_targeting.rs:193](../../src/sim/combat/combat_targeting.rs#L193) passive target acquisition
- [combat/mod.rs:1049](../../src/sim/combat/mod.rs#L1049) garrison passive acquisition
- [combat/mod.rs:1381](../../src/sim/combat/mod.rs#L1381) combat fire gate
- [combat_aoe.rs:59](../../src/sim/combat/combat_aoe.rs#L59) AOE damage spread (out of scope)
- [app_cursor.rs:346](../../src/app_cursor.rs#L346) cursor "in range" check

**Position model.** [components.rs:31-49](../../src/sim/components.rs#L31-L49):
`pos.rx, pos.ry: u16` cell coords, `pos.z: u8` cell elevation level, `pos.sub_x, sub_y:
SimFixed` sub-cell lepton offsets. Aircraft altitude is on the locomotor:
[locomotor.rs:140](../../src/sim/movement/locomotor.rs#L140) `Locomotor.altitude: SimFixed`.

**No `LEPTONS_PER_LEVEL` constant exists yet.** The visual pixel projection uses
`HEIGHT_STEP = 15.0` (rendering-only); gameplay distance lives in lepton space and needs
the gameplay-grade `LevelHeight = 104` from gamemd `0x89DDB8`
([COORDINATE_SYSTEM_GAMEMD.md:127-131](../../../ra2-rust-game-docs/COORDINATE_SYSTEM_GAMEMD.md#L127-L131)).

**gamemd contract** (verified, see [TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md)):

```c
__thiscall TechnoClass::InRange(
    /* this */ TechnoClass* attacker,
    CoordStruct const* src,           // caller-built source position
    AbstractClass* target,
    WeaponTypeClass* weapon
) -> bool
```

Source is caller-supplied (often `attacker->Get_Coords()` with optional cell-snap);
target's coords come from `target->Get_Coords()` (vtable+0x48 reading struct
`+0x9C/+0xA0/+0xA4`).

## Impact Analysis

**Files modified** (Stage 1):
- `src/util/lepton.rs` — new constants: `LEPTONS_PER_LEVEL`, `WEAPON_RANGE_ALWAYS_IN_RANGE_LEPTONS`, `HIGH_FLIGHT_THRESHOLD_LEPTONS`
- `src/sim/combat/in_range.rs` — **new file**, `compute_in_range` + helpers
- `src/sim/combat/mod.rs` — `pub mod in_range;` + re-export
- `src/sim/combat/combat_targeting.rs:193` — call site swap
- `src/sim/combat/mod.rs:1049, :1381` — call site swaps
- `src/app_cursor.rs:346` — call site swap
- One state-hash version constant — bumped (replay break)

**Files NOT modified:**
- `src/sim/combat/combat_aoe.rs` — keeps 2D path. AOE Z-awareness is its own brainstorm.
- `src/sim/components.rs` — no Position struct changes. `effective_z_leptons` computed per call.

**Threading verified:**
- All four call sites already have `&RuleSet`, `&StringInterner`, `&EntityStore` in scope.
- Cursor site needs `&MapData` (or whatever the cell-elevation accessor is) — verify in scope before implementation.

**Determinism risk (accepted):** Targeting now reads `loco.altitude` (a `SimFixed` that
changes mid-tick during ascent/descent/dive). State hash will differ between the old and
new versions whenever an aircraft is mid-flight in a targeting calculation. **Replay
version bump required**, documented as one-time desync.

**Sqrt approximation risk (accepted):** gamemd uses `Sqrt_Approx` (float32-LUT,
non-deterministic across platforms in worst case, fine on x86). We use `isqrt_i64`
(precise integer, deterministic by construction). At very large distances, ±1 lepton
divergence at boundaries is possible. Per CLAUDE.md parity bar
("indistinguishable in a single skirmish"), this is acceptable: the divergence is at
the scale of 1 lepton over typical battle ranges (≤50 cells = ≤12,800 leptons).

## Chosen Approach

**Approach B — Faithful**: single `compute_in_range` function that mirrors gamemd's
InRange architecture. All targeting + cursor sites converge on it. Stages 2-N add bonus
chain match-arms inside one helper, no call site rewrites.

Rejected alternatives summarized at the end.

## Tiny-Detail Ledger

Constraints Stage 1 must preserve. Each item cites the verified source in the research
doc. **Items marked "Stage 1"** are implemented now; **"Stage 2"/"Stage Arcing"** are
named-but-deferred so they don't get silently dropped.

### Distance computation

| # | Detail | Source | Stage |
|---|--------|--------|-------|
| L1 | 3D Euclidean: `(int)Math::ftol(Sqrt_Approx(dx² + dy² + dz²))`. Rust uses `isqrt_i64` — deliberate parity tightening (precise integer vs float-LUT, ±1 lepton at boundaries). | doc §2.1, §0 unchanged | Stage 1 |
| L2 | Source coords supplied by caller as 3-tuple `(i64, i64, i64)`. InRange does NOT call `attacker.get_coords()`. | doc §1 | Stage 1 |
| L3 | Target coords for an `Entity`: `tx = pos.rx*256 + sub_x`, `ty = pos.ry*256 + sub_y`, `tz = effective_z_leptons(target)`. | doc §4.2 | Stage 1 |
| L4 | Target Z is **absolute leptons** (= `cell_level × 104` + altitude + bridge offset). Single source of truth. | doc §4.2 | Stage 1 |
| L5 | Aircraft altitude is added to base Z by `effective_z_leptons` (reads `loco.altitude`). | doc §4.2 | Stage 1 |
| L6 | When `target.IsLowFlying()`, target.Z is **overwritten** with `ground_height_leptons(cell) + (cell_on_bridge ? BRIDGE_HEIGHT_DELTA : 0)`. Altitude is dropped. | doc §4.3 | Stage 1 |
| L7 | `IsLowFlying`/`IsHighFlying` split at `HIGH_FLIGHT_THRESHOLD_LEPTONS`. Mutually exclusive. Initial threshold = 1000 lep (placeholder, citation OQ-5). | doc §4.3 | Stage 1 |

### Boundary semantics

| # | Detail | Source | Stage |
|---|--------|--------|-------|
| L8 | Max range: `dist <= range` **inclusive** (scalar after `isqrt_i64`). | doc §2.2 | Stage 1 |
| L9 | Min range: `dist < MinimumRange` **strict** (scalar after `isqrt_i64`). | doc §2.2 | Stage 1 |
| L10 | Sentinel: `weapon.Range == -512 leptons` (`-0x200`) → return true unconditionally. Checked **before** any other logic. | doc §2 + §7 | Stage 1 |

### Range-VALUE chain

| # | Detail | Source | Stage |
|---|--------|--------|-------|
| L11 | Order: AirRange → Garrison REPLACES → Bunker → OpenTopped → Foundation → Height-fire bonus. | doc §5 | (mixed) |
| L12 | Garrison REPLACES running range; everything else ADDS. | doc §5 item 4 | done elsewhere |
| L13 | AirRange = `attacker.Type.AirRange` raw leptons (no ×256). Triggered by `target.IsHighFlying()`. | doc §5 item 3 | **Stage 1** |
| L14 | Bunker = `Rules.BunkerWeaponRangeBonus × 256`; gated by attacker bunkered AND `attacker.WhatAmI() != 6`. | doc §5 item 5 | Stage 2 |
| L15 | OpenTopped = `Rules.OpenToppedRangeBonus × 256`; gated by attacker open-topped flag. | doc §5 item 6 | Stage 2 |
| L16 | Foundation = `(BuildH + BuildW) × 0x40` leptons. Branch A only, target = Building. NOT applied to MinRange. | doc §5 item 8 | **Stage 1** |
| L17 | Height-fire bonus: gated by `weapon.Projectile.SubjectToElevation`. Computes `(target_cell_height − attacker_cell_height) / Rules.ElevationIncrement` plus a ballistic term. Adds leptons to max range. | doc §0 corrections, §5 items 7+9 | **Stage 1** |

### Z source plumbing

| # | Detail | Source | Stage |
|---|--------|--------|-------|
| L18 | `LEPTONS_PER_LEVEL = 104`. Citation gamemd `0x89DDB8`. | COORDINATE_SYSTEM_GAMEMD.md:127-131 | Stage 1 |
| L19 | `BRIDGE_HEIGHT_DELTA = DAT_00B0EB24` (runtime-init in gamemd, exact value pending). Initial value = `4 × LEPTONS_PER_LEVEL = 416 leptons` matching `BridgeHeight=4` Rules default. | doc §7, OQ-5 | Stage 1 (placeholder + open question) |

### Special-case branches

| # | Detail | Source | Stage |
|---|--------|--------|-------|
| L20 | **Arcing weapons** (`weapon.Projectile.Arcing=yes`) take a 2D path with separate slope arc check. **Stage 1 = early gate → 2D fallthrough** (preserves V3/Prism/etc current behavior). | doc §3.3, §0 corrections | Stage 1 (gate only) / Stage Arcing (full) |
| L21 | `attacker.WhatAmI() == 3` Branch A1 — **proven dead in YR**. Only `*TypeClass` templates inherit; instance attackers are 1/2/6/0xF. **Skip permanently.** | doc §0 corrections | Skip (justified) |
| L22 | **Bridge Z LOS gate**: if attacker_cell_has_bridge_flag AND `attacker.Z < bridge_top` AND `target.Z >= bridge_top` → reject (firing through deck). | doc §0 corrections, §6 | **Stage 1** |
| L23 | `Floater=yes` (`weapon.Projectile.byte+0x295`) — TS-legacy gravity override; no standard YR projectile sets it. | doc §0 corrections | Skip |

### Determinism

| # | Detail | Source | Stage |
|---|--------|--------|-------|
| L24 | State hash changes; replay format bumps once. Documented in design doc + commit message. | user accepted Q2 | Stage 1 |
| L25 | All math integer/SimFixed. No floats. `isqrt_i64` deterministic. | n/a | Stage 1 |

## Design

### Components

```
src/util/lepton.rs                               [+constants]
    pub const LEPTONS_PER_LEVEL: i64 = 104;
    pub const WEAPON_RANGE_ALWAYS_IN_RANGE_LEPTONS: i64 = -512;
    pub const HIGH_FLIGHT_THRESHOLD_LEPTONS: i64 = 1000;  // placeholder
    pub const BRIDGE_HEIGHT_DELTA_LEPTONS: i64 = 416;     // placeholder = 4 levels

src/sim/combat/in_range.rs                       [NEW]
    pub(crate) enum RangeTarget<'a> { Entity(&'a GameEntity), Cell {...} }
    pub(crate) fn compute_in_range(...) -> bool

    // private helpers
    fn is_low_flying(entity) -> bool
    fn is_high_flying(entity) -> bool
    fn effective_z_leptons(entity) -> i64
    fn ground_z_with_bridge_offset(pos, map) -> i64
    fn compute_effective_max_range_leptons(attacker, target, weapon, rules, ...) -> i64
    fn compute_height_fire_bonus_leptons(attacker, target_opt, src, map, rules) -> i64
    fn compute_in_range_arcing_2d(attacker, src, target, weapon) -> bool
    fn cell_has_bridge(rx, ry, map) -> bool

src/sim/combat/mod.rs                            [+1 mod, +1 re-export]
    pub mod in_range;
    pub use self::in_range::{compute_in_range, RangeTarget};
```

### Interfaces / Contracts

```rust
pub(crate) enum RangeTarget<'a> {
    Entity(&'a GameEntity),
    Cell { rx: u16, ry: u16, z_level: u8 },
}

pub(crate) fn compute_in_range(
    attacker: &GameEntity,
    src: (i64, i64, i64),                    // X, Y, Z leptons
    target: RangeTarget<'_>,
    weapon: &Weapon,
    rules: &RuleSet,
    interner: &StringInterner,
    map: &MapData,
) -> bool;
```

**Contract:**
- `src` is caller-built. Standard pattern: `(attacker.x_lep, attacker.y_lep, effective_z_leptons(attacker))`. AntiAircraft cell-snap deferred (Stage Arcing brainstorm).
- For `RangeTarget::Cell`, target Z = `z_level * LEPTONS_PER_LEVEL`. No LowFlying snap, no AirRange, no Foundation. Bridge LOS gate still applies.
- For `RangeTarget::Entity`, full Stage 1 logic applies.
- Return value `true` = target is in firable range; `false` = out of range / blocked by LOS / inside min range.

**Existing helpers stay:**
- `lepton_distance_sq_raw(...)` — kept, unchanged. AOE site keeps using it.
- `is_within_range_leptons(...)` — kept, unchanged. Same.
- New code does NOT call these; uses `isqrt_i64`-based comparisons internally.

### Data Flow

```
caller (e.g. combat_targeting.rs:193)
  │
  ├─ look up attacker_entity from EntityStore (already in scope)
  ├─ build src = (ax_lep, ay_lep, effective_z_leptons(attacker_entity))
  ├─ resolve weapon via rules + selected.weapon
  └─ call compute_in_range(attacker, src, RangeTarget::Entity(candidate), weapon, rules, interner, map)
       │
       ├─ sentinel? → return true
       ├─ arcing? → compute_in_range_arcing_2d (stub: dist² 2D <= range²)
       ├─ effective_max_range = base + AirRange + Foundation + HeightFire
       ├─ resolve target Z (LowFlying snap with bridge offset, or effective_z_leptons)
       ├─ dist_sq = dx² + dy² + dz²
       ├─ if min_range > 0: isqrt(dist_sq) < min_range → false
       ├─ if isqrt(dist_sq) > effective_max_range → false
       ├─ bridge LOS gate (attacker on bridge cell, geometry rejects) → false
       └─ return true
```

### Error Handling

`compute_in_range` is a pure-bool function. No error path. Edge cases:
- Missing locomotor on a non-aircraft entity → `effective_z_leptons` falls through to `pos.z * LEPTONS_PER_LEVEL`. Correct.
- Missing `RuleObject` for attacker (interner resolves to non-rule entry) → AirRange bonus = 0. Defensive, not a panic.
- Missing weapon → caller's responsibility; function takes `&Weapon` (already-resolved reference).
- Negative effective range after bonus chain → impossible in Stage 1 (all bonuses additive non-negative). Stage 2 Garrison-REPLACES could produce 0; clamp to 0 = "never in range" semantics, matches gamemd.

### Determinism

- `SimFixed` arithmetic: deterministic ✓
- `isqrt_i64`: precise integer, deterministic by construction ✓
- `i64` overflow: max delta ≈ 200 cells × 256 lep + 1500 altitude ≈ 53,000 lep. Squared = 2.8e9. Sum of three squares ≈ 8.5e9. i64 max ≈ 9.2e18. Headroom: 9 orders of magnitude. ✓
- No floats anywhere in the function ✓
- BTreeMap iteration order on `EntityStore` unchanged ✓

**Replay format bump** required. The state hash includes targeting decisions; Stage 1
changes the inputs (now reads altitude). Add a one-line version constant bump where
the replay/save format version lives. Document in commit message + any save-load
compatibility table.

### Testing Strategy

**Unit tests in `in_range.rs` (close to the code):**

1. **Sentinel** — `weapon.range = -2 cells` (= -512 leptons) returns `true` regardless of distance.
2. **Boundary inclusive max** — units at exactly `range` cells distance return `true`. Units at `range + 1 lepton` return `false`.
3. **Boundary strict min** — units at exactly `MinimumRange` return `true`. Units at `MinimumRange - 1 lepton` return `false`.
4. **3D vs 2D** — given `dx=dy=0, dz=10 levels = 1040 leptons`, a weapon with `range=4 cells = 1024 leptons` returns `false` (3D-aware). Same input with weapon `range=5 cells` returns `true`. Without our change, both would erroneously return `true`.
5. **LowFlying snap** — Aircraft target with altitude=500 lep on cell level 0 (LowFlying), attacker at cell level 0 at horizontal distance 4 cells. With weapon range 4 cells: returns `true` (snapped to ground; dz = 0). Without snap: 4 cells horizontal + 500 lep vertical → dist > range → false. Test catches a regression in snap logic.
6. **HighFlying AirRange** — Aircraft target at altitude 1500 lep on level 0, attacker at level 0 horizontal distance 6 cells, weapon `range=4 cells, AirRange=512 lep` (= 2 cells). Effective range = 4+2 = 6 cells. dz = 1500. dist_sq = (6×256)² + 1500² = 2,361,856 + 2,250,000 = 4,611,856. dist ≈ 2,148. range_lep = 6 × 256 = 1,536. Returns `false` (out of range even with AirRange — Z penalty dominates). Documents that HighFlying does NOT snap.
7. **Foundation bonus** — building target with `4×2` foundation, attacker at horizontal distance equal to `weapon.range + 0.5 cells`. Bonus = `(4+2)×64 = 384 leptons = 1.5 cells`. Returns `true`. Without bonus: returns `false`.
8. **Sentinel beats min range** — weapon with `range=-2 cells, MinimumRange=10 cells`: returns `true` even at distance 0 (sentinel wins).
9. **Cell target** — `RangeTarget::Cell{rx, ry, z_level: 0}`, attacker on level 5. dz = 5×104 = 520 lep. Test that 3D distance is computed (ground-snap only applies to entities, not cells) and that AirRange / Foundation bonuses do NOT apply.
10. **Arcing fallthrough** — weapon with `arcing=true`, attacker and target with large dz. Function should return same result as the existing 2D `lepton_distance_sq_raw` + `is_within_range_leptons` — i.e. dz is ignored for arcing weapons (Stage 1 stub behavior).

**Integration tests:**

11. Existing combat integration tests in `src/sim/combat/*` should continue to pass with no behavioral change for ground-vs-ground engagements at level 0 (most existing tests). Aircraft-vs-ground tests will see the new behavior.
12. **State-hash test** — confirm two simulation runs with same seed produce identical state hashes after the change (determinism preserved). Run before merging.

**Manual verification (post-implementation):**

13. Skirmish: spawn a Kirov at cruise altitude over an SAM site. SAM should engage at expected range matching gamemd (within ±1 cell tolerance). Compare with current 2D behavior — Kirov was previously gettable from anywhere within 2D range; now it should track gamemd.
14. Skirmish: cliff scenario. Tank atop cliff vs. tank below. Attacker on high ground should fire at slightly extended range (height-fire bonus). Without this Stage 1 item, the bonus is missing — verify the bonus is now present.
15. Skirmish: under-bridge. Tank under bridge fires up at infantry on deck — should be blocked by bridge LOS gate.

## Architectural Decisions

**Pattern followed:**
- Module split (`combat/in_range.rs` separate file) follows the existing pattern of subsystem-per-file in combat/.
- `pub(crate)` visibility matches existing combat helpers.
- Re-export from `combat/mod.rs` matches the pattern used for `combat_targeting`, `combat_aoe`.
- Integer-only math, deterministic by construction, matches the simulation's Cargo-approved-crates rule (no floats in sim).

**Patterns deviated from / new conventions:**
- **`RangeTarget` enum** is new — no existing combat code uses target-shape polymorphism this way. Justified: gamemd's InRange takes an AbstractClass* that can be Entity or coord-derived; the enum makes the two cases explicit and forces callers to declare what they're firing at.
- **Caller-built `src`** parameter is new. Existing combat helpers read everything from snapshots. This deviation matches gamemd's explicit caller-builds-source contract and gives Stage Arcing a clean place to inject AntiAircraft cell-snap.
- **`isqrt_i64`-based scalar comparison** instead of squared-leptons comparison. Existing 2D code uses squared. New 3D code uses scalar. Justified: matches gamemd's `(int)sqrt(...) <= range` semantics within 0–1 lepton; squared comparison can diverge by 1 lepton at exact boundaries. Documented in the doc as a parity tightening.

**Tech debt introduced:**
- `HIGH_FLIGHT_THRESHOLD_LEPTONS` and `BRIDGE_HEIGHT_DELTA_LEPTONS` are placeholder constants pending OQ-5 in the research doc. Marked TODO with citation; refinable in one place.
- Height-fire bonus formula: ballistic-term details need a 5-minute Ghidra re-read of `FUN_006F6F60` before implementation. Naive Stage 1 formula = `dh_levels × LEPTONS_PER_LEVEL` (the linear part); the ballistic-distance-and-height term will be confirmed in the implementation phase. This is a known pre-implementation prerequisite, not a brainstorm blocker.

**Plan to address tech debt:**
- OQ-5 resolution scheduled for whoever next inspects rules.ini parsers; pinned to a single constant + comment in `util/lepton.rs`.
- Ballistic-term Ghidra re-read becomes Step 1 of the `/write-plan` task.

## Alternatives Considered

**Approach A — "Surgical": extend existing helpers in place.** Add `lepton_distance_sq_3d_raw` next to `lepton_distance_sq_raw`, inline AirRange/LowFlying/etc at each call site. Rejected: Stage 2 (Bunker/OpenTopped/Veteran) would require re-touching all 4 sites again. Doesn't model gamemd's single-InRange-function architecture. Diff size win not worth the maintenance hit.

**Approach C — "Layered util": low-level 3D distance in `util/lepton.rs` + sim-level wrapper.** Same shape as Approach B but split into two modules. Rejected: the "reusable util" benefit is speculative — the 3D distance has no consumer outside InRange currently (AOE explicitly stays 2D). Premature generalization. Revisit when a second consumer appears.

**Position struct change — `pos.z: u8` → `pos.z_leptons: i32`.** Rejected for Stage 1: large refactor, touches render/UI/sim, high risk of breaking unrelated code during a "Stage 1 fix". `effective_z_leptons` helper computes per call instead — cheap, low-risk. Revisit if profiling shows the helper is hot.

**Cached `z_leptons` on Position.** Rejected for Stage 1: cache coherency is a foot-gun; every place updating `pos.z` or `loco.altitude` would need to also update the cache. Helper computation per call is simpler and correct by construction.

**Squared-leptons comparison instead of `isqrt_i64`.** Considered as "matches existing pattern, less code". Rejected: produces ±1 lepton drift from gamemd at exact range boundaries. The existing 2D pattern already has this drift; the 3D rewrite is the right place to fix it. AOE site stays squared (out of scope).

**Promote Stage 2 items (Bunker/OpenTopped/Veteran) into Stage 1.** Considered as "max parity in one PR". Rejected as currently scoped: those items need attacker-state research the Stage 1 ledger doesn't carry. Stage 2 brainstorm should pick them up after Stage 1 lands.
