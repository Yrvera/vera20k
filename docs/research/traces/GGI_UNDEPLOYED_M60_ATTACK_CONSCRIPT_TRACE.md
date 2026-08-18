# Guardian GI Undeployed M60 Attack vs Conscript Trace

Scenario: rookie Guardian GI (`GGI`) is undeployed at cell `(50,50)` and attacks an enemy Conscript (`E2`) at exactly 4 cells in standard Yuri's Revenge rules.

Scope is only this concrete shot. Adjacent deployed/BFRT/IFV/Garrison behavior is noted only where it explains why the undeployed shot must remain primary `M60`.

## Verdict Tally

PASS: 7 | FAIL: 0 | UNCHECKED: 4 | NOT-IMPLEMENTED: 0

## Sources Checked

- `C:/Users/enok/Documents/ra2-rust-game-docs/GGI_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/units/allied/GGI.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/TECHNOCLASS_COMBAT_WEAPON_SYSTEMS_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/timing/weapon-rof-burst.md`
- `ini/rulesmd.ini`, `ini/artmd.ini`, `ini/soundmd.ini`
- Rust source under `src/sim/combat`, `src/sim/animation.rs`, `src/rules`, `src/app_fire_effects.rs`, and `src/audio/events.rs`

Ghidra note: no mutating Ghidra tools were used. The session exposed no decompiler/search tools beyond read-only IOC extraction, so live decompilation gaps were resolved from existing verified binary reports rather than new binary output.

## Scenario Values

### Rules / Art Data

- `GGI`: `Primary=M60`, `Secondary=MissileLauncher`, `DeployFire=yes`, `Crushable=yes`, `DeployedCrushable=no`, `ElitePrimary=M60E`, `EliteSecondary=MissileLauncherE`.
  Evidence: `ini/rulesmd.ini:3863-3912`.
- Undeployed GGI art: `Sequence=GuardianGISequence`, `FireUp=2`, `PrimaryFireFLH=80,0,105`, `SecondaryFireFLH=80,0,90`.
  Evidence: `ini/artmd.ini:291-299`.
- `GuardianGISequence` primary fire visual is `FireUp=204,6,6`; deployed fire visual is separate `DeployedFire=323,6,6`.
  Evidence: `ini/artmd.ini:14166-14187`.
- Conscript `E2`: `Image=CONS`, `Strength=125`, `Armor=flak`.
  Evidence: `ini/rulesmd.ini:4327-4339`.
- `M60`: `Damage=15`, `ROF=20`, `Range=4`, `Projectile=InvisibleLow`, `Speed=100`, `Warhead=SA`, `Report=GIAttack`, `Anim=MGUN-N,...,MGUN-NW`, `OccupantAnim=UCFLASH`.
  Evidence: `ini/rulesmd.ini:22922-22931`.
- `MissileLauncher`: `Damage=40`, `ROF=40`, `Range=8`, `Projectile=AAHeatSeeker2`, `Warhead=GUARDWH`, `Report=GuardianGIDeployedAttack`, `MinimumRange=1`.
  Evidence: `ini/rulesmd.ini:22569-22578`.
- `SA`: `Verses=100%,80%,80%,50%,25%,25%,75%,50%,25%,100%,100%`, `InfDeath=1`, `ProneDamage=70%`.
  Evidence: `ini/rulesmd.ini:26466-26473`.
- `GIAttack`: samples `igiat1a igiat1b igiat1c`, random interrupt, `VShift=15`, `Volume=65`.
  Evidence: `ini/soundmd.ini:1049-1053`.

## Pipeline

`Attack order` -> `attack_target set` -> `weapon selection` -> `target legality` -> `range/cooldown/fire-frame gates` -> `M60 direct damage` -> `fire event` -> `GIAttack sound + MGUN muzzle anim from PrimaryFireFLH` -> `Conscript HP visibly reduced`

## Stage Results

### 1. Attack Target Setup

Our path: `issue_attack_command` sets `attacker.attack_target = Some(AttackTarget::new(target_id))` without setting facing directly.
Evidence: `src/sim/combat/mod.rs:394-440`.

gamemd: existing GGI report identifies the shared InfantryClass AI/fire path as active in standard YR and confirms GGI has no special hardcoded branch.
Evidence: `GGI_GHIDRA_REPORT.md`, overview and active-in-YR notes.

Verdict: PASS for scenario binding and active YR path. Exact input-click frame ordering was not separately captured here.

### 2. Weapon Selection: M60, Not MissileLauncher

