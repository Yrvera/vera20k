# Combat Documentation Index

This directory is the canonical reference for **every** combat-math subsystem,
Warhead, Projectile, and unit-specific hardcoded Weapon behavior in
gamemd.exe / rulesmd.ini. The goal: a reader can reproduce any damage number
from these docs alone — no rulesmd.ini, artmd.ini, or Ghidra session required
at implementation time.

## Layout

- `systems/<subsystem>.md` — math + dispatch layer (damage formula, verses,
  ROF, range, splash, special mechanics)
- `warheads/<WH-ID>.md` — one file per warhead in `[Warheads]`
- `projectiles/<PROJ-ID>.md` — one file per projectile section referenced by
  any weapon's `Projectile=`
- `weapons/<WPN-ID>.md` — **only** weapons with unit-specific hardcoded
  branches; vanilla stat-block weapons are fully covered by their warhead +
  projectile pages

## Citation rules

- Every Ghidra finding **must** carry three confidence axes per
  `feedback_research_confidence_axes`:
  - **Content**: does the decompilation match the claimed logic? (verified by reading it)
  - **Identity**: is this the function we think it is? (verified by string anchors / xrefs)
  - **Binding**: does the runtime actually call this from the claimed path? (verified by `get_function_callers`)
- TS-legacy filter is mandatory: any code path not confirmed reachable in a
  vanilla YR skirmish must be flagged explicitly as such.
- Quote rulesmd.ini values **verbatim**; never paraphrase.
- Cross-reference existing root-level docs in `ra2-rust-game-docs/` rather
  than duplicating their content. The index links the canonical source.

## Status legend

