# Grizzly 105mm Projectile Arc / Impact Timing - Ghidra Report

Date: 2026-05-22

Target: `GRIZZLY_105MM_PROJECTILE_ARC_IMPACT_TIMING`

## Working Notes

- Target question: after stock MTNK `[105mm]` reaches `TechnoClass::Fire_At`, does `Projectile=Cannon` produce immediate damage or a live arcing `BulletClass` whose impact/detonation occurs after flight?
- Non-goals: target acquisition, damage formulas, elite burst cadence, FLH orientation, and complete all-projectile physics.
- Evidence needed to mark COMPLETE: INI/default proof for `[105mm]` and `[Cannon]`, binary proof of bullet creation/launch, binary proof of arcing `BulletClass::AI` movement, binary proof that warhead damage is detonated from bullet impact, and a Rust-facing handoff.
- Stop conditions: no read-only Ghidra access, no path from `Fire_At` to `BulletClass`, or no evidence that `Cannon` is active in standard YR.

## Summary

Stock Grizzly `[105mm]` uses the generic visible arcing `Cannon` projectile. In YR this is not an instant-hit weapon: `TechnoClass::Fire_At @ 0x006FDD50` allocates a `BulletClass`, computes a trajectory from the FLH/source coordinate toward the target coordinate, launches it through `BulletClass::Fire @ 0x00468670`, and returns the bullet pointer.

Damage/application is downstream of live projectile flight. `BulletClass::AI @ 0x004666E0` advances `ROT <= 0` / `Arcing=yes` bullets with gravity, terrain/building/bridge/cell proximity checks, and only calls `BulletClass::BulletDetonation @ 0x00468D80` once a detonation condition is reached. `BulletClass::BulletDetonation` then calls `WarheadTypeClass::Detonate @ 0x004690B0`, whose normal branch calls `Apply_area_damage`.

The current Rust shape applies combat damage in `src/sim/combat/mod.rs` during the same tick as `fire_events.push(...)`, while `src/app_fire_effects.rs` separately draws a render-only projectile whose duration is based on `weapon.speed` and clamped to 160-900 ms. That is a player-visible parity gap: health/death/impact effects should be tied to the generic `Cannon` bullet detonation tick, not the fire tick. This should be implemented generically for visible arcing bullets, not as a Grizzly-specific special case.

Status: COMPLETE for the scoped binary path and Rust handoff. Exact stock frame counts for every range/elevation/wall case remain a follow-up simulation/golden-test topic.

## Verified Findings

### 1. Stock MTNK uses visible arcing `Cannon`

Active in YR: Yes.

Evidence:
- `ini/rulesmd.ini:23325..23334`: `[105mm] Damage=65`, `ROF=60`, `Range=5`, `Projectile=Cannon`, `Speed=40`, `Warhead=AP`, `Report=GrizzlyTankAttack`, `Anim=GUNFIRE`, `Bright=yes`.
- `ini/rulesmd.ini:25445..25450`: `[Cannon] Image=120MM`, `Arcing=true`, `SubjectToCliffs=yes`, `SubjectToElevation=yes`, `SubjectToWalls=yes`.
- `BulletTypeClass::ReadINI @ 0x0046BEE0` reads projectile fields into active offsets: `Arcing -> +0x29B`, `SubjectToCliffs -> +0x296`, `SubjectToElevation -> +0x297`, `SubjectToWalls -> +0x298`, `ROT -> +0x2DC`, and inherited `Image` metadata. See `BULLETTYPECLASS_GHIDRA_REPORT.md`.

### 2. `Fire_At` launches a live bullet, not a direct damage event

Active in YR: Yes.

Evidence:
- `TechnoClass::Fire_At @ 0x006FDD50` decompile: after target/source coordinate setup, it allocates a `BulletClass`, sets the weapon pointer, calls vtable `+0xD4` conceal, computes projectile velocity, then calls bullet vtable `+0x1F0` with source coordinate and velocity.
- Assembly range `0x006FE4B0..0x006FF1D0` includes the allocate/conceal/trajectory branch; range `0x006FF1D0..0x006FF267` performs the vtable `+0x1F0` launch call and handles failure by deleting the bullet.
- `BULLETCLASS_INIT_AND_FIRE_GHIDRA_REPORT.md` independently verifies BulletClass vtable `+0x1F0 == BulletClass::Fire @ 0x00468670`.

### 3. `BulletClass::Fire` reveals, stores velocity, stores source/target state, arms detector, and submits the bullet

Active in YR: Yes.

