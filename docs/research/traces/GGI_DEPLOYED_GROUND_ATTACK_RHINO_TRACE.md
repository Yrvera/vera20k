# Guardian GI Deployed Ground Attack vs Rhino Trace

Scenario: rookie Guardian GI (`GGI`) is already deployed at cell `(50,50)` and attacks an enemy Rhino Tank (`HTNK`) at 6 cells in standard Yuri's Revenge rules.

Scope is only this one attack. Adjacent observations are listed at the end and were not expanded into separate traces.

## Verdict

**PARTIAL**: gamemd weapon-selection, infantry fire-frame, damage, and projectile evidence was verified from existing docs plus read-only Ghidra decompilation, and Rust implementation paths were traced. No live scenario harness was run, so stages that require an actual rendered/audio frame capture are marked `UNCHECKED` unless the implementation output is directly determined by code.

Tally: **PASS: 1 | FAIL: 6 | UNCHECKED: 3 | NOT-IMPLEMENTED: 1**

## Pipeline

`Attack target set` -> `select weapon` -> `range/min-range gate` -> `enter DeployedFire` -> `fire-frame gate` -> `TechnoClass::Fire_At` -> `spawn DRAGON/AAHeatSeeker2 projectile` -> `impact GUARDWH damage` -> `report sound lookup` -> `render missile/explosion/damage result`.

## Concrete Rules Inputs

- `GGI`: `Primary=M60`, `Secondary=MissileLauncher`, `DeployFire=yes`, `Deployer=yes`, rookie `EliteSecondary` not used. Source: `ini/rulesmd.ini:3867-3869`, `ini/rulesmd.ini:3898-3910`.
- `HTNK`: Rhino has `Strength=400`, `Armor=heavy`. Source: `ini/rulesmd.ini:7687-7690`.
- `MissileLauncher`: `Damage=40`, `ROF=40`, `Range=8`, `Burst=1`, `Projectile=AAHeatSeeker2`, `Speed=30`, `Warhead=GUARDWH`, `Report=GuardianGIDeployedAttack`, `MinimumRange=1`. Source: `ini/rulesmd.ini:22569-22578`.
- `AAHeatSeeker2`: `Arm=2`, `Ranged=yes`, `AA=yes`, `AG=yes`, `Image=DRAGON`, `ROT=60`, ignores cliffs/elevation. Source: `ini/rulesmd.ini:25678-25689`.
- `GUARDWH`: heavy armor Verses column is `100%`, `CellSpread=.5`, `PercentAtMax=.5`, `AnimList=...`. Source: `ini/rulesmd.ini:26902-26912`.
- `GGI` art: `FireUp=2`, `PrimaryFireFLH=80,0,105`, `SecondaryFireFLH=80,0,90`; `GuardianGISequence` has `DeployedFire=323,6,6`. Source: `ini/artmd.ini:291-299`, `ini/artmd.ini:14166-14185`.
- `DRAGON` art has `UseLineTrail=yes`, trail color `216,216,255`, `Rotates=yes`. Source: `ini/artmd.ini:14755-14760`.
- `GuardianGIDeployedAttack` is not defined in `soundmd.ini`; grep found only weapon references in `rulesmd.ini`. Source: `ini/soundmd.ini:1027-1073`, `ini/rulesmd.ini:22577`.

## Stage Results

