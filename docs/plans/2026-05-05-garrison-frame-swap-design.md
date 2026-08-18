# Garrison Frame Swap Design

## Goal

Make `CanBeOccupied=yes` building bodies render the correct SHP frame based on
occupant count and damage tier, matching gamemd.exe's `BuildingClass::GetCurrentFrame`
branch 4 formula.

## Architecture Context

Building body frame selection currently lives in [src/app_instances/shp.rs:138-139](../../src/app_instances/shp.rs#L138-L139)
as a hardcoded stub: `EntityCategory::Structure => (0, None)`. The atlas builder at
[src/render/sprite_atlas.rs:285-290](../../src/render/sprite_atlas.rs#L285-L290)
registers only `frame: 0` for every Structure entity.

The supporting data is already in place:
- `ObjectType.can_be_occupied: bool` — [src/rules/object_type.rs:842](../../src/rules/object_type.rs#L842)
- `ObjectType.tech_level: i32` (default -1) — [src/rules/object_type.rs:693](../../src/rules/object_type.rs#L693)
- `RuleSet.general.condition_yellow: f32` (0.5) and `condition_red: f32` (0.25) — [src/rules/ruleset.rs:597-632](../../src/rules/ruleset.rs#L597-L632)
- `entity.health: Health { current: u16, max: u16 }` — [src/sim/components.rs:87-92](../../src/sim/components.rs#L87-L92)
- `entity.passenger_role.cargo().map(|c| c.count())` — occupant count, used by
  garrison combat at [src/sim/combat/mod.rs:693+](../../src/sim/combat/mod.rs#L693)

Architectural rule from CLAUDE.md: `sim/` does not depend on `render/`. Frame index
is render-side; no sim changes needed.

## Impact Analysis

**Files modified:**
- [src/app_instances/shp.rs](../../src/app_instances/shp.rs) — replace the `Structure → (0, None)` stub with a helper call; add a fallback for missing atlas entries on garrisonable buildings.
- [src/render/sprite_atlas.rs](../../src/render/sprite_atlas.rs) — register frames 1, 2, 3 in addition to frame 0 for `can_be_occupied=true` Structure entities.

**Reads only:** `entity.health`, `entity.passenger_role`, `entity.type_ref`, rules.

**No sim changes.** Determinism contract preserved; sim state hash unaffected.

**Risk areas:**
- Atlas size growth: ~95 garrisonable types × up to 4 frames × house-color variants. Modest.
- SHPs with fewer than 4 frames: the atlas builder will skip the missing ones; the renderer falls back to frame 0 (Approach A — see below).
- Owner-remap on garrison already works through the existing `house_color_map` path; no extra work.

## Chosen Approach

**Approach A — register frames 0..3 for `can_be_occupied=true` buildings; fall back to frame 0 in the renderer if a requested frame is missing from the atlas.**

Picked over alternatives because all standard YR civilian garrisonable SHPs (CABHUT,
CALA01-10, CAGAS01, CABNK01-04, etc.) ship with ≥4 frames designed for this exact
purpose. Probing SHP frame counts at atlas-build time (Approach B) adds complexity
for no real-world benefit. Warning logs for missing fallbacks (Approach C) would be
noise on map-mod content; the explicit per-garrisonable retry in the renderer is
sufficient.

## Design

### Components

**1. Helper function `building_frame_index`** — module-private fn in
[src/app_instances/shp.rs](../../src/app_instances/shp.rs), pure, no I/O,
fully unit-testable.

Signature:

```rust
fn building_frame_index(
    occupant_count: u8,
    health_current: u16,
    health_max: u16,
    tech_level: i32,
    condition_yellow: f32,
    condition_red: f32,
) -> u16
```

Body:

```rust
let mut base: u16 = 0;
if occupant_count > 0 {
    base = 2;
}
let ratio = if health_max == 0 {
    1.0
} else {
    health_current as f32 / health_max as f32
};
let red_tier = ratio <= condition_red;
let yellow_tier = tech_level > 0 && ratio <= condition_yellow;
if red_tier || yellow_tier {
    base += 1;
}
if tech_level == -1 && base == 3 {
    return 1;
}
base
```

**2. Atlas registration patch** — extends the `EntityCategory::Structure` arm at
[src/render/sprite_atlas.rs:285](../../src/render/sprite_atlas.rs#L285) to register
frames 1-3 for buildings with `CanBeOccupied=yes`.

**3. Render-side frame selection** — replaces the stub at
[src/app_instances/shp.rs:138](../../src/app_instances/shp.rs#L138), calling the
helper with sim-derived inputs.

**4. Atlas-lookup fallback** — at the existing
`atlas.get(&key)` site (around [shp.rs:157-159](../../src/app_instances/shp.rs#L157-L159)),
when the lookup fails for a `can_be_occupied=true` Structure with `frame != 0`,
retry with `frame: 0`. Non-garrisonable buildings keep their existing skip-and-warn
behavior.

### Interfaces / Contracts

The helper is private, has no callers outside its file, and returns a u16 frame
index in `[0, 3]`. Inputs are all owned primitives — no entity references, no
rule-set references — which keeps the function trivial to unit-test.

The atlas registration emits standard `ShpSpriteKey { type_id, facing: 0, frame, house_color }`
entries; consumers downstream are unchanged.

### Data Flow

```
   sim state (per-tick)
   ├─ entity.health
   ├─ entity.passenger_role.cargo().count()
   └─ entity.type_ref ─→ rules ─→ tech_level + can_be_occupied
                                       │
   rules.general ─→ condition_yellow, condition_red
                                       │
                                       ▼
              building_frame_index(...) ─→ u16 frame
                                       │
                                       ▼
   ShpSpriteKey { type_id, facing: 0, frame, house_color }
                                       │
                                       ▼
                        atlas.get(&key)
                       /              \
                  Some(entry)      None (frame != 0 + can_be_occupied)
                      │                       │
                draw normally          retry with frame 0
```

### Error Handling

- `entity.health.max == 0` → ratio = 1.0 → frame 0 (treated as "healthy"). Rare
  edge case (entity not yet fully initialized); avoiding division-by-zero is the
  only requirement.
- `rules.object(type_str)` returns None → treat as `can_be_occupied=false, tech_level=-1` →
  fall through to frame 0. Same behavior as today's stub for unknown types.
- Missing atlas entry for a garrisonable building's frame 1/2/3 → fall back to
  frame 0 (Approach A). For non-garrisonable buildings, existing behavior preserved.

### Testing Strategy

Unit tests in `app_instances/shp.rs` cover the formula's truth table:

| Test | tech_level | occupants | health_ratio | expected |
|---|---|---|---|---|
| civilian, empty, healthy | -1 | 0 | 1.0 | 0 |
| civilian, empty, yellow tier | -1 | 0 | 0.4 | 0 |
| civilian, empty, red tier | -1 | 0 | 0.2 | 1 |
| civilian, occupied, healthy | -1 | 1 | 1.0 | 2 |
| civilian, occupied, yellow tier | -1 | 1 | 0.4 | 2 |
| civilian, occupied, red tier (collapse) | -1 | 1 | 0.2 | 1 |
| buildable, empty, healthy | 5 | 0 | 1.0 | 0 |
| buildable, empty, yellow tier | 5 | 0 | 0.4 | 1 |
| buildable, occupied, healthy | 5 | 1 | 1.0 | 2 |
| buildable, occupied, red tier | 5 | 1 | 0.2 | 3 |
| edge, max == 0 | -1 | 0 | n/a | 0 |

No integration tests required for this change — the renderer/atlas paths are
exercised by every skirmish; visual verification is the acceptance gate.

## Architectural Decisions

- **No new abstractions.** Helper sits next to existing `resolve_infantry_shp_frame`
  in the same file.
- **Render-side floats.** Sim uses integer health comparisons via `condition_red_x1000`
  (precedent: [src/sim/passenger.rs:213-217](../../src/sim/passenger.rs#L213-L217));
  render is allowed floats per CLAUDE.md.
- **Atlas grows for `can_be_occupied` only.** Avoids registering unused frames for
  non-garrison buildings.
- **Fallback retry instead of warn-log.** SHPs vary in frame count for legitimate
  reasons (mod content); a deduped warn-log would be noise. Per-garrisonable retry
  to frame 0 keeps the failure mode "draws as empty" instead of "disappears."

## Alternatives Considered

- **Approach B (probe SHP frame count at atlas-build time)** — rejected as
  unnecessary complexity. Standard YR civilian garrisonable SHPs all have ≥4
  frames by design; probing adds I/O cost during atlas build for no observable
  gain.
- **Approach C (warn-log on missing fallback)** — rejected because mod content
  may legitimately use fewer frames. Approach A's silent retry to frame 0 covers
  the same scenarios without log noise.
- **Gate the formula on `health_ratio <= ConditionYellow`** (matching the binary's
  `+0x534 != 0` BState gate literally) — rejected. Civilian map-placed buildings
  almost certainly stay at the constructor's `+0x534 = -1` initial value, meaning
  the binary's branch 4 fires unconditionally for them in practice. Unconditional
  application matches expected player-visible behavior; if observation shows otherwise,
  the fix is a one-line wrap.
- **Place the helper in `RuleSet` or `ObjectType`** — rejected as misplacement.
  The result is a render-side concern (frame index for SHP atlas lookup); putting
  it on rules would create a render-shaped API on a sim-adjacent type.

## Out of scope

These systems are explicitly deferred:
- Damaged-frame swap for non-CanBeOccupied buildings (general Anim1..Anim4 max formula, branch 6 of GetCurrentFrame).
- Gate, LaserFence, FirestormWall frame paths (branches 1, 2, 5).
- Selling-decay frame formula (branch 3b).
- Anim-overlay swap system (`FUN_00458330` and the slot-by-slot occupied/healthy/damaged variant table).
- Fog-of-war snapshot frame caching.
