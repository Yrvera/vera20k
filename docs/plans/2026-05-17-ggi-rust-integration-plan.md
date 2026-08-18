# Guardian GI (GGI) Rust Integration — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.
> Commit at the end of each task. Tasks are ordered by dependency.

**Goal:** Wire GGI into the engine to produce gamemd-indistinguishable observable behavior for walking M60 fire, deployed MissileLauncher fire, BFRT/IFV transport cases, and veteran/elite tier swaps.

**Architecture:** Six additive gap closures across rules parsing, deploy state, weapon selection, transport routing, and a new homing-missile movement module. All changes preserve the `sim/` → no `render/`/`ui/`/`audio/`/`net/` dependency invariant.

**Design Doc:** [`docs/plans/2026-05-17-ggi-rust-integration-design.md`](2026-05-17-ggi-rust-integration-design.md)
**Source research:** [`ra2-rust-game-docs/GGI_GHIDRA_REPORT.md`](../../../ra2-rust-game-docs/GGI_GHIDRA_REPORT.md)

---

## Grounding Summary

**What the docs already tell us:**
- `GGI_GHIDRA_REPORT.md` (9 sections, ~55KB, written 2026-05-17) covers the full GGI surface: parse path, deploy state machine, fire-frame anchor, eligibility (GetFireError), crush gate, BFRT/IFV routing, weapon/projectile/warhead readers, damage formula, missile homing flight curve, and the `ProneDamage`-is-dead-in-YR parity trap.
- `GI_GHIDRA_REPORT.md` (the E1 dossier; title corrected 2026-05-17) covers shared infantry infrastructure — reusable for everything not GGI-specific.
- `IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md` confirms `OpenTransportWeapon=1` semantics.
- `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`, `WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md`, `BULLETCLASS_INIT_AND_FIRE_GHIDRA_REPORT.md` cover generic type layout.

**What Ghidra confirmed during this investigation:**
- `Math__ftol @ 0x007c5f00` uses truncation toward zero (control word `0x0E7F`, RC bits 10-11 = `11`). Rust `as i32` cast matches exactly.
- `ProneDamage` (`WarheadType+0xF8`) is dead data — exhaustive byte-pattern sweep found zero runtime consumers.
- `InfantryClass::vtable+0x160` is `TechnoClass__IsIronCurtainActive` (inherited), NOT an InfantryClass override. Deploy-uncrushable enforcement is solely the `+0x2A4` byte check in `CanCrushCheck` Branch B.
- DeploySound/UndeploySound live on `TechnoTypeClass+0x56C/+0x570` (NOT on InfantryTypeClass `+0xEA4/+0xEA8` — those are Water sounds).
- ROT scaling: `BulletType+0x2DC` raw int, modulated by `cos((frame%15)*2π/15) × MissileROTVar + MissileROTVar + 1.0` (range 1.0–3.0).
- `BulletType+0x5A4/+0x5C8` (ProneFire/Tunnel) are actually `SequenceClass` fields reached via `InfantryType+0xE3C → SequenceClass +0x5A4/+0x5C8`. Corrects §3.3: deployed-fire anchor is `SecondaryFire` art-key when ProneFire is defined.

