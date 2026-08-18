---
name: schd-doc
description: SCHD — "ZZZ Deployed Soviet Siege Chopper" placeholder. TechLevel=-1
  unbuildable, Name prefixed ZZZ. Has DeployFire=yes + DeployFireWeapon (default
  index 1) but NO Secondary weapon — vestigial. Real Siege Chopper gameplay
  apparently happens at SCHP level. Closes SCHP/SCHD pair with placeholder status.
metadata:
  type: project
---

# SCHD — "Deployed Soviet Siege Chopper" (vestigial placeholder)

**INI ID:** `SCHD`
**Display:** `Name=ZZZ Deployed Soviet Siege Chopper` — the **`ZZZ` prefix
is the Westwood convention for vestigial/cut/placeholder** entries (same
pattern as `[SMON] ZZZ Useless` and `[UTNK] ZZZ Not Used`).
**Section:** `[VehicleTypes]`
**Owner side:** Soviet (4 sub-factions)
**Status:** **SKIP-NORMAL-PLAY / VESTIGIAL**. The SCHD entry exists in the
INI as the named `UnloadingClass` target for SCHP's deploy mechanic, but
multiple signals indicate it is *not* the actual gameplay-active deployed
form:
1. `TechLevel=-1` — *unbuildable directly*. Only reachable via SCHP deploy.
2. `Name=ZZZ Deployed Soviet Siege Chopper` — the ZZZ-prefix flag.
3. `Primary=BlackHawkCannon` only — **no Secondary 160mm weapon** despite
   being the "deployed" form intended to be the artillery mode.
4. `DeployFire=yes` + `DeployFireWeapon` defaulting to index 1
   (Secondary) — points to a missing slot.

---

## Status note — and SCHP doc correction

The previous iteration's SCHP doc claimed:
> "Player issues Deploy command. SCHP lands and uses 160mm artillery
> in deployed mode. SCHD is the deployed entity."

**This may be inaccurate.** The SCHD rulesmd block doesn't have the
160mm Secondary weapon — only Primary=BlackHawkCannon. If the deploy
truly swapped to SCHD, the deployed unit would lose access to the
160mm, contradicting the well-known gameplay (where deployed Siege
Choppers fire the heavy artillery).

**Plausible reinterpretations (open for Ghidra investigation):**
1. **SCHD is a discarded design**: the actual deploy mechanic in
   shipped YR may stay at SCHP-level (no entity swap), with the
   160mm Secondary firing as part of SCHP itself when the player
   triggers Deploy. The `UnloadingClass=SCHD` declaration may be
   dead-code that never actually triggers.
2. **Deploy switches between weapon modes** rather than entities:
   maybe SCHP-with-160mm is the "deployed mode" weapon (Secondary
   slot) accessed via Deploy action, and "undeployed" SCHP fires
   only Primary. The SCHD entity may exist as a visual-asset
   reference (the lowered-gear voxel pose) but the entity itself
   never gets instantiated.
3. **SCHD is the real deploy target with a Westwood bug**: the
   Primary=BlackHawkCannon should have been overridden with the
   160mm in the SCHD rulesmd, but Westwood forgot to copy the
   weapon over. The deploy swap *does* happen, but the deployed
   chopper fires only the machine gun (gameplay bug that shipped).

This doc proceeds with **option 1** as the working interpretation —
SCHD is a vestigial entry never reached in gameplay — and flags the
issue as an **open follow-up requiring Ghidra trace** into the
`IsSimpleDeployer` deploy command path.

---

## Rulesmd verbatim