- **TODO** — not yet written
- **IN-PROGRESS** — work started, gaps remain (see entry's gap note)
- **DONE** — fully verified, no open follow-ups
- **DONE-elsewhere → `<doc>`** — covered by existing root-level doc; this
  page can either link to it or migrate/normalize its content into the new
  subfolder structure

---

## Systems

Math layer. **These are load-bearing for every warhead/projectile/weapon
doc** — if these are shallow, everything citing them compounds the gap.

Priority order (work top→bottom):

1. **DONE** [`systems/damage_formula.md`](systems/damage_formula.md) — master damage formula at `0x00489180`. 3-axis confidence applied. Caller binding verified (3 sites: pre-fire estimate, ObjectClass::ReceiveDamage, TechnoClass::ReceiveDamage Psychedelic path). Open follow-up: ScenarioClass flag `0x20` writer not traced (deferred to `combat_damage_globals.md`).
2. **DONE** [`systems/verses_armor_matrix.md`](systems/verses_armor_matrix.md) — 11 armor types verified by reading the pointer table at `0x007e5210` (11 entries: none/flak/plate/light/medium/heavy/wood/steel/concrete/special_1/special_2). Verses parser at tail of `WarheadTypeClass__ReadINI 0x0075DD80` documented branch-by-branch. `IsNonDamaging` derivation (`wh+0x149 = (Verses[medium]==0 && Verses[wood]==0)`) confirmed. Open follow-up: `IsNonDamaging` consumer xrefs (MEDIUM binding on the read side, deferred to `can_target_gates.md`/`target_acquisition.md`).
3. **DONE** [`systems/rof_burst_timing.md`](systems/rof_burst_timing.md) — Burst/ROF cadence + GetROF (`0x006FCFA0`) decompiled and documented branch-by-branch. Live verification of `+0x9C` (Burst), `+0xB0` (ROF), `+0x3B8` (CurrentBurstIndex), sticky-beam gate offsets, InfantryType `+0xE48..0xE54` (BurstDelay0..3 with UNSAFE caveat on 2/3). Gattling scatter-table layout reproduced. 3-axis confidence applied. Open follow-ups: (a) vet/elite ROF multiplier identity in branch 4, (b) `field_0x298` half-ROF flag writer, (c) CurrentBurstIndex reset audit, (d) `+0x314` railgunParticleSys offset (defer to `rail_gun.md`).
4. **DONE** [`systems/range_min_max.md`](systems/range_min_max.md) — Full `TechnoClass::InRange` at `0x006F7220` covered: two-phase check (min<, max<=), three distance flavors (3D / 2D / 2D+arc), Range=-0x200 sentinel, full effective-range chain (base + AirRange + Garrison-REPLACE + Bunker + OpenTopped + height-fire + Foundation), low-flying target Z override, bridge LOS gate. Branch A1 (`WhatAmI()==3`) confirmed dead in YR via vtable+RTTI scan. Projectile-byte interpretations carry MEDIUM content confidence with asm re-verification flagged as follow-up #1. 3 other minor follow-ups (DAT constant values, OccupyWeaponRange section, IsHighFlying override) all LOW priority.
5. **DONE** [`systems/splash_cellspread.md`](systems/splash_cellspread.md) — `Apply_area_damage` at `0x00489280` fully documented across 18 sections covering target collection (airborne + ground), per-target distance (with building / aircraft special cases), C4Warhead self-target gate, ProtectedFromAOE filter, bridge-infantry tolerance, aircraft distance halving, sparky push, IC-barrel recursive chain (Rules+0xFA8 C4Warhead), bridge destruction (low/high overlay ranges), and pre-spawned warhead Particle. 19 callers verified live. Veinhole+wood-armor branch flagged as TS-legacy. 7 open follow-ups (Rules offset INI key tracing, cell-scan tables `DAT_007ed3d0`/`DAT_00abd490/492` value enumeration).
6. **DONE** [`systems/can_target_gates.md`](systems/can_target_gates.md) — `TechnoClass::GetFireError 0x006FC0B0` decompiled and gate-by-gate enumerated (66 gates in 25 phases A..Y). BuildingClass override at `0x00447F10` documented as power/deploy wrapper. FireError code mapping (0=OK/1=AMMO/2=REARM/3=BUSY/5=ILLEGAL/6=CANT/8=MOVING/9=CLOAKED). `warhead.Verses[target.Armor]==0` confirmed as engine-side target-block gate (#59). 8 open follow-ups (full enum, weapon flag identities `+0x142/+0x143/+0x14F/+0x15C`, TechnoType flag identities `+0xD27/+0xD94/+0xD97/+0xE13`, ForceFire-bypass trace).
7. **DONE** [`systems/anti_air_dispatch.md`](systems/anti_air_dispatch.md) — `TechnoClass::SelectWeaponAgainst 0x006F3330` decompiled and decision tree mapped (14 phases A..Z). Return-code semantics established (0=Primary, 1=Secondary, stage×2/stage×2+1 for Gattling, type.field_0xD50 for open-topped passenger weapon override). Verses-driven swap (Phase N) confirmed as the load-bearing branch. Airstrike, Magnetron, NavalGunboat, ElectricAssault, Dogfight, Cell-water-target, and OpenTopped passenger overrides all documented. 8 open follow-ups (`+0x5EC/+0x5ED` building-airstrike flags, naval-flag triplet identity, Verses tie-breaker order).
8. **DONE** [`systems/veterancy_weapon_swap.md`](systems/veterancy_weapon_swap.md) — Three-tier system (Rookie<1.0, Veteran [1.0,2.0), Elite>=2.0) verified via live decomp of `VeterancyClass::IsRookie/IsVeteran/IsElite/Reset`. `TechnoClass::GetWeapon 0x0070E140` decompiled — Elite-tier-only weapon swap via parallel `type+0x898` (regular, stride 0x1C) vs `type+0xA94` (elite, stride 0x1C) slot arrays. `ElitePrimary`/`EliteSecondary` string xrefs to `TechnoTypeClass::ReadINI 0x00712a32/0x00712a5f` confirmed. 10 open follow-ups including Veterancy field offset on TechnoClass instance (HIGH priority) and full ReadGeneral offset trace.
9. **DONE** [`systems/friendly_fire.md`](systems/friendly_fire.md) — `AffectsAllies` flag at `wh+0x179` confirmed via live xref + `TechnoClass::ReceiveDamage 0x00701900` decomp. Single concrete check site identified (the AffectsAllies gate in ReceiveDamage, not in GetFireError). Established: weapon FIRES at friendlies regardless; damage is zeroed at impact. Splash composition (per-target dispatch) means friendly splash radius takes 0 dmg with `AffectsAllies=no`. ForceFire does NOT bypass the damage gate. AI vs player asymmetry is upstream (target selection), not at this gate. Psychedelic separate ally gate (`wh+0x16D`) documented alongside. 5 open follow-ups.
10. **DONE** [`systems/accuracy_inaccurate.md`](systems/accuracy_inaccurate.md) — All three flag offsets verified via live xref (`Inaccurate` `+0x2A2` → BulletTypeClass::ReadINI `0x0046C0EF`; `FlakScatter` `+0x2A3` → `0x0046C105`; `BallisticScatter` Rules+`0x1734` → RulesClass::ReadCombatDamage `0x0066CD53`). Key finding: `Inaccurate` is NOT angle-jitter — it gates the detonation-time target-snap (`< 32 leptons` re-read suppressed) plus the near-miss pre-impact damage. Actual random scatter is the `Inviso AND FlakScatter` combination in BulletClass::Fire, with formula `jitter = (RandomRanged(0, BallisticScatter×2) × dist) / (Owner+0xB4)` at random angle. `Proximity` flag (`+0x29F`) confirmed dead-read. 7 open follow-ups including HIGH-priority `Owner+0xB4` field identity (the scatter divisor).
11. **DONE** [`systems/airburst.md`](systems/airburst.md) — Live decomp of `BulletClass::BulletDetonation 0x00468D80` confirmed Airburst gate at `BulletType+0x294`, dual-path fork (Cluster loop vs single Detonate). Spawn block in `WarheadTypeClass::Detonate` produces **exactly 9** sub-bullets (8 hardcoded loop + 1 explicit) targeting the 3×3 cell footprint around impact. Each sub-bullet gets full `AirburstWeapon.Damage` (no scaling). Launch velocity uses 45°-cone random angle + `Speed/10` magnitude, immediately overridden by sub-bullet homing if `ROT>0`. Sole shipping use: V3 Rocket via `[V3AirburstP]`. Flak Cannon does NOT use Airburst (confirmed; uses `Inaccurate+FlakScatter` instead). 5 open follow-ups (AirburstWeapon validation, ClusterBits default Cluster, 45°-cone intent).
12. **IN-PROGRESS** [`systems/ambient_damage.md`](systems/ambient_damage.md) — Parser side fully verified: `AmbientDamage=` at `weapon+0x98`, parsed first in `WeaponTypeClass::ReadINI 0x007720bb`. Exhaustive retail-INI survey: 5 weapons (LtRail=150, MechRailgun=200, FireballLauncher=2, SonicZap=10, SonicZapE=15). Two usage patterns (pure ambient vs ambient + impact damage). INI comments confirm "use this for the railgun damage field. Leave damage = 0". **Consumer code site NOT TRACED in this iteration** — flagged as HIGH-priority open follow-up #1 (likely lives in RadBeam::AI or WaveClass::AI per-tick damage callback). 7 open follow-ups; spec assumptions (Verses applies, AffectsAllies applies, no CellSpread) are inferred not verified.
13. **DONE** [`systems/chain_reaction.md`](systems/chain_reaction.md) — Live decomp of `OverlayTypeClass::ReadINI 0x005FE7F0` (2026-05-17) revealed the correct OverlayType layout: `+0x2A9=Tiberium`, `+0x2B0=Explodes`, `+0x2B1=ChainReaction` (corrects `splash_cellspread.md` flag-identity swap — fixed in this iteration). Three distinct chain mechanisms documented: (a) `Reduce_Tiberium` gate via warhead `Tiberium=yes` × overlay `ChainReaction=yes` × `Tiberium=`/non-tiberium overlay logic — partially live; (b) `TiberiumExplosionDamage=0` in retail rulesmd.ini disables the TS-era chain shockwave — dormant; (c) `Explodes=yes` IC-barrel mechanism with recursive C4Warhead Apply_area_damage — fully live. Veinhole / Veins system confirmed TS-dead. 7 open follow-ups including `TiberiumExplosionDamage` Rules offset trace and ChainReaction default value.
14. **DONE** [`systems/mind_control.md`](systems/mind_control.md) — Both MC mechanisms documented: (1) CaptureManager-based reversible MC via `wh+0x155 MindControl` flag, allocating `CaptureManagerClass` (0x50 bytes) per controller with MCNode (0x14) per victim; (2) Psychic Dominator permanent MC via `+0x2C4 PermanentlyMindControlled` flag, bypasses CaptureManager entirely. CanCapture live-decompiled (`0x00471C90`) confirming gates: ImmuneToPsionics, warping-infantry, already-MC, IronCurtain/ForceShield, capacity vs override-mode, mission-0x12/0x13. Mastermind overload damage tier system (`Rules.OverloadCount/Damage/Frames` DynVectors) with self-damage via Rules.C4Warhead. Six FreeAll trigger sites enumerated (controller death, transport-enter, chronoshift, etc.). 8 open follow-ups including ImmuneToPsychicDominator (`type+0xD6A`) INI key identity (MEDIUM).
15. **DONE** [`systems/temporal.md`](systems/temporal.md) — `Temporal=` flag at `wh+0x15A` verified live, parsed in `WarheadTypeClass::ReadINI 0x0075D590`. `CanWarpTarget 0x0071AE50` decompiled live confirming Warpable (`type+0xD3A`), IsInvulnerable, infantry-on-Grinder gates. Erase math: WarpHP = Strength × 10, decremented per-tick by SUM of chain-attackers' weapon.Damage. Multi-attacker stacking via doubly-linked chain (head runs Update, SumChainDamage recurses up to 51 deep) with head-detach inheriting remaining WarpHP. Detach = instant snap-back, no recovery curve. Building completion: parachute occupants + SuperClass::Suspend + Undock + standard kill chain. 8 open follow-ups including cascade priority swap question between Temporal/Parasite and Update-time immunity re-check.
16. **DONE** [`systems/ion_cannon.md`](systems/ion_cannon.md) — Confirmed FULLY DORMANT in YR with VERY HIGH confidence: IonBlastClass C++ class has 0 code references (only unreferenced RTTI strings at `0x008280D8`/`0x00828108` remain); `IonCannonSpecial` SW is defined but commented out of `[SuperWeaponTypes]`; `SuperClass::Launch` switch has NO case for IonCannon (would no-op if re-enabled); `IsIonCannon` weapon-flag string doesn't exist in the binary at all. Only LIVE Ion-related Rules field is `Rules+0x298 IonBlast` AnimType, repurposed by Genetic Mutator (SuperClass::Launch Case 9). Documented to **prevent re-implementation of TS dead code**. `IonWH` warhead (separate) IS live via `LightningWarhead=IonWH` for Lightning Storm.
17. **DONE** [`systems/rail_gun.md`](systems/rail_gun.md) — Two distinct flag systems documented: `IsRadBeam=yes` (visual via `TechnoClass::SpawnRadBeam 0x006FD620`, color from `Rules+0x1830/+0x1866/+0x1869`) and `IsRailgun=yes` at `weapon+0x12D` (sticky-beam gates in GetROF Phase 2 + GetFireError Phase D via `Owner+0x314 railgunParticleSys`). Three damage regimes identified: Regime A (Desolator standard projectile damage), Regime B (Chrono Legionnaire Temporal-erase), Regime C (LtRail/MechRailgun `Damage=0 + AmbientDamage=N` — consumer site STILL NOT TRACED, same open follow-up as iteration 13). RadEruption (Desolator deploy) confirmed LIVE in YR via `[DeplDesoWeapon] IsRadEruption=yes` (corrects existing canonical doc's TS-dormant flag). 6 open follow-ups.
18. **DONE** [`systems/radiation.md`](systems/radiation.md) — `RadLevel=` flag at `weapon+0x158` (live xref `0x00849298`). `ImmuneToRadiation=` at `type+0xD37` (live xref `0x00843854`). `Radiation=yes` warhead flag at `wh+0x177`. `RadSiteClass` (0x74 bytes) struct + 10 `[Radiation]` Rules keys fully mapped. RadSite creation/Activate/AI/decay flow documented. Augmentation behavior: AddRadLevel stacks on cell-reuse. Per-cell linear falloff = `((SpreadInLeptons-dist)/SpreadInLeptons) × RadLevel`. Damage application to units flagged as HIGH-priority open follow-up #2 (per-unit dispatch site not yet traced). 8 open follow-ups including spread-value source and RadLevelFactor consumer.
19. **DONE** [`systems/suicide_weapons.md`](systems/suicide_weapons.md) — Two distinct self-destruct mechanisms documented: (a) `Suicide=yes` at `weapon+0x144` (live xref `0x00843050` → `WeaponTypeClass::ReadINI 0x0077228D`) — firer self-targets via Fire_At short-circuit, dies in own explosion; (b) `DeathWeapon=` on TechnoTypeClass (live xref `0x0083B11C` → ReadINI `0x007122F0` + Rules default at `0x0066C58A`) — weapon fired at unit's position on death, optionally scaled by `DeathWeaponDamageModifier` (live xref `0x00844488`). Retail INI survey: 4 Suicide weapons (Demobomb, IvanBomb/CRIvanBomb, CRNuke); ~15+ DeathWeapon assignments (Terrorist, Demo Truck, IFV-Ivan, Kirov/Harrier/BEAGLE w/ 0.1 modifier, NukeCarrier w/ 0.5, barrels). Demo Truck double-explosion question flagged. Terrorist self-trigger mechanism unresolved. 8 open follow-ups including HIGH-priority DeathWeapon dispatch site trace.
20. **DONE** [`systems/emp.md`](systems/emp.md) — **EMP is FUNCTIONALLY DORMANT in YR**. Only retail warhead with `EMEffect=yes` is `[EMPuls]` which is explicitly marked `;gs disabled in code` in rulesmd.ini line 26413. `EMPulseClass` C++ class exists with full Apply/recovery code but has no live caller. `EMPulseWarhead=EMPuls` Rules ref points to the disabled warhead. Only LIVE Ion-style consumer is `EMPulseSparkles` AnimType, repurposed by RadSite visuals (per radiation.md). Tesla weapons clarified as NOT delivering EMP (just damage). `TechnoClass.EMPLockRemaining (+0x504)` field is read by various consumers but never WRITTEN in YR. Existing canonical doc's "case 3 in SuperClass::Launch" claim corrected (case 3 is actually ChronoSphere — open follow-up #4 to fix canonical). Twin to ion_cannon.md as a TS-legacy dormancy doc.
21. **IN-PROGRESS** [`systems/parasite.md`](systems/parasite.md) — `Parasite=yes` warhead flag at `wh+0x159` (live xref `0x0081717C`). Three retail warheads identified: `[Parasite]` (Terror Drone, vehicles only), `[ParasiteDog]` (Attack Dog, infantry only), `[ParasitePlus]` (Squid, all armors). `ParasiteClass` constructor at `0x006292B0` decompiled live; ~0x58-byte struct allocated via `TechnoClass::Init_Managers 0x006F3F40`. Architecturally analogous to CaptureManagerClass. **Per-tick AI loop and damage formula NOT decompiled** — flagged as HIGH-priority open follow-ups #2-4. Cross-class fields on TechnoClass (attacker.Parasite / victim.ParasitedBy) also unverified. 11 open follow-ups total. IN-PROGRESS rather than DONE due to lifecycle gaps.
22. **DONE** [`systems/sonic.md`](systems/sonic.md) — **CRITICAL CORRECTION** to existing canonical addendum: `IsSonic=Yes` (capital Y) IS in retail rulesmd.ini at lines 23688 + 25107 for `[SonicZap]` + `[SonicZapE]`. The addendum's case-sensitive grep missed it; INI parser is case-insensitive. WaveClass type-0 IS LIVE in YR (NOT TS-legacy dead). Confirmed via `IsSonic=` at `weapon+0x130`. Sonic Tank fires THREE parallel mechanisms per shot: (1) WaveClass type-0 visual (no damage), (2) standard BulletClass with `Projectile=Sonic` + `SonicWarhead` (Damage=4/8), (3) AmbientDamage along path (10/15, consumer still untraced). Magnetron's IsMagBeam=yes (`+0x15C`) cross-referenced for type-3 wave. 8 open follow-ups including HIGH-priority correction to existing canonical addendum + the recurring AmbientDamage consumer trace.
23. **TODO** [`systems/locomotor_warhead.md`](systems/locomotor_warhead.md) — Magnetron `LocomotorBeam` warhead, lifting, drop damage, target eligibility. **Existing canonical source:** [`../MAGNETRON_SYSTEM_GHIDRA_REPORT.md`](../MAGNETRON_SYSTEM_GHIDRA_REPORT.md).
24. **TODO** [`systems/animlist_warhead_anim.md`](systems/animlist_warhead_anim.md) — `AnimList=`, `Bright=`, `InfDeath=`, anim-on-damage selection by damage-vs-strength ratio, building-vs-unit selection. **Existing canonical sources:** [`../DAMAGE_FIRE_ANIMS_GHIDRA.md`](../DAMAGE_FIRE_ANIMS_GHIDRA.md), [`../ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md`](../ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md).
25. **TODO** [`systems/death_weapon.md`](systems/death_weapon.md) — `DeathWeapon=`, `DeathWeaponDamageModifier=`, on-death detonation, no-damage default `DeathWH`.
26. **TODO** [`systems/percentatmax_falloff.md`](systems/percentatmax_falloff.md) — full math derivation of `PercentAtMax` lerp at the edge of CellSpread, integer truncation rules, zero-cellspread case (covered partially by `damage_formula.md`; this is the detailed reference).
27. **TODO** [`systems/maxdamage_clamp.md`](systems/maxdamage_clamp.md) — `MaxDamage=10000` global clamp, when it triggers, interaction with negative damage.
28. **TODO** [`systems/warhead_detonate_dispatch.md`](systems/warhead_detonate_dispatch.md) — top-level `WarheadType__Detonate` dispatcher, every branch ordered (Particle/AnimList/Wall/Wood/Tiberium/Sparky/Bright/Temporal/MindControl/Parasite/Radiation/EMPulse/Conventional/IonCannon/RailGun). **Existing canonical source:** [`../WARHEAD_DETONATE_GHIDRA_REPORT.md`](../WARHEAD_DETONATE_GHIDRA_REPORT.md).
29. **TODO** [`systems/receive_damage_pipeline.md`](systems/receive_damage_pipeline.md) — target-side `ReceiveDamage` pipeline, armor lookup, kill detection, retaliation trigger, score/credit attribution. **Existing canonical sources:** [`../RECEIVE_DAMAGE_GHIDRA_REPORT.md`](../RECEIVE_DAMAGE_GHIDRA_REPORT.md), [`../RECEIVE_DAMAGE_PIPELINE_VERIFICATION_REPORT.md`](../RECEIVE_DAMAGE_PIPELINE_VERIFICATION_REPORT.md).
30. **TODO** [`systems/projectile_arc_gravity.md`](systems/projectile_arc_gravity.md) — `Arcing=`, `Vertical=`, `Lobber=`, gravity constant, ballistic trajectory math, `Inviso=`.
31. **TODO** [`systems/projectile_rot_homing.md`](systems/projectile_rot_homing.md) — `ROT=`, homing turn rate, target-lost behavior, dud handling.
32. **TODO** [`systems/projectile_special_props.md`](systems/projectile_special_props.md) — `Inviso=yes` (instant-hit), `AA`/`AG`, `Image=`/`Trailer=`, `Cluster=`, `ShrapnelCount=`/`ShrapnelWeapon=`, `Bouncy=`, `Splits=`, `IsLaser=` interactions.
33. **TODO** [`systems/bullet_lifecycle.md`](systems/bullet_lifecycle.md) — `BulletClass` init/fire/move/explode lifecycle. **Existing canonical sources:** [`../BULLETCLASS_INIT_AND_FIRE_GHIDRA_REPORT.md`](../BULLETCLASS_INIT_AND_FIRE_GHIDRA_REPORT.md), [`../BULLETCLASS_LIFECYCLE_AND_TIER1_VERIFICATIONS_GHIDRA_REPORT.md`](../BULLETCLASS_LIFECYCLE_AND_TIER1_VERIFICATIONS_GHIDRA_REPORT.md), [`../BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md`](../BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md).
34. **TODO** [`systems/gattling_spool.md`](systems/gattling_spool.md) — Gattling weapon stage progression, damage ramp, cooldown. **Existing canonical source:** [`../GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md`](../GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md).
35. **TODO** [`systems/prism_cascade.md`](systems/prism_cascade.md) — Prism Tower chain firing, support tower stacking, damage forwarding. **Existing canonical sources:** [`../PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md`](../PRISM_CASCADE_TRIGGER_GHIDRA_REPORT.md), [`../PRISM_CASCADE_EXTENSION_GHIDRA_REPORT.md`](../PRISM_CASCADE_EXTENSION_GHIDRA_REPORT.md), [`../PRISM_FORWARDING_GHIDRA_REPORT.md`](../PRISM_FORWARDING_GHIDRA_REPORT.md).
36. **TODO** [`systems/opportunity_fire.md`](systems/opportunity_fire.md) — opportunity-fire trigger conditions, scan radius, target priority. **Existing canonical source:** [`../OPPORTUNITY_FIRE_GHIDRA_REPORT.md`](../OPPORTUNITY_FIRE_GHIDRA_REPORT.md).
37. **TODO** [`systems/target_acquisition.md`](systems/target_acquisition.md) — auto-target scan, threat scoring, target switching. **Existing canonical source:** [`../TARGET_ACQUISITION_GHIDRA_REPORT.md`](../TARGET_ACQUISITION_GHIDRA_REPORT.md).
38. **TODO** [`systems/turret_tracking.md`](systems/turret_tracking.md) — turret rotation rate, fire timing, facing match. **Existing canonical source:** [`../UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md`](../UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md).
39. **TODO** [`systems/country_multipliers.md`](systems/country_multipliers.md) — country-specific cost/firepower/armor/speed multipliers. **Existing canonical source:** [`../COUNTRY_MULTIPLIERS_APPLICATION.md`](../COUNTRY_MULTIPLIERS_APPLICATION.md).
40. **TODO** [`systems/wall_destruction.md`](systems/wall_destruction.md) — `Wall=yes` warhead, wall HP, connection cleanup. **Existing canonical source:** [`../WALL_CONNECTION_AND_DESTRUCTION_GHIDRA_REPORT.md`](../WALL_CONNECTION_AND_DESTRUCTION_GHIDRA_REPORT.md).
41. **TODO** [`systems/building_damage_states.md`](systems/building_damage_states.md) — damaged/critical/destroyed transition thresholds, smoke anim hookup, repair eligibility. **Existing canonical sources:** [`../BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md`](../BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md), [`../HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`](../HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md).
42. **TODO** [`systems/fire_at_pipeline.md`](systems/fire_at_pipeline.md) — full `FireAt` pipeline from TechnoClass + weapon → projectile spawn. **Existing canonical sources:** [`../FIRE_AT_ANALYSIS.md`](../FIRE_AT_ANALYSIS.md), [`../FIRE_AT_PIPELINE_GHIDRA_REPORT.md`](../FIRE_AT_PIPELINE_GHIDRA_REPORT.md).
43. **TODO** [`systems/warhead_struct_layout.md`](systems/warhead_struct_layout.md) — full `WarheadTypeClass` struct field map, every offset, every read site. **Existing canonical sources:** [`../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md`](../WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md), [`../WARHEADTYPECLASS_REINVESTIGATION_GHIDRA_REPORT.md`](../WARHEADTYPECLASS_REINVESTIGATION_GHIDRA_REPORT.md).
44. **TODO** [`systems/weapon_struct_layout.md`](systems/weapon_struct_layout.md) — full `WeaponTypeClass` struct field map, every offset, every read site. **Existing canonical sources:** [`../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`](../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md), [`../WEAPONTYPECLASS_VERIFICATION_AND_CONSUMERS_GHIDRA_REPORT.md`](../WEAPONTYPECLASS_VERIFICATION_AND_CONSUMERS_GHIDRA_REPORT.md).
45. **TODO** [`systems/projectile_struct_layout.md`](systems/projectile_struct_layout.md) — full `BulletTypeClass` (projectile) struct field map.
46. **TODO** [`systems/combat_damage_globals.md`](systems/combat_damage_globals.md) — every `[CombatDamage]` global (`AtomDamage`, `IonStormDuration`, `C4Delay`, `BallisticScatter`, etc.) with consumers traced.
47. **TODO** [`systems/superweapon_dispatch.md`](systems/superweapon_dispatch.md) — superweapon-launched warhead/weapon delivery (Nuke, IronCurtain damage interaction, Lightning, PsychicDominator, GeneticMutator). Cross-ref to existing per-SW docs.

---

## Warheads

105 warheads in `[Warheads]` (verified at rulesmd.ini:2876–2981).
**TODO** for all entries unless noted. Each doc:
- Quotes the `[WH-ID]` section verbatim
- Documents every key with confidence and behavioral effect
- Lists every weapon that references the warhead
- Captures any hardcoded branch keyed by the warhead ID string in gamemd.exe

1. **TODO** [`warheads/EMPuls.md`](warheads/EMPuls.md)
2. **TODO** [`warheads/SonicWarhead.md`](warheads/SonicWarhead.md)
3. **TODO** [`warheads/TankOGas.md`](warheads/TankOGas.md)
4. **TODO** [`warheads/SA.md`](warheads/SA.md)
5. **TODO** [`warheads/HE.md`](warheads/HE.md)
6. **TODO** [`warheads/AP.md`](warheads/AP.md)
7. **TODO** [`warheads/Gas.md`](warheads/Gas.md)
8. **TODO** [`warheads/Fire.md`](warheads/Fire.md)
9. **TODO** [`warheads/HollowPoint.md`](warheads/HollowPoint.md)
10. **TODO** [`warheads/Super.md`](warheads/Super.md)
11. **TODO** [`warheads/Organic.md`](warheads/Organic.md)
12. **TODO** [`warheads/Slimer.md`](warheads/Slimer.md)
13. **TODO** [`warheads/FirestormWH.md`](warheads/FirestormWH.md) — TS-legacy candidate
14. **TODO** [`warheads/IonCannonWH.md`](warheads/IonCannonWH.md) — TS-legacy candidate
15. **TODO** [`warheads/RailShot.md`](warheads/RailShot.md)
16. **TODO** [`warheads/Mechanical.md`](warheads/Mechanical.md)
17. **TODO** [`warheads/VeinholeWH.md`](warheads/VeinholeWH.md) — TS-legacy candidate
18. **TODO** [`warheads/IonWH.md`](warheads/IonWH.md) — TS-legacy candidate
19. **TODO** [`warheads/ARTYHE.md`](warheads/ARTYHE.md)
20. **TODO** [`warheads/PlasmaWH.md`](warheads/PlasmaWH.md)
21. **TODO** [`warheads/SAMWH.md`](warheads/SAMWH.md)
22. **TODO** [`warheads/ORCAAP.md`](warheads/ORCAAP.md) — TS-legacy candidate
23. **TODO** [`warheads/RailShot2.md`](warheads/RailShot2.md)
24. **TODO** [`warheads/ORCAHE.md`](warheads/ORCAHE.md) — TS-legacy candidate
25. **TODO** [`warheads/Controller.md`](warheads/Controller.md) — MindControl
26. **TODO** [`warheads/PsiPulse.md`](warheads/PsiPulse.md)
27. **TODO** [`warheads/IvanBomb.md`](warheads/IvanBomb.md)
28. **TODO** [`warheads/IvanWH.md`](warheads/IvanWH.md)
29. **TODO** [`warheads/Electric.md`](warheads/Electric.md) — Tesla
30. **TODO** [`warheads/ElectricAssault.md`](warheads/ElectricAssault.md) — Shock Trooper
31. **TODO** [`warheads/V3HE.md`](warheads/V3HE.md)
32. **TODO** [`warheads/Parasite.md`](warheads/Parasite.md) — Terror Drone
33. **TODO** [`warheads/NUKE.md`](warheads/NUKE.md)
34. **TODO** [`warheads/BlimpHE.md`](warheads/BlimpHE.md) — Kirov
35. **TODO** [`warheads/ParasitePlus.md`](warheads/ParasitePlus.md)
36. **TODO** [`warheads/DeathWH.md`](warheads/DeathWH.md)
37. **TODO** [`warheads/ParasiteDog.md`](warheads/ParasiteDog.md)
38. **TODO** [`warheads/BombDisarm.md`](warheads/BombDisarm.md)
39. **TODO** [`warheads/Snapshot.md`](warheads/Snapshot.md) — Mirage disguise capture
40. **TODO** [`warheads/RadBeamWarhead.md`](warheads/RadBeamWarhead.md) — Desolator
41. **TODO** [`warheads/RadEruptionWarhead.md`](warheads/RadEruptionWarhead.md) — Desolator deploy
42. **TODO** [`warheads/RadSite.md`](warheads/RadSite.md) — radiation tick
43. **TODO** [`warheads/GrandCannonWH.md`](warheads/GrandCannonWH.md)
44. **TODO** [`warheads/UltraAP.md`](warheads/UltraAP.md)
45. **TODO** [`warheads/HowitzerWH.md`](warheads/HowitzerWH.md)
46. **TODO** [`warheads/CometWH.md`](warheads/CometWH.md) — Lightning Storm
47. **TODO** [`warheads/MaverickHE.md`](warheads/MaverickHE.md)
48. **TODO** [`warheads/FlakWH.md`](warheads/FlakWH.md)
49. **TODO** [`warheads/OilExplosionWH.md`](warheads/OilExplosionWH.md)
50. **TODO** [`warheads/TankSnapshot.md`](warheads/TankSnapshot.md)
51. **TODO** [`warheads/CRNUKEWH.md`](warheads/CRNUKEWH.md)
52. **TODO** [`warheads/ChronoBeam.md`](warheads/ChronoBeam.md) — Chrono Legionnaire
53. **TODO** [`warheads/HollowPoint2.md`](warheads/HollowPoint2.md)
54. **TODO** [`warheads/NukeMaker.md`](warheads/NukeMaker.md)
55. **TODO** [`warheads/FakeC4WH.md`](warheads/FakeC4WH.md) — Chrono Commando
56. **TODO** [`warheads/HollowPointNoBuilding.md`](warheads/HollowPointNoBuilding.md)
57. **TODO** [`warheads/V3WH.md`](warheads/V3WH.md)
58. **TODO** [`warheads/FlakTWH.md`](warheads/FlakTWH.md)
59. **TODO** [`warheads/APSplash.md`](warheads/APSplash.md)
60. **TODO** [`warheads/DMISLWH.md`](warheads/DMISLWH.md) — Dreadnought
61. **TODO** [`warheads/DemobombWH.md`](warheads/DemobombWH.md) — Demo Truck
62. **TODO** [`warheads/MirageWH.md`](warheads/MirageWH.md) — Mirage Tank
63. **TODO** [`warheads/SSA.md`](warheads/SSA.md)
64. **TODO** [`warheads/SSAB.md`](warheads/SSAB.md)
65. **TODO** [`warheads/ApocAP.md`](warheads/ApocAP.md)
66. **TODO** [`warheads/HARVWH.md`](warheads/HARVWH.md)
67. **TODO** [`warheads/V3EWH.md`](warheads/V3EWH.md)
68. **TODO** [`warheads/DMISLEWH.md`](warheads/DMISLEWH.md)
69. **TODO** [`warheads/TerrorBombWH.md`](warheads/TerrorBombWH.md) — Terrorist
70. **TODO** [`warheads/FlakGuyWH.md`](warheads/FlakGuyWH.md)
71. **TODO** [`warheads/CRTerrorBombWH.md`](warheads/CRTerrorBombWH.md) — IFV Terrorist
72. **TODO** [`warheads/HollowPoint3.md`](warheads/HollowPoint3.md)
73. **TODO** [`warheads/ControllerBuilding.md`](warheads/ControllerBuilding.md) — Yuri Prime
74. **TODO** [`warheads/SuperPsiPulse.md`](warheads/SuperPsiPulse.md)
75. **TODO** [`warheads/DominatorWH.md`](warheads/DominatorWH.md) — Psychic Dominator
76. **TODO** [`warheads/AirstrikeFlare.md`](warheads/AirstrikeFlare.md)
77. **TODO** [`warheads/VirusGas.md`](warheads/VirusGas.md) — Virus init
78. **TODO** [`warheads/Virus.md`](warheads/Virus.md) — Virus tick
79. **TODO** [`warheads/PsychGasCreate.md`](warheads/PsychGasCreate.md)
80. **TODO** [`warheads/PsychGas.md`](warheads/PsychGas.md)
81. **TODO** [`warheads/Battering.md`](warheads/Battering.md)
82. **TODO** [`warheads/GattWH.md`](warheads/GattWH.md) — Gattling
83. **TODO** [`warheads/Mutate.md`](warheads/Mutate.md) — Genetic Mutator
84. **TODO** [`warheads/CMISLWH.md`](warheads/CMISLWH.md) — Cruise Missile
85. **TODO** [`warheads/CMISLEWH.md`](warheads/CMISLEWH.md)
86. **TODO** [`warheads/Smashing.md`](warheads/Smashing.md) — Brute
87. **TODO** [`warheads/LocomotorBeam.md`](warheads/LocomotorBeam.md) — Magnetron
88. **TODO** [`warheads/MIGWH.md`](warheads/MIGWH.md) — Boris MIG
89. **TODO** [`warheads/LUNARWH.md`](warheads/LUNARWH.md)
90. **TODO** [`warheads/GUARDWH.md`](warheads/GUARDWH.md) — Guardian GI
91. **TODO** [`warheads/AntiB.md`](warheads/AntiB.md)
92. **TODO** [`warheads/AntiPerson.md`](warheads/AntiPerson.md) — Floating Disc
93. **TODO** [`warheads/PJABWH.md`](warheads/PJABWH.md)
94. **TODO** [`warheads/APSplash2.md`](warheads/APSplash2.md)
95. **TODO** [`warheads/SAFlame.md`](warheads/SAFlame.md)
96. **TODO** [`warheads/SSABFlame.md`](warheads/SSABFlame.md)
97. **TODO** [`warheads/DiskWH.md`](warheads/DiskWH.md)
98. **TODO** [`warheads/NukeB.md`](warheads/NukeB.md)
99. **TODO** [`warheads/BORISWH.md`](warheads/BORISWH.md)
100. **TODO** [`warheads/SCHOPWH.md`](warheads/SCHOPWH.md) — Siege Chopper
101. **TODO** [`warheads/TRexWH.md`](warheads/TRexWH.md)
102. **TODO** [`warheads/MutateExplosion.md`](warheads/MutateExplosion.md)
103. **TODO** [`warheads/TRexInfWH.md`](warheads/TRexInfWH.md)
104. **TODO** [`warheads/Crush.md`](warheads/Crush.md)
105. **TODO** [`warheads/BlimpHEEffect.md`](warheads/BlimpHEEffect.md)

---

## Projectiles

55 unique projectile section names referenced by `Projectile=` across all weapons in rulesmd.ini.
Note: `Invisiblelow` is a case-typo variant of `InvisibleLow` (both used in source; document under canonical `InvisibleLow.md` with note).

1. **TODO** [`projectiles/Invisible.md`](projectiles/Invisible.md)
2. **TODO** [`projectiles/InvisibleLow.md`](projectiles/InvisibleLow.md) (also covers `Invisiblelow` typo)
3. **TODO** [`projectiles/InvisibleHigh.md`](projectiles/InvisibleHigh.md)
4. **TODO** [`projectiles/InvisibleAll.md`](projectiles/InvisibleAll.md)
5. **TODO** [`projectiles/InvisibleMedium.md`](projectiles/InvisibleMedium.md)
6. **TODO** [`projectiles/InvisibleVertical.md`](projectiles/InvisibleVertical.md)
7. **TODO** [`projectiles/Invisible2.md`](projectiles/Invisible2.md)
8. **TODO** [`projectiles/Invisible3.md`](projectiles/Invisible3.md)
9. **TODO** [`projectiles/Invisible4.md`](projectiles/Invisible4.md)
10. **TODO** [`projectiles/Cannon.md`](projectiles/Cannon.md)
11. **TODO** [`projectiles/Cannon2.md`](projectiles/Cannon2.md)
12. **TODO** [`projectiles/Ballistic.md`](projectiles/Ballistic.md)
13. **TODO** [`projectiles/HeatSeeker.md`](projectiles/HeatSeeker.md)
14. **TODO** [`projectiles/AAHeatSeeker.md`](projectiles/AAHeatSeeker.md)
15. **TODO** [`projectiles/AAHeatSeeker2.md`](projectiles/AAHeatSeeker2.md)
16. **TODO** [`projectiles/AirToGroundMissile.md`](projectiles/AirToGroundMissile.md)
17. **TODO** [`projectiles/NavalToGroundSeeker.md`](projectiles/NavalToGroundSeeker.md)
18. **TODO** [`projectiles/Torpedo.md`](projectiles/Torpedo.md)
19. **TODO** [`projectiles/ProtonTorpedo.md`](projectiles/ProtonTorpedo.md)
20. **TODO** [`projectiles/DepthCharge.md`](projectiles/DepthCharge.md)
21. **TODO** [`projectiles/ASWVirt.md`](projectiles/ASWVirt.md)
22. **TODO** [`projectiles/NormalBomb.md`](projectiles/NormalBomb.md)
23. **TODO** [`projectiles/BlimpBombP.md`](projectiles/BlimpBombP.md)
24. **TODO** [`projectiles/V3AirburstP.md`](projectiles/V3AirburstP.md)
25. **TODO** [`projectiles/DredMissile.md`](projectiles/DredMissile.md)
26. **TODO** [`projectiles/ChemMissile.md`](projectiles/ChemMissile.md)
27. **TODO** [`projectiles/Lobbed.md`](projectiles/Lobbed.md)
28. **TODO** [`projectiles/Lobbed2.md`](projectiles/Lobbed2.md)
29. **TODO** [`projectiles/QuadShell.md`](projectiles/QuadShell.md)
30. **TODO** [`projectiles/GrandCannonBall.md`](projectiles/GrandCannonBall.md)
31. **TODO** [`projectiles/ProtonBlast.md`](projectiles/ProtonBlast.md)
32. **TODO** [`projectiles/MedusaProjectile.md`](projectiles/MedusaProjectile.md)
33. **TODO** [`projectiles/FlakProj.md`](projectiles/FlakProj.md)
34. **TODO** [`projectiles/FlakTProj.md`](projectiles/FlakTProj.md)
35. **TODO** [`projectiles/PulsPr.md`](projectiles/PulsPr.md) — Tesla pulse
36. **TODO** [`projectiles/Electricbounce.md`](projectiles/Electricbounce.md)
37. **TODO** [`projectiles/Sonic.md`](projectiles/Sonic.md)
38. **TODO** [`projectiles/Psychic.md`](projectiles/Psychic.md)
39. **TODO** [`projectiles/PsychicControl.md`](projectiles/PsychicControl.md)
40. **TODO** [`projectiles/LLine.md`](projectiles/LLine.md)
41. **TODO** [`projectiles/LLine2.md`](projectiles/LLine2.md)
42. **TODO** [`projectiles/LargeCometP.md`](projectiles/LargeCometP.md) — Lightning Storm
43. **TODO** [`projectiles/SmallCometP.md`](projectiles/SmallCometP.md)
44. **TODO** [`projectiles/SuperCometP.md`](projectiles/SuperCometP.md)
45. **TODO** [`projectiles/SuperSmallCometP.md`](projectiles/SuperSmallCometP.md)
46. **TODO** [`projectiles/SmallTeslaP.md`](projectiles/SmallTeslaP.md)
47. **TODO** [`projectiles/ClusterBits.md`](projectiles/ClusterBits.md)
48. **TODO** [`projectiles/GiantNukeUp.md`](projectiles/GiantNukeUp.md)
49. **TODO** [`projectiles/GiantNukeDown.md`](projectiles/GiantNukeDown.md)
50. **TODO** [`projectiles/DogShard.md`](projectiles/DogShard.md)
51. **TODO** [`projectiles/JUMP.md`](projectiles/JUMP.md) — Yuri infantry jump-grab
52. **TODO** [`projectiles/DOGJUMP.md`](projectiles/DOGJUMP.md) — Attack Dog
53. **TODO** [`projectiles/ADOGJUMP.md`](projectiles/ADOGJUMP.md) — Allied Dog
54. **TODO** [`projectiles/SQDJUMP.md`](projectiles/SQDJUMP.md) — Squid grab

---

## Weapons-with-hardcoded-behavior

Only weapons with **unit-specific hardcoded branches** in gamemd.exe get a
dedicated doc here. Vanilla stat-block weapons are covered by their warhead
+ projectile pages. Each doc:
- Quotes the `[WPN-ID]` section verbatim
- Documents the unit-specific code branches (function addresses, conditions, special timing)
- Cross-references to the warhead and projectile docs (does NOT duplicate them)

Seed list (expand as Ghidra investigation reveals more):

1. **TODO** [`weapons/TanyaPistol.md`](weapons/TanyaPistol.md) — Tanya pistol, 1-shot infantry kill, no FF on allied infantry, animation lock.
2. **TODO** [`weapons/TanyaC4.md`](weapons/TanyaC4.md) — Tanya C4 secondary, building-only, delay timer, on-hit kill.
3. **TODO** [`weapons/YuriPsiBeam.md`](weapons/YuriPsiBeam.md) — Yuri mind-control beam, link establishment, one-controlled-at-a-time.
4. **TODO** [`weapons/YuriPrimeBeam.md`](weapons/YuriPrimeBeam.md) — Yuri Prime variant, building-capture enabled, longer range.
5. **TODO** [`weapons/ChronoLegionFreeze.md`](weapons/ChronoLegionFreeze.md) — Chrono Legionnaire temporal weapon.
6. **TODO** [`weapons/DesolatorDeploy.md`](weapons/DesolatorDeploy.md) — Desolator deploy radiation eruption.
7. **TODO** [`weapons/DesolatorBeam.md`](weapons/DesolatorBeam.md) — Desolator un-deployed rad-beam.
8. **TODO** [`weapons/MirageWeapon.md`](weapons/MirageWeapon.md) — Mirage Tank disguise + heat-ray combination.
9. **TODO** [`weapons/IFVWeaponTable.md`](weapons/IFVWeaponTable.md) — IFV weapon swap on passenger type.
10. **TODO** [`weapons/BattleFortressGarrison.md`](weapons/BattleFortressGarrison.md) — Battle Fortress per-passenger weapon firing.
11. **TODO** [`weapons/BoomerDualLaunch.md`](weapons/BoomerDualLaunch.md) — Boomer dual missile launch.
12. **TODO** [`weapons/DemoTruckSuicide.md`](weapons/DemoTruckSuicide.md) — Demo Truck explode on attack-move arrive.
13. **TODO** [`weapons/TerroristSuicide.md`](weapons/TerroristSuicide.md) — Terrorist suicide, IFV-mounted variant.
14. **TODO** [`weapons/CrazyIvanBombs.md`](weapons/CrazyIvanBombs.md) — Crazy Ivan timed bomb placement.
15. **TODO** [`weapons/ChronoCommandoFakeC4.md`](weapons/ChronoCommandoFakeC4.md) — fake-C4 chrono trigger.
16. **TODO** [`weapons/PrismTowerCascade.md`](weapons/PrismTowerCascade.md) — Prism Tower main + cascade math.
17. **TODO** [`weapons/TeslaZap.md`](weapons/TeslaZap.md) — Tesla Coil Power-on/off, Tesla Trooper chargeable bonus.
18. **TODO** [`weapons/MagnetronLift.md`](weapons/MagnetronLift.md) — Magnetron lift + drop damage chain.
19. **TODO** [`weapons/GattlingSpool.md`](weapons/GattlingSpool.md) — Gattling Cannon / Gattling Tank stage progression.
20. **TODO** [`weapons/InitiateBurst.md`](weapons/InitiateBurst.md) — Initiate fire burst behaviors.
21. **TODO** [`weapons/YuriCloneBurst.md`](weapons/YuriCloneBurst.md) — Yuri Clone behavior.
22. **TODO** [`weapons/ApocalypseDual.md`](weapons/ApocalypseDual.md) — Apocalypse cannon + missile pair firing.
23. **TODO** [`weapons/PatriotMissile.md`](weapons/PatriotMissile.md) — Patriot Missile AA hardcoded behavior.
24. **TODO** [`weapons/AegisCruiser.md`](weapons/AegisCruiser.md) — Aegis AA logic.
25. **TODO** [`weapons/V3Launch.md`](weapons/V3Launch.md) — V3 ground-to-ground launch behavior.
26. **TODO** [`weapons/DreadnoughtLaunch.md`](weapons/DreadnoughtLaunch.md) — Dreadnought missile launch.
27. **TODO** [`weapons/KirovBomb.md`](weapons/KirovBomb.md) — Kirov bomb-drop logic.
28. **TODO** [`weapons/FloatingDiscDrain.md`](weapons/FloatingDiscDrain.md) — Floating Disc power-drain weapon.
29. **TODO** [`weapons/PsiCorpsBlast.md`](weapons/PsiCorpsBlast.md) — Psi-Corps Trooper area blast.
30. **TODO** [`weapons/HarrierPenaltyAirstrike.md`](weapons/HarrierPenaltyAirstrike.md) — Harrier / Black Eagle attack-airfield-return.
31. **TODO** [`weapons/SiegeChopperDeploy.md`](weapons/SiegeChopperDeploy.md) — Siege Chopper deploy artillery mode.

---

## Iteration log

| Date | Iteration | Action |
|---|---|---|
| 2026-05-17 | 1 | INDEX_COMBAT.md created. Enumerated 105 warheads, 55 projectiles, 47 systems, 31 hardcoded weapons. Picked next: `systems/damage_formula.md`. |
| 2026-05-17 | 2 | **DONE** `systems/damage_formula.md`. Migrated master `FUN_00489180` formula from `DAMAGE_MATH_GHIDRA_REPORT.md` with 3-axis confidence (Content/Identity/Binding all HIGH). Caller list re-verified live (`get_function_callers 0x00489180`: 3 sites). Cross-references stubbed for the 6 sibling system docs that will own the other pipeline stages. One deferred item: ScenarioClass `0x20` global writer not traced — will be picked up by `combat_damage_globals.md`. Next: `systems/verses_armor_matrix.md`. |
| 2026-05-17 | 3 | **DONE** `systems/verses_armor_matrix.md`. Live verification of all 11 armor names by reading the lookup-table pointers at `0x007e5210` (each pointer dereferenced to ASCII string). Verses parser inside `WarheadTypeClass__ReadINI` (`0x0075DD80`) decompiled and documented branch-by-branch (`%` vs decimal dispatch). `IsNonDamaging` derivation traced (`wh+0x149` set by `Verses[medium]==0 && Verses[wood]==0` at tail of ReadINI). `Armor=` write site (`+0x9C`) verified via `ObjectTypeClass__ReadINI 0x005f9490`. Open follow-up: `wh+0x149` consumer xrefs (deferred). Next: `systems/rof_burst_timing.md` (existing source `BURST_WEAPON_FIRING_GHIDRA_REPORT.md` to verify and migrate). |
| 2026-05-17 | 4 | **DONE** `systems/rof_burst_timing.md`. Full migration of Burst/ROF/BurstDelay mechanics with live GetROF decomp at `0x006FCFA0` verifying every offset: `param_1[0xee]=+0x3B8` (CurrentBurstIndex), weapon `+0x9C` (Burst), `+0xB0` (ROF), `+0x130/+0x12A/+0x129/+0x12D` (sonic/fire/spark/railgun sticky-beam gates). InfantryType `BurstDelay0/1` safe / `BurstDelay2/3` UNSAFE caveat preserved (DVC overlap at `+0xE50`). Gattling 8-octant scatter table reproduced. Four open follow-ups carried forward. Next: `systems/range_min_max.md`. |
| 2026-05-17 | 5 | **DONE** `systems/range_min_max.md`. Migrated `TechnoClass::InRange 0x006F7220` with live re-decomp confirming every offset (`weapon+0xB4` Range, `+0xB8` MinimumRange, `+0xA0` Projectile, `+0x29B` Arcing, `+0x297` SubjectToElevation, `+0x295` Floater, attacker `+0x2E4` Bunker / `+0x82` OpenTopped, Rules `+0xF48/+0xF54/+0xF5C/+0x16B8/+0x1838`). 2026-05-07 corrections preserved: Branch A1 (WhatAmI==3) confirmed dead via vtable+RTTI; height-fire (not "RadioLink") gated by `Projectile.SubjectToElevation`; bridge gate is LOS occlusion. One MEDIUM-priority follow-up (Projectile byte-flag asm re-verification due to Ghidra decompiler register-tracking ambiguity). Next: `systems/splash_cellspread.md`. |
| 2026-05-17 | 6 | **DONE** `systems/splash_cellspread.md`. Live decomp of `Apply_area_damage 0x00489280` + caller list (19 sites) traced. Documented all 18 sections: max-radius, bridge-tolerance precheck, airborne target collection (if impact above ground), ground cell scan via `DAT_007ed3d0`/`DAT_00abd490/492` tables, per-target distance (building special cases incl. cell-vs-impact and high-impact `2×LevelHeight` subtraction), C4Warhead self-target gate, ProtectedFromAOE (scenario flag 0x800), aircraft distance halving, damage-vector dispatch, sparky push, IC-barrel recursive Apply_area_damage chain (Rules+0xFA8), bridge destruction (low/high overlay ranges + retry loop), pre-spawned warhead Particle. Veinhole+wood-armor branch flagged TS-legacy. 7 open follow-ups (Rules offsets `+0xB40/+0xB4C/+0xFA8/+0xFAC/+0xFF0/+0x1740` INI-key tracing; cell-scan table values). Next: `systems/can_target_gates.md`. |
| 2026-05-17 | 7 | **DONE** `systems/can_target_gates.md`. Live decomp of `TechnoClass::GetFireError 0x006FC0B0` (66 gates across 25 phases A..Y) + `BuildingClass::GetFireError 0x00447F10` wrapper + `CanFireAt 0x006F77B0` range-only wrapper. FireError codes 0/1/2/3/5/6/8/9 mapped to OK/AMMO/REARM/BUSY/ILLEGAL/CANT/MOVING/CLOAKED. Critical finding: gate #59 (`warhead.Verses[target.Armor]==0`) is the engine-side weapon-cannot-target-armor block (drives primary/secondary swap). 8 open follow-ups including full enum trace, TechnoType flag identities, and ForceFire bypass mechanism. Next: `systems/anti_air_dispatch.md`. |
| 2026-05-17 | 8 | **DONE** `systems/anti_air_dispatch.md`. `SelectWeaponAgainst 0x006F3330` decompiled with 14-phase decision tree (A=deploy-stick, B=garrison→0, C=load weapons, D=open-topped passenger override, E=Gattling stage dispatch, F=Airstrike, G=Magnetron, H=NavalGunboat, I=anim-state swap, J=internal-garrison, K=ElectricAssault, L=Dogfight, M=Cell-water, N=Verses-driven, Z=default Primary). Confirmed Phase N Verses swap is the load-bearing branch. Three distinct "naval" flags identified at `weapon+0x142`, `type+0x5EF`, `type+0x604` (resolution deferred). FUN_00717880 (DeployFire predicate, reads type+0x808) decompiled. 8 open follow-ups (building-airstrike flag identities, naval-flag triplet, Verses tie-breaker order, OpenTransportWeapon parser). Next: `systems/veterancy_weapon_swap.md`. |
| 2026-05-17 | 9 | **DONE** `systems/veterancy_weapon_swap.md`. Veterancy three-tier system confirmed via live decomp of all four predicates (`VeterancyClass::IsRookie/IsVeteran/IsElite/Reset` at `0x0074FFC0/0x0074FF90/0x00750010/0x00750080`) — Rookie [0,1), Veteran [1,2), Elite [2,∞). Float thresholds `0.0` / `1.0` / `2.0` verified via live constant reads (`FLOAT_007e1748`, `_DAT_007e2ac8`, `_g_BridgeDiag_BothSides_2_0` — last is Ghidra-mislabeled `2.0f`). `TechnoClass::GetWeapon 0x0070E140` decompiled showing Elite-only weapon-pointer swap via parallel slot arrays at `type+0x898` (regular) and `type+0xA94` (elite), stride `0x1C` per slot. `ElitePrimary`/`EliteSecondary` parser xrefs at `0x00712a32/0x00712a5f` verified. 10 open follow-ups including Veterancy field offset on TechnoClass instance (HIGH-priority, unresolved this pass), promotion formula trace, full ability-name → byte-offset map. Next: `systems/friendly_fire.md`. |
| 2026-05-17 | 10 | **DONE** `systems/friendly_fire.md`. `AffectsAllies` flag at `wh+0x179` (default false) verified via live xref of string `"AffectsAllies"` at `0x00847CC8` and live decomp of `TechnoClass::ReceiveDamage 0x00701900`. The friendly-fire damage gate is the ONLY ally-status block in the damage path; weapon fires normally at allies, damage is zeroed at impact. Splash composition is per-target ReceiveDamage so AffectsAllies applies per-target. ForceFire does NOT bypass this gate. AI vs player asymmetry exists only in target-selection (upstream), not at the gate. Psychedelic separate ally gate (`wh+0x16D`) documented as second layer. 5 open follow-ups (ambient-damage NULL-sourceHouse caller trace, IvanBomb AffectsAllies value confirmation). Next: `systems/accuracy_inaccurate.md`. |
| 2026-05-17 | 11 | **DONE** `systems/accuracy_inaccurate.md`. All three flag identities verified via live xrefs (`Inaccurate` `+0x2A2`, `FlakScatter` `+0x2A3`, `BallisticScatter` Rules+`0x1734`). Key clarification: `Inaccurate=` does NOT angle-jitter — it gates the detonation-time target-snap and the 32/42/128-lepton pre-impact near-miss damage. Random scatter is the `Inviso AND FlakScatter` combination at launch with formula `jitter = (RandomRanged(0, BallisticScatter×2) × dist) / (Owner+0xB4)` × random angle. `Proximity` flag (`+0x29F`) confirmed dead-read (parsed but no consumer). 7 open follow-ups including `Owner+0xB4` divisor identity (HIGH priority). Next: `systems/airburst.md`. |
| 2026-05-17 | 12 | **DONE** `systems/airburst.md`. Live decomp of `BulletClass::BulletDetonation 0x00468D80` (2026-05-17) confirmed the Airburst/Cluster dispatch fork. Spawn block at end of `WarheadTypeClass::Detonate` produces hardcoded 9 sub-bullets (8-loop + 1 explicit) targeting a 3×3 cell footprint, each carrying full `AirburstWeapon.Damage` without scaling. Velocity construction quirks documented (45°-cone launch direction, `sin(3π/2)`-based "horizontal vector" trick). `[V3AirburstP]` confirmed sole shipping use; Flak Cannon clarified as NOT using Airburst (uses Inaccurate+FlakScatter instead). 5 open follow-ups (AirburstWeapon validation, ClusterBits default Cluster, 45°-cone intent). Next: `systems/ambient_damage.md`. |
| 2026-05-17 | 13 | **IN-PROGRESS** `systems/ambient_damage.md`. Parser side verified live: `AmbientDamage=` at `weapon+0x98` is the first ReadInt in `WeaponTypeClass::ReadINI 0x007720bb`. Exhaustive retail-INI use: 5 weapons (LtRail=150, MechRailgun=200, FireballLauncher=2, SonicZap=10, SonicZapE=15). INI authors explicitly tag the field "use this for the railgun damage field. Leave damage = 0". **Consumer trace incomplete** — flagged as HIGH-priority open follow-up #1; likely consumer is RadBeam::AI / WaveClass::AI per-tick damage callback. Spec assumptions about Verses/AffectsAllies/no-CellSpread are inferred not verified. Marking IN-PROGRESS with a gap note rather than DONE. Next: `systems/chain_reaction.md`. |
| 2026-05-17 | 14 | **DONE** `systems/chain_reaction.md`. Live decomp of `OverlayTypeClass::ReadINI 0x005FE7F0` (2026-05-17) yielded correct OverlayType byte layout: `+0x2A9=Tiberium`, `+0x2B0=Explodes`, `+0x2B1=ChainReaction` — corrects a flag-identity swap in iteration-6 `splash_cellspread.md` (now fixed in-place this iteration). Three chain mechanisms separately documented: (1) Reduce_Tiberium dispatch via warhead.Tiberium × overlay.ChainReaction × tiberium-overlay gate — partially live; (2) TS-era `TiberiumExplosionDamage` chain shockwave — dormant (`=0` in retail rulesmd.ini line 811, explicitly commented as disabling the chain); (3) IC-barrel recursive Apply_area_damage chain via `Explodes=yes` + Rules.C4Warhead — fully live. Veinhole / Veins system confirmed TS-dead. 7 open follow-ups (TiberiumExplosionDamage Rules offset, ChainReaction default, splash_cellspread regression fix completed). Next: `systems/mind_control.md`. |
| 2026-05-17 | 15 | **DONE** `systems/mind_control.md`. Both MC mechanisms documented with full 3-axis confidence: (1) CaptureManager-based reversible MC via `wh+0x155 MindControl` (live xref to ReadINI `0x0075D7CF`) + `weapon+0x140 InfiniteMindControl` (live xref `0x00772218`). `CaptureManagerClass` (0x50 bytes) + `MCNode` (0x14) struct layouts verified. `CanCapture 0x00471C90` decomp confirmed live (every gate matches existing canonical doc). (2) Psychic Dominator permanent MC via `+0x2C4` flag — completely separate code path at `PsychicDominator::MindControlArea 0x0053B080`. Mutually-exclusive warhead-special priority cascade (11 levels) documented. Mastermind overload damage tier system via `Rules.OverloadCount/Damage/Frames` DynVectors with default tiers (0/0/50/100/500 dmg at 3/6/10/50 victim thresholds), self-damaging via Rules.C4Warhead. 6 FreeAll trigger sites enumerated. 8 open follow-ups including ImmuneToPsychicDominator (`type+0xD6A`) INI key identity. Next: `systems/temporal.md`. |
| 2026-05-17 | 16 | **DONE** `systems/temporal.md`. `Temporal=` flag at `wh+0x15A` (live xref `0x00817168` → ReadINI `0x0075D590`). `Warpable=` at `type+0xD3A` (live xref `0x00843778`). `OpenToppedWarpDistance` Rules+`0xF60` (live xref `0x0083AFD4`). `CanWarpTarget 0x0071AE50` decompiled live, confirming Warpable check + IsInvulnerable + infantry-on-Grinder same-cell gate (with `type+0x16BD` grinder flag). Erase formula: `WarpHP = target.Strength × 10`, decremented by chain damage sum each tick (depth-cap 51). Doubly-linked chain stacking with head-detach WarpHP transfer (loss of head ≠ progress reset). Instant snap-back on Detach (no recovery curve). Building-target completion includes parachute occupants + SuperClass::Suspend + UndockUnit. 8 open follow-ups including warhead cascade priority order (Temporal vs Parasite swap question) and Update-time immunity re-check. Next: `systems/ion_cannon.md` (TS-legacy candidate). |
| 2026-05-17 | 17 | **DONE** `systems/ion_cannon.md`. Confirmed FULLY DORMANT in YR with VERY HIGH confidence. IonBlastClass C++ class has 0 code refs (only unreferenced RTTI ghosts remain). IonCannonSpecial SW is registered-but-commented-out. SuperClass::Launch switch (`0x006CC390`) has NO case for IonCannon — even mod re-activation wouldn't make it work. `IsIonCannon` weapon flag doesn't exist in the binary at all (verified via search_strings: 0 matches). Only LIVE consumer of "Ion" infrastructure is `Rules+0x298 IonBlast` AnimType, repurposed by Genetic Mutator SW (Case 9). Clarified `IonWH` warhead is separate and IS live (Lightning Storm uses it via `LightningWarhead=IonWH`). Doc serves to prevent re-implementing dead TS code. 4 minor follow-ups. Next: `systems/rail_gun.md`. |
| 2026-05-17 | 18 | **DONE** `systems/rail_gun.md`. Two distinct flag systems: `IsRadBeam=yes` (visual, with `temporal=yes`-driven color swap green/blue) and `IsRailgun=yes` at `weapon+0x12D` (live xref `0x00849368`, sticky-beam ROF/FireError gates via `Owner+0x314 railgunParticleSys`). `SpawnRadBeam 0x006FD620` re-decompiled live — confirmed pure visual setup, NO damage dispatch. Three damage regimes mapped: A=Desolator standard projectile, B=Chrono Legionnaire Temporal (Damage as WarpHP decrement), C=LtRail/MechRailgun AmbientDamage path (consumer site STILL not traced). RadEruption confirmed LIVE in YR via `[DeplDesoWeapon] IsRadEruption=yes` (corrects existing canonical doc's TS-suspect flag). 6 open follow-ups including the AmbientDamage consumer (HIGH priority, recurring from iteration 13). Next: `systems/radiation.md`. |
| 2026-05-17 | 19 | **DONE** `systems/radiation.md`. `RadLevel=` at `weapon+0x158` (live xref `0x00849298`), `ImmuneToRadiation=` at `type+0xD37` (live xref `0x00843854`), `Radiation=yes` warhead flag at `wh+0x177`. RadSiteClass 0x74-byte struct fully mapped. All 10 `[Radiation]` Rules keys at offsets `+0x1804..+0x1834` documented (RadDurationMultiple, RadApplicationDelay, RadLevelMax, RadLevelDelay, RadLightDelay, RadLevelFactor, RadLightFactor, RadTintFactor, RadColor, RadSiteWarhead). RadSite lifecycle: creation in WarheadTypeClass::Detonate post-area-damage block → Activate computes per-step decrements → SetCellRadLevels initializes linear falloff (`(SpreadInLeptons-dist)/SpreadInLeptons × RadLevel` per cell) → AI ticks decay per RadLevelDelay/RadLightDelay → self-destruct at RemainingDuration<=0. Augmentation behavior (AddRadLevel stacks on same-cell reuse) verified. **Per-unit damage application site NOT traced** — HIGH-priority open follow-up #2. 8 open follow-ups including spread source, per-unit damage dispatch, RadLevelMax clamp on augment, RadLevelFactor consumer. Next: `systems/suicide_weapons.md`. |
| 2026-05-17 | 20 | **DONE** `systems/suicide_weapons.md`. Two distinct mechanisms: (a) `Suicide=yes` at `weapon+0x144` (live xref `0x00843050` → `WeaponTypeClass::ReadINI 0x0077228D`) — Fire_At short-circuits with SetTarget(self), firer dies in own explosion; (b) `DeathWeapon=` on TechnoTypeClass (live xref `0x0083B11C` → ReadINI `0x007122F0`; Rules default at `0x0066C58A`) plus `DeathWeaponDamageModifier=` (live xref `0x00844488`) — weapon fired at unit position on death. Retail INI survey: 4 Suicide weapons (Demobomb, IvanBomb/CRIvanBomb, CRNuke); 15+ DeathWeapon assignments. Composition: Demo Truck has BOTH flags → potential double-explosion. Aircraft crash damage controlled via `DeathWeaponDamageModifier=.1` (Kirov, Harrier, BEAGLE). 8 open follow-ups including HIGH-priority DeathWeapon dispatch site trace + Demo Truck double-explosion verification. All math-layer system docs are now DONE/IN-PROGRESS — next phase begins with the long-tail per-system docs. Next: `systems/emp.md`. |
| 2026-05-17 | 21 | **DONE** `systems/emp.md`. **EMP is FUNCTIONALLY DORMANT in YR.** Live xrefs verified: `"EMEffect"` `0x00847D60`, `"EMPulse"` `0x0081721C`, `"EMPulseSparkles"` `0x0083CCA4`. INI survey: only `[EMPuls]` warhead has `EMEffect=yes` and is explicitly marked `;gs disabled in code` (line 26413). `EMPulseWarhead=EMPuls` (line 587) and `EMPulseProjectile=PulsPr` (line 588) Rules refs point to the disabled warhead. `EMPulseClass` C++ class (0x34 bytes) and its Apply/ReceiveEMP/BuildingClass::ApplyOfflineEffects code paths are FULLY DEAD — no live caller. `TechnoClass.EMPLockRemaining (+0x504)` is read by consumers but never written. Twin doc to ion_cannon.md (both TS-legacy dormancy). `EMPulseSparkles` AnimType is the ONE live "EMP" asset, repurposed by RadSite visuals. Correction flagged in existing RADIATION_EMP doc: claim of "case 3 = EMPulse SW in SuperClass::Launch" is wrong (case 3 is ChronoSphere). 6 open follow-ups. Next: `systems/parasite.md`. |
| 2026-05-17 | 22 | **IN-PROGRESS** `systems/parasite.md`. `Parasite=yes` warhead flag at `wh+0x159` verified (live xref `"Parasite"` at `0x0081717C`). Three retail warheads documented from rulesmd.ini: `[Parasite]` Terror Drone (vehicles 100%, buildings 0%), `[ParasiteDog]` Attack Dog (infantry 100%, vehicles 0%), `[ParasitePlus]` Squid Grab (100% everywhere). `ParasiteClass` constructor (`0x006292B0`) decompiled live — ~0x58-byte struct allocated via `TechnoClass::Init_Managers 0x006F3F40` (same allocator as CaptureManagerClass; architecturally analogous). Global array at `0x00AC4914..0x00AC4924`. Cascade priority 5 in WarheadTypeClass::Detonate (per existing canonical mind_control doc). **Per-tick AI loop, damage-per-tick value source, tick rate, cross-class TechnoClass field offsets — all unverified.** IN-PROGRESS rather than DONE due to lifecycle gaps. 11 open follow-ups total including HIGH-priority Update decomp + damage source. Next: `systems/sonic.md`. |
| 2026-05-17 | 23 | **DONE** `systems/sonic.md`. **Critical correction:** existing canonical `WAVECLASS_AI_AND_CORRECTIONS_ADDENDUM.md` §3 claim that "IsSonic is TS-LEGACY DEAD CODE IN YR" is WRONG — `IsSonic=Yes` (capital Y) IS in retail rulesmd.ini at lines 23688+25107 for `[SonicZap]` and `[SonicZapE]`. The addendum's grep used lowercase `yes` and missed these; INI parser is case-insensitive. WaveClass type-0 IS LIVE in YR for Sonic Tank. Three parallel mechanisms per Sonic Tank shot: (1) WaveClass type-0 visual via `weapon+0x130 IsSonic` (no damage), (2) standard BulletClass + SonicWarhead damage (Damage=4/8), (3) AmbientDamage path (10/15, consumer still untraced per ambient_damage.md). `firer+0x324 CurrentWave` slot tracks active wave per firer. 8 open follow-ups including correction to existing canonical doc. Next: `systems/locomotor_warhead.md`. |