**Existing repo pattern this mirrors:**
- Veteran/Elite weapon swap (Gap D) — model is [`combat_weapon.rs:select_garrison_weapon`](../../src/sim/combat/combat_weapon.rs#L157-L197) which already does `is_elite = veterancy >= 200` with elite/base fallback.
- Art sequence data (Gap B) — extends the existing `art_data.rs` registry introduced by commit `1391629`.
- Homing missile module (Gap E) — parallel to existing [`rocket_movement.rs`](../../src/sim/movement/rocket_movement.rs) for ballistic projectiles.

**INI keys that drive the behavior** (rulesmd.ini + artmd.ini):
- `[GGI]` rules: `ElitePrimary=M60E`, `EliteSecondary=MissileLauncherE`, `OpenTransportWeapon=1`, `DeployedCrushable=no` (already parsed), `IFVMode=16`, voice keys.
- `[GGI]` art: `Cameo`, `Sequence=GuardianGISequence`, `FireUp=2`, `PrimaryFireFLH`, `SecondaryFireFLH`.
- `[GuardianGISequence]` art: `Deploy=300,15,0`, `Undeploy=180,2,2`, `Deployed=315,1,1`, `DeployedFire=323,6,6`, `FireProne=252,6,6`.
- Weapons: `[M60]/[M60E]` (damage 15→25), `[MissileLauncher]/[MissileLauncherE]` (damage 40→50, ROF 40→20, Speed 30→40).
- Warheads: `[SA]` (Verses 100,80,80,50,25,25,75,50,25,100,100), `[GUARDWH]` (Verses 20,20,20,100,50,100,10,10,10,100,100).
- Projectiles: `[InvisibleLow]` (Inviso), `[AAHeatSeeker2]` (AA, AG, Arm=2, ROT=60, Ranged, Image=DRAGON).
- `[General]` rules: `MissileROTVar=` (default 1.0 if absent).

**What's still unknown after grounding:**
- The actual projectile-spawn dispatch site in current code — the impact analysis flagged that `attach_rocket_state` is only called from tests, not production combat code. The homing module can be built in isolation; the dispatch hook may need its own follow-up if no spawn site exists yet. **Deferred to implementation discovery (Task 28).**
- Whether `MissileROTVar` is already parsed in any form (likely not). **Resolved at Task 26 — search and add.**
- Exact frames-per-tick conversion ratio: design assumed `80ms/frame ÷ 22ms/tick`, validated against existing 55-tick fallback for GGI's 15-frame deploy. Will hold to ±1 tick.

---

## Key Technical Decisions

- **Parallel homing module (not generalize)**: keep `rocket_movement.rs` untouched, add `homing_movement.rs` alongside. **Confidence: high.** Source: user choice during brainstorm.
- **`WeaponOverride` enum replaces `Option<u32> ifv_weapon_index`**: IFV-slot and OpenTransport routings have incompatible index semantics. **Confidence: high.** Source: GGI_GHIDRA_REPORT.md §3.7.
- **Veteran tier (100..200) does NOT swap weapons; only Elite (≥200)**: VeteranAbilities applies multipliers only. **Confidence: high.** Source: existing `select_garrison_weapon` pattern; GGI report §4.1.
- **Frames-to-ticks conversion `frames × 80 / 22`**: bounded ±1 tick. **Confidence: medium.** Source: existing `DEPLOY_DEFAULT_TICKS` comment + empirical match for GGI 15-frame deploy.
- **`SIDEWINDER_TABLE` precomputed at compile time (15 SimFixed entries)**: deterministic; replaces runtime cosine. **Confidence: high.** Source: GGI report §9.4.
- **`atan2_bam` uses f32 internally**: bounded jitter (≤±1 BAM) cannot flip the monotonic `<=` comparison in `within_rot_bam`. **Confidence: medium.** Source: design doc Architectural Decisions; replace with SimFixed BAM table only if lockstep desync surfaces.

Low-confidence items flagged for `/review-plan`: frames-to-ticks ratio, `atan2_bam` f32 use.

---

## Open Questions

### Resolved During Planning

- **ProneDamage application site** — RESOLVED: dead data in YR. Do not implement.
- **ROT INI-to-BAM scaling** — RESOLVED: `LowByte(ftol(ROT × sidewinder(frame))) << 8` per §9.4.
- **`+0x5A4/+0x5C8` attribution** — RESOLVED: SequenceClass fields, not BulletType. Affects deployed-fire anchor selection.
- **DeploySound offset** — RESOLVED: `TechnoTypeClass+0x56C/+0x570`. Tests confirm fields are parsed; only the emit order needs fixing.
- **IsImmuneToCrush hypothesis** — RESOLVED: refuted. Deploy gate is solely the `+0x2A4` byte read in `CanCrushCheck` Branch B.

### Deferred to Implementation

- **Projectile spawn dispatch site** — current code only calls `attach_rocket_state` from tests. The homing module can be built and tested in isolation; the production-code dispatch wire-up is a Task 28 follow-up that may itself need a small fire-pipeline change.
- **Exact tick rate for SHP frame conversion** — `80ms/frame ÷ 22ms/tick` is an approximation. Verify empirically against gamemd in-game once a GGI build runs end-to-end.
- **GGI deployed-fire spawn frame** — §9.5 says SecondaryFire art-key (absent → 0). Currently the fire-frame anchor uses the strict `==` mechanism from commit `1391629`. Verify the recent code picks `0` (not `2`) for GGI's deployed case during Task 9 test pass.
- **Whether `attach_rocket_state` callers exist outside `rocket_movement.rs::tests`** — confirm during Task 28; if not, the production dispatch is a separate plan.

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/world/world_commands.rs:517-562` | Reorder DeploySound emit before state write (Gap F) |
| Inspect | `src/sim/combat/combat_targeting.rs` | Verify no auto-deploy on target acquisition (Gap C) |
| Modify | `src/rules/object_type.rs` | Add `elite_primary`, `elite_secondary`, `open_transport_weapon` fields (Gaps D + G) |
| Modify | `src/sim/combat/combat_weapon.rs` | Add `WeaponOverride` enum + veterancy-aware select (Gaps D + G) |
| Modify | `src/sim/game_entity.rs` | Replace `ifv_weapon_index: Option<u32>` with `weapon_override: Option<WeaponOverride>` (Gap G) |
| Modify | `src/sim/passenger.rs:408-412` | Set `WeaponOverride::IfvSlot` or `OpenTransport` at boarding (Gap G) |
| Modify | `src/sim/combat/combat_targeting.rs`, `src/sim/combat/mod.rs` | Update `select_weapon*` callers (pass veterancy + new override type) (Gap D) |
| Modify | `src/rules/art_data.rs` | Add `deploy_frames`, `undeploy_frames`, `deployed_fire_frames` to art entry (Gap B) |
| Modify | `src/rules/ruleset.rs` | Parse sequence Length fields from artmd.ini (Gap B) |
| Modify | `src/sim/deploy.rs` | Add `DeployPhaseKind` enum + `frames_to_ticks` helper; change `compute_anim_ticks` signature (Gap B) |
| Modify | `src/sim/world/world_commands.rs:520,526` | Look up art entry, pass into `compute_anim_ticks` (Gap B) |
| Create | `src/sim/movement/homing_movement.rs` | New homing missile module (~300 LoC) (Gap E) |
| Modify | `src/sim/movement/mod.rs` | Export new module (Gap E) |
| Modify | `src/sim/game_entity.rs` | Add `homing_state: Option<HomingState>` field (Gap E) |
| Modify | `src/sim/world/mod.rs` | Call `tick_homing_movement` in "air + special movement" phase (Gap E) |
| Modify | `src/sim/world/world_hash.rs` | Include `homing_state` and `weapon_override` in deterministic hash (Gaps E + G) |
| Modify | `src/rules/ruleset.rs` | Parse `[General]MissileROTVar=` (Gap E) |

---

## Interface Changes

- **`select_weapon_with_ifv` → `select_weapon_with_override`**: signature changes from `(rules, obj, target_cat, armor, Option<u32>)` to `(rules, obj, target_cat, armor, veterancy: u16, Option<WeaponOverride>)`. Callers at [`combat_targeting.rs:204`](../../src/sim/combat/combat_targeting.rs#L204) and [`combat/mod.rs:1561`](../../src/sim/combat/mod.rs#L1561) updated atomically.
- **`select_weapon`**: gains `veterancy: u16` parameter. Same call sites.
- **`compute_anim_ticks(art: Option<&ArtEntry>, phase: DeployPhaseKind) -> u16`**: signature change. Callers at [`world_commands.rs:520,526`](../../src/sim/world/world_commands.rs#L520) updated.
- **`GameEntity.ifv_weapon_index: Option<u32>` → `weapon_override: Option<WeaponOverride>`**: rename + type change. All readers must update; grep `ifv_weapon_index` to find them.
- **`GameEntity.homing_state: Option<HomingState>`**: new field, additive. Serde default `None`.
- **`ObjectType.elite_primary`, `elite_secondary`, `open_transport_weapon`**: new optional fields with serde defaults. Additive; existing INI parsing unaffected.

---

## Sim Checklist

(All changes touch `sim/`)

- [x] All math uses `fixed`-point — homing module uses `SimFixed` for speed/altitude/vz/stall_ema; BAM angles are integer `u16`; render-only `pitch` stays `f32`.
- [x] New state included in deterministic state hash — `homing_state` and `weapon_override` added to `world_hash.rs`.
- [x] No dependencies on render/ui/sidebar/audio/net — verified module-by-module.
- [x] Tick ordering — `tick_homing_movement` runs in "air + special movement" phase alongside `tick_rocket_movement`. No new phase.
- [x] BTreeMap iteration order — homing module iterates `entities.keys_sorted()` matching the existing `rocket_movement.rs` pattern.

---

## Risk Areas

From the design's Impact Analysis:

1. **`WeaponOverride` rename (Gap G)** — touches every reader of `ifv_weapon_index`. Risk: missed reader silently breaks IFV firing. Mitigation: grep `ifv_weapon_index` before commit; compile error if any reader missed (type change forces it).
2. **`compute_anim_ticks` signature change (Gap B)** — only 2 callers, both in `world_commands.rs`. Risk: low.
3. **Homing module determinism (Gap E)** — sin/cos and atan2 use f32. Risk: lockstep desync. Mitigation: `SIDEWINDER_TABLE` precomputed; `atan2_bam` result truncated and only used in monotonic `<=` comparison.
4. **State hash schema bump** — adding new state requires bumping the hash schema version (existing pattern). Risk: replay-format breakage. Mitigation: bump version + document in commit.
5. **Frames-to-ticks approximation** — bounded ±1 tick. Risk: low; visual deploy may stutter by 1 frame at the transition.

---

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | DeploySound order (before state write) | Player hears the deploy sound; if it fires after state mutation, animation and audio drift by 1 tick | Inspect emit order in test |
| Task 1 | UndeploySound order | Same | Inspect emit order in test |
| Task 8 | BFRT-vs-IFV weapon routing | GGI inside BFRT fires its own MissileLauncher; inside IFV fires CRMissileLauncher; wrong routing = wrong damage numbers and wrong projectile | Unit test both routes |
| Task 10 | Elite weapon swap at exact threshold 200 | Elite GGI deals M60E damage (25) not M60 (15); 67% damage difference at the threshold | Unit test at v=199 and v=200 |
| Task 16 | Per-type deploy duration from art Length | Deploy animation and sim state stay in sync visually | Unit test 15-frame → 54-tick conversion |
| Task 21 | Sidewinder ROT formula | AAHeatSeeker2 missile flight visibly oscillates ("sidewinder" name); a constant ROT produces a different curve player can see | Unit test sidewinder table values; integration test missile arc shape |
| Task 22 | `vz >>= 2` damper when `Floater=0` | Missiles flatten quickly toward cruise altitude — the distinctive YR missile feel; without it missiles fly too steep | Unit test vz halves per tick |
| Task 23 | Cruise altitude dead-band ±20, snap ±18 | Missile altitude oscillation pattern at cruise | Unit test dz=±20 no-snap, dz=±21 snap |
| Task 24 | Stall-detect 60-frame window + EMA | Missile self-destruct when target unreachable — player sees missiles giving up gracefully, not flying forever | Unit test stationary unreachable target → self-destruct |
| Task 25 | Inclusive `<=` snap in `within_rot_bam` | Off-by-one would cause missile to over-rotate by one BAM step at the boundary, producing a wobble player could see | Unit test boundary case |

---

# Tasks

## Phase 1 — Quick wins

### Task 1: Reorder DeploySound/UndeploySound emit before state write (Gap F)

**Why:** Per GGI report §3.1, gamemd plays the deploy/undeploy voc BEFORE writing the Doing field. Current code writes state first then emits.

**Files:**
- Modify: `src/sim/world/world_commands.rs:517-562`

**Pattern:** No new pattern. Local reorder within the existing match arm.

**Step 1: Move sound emits to before state write**

Open `src/sim/world/world_commands.rs`, locate the deploy command handler (around line 510-562). Move the two `emit_deploy_sound` / `emit_undeploy_sound` blocks (lines 539-561) to immediately before line 537 `entity.deploy_state = new_phase;`.

After edit, the block should read:

```rust
// (lines 514-536 unchanged: compute new_phase + emit_deploy_sound + emit_undeploy_sound)

// Sound plays BEFORE state field write (matches gamemd Do_Action @ 0x0051d6f0).
if emit_deploy_sound {
    if let Some(sound_name) = deploy_sound {
        let sound_id = self.interner.intern(&sound_name);
        self.sound_events
            .push(crate::sim::world::SimSoundEvent::EntityDeployed {
                deploy_sound_id: sound_id,
                rx,
                ry,
            });
    }
}
if emit_undeploy_sound {
    if let Some(sound_name) = undeploy_sound {
        let sound_id = self.interner.intern(&sound_name);
        self.sound_events.push(
            crate::sim::world::SimSoundEvent::EntityUndeployed {
                undeploy_sound_id: sound_id,
                rx,
                ry,
            },
        );
    }
}
entity.deploy_state = new_phase;
true
```

**Step 2: Verify existing tests still pass**

Run: `cargo test deploy -- --nocapture`
Expected: PASS (existing tests verify presence of emit, not order).

**Step 3: Add test for emit-before-state order**

In `src/sim/deploy_tests.rs`, add:

```rust
#[test]
fn test_deploy_sound_emits_before_state_write() {
    // Verify the sim_sound_events buffer has the EntityDeployed event
    // recorded BEFORE the entity transitions to Deploying.
    //
    // gamemd reference: InfantryClass::Do_Action @ 0x0051d6f0 writes
    // the Doing field AFTER VocClass__PlayAt(DeploySound).
    let (mut sim, rules) = make_test_sim_with_gi();
    let entity_id = spawn_test_gi(&mut sim);

    // Pre-condition: entity has no deploy state.
    assert!(sim.entities.get(entity_id).unwrap().deploy_state.is_none());

    // Snapshot sound events before the command.
    let events_before = sim.sound_events.len();

    sim.execute_command(
        &Command::Deploy { entity_id },
        Some(&rules),
    );

    // After: state IS Deploying AND sound event WAS emitted.
    let entity = sim.entities.get(entity_id).unwrap();
    assert!(matches!(
        entity.deploy_state,
        Some(crate::sim::deploy::DeployPhase::Deploying { .. })
    ));

    // The new event is the EntityDeployed event.
    assert_eq!(sim.sound_events.len(), events_before + 1);
    assert!(matches!(
        sim.sound_events.last().unwrap(),
        crate::sim::world::SimSoundEvent::EntityDeployed { .. }
    ));
}
```

(Adapt helper names — `make_test_sim_with_gi`, `spawn_test_gi`, `execute_command` — to whatever the existing test harness exposes. If they don't exist, use the patterns from neighboring tests in the same file.)

**Step 4: Run new test**

Run: `cargo test test_deploy_sound_emits_before_state_write -- --nocapture`
Expected: PASS

**Step 5: Commit**

Commit message: `sim: reorder DeploySound emit before deploy state write`

---

### Task 2: Verify no auto-deploy on air-target acquisition (Gap C)

**Why:** Per GGI report §3.10, gamemd does NOT auto-deploy when an air target enters range. Verify current code doesn't either.

**Files:**
- Inspect: `src/sim/combat/combat_targeting.rs`
- Inspect: `src/ui/` or wherever cursor logic lives (locate during task)

**Pattern:** Inspection task. No code changes expected.

**Step 1: Grep for auto-deploy code paths**

Run:
```
grep -rn "DeployPhase::Deploying" src/sim/combat/
grep -rn "deploy_state = Some" src/sim/
grep -rn "Command::Deploy" src/sim/
```

Identify every writer of `deploy_state`. Expect: only `world_commands.rs` (player Command::Deploy) and `deploy.rs` (tick advance) should write it.

**Step 2: Inspect `combat_targeting.rs` for any deploy-related branch**

Open `src/sim/combat/combat_targeting.rs`. Search for `deploy` (case-insensitive). Expect: zero matches that would auto-initiate a deploy. If `select_weapon` is called from targeting, that's expected — it doesn't trigger deploy.

**Step 3: Inspect cursor / What_Action surface**

Grep for "What_Action", "cursor", "attack_cursor". If no such surface exists in `src/ui/` yet, document this as a follow-up.

**Step 4: Document findings**

Write a 5-line comment block in `src/sim/combat/combat_targeting.rs` near the top, summarizing:
- "GGI auto-deploy on air target: NOT present in current code. Verified against gamemd GGI_GHIDRA_REPORT.md §3.10."
- If a cursor surface exists, note its location and the AA-cursor finding.
- If a cursor surface does NOT yet exist, note that as a separate follow-up.

**Step 5: No code change → no test addition needed. Skip to commit.**

**Step 6: Commit**

Commit message: `combat: confirm no auto-deploy on air-target acquisition (GGI §3.10)`

---

## Phase 2 — ObjectType field additions (foundation for D & G)

### Task 3: Add `elite_primary` and `elite_secondary` fields to ObjectType (Gap D)

**Why:** GGI promotes M60→M60E and MissileLauncher→MissileLauncherE at Elite tier. Fields must exist before the weapon-select logic can use them.

**Files:**
- Modify: `src/rules/object_type.rs`

**Pattern:** Mirrors existing `elite_occupy_weapon` field (already in ObjectType).

**Step 1: Add struct fields**

Open `src/rules/object_type.rs`. Find the existing `elite_occupy_weapon: Option<String>` field. Add immediately above or below:

```rust
/// `ElitePrimary=` from rules.ini. Replaces `primary` when the unit is at
/// Elite tier (veterancy >= 200). Falls back to `primary` if absent.
///
/// gamemd reference: GGI_GHIDRA_REPORT.md §4.1.
#[serde(default)]
pub elite_primary: Option<String>,

/// `EliteSecondary=` from rules.ini. Replaces `secondary` when the unit is at
/// Elite tier (veterancy >= 200). Falls back to `secondary` if absent.
#[serde(default)]
pub elite_secondary: Option<String>,
```

**Step 2: Add INI parse**

Find the parse block where `primary` and `secondary` are read (search for `"Primary"` string literal in the file). Add right after them:

```rust
elite_primary: ini.get(section, "ElitePrimary").map(|s| s.to_string()),
elite_secondary: ini.get(section, "EliteSecondary").map(|s| s.to_string()),
```

Match the existing parsing idiom (likely `ini.get(...).map(|s| s.to_string())` or similar — copy the shape of how `primary` is parsed).

**Step 3: Add unit test**

In the same file's `#[cfg(test)] mod tests`, add:

```rust
#[test]
fn test_parses_elite_weapon_overrides() {
    let ini_str = "\
[InfantryTypes]
0=GGI

[GGI]
Name=Guardian GI
Cost=400
Strength=100
Armor=none
Primary=M60
Secondary=MissileLauncher
ElitePrimary=M60E
EliteSecondary=MissileLauncherE
";
    let ini = crate::rules::ini_parser::IniFile::from_str(ini_str);
    let rules = crate::rules::ruleset::RuleSet::from_ini(&ini).expect("parse");
    let ggi = rules.object("GGI").expect("GGI exists");
    assert_eq!(ggi.primary.as_deref(), Some("M60"));
    assert_eq!(ggi.secondary.as_deref(), Some("MissileLauncher"));
    assert_eq!(ggi.elite_primary.as_deref(), Some("M60E"));
    assert_eq!(ggi.elite_secondary.as_deref(), Some("MissileLauncherE"));
}

#[test]
fn test_elite_weapons_default_to_none() {
    let ini_str = "\
[InfantryTypes]
0=BASIC

[BASIC]
Name=Basic
Cost=100
Strength=50
Armor=none
Primary=M60
";
    let ini = crate::rules::ini_parser::IniFile::from_str(ini_str);
    let rules = crate::rules::ruleset::RuleSet::from_ini(&ini).expect("parse");
    let basic = rules.object("BASIC").expect("exists");
    assert_eq!(basic.elite_primary, None);
    assert_eq!(basic.elite_secondary, None);
}
```

**Step 4: Run tests**

Run: `cargo test test_parses_elite_weapon_overrides test_elite_weapons_default_to_none -- --nocapture`
Expected: PASS

**Step 5: Commit**

Commit message: `rules: parse ElitePrimary/EliteSecondary on ObjectType`

---

### Task 4: Add `open_transport_weapon` field to ObjectType (Gap G)

**Why:** BFRT-style open-topped transports (no Gunner=yes) need this field to know which passenger weapon to fire. Default -1 per gamemd ctor.

**Files:**
- Modify: `src/rules/object_type.rs`

**Pattern:** New field; default value provided by a `default_neg_one` helper (or use existing if one exists for similar int-with-sentinel fields).

**Step 1: Add struct field**

In `src/rules/object_type.rs`, near the existing `ifv_mode` field:

```rust
/// `OpenTransportWeapon=` from rules.ini. Index meaning when consumed:
///   - `0`  → fire passenger's Primary
///   - `1`  → fire passenger's Secondary
///   - `-1` → no override (default sentinel)
///
/// Only consumed when this unit is inside an open-topped transport that
/// does NOT have `Gunner=yes`. For Gunner transports, the passenger's
/// `IFVMode` + transport's `weapon_list[]` take over instead.
///
/// gamemd reference: GGI_GHIDRA_REPORT.md §3.7, TechnoTypeClass+0xD50,
/// default -1 in TechnoTypeClass ctor.
#[serde(default = "default_open_transport_weapon")]
pub open_transport_weapon: i32,
```

Below the struct definition (or wherever existing default helpers live):

```rust
fn default_open_transport_weapon() -> i32 { -1 }
```

**Step 2: Add INI parse**

Find the parse block where `ifv_mode` is read. Add nearby:

```rust
open_transport_weapon: ini
    .get(section, "OpenTransportWeapon")
    .and_then(|s| s.trim().parse::<i32>().ok())
    .unwrap_or(-1),
```

Match the existing int-parsing idiom for `ifv_mode` (copy shape).

**Step 3: Add unit test**

```rust
#[test]
fn test_parses_open_transport_weapon() {
    let ini_str = "\
[InfantryTypes]
0=GGI

[GGI]
Name=Guardian GI
Cost=400
Strength=100
Armor=none
Primary=M60
Secondary=MissileLauncher
OpenTransportWeapon=1
";
    let ini = crate::rules::ini_parser::IniFile::from_str(ini_str);
    let rules = crate::rules::ruleset::RuleSet::from_ini(&ini).expect("parse");
    let ggi = rules.object("GGI").expect("exists");
    assert_eq!(ggi.open_transport_weapon, 1);
}

#[test]
fn test_open_transport_weapon_defaults_to_neg_one() {
    let ini_str = "\
[InfantryTypes]
0=E1

[E1]
Name=GI
Cost=200
Strength=125
Armor=none
Primary=M60
";
    let ini = crate::rules::ini_parser::IniFile::from_str(ini_str);
    let rules = crate::rules::ruleset::RuleSet::from_ini(&ini).expect("parse");
    let e1 = rules.object("E1").expect("exists");
    assert_eq!(e1.open_transport_weapon, -1);
}
```

**Step 4: Run tests**

Run: `cargo test test_parses_open_transport_weapon test_open_transport_weapon_defaults_to_neg_one -- --nocapture`
Expected: PASS

**Step 5: Commit**

Commit message: `rules: parse OpenTransportWeapon (default -1) on ObjectType`

---

## Phase 3 — WeaponOverride enum + routing

### Task 5: Introduce `WeaponOverride` enum (Gap G)

**Why:** The single `Option<u32> ifv_weapon_index` collapsed two incompatible override semantics (transport's weapon_list[idx] vs passenger's primary/secondary by 0/1). A typed enum makes the routing explicit and caller-readable.

**Files:**
- Modify: `src/sim/combat/combat_weapon.rs`

**Pattern:** New small enum at top of file, mirrors existing `WeaponSlot` enum style.

**Step 1: Add enum near top of file**

In `src/sim/combat/combat_weapon.rs`, near the existing `WeaponSlot` enum (around line 31):

```rust
/// Weapon-selection override used by transport passengers.
///
/// Two transport semantics are distinguished:
///
/// - **`IfvSlot(idx)`** — Gunner=yes transports (e.g., IFV). The transport
///   fires its own `weapon_list[idx]` where `idx` is the passenger's IFVMode.
///   The attacker passed to `select_weapon_*` is the TRANSPORT's ObjectType.
///
/// - **`OpenTransport(slot)`** — Open-topped non-Gunner transports (e.g., BFRT).
///   The transport fires the passenger's own Primary (slot=0) or Secondary
///   (slot=1) per the passenger's `OpenTransportWeapon=` INI value.
///   The attacker passed to `select_weapon_*` is the PASSENGER's ObjectType.
///
/// gamemd reference: GGI_GHIDRA_REPORT.md §3.7.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WeaponOverride {
    /// Transport's weapon_list[idx], used when transport is `Gunner=yes`.
    IfvSlot(u32),
    /// Passenger's own primary (0) or secondary (1), used for open-topped
    /// non-Gunner transports with `OpenTransportWeapon != -1`.
    OpenTransport(u32),
}
```

**Step 2: Test enum invariants**

```rust
#[test]
fn test_weapon_override_variants() {
    // Just verify the enum compiles + serialization round-trips.
    let ifv = WeaponOverride::IfvSlot(16);
    let open = WeaponOverride::OpenTransport(1);
    let ifv_json = serde_json::to_string(&ifv).unwrap();
    let open_json = serde_json::to_string(&open).unwrap();
    let ifv_back: WeaponOverride = serde_json::from_str(&ifv_json).unwrap();
    let open_back: WeaponOverride = serde_json::from_str(&open_json).unwrap();
    assert_eq!(ifv, ifv_back);
    assert_eq!(open, open_back);
}
```

(If `serde_json` is not a dependency, omit the serialization round-trip and just check the construction and equality.)

**Step 3: Run test**

Run: `cargo test test_weapon_override_variants -- --nocapture`
Expected: PASS

**Step 4: Commit**

Commit message: `combat: add WeaponOverride enum (IfvSlot / OpenTransport)`

---

### Task 6: Rename `GameEntity.ifv_weapon_index` → `weapon_override`; bump state hash (Gap G)

**Why:** The field type changes from `Option<u32>` to `Option<WeaponOverride>`. The compiler will surface every reader so nothing silently breaks.

**Files:**
- Modify: `src/sim/game_entity.rs`
- Modify: `src/sim/world/world_hash.rs`
- Modify: `src/sim/passenger.rs` (callers)
- Modify: `src/sim/combat/combat_targeting.rs` (callers)
- Modify: `src/sim/combat/mod.rs` (callers)

**Pattern:** Field rename + type change. Compile errors guide all updates.

**Step 1: Update `GameEntity`**

In `src/sim/game_entity.rs`, locate `ifv_weapon_index: Option<u32>`. Replace with:

```rust
/// Weapon-selection override applied when this entity is acting as a
/// transport firing a passenger's weapon. See `WeaponOverride` for the
/// semantics of each variant.
///
/// Set by `passenger.rs::tick_loading` when a passenger boards.
/// Cleared when the transport is empty.
#[serde(default)]
pub weapon_override: Option<crate::sim::combat::combat_weapon::WeaponOverride>,
```

Remove the old `ifv_weapon_index` field entirely.

**Step 2: Update state hash**

In `src/sim/world/world_hash.rs`, find where `ifv_weapon_index` is hashed. Replace the hashed value with `weapon_override` using the existing serde-driven hashing or whatever idiom is present. If the file uses `#[derive(Hash)]` on a snapshot struct, `WeaponOverride` already derives the needed traits.

**Step 3: Find all readers**

Run:
```
grep -rn "ifv_weapon_index" src/
```

For each match, update to use `weapon_override`. Most readers in combat will need to dispatch on the variant — see Task 8.

**Step 4: Bump hash schema version**

If the codebase has a state-hash schema version constant (search for `HASH_SCHEMA_VERSION` or similar), bump it by 1. Document the bump in the commit message.

**Step 5: Run compilation**

Run: `cargo check`
Expected: compilation errors at every old `ifv_weapon_index` reader. Fix them by reading the new field — for now, just rename references; Task 8 wires the variant dispatch properly. Use placeholder code where dispatch logic is needed:

```rust
// Temporary placeholder — proper variant dispatch added in Task 8.
let ifv_idx = match attacker.weapon_override {
    Some(crate::sim::combat::combat_weapon::WeaponOverride::IfvSlot(i)) => Some(i),
    _ => None,
};
```

Use this exact placeholder shape in `combat_targeting.rs` and `combat/mod.rs` so existing IFV tests pass.

**Step 6: Run existing tests**

Run: `cargo test`
Expected: all existing IFV tests pass (the placeholder preserves IfvSlot behavior).

**Step 7: Commit**

Commit message: `sim: rename ifv_weapon_index → weapon_override; bump hash schema`

---

### Task 7: Update `passenger.rs` to set `WeaponOverride` based on Gunner flag (Gap G)

**Why:** Boarding logic must distinguish IFV (Gunner=yes) from BFRT-style (OpenTopped + no Gunner) when setting the transport's weapon override.

**Files:**
- Modify: `src/sim/passenger.rs:408-412`

**Pattern:** Local branch on existing `transport_gunner` flag, additional read of new `open_transport_weapon` field.

**Step 1: Replace boarding override block**

In `src/sim/passenger.rs`, locate the existing block (around lines 408-412):

```rust
// IFV weapon swap: if transport is Gunner=yes, set weapon index.
if transport_gunner {
    if let Some(t) = sim.entities.get_mut(transport_id) {
        t.ifv_weapon_index = Some(pax_ifv_mode);
    }
}
```

Replace with:

```rust
// Transport weapon override: distinguish IFV (Gunner=yes) from BFRT-style
// (OpenTopped + no Gunner). gamemd reference: GGI_GHIDRA_REPORT.md §3.7.
let new_override = if transport_gunner {
    Some(crate::sim::combat::combat_weapon::WeaponOverride::IfvSlot(pax_ifv_mode))
} else if transport_open_topped && pax_open_transport_weapon >= 0 {
    Some(crate::sim::combat::combat_weapon::WeaponOverride::OpenTransport(
        pax_open_transport_weapon as u32,
    ))
} else {
    None
};
if let Some(t) = sim.entities.get_mut(transport_id) {
    t.weapon_override = new_override;
}
```

**Step 2: Add or look up `transport_open_topped` and `pax_open_transport_weapon`**

Search the surrounding function for `transport_gunner` to find where it's computed. Add similar reads for the new flags. The transport's `open_topped` flag should already exist on ObjectType (check via grep). The passenger's `open_transport_weapon` is the field added in Task 4.

If `transport_open_topped` doesn't yet exist as a parsed field on ObjectType: grep for `OpenTopped` in `rules/object_type.rs`. If absent, add it in this task (mirror of `gunner` field). Otherwise skip.

**Step 3: Add unloading/clear logic**

Find the matching unloading or transport-death path that should reset the override. Add:

```rust
if let Some(t) = sim.entities.get_mut(transport_id) {
    t.weapon_override = None;
}
```

at the point where the transport becomes empty.

**Step 4: Compile and run existing tests**

Run: `cargo check && cargo test passenger`
Expected: PASS (existing IFV tests still pass — they use IfvSlot variant via the Task 6 placeholder).

**Step 5: Commit**

Commit message: `sim: route WeaponOverride by Gunner/OpenTopped on passenger boarding`

---

### Task 8: Add `select_weapon_with_override` with variant dispatch + tier helpers (Gaps D + G)

**Why:** Replaces `select_weapon_with_ifv`. Handles all four cases: IFV slot, OpenTransport primary/secondary, no override, plus elite-tier swap.

**Files:**
- Modify: `src/sim/combat/combat_weapon.rs`

**Pattern:** Mirrors existing `select_garrison_weapon` (which already has `is_elite = veterancy >= 200` logic).

**Step 1: Add tier helpers**

Near the top of `combat_weapon.rs` (after the `WeaponOverride` enum from Task 5):

```rust
/// Returns the weapon ID for the unit's primary slot at the given veterancy.
/// Elite (>=200) prefers `elite_primary` with fallback to `primary`.
/// gamemd reference: GGI_GHIDRA_REPORT.md §4.1.
pub(crate) fn primary_for_tier(obj: &ObjectType, veterancy: u16) -> Option<&str> {
    let is_elite = veterancy >= 200;
    if is_elite {
        obj.elite_primary.as_deref().or(obj.primary.as_deref())
    } else {
        obj.primary.as_deref()
    }
}

/// Same for secondary. Elite prefers `elite_secondary` then `secondary`.
pub(crate) fn secondary_for_tier(obj: &ObjectType, veterancy: u16) -> Option<&str> {
    let is_elite = veterancy >= 200;
    if is_elite {
        obj.elite_secondary.as_deref().or(obj.secondary.as_deref())
    } else {
        obj.secondary.as_deref()
    }
}
```

**Step 2: Replace `select_weapon` and `select_weapon_with_ifv` with new signatures**

Update `select_weapon` to take veterancy:

```rust
pub(crate) fn select_weapon<'a>(
    rules: &'a RuleSet,
    attacker_obj: &'a ObjectType,
    target_category: EntityCategory,
    target_armor: &str,
    veterancy: u16,
) -> Option<SelectedWeapon<'a>> {
    select_weapon_with_override(rules, attacker_obj, target_category, target_armor, veterancy, None)
}
```

Replace `select_weapon_with_ifv` with `select_weapon_with_override`:

```rust
pub(crate) fn select_weapon_with_override<'a>(
    rules: &'a RuleSet,
    attacker_obj: &'a ObjectType,
    target_category: EntityCategory,
    target_armor: &str,
    veterancy: u16,
    override_: Option<WeaponOverride>,
) -> Option<SelectedWeapon<'a>> {
    // Override dispatch first.
    match override_ {
        Some(WeaponOverride::IfvSlot(idx)) => {
            // For IfvSlot, attacker_obj is the TRANSPORT — read its weapon_list.
            if let Some(weapon_id) = attacker_obj.weapon_list.get(idx as usize) {
                if let Some(result) = try_weapon(
                    rules,
                    weapon_id,
                    target_category,
                    target_armor,
                    WeaponSlot::Primary,
                ) {
                    return Some(result);
                }
            }
            // IFV slot failed — fall through to default primary/secondary on transport.
        }
        Some(WeaponOverride::OpenTransport(slot)) => {
            // For OpenTransport, attacker_obj is the PASSENGER — fire its
            // primary (0) or secondary (1) directly, no fallback.
            let weapon_id = match slot {
                0 => primary_for_tier(attacker_obj, veterancy),
                1 => secondary_for_tier(attacker_obj, veterancy),
                _ => None,
            };
            if let Some(wid) = weapon_id {
                return try_weapon(
                    rules,
                    wid,
                    target_category,
                    target_armor,
                    if slot == 0 { WeaponSlot::Primary } else { WeaponSlot::Secondary },
                );
            }
            return None;
        }
        None => {} // fall through
    }

    // Default Primary → Secondary path with tier-aware weapon ID lookup.
    if let Some(wid) = primary_for_tier(attacker_obj, veterancy) {
        if let Some(result) = try_weapon(
            rules,
            wid,
            target_category,
            target_armor,
            WeaponSlot::Primary,
        ) {
            return Some(result);
        }
    }
    if let Some(wid) = secondary_for_tier(attacker_obj, veterancy) {
        if let Some(result) = try_weapon(
            rules,
            wid,
            target_category,
            target_armor,
            WeaponSlot::Secondary,
        ) {
            return Some(result);
        }
    }
    None
}
```

Remove the old `select_weapon_with_ifv` function entirely.

**Step 3: Update existing tests in same file**

Update `select_weapon` callers in this file's `#[cfg(test)] mod tests` to pass `0` for veterancy:

```rust
let result = select_weapon(&rules, ifv, EntityCategory::Unit, "light", 0);
```

**Step 4: Compile**

Run: `cargo check`
Expected: compile errors at external callers — these are addressed in Task 9.

**Step 5: Run in-file tests**

Run: `cargo test --package <crate> combat_weapon -- --nocapture`
Expected: existing in-file tests pass (with veterancy=0 added to calls).

**Step 6: Commit**

Commit message: `combat: add select_weapon_with_override (variants + tier swap)`

---

### Task 9: Update external `select_weapon*` callers (Gaps D + G)

**Why:** Compile error from Task 8 — every caller in `combat_targeting.rs` and `combat/mod.rs` must pass veterancy + the new override type.

**Files:**
- Modify: `src/sim/combat/combat_targeting.rs` (around line 204)
- Modify: `src/sim/combat/mod.rs` (around line 1561)

**Pattern:** Mechanical signature update. The Task 6 placeholder code needs to be replaced with proper `WeaponOverride` pass-through.

**Step 1: Update `combat_targeting.rs:204`**

Locate the existing `select_weapon_with_ifv` call. Replace with:

```rust
let selected = select_weapon_with_override(
    rules,
    attacker_obj,
    target_category,
    target_armor,
    attacker.veterancy,
    attacker.weapon_override,
);
```

Remove the Task 6 placeholder match-on-ifv_idx.

**Step 2: Update `combat/mod.rs:1561`**

Same change — pass `snap.veterancy` and `snap.weapon_override` (or equivalent — match the snapshot type used at the call site).

If the snapshot struct doesn't yet carry `weapon_override`, add it as a `Copy`-able field:

```rust
// In whichever struct snapshots entity combat state:
pub weapon_override: Option<WeaponOverride>,
pub veterancy: u16,
```

And populate it at the snapshot construction site.

**Step 3: Compile**

Run: `cargo check`
Expected: PASS (no more compile errors).

**Step 4: Run all tests**

Run: `cargo test`
Expected: all pass. Existing IFV tests should produce the same result because `WeaponOverride::IfvSlot(idx)` reproduces the old `Option<u32>` semantics.

**Step 5: Commit**

Commit message: `combat: update select_weapon callers to pass veterancy + override`

---

### Task 10: Tests — Elite tier swap and BFRT vs IFV routing (Gaps D + G)

**Why:** Lock in the parity-critical behaviors with unit tests so future changes can't silently break them.

**Files:**
- Modify: `src/sim/combat/combat_weapon.rs` (test module)

**Pattern:** Mirror existing tests in the same file.

**Step 1: Add elite-swap tests**

In the `#[cfg(test)] mod tests` block:

```rust
#[test]
fn test_rookie_uses_base_primary() {
    let rules = make_ggi_rules();
    let ggi = rules.object("GGI").unwrap();
    let sel = select_weapon(&rules, ggi, EntityCategory::Infantry, "none", 0).unwrap();
    assert_eq!(sel.weapon_id, "M60");
}

#[test]
fn test_veteran_still_uses_base_primary() {
    // Veteran tier (100..200) does NOT swap weapons in gamemd.
    let rules = make_ggi_rules();
    let ggi = rules.object("GGI").unwrap();
    let sel = select_weapon(&rules, ggi, EntityCategory::Infantry, "none", 199).unwrap();
    assert_eq!(sel.weapon_id, "M60", "veteran at v=199 must still fire M60");
}

#[test]
fn test_elite_uses_elite_primary_at_threshold() {
    let rules = make_ggi_rules();
    let ggi = rules.object("GGI").unwrap();
    let sel = select_weapon(&rules, ggi, EntityCategory::Infantry, "none", 200).unwrap();
    assert_eq!(sel.weapon_id, "M60E", "elite at v=200 must fire M60E");
}

#[test]
fn test_elite_uses_elite_secondary() {
    // GGI deployed (forced via Secondary slot) at elite tier fires MissileLauncherE.
    let rules = make_ggi_rules();
    let ggi = rules.object("GGI").unwrap();
    let sel = select_weapon_with_override(
        &rules,
        ggi,
        EntityCategory::Aircraft,
        "light",
        200,
        Some(WeaponOverride::OpenTransport(1)),
    )
    .unwrap();
    assert_eq!(sel.weapon_id, "MissileLauncherE");
}
```

**Step 2: Add `make_ggi_rules` test helper**

In the same test module:

```rust
fn make_ggi_rules() -> RuleSet {
    let ini_str = "\
[InfantryTypes]
0=GGI

[GGI]
Name=Guardian GI
Cost=400
Strength=100
Armor=none
Primary=M60
Secondary=MissileLauncher
ElitePrimary=M60E
EliteSecondary=MissileLauncherE
OpenTransportWeapon=1

[M60]
Damage=15
ROF=20
Range=4
Projectile=InvisibleLow
Warhead=SA

[M60E]
Damage=25
ROF=20
Range=4
Projectile=InvisibleLow
Warhead=SA

[MissileLauncher]
Damage=40
ROF=40
Range=8
Projectile=AAHeatSeeker2
Warhead=GUARDWH

[MissileLauncherE]
Damage=50
ROF=20
Range=8
Projectile=AAHeatSeeker2
Warhead=GUARDWH

[InvisibleLow]
AG=yes
AA=no

[AAHeatSeeker2]
AG=yes
AA=yes
ROT=60
Arm=2

[SA]
Verses=100%,80%,80%,50%,25%,25%,75%,50%,25%,100%,100%

[GUARDWH]
Verses=20%,20%,20%,100%,50%,100%,10%,10%,10%,100%,100%
";
    let ini = crate::rules::ini_parser::IniFile::from_str(ini_str);
    RuleSet::from_ini(&ini).expect("parse GGI test rules")
}
```

**Step 3: Add BFRT-style OpenTransport routing test**

```rust
#[test]
fn test_open_transport_routes_to_passenger_secondary() {
    // GGI inside BFRT (no Gunner): override = OpenTransport(1) → fires GGI's
    // own Secondary = MissileLauncher (AA-capable). Without override, GGI's
    // Primary (M60) would be rejected against Aircraft for AA=no.
    let rules = make_ggi_rules();
    let ggi = rules.object("GGI").unwrap();
    let sel = select_weapon_with_override(
        &rules,
        ggi,
        EntityCategory::Aircraft,
        "light",
        0,
        Some(WeaponOverride::OpenTransport(1)),
    )
    .unwrap();
    assert_eq!(sel.weapon_id, "MissileLauncher");
}

#[test]
fn test_open_transport_primary_fires_passenger_primary() {
    let rules = make_ggi_rules();
    let ggi = rules.object("GGI").unwrap();
    let sel = select_weapon_with_override(
        &rules,
        ggi,
        EntityCategory::Infantry,
        "none",
        0,
        Some(WeaponOverride::OpenTransport(0)),
    )
    .unwrap();
    assert_eq!(sel.weapon_id, "M60");
}
```

**Step 4: Run tests**

Run: `cargo test --package <crate> combat_weapon -- --nocapture`
Expected: all PASS.

**Step 5: Commit**

Commit message: `combat: tests for Elite tier swap + OpenTransport routing`

---

## Phase 4 — Art-driven deploy duration

### Task 11: Add sequence frame fields to `ArtEntry` (Gap B)

**Why:** Per-type deploy/undeploy/deployed-fire frame counts must come from artmd.ini, not the hardcoded 55-tick fallback.

**Files:**
- Modify: `src/rules/art_data.rs`

**Pattern:** Additive fields on the existing art registry from commit `1391629`.

**Step 1: Open `src/rules/art_data.rs` and locate the per-image entry struct**

(Likely named `ArtEntry`, `ArtData`, or similar. Find it via grep or by reading the file.)

**Step 2: Add new fields**

Add to the struct:

```rust
/// Middle integer of `Deploy=<start>,<frames>,<rate>` in artmd.ini sequence
/// section. `None` when the sequence doesn't define a Deploy entry.
/// gamemd reference: GGI_GHIDRA_REPORT.md §3.1 — drives Deploy → Deployed
/// transition timing.
#[serde(default)]
pub deploy_frames: Option<u16>,

/// Middle integer of `Undeploy=<start>,<frames>,<rate>`.
#[serde(default)]
pub undeploy_frames: Option<u16>,

/// Middle integer of `DeployedFire=<start>,<frames>,<rate>`. Used by the
/// fire-frame anchor selection when this entity is in DeployedFire state.
#[serde(default)]
pub deployed_fire_frames: Option<u16>,
```

**Step 3: Test default values**

```rust
#[test]
fn test_art_entry_sequence_frames_default_none() {
    let entry: ArtEntry = Default::default();  // or however a default ArtEntry is constructed
    assert_eq!(entry.deploy_frames, None);
    assert_eq!(entry.undeploy_frames, None);
    assert_eq!(entry.deployed_fire_frames, None);
}
```

(Adapt the test to whatever construction pattern `ArtEntry` exposes — `Default`, `ArtEntry::new()`, etc.)

**Step 4: Run test**

Run: `cargo test test_art_entry_sequence_frames_default_none -- --nocapture`
Expected: PASS

**Step 5: Commit**

Commit message: `rules/art: add deploy/undeploy/deployed_fire frame counts`

---

### Task 12: Parse sequence Length fields from artmd.ini (Gap B)

**Why:** Wire INI input into the fields added in Task 11.

**Files:**
- Modify: `src/rules/ruleset.rs` (or wherever artmd.ini sequences are parsed — find via grep "Sequence" or "[GuardianGISequence]")

**Pattern:** Extend the existing sequence parser to extract the middle integer of `Key=<start>,<frames>,<rate>` for three specific keys.

**Step 1: Locate the artmd sequence parser**

Run: `grep -rn "Sequence=" src/rules/`
Find where the sequence section is read (likely a per-image lookup that opens the named sequence section).

**Step 2: Add parsing helper**

In the same file, add or extend:

```rust
/// Parse a sequence entry value of the form `<start>,<frames>,<rate>` and
/// return the middle integer (frame count). Returns `None` on malformed input.
fn parse_sequence_frames(value: &str) -> Option<u16> {
    let mut parts = value.split(',').map(str::trim);
    let _start = parts.next()?;
    let frames = parts.next()?.parse::<u16>().ok()?;
    Some(frames)
}
```

**Step 3: Read the three keys from the sequence section**

Where the sequence section is loaded into an ArtEntry, add:

```rust
entry.deploy_frames = sequence_section
    .get("Deploy")
    .and_then(|v| parse_sequence_frames(v));
entry.undeploy_frames = sequence_section
    .get("Undeploy")
    .and_then(|v| parse_sequence_frames(v));
entry.deployed_fire_frames = sequence_section
    .get("DeployedFire")
    .and_then(|v| parse_sequence_frames(v));
```

Match the existing idiom for whatever sequence-section accessor the file uses.

**Step 4: Test against GGI sequence**

In the same file's `#[cfg(test)] mod tests`:

```rust
#[test]
fn test_parses_guardian_gi_sequence_frames() {
    let ini_str = "\
[GuardianGISequence]
Ready=0,1,1
Walk=8,6,6
Deploy=300,15,0
Undeploy=180,2,2
Deployed=315,1,1
DeployedFire=323,6,6
FireUp=204,6,6
FireProne=252,6,6
";
    // Parse via whatever the existing test pattern uses for sequence sections.
    let ini = crate::rules::ini_parser::IniFile::from_str(ini_str);
    let entry = parse_sequence_into_art_entry("GuardianGISequence", &ini);  // adapt to real API
    assert_eq!(entry.deploy_frames, Some(15));
    assert_eq!(entry.undeploy_frames, Some(2));
    assert_eq!(entry.deployed_fire_frames, Some(6));
}
```

If no public helper exists to parse a single sequence section in isolation, write a thin test wrapper that calls the relevant internal pieces.

**Step 5: Run test**

Run: `cargo test test_parses_guardian_gi_sequence_frames -- --nocapture`
Expected: PASS

**Step 6: Commit**

Commit message: `rules: parse Deploy/Undeploy/DeployedFire frame counts from artmd.ini`

---

### Task 13: Add `DeployPhaseKind` enum + `frames_to_ticks` helper; change `compute_anim_ticks` signature (Gap B)

**Why:** `compute_anim_ticks` must take per-type art data and a phase enum so it can return the right frame count.

**Files:**
- Modify: `src/sim/deploy.rs`

**Pattern:** Extends existing module. The fallback constant `DEPLOY_DEFAULT_TICKS` remains for when no art data is available.

**Step 1: Add enum + helper above `compute_anim_ticks`**

In `src/sim/deploy.rs`:

```rust
/// Which phase of the deploy state machine to look up frames for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeployPhaseKind {
    Deploying,
    Undeploying,
}

/// Convert SHP animation frames to sim ticks.
///
/// gamemd animates at ~80 ms per SHP frame; our sim runs at SIM_TICK_MS=22.
/// Ratio ≈ 80/22 = 3.64. For GGI's `Deploy=15` frames → 54 ticks (matches
/// the existing 55-tick fallback within ±1, validating the approximation).
///
/// Bounded approximation: ±1 tick. Acceptable because gamemd's animation
/// Rate field isn't fully modeled in this engine and the visual deploy
/// duration is dominated by SHP framerate, not sim tick rate.
pub(crate) fn frames_to_ticks(frames: u16) -> u16 {
    ((frames as u32) * 80 / 22) as u16
}
```

**Step 2: Replace `compute_anim_ticks` with art-aware signature**

Replace the existing function:

```rust
/// Resolve the number of sim ticks the deploy or undeploy phase should run.
///
/// Reads the per-type art-INI sequence frame count when available;
/// falls back to `DEPLOY_DEFAULT_TICKS` when no art entry exists or the
/// sequence doesn't define the requested phase.
pub(crate) fn compute_anim_ticks(
    art: Option<&crate::rules::art_data::ArtEntry>,
    phase: DeployPhaseKind,
) -> u16 {
    let frames = art.and_then(|a| match phase {
        DeployPhaseKind::Deploying => a.deploy_frames,
        DeployPhaseKind::Undeploying => a.undeploy_frames,
    });
    frames.map(frames_to_ticks).unwrap_or(DEPLOY_DEFAULT_TICKS)
}
```

(Adapt the `ArtEntry` import path to whatever the actual module is.)

**Step 3: Compile**

Run: `cargo check`
Expected: errors at `world_commands.rs:520,526` (the two callers). Fixed in Task 14.

**Step 4: Add unit tests for the new helper**

In `src/sim/deploy.rs` test module:

```rust
#[test]
fn test_frames_to_ticks_ggi_deploy() {
    // 15-frame deploy converts to 54 ticks (gamemd-equivalent ~1200ms).
    assert_eq!(frames_to_ticks(15), 54);
}

#[test]
fn test_frames_to_ticks_short_undeploy() {
    // 2-frame undeploy → 7 ticks (~160ms).
    assert_eq!(frames_to_ticks(2), 7);
}

#[test]
fn test_compute_anim_ticks_no_art_falls_back() {
    let ticks = compute_anim_ticks(None, DeployPhaseKind::Deploying);
    assert_eq!(ticks, DEPLOY_DEFAULT_TICKS);
}

#[test]
fn test_compute_anim_ticks_uses_art_deploy_frames() {
    let mut art = crate::rules::art_data::ArtEntry::default();
    art.deploy_frames = Some(15);
    art.undeploy_frames = Some(2);
    assert_eq!(compute_anim_ticks(Some(&art), DeployPhaseKind::Deploying), 54);
    assert_eq!(compute_anim_ticks(Some(&art), DeployPhaseKind::Undeploying), 7);
}

#[test]
fn test_compute_anim_ticks_missing_phase_falls_back() {
    let mut art = crate::rules::art_data::ArtEntry::default();
    art.deploy_frames = Some(15);
    // undeploy_frames intentionally None → fallback for undeploy.
    assert_eq!(compute_anim_ticks(Some(&art), DeployPhaseKind::Undeploying), DEPLOY_DEFAULT_TICKS);
}
```

**Step 5: Run new tests**

Run: `cargo test --package <crate> deploy -- --nocapture`
Expected: PASS (callers still have compile errors but the in-file tests for the helper pass).

**Step 6: Commit (or chain to Task 14 first — see Task 14 Step 1)**

Commit message: `sim: art-aware compute_anim_ticks (DeployPhaseKind + frames_to_ticks)`

---

### Task 14: Update `world_commands.rs` callers of `compute_anim_ticks` (Gap B)

**Why:** Fix compile errors from Task 13's signature change.

**Files:**
- Modify: `src/sim/world/world_commands.rs:520,526`

**Pattern:** Look up art entry for the entity's image alias from the rules registry.

**Step 1: Update the Deploy command branch**

In the deploy command handler (around line 514-536):

Replace line 520:

```rust
ticks_remaining: crate::sim::deploy::compute_anim_ticks(),
```

With:

```rust
ticks_remaining: crate::sim::deploy::compute_anim_ticks(
    rules.and_then(|r| r.art_for_object(entity)),  // adapt to real lookup API
    crate::sim::deploy::DeployPhaseKind::Deploying,
),
```

And replace line 526 similarly with `DeployPhaseKind::Undeploying`.

**Step 2: Locate the art-lookup API**

If `r.art_for_object(entity)` doesn't exist:
- Get the entity's image alias / type_id (whatever links it to the art entry).
- Call the existing art-lookup method (search `art_data.rs` exports for `get`, `lookup`, etc.).

If no convenient API exists, write a small helper in `art_data.rs`:

```rust
/// Look up the ArtEntry for a given image alias. Returns None if not found.
pub fn lookup(&self, image_alias: &str) -> Option<&ArtEntry> {
    self.entries.get(image_alias)
}
```

**Step 3: Pass the rules reference**

If `rules` isn't yet a parameter of the surrounding function, add it. Check the signature — most command handlers already receive `rules: Option<&RuleSet>` based on the surrounding code.

**Step 4: Compile**

Run: `cargo check`
Expected: PASS.

**Step 5: Run deploy tests**

Run: `cargo test deploy`
Expected: PASS — existing tests use entities without art entries (fall back to `DEPLOY_DEFAULT_TICKS=55`), so behavior is preserved.

**Step 6: Commit**

Commit message: `sim: pass art entry to compute_anim_ticks at command intake`

---

### Task 15: Integration test — GGI deploy reads 54 ticks from art (Gap B)

**Why:** End-to-end verification that art INI → ObjectType art lookup → compute_anim_ticks produces the right value.

**Files:**
- Modify: `src/sim/deploy_tests.rs`

**Pattern:** Existing deploy tests use a test sim + spawn helper.

**Step 1: Add test**

```rust
#[test]
fn test_ggi_deploy_uses_art_frame_count() {
    // GGI's GuardianGISequence has Deploy=300,15,0 — 15 frames.
    // 15 * 80 / 22 = 54 ticks.
    let (mut sim, rules) = make_test_sim_with_ggi();
    let ggi_id = spawn_test_ggi(&mut sim);

    sim.execute_command(&Command::Deploy { entity_id: ggi_id }, Some(&rules));

    let entity = sim.entities.get(ggi_id).unwrap();
    match entity.deploy_state {
        Some(crate::sim::deploy::DeployPhase::Deploying { ticks_remaining }) => {
            assert_eq!(ticks_remaining, 54, "GGI deploy = 15 frames * 80 / 22 = 54 ticks");
        }
        other => panic!("expected Deploying, got {:?}", other),
    }
}
```

(`make_test_sim_with_ggi` / `spawn_test_ggi` — extend the existing test harness with a GGI fixture if it doesn't exist. Use the rules from Task 10's `make_ggi_rules` plus the GuardianGISequence art entry.)

**Step 2: Run test**

Run: `cargo test test_ggi_deploy_uses_art_frame_count -- --nocapture`
Expected: PASS.

**Step 3: Commit**

Commit message: `sim: integration test — GGI deploy uses 54-tick art-derived duration`

---

## Phase 5 — Homing missile module (Gap E)

### Task 16: Create `homing_movement.rs` skeleton with `HomingState` + `HomingPhase` (Gap E)

**Why:** Foundation for the homing module. Establish struct + enum + module exports.

**Files:**
- Create: `src/sim/movement/homing_movement.rs`
- Modify: `src/sim/movement/mod.rs`
- Modify: `src/sim/game_entity.rs` (add `homing_state` field)

**Pattern:** Mirrors `src/sim/movement/rocket_movement.rs` structure.

**Step 1: Create the module file**

Create `src/sim/movement/homing_movement.rs`:

```rust
//! Homing missile flight — per-tick yaw correction toward a tracked target.
//!
//! Used for projectiles with `Ranged=yes` (BulletType+0x2A0), such as
//! AAHeatSeeker2 fired by Guardian GI's MissileLauncher. Distinct from
//! `rocket_movement.rs` which handles ballistic-arc projectiles (V3,
//! dumb-fire) — keep them separate; do not merge.
//!
//! gamemd reference: GGI_GHIDRA_REPORT.md §3.7 / §8.1 / §9.4.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/entity_store, sim/game_entity.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::sim::entity_store::EntityStore;
use crate::util::fixed_math::SimFixed;

/// Phase within the homing missile state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HomingPhase {
    /// Arming: per-tick decrement until ready to detonate on impact.
    Arming,
    /// Cruise: tracking target with sidewinder yaw + cruise altitude control.
    Cruise,
    /// Stall failsafe: target unreachable, detonate next tick.
    SelfDestruct,
    /// Impact: caller despawns this tick.
    Detonation,
}

/// State for an in-flight homing missile.
///
/// Sim-critical numeric fields use `SimFixed` for deterministic lockstep.
/// BAM angles are `u16` (wrapping integer arithmetic is exact).
/// Render-only `pitch` stays `f32`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HomingState {
    pub phase: HomingPhase,

    // Target tracking
    pub target_id: Option<u64>,
    pub last_known_rx: u16,
    pub last_known_ry: u16,

    // Flight kinematics
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
```

**Step 2: Export from `movement/mod.rs`**

In `src/sim/movement/mod.rs`, add:

```rust
pub mod homing_movement;
```

near the existing `pub mod rocket_movement;` line.

**Step 3: Add field to GameEntity**

In `src/sim/game_entity.rs`, near `rocket_state`:

```rust
/// Active homing missile state, if this entity is an in-flight homing
/// projectile. Set by `homing_movement::attach_homing_state`, cleared on
/// detonation.
#[serde(default)]
pub homing_state: Option<crate::sim::movement::homing_movement::HomingState>,
```

**Step 4: Compile**

Run: `cargo check`
Expected: PASS — unused-import / unused-struct warnings are fine for now.

**Step 5: Commit**

Commit message: `sim/movement: scaffold homing_movement module + HomingState`

---

### Task 17: Add `SIDEWINDER_TABLE` constant + BAM math helpers (Gap E)

**Why:** Determinism foundation — replace the runtime cosine with a precomputed 15-entry table. Also add the inclusive-snap helpers gamemd uses.

**Files:**
- Modify: `src/sim/movement/homing_movement.rs`

**Pattern:** Module-level `const` for the table; small free functions for helpers.

**Step 1: Add SIDEWINDER_TABLE**

Append to `homing_movement.rs`:

```rust
/// Precomputed cosine table for the sidewinder modulation: cos(2π * i / 15) for i in 0..15.
///
/// Replaces runtime cosine evaluation in the homing flight loop. Values
/// are precomputed in SimFixed for deterministic lockstep.
///
/// gamemd reference: GGI_GHIDRA_REPORT.md §9.4 — the 15-frame oscillation
/// is the "sidewinder" name's origin.
const SIDEWINDER_TABLE: [SimFixed; 15] = [
    SimFixed::lit("1.0"),                       // cos(0)
    SimFixed::lit("0.91354545764260087"),       // cos(2π/15)
    SimFixed::lit("0.66913060635885821"),       // cos(4π/15)
    SimFixed::lit("0.30901699437494745"),       // cos(6π/15)
    SimFixed::lit("-0.10452846326765346"),      // cos(8π/15)
    SimFixed::lit("-0.5"),                      // cos(10π/15)
    SimFixed::lit("-0.80901699437494745"),      // cos(12π/15)
    SimFixed::lit("-0.97814760073380562"),      // cos(14π/15)
    SimFixed::lit("-0.97814760073380562"),      // cos(16π/15)
    SimFixed::lit("-0.80901699437494745"),      // cos(18π/15)
    SimFixed::lit("-0.5"),                      // cos(20π/15)
    SimFixed::lit("-0.10452846326765346"),      // cos(22π/15)
    SimFixed::lit("0.30901699437494745"),       // cos(24π/15)
    SimFixed::lit("0.66913060635885821"),       // cos(26π/15)
    SimFixed::lit("0.91354545764260087"),       // cos(28π/15)
];

/// Lookup the sidewinder cosine for the given frame counter.
fn sidewinder_cos(frame_counter: u32) -> SimFixed {
    SIDEWINDER_TABLE[(frame_counter % 15) as usize]
}
```

**Step 2: Add BAM helpers**

```rust
/// Inclusive ROT cap check: returns true when current yaw can snap directly
/// to target this tick (i.e., |delta| <= cap).
///
/// gamemd reference: GGI_GHIDRA_REPORT.md §8.1 — `Facing__IsWithinROT`
/// uses `<=` (inclusive snap at boundary).
pub(crate) fn within_rot_bam(cur: u16, tgt: u16, cap: u16) -> bool {
    let delta_signed = (cur.wrapping_sub(tgt)) as i16;
    (delta_signed.unsigned_abs() as u16) <= cap
}

/// Step current BAM angle toward target by at most `cap`; snap to target
/// when within `cap`. Picks the shortest-arc direction via wrapping i16
/// subtraction.
pub(crate) fn step_toward_bam_inclusive(cur: u16, tgt: u16, cap: u16) -> u16 {
    if within_rot_bam(cur, tgt, cap) {
        return tgt;
    }
    let delta_signed = (tgt.wrapping_sub(cur)) as i16;
    if delta_signed > 0 {
        cur.wrapping_add(cap)
    } else {
        cur.wrapping_sub(cap)
    }
}

/// Compute the BAM heading from a delta vector. Uses f32 atan2 internally;
/// the result is truncated to u16 BAM. Bounded jitter ≤ ±1 BAM cannot flip
/// the monotonic `<=` comparison in `within_rot_bam` (cap is always >>1 BAM),
/// so this is lockstep-safe.
pub(crate) fn atan2_bam(dy: SimFixed, dx: SimFixed) -> u16 {
    use crate::util::fixed_math::sim_to_f32;
    let angle_rad = sim_to_f32(dy).atan2(sim_to_f32(dx));
    let bam_f = angle_rad * (32768.0 / std::f32::consts::PI);
    (bam_f as i32).rem_euclid(65536) as u16
}
```

**Step 3: Add unit tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::fixed_math::SimFixed;

    #[test]
    fn test_sidewinder_table_min_max() {
        let max = SIDEWINDER_TABLE.iter().fold(SimFixed::from_num(-2), |a, &v| a.max(v));
        let min = SIDEWINDER_TABLE.iter().fold(SimFixed::from_num(2), |a, &v| a.min(v));
        assert!(max <= SimFixed::from_num(1));
        assert!(max >= SimFixed::lit("0.9"));
        assert!(min <= SimFixed::lit("-0.9"));
        assert!(min >= SimFixed::from_num(-1));
    }

    #[test]
    fn test_sidewinder_cos_wraps_at_15() {
        assert_eq!(sidewinder_cos(0), sidewinder_cos(15));
        assert_eq!(sidewinder_cos(7), sidewinder_cos(22));
    }

    #[test]
    fn test_within_rot_bam_inclusive_at_boundary() {
        // At exact ROT distance, should snap (inclusive).
        assert!(within_rot_bam(0x0000, 0x0100, 0x0100));
        assert!(within_rot_bam(0x0100, 0x0000, 0x0100));
        assert!(!within_rot_bam(0x0000, 0x0101, 0x0100));
    }

    #[test]
    fn test_step_toward_bam_inclusive_snaps_at_cap() {
        // Exactly at cap distance → snap to target.
        assert_eq!(step_toward_bam_inclusive(0x0000, 0x0100, 0x0100), 0x0100);
    }

    #[test]
    fn test_step_toward_bam_inclusive_steps_outside_cap() {
        // Beyond cap → step by cap toward target.
        assert_eq!(step_toward_bam_inclusive(0x0000, 0x0200, 0x0100), 0x0100);
        assert_eq!(step_toward_bam_inclusive(0x0000, 0xFE00, 0x0100), 0xFF00);
    }

    #[test]
    fn test_step_toward_bam_wraps_around() {
        // Shortest arc across the wrap.
        assert_eq!(step_toward_bam_inclusive(0x0000, 0xFF00, 0x0100), 0xFF00);
    }

    #[test]
    fn test_atan2_bam_cardinal_directions() {
        // +x → 0 BAM, +y → 0x4000 BAM (90°).
        let zero_x = atan2_bam(SimFixed::from_num(0), SimFixed::from_num(1));
        let pos_y = atan2_bam(SimFixed::from_num(1), SimFixed::from_num(0));
        assert!(zero_x < 8 || zero_x > 0xFFF8, "0 BAM ≈ +x (got 0x{:04X})", zero_x);
        assert!(
            (pos_y as i32 - 0x4000_i32).abs() < 8,
            "0x4000 BAM ≈ +y (got 0x{:04X})",
            pos_y
        );
    }
}
```

**Step 4: Run tests**

Run: `cargo test homing_movement -- --nocapture`
Expected: PASS.

**Step 5: Commit**

Commit message: `sim/movement: add SIDEWINDER_TABLE + BAM math helpers`

---

### Task 18: Implement `attach_homing_state` (Gap E)

**Why:** Constructor function called by the projectile spawn pipeline (Task 28). Initializes `HomingState` from weapon + projectile + Rules parameters.

**Files:**
- Modify: `src/sim/movement/homing_movement.rs`

**Pattern:** Mirrors `rocket_movement::attach_rocket_state`.

**Step 1: Add the function**

Append to `homing_movement.rs`:

```rust
/// Attach a homing missile state to an entity at the given origin, targeting
/// `target_id`. The entity should already exist in the EntityStore with a
/// position. Returns false if the entity doesn't exist.
///
/// Parameters:
/// - `weapon_speed`: from WeaponType.Speed
/// - `rot_ini`: from BulletType.ROT (raw INI int, NOT pre-scaled)
/// - `arm_frames`: from BulletType.Arm
/// - `floater`, `very_high`: from BulletType
/// - `missile_rot_var`: from Rules.General.MissileROTVar (default 1.0)
#[allow(clippy::too_many_arguments)]
pub fn attach_homing_state(
    entities: &mut EntityStore,
    bullet_id: u64,
    origin: (u16, u16),
    target_id: u64,
    target_pos: (u16, u16),
    weapon_speed: SimFixed,
    rot_ini: u16,
    arm_frames: u16,
    floater: bool,
    very_high: bool,
    missile_rot_var: SimFixed,
) -> bool {
    let Some(entity) = entities.get_mut(bullet_id) else {
        return false;
    };

    let initial_yaw_bam = atan2_bam(
        SimFixed::from_num(target_pos.1 as i32 - origin.1 as i32),
        SimFixed::from_num(target_pos.0 as i32 - origin.0 as i32),
    );

    entity.homing_state = Some(HomingState {
        phase: if arm_frames > 0 {
            HomingPhase::Arming
        } else {
            HomingPhase::Cruise
        },
        target_id: Some(target_id),
        last_known_rx: target_pos.0,
        last_known_ry: target_pos.1,
        yaw_bam: initial_yaw_bam,
        pitch_bam: 0x4000, // 90° BAM = horizontal at start
        speed: weapon_speed.max(SimFixed::from_num(1)),
        altitude: SimFixed::from_num(0),
        vz: SimFixed::from_num(0),
        rot_ini,
        missile_rot_var,
        floater,
        very_high,
        arm_ticks_remaining: arm_frames,
        frame_counter: 0,
        stall_counter: 0,
        stall_ema: SimFixed::from_num(0),
        last_distance_to_target: SimFixed::from_num(0),
        pitch: 0.0,
    });
    true
}
```

**Step 2: Add unit test**

```rust
#[test]
fn test_attach_homing_state_initializes() {
    use crate::sim::entity_store::EntityStore;
    use crate::sim::game_entity::GameEntity;

    let mut entities = EntityStore::new();
    let bullet = GameEntity::test_default(1, "AAHeatSeeker2", "Allied", 5, 5);
    entities.insert(bullet);

    let attached = attach_homing_state(
        &mut entities,
        /*bullet_id=*/ 1,
        /*origin=*/ (5, 5),
        /*target_id=*/ 42,
        /*target_pos=*/ (15, 5),
        /*weapon_speed=*/ SimFixed::from_num(30),
        /*rot_ini=*/ 60,
        /*arm_frames=*/ 2,
        /*floater=*/ false,
        /*very_high=*/ false,
        /*missile_rot_var=*/ SimFixed::from_num(1),
    );
    assert!(attached);

    let entity = entities.get(1).unwrap();
    let h = entity.homing_state.as_ref().unwrap();
    assert_eq!(h.phase, HomingPhase::Arming);
    assert_eq!(h.target_id, Some(42));
    assert_eq!(h.last_known_rx, 15);
    assert_eq!(h.arm_ticks_remaining, 2);
    // Initial yaw should be ~0 BAM (+x toward target).
    assert!(h.yaw_bam < 8 || h.yaw_bam > 0xFFF8);
}
```

**Step 3: Run test**

Run: `cargo test test_attach_homing_state_initializes -- --nocapture`
Expected: PASS.

**Step 4: Commit**

Commit message: `sim/movement: implement attach_homing_state initializer`

---

### Task 19: Implement core per-tick yaw + position update (Gap E)

**Why:** The heart of the homing flight loop — sidewinder ROT modulation, yaw correction, position integration with truncation.

**Files:**
- Modify: `src/sim/movement/homing_movement.rs`

**Pattern:** Mirrors `tick_rocket_movement` shape; new per-tick logic specific to homing.

**Step 1: Add `tick_homing_movement` function**

Append to `homing_movement.rs`:

```rust
use crate::util::fixed_math::{dt_from_tick_ms, sim_to_f32};

