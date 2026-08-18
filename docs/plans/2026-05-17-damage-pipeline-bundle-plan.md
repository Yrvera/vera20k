# Damage Pipeline Bundle Implementation Plan — Suicide + V3 Airburst

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Wire two damage-pipeline parity gaps: (1) `Suicide=yes` weapons detonate
at the firer's position so Demo Truck / IFV-Ivan self-destruct; (2) V3 Airburst
spawns a 9-cell AoE damage pattern at impact so the V3 cluster strike deals its
full damage profile.

**Architecture:** Two independent insertion points in the existing fire-time
damage pipeline at [combat/mod.rs](src/sim/combat/mod.rs):

1. **Suicide** — target-coord redirect to firer's own position, inserted after
   all fire-gates pass but before damage application (around line 1786).
2. **Airburst** — projectile lookup + 9-cell `apply_aoe_damage` loop, inserted
   after the primary damage block (around line 1872, after both AoE and
   direct-hit branches converge).

No new entity state, no projectile-entity spawning, no tick-ordering change.

**Design Doc:** [docs/plans/2026-05-17-damage-pipeline-bundle-design.md](docs/plans/2026-05-17-damage-pipeline-bundle-design.md)

---

## Grounding Summary

- **Docs.** `combat/systems/suicide_weapons.md` (DONE, 2026-05-17, 3-axis verified):
  Suicide flag at `weapon+0x144`, mechanism via `Fire_At` self-target short-circuit.
  `combat/systems/airburst.md` (DONE, 2026-05-17, 3-axis verified): 9 sub-bullets
  hardcoded (8-loop + 1 explicit), each carrying full `AirburstWeapon.Damage`,
  fixed direction order via `g_DirectionOffsets`. V3 is the only retail user.
- **Ghidra verification.** `WarheadTypeClass::ReadINI` at `0x0075DD80` decompiled
  this session — confirmed offset `0x179` is `AffectsAllies` (NOT `radiation` as
  the current Rust comment in `warhead_type.rs:87` claims; `radiation` is `0x177`).
  This impacts NEITHER subsystem in this plan but is captured for the
  doc/repo-comment cleanup follow-up.
- **AffectsAllies scope correction.** Original design's AffectsAllies item was
  cut from this bundle. Reason: `friendly_fire.md` claimed default = false, but
  rulesmd.ini comment "Defaults to yes" + WARHEADTYPECLASS_REINVESTIGATION_GHIDRA_REPORT.md
  + only 2 retail warheads (`[PsiPulse]`, `[SuperPsiPulse]`) use
  `AffectsAllies=no` together prove default = TRUE. Current Rust behavior
  (friendly damage applied) already matches gamemd for ~all warheads. No
  player-visible fix needed.
