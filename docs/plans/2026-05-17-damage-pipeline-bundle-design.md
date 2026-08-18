# Damage Pipeline Bundle — AffectsAllies + Suicide + V3 Airburst Design

## Goal

Close three independent damage-pipeline parity gaps in one bundle: (1) wire the
`AffectsAllies` warhead flag so splash damage stops hitting allied units by default,
(2) make `Suicide=yes` weapons detonate at the firer's position so Demo Truck /
IFV-Ivan self-destruct, (3) spawn 9-cell AoE damage from V3 Airburst at impact so
the V3 cluster strike deals its full damage profile.

## Architecture Context

The combat damage pipeline in `src/sim/combat/` is fully **instant-at-fire-time**.
`tick_combat` in [combat/mod.rs](src/sim/combat/mod.rs) iterates entities ready to
fire; after fire-gate / facing / animation checks, it computes either:

- A splash hit via `apply_aoe_damage` ([combat/combat_aoe.rs:48](src/sim/combat/combat_aoe.rs#L48))
  returning `Vec<(target_id, damage)>` per target inside the warhead's CellSpread.
- A direct-hit single-target damage at [combat/mod.rs:1841](src/sim/combat/mod.rs#L1841)
  with `base_damage * verses_pct / 100`.

Damage is queued into `damage_events: Vec<(target_id, dmg, attacker_id, wh_iid)>`
and applied to entity HP later in the same tick. There is no in-flight projectile
delivering damage — `RocketState` and `HomingState` infrastructure exists in
[movement/rocket_movement.rs](src/sim/movement/rocket_movement.rs) and
[movement/homing_movement.rs](src/sim/movement/homing_movement.rs) but no production
code spawns them; their detonation lists are discarded at
[world/mod.rs:1099-1105](src/sim/world/mod.rs#L1099-L1105).

The `apply_aoe_damage` function accepts `_attacker_owner: &str` — currently
**unused** (leading underscore). The infrastructure for an owner-aware gate is
already plumbed; it just isn't read.

The current comment at [combat_aoe.rs:43-45](src/sim/combat/combat_aoe.rs#L43-L45)
claims "Friendly fire IS applied — CellSpread does not discriminate by owner,
matching RA2 behavior (e.g., V3 rockets can damage your own units)." Per the
GREEN-verified `friendly_fire.md` doc, this comment is **wrong about gamemd**:
the receiver-side `TechnoClass::ReceiveDamage` gate at `0x00701900` zeros damage
for `AffectsAllies=no` warheads when attacker and target are allied. This is the
default for virtually every shipping warhead (AP, GUN, tank cannons, etc.).

The combat module has its own house-alliance helper available: the existing
`HouseAllianceMap` + `crate::map::houses::are_houses_friendly(alliances, a, b)`
helper used by [bump_crush.rs:162](src/sim/movement/bump_crush.rs#L162) and
[miner_system.rs:921](src/sim/miner/miner_system.rs#L921).

For Suicide and Airburst, the parsers already populate the relevant struct
fields ([weapon_type.rs](src/rules/weapon_type.rs): `weapon.suicide`,
[projectile_type.rs](src/rules/projectile_type.rs): `projectile.airburst`,
`projectile.airburst_weapon`) — zero consumers in `sim/` today.

## Impact Analysis

**Files touched (production):**
- `src/rules/warhead_type.rs` — add `affects_allies: bool` field, parse
  `AffectsAllies=` (default `false`).
- `src/sim/combat/combat_aoe.rs` — add ally-gate inside per-target damage
  computation; remove misleading comment; add `apply_airburst_spawn` helper.
- `src/sim/combat/mod.rs` — add Suicide self-target redirect at fire-time
  (just before `apply_aoe_damage` call); add Airburst dispatch right after
  the primary `apply_aoe_damage`; add ally-gate in the direct-hit damage path.

**Files touched (tests):**
- `src/rules/warhead_type.rs` (test module) — `AffectsAllies` parse default + override tests.
- `src/sim/combat/combat_tests.rs` — friendly-fire zero (default), friendly-fire
  pass (`AffectsAllies=yes`), Suicide Demo Truck self-damages at own cell, V3
  Airburst delivers damage to 9 cells.

**What might break:**
- **Existing combat tests asserting friendly splash damage.** The default flip
  on `AffectsAllies` will zero splash damage to allies. Tests that validated
  the wrong-current behavior (e.g., `v3_splash_friendly_collateral` style
  tests, if any exist) need updating. Grep for `same_owner.*damage` /
  `friendly.*splash` patterns during implementation. **Required regression
  check:** run full `sim::combat` suite after gate lands.
- **Aircraft splash on own units.** Flying ground unit's splash on own
  aircraft was probably damaging them. After fix, no damage. Visible
  regression watch.
- **Tests for V3** at [combat_tests.rs:1488](src/sim/combat/combat_tests.rs#L1488)
  use a V3 placeholder with `[V3WH] CellSpread=1` (no `Airburst=yes`).
  Airburst dispatch won't fire for those tests — they should continue passing
  unchanged. **Confirm during implementation.**

**Determinism:**
- AffectsAllies gate: boolean check on interned owner IDs + alliance map
  lookup. Lockstep-safe.
- Suicide redirect: deterministic coord overwrite (attacker position).
- Airburst 9-cell loop: fixed direction order
  `[(0,-1), (1,-1), (1,0), (1,1), (0,1), (-1,1), (-1,0), (-1,-1)]`
  (N → NE → E → SE → S → SW → W → NW), matching gamemd's
  `Pathfinding_update_continued(dir 0..7)`. Each iteration calls
  `apply_aoe_damage` whose internal iteration order over occupancy is
  unchanged.
- No new state on `GameEntity`. No tick-ordering change. No state-hash field
  additions.

**Blast radius low.** The three changes sit at three well-isolated points in
the central damage path. No cross-module surface area change.

## Chosen Approach

**Approach A: three independent small changes, one combined design doc.**

Each subsystem gets its own implementation slot. No shared infrastructure
between them. Bundle keeps related fixes together (~150 lines of production
code + tests). Staged commits per subsystem (3 commits) give granular revert.

Rejected alternatives:
- **Approach B (three separate brainstorm/plan cycles):** 3× the design-doc
  overhead for the same total scope. Loses the bundle's shared test-fixture
  reuse.
- **Approach C (pick one — only AffectsAllies):** leaves Demo Truck silent and
  V3 single-impact. Both are visible parity gaps named in the gap-scan; cutting
  them on grounds of "smaller scope" is convenience-disguised parity drift.

## Tiny-Detail Ledger

24 parity-relevant items. Items #7, #21 deferred per user-accepted scope; item
#12 accepted as known drift. Each implementation site below cites its ledger
items.

### AffectsAllies

| # | Detail | Source |
|---|---|---|
| 1 | `AffectsAllies` field at `wh+0x179`, default `false`, parsed at `WarheadTypeClass::ReadINI 0x0075DD80` | [doc: friendly_fire.md §1; GHIDRA string `0x00847CC8` → ReadINI `0x0075d9df`] |
| 2 | Gate is **receiver-side** in `TechnoClass::ReceiveDamage 0x00701900`; runs per-target after AoE collection, not at target-collection time. Animations / sounds still play; only damage is zeroed | [doc: friendly_fire.md §2-3] |
| 3 | Check: `sourceHouse != NULL && IsAlliedWith(sourceHouse.Owner, target.Owner)` → damage = 0. Self is always considered allied | [doc: friendly_fire.md §2 verbatim decomp; §9 edge case "Self"] |
| 4 | Ambient damage (sourceHouse == NULL: death-weapon, anim damage, scripted triggers) **bypasses** the gate — applies to everyone | [doc: friendly_fire.md §9 edge case 4] |
| 5 | ForceFire (Ctrl-click on ally) does NOT bypass the gate: weapon fires, animation plays, damage = 0 | [doc: friendly_fire.md §5] |
| 6 | Cell-side warhead effects (`Wall=`, `Tiberium=`, `WallAbsoluteDestroyer=`) are NOT ally-gated — friendly tank shell still destroys overlays in radius | [doc: friendly_fire.md §4 sub-section] |
| 7 | Psychedelic warhead has a SECOND independent ally gate (`wh+0x16D`); layers with AffectsAllies. **Deferred — Psychedelic not implemented yet.** | [doc: friendly_fire.md §7] |

### Suicide

| # | Detail | Source |
|---|---|---|
| 8 | `Suicide` flag at `weapon+0x144`, parsed at `WeaponTypeClass::ReadINI 0x0077228D` | [doc: suicide_weapons.md §1] |
| 9 | Mechanism: in `Fire_At`, set target to self → projectile lands at firer position → warhead detonates at firer. C4Warhead self-target gate bypassed by Suicide branch | [doc: suicide_weapons.md §3.1 + `FIRE_AT_PIPELINE_GHIDRA_REPORT.md`] |
| 10 | Retail Suicide weapons: `Demobomb` (Demo Truck), `IvanBomb` (Ivan place), `CRIvanBomb` (IFV-Ivan), `CRNuke` (IFV-Ivan special) | [doc: suicide_weapons.md §2] |
| 11 | Demo Truck composition: Suicide + `DeathWeapon=Demobomb` → Suicide fires Demobomb at self (self-dies) → existing `death_weapon_aoe` path fires Demobomb again at death position → **TWO detonations** in nearly the same tick. UNKNOWN: whether engine gates double-fire (open follow-up #3 in doc) | [doc: suicide_weapons.md §4; UNKNOWN — open follow-up #3] |
| 12 | IvanBomb edge case: Crazy Ivan doesn't die from placing a bomb in gamemd despite `Suicide=yes`. Interaction with IvanBomb warhead cascade (`wh+0x157`) is the suspected gate but unverified. **User accepted: Ivan will self-detonate in our impl as known drift** | [doc: suicide_weapons.md §3.2; UNKNOWN — open follow-up #2] |

### V3 Airburst (simplified — damage only, no visual sub-bullets)

| # | Detail | Source |
|---|---|---|
| 13 | `Airburst` flag at `BulletType+0x294`, `AirburstWeapon` ptr at `+0x2B0`, both parsed in `BulletTypeClass::ReadINI` | [doc: airburst.md §3] |
| 14 | Spawn block at end of `WarheadTypeClass::Detonate` — primary detonate (`Apply_area_damage` + anim) runs FIRST, then 9-bullet spawn runs | [doc: airburst.md §5, §9 step 6-7] |
| 15 | Exactly **9** sub-bullets — hardcoded `counter=8` loop + 1 explicit spawn at impact cell. Not INI-driven | [doc: airburst.md §5, §12 constants] |
| 16 | 8-loop targets neighbor cells via `Pathfinding_update_continued(dir 0..7)`; direction order follows `g_DirectionOffsets` (N/NE/E/SE/S/SW/W/NW) | [doc: airburst.md §5-6] |
| 17 | 9th sub-bullet targets the impact cell itself (`GetOccupiedCell()`); after the 8 loop iterations | [doc: airburst.md §5-6] |
| 18 | Each sub-bullet carries `AirburstWeapon.Damage` **unscaled** — no division by 9, no falloff at spawn. Per-cell warhead falloff still applies normally inside each sub-`apply_aoe_damage` | [doc: airburst.md §5 "Damage scaling? None"; §8 §14] |
| 19 | Sub-bullet warhead = `AirburstWeapon.Warhead`. Independent from primary warhead | [doc: airburst.md §3; §9 step 9] |
| 20 | `Cluster=N` field on a bullet with `Airburst=yes` is **dead** — gated off (Airburst is the alt branch). Rust must not double-apply both | [doc: airburst.md §2, §4] |
| 21 | `[ClusterBits]` (V3's sub-bullet type) has `ROT=60` — homing. **Deferred-visual:** in our simplified scope, sub-bullet flight is not modeled. Damage applies instantly at each of 9 cells. Visual parity drift documented as follow-up blocked on projectile→damage pipeline | [doc: airburst.md §9 INI] |
| 22 | If primary impact cell is on map edge: 8-neighbor lookup may yield out-of-bounds cells. For Rust: simply skip out-of-bounds cells (no `apply_aoe_damage` at invalid coords) | [doc: airburst.md §14 edge case] |
| 23 | Recursive airburst (sub-bullet's projectile has `Airburst=yes`) is unbounded in gamemd — modder risk. Stock YR doesn't do it. For Rust: guard against recursive spawn with a single-level limit | [doc: airburst.md §14 edge case 3] |
| 24 | `AffectsAllies` applies per sub-bullet detonation — Airburst + AffectsAllies compose correctly because each sub-cell calls `apply_aoe_damage` which now includes the gate | [doc: airburst.md §14 edge case + friendly_fire.md §4] |

## Design

### Components

Three independent slots in the existing combat pipeline. No new modules, no new
entity state.

**1. AffectsAllies field on WarheadType:**

```rust
// src/rules/warhead_type.rs — add to WarheadType struct
pub affects_allies: bool,  // default false per gamemd wh+0x179

// in parser
affects_allies: section.get_bool("AffectsAllies").unwrap_or(false),
```

**2. AffectsAllies gate inside `push_entity_aoe_damage`:**

The function already receives the attacker owner as `_attacker_owner: &str` — drop
the underscore and use it. New gate:

```rust
// Pseudocode — inside push_entity_aoe_damage in combat_aoe.rs
if !warhead.affects_allies
    && !attacker_owner.is_empty()                          // ambient damage (no firer) bypasses (ledger #4)
    && are_houses_friendly(alliances, attacker_owner, target_owner_str)
{
    return;  // skip pushing to damage_list — friendly target, AffectsAllies=no
}
```

The `alliances: &HouseAllianceMap` will need to be plumbed through from
`tick_combat` (which has access via `sim.house_alliances`). Same pattern as
`tick_combat`'s existing usage in retaliation paths.

**3. AffectsAllies gate in direct-hit damage (combat/mod.rs:1841):**

Same gate, mirrored. After computing `actual_damage`, before pushing to
`damage_events`:

```rust
if !warhead.affects_allies && /* attacker has owner */ && /* target is allied */ {
    // zero the damage; do not push to damage_events
    continue;  // or skip push
}
```

The direct-hit path knows `snap.owner` (attacker owner ID) and the target entity
ID — same alliance check.

**4. Suicide self-target redirect in `tick_combat`:**

Right before the damage application block at combat/mod.rs ~1786, after weapon
selection but before any `apply_aoe_damage` / direct-hit dispatch:

```rust
// Suicide weapon: redirect target to attacker's own cell (ledger #8, #9, #10).
// The warhead then detonates at the firer's position; firer dies in own splash.
// Composition with DeathWeapon (Demo Truck pattern, ledger #11): existing
// death_weapon_aoe runs after HP reaches 0 — TWO detonations naturally.
let (target_rx, target_ry, target_sub_x, target_sub_y) = if weapon.suicide {
    (snap.position.rx, snap.position.ry, snap.position.sub_x, snap.position.sub_y)
} else {
    (target_rx, target_ry, target_sub_x, target_sub_y)
};
```

Suicide override happens BEFORE the AffectsAllies gate runs. The firer hits
itself with `IsAlliedWith(self, self) == true`. To make damage actually land on
the firer, the AffectsAllies gate needs to skip when target == attacker
self-hit AND warhead has the C4Warhead self-target intent OR when the
suicide-redirected hit is on the firer itself.

**Wait — that's a subtle interaction.** Per ledger #3: `IsAlliedWith(self,
self) == true`. Per ledger #9: "C4Warhead self-target gate bypassed by Suicide
branch." So in gamemd, Suicide bypasses the ally gate for the self-hit. We need
the same: when Suicide redirects the target, the AffectsAllies gate must NOT
block damage to the firer.

**Cleanest fix:** the Suicide redirect makes `attacker_owner == target_owner`
trivially true. We need a `bypass_ally_gate: bool` flag passed down, set true
when `weapon.suicide`. Or: gate doesn't apply when `target_id == attacker_id`
AND the weapon was suicide-targeted. The simplest signal: the redirect itself
implies bypass.

**Decision:** pass a `is_suicide: bool` flag into `apply_aoe_damage` (and the
direct-hit path). When `is_suicide`, the AffectsAllies gate skips the
attacker-self target only. Other targets in radius still get the gate.

**5. Airburst dispatch right after primary `apply_aoe_damage`:**

At combat/mod.rs ~line 1813 (after the existing `apply_aoe_damage` returns its
hit list), check if the projectile has Airburst:

```rust
// Look up projectile from weapon
let projectile = weapon.projectile.as_deref()
    .and_then(|p_id| rules.projectile(p_id));
if let Some(proj) = projectile {
    if proj.airburst {
        if let Some(ab_weapon_id) = proj.airburst_weapon.as_deref() {
            if let Some(ab_weapon) = rules.weapon(ab_weapon_id) {
                let ab_warhead = rules.warhead(ab_weapon.warhead.as_deref().unwrap_or(""));
                if let Some(ab_wh) = ab_warhead {
                    // Spawn 9 AoE detonations: 8 neighbors + impact cell
                    let airburst_hits = apply_airburst_spawn(
                        entities,
                        target_rx, target_ry,
                        ab_weapon.damage,
                        ab_wh,
                        rules,
                        interner,
                        attacker_owner_str,
                        alliances,
                        layer_context,
                    );
                    for (target_id, dmg) in airburst_hits {
                        damage_events.push((target_id, dmg, snap.stable_id, ab_wh_iid));
                    }
                }
            }
        }
    }
}
```

**6. `apply_airburst_spawn` helper in combat_aoe.rs:**

```rust
// Pseudocode
pub(crate) fn apply_airburst_spawn(
    entities: &EntityStore,
    impact_rx: u16,
    impact_ry: u16,
    base_damage: i32,
    warhead: &WarheadType,
    rules: &RuleSet,
    interner: &StringInterner,
    attacker_owner: &str,
    alliances: &HouseAllianceMap,
    layer_context: AoELayerContext<'_>,
) -> Vec<(u64, u16)> {
    // 8 directions matching gamemd's g_DirectionOffsets order (N/NE/E/SE/S/SW/W/NW)
    const NEIGHBORS: [(i32, i32); 8] = [
        (0, -1), (1, -1), (1, 0), (1, 1),
        (0, 1), (-1, 1), (-1, 0), (-1, -1),
    ];
    let mut all_hits: Vec<(u64, u16)> = Vec::new();
    // 8 neighbor cells + impact cell
    let mut cells: Vec<(u16, u16)> = Vec::with_capacity(9);
    for (dx, dy) in NEIGHBORS.iter() {
        let nx = impact_rx as i32 + dx;
        let ny = impact_ry as i32 + dy;
        if nx >= 0 && ny >= 0 && nx <= u16::MAX as i32 && ny <= u16::MAX as i32 {
            cells.push((nx as u16, ny as u16));
        }
    }
    cells.push((impact_rx, impact_ry));  // 9th: impact cell itself

    for (cx, cy) in cells {
        let hits = apply_aoe_damage(
            entities, cx, cy, base_damage, warhead, rules, interner,
            attacker_owner, alliances, layer_context,
            /* is_suicide = */ false,
            /* is_airburst_recurse_guard = */ true,  // ledger #23
        );
        all_hits.extend(hits);
    }
    all_hits
}
```

The `is_airburst_recurse_guard: true` prevents sub-bullet `apply_aoe_damage`
from itself triggering Airburst (the sub-warhead's projectile shouldn't recurse).
Actually — the recursion guard belongs at the CALLER side (combat/mod.rs), since
`apply_aoe_damage` doesn't check Airburst itself. So the guard is: don't run the
Airburst dispatch block at line ~1813 when we're already inside an Airburst
sub-spawn. Implementation: track a per-tick recurse flag, OR simply don't call
the Airburst dispatch from inside `apply_airburst_spawn` (since
`apply_aoe_damage` doesn't know about projectiles, the recursion can't happen
unless we explicitly do it — easy to avoid).

**Simpler:** `apply_airburst_spawn` calls `apply_aoe_damage` directly (which is
warhead-only, no projectile lookup). The Airburst dispatch only runs at the
PRIMARY fire-time block in `tick_combat`. So recursion is impossible by
construction; no flag needed.

### Interfaces / Contracts

- `WarheadType` gains one field: `affects_allies: bool`. Default `false`.
- `apply_aoe_damage` signature gains two parameters: `alliances:
  &HouseAllianceMap` and `is_suicide: bool`. Existing callers update accordingly.
- New module-private function `apply_airburst_spawn` in `combat_aoe.rs`.
- No public API additions beyond `affects_allies` and the new helper.

### Data Flow

```
tick_combat
├─ for each entity ready to fire:
│  ├─ select weapon + warhead
│  ├─ [NEW] if weapon.suicide: override target to attacker's own position
│  ├─ apply_aoe_damage(... alliances, is_suicide=weapon.suicide)
│  │   └─ for each target in CellSpread:
│  │       └─ [NEW] AffectsAllies gate (skip if !affects_allies && allied && !suicide-self)
│  ├─ [NEW] if projectile.airburst && airburst_weapon set:
│  │   └─ apply_airburst_spawn at 9 cells
│  │       └─ each cell: apply_aoe_damage (uses sub-warhead's AffectsAllies independently)
│  └─ direct-hit path (warhead.cell_spread == 0):
│      └─ [NEW] AffectsAllies gate before push to damage_events
└─ damage_events → HP application later in tick
```

### Error Handling

- Missing `AirburstWeapon` referenced by `airburst_weapon: Some("V3Cluster")`
  but no `[V3Cluster]` weapon in rules → skip Airburst dispatch (no panic, no
  default damage). Log warning if helpful; not required.
- Missing warhead on `AirburstWeapon` → same: skip dispatch.
- Suicide weapon with no warhead → existing path already handles missing
  warhead via early-return in damage block; no new error case.
- AffectsAllies on a warhead missing from rules → not possible (warhead lookup
  happens before this gate).

### Testing Strategy

Unit tests in `combat_tests.rs`:

**AffectsAllies:**
1. `friendly_splash_zero_with_affects_allies_no` — V3-style splash, attacker and
   target same owner, default `AffectsAllies=false` → target HP unchanged.
2. `friendly_splash_passes_with_affects_allies_yes` — same setup but
   `AffectsAllies=yes` → target HP reduced.
3. `enemy_splash_normal_with_affects_allies_no` — same setup, target different
   owner → damage applied (unchanged behavior).
4. `direct_hit_friendly_zero_with_affects_allies_no` — direct-hit on ally,
   `AffectsAllies=no` → HP unchanged.
5. `force_fire_friendly_zero_with_affects_allies_no` — explicit ForceFire on
   ally, weapon fires, damage = 0 (ledger #5).
6. `ambient_damage_skips_ally_gate` — damage with no attacker (e.g., death
   weapon dispatch) → applies regardless of alliance (ledger #4). **Note:**
   may not be directly testable until death-weapon attacker attribution is
   reviewed — add as a doc-comment if not testable now.

**Suicide:**
7. `suicide_weapon_damages_firer` — entity with Suicide weapon fires at distant
   target; damage lands at attacker's cell; attacker HP drops.
8. `suicide_weapon_kills_firer` — Demo Truck-style; attacker HP < weapon damage;
   attacker dies.
9. `suicide_plus_deathweapon_double_detonates` — Demo Truck pattern: Suicide
   weapon damage + DeathWeapon dispatch on death → two damage events from same
   warhead at attacker's cell (ledger #11). May produce nearly-doubled damage.
10. `suicide_bypasses_friendly_gate_for_self` — Suicide weapon with
    `AffectsAllies=no` warhead: firer's self-hit still applies damage (Suicide
    bypass per ledger #9).

**V3 Airburst:**
11. `airburst_spawns_nine_cells` — fake V3 setup: primary `[V3AirburstP]` with
    `Airburst=yes` + `AirburstWeapon=V3Cluster`; impact at (10,10); damage
    appears at 9 cells (10,10) + 8 neighbors. Check by placing one target per
    cell and asserting each took the sub-bullet damage.
12. `airburst_uses_subweapon_damage_unscaled` — each of 9 cells gets
    `AirburstWeapon.Damage` (e.g., 80), not `primary.Damage / 9` (ledger #18).
13. `airburst_uses_subweapon_warhead` — sub-warhead's Verses applies, not the
    primary warhead's (ledger #19).
14. `airburst_no_recurse_when_sub_projectile_has_airburst` — if AirburstWeapon
    points to a weapon whose projectile also has `Airburst=yes`, only one level
    of spawn fires (no 81-cell explosion). Ledger #23.
15. `airburst_skips_offmap_cells` — impact at (0, 0); 8-neighbor lookup
    produces cells with negative coords; those are skipped without panic.
    Ledger #22.
16. `airburst_composes_with_affects_allies` — V3 with `AirburstWeapon.Warhead`
    set to a warhead with `AffectsAllies=no`; sub-bullet damage skips allied
    units in each cell (ledger #24).
17. `airburst_runs_after_primary_detonate` — primary V3 warhead damage AND
    sub-bullet damage both apply (ledger #14). Test by giving primary warhead
    a different `AnimList` than sub-warhead; assert both anims spawn.

### Determinism Considerations

- AffectsAllies gate iterates the existing target list in BTreeMap order
  (deterministic). The skip is purely a boolean check.
- Suicide redirect uses attacker's existing position values (deterministic).
- Airburst direction loop is a fixed array; iteration order is fixed.
- `apply_aoe_damage` internal iteration uses `BTreeSet<u64>` `seen` set
  (already deterministic per occupancy list order in
  [combat_aoe.rs:76](src/sim/combat/combat_aoe.rs#L76)).
- No new state on `GameEntity`; no state-hash change.
- No RNG involved.

## Architectural Decisions

- **Pattern followed:** use existing `HouseAllianceMap::are_houses_friendly`
  helper (same pattern as movement/bump_crush). Don't introduce a new alliance
  check.
- **Pattern followed:** apply gates at existing damage application sites
  (`apply_aoe_damage` per-target, direct-hit block). Don't add a new "damage
  filter" layer.
- **Pattern followed:** Airburst is a wrapper around `apply_aoe_damage` (a
  loop) rather than a new spawn dispatcher. Matches the no-projectile-entity
  reality of the current pipeline.
- **Pattern deviated from:** Sub-bullet visuals are NOT spawned (gamemd
  spawns 9 BulletClass instances; we apply damage instantly). Reason:
  projectile→damage pipeline doesn't exist yet, and bolting visual entities
  on top of instant damage produces a wrong-order visual (damage flash before
  visual sub-bullets arrive). Documented as deferred follow-up.
- **Tech debt acknowledged:**
  - The misleading comment at combat_aoe.rs:43-45 gets removed (was actively
    wrong about gamemd behavior).
  - The `apply_aoe_damage` signature grows by two parameters
    (`alliances`, `is_suicide`). Acceptable — both are sim-pipeline data that
    needs to flow through.

## Alternatives Considered

- **Approach B (three separate brainstorm cycles):** rejected — same total
  scope with 3× the design-doc overhead. Lost the shared test fixture
  benefit.
- **Approach C (only AffectsAllies):** rejected — leaves visible Demo Truck +
  V3 parity gaps that the user explicitly named as worth fixing in the
  gap-scan handoff. Cutting them on grounds of smaller scope would be
  convenience-disguised drift.
- **Airburst full sub-bullet spawn (visual entities):** rejected with user
  confirmation — the projectile→damage pipeline isn't wired up; spawning
  visual entities now without damage hookup produces a wrong-order visual
  (damage lands first, visual bullets arrive empty). Tracked as deferred
  follow-up.
- **Suicide as warhead-side flag instead of weapon-side:** gamemd defines
  Suicide on WeaponType (`+0x144`), not WarheadType. We follow gamemd's data
  model.
- **AffectsAllies as a per-firer override (e.g., command-line ForceFire
  override-bypass):** rejected per ledger #5 — ForceFire does NOT bypass the
  gate in gamemd. Our design matches.

## Deferred Follow-Ups (NOT in this design's scope)

1. **Visual sub-bullet flight for Airburst** — spawning 9 HomingState entities
   that animate the V3 cluster bits as in-flight projectiles. Blocked on
   projectile→damage pipeline being wired up (separate, large refactor).
   Tracked at [world/mod.rs:1102-1103](src/sim/world/mod.rs#L1102-L1103)
   already.
2. **Psychedelic warhead ally gate (`wh+0x16D`)** — second independent ally
   gate, layers with AffectsAllies. Lands when Psychedelic warhead support is
   implemented (currently warhead.psychedelic parses but no consumer).
3. **IvanBomb warhead cascade interaction** — fixing the open follow-up #2 in
   suicide_weapons.md (verify Suicide + IvanBomb interaction). Will resolve
   the known drift accepted in ledger #12.
4. **DeathWeaponDamageModifier scaling** — currently DeathWeapon damage isn't
   scaled by per-unit `DeathWeaponDamageModifier` (Kirov 0.1, NukeCarrier
   0.5). Out of this bundle's scope. Tracked in suicide_weapons.md open
   follow-up #5.
5. **Demo Truck double-fire gate** — verify whether gamemd suppresses
   DeathWeapon when the death cause was the unit's own Suicide weapon
   (suicide_weapons.md open follow-up #3). If yes, our impl's "two
   detonations" needs a gate. Currently we ship with both detonations firing
   (matches the naive reading of the gamemd ReceiveDamage pipeline).