/// Advance all in-flight homing missile state machines.
///
/// Called once per simulation tick from `advance_tick()` in the
/// "air + special movement" phase, after `tick_rocket_movement`.
///
/// Returns a list of entity IDs that detonated this tick.
pub fn tick_homing_movement(
    entities: &mut EntityStore,
    tick_ms: u32,
    _sim_tick: u64,
) -> Vec<u64> {
    let mut detonated: Vec<u64> = Vec::new();
    if tick_ms == 0 {
        return detonated;
    }

    let keys = entities.keys_sorted();
    for &id in &keys {
        // Pre-fetch target position before we mutably borrow the bullet.
        let target_pos_opt = {
            let Some(bullet) = entities.get(id) else { continue };
            let Some(h) = bullet.homing_state.as_ref() else { continue };
            h.target_id
                .and_then(|tid| entities.get(tid).map(|t| (t.position.rx, t.position.ry)))
        };

        let Some(bullet) = entities.get_mut(id) else { continue };
        let Some(h) = bullet.homing_state.as_mut() else { continue };

        // 1. Refresh last-known pos if target still alive.
        if let Some(pos) = target_pos_opt {
            h.last_known_rx = pos.0;
            h.last_known_ry = pos.1;
        } else {
            h.target_id = None; // target died — fly to last-known
        }

        // 2. Compute desired yaw from current pos → last_known.
        let dx_cells = SimFixed::from_num(h.last_known_rx as i32 - bullet.position.rx as i32);
        let dy_cells = SimFixed::from_num(h.last_known_ry as i32 - bullet.position.ry as i32);
        let desired_yaw = atan2_bam(dy_cells, dx_cells);

        // 3. Sidewinder modulation.
        let sidewinder =
            sidewinder_cos(h.frame_counter) * h.missile_rot_var + h.missile_rot_var + SimFixed::from_num(1);
        let delta_far_simfixed: SimFixed = sidewinder * SimFixed::from_num(h.rot_ini as i32);
        let delta_far: i32 = delta_far_simfixed.to_num::<i32>(); // truncation

        // 4. Close-range branch (<256 leptons = <1 cell).
        let dist_leptons_approx: i32 = (dx_cells.to_num::<i32>().abs() + dy_cells.to_num::<i32>().abs()) * 256;
        let close_range: bool = dist_leptons_approx < 256;
        let delta_int: i32 = if close_range {
            ((h.frame_counter % 15) as i32 * 3) / 2 // (frame % 15) * 1.5, truncated
        } else {
            delta_far
        };

        // 5. ROT_BAM = LowByte(delta) << 8 (matches gamemd shift).
        let delta_byte: u8 = (delta_int as u32 & 0xFF) as u8;
        let rot_bam_per_tick: u16 = (delta_byte as u16) << 8;

        // 6. Yaw step with inclusive snap.
        h.yaw_bam = step_toward_bam_inclusive(h.yaw_bam, desired_yaw, rot_bam_per_tick);

        // 7. Rebuild velocity (vx, vy) — preserves horizontal magnitude.
        let dt = dt_from_tick_ms(tick_ms);
        let v_horizontal = h.speed * dt; // cells this tick
        let yaw_radians = (h.yaw_bam as i32 - 0x4000) as f32 * (std::f32::consts::PI / 32768.0);
        let vx_cells_f32 = sim_to_f32(v_horizontal) * yaw_radians.cos();
        let vy_cells_f32 = sim_to_f32(v_horizontal) * yaw_radians.sin();

        // 8. Position += truncated velocity (per gamemd Math__ftol).
        let new_rx = bullet.position.rx as i32 + (vx_cells_f32 as i32);
        let new_ry = bullet.position.ry as i32 + (vy_cells_f32 as i32);
        bullet.position.rx = new_rx.clamp(0, u16::MAX as i32) as u16;
        bullet.position.ry = new_ry.clamp(0, u16::MAX as i32) as u16;

        // 9. vz damper when not Floater.
        if !h.floater {
            let signum: i32 = if h.vz > SimFixed::from_num(0) { 1 } else if h.vz < SimFixed::from_num(0) { -1 } else { 0 };
            h.vz = (h.vz + SimFixed::from_num(signum * 3)) / SimFixed::from_num(4);
        }

        // 10. Arm decrement.
        if h.arm_ticks_remaining > 0 {
            h.arm_ticks_remaining -= 1;
            if h.arm_ticks_remaining == 0 && h.phase == HomingPhase::Arming {
                h.phase = HomingPhase::Cruise;
            }
        }

        // 11. Detonation proximity check.
        let dist_sq: i32 = {
            let dx = h.last_known_rx as i32 - bullet.position.rx as i32;
            let dy = h.last_known_ry as i32 - bullet.position.ry as i32;
            dx * dx + dy * dy
        };
        if dist_sq <= 0 && h.arm_ticks_remaining == 0 {
            h.phase = HomingPhase::Detonation;
            detonated.push(id);
        }

        h.frame_counter = h.frame_counter.wrapping_add(1);
    }

    detonated
}
```

(NOTE: this core loop will be refined in Tasks 20-21 to add the cruise altitude controller and stall detection. For now it has the yaw + position + arm + basic proximity-detonation.)

**Step 2: Add basic flight test**

```rust
#[test]
fn test_homing_missile_reaches_static_target() {
    use crate::sim::entity_store::EntityStore;
    use crate::sim::game_entity::GameEntity;

    let mut entities = EntityStore::new();
    let target = GameEntity::test_default(42, "KIROV", "Soviet", 25, 5);
    entities.insert(target);
    let bullet = GameEntity::test_default(1, "AAHeatSeeker2", "Allied", 5, 5);
    entities.insert(bullet);

    attach_homing_state(
        &mut entities,
        1,
        (5, 5),
        42,
        (25, 5),
        SimFixed::from_num(30),
        60,
        0,
        false,
        false,
        SimFixed::from_num(1),
    );

    let mut detonated = false;
    for _ in 0..200 {
        let det = tick_homing_movement(&mut entities, 22, 0);
        if det.contains(&1) {
            detonated = true;
            break;
        }
    }
    assert!(detonated, "homing missile should detonate when reaching static target");
}
```

**Step 3: Run test**

Run: `cargo test test_homing_missile_reaches_static_target -- --nocapture`
Expected: PASS.

**Step 4: Commit**

Commit message: `sim/movement: implement homing per-tick yaw + position update`

---

### Task 20: Add cruise altitude controller (Gap E)

**Why:** Per §8.1 — missiles flatten via vz damper and adjust altitude via the cruise controller. Without this, missiles fly at the wrong angle.

**Files:**
- Modify: `src/sim/movement/homing_movement.rs`

**Pattern:** Insert between the vz damper (current step 9) and the proximity check (current step 11).

**Step 1: Add cruise altitude logic**

In `tick_homing_movement`, after the vz damper, before the arm decrement, insert:

```rust
// Cruise altitude controller (gamemd reference: GGI_GHIDRA_REPORT.md §8.1).
// Only runs when not Floater AND we're above ~3 cells of altitude AND ROT > 1.
let high_alt_branch = !h.floater
    && bullet.position.z > SimFixed::from_num(3 * 256)
    && h.rot_ini > 1;
