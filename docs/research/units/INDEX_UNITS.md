# Unit Reference Index

Master list of every entry in `[InfantryTypes]`, `[VehicleTypes]`, `[AircraftTypes]`,
`[BuildingTypes]` from `rulesmd.ini`. Source-of-truth is the INI; owner column is
the inferred Owner= for the dossier hierarchy. Filenames use the **INI ID** (not
display name) — e.g. `allied/E1.md`, not `allied/GI.md`.

Status: TODO / IN-PROGRESS / DONE / SKIP-DUPLICATE (Owner=) / N/A (terrain prop)

Priority gate (filled top-down):
1. Core combat units every match (basic infantry, MBT, basic defense)
2. Units with hardcoded behavior (Yuri Prime, Chrono Legionnaire, Mirage, IFV,
   Desolator, Boomer, Tanya, Engineer, MCV, Slave Miner, Battle Fortress,
   Prism Tank, etc.)
3. Side order: Allied → Soviet → Yuri → Civilian/Tech
4. Within a side: Infantry → Vehicle → Aircraft → Structure

---

## InfantryTypes

| ID         | Display                    | Side      | Notes              | Status |
|------------|----------------------------|-----------|--------------------|--------|
| E1         | G.I.                        | Allied    | Deploy-fortify     | DONE |
| E2         | Conscript                   | Soviet    | Basic Soviet inf   | DONE |
| SHK        | Tesla Trooper               | Soviet    | Tesla charge-coil  | DONE |
| ENGINEER   | Allied Engineer             | Allied    | Capture/repair     | DONE |
| JUMPJET    | Rocketeer                   | Allied    | Jumpjet locomotor  | DONE |
| GHOST      | Navy SEAL                   | Allied    | C4 building/sub    | DONE |
| YURI       | Yuri                        | Yuri      | Mind-control       | DONE |
| IVAN       | Crazy Ivan                  | Soviet    | Bomb plant         | DONE |
| DESO       | Desolator                   | Soviet    | Deploy radiation   | DONE |
| DOG        | Attack Dog (Soviet)         | Soviet    | Anti-inf bite      | DONE |
| CIV1       | Civilian Male 1             | Civilian  |                    | TODO |
| CIV2       | Civilian Female 1           | Civilian  |                    | TODO |
| CIV3       | Civilian Male 2             | Civilian  |                    | TODO |
| CTECH      | Civilian Technician         | Civilian  | Tech building exit | TODO |
| WEEDGUY    | Weed Guy / Yuri scientist?  | Civilian  |                    | TODO |
| CLEG       | Chrono Legionnaire          | Allied    | Freeze-erase       | DONE |
| SPY        | Spy                         | Allied    | Building infiltrate| DONE |
| CCOMAND    | Chrono Commando             | Allied    | Special/campaign   | DONE |
| PTROOP     | Psi-Corp Trooper            | Yuri      | Tech-steal: RequiresStolenThirdTech, MindControl | DONE |
| CIVAN      | Chrono Ivan                 | Soviet (tech-steal) | RequiresStolenSovietTech; Ivan bomb + Teleport locomotor | DONE |
| YURIPR     | Yuri Prime                  | Yuri      | AoE mind-control   | DONE |
| SNIPE      | Sniper                      | Allied    | One-shot infantry  | DONE |
| COW        | Cow                         | Civilian  |                    | TODO |
| ALL        | Herbert the Alligator       | Civilian  |                    | TODO |
| TANY       | Tanya                       | Allied    | C4 + sidearm       | DONE |
| FLAKT      | Flak Trooper                | Soviet    | AA/AG flak         | DONE |
| TERROR     | Terrorist (Cuban)           | Soviet    | Suicide bomb (RequiredHouses=Confederation) | DONE |
| SENGINEER  | Soviet Engineer             | Soviet    | Capture/repair     | DONE |
| ADOG       | Allied Attack Dog           | Allied    | Anti-inf bite      | DONE |
| VLADIMIR   | Vladimir (campaign)         | Soviet    | Campaign placeholder; Dummy voices, E1 anim | DONE |
| PENTGEN    | General (Pentagon evac)     | Allied    | Campaign placeholder; GI voices, GenSequence anim; **Owner= bug: Soviet houses** | DONE |
| PRES       | President Dugan             | Civilian  | Campaign           | TODO |
| SSRV       | Secret Service              | Civilian  | Campaign escort    | TODO |
| CIVA-CIVC  | Civilian variants           | Civilian  |                    | TODO |
| CIVBBP     | City civilian variants      | Civilian  |                    | TODO |
| CIVBFM     | Civilian variants           | Civilian  |                    | TODO |
| CIVBF      | Civilian variants           | Civilian  |                    | TODO |
| CIVBTM     | Civilian variants           | Civilian  |                    | TODO |
| CIVSFM     | Civilian variants           | Civilian  |                    | TODO |
| CIVSF      | Civilian variants           | Civilian  |                    | TODO |
| CIVSTM     | Civilian variants           | Civilian  |                    | TODO |
| POLARB    | Polar Bear                  | Civilian  |                    | TODO |
| JOSH      | Joshua (campaign/civ)       | Civilian  |                    | TODO |
| YENGINEER | Yuri Engineer               | Yuri      | Capture/repair     | DONE |
| GGI       | Guardian G.I.               | Allied    | Deploy AT          | DONE |
| INIT      | Yuri Initiate               | Yuri      | Psychic blast      | DONE |
| BORIS     | Boris                       | Soviet    | AK + airstrike     | DONE |
| BRUTE     | Yuri Brute                  | Yuri      | Anti-armor melee   | DONE |
| VIRUS     | Virus                       | Yuri      | Plague sniper      | DONE |
| CLNT      | President Clinton           | Civilian  | Campaign           | TODO |
| ARND      | Arnie (Schwarzenegger)      | Civilian  | Campaign easter egg| TODO |
| STLN      | Stalin (statue cameo)       | Civilian  | Campaign           | TODO |
| CAML      | Camel                       | Civilian  |                    | TODO |
| EINS      | Einstein                    | Civilian  | Campaign           | TODO |
| MUMY      | Mummy                       | Civilian  | Animal/easter egg  | TODO |
| RMNV      | Romanov                     | Soviet    | Campaign           | TODO |
| LUNR      | Lunar civilian/astronaut    | Civilian  | Campaign           | TODO |
| DNOA      | Dino A                      | Civilian  | Easter egg         | TODO |
| DNOB      | Dino B                      | Civilian  | Easter egg         | TODO |
| SLAV      | Slave (Yuri ore worker)     | Yuri      | Slaved=yes; SHOVEL melee + ore-harvest; dual-mode voices | DONE |
| WWLF      | Werewolf                    | Civilian  | Easter egg         | TODO |
| YDOG      | Attack Dog (Yuri-built, Soviet variant) | Yuri | Image=DOG, BadTeeth, RequiredHouses=YuriCountry | DONE |
| YADOG     | Allied Attack Dog (Yuri-built) | Yuri | Image=ADOG, GoodTeeth, RequiredHouses=YuriCountry | DONE |

## VehicleTypes

