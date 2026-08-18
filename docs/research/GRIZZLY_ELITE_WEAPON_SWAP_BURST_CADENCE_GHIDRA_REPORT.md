# Grizzly Elite Weapon Swap Burst Cadence - Ghidra Research Report

**Address(es):** `0x0070E140`, `0x006FCFA0`, `0x006FDD50`, `0x0070D0D0`, `0x0070BE80`, `0x00772080`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Stock YR `[MTNK]` elite primary swap to `[105mmE]`, burst cadence, end-of-burst ROF cooldown, damage/warhead selection, and firing-relevant `FIREPOWER`/`ROF`/`SELF_HEAL` ability effects.  
**Non-Scope:** XP award formula beyond the already-verified elite threshold, projectile physics after `Cannon` launch, FLH/muzzle origin, garrison/open-topped overrides, crate/naval/house spy edge modifiers except where they are in the same ROF formula.  
**Confidence:** Medium-high. This pass could not use fresh Ghidra MCP tools in the current session, so it relies on existing high-confidence binary-backed reports and direct INI/Rust scans. Handoff-critical claims cite decompiled-address evidence from those reports.  
**Active in YR:** Yes.

## 1. Overview

Stock YR Grizzly (`[MTNK]`) becomes elite at `Veterancy >= 2.0f` and swaps primary slot 0 from `[105mm]` to `[105mmE]`. The elite weapon keeps `Damage=65`, changes `ROF` to `50`, changes `Warhead` to `GRIZAPE`, changes `Anim` to `VTMUZZLE`, and adds `Burst=2`.

The elite burst is not two bullets in one `Fire_At` call. It is two independent fire calls separated by a non-infantry `RandomRanged(3,5)` delay; after the second shot, `GetROF` computes the full cooldown from `105mmE.ROF=50`, adds `Random(0,2)`, and applies `VeteranROF=0.6` because elite MTNK has the `ROF` ability.

## 2. Class Layout / Key Offsets

| Field | Offset | Type | Purpose | Active in YR |
|---|---:|---|---|---|
| `TechnoClass.Veterancy` | `+0x150` | float | Elite threshold source | Yes |
| `TechnoClass.FireTimer.StartFrame` | `+0x2EC` | int | current-frame snapshot after shot | Yes |
| `TechnoClass.FireTimer.InitialValue` | `+0x2F4` | int | cooldown compare value | Yes |
| `TechnoClass.FireTimer.ROF` | `+0x2F8` | int | active cooldown | Yes |
| `TechnoClass.CurrentBurstIndex` | `+0x3B8` | int | burst slot counter, wraps `% weapon.Burst` | Yes |
| `TechnoType.VeteranAbilities` | `+0x29C..0x2AD` | bool[18] | veteran ability flags | Yes |
| `TechnoType.EliteAbilities` | `+0x2AE..0x2BF` | bool[18] | elite ability flags | Yes |
| `TechnoType.Primary slot 0` | `+0x898` | weapon slot | regular primary | Yes |
| `TechnoType.ElitePrimary slot 0` | `+0xA94` | weapon slot | elite primary | Yes |
| `WeaponType.Burst` | `+0x9C` | int | shots per burst sequence | Yes |
| `WeaponType.Damage` | `+0xA4` | int | base per-shot damage | Yes |
| `WeaponType.Warhead` | `+0xAC` | pointer | warhead selected for projectile/detonation | Yes |
| `WeaponType.ROF` | `+0xB0` | int | end-of-burst delay base | Yes |
| `Rules.VeteranCombat` | `+0x670` | double | damage multiplier when `FIREPOWER` ability is present | Yes |
| `Rules.VeteranROF` | `+0x690` | double | ROF delay multiplier when `ROF` ability is present | Yes |

## 3. Stock INI Data

YR `rulesmd.ini` is authoritative over base `rules.ini`.