if high_alt_branch {
    let target_alt_leptons: i32 = if h.floater || h.very_high {
        10 * 64
    } else {
        // min(target_alt / 256, 5) * 64 lepton-per-cruise-step
        let cap = if let Some(tid) = h.target_id {
            entities
                .get(tid)
                .map(|t| (t.position.z.to_num::<i32>() / 256).min(5))
                .unwrap_or(5)
        } else {
            5
        };
        cap * 64
    };

    let self_z: i32 = bullet.position.z.to_num::<i32>();
    let dz: i32 = self_z - target_alt_leptons - /*ground_height=*/ 0;

    // Dead-band ±20 leptons → no clamp. Snap ±18 outside.
    if dz.abs() > 20 {
        let snap: i32 = if dz > 0 { -18 } else { 18 };
        bullet.position.z = SimFixed::from_num((self_z + snap).max(0));
    }

    // Half-threshold pitch BAMs.
    let pitch_target: u16 = if dz < -32 {
        0x2000 // tilt up
    } else if dz > 32 {
        0x4800 // tilt down
    } else {
        0x4000 // level off
    };
    let pitch_step: u16 = rot_bam_per_tick / 2;
    h.pitch_bam = step_toward_bam_inclusive(h.pitch_bam, pitch_target, pitch_step);
}
```

(Adapt the entity Z-coord field — if `position.z` doesn't exist as SimFixed, use whatever the engine's altitude representation is. The impact analysis didn't deeply check the position/altitude struct shape; verify during implementation.)

**Step 2: Hoist target lookup**

The cruise block needs to read the target's altitude. The current target-pos fetch in step 1 only reads `(rx, ry)`. Extend it to also fetch `z`:

```rust
let target_pos_opt: Option<(u16, u16, SimFixed)> = {
    let Some(bullet) = entities.get(id) else { continue };
    let Some(h) = bullet.homing_state.as_ref() else { continue };
    h.target_id.and_then(|tid| entities.get(tid).map(|t| (t.position.rx, t.position.ry, t.position.z)))
};
```

Use `.0`, `.1`, `.2` accessors in steps that need them.

**Step 3: Add unit tests**

```rust
#[test]
fn test_cruise_dead_band_no_snap() {
    // |dz| ≤ 20 leptons → no z snap.
    let mut entities = setup_test_entities();
    let bullet_id = spawn_test_homing_missile(&mut entities, /*z=*/ SimFixed::from_num(64 * 5));
    // Target at z=64*5 → dz=0, well inside dead-band.
    tick_homing_movement(&mut entities, 22, 0);
    let z_after = entities.get(bullet_id).unwrap().position.z;
    assert!(z_after >= SimFixed::from_num(64 * 5 - 20));
    assert!(z_after <= SimFixed::from_num(64 * 5 + 20));
}