| # | Stage | gamemd output | VERA20k output | Verdict |
|---|---|---|---|---|
| 1 | Scenario geometry | 6 cells = 1536 leptons using 256 leptons/cell; this is between min 1 cell and max 8 cells for `MissileLauncher`. | Range code uses `range_cells * 256`, so the same supplied distance is 1536 leptons. | PASS |
| 2 | Weapon selection | `InfantryClass::SelectWeapon @ 0x005218e0` returns `type+0x6A8` for deployed Doing `0x1B..0x1E`; GGI default deploy weapon is slot `1`, so `MissileLauncher`. Active YR path, verified by read-only Ghidra. | `select_weapon_with_override` only applies transport overrides, then tries primary first. Deployed state is not passed into selection, so `M60` is selected because it can target ground heavy armor. See `src/sim/combat/mod.rs:1568` and `src/sim/combat/combat_weapon.rs:197`. | FAIL |
| 3 | Range gate | `MissileLauncher`: range 8, minimum 1. At 6 cells, fire is legal. | Chosen `M60` has range 4 (`ini/rulesmd.ini:22922-22925`), so `is_within_range_leptons` rejects 6 cells and combat continues without firing. See `src/sim/combat/mod.rs:1650` and `src/sim/combat/mod.rs:1710`. | FAIL |
| 4 | ROF / burst | `MissileLauncher` fires one missile and starts ROF 40 frames after `TechnoClass::Fire_At`; Ghidra shows fire timer set from `GetROF` after the shot. | Because range rejects before fire, no cooldown is set for this scenario. The alternative secondary path was not executed in a harness. | UNCHECKED |
| 5 | DeployedFire animation | `InfantryClass::Fire_At_Target @ 0x005206b0` enters Doing `0x1D` (`DeployedFire`) for deployed Doing `0x1B..0x1E`. Active YR path, verified by read-only Ghidra. | Since the selected primary is out of range, combat never calls the animation switch, so the unit stays in deployed idle instead of firing. See `src/sim/combat/mod.rs:1764-1779`. | FAIL |
| 6 | Fire-frame gate | For secondary deployed fire, Ghidra shows the frame anchor switches to `type+0xE48` when the sequence has `FireProne`; existing GGI research resolves absent `SecondaryFire=` to frame `0` for GGI. | Art parser defaults absent `SecondaryFire=` to `FireUp`, and absent `SecondaryProne=` to `SecondaryFire`, producing frame `2` for GGI. Deployed secondary fire uses `secondary_prone_frame`. See `src/rules/art_data.rs:315-322` and `src/sim/combat/mod.rs:223-228`. | FAIL |
| 7 | FLH / fire origin | Missile should spawn from secondary FLH `80,0,90` relative to the deployed infantry facing. | No `SimFireEvent` is produced in this scenario, so no secondary FLH origin is resolved. If a secondary event existed, `resolve_fire_origin_from_art` would choose `SecondaryFireFLH` from the weapon slot. See `src/app_fire_effects.rs:38-58`. | FAIL |
| 8 | Projectile | `TechnoClass::Fire_At @ 0x006FDD50` allocates a `BulletClass`; for `AAHeatSeeker2`, the visible projectile is `DRAGON` with homing/ROT behavior. Active YR path, verified by read-only Ghidra. | General weapon projectile flight is not implemented in this combat path; damage/effects are applied immediately at the target cell when a shot fires. In this scenario the shot does not fire at all. See `src/sim/combat/mod.rs:1785-1903`. | NOT-IMPLEMENTED |
| 9 | Warhead damage | On direct impact at target center: base 40, heavy armor Verses 100%, distance 0 within `CellSpread=.5`; `FUN_00489180` returns 40. Rhino HP should become 360 after impact, before other defender-side effects. | Current scenario output is 0 damage because no missile is fired. See range failure above and damage dispatch at `src/sim/combat/mod.rs:1798-1817`. | FAIL |
| 10 | Report sound | Retail `Report=GuardianGIDeployedAttack` is a dangling sound ID; existing GGI doc and local soundmd grep indicate deployed GGI missile fire is silent. | No shot means no report event. If a shot existed, `SimFireEvent.report_sound_id` would carry `GuardianGIDeployedAttack`; audio would need registry/assets resolution checked to prove audible silence. See `src/sim/combat/mod.rs:1905-1923` and `src/audio/sfx.rs:150-185`. | UNCHECKED |
| 11 | Screen-visible result | Deployed GGI should visibly enter `DeployedFire`, produce a `DRAGON` missile/trail from secondary FLH, then show a GUARDWH explosion and Rhino health loss after projectile travel. | GGI remains deployed idle and Rhino remains at 400 HP because the primary range gate prevents fire. | FAIL |
| 12 | End-to-end screenshot/audio capture | Requires retail frame capture and VERA20k scenario capture. | Not run in this subagent trace. | UNCHECKED |

## Player-Visible Failures

1. **Weapon selection/range**: deployed Guardian GI at 6 cells does not fire in VERA20k; retail fires a missile.
2. **DeployedFire animation**: VERA20k never shows the deployed firing sequence in this scenario.
3. **Projectile**: VERA20k has no visible `DRAGON` homing missile/trail for this weapon path.
4. **Damage**: Rhino remains at 400 HP in VERA20k; retail should reduce it to 360 after a direct missile hit.
5. **Fire-frame timing**: even after weapon selection is fixed, VERA20k's parsed secondary deployed fire anchor is frame 2, while the current gamemd evidence points to frame 0 for GGI.

## Evidence Notes

- Read-only Ghidra was used only for direct verification and did not mutate the program.
- Verified gamemd references used:
  - `InfantryClass::SelectWeapon` / `FUN_005218e0`: deployed Doing `0x1B..0x1E` returns `type+0x6A8`.
  - `InfantryClass::Fire_At_Target @ 0x005206b0`: deployed state enters `0x1D`; frame gate compares strict equality to the selected fire-frame anchor; secondary path can select `type+0xE48`.
  - `TechnoClass::Fire_At @ 0x006FDD50`: allocates `BulletClass`, then sets ROF timer after the shot.
  - `FUN_00489180`: master per-target damage transform, including CellSpread and Verses.
- Existing research docs used: `GGI_GHIDRA_REPORT.md`, `units/allied/GGI.md`, `TECHNOCLASS_COMBAT_WEAPON_SYSTEMS_REPORT.md`, and `combat/systems/damage_formula.md`.
- Active standard YR status: existing GGI report marks the deploy weapon-select/fire path, fire-at path, and damage path active in stock YR; the read-only decompilations matched those active paths. TS-legacy-only notes (`ProneDamage`, `DeployedSounds`) were not treated as runtime behavior.

## Adjacent Findings

- `ProneDamage=50%` on `GUARDWH` is parsed but the current GGI research says it is dead data in YR; this matters for infantry targets, not the Rhino scenario.
- `GuardianGIDeployedAttack` is also referenced by other missile weapons in `rulesmd.ini`; this trace only covers rookie deployed `GGI` versus `HTNK`.
- `OpenTransportWeapon=1` and `IFVMode=16` are GGI-adjacent transport behaviors and were not traced here.