| Section | Key | Stock YR value | Base RA2 value | Effect |
|---|---|---:|---:|---|
| `[MTNK]` | `Primary` | `105mm` | `105mm` | regular/veteran primary |
| `[MTNK]` | `ElitePrimary` | `105mmE` | `105mmE` | elite slot 0 |
| `[MTNK]` | `VeteranAbilities` | `STRONGER,FIREPOWER,SIGHT,FASTER` | same | no veteran `ROF` |
| `[MTNK]` | `EliteAbilities` | `SELF_HEAL,STRONGER,FIREPOWER,ROF` | same | elite adds `ROF` and `SELF_HEAL` |
| `[105mm]` | `Damage` / `ROF` / `Warhead` / `Burst` | `65` / `60` / `AP` / default `1` | same | regular Grizzly cannon |
| `[105mmE]` | `Damage` / `ROF` / `Warhead` / `Burst` | `65` / `50` / `GRIZAPE` / `2` | `55` / `75` / `GRIZAPE` / `2` | YR override is materially stronger than base RA2 |
| `[GRIZAPE]` | `Verses` | `100,100,100,100,100,100,65,45,60,60,100` | same | elite warhead damage table |
| `[General]` | `VeteranCombat` | `1.1` | `1.1` | FIREPOWER damage delay-independent multiplier |
| `[General]` | `VeteranROF` | `0.6` | `0.6` | ROF delay multiplier |
| `[General]` | `RepairRate` | `.016` | `.016` | self-heal pulse interval via `RepairRate * 900` |

## 4. Core Logic

### 4.1 Elite Threshold And Weapon Swap

`VeterancyClass::IsElite @ 0x00750010` returns true when the `TechnoClass+0x150` float is `>= 2.0f`. `TechnoClass::GetWeapon @ 0x0070E140` checks `IsElite`; if true, it looks up the elite weapon slot at `type + 0xA94 + idx * 0x1C` and returns it if the slot's weapon pointer is non-null. Otherwise it falls back to the regular slot at `type + 0x898 + idx * 0x1C`.

For stock Grizzly slot 0, elite `GetWeapon(0)` returns `[105mmE]`; rookie and veteran `GetWeapon(0)` return `[105mm]`. Veteran tier does not swap weapons.

### 4.2 Burst Cadence

`TechnoClass::Fire_At @ 0x006FDD50` launches exactly one projectile per call. After the shot it increments `CurrentBurstIndex`, calls `GetROF @ 0x006FCFA0`, stores the returned cooldown in `+0x2F8/+0x2F4`, snapshots `g_CurrentFrameCounter` to `+0x2EC`, then wraps `CurrentBurstIndex %= weapon.Burst`.

`GetROF @ 0x006FCFA0` reads `CurrentBurstIndex` after `Fire_At` has incremented it:

- For a non-infantry mid-burst shot where `0 < CurrentBurstIndex < weapon.Burst`, return `RandomRanged(3,5)`.
- For the last shot of the burst, compute full cooldown from weapon `ROF`, house ROF bonus, random `0..2` jitter, and ability/modifier branches.

For stock elite Grizzly `Burst=2`:

1. Shot 1 fires with `[105mmE]`, then `CurrentBurstIndex` becomes 1 and cooldown is random 3, 4, or 5 ticks.
2. Shot 2 fires with `[105mmE]`, then `CurrentBurstIndex` becomes 0 after modulo and full ROF cooldown is armed.

### 4.3 Elite ROF Ability Composition

The corrected veterancy report verifies `HasWeaponAbility(this, 4)` as the `ROF` bonus gate in `GetROF`, checking `type+0x2A0` and `type+0x2B2`. `ROF` is index 4 in the ability table. For elite MTNK, `EliteAbilities` includes `ROF`, so the full end-of-burst cooldown applies `Rules.VeteranROF`.

Stock non-garrison, no crate, no special house ROF edge:

```text
base_delay = ftol(105mmE.ROF * HouseROFBonus + Random(0,2))
           = 50, 51, or 52  (assuming default HouseROFBonus=1)

elite_rof_delay = ftol(base_delay * VeteranROF)
                = ftol(base_delay * 0.6)
                = 30, 30, or 31 ticks
```

This is after the 3..5 tick inter-shot delay between the two burst shots. The weapon's own `ROF=50` and the `ROF` ability both apply; do not choose one or the other.

### 4.4 Damage And Warhead Selection

`WeaponTypeClass::ReadINI @ 0x00772080` parses `Damage`, `ROF`, `Burst`, `Warhead`, and `Projectile` into the verified `WeaponTypeClass` fields. `TechnoClass::Fire_At @ 0x006FDD50` consumes `weapon+0xA4 Damage` and `weapon+0xAC Warhead`.