#[test]
fn test_cruise_outside_dead_band_snaps_by_18() {
    let mut entities = setup_test_entities();
    let bullet_id = spawn_test_homing_missile(&mut entities, /*z=*/ SimFixed::from_num(64 * 5 + 30));
    // Target at cruise_alt=64*5 → dz=+30 > 20 → snap down by 18.
    let z_before = SimFixed::from_num(64 * 5 + 30);
    tick_homing_movement(&mut entities, 22, 0);
    let z_after = entities.get(bullet_id).unwrap().position.z;
    let delta = z_before.to_num::<i32>() - z_after.to_num::<i32>();
    assert_eq!(delta, 18);
}
```

(Define `setup_test_entities` / `spawn_test_homing_missile` helpers — extend Task 18's test fixture.)

**Step 4: Run tests**

Run: `cargo test homing_movement cruise -- --nocapture`
Expected: PASS.

**Step 5: Commit**

Commit message: `sim/movement: cruise altitude controller for homing missiles`

---

### Task 21: Add stall-detect failsafe (Gap E)

**Why:** Per §8.1 — missile self-destructs if 60+ frames pass and distance-to-target doesn't decrease (EMA-tracked). Otherwise missiles fly forever.

**Files:**
- Modify: `src/sim/movement/homing_movement.rs`

**Pattern:** Sliding-window then EMA in the same per-tick body.

**Step 1: Add stall logic after the proximity check**

In `tick_homing_movement`, between the proximity-detonation block and the `frame_counter` increment:

```rust
// Stall detection (gamemd reference: §8.1 — 60-frame window, then EMA).
let dist_now_simfixed: SimFixed = {
    let dx = SimFixed::from_num(h.last_known_rx as i32 - bullet.position.rx as i32);
    let dy = SimFixed::from_num(h.last_known_ry as i32 - bullet.position.ry as i32);
    (dx * dx + dy * dy)
        .checked_sqrt()
        .unwrap_or(SimFixed::from_num(0))
};
let delta_dist = h.last_distance_to_target - dist_now_simfixed;
h.last_distance_to_target = dist_now_simfixed;