gamemd: `InfantryClass::SelectWeapon @ 0x005218E0` returns `DeployFireWeapon=1` only for deployed sequence IDs `0x1B..0x1E`; otherwise undeployed GGI returns primary slot 0. For GGI, primary slot 0 is `M60`.
Evidence: `GGI_GHIDRA_REPORT.md` section 3.2 and hardcoded behavior audit.

Our path: `select_weapon_with_override` tries primary before secondary when there is no IFV/open-transport override. For rookie `GGI` vs infantry, `M60` is selected.
Evidence: `src/sim/combat/combat_weapon.rs:149-220`; regression fixture at `src/sim/combat/combat_weapon.rs:467-539`.

Computed values:

- gamemd weapon slot: `0` -> `M60`.
- our weapon slot: `WeaponSlot::Primary` -> `M60`.

Verdict: PASS.

### 3. Target Legality

Target is a ground infantry object with `Armor=flak`. `InvisibleLow` has no explicit `AA`/`AG`, and Rust projectile parsing defaults `AG=true`, `AA=false`, matching the standard ground-projectile behavior needed here.
Evidence: `ini/rulesmd.ini:25385-25390`; `src/rules/projectile_type.rs:8-14`, `src/rules/projectile_type.rs:169-174`.

Warhead gate:

- `SA` versus `flak` is the second armor column: `80%`.
- This is greater than zero, so the target is legal.

Our path checks projectile engagement and `Verses > 0` before accepting the weapon.
Evidence: `src/sim/combat/combat_weapon.rs:277-306`, `src/sim/combat/combat_weapon.rs:314-330`.

gamemd: the combat report identifies projectile/Verses gates in the live `GetFireError`/selection chain and marks these fields live in YR.

Verdict: PASS.

### 4. Range

Scenario distance: target is 4 cells away on flat ground.

Computed baseline:

- 1 cell = 256 leptons.
- Distance = `4 * 256 = 1024` leptons.
- M60 range = `4 * 256 = 1024` leptons.
- Our range helper checks `dist_sq <= range_sq`.
  Evidence: `src/sim/combat/mod.rs:2188-2190`; 3D range path at `src/sim/combat/in_range.rs:150-191`.

gamemd report confirms `InRange @ 0x006F7220` reads weapon range in leptons and performs 3D distance logic, but this run did not re-decompile the exact equality branch at the boundary.
Evidence: `TECHNOCLASS_COMBAT_WEAPON_SYSTEMS_REPORT.md:214-242`.

Verdict: UNCHECKED for literal boundary equality, because both exact gamemd branch output and our output were not obtained from an executable run in this trace. Expected player result: in range and fires.

### 5. Fire Animation / Fire Frame Gate

gamemd: GGI uses shared infantry fire path. The report states `Fire_At_Target @ 0x005206B0` reads `FireUp` and that GGI's standing primary gate is `FireUp=2`; deployed sequences are separate.
Evidence: `GGI_GHIDRA_REPORT.md`, runtime fields and per-tick AI/fire chain.

Our path:

- `WeaponSlot::Primary`, not prone, not deployed -> `SequenceKind::Attack`.
- `infantry_fire_frame` returns `obj.fire_up_frame`; after art merge, GGI `fire_up_frame=2`.
- Animation sequence visual is `GuardianGISequence` `FireUp=204,6,6`.
Evidence: `src/sim/combat/mod.rs:195-230`; `src/rules/ruleset.rs:1712-1730`; `src/sim/animation.rs:56-66`; `ini/artmd.ini:14171`.

Verdict: PASS for selected sequence and fire frame value. UNCHECKED for exact tick number from attack command to damage, because this trace did not run gamemd and Rust side by side.

### 6. Damage

Concrete damage:

- Weapon damage: `15`.
- Target armor: `flak`.
- `SA` versus flak: `80%`.
- Direct-hit damage: `15 * 80 / 100 = 12`.
- Conscript HP: `125 - 12 = 113`.

Our path:

- Direct non-AoE damage computes `base_damage * selected.verses_pct / 100`.
- Prone modifier is not applied because the scenario Conscript is not specified prone.
- Damage subtracts from `target.health.current` with saturating subtraction.
Evidence: `src/sim/combat/mod.rs:1785-1853`, `src/sim/combat/mod.rs:2032-2059`.