Evidence:
- `BulletClass::Fire @ 0x00468670` decompile: first calls `ObjectClass::Reveal`, copies six 32-bit words into `BulletClass+0xE8..+0xFF` velocity, stores source/target coordinates at `+0x134..+0x148`, arms `ProximityDetector` with `BulletTypeClass+0x2F0`, normalizes ROT bullets only when `+0x2DC > 0`, then submits the object if alive.
- Assembly `0x0046867F..0x004686A2`: `ObjectClass::Reveal` call then velocity copy into `+0xE8`.
- Assembly `0x004686A5..0x0046872B`: source coordinate copy and owner/target coordinate storage.
- Assembly `0x00468A3F..0x00468A93`: `Arm` read from `BulletType+0x2F0`, detector setup at `this+0xB8`.
- Assembly `0x00468B5D..0x00468B6D`: alive check then `DisplayClass::Submit_Object`.

### 4. `Cannon` uses the arcing `BulletClass::AI` path with gravity and collision gates

Active in YR: Yes.

Evidence:
- `BulletClass::AI @ 0x004666E0` decompile: movement branches on `BulletTypeClass+0x2DC` (`ROT`). For `ROT <= 0`, it further branches on `BulletTypeClass+0x29B` (`Arcing`). `Arcing=yes` reads velocity, applies gravity from `RulesClass+0x16B8` unless `Floater +0x295` selects the alternate gravity helper, moves the bullet, and checks ground/building/bridge/cell/proximity detonation gates.
- `FUN_00773070 @ 0x00773070` decompile and assembly `0x00773070..0x007730C9`: for weapons whose projectile pointer `weapon+0xA0` has `ROT == 0`, it uses `RulesClass+0x16B8` or floater gravity instead of simply returning `weapon+0xA8` speed; otherwise it returns the weapon speed. This confirms `Cannon` participates in generic ballistic setup.
- `FUN_0048A8D0 @ 0x0048A8D0` / `FUN_0048A9D0 @ 0x0048A9D0`: ballistic launch-angle solver used from the arcing branch in `TechnoClass::Fire_At`.

### 5. Warhead damage is applied from bullet detonation, after projectile flight

Active in YR: Yes.

Evidence:
- `BulletClass::BulletDetonation @ 0x00468D80` decompile: reads current bullet location `+0x9C/+0xA0/+0xA4`, may snap to target coordinates for close non-airburst/non-inaccurate cases, then calls `WarheadTypeClass::Detonate @ 0x004690B0` for each `Cluster` count.
- Assembly `0x00468FF4..0x00469038`: non-airburst branch reads `BulletType+0x2AC` cluster count and calls `0x004690B0`.
- `WarheadTypeClass::Detonate @ 0x004690B0` decompile: normal warhead branch calls `Apply_area_damage` using bullet-owned owner/warhead/damage context. This is downstream from `BulletClass::AI` detonation, not from the `Fire_At` frame for visible arcing `Cannon`.

## Implementation Handoff

- Verified behavior: stock MTNK `[105mm]` creates a live generic `Cannon` arcing bullet and applies damage only on bullet detonation -> Rust delta: `src/sim/combat/mod.rs` currently pushes `damage_events` and subtracts health in the same combat tick as fire -> affected surface: `src/sim/combat/mod.rs`, `src/sim/game_entity.rs`, a new or extended projectile movement module, and `src/sim/world/mod.rs` fire-event plumbing -> acceptance scenario: a Grizzly firing at a target five cells away emits a fire event on tick `T`, target HP is unchanged on `T`, and HP drops only when the simulated `Cannon` bullet detonates -> proposed test name: `grizzly_105mm_damage_waits_for_cannon_impact` -> risk: immediate damage makes shells visually cosmetic and causes early deaths/retargeting.

- Verified behavior: `Cannon` is generic `Arcing=yes`, `ROT=0`, `Image=120MM`, with cliff/elevation/wall flags read from `BulletTypeClass` -> Rust delta: render-only projectile visuals in `src/app_fire_effects.rs` use a straight interpolation duration based on `weapon.speed`, while sim has no generic arcing projectile entity for Cannon -> affected surface: `src/rules/projectile_type.rs`, `src/sim/movement` or projectile subsystem, `src/app_fire_effects.rs` as consumer only -> acceptance scenario: `[Cannon]` and any other visible arcing projectile share one ballistic sim path, with image/render driven from projectile type and damage driven by impact -> proposed test name: `arcing_cannon_projectile_uses_generic_bullet_flight_path` -> risk: a Grizzly-only fix would leave Rhino, Apocalypse, elite Harvester, and modded `Cannon` users mismatched.