if h.stall_counter < 60 {
    h.stall_counter += 1;
    h.stall_ema += delta_dist;
} else {
    // EMA: ema = ema * 0.9 + delta_dist * 0.1
    h.stall_ema = h.stall_ema * SimFixed::lit("0.9") + delta_dist * SimFixed::lit("0.1");
    if h.stall_ema <= SimFixed::lit("0.5") && !h.floater {
        h.phase = HomingPhase::SelfDestruct;
        detonated.push(id);
    }
}
```

If `SimFixed::checked_sqrt` doesn't exist, use the project's existing distance helper (similar to `fixed_distance` in bump_crush.rs).

**Step 2: Add unit test**

```rust
#[test]
fn test_stall_detect_self_destructs_on_unreachable_target() {
    let mut entities = setup_test_entities();
    // Set up a missile targeting a position behind a "wall" we can't model
    // — instead, set bullet speed to 0 to simulate "can't close distance".
    let bullet_id = spawn_test_homing_missile_speed(
        &mut entities,
        /*speed=*/ SimFixed::from_num(0),
    );

    let mut self_destructed = false;
    for tick in 0..200 {
        let det = tick_homing_movement(&mut entities, 22, tick as u64);
        if det.contains(&bullet_id) {
            self_destructed = true;
            break;
        }
    }
    assert!(self_destructed, "stalled missile should self-destruct after 60-frame EMA");
}
```

**Step 3: Run test**

Run: `cargo test test_stall_detect_self_destructs -- --nocapture`
Expected: PASS.

**Step 4: Commit**

Commit message: `sim/movement: stall-detect failsafe for homing missiles`

---

### Task 22: Wire `tick_homing_movement` into `World::advance_tick` (Gap E)

**Why:** Without registration in the tick pipeline, the new module never runs.

**Files:**
- Modify: `src/sim/world/mod.rs`

**Pattern:** Insert after the existing `tick_rocket_movement` call, in the same phase.

**Step 1: Find rocket call site**

Search `src/sim/world/mod.rs` for `tick_rocket_movement` (likely in a `advance_tick` method).

**Step 2: Add homing call right after**

```rust
// Existing:
let rocket_detonated = crate::sim::movement::rocket_movement::tick_rocket_movement(
    &mut self.entities,
    tick_ms,
    self.sim_tick,
);
// New:
let homing_detonated = crate::sim::movement::homing_movement::tick_homing_movement(
    &mut self.entities,
    tick_ms,
    self.sim_tick,
);
```

Combine the two detonation lists for downstream damage dispatch:

```rust
let all_detonated: Vec<u64> = rocket_detonated.into_iter().chain(homing_detonated).collect();
// pass `all_detonated` to whatever damage/despawn dispatcher exists.
```

(Adapt to actual existing rocket-detonation dispatch — match the pattern.)

**Step 3: Compile**

Run: `cargo check`
Expected: PASS.

**Step 4: Run full test suite**

Run: `cargo test`
Expected: PASS.

**Step 5: Commit**

Commit message: `sim/world: register tick_homing_movement in advance_tick`

---

### Task 23: Include `homing_state` + `weapon_override` in deterministic state hash (Gap E + G)

**Why:** New per-entity fields must affect the state hash for lockstep determinism.

**Files:**
- Modify: `src/sim/world/world_hash.rs`

**Pattern:** Follow the existing pattern for hashing per-entity state.

**Step 1: Locate the entity-state hash function**

Search `world_hash.rs` for `rocket_state` or any existing per-entity hash contribution.

**Step 2: Add new fields**

Add hash contributions for `homing_state` and `weapon_override`. If the existing pattern uses `#[derive(Hash)]` on a snapshot struct, ensure `HomingState` and `WeaponOverride` derive `Hash` (or add explicit `impl Hash` if any field is `f32`).