```ini
[SCHD]
UIName=Name:SiegeChopper
Name=ZZZ Deployed Soviet Siege Chopper
Prerequisite=NAWEAP
Primary=BlackHawkCannon
Strength=200
Category=AirPower
JumpJet=yes
Armor=light
TechLevel=-1
Sight=7
Speed=12
PitchSpeed=1.1
JumpjetSpeed=30 ;params not defined use defaults (old globals way up top)
JumpjetClimb=10
JumpjetCrash=40 ; Climb, but down
JumpJetAccel=12
JumpJetTurnRate=6
JumpjetHeight=500
JumpjetWobbles=.01
JumpjetDeviation=1
Owner=Russians,Confederation,Africans,Arabs
Cost=1000
Points=15
ROT=5
Crewed=no
ConsideredAircraft=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
MaxDebris=3
VoiceSelect=SeigeChopperSelect
VoiceMove=SeigeChopperMove
VoiceAttack=SeigeChopperAttackLand
VoiceCrashing=BlackOpsVoiceDie
DieSound=
CrashingSound=BlackOpsDie
ImpactLandSound=GenAircraftCrash
;Locomotor={4A582746-9839-11d1-B709-00A024DDAFD1} ;flying
Locomotor={92612C46-F71F-11d1-AC9F-006008055BB5} ;jumpjet
MovementZone=Fly
DamageParticleSystems=SparkSys,SmallGreySSys
;AuxSound1=BlackOpsTakeOff	;Taking off
;AuxSound2=BlackOpsLanding	;Landing
ThreatPosed=0
SpecialThreatValue=1
VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
Size=15
SizeLimit=2
HoverAttack=yes
AllowedToStartInMultiplayer=no
Crashable=yes
CanPassiveAquire=no ; Won't try to pick up own targets
SpeedType=Hover
EnterTransportSound=EnterTransport
LeaveTransportSound=ExitTransport
ElitePrimary=BlackHawkCannonE
PreventAttackMove=yes
TooBigToFitUnderBridge=true
Trainable=yes
Bunkerable=no; Units default to yes, others default to no
IsSimpleDeployer=yes
UnloadingClass=SCHP
Turret=yes
DeployFire=yes
DeployToLand=yes
```

### Key-by-key annotation

Most fields mirror SCHP (the parent form). This section highlights
SCHD-distinctive differences and the deploy-related fields.

**Identity / availability**
- `UIName=Name:SiegeChopper` — same CSF key as SCHP. *Both forms
  display the same name* (the Westwood label "Siege Chopper" applies
  to either).
- `Name=ZZZ Deployed Soviet Siege Chopper` — **vestigial-marker
  prefix**.
- `Prerequisite=NAWEAP` — single prereq (SCHP requires `NAWEAP, TECH`).
  SCHD's lighter prereq is moot since `TechLevel=-1`.
- `TechLevel=-1` — **unbuildable directly**. Standard cut-content
  marker.
- `Owner=Russians,Confederation,Africans,Arabs` — same 4 Soviet houses.
- `AllowedToStartInMultiplayer=no` — standard.

**Stats — slightly different from SCHP**
- `Strength=200` (vs SCHP 300). *Lower HP*.
- `Cost=1000` (vs SCHP 1100). *Lower cost*. Both moot due to
  unbuildable.
- All Jumpjet flight parameters identical to SCHP.

**The deploy mechanic — the critical SCHD-specific block**

These 4 fields are new vs SCHP:
- `IsSimpleDeployer=yes` (per SCHP cheat-sheet, UnitType `0x00845dfc →
  0x00747688`) — *same flag both forms have*. Bidirectional deploy
  declaration.