- Verified behavior: `SubjectToCliffs`, `SubjectToElevation`, and `SubjectToWalls` are active `[Cannon]` flags and `BulletClass::AI` includes terrain/building/bridge detonation gates -> Rust delta: current direct-hit combat bypasses projectile terrain collision timing -> affected surface: projectile movement plus terrain/bridge/wall integration, not combat weapon selection -> acceptance scenario: a `Cannon` shell can detonate before the intended target when the ballistic path hits blocking terrain/bridge/wall according to projectile flags -> proposed test name: `cannon_shell_impact_can_precede_target_on_blocking_terrain` -> risk: straight line damage-to-target ignores common visible obstacles.

## Negative Facts / Do Not Do

- Do not implement `[105mm]` as hitscan or same-tick damage. Evidence: `Fire_At @ 0x006FDD50` launches a `BulletClass`, while `WarheadTypeClass::Detonate` is reached from `BulletClass::BulletDetonation @ 0x00468D80`.
- Do not make this Grizzly-specific. Evidence: `[105mm] Projectile=Cannon` is an INI reference; `[Cannon]` is a reusable `BulletTypeClass` read by generic `BulletClass` code.
- Do not treat the projectile image as `SABOT` for stock YR MTNK. Evidence: `rulesmd.ini:25445..25446` says `[Cannon] Image=120MM`; MTNK docs only mention `SABOT` as a related TNKD/Rhino-family shorthand, not the stock Grizzly image.
- Do not use `weapon.Speed=40` as the sole timing formula for arcing bullets. Evidence: `FUN_00773070 @ 0x00773070` takes the `ROT==0` projectile path through gravity-derived setup instead of directly returning `weapon+0xA8`.
- Do not keep projectile visuals as app-only if sim health/death remains immediate. Evidence: YR bullet object is both visible and authoritative for later detonation/damage.

## Focused Rust Scan

- `damage_events`, immediate health subtraction, `fire_events.push` -> `src/sim/combat/mod.rs:1476`, `src/sim/combat/mod.rs:1859..1913`, `src/sim/combat/mod.rs:1979`, `src/sim/combat/mod.rs:2108..2120` -> existing tests around direct combat/bridge impacts -> likely ownership: `sim/combat` must stop applying delayed-projectile weapons immediately.
- `ProjectileType` parsed fields -> `src/rules/projectile_type.rs:43..108`, `src/rules/projectile_type.rs:177..221` -> existing parser tests include `test_arcing_projectile` and `test_arm_is_projectile_arming_delay_not_speed` -> likely ownership: `rules` already has enough data for first generic `Cannon` handoff.
- Render-only projectile visuals -> `src/app_fire_effects.rs:379..425`, `src/app_instances/overlays.rs:598..641` -> existing app fire effects tests -> likely ownership: app/render should consume sim projectile state or at least not define authoritative impact timing.
- Existing projectile sim surfaces -> `src/sim/movement/homing_movement.rs`, `src/sim/movement/rocket_movement.rs`, `src/sim/game_entity.rs` projectile state comments -> likely ownership: add a generic bullet/arcing projectile path instead of reusing homing missile movement blindly.

## Remaining Uncertainty

- Exact frame count from Grizzly FLH to each target distance/elevation was not solved into a numeric golden table in this slot. The binary path is clear, but a follow-up should run or port the ballistic solver before writing exact frame-count tests.
- Object-list scheduling can affect whether a newly launched bullet receives its first `BulletClass::AI` update in the same global frame or the next frame. The handoff should make first-update ordering explicit when implementing projectile entities.
- Terrain collision details are verified as present in `BulletClass::AI`, but this slot did not exhaustively decode every `SubjectToCliffs/SubjectToWalls/SubjectToElevation` branch.

## Stale Doc Fixes Suggested

- `C:/Users/enok/Documents/ra2-rust-game-docs/units/allied/MTNK.md`: replace "Bullet speed" in the `[105mm]` / `[105mmE]` comparison with "weapon speed field; arcing `Cannon` launch/timing is handled by generic `BulletClass` ballistic setup and impact occurs on bullet detonation, not on the firing tick."
- `C:/Users/enok/Documents/ra2-rust-game-docs/units/allied/MTNK.md`: expand the projectile section with: "Stock Grizzly uses `[Cannon] Image=120MM`, not a Grizzly-specific projectile. `TechnoClass::Fire_At` launches a live arcing `BulletClass`; damage and impact animations occur when `BulletClass::AI` reaches a detonation condition and calls `WarheadTypeClass::Detonate`."

## Status

COMPLETE.