The veterancy report verifies `HasWeaponAbility(this, 2)` as `FIREPOWER` and the damage consumer at `Fire_At @ 0x006FE35E`. For stock elite MTNK, `FIREPOWER` is present both via inherited veteran flags and elite flags. With stock `VeteranCombat=1.1`, each `[105mmE]` projectile uses a damage payload equivalent to `ftol(65 * 1.1) = 71` before warhead `Verses`, prone, AoE, and armor-specific effects.

The selected warhead is `[GRIZAPE]`, not `[AP]`, because `GetWeapon(0)` returns `[105mmE]` and `[105mmE]` has `Warhead=GRIZAPE`.

### 4.5 SELF_HEAL Relevance To Firing

`SELF_HEAL` is ability index 9 (`type+0x2A5` / `type+0x2B7`) and is checked by `FUN_0070BE80`. It does not alter weapon selection, burst state, fire cooldown, or projectile damage. It affects combat state indirectly by restoring `+1 HP` on eligible pulses while damaged and alive.

The corrected self-heal path uses `RepairRate * 900` as the pulse interval. Stock `RepairRate=.016` gives `ftol(.016 * 900) = 14` frames. The older `SelfHealUnitFrames`/`SelfHealUnitAmount` names do not drive this per-unit veterancy self-heal path.

## 5. Current Rust Implementation Status

Current Rust parses `ElitePrimary`/`EliteSecondary` and selects elite primary at `veterancy >= 200` in `src/sim/combat/combat_weapon.rs`. `WeaponType` parses `Damage`, `ROF`, `Warhead`, and `Burst`.

Current Rust combat still differs in several Grizzly-relevant ways:

- `src/sim/combat/mod.rs` uses `BURST_INTER_SHOT_DELAY` instead of binary `RandomRanged(3,5)` for non-infantry burst inter-shot delay.
- `src/sim/combat/mod.rs` converts `ROF` through wall-clock `rof_to_cooldown_ticks(weapon.rof, tick_ms)`, without the binary `+ Random(0,2)` jitter or `VeteranROF=0.6` ability multiplier.
- `src/sim/combat/mod.rs` applies `weapon.damage` directly except garrison `OccupyDamageMultiplier`; it does not apply `FIREPOWER`/`VeteranCombat=1.1`.
- No self-heal pulse implementation was observed in the combat fire loop; if implemented elsewhere it was not found in this scoped scan.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `[MTNK]` stock YR data | verified | `ini/rulesmd.ini:6603..6648` | none |
| `[105mmE]` stock YR data | verified | `ini/rulesmd.ini:24786..24796` | none |
| `[GRIZAPE]` stock YR data | verified | `ini/rulesmd.ini:26808..26817` | none |
| Elite threshold | verified-via-existing-doc | `VeterancyClass::IsElite @ 0x00750010` in `VETERANCY_SYSTEM_GHIDRA_REPORT.md` | no fresh decompile in this pass |
| Elite weapon swap | verified-via-existing-doc | `TechnoClass::GetWeapon @ 0x0070E140` in `veterancy_weapon_swap.md` and `VETERANCY_SYSTEM_GHIDRA_REPORT.md` | no fresh decompile in this pass |
| Burst scheduler | verified-via-existing-doc | `Fire_At @ 0x006FDD50`, `GetROF @ 0x006FCFA0` in `BURST_WEAPON_FIRING_GHIDRA_REPORT.md` | no fresh decompile in this pass |
| ROF ability multiplier | verified-via-existing-doc | `GetROF @ 0x006FD0F0..0x006FD145` in `VETERANCY_SYSTEM_GHIDRA_REPORT.md` | house/crate/garrison edge branches out of scope |
| FIREPOWER damage multiplier | verified-via-existing-doc | `Fire_At @ 0x006FE35E` in `VETERANCY_SYSTEM_GHIDRA_REPORT.md`; `WeaponType+0xA4` consumer in `WEAPONTYPECLASS_VERIFICATION_AND_CONSUMERS_GHIDRA_REPORT.md` | exact downstream armor/prone/AoE already covered elsewhere |
| SELF_HEAL timing | verified-via-existing-doc | `FUN_0070BE80`, `AI_Update @ 0x006FA756`, `RepairRate @ 0x00670E2A` in `VETERANCY_SYSTEM_GHIDRA_REPORT.md` | visual/animation self-heal feedback out of scope |
| Current Rust weapon selection | verified | Codegraph + `src/sim/combat/combat_weapon.rs` | add MTNK-specific acceptance |
| Current Rust burst/cooldown/damage | verified | `src/sim/combat/mod.rs` scan | implement parity gaps later |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-001 - Does stock Grizzly use a hardcoded elite branch? -> No; generic `GetWeapon(0)` uses elite slot when `IsElite` is true.` (evidence: `0x0070E140`; `ini/rulesmd.ini:6647`)
- `[RESOLVED] OQ-002 - What is the elite threshold? -> `Veterancy >= 2.0f`.` (evidence: `0x00750010`)
- `[RESOLVED] OQ-003 - What does stock YR `[105mmE]` contain? -> `Damage=65`, `ROF=50`, `Warhead=GRIZAPE`, `Burst=2`.` (evidence: `ini/rulesmd.ini:24786..24796`)
- `[RESOLVED] OQ-004 - Does base RA2 `[105mmE]` differ? -> Yes, base `rules.ini` has `Damage=55`, `ROF=75`; YR `rulesmd.ini` overrides to `65`/`50`.` (evidence: `ini/rules.ini:17735..17745`; `ini/rulesmd.ini:24786..24796`)
- `[RESOLVED] OQ-005 - Are the two burst shots same-tick? -> No; one `Fire_At` per shot, with non-infantry mid-burst delay random 3..5 ticks.` (evidence: `0x006FDD50`, `0x006FCFA0`)
- `[RESOLVED] OQ-006 - Does `ROF` ability use the `ROF` token or `FIREPOWER` token? -> Corrected evidence says `HasWeaponAbility(this,4)` = `ROF`, not `FIREPOWER`.` (evidence: `0x006FD0F0` region; ability table)
- `[RESOLVED] OQ-007 - Does `FIREPOWER` also affect damage? -> Yes, index 2 gates damage scaling in `Fire_At`.` (evidence: `0x006FE35E`)
- `[RESOLVED] OQ-008 - Does `SELF_HEAL` affect firing cadence? -> No direct firing effect; it affects HP pulses only.` (evidence: `FUN_0070BE80`; `AI_Update @ 0x006FA756`)
- `[DEFERRED] OQ-009 - Fresh decompile validation in this slot` (category: `requires-different-system-context`; reason: Ghidra MCP tools were not available in this session; next-step-if-pursued: re-run this slot with Ghidra MCP and spot-check `0x006FCFA0`, `0x006FDD50`, `0x0070E140`)
- `[DEFERRED] OQ-010 - Exact stock HouseROFBonus source and default path` (category: `out-of-scope`; reason: not Grizzly-specific and default path behaves as multiplier 1 for ordinary stock firing; next-step-if-pursued: trace `HouseClass+0x1A8` initialization)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Elite MTNK uses `[105mmE]` only at elite threshold; veteran still uses `[105mm]` | `0x0070E140`, `0x00750010`, `ini/rulesmd.ini:6647` | mostly present | `src/sim/combat/combat_weapon.rs`, rules parsing tests | keep threshold and fallback semantics, add stock MTNK acceptance | MTNK with veterancy 199 selects `105mm`; veterancy 200 selects `105mmE` | do not swap at Veteran tier |
| Elite MTNK `Burst=2` is two separate fire calls with 3..5 tick non-infantry inter-shot delay, then full cooldown | `0x006FDD50`, `0x006FCFA0`, `ini/rulesmd.ini:24796` | mismatch: fixed 1 tick style delay and local burst state | `src/sim/combat/mod.rs`, `AttackTarget`/entity burst state | use deterministic sim RNG `3..5` for mid-burst and preserve binary-style burst counter behavior | elite MTNK fires two `GRIZAPE` projectiles separated by 3-5 ticks | do not emit both shots in one tick |
| End-of-burst elite MTNK cooldown uses `105mmE.ROF=50`, random `0..2`, then `VeteranROF=0.6` because elite has `ROF` ability | `0x006FD0B5..0x006FD145`, `ini/rulesmd.ini:20`, `ini/rulesmd.ini:6642`, `ini/rulesmd.ini:24788` | mismatch: no jitter or VeteranROF ability multiplier | `src/sim/combat/mod.rs`, `src/rules/ruleset.rs`, veterancy ability parsing | compute stock cooldown as `ftol((50 + rng0_2) * 0.6)` after final burst shot, before extra non-Grizzly modifiers | elite MTNK full cooldown is 30/30/31 ticks after shot 2 in default stock case | do not treat `ROF` token as a 25% reduction; stock is delay multiplier 0.6 |
| Elite MTNK damage uses `[105mmE] Damage=65` plus `FIREPOWER`/`VeteranCombat=1.1`, then `GRIZAPE` verses | `0x006FE35E`, `WeaponType+0xA4/+0xAC`, `ini/rulesmd.ini:16`, `ini/rulesmd.ini:24787..24792` | mismatch: no FIREPOWER damage multiplier | `src/sim/combat/mod.rs`, damage tests | per elite shot uses damage payload `ftol(65 * 1.1)=71` before verses/prone/AoE | elite MTNK vs infantry `none` armor applies `GRIZAPE` 100% from damage 71 per direct shot before splash/prone | do not hardcode `105mmE` to 65 final damage |
| `SELF_HEAL` does not modify firing but heals +1 HP every `ftol(RepairRate*900)` eligible frames | `FUN_0070BE80`, `AI_Update @ 0x006FA756`, `RepairRate @ 0x00670E2A` | unchecked/missing in scoped fire path | world/health tick surface, not weapon selection | implement separately from combat cooldown | damaged elite MTNK heals +1 around every 14 frames while alive and below max | do not use `SelfHealUnitAmount=5` for per-unit veteran self-heal |

## 9. Negative Facts / Do Not Do

- Do not use base RA2 `[105mmE] Damage=55` or `ROF=75` for YR Grizzly; `rulesmd.ini` overrides to `Damage=65`, `ROF=50`.
- Do not make `Burst=2` simultaneous. It is two `Fire_At` dispatches separated by 3..5 ticks for non-infantry.
- Do not apply the elite `ROF` ability to the first inter-shot delay. It applies in the full end-of-burst cooldown branch.
- Do not confuse `FIREPOWER` and `ROF`: `FIREPOWER` is damage index 2; `ROF` is delay index 4.
- Do not use MTNK.md's old `+25% damage` / `-25% ROF` prose as binary truth. Stock binary-backed rules are `VeteranCombat=1.1` and `VeteranROF=0.6`.
- Do not use `SelfHealUnitAmount=5` as the per-pulse elite Grizzly heal amount; the verified per-unit self-heal path increments raw HP by 1.
- Do not reset burst state just because the target changes unless a separate verified reset write is found.

## 10. Remaining Uncertainty

This report is PARTIAL as a fresh Ghidra investigation because direct Ghidra MCP tools were unavailable in this session. The key behavior is covered by existing high-confidence binary-backed reports, but a future verification pass should cold-read `0x006FCFA0`, `0x006FDD50`, and `0x0070E140`.

The exact source/default proof for `HouseClass+0x1A8` house ROF bonus was not traced here; ordinary stock Grizzly firing assumes the default effective multiplier is 1.0.

## 11. Stale Docs / Replacement Wording

Replace the Grizzly elite cadence wording in `units/allied/MTNK.md` with:

`ElitePrimary=105mmE` is a generic elite weapon-slot swap, active only when `Veterancy >= 2.0f`. In stock YR rulesmd.ini, `[105mmE]` is `Damage=65`, `ROF=50`, `Warhead=GRIZAPE`, and `Burst=2`. The burst is two separate `Fire_At` calls: shot 1 arms a non-infantry random 3..5 tick inter-shot delay, shot 2 arms the full cooldown. Because elite MTNK has the `ROF` ability, the full cooldown applies `[General] VeteranROF=0.6`, so default stock end-of-burst delay is `ftol((50 + Random(0,2)) * 0.6) = 30, 30, or 31` ticks before other non-stock modifiers. Because elite MTNK has `FIREPOWER`, each shot's payload is `ftol(65 * [General] VeteranCombat=1.1) = 71` before `GRIZAPE` verses/prone/AoE handling. `SELF_HEAL` does not affect firing cadence; it heals +1 HP on eligible `RepairRate * 900` pulses.

## Sources

- `C:/Users/enok/Documents/ra2-rust-game-docs/VETERANCY_SYSTEM_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/combat/systems/veterancy_weapon_swap.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BURST_WEAPON_FIRING_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/timing/weapon-rof-burst.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/WEAPONTYPECLASS_VERIFICATION_AND_CONSUMERS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/units/allied/MTNK.md`
- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`
- `C:/Users/enok/Documents/ra2-rust-game/ini/rules.ini`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/combat/combat_weapon.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/combat/mod.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/rules/weapon_type.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/rules/object_type.rs`
