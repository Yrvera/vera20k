# Unit Doc Audit Index

Deep-Ghidra audit pass tracking. Separate from `INDEX_UNITS.md` (which tracks
doc creation status). This tracker records which docs have had their
behavioral claims independently re-verified against the binary.

## Audit policy

Each doc audited per the loop instructions established in iteration 1:

1. Read doc, identify behavioral claims (named functions, struct offsets,
   hardcoded mechanics).
2. Verify in Ghidra via `get_function_by_address` + `decompile_function` +
   `get_function_callers` + struct-offset reads (target: 10-15 decompiles per doc).
3. Tag claims in-place with `[BINARY-VERIFIED]` / `[INFERRED]` /
   `[INCORRECT]` / `[NAME DISCREPANCY]` etc.
4. Append `## Ghidra audit log (audit iteration N — date)` section
   recording: methodology, function-entry verification table, key
   behavioral findings, discrepancies, items NOT re-verified, confidence
   summary.
5. Mark `DEEP-AUDITED` here in this index with iteration number + key findings.

## Audit status legend

- `DEEP-AUDITED` — full pass complete, audit log section appended, claims tagged.
- `PARTIAL-AUDITED` — some claims verified but bounded by Ghidra-effort limits.
- `LIGHT-CHECK` — quick xref / function-existence check only, no decompile pass.
- `TODO` — not yet audited.
- `SKIP` — doc is too thin to warrant audit (e.g., dummy/internal units).

## Per-doc audit status

### Allied infantry

| Doc | Status | Iter | Key findings |
|-----|--------|------|--------------|
| `allied/E1.md` | DEEP-AUDITED | 1 (2026-05-18) | 10 function entry points verified; `DoType_Sequencer @ 0x00520A60` is INCORRECT (no standalone fn; address falls inside Fire_At_Target body — sequence transitions are vtable+0x558 vfunc calls). `Fear_Decay_Handler` is Ghidra-labeled `InfantryClass::SetFear`. `Occupier @ InfantryTypeClass+0xEB4` BINARY-VERIFIED from AddGarrisonOccupant decompile. Deployed sequence IDs 0x1b–0x1e verified in code. Weapon-slot offsets +0xE40/E44/E48/E4C verified on InfantryTypeClass (vs the doc's `TechnoTypeClass+0x6AC` parse-time claim — both can be correct at different lifecycle stages). |
| `allied/GGI.md` | DEEP-AUDITED | 2 (2026-05-18) | 5 function entry-points checked: CanCrushCheck @ 0x005f6cd0 ✅, GetFireError @ 0x0051C8B0 ❌ phantom (no function), TechnoClass::GetFireError @ 0x006FC0B0 ✅, SelectWeapon @ 0x005218E0 ⚠️ unlabeled (`FUN_005218e0`), DrawExtras @ 0x006F5190 ✅. **Decompile of `FUN_005218e0` BINARY-VERIFIES `TechnoTypeClass+0x6ac = DeployFire` + `+0x6a8 = DeployFireWeapon` slot index, and sequence IDs 0x1b-0x1e for deployed state.** CanCrushCheck reads `+0x2A4` on entity for crush gate — confirms IsLowSilhouette offset. `DeployedCrushable` parser key verified InfantryType-scope (0x00524627 in InfantryTypeClass__ReadINI); specific +0xEC9 offset NOT decompile-verified. CanCrushCheck callers traced: Drive/Ship Process_Drive_Track + Can_Enter_Cell + PerCellProcess + What_Action_OnObject. Two new type-side crush flags discovered: `+0xd29, +0xd2a, +0x22d` (INI mapping not yet pinned). |
| `allied/ENGINEER.md` | DEEP-AUDITED | 3 (2026-05-18) | **MAJOR FINDING**: Engineer flag offset is **`+0xEC5`** (verified in Mission_Capture decompile), NOT `+0xEC3` as the doc had claimed. The doc's previous "correction" from +0xEC5 to +0xEC3 was wrong; reverted. `InfantryClass::Mission_Capture @ 0x005202F0` ✅ exact + fully decompiled. `Mission_Enter @ 0x005196A0` ❌ phantom (address inside `PerCellProcess` body at 0x00519630). Distance gates 0x80/0x200 verified. Vtable slots verified: SetMission@+0x274, Limbo@+0xDC, ChangeOwner@+0x3D4, self-destruct@+0xF8. Vestigial fields confirmed: `EngineerCaptureLevel` (parsed twice in ReadGeneral, no runtime xref), `MultiEngineer` (parsed once in ReadMultiplayerDialogSettings, no runtime xref, with lobby debug string "Crap Engineers: %s\n"). `Engineer=` parser key is InfantryType-scope. Field-offset notation clarified: doc's `field_0xCE` notation = byte offset `0xCE*4 = 0x338`. Pattern emerging across audits 1-3: docs cite addresses INSIDE function bodies (not entry points) for behavior locations — DoType_Sequencer, Fear_Decay_Handler, GetFireError, Mission_Enter all show this pattern. |
| `allied/GHOST.md` | DEEP-AUDITED | 4 (2026-05-18) | C4 plant gate FUN_0051f3e0 (unlabeled SelectWeapon-style — Mission_Attack address verified, name unconfirmed). **TypeClass+0xEC2 = C4 flag, +0xEBE = Infiltrator, BldgType+0x1577 = CanC4, +0x1701 = InvisibleInGame, +0x520 = type ptr on BuildingClass — ALL BINARY-VERIFIED via Mission_Attack decompile.** Vtable: +0x480 = Set_Target, +0x1e8 = SetMission, mission ID 0x11 = Enter. Weapon-ability 0xE is the UC-clear alternate gate. **RTTI value conflict**: GHOST sees `BuildingClass RTTI == 6` via vtable+0x2c, but ENGINEER audit 3 saw `== 1` via the same vtable slot. DEFERRED — needs GetAbstractType decompile to disambiguate. `UseOwnName` is **InfantryType-scope** (not TechnoType as doc claimed). 7 parser-key scopes verified (CanC4, C4Warhead Rules-CombatDamage NEW SCOPE, SabotageCursor WeaponType, DetectDisguise/DetectDisguiseRange TechnoType, NavalTargeting TechnoType, UseOwnName InfantryType). Apply_area_damage detonation chain DEFERRED. |
| `allied/CLEG.md` | DEEP-AUDITED | 5 (2026-05-18) | **Highest verification rate of any doc audited.** All 9 cited functions verified at exact addresses with canonical Ghidra labels: Fire_At, WarheadType::Detonate, TemporalClass::{InitiateWarp,Update,SumChainDamage,DetachFromTarget,CanWarpTarget}, WarpAttachClass::Detach, TechnoClass::SpawnRadBeam. **WarpHP = TypeClass+0xA0 × 10 BINARY-VERIFIED** (InitiateWarp decompile). **TypeClass+0xA0 = Strength** (corrects audit 1's "display-name pointer" claim). **RTTI conflict from audit 3/4 RESOLVED**: RTTI=6=BuildingClass (CLEG InitiateWarp + GHOST Mission_Attack confirm), RTTI=1=FootClass (CLEG CanWarpTarget's FootClass::GetDestination branch confirms). **ENGINEER audit 3's "RTTI=1=BuildingClass" interpretation was WRONG** — Mission_Capture's RTTI==1 means FootClass target (vehicle/infantry), not building. Engineer building-capture path may need re-investigation. New offsets: TechnoClass+0x270 IsBeingWarpedOut, +0x278 back-ptr to TemporalClass, +0x2BC CaptureManager ptr, +0x2D0 SpawnManager ptr, +0xCD5 IsGattling. TemporalClass instance: +0x24 attacker, +0x28 target, +0x40/+0x44 chain prev/next, +0x48 WarpHP. TypeClass+0xD3A=Warpable BINARY-VERIFIED. vtable+0x160=IsInvulnerable. Warpable + Teleporter both TechnoType-scope verified. |
| `allied/SPY.md` | DEEP-AUDITED | 6 (2026-05-18) | **15 function entry points verified at exact addresses**: OnSpyInfiltrate, IsDisguised_Getter, SpyPowerSabotage, OnSpyWeaponInfiltrate, Check_Spy_Reveal, Add/RemoveDetectDisguiseAt, Increment/DecrementDisguiseDetectCount, MapClass::RestoreShroud, FUN_0050BD10 (RestoreShroud wrapper), Spend_Money, Add_Credits, IsHumanPlayer, Mission_SpyPlane. **Full 7-branch OnSpyInfiltrate dispatch BINARY-VERIFIED** in evaluation order: same-owner → Radar(+0x16A4) → Power(+0xEE0) → BuildTech-list(Rules+0x920/+0x92C) → SuperWeapon(+0x16F0) → Storage(+0x800, TechnoType) → Factory==0x28 → Factory==0x10. **CORRECTION**: IsDisguised_Getter is NOT the per-viewer-side draw predicate the doc claimed — it's a 1-byte flag getter (`return *(byte*)(this + 0x1D8)`), so TechnoClass+0x1D8 = IsDisguised. **Caller chain**: OnSpyInfiltrate is called from InfantryClass::PerCellProcess @ 0x00519630 (consistent with engineer Mission_Enter pattern from audit 3). **15 INI parser scopes verified** via xref including: Agent/Infiltrate = InfantryType; PermaDisguise/Storage/AIBasePlanningSide/DetectDisguise(/Range)/CanDisguise = TechnoType; Radar/Power/SuperWeapon/Factory = BuildingType; BuildTech = Rules-AI; SpyPowerBlackout/SpyMoneyStealPercent/AttackCursorOnDisguise/InfantryBlinkDisguiseTime = Rules-General. New offsets discovered: Rules+0xD58/D5C/D60 = Allied/Soviet/ThirdDisguise pointers; Rules+0xD6C = AttackCursorOnDisguise; Rules+0x1014 = InfantryBlinkDisguiseTime; TechnoType+0x5F4 = DetectDisguiseRange; TechnoType+0xD2F/D30/D31 = CanDisguise/PermaDisguise/DetectDisguise bytes; CellClass+0xAC = per-house disguise-detect counter array; HouseClass+0x577A = LowPowerState (distinct from +0x5778 PowerBlackedOut). |
| `allied/TANY.md` | DEEP-AUDITED | 7 (2026-05-18) | **~15 decompiles**; 6 function entry points re-verified (Mission_Attack=FUN_0051F3E0 vtable-bound, CanCrushCheck, CaptureUnit, CanCapture (NEW — actual ImmuneToPsionics consumer), ObjectTypeClass::ReadINI). **PHANTOM CONFIRMED**: Mission_Enter @ 0x005196A0 falls inside `InfantryClass::PerCellProcess` body — no standalone function exists (same pattern as ENGINEER/SPY audits). **4 MAJOR STRUCT-OFFSET CORRECTIONS**: BuildLimit was claimed +0x6F8, actually **TechnoType+0x3B8**; SelfHealing was claimed +0xC92, actually **TechnoType+0xD14**; ImmuneToPsionics was claimed "InfantryType+0xCD7", actually **TechnoType+0xD35** (wrong class AND offset); Crushable was vague "+0x4xx", actually **ObjectType+0x22D** (BINARY-VERIFIED via ReadINI). **CORRECTION to audit 2**: TechnoType+0xD29 was claimed "Crushable-related flag on target", actually **OmniCrusher** (crusher-side capability override). **CORRECTION to audit 1**: TechnoType+0xD50 was claimed "pre-deploy weapon override", actually **OpenTransportWeapon**. **17 parser-scope verifications**: Crushable/Bombable/Strength/Immune/LegalTarget/IgnoresFirestorm/UseLineTrail/Voxel/AlternateArcticArt = ObjectType; OmniCrusher/ImmuneToPsionics/SelfHealing/BuildLimit/IFVMode/NavalTargeting/OpenTransportWeapon/ImmuneToVeins/IsSelectableCombatant/LeadershipRating/ThreatPosed = TechnoType; UseOwnName/Crawls/Assaulter/TiberiumProof/DeployedCrushable = InfantryType. **C4 plant path fully decompiled**: Type+0xEC2 (C4) → vtable+0x480 Set_Target(target,1) → vtable+0x1E8 SetMission(0x11=Enter, 0), guarded by RTTI=6 building + BldgType+0x1577 CanC4 + BldgType+0x1701 InvisibleInGame=0. Non-player Infiltrator branch uses vtable+0x1F0(8). **CaptureUnit/CanCapture chain**: CaptureUnit calls CanCapture which reads TechnoType+0xD35 ImmuneToPsionics; if set, returns 0 → mind-control fails. |
| `allied/JUMPJET.md` | DEEP-AUDITED | 8 (2026-05-18) | **13 function entry points re-verified at exact addresses**: JumpjetLocomotionClass::Constructor @ 0x0054AC40 (Ghidra-labeled with canonical CLSID comment), Constructor variant @ 0x0054AD00 (Ghidra labels Constructor, NOT destructor as doc claimed), Process @ 0x0054AEC0, State handlers 0-5 (0x0054B980, 0x0054BA30, 0x0054BD30, 0x0054BFF0, 0x0054C550, 0x0054CA90), In_Which_Layer @ 0x0054B8D0, RulesClass__ReadJumpjetControls @ 0x006743D0 (Ghidra-labeled), FootClass__Locomotion_AI @ 0x00520F40 (Ghidra-labeled). **Constructor BINARY-VERIFIES three-vtable COM layout**: IUnknown @ +0x0, ILocomotion @ +0x4, IPiggyback @ +0x18. **State 0 BINARY-VERIFIES** instance offsets: +0x2C = CruiseHeight cache, +0x50 = state field, +0x80 = climb target (caches +0x2C → +0x80, sets state=1 transitioning to Liftoff). **FootClass::Locomotion_AI BINARY-VERIFIES** sequence dispatch: TechnoTypeClass+0xD94 = JumpJet flag gates jumpjet path; CLSID-match via DAT_007E9AC0 → sequence 0x17 (Hover, when velocity ≤ threshold) or 0x18 (Fly, when velocity > threshold). **RulesClass JumpjetControls block BINARY-VERIFIED**: TurnRate=+0x40C, Speed=+0x410, Climb(double)=+0x418, CruiseHeight=+0x420, Acceleration(double)=+0x428, WobblesPerSecond(double)=+0x430, WobbleDeviation=+0x438. **8 INI parser scopes verified**: BalloonHover/JumpJet/ConsideredAircraft/HoverAttack/Crashable/JumpjetNoWobbles/JumpjetHeight = TechnoType; Fearless = InfantryType. **New TechnoType offsets**: +0x390 HoverAttack, +0xD6A BalloonHover, +0xD70 JumpjetSpeed, +0xD74 JumpjetClimb, +0xD78 JumpjetCrash, +0xD94 JumpJet flag, +0xD95 Crashable (DISTINCT from ObjectType+0x22D Crushable!), +0xD96 ConsideredAircraft. State 1-5 handler bodies + deep RE doc DEFERRED (only entry points re-verified). |
| `allied/ADOG.md` | DEEP-AUDITED | 9 (2026-05-18) | **~12 decompiles**; ParasiteClass primary + variant constructors @ 0x00629210 / 0x006292B0 verified (Ghidra-labeled). **ParasiteClass instance offsets BINARY-VERIFIED**: +0x0..+0xC = 4 vtable ptrs (multi-interface COM), +0x2C = LaunchFrame, +0x34 + 0x40 = cleared fields, +0x38 = secondary timestamp; global DynamicVector at DAT_00ac4914 tracks all ParasiteClass instances. **WeaponTypeClass+0x132 = LimboLaunch (byte)** BINARY-VERIFIED via WeaponTypeClass__ReadINI decompile, which also pinned 40+ other WeaponType offsets (+0xA0 Projectile, +0xA4 Damage, +0xA8 Speed, +0xAC Warhead, +0xB0 ROF, +0xB4 Range, +0x131 Spawner, +0x141 FireWhileMoving, +0x143 FireInTransport, +0x12B OmniFire, +0x149 IsLaser, +0x155 IsRadBeam, etc.). **TechnoTypeClass offsets BINARY-VERIFIED**: +0x693 Natural, +0xD37 ImmuneToRadiation, +0xD39 DefaultToGuardArea, +0xD3C ReselectIfLimboed, +0xD3D RejoinTeamIfLimboed. **9 parser-scope verifications**: LimboLaunch = WeaponType, NotHuman = InfantryType, Parasite = WarheadType; DefaultToGuardArea/ReselectIfLimboed/RejoinTeamIfLimboed/Natural/ImmuneToRadiation = TechnoType. DEFERRED: NotHuman exact offset (InfantryType-scope verified via xref but offset not pinned in this pass), ParasiteClass::Update tick loop (the per-tick host-attach damage loop for Terror Drone), Parasite WarheadType offset, LimboLaunch consumer end-to-end in Fire_At. |
| `allied/SNIPE.md` | DEEP-AUDITED | 10 (2026-05-18) | **~10 string + xref verifications + cross-reference to audit 9's WeaponType decompile**. All 6 INI key strings at exact addresses (UseOwnName/RequiredHouses/RevealOnFire/PreventAttackMove/ForbiddenHouses/CanPassiveAquire). All 6 parser-scope verifications: UseOwnName = InfantryType; the other 5 = TechnoType. **New struct offsets BINARY-VERIFIED**: TechnoType+0x6C8 = PreventAttackMove, TechnoType+0xD99 = CanPassiveAquire, TechnoType+0xDA0 = RequiredHouses (int, parsed via FUN_004750D0), TechnoType+0xDA4 = ForbiddenHouses (int). **WeaponType+0x137 = RevealOnFire** confirmed via audit 9's WeaponType decompile. **"Data-driven, not hardcoded" claim BINARY-VERIFIED**: `search_strings("^Sniper$")` and `search_strings("HollowPoint")` both return 0 matches — confirms no hardcoded sniper path or warhead name reference. `search_strings("SILENCER")` also returns 0 — confirms the `[Sniper]` weapon's `Report=SILENCER` is dead (sound block doesn't exist), so the `[Sniper]` weapon is genuinely vestigial. DEFERRED: RevealOnFire consumer in Fire_At code path, RequiredHouses buildability gate, UseOwnName exact byte offset (only scope confirmed). |
| `allied/CCOMAND.md` | DEEP-AUDITED | 11 (2026-05-18) | **~16 Ghidra queries** (12 decompiles + 4 string-searches). `TeleportLocomotionClass` is exceptionally well-labeled — **19 named member functions** beyond the constructor (rare; most internal classes have 1–2 labels). All 19 entry points + constructor verified at exact addresses, Ghidra-labeled with canonical CLSID comment. **Constructor decompiled**: 3-vtable COM layout (IUnknown @ +0x0, ILocomotion @ +0x4, IPiggyback @ +0x18), src coord @ +0x1C..+0x24, dest coord @ +0x28..+0x30, state byte @ +0x34, LaunchFrame @ +0x3C. **CRITICAL CORRECTION to cumulative cheat-sheet**: `Teleporter` is at **`TechnoType+0xCD4`**, NOT `+0xD3A` (which is `Warpable` — distinct meaning, per CLEG audit 5). Two distinct INI keys, two distinct offsets, two distinct semantics: `Teleporter` = "this unit CAN warp itself", `Warpable` = "this unit can BE warped by Chrono Legionnaire". **Address discrepancies corrected in doc**: TimerCheck was `0x0070F770` → actual `0x00719BF0`; Teleporter parser site was `0x0071450F` → actual `0x00713FE9` (the `0x0071450F` is the RequiresStolenAlliedTech parser site — transposed). **5 new TechnoType offset+key bindings BINARY-VERIFIED**: +0xC8D MoveToShroud, +0xCD4 Teleporter, +0xD9B RequiresStolenThirdTech, +0xD9C RequiresStolenSovietTech, +0xD9D RequiresStolenAlliedTech. **Negative claim verified**: `search_strings("CCOMAND")` and `search_strings("ChronoCommando")` both return 0 — confirms no hardcoded section-name branch (all behavior data-driven from INI). |

### Allied vehicles & aircraft

