# Combat 3D Target Coordinate And Height Resolution Trace

Date: 2026-05-21

Scope: one concrete scenario only. A stock Allied Grizzly tank (`[MTNK]`) fires its stock primary weapon (`[105mm]`, projectile `[Cannon]`) at a stock ground target standing on an elevated or bridge-deck cell near the range boundary.

Concrete setup used for numeric checks:

- Attacker: `MTNK` at cell `(10,10)`, subcell `(128,128)`, elevation level `0`.
- Target: `MTNK` at cell `(15,10)`, subcell `(128,128)`, elevation level `4`.
- Elevation/lepton conversion: `4 * 104 = 416` leptons.
- Horizontal separation: `5 * 256 = 1280` leptons.
- Weapon: `[105mm] Range=5`, `Projectile=Cannon`.
- Projectile: `[Cannon] Arcing=true`, `SubjectToElevation=yes`.

## Verdict Summary

PASS: 0 | FAIL: 0 | UNCHECKED: 6 | NOT-IMPLEMENTED: 2

Overall status: PARTIAL. The trace reached the active YR range and fire paths, but exact gamemd numerical output for the arcing slope helper and projectile screen endpoint was not computed. Per swarm rules those stages are UNCHECKED, not PASS.

## Sources Checked

- Research: `C:/Users/enok/Documents/ra2-rust-game-docs/TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md`
- Research: `C:/Users/enok/Documents/ra2-rust-game-docs/WEAPONTYPECLASS_RUST_VS_FIRE_AT_TRACE.md`
- INI: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`
- Rust: `C:/Users/enok/Documents/ra2-rust-game/src/sim/combat/mod.rs`
- Rust: `C:/Users/enok/Documents/ra2-rust-game/src/sim/combat/in_range.rs`
- Rust: `C:/Users/enok/Documents/ra2-rust-game/src/app_fire_effects.rs`
- Rust: `C:/Users/enok/Documents/ra2-rust-game/src/util/lepton.rs`
- Rust: `C:/Users/enok/Documents/ra2-rust-game/src/sim/components.rs`
- Ghidra read-only decompilation:
  - `0x006F7220`, `TechnoClass__InRange`
  - `0x006FDD50`, `TechnoClassFireAtSpawnsBullet`
  - `0x005F65A0`, `ObjectClass__GetCoords`
  - `0x0041C380`, `CoordStruct__Distance3D`

All gamemd references above are active standard Yuri's Revenge paths for normal weapon range/fire handling. No TS-only dormant path was used as a verdict source.

## Active Gamemd Path

`TechnoClass__InRange` at `0x006F7220` is the active range decision path. It receives attacker, source `CoordStruct`, target object, and weapon. It reads the weapon range, resolves target coordinates through the target object coordinate path, applies minimum and maximum range checks, and has special handling for arcing projectiles, high/low flight, elevation, building foundation bonus, and bridge/deck height gates.

For `[105mm]`, the selected projectile is `[Cannon]`; because `[Cannon] Arcing=true`, this scenario enters the arcing branch, not the default 3D Euclidean max-range branch. The branch uses horizontal distance plus additional arcing/slope logic. `SubjectToElevation=yes` is active for `[Cannon]`, so height/elevation range contribution is part of the gamemd behavior that matters near the boundary.

`TechnoClassFireAtSpawnsBullet` at `0x006FDD50` is the active fire path. It allocates a `BulletClass` for non-invisible projectiles such as `120MM`, resolves target coordinates, initializes projectile flight, emits weapon animation/sound, then reveals on fire. This is not a dormant legacy path.

## Our Path

The Rust combat tick resolves an entity target through `target_coords` and, when terrain and attacker entity data are available, delegates normal weapon range checks to `in_range::compute_in_range`.

For the concrete target:

- Rust attacker source leptons: `(10*256+128, 10*256+128, 0*104) = (2688, 2688, 0)`.
- Rust target leptons: `(15*256+128, 10*256+128, 4*104) = (3968, 2688, 416)`.
- Rust arcing horizontal distance: `sqrt((3968-2688)^2 + (2688-2688)^2) = 1280`.
- Rust base range: `5 * 256 = 1280`.
- Rust arcing range verdict for this setup: in range before turret/cooldown checks because `1280 <= 1280`.

The non-arcing 3D distance for the same coordinates would be `sqrt(1280^2 + 416^2) ~= 1346`, but that is not the active gamemd branch for `[105mm]/[Cannon]`.

## Stage Table

| Stage | Gamemd evidence | Rust evidence | Verdict |
|---|---|---|---|
| Weapon/projectile selection | `[MTNK] Primary=105mm`; `[105mm] Projectile=Cannon`; `[Cannon] Arcing=true`, `SubjectToElevation=yes`; active `TechnoClass__InRange` consumes the selected weapon/projectile behavior. | `src/sim/combat/mod.rs:1591` selects weapon data and `src/sim/combat/in_range.rs:167` dispatches arcing projectiles to `compute_in_range_arcing_2d`. | UNCHECKED: INI identity and branch shape are verified, but no gamemd runtime selected-weapon numeric trace was captured for this exact map object. |
| Target coordinate extraction | `ObjectClass__GetCoords` at `0x005F65A0` copies object X/Y/Z coordinate fields; `TechnoClass__InRange` calls the target coordinate path. | `src/sim/combat/mod.rs:295` returns unit target position; for this target it resolves `(3968,2688)` and target Z `416` leptons through `src/sim/combat/in_range.rs:242`. | UNCHECKED: Rust numbers are computed; equivalent gamemd concrete object field values were not sampled live. |
| Target height contribution | Active gamemd range code uses target Z/ground/bridge height paths; low-flying and bridge cases are explicit in `TechnoClass__InRange`. For this ground target, the relevant concrete value should be the target object's absolute Z. | `src/sim/combat/in_range.rs:31` computes `position.z * LEPTONS_PER_LEVEL`; `src/util/lepton.rs:76` defines `104`, yielding `416` leptons. | UNCHECKED: Rust target Z is computed; gamemd target object Z for this exact scenario was not numerically sampled. |
| Range distance and fire/no-fire boundary | Active gamemd `[Cannon]` arcing branch uses horizontal distance plus arcing/slope logic and inclusive max-range comparison; exact slope-helper result was not computed. | `src/sim/combat/in_range.rs:212` uses only 2D distance for arcing projectiles. For this scenario it computes `1280 <= 1280` and allows fire if cooldown/facing permit. | UNCHECKED: Rust fire decision is computed, but exact gamemd arcing slope-helper output was not computed. |
| Elevation range contribution | Active gamemd code has `SubjectToElevation` height-fire bonus logic for projectiles such as `[Cannon]`, which affects uphill/elevated boundary cases. | `src/sim/combat/in_range.rs:212` arcing range uses base range only. `src/sim/combat/in_range.rs:135` has `height_fire_bonus_leptons` as a zero stub even outside the arcing branch. | NOT-IMPLEMENTED: player-visible fire/no-fire can diverge near elevated target range boundaries where gamemd extends or modifies effective range. |
| Bridge/deck height gate | Active gamemd range code contains a bridge/deck vertical gate preventing under-bridge shots through the deck to targets above it. | `src/sim/combat/in_range.rs:347` implements a non-arcing bridge gate; the arcing helper path at `src/sim/combat/in_range.rs:212` does not run this same gate. | UNCHECKED: the concrete attacker is not placed under the same bridge deck, so this gate is adjacent to this scenario and was not verdict-tested here. |
| Projectile creation and hit timing | Active gamemd `TechnoClassFireAtSpawnsBullet` allocates and initializes a real `BulletClass` for visible `[Cannon]`/`120MM` projectile flight before detonation/hit. | `src/sim/combat/mod.rs:1844` proceeds directly through damage/effect handling on the fire tick; `src/app_fire_effects.rs:234` creates an app-side visual interpolation only. | NOT-IMPLEMENTED: authoritative projectile flight and delayed hit/miss resolution are absent for this stock cannon shot. |
| Projectile/fire-effect target screen position | Active gamemd fire path resolves target coordinates for the bullet and projectile rendering path. Exact screen endpoint was not numerically computed in this run. | `src/app_fire_effects.rs:185` chooses the app visual destination from the current target entity `position.screen_x/screen_y`; `src/sim/world/mod.rs:193` fire events do not carry resolved fire-time target coords or target Z leptons. | UNCHECKED: Rust visual endpoint source is identified, but literal gamemd endpoint pixels were not computed. |

## Player-Visible Findings

1. NOT-IMPLEMENTED, height/elevation range contribution: `[Cannon] SubjectToElevation=yes` is active in gamemd, but Rust's arcing path ignores height-fire bonus and the shared helper returns zero. Near elevated boundary shots may fire in gamemd and not fire in Rust, or have different allowable boundary behavior. Rust: `src/sim/combat/in_range.rs:135`, `src/sim/combat/in_range.rs:212`. Gamemd: active `TechnoClass__InRange` at `0x006F7220`.

2. NOT-IMPLEMENTED, authoritative projectile flight: gamemd creates a real `BulletClass` for the visible `120MM` projectile, while Rust resolves combat damage/effects on the fire tick and only creates an app-side projectile visual. The player-visible result can differ in hit timing, misses against moving targets, and impact location. Rust: `src/sim/combat/mod.rs:1844`, `src/app_fire_effects.rs:234`. Gamemd: active `TechnoClassFireAtSpawnsBullet` at `0x006FDD50`.

## Adjacent Findings

- Building targets on elevated terrain need a separate trace. Rust range targeting uses foundation center in `src/sim/combat/mod.rs:295`, while app projectile visuals use the entity's cached screen position in `src/app_fire_effects.rs:185`; this may diverge for multi-cell buildings, but that is outside this unit-target trace.
- Moving targets need a separate trace. `SimFireEvent` does not carry resolved fire-time target coordinates, so the app-side visual may resolve against the target's later position or absence.
- Garrison and special override range paths still have 2D-only behavior in parts of `src/sim/combat/mod.rs`; this trace did not cover garrison fire.
- Under-bridge-to-deck shots need a separate bridge-specific trace. The active gamemd bridge vertical gate exists, but this concrete attacker was not under the same bridge deck.