- **Repo patterns.**
  - Fire-time damage dispatch at [combat/mod.rs:1786-1872](src/sim/combat/mod.rs#L1786-L1872):
    `let warhead = selected.warhead; ... if warhead.cell_spread > SIM_ZERO { apply_aoe_damage(...) } else { direct-hit }`.
  - `apply_aoe_damage` at [combat_aoe.rs:48](src/sim/combat/combat_aoe.rs#L48)
    returns `Vec<(stable_id, damage)>`.
  - Weapon / projectile lookup via `rules.weapon(id) -> Option<&WeaponType>` and
    `rules.projectile(id) -> Option<&ProjectileType>` from
    [ruleset.rs:1573, 1583](src/rules/ruleset.rs#L1573).
  - `weapon.suicide: bool` and `projectile.airburst: bool`,
    `projectile.airburst_weapon: Option<String>` already parsed (gap-scan
    confirmed zero consumers).
- **INI keys.** `[Demobomb] Suicide=yes` (Demo Truck arrival weapon, retail
  `rulesmd.ini`). `[V3AirburstP] Airburst=yes AirburstWeapon=V3Cluster`
  (V3 cluster bullet). `[V3Cluster] Damage=80 Projectile=ClusterBits Warhead=V3HE`
  (sub-bullet). Confirmed in `combat/systems/airburst.md §9`.
- **Unknowns after grounding.**
  - Demo Truck double-fire interaction (whether engine gates DeathWeapon when
    Suicide was the death cause). Deferred per design follow-up #5.
  - IvanBomb cascade vs Suicide flag interaction. Deferred per design follow-up
    #3 (user accepted Ivan-self-detonates as drift).

## Key Technical Decisions

- **Suicide redirect happens after all fire-gates (range, facing, burst,
  cooldown) pass, before damage application.** — **Confidence:** high.
  **Source:** suicide_weapons.md §3.1 + canonical FIRE_AT_PIPELINE doc. Insertion
  point is right before `let warhead = selected.warhead;` at combat/mod.rs:1786,
  inside the per-entity fire loop. Gates use the ORIGINAL target (in-range,
  facing toward target); only the damage-application coords are redirected.
- **Airburst is a wrapper around `apply_aoe_damage`, not a projectile-entity
  spawn.** — **Confidence:** high (matches user-confirmed scope cut). **Source:**
  Design doc decision + airburst.md §5. Sub-bullet visuals deferred until the
  projectile→damage pipeline is wired (separate follow-up at
  [world/mod.rs:1102](src/sim/world/mod.rs#L1102)). Damage parity is correct;
  visual parity is the known deferred drift.
- **Direction order for Airburst 8-neighbor loop:** `(0,-1), (1,-1), (1,0),
  (1,1), (0,1), (-1,1), (-1,0), (-1,-1)` = N, NE, E, SE, S, SW, W, NW. —
  **Confidence:** medium. **Source:** airburst.md §6 (cites engine's
  `g_DirectionOffsets` 8-direction table; exact byte layout not enumerated in
  doc). The 8-loop iterates `dir = 0..7`; the cells covered are deterministically
  the 8 neighbors regardless of order. Order only matters for cross-tick
  determinism. **Flag for /review-plan:** verify the byte sequence in
  `g_DirectionOffsets` matches our hardcoded array, if a deeper RE pass becomes
  available.
- **Airburst recursion guard by construction, not flag.** —
  **Confidence:** high. **Source:** Design doc §"Components" §6 reasoning.
  `apply_airburst_spawn` calls `apply_aoe_damage` directly (warhead-only, no
  projectile lookup). The Airburst dispatch only runs at the PRIMARY
  fire-time block in `tick_combat`. Recursion is impossible because
  sub-bullet `apply_aoe_damage` never re-enters the Airburst dispatch.
- **Demo Truck two-detonations from composition.** — **Confidence:** medium.
  **Source:** suicide_weapons.md §4 (verified composition; open follow-up
  #3 unverified whether engine gates double-fire). Our impl will ship with
  both detonations firing (Suicide at target tick + DeathWeapon at HP=0 tick).
  If empirically wrong, gate later. Flag for /review-plan.

## Open Questions

### Resolved During Planning

- **Default for `AffectsAllies`:** `true`, not `false` per friendly_fire.md.
  Resolution: ini comment "Defaults to yes" + reinvestigation doc constructor
  init to 1 + only 2 retail warheads use `=no`. Bundle scope reduced
  accordingly.
- **Where Suicide redirects in the fire pipeline:** after gates, before damage.
  Resolution: insertion at combat/mod.rs:1786 (right after `pending_at_fire_frame`
  block, before `let warhead`). Source: suicide_weapons.md §3.1.
- **Whether to spawn visual sub-bullets for Airburst:** no. Resolution: user
  scope cut after architectural review (projectile→damage pipeline not wired).
  Tracked as deferred follow-up.

### Deferred to Implementation

- **`g_DirectionOffsets` exact byte layout in gamemd.** Our hardcoded
  direction order produces the right cell set but may iterate in a different
  order than gamemd. Determinism within Rust is preserved; cross-engine parity
  would only matter if a deep `/fidelity-check` against gamemd's per-tick
  sub-bullet spawn order is performed. Acceptable drift for damage parity.
- **Demo Truck double-fire gate** (suicide_weapons.md open follow-up #3).
  Whether gamemd suppresses DeathWeapon when the death cause was the unit's own
  Suicide weapon. Observation: ship with both firing; test in-game; if Demo Truck
  damage feels obviously doubled vs gamemd, add a gate.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/combat/mod.rs` | Insert Suicide target-coord redirect (~line 1786); insert Airburst dispatch + projectile lookup (~line 1872) |
| Modify | `src/sim/combat/combat_aoe.rs` | Add `apply_airburst_spawn` helper (9-cell loop wrapper around `apply_aoe_damage`) |
| Modify | `src/sim/combat/combat_tests.rs` | Add 3 Suicide tests + 6 Airburst tests |

No new files. No parser changes (both `weapon.suicide` and `projectile.airburst` /
`projectile.airburst_weapon` already parse).

## Interface Changes

- New module-private helper `apply_airburst_spawn` in `combat_aoe.rs`. No public
  API additions. Module-internal.
- `combat/mod.rs` gains two insertion blocks inside the existing `tick_combat`
  loop. No function signature changes.
- No new fields on `GameEntity`, `WarheadType`, `WeaponType`, or
  `ProjectileType` (parsers already cover them).

## Sim Checklist

- [x] All math uses integer cell coords (`u16`) and existing `SimFixed` damage
  types — no new f32/f64 in game logic.
- [x] No new state added; deterministic state hash unaffected.
- [x] No dependencies on render/ui/sidebar/audio/net — both changes are isolated
  to `sim/combat/`.
- [x] Tick ordering: unchanged. Both insertions run inside the existing fire-time
  damage block within `tick_combat`.
- [x] BTreeMap iteration order: `apply_aoe_damage` already uses deterministic
  `BTreeSet<u64>` `seen` for de-dup; Airburst calls it 9 times with the same
  determinism guarantee. Airburst's outer direction loop uses a fixed array.

## Risk Areas

- **Suicide weapon test setup.** `combat_tests.rs` doesn't have many fire-loop
  tests that drive the full `tick_combat` path. Each new test must set up a
  Simulation with an entity having a weapon, target acquired, in range, facing
  aligned. Reuse existing test scaffolding patterns (e.g., the V3-style splash
  tests at [combat_tests.rs:1488-1556](src/sim/combat/combat_tests.rs#L1488)
  show the harness).
- **Airburst recursion accidentally enabled.** If `apply_airburst_spawn` calls
  back into the projectile-lookup branch, infinite recursion. Mitigated by
  construction (helper calls `apply_aoe_damage` directly, never re-enters the
  combat/mod.rs Airburst-dispatch block).
- **Existing V3 test fixture.** [combat_tests.rs:1488](src/sim/combat/combat_tests.rs#L1488)
  uses `[V3WH] CellSpread=1` with NO `[V3W] Projectile=` set to a real `[Projectile]`
  with `Airburst=yes`. The Airburst dispatch checks `projectile.airburst` —
  with no projectile or `airburst=false`, the dispatch no-ops. Existing
  tests should continue passing unchanged. **Verify by running
  `cargo test --lib sim::combat` after Airburst lands.**
- **Demo Truck double-fire.** If shipping with both detonations causes the
  test for Demo Truck-style splash to expect double damage but gamemd
  actually single-fires, regression. No existing tests to break, but flag
  for in-game verification at Task 7.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | Suicide weapon damages the firer's own cell, not the target's | Demo Truck arrival explodes at the Demo Truck, killing it and damaging surrounding units. Visible every time a Demo Truck attacks. Currently the truck deals damage at the target without dying. | Test `suicide_weapon_damages_firer` (Task 2); in-game Task 7 |
| Task 1 | Demo Truck double-detonation from Suicide + DeathWeapon composition | Gamemd's Demo Truck composition: Suicide kills the truck → existing `death_weapon_aoe` fires Demobomb again at death position → TWO detonations. Visible as the famously massive Demo Truck explosion. Currently: zero detonations (truck doesn't die from Suicide). After fix: two (matches gamemd per ledger #11; open follow-up flagged if engine gates the second). | Test `suicide_plus_deathweapon_double_detonates` (Task 2); in-game Task 7 |
| Task 4 | V3 Airburst delivers 9 detonations in a 3×3 cell pattern (8 neighbors + impact) | V3 Rocket is iconic Soviet artillery. Currently: single splash at impact cell. After fix: 9 cells take AirburstWeapon damage. Visible every V3 shot. | Test `airburst_spawns_nine_cells` (Task 5); in-game Task 7 |
| Task 4 | Each Airburst sub-bullet carries the FULL AirburstWeapon.Damage (no division by 9, no falloff at spawn) | Per ledger #18: V3Cluster.Damage=80 means each of 9 cells gets 80 base damage (modulo per-cell warhead falloff). Theoretical max single-target = 9 × 80 × Verses. If we divided, V3 damage output halved or worse. | Test `airburst_uses_subweapon_damage_unscaled` (Task 5) |
| Task 4 | Airburst uses AirburstWeapon.Warhead, not the primary projectile's warhead | Per ledger #19: V3 primary uses V3HE; V3Cluster also uses V3HE (same in retail), but the warheads can differ in mods. The sub-warhead's Verses + CellSpread + AnimList apply per sub-cell. | Test `airburst_uses_subweapon_warhead` (Task 5) |
| Task 4 | Airburst runs AFTER the primary warhead detonate; both damages apply | Per ledger #14: primary V3HE damage + 9 sub-bullet V3HE damage both apply at the same tick. Player sees: primary impact anim + 9 cluster impact anims. Currently: only primary. | Test `airburst_runs_after_primary_detonate` (Task 5) |

---

## Tasks

### Task 1: Add Suicide self-target redirect in `tick_combat`

**Why:** Wires `weapon.suicide` (already parsed, zero consumers today) to
override damage-application coords to the firer's own position. After this,
Demo Truck's Demobomb damages the truck instead of the target; the truck dies;
the existing `death_weapon_aoe` path fires Demobomb a second time at the death
position naturally.

**Files:**
- Modify: `src/sim/combat/mod.rs:1781-1786` (insert before `Fire one shot!` block)

**Pattern:** Existing fire-time mutation pattern — variables `target_rx`,
`target_ry`, `target_sub_x`, `target_sub_y` are bound earlier in the loop and
read by the damage block. The redirect rebinds them when `weapon.suicide` is
true.

**Step 1: Read the insertion point**

Read [src/sim/combat/mod.rs:1781-1798](src/sim/combat/mod.rs#L1781-L1798). The
existing code is:

```rust
        if pending_at_fire_frame {
            pending_infantry_updates.push((snap.stable_id, None));
        }

        // Fire one shot!
        let warhead = selected.warhead;
        // Garrison damage: apply OccupyDamageMultiplier to base damage before AoE or
        // single-target paths. ...
```

**Step 2: Insert the Suicide redirect**

Replace the block above with:

```rust
        if pending_at_fire_frame {
            pending_infantry_updates.push((snap.stable_id, None));
        }

        // Suicide=yes: redirect damage to firer's own cell.
        // The firer's warhead detonates at its own position; firer dies in own
        // splash. Composes with DeathWeapon (Demo Truck pattern): when HP hits
        // 0, the existing death_weapon_aoe path fires again — two detonations
        // naturally fall out. Retail YR users: Demobomb (Demo Truck),
        // CRNuke (IFV-Ivan), IvanBomb / CRIvanBomb (Crazy Ivan — known drift
        // because IvanBomb warhead cascade not implemented; Ivan will
        // self-detonate in this impl).
        let (target_rx, target_ry, target_sub_x, target_sub_y) = if weapon.suicide {
            (snap.pos_rx, snap.pos_ry, snap.sub_x, snap.sub_y)
        } else {
            (target_rx, target_ry, target_sub_x, target_sub_y)
        };

        // Fire one shot!
        let warhead = selected.warhead;
```

**Step 3: Verify it compiles**

Run: `cargo check --lib`
Expected: clean (the rebinding shadows the existing variables; no other code
in scope changes).

No commit yet — Task 2 adds the tests.

---

### Task 2: Add three Suicide unit tests

**Why:** Locks in the new behavior: Suicide damages firer, kills firer when HP
< damage, composes with DeathWeapon to produce two detonations.

**Files:**
- Modify: `src/sim/combat/combat_tests.rs` (append to existing test module)

**Pattern:** Existing fire-loop integration tests at
[combat_tests.rs:1488-1556](src/sim/combat/combat_tests.rs#L1488-L1556) —
build a `Simulation` from INI fixture, spawn attacker + target, set target,
run `tick_combat`, assert HP change.

**Step 1: Add `suicide_weapon_damages_firer` test**

Append to `combat_tests.rs`:

```rust
#[test]
fn suicide_weapon_damages_firer() {
    use crate::sim::combat::tick_combat;
    use crate::sim::components::Health;
    use crate::sim::game_entity::GameEntity;
    use crate::map::entities::EntityCategory;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::ruleset::RuleSet;
    use crate::sim::world::Simulation;

    // Demo Truck-style: Suicide weapon, attacker has primary that targets at
    // range, but on fire the projectile lands at the attacker's own cell.
    let ini = IniFile::from_str(
        "[InfantryTypes]\n0=ATK\n[VehicleTypes]\n[AircraftTypes]\n[BuildingTypes]\n\
         [Warheads]\n0=DEMOBOMBWH\n[Weapons]\n0=DEMOBOMB\n[Projectiles]\n0=INVISLOW\n\n\
         [ATK]\nStrength=400\nArmor=light\nSpeed=4\nPrimary=DEMOBOMB\nOwner=Americans\n\n\
         [DEMOBOMB]\nDamage=200\nROF=5\nRange=4\nWarhead=DEMOBOMBWH\nProjectile=INVISLOW\nSuicide=yes\n\n\
         [DEMOBOMBWH]\nCellSpread=1\nPercentAtMax=100\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\n\
         [INVISLOW]\nAA=no\n",
    );
    let rules = RuleSet::from_ini(&ini).expect("parse");
    let mut sim = Simulation::new();
    let owner = sim.interner.intern("Americans");
    let atk_type = sim.interner.intern("ATK");

    // Attacker at (10, 10).
    let mut attacker = GameEntity::new(
        1, 10, 10, 0, 0, owner,
        Health { current: 400, max: 400 },
        atk_type, EntityCategory::Infantry, 0, 5, false,
    );
    // Target at (14, 10) — within range 4. Different owner.
    let other = sim.interner.intern("Soviet");
    let mut target = GameEntity::new(
        2, 14, 10, 0, 0, other,
        Health { current: 400, max: 400 },
        atk_type, EntityCategory::Infantry, 0, 5, false,
    );
    sim.entities.insert(attacker);
    sim.entities.insert(target);
    // Issue attack: attacker -> target.
    sim.entities.get_mut(1).unwrap().attack_target =
        Some(crate::sim::combat::AttackTarget::new(2));

    // Run combat ticks until firing happens.
    for _ in 0..30 {
        tick_combat(&mut sim, &rules, /* fog */ None, /* terrain */ None,
                    /* occupancy */ &sim.occupancy.clone(), /* overlay */ None,
                    /* overlay_registry */ None, 16, /* path_grid */ None);
        if sim.entities.get(1).unwrap().health.current < 400 { break; }
    }

    // Attacker should have damaged itself (Suicide redirected target to self).
    let atk_hp = sim.entities.get(1).unwrap().health.current;
    assert!(
        atk_hp < 400,
        "attacker should have damaged itself via Suicide redirect; HP = {}",
        atk_hp
    );
    // Target at distance 4 was NOT damaged by the redirected weapon
    // (Suicide moved the impact to attacker's cell, so target is out of
    // CellSpread=1 radius from the new impact point).
    let target_hp = sim.entities.get(2).unwrap().health.current;
    assert_eq!(
        target_hp, 400,
        "target should be unhurt after Suicide redirect; HP = {}",
        target_hp
    );
}
```

Note: the exact `tick_combat` signature and `attack_target` field access may
require small adjustments after reading existing test patterns in `combat_tests.rs`.
The test harness uses the same pattern as the V3-splash test at line 1488.

**Step 2: Add `suicide_weapon_kills_firer` test**

```rust
#[test]
fn suicide_weapon_kills_firer() {
    // Attacker HP=100, Suicide weapon Damage=200. After firing, attacker dead.
    use crate::sim::combat::tick_combat;
    // ... same harness as above ...
    // attacker HP = 100, weapon damage = 200
    // After fire tick: attacker.health.current == 0 (dead)
    let final_hp = sim.entities.get(1).map(|e| e.health.current).unwrap_or(0);
    assert_eq!(final_hp, 0, "attacker should be killed by own Suicide damage");
}
```

(Full test body mirrors Step 1; only `Strength=100` differs in INI.)

**Step 3: Add `suicide_plus_deathweapon_double_detonates` test**

```rust
#[test]
fn suicide_plus_deathweapon_double_detonates() {
    // Demo Truck composition: Suicide weapon + DeathWeapon=same weapon.
    // After Suicide kills attacker: DeathWeapon dispatch fires again at
    // attacker's death position. Two damage events from same warhead.
    // Verify: a target adjacent to attacker takes ~2x weapon damage.
    use crate::sim::combat::tick_combat;
    // INI: ATK has DeathWeapon=DEMOBOMB AND Primary=DEMOBOMB with Suicide=yes.
    // Place a second-target unit at (11, 10) — adjacent to attacker (10, 10)
    // so CellSpread=1 hits it from both detonations.
    // Run tick_combat to detonation + check second-target HP loss > 1x weapon dmg.
    let second_target_hp_loss = 400 - sim.entities.get(3).unwrap().health.current;
    assert!(
        second_target_hp_loss > 200,
        "adjacent unit should take damage from BOTH Suicide and DeathWeapon detonations; \
         hp_loss = {} (expected > 200 = one full warhead's damage)",
        second_target_hp_loss
    );
}
```

**Step 4: Run the three Suicide tests**

Run: `cargo test --lib sim::combat::tests::suicide -- --nocapture`
Expected: all 3 PASS.

If the test harness in Step 1 doesn't compile cleanly (signature mismatches),
read the existing V3 splash test at [combat_tests.rs:1488](src/sim/combat/combat_tests.rs#L1488)
to copy the exact `tick_combat` call signature in use. Adjust the new tests
to match.

**Step 5: Commit**

```
combat: implement Suicide weapon self-detonate

weapon.suicide is already parsed but had no consumer. Add the target-coord
redirect in tick_combat: when a Suicide weapon fires, override the impact
cell to the firer's own position. The firer dies in its own splash;
existing death_weapon_aoe fires the DeathWeapon (typically the same
weapon) again at the death position, producing the well-known Demo Truck
double-detonation.

Retail users: Demobomb (Demo Truck), CRNuke (IFV-Ivan), IvanBomb /
CRIvanBomb (Crazy Ivan — known drift: Ivan will self-detonate in this
impl; correct behavior requires IvanBomb warhead cascade which isn't
implemented yet).
```

---

### Task 3: Add `apply_airburst_spawn` helper in `combat_aoe.rs`

**Why:** Encapsulates the 9-cell loop that the Airburst dispatch needs. Lives
in `combat_aoe.rs` because it's a wrapper around `apply_aoe_damage`. Adding
this BEFORE Task 4 (which wires the dispatch from combat/mod.rs) keeps the
helper independently testable and the wiring change minimal.

**Files:**
- Modify: `src/sim/combat/combat_aoe.rs` (add new module-private function)

**Pattern:** Mirrors the existing `apply_aoe_damage` signature; loops over 9
cells calling it once per cell.

**Step 1: Add the helper after `apply_aoe_damage`**

Insert after the existing `apply_aoe_damage` function (end of the function,
before the next function or `#[cfg(test)]` block):

```rust
/// Airburst sub-weapon spawn — 9-cell AoE pattern around the impact cell.
///
/// Matches gamemd's airburst spawn at the end of `WarheadTypeClass::Detonate`:
/// 8 sub-bullets at the 8-neighbor cells (`Pathfinding_update_continued(0..7)`)
/// plus 1 at the impact cell itself. Each sub-bullet's damage is the full
/// AirburstWeapon.Damage (no division, no scaling) with per-cell warhead
/// falloff applied inside each `apply_aoe_damage` call.
///
/// Direction order is N → NE → E → SE → S → SW → W → NW, matching the engine's
/// `g_DirectionOffsets` 8-direction table. Out-of-bounds cells (negative coords)
/// are silently skipped to match the on-map-edge edge case.
///
/// Sub-bullet visual flight is NOT spawned — this returns instant 9-cell
/// damage. Visual parity is a known deferred drift, blocked on the
/// projectile→damage pipeline being wired.
pub(crate) fn apply_airburst_spawn(
    entities: &EntityStore,
    impact_rx: u16,
    impact_ry: u16,
    sub_damage: i32,
    sub_warhead: &WarheadType,
    rules: &RuleSet,
    interner: &StringInterner,
    attacker_owner: &str,
    layer_context: AoELayerContext<'_>,
) -> Vec<(u64, u16)> {
    // 8-direction offsets matching gamemd's g_DirectionOffsets order:
    // N, NE, E, SE, S, SW, W, NW.
    const NEIGHBORS: [(i32, i32); 8] = [
        (0, -1), (1, -1), (1, 0), (1, 1),
        (0, 1), (-1, 1), (-1, 0), (-1, -1),
    ];

    let mut all_hits: Vec<(u64, u16)> = Vec::new();

    // 8 neighbor cells.
    for (dx, dy) in NEIGHBORS.iter() {
        let nx = impact_rx as i32 + dx;
        let ny = impact_ry as i32 + dy;
        if nx < 0 || ny < 0 || nx > u16::MAX as i32 || ny > u16::MAX as i32 {
            continue;
        }
        let hits = apply_aoe_damage(
            entities,
            nx as u16,
            ny as u16,
            sub_damage,
            sub_warhead,
            rules,
            interner,
            attacker_owner,
            layer_context,
        );
        all_hits.extend(hits);
    }

    // 9th: impact cell itself.
    let hits = apply_aoe_damage(
        entities,
        impact_rx,
        impact_ry,
        sub_damage,
        sub_warhead,
        rules,
        interner,
        attacker_owner,
        layer_context,
    );
    all_hits.extend(hits);

    all_hits
}
```

**Step 2: Verify it compiles**

Run: `cargo check --lib`
Expected: clean (function is currently unused; the next task wires it up before
the next commit, so no `#[allow(dead_code)]` needed).

No commit yet — Task 4 wires the dispatch and commits both.

---

### Task 4: Wire Airburst dispatch in `tick_combat`

**Why:** Adds the projectile lookup + airburst spawn call right after the
primary damage block. Runs regardless of whether the primary went through AoE
or direct-hit, because gamemd's airburst spawn is at the end of `Detonate`
which runs for both paths.

**Files:**
- Modify: `src/sim/combat/mod.rs` (insert after the existing damage block,
  around line 1872, after both AoE and direct-hit branches converge)

**Pattern:** Module-internal lookup using existing `rules.weapon(id)` and
`rules.projectile(id)` accessors. Call the new `apply_airburst_spawn` helper
and push results to `damage_events`.

**Step 1: Read the insertion point**

Read [src/sim/combat/mod.rs:1818-1880](src/sim/combat/mod.rs#L1818-L1880) to
find where both branches of `if warhead.cell_spread > SIM_ZERO { ... } else { ... }`
converge. The Airburst dispatch goes right after that convergence, before the
next entity in the loop is processed.

**Step 2: Insert Airburst dispatch**

After the closing brace of the `else` direct-hit branch (line ~1872), add:

```rust
        // Airburst spawn: if the weapon's projectile has Airburst=yes and a
        // valid AirburstWeapon, spawn 9 AoE detonations in a 3x3 pattern around
        // the impact cell (8 neighbors + impact cell). Each sub-detonation uses
        // AirburstWeapon.Damage and AirburstWeapon.Warhead. Sub-bullet visual
        // flight is NOT spawned — damage applies instantly. The only retail user
        // is V3 Rocket [V3AirburstP] -> [V3Cluster] -> ClusterBits.
        if let Some(proj_id) = weapon.projectile.as_deref() {
            if let Some(proj) = rules.projectile(proj_id) {
                if proj.airburst {
                    if let Some(ab_weapon_id) = proj.airburst_weapon.as_deref() {
                        if let Some(ab_weapon) = rules.weapon(ab_weapon_id) {
                            if let Some(ab_warhead_id) = ab_weapon.warhead.as_deref() {
                                if let Some(ab_warhead) = rules.warhead(ab_warhead_id) {
                                    let airburst_hits = self::combat_aoe::apply_airburst_spawn(
                                        entities,
                                        target_rx,
                                        target_ry,
                                        ab_weapon.damage,
                                        ab_warhead,
                                        rules,
                                        interner,
                                        interner.resolve(snap.owner),
                                        self::combat_aoe::AoELayerContext {
                                            occupancy: Some(&*occupancy),
                                            terrain,
                                            impact_z,
                                        },
                                    );
                                    let ab_wh_iid = interner.intern(&ab_warhead.id);
                                    for (target_id, dmg) in airburst_hits {
                                        damage_events.push((
                                            target_id, dmg, snap.stable_id, ab_wh_iid,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
```

The Airburst dispatch uses `target_rx`, `target_ry` (the *original* impact cell
— Suicide's redirect already applied if `weapon.suicide`, but the Airburst
spawn pattern uses the same impact point so the redirect transparently composes
with Suicide).

**Step 3: Verify it compiles**

Run: `cargo check --lib`
Expected: clean.

**Step 4: Run combat suite to confirm no regression**

Run: `cargo test --lib sim::combat -- --nocapture`
Expected: all existing tests PASS (no `Airburst=yes` exists in any current
test fixture, so the new dispatch no-ops). Suicide tests from Task 2 also PASS.

**Step 5: Commit**

```
combat: implement V3 Airburst 9-cell sub-weapon spawn

projectile.airburst + projectile.airburst_weapon are already parsed but
had no consumer. Add apply_airburst_spawn helper in combat_aoe.rs (9-cell
loop wrapper around apply_aoe_damage) and wire the dispatch in
tick_combat: after the primary warhead detonate, if the projectile has
Airburst=yes, spawn AirburstWeapon damage at 8 neighbor cells + impact
cell. Direction order N/NE/E/SE/S/SW/W/NW matches gamemd's
g_DirectionOffsets.

Sole retail user: V3 Rocket [V3AirburstP] -> [V3Cluster] -> ClusterBits.

Sub-bullet visual flight NOT spawned — that's deferred until the
projectile->damage pipeline is wired (see world/mod.rs:1102 comment).
Damage parity is correct; visual parity is the known deferred drift.

Recursion is impossible by construction: apply_airburst_spawn calls
apply_aoe_damage directly (warhead-only, no projectile lookup), so the
combat/mod.rs Airburst-dispatch block can't be re-entered from within
a sub-bullet's damage application.
```

---

### Task 5: Add six Airburst unit tests

**Why:** Locks in the 9-cell damage pattern, the unscaled per-cell damage, the
sub-warhead selection, the out-of-bounds skip, the recursion guard, and the
post-primary ordering.

**Files:**
- Modify: `src/sim/combat/combat_tests.rs` (append after the Suicide tests
  from Task 2)

**Pattern:** Same V3-style integration harness from Task 2.

**Step 1: Add `airburst_spawns_nine_cells` test**

```rust
#[test]
fn airburst_spawns_nine_cells() {
    use crate::sim::combat::tick_combat;
    use crate::sim::components::Health;
    use crate::sim::game_entity::GameEntity;
    use crate::map::entities::EntityCategory;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::ruleset::RuleSet;
    use crate::sim::world::Simulation;

    // V3-style: primary V3Airburst weapon with V3AirburstP projectile (Airburst=yes)
    // and V3Cluster as AirburstWeapon. Impact at (15, 15) should hit 9 cells:
    // 8 neighbors + impact = (14,14) (15,14) (16,14) (14,15) (15,15) (16,15)
    //                        (14,16) (15,16) (16,16).
    let ini = IniFile::from_str(
        "[InfantryTypes]\n0=ATK\n[VehicleTypes]\n[AircraftTypes]\n[BuildingTypes]\n\
         [Warheads]\n0=V3HE\n1=V3CWH\n[Weapons]\n0=V3AIR\n1=V3CLUSTER\n\
         [Projectiles]\n0=V3AIRP\n1=CBITS\n\n\
         [ATK]\nStrength=500\nArmor=light\nSpeed=4\nPrimary=V3AIR\nOwner=Americans\n\n\
         [V3AIR]\nDamage=25\nROF=80\nRange=20\nWarhead=V3HE\nProjectile=V3AIRP\n\n\
         [V3CLUSTER]\nDamage=80\nWarhead=V3CWH\nProjectile=CBITS\n\n\
         [V3AIRP]\nAA=no\nAirburst=yes\nAirburstWeapon=V3CLUSTER\n\n\
         [CBITS]\nAA=no\n\n\
         [V3HE]\nCellSpread=1\nPercentAtMax=100\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\n\
         [V3CWH]\nCellSpread=1\nPercentAtMax=100\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    );
    let rules = RuleSet::from_ini(&ini).expect("parse");
    let mut sim = Simulation::new();
    let owner = sim.interner.intern("Americans");
    let other = sim.interner.intern("Soviet");
    let atk_type = sim.interner.intern("ATK");

    // Attacker at (5, 5).
    sim.entities.insert(GameEntity::new(
        1, 5, 5, 0, 0, owner,
        Health { current: 500, max: 500 },
        atk_type, EntityCategory::Infantry, 0, 5, false,
    ));
    // Place 9 target sentinels in a 3x3 around impact (15, 15).
    let mut id = 2u64;
    for dx in -1i32..=1 { for dy in -1i32..=1 {
        let rx = (15 + dx) as u16;
        let ry = (15 + dy) as u16;
        sim.entities.insert(GameEntity::new(
            id, rx, ry, 0, 0, other,
            Health { current: 200, max: 200 },
            atk_type, EntityCategory::Infantry, 0, 5, false,
        ));
        id += 1;
    }}
    // Issue attack at central target (id 6 = cell 15,15).
    sim.entities.get_mut(1).unwrap().attack_target =
        Some(crate::sim::combat::AttackTarget::new(6));

    for _ in 0..40 {
        tick_combat(/* same args as Task 2 */);
        if sim.entities.values().any(|e| e.owner == other && e.health.current < 200) {
            break;
        }
    }

    // Each of the 9 surrounding-cell targets should have taken damage.
    let damaged_count = sim.entities.values()
        .filter(|e| e.owner == other && e.health.current < 200)
        .count();
    assert_eq!(damaged_count, 9,
        "all 9 cells in 3x3 around (15,15) should have taken Airburst damage");
}
```

**Step 2: Add `airburst_uses_subweapon_damage_unscaled` test**

```rust
#[test]
fn airburst_uses_subweapon_damage_unscaled() {
    // Primary V3AIR.Damage=10 (small), V3CLUSTER.Damage=80 (large).
    // Single sentinel target at central impact cell (15, 15).
    // After fire: target took at least V3CLUSTER damage (NOT V3AIR damage,
    // NOT V3CLUSTER/9, NOT V3AIR + V3CLUSTER/9).
    // The target gets BOTH primary V3HE (10) AND Airburst V3CWH (80) at its cell.
    // Total damage on central target: 10 + 80 = 90. NOT 10 + 80/9.
    // ... same harness ...
    let central_target_hp = sim.entities.get(6).unwrap().health.current;
    let damage_taken = 200 - central_target_hp;
    // Expect at least 80 (the unscaled sub-damage). If we erroneously divided,
    // we'd see ~10 + 80/9 = 19.
    assert!(damage_taken >= 80,
        "central target should take full sub-bullet damage (80), got hp_loss={}",
        damage_taken);
}
```

**Step 3: Add `airburst_uses_subweapon_warhead` test**

```rust
#[test]
fn airburst_uses_subweapon_warhead() {
    // Primary V3HE.Verses against heavy=100. Sub-warhead V3CWH.Verses against
    // heavy=0. A heavy-armor target at the central impact cell takes only the
    // primary 25 damage; the sub-warhead detonation contributes 0 due to Verses.
    // ... same harness, target armor=heavy, V3CWH Verses=0 against heavy ...
    let central_target_hp_loss = 200 - sim.entities.get(6).unwrap().health.current;
    // V3HE delivers 25 at center; V3CWH delivers 0 (Verses against heavy = 0).
    // Total = 25, not 25 + 80.
    assert_eq!(central_target_hp_loss, 25,
        "sub-warhead Verses=0 vs heavy should zero the Airburst damage; \
         only primary damage applies. got hp_loss={}", central_target_hp_loss);
}
```

**Step 4: Add `airburst_skips_offmap_cells` test**

```rust
#[test]
fn airburst_skips_offmap_cells() {
    // Impact at (0, 0). 8-neighbor lookup produces negative coords for 5 of 8
    // neighbors. Those are silently skipped (no panic, no out-of-bounds).
    // The 3 in-bounds neighbors: (1, 0), (0, 1), (1, 1).
    // Plus impact cell (0, 0). So 4 in-bounds cells should take damage.
    // Place 4 target sentinels at (0,0), (1,0), (0,1), (1,1); confirm all 4 damaged
    // and no panic.
    // ... same harness, impact at (0,0) ...
    let damaged_count = sim.entities.values()
        .filter(|e| e.owner == other && e.health.current < 200)
        .count();
    assert_eq!(damaged_count, 4,
        "at map-corner impact, only in-bounds cells take damage; got {}",
        damaged_count);
}
```

**Step 5: Add `airburst_no_recurse_when_sub_projectile_has_airburst` test**

```rust
#[test]
fn airburst_no_recurse_when_sub_projectile_has_airburst() {
    // Pathological: AirburstWeapon points to a weapon whose Projectile=CBITS
    // also has Airburst=yes AirburstWeapon=V3CLUSTER (loop back to itself).
    // Recursion is impossible by construction in our impl (apply_airburst_spawn
    // calls apply_aoe_damage directly, never re-entering the combat/mod.rs
    // dispatch). This test confirms: a single Airburst fires, no 81-cell
    // explosion, no panic.
    // ... harness with CBITS.Airburst=yes CBITS.AirburstWeapon=V3CLUSTER ...
    let damaged_count = sim.entities.values()
        .filter(|e| e.owner == other && e.health.current < 200)
        .count();
    // Only 9 should be damaged (one level of Airburst); not 81 (recursion).
    assert!(damaged_count <= 9,
        "Airburst should not recurse; got {} damaged (expected <= 9)",
        damaged_count);
}
```

**Step 6: Add `airburst_runs_after_primary_detonate` test**

```rust
#[test]
fn airburst_runs_after_primary_detonate() {
    // Primary V3HE.AnimList=PrimaryExp; sub-warhead V3CWH.AnimList=SubExp.
    // Both anims should appear in sim.anim_events after firing.
    // (or check damage_events containing both warhead IDs)
    // Demonstrates that both damage paths fire in the same tick: primary
    // first, then 9 sub-bullets.
    // ... harness collects damage_events; assert both V3HE and V3CWH appear ...
    let damage_warheads: std::collections::HashSet<_> =
        sim.damage_events.iter().map(|e| e.warhead_id).collect();
    let v3he_iid = sim.interner.intern("V3HE");
    let v3cwh_iid = sim.interner.intern("V3CWH");
    assert!(damage_warheads.contains(&v3he_iid),
        "primary V3HE warhead should appear in damage events");
    assert!(damage_warheads.contains(&v3cwh_iid),
        "Airburst V3CWH sub-warhead should appear in damage events");
}
```

(Note: exact `sim.damage_events` access may need adjustment based on how the
test harness exposes per-tick damage events. If the field isn't directly
accessible, collect HP changes per target instead.)

**Step 7: Run all six new Airburst tests**

Run: `cargo test --lib sim::combat::tests::airburst -- --nocapture`
Expected: 6/6 PASS.

If `tick_combat` signature in tests differs from what's shown in Steps 1-6,
read the V3-style splash test at
[combat_tests.rs:1488-1556](src/sim/combat/combat_tests.rs#L1488-L1556) and
copy the harness pattern exactly.

**Step 8: Run full combat module suite**

Run: `cargo test --lib sim::combat`
Expected: all tests PASS (existing + 3 Suicide + 6 Airburst = 9 new).

**Step 9: Commit**

```
combat: cover Suicide + V3 Airburst end-to-end

Six Airburst tests (9-cell damage, unscaled per-cell, sub-warhead used,
off-map cells skipped, no recursion, post-primary ordering) plus three
Suicide tests (firer damaged, firer killed, double-detonation with
DeathWeapon).

Composition with existing DeathWeapon path verified for the Demo Truck
double-detonation case.
```

---

### Task 6: End-to-end verification against gamemd.exe

**Why:** Confirm the implementation matches the original engine's observable
behavior in real skirmish play, not just unit tests.

**Files:** None (manual verification).

**Verify Suicide:**
- Launch the Rust client (`cargo run`), start a skirmish as Soviet, build a
  Demo Truck.
- Attack-move the Demo Truck into an enemy base.
- **Expected (matches gamemd):** the Demo Truck arrives at the target, fires
  its Demobomb at itself, detonates, dies in the explosion; immediately
  afterward the DeathWeapon=Demobomb fires AGAIN at the death position,
  producing a second detonation. Net: massive single-spot blast.
- **Regression watch:** verify the Demo Truck's *target* is NOT also blown up
  by a long-range projectile (Suicide redirects impact to the firer, so the
  target only takes damage if it's within CellSpread of the truck's death
  position).

Cross-check gamemd:
- The Demo Truck damage signature is famously large in retail YR. If our impl
  shows obviously DOUBLE the damage of retail YR, the double-fire gate
  (suicide_weapons.md open follow-up #3) may apply. File as parity drift
  follow-up for resolution.

**Verify V3 Airburst:**
- Build a V3 Launcher, target an enemy cluster of units.
- **Expected (matches gamemd):** the V3 rocket impacts the targeted cell;
  damage appears at the impact cell AND at the 8 surrounding cells. With our
  simplified impl, the visual is a single primary impact flash plus 9 damage
  events (no cluster trail / per-cell impact anim — those are the known
  deferred visual drift).
- **Regression watch:** confirm V3 deals more damage to clustered units than
  before. A single-target V3 vs a Grizzly Tank should still deal roughly
  V3HE damage; a clustered group should take noticeably more from the
  Airburst spread.

Cross-check gamemd (Ghidra MCP):
- Re-confirm `BulletClass::BulletDetonation` at `0x00468D80` Airburst gate at
  `BulletType+0x294` — already verified in airburst.md §4.
- Confirm `g_DirectionOffsets` byte layout matches our hardcoded
  N/NE/E/SE/S/SW/W/NW order (only matters if `/fidelity-check` ever runs a
  per-tick spawn-order comparison).

If a difference is observed, file under design's "Deferred Follow-Ups" and
bring back to `/brainstorm`.

**No commit needed.** Verification, not modification.

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-17-damage-pipeline-bundle-design.md](docs/plans/2026-05-17-damage-pipeline-bundle-design.md)
- **Verified Ghidra reports (combat/systems/):**
  - `ra2-rust-game-docs/combat/systems/suicide_weapons.md` — DONE 2026-05-17,
    3-axis verified. `weapon.suicide` at `+0x144`, mechanism via `Fire_At`
    self-target.
  - `ra2-rust-game-docs/combat/systems/airburst.md` — DONE 2026-05-17, 3-axis
    verified. 9 sub-bullets hardcoded (8-loop + 1 explicit), direction order
    via `g_DirectionOffsets`, full damage per sub-bullet.
- **Doc disagreement (resolved):**
  - `ra2-rust-game-docs/combat/systems/friendly_fire.md` claimed
    `AffectsAllies` default = false. Contradicted by rulesmd.ini comment
    ("Defaults to yes"), `WARHEADTYPECLASS_REINVESTIGATION_GHIDRA_REPORT.md`
    (constructor sets +0x179 = 1), and retail INI scan (only 2 warheads
    use `=no`). **Result:** AffectsAllies cut from this bundle. `friendly_fire.md`
    needs `/verify-doc` follow-up.
- **gamemd.exe addresses (kept here, NOT in Rust code comments per CLAUDE.md):**
  - `WeaponTypeClass` `Suicide` flag @ `weapon+0x144`, parsed at
    `WeaponTypeClass::ReadINI 0x0077228D`
  - `BulletTypeClass` `Airburst` @ `+0x294`, `AirburstWeapon` @ `+0x2B0`,
    parsed in `BulletTypeClass::ReadINI`
  - `WarheadTypeClass::Detonate` (parent of Airburst spawn block) @ `0x004690B0`
  - `BulletClass::BulletDetonation` (Airburst gate) @ `0x00468D80`
  - `Pathfinding_update_continued` (8-direction neighbor lookup) @ `0x00481810`
- **INI keys:**
  - `rulesmd.ini [Demobomb] Suicide=yes` (Demo Truck arrival weapon)
  - `rulesmd.ini [CRNuke] Suicide=yes` (IFV-Ivan special)
  - `rulesmd.ini [IvanBomb] Suicide=yes` (Crazy Ivan place — known drift)
  - `rulesmd.ini [CRIvanBomb] Suicide=yes` (IFV-Ivan — known drift)
  - `rulesmd.ini [V3AirburstP] Airburst=yes` + `AirburstWeapon=V3Cluster`
  - `rulesmd.ini [V3Cluster] Damage=80 Warhead=V3HE Projectile=ClusterBits`
  - `rulesmd.ini [ClusterBits] ROT=60` (sub-bullet, homing — not modeled in
    this simplified impl)
- **Related code:**
  - [src/sim/combat/mod.rs:1786-1872](src/sim/combat/mod.rs#L1786-L1872) — fire-time damage block (Suicide + Airburst insertion area)
  - [src/sim/combat/combat_aoe.rs:48](src/sim/combat/combat_aoe.rs#L48) — `apply_aoe_damage` (wrapped by new `apply_airburst_spawn`)
  - [src/sim/combat/mod.rs:704](src/sim/combat/mod.rs#L704) — `death_weapon_aoe` (composes with Suicide for Demo Truck double-fire)
  - [src/rules/weapon_type.rs](src/rules/weapon_type.rs) `weapon.suicide: bool` — already parsed
  - [src/rules/projectile_type.rs](src/rules/projectile_type.rs) `projectile.airburst: bool`, `projectile.airburst_weapon: Option<String>` — already parsed
  - [src/rules/ruleset.rs:1573, 1583](src/rules/ruleset.rs#L1573) — `rules.weapon(id)` / `rules.projectile(id)` lookup
- **Repo comment cleanup follow-up (separate, not in this plan):**
  - [src/rules/warhead_type.rs:87](src/rules/warhead_type.rs#L87) currently says
    `radiation` is at `+0x179`. Per Ghidra verification this session,
    `radiation` is at `+0x177`; `+0x179` is `AffectsAllies`. Fix when
    AffectsAllies parser eventually lands.