gamemd: existing combat report states `Fire_At @ 0x006FDD50` computes damage and immediately subtracts expected damage from target HP for non-inaccurate bullets; GGI report binds GGI to shared `ApplyWarheadDamage @ 0x00489180`.
Evidence: `TECHNOCLASS_COMBAT_WEAPON_SYSTEMS_REPORT.md:322-347`; `GGI_GHIDRA_REPORT.md` address list.

Verdict: PASS.

### 7. ROF / Cooldown

Concrete cooldown:

- M60 `ROF=20`.
- Standard YR simulation cadence is 15 Hz.
- gamemd report: ROF timer fields are set from weapon ROF and remaining cooldown is the ROF value.
- Rust `rof_to_cooldown_ticks(20, 67)` computes `ceil(20*1000/15)=1334ms`, then `ceil(1334/67)=20` ticks.
Evidence: `TECHNOCLASS_COMBAT_WEAPON_SYSTEMS_REPORT.md:349-356`, `TECHNOCLASS_COMBAT_WEAPON_SYSTEMS_REPORT.md:633-653`; `src/sim/combat/mod.rs:1948-1962`, `src/sim/combat/mod.rs:2193-2202`.

Verdict: PASS for cooldown duration of 20 game ticks after this shot.

### 8. Report Sound

gamemd expected firing report for undeployed rifle shot: `M60.Report=GIAttack`, resolving to `igiat1a/igiat1b/igiat1c`.
Evidence: `ini/rulesmd.ini:22929`; `ini/soundmd.ini:1049-1053`; `units/allied/GGI.md:344`.

Our path:

- `SimFireEvent.report_sound_id` is set for non-garrison fire.
- App fire effects convert it to `GameSoundEvent::WeaponFired { sound_id: "GIAttack", screen_pos: FLH origin }`.
Evidence: `src/sim/combat/mod.rs:1905-1929`; `src/app_fire_effects.rs:121-125`, `src/app_fire_effects.rs:156-173`; `src/audio/events.rs:22-31`.

Verdict: PASS for sound ID. UNCHECKED for exact random sample choice (`igiat1a/b/c`) because this trace did not compare RNG state against gamemd.

### 9. Muzzle Animation and FLH

gamemd expected:

- Primary FLH is `80,0,105`.
- Weapon muzzle anim list is `MGUN-N,MGUN-NE,MGUN-E,MGUN-SE,MGUN-S,MGUN-SW,MGUN-W,MGUN-NW`.
- GGI standing unit animation is `GuardianGISequence.FireUp=204,6,6`.
Evidence: `ini/artmd.ini:298`, `ini/rulesmd.ini:22930`, `ini/artmd.ini:14171`.

Our path:

- Fire origin uses `resolve_flh(primary_fire_flh, secondary_fire_flh, ..., WeaponSlot::Primary, veterancy=0)`.
- Muzzle anim selection uses 8-way facing-based selection.
Evidence: `src/app_fire_effects.rs:26-35`, `src/app_fire_effects.rs:38-65`, `src/app_fire_effects.rs:127-150`.

Verdict: UNCHECKED for literal pixel equality. The data values match, but this trace did not compute and compare gamemd screen coordinates for a concrete facing.

### 10. Screen-Visible Result

Expected player-visible result in standard YR:

- Undeployed GGI does not switch to missile stance.
- It plays normal standing Guardian GI rifle fire frames.
- A directional MGUN muzzle flash appears from primary FLH.
- `GIAttack` rifle report plays.
- Conscript remains alive, HP reduced from `125` to `113`.

Our traced output:

- Selected weapon: `M60`.
- Damage event: `12`.
- Cooldown: `20` ticks.
- Fire event: primary weapon, `GIAttack`, weapon anim list.

Verdict: UNCHECKED for literal screen pixels/audio sample/tick capture; no FAIL identified from code/data trace.

## Adjacent Findings

- Deployed GGI uses `MissileLauncher`, not `M60`, because `DeployFireWeapon` defaults to secondary slot 1 while the infantry is in deployed sequence IDs `0x1B..0x1E`. This is not traced further here.
- `MissileLauncher.Report=GuardianGIDeployedAttack` is a dangling sound reference in `soundmd.ini`, so deployed missile fire is silent in stock YR. Not relevant to this undeployed M60 shot.
- Battle Fortress/OpenTransport routes GGI to secondary by `OpenTransportWeapon=1`; not relevant to an undeployed infantryman standing on the map.

## Top Player-Visible FAIL / NOT-IMPLEMENTED Findings

None for this concrete trace. Remaining risk is exact parity not proved for boundary range comparison, attack-frame timing, random report-sample choice, and FLH pixel output because this run did not capture both executables numerically.