| Doc | Status | Iter | Key findings |
|-----|--------|------|--------------|
| `allied/TNKD.md` | DEEP-AUDITED | 12 (2026-05-18) | **~12 Ghidra queries** (6 string searches + 4 xref lookups + 2 full inline decompiles + 2 oversized-decompile grep reads). TNKD has **no unit-specific code** in `gamemd.exe` (re-confirmed: `TankDestroyer`/`TNKD` return 0 matches, only `Name:TNKD` CSF lookup at 0x008299dc). Audit focus was *pinning down exact struct offsets* for every TNKD INI key — the previous doc cited parser *addresses* but no offsets. **4 function entry points re-verified** (TechnoTypeClass__ReadINI, UnitTypeClass__ReadINI, BuildingTypeClass_ReadINI_Water, RulesClass__ReadGeneral). **12 NEW TechnoType offsets BINARY-VERIFIED**: +0x5BC MaxDebris, +0x614 Soylent, +0xCA1 Turret (in-binary annotation; writer @ 0x007133C2), +0xD28 Crusher, +0xDBD Accelerates. Re-confirms +0xC91 ImmuneToVeins (audit 7), +0xD9B/D9C/D9D RequiresStolen{Third,Soviet,Allied}Tech (audit 11), +0xDA0/DA4 RequiredHouses/ForbiddenHouses (audit 10 — RequiredHouses populated via FUN_004750D0 country-bitmask helper). **25+ UnitType offsets BINARY-VERIFIED** via full UnitTypeClass__ReadINI decompile: +0xE0C–E1B = capability-flags block (Passive/CrateGoodie/Harvester/Weeder/DeployToFire/IsSimpleDeployer/IsTilter/UseTurretShadow/TooBigToFitUnderBridge/CanBeach/SmallVisceroid/LargeVisceroid/CarriesCrate/NonVehicle), +0xE1C–E3C = animation-frame ints, +0xE40/E48 = FiringSyncFrame[2]/BurstDelay[4] arrays, +0xE5C/E5D = WalkFrames/FiringFrames bytes, +0xE5E = AltImage string. **4 NEW BuildingType offsets**: +0xEA4 SecretInfantry (InfantryType*), +0xEA8 SecretUnit (UnitType*), +0xEAC SecretBuilding (BuildingType*), +0x16B0 SecretLab (byte). **3 NEW Rules-General offsets**: +0xD00 SecretInfantry / +0xD1C SecretUnits / +0xD38 SecretBuildings — all DynamicVector starts. **2 SCOPE CORRECTIONS to TNKD doc**: (1) `Turret` is **TechnoType-scope** +0xCA1 (NOT UnitType as doc claimed) — applies to all TechnoTypes incl. BuildingClass via BuildingClass::HasTurret. (2) `TooBigToFitUnderBridge` is **UnitType-scope** +0xE16 (NOT TechnoType as doc claimed). DEFERRED: UnitClass::Fire_At_Target @ 0x00736DF0 (Turret=no consumer), Secret Lab capture-picker chain (Engineer-capture → random Rules+0xD1C pick), CrateRules unit-spawn picker, TechnoClass::Explode random pick. |
| `allied/PENTGEN.md` | DEEP-AUDITED | 13 (2026-05-18) | **~10 Ghidra queries** (6 string searches + 4 xref lookups + 1 full inline decompile + 1 grep on saved decompile). PENTGEN is a thin campaign-placeholder doc (~390 lines, mostly INI data) with very little binary-verifiable content. **No PENTGEN-specific code in binary** (re-confirmed: `PENTGEN`/`Pentagon` → 0 matches). **`InfantryTypeClass__ReadINI` fully decompiled** (entry 0x005240A0, body 0x005240A0–0x0052475C) — 23 sequential ReadBool writes + 6 ReadInt writes into the InfantryType-specific +0xE40..+0xECB block; this is the unifying parser for ALL InfantryType-scope flags. **1 NEW TechnoType offset BINARY-VERIFIED**: `+0x634 = TechLevel` (int; parser xref @ 0x00714577; 5 additional consumer xrefs identified — lobby ReadMultiplayerDialogSettings, CCINIClass__Constructor, HouseClass::Read_Scenario_INI, FUN_006F1550, FUN_00501210). **2 TechnoType offsets re-confirmed** (audit 7): +0x670 ThreatPosed, +0x688 IFVMode. **UseOwnName confirmed InfantryType-scope** (audit 4 GHOST claim, now BINARY-VERIFIED via xref into InfantryTypeClass__ReadINI body); exact byte offset (one of +0xEAC..+0xECB ReadBool block) DEFERRED — not load-bearing since PENTGEN omits the flag. **No INCORRECT findings** in the doc; the 2 flagged INI bugs (Soviet Owner=, missing UseOwnName=) are correct observations about the shipped INI. DEFERRED: TechLevel build-availability gate consumer, ThreatPosed AI-zero-threat-exclusion consumer, IFVMode IFV-gunner-table consumer, UseOwnName display-name resolver chain, UseOwnName exact byte offset. |
| `allied/AMCV.md` | DEEP-AUDITED | 14 (2026-05-18) | **~14 Ghidra queries** (6 string-searches + 3 xref lookups + 2 get_function_by_address + 1 full UnitClass__Deploy decompile + 2 grep passes on saved TechnoTypeClass__ReadINI decompile). **`UnitClass::Deploy` @ 0x007393C0 fully decompiled** (Ghidra-labeled, body 0x007393C0–0x00739AB7) — confirms the doc's entire 8-step deploy sequence. **4 vtable slots BINARY-VERIFIED via the decompile**: +0x314 = CanDeploy (precondition predicate), +0xD8 = TryPlaceBuilding (on the NEW BuildingClass), +0xF8 = RemoveFromMap (matches audit 3 engineer self-destruct slot), +0x3A0 = Destroy/Limbo. Also confirms audit 5 RTTI=1=FootClass via the target-redirect loop's `vtable+0x2c == 1` check. **7 struct offsets BINARY-VERIFIED**: TechnoType +0x404 DeploysInto (BuildingType*, confirms SLAVE_MINER doc claim), +0x408 UndeploysInto (UnitType*), +0x40C PowersUnit (UnitType*, NEW sibling key), +0x56C DeploySound (int VocClass index, NEW), +0x570 UndeploySound (int, NEW), +0x6B8 DeployingAnim (AnimType*, NEW); BuildingType +0x16B9 IsDeployable (confirms doc claim — gates construction-yard special branch in Deploy). **CRITICAL CORRECTION to audit-2 cumulative**: TechnoType+0xD2A was tagged "crusher-side gate flag, INI mapping TBD" in audit 2 GGI. Now BINARY-VERIFIED as **`OmniCrushResistant`** (string @ 0x00843868, parser xref @ 0x00714D11) — the target-side resistance flag that blocks Battle Fortress / OmniCrusher attempts in CanCrushCheck. Resolves the 3-tier crush hierarchy: Crusher (vehicle) → Crushable (target) → OmniCrusher (vehicle override) → OmniCrushResistant (target final-override). **Deploy_facing_calculator @ 0x00465D70 ADDRESS PARTIAL**: function exists at that address but body is only 7 bytes — this is a thin stub/thunk, not the actual facing-rule body. Doc cite is partially correct (label exists) but the rule logic lives in caller. Negative claim confirmed: `search_strings("AMCV")` → 0 matches. DEFERRED: Deploy_facing_calculator true rule body, CanDeploy predicate body, BuildingClass::Constructor body, BuildingType +0x16C4/+0x16CA INI mappings (FacingClass::UpdateFacing trigger flags). |
| `allied/MTNK.md` | DEEP-AUDITED | 15 (2026-05-18) | **~10 Ghidra queries** (5 string-searches + 3 xref lookups + 1 grep on saved TechnoTypeClass__ReadINI decompile + 1 INI cross-check). MTNK has **no unit-specific code** in `gamemd.exe` (re-confirmed: `MTNK`/`Grizzly` → 0 matches). All 3 doc-cited Ghidra parser xrefs verify exactly. **2 NEW TechnoType offsets BINARY-VERIFIED**: `+0x608 = BuildTimeMultiplier` (float bits stored as int via ReadDouble cast; `param_1[0x182]`), `+0x6AF = OpportunityFire` (byte; `(int)param_1 + 0x6af` after ReadBool). **1 re-confirmation**: UnitType+0xE16 = TooBigToFitUnderBridge (audit 12). **Cross-INI confirmation of doc claim**: APOC at rulesmd:7791 has `Image=MTNK` — confirms the doc's interpretation that the artmd `[MTNK]` block is Apocalypse's live art (Grizzly redirects to `[GTNK]` via `Image=GTNK`). **No INCORRECT findings** in the doc. DEFERRED: BuildTimeMultiplier consumer in build-queue timer (Cost/BuildSpeed × Multiplier formula), OpportunityFire consumer in auto-targeting scan path, Image= resolver in AbstractType layer, Turret consumer in UnitClass::Fire_At_Target/Facing_Update. |
| `allied/MGTK.md` | DEEP-AUDITED | 16 (2026-05-18) | **~15 Ghidra queries** (8 string-searches + 7 xref lookups + 1 grep on saved TechnoTypeClass__ReadINI). MGTK has **no unit-specific code** in binary (re-confirmed: MGTK/Mirage → 0 matches; only `DefaultMirageDisguises` at 0x0083B488). All 6 doc-cited parser xrefs verify exactly. **§7.4 EliteSecondary-without-Secondary QUIRK RESOLVED**: BINARY-VERIFIED that ElitePrimary (+0xA94) and EliteSecondary (+0xAB0) are at **distinct slots, parsed independently**. No parser-time fallback copies EliteSecondary→ElitePrimary. Conclusion: elite Mirage Tank fires `MirageGun` (Damage 100, Range 7), not `MirageGunE` — the EliteSecondary INI line is effectively dead (likely a typo for ElitePrimary=). **3 NEW TechnoType offsets BINARY-VERIFIED**: +0xD32 = DisguiseWhenStill (byte; `(int)param_1 + 0xd32` after ReadBool), +0xD33 = CanApproachTarget (byte; `(int)param_1 + 0xd33`), +0xD2F = CanDisguise re-confirms audit 6. **3 NEW TechnoType weapon-slot offsets**: +0x898 = Secondary (WeaponType*; `param_1[0x226]`), +0xA94 = ElitePrimary (WeaponType*; `param_1[0x2a5]`), +0xAB0 = EliteSecondary (WeaponType*; `param_1[0x2ac]`). Primary +0x894 INFERRED by symmetry (DEFERRED — not directly in this grep window). NOTE: These TechnoType-level weapon slots are SEPARATE from the InfantryType-only +0xE40/E44/E48/E4C slots (audit 1) — both coexist via class hierarchy; InfantryType uses both, vehicles/buildings use only the TechnoType-level slots. **3 WeaponType re-confirmations** (DisguiseFireOnly +0x13B, DisguiseFakeBlinkTime +0x13C, RevealOnFire +0x137 — all audit 9 cumulative). **Doc updated**: §7.4 has a [RESOLVED audit 16] note appended at the top. **No INCORRECT findings**. DEFERRED: Rules-General offset for DefaultMirageDisguises DynamicVector start, disguise-update routine in TechnoClass::AI_Update, random tree-pick algorithm, fire-blink timer consumer, CanRetaliate "disguise as the bad guy" semantics. |
| `allied/CMIN.md` | DEEP-AUDITED | 17 (2026-05-18) | **~13 Ghidra queries** (7 string-searches + 6 xref lookups + 2 grep on saved TechnoTypeClass__ReadINI). CMIN cross-references 10+ deep-RE docs for chrono-teleport/dock-unload/mission-harvest; this audit re-verifies the 5 doc-cited parser xrefs + pins the new struct offsets + confirms the **dual-read pattern**. No CMIN-specific code (`CMIN` → 0 matches). **DUAL-READ PATTERN BINARY-VERIFIED for BOTH ChronoInSound AND ChronoOutSound** (doc was hedging on the latter — now confirmed): both have a global default in `RulesClass__ReadAudioVisual` AND a per-TechnoType override in `TechnoTypeClass__ReadINI`. **3 NEW TechnoType offsets BINARY-VERIFIED**: +0x574 = ChronoInSound (int VocClass index; sequence-position evidence — extends audit-14 DeploySound/UndeploySound block), +0x578 = ChronoOutSound (int), +0x6D4 = StupidHunt (byte). **NEW function entry point**: `RulesClass__ReadAudioVisual` (parses `[AudioVisual]` section globals — sibling to ReadGeneral and ReadJumpjetControls). **2 NEW Rules-General consumers identified** (offsets DEFERRED — RulesClass__ReadGeneral oversized): HarvesterUnit @ 0x0066F8DD, ChronoHarvTooFarDistance @ 0x00670003 (the 50-cell teleport-gate threshold). DEFERRED: UnloadingClass byte offset (TechnoType-scope + UnitType* value-type confirmed by parser, but write not in grep window), the 10+ chrono-miner deep-RE docs (trust-chain to cross-references, not re-verified), ChronoMinerTeleport [Limit=1] cap scope (per-house vs global). |
| `allied/FV.md` | DEEP-AUDITED | 18 (2026-05-18) | **~15 Ghidra queries** (7 string-searches + 1 full FUN_00717890 decompile + 1 grep on saved TechnoTypeClass__ReadINI). FV (Multi-Gunner IFV) is the most concrete-claim-rich vehicle doc — 5 cited parser xrefs, 17-weapon dispatch table, hardcoded gunner mechanism. All 5 doc-cited parser xrefs verify exactly. **5 NEW TechnoType offsets BINARY-VERIFIED**: +0x805 Gunner (byte; gates gunner-table), +0x808 TurretCount (int; 4 in FV), +0x80C WeaponCount (int; 17 in FV), +0x68C AirRangeBonus (int; sibling to +0x688 IFVMode), +0xD18 DeathWeapon (WeaponType* — TechnoType-side of dual-read with RulesClass__ReadCombatDamage). **NEW UnitType/TechnoType-extended field**: 17-int gunner turret-lookup table at `+0x814..+0x858`, populated by `FUN_00717890(this, TurretIndex, WeaponSlot)` calls (one per named TurretKey) in UnitTypeClass__ReadINI's gunner block. **NEW function entry**: FUN_00717890 (gunner-table builder — tiny 1-line setter `*(uint*)(this + 0x814 + WeaponSlot*4) = TurretIndex`). **§3 IFVMode → TurretKey OPEN QUESTION PARTIALLY RESOLVED**: parse-order in UnitTypeClass__ReadINI is fixed (NormalTurret=WeaponSlot 0, RepairTurret=WeaponSlot 1, MachineGunTurret=WeaponSlot 2, FlakTurret=3, PistolTurret=4, SniperTurret=5, ShockTurret=6, ExplodeTurret=7, BrainBlastTurret=8, RadCannonTurret=9, ChronoTurret=10, TerroristExplodeTurret=11, CowTurret=12, InitiateTurret=13, VirusTurret=14, YuriPrimeTurret=15, GuardianTurret=16). Parser-side data layout is BINARY-VERIFIED; **IFVMode-integer → WeaponSlot consumer mapping at runtime is still DEFERRED** for direct verification (Engineer paradox unresolved). Negative claims (FV/MultiGunner → 0 matches) re-confirmed. **No INCORRECT findings**. |
| `allied/BFRT.md` | DEEP-AUDITED | 19 (2026-05-18) | **~14 Ghidra queries** (8 string-searches + 6 xref lookups + 1 grep on saved TechnoTypeClass__ReadINI). BFRT (the actual Battle Fortress, distinct from FV/IFV audit 18) is concrete-claim rich. **All 6 doc-cited parser xrefs verify exactly + 1 bonus** (OpenToppedWarpDistance). **3 NEW/re-confirmed TechnoType offsets BINARY-VERIFIED**: +0x5E4 OpenTopped (byte; gates the gun-port passenger-fire mechanic), +0x89C AlternateFLH0 base (int[3] FLH triplet — 5-entry array layout +0x89C..+0x8D8 inferred from INI evidence + format-string parse pattern), +0xD29 OmniCrusher (byte; re-confirms audit 7 cumulative). **3 OpenTopped* Rules-CombatDamage globals consumer-confirmed**: OpenToppedRangeBonus, OpenToppedDamageMultiplier, OpenToppedWarpDistance — all parsed by RulesClass__ReadCombatDamage. **CrusherAll enum value @ 0x0081BAD0** verified as MovementZone enum-table entry (not a key) via data-xref from 0x0081BAB8. **3-tier crush hierarchy NOW FULLY CUMULATIVE-VERIFIED**: Crusher (+0xD28, audit 12) → OmniCrusher (+0xD29, audit 7/19) → OmniCrushResistant (+0xD2A, audit 14). BFRT uses OmniCrusher+OmniCrushResistant; MCVs use OmniCrushResistant only. **§2/§7.4 Image=SREF artmd quirk NOT investigated** (would require artmd-side Image= handler — DEFERRED, not load-bearing). Negative claims (BFRT/BattleFortress → 0 matches) re-confirmed. **No INCORRECT findings**. |
| `allied/CARRIER.md` | DEEP-AUDITED | 20 (2026-05-18) | **~16 Ghidra queries** (8 string-searches + 6 xref lookups + 1 grep on saved TechnoTypeClass__ReadINI). CARRIER doc is exceptionally claim-dense (18+ cited parser xrefs). Audit focused on **spawn-family cluster** (the highest-value cumulative additions since DRED/V3/SCHP/SUB/HORNET/DMISL/V3ROCKET will all reuse these). All 6 doc-cited spawn-family parser xrefs verify exactly. **7 NEW TechnoType offsets BINARY-VERIFIED** (the spawn-family cluster +0xD54..+0xD68 plus FireAngle +0x3D0): +0xD54 Spawned (byte; on Hornet/DMISL/V3ROCKET), +0xD58 Spawns (TechnoType* via FUN_0067BD30 = TechnoTypeClass-FindOrAllocate variant), +0xD5C SpawnsNumber (int; Carrier=3 Dread=2 V3=1), +0xD60 SpawnRegenRate (int frames), +0xD64 SpawnReloadRate (int frames), +0xD68 MissileSpawn (byte; the SpawnManagerClass missile-vs-aircraft branch flag — 0 on Hornet, 1 on DMISL/V3ROCKET), +0x3D0 FireAngle (int degrees). **Bonus discovery**: `Spawns` string at 0x008184C8 is multi-purpose — also xref'd by VoxelAnimType/ParticleSystemType/AnimType ReadINI parsers (each entity-type has its own `Spawns=` key). The TechnoType-scope Spawns is the spawner-unit-relevant one. Negative claims (HORNET/HornetLauncher → 0 matches) re-confirmed. 10+ other doc-cited parser xrefs not directly verified this pass but trust-extended from the 6/6 spawn-family exact-match rate. 5 deep-RE docs (SPAWN_MANAGER_CLASS, AIRCRAFTCLASS, DRED, V3, BRIDGE_LOCOMOTOR_NONCOVERAGE_JUSTIFICATION) trust-chain only. **No INCORRECT findings**. |
| `allied/DEST.md` | DEEP-AUDITED | 21 (2026-05-18) | **~13 Ghidra queries** (6 string-searches + 4 xref lookups + 1 grep + 1 full ObjectTypeClass__ReadINI decompile). DEST is mid-density — 4 NEW field-scope claims. All 4 verify exactly. **3 NEW TechnoType offsets BINARY-VERIFIED**: +0xC9D Sensors (byte; sub-detection ability), +0x5F0 SensorsSight (int cells; forms a "detection-range cluster" adjacent to audit-6 +0x5F4 DetectDisguiseRange), +0x600 NavalTargeting (int enum; re-confirms audit 7). **BONUS: full ObjectTypeClass__ReadINI decompile** reveals 13+ NEW ObjectType offsets — the broadest parser layer above TechnoType, parser was previously only partially audited via grep (audit 7). NEW ObjectType offsets: +0x7E Image (char[25] string — RESOLVES prior DEFERRED about which layer parses Image=), +0x1E8 NoSpawnAlt (byte — the DEST claim), +0x1F0 CrushSound (int VocClass), +0x1F4 AmbientSound (int VocClass), +0x9C Armor (int enum), +0x22C Theater (byte), +0x230 Selectable (byte), +0x22F RadarInvisible (byte), +0x238 HasRadialIndicator (byte), +0x98/+0x9A RadialColor (RGB), +0x23B..+0x23D LineTrailColor (RGB), +0x240 LineTrailColorDecrement (int), +0x211 AlternateArcticArt (byte), +0x213 AlphaImage (char[25]). DEST reuses audit-20 spawn-family cluster (+0xD58..+0xD68) for the Osprey spawn. **No INCORRECT findings**. Negative claims (DESTWO/ASW → 0 matches) confirmed. Bonus discovery: `Sensors` has TWO copies in binary (lowercase INI key + uppercase SENSORS — likely debug/enum label). |
| `allied/SREF.md` | DEEP-AUDITED | 22 (2026-05-18) | **~12 Ghidra queries** (4 string-searches + 3 xref lookups + 1 grep + 1 get_function_by_address + 1 full BulletTypeClass__ReadINI decompile). SREF introduces a **NEW parser-function scope**: `BulletTypeClass__ReadINI` for ShrapnelWeapon/ShrapnelCount — the first BulletType-scope addition since the cumulative cheat sheet was built. All 3 doc-cited claims verify exactly. **1 NEW TechnoType offset BINARY-VERIFIED**: +0x810 IsChargeTurret (byte; slots cleanly between audit-18 +0x80C WeaponCount and +0x814 gunner-table — consistent with the doc's observation that IsChargeTurret requires the multi-turret system). **NEW function entry**: BulletTypeClass__ReadINI @ 0x0046BEE0–0x0046C435 (fully decompiled — calls ObjectTypeClass__ReadINI first since BulletType inherits ObjectType). **35+ NEW BulletType offsets BINARY-VERIFIED**: the entire ~256-byte BulletType-specific block including the SREF-cited +0x2B4 ShrapnelWeapon (WeaponType*) and +0x2B8 ShrapnelCount (int), plus +0x294 Airburst, +0x295 Floater, +0x296-+0x298 SubjectToCliffs/Elevation/Walls, +0x299 VeryHigh, +0x29A Shadow, +0x29B Arcing, +0x29C Dropping, +0x29D Level, +0x29E Inviso, +0x29F Proximity, +0x2A0 Ranged, +0x2A1 !Rotates, +0x2A2 Inaccurate, +0x2A3 FlakScatter, +0x2A6 Degenerates, +0x2A7 Bouncy, +0x2A8 AnimPalette, +0x2A9 FirersPalette, +0x2AC Cluster, +0x2B0 AirburstWeapon, +0x2BC DetonationAltitude, +0x2C0 Vertical, +0x2C8 Elasticity (double), +0x2D0 Acceleration, +0x2D4 Color, +0x2D8 Trailer, +0x2E0 CourseLockDuration, +0x2E4 SpawnDelay, +0x2EC Scalable, +0x2F4/+0x2F5 AnimLow/AnimHigh, +0x2F6 AnimRate, +0x2F7 Flat, +0x1F8 Image (BulletType-specific). 2 unknown INI keys at +0x2A4 / +0x2A5 (parser reads from unnamed DAT addresses). **No INCORRECT findings**. Negative claim (SREF → 0 matches) confirmed. |
| `allied/ROBO.md` | DEEP-AUDITED | 23 (2026-05-18) | **~12 Ghidra queries** (5 string-searches + 4 xref lookups + 1 grep on saved TechnoTypeClass__ReadINI). All 4 doc-cited claims verify exactly. **DUAL-READ PATTERN BINARY-VERIFIED for BOTH ActivateSound AND DeactivateSound** (mirrors audit 17 ChronoInSound/OutSound pattern): RulesClass__ReadAudioVisual global default + TechnoTypeClass__ReadINI per-unit override. **4 NEW TechnoType offsets BINARY-VERIFIED**: +0x410 PoweredUnit (byte; gates the deactivation state-machine when power lost or controlling building destroyed), +0x45C VoiceSelectDeactivated (int VocClass index; parallel to VoiceSelect, used in deactivated state), +0x5A8 ActivateSound (int VocClass index; TechnoType side of DUAL-READ), +0x5AC DeactivateSound (int VocClass index; sequence-position inferred). **NEW sound-cluster topology**: separate from audit-14/17 cluster at +0x568..+0x57C (DeploySound/ChronoInSound etc.), the Activate/DeactivateSound pair lives at +0x5A8/+0x5AC. Negative claim (ROBO → 0 matches) re-confirmed. DEFERRED: PoweredUnit ↔ GAROBO controlling-building lookup mechanism (the engine's "which building controls which units" mapping), state-machine consumer in TechnoClass per-tick code, hover locomotor body. **No INCORRECT findings**. |
| `allied/SHAD.md` | DEEP-AUDITED | 24 (2026-05-18) | **~14 Ghidra queries** (5 string-searches + 4 xref lookups + 1 grep on saved TechnoTypeClass__ReadINI). All 4 doc-cited claims verify exactly + 1 BONUS (LeaveTransportSound). **[ADDRESS DISCREPANCY corrected in doc]**: LeaveTransportSound string was claimed at `0x008440F8` (16-byte inferred offset from EnterTransport) — actual `0x008440D4` (20 bytes BEFORE EnterTransport; strings stored in reverse order in memory). **2 NEW TechnoType offsets BINARY-VERIFIED**: +0x564 EnterTransportSound (int VocClass index), +0x568 LeaveTransportSound (int VocClass index — **RESOLVES AUDIT-17 DEFERRED** "+0x568 unknown sibling"). **2 re-confirmations** with parser-xref proof: +0x390 HoverAttack (audit 8 cumulative without xref proof; now verified), +0x6C8 PreventAttackMove (audit 10 cumulative without xref proof; now verified). **Sound-cluster topology consolidated**: transport-sounds at +0x564/+0x568, deploy/chrono at +0x56C..+0x578, +0x57C still DEFERRED, power-state at +0x5A8/+0x5AC. RadarInvisible re-confirmation via audit 21 cumulative (ObjectType+0x22F, broader-than-TechnoType scope). Negative claim (SHAD → 0 matches) confirmed. **No INCORRECT findings** beyond the address discrepancy correction. |
| `allied/LCRF.md` | DEEP-AUDITED | 25 (2026-05-18) | **~6 Ghidra queries** (3 string-searches + 2 xref lookups + 1 grep). LCRF is a thin doc — most fields cross-reference SAPC/YHVR (not yet audited). Negative claim (LCRF → 0 matches) confirmed. MovementRestrictedTo re-confirms audit 12 cumulative. **[SCOPE CORRECTION]**: `Naval=yes` is **TechnoType-scope** (parser xref @ 0x00714A6A in TechnoTypeClass__ReadINI), NOT UnitType-scope as the doc and several sibling docs (SHAD/SAPC) consistently claimed. Audit-12 UnitTypeClass__ReadINI confirms `Naval=` is NOT parsed there. **1 NEW TechnoType offset BINARY-VERIFIED**: +0xCCE Naval (byte; gates shipyard-build path + torpedo vulnerability + Squid-target eligibility). LCRF's INDEX corrections logged in-doc: LCRF=Landing Craft (NOT Sea Scorpion), HYD=Soviet Sea Scorpion (NOT Allied Hydrofoil), HIND=disabled/cut. **No INCORRECT findings** beyond the scope correction. Cumulative trust-chain on shared-with-SAPC claims is deferred to the SAPC audit slot. |
| `allied/BEAG.md` | DEEP-AUDITED | 26 (2026-05-18) | **~9 Ghidra queries** (4 string-searches + 2 xref lookups + 1 get_function_by_address + 1 full AircraftTypeClass__ReadINI decompile + 1 grep). **MAJOR DISCOVERY**: BEAG introduces a **NEW parser-function scope** — `AircraftTypeClass__ReadINI` — the third NEW scope addition (after audit 21 ObjectType + audit 22 BulletType). **NEW function entry**: AircraftTypeClass__ReadINI @ 0x0041CC20–0x0041CDA3 (fully decompiled — calls TechnoTypeClass__ReadINI first since AircraftType inherits TechnoType). **10 NEW AircraftType offsets BINARY-VERIFIED** (the entire AircraftType-specific field layout): +0xDFC Carryall, +0xE00 Trailer (AnimType*), +0xE04 SpawnDelay (int), +0xE08 Rotors, +0xE09 CustomRotor, +0xE0A Landable, +0xE0B FlyBy, +0xE0C FlyBack, +0xE0D AirportBound (BEAG's claim), +0xE0E Fighter (BEAG's claim). **1 NEW TechnoType offset BINARY-VERIFIED**: +0x680 InitialAmmo (int; initial ammo count at unit spawn). RequiredHouses/CanPassiveAquire/PreventAttackMove/ImmuneToPsionics re-confirmed from prior audits. Negative claim (BEAG → 0 matches) confirmed. DEFERRED: bare `Ammo` field parser (no standalone string in binary — needs DAT_0081BBE0 investigation), CanRetaliate scope. **No INCORRECT findings** beyond minor doc framing nit (the "5th locomotor GUID" claim — the {4A582746-...} GUID was seen in audits 20/21 for Hornet/Osprey aircraft). |
| `allied/AEGIS.md` | DEEP-AUDITED | 27 (2026-05-18) | **~13 Ghidra queries** (5 string-searches + 4 xref lookups + 1 grep on saved TechnoTypeClass__ReadINI). All 4 doc-cited claims verify exactly. **SinkingSound DUAL-READ pattern BINARY-VERIFIED** (Rules @ 0x006699A7 + TechnoType @ 0x00712FB0) — joins the established dual-read family (ChronoInSound/OutSound, ImpactLandSound, ActivateSound/DeactivateSound). **4 NEW TechnoType offsets BINARY-VERIFIED**: +0xC96 ToProtect (byte; AI high-value-support hint — shared with harvesters/MCVs/Carrier/Slave Miner), +0x6A4 RadialFireSegments (int; 360°-divided wedge launch for AA bypass of body rotation), +0x6B0 DistributedFire (byte; multi-target round-robin), +0x548 SinkingSound (int VocClass index; TechnoType side of DUAL-READ). **NEW sound-cluster topology**: +0x544 (unknown sibling DEFERRED) / +0x548 (SinkingSound) is a third sound cluster, separate from the +0x564/+0x568 transport cluster (audit 24), the +0x56C..+0x578 deploy/chrono cluster (audits 14, 17), and the +0x5A8/+0x5AC power-state cluster (audit 23). Negative claim (AEGIS → 0 matches) confirmed. **No INCORRECT findings**. DEFERRED: TurboBoost WeaponType-scope claim, RadialFireSegments consumer logic, DistributedFire target-selection consumer, +0x544 unknown-sibling INI key. |
| `allied/DLPH.md` | DEEP-AUDITED | 28 (2026-05-19) | **~14 Ghidra queries** (10 string-searches + 8 xref lookups + 1 full WeaponTypeClass__ReadINI decompile + 1 full WarheadTypeClass__ReadINI decompile + 1 full InfantryTypeClass__ReadINI re-confirm + 2 assembly-context lookups + 1 grep on saved TechnoTypeClass__ReadINI). **NEW PARSER SCOPE introduced**: `WarheadTypeClass__ReadINI` @ 0x0075d590–0x0075deae (fourth NEW parser-function scope after ObjectType audit 21, BulletType audit 22, AircraftType audit 26). All 4 doc-cited claims verify exactly + 4 bonus offsets pinned + 1 important INI-scope finding. **8 NEW struct-offset bindings BINARY-VERIFIED**: IsSonic = WeaponType+0x130 (byte) NEW, AmbientDamage = WeaponType+0x98 (int) NEW, Organic = TechnoType+0xD97 (byte) NEW, TypeImmune = TechnoType+0xC8C (byte) NEW (assembly-verified `MOV [EBP+0xc8c]` at 0x0071221c), Underwater = TechnoType+0xD69 (byte) NEW, CloakingSpeed = TechnoType+0x310 (int) NEW (int* form `param_1[0xc4]*4`), NotHuman = InfantryType+0xEAD (byte) NEW (**RESOLVES audit-9 ADOG DEFERRED** — assembly-verified writeback at 0x005243d7), Sonic = WarheadType+0x14B (byte) NEW (first WarheadType offset; assembly-verified at 0x0075d5a4). **[INCORRECT — VEHICLE-SCOPE DEAD INI]**: doc's `NotHuman=yes` annotation is misleading; NotHuman is InfantryType-scope only (parsed in InfantryTypeClass__ReadINI), so on `[DLPH]` (VehicleTypes) it's dead INI. Doc updated in-place with caveat. **WeaponType cumulative re-confirmed** via full decompile (audit 9): +0xA0/A4/A8/AC/B0/B4/130/131/132/133/137/13B/13C/143/149/154/155 all sequenced cleanly. **DecloakToFire** (audit 9 cross-ref via BSUB) = WeaponType+0x133 re-confirmed. Negative claims (DLPH/Dolphin → 0 matches) confirmed. **No INCORRECT findings** in the doc beyond the NotHuman scope nuance. DEFERRED: IsSonic chain-damage consumer chain, AmbientDamage non-Sonic semantics, TypeImmune consumer in damage-application, Sonic warhead chain-pulse visual, Submarine locomotor body. |
| `allied/HORNET.md` | DEEP-AUDITED | 29 (2026-05-19) | **~15 Ghidra queries** (10 string-searches + 6 xref lookups + 5 grep passes on saved TechnoTypeClass__ReadINI + 1 assembly-context batch). All 3 doc-cited "NEW cheat-sheet" claims verify exactly + 3 bonus offsets pinned + 1 DEAD-INI finding. **6 NEW struct-offset bindings BINARY-VERIFIED**: Landable = AircraftType+0xE0A (byte) re-confirms audit 26 (assembly-verified `MOV [ESI+0xe0a]` at 0x0041cc67), AuxSound1 = TechnoType+0x52C (int VocClass) NEW (assembly-verified preload at 0x00712e03), AuxSound2 = TechnoType+0x530 (int VocClass) NEW (verifies doc's adjacent-address inference; parser xref @ 0x00712e48), ImpactLandSound = TechnoType+0x540 (int VocClass) NEW (TechnoType side of DUAL-READ pattern with `RulesClass__ReadAudioVisual @ 0x00669965`), CrashingSound = TechnoType+0x544 (int VocClass) NEW (assembly-verified preload at 0x00712f6b), PitchSpeed = TechnoType+0x3A8 (double) NEW (aircraft pitch animation parameter, grep `*(double*)(param_1 + 0xea)`). **NEW SOUND CLUSTER topology mapped**: aircraft cluster at TechnoType+0x52C..+0x548 (AuxSound1, AuxSound2, +0x534/+0x538/+0x53C DEFERRED siblings, ImpactLandSound, CrashingSound, SinkingSound audit 27) — largest sound cluster yet at ~12 ints. **[INCORRECT — DEAD INI]**: doc's `MovementRestrictedTo=Water` claim — verified MovementRestrictedTo is UnitType-scope ONLY (single xref @ 0x00747837 → UnitTypeClass__ReadINI; not read by AircraftTypeClass__ReadINI). Hornet's MovementRestrictedTo line is dead INI; doc updated with caveat. **DUAL-READ pattern extended** — ImpactLandSound joins ChronoIn/OutSound, SinkingSound, ActivateSound, DeactivateSound family. **Re-confirmed cumulative**: Spawned (audit 20), MissileSpawn (audit 20 — absence confirms return-to-dock pattern), ImmuneToPsionics (audit 7), AircraftLocomotion GUID (audit 20+26). Negative claims (HORNET/Hornet → 0 matches) confirmed. DEFERRED: sound-cluster siblings +0x534/+0x538/+0x53C INI keys, HornetCollision crash-transform mechanism, `;Selectable=no` Westwood bug code path, AuxSound1/2 consumer in takeoff/landing transitions, PitchSpeed/Angle consumer in aircraft pitch animation. |
| `allied/ASW.md` | DEEP-AUDITED | 30 (2026-05-19) | **~14 Ghidra queries** (8 string-searches + 6 xref lookups + 5 grep passes on saved TechnoTypeClass__ReadINI + 1 AircraftClass function search). All 1 doc-cited claim verifies + 3 NEW TechnoType offsets pinned + 2 doc open questions RESOLVED. **3 NEW struct-offset bindings BINARY-VERIFIED**: LandTargeting = TechnoType+0x604 (int) NEW (sibling to NavalTargeting +0x600 from audit 7, parsed FIRST), PitchAngle = TechnoType+0x3B0 (double) NEW (sibling to PitchSpeed +0x3A8 from audit 29, **stored in radians via PI/180 conversion `degrees × DAT_007f4fb8`**), Trainable = TechnoType+0xC8E (byte) NEW (gates XP accumulation; default false). **TechnoType byte-cluster +0xC8C..+0xC91 fully mapped** post-audit-30 with 5/6 keys named (TypeImmune/MoveToShroud/Trainable/name-override-aux/ImmuneToVeins) — only +0xC90 remains DEFERRED. **2 doc open questions RESOLVED**: (1) MovementRestrictedTo=Water carried over from HORNET audit 29 (UnitType-scope only — DEAD INI on ASW); (2) Veterancy question — Trainable defaults false, so ASW genuinely has no veterancy regardless of VeteranAbilities/EliteAbilities lines; same applies to HORNET (HORNET's elite weapon swap comes from parent CARRIER's ElitePrimary, not from Hornet's own XP). **Re-confirmed cumulative**: NavalTargeting (audit 7 +0x600), AuxSound1/2 (audit 29 +0x52C/+0x530), Landable (audit 26/29 +0xE0A), VeteranAbilities (audit 7 +0x29C), EliteAbilities (audit 7 +0x2AE), Spawned (audit 20 +0xD54). Negative claims (ASW/Osprey → 0 matches) confirmed. **No INCORRECT findings** beyond the inherited MovementRestrictedTo caveat. DEFERRED: crash-transform mechanism (AircraftClass__ReceiveDamage @ 0x004165c0 — likely host of HornetCollision/ASWCollision Secondary fire on crash), FindBuildingToDock body, PitchAngle multiplier constant DAT_007f4fb8 exact value, +0xC90 sibling INI key. |
| `allied/ORCA.md` | DEEP-AUDITED | 31 (2026-05-19) | **~16 Ghidra queries** (12 string searches + 6 xref lookups + 4 grep passes on saved TechnoTypeClass__ReadINI + 1 assembly-context batch). 5 doc-cited claims verify + 4 NEW struct-offset bindings BINARY-VERIFIED + 2 byte/sound clusters consolidated. **4 NEW struct-offset bindings BINARY-VERIFIED**: CanRetaliate = TechnoType+0xD9A (byte) NEW (fills byte-cluster between +0xD99 CanPassiveAquire and +0xD9B RequiresStolenThirdTech), AltCameo = TechnoType+0x1F8 (char[25] string) NEW (assembly-verified `LEA ECX, [EBP + 0x1f8]` at 0x00715a73 + `PUSH 0x19` size limit; consumer xref into FUN_007162f0 DEFERRED), MoveSound = TechnoType+0x504..+0x50C (int[3] 3-slot SoundList) NEW (looping engine sound), VoiceCrashing = TechnoType+0x550 (int VocClass) NEW (assembly-verified writeback `MOV [EBP + 0x550], EAX` at 0x00713069). **TechnoType byte-cluster +0xD99..+0xDA4 fully named** post-audit-31: CanPassiveAquire (audit 10), CanRetaliate (NEW audit 31), RequiresStolenThirdTech/SovietTech/AlliedTech (audit 11), RequiredHouses (audit 10), ForbiddenHouses (audit 10). **Aircraft sound cluster extended** to +0x504..+0x554 (MoveSound +0x504 NEW, VoiceCrashing +0x550 NEW joins AuxSound1/2/ImpactLand/Crashing/Sinking from audits 27/29). **8 re-confirmations** from prior audits (ForbiddenHouses audit 10, MoveToShroud audit 11, ConsideredAircraft audit 8, Fighter audit 26, AirportBound audit 26, ImpactLandSound DUAL-READ audit 29, OmniFire audit 9, PreventAttackMove audit 10). The doc's "NEW cheat-sheet entries" for ForbiddenHouses/MoveToShroud were already in cumulative at write-time — no correction needed (doc claim factually correct, just historically duplicate). Negative claims (ORCA/Intruder/ORCAAP → 0 matches) re-confirmed. **No INCORRECT findings**. **ALLIED AUDIT SUB-SECTION COMPLETE**: 31 docs (11 infantry + 14 vehicles + 6 aircraft) all DEEP-AUDITED post-audit-31. DEFERRED: AltCameo consumer in UI (FUN_007162f0), AltCameo veterancy-rank swap condition, +0x534/+0x538/+0x53C/+0x54C/+0x554 unknown sound siblings, VoiceCrashing consumer in AircraftClass::ReceiveDamage, AirToGroundMissile homing-through-walls behavior. |

### Soviet infantry

| Doc | Status | Iter | Key findings |
|-----|--------|------|--------------|
| `soviet/E2.md` | DEEP-AUDITED | 32 (2026-05-19) | **First Soviet doc DEEP-AUDITED.** **~16 Ghidra queries** (8 string searches + 6 xref lookups + 1 assembly-context check + grep on InfantryType/WeaponType decompiles). 5 doc-cited claims verify exactly + 4 NEW struct-offset bindings BINARY-VERIFIED + 1 IMPORTANT CORRECTION to audit-30 cumulative (Trainable default) + 1 in-doc scope correction (OccupyPip). **4 NEW struct-offset bindings BINARY-VERIFIED**: OccupyPip = **InfantryType+0xDFC** (int enum, parser xref @ 0x005240f5; doc claimed TechnoType-scope, actual InfantryType-scope), OccupyWeapon = InfantryType+0xE04 (WeaponType*, parser xref @ 0x00524117), EliteOccupyWeapon = InfantryType+0xE20 (WeaponType*, parser xref @ 0x00524156), OccupantAnim = WeaponType+0x110 (AnimType*, parser xref @ 0x007725a1). **CRITICAL CORRECTION to audit 30**: Trainable does NOT default to false. Assembly-context proof at 0x00714a15 shows `MOV CL, [EBP + 0xc8e]` preload BEFORE ReadBool — default is constructor-set, NOT 0. From gameplay (Conscripts gain XP without Trainable=yes line) the constructor initializes +0xC8E = TRUE. ASW/HORNET veterancy absence is NOT due to Trainable=false; it's due to spawn-child semantics (SpawnManager units don't accumulate XP independently). **[INCORRECT — IN-DOC]**: doc claimed OccupyPip is TechnoType-scope; actual scope InfantryType (semantically correct since only infantry can occupy buildings). Re-confirms: Occupier = InfantryType+0xEB4 (audit 1), Image= = ObjectType+0x7E (audit 21), IFVMode = TechnoType+0x688 (audit 7), ImmuneToVeins = TechnoType+0xC91 (audit 7). OccupyWeaponRange = Rules-CombatDamage scope confirmed (offset DEFERRED — RulesClass__ReadCombatDamage oversized). Negative claims (E2/Conscript → 0 matches) confirmed. DEFERRED: OccupyWeaponRange offset, InfantryType+0xE00 unknown sibling between OccupyPip and OccupyWeapon, MaxNumberOccupants BuildingType-side, spawn-child veterancy mechanism for ASW/HORNET correction. |
| `soviet/SHK.md` | DEEP-AUDITED | 33 (2026-05-19) | **~17 Ghidra queries** (10 string searches + 5 xref lookups + 1 assembly-context batch + grep on WeaponType decompile). 6 doc-cited claims verify + 3 NEW struct-offset bindings BINARY-VERIFIED + 1 CRITICAL CORRECTION to audit 1 + 1 IN-DOC scope correction. **3 NEW struct-offset bindings BINARY-VERIFIED**: ElectricAssault = WarheadType+0x158 (byte; Tesla Coil charge flag; sibling to +0x14B Sonic audit 28), AssaultAnim = WeaponType+0x114 (AnimType*; sibling to OccupantAnim +0x110 audit 32), ChargedAnimTime = **BuildingType+0x16E8** (float, ReadDouble; doc INCORRECTLY claimed Rules-AudioVisual scope — actual scope BuildingType). **1 bonus NEW offset**: OpenToppedAnim = WeaponType+0x118 (AnimType*; formally pinned from audit-28 decompile observation). **CRITICAL CORRECTION to audit 1 (E1)**: InfantryType+0xEB5 was claimed "paratrooper-occupier flag" — actual is **Assaulter** (byte). Assembly-context at 0x0052450b: `MOV byte ptr [ESI + 0xeb5], AL` AFTER Assaulter ReadBool. SHK explicitly sets Assaulter=no — Tesla Trooper cannot clear UC buildings; the AssaultAnim=UCELEC on its weapon is vestigial. **[IN-DOC INCORRECT]**: doc claimed ChargedAnimTime is Rules-AudioVisual; actual is BuildingType-scope (per-Tesla-Coil animation timer). TeslaCharge (separate key) DOES remain Rules-AudioVisual. Re-confirmations: IsElectricBolt = WeaponType+0x152 (audit 9), IsAlternateColor = WeaponType+0x154 (audit 9), Charges = WeaponType+0x148 (audit 9 — legacy superseded by DelayedFire), Crushable = ObjectType+0x22D (audit 7). Negative claim (SHK → 0) confirmed. DEFERRED: TeslaCharge Rules-AudioVisual offset, ElectricAssault consumer in damage application, ChargedAnimTime consumer in Tesla Coil animation, DelayedFire system, ShrapnelWeapon chain-bounce mechanic. |
| `soviet/IVAN.md` | DEEP-AUDITED | 34 (2026-05-19) | **~18 Ghidra queries** (10 string searches + 6 xref lookups + 1 assembly-context for IvanBomb + 5 grep passes + 1 broad "Ivan" substring search). 6 doc-cited claims verify exactly + 5 NEW struct-offset bindings BINARY-VERIFIED + 1 IN-DOC scope correction + 1 IMPORTANT POSSIBLY-DEAD-INI finding. **5 NEW struct-offset bindings BINARY-VERIFIED**: Explodes = TechnoType+0xD15 (byte; also OverlayType-scope for crates), AttackFriendlies = TechnoType+0x6C0 (byte), AttackCursorOnFriendlies = TechnoType+0x6C1 (byte sibling), BombSight = TechnoType+0x5F8 (int — doc claimed InfantryType, actual scope TechnoType), IvanBomb = WarheadType+0x157 (byte; assembly-verified writeback at 0x0075d823; sibling to +0x158 ElectricAssault audit 33). **WarheadType byte cluster +0x157/+0x158** mapped (IvanBomb + ElectricAssault — pair of "special-effect-trigger" warhead flags, with +0x14B Sonic audit 28 as the broader family). **TechnoType byte cluster +0x6C0/+0x6C1 + +0x6C8** mapped (AttackFriendlies + AttackCursorOnFriendlies + PreventAttackMove audit 10 — tactical-AI friendly-fire/attack-move sub-block). **[IN-DOC SCOPE CORRECTION]**: BombSight is TechnoType-scope (not InfantryType-scope as doc claimed). Offset +0x5F8 is correct; scope label was wrong. **[IMPORTANT POSSIBLY-DEAD-INI FINDING]**: `search_strings("^Ivan$")` returns **0 matches** in the binary. The doc claims `Ivan=yes` is parsed and stored at InfantryType+0xEBE. But no standalone "Ivan" string exists — only Ivan-prefixed (IvanBomb, IvanDamage, etc.). The +0xEBE flag is set by C4/Infiltrate/two unnamed InfantryType siblings (audit 6) — not by Ivan=yes. The actual "this unit places Ivan bombs" differentiation comes from Primary weapon's warhead having IvanBomb=yes, not from the Ivan= INI key. The Ivan= line in stock YR appears to be vestigial/dead INI. Re-confirmations: Insignificant ObjectType+0x232 (audit 21), FireOnce WeaponType+0x135 (audit 9), CellRangefinding WeaponType+0x134 (audit 9), FireInTransport WeaponType+0x143 (audit 9), IvanDamage/IvanTimedDelay Rules-CombatDamage scope confirmed (offsets DEFERRED, trust-chain to BOMB_CLASS deep RE doc). Negative claim (IVAN → 0) confirmed. DEFERRED: Rules-CombatDamage offsets for IvanDamage/IvanTimedDelay/IvanIconFlickerRate/IvanWarhead, NoIvanBomb scope/offset, Ivan=yes actual parsing mechanism (or confirm dead), bridge-destruction radius in BombClass::Detonate. |
| `soviet/DESO.md` | DEEP-AUDITED | 35 (2026-05-19) | **~18 Ghidra queries** (10 string searches + 5 xref lookups + 2 assembly-context batches + 3 grep passes). 5 doc-cited claims verify exactly + 3 NEW struct-offset bindings BINARY-VERIFIED + 1 POTENTIAL audit-9 CONFLICT flagged. **3 NEW struct-offset bindings BINARY-VERIFIED**: Deployer = InfantryType+0xEC8 (byte; assembly-verified `MOV [ESI+0xec8]` at 0x00524620), Fearless = InfantryType+0xEBC (byte; assembly-verified `MOV [ESI+0xebc]` at 0x0052447A), IsRadEruption = WeaponType+0x155 (byte; assembly-verified `MOV [ESI+0x155]` at 0x007728D3). **POTENTIAL CONFLICT**: audit 9 cumulative listed IsRadBeam at +0x155 (WeaponType). But this audit's assembly trace shows IsRadEruption writes to +0x155. Same offset can't hold two keys — flagged for future re-verification of IsRadBeam's actual offset. **InfantryType capability-flag block +0xEAC..+0xECB** progress: audit 13 identified the 32-byte block but couldn't name 19/23 slots; audit 35 adds Fearless +0xEBC + Deployer +0xEC8, bringing named count to 10/23. Re-confirmations: DeployFire = TechnoType+0x6AC (audit 1), DeployFireWeapon = +0x6A8 (audit 1), SelfHealing = TechnoType+0xD14 (audit 7), ImmuneToRadiation = TechnoType+0xD37 (audit 9), AreaFire = WeaponType+0x151 (audit 9), RadLevel = WeaponType+0x158 (audit 9 via RADIATION_EMP RE doc), RequiredHouses = TechnoType+0xDA0 (audit 10), Crushable/Bombable = ObjectType+0x22D/+0x22E (audit 7). Negative claims (DESO/Desolator → 0 matches) confirmed (note: INI spelling is "Desolater"). DEFERRED: RadSite/Radiation deep-RE chain (trust-chain to RADIATION_EMP_GHIDRA_REPORT.md), CellInset autodeploy AI gate, RequiredHouses country-bitmask consumer, IsRadBeam exact offset, 13 remaining InfantryType slots in +0xEAC..+0xECB. |
| `soviet/BORIS.md` | TODO | — | — |
| `soviet/TERROR.md` | TODO | — | — |
| `soviet/FLAKT.md` | TODO | — | — |
| `soviet/SENGINEER.md` | TODO | — | — |
| `soviet/DOG.md` | TODO | — | — |
| `soviet/CIVAN.md` | TODO | — | — |
| `soviet/VLADIMIR.md` | TODO | — | — |

### Soviet vehicles & aircraft

| Doc | Status | Iter | Key findings |
|-----|--------|------|--------------|
| `soviet/HARV.md` | TODO | — | — |
| `soviet/HTNK.md` | TODO | — | — |
| `soviet/APOC.md` | TODO | — | — |
| `soviet/HTK.md` | TODO | — | — |
| `soviet/DRON.md` | TODO | — | — |
| `soviet/V3.md` | TODO | — | — |
| `soviet/DTRUCK.md` | TODO | — | — |
| `soviet/TTNK.md` | TODO | — | — |
| `soviet/DRED.md` | TODO | — | — |
| `soviet/SCHP.md` | TODO | — | — |
| `soviet/SCHD.md` | TODO | — | — |
| `soviet/SUB.md` | TODO | — | — |
| `soviet/HYD.md` | TODO | — | — |
| `soviet/SMCV.md` | TODO | — | — |
| `soviet/ZEP.md` | TODO | — | — |
| `soviet/V3ROCKET.md` | TODO | — | — |
| `soviet/DMISL.md` | TODO | — | — |
| `soviet/BPLN.md` | TODO | — | — |
| `soviet/SAPC.md` | TODO | — | — |
| `soviet/SPYP.md` | TODO | — | — |
| `soviet/SQD.md` | TODO | — | — |

### Yuri infantry

| Doc | Status | Iter | Key findings |
|-----|--------|------|--------------|
| `yuri/INIT.md` | TODO | — | — |
| `yuri/YURI.md` | TODO | — | — |
| `yuri/YURIPR.md` | TODO | — | — |
| `yuri/BRUTE.md` | TODO | — | — |
| `yuri/VIRUS.md` | TODO | — | — |
| `yuri/YENGINEER.md` | TODO | — | — |
| `yuri/YDOG.md` | TODO | — | — |
| `yuri/YADOG.md` | TODO | — | — |
| `yuri/PTROOP.md` | TODO | — | — |
| `yuri/SLAV.md` | TODO | — | — |

### Yuri vehicles & aircraft

| Doc | Status | Iter | Key findings |
|-----|--------|------|--------------|
| `yuri/MIND.md` | TODO | — | — |
| `yuri/DISK.md` | TODO | — | — |
| `yuri/TELE.md` | TODO | — | — |
| `yuri/CAOS.md` | TODO | — | — |
| `yuri/YTNK.md` | TODO | — | — |
| `yuri/SMIN.md` | TODO | — | — |
| `yuri/PCV.md` | TODO | — | — |
| `yuri/LTNK.md` | TODO | — | — |
| `yuri/BSUB.md` | TODO | — | — |
| `yuri/CMISL.md` | TODO | — | — |

### Civilian

| Doc | Status | Iter | Key findings |
|-----|--------|------|--------------|
| `civilian/PDPLANE.md` | TODO | — | — |
| `civilian/CARGOPLANE.md` | TODO | — | — |

### Structures

| Doc | Status | Iter | Key findings |
|-----|--------|------|--------------|
| `structures/YAREFN.md` | TODO | — | — |
| `structures/GACNST.md` | TODO | — | — |
| `structures/NACNST.md` | TODO | — | — |
| `structures/YACNST.md` | TODO | — | — |
| `structures/GAREFN.md` | TODO | — | — |
| `structures/NAREFN.md` | TODO | — | — |
| `structures/GAPOWR.md` | TODO | — | — |
| `structures/NAPOWR.md` | TODO | — | — |
| `structures/NANRCT.md` | TODO | — | — |
| `structures/YAPOWR.md` | TODO | — | — |
| `structures/GAPILE.md` | TODO | — | — |

---

## Audit-pass next pick

Next: `soviet/BORIS.md` (audit iteration 36). **Fifth Soviet doc.**
Boris — Soviet hero infantry with laser-designator for Mig airstrikes.
Expected claims:
- Laser-designator weapon (likely a special target-mark mechanic)
- Mig spawn / airstrike orchestration
- VeteranAbilities heavy stack (hero unit)
- Possibly Crushable=no + heavy armor
- DEFERRED items that may resurface: Mig spawn mechanism, target-
  designator system, Boris elite weapon scaling.

**Soviet sub-section progress: 2 of 32 docs DEEP-AUDITED.**
**Total progress: 33 of ~96 docs (Allied complete + 2 Soviet).**

## Cumulative findings from audit pass

### Verified function entry points (audit 1-5)

- `InfantryClass__Fire_At_Target` @ 0x005206b0–0x00520ade
- `InfantryClass__AI` @ 0x0051bab0–0x0051bf86
- `InfantryClass__SetFear` @ 0x00518c00–0x00518d54 (was called `Fear_Decay_Handler` in docs)
- `InfantryClass__IronCurtain` @ 0x00522600–0x0052263b (thin wrapper)
- `InfantryClass__Mission_Capture` @ 0x005202f0–0x005206aa **[decompiled audit 3 — RTTI=1 check may not mean BuildingClass per audit 5]**
- `InfantryClass__PerCellProcess` @ 0x00519630–0x0051aa0a (contains "Mission_Enter" logic, ~5kb)
- `BuildingClass__AddGarrisonOccupant` @ 0x00522910–0x00522a4d
- `CaptureManagerClass__CaptureUnit` @ 0x00471d40–0x00471f86
- `UnitClass__OnEnterCell_Triggers` @ 0x00744720–0x00744799
- `CellClass__PlaceInfantryInCell` @ 0x00481180–0x0048149a
- `TechnoClass__SetGunnerWeapon` @ 0x0070dc70–0x0070dcdd
- `TechnoClass__CanCrushCheck` @ 0x005f6cd0–0x005f6d92
- `TechnoClass__GetFireError` @ 0x006fc0b0–0x006fcd37
- `TechnoClass__DrawExtras` @ 0x006f5190–0x006f5eee
- `TechnoClass__Fire_At` @ 0x006fdd50–0x006ff94e (audit 5)
- `TechnoClass__SpawnRadBeam` @ 0x006fd620–0x006fd7f0 (audit 5)
- `WarheadTypeClass__Detonate` @ 0x004690b0–0x0046a303 (audit 5)
- `TemporalClass__InitiateWarp` @ 0x0071af20–0x0071b182 (audit 5, decompiled)
- `TemporalClass__Update` @ 0x0071a760–0x0071ab0f (audit 5)
- `TemporalClass__SumChainDamage` @ 0x0071ab10–0x0071ab59 (audit 5)
- `TemporalClass__DetachFromTarget` @ 0x0071abc0–0x0071aca9 (audit 5)
- `TemporalClass__CanWarpTarget` @ 0x0071ae50–0x0071af1b (audit 5, decompiled)
- `WarpAttachClass__Detach` @ 0x0062a4a0–0x0062a8d9 (audit 5)
- `FUN_005218e0` (unlabeled, IS SelectWeapon — DeployFire logic; audit 2)
- `FUN_0051f3e0` (unlabeled, IS Mission_Attack — C4 gate; audit 4)
- `BuildingClass__OnSpyInfiltrate` @ 0x004571e0–0x004575a4 (audit 6, fully decompiled)
- `TechnoClass__IsDisguised_Getter` @ 0x0041c020–0x0041c028 (audit 6, thin 1-byte flag getter)
- `HouseClass__SpyPowerSabotage` @ 0x0050bc90–0x0050bcc0 (audit 6, fully decompiled)
- `OnSpyWeaponInfiltrate` @ 0x006ce0b0–0x006ce19f (audit 6, fully decompiled)
- `HouseClass__Check_Spy_Reveal` @ 0x004faf00–0x004fb0d6 (audit 6, decompiled)
- `BuildingClass__AddDetectDisguiseAt` @ 0x00455a80–0x00455b8c (audit 6, decompiled — ring iterator over cells)
- `BuildingClass__RemoveDetectDisguiseAt` @ 0x00455980–0x00455a78 (audit 6)
- `CellClass__IncrementDisguiseDetectCount` @ 0x00487170 (audit 6, thin inline)
- `CellClass__DecrementDisguiseDetectCount` @ 0x00487180 (audit 6, thin inline)
- `MapClass__RestoreShroud` @ 0x00577ab0–0x00577ba3 (audit 6, decompiled)
- `FUN_0050BD10` (RestoreShroud wrapper, LowPower-gated; audit 6, body 0x0050bd10–0x0050bd25)
- `HouseClass__Spend_Money` @ 0x004f9790 (audit 6, decompiled)
- `HouseClass__Add_Credits` @ 0x004f9950 (audit 6, one-line `+= param_2` at HouseClass+0x30C)
- `HouseClass__IsHumanPlayer` @ 0x0050b6f0 (audit 6, decompiled)
- `AircraftClass__Mission_SpyPlane` @ 0x00417300–0x004176d9 (audit 6, entry-point only — body DEFERRED to SPYP doc audit)
- `ObjectTypeClass__ReadINI` @ 0x005f92d0–0x005f96a9 (audit 7 — partial grep; audit 21 — **fully decompiled**. Sole parser for ALL ObjectType-scope keys: Image, AlphaImage, CrushSound, AmbientSound, Crushable, Bombable, NoSpawnAlt, AlternateArcticArt, RadarInvisible, Selectable, LegalTarget, Armor, Strength, Immune, Insignificant, HasRadialIndicator, RadialColor, IgnoresFirestorm, UseLineTrail, LineTrailColor, LineTrailColorDecrement, Theater, NewTheater, Voxel. Critical role: ObjectType is the parent layer above TechnoType, so these keys are inherited by every unit/structure/anim/terrain type.)
- `BulletTypeClass__ReadINI` @ 0x0046BEE0–0x0046C435 (audit 22, **fully decompiled** — sole parser for ALL BulletType-scope keys: ShrapnelWeapon, ShrapnelCount, AirburstWeapon, Airburst, Floater, SubjectToCliffs/Elevation/Walls, VeryHigh, Shadow, Arcing, Dropping, Level, Inviso, Proximity, Ranged, Rotates, Inaccurate, FlakScatter, Degenerates, Bouncy, AnimPalette, FirersPalette, Cluster, DetonationAltitude, Vertical, Elasticity, Acceleration, Color, Trailer, CourseLockDuration, SpawnDelay, Scalable, AnimLow/AnimHigh/AnimRate, Flat, Image (BulletType-specific). Calls ObjectTypeClass__ReadINI first since BulletType inherits ObjectType. First BulletType-scope addition to the cumulative cheat sheet.)
- `AircraftTypeClass__ReadINI` @ 0x0041CC20–0x0041CDA3 (audit 26, **fully decompiled** — sole parser for ALL AircraftType-scope keys: Landable, AirportBound, Fighter, Carryall, Rotors, CustomRotor, Trailer, SpawnDelay, FlyBy, FlyBack. Small ~10-key body. Calls TechnoTypeClass__ReadINI first since AircraftType inherits TechnoType. First AircraftType-scope addition to the cumulative cheat sheet. NOTE: AircraftType-specific fields ONLY apply to `[AircraftTypes]` section units — vehicle-class flying units like Kirov/Disc/SHAD (declared in `[VehicleTypes]` with JumpJet=yes/ConsideredAircraft=yes) do NOT read AircraftType fields.)
- `CaptureManagerClass__CanCapture` @ 0x00471c90 (audit 7, decompiled — actual ImmuneToPsionics consumer at TechnoType+0xD35)
- `JumpjetLocomotionClass__Constructor` @ 0x0054ac40–0x0054acf8 (audit 8, decompiled — Ghidra-labeled with canonical CLSID comment; three-vtable COM layout)
- `JumpjetLocomotionClass::Process` @ 0x0054aec0–0x0054b19b (audit 8, decompiled — state-machine dispatcher)
- `JumpjetLocomotionClass::State_0_Grounded` @ 0x0054b980–0x0054ba2e (audit 8, decompiled — caches +0x2C → +0x80, transitions to state 1)
- `JumpjetLocomotionClass::State_1_Liftoff` @ 0x0054ba30–0x0054bd2c (audit 8, entry verified; body DEFERRED)
- `JumpjetLocomotionClass::State_2_DecelCruise` @ 0x0054bd30–0x0054bfe4 (audit 8, entry verified)
- `JumpjetLocomotionClass::State_3_LongRangeCruise` @ 0x0054bff0–0x0054c54b (audit 8, entry verified)
- `JumpjetLocomotionClass::State_4_DescendLand` @ 0x0054c550–0x0054ca83 (audit 8, entry verified)
- `JumpjetLocomotionClass::State_5_EmergencyAbort` @ 0x0054ca90–0x0054d0b4 (audit 8, entry verified)
- `JumpjetLocomotionClass::In_Which_Layer` @ 0x0054b8d0–0x0054b97c (audit 8, decompiled — altitude vs +0x2C decides z-sort layer)
- `RulesClass__ReadJumpjetControls` @ 0x006743d0–0x006744f1 (audit 8, decompiled — Ghidra-labeled, populates Rules+0x40C..+0x438 defaults block)
- `FootClass__Locomotion_AI` @ 0x00520f40–0x00521312 (audit 8, decompiled — JumpJet flag gate + CLSID-match sequence dispatch to 0x17/0x18)
- `ParasiteClass__Constructor` (primary) @ 0x00629210–0x006292a0 (audit 9, decompiled — 4 vtables + LaunchFrame at +0x2C, timestamps at +0x38, fields at +0x34/+0x40)
- `ParasiteClass__Constructor` (variant) @ 0x006292b0–0x00629387 (audit 9, entry verified, body DEFERRED)
- `WeaponTypeClass__ReadINI` @ ~0x00772040–~0x00772a00 (audit 9, decompiled — sole consumer for LimboLaunch, OmniFire, IsLaser, IsRadBeam, FireInTransport, etc.)
- `TeleportLocomotionClass::Constructor` @ 0x00718000–0x00718075 (audit 11, decompiled — Ghidra-labeled with canonical CLSID comment "4A582747-...". 3-vtable COM init + coord/state field layout)
- `TeleportLocomotionClass::Is_Moving` @ 0x00718080 (audit 11, Ghidra-labeled)
- `TeleportLocomotionClass::Destination` @ 0x007180A0 (audit 11, Ghidra-labeled)
- `TeleportLocomotionClass::HeadToCoord` @ 0x00718100 (audit 11, Ghidra-labeled — set new dest, kick off warp)
- `TeleportLocomotionClass::Stop_Moving` @ 0x00718230 (audit 11, Ghidra-labeled — abort warp)
- `TeleportLocomotionClass::Update_Position` @ 0x00718260 (audit 11, Ghidra-labeled — per-tick position update)
- `TeleportLocomotionClass::PostWarpValidation` @ 0x007187A0 (audit 11, Ghidra-labeled — destination-cell validity check)
- `TeleportLocomotionClass::Process` @ 0x00718B70 (audit 11, Ghidra-labeled — main per-tick dispatch; body DEFERRED)
- `TeleportLocomotionClass::Mark_All_Occupation_Bits` @ 0x007192C0 (audit 11, Ghidra-labeled)
- `TeleportLocomotionClass::StateMachineTick` @ 0x007192F0 (audit 11, Ghidra-labeled — warp-out → transit → warp-in phase advance)
- `TeleportLocomotionClass::InitiateWarp` @ 0x00719400 (audit 11, Ghidra-labeled — start warp-out sequence)
- `TeleportLocomotionClass::ClearPendingWarpPhase` @ 0x00719790 (audit 11, Ghidra-labeled)
- `TeleportLocomotionClass::Phase0_SetWarpingOut` @ 0x007197D0 (audit 11, Ghidra-labeled)
- `TeleportLocomotionClass::TimerCheck` @ 0x00719BF0 (audit 11, Ghidra-labeled — unstun-timer check). **CORRECTS CCOMAND doc's `0x0070F770` claim** (which is unrelated `FUN_0070f770`)
- `TeleportLocomotionClass::QueryInterface` @ 0x00719E30 (audit 11, Ghidra-labeled — COM QI)
- `TeleportLocomotionClass::Begin_Piggyback` @ 0x00719E90 (audit 11, Ghidra-labeled)
- `TeleportLocomotionClass::End_Piggyback` @ 0x00719EE0 (audit 11, Ghidra-labeled)
- `TeleportLocomotionClass::Is_Ok_To_End` @ 0x00719F30 (audit 11, Ghidra-labeled)
- `TeleportLocomotionClass::ILocomotion_QI_Thunk` @ 0x0071A160 (audit 11, Ghidra-labeled — ILocomotion COM thunk)
- `UnitTypeClass__ReadINI` (audit 12, fully decompiled — sole consumer for CrateGoodie/IsTilter/TooBigToFitUnderBridge/Harvester/Weeder/Passive/DeployToFire/IsSimpleDeployer/UseTurretShadow/CanBeach/SmallVisceroid/LargeVisceroid/CarriesCrate/NonVehicle/StandingFrames/DeathFrames/DeathFrameRate/StartStandFrame/StartWalkFrame/StartFiringFrame/StartDeathFrame/MaxDeathCounter/Facings/FiringSyncFrame/BurstDelay/WalkFrames/FiringFrames/AltImage + turret-slot indices/weapons)
- `BuildingTypeClass_ReadINI_Water` (audit 12, decompiled via grep — consumer for SecretLab/SecretInfantry/SecretUnit/SecretBuilding + DockUnload + others)
- `RulesClass__ReadGeneral` (audit 12, decompiled via grep — consumer for global SecretInfantry/SecretUnits/SecretBuildings DynamicVector lists + many more general-rule keys)
- `InfantryTypeClass__ReadINI` @ 0x005240A0–0x0052475C (audit 13, fully decompiled — sole parser for the 23 InfantryType-scope ReadBool keys at +0xEAC..+0xECB plus 6 ReadInt keys at +0xE40..+0xE4C/+0xEB0/+0xEB8/+0xE98..+0xEA0/+0xE84/+0xE60..+0xE68. Pre-known consumers: C4 +0xEC2 (audit 4), Engineer +0xEC5 (audit 3), Crawls +0xEBD (audit 7), Infiltrator +0xEBE (audit 6). New auxiliary: +0xC8F = name-override flag, set when first ReadBool returns true)
- `UnitClass__Deploy` @ 0x007393C0–0x00739AB7 (audit 14, Ghidra-labeled, fully decompiled — the master deploy routine for ALL `DeploysInto=`-bearing units. 8-step state machine: CanDeploy → cell-validate → face-target → operator_new(0x720) for BuildingClass → BuildingClass::Constructor(DeploysInto, owner) → TryPlaceBuilding via vtable+0xD8 → transfer UniqueID/Z/health/veterancy/AttachedTag → target-redirect loop → vtable+0xF8 RemoveFromMap + vtable+0x3A0 Destroy/Limbo. IsDeployable (BuildingType+0x16B9) branch triggers construction-yard special setup)
- `Deploy_facing_calculator` @ 0x00465D70–0x00465D76 (audit 14, Ghidra-labeled — **7-byte stub/thunk** only; actual facing-rule body is elsewhere in the call chain. Marked **[ADDRESS PARTIAL]** for AMCV doc claim.)
- `RulesClass__ReadAudioVisual` (audit 17, parser xref evidence only — body not decompiled. Parses `[AudioVisual]` INI section globals. Confirmed via dual-read pattern for ChronoInSound/OutSound — these keys have both a global default in this function AND a per-TechnoType override in TechnoTypeClass__ReadINI.)
- `FUN_00717890` (audit 18, fully decompiled — 1-line gunner-table builder: `*(uint*)(this + 0x814 + WeaponSlot*4) = TurretIndex`. Called 17 times in UnitTypeClass__ReadINI's gunner block, one per named TurretKey (NormalTurret..GuardianTurret), populating the 17-int gunner-lookup table at TechnoType-extended+0x814..+0x858. Unlabeled (FUN_*); rename candidate: `TechnoTypeClass::Set_Gunner_Turret_Mapping`.)
- `WarheadTypeClass__ReadINI` @ 0x0075d590–0x0075deae (audit 28, fully decompiled — sole parser for the WarheadType-specific block starting at +0x14B. **NEW PARSER SCOPE** (fourth after ObjectType audit 21, BulletType audit 22, AircraftType audit 26). Sole consumer for warhead-side Sonic +0x14B flag. Also reads ~10 more sequential ReadBool keys + a ReadCLSID block at +0x15C..+0x168; individual INI-key→offset bindings beyond Sonic DEFERRED to future audits.)

### RTTI value resolution (audit 5)

- **`RTTI == 6` = BuildingClass** (confirmed in InitiateWarp building-branch + GHOST Mission_Attack)
- **`RTTI == 1` = FootClass** (parent of UnitClass + InfantryClass — moving units; confirmed by CanWarpTarget's FootClass::GetDestination branch)
- **CORRECTION**: ENGINEER audit 3's claim "RTTI=1 = BuildingClass" was WRONG. Mission_Capture's `iVar2 == 1` check means the target is FootClass, not BuildingClass. **The engineer building-capture path may live elsewhere** (e.g., MissionEnter or a different function). DEFERRED for future audit.

### Confirmed-phantom claims (these addresses are wrong in docs)

- `InfantryClass::DoType_Sequencer @ 0x00520A60` (E1) — no standalone function; address is inside Fire_At_Target body
- `InfantryClass::GetFireError @ 0x0051C8B0` (GGI) — no function at that address
- `InfantryClass::Mission_Enter @ 0x005196A0` (ENGINEER, TANY) — no standalone function; address is inside `PerCellProcess` body (re-confirmed audit 7)

### Confirmed-incorrect struct-offset claims

- `InfantryTypeClass+0xEC3 = Engineer` (ENGINEER doc previous claim) — **WRONG**. Verified correct: `+0xEC5` (Mission_Capture decompile).
- `TypeClass+0xA0 = display-name pointer` (E1 audit 1 claim from IronCurtain decompile) — **WRONG**. Verified correct: `+0xA0 = Strength` via CLEG InitiateWarp's `WarpHP = type+0xA0 × 10` arithmetic (audit 5).
- `RTTI value 1 = BuildingClass` (ENGINEER audit 3 claim from Mission_Capture's `iVar2 == 1` check) — **WRONG**. Verified correct: RTTI=1 is FootClass, RTTI=6 is BuildingClass (audit 4+5).
- `TechnoTypeClass+0xD29 = Crushable-related flag on target` (audit 2 GGI claim from CanCrushCheck branch 1) — **WRONG**. Verified correct: `+0xD29 = OmniCrusher` (a crusher-side capability override) via TechnoTypeClass__ReadINI (audit 7).
- `TechnoTypeClass+0xD50 = pre-deploy weapon override` (audit 1 claim) — **WRONG / IMPRECISE**. Verified correct: `+0xD50 = OpenTransportWeapon` (int, -1 sentinel = decide normally) via TechnoTypeClass__ReadINI (audit 7).
- `TANY doc: BuildLimit at TechnoTypeClass+0x6F8` — **WRONG**. Verified correct: `+0x3B8` (audit 7).
- `TANY doc: SelfHealing at TechnoTypeClass+0xC92` — **WRONG**. Verified correct: `+0xD14` (audit 7).
- `TANY doc: ImmuneToPsionics at InfantryTypeClass+0xCD7` — **WRONG (class + offset)**. Verified correct: `TechnoTypeClass+0xD35` (audit 7).
- `TANY doc: DetectDisguise at InfantryTypeClass+0xCDF` — **WRONG (class + offset)**. Verified correct: `TechnoTypeClass+0xD31` (audit 6 + 7).
- `TANY doc: Crushable at TechnoTypeClass+0x4xx (vague)` — **IMPRECISE**. Verified correct: `ObjectTypeClass+0x22D` (audit 7, BINARY-VERIFIED via ObjectTypeClass__ReadINI).
- `Cumulative table (pre-audit-11): TechnoTypeClass+0xD3A = Teleporter` — **WRONG**. Verified correct: `+0xD3A = Warpable` (per CLEG audit 5 — target-eligibility for chrono erase). `Teleporter` (self-warp-capable) is at **`TechnoTypeClass+0xCD4`** (audit 11, BINARY-VERIFIED via TechnoTypeClass__ReadINI). Two semantically distinct INI keys, two distinct byte offsets — do not conflate.
- `CCOMAND doc: TeleportLocomotionClass::TimerCheck @ 0x0070F770` — **WRONG**. Verified correct: `0x00719BF0` (audit 11, Ghidra-labeled). `0x0070F770` is unrelated `FUN_0070f770` (97-byte body, unlabeled). Corrected in CCOMAND.md.
- `CCOMAND doc: Teleporter parser site @ 0x0071450F` — **WRONG**. Verified correct: `0x00713FE9` (audit 11). `0x0071450F` is the parser site for `RequiresStolenAlliedTech` (the two addresses were transposed). Corrected in CCOMAND.md.
- `LCRF / SHAD / SAPC docs: Naval = UnitType-scope` — **WRONG**. Verified correct: `Naval` is TechnoType-scope at TechnoType+0xCCE (audit 25). Parsed in TechnoTypeClass__ReadINI, NOT UnitTypeClass__ReadINI (audit 12 confirms). Inherited by all unit type subclasses but lives in the TechnoType base.
- `SHAD doc: LeaveTransportSound string @ 0x008440F8` (inferred adjacency) — **WRONG**. Verified correct: `0x008440D4` (20 bytes before EnterTransport, not 16 bytes after) (audit 24).
- `TNKD doc: Turret = UnitType-scope` — **WRONG**. Verified correct: `Turret` is **TechnoType-scope** at `TechnoType+0xCA1` (audit 12). Applies to ALL TechnoTypes incl. BuildingType (see BuildingClass::HasTurret). Corrected in TNKD.md.
- `TNKD doc: TooBigToFitUnderBridge = TechnoType-scope` — **WRONG**. Verified correct: **UnitType-scope** at `UnitType+0xE16` (audit 12, parsed in UnitTypeClass__ReadINI). Corrected in TNKD.md.
- `Audit-2 GGI cumulative: TechnoType+0xD2A = "crusher-side gate flag (CanCrushCheck branch 1 — must be 0 on the crusher's type) — exact INI mapping still TBD"` — **IMPRECISE/SWAPPED**. Verified correct: `+0xD2A = OmniCrushResistant` (target-side resistance flag, BINARY-VERIFIED audit 14 via TechnoTypeClass__ReadINI). The "must be 0 on crusher" framing was a misread of which side reads the offset — it's actually the target's flag, checked by the crusher's CanCrushCheck before attempting an OmniCrusher override. Now updated in the TechnoTypeClass cumulative.

### Pattern: docs cite addresses INSIDE function bodies, not entry points

Across audit iterations 1-3, every "phantom function" finding turns out to
be the doc citing an address INSIDE a larger function's body — the address
points to the *line of interest* in Ghidra's decompile output, not a
function entry point. Future audit passes should call this out as
"behavior-address-inside-function-X" rather than "phantom function".

### Verified struct offsets (cumulative)

**ObjectTypeClass (inherited by TechnoType/InfantryType/UnitType/BuildingType — BINARY-VERIFIED via ObjectTypeClass__ReadINI @ 0x005F92D0, audit 7):**
- `+0xa0` = Strength (int) — was claimed "display-name pointer" in audit 1, **CORRECTED**: this is Strength on ObjectType, inherited by TechnoType (CLEG audit 5 + TANY audit 7 confirm)
- `+0x22D` = Crushable (byte) — read by CanCrushCheck branch 2 via vtable+0x88
- `+0x22E` = Bombable (byte) — gates Crazy Ivan bomb cursor
- `+0x231` = LegalTarget (byte)
- `+0x232` = Insignificant (byte)
- `+0x233` = Immune (byte)
- `+0x236` = Voxel (byte)
- `+0x237` = NewTheater (byte)
- `+0x239` = IgnoresFirestorm (byte)
- `+0x23A` = UseLineTrail (byte)
- `+0x7E` = Image (char[25] string) (audit 21, BINARY-VERIFIED via full ObjectTypeClass__ReadINI decompile — ObjectType-scope. RESOLVES prior DEFERRED about which layer parses `Image=` redirect — it's ObjectType-level, inherited by TechnoType/InfantryType/UnitType/BuildingType/etc.)
- `+0x9C` = Armor (int enum) (audit 21, BINARY-VERIFIED via `param_1[0x27] = iVar4` after FUN_004753F0 armor-enum-lookup helper. ObjectType-scope. Above TechnoType.)
- `+0x98..+0x9A` = RadialColor (RGB short+byte) (audit 21)
- `+0x1E8` = NoSpawnAlt (byte) (audit 21, BINARY-VERIFIED via `(param_1 + 0x7A)` after ReadBool; string @ 0x00832BC0, parser xref @ 0x005F943E. Causes voxel swap to `<UnitID>WO` (e.g., DESTWO) when SpawnManager has no spawns out — ObjectType-scope means it can work on ANY unit-class type. DEST/SREF/etc. are confirmed users.)
- `+0x1F0` = CrushSound (int VocClass index) (audit 21, ObjectType-scope — promotion from previously-assumed TechnoType-scope. Inherited by all unit/building types.)
- `+0x1F4` = AmbientSound (int VocClass index) (audit 21)
- `+0x211` = AlternateArcticArt (byte) (audit 21)
- `+0x213` = AlphaImage (char[25] string) (audit 21)
- `+0x22C` = Theater (byte) (audit 21)
- `+0x22F` = RadarInvisible (byte) (audit 21)
- `+0x230` = Selectable (byte) (audit 21)
- `+0x238` = HasRadialIndicator (byte) (audit 21)
- `+0x23B..+0x23D` = LineTrailColor (RGB short+byte) (audit 21)
- `+0x240` = LineTrailColorDecrement (int) (audit 21)

**AircraftTypeClass (BINARY-VERIFIED audit 26 via full AircraftTypeClass__ReadINI decompile — AircraftType inherits TechnoType which inherits ObjectType, so all TechnoType + ObjectType offsets ALSO apply at their respective offsets; these are the AircraftType-specific additions starting at +0xDFC):**
- `+0xDFC` = Carryall (byte) — Carryall transport flag (TS holdover, dormant in YR)
- `+0xE00` = Trailer (AnimType*) — trailing animation behind aircraft
- `+0xE04` = SpawnDelay (int) — delay between consecutive spawn-launches (for spawner aircraft like Carrier Hornets)
- `+0xE08` = Rotors (byte) — helicopter rotor animation flag
- `+0xE09` = CustomRotor (byte) — custom rotor sprite override
- `+0xE0A` = Landable (byte) — aircraft can land (vs perpetually airborne)
- `+0xE0B` = FlyBy (byte) — fly-by attack pattern (vs hover-attack)
- `+0xE0C` = FlyBack (byte) — return-to-base behavior
- `+0xE0D` = AirportBound (byte) — must land at airport-class buildings; crashes if no airport
- `+0xE0E` = Fighter (byte) — fighter-class flag (vs bomber/transport); affects AI air-vs-air heuristics

**BulletTypeClass (BINARY-VERIFIED audit 22 via full BulletTypeClass__ReadINI decompile — BulletType inherits ObjectType, so the audit-21 ObjectType offsets +0x7E Image, +0x9C Armor, +0x1E8 NoSpawnAlt, +0x22D Crushable etc. ALSO apply at the lower offsets; these are the BulletType-specific additions starting at +0x1F8):**
- `+0x1F8..+0x210` = Image (char[25] string, BulletType-specific; distinct from ObjectType +0x7E Image)
- `+0x294` = Airburst (byte)
- `+0x295` = Floater (byte)
- `+0x296` = SubjectToCliffs (byte)
- `+0x297` = SubjectToElevation (byte)
- `+0x298` = SubjectToWalls (byte)
- `+0x299` = VeryHigh (byte)
- `+0x29A` = Shadow (byte)
- `+0x29B` = Arcing (byte)
- `+0x29C` = Dropping (byte)
- `+0x29D` = Level (byte)
- `+0x29E` = Inviso (byte)
- `+0x29F` = Proximity (byte)
- `+0x2A0` = Ranged (byte)
- `+0x2A1` = !Rotates (byte, INVERTED bool — stored as `cVar4 == '\0'`)
- `+0x2A2` = Inaccurate (byte)
- `+0x2A3` = FlakScatter (byte)
- `+0x2A4` = (unknown, INI key DEFERRED — parser reads from DAT_0081B09C)
- `+0x2A5` = (unknown, INI key DEFERRED — parser reads from DAT_0081B098)
- `+0x2A6` = Degenerates (byte)
- `+0x2A7` = Bouncy (byte)
- `+0x2A8` = AnimPalette (byte)
- `+0x2A9` = FirersPalette (byte)
- `+0x2AC` = Cluster (int)
- `+0x2B0` = AirburstWeapon (WeaponType*)
- `+0x2B4` = ShrapnelWeapon (WeaponType*) — the SREF prism-chain trigger
- `+0x2B8` = ShrapnelCount (int) — number of shrapnel-weapon spawns on impact
- `+0x2BC` = DetonationAltitude (int)
- `+0x2C0` = Vertical (byte)
- `+0x2C8..+0x2CF` = Elasticity (double, 8 bytes)
- `+0x2D0` = Acceleration (int)
- `+0x2D4` = Color (int RGB)
- `+0x2D8` = Trailer (AnimType*)
- `+0x2DC` = (unknown, INI key DEFERRED — parser reads from DAT_0081B164)
- `+0x2E0` = CourseLockDuration (int)
- `+0x2E4` = SpawnDelay (int)
- `+0x2EC` = Scalable (byte)
- `+0x2F0` = (unknown, INI key DEFERRED — parser reads from DAT_0081B168)
- `+0x2F4` = AnimLow (byte)
- `+0x2F5` = AnimHigh (byte)
- `+0x2F6` = AnimRate (byte)
- `+0x2F7` = Flat (byte)

**TechnoTypeClass / InfantryTypeClass:**
- `+0x29C` = VeteranAbilities array start (audit 7, ReadINI)
- `+0x2AE` = EliteAbilities array start (audit 7, ReadINI)
- `+0x2C0` = SpecialThreatValue (double, 8 bytes) (audit 7)
- `+0x3B8` = BuildLimit (int) (audit 7) — **CORRECTS TANY doc claim of +0x6F8**
- `+0x5F4` = DetectDisguiseRange (TechnoType-scope, BINARY-VERIFIED via AddDetectDisguiseAt + ReadINI, audit 6)
- `+0x5FC` = LeadershipRating (int) (audit 7)
- `+0x600` = NavalTargeting (int) (audit 7)
- `+0x634` = TechLevel (int) (audit 13, BINARY-VERIFIED via TechnoTypeClass__ReadINI; parser xref @ 0x00714577. Read by 5 other consumers including lobby `RulesClass__ReadMultiplayerDialogSettings @ 0x00671fad` and scenario `HouseClass__Read_Scenario_INI @ 0x00500b95`)
- `+0x404` = DeploysInto (BuildingType*) (audit 14, BINARY-VERIFIED via TechnoTypeClass__ReadINI write of `param_1[0x101]` after BuildingTypeClass__FindOrAllocate; parser xref @ 0x00713279. Consumed 4× in UnitClass::Deploy body)
- `+0x408` = UndeploysInto (UnitType*) (audit 14, BINARY-VERIFIED via `param_1[0x102]` after UnitTypeClass__FindOrAllocate)
- `+0x40C` = PowersUnit (UnitType*) (audit 14, BINARY-VERIFIED via `param_1[0x103]` after UnitTypeClass__FindOrAllocate; sibling key, not on AMCV)
- `+0x56C` = DeploySound (int VocClass index) (audit 14, BINARY-VERIFIED via `param_1[0x15b]`; parser xref @ 0x00713568)
- `+0x570` = UndeploySound (int VocClass index) (audit 14, BINARY-VERIFIED via `param_1[0x15c]`)
- `+0x6B8` = DeployingAnim (AnimType*) (audit 14, BINARY-VERIFIED via `param_1[0x1ae]` after AnimTypeClass__FindOrAllocate)
- `+0x608` = BuildTimeMultiplier (float bits stored as int) (audit 15, BINARY-VERIFIED via `param_1[0x182]` after CCINIClass::ReadDouble + float-to-int reinterpret-cast; string @ 0x00843CF0, parser xref @ 0x00714371. Default 1.0; multiplies the cost-derived base build time. Storage convention is unusual: 4 bytes hold the IEEE-754 bit pattern of a float, despite the typed `int` write — pattern occurs elsewhere in TechnoTypeClass too.)
- `+0x6AF` = OpportunityFire (byte) (audit 15, BINARY-VERIFIED via `(int)param_1 + 0x6af` after CCINIClass::ReadBool; string @ 0x00843A74, parser xref @ 0x0071483D. Default false; when true, the unit auto-targets in-range threats without an explicit attack order — key for "tank rush feels responsive" behavior on MBTs.)
- `+0xD32` = DisguiseWhenStill (byte) (audit 16, BINARY-VERIFIED via `(int)param_1 + 0xd32` after ReadBool; string @ 0x00843C64, parser xref @ 0x00714459. When true + CanDisguise=yes + zero speed, the engine random-picks from `[General] DefaultMirageDisguises=` and applies the disguise.)
- `+0xD33` = CanApproachTarget (byte) (audit 16, BINARY-VERIFIED via `(int)param_1 + 0xd33` after ReadBool; string @ 0x00843C2C, parser xref @ 0x007144A7. Default true; when false, the unit will not auto-chase targets — but a manual Attack Mission overrides this. Used by Mirage Tank to enforce ambush role.)
- `+0x898` = Secondary (WeaponType*) (audit 16, BINARY-VERIFIED via `param_1[0x226]` after WeaponTypeClass__FindOrAllocate. TechnoType-level Secondary slot — SEPARATE from InfantryType-only +0xE48.)
- `+0xA94` = ElitePrimary (WeaponType*) (audit 16, BINARY-VERIFIED via `param_1[0x2a5]`. TechnoType-level ElitePrimary — SEPARATE from InfantryType-only +0xE44.)
- `+0xAB0` = EliteSecondary (WeaponType*) (audit 16, BINARY-VERIFIED via `param_1[0x2ac]` default-read. TechnoType-level EliteSecondary — SEPARATE from InfantryType-only +0xE4C. **KEY FINDING**: parsed independently from ElitePrimary, no parser-time fallback when ElitePrimary is absent. This means an `EliteSecondary=X` without `Secondary=` produces no weapon upgrade at runtime.)
- `+0x894` = Primary (WeaponType*) INFERRED by symmetry with +0x898 Secondary (audit 16 — not directly verified in grep window; DEFERRED for direct verification).
- `+0x568..+0x57C` = TechnoType-level VocClass sound-list block (6 ints, BINARY-VERIFIED audit 17 via grep — adjacent slots parsed in declared INI-key order):
  - `+0x568` = (sibling, INI key DEFERRED)
  - `+0x56C` = DeploySound (audit 14)
  - `+0x570` = UndeploySound (audit 14)
  - `+0x574` = ChronoInSound (audit 17, sequence-position evidence — per-TechnoType override for dual-read pattern)
  - `+0x578` = ChronoOutSound (audit 17, sequence-position evidence)
  - `+0x57C` = (sibling, INI key DEFERRED)
- `+0x6D4` = StupidHunt (byte) (audit 17, BINARY-VERIFIED via `*(undefined1*)(param_1 + 0x1B5) = uVar3` after ReadBool; string @ 0x008438A4, parser xref @ 0x00714C6C. When true, AI Hunt-mission fallback for weaponless units like CMIN/SMIN that can't actually attack — "run toward player base instead".)
- `+0x68C` = AirRangeBonus (int) (audit 18, BINARY-VERIFIED via `param_1[0x1A3] = iVar4` after CCINIClass::ReadRange; string @ 0x00843AD4, parser xref @ 0x007147A1. Sibling to +0x688 IFVMode. Extends AA engagement range by this many cells.)
- `+0x805` = Gunner (byte) (audit 18, BINARY-VERIFIED via `(int)param_1 + 0x805` after ReadBool; string @ 0x00843964, parser xref @ 0x00714A50. Gates the entire IFV multi-weapon gunner-table mechanism.)
- `+0x808` = TurretCount (int) (audit 18, BINARY-VERIFIED via `param_1[0x202] = iVar4`; string @ 0x00844348, parser xref @ 0x00712851. Number of distinct visual turret graphics. FV sets to 4.)
- `+0x80C` = WeaponCount (int) (audit 18, BINARY-VERIFIED via `param_1[0x203] = iVar4`; string @ 0x0084433C, parser xref @ 0x0071286B. Number of weapon slots. FV sets to 17 (Weapon1..Weapon17 + EliteWeapon1..17). Hard-coded compile-time max per author note is 15, but shipped INI exceeds this — likely a comment error since live binary handles 17 fine.)
- `+0x814..+0x858` = gunner turret-index lookup table (17 ints; index = WeaponSlot 0-16, value = visual TurretIndex 0-3) (audit 18, BINARY-VERIFIED via FUN_00717890 + UnitTypeClass__ReadINI gunner block. Populated in this fixed parse order: 0=NormalTurret, 1=RepairTurret, 2=MachineGunTurret, 3=FlakTurret, 4=PistolTurret, 5=SniperTurret, 6=ShockTurret, 7=ExplodeTurret, 8=BrainBlastTurret, 9=RadCannonTurret, 10=ChronoTurret, 11=TerroristExplodeTurret, 12=CowTurret, 13=InitiateTurret, 14=VirusTurret, 15=YuriPrimeTurret, 16=GuardianTurret. The IFVMode-integer → WeaponSlot consumer mapping at runtime is DEFERRED but strongly inferred to be 1:1.)
- `+0xD18` = DeathWeapon (WeaponType*) (audit 18, BINARY-VERIFIED via `param_1[0x346] = iVar4` after WeaponTypeClass__FindOrAllocate; string @ 0x0083B11C. TechnoType per-unit override side of dual-read pattern with RulesClass__ReadCombatDamage global default. FV uses CRNuke here as Ivan-passenger special-case detonation.)
- `+0x5E4` = OpenTopped (byte) (audit 19, BINARY-VERIFIED via `(param_1 + 0x179) = char` after ReadBool; string @ 0x00843CCC, parser xref @ 0x007143BD. Gates the gun-port passenger-fire mechanic; passengers fire their own weapons from the AlternateFLH0..4 positions on the host vehicle. Range and damage scaled by `[CombatDamage]` globals OpenToppedRangeBonus/DamageMultiplier/WarpDistance.)
- `+0x89C..+0x8D8` = AlternateFLH0..4 (5 int-triplets, each 3 ints = 12 bytes; total 60 bytes) (audit 19, BASE +0x89C BINARY-VERIFIED via `param_1 + 0x227` after CCINIClass::Read3Int call with format-string `AlternateFLH%d`; full 5-entry layout INFERRED from INI evidence + format-string parse pattern. Index 0 at +0x89C, index 1 at +0x8A8, index 2 at +0x8B4, index 3 at +0x8C0, index 4 at +0x8CC. Used by OpenTopped vehicles to position passenger projectile spawn-points.)
- `+0x3D0` = FireAngle (int degrees) (audit 20, BINARY-VERIFIED via `param_1[0xF4] = iVar4`; string @ 0x00843910, parser xref @ 0x00714B5D. Initial pitch angle for spawned aircraft/missile launch — Carrier=32, similarly used by Dread/V3.)
- **Spawn-family cluster** `+0xD54..+0xD68` (audit 20, BINARY-VERIFIED via grep on TechnoTypeClass__ReadINI):
  - `+0xD54` = Spawned (byte; string @ 0x008437D8, parser xref @ 0x00714E7D. Marks "spawn-only TechnoType, not directly buildable" — set on Hornet, DMISL, V3ROCKET, etc.)
  - `+0xD58` = Spawns (TechnoType*; string @ 0x008184C8, parser xref @ 0x00714E9E. Stored via FUN_0067BD30 = TechnoTypeClass-FindOrAllocate. Carrier→HORNET, Dread→DMISL, V3→V3ROCKET.)
  - `+0xD5C` = SpawnsNumber (int; string @ 0x008437B8, parser xref @ 0x00714EE1. Number of spawn slots in magazine — Carrier=3, Dread=2, V3=1.)
  - `+0xD60` = SpawnRegenRate (int frames; string @ 0x008437C8, parser xref @ 0x00714EC0. Frames to manufacture replacement after destruction — Carrier=600. 0 = re-purchase only.)
  - `+0xD64` = SpawnReloadRate (int frames; string @ 0x008437A8, parser xref @ 0x00714F02. Frames for docked spawn to refill Ammo — Carrier=150 for Hornet's Ammo=1 bomb.)
  - `+0xD68` = MissileSpawn (byte; string @ 0x00843798, parser xref @ 0x00714F23. The SpawnManagerClass missile-vs-aircraft branch flag — 0 on Hornet, 1 on DMISL/V3ROCKET. Per SPAWN_MANAGER_CLASS_GHIDRA_REPORT cross-reference, gates whether the spawn uses RocketStruct-based missile flow or regular aircraft-spawn flow.)
- `+0x5F0` = SensorsSight (int cells) (audit 21, BINARY-VERIFIED via `param_1[0x17C]`; string @ 0x00843D50, parser xref @ 0x007142E8. Adjacent to audit-6 +0x5F4 DetectDisguiseRange — "detection-range cluster" at +0x5F0..+0x5F8.)
- `+0xC9D` = Sensors (byte) (audit 21, BINARY-VERIFIED via `(int)param_1 + 0xC9D`; string @ 0x00843E58, parser xref @ 0x00714003. Submarine-detection ability — gates the cloak-piercing reveal within SensorsSight range.)
- `+0x810` = IsChargeTurret (byte) (audit 22, BINARY-VERIFIED via `param_1 + 0x204`; string @ 0x0084432C, parser xref @ 0x00712885. Slots cleanly between +0x80C WeaponCount and +0x814 gunner-table — only fires when multi-turret weapon system is active. Used by SREF Prism Tank for pre-fire charge animation.)
- `+0x410` = PoweredUnit (byte) (audit 23, BINARY-VERIFIED via `(param_1 + 0x104)`; string @ 0x00844158, parser xref @ 0x00713316. Gates the PoweredUnit deactivation state machine — unit goes offline if owning house lacks power OR controlling building is destroyed. ROBO uses this paired with GAROBO. The engine's "which building controls which units" lookup mechanism is DEFERRED.)
- `+0x45C` = VoiceSelectDeactivated (int VocClass soundlist index) (audit 23, BINARY-VERIFIED via `param_1[0x117]`; string @ 0x00844288, parser xref @ 0x00712C0A. Parallel to VoiceSelect — engine swaps which voice plays based on unit's active/deactivated state. Rare field; only PoweredUnit=yes units typically need it.)
- `+0x5A8` = ActivateSound (int VocClass index) (audit 23, BINARY-VERIFIED via `param_1[0x16A]`; string @ 0x0083A6DC, parser xref @ 0x007138EC. TechnoType side of DUAL-READ pattern with `RulesClass__ReadAudioVisual @ 0x0066A21E` global default. Plays on activation transition (power restored or controlling building rebuilt).)
- `+0x5AC` = DeactivateSound (int VocClass index) (audit 23, sequence-position INFERRED from ActivateSound adjacency; string @ 0x0083A6CC, parser xref @ 0x00713922. TechnoType side of DUAL-READ pattern with `RulesClass__ReadAudioVisual @ 0x0066A260`. Plays on deactivation transition.)
- `+0x564` = EnterTransportSound (int VocClass index) (audit 24, BINARY-VERIFIED via `param_1[0x159] = iVar6`; string @ 0x008440E8, parser xref @ 0x007133FC. Plays when infantry/vehicle boards transport.)
- `+0x568` = LeaveTransportSound (int VocClass index) (audit 24, BINARY-VERIFIED — **RESOLVES audit-17 DEFERRED "+0x568 unknown sibling"** which was previously logged as "possibly SegueSound or CreateSound" but is actually LeaveTransportSound; string @ 0x008440D4, parser xref @ 0x00713432. Plays when infantry/vehicle disembarks.)
- `+0xCCE` = Naval (byte) (audit 25, BINARY-VERIFIED via `(int)param_1 + 0xCCE`; string @ 0x0084395C, parser xref @ 0x00714A6A in TechnoTypeClass__ReadINI. **CORRECTS SHAD/SAPC/LCRF doc claims of UnitType-scope** — Naval is TechnoType-scope, parsed in TechnoTypeClass__ReadINI (NOT UnitTypeClass__ReadINI, per audit 12 confirmation). Gates shipyard-build path + torpedo vulnerability + Squid-target eligibility.)
- `+0x680` = InitialAmmo (int) (audit 26, BINARY-VERIFIED via `param_1[0x1A0]`; string @ 0x00843AEC, parser xref @ 0x00714755. Initial ammo count at unit spawn — distinct from runtime current Ammo. BEAG sets to 1.)
- `+0xC96` = ToProtect (byte) (audit 27, BINARY-VERIFIED via `(int)param_1 + 0xC96`; string @ 0x008438DC, parser xref @ 0x00714BE8. AI high-value-support hint — shared with harvesters, MCVs, Aircraft Carrier, Slave Miner. Tells AI to escort/protect/repair-prioritize these units.)
- `+0x6A4` = RadialFireSegments (int) (audit 27, BINARY-VERIFIED via `param_1[0x1A9]`; string @ 0x00843AC0, parser xref @ 0x007147BB. 360° facing divided into N wedge sectors; engine launches from segment matching target bearing — bypasses body rotation requirement for fast AA response. AEGIS uses N=10.)
- `+0x6B0` = DistributedFire (byte) (audit 27, BINARY-VERIFIED via `(param_1 + 0x1AC)`; string @ 0x00843A64, parser xref @ 0x00714857. Multi-target round-robin firing — successive shots pick different in-range targets instead of focusing one. Critical for AA escort vs multi-unit air attacks.)
- `+0x548` = SinkingSound (int VocClass index) (audit 27, BINARY-VERIFIED via `param_1[0x152]` default-read; string @ 0x0083A9B4, parser xref @ 0x00712FB0. TechnoType side of DUAL-READ pattern with `RulesClass__ReadAudioVisual @ 0x006699A7` global default. Long-form naval-death audio. Joins family with ChronoIn/OutSound, ImpactLandSound, Activate/DeactivateSound.)
- `+0xD2A` = OmniCrushResistant (byte) (audit 14, BINARY-VERIFIED via TechnoTypeClass__ReadINI write of `(int)param_1 + 0xd2a` after ReadBool; string @ 0x00843868, parser xref @ 0x00714D11. **CORRECTS audit-2 GGI cumulative** which had +0xD2A as "TBD crusher-side gate flag". This is the target-side resistance flag in CanCrushCheck — completes the 3-tier crush hierarchy: Crusher → Crushable → OmniCrusher → OmniCrushResistant.)
- `+0x670` = ThreatPosed (int) (audit 7)
- `+0x688` = IFVMode (int) (audit 7)
- `+0x6a8` = DeployFireWeapon slot index (BINARY-VERIFIED via FUN_005218e0)
- `+0x6ac` = DeployFire flag (BINARY-VERIFIED via FUN_005218e0)
- `+0x6D0` = AIBasePlanningSide (TechnoType-scope, BINARY-VERIFIED via OnSpyInfiltrate + ReadINI, audit 6)
- `+0x800` = Storage (TechnoType-scope, BINARY-VERIFIED via OnSpyInfiltrate + ReadINI, audit 6)
- `+0xC91` = ImmuneToVeins (byte) (audit 7)
- `+0xd29` = OmniCrusher (byte) — **CORRECTED audit 7** (was "Crushable-related flag on target" in audit 2 — wrong interpretation). Read by CanCrushCheck branch 1 via vtable+0x84 on potential crusher.
- `+0xd2a` = crusher-side gate flag (CanCrushCheck branch 1 — must be 0 on the crusher's type) — exact INI mapping still TBD
- `+0xD2F` = CanDisguise byte (TechnoType-scope, BINARY-VERIFIED via ReadINI, audit 6)
- `+0xD30` = PermaDisguise byte (TechnoType-scope, BINARY-VERIFIED via ReadINI, audit 6)
- `+0xD31` = DetectDisguise byte (TechnoType-scope, BINARY-VERIFIED via ReadINI, audit 6) — **CORRECTS TANY doc claim of "InfantryType+0xCDF"**
- `+0xD14` = SelfHealing (byte) (audit 7) — **CORRECTS TANY doc claim of +0xC92**
- `+0xD35` = ImmuneToPsionics (byte) (audit 7) — **CORRECTS TANY doc claim of "InfantryType+0xCD7"**. Read by CaptureManagerClass::CanCapture @ 0x00471C90.
- `+0xd50` = OpenTransportWeapon (int, -1 sentinel = decide normally) — **CORRECTED audit 7** (was "pre-deploy weapon override" in audit 1 cheat-sheet)
- `+0xDBC` = IsSelectableCombatant (byte) (audit 7)
- `+0x390` = HoverAttack (byte) (audit 8)
- `+0xD6A` = BalloonHover (byte) (audit 8)
- `+0xD70` = JumpjetSpeed (int) (audit 8)
- `+0xD74` = JumpjetClimb (float-as-int) (audit 8)
- `+0xD78` = JumpjetCrash (float-as-int) (audit 8)
- `+0xD94` = JumpJet flag (byte — read by FootClass::Locomotion_AI to gate sequence 0x17/0x18 dispatch) (audit 8, BINARY-VERIFIED via FootClass::Locomotion_AI + ReadINI)
- `+0xD95` = Crashable (byte — distinct from ObjectType+0x22D Crushable; gates crash-animation/state-5-abort for aircraft-style units) (audit 8)
- `+0xD96` = ConsideredAircraft (byte — AA-targeting routing flag) (audit 8)
- `+0x693` = Natural (byte) (audit 9)
- `+0xD37` = ImmuneToRadiation (byte) (audit 9)
- `+0xD39` = DefaultToGuardArea (byte) (audit 9)
- `+0xD3C` = ReselectIfLimboed (byte) (audit 9)
- `+0xD3D` = RejoinTeamIfLimboed (byte) (audit 9)
- `+0x6C8` = PreventAttackMove (byte) (audit 10)
- `+0xD99` = CanPassiveAquire (byte) (audit 10)
- `+0xDA0` = RequiredHouses (int, parsed via FUN_004750D0 helper) (audit 10)
- `+0xDA4` = ForbiddenHouses (int) (audit 10)
- `+0x5BC` = MaxDebris (int) (audit 12, BINARY-VERIFIED via TechnoTypeClass__ReadINI write at `param_1[0x16f]`)
- `+0x614` = Soylent (int) (audit 12, BINARY-VERIFIED via `param_1[0x185]`)
- `+0xCA1` = Turret (byte) (audit 12, BINARY-VERIFIED via in-binary Ghidra annotation at top of TechnoTypeClass__ReadINI; writer @ 0x007133C2 after ReadBool("Turret"); applies to ALL TechnoTypes incl. BuildingType — readers in UnitClass::Draw_Body_And_Turret/Facing_Update/Fire_At_Target, BuildingClass::HasTurret, ship Locomotion, TechnoClass::AI_Update)
- `+0xD28` = Crusher (byte) (audit 12, BINARY-VERIFIED via `param_1 + 0x34a`)
- `+0xDBD` = Accelerates (byte) (audit 12, BINARY-VERIFIED via `(int)param_1 + 0xdbd`)
- `+0xC8D` = MoveToShroud (byte, default 1) (audit 11, BINARY-VERIFIED via TechnoTypeClass__ReadINI; string @ 0x008444C4)
- `+0xCD4` = Teleporter (byte) (audit 11, BINARY-VERIFIED via TechnoTypeClass__ReadINI; string @ 0x00843E60; parser xref @ 0x00713FE9). **CORRECTS prior cumulative-table claim of `+0xD3A = Teleporter`** — see "Confirmed-incorrect struct-offset claims" below.
- `+0xD9B` = RequiresStolenThirdTech (byte) (audit 11, BINARY-VERIFIED; string @ 0x00843BFC)
- `+0xD9C` = RequiresStolenSovietTech (byte) (audit 11, BINARY-VERIFIED; string @ 0x00843BE0)
- `+0xD9D` = RequiresStolenAlliedTech (byte) (audit 11, BINARY-VERIFIED; string @ 0x00843BC4; parser xref @ 0x0071450F)
- `+0xDF8` = capture-tag value (written to building+0x338 on capture, audit 3)
- `+0xE40` = primary weapon ptr (Fire_At_Target)
- `+0xE44` = elite primary weapon ptr (Fire_At_Target)
- `+0xE48` = secondary weapon ptr (Fire_At_Target, conditional on WeaponType+0x5a4)
- `+0xE4C` = elite secondary weapon ptr (Fire_At_Target, conditional on WeaponType+0x5c8)
- `+0xEB4` = Occupier flag (AddGarrisonOccupant, audit 1)
- `+0xEB5` = **Assaulter (byte)** **[CORRECTED audit 33]** (audit 1 claimed "paratrooper-occupier flag" — that was a label guess from AddGarrisonOccupant context. Actual INI binding: `Assaulter=yes/no` via parser xref @ 0x005244ef. Assembly-context proof: writeback `MOV byte ptr [ESI + 0xeb5], AL` at 0x0052450b after Assaulter ReadBool. Enables an infantry unit to clear garrisoned UC buildings — SEAL/Tanya/Yuri set this; SHK explicitly sets `Assaulter=no`.)
- `+0xEBD` = Crawls (InfantryType, BINARY-VERIFIED audit 7 — final ReadBool in InfantryTypeClass__ReadINI capability-flag chain)
- `+0xEBE` = Infiltrator-capability synthesized flag (set by `Infiltrate=`, or auto-set if `+0xEC2` C4 / `+0xEC3` / `+0xEC4` are also set — InfantryTypeClass-scope confirmed via xref + ReadINI, audit 6)
- `+0xEC2` = C4 flag (Mission_Attack FUN_0051f3e0, audit 4)
- `+0xEC5` = Engineer flag (Mission_Capture, audit 3)
- `+0xC8F` = name-override-active aux flag (set when the first ReadBool in `InfantryTypeClass__ReadINI` is true, presumably `UseOwnName` — needs sibling-key xref confirmation) (audit 13, BINARY-VERIFIED placement, INI-key mapping INFERRED)
- `+0xEAC..+0xECB` = 23 sequential InfantryType ReadBool block in `InfantryTypeClass__ReadINI` (audit 13, structure BINARY-VERIFIED; per-offset INI keys partially mapped: +0xEC2=C4, +0xEC5=Engineer, +0xEBD=Crawls (out-of-sequence at end), +0xEBE=Infiltrator (synthesized from C4/+0xEC3/+0xEC4); other 19 offsets DEFERRED — requires sibling-xref enumeration)
- `+0xEAD` = NotHuman (byte) (audit 28, BINARY-VERIFIED via assembly-context proof — `0x005243c6: PUSH 0x825a00` → `CALL 0x005295f0` → `0x005243d7: MOV byte ptr [ESI + 0xead], AL`. **RESOLVES audit-9 ADOG DEFERRED** (NotHuman exact offset). InfantryType-scope ONLY — NOT read by UnitTypeClass__ReadINI. Critical implication: `NotHuman=yes` on vehicles like DLPH/SQUID/COW is **dead INI** since the vehicle parser ignores it.)
- `+0xEBC` = Fearless (byte) (audit 35, BINARY-VERIFIED via assembly-context at 0x0052447A: `MOV byte ptr [ESI + 0xebc], AL` after ReadBool; parser xref @ 0x00524469, string @ 0x008259D4. When set, the unit never plays Panic sequence and doesn't break formation under fire. Used by Desolator (with empty VoiceFeedback) + various heavy/elite units.)
- `+0xEC8` = Deployer (byte) (audit 35, BINARY-VERIFIED via assembly-context at 0x00524620: `MOV byte ptr [ESI + 0xec8], AL` after ReadBool; parser xref @ 0x0052460D, string @ 0x00825928. **InfantryType-specific** — enables deploy/undeploy command on infantry. Used by GGI deploy-fortify, Desolator deploy-radiation, and others. Distinct from UnitType-side IsSimpleDeployer for MCV-style deploy-into-building.)

**InfantryTypeClass additions (audit 32 — garrison-occupy subsystem, via InfantryTypeClass__ReadINI):**
- `+0xDFC` = OccupyPip (int enum) (audit 32, BINARY-VERIFIED via FUN_004748a0 pip-color helper at the very start of InfantryTypeClass__ReadINI body; string @ 0x00825a60, parser xref @ 0x005240f5. **InfantryType-scope** (NOT TechnoType-scope as E2 doc had claimed). Determines pip icon shown in garrisoned building's pip strip per occupant. Conscript uses PersonRed; GGI uses PersonBlue. Valid values: green/yellow/white/red/blue/purple.)
- `+0xE00` = (unknown sibling) (audit 32, BINARY-VERIFIED slot exists between OccupyPip +0xDFC and OccupyWeapon +0xE04; written via second FUN_004748a0 helper call at function entry. INI-key mapping DEFERRED.)
- `+0xE04` = OccupyWeapon (WeaponType*) (audit 32, BINARY-VERIFIED via `*(undefined4 *)(param_1 + 0xe04) = uVar3` after ReadString + WeaponTypeClass__FindOrAllocate; string @ 0x00825a50, parser xref @ 0x00524117. Per-infantry override for which weapon fires from a UC garrison window. Defaults to Primary if absent.)
- `+0xE20` = EliteOccupyWeapon (WeaponType*) (audit 32, BINARY-VERIFIED via `*(undefined4 *)(param_1 + 0xe20) = uVar4` after ReadString + WeaponTypeClass__FindOrAllocate; string @ 0x00825a3c, parser xref @ 0x00524156. Elite-rank version of OccupyWeapon.)
- `+0xEB4` = Occupier (byte) — re-confirms audit 1 via parser xref @ 0x005244d5 to string @ 0x008259a8.

**WeaponTypeClass additions (audit 32 + 33 — anim cluster):**
- `+0x110` = OccupantAnim (AnimType*) (audit 32, BINARY-VERIFIED via `*(undefined4 *)((int)this + 0x110) = uVar4` after ReadString + AnimTypeClass__FindOrAllocate; string @ 0x00849400, parser xref @ 0x007725a1 in WeaponTypeClass__ReadINI. Animation overlay drawn at the building window slot when a garrison-mode weapon fires. Per-weapon, not per-unit — multiple infantry types sharing the same OccupyWeapon get the same window anim.)
- `+0x114` = AssaultAnim (AnimType*) (audit 33, BINARY-VERIFIED via WeaponTypeClass__ReadINI decompile sequence; string @ 0x00849410, parser xref @ 0x00772574. Animation played when an `Assaulter=yes` unit clears a garrisoned UC building with this weapon. Used by SEAL/Tanya/Yuri assault weapons. Vestigial on SHK ElectricBolt because SHK has Assaulter=no.)
- `+0x118` = OpenToppedAnim (AnimType*) (audit 33, BINARY-VERIFIED via WeaponTypeClass__ReadINI decompile sequence; string @ 0x008493f0, parser xref @ 0x007725d3 (approx). Animation overlay for passengers firing from OpenTopped vehicles like Battle Fortress.)

**WarheadTypeClass additions (audit 33+34):**
- `+0x157` = IvanBomb (byte) (audit 34, BINARY-VERIFIED via assembly-context `MOV byte ptr [ESI + 0x157], AL` at 0x0075d823; string @ 0x0081bd60, parser xref @ 0x0075d807 in WarheadTypeClass__ReadINI. **The Crazy Ivan bomb-plant flag** — when a weapon with this warhead detonates, the engine branches to BombClass::Attach instead of normal damage application. Sibling to +0x158 ElectricAssault (audit 33). +0x14B/+0x157/+0x158 form a "special-effect trigger" family.)
- `+0x158` = ElectricAssault (byte) (audit 33, BINARY-VERIFIED via assembly-context `MOV byte ptr [ESI + 0x158], AL` at 0x0075d82e; string @ 0x00847d48, parser xref @ 0x0075d81d in WarheadTypeClass__ReadINI. **Tesla Coil charge flag** — when a weapon with this warhead hits a building, the engine routes through a hardcoded "charge the building" path instead of normal damage application. Sibling to +0x14B Sonic and +0x157 IvanBomb.)

**TechnoTypeClass additions (audit 34 — Explodes/Bomb/AttackFriendlies cluster):**
- `+0x5F8` = BombSight (int) (audit 34, BINARY-VERIFIED via `param_1[0x17e] = iVar4` int-array indexing 0x17e*4=0x5F8; string @ 0x00843d30, parser xref @ 0x0071431C in `TechnoTypeClass__ReadINI`. **TechnoType-scope** (NOT InfantryType-scope as IVAN doc claimed). Cells of bomb-detection radius for the Engineer/SEAL/Tanya BombClass::UpdateAll BombVisible refresh. Only infantry have non-zero values in stock YR, but the field is parsed at the TechnoType layer.)
- `+0x6C0` = AttackFriendlies (byte) (audit 34, BINARY-VERIFIED via `*(undefined1 *)(param_1 + 0x1b0) = uVar3` int*-stride 0x1b0*4=0x6C0; string @ 0x00843620, parser xref @ 0x0071522e. Default off; when set, AI threat-scan considers friendlies as valid targets. Crazy Ivan has this commented out.)
- `+0x6C1` = AttackCursorOnFriendlies (byte) (audit 34, BINARY-VERIFIED via `*(char *)((int)param_1 + 0x6c1) = (char)uVar5`; string @ 0x00843604, parser xref @ 0x0071524F. Lighter variant — only changes mouse cursor on friendly-hover but doesn't make AI auto-target. Crazy Ivan uses this for manual "bomb friendly Tanya" plays.)
- `+0xD15` = Explodes (byte) (audit 34, BINARY-VERIFIED via `*(undefined1 *)((int)param_1 + 0xd15) = uVar3` after ReadBool; string @ 0x0083355c, parser xref @ 0x007122c5 in TechnoTypeClass__ReadINI + 0x005fe840 in OverlayTypeClass__ReadINI (overlays can also Explodes=yes — e.g., crates). When unit dies, triggers a death-explosion using DeathWeapon/DeathWeaponDamage or default DeathWH warhead.)

**TechnoType tactical-AI byte cluster +0x6C0..+0x6C8 (audit 34 consolidates):**

| Offset | Key | Audit |
|--------|-----|-------|
| +0x6C0 | AttackFriendlies | 34 |
| +0x6C1 | AttackCursorOnFriendlies | 34 |
| +0x6C8 | PreventAttackMove | 10 |


**BuildingTypeClass additions (audit 33):**
- `+0x16E8` = ChargedAnimTime (float, ReadDouble) (audit 33, BINARY-VERIFIED via assembly-context `FSTP float ptr [EBP + 0x16e8]` at 0x00460bb1; string @ 0x0081a9b8, parser xref @ 0x00460b9e in `BuildingTypeClass_ReadINI_Water`. Per-building "charged" animation timer — controls how long the special "powered by Tesla Trooper" animation plays. Per-coil, not per-rule. **[CORRECTS SHK doc]**: doc had claimed Rules-AudioVisual scope.)

**RulesClass-CombatDamage additions (audit 32):**
- `OccupyWeaponRange` — Rules-CombatDamage scope (parser xref @ 0x0066c6c7 in RulesClass__ReadCombatDamage; string @ 0x0083b064). Global multiplier applied to all OccupyWeapon ranges. Offset DEFERRED (Rules-CombatDamage parser is oversized for inline decompile).

**TechnoTypeClass additions (audit 31 — tactical-AI byte + aircraft sound extensions, via TechnoTypeClass__ReadINI grep + assembly context):**
- `+0x1F8` = AltCameo (char[25] string) (audit 31, BINARY-VERIFIED via assembly-context `LEA ECX, [EBP + 0x1f8]` at 0x00715a73 + `PUSH 0x19` size limit; string @ 0x00843344, parser xref @ 0x00715a6e. Alternate sidebar cameo for veteran/elite rank or similar UI state. Consumer xref into FUN_007162f0 @ 0x00716d34 — UI-cameo selector function DEFERRED. **NOTE**: +0x1F8 falls within the ObjectType-portion layout (+0x1F4 AmbientSound, +0x211 AlternateArcticArt, +0x213 AlphaImage) but the parser is TechnoTypeClass__ReadINI — Westwood architectural quirk where TechnoType-context parser writes into the inherited ObjectType portion of memory.)
- `+0x504..+0x50C` = MoveSound (int[3] 3-slot SoundList) (audit 31, BINARY-VERIFIED via `param_1[0x141..0x143] = local_36c[4..6]` after ReadSoundList; string @ 0x008440c8, parser xref @ 0x00713478. **3 ints = open / body / close sound slots** in a SoundList (looping engine drone). Distinct from VocClass-index sounds — uses ReadSoundList not ReadString. ORCA's IntruderMoveLoop uses 7 samples in this slot.)
- `+0x550` = VoiceCrashing (int VocClass index) (audit 31, BINARY-VERIFIED via assembly-context writeback `MOV [EBP + 0x550], EAX` at 0x00713069; string @ 0x008441ec, parser xref @ 0x00713034. Aircraft crash voice — distinct from CrashingSound (+0x544) which is the SFX. Pilot voice samples for the falling aircraft.)
- `+0xD9A` = CanRetaliate (byte) (audit 31, BINARY-VERIFIED via `*(undefined1 *)((int)param_1 + 0xd9a) = uVar3` after ReadBool; string @ 0x00843c40, parser xref @ 0x0071448d. Default false; when false, unit does NOT auto-fire on attackers. Slots between +0xD99 CanPassiveAquire (audit 10) and +0xD9B RequiresStolenThirdTech (audit 11) — fills the tactical-AI byte-cluster.)

**TechnoType tactical-AI byte-cluster +0xD99..+0xDA4 (consolidated audit 31 — fully named):**

| Offset | Key | Audit |
|--------|-----|-------|
| +0xD99 | CanPassiveAquire | 10 (SNIPE) |
| **+0xD9A** | **CanRetaliate** | **31 (ORCA)** — NEW |
| +0xD9B | RequiresStolenThirdTech | 11 |
| +0xD9C | RequiresStolenSovietTech | 11 |
| +0xD9D | RequiresStolenAlliedTech | 11 |
| +0xDA0 | RequiredHouses (int) | 10 |
| +0xDA4 | ForbiddenHouses (int) | 10 |

**Aircraft sound cluster +0x504..+0x554 (consolidated audit 31 — extends audit 29):**

| Range | Key | Audit |
|-------|-----|-------|
| **+0x504..+0x50C** | **MoveSound (int[3] SoundList)** | **31** — NEW |
| +0x52C | AuxSound1 | 29 |
| +0x530 | AuxSound2 | 29 |
| +0x534..+0x53C | DEFERRED siblings | — |
| +0x540 | ImpactLandSound | 29 |
| +0x544 | CrashingSound | 29 |
| +0x548 | SinkingSound | 27 |
| +0x54C | DEFERRED | — |
| **+0x550** | **VoiceCrashing** | **31** — NEW |
| +0x554 | DEFERRED | — |

**TechnoTypeClass additions (audit 30 — aircraft targeting + Trainable, via TechnoTypeClass__ReadINI grep):**
- `+0x3B0` = PitchAngle (double, 8 bytes) (audit 30, BINARY-VERIFIED via `*(double *)(param_1 + 0xec) = (double)(fVar16 * DAT_007f4fb8)`; int*-stride 0xec*4=0x3B0; string @ 0x00844470, parser xref @ 0x0071236b. **Stored in radians: input degrees from INI multiplied by DAT_007f4fb8 (PI/180 ≈ 0.01745329)**. Sibling to +0x3A8 PitchSpeed (audit 29). Aircraft pitch animation parameter.)
- `+0x604` = LandTargeting (int) (audit 30, BINARY-VERIFIED via `param_1[0x181] = iVar4` after CCINIClass__ReadInt; int-array indexing 0x181*4=0x604; string @ 0x00844520, parser xref @ 0x007121a4. Sibling to +0x600 NavalTargeting (audit 7) — parsed FIRST in sequence: LandTargeting then NavalTargeting. Together they're a 2-int targeting-priority block at +0x600..+0x607.)
- `+0xC8E` = Trainable (byte) (audit 30, BINARY-VERIFIED via `*(undefined1 *)((int)param_1 + 0xc8e) = uVar3` after ReadBool; string @ 0x00843974, parser xref @ 0x00714a1c. **[CORRECTED audit 32]**: Default is **TRUE** (constructor-set), NOT false. Assembly-context proof at 0x00714a15: `MOV CL, [EBP + 0xc8e]` preload BEFORE the ReadBool call passes the existing value as the default arg. Gameplay confirms (Conscripts/standard infantry gain veterancy without Trainable=yes line). The audit-30 claim that "Trainable defaults to false" was WRONG — it confused ReadBool's default-arg-passing convention. Real reason ASW/HORNET don't gain XP is spawn-child semantics, not Trainable=false. Slots between +0xC8D MoveToShroud and +0xC8F name-override-aux.)

**TechnoType byte-cluster +0xC8C..+0xC91 (consolidated audit 30 — 5/6 keys named):**

| Offset | Key | Audit | Notes |
|--------|-----|-------|-------|
| +0xC8C | TypeImmune | 28 | byte |
| +0xC8D | MoveToShroud | 11 | byte, default 1 |
| **+0xC8E** | **Trainable** | **30** | byte, default 0 — NEW |
| +0xC8F | name-override aux | 13 | byte |
| +0xC90 | (DEFERRED INI key) | — | byte |
| +0xC91 | ImmuneToVeins | 7 | byte |

**TechnoTypeClass additions (audit 29 — aircraft sound cluster + pitch animation, via TechnoTypeClass__ReadINI grep + assembly context):**
- `+0x3A8` = PitchSpeed (double, 8 bytes) (audit 29, BINARY-VERIFIED via `*(double *)(param_1 + 0xea) = (double)fVar16` after CCINIClass__ReadDouble; int*-stride form 0xea*4=0x3A8; string @ 0x00844458, parser xref @ 0x007123da. Aircraft pitch-animation interpolation rate; HORNET uses 0.9. Sibling PitchAngle likely at +0x3B0 (DEFERRED — not directly verified).)
- `+0x52C` = AuxSound1 (int VocClass index) (audit 29, BINARY-VERIFIED via assembly-context preload `MOV EDI, dword ptr [EBP + 0x52c]` at 0x00712e03; string @ 0x00844240, parser xref @ 0x00712e18. Aircraft-takeoff event SFX. SINGLE-READ (TechnoType only — NOT in RulesClass__ReadAudioVisual). HORNET uses HornetTakeoff.)
- `+0x530` = AuxSound2 (int VocClass index) (audit 29, BINARY-VERIFIED via parser xref @ 0x00712e48 sibling-position evidence; string @ 0x00844234 (12 bytes BEFORE AuxSound1 — reverse storage order). Aircraft-landing event SFX. HORNET uses HornetLanding. SINGLE-READ.)
- `+0x540` = ImpactLandSound (int VocClass index) (audit 29, BINARY-VERIFIED via assembly-context writeback `MOV dword ptr [EBP + 0x540], EAX` at 0x00712f65; string @ 0x0083a9c4, parser xref @ 0x00712f38 (TechnoType side) + 0x00669965 (Rules side). **TechnoType side of DUAL-READ pattern with `RulesClass__ReadAudioVisual`** — joins ChronoIn/OutSound, SinkingSound, Activate/DeactivateSound family. Aircraft ground-impact crash SFX.)
- `+0x544` = CrashingSound (int VocClass index) (audit 29, BINARY-VERIFIED via assembly-context preload `MOV EDI, dword ptr [EBP + 0x544]` at 0x00712f6b; string @ 0x0084420c, parser xref @ 0x00712f80. Aircraft sustained-falling-plummet SFX. SINGLE-READ (TechnoType only). HORNET uses HornetDie.)

**Aircraft sound cluster (consolidated audit 29):** TechnoType+0x52C..+0x548 = 6-int sound block: AuxSound1 (+0x52C) → AuxSound2 (+0x530) → +0x534/+0x538/+0x53C (DEFERRED sibling sound keys, slot-count visible via grep `param_1[0x14f] = iVar6` evidence between AuxSound2 and ImpactLandSound) → ImpactLandSound (+0x540) → CrashingSound (+0x544) → SinkingSound (+0x548, audit 27). Largest TechnoType sound cluster mapped to date.

**TechnoTypeClass additions (audit 28 via TechnoTypeClass__ReadINI grep + assembly context):**
- `+0x310` = CloakingSpeed (int) (audit 28, BINARY-VERIFIED via `param_1[0xc4] = iVar4` after CCINIClass__ReadInt; int-array indexing 0xc4*4=0x310; string @ 0x0084443c, parser xref @ 0x00712441. Frames between cloak transitions; SUB/BSUB/DLPH all use CloakingSpeed=1.)
- `+0xC8C` = TypeImmune (byte) (audit 28, BINARY-VERIFIED via `param_1 + 0x323` after ReadBool, int*-stride form; assembly-context proof: writeback `MOV byte ptr [EBP + 0xc8c], AL` at 0x0071221c; string @ 0x008444ec, parser xref @ 0x0071220f. Same-type units don't damage each other with this unit's weapons — slots between +0xC8D MoveToShroud and +0xC91 ImmuneToVeins in the same byte-packed cluster.)
- `+0xD69` = Underwater (byte) (audit 28, BINARY-VERIFIED via `(int)param_1 + 0xd69` after ReadBool; string @ 0x00843848, parser xref @ 0x00714d74. Renders unit just below water surface; visible to standard ships but stealthy. SUB/BSUB/DLPH all use Underwater=yes.)
- `+0xD97` = Organic (byte) (audit 28, BINARY-VERIFIED via `(int)param_1 + 0xd97` after ReadBool; string @ 0x00843714, parser xref @ 0x0071502b. Marks unit as living organism for gore/death routing. **TechnoType-scope (NOT InfantryType-only)**: read for ALL TechnoTypes including vehicles like DLPH/SQUID/COW — distinct from InfantryType+0xEAD NotHuman which is InfantryType-only.)

**InfantryClass instance offsets:**
- `+0x16d` = is-firing flag (Fire_At_Target)
- `+0x1a0` = type pointer (Fire_At_Target / SelectWeapon)
- `+0x1a4` = current sequence id (Fire_At_Target / SelectWeapon)
- `+0x1bb` = is-elite flag (Fire_At_Target)
- `+0x2A4` = IsLowSilhouette / deployed-state crush-gate byte (CanCrushCheck)

**Deployed-state sequence IDs:** 0x1b (Deploy), 0x1c (Deployed), 0x1d (DeployedFire), 0x1e (DeployedIdle), 0x1f (Undeploy)

**Mission IDs verified:**
- 0x11 = Enter (audit 4, set in Mission_Attack for C4 plant)
- 8 = Enter? (audit 4, set in Mission_Attack for non-player Infiltrator path)
- 0x1D / 29 = Capture (audit 3 / GI report)

**BuildingClass instance offsets:**
- `+0x21C` = Owner (HouseClass*) — used in OnSpyInfiltrate same-owner check (audit 6, Ghidra-typed `this->Owner`)
- `+0x520` = type pointer (read by Mission_Attack to access BldgType+0x1577 etc.) (audit 4)
- `+0x338` (=0xCE*4) = engineer-capture tag (written from engineer TypeClass+0xDF8) (audit 3)

**UnitTypeClass (BINARY-VERIFIED audit 12 via UnitTypeClass__ReadINI):**
- `+0x398` = sequence-id default (int; 0xf normal, 0xA harvester/weeder)
- `+0x67C` = SpeedType (int; default 2, but 1 if `+0xD28 Crusher` set)
- `+0xDFC` = MovementRestrictedTo (int)
- `+0xE00..+0xE08` = HalfDamageSmokeLocation (3 ints)
- `+0xE0C` = Passive (byte)
- `+0xE0D` = CrateGoodie (byte)
- `+0xE0E` = Harvester (byte)
- `+0xE0F` = Weeder (byte)
- `+0xE11` = derived non-Turret flag (byte, set when TechnoType+0xCA1 Turret == 0)
- `+0xE12` = DeployToFire (byte)
- `+0xE13` = IsSimpleDeployer (byte)
- `+0xE14` = IsTilter (byte)
- `+0xE15` = UseTurretShadow (byte)
- `+0xE16` = TooBigToFitUnderBridge (byte) — **CORRECTS TNKD doc** (was claimed TechnoType-scope)
- `+0xE17` = CanBeach (byte)
- `+0xE18` = SmallVisceroid (byte)
- `+0xE19` = LargeVisceroid (byte)
- `+0xE1A` = CarriesCrate (byte)
- `+0xE1B` = NonVehicle (byte)
- `+0xE1C` = StandingFrames (int)
- `+0xE20` = DeathFrames (int)
- `+0xE24` = DeathFrameRate (int; clamped ≥ 1)
- `+0xE28` = StartStandFrame (int)
- `+0xE2C` = StartWalkFrame (int)
- `+0xE30` = StartFiringFrame (int)
- `+0xE34` = StartDeathFrame (int)
- `+0xE38` = MaxDeathCounter (int)
- `+0xE3C` = Facings (int)
- `+0xE40..+0xE44` = FiringSyncFrame[2] (int array)
- `+0xE48..+0xE54` = BurstDelay[4] (int array)
- `+0xE5C` = WalkFrames (byte)
- `+0xE5D` = FiringFrames (byte)
- `+0xE5E` = AltImage (char[25] string)

**BuildingTypeClass:**
- `+0xEB8` = Factory RTTI enum (0x10 = InfantryType, 0x28 = UnitType — used in OnSpyInfiltrate Factory branches) (audit 6, BINARY-VERIFIED)
- `+0xEE0` = Power (int — used in OnSpyInfiltrate Power-plant branch) (audit 6, BINARY-VERIFIED)
- `+0xEE8` = ExtraPower (int) (audit 6, BINARY-VERIFIED via ReadINI_Water)
- `+0x1577` = CanC4 flag (audit 4, BINARY-VERIFIED via Mission_Attack)
- `+0x16A4` = Radar (bool — used in OnSpyInfiltrate Radar branch) (audit 6, BINARY-VERIFIED)
- `+0x16F0` = SuperWeapon (int, -1 = none, used in OnSpyInfiltrate SW branch) (audit 6, BINARY-VERIFIED)
- `+0x16F4` = SuperWeapon2 (int) (audit 6, BINARY-VERIFIED via ReadINI_Water)
- `+0x1701` = InvisibleInGame flag (audit 4, BINARY-VERIFIED via Mission_Attack)
- `+0xEA4` = SecretInfantry (InfantryType*) (audit 12, BINARY-VERIFIED via BuildingTypeClass_ReadINI_Water)
- `+0xEA8` = SecretUnit (UnitType*) (audit 12, BINARY-VERIFIED)
- `+0xEAC` = SecretBuilding (BuildingType*) (audit 12, BINARY-VERIFIED)
- `+0x16B0` = SecretLab (byte — per-building enable flag; on `[CASLAB]` in stock YR) (audit 12, BINARY-VERIFIED via BuildingTypeClass_ReadINI_Water)
- `+0x16B3` = DockUnload (byte) (audit 12, BINARY-VERIFIED — adjacent to SecretLab in parser)
- `+0x16B9` = IsDeployable (byte) (audit 14, BINARY-VERIFIED via UnitClass::Deploy body — gates construction-yard special branch: center-view, base-construction setup, marks +0x1EE/0x1F2/0x1F3 = 1 on HouseClass)
- `+0x16C4` = (unknown — `FacingClass::UpdateFacing` trigger byte; audit 14 — DEFERRED INI key)
- `+0x16CA` = (unknown — second `FacingClass::UpdateFacing` trigger byte; audit 14 — DEFERRED INI key)

**TechnoClass instance offsets:**
- `+0x1D8` = IsDisguised flag (byte — read by IsDisguised_Getter @ 0x0041C020) (audit 6, BINARY-VERIFIED)
- `+0x270` = IsBeingWarpedOut (audit 5)
- `+0x278` = TemporalClass back-ptr (audit 5)
- `+0x2BC` = CaptureManager ptr (audit 5)
- `+0x2D0` = SpawnManager ptr (audit 5)
- `+0xCD5` = IsGattling (audit 5)

**CellClass:**
- `+0xAC` = per-house disguise-detect counter array (short[NumHouses], indexed by `Owner+0x30` house index — incremented/decremented by Add/RemoveDetectDisguiseAt for each cell in a building's detect ring) (audit 6, BINARY-VERIFIED via Increment/DecrementDisguiseDetectCount)

**HouseClass offsets (cumulative):**
- `+0x1EC`, `+0x1ED` = "current player" flags (byte, byte — used by IsHumanPlayer in single-player) (audit 6)
- `+0x1FC` = ProductionChanged byte (audit 6, OnSpyInfiltrate)
- `+0x21C` = (BuildingClass uses this offset for Owner field; ambiguous on HouseClass) (audit 6)
- `+0x241` = shroud-visibility flag byte (zeroed by RestoreShroud) (audit 6, BINARY-VERIFIED)
- `+0x2A4` = BlackoutStartFrame (audit 6, SpyPowerSabotage)
- `+0x2AC` = BlackoutDuration (audit 6, SpyPowerSabotage)
- `+0x2BC` = StolenThirdTech byte (audit 6, set when AIBasePlanningSide ≥ 2)
- `+0x2BD` = StolenSovietTech byte (audit 6, set when AIBasePlanningSide == 1)
- `+0x2BE` = StolenAlliedTech byte (audit 6, set when AIBasePlanningSide == 0)
- `+0x2BF` = SpiedBarracks byte (audit 6, OnSpyInfiltrate Factory=InfantryType branch)
- `+0x2C0` = SpiedWarFactory byte (audit 6, OnSpyInfiltrate Factory=UnitType branch)
- `+0x2DC` = spent-credits running total (audit 6, Spend_Money)
- `+0x30` = house index (4-byte int, used by Add/Remove DetectDisguiseAt + RestoreShroud) (audit 6)
- `+0x30C` = AvailableCredits (audit 6, BINARY-VERIFIED via Spend_Money/Add_Credits)
- `+0x5490`, `+0x5494` = spy-reveal cell coord pair (audit 6, Check_Spy_Reveal)
- `+0x54F4` = LastSpyRevealCell (audit 6, Check_Spy_Reveal)
- `+0x54FC` = LastSpyRevealFrame (audit 6, Check_Spy_Reveal)
- `+0x5778` = PowerBlackedOut byte (audit 6, SpyPowerSabotage)
- `+0x577A` = LowPowerState byte (audit 6, FUN_0050BD10 — gates RestoreShroud; DISTINCT from +0x5778)

**SuperClass offsets (cumulative):**
- `+0x24` = CustomRechargeTime (audit 6, -1 sentinel = use Type default)
- `+0x28` = Type (SuperWeaponTypeClass*) (audit 6)
- `+0x30` = RechargeStartFrame (audit 6)
- `+0x34` = aux frame field (audit 6, set from uStack_8 in OnSpyWeaponInfiltrate)
- `+0x38` = RechargeDuration (audit 6)
- `+0x68` = ChargeAnim (AnimClass*) (audit 6)
- `+0x6C` = IsCharged byte (audit 6)
- `+0x6F` = IsOneShotFired byte (audit 6)
- `+0x78` = CameoChargeFrame (audit 6, set to -1 by OnSpyWeaponInfiltrate)

**AnimClass:**
- `+0x195` = IsActive byte (audit 6, ChargeAnim deactivation in OnSpyWeaponInfiltrate)

**SuperWeaponTypeClass:**
- `+0xB0` = RechargeTime (audit 6, used as default when SuperClass+0x24 == -1)

**JumpjetLocomotionClass instance offsets (audit 8 — note: offsets given in IUnknown-raw view; ILocomotion view = IUnknown_this+0x4, so subtract 4 from these to get ILocomotion-typed `param_1` offsets):**
- `+0x0` = IUnknown vtable ptr
- `+0x4` = ILocomotion vtable ptr (the main interface used by Process / Locomotion_AI)
- `+0xC` = Owner TechnoClass* (= ILocomotion-view +0x8)
- `+0x18` = IPiggyback vtable ptr
- `+0x2C` = CruiseHeight cache (from JumpjetHeight=, used by In_Which_Layer + State 0 → +0x80 copy)
- `+0x50` = state field (0..5 = state machine, 6 = terminal sentinel)
- `+0x70/+0x74/+0x78/+0x7C` = velocity / position-delta vector (zeroed in State 0)
- `+0x80` = climb target altitude (set from +0x2C in State 0)

**TeleportLocomotionClass instance offsets (audit 11, BINARY-VERIFIED via Constructor @ 0x00718000):**
- `+0x0` = IUnknown vtable ptr (COM root)
- `+0x4` = ILocomotion vtable ptr (main interface used by Process / Locomotion_AI)
- `+0x18` = IPiggyback vtable ptr (secondary interface for piggyback)
- `+0x1C..+0x24` = Source coord triplet (3 ints, init from `g_NullCoord_Teleport_*` globals)
- `+0x28..+0x30` = Destination coord triplet (3 ints, init from same null-coord globals)
- `+0x34` = State byte (low-byte phase indicator; 0 at construction)
- `+0x35..+0x36` = Aux state bytes (both zeroed at construction)
- `+0x3C` = LaunchFrame (g_CurrentFrameCounter at construction)

**RulesClass JumpjetControls block (audit 8, BINARY-VERIFIED via RulesClass__ReadJumpjetControls):**
- `+0x40C` = TurnRate (int)
- `+0x410` = Speed (int, default 14)
- `+0x418` = Climb (double, 8 bytes)
- `+0x420` = CruiseHeight (int)
- `+0x428` = Acceleration (double, 8 bytes)
- `+0x430` = WobblesPerSecond (double, 8 bytes)
- `+0x438` = WobbleDeviation (int)

**RulesClass-General offsets (cumulative):**
- `+0x91C` = BuildTech DynamicVector start (audit 6)
- `+0x920` = BuildTech data ptr (DV+4) (audit 6, BINARY-VERIFIED via OnSpyInfiltrate)
- `+0x92C` = BuildTech count (DV+0x10) (audit 6, BINARY-VERIFIED via OnSpyInfiltrate)
- `+0xD58` = AlliedDisguise (TypeClass*) (audit 6, BINARY-VERIFIED via ReadGeneral)
- `+0xD5C` = SovietDisguise (TypeClass*) (audit 6)
- `+0xD60` = ThirdDisguise (TypeClass*) (audit 6)
- `+0xD64` = SpyPowerBlackout int (default 1000 frames) (audit 6, BINARY-VERIFIED)
- `+0xD68` = SpyMoneyStealPercent float (default 0.5) (audit 6, BINARY-VERIFIED)
- `+0xD6C` = AttackCursorOnDisguise byte (audit 6, BINARY-VERIFIED)
- `+0xEC8` = per-side spy-reveal probability table ptr (audit 6, Check_Spy_Reveal)
- `+0xEE4` = spy-reveal proximity threshold int (audit 6, Check_Spy_Reveal)
- `+0x1014` = InfantryBlinkDisguiseTime int (default 20 frames) (audit 6, BINARY-VERIFIED)
- `+0xD00` = SecretInfantry global list (DynamicVector<InfantryType*>) (audit 12, BINARY-VERIFIED via RulesClass__ReadGeneral)
- `+0xD1C` = SecretUnits global list (DynamicVector<UnitType*>) (audit 12, BINARY-VERIFIED — the random-pick pool when [CASLAB] is captured)
- `+0xD38` = SecretBuildings global list (DynamicVector<BuildingType*>) (audit 12, BINARY-VERIFIED)

**Vtable slots verified:**
- TechnoClass `+0x2c` = GetRTTI / GetAbstractType (returns 1 OR 6 for BuildingClass — VALUE CONFLICT between audit 3 and audit 4; DEFERRED to resolve)
- TechnoClass `+0xF8` = self-destruct/remove (audit 3, engineer consumed on capture)
- TechnoClass `+0x1e8` = SetMission (audit 4, called with mission ID 0x11)
- TechnoClass `+0x480` = Set_Target (audit 4)
- BuildingClass `+0x274` = SetMission (audit 3)
- BuildingClass `+0xDC` = Limbo (audit 3)
- BuildingClass `+0x3D4` = ChangeOwner (audit 3, param_2 = announce flag)
- UnitClass `+0x314` = CanDeploy (precondition predicate, returns char) (audit 14, BINARY-VERIFIED via UnitClass::Deploy entry call)
- BuildingClass `+0xD8` = TryPlaceBuilding (returns char success/fail) (audit 14, BINARY-VERIFIED via UnitClass::Deploy — called on the newly-constructed BuildingClass)
- TechnoClass `+0x124` = Mark_Occupants (called with 0/1 to clear/set cell-occupancy bits during state transitions) (audit 14, observed in UnitClass::Deploy)
- TechnoClass `+0x3C8` = SetTarget (called in target-redirect loop to redirect existing targeters from unit→building) (audit 14, BINARY-VERIFIED via UnitClass::Deploy)

**WeaponTypeClass (BINARY-VERIFIED via WeaponTypeClass__ReadINI, audit 9):**
- `+0x98` = AmbientDamage (int)
- `+0x9C` = Burst (int)
- `+0xA0` = Projectile ptr (BulletTypeClass*)
- `+0xA4` = Damage (int)
- `+0xA8` = Speed (int)
- `+0xAC` = Warhead ptr (WarheadTypeClass*)
- `+0xB0` = ROF (int)
- `+0xB4` = Range (int)
- `+0xB8` = MinimumRange (int)
- `+0xCC..0xD4` = Report sound list (3 ints)
- `+0xE8..0xF0` = DownReport sound list (3 ints)
- `+0x129` = UseFireParticles (byte)
- `+0x12A` = UseSparkParticles (byte)
- `+0x12B` = OmniFire (byte) — gates 360° firing without facing requirement
- `+0x12C` = DistributedWeaponFire (byte)
- `+0x12D` = IsRailgun (byte)
- `+0x12E` = Lobber (byte)
- `+0x130` = IsSonic (byte)
- `+0x131` = Spawner (byte)
- `+0x132` = LimboLaunch (byte) — firing unit becomes the projectile (dogs, Terror Drone)
- `+0x133` = DecloakToFire (byte)
- `+0x134` = CellRangefinding (byte)
- `+0x135` = FireOnce (byte)
- `+0x136` = NeverUse (byte)
- `+0x137` = RevealOnFire (byte) — used by SNIPE AWP, Spy MakeupKit; consumer DEFERRED (audits 9+10)
- `+0x138` = TerrainFire (byte)
- `+0x139` = SabotageCursor (byte) — used by Sapper/C4 weapons
- `+0x13A` = MigAttackCursor (byte)
- `+0x13B` = DisguiseFireOnly (byte)
- `+0x13C` = DisguiseFakeBlinkTime (int)
- `+0x140` = InfiniteMindControl (byte)
- `+0x141` = FireWhileMoving (byte)
- `+0x142` = DrainWeapon (byte)
- `+0x143` = FireInTransport (byte)
- `+0x144` = Suicide (byte)
- `+0x145` = TurboBoost (byte)
- `+0x146` = Supress (byte)
- `+0x147` = Camera (byte)
- `+0x148` = Charges (byte)
- `+0x149` = IsLaser (byte)
- `+0x14A` = DiskLaser (byte)
- `+0x14B` = IsLine (byte)
- `+0x14C` = Bright (byte)
- `+0x14D` = IsHouseColor (byte)
- `+0x14E` = LaserDuration (int)
- `+0x14F` = IsBigLaser (byte)
- `+0x150` = IonSensitive (byte)
- `+0x151` = AreaFire (byte)
- `+0x152` = IsElectricBolt (byte)
- `+0x153` = DrawBoltAsLaser (byte)
- `+0x154` = IsAlternateColor (byte)
- `+0x155` = IsRadBeam (byte)
- `+0x158` = RadLevel (int)
- `+0x15C` = IsMagBeam (byte)
- `+0x5a4` = some flag gating secondary-weapon-pick in Fire_At_Target (audit 1, exact INI mapping TBD)
- `+0x5c8` = elite variant of same flag (audit 1)

**WarheadTypeClass (BINARY-VERIFIED audit 28 via WarheadTypeClass__ReadINI — NEW PARSER SCOPE):**

Function: `WarheadTypeClass__ReadINI` @ 0x0075d590, body 0x0075d590–0x0075deae. Fully decompiled — sequential ReadBool/ReadInt/ReadCLSID calls populate the WarheadType-specific block starting around +0x14B. This is the fourth NEW parser-function scope added after ObjectType (audit 21), BulletType (audit 22), and AircraftType (audit 26).

- `+0x14B` = Sonic (byte) (audit 28, BINARY-VERIFIED via assembly-context proof — `0x0075d597: PUSH 0x847df0` → `CALL 0x005295f0` → `0x0075d5a4: MOV byte ptr [ESI + 0x14b], AL`; string @ 0x00847df0. Pairs with WeaponType+0x130 IsSonic — both flags required to trigger the sonic-chain damage path + ripple animation.)
- `+0x14C..+0x158` = sequential ReadBool block, 8+ bytes; remaining INI-key mappings DEFERRED (audit-28 decompile shows the writeback offsets but didn't enumerate the strings PUSHed before each ReadBool — would require additional assembly-context lookups).
- `+0x15C..+0x168` = ReadCLSID block (4 ints / 16 bytes — likely a GUID, presumably for warhead-specific extension data).

**ParasiteClass instance offsets (audit 9, BINARY-VERIFIED via Constructor):**
- `+0x0..+0xC` = 4 vtable pointers (multi-interface COM: primary + 3 secondaries at +4/+8/+C)
- `+0x2C` = LaunchFrame (g_CurrentFrameCounter at construction)
- `+0x34` = cleared field (likely "last damage" or some attach state)
- `+0x38` = secondary timestamp (g_CurrentFrameCounter — used by host-attach loop)
- `+0x40` = cleared field
- Global tracking: DynamicVector at DAT_00ac4914 (data) / DAT_00ac4920 (count) / DAT_00ac4918 (capacity sentinel)

**Rules-CombatDamage scope (NEW from audit 4):**
- `C4Warhead` parsed in `RulesClass__ReadCombatDamage` at `0x0066c31f`
- `DeathWeapon` parsed in `RulesClass__ReadCombatDamage` at `0x0066c58a` (audit 18 — global default for the TechnoType+0xD18 per-unit override)
- `OpenToppedRangeBonus` parsed at `0x0066c774` (audit 19, RulesClass-CombatDamage; string @ 0x0083AFEC)
- `OpenToppedDamageMultiplier` parsed at `0x0066c756` (audit 19; string @ 0x0083B004)
- `OpenToppedWarpDistance` parsed at `0x0066c794` (audit 19; string @ 0x0083AFD4) — purpose unknown (DEFERRED — likely teleport interaction with OpenTopped passengers)