| ID        | Display                       | Side       | Notes                  | Status |
|-----------|-------------------------------|------------|------------------------|--------|
| AMCV      | Allied MCV                    | Allied     | Deploy → GACNST; build-tree entry; OmniCrushResistant | DONE |
| HARV      | War Miner                     | Soviet     | Soviet harvester (20mmRapid turret, UnloadingClass=HORV) — **index correction: NOT Chrono Miner** | DONE |
| APOC      | Apocalypse Tank               | Soviet     | Tier-4 dual-weapon AG+AA; Image=MTNK; Explodes=yes; TargetLaser=yes | DONE |
| HTNK      | Rhino Heavy Tank              | Soviet     | Soviet MBT; 120mm + RHINAPE elite; MovementZone=Destroyer | DONE |
| SAPC      | Amphibious Transport ("Armored Transport") | Soviet | **INDEX CORRECTION: Soviet not Allied** (Prerequisite=NAYARD, Owner=Russians/Confederation/Africans/Arabs); Image=TRS shared with Yuri YHVR; Passengers=12 + SizeLimit=6 (largest in game, **fits an MCV**); Hover locomotor + MovementZone=Amphibious (NOT Destroyer because "I can't have a destroyer zone without a weapon!" — Westwood engine constraint); Strength=300/Cost=900; **no weapon** (;Primary=M60 commented); Naval=yes; DeployTime=.022 fast cycle | DONE |
| CAR       | Civilian Car                  | Civilian   | Driveable              | TODO |
| BUS       | Civilian Bus                  | Civilian   |                        | TODO |
| WINI      | Winnebago                     | Civilian   |                        | TODO |
| PICK      | Pickup Truck                  | Civilian   |                        | TODO |
| MTNK      | Grizzly Battle Tank           | Allied     | Allied MBT; 105mm + GRIZAPE elite; Image=GTNK | DONE |
| HORV      | War Miner (unloading form)    | Soviet     | UnloadingClass for HARV; visual-only during dock-unload | TODO |
| TRUCKA    | Truck variant                 | Civilian   |                        | TODO |
| TRUCKB    | Truck variant                 | Civilian   |                        | TODO |
| CARRIER   | Aircraft Carrier              | Allied     | Spawns HORNET (NOT ASW — correction); reusable aircraft return-and-reload spawner (NOT MissileSpawn); Hornet has own veterancy + ElitePrimary=HornetBombE swap | DONE |
| V3        | V3 Rocket Launcher            | Soviet     | Spawns=V3ROCKET (SpawnReloadRate=0 one-shot); NoSpawnAlt voxel swap | DONE |
| ZEP       | Kirov Airship                 | Soviet     | Tier-3 heavy bomber; Jumpjet locomotor with JumpjetHeight=750 (game highest); BalloonHover=yes + Crashable=yes; BlimpBomb 250dmg vertical-drop bombs (Projectile=BlimpBombP, Vertical=yes); JumpjetNoWobbles=yes; OmniFire on weapon; Strength=2000; Owner= Russians/Confederation/Africans/Arabs (NOT YuriCountry); CreateSound=KirovCreated Type=global broadcast; **no DeathWeapon=** (loop-prompt error — BlimpBombEffect is DISK's death weapon, not Kirov's) | DONE |
| DRON      | Terror Drone                  | Soviet     | Parasite warhead (DroneJump+LimboLaunch); ParasiteClass attach state machine | DONE |
| HTK       | Flak Track (Half Track)       | Soviet     | 5-passenger transport, dual AG+AA flak — **index correction: Soviet, not Allied IFV** | DONE |
| DEST      | Destroyer                     | Allied     | Spawns ASW heli; dual-weapon (155mm primary + Osprey secondary); Sensors=yes (anti-sub detection, +SensorsSight=8); ElitePrimary=155mmE Burst=2; NoSpawnAlt to DESTWO voxel | DONE |
| SUB       | Typhoon Attack Submarine      | Soviet     | Tier-2 single-weapon torpedo sub; Strength=600/Cost=1000 (half of BSUB); Prerequisite=NAYARD only (earliest sub — NO RADAR); SubTorpedo Damage=100 single-shot APSplash, ElitePrimary Burst=2; DecloakToFire=no; Cloakable+CloakingSpeed=1 fast cloak; Underwater=yes; Sensors+SensorsSight=7; NavalTargeting=5/LandTargeting=1; **NO Unnatural=yes** (Squid grabs SUB, vs BSUB which gets punched); MoveSound borrows Squid audio; SubFear silent-block VoiceFeedback | DONE |
| AEGIS     | Aegis Cruiser                 | Allied     | Tier-7 naval AA specialist; Strength=800/Armor=light/Cost=1200; Medusa AA-only missile (Range=12 longest AA in game, AG=no projectile constraint); SAMWH 0% vs wood/steel/concrete; ElitePrimary=MedusaE Burst=2 ROF=5 Range=14 (6× DPS elite); **RadialFireSegments=10** (360° split for radial launch); **DistributedFire=yes** (multi-target round-robin); **ToProtect=yes** (AI high-value hint); **SinkingSound DUAL-READ Rules+TechnoType**; AegisAttackCommand has 8-sample voice pool; disabled Ammo system (dormant but engine-supported); 4 new cheat-sheet entries | DONE |
| LCRF      | Allied Landing Craft          | Allied     | **INDEX CORRECTION: Allied transport, NOT Soviet Sea Scorpion** (Owner=Allied 5 houses, Prerequisite=GAYARD, Passengers=12, no weapon); third member of amphibious transport trio with SAPC/YHVR; **Armor=light** (vs SAPC/YHVR heavy — Allied weaker); **TechLevel=4** (vs SAPC/YHVR TL=2 — Allied 2-tier delay); ThreatPosed=3 (lowest); StupidHunt=yes; Size=16; voice=HoverAllied | DONE |
| DRED      | Dreadnought                   | Soviet     | Spawns DMISL; salvo of 2 hardcoded RocketStruct slot DMisl* (DMislWarhead/DMislEliteWarhead Rules-global); NoSpawnAlt swap to DREDWO voxel | DONE |
| SHAD      | Nighthawk Transport (BlackHawk) | Allied   | Tier-7 5-passenger jumpjet transport; Speed=14, JumpjetHeight=500, BlackHawkCannon (35dmg ROF=40 OmniFire QuadShell, 8-dir MGUN anim); Trainable=yes (rare for transport); SizeLimit=2; HoverAttack=yes; PreventAttackMove=yes + CanPassiveAquire=no = fully passive transport; RadarInvisible=yes at **ObjectType scope** (broader than TechnoType, same as NoSpawnAlt); EnterTransportSound/LeaveTransportSound TechnoType; DisableVoxelCache/DisableShadowCache art-side performance flags | DONE |
| SQD       | Giant Squid                   | Soviet     | Tier-9 organic naval predator; Strength=200/Cost=1000; SHP-rendered (Voxel=no, WalkFrames=20/FiringFrames=16); Primary=SquidGrab LimboLaunch SQDJUMP carrier + ParasitePlus warhead (**Culling=yes** kills outright at Red HP, **Paralyzes=32767** frozen-while-grabbed); Secondary=SquidPunch (basic InvisibleLow no-AA, **elite InvisibleAll restores AA capability**); ROF=99 "ignored by special Squid code" — hardcoded damage tick rate; SuppressionThreshold=250 grapple-break threshold; Cloak triple-stack with **CloakingSpeed=5 slow**; ImmuneToPsionics+Parasiteable=no; **NoShadow=yes**; **Bombable=yes ObjectType** (4 new cheat-sheet entries) | DONE |
| DLPH      | Dolphin                       | Allied     | Tier-5 anti-sub sonic specialist; Strength=200/Cost=500/Speed=8 (cheapest+fastest naval); **SHP-rendered (Voxel=no)** with WalkRate=4/IdleRate=8 + WalkFrames=6/FiringFrames=6 (verbatim "sprite is terribly hack"); SonicZap Damage=4+AmbientDamage=10 with IsSonic=yes (chain damage along sonic line); Elite Burst=2 ROF=80 (~6× DPS); **Organic=yes** + **NotHuman=yes** + **TypeImmune=yes** (4 new cheat-sheet entries); Cloakable+Underwater+Sensors stealth triple-stack; submarine locomotor reuse | DONE |
| SMCV      | Soviet MCV                    | Soviet     | Deploys into NACNST; near-perfect mirror of AMCV (same Strength=1000/Armor=heavy/Speed=4/Cost=3000/TechLevel=10); Prerequisite=NAWEAP,NADEPT; CrateGoodie=yes; three-tier crush system (Crusher=yes + OmniCrushResistant=yes); Crewed=yes (Soviet ejects E2); SpecialThreatValue=1 AI hint; Trainable=no; ZFudgeTunnel=15 (TS-legacy dormant) | DONE |
| TNKD      | Tank Destroyer                | Allied     | German MBT, AT-only; Secret Lab pickup + CrateGoodie | DONE |
| HOWI      | Prism Tank                    | Allied     | Bounce/chain prism     | TODO |
| TTNK      | Tesla Tank                    | Soviet     | TankBolt (Electric warhead) + elite Electricbounce chain-lightning; RequiredHouses=Russians; SecretUnits pool | DONE |
| HIND      | "Hind Transport" (disabled)   | Soviet (cut) | **INDEX CORRECTION: NOT Flak Track** (Flak Track is HTK, done); HIND is TechLevel=-1 cut/disabled airborne transport, JumpJet=yes, BlackHawkCannon weapon. Real ID for it is "Hind Transport" per rulesmd. SKIP-DUPLICATE or document as cut content | SKIP-DUPLICATE |
| LTNK      | Lasher Light Tank             | Yuri       | Yuri's main MBT; Strength=300/Speed=7/Sight=8/Cost=700/TechLevel=2; ATGUN (Dmg=65 AP warhead, weak vs inf 25%); ElitePrimary=ATGUNE Burst=2 + RHINAPE (100% Verses vs armor) — ~2.4× firepower upgrade at elite; Accelerates=false (instant Speed ramp); Crewed=no (no infantry eject); BuildTimeMultiplier=1.5; no OmniCrushResistant (vulnerable to Apoc) | DONE |
| CMON      | Chrono Miner archetype dup    | -          | duplicate              | SKIP-DUPLICATE |
| CMIN      | Chrono Miner                  | Allied     | Allied harvester (no turret, teleports home >50 cells, UnloadingClass=CMON) | DONE |
| SREF      | Prism Tank                    | Allied     | Tier-8 siege artillery; Strength=150/Armor=light/Cost=1200; uses TurretCount=4 + WeaponCount=1 + Weapon1=Comet "abusive" multi-turret syntax to enable IsChargeTurret=true; Comet IsLaser+IsHouseColor LaserDuration=15; ShrapnelWeapon chain (LargeCometP→CometFragment×5, elite SuperCometP→SuperCometFragment×5→CometFragment×3 = 21 hits/shot); CometWH 200% vs wood/steel/concrete (anti-structure); BFRT artmd Image=SREF (shared voxel) | DONE |
| XCOMET    | Dummy comet                   | Internal   | Weapon-host hack       | TODO |
| HYD       | Sea Scorpion                  | Soviet     | **INDEX CORRECTION: Soviet Sea Scorpion, NOT Allied Hydrofoil** (Name=Sea Scorpion, Owner=Russians/Confederation/Africans/Arabs, Prerequisite=NAYARD,NARADR); Soviet naval AA counterpart to AEGIS — closes naval AA pair; **dual-weapon** Primary=FlakTrackGun (AG, Range=5) + Secondary=FlakWeapon (AA, Range=12, shared with NAFLAK Flak Cannon); Strength=400/Cost=600 (half of AEGIS); Speed=8 (2× AEGIS); both weapons elite-swap Burst=2; MovementRestrictedTo=Water + MovementZone=Water double constraint; FlakScatter+Inaccurate BulletType flags (NEW cheat-sheet) | DONE |
| MGTK      | Mirage Tank                   | Allied     | DisguiseWhenStill (tree disguise); Image=RTNK; EliteSecondary-without-Secondary quirk | DONE |
| FV        | Multi-Gunner IFV              | Allied     | 1-passenger weapon-swap; TurretCount=4, WeaponCount=17 — **index correction: NOT Battle Fortress** | DONE |
| DeathDummy| Dummy (DeathWeapon host)      | Internal   |                        | TODO |
| VLAD      | Vladimir mobile?              | Soviet     | Campaign               | TODO |
| DTRUCK    | Demolitions Truck             | Soviet     | Suicide=yes weapon + Explodes=yes + DeathWeapon=Demobomb; RequiredHouses=Africans; SecretUnits pool | DONE |
| PROPA     | Propaganda Truck?             | Civilian   |                        | TODO |
| CONA      | Concrete (engineer-related)?  | Civilian   |                        | TODO |
| COP       | Police car                    | Civilian   |                        | TODO |
| EUROC     | Civilian European car         | Civilian   |                        | TODO |
| LIMO      | Limousine                     | Civilian   |                        | TODO |
| STANG     | Mustang (civilian)            | Civilian   |                        | TODO |
| SUVB      | SUV black                     | Civilian   |                        | TODO |
| SUVW      | SUV white                     | Civilian   |                        | TODO |
| TAXI      | Taxi                          | Civilian   |                        | TODO |
| PTRUCK    | Civilian Pickup               | Civilian   |                        | TODO |
| CRUISE    | Cruise ship?                  | Civilian   |                        | TODO |
| TUG       | Tug boat                      | Civilian   |                        | TODO |
| CDEST     | Civilian destroyer?           | Civilian   |                        | TODO |
| YHVR      | Yuri Hover Transport          | Yuri       | Near-mirror of Soviet SAPC; UIName=Name:SAPC (shared display); Owner=YuriCountry; Prerequisite=YAYARD; Passengers=12, SizeLimit=6 (carries Yuri MCV/PCV); Hover locomotor + Amphibious zone; **StupidHunt=yes** (Yuri-unique: weaponless Hunt-mission bypass — runs toward player base instead of freezing in scan-fail loop); Trainable=no explicit; **`;Image=TRS` COMMENTED — uses own yhvr.vxl**, NOT shared with SAPC (corrects SAPC doc claim) | DONE |
| PCV       | Yuri Construction Vehicle (MCV) | Yuri     | Deploys into YACNST; near-mirror of SMCV/AMCV with 3 Yuri-specific diffs: **Sight=8** (vs 6), **Prerequisite=YAWEAP,YAGRND** (Grinder instead of Service Depot), **Owner=YuriCountry** (single house, no sub-factions); Crewed=yes ejects Initiates; closes MCV trio | DONE |
| SMIN      | Slave Miner (vehicle)         | Yuri       | DeploysInto=YAREFN; SlaveManagerClass at TechnoClass+0x2D8; Enslaves=SLAV, SlavesNumber=5, SlaveRegenRate=500, SlaveReloadRate=25; Storage=20 pre-deploy buffer; brain-transplant SlaveManager re-bind on deploy; OmniCrushResistant + ImmuneToPsionics economic-protection flags; ElitePrimary=20mmRapidE cannon-shell upgrade | DONE |
| SMON      | "ZZZ Useless; Slave Miner(noback)" placeholder | Yuri (dead) | TechLevel=-1, AllowedToStartInMultiplayer=no, Image=CMON, teleport locomotor — vestigial cut entry. **NOT the deployed Slave Miner** (that's YAREFN, BuildingTypes) | SKIP-DUPLICATE |
| YCAB      | Yuri civilian variant?        | Civilian   |                        | TODO |
| YTNK      | Gattling Tank                 | Yuri       | IsGattling=yes multi-stage system (TechnoType +0xCD5/+0xCD8/+0xD0C/+0xD10); WeaponStages=3, Stage1/2/3=200/400/600 (Elite halved 100/200/300); RateUp=1/RateDown=50 grace-period mechanic; CurrentGattlingStage/GattlingValue per-instance accumulator; ground stages upgrade warhead GattWH→SA→SSA, air stages upgrade dmg+ROF 25/16→30/8→40/4; vestigial GattlingCycleCount | DONE |
| BFRT      | Battle Fortress               | Allied     | 5-passenger OpenTopped crusher; OmniCrusher; CrusherAll zone; Image=SREF | DONE |
| TELE      | **Magnetron** (Yuri tier-3)   | Yuri       | INDEX CORRECTION: this is the Magnetron, NOT a chrono trooper transport. Hardcoded LocomotorBeam warhead (IsLocomotor=yes + Locomotor GUID swap, shares engine path with Chronosphere) + MagneShake secondary anti-building; IsMagBeam visual; Bunkerable=no; CanPassiveAquire=no | DONE |
| CAOS      | Chaos Drone                   | Yuri       | Psychedelic=yes warhead triggers berserk_flag (+0x298) + berserk_timer (+0x29C) state machine on targets; alliance-bypass via Scan_Cell_For_Target; BerserkFriendly=yes self-immunity; VirtualScanner NeverUse=yes scan extender; Trainable=no (no veterancy) | DONE |
| DDBX      | DeathDummy box?               | Internal   |                        | TODO |
| BCAB      | Civilian cab                  | Civilian   |                        | TODO |
| BSUB      | Boomer Submarine              | Yuri       | Tier-2 dual-weapon naval; Strength=1200 (highest naval HP); Primary=BoomerTorpedo Burst=2 (elite Burst=4), DecloakToFire=no (fires from cloak); Secondary=CruiseLauncher Range=20 Burst=2 spawns CMISL (SpawnsNumber=2, SpawnReloadRate=0 one-shot suicide missiles); NavalTargeting=7/LandTargeting=2 dual-priority bias; Underwater=yes + Unnatural=yes (Squid punches instead of grabs); Cloakable+Sensors+SensorsSight=8; Sub locomotor GUID (...74E1); SecondSpawnOffset=-70,0,0 separates burst missile positions; VoiceSecondaryWeaponAttack split Water/Land | DONE |
| SCHP      | Soviet Siege Chopper          | Soviet     | **INDEX CORRECTION: Soviet attack/deploy helicopter, NOT Yuri sub-pen** (Owner=Russians/Confederation/Africans/Arabs); Tier-7 deployable artillery helicopter; **IsSimpleDeployer=yes + UnloadingClass=SCHD + DeployingAnim=SCHPDEPL** (unit-to-unit transformation trio — but SCHD is vestigial per iter 77!); JumpJet vehicle (not AircraftType); dual-weapon: Primary=BlackHawkCannon + Secondary=160mm (Range=12, **SCHOPWH Deform=15% terrain deformation**); Lobber=no WeaponType; VoiceSecondaryWeaponAttack air/land split; "Seige" typo throughout; 3 new cheat-sheet entries; **OPEN: deploy entity-swap claim needs Ghidra verification** | DONE |
| JEEP      | Allied IFV alt?               | -          |                        | TODO |
| MIND      | Master Mind                   | Yuri       | InfiniteMindControl=yes weapon; OverloadCount/Damage/Frames self-damage; 5 AlternateFLH beam lines | DONE |
| DISK      | Floating Disc                 | Yuri       | Dedicated DiskLaserClass (0x40 bytes, vtable@0x007E5FB8) for ring effect; DrainWeapon=yes hardcoded credit/power siphon vs buildings; VehicleType+ConsideredAircraft hybrid; JumpjetLocomotion+BalloonHover; DeathWeapon=BlimpBombEffect ×.1 modifier | DONE |
| UTNK      | "ZZZ Not Used" placeholder    | Soviet (vestigial) | TechLevel=-1 dead entry; Image=HTNK, Primary=Comet, MovementZone=Destroyer; **NOT the Magnetron** (Magnetron is [TELE]) | SKIP-DUPLICATE |
| ROBO      | Robot Tank                    | Allied     | Tier-2 Allied hover anti-armor; Hover locomotor (...742 GUID) + AmphibiousDestroyer MovementZone; Speed=10 (game-fastest non-air); Strength=180/Cost=600/BuildTimeMultiplier=1.3; Prerequisite=GAWEAP,GAROBO; **PoweredUnit=yes** state machine (deactivates if no power or GAROBO destroyed) — VoiceSelectDeactivated + ActivateSound/DeactivateSound (DUAL-READ Rules+TechnoType); ImmuneToPsionics+Radiation+Veins triple-stack (Yuri-counter); Trainable=no (vestigial Vet/Elite lists); Robogun=ATGUN clone | DONE |
| YDUM      | Yuri dummy                    | Internal   |                        | TODO |
| SCHD      | "ZZZ Deployed Soviet Siege Chopper" (vestigial) | Soviet (vestigial) | **INDEX CORRECTION: NOT Schoolbus** — SCHD is the SCHP deploy-target placeholder (Name=ZZZ prefix, TechLevel=-1, Primary=BlackHawkCannon but **NO Secondary** despite DeployFire=yes pointing to it); bidirectional UnloadingClass=SCHP pair; multiple vestigial-content indicators (borrowed BlackOps* crash sounds, simpler artmd, no AltCameo); 3 new cheat-sheet entries: DeployFire+DeployFireWeapon+DeployToLand (all TechnoType). **Likely never instantiated in actual gameplay**; SCHP-deploy may stay at SCHP-level — Ghidra trace required to confirm | DONE |
| DOLY      | Dolly (camera car)?           | Civilian   |                        | TODO |
| CBLC      | Cable car?                    | Civilian   |                        | TODO |
| FTRK      | Food truck?                   | Civilian   |                        | TODO |
| AMBU      | Ambulance                     | Civilian   |                        | TODO |
| CIVP      | Civilian pickup variant       | Civilian   |                        | TODO |

## AircraftTypes

| ID         | Display                  | Side    | Notes                | Status |
|------------|--------------------------|---------|----------------------|--------|
| APACHE     | Hind/Apache              | Soviet  | Gatling rotor        | TODO |
| ORCA       | Intruder                 | Allied (non-Korean) | **INDEX CORRECTION: NOT a Nighthawk alias / NOT unused — ORCA is the Intruder, an active Allied non-Korean fighter** (Owner=British,French,Germans,Americans; ForbiddenHouses=Alliance excludes Korea — Korea gets BEAG); Image=FALC artmd redirect; ;Selectable-style commentary absent (no `;Selectable=no`); Tier-3 fighter pair-closer with BEAG (BEAG=Korea-only, ORCA=non-Korea-only — partition mutually exclusive); **AllowedToStartInMultiplayer=no** explicit (despite being active in skirmish — TechLevel=3, Prerequisite=RADAR governs build availability); Primary=Maverick (Dmg=150, Range=6, ORCAAP warhead PenetratesBunker=yes) + ElitePrimary=MaverickE (Dmg=300, Range=9); Ammo=1 + AirportBound=yes + Landable=yes single-shot reload-at-airport cycle; Dock=GAAIRC,AMRADR; Aircraft locomotor GUID `{4A582746-9839-11d1-B709-00A024DDAFD1}` (...746); Fighter=yes (AircraftType-scope field); CanPassiveAquire=no + CanRetaliate=no + PreventAttackMove=yes triple-disable script-only control (verbatim Westwood "Won't try to pick up own targets"/"Won't fire back when hit"); active AuxSound1=IntruderTakeOff + AuxSound2=IntruderLanding; AmerParaDropInf-style country partition mirrored at fighter level (Korea↔non-Korea); PadAircraft=ORCA,BEAG Rules-global tag (NEW cheat-sheet — Rules-General scope); ForbiddenHouses TechnoType-scope (NEW cheat-sheet); 3 new Ghidra cheat-sheet entries (PadAircraft Rules-General + ForbiddenHouses TechnoType + MoveToShroud TechnoType) | DONE |
| HORNET     | Hornet (Carrier-spawned) | Allied  | Carrier's reusable strike plane; **return-to-dock pattern** (Spawned=yes but NOT MissileSpawn=yes); Strength=75/Ammo=1; HornetBomb basic + HornetBombE elite (parent CARRIER's veterancy triggers child weapon swap); **HornetCollision Secondary = crash-transform projectile** ("crashing Hornet turns into this bullet at the last second" verbatim); AircraftLocomotion (...746); **active AuxSound1=HornetTakeoff + AuxSound2=HornetLanding** (canonical example of the takeoff/landing SFX system, most other units have these commented); Westwood `;Selectable=no` bug commentary (selectable in shipped YR because unselectable breaks landing); new cheat-sheet entries: Landable (AircraftType), AuxSound1 (TechnoType) | DONE |
| V3ROCKET   | V3 Rocket projectile     | Soviet  | V3 Launcher's spawn-child kamikaze missile; **Spawned=yes + MissileSpawn=yes** (one-shot suicide); RocketLocomotion GUID ({B7B49766-...}) — **6th distinct locomotor type**; **no weapons defined** — damage from Rules-global `V3Warhead=V3WH` lookup; Strength=50, Armor=special_2 (V3WH 0% vs special_2 = rockets don't damage other rockets); V3WH Deform=10% DeformThreshhold=300 (rare cratering), ProneDamage=70% "Presumes air burst"; Selectable=no EXPLICITLY set (confirms HORNET's bug only affects landing units); active AuxSound1=V3Attack + commented AuxSound2 (no landing event); Trainable=no + DontScore=yes + Explodes=no + NoShadow=yes triple-flag "transient projectile" semantics | DONE |
| ASW        | Osprey ASW (Destroyer-spawned) | Allied | Destroyer's return-to-dock anti-sub helicopter; Strength=30 (lowest documented HP); Cost=50; ASWBomb DepthCharge anti-naval (APSplash warhead, shared with SubTorpedo); **NO ElitePrimary** (Destroyer's elite doesn't upgrade Osprey weapon — vs HORNET where CARRIER elite swaps to HornetBombE); ASWCollision crash-transform Secondary (same pattern as HornetCollision); active AuxSound1/AuxSound2 cycle; vospatta sample shared with HornetAttack; NavalTargeting=2 (lowest documented); `[OspreyCollision]` silent block; closes Allied return-to-dock spawn pair (HORNET ✓ + ASW ✓); AuxSound2 new cheat-sheet entry | DONE |
| DMISL      | Dread Missile (Dreadnought-spawned) | Soviet | Dreadnought's spawn-child kamikaze missile; near-mirror of V3ROCKET with 5 differences: Sight=0, Speed=18, ROT=4, **FlyBack=true** (NOT on V3ROCKET — correlates with parent Burst=2), AuxSound1=DreadnoughtAttack; **TWO Rules-global warheads**: DMislWarhead=DMISLWH (basic, CellSpread=1.5) + DMislEliteWarhead=DMISLEWH (elite, CellSpread=3 + AnimList=MININUKE mini-nuke mushroom cloud visual); DMISLWH anti-armor (80% vs medium/heavy, weaker vs structures); no weapons defined; closes part of kamikaze missile trio (V3ROCKET ✓ + DMISL ✓ + CMISL pending) | DONE |
| PDPLANE    | Paradrop Plane (Cargo Plane) | Generic | Universal Owner (all 10 countries); spawn-child for ParaDropSpecial + AmericanParaDropSpecial superweapons; **3rd spawn-child paradigm: drop-and-exit** (Spawned=yes but NOT MissileSpawn AND Landable=no — flies in, drops cargo, exits map edge); Strength=400 (most durable spawn-child); **Primary=ParaDropWeapon is DUMMY** ("Doesn't really fire it" verbatim Westwood comment); Category=AirLift (vs combat AirPower); PitchAngle=0 flat-flight requirement for paradrop drop; Rules-global country tables AmerParaDropInf=E1×8 / AllyParaDropInf=E1×6 / SovParaDropInf=E2×9 / YuriParaDropInf=INIT×6; ParadropRadius=1024 (4-cell drop threshold); Type=ParaDrop vs Type=AmerParaDrop dispatcher distinction | DONE |
| BEAG       | Black Eagle              | Allied (Korea) | Tier-3 strike fighter; **RequiredHouses=Alliance** (Korea-exclusive — joins TNKD/DTRUCK/TTNK faction-exclusive roster); Strength=200/Cost=1200; **Ammo=1 + AirportBound=yes** single-shot reload-at-GAAIRC cycle (no airport=crash); Maverick2 Damage=200 Range=6 (elite 400/Range=9); ORCAAP warhead PenetratesBunker=yes; PreventAttackMove+CanPassiveAquire+CanRetaliate=no triple-disable script-only control; **NEW SCOPE DISCOVERY: AircraftTypeClass__ReadINI** (0x0041cxxx range) — Fighter and AirportBound are AircraftType-only fields; Aircraft locomotor GUID (...746) is 5th distinct locomotor | DONE |
| CARGOPLANE | Transport Plane          | Civilian/Generic | Image=PDPLANE redirect sibling (shared voxel); UIName=Name:PDPLANE shared CSF label; **NOT Spawned=yes** (key diff vs PDPLANE — bypasses SpawnManager, engine-direct spawn via AI campaign reinforcement scripts); **NO Primary weapon at all** (PDPLANE has dummy ParaDropWeapon; CARGOPLANE has none); Category=AirPower (vs PDPLANE's AirLift — possible Westwood typo/oversight); `;Selectable=no` commented (active on PDPLANE); universal Owner; Strength=400; likely vestigial/campaign-only — no observable usage in standard skirmish; closes cargo-plane pair (PDPLANE ✓ + CARGOPLANE ✓) | DONE |
| BPLN       | Soviet MIG (Boris Attack Plane) | Soviet | **INDEX CORRECTION: Soviet, NOT Allied B-2** (Name=Soviet MIG; artmd "Boris Attack Plane"); summoned by Boris's per-techno **AirstrikeTeam=2/EliteAirstrikeTeam=4** hardcoded airstrike system (5th spawn pathway — Airstrike paradigm); **first spawn-child with REAL combat weapon** — Primary=Maverick3 Damage=750! Burst=2 (4 missiles/strike basic, 8 elite!); ElitePrimary=Maverick3E (Damage=400/Range=9, switches to ORCAAP PenetratesBunker warhead); FlyBy=true; DeathWeapon=BlimpBomb ×.1 (verbatim "needs death weapon or one laser blast's worth of crash damage — this gives control"); Fighter=yes; vblelo* audio shared with BEAG; Remapable=yes (only spawn-child with house tint) | DONE |
| SPYP       | Soviet Spy Plane         | Soviet (NARADR superweapon) | **INDEX CORRECTION: Soviet, NOT Allied** (provider=NARADR Radar Tower, not GASPYSAT which uses passive SpySat=yes); drop-and-exit reveal-shroud aircraft; Strength=600; **Primary=SpyCameraWeapon with repurposed Damage=6 ("range of shroud to reveal") and Range=20 ("howfar away to start revealing")**; **FlyBy=true** ("Don't slow down over your target" — NEW field); **DeathWeapon=BlimpBomb + DeathWeaponDamageModifier=.1** (verbatim "needs a death weapon or it will do nothing when it crashes since its weapon is a camera" — shared mechanism with DISK); ShadowIndex=3 "draw plane body, not propellers"; SuperWeapon=SpyPlaneSpecial (NARADR, RechargeTime=4min) | DONE |
| CMISL      | Cruise Missile (Boomer Sub child) | **Yuri** | **INDEX CORRECTION: Yuri-spawned, NOT Allied-structure-spawned** (Owner=YuriCountry, parent=BSUB); closes kamikaze missile trio (V3ROCKET+DMISL+CMISL); near-clone of DMISL with Yuri ownership; Image=BSUBMISL art redirect; UIName=Name:DMISL (CSF label shared with Dread Missile — Westwood dev shortcut); Speed=20 (fastest of trio); Rules-global CMislWarhead=CMISLWH + CMislEliteWarhead=CMISLEWH (byte-identical to DMISLWH/DMISLEWH — no faction asymmetric damage); FlyBack=true (confirms Burst>1 correlation across trio); AuxSound1=BoomerAttack1 audio cohesion with parent; warhead-comment Westwood typo "this is the warhead on a DredMissile" copy-pasted | DONE |

## BuildingTypes (core gameplay)

Civilian decoration buildings (CAxxx, CIVxxx, lamps) listed as bulk TODO at end.

### Allied (GA*)
| ID        | Display                    | Notes                       | Status |
|-----------|----------------------------|-----------------------------|--------|
| GAPOWR    | Allied Power Plant         | Tier-1 power source; Power=200 base, **Upgrades=2** (Power Turbine slots add +100 each → 400 max fully upgraded); Strength=750/Armor=wood (more fragile than refinery despite criticality — strategic harassment target); Cost=800/TechLevel=1/Adjacent=2/Sight=4 (smallest sight of any Allied building); Capturable=true + Spyable=yes (spy infiltrate → temporary base-wide power outage per SPY_INFILTRATION_SYSTEM doc) + Drainable=yes + PoweredSpecial=yes + Crewed=yes; ImmuneToPsionics=no (psi-vulnerable); 6-anim Explosion palette with **`gtpowexp` power-plant-specific TS-era explosion SHP**; DieSound=PowerPlantDie (2-sample bpowdiea/bpowdieb Priority=high audio cue); Foundation=2x2 (smallest among Allied core buildings); **first documented building actively using power-state animation flags** `ActiveAnimPoweredSpecial=true` + `ActiveAnimPowered=false` (active anim gated by power-special state, likely upgrade-related); **16-entry power-state animation family discovered**: PoweredSpecial × LowPower × SuperLowPower × all anim slots = engine's rich power-state matrix (IdleAnimPoweredSpecial, LowPowerPoweredSpecial, SuperLowPowerPoweredSpecial, etc.); AIBuildThis= NOT set (AI uses Rules-global `BuildPower=NAPOWR,GAPOWR,YAPOWR` instead per power-plant lookup); GAPOWR_A 8-frame infinite loop Rate=220 + GAPOWR_AD frames 8-16 (faster cycle than typical 200); **3 NEW Ghidra cheat-sheet entries**: Upgrades (BuildingType), PowersUpBuilding (BuildingType — companion upgrade field), Drainable (TechnoType — Yuri drain-weapon target permission) | DONE |
| GAREFN    | Allied Ore Refinery        | Tier-1 harvester dock; **FreeUnit=CMIN** (free Chrono Miner on construction — effectively a 500-credit upgrade over CMIN's 1500 cost); **Universal Owner** (all 10 factions); Refinery=yes + DockUnload=yes + NumberOfDocks=1 + Storage=200 + Soylent=300 + Spyable=yes (spy infiltration steals ~50% credits) + Drainable=yes + ResourceDestination=yes; Strength=1000/Armor=wood (weaker than ConYard concrete); Power=-50 consumption; AIBasePlanningSide=0 (Good); **4-layer ActiveAnim system** (L1+L2+L3+L4) — first documented building actively using all 4 slots, confirms engine ActiveAnimTwo/Three/Four are not just latent; SpecialAnim=GAREFNOR ore-dump trigger (19-frame one-shot, paired with RefinerySmokeFrames=50); QueueingCell=4,1 + WaitingOffset pattern for harvester queueing; NumberImpassableRows=3 fix-from-Westwood-comment (prevents harvester drive-on-top bug); ZShapePointMove tuned by SJM (Steve Mariotti); AddOccupy1-2 west + RemoveOccupy1 dock cell for non-rectangular passability; Foundation=4x3 / OccupyHeight=2; Cameo=REFICON shared style; orphan `3x3Refinery` string in binary at 0x0081bb98 (TS-legacy code path — no YR refinery is 3x3); ;//gs revertNumberOfWaitingPoints=8 + ;WantsExtraSpace=yes commented (latent engine fields); **8 NEW Ghidra cheat-sheet entries**: Refinery, DockUnload, NumberOfDocks, FreeUnit, NumberImpassableRows (all BuildingType) + Storage, Soylent, ResourceDestination (all TechnoType-scope) | DONE |
| GACNST    | Allied Construction Yard   | **Build-tree root** — every Allied building has Prerequisite=GACNST (direct or transitive). Bidirectional pair with AMCV (`UndeploysInto=AMCV`); 4x4 foundation (largest building footprint); Strength=1000/Armor=concrete; Adjacent=2 (largest in game — anchors build-adjacency radius); Factory=BuildingType; TechLevel=-1 (hide-from-build-list; acquire only via AMCV deploy or pre-placed map start); Owner= all 5 Allied factions (universal vs ORCA's Korea-exclusion); Capturable=true (Engineer-capture transfers tech tree); Crewed=yes (E1 eject on death); ImmuneToPsionics=no explicit override (Westwood comment "defaults to yes for buildings, no for others"); 5-anim Explosion + 10-anim DebrisAnims palette + MaxDebris=15/MinDebris=7 (highest in any unit doc so far); AIBuildThis=yes + ProtectWithWall=yes + EligibileForAllyBuilding=yes (Westwood typo preserved in binary); `BuildConst=GACNST,NACNST,YACNST` Rules-AI-table; ;DestroyAnim=GACNSTDM TS-era leftover (commented out — Explosion= replaces it); Buildup=GACNSTMK + DemandLoadBuildup=true + FreeBuildup=true memory-thrift triple-flag; ActiveAnim+ProductionAnim Z-sort layering (-130/-10 ZAdjust); **8 NEW Ghidra cheat-sheet entries**: ConstructionYard, UndeploysInto, BuildConst (Rules-AI scope at 0x00672xxx — NEW range discovery), ProtectWithWall, Factory, Adjacent, EligibileForAllyBuilding, AIBuildThis (6 BuildingType + 1 TechnoType + 1 Rules-AI) | DONE |
| GAPILE    | Allied Barracks            | Tier-2 infantry producer; Cost=500/Strength=500 (lowest core building HP — fragile harassment target)/Armor=steel/Sight=5/Power=-10 minimal consumption; Factory=InfantryType; **GDIBarracks=yes engine-side flag — TS-era parser key heritage** (GDI=Allied in TS terminology, retained in YR engine despite faction renaming); engine has **three side-specific Barracks flags at consecutive parser addresses**: GDIBarracks (Allied) + NODBarracks (Soviet) + YuriBarracks (Yuri), all BuildingType scope at 0x00460b15/b2f/b45 (26-byte-spaced — tight parser triple sequence); ExitCoord=-64,64,0 (infantry exit position relative to foundation anchor in leptons); AIBasePlanningSide=0 (Good); Capturable+Spyable+Crewed; ImmuneToPsionics=no; Spy infiltrate → veterancy promotion mechanic (per SPY_INFILTRATION_SYSTEM doc); DamageParticleSystems active 3-system; Westwood verbatim comment "needs different Given Name to avoid editor confusion" — Name= field has special editor-tooling purpose distinct from CSF; Foundation=3x2/Height=4/OccupyHeight=2; AddOccupy1=-1,-1 for chimney NW extension; ActiveAnimPowered=no (always-on, not power-gated like power-plants); ;DestroyAnim=GAPILEDM commented (TS-era dead-code); commented ActiveAnimTwo/Three latent engine support; **4 NEW Ghidra cheat-sheet entries**: GDIBarracks/NODBarracks/YuriBarracks (3 sibling flags, BuildingType) + ExitCoord (BuildingType) | DONE |
| GASAND    | Allied Sandbag Wall?       |                             | TODO |
| GADEPT    | Service Depot              | Repair pad                  | TODO |
| GATECH    | Battle Lab                 | Tech prereq                 | TODO |
| GAWEAP    | War Factory                | Vehicle producer            | TODO |
| GAWALL    | Allied Wall                |                             | TODO |
| GAYARD    | Naval Shipyard             |                             | TODO |
| GACSPH    | Chronosphere               | Superweapon                 | TODO |
| GAWEAT    | Weather Controller         | Superweapon                 | TODO |
| GADUMY    | Allied dummy gate?         | Internal                    | TODO |
| GALITE    | Allied light                | Lamp                        | TODO |
| GAGREEN   | Grand Cannon support?      |                             | TODO |
| GASPYSAT  | Spy Satellite Uplink       | Reveals map                 | TODO |
| GAGAP     | Gap Generator              | Reshroud                    | TODO |
| GTGCAN    | Grand Cannon               | Defense                     | TODO |
| GAPILL    | Pillbox                    | Defense (Allied infantry pop)| TODO |
| GAOREP    | Ore Purifier               | 25% ore bonus               | TODO |
| GAAIRC    | Airforce Command HQ        | Aircraft prereq             | TODO |
| GAROBO    | Robot Control Center       | Robot Tank enabler          | TODO |
| GAFWLL    | Temp Yuri wall             |                             | TODO |
| GAGATE_A  | Allied Gate                |                             | TODO |

### Soviet (NA*)
| ID        | Display                    | Notes                       | Status |
|-----------|----------------------------|-----------------------------|--------|
| NAPOWR    | Soviet Tesla Reactor       | Soviet primary power plant; **cheaper-weaker-no-upgrades trade-off** vs GAPOWR — Cost=600 (25% less)/Power=150 (25% less) with **identical 0.25 Power/Cost ratio** (Soviet just buys in smaller granularity); **Upgrades= absent** (no Power Turbine slots — vs GAPOWR's 2; Soviet branching strategy: spam cheap NAPOWR OR commit to NANRCT Nuclear Reactor); Strength=750/Armor=wood/Sight=4 parity with GAPOWR; AIBasePlanningSide=1 (Evil); Capturable+Spyable+Drainable+Crewed+PoweredSpecial all parity; ImmuneToPsionics=no; DieSound=PowerPlantDie shared with GAPOWR (Westwood audio reuse); **MaxDebris=15** (vs GAPOWR's 6 — 2.5× dramatic Tesla coil destruction); **DamageParticleSystems=SparkSys+SmallGreySSys+BigGreySmokeSys active** vs GAPOWR's commented (Soviet visible electrical-arcing-when-damaged aesthetic); Explosion extra-anim `tstlexp` Tesla-specific (vs GAPOWR's `gtpowexp`); Foundation=3x2/Height=3 (wider+shorter than GAPOWR's 2x2/Height=4); NAPOWR_A 18-frame infinite loop Rate=300 (slower per-frame, 2.25× longer total than GAPOWR_A); **DoubleThick=true** on anim (electrical-arc rendering emphasis); DetailLevel=2 gating; ActiveAnimPoweredSpecial=true + ActiveAnimPowered=false same gating as GAPOWR (despite no Upgrades — open question on PoweredSpecial trigger); NAAPWR orphan in artmd at 3277 (Cameo=tpwricon lowercase — possibly TS-era variant, deferred); **2 NEW dual-scope Ghidra cheat-sheet entries**: DoubleThick (AnimType + BuildingType), DetailLevel (OptionsClass + AnimType — user-options-readable per-anim render threshold) | DONE |
| NATECH    | Battle Lab                 | Tech prereq                 | TODO |
| NAHAND    | Barracks                   | Infantry producer           | TODO |
| NARADR    | Radar Tower                | Radar                       | TODO |
| NAWEAP    | War Factory                | Vehicle producer            | TODO |
| NAREFN    | Soviet Ore Refinery        | Soviet sister to GAREFN; **near-perfect mechanical mirror** (same Cost=2000/Strength=1000/Storage=200/Soylent=300/Power=-50/Sight=6/Adjacent=2/TechLevel=1/Owner= all 10 factions); 5 rulesmd diffs (FreeUnit=HARV vs CMIN, AIBasePlanningSide=1 Evil vs 0 Good, Prerequisite=NACNST, RefinerySmokeOffsetOne/Two slightly different — taller silhouette, MaxDebris=8 with no MinDebris/DebrisAnims override); art diffs (Height=6 taller, OccupyHeight=4, Cameo=NREFICON, **8 RemoveOccupy slots** vs GAREFN's 1 for taller-silhouette passability cleanup, no AddOccupy, **all 4 ActiveAnim layers have Damaged variants** vs GAREFN's none, single DamageFireOffset=30,30); **8 commented WaitingOffset0-7** entries (latent engine queue system, parser key string not found in binary — may have been removed); **;DockingOffset0=256,0,0 commented** (engine supports DockingOffset%d format-string loop); commented `;PreProductionAnim=NAREFN_A` + `;ProductionAnim=NAREFN_AR Reverse=yes` + `;ActiveAnimTwoPowered=no` (Westwood iterated full PreProductionAnim+Reverse=yes+power-state-anim system before reverting to L1-L4); **3 NEW Ghidra cheat-sheet entries**: QueueingCell (BuildingType), DockingOffset%d (BuildingType format-string loop), `3x3Refinery` **dead-code orphan confirmed** (string at 0x0081bb98 has NO xrefs — TS-legacy with no active code path in YR) | DONE |
| NAWALL    | Soviet Wall                |                             | TODO |
| NAPSIS    | Psychic Sensor             | Attack-warning              | TODO |
| NALASR    | Sentry Gun                 | Anti-inf defense            | TODO |
| NASAM     | Flak Cannon (alt id?)      | Anti-air defense            | TODO |
| NAIRON    | Iron Curtain               | Superweapon                 | TODO |
| NACNST    | Soviet Construction Yard   | Soviet sister to GACNST; near-identical mechanics with 5 rulesmd diffs (UndeploysInto=SMCV, Owner= 4 Soviet factions excluding YuriCountry, **DebrisAnim=** (singular lowercase typo vs GACNST DebrisAnims=), MinDebris=5 vs 7, DamageParticleSystems=SparkSys+SmallGreySSys+BigGreySmokeSys active vs commented on GACNST); art diffs (Height=6 taller, OccupyHeight=4, 3-layer anim system Active+Idle+Production where GACNST has 2, **RemoveOccupy1-8 8 cell clearance for crane arm extending NW of foundation** — NACNST-specific feature absent on GACNST/YACNST); IdleAnim=NACNST_C with separate Damaged variant; latent commented engine fields (ActiveAnimTwo/Three, PreProductionAnim — TS-era latent capabilities still in engine); ;DestroyAnim=NACNSTD + ;[NACNSTD] dead-code pair on both sides; **4 NEW Ghidra cheat-sheet entries**: RemoveOccupy%d (BuildingType format-string-loop), OccupyHeight (BuildingType), TogglePower (BuildingType + NoTogglePower paired string), AIBasePlanningSide (**TechnoType-scope** surprise — broader than expected) | DONE |
| NADEPT    | Service Depot              | Repair pad                  | TODO |
| NAYARD    | Soviet Shipyard            |                             | TODO |
| NAMISL    | Nuclear Missile Silo       | Superweapon                 | TODO |
| TESLA     | Tesla Coil                 | Soviet defense              | TODO |
| ATESLA    | Tesla Coil alt?            |                             | TODO |
| NANRCT    | Soviet Nuclear Reactor     | Tier-9 Soviet heavy power; **Power=2000** (13.3× NAPOWR, highest per-building in game); **Power/Cost=2.0** (8× NAPOWR efficiency); Strength=1000/Armor=concrete/Cost=1000/Prerequisite=NATECH,NACNST; **Explodes=yes + DeathWeapon=NukePayload + DeathWeaponDamageModifier=0.5** (nuclear blast on destruction — Damage=600×0.5=300 at Range=30 + RadLevel=500 persistent radiation field, uses NAMISL's NUKE warhead; mini-nuke strategic discouragement of base clustering); **Powered=no** (power source doesn't consume power); **`IsImmuneToRadiation=yes` Westwood typo** — engine reads `ImmuneToRadiation` (no Is prefix); IsImmuneToRadiation has NO xrefs / NOT in binary string table → **NANRCT NOT actually radiation-immune in gamemd.exe** (significant parity finding — reimplementations must ignore the typo'd field to match binary); Points=30 (LESS than NAPOWR's 40 — Westwood balanced score-on-kill down because destroyer gets bonus collateral damage); DamageParticleSystems=SmallGreySSys+BigGreySmokeSys (no SparkSys — nuclear smoke, not electrical arcs); no Explosion= (relies entirely on NukePayload visuals); Foundation=4x4/Height=4/OccupyHeight=3 + 5 RemoveOccupy slots for cooling-tower extension; **first documented building actively using `LowPower`/`LowPowerDamaged`/`LowPowerPowered=false` low-power animation system** (NANRCT_P + NANRCT_PD sub-anims, 1-frame infinite at DetailLevel=2+DoubleThick=true); all 4 sub-anims DoubleThick=true for nuclear-glow rendering; **65-string Powered family** discovered in binary (vs 16 earlier — engine has very rich power-state matrix); **3 NEW Ghidra cheat-sheet entries + 1 dead-field**: Explodes (dual TechnoType+OverlayType scope), ImmuneToRadiation (TechnoType — canonical key), LowPowerDamaged (BuildingType); IsImmuneToRadiation confirmed dead/typo'd | DONE |
| NAFLAK    | Flak Cannon                | AA defense                  | TODO |
| NACLON    | Cloning Vats               | Infantry duplicator         | TODO |
| NAPSYB    | Psychic Beacon (small)     |                             | TODO |
| NAPSYA    | Psychic Beacon (large)     | Superweapon                 | TODO |
| NATBNK    | Battle Bunker              | Garrisonable defense        | TODO |
| NABNKR    | Bunker variant             |                             | TODO |
| NAINDP    | Soviet industrial plant    | 50% cheaper vehicles        | TODO |

### Yuri (YA*)
| ID        | Display                    | Notes                       | Status |
|-----------|----------------------------|-----------------------------|--------|
| YACNST    | Yuri Construction Yard     | Closes ConYard trio (GACNST+NACNST+YACNST done); **mechanically nearest sibling to GACNST** (inherits compact Height=4/OccupyHeight=3, both DamageFireOffsets, MinDebris=7, DebrisAnims= 10-anim list identical to GACNST, ;DamageParticleSystems commented like GACNST); **animation-system inherits NACNST's 3-layer Active+Idle+Production** (vs GACNST's 2-layer); 5 YACNST-unique rulesmd traits: **Sight=10** (vs 8 — only ConYard with vision boost, possibly Yuri psychic theme), Owner=YuriCountry (single-faction, vs Allied 5/Soviet 4), UndeploysInto=PCV, ;Image=GACNST commented (Westwood iteration artifact — asset-sharing reverted), ;DestroyAnim=GACNSTDM typo (references GACNST's anim not YACNSTDM); **Cameo=YCONICON explicit** (only ConYard with explicit Cameo override — routes outside the implicit <INI_ID>ICON convention to `yconicon.shp`); production anim shortest in trio (18 frames vs 20/21); no RemoveOccupy needed (4x4 visual = 4x4 foundation, same as GACNST); **3 NEW Ghidra cheat-sheet entries**: IdleAnimYSort (BuildingType + discovered 4-stage `IdleAnimPowered`/`PoweredLight`/`PoweredEffect`/`PoweredSpecial` power-state matrix), ActiveAnimTwoPoweredSpecial (BuildingType — full ActiveAnimTwo+power-state matrix engine-supported despite no ConYard using it), DamageFireOffset (BuildingType base read path) | DONE |
| YAPOWR    | Yuri Bio Reactor           | Yuri power plant — **closes power quartet** (GAPOWR+NAPOWR+NANRCT+YAPOWR); **garrison-boost mechanic is Yuri-unique faction feature**: base Power=150 + Passengers=5 × ExtraPower=100 = **650 max power per building** (4.33× base); Cost=600 parity with NAPOWR; with **Upgrades=2 fillable** (Yuri matches GAPOWR's upgrades + has garrison-boost on top) max effective = 850 power (highest single-building potential of any faction); UnitAbsorb=no + InfantryAbsorb=yes + SizeLimit=15 + PipScale=Passengers — infantry-only absorption-style garrison (distinct from RA2's CanBeOccupied/MaxNumberOccupants occupy-style, which YAPOWR has **commented out** — both systems coexist in engine); **`Strength=700` weakest of quartet** (balance for high potential); `UIName=Name:BioR` non-standard CSF lookup (other power plants use Name:<INI_ID>); **`AIBasePlanningSide=2`** confirms Yuri is THIRD AI side, not folded into Soviet (extending the "0 for Good, 1 for Evil" verbatim comment); ActiveAnimTwo=YAPOWR_B labeled "**powered up** active animation" — visual feedback when passengers absorbed (engine triggers ActiveAnimTwo on Passengers>0 — distinct from standard power-state animation system); IdleAnim=YAPOWR_C "lights" anim Rate=175 (slightly faster than active Rate=220); Westwood verbatim designer commentary "Should engineer capture or enter it? Dunno, so ban capture" preserved in shipping INI; Explosion uses Allied's gtpowexp not Yuri-specific (Westwood inconsistency); DamageFireOffset0/1 identical to GAPOWR (parity); Foundation=2x2/Height=4/OccupyHeight=3 (smallest power plant footprint, same as GAPOWR); `Image=YAPOWR` explicit declaration (most blocks implicit); **4 NEW Ghidra cheat-sheet entries** (the garrison-boost field family): UnitAbsorb (BuildingType), InfantryAbsorb (BuildingType), ExtraPower (BuildingType — the +100 per passenger multiplier), Passengers (TechnoType — shared with transport vehicles BFRT/SHAD/HTK/SAPC) | DONE |
| YABRCK    | Yuri Barracks              | Infantry producer           | TODO |
| YAWEAP    | Yuri War Factory           |                             | TODO |
| YAYARD    | Yuri Sub Pen               |                             | TODO |
| YADEPT    | Yuri Service Depot         |                             | TODO |
| YATECH    | Yuri Battle Lab            | Tech                        | TODO |
| YAGGUN    | Gattling Cannon            | Defense                     | TODO |
| YAPSYT    | Psychic Tower              | Mind-control defense        | TODO |
| YAGRND    | Grinder                    | Recycle units → cash        | TODO |
| YAGNTC    | Genetic Mutator            | Superweapon                 | TODO |
| YAPPET    | Psychic Dominator          | Superweapon                 | TODO |
| YACOMD    | Yuri radar/command         |                             | TODO |
| YAPPPT    | Tank Bunker?               |                             | TODO |
| YAREFN    | Slave Miner deploy form    | Deployed [SMIN]; UndeploysInto=SMIN; BaseNormal=no; ImmuneToPsionics+Capturable=false economic-protection; Trainable=yes building-veterancy; Enslaves=SLAV/SlavesNumber=5/SlaveRegenRate=500/SlaveReloadRate=25 — brain-transplant SlaveManager bound to building post-deploy | DONE |
| YAROCK    | Yuri statue/decor?         |                             | TODO |

### Tech / civilian gameplay buildings (CA*)
| ID        | Display                    | Notes                       | Status |
|-----------|----------------------------|-----------------------------|--------|
| CABHUT    | Bridge Hut                 | C4-destroyable              | TODO |
| CALAB     | Tech Lab (Einstein)        | Campaign                    | TODO |
| CAOUTP    | Tech Outpost               | Free IFV                    | TODO |
| CATHOSP   | Tech Hospital              | Heals infantry              | TODO |
| CAAIRP    | Tech Airport               | Paradrop                    | TODO |
| CAOILD    | Tech Oil Derrick           | Periodic credits            | TODO |
| CAMACH    | Tech Machine Shop          | Auto-repair vehicles        | TODO |
| AMMOCRAT  | Ammo Crate? (tech)         | Power-up?                   | TODO |
| AMRADR    | American Radar?            | Campaign                    | TODO |
| CASLAB    | Slab civilian              |                             | TODO |
| CATIME    | Times Square structure     | Campaign                    | TODO |

### Civilian decoration (bulk)

All `CIV*`, `CITYxx`, `CAHSExx`, `CAFARMxx`, `CALITxx`, `CAMISCxx`, `CAPOLxx`,
`CASINxx`, `CAPARSxx`, `CASTLxx`, `CAWASHxx`, `CAARMYxx`, `CAUSFGL`, `CALA*`,
`CALOND*`, `CAMOON01`, `CATRANxx`, `CAEAST01`, `CAEGYP*`, `CAMOR*`, `CASANFxx`,
`CASEATxx`, `CAFRMxx`, `CASKFGL`, `CALBFGL`, `CAGEFGL`, `CAUKFGL`, `CAPOFGL`,
`CARUFGL`, `CAFRFGL`, `CACUFGL`, `CARUSxx`, `CAMSCxx`, `CACHIGxx`, `CABARRxx`,
`CABUNKxx`, `CAEUR*`, `CAMEXxx`, `CAMIAMxx`, `CANEWY*`, `CANWY*`, `CAPARKxx`,
`CATEXSxx`, `CAGAS01`, `CASTRTxx`, `CAGARD01`, `CAFNCB`, `CAFNCW`, `CAFNCP`,
`CAKRMW`, `CAMOVxx`, `CAIND01`, `CACOLO01`, `CABARN02`, `CAWA2*`, `CATS01`,
`CAWT01`, `CAUSFGL`, `CAARMY*`, lamps (`*LAMP*`), `MAYAN`, all `INxxx` (interior
lights) — collectively **bulk TODO**; produce minimal docs grouped by family
unless they have unique gameplay.

---

## How to use this index in /loop iterations

1. Find next TODO (priority order above)
2. Mark IN-PROGRESS
3. Write `units/<side>/<ID>.md` per template in the loop prompt
4. Confirm key-by-key coverage of rulesmd + artmd sections
5. Confirm at least one Ghidra search against the ID
6. Mark DONE (or leave IN-PROGRESS with gap note)