For `HomingState` (which has `pitch: f32` and `SimFixed` fields):

```rust
// In homing_movement.rs, add an explicit Hash impl that excludes pitch:
impl std::hash::Hash for HomingState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.phase.hash(state);
        self.target_id.hash(state);
        self.last_known_rx.hash(state);
        self.last_known_ry.hash(state);
        self.yaw_bam.hash(state);
        self.pitch_bam.hash(state);
        self.speed.to_bits().hash(state);
        self.altitude.to_bits().hash(state);
        self.vz.to_bits().hash(state);
        self.rot_ini.hash(state);
        self.missile_rot_var.to_bits().hash(state);
        self.floater.hash(state);
        self.very_high.hash(state);
        self.arm_ticks_remaining.hash(state);
        self.frame_counter.hash(state);
        self.stall_counter.hash(state);
        self.stall_ema.to_bits().hash(state);
        self.last_distance_to_target.to_bits().hash(state);
        // intentionally omit `pitch: f32` — render-only
    }
}
```

(Adapt `.to_bits()` to whatever SimFixed's underlying type method is — likely `.to_bits()` or `.to_num::<i32>()`.)

**Step 3: Bump hash schema version**

Find the constant (search `HASH_SCHEMA` or version numbers) and bump it.

**Step 4: Add hash test**

```rust
#[test]
fn test_world_hash_includes_homing_state() {
    let mut sim_a = make_minimal_sim();
    let mut sim_b = sim_a.clone();
    // Mutate homing state in sim_b — hashes must differ.
    if let Some(e) = sim_b.entities.iter_mut().next().and_then(|(_, e)| Some(e)) {
        e.homing_state = Some(make_test_homing_state());
    }
    assert_ne!(compute_world_hash(&sim_a), compute_world_hash(&sim_b));
}
```

**Step 5: Run test**

Run: `cargo test test_world_hash_includes_homing_state -- --nocapture`
Expected: PASS.

**Step 6: Commit**

Commit message: `sim/world: hash homing_state + weapon_override; bump schema`

---

### Task 24: Parse `[General]MissileROTVar=` into Rules (Gap E)

**Why:** Sidewinder formula needs this Rules constant. Default 1.0 when absent (gamemd default).

**Files:**
- Modify: `src/rules/ruleset.rs` or `src/rules/general.rs` (whichever owns `[General]` parse)

**Pattern:** Match existing `[General]` constant parses.

**Step 1: Locate `[General]` section parse**

Run: `grep -rn "\\[General\\]" src/rules/`. Find the function that reads `[General]` keys.

**Step 2: Add field on the General struct**

```rust
/// `MissileROTVar=` from rules.ini [General]. Controls the amplitude of
/// the sidewinder cosine modulation in homing missile flight.
/// Default 1.0 produces oscillation in [1.0, 3.0]× ROT_INI.
///
/// gamemd reference: GGI_GHIDRA_REPORT.md §9.4.
pub missile_rot_var: SimFixed,
```

And in the default impl:

```rust
missile_rot_var: SimFixed::from_num(1),
```

**Step 3: Parse it**

```rust
general.missile_rot_var = general_section
    .get("MissileROTVar")
    .and_then(|s| s.trim().parse::<f64>().ok())
    .map(SimFixed::from_num)
    .unwrap_or(SimFixed::from_num(1));
```

**Step 4: Test default + parse**

```rust
#[test]
fn test_missile_rot_var_defaults_to_one() {
    let ini_str = "[General]\n";
    let ini = crate::rules::ini_parser::IniFile::from_str(ini_str);
    let rules = crate::rules::ruleset::RuleSet::from_ini(&ini).expect("parse");
    assert_eq!(rules.general.missile_rot_var, SimFixed::from_num(1));
}

#[test]
fn test_missile_rot_var_parsed() {
    let ini_str = "[General]\nMissileROTVar=2.5\n";
    let ini = crate::rules::ini_parser::IniFile::from_str(ini_str);
    let rules = crate::rules::ruleset::RuleSet::from_ini(&ini).expect("parse");
    assert!((rules.general.missile_rot_var - SimFixed::lit("2.5")).abs() < SimFixed::lit("0.001"));
}
```

**Step 5: Run tests**

Run: `cargo test missile_rot_var -- --nocapture`
Expected: PASS.

**Step 6: Commit**

Commit message: `rules: parse [General]MissileROTVar (default 1.0)`

---

### Task 25: Dispatch hook — branch ballistic vs homing at projectile spawn (Gap E)

**Why:** Tie the new module into the actual fire pipeline. The impact analysis flagged the spawn site as not visible from grep — this task finds it.

**Files:**
- Modify: wherever projectile spawn currently dispatches (locate during task)

**Pattern:** Branch on `BulletType.ranged` at the dispatch point.

**Step 1: Find the spawn dispatch site**

Run:
```
grep -rn "attach_rocket_state" src/
grep -rn "rocket_state = Some" src/
grep -rn "spawn.*projectile" src/
grep -rn "fire_weapon" src/
```

If `attach_rocket_state` is only called from tests (likely per the impact analysis), the projectile-spawn pipeline doesn't exist in production code yet. In that case:

**Path A — Production spawn site exists:** add the dispatch branch.

**Path B — No production spawn site:** STOP. Document the finding in a `FOLLOW_UP.md` under `docs/plans/` and skip Task 25-26. The homing module is ready to be wired when the projectile-spawn pipeline lands.

**Step 2 (Path A): Add dispatch**

At the spawn site:

```rust
let projectile = rules.projectile(&weapon.projectile.as_deref().unwrap_or(""))?;
if projectile.ranged {
    crate::sim::movement::homing_movement::attach_homing_state(
        &mut self.entities,
        bullet_id,
        origin,
        target_id,
        target_pos,
        weapon.speed,
        projectile.rot,
        projectile.arm,
        projectile.floater,
        projectile.very_high,
        rules.general.missile_rot_var,
    );
} else {
    crate::sim::movement::rocket_movement::attach_rocket_state(
        &mut self.entities,
        bullet_id,
        origin,
        target_pos,
        weapon.speed,
    );
}
```

**Step 2 (Path B): Document and stop**

Create `docs/plans/2026-05-17-projectile-spawn-pipeline-followup.md`:

```markdown
# Projectile Spawn Pipeline — Follow-Up

During the GGI Rust integration plan (2026-05-17), Task 25 attempted to
wire `homing_movement::attach_homing_state` into the projectile-spawn
dispatch site. Discovery: `attach_rocket_state` is currently only called
from `rocket_movement.rs`'s own test module. No production code path
spawns projectiles via either rocket_movement or homing_movement.

The homing module is fully implemented and unit-tested in isolation.
Production wiring requires a separate brainstorm/plan covering the
projectile-spawn pipeline as a whole.
```

**Step 3: Compile**

Run: `cargo check`
Expected: PASS.

**Step 4: Run tests**

Run: `cargo test`
Expected: PASS.

**Step 5: Commit**

Commit message (Path A): `sim/world: dispatch ballistic vs homing at projectile spawn`
Commit message (Path B): `docs: follow-up — projectile spawn pipeline missing for GGI homing wire-up`

---

### Task 26: Integration test — GGI deployed fires homing missile at static target (Gap E)

**Why:** End-to-end smoke test that exercises the new module.

**Files:**
- Modify: `src/sim/movement/homing_movement.rs` (test module)

**Pattern:** Self-contained integration test in the module's test module.

**Step 1: Add test**

```rust
#[test]
fn test_homing_missile_curves_toward_moving_target() {
    use crate::sim::entity_store::EntityStore;
    use crate::sim::game_entity::GameEntity;

    let mut entities = EntityStore::new();
    let target = GameEntity::test_default(42, "KIROV", "Soviet", 30, 5);
    entities.insert(target);
    let bullet = GameEntity::test_default(1, "AAHeatSeeker2", "Allied", 5, 5);
    entities.insert(bullet);

    attach_homing_state(
        &mut entities,
        1,
        (5, 5),
        42,
        (30, 5),
        SimFixed::from_num(20),
        60,
        0,
        false,
        false,
        SimFixed::from_num(1),
    );

    // Move target during flight to verify retargeting.
    let mut detonated = false;
    for tick in 0..300 {
        // After tick 10, move target away.
        if tick == 10 {
            entities.get_mut(42).unwrap().position.rx = 35;
            entities.get_mut(42).unwrap().position.ry = 10;
        }
        let det = tick_homing_movement(&mut entities, 22, tick as u64);
        if det.contains(&1) {
            detonated = true;
            break;
        }
    }
    assert!(detonated, "missile should track moving target and detonate");
}
```

**Step 2: Run test**

Run: `cargo test test_homing_missile_curves_toward_moving_target -- --nocapture`
Expected: PASS.

**Step 3: Commit**

Commit message: `sim/movement: integration test — homing missile tracks moving target`

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-17-ggi-rust-integration-design.md](2026-05-17-ggi-rust-integration-design.md)
- **Ghidra report:** [ra2-rust-game-docs/GGI_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/GGI_GHIDRA_REPORT.md) — 9 sections, all parity-critical findings
- **Cross-reference dossier:** [ra2-rust-game-docs/GI_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/GI_GHIDRA_REPORT.md) — E1 (shared infantry infrastructure)
- **Key gamemd.exe addresses** (kept here, not in Rust code comments):
  - `0x0051d6f0` — `InfantryClass::Do_Action` (sound-before-write order)
  - `0x00712170` — `TechnoTypeClass::ReadINI` (DeploySound parse at +0x56C/+0x570)
  - `0x005f6cd0` — `TechnoClass::CanCrushCheck` (deploy uncrushable Branch B)
  - `0x006fdd50` — `TechnoClass::Fire_At` (NO ProneDamage application — verified absent)
  - `0x00489180` — `ApplyWarheadDamage` (Verses formula, no ProneDamage)
  - `0x007c5f00` — `Math__ftol` (truncation toward zero confirmed)
  - `0x005b20f0` — `BulletClass::HomingTrack` (sidewinder + cruise altitude + stall)
  - `0x00772080` — `WeaponTypeClass::ReadINI` (Range at +0xB4 leptons, MinimumRange at +0xB8)
  - `0x0046bee0` — `BulletTypeClass::ReadINI` (AA at +0x2A4, AG at +0x2A5, ROT at +0x2F0, Arm at +0x2DC, Ranged at +0x2A0)
  - `0x0075d3a0` — `WarheadTypeClass::ReadINI_Body` (Verses at +0xA0, 11 doubles)
- **INI keys driven:**
  - `rulesmd.ini` `[GGI]` line 3863, `[M60]`/`[M60E]`, `[MissileLauncher]`/`[MissileLauncherE]`, `[SA]`, `[GUARDWH]`, `[AAHeatSeeker2]`, `[General]MissileROTVar`
  - `artmd.ini` `[GGI]` line 291, `[GuardianGISequence]` line 14166
- **Related code:**
  - [src/sim/movement/rocket_movement.rs](../../src/sim/movement/rocket_movement.rs) — ballistic-arc sibling
  - [src/sim/combat/combat_weapon.rs](../../src/sim/combat/combat_weapon.rs#L157-L197) — `select_garrison_weapon` (model for tier swap)
  - [src/rules/art_data.rs](../../src/rules/art_data.rs) — art entry registry (from commit `1391629`)
- **Prior commits:**
  - `e3a724f` — low-silhouette crush (closed §6 gap #1 + §9.2 deploy gate)
  - `1391629` — fire-to-anim sync (closed §6 gap #8, fire-frame `==` anchor)
  - `0b1877b`, `bfeda02` — fear/prone runtime (relevant context for deploy state interactions)
