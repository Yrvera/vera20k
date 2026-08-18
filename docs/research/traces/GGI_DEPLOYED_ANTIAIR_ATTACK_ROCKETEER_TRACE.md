# Guardian GI Deployed Anti-Air Attack vs Rocketeer Trace

Scenario: rookie `[GGI]` is already deployed at cell `(50,50)` and attacks an enemy `[JUMPJET]` Rocketeer at 6 cells in standard Yuri's Revenge rules. For numeric distance, this trace uses a cardinal target offset `(56,50)`, same ground level, cell centers, no bridge.

Status: PARTIAL. The current Rust path fails before the expected gamemd firing stage, so downstream projectile flight, impact tick, and exact screen pixels could not be compared as a reached Rust output. I still checked the relevant production code paths read-only and recorded masked downstream gaps.

## Evidence Used

- Research docs checked first:
  - `docs/research/GGI_GHIDRA_REPORT.md`
  - `docs/research/units/allied/GGI.md`
  - `docs/research/units/allied/JUMPJET.md`
  - `docs/research/FIRE_AT_PIPELINE_GHIDRA_REPORT.md`
  - `docs/research/TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md`
  - `docs/research/BULLETCLASS_TRAJECTORY_AND_HOMING.md`
- INI evidence:
  - `ini/rulesmd.ini:3863` `[GGI]`
  - `ini/rulesmd.ini:3916` `[JUMPJET]`
  - `ini/rulesmd.ini:3960` `JumpjetHeight=500`
  - `ini/rulesmd.ini:22569` `[MissileLauncher]`
  - `ini/rulesmd.ini:22922` `[M60]`
  - `ini/rulesmd.ini:25678` `[AAHeatSeeker2]`
  - `ini/rulesmd.ini:26902` `[GUARDWH]`
  - `ini/artmd.ini:287` `[GGI]`, `FireUp=2`, `PrimaryFireFLH=80,0,105`, `SecondaryFireFLH=80,0,90`
  - `ini/artmd.ini:14166` `[GuardianGISequence]`, `DeployedFire=323,6,6`
  - `ini/soundmd.ini:1041` `[GuardianGiDeployedAttack]`, `Sounds= iggiat2a iggiat2b`
- Read-only Ghidra checks:
  - `InfantryClass__SelectWeapon` at `0x005218e0`
  - `InfantryClass__Fire_At_Target` at `0x005206b0`
  - `TechnoClass__GetFireError` at `0x006FC0B0`
  - `TechnoClass__InRange` at `0x006F7220`
  - `TechnoClass__Fire_At` at `0x006FDD50`
- No mutating Ghidra tools were used.
- No tests or builds were run because the task allowed writing exactly one file and test/build artifacts would write outside this report.

## Concrete Expected gamemd Values

- Rocketeer is a live standard YR JumpJet infantry with `ConsideredAircraft=yes`, `JumpJet=yes`, `BalloonHover=yes`, `HoverAttack=yes`, `JumpjetHeight=500`.
- Deployed rookie GGI uses `DeployFireWeapon=1`, so weapon slot 1, `Secondary=MissileLauncher`.
- `MissileLauncher`: `Damage=40`, `ROF=40`, `Range=8`, `MinimumRange=1`, `Burst=1`, `Projectile=AAHeatSeeker2`, `Warhead=GUARDWH`, `Report=GuardianGIDeployedAttack`.
- `AAHeatSeeker2`: `AA=yes`, `AG=yes`, `Ranged=yes`, `Image=DRAGON`, `ROT=60`, `Arm=2`, `Shadow=no`, `Proximity=no`.
- Range at 6 cells: distance `6 * 256 = 1536` leptons. Legal interval is `[1, 8]` cells, with max inclusive and min strict in gamemd. Result: in range.
- GGI deployed fire sequence: `DeployedFire=323,6,6`; shot occurs when current animation frame equals `FireUp=2`.
- FLH for this shot: `SecondaryFireFLH=80,0,90`.
- Direct centered damage to Rocketeer armor `none`: `40 * GUARDWH.Verses[none=20%] / 100 = 8`. Rocketeer health goes `125 -> 117` at missile detonation, not at fire-frame start.
- Report sound resolves to `[GuardianGiDeployedAttack]`, with sound candidates `iggiat2a` and `iggiat2b`.

## Stage Verdicts