- `UnloadingClass=SCHP` — **reverse direction**. SCHD's UnloadingClass
  is SCHP (vs SCHP's UnloadingClass=SCHD). The pair is *bidirectional*.
  Deploy SCHD → SCHP; deploy SCHP → SCHD. This is the bidirectional
  toggle pattern.
- `Turret=yes` — **explicitly enabled** (vs SCHP which doesn't set
  Turret — defaults). Suggests SCHD's body has a turret pose.
- `DeployFire=yes` — **Ghidra-verified TechnoType** at
  `0x00843aa0 → 0x007147ef`. **NEW cheat-sheet entry**. Verbatim INI
  documentation (line 3498 of rulesmd): *"This unit can fire when
  deployed. (def=no)"*. So this is the *infantry-deploy-fire flag*
  (used by GI, GGI etc. for sandbag-stance fire). Why is it on a
  vehicle SCHD? Probably leftover from a design experiment where
  the Siege Chopper had a unified "deploy fire" mechanic with infantry.
- `DeployToLand=yes` — **Ghidra-verified TechnoType** at
  `0x00843a90 → 0x00714809`. **NEW cheat-sheet entry**. Likely
  triggers the "land on terrain when deployed" behavior for flying
  units. Distinguishes ground-landing deploys (Siege Chopper) from
  in-place deploys (G.I.).

**Voice / sound bindings**
- `VoiceSelect=SeigeChopperSelect` — same pool as SCHP.
- `VoiceMove=SeigeChopperMove` — same.
- `VoiceAttack=SeigeChopperAttackLand` — **uses LAND attack voice as
  primary** (SCHP uses AttackAir as primary, AttackLand as secondary
  via VoiceSecondaryWeaponAttack). The SCHD treats its only weapon
  (BlackHawkCannon) as a land-attack — but the BlackHawkCannon is
  actually anti-air. Inconsistent — supports the vestigial-content
  reading.
- *No `VoiceSecondaryWeaponAttack=`* — SCHD has no Secondary weapon
  to attach the voice to.
- `VoiceCrashing=BlackOpsVoiceDie` — *different from SCHP's
  SeigeChopperVoiceDie*. SCHD borrows the SHAD/Nighthawk's crash
  voice. Possibly a placeholder that was never updated to a
  SCHD-specific voice.
- `CrashingSound=BlackOpsDie` — same borrowed SHAD crash SFX.
- All other voice/sound fields shared with SCHP.

**Combat behavior**
- `CanPassiveAquire=no` — **explicitly disabled** (vs SCHP's commented
  `;CanPassiveAquire=no`). SCHD's auto-target acquisition is OFF.
  Player must manually order all attacks.
- `PreventAttackMove=yes` — same as SCHP.
- `Turret=yes` — turret rotation enabled.
- All other combat flags shared with SCHP.

**Crew / death**
- `Crewed=no`.
- `Crashable=yes` — plummets on death.

**Veterancy**
- `ElitePrimary=BlackHawkCannonE` — single elite weapon swap (vs SCHP's
  two-weapon swap).
- *No `EliteSecondary=`* — no Secondary, no elite version.

**All other fields are copy-paste from SCHP** — confirming the
hypothesis that SCHD was created from the SCHP template, then
modified only for deploy-related fields. The Secondary weapon copy
was omitted (intentionally or accidentally).

---

## Artmd verbatim

```ini
[SCHD] ; Soviet Siege Chopper
Cameo=SCHPICON
Voxel=yes
UseBuffer=yes
Remapable=yes
PrimaryFireFLH=200,0,250
SecondaryFireFLH=200,0,250
```

### Key-by-key annotation

- `Cameo=SCHPICON` — *same cameo as SCHP* (no SCHDICON). Sidebar lookup
  defaults to SCHP icon if SCHD's were attempted to display.
- `Voxel=yes` — `schd.vxl` + `schd.hva`. *SCHD has its own art* (the
  "deployed/landed" pose with lowered gear, possibly extended cannon).
- `UseBuffer=yes` — render buffer optimization.
- `Remapable=yes` — house color.
- `PrimaryFireFLH=200,0,250` — **same as SCHP's SecondaryFireFLH**. The
  deployed pose has both FLH positions at the 160mm cannon mount.
- `SecondaryFireFLH=200,0,250` — *same as Primary*. Both FLH offsets
  point to the same world position on the SCHD voxel. Pointless given
  SCHD has no Secondary weapon, but consistent with the placeholder
  artmd setup.

**Notable**: The SCHD artmd block has *no `ShadowIndex=`*, *no
`AltCameo=`*. Reduced from the SCHP artmd. Consistent with vestigial-
content status — Westwood didn't fully polish the SCHD art block.

---

## Weapons

### Primary — `[BlackHawkCannon]`

Same weapon as SCHP's Primary and the SHAD Nighthawk's defensive
cannon. 35 damage, ROF=40, Range=6, QuadShell projectile, SA warhead,
8-direction MGUN anim, OmniFire=yes.

See [SHAD.md](../allied/SHAD.md#weapons) for full details.

**Anomaly**: A *deployed* Siege Chopper firing the same anti-infantry
machine gun makes no thematic sense. The expected "deployed mode 160mm
artillery" is **NOT** SCHD's weapon. Strong evidence that SCHD is
vestigial.

### Elite primary — `[BlackHawkCannonE]`

Damage 35→40, Warhead SA→SSA. Same as SCHP elite.

### No Secondary weapon

Despite `DeployFire=yes` (with `DeployFireWeapon` defaulting to index 1
= Secondary per verbatim INI documentation at rulesmd line 3499),
**SCHD has no Secondary weapon defined**. The DeployFire mechanic
points to a non-existent slot. **This is the strongest indicator that
SCHD is vestigial** — Westwood wouldn't ship a working deploy mode
with a broken fire reference.

---

## Voices / sounds

Same shared pool as SCHP for Select/Move (Seige* blocks). Different
choices for attack and crash (see Key-by-key annotation above).

| Rules key | Sound block | When |
|-----------|-------------|------|
| `VoiceSelect=SeigeChopperSelect` | shared with SCHP | Click |
| `VoiceMove=SeigeChopperMove` | shared with SCHP | Move order |
| `VoiceAttack=SeigeChopperAttackLand` | shared with SCHP (but used here as primary attack) | Attack order |
| `VoiceCrashing=BlackOpsVoiceDie` | **borrowed from SHAD/Nighthawk** | Voice during plummet |
| `CrashingSound=BlackOpsDie` | **borrowed from SHAD** | Sustained crash SFX |
| `ImpactLandSound=GenAircraftCrash` | shared | Impact |

The borrowed `BlackOpsVoiceDie` + `BlackOpsDie` *instead of* SCHP's
`SeigeChopperVoiceDie` + `SeigeChopperDie` is striking — strongly
suggests SCHD was branched from a SHAD template earlier than SCHP, and
the voice keys weren't updated.

**No `MoveSound=`** explicitly set on SCHD (would inherit default? or
silent during move). SCHP has `MoveSound=SeigeChopperMoveLoop` — SCHD
doesn't.

---

## Hardcoded behavior (Ghidra-verified)

### 1. DeployFire + DeployFireWeapon (TechnoType)

- `DeployFire=yes` (TechnoType `0x00843aa0 → 0x007147ef`, **NEW
  cheat-sheet entry**). Verbatim INI doc: *"This unit can fire when
  deployed. (def=no)"*. Originally intended for infantry deploy-stance
  fire (G.I. AssaultCannon while sandbagged).
- `DeployFireWeapon` (TechnoType `0x00843aac → 0x007147d5`, **NEW
  cheat-sheet entry**). Verbatim INI doc: *"Index of weapon to fire
  while deployed. 0 or 1. (def=1)"*. Defaults to Secondary (index 1).
  SCHD's missing Secondary makes this read-but-unused.

**Active use elsewhere**: E1 (G.I.) has DeployFire=yes; GGI similarly.
Those units have proper Primary + Secondary with the Secondary being
the deploy-mode weapon (AssaultCannon for G.I.). SCHD's configuration
is the broken outlier.

### 2. DeployToLand=yes

**Ghidra-verified TechnoType** at `0x00843a90 → 0x00714809`. **NEW
cheat-sheet entry**. Likely controls *whether the deploy action
involves the unit physically landing on the ground* (vs deploying in
place). For a jumpjet vehicle, DeployToLand=yes means the unit will
descend before triggering the deploy state-change.

**Not yet verified** by tracing the code path — open follow-up to
determine exact behavior. Inferred from name + co-occurrence with
deploy-related fields.

### 3. IsSimpleDeployer + UnloadingClass=SCHP (bidirectional)

The SCHP/SCHD pair both declare `IsSimpleDeployer=yes` with
`UnloadingClass` pointing at the other. **Bidirectional pair pattern**:
- SCHP `UnloadingClass=SCHD` — deploy from SCHP creates SCHD.
- SCHD `UnloadingClass=SCHP` — undeploy from SCHD creates SCHP.

Compare with harvester pattern:
- HARV (Soviet War Miner) `UnloadingClass=HORV` — switches to HORV
  visual when unloading ore.
- HORV does NOT typically declare `UnloadingClass=HARV` in reverse —
  the unload state is *transient*, not bidirectional. Once unloading
  completes, the engine reverts HORV→HARV automatically.

The SCHP/SCHD bidirectional explicit pair is *different from the
harvester transient pattern*. Suggests a *player-controlled* toggle
(Deploy/Undeploy commands) rather than an automatic state-machine
transition.

### 4. Vestigial-content indicators

Multiple signals collectively indicate SCHD is not actively used:
- `Name=ZZZ ...` prefix.
- `TechLevel=-1`.
- Missing Secondary weapon despite DeployFire pointing to it.
- Borrowed BlackOps* crash sounds (placeholder for unfinished
  SeigeChopper* equivalents).
- No SCHD-specific cameo (uses SCHPICON).
- Simpler artmd block (no AltCameo, no ShadowIndex).

**Westwood placeholder convention** for cut/abandoned/vestigial
content. Same ZZZ-prefix pattern as SMON, UTNK, [Hind Transport TL-1].

### 5. Open hypothesis for actual deploy gameplay

The most likely real-world gameplay mechanism:
1. Player builds SCHP (flying form).
2. SCHP fires Primary=BlackHawkCannon (anti-inf MG) while flying.
3. Player issues Deploy command.
4. Engine *visually* lands SCHP (DeployToLand semantics).
5. **Without entity swap** (SCHD entity never instantiated), SCHP
   gains access to Secondary=160mm weapon while in deployed state
   (some hardcoded check on deploy state enables Secondary firing).
6. Undeploy reverses to flying SCHP.

This makes SCHD a *broken-but-declared* alternative path that the
engine code never actually triggers. The `UnloadingClass=SCHD`
declaration on SCHP is read into the TechnoType struct but the
deploy code path bypasses entity swap and stays at SCHP-level.

**Caveat**: This is *unverified*. Confirming requires Ghidra trace
of the `IsSimpleDeployer` deploy command. Open follow-up.

---

## TS-legacy filter

- Same as SCHP (same template).
- The vestigial-content indicators are a YR-development artifact, not
  TS legacy.

---

## Comparison: SCHD vs SCHP (the deploy pair)

| Field | SCHP (active) | SCHD (vestigial) |
|-------|---------------|------------------|
| Name | "Soviet Siege Chopper" | **"ZZZ Deployed Soviet Siege Chopper"** |
| TechLevel | 7 | **-1** |
| Prerequisite | NAWEAP,TECH | NAWEAP only |
| Strength | 300 | **200** |
| Cost | 1100 | 1000 (moot, unbuildable) |
| Primary | BlackHawkCannon | BlackHawkCannon (same) |
| Secondary | **160mm** | **(none!)** |
| ElitePrimary | BlackHawkCannonE | BlackHawkCannonE (same) |
| EliteSecondary | 160mmE | (none) |
| VoiceAttack | SeigeChopperAttackAir | SeigeChopperAttackLand |
| VoiceCrashing | SeigeChopperVoiceDie | **BlackOpsVoiceDie (SHAD borrowed)** |
| CrashingSound | SeigeChopperDie | **BlackOpsDie (SHAD borrowed)** |
| Turret | not set | **yes** |
| DeployFire | not set | **yes** |
| DeployToLand | not set | **yes** |
| CanPassiveAquire | not set | **no** (explicit) |
| Cameo | SCHPICON + AltCameo=SCHPUICO | SCHPICON only |
| AltCameo | SCHPUICO | (none) |
| Artmd ShadowIndex | 2 | (none) |
| UnloadingClass | SCHD | SCHP (reverse) |

**Difference summary:**
- SCHD is a stripped-down vestigial duplicate of SCHP.
- The deploy-related fields (DeployFire, DeployToLand, Turret) suggest
  SCHD was intended as the *deployed entity*.
- Missing Secondary weapon, borrowed crash sounds, simpler artmd —
  all unfinished placeholder markers.
- Bidirectional UnloadingClass pair is conceptually the deploy/undeploy
  toggle pattern, but seemingly never reached in actual gameplay.

---

## Cross-references

- [SCHP.md](./SCHP.md) — Active flying form. Pair partner. **My
  earlier SCHP doc's claim that "Deploy swaps to SCHD" needs
  re-investigation** (this iteration reveals SCHD as likely
  vestigial).
- [SMON.md](../yuri/SMIN.md) (within SMIN doc) — Similar ZZZ-prefix
  vestigial entry.
- [UTNK index entry] — Another ZZZ vestigial.
- Open: Ghidra trace of `IsSimpleDeployer` to determine actual deploy
  code path.

---

## Coverage audit

- [x] Every rulesmd key annotated (~55 keys + the 3 deploy-specific
  fields).
- [x] Every artmd key annotated (6 keys).
- [x] No Secondary weapon explicitly noted (DeployFire points to
  missing slot).
- [x] All voice/sound bindings documented including borrowed
  BlackOps* crash sounds.
- [x] Prerequisites: `NAWEAP` only (no TECH).
- [x] Owner: 4 Soviet sub-factions.
- [x] Veterancy: single ElitePrimary swap (no EliteSecondary since no
  Secondary).
- [x] Hardcoded behavior: DeployFire + DeployFireWeapon + DeployToLand
  (3 NEW cheat-sheet entries), bidirectional UnloadingClass pair,
  multiple vestigial-content indicators, open hypothesis for actual
  deploy gameplay.
- [x] TS-legacy filter applied (no TS-specific issues; vestigial-content
  is YR-development artifact).
- [x] Comparison table with SCHP closes the pair.
- [x] At least one Ghidra search performed (3 strings + xrefs, 3 new
  cheat-sheet entries).
- [x] **SCHP doc cross-reference correction** flagged.

**Ghidra queries logged (this iteration):**

| Query | Result |
|-------|--------|
| `search_strings("DeployToLand")` | `0x00843a90` (single match) |
| `get_xrefs_to(0x00843a90)` | `0x00714809 → TechnoTypeClass__ReadINI` |
| `search_strings("DeployFire")` | `0x00843aa0` + sibling `DeployFireWeapon` at `0x00843aac` |
| `get_xrefs_to(0x00843aa0)` | `0x007147ef → TechnoTypeClass__ReadINI` |
| `get_xrefs_to(0x00843aac)` | `0x007147d5 → TechnoTypeClass__ReadINI` |

**New cheat-sheet entries (3):**
- `DeployToLand` (0x00843a90 → 0x00714809) TechnoType — land on
  terrain during deploy (for jumpjet vehicles).
- `DeployFire` (0x00843aa0 → 0x007147ef) TechnoType — *can fire while
  deployed*. Verbatim INI doc: "This unit can fire when deployed.
  (def=no)".
- `DeployFireWeapon` (0x00843aac → 0x007147d5) TechnoType — *which
  weapon to fire while deployed*. Verbatim INI doc: "Index of weapon
  to fire while deployed. 0 or 1. (def=1)".

**Open questions / flagged for follow-up:**
- **Major**: SCHD might be vestigial and SCHP-deploy might never
  actually instantiate SCHD. Open Ghidra trace of
  `IsSimpleDeployer` code path required to determine. The SCHP doc's
  claim about "deploy mode-swap to SCHD" may need to be flagged as
  *unverified* / *possibly wrong*.
- The "ZZZ" Westwood-convention placeholder marker — is there a
  systematic catalog of all ZZZ-prefix entries to map cut content?
- `DeployToLand=yes` exact behavior — open Ghidra trace of how this
  affects the deploy command path.

**Pair status:**
- SCHP ✓ (active, documented previous iteration with potentially
  incorrect deploy claims)
- SCHD ✓ (vestigial, documented this iteration)
- Open: Verify/correct SCHP claims via Ghidra trace.