| Stage | gamemd output | Rust output for this scenario | Verdict |
|---|---:|---:|---|
| 1. YR-active path | Standard infantry fire path is active in YR (`0x005206b0`, `0x005218e0`, `0x006FC0B0`, `0x006FDD50`) | Combat tick path is active for entities with attack targets | PASS |
| 2. Target legality category | Rocketeer is air target for AA legality because `ConsideredAircraft=yes` and airborne JumpJet | Rocketeer spawns as `EntityCategory::Infantry`; `considered_aircraft` is parsed but not consumed | FAIL |
| 3. Weapon selection | Deployed GGI returns `DeployFireWeapon=1`, therefore `MissileLauncher` secondary | `select_weapon_with_override` has no deployed `DeployFireWeapon` override; with target category Infantry it selects primary `M60` | FAIL |
| 4. Range gate | `MissileLauncher` range 8, min 1; 1536 leptons is legal | Selected `M60` range 4; 1536 leptons is greater than 1024, so Rust does not fire | FAIL |
| 5. DeployedFire animation | Starts `DeployedFire`, fires at frame 2 of 6 | Not reached because range gate rejects the selected M60 | FAIL |
| 6. FLH | `SecondaryFireFLH=80,0,90` | No fire origin emitted; if M60 path fired it would use primary FLH `80,0,105` | FAIL |
| 7. ROF | Firing writes `ROF=40` frame cooldown | No cooldown is written because no shot fires | FAIL |
| 8. Damage amount and timing | 8 damage at missile detonation after projectile flight | 0 damage in the actual scenario because no shot fires | FAIL |
| 9. Projectile AA behavior | Spawns DRAGON `AAHeatSeeker2` homing BulletClass with ROT 60 and Arm 2 | Not reached; production fire path also has no projectile-spawn dispatch for this weapon | NOT-IMPLEMENTED |
| 10. Report sound | Plays `GuardianGIDeployedAttack` (`iggiat2a` or `iggiat2b`) from the fire position | No sound event because no shot fires. Sound parser also ignores FShift/VShift/Control/Limit, so exact audible parity remains unchecked after the early fix | FAIL |
| 11. Screen-visible result | On frame 2 of deployed fire, missile launches from GGI secondary FLH, tracks Rocketeer, detonates for 8 damage | GGI does not fire at all at 6 cells; no deployed attack frame, no missile, no report sound, no damage | FAIL |

Verdict tally: PASS: 1 | FAIL: 9 | UNCHECKED: 0 | NOT-IMPLEMENTED: 1

## Primary Findings

1. Target legality fails first. `src/sim/world/world_spawn.rs:310` maps all infantry-section objects, including Rocketeer, to `EntityCategory::Infantry`; `src/rules/object_type.rs:944` parses `ConsideredAircraft` but `rg` found no runtime consumer. gamemd treats this active standard YR Rocketeer as an air target for `GetFireError` AA legality.
2. Weapon selection is wrong for deployed GGI. `src/sim/combat/combat_weapon.rs:197` falls through Primary then Secondary and `src/sim/combat/mod.rs:1568` calls it without a deployed-fire override; gamemd `InfantryClass__SelectWeapon 0x005218e0` returns `DeployFireWeapon=1` for deployed fire actions, selecting `MissileLauncher`.
3. The wrong weapon makes the 6-cell attack fail range. Rust selects `M60` (`Range=4`) and `src/sim/combat/in_range.rs:197` rejects distance `1536 > 1024`; gamemd uses `MissileLauncher` (`Range=8`) and accepts `1536 <= 2048`.
4. The visible attack never begins. Rust does not switch to `DeployedFire`, emit FLH, queue report sound, or damage the target because the range gate rejects before fire sync; gamemd starts `DeployedFire=323,6,6` and fires on frame `2`.
5. Projectile behavior is not implemented in the reached production fire path. `src/sim/world/mod.rs:1109` notes projectile-spawn dispatch is deferred, while `src/sim/combat/mod.rs:1799`/`1816` applies warhead effects directly when a shot fires. gamemd `TechnoClass__Fire_At 0x006FDD50` creates a BulletClass for `AAHeatSeeker2`.

## Adjacent Findings

- `src/audio/sfx.rs:164` and `src/audio/sfx.rs:201` choose sound candidates by incrementing a local counter; `SoundEntry` does not store `FShift`, `VShift`, `Control`, or `Limit`. This is adjacent to this trace because the current scenario never reaches report playback, but it will matter after the no-fire bug is fixed.
- `src/sim/combat/mod.rs:217` uses `secondary_prone_frame` for deployed secondary fire. For GGI this happens to equal `FireUp=2` because `src/rules/art_data.rs:316`/`320` default `SecondaryFire` to `FireUp` and `SecondaryProne` to `SecondaryFire`, but this is a fragile match and not proof of general deployed-fire parity.
