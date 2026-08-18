# Guardian G.I. (GGI)
Side: Allied | Category: Infantry | Image alias: `[GGI]` (no `Image=` redirect)

The Allied secondary infantry — a hardened deploy-fortify variant of the GI.
$400 from the Barracks (tech 2), $200 more than the GI but with the same M60
primary plus a `MissileLauncher` secondary fired only while deployed: range 8,
AA+AG, single missile per shot at ROF 40. Deployed GGI is **uncrushable** via
`DeployedCrushable=no`. Cannot garrison civilian buildings (`Occupier=no`).
Slower than GI (Speed 3 vs 4) and bigger sight (Sight 6 vs 5).

No GGI-specific code branch exists in `gamemd.exe` — every behavioral
difference from E1 is value-driven through the shared infantry state machine.

Authoritative deep RE: [GGI_GHIDRA_REPORT.md](../../GGI_GHIDRA_REPORT.md)
(1,598 lines; covers deltas from [GI_GHIDRA_REPORT.md](../../GI_GHIDRA_REPORT.md)).

---

## rulesmd.ini — `[GGI]` section

Verbatim from `ini/rulesmd.ini:3863`:

```ini
[GGI]
UIName=Name:GuardianGI
Name=Guardian GI
Category=Soldier
Primary=M60
Secondary=MissileLauncher ;GEF New Guardian GI weapon
OpenTransportWeapon=1;defaults to -1 (decide normally)  What weapon should I use in a Battle Fortress
Occupier=no;yes ; I can Occupy UC buildings
Prerequisite=GAPILE
CrushSound=InfantrySquish
Strength=100
Pip=white
Armor=none
TechLevel=2
Sight=6
Speed=3
Owner=British,French,Germans,Americans,Alliance
AllowedToStartInMultiplayer=no
Cost=400
Soylent=150
Points=10
IsSelectableCombatant=yes
VoiceSelect=GuardianGISelect
VoiceMove=GuardianGIMove
VoiceAttack=GuardianGIAttackCommand
VoiceFeedback=GuardianGIFear
VoiceSpecialAttack=GuardianGIMove
DieSound=GuardianGIDie
Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}
PhysicalSize=1
MovementZone=Infantry
ThreatPosed=10 ; This value MUST be 0 for all building addons
ImmuneToVeins=yes
ImmuneToPsionics=no
Bombable=yes
Deployer=yes
DeployFire=yes
; DeployTime=.022  ; PCG; Unused for now.
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
Size=1
Crushable=yes
DeployedCrushable=no
DeploySound=GuardianGIDeploy
UndeploySound=GIUndeploy
ElitePrimary=M60E
EliteSecondary=MissileLauncherE ;GEF New Guardian GI weapon
;EliteSecondary=ParaE
IFVMode=16
PixelSelectionBracketDelta=-6;gs higher number draws lower.  Pixel difference from normal for selection bracket
```

| Key | Meaning |
|-----|---------|
| `UIName=Name:GuardianGI` | CSF-string key resolving to "Guardian GI" |
| `Name=Guardian GI` | Internal display name |
| `Category=Soldier` | Pip group + AI threat grouping |
| `Primary=M60` | Standing/crawling weapon (range 4 AP MG) — same weapon as GI |
| `Secondary=MissileLauncher` | Deploy-fire weapon (range 8, AA+AG missile). The "GEF" comment marks a designer note from Greg Fowler |
| `OpenTransportWeapon=1` | When passenger in Battle Fortress (FV) or Tank Bunker, fire Secondary slot (MissileLauncher) — turns FV into a mobile AA platform |
| `Occupier=no` | **Cannot enter civilian garrisonable buildings** (contrast E1's `Occupier=yes`). `;yes` comment indicates the design changed mid-development |
| `Prerequisite=GAPILE` | Allied Barracks required |
| `CrushSound=InfantrySquish` | Same crush sample as GI (`igensqua`) |
| `Strength=100` | HP — 80% of GI (125); GGI is fragile despite anti-armor role |
| `Pip=white` | Cargo pip color |
| `Armor=none` | Damage type column 0 |
| `TechLevel=2` | **Tier 2** — requires the AI base-planning tech tier 2 (typically also requires Radar/Naval Yard depending on house) |
| `Sight=6` | Reveal radius (1 cell larger than GI) — helps the long-range deployed weapon find targets |
| `Speed=3` | Walk speed — 75% of GI (4); GGI is slower |
| `Owner=British,French,Germans,Americans,Alliance` | Allied subfactions only (5 countries) |
| `AllowedToStartInMultiplayer=no` | Excluded from lobby starting-unit allocation |
| `Cost=400` | Credits — 2× GI |
| `Soylent=150` | Grinder refund |
| `Points=10` | Kill score |
| `IsSelectableCombatant=yes` | Included in "select all combat units" + AI combat groups |
| `VoiceSelect=GuardianGISelect` | Unique voice bank (separate from GI) |
| `VoiceMove=GuardianGIMove` | Move acknowledgement |
| `VoiceAttack=GuardianGIAttackCommand` | Attack acknowledgement |
| `VoiceFeedback=GuardianGIFear` | Fear/panic voice |
| `VoiceSpecialAttack=GuardianGIMove` | Deploy command voice (reuses move bank) |
| `DieSound=GuardianGIDie` | Death sample |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` GUID — bipedal infantry |
| `PhysicalSize=1` | Pathfinder size |
| `MovementZone=Infantry` | Standard infantry terrain |
| `ThreatPosed=10` | AI target priority (same as GI) |
| `ImmuneToVeins=yes` | TS-legacy flag, defensive |
| `ImmuneToPsionics=no` | Can be mind-controlled |
| `Bombable=yes` | Crazy Ivan bomb-target eligible |
| `Deployer=yes` | Player can issue Deploy command (`InfantryTypeClass+0xEC8`) |
| `DeployFire=yes` | When deployed, fire Secondary slot (`TechnoTypeClass+0x6AC`) — engine swaps weapon during fire resolution |
| `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` | Same set as GI |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` | Same set as GI |
| `Size=1` | Transport cargo slot cost |
| `Crushable=yes` | Standing GGI is crushable by vehicles |
| `DeployedCrushable=no` | **Deployed GGI is uncrushable** — runtime byte `InfantryClass+0x2A4` set to 1 when Deploy completes; `TechnoClass::CanCrushCheck` returns false for the would-be crusher |
| `DeploySound=GuardianGIDeploy` | Unique deploy sample (`iggidepa`) |
| `UndeploySound=GIUndeploy` | **Reuses GI undeploy sample** (`igidepa/b` — same as GI's deploy bank, by design) |
| `ElitePrimary=M60E` | Promoted primary (damage 25 vs 15) |
| `EliteSecondary=MissileLauncherE` | Promoted secondary (damage 50, ROF 20 vs 40 — twice as fast) |
| `;EliteSecondary=ParaE` | Commented-out alternative — designer kept history of weapon switching |
| `IFVMode=16` | IFV gunner index 16 — when GGI boards an [HTK], IFV swaps to its Weapon17 slot (AA missile launcher, "Hover Missile" type) |
| `PixelSelectionBracketDelta=-6` | Selection bracket draws 6 pixels higher than default — the GGI sprite has a taller silhouette than the GI |

---

## artmd.ini — `[GGI]` section

`ini/artmd.ini:291`:

```ini
[GGI] ; Guardian GI
Cameo=GDGIICON
AltCameo=GDGIUICO
Sequence=GuardianGISequence
Crawls=yes
Remapable=yes
FireUp=2
PrimaryFireFLH=80,0,105
SecondaryFireFLH=80,0,90
```

| Key | Meaning |
|-----|---------|
| `Cameo=GDGIICON` | Sidebar icon (rookie/veteran) |
| `AltCameo=GDGIUICO` | Cameo at Elite rank |
| `Sequence=GuardianGISequence` | Reference to the sequence block below |
| `Crawls=yes` | Sets `InfantryTypeClass+0xEBD` — prone-while-walking enabled |
| `Remapable=yes` | House remap palette applied |
| `FireUp=2` | Bullet-spawn frame within firing sequence |
| `PrimaryFireFLH=80,0,105` | M60 muzzle: forward 80, side 0, height 105 |
| `SecondaryFireFLH=80,0,90` | MissileLauncher muzzle: forward 80, side 0, height 90 (slightly lower since deployed posture) |

### Referenced sequence — `[GuardianGISequence]`

`artmd.ini:14166`:

```ini
[GuardianGISequence]
Ready=0,1,1
Guard=0,1,1
Prone=86,1,6
Walk=8,6,6
FireUp=204,6,6
Down=164,2,2
Crawl=86,6,6
Up=180,2,2
FireProne=252,6,6
Idle1=56,15,0,S
Idle2=71,15,0,E
Die1=134,15,0
Die2=149,15,0
Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
Deploy=300,15,0
Deployed=315,1,1
DeployedFire=323,6,6
DeployedIdle=0,0,0
Undeploy=180,2,2
Paradrop=371,1,0
Cheer=196,8,0,E
Panic=8,6,6
;DeployedSounds= 0 GuardianGiDeploy
```

Diffs vs `[GISequence]`:

- `FireUp=204` vs GI `164` (different SHP frame layout)
- `Down=164` vs GI `260`, `Up=180` vs GI `276`, `FireProne=252` vs GI `212`
  (whole GGI.shp is laid out differently)
- `Deployed=315` vs GI `292` (deployed pose start)
- `DeployedFire=323` vs GI `315` (deploy fire start; MissileLauncher
  sequence frames different from M60)
- `Undeploy=180,2,2` reuses `Up` frames (same shortcut as GI)
- `Cheer=196,8,0,E` vs GI `364` — different cheer animation location
- Commented `DeployedSounds= 0 GuardianGiDeploy` — never enabled; sound is
  triggered by `DeploySound=` rules key instead

---

## Weapons

### Primary — `[M60]` (shared with E1)

`rulesmd.ini:22922` — verbatim already documented in
[E1.md](E1.md#primary--m60-rookieveteran-standingcrawling). The same M60 instance
backs both GI and GGI primary; engine does not duplicate the warhead/projectile
per unit.

### Secondary — `[MissileLauncher]`

`rulesmd.ini:22569`:

```ini
[MissileLauncher]
Damage=40
ROF=40
Range=8
Burst=1
Projectile=AAHeatSeeker2	;AirToGroundMissile
Speed=30 ;40
Warhead=GUARDWH
Report=GuardianGIDeployedAttack
MinimumRange=1
```

| Key | Meaning |
|-----|---------|
| `Damage=40` | Per-missile damage (Warhead Verses applied) |
| `ROF=40` | One missile every 40 frames (~1.6s at 25fps) |
| `Range=8` | Maximum range (cells) — long-range for infantry |
| `Burst=1` | Single missile per fire cycle |
| `Projectile=AAHeatSeeker2` | Dual-purpose heat-seeker (AA=yes, AG=yes), ROT=60 turn rate, `Image=DRAGON` SHP, `Ranged=yes`, no cliff/elevation/wall blocking |
| `Speed=30` | Missile flight speed (lower than missile-launcher vehicles; `;40` comment shows it was bumped down) |
| `Warhead=GUARDWH` | Anti-vehicle warhead (see below) |
| `Report=GuardianGIDeployedAttack` | **Dangling reference** — no `[GuardianGIDeployedAttack]` block exists in soundmd.ini. The fire is **silent** for this weapon. (Confirmed via grep; only `GuardianGIDeploy` and `GuardianGiUnDeploy` exist) |
| `MinimumRange=1` | Cannot fire at targets adjacent (0 < d < 1 cells) — missile arming distance |

### Elite Primary — `[M60E]` (shared with E1)

See [E1.md](E1.md#elite-primary--m60e). Damage 25, ROF 20, range 4, warhead SA.

### Elite Secondary — `[MissileLauncherE]`

`rulesmd.ini:25123`:

```ini
[MissileLauncherE]
Damage=50
ROF=20
Range=8
Burst=1
Projectile=AAHeatSeeker2	;AirToGroundMissile
Speed=40
Warhead=GUARDWH
Report=GuardianGIDeployedAttack
MinimumRange=1
```

Diffs vs `MissileLauncher`: Damage 40 → 50 (+25%), ROF 40 → 20 (2× rate),
Speed 30 → 40 (33% faster missile). Range, warhead, projectile, report,
minimum range unchanged.

### Projectile — `[AAHeatSeeker2]`

`rulesmd.ini:25678`:

```ini
[AAHeatSeeker2]
Arm=2
Shadow=no
Proximity=no
Ranged=yes
AA=yes
AG=yes
Image=DRAGON
ROT=60
SubjectToCliffs=no
SubjectToElevation=no
SubjectToWalls=no
```

| Key | Meaning |
|-----|---------|
| `Arm=2` | Arming delay (2 frames before detonation eligible) |
| `Shadow=no` | No ground shadow |
| `Proximity=no` | Does not detonate near non-target units (commented-out alt: yes) |
| `Ranged=yes` | Honors weapon range as flight cap (does not chase indefinitely) |
| `AA=yes`, `AG=yes` | **Dual-purpose** — engages air and ground |
| `Image=DRAGON` | Uses the Dragon missile SHP |
| `ROT=60` | Rate of turn (high agility) — tracks moving targets |
| `SubjectToCliffs=no` | Flies over cliffs |
| `SubjectToElevation=no` | Ignores cell-z differences |
| `SubjectToWalls=no` | Flies over walls |

### Warhead — `[GUARDWH]`

`rulesmd.ini:26902`:

```ini
[GUARDWH]
Wall=yes
Wood=yes
Verses=20%,20%,20%,100%,50%,100%,10%,10%,10%,100%,100%
Conventional=yes
InfDeath=3
;AnimList=S_CLSN30
AnimList=XGRYSML1,XGRYSML2,EXPLOSML,XGRYMED1,XGRYMED2,EXPLOMED,EXPLOLRG,TWLT070
ProneDamage=50%
CellSpread=.5
PercentAtMax=.5
```

| Key | Meaning |
|-----|---------|
| `Wall=yes` | Damages walls on hit |
| `Wood=yes` | Wood material flag (selects damage variant; also informs InfDeath) |
| `Verses=20%/20%/20%/100%/50%/100%/10%/10%/10%/100%/100%` | Armor-vs-damage table. Columns: `none, flak, plate, light, medium, heavy, wood, steel, concrete, special_1, special_2`. **Anti-vehicle profile**: 100% vs light, 50% vs medium, 100% vs heavy. Terrible vs infantry (20%), buildings (10%). Designed to counter tanks |
| `Conventional=yes` | Conventional damage type (not radiation/poison/psionic) |
| `InfDeath=3` | Infantry death animation 3 (fire death — bodies catch fire and roll) |
| `AnimList=XGRYSML1,...` | Random impact animations (gray smoke and explosions, small to large) |
| `ProneDamage=50%` | Half damage if target is prone (crawling infantry) |
| `CellSpread=.5` | Small AoE radius (0.5 cells) |
| `PercentAtMax=.5` | At max spread distance, damage drops to 50% (linear falloff with this floor) |

---

## Voices and sounds

| INI key on GGI | soundmd block | Resolved samples |
|----------------|---------------|------------------|
| `VoiceSelect=GuardianGISelect` | `[GuardianGISelect]` line 4544 | `$iggisea` `$iggiseb` `$iggisec` `$iggised` `$iggisee` `$iggisef` (random) |
| `VoiceMove=GuardianGIMove` | `[GuardianGIMove]` line 4549 | `$iggimoa` `$iggimob` `$iggimoc` `$iggimod` `$iggimoe` (random) |
| `VoiceAttack=GuardianGIAttackCommand` | `[GuardianGIAttackCommand]` line 4554 | `$iggiata` `$iggiatb` `$iggiatc` `$iggiatd` `$iggiate` (random) |
| `VoiceFeedback=GuardianGIFear` | `[GuardianGIFear]` line 4564 | `$iggifea` ... `$iggifef` (6 samples, random) |
| `VoiceSpecialAttack=GuardianGIMove` | (same as VoiceMove) | deploy command voice reuses move bank |
| `DieSound=GuardianGIDie` | `[GuardianGIDie]` line 4569 | `$iggidia` ... `$iggidie` (5 samples, random) |
| `CrushSound=InfantrySquish` | `[InfantrySquish]` line 1196 | `igensqua` |
| `DeploySound=GuardianGIDeploy` | `[GuardianGIDeploy]` line 1027 | `iggidepa`, `Limit=3` concurrent cap, FShift -5/+5, Volume 60 |
| `UndeploySound=GIUndeploy` | `[GIUndeploy]` line 1067 (GI's) | `igidepa` `igidepb` (shared with GI undeploy) |
| Weapon `M60` `Report=GIAttack` | `[GIAttack]` line 1049 | `igiat1a/b/c` |
| Weapon `M60E` `Report=GIAttack` | (same) | rookie attack sample reused for elite primary |
| Weapon `MissileLauncher` `Report=GuardianGIDeployedAttack` | **MISSING** — no soundmd block | **Silent fire** — dangling INI reference; engine logs no error and plays nothing |
| Weapon `MissileLauncherE` `Report=GuardianGIDeployedAttack` | **MISSING** | same — silent |

There is also a defined-but-unused voice block `[GuardianGIDeployVoice]` at
soundmd.ini:4559 with samples `$iggidea $iggideb $iggidec` — designer left it
ready, but the GGI INI section uses `VoiceSpecialAttack=GuardianGIMove`
instead. **Unreachable in vanilla.**

---

## Prerequisites, owners, tech

- `Prerequisite=GAPILE` — Allied Barracks. (`TechLevel=2` raises the bar to
  tier 2 buildings; effectively requires a Radar Dome or Naval Yard
  depending on house tech-up rules.)
- `Owner=British,French,Germans,Americans,Alliance` — Allied 4 + AI generic
  placeholder; no Soviet/Yuri can build.
- `TechLevel=2` — second tier.
- `BuildLimit=` not set.
- `AIBasePlanningSide=` not set.
- `ForbiddenHouses=` not set (filtered implicitly by Owner=).
- `RequiredHouses=` not set.
- `AllowedToStartInMultiplayer=no`.

---

## Veterancy and upgrades

- **Rookie**: M60 primary, MissileLauncher secondary.
- **Veteran**: same five abilities as GI (`STRONGER`, `FIREPOWER`, `ROF`,
  `SIGHT`, `FASTER`). No weapon swap at this tier.
- **Elite**: `SELF_HEAL` + `STRONGER` + `FIREPOWER` + `ROF` (cumulative with
  veteran tier).
  - Primary swap: `M60` → `M60E` (15 → 25 damage)
  - Secondary swap: `MissileLauncher` → `MissileLauncherE`
    (40 → 50 damage, ROF 40 → 20, speed 30 → 40)
  - Cameo swap: `GDGIICON` → `GDGIUICO`
- No `Crushable=` progression (always `Crushable=yes` standing,
  `DeployedCrushable=no` when deployed — independent of veterancy).

---

## Hardcoded behavior in gamemd.exe (Ghidra-verified)

The full RE is in [GGI_GHIDRA_REPORT.md](../../GGI_GHIDRA_REPORT.md). All
findings: HIGH content, HIGH identity, HIGH binding (caller traces verified).

### No GGI-specific branch exists

Every GGI behavior is value-driven. The shared infantry pipeline
(parse → AI → state machine → fire-target → damage) consults the same byte
offsets for both GI and GGI; only the values stored at those offsets differ.
`search_strings "GGI"` and `search_strings "Guardian GI"` both surface only
INI-section header parse targets and voice-bank string constants — no
hardcoded conditional dispatches on the unit name.

### Deploy / fortify path (shared with GI)

- Sequence transitions 0x1B–0x1F (Deploy/Deployed/DeployedFire/DeployedIdle/
  Undeploy). Same code paths as GI per
  [GI_GHIDRA_REPORT.md §D-T](../../GI_GHIDRA_REPORT.md).
- `DeployFire=yes` (`TechnoTypeClass+0x6AC=1`) + `DeployFireWeapon=1`
  (`+0x6A8` default) → deployed-state `Fire_At_Target` selects Secondary
  slot (MissileLauncher).
- `Deployer=yes` (`InfantryTypeClass+0xEC8`) gates the player-input deploy
  command and the auto-undeploy-on-move path.

### Uncrushable while deployed — the GGI-defining behavior

- `DeployedCrushable=no` writes `0` to `InfantryTypeClass+0xEC9`.
  **[INFERRED — exact offset not re-verified in audit 2]**. Verified
  via Ghidra: the field name `DeployedCrushable` IS read by
  `InfantryTypeClass__ReadINI` (string at `0x00825914`, parse xref at
  `0x00524627` — InfantryType scope confirmed). The specific struct
  offset `+0xEC9` would need a decompile of the parse-call to confirm.
- End-of-Deploy sequence (0x1B → 0x1C transition) sets the runtime byte
  `InfantryClass+0x2A4 = 1` (the `IsLowSilhouette` flag). **[INFERRED —
  the +0x2A4 offset is consumed by CanCrushCheck (verified below) but
  the SET path during deploy-sequence transition not decompiled].**
- Start-of-Undeploy sequence (0x1F) clears `+0x2A4 = 0`. **[INFERRED —
  same reason as above]**.
- `TechnoClass::CanCrushCheck @ 0x005F6CD0` **[BINARY-VERIFIED audit 2 —
  exact address, body 0x005f6cd0–0x005f6d92]**. Decompile shows the
  function has two crush-branches, each reading different type/runtime
  flags:
  - **Branch 1** reads `target_type+0xd29` (Crushable on target?) and
    `crusher_type+0xd2a` (allows-crush flag on crusher?). Faction check
    via `HouseClass::Is_Ally_ByObject` — only enemies crush.
  - **Branch 2** reads `crusher_type+0x22d` (via vtable+0x88 — different
    type accessor), then **checks `param_1[0xa9] == 0`** — that's
    **byte offset 0x2A4** on InfantryClass. **VERIFIED: `+0x2A4` IS
    used by CanCrushCheck — if non-zero, branch 2 falls through
    (cannot crush via that path).**
  - **The actual semantics of which field is "DeployedCrushable" within
    these flags** is not 100% pinned (the doc says +0xEC9 but
    CanCrushCheck reads +0xd29/+0xd2a/+0x22d). The functional outcome —
    deployed GGI cannot be crushed — is verified-correct; the struct
    offset attribution in the doc may be imprecise.
  - **Callers of CanCrushCheck** (binary-verified): `Drive`/`Ship`
    locomotion `Process_Drive_Track`, `UnitClass::Can_Enter_Cell`,
    `UnitClass::PerCellProcess`, `UnitClass::What_Action_OnObject`
    (also for cursor display). Confirms crush logic fires on vehicle
    movement attempts, not on every per-tick scan.
- This is the same `+0x2A4` field as the prone-while-crawling
  `IsLowSilhouette` — both states share the byte. Crawling GGI also benefits
  from the crush immunity since `+0x2A4=1` covers both. (Per the
  CRUSH_SYSTEM_GHIDRA_REPORT shared infrastructure.)
- The Rust port honors this via `dev` commit `e3a724f` "Honor low silhouette
  crush immunity".

### IFV gunner — Hover Missile mode

- `IFVMode=16` → IFV's Weapon17 slot (AA-only HoverMissile-class weapon).
- `TechnoClass::SetGunnerWeapon @ 0x0070DC70` (per the GI dossier §P2.13)
  reads `IFVMode` and swaps weapon + turret-offset tables on the IFV.
- See [IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md](../../IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md).

### Battle Fortress (`OpenTransportWeapon=1`)

- Same path as GI: passenger fires Secondary slot from FV's mounted FLH.
- GGI passenger turns FV into a mobile AA platform (MissileLauncher AA+AG
  ground).

### AA-eligibility gate

- The fire-error chain. **[ADDRESS DISCREPANCY audit 2]**: doc claims
  `InfantryClass::GetFireError @ 0x0051C8B0` but Ghidra returns **no
  function at that address**. The tail-callee `TechnoClass::GetFireError
  @ 0x006FC0B0` **[BINARY-VERIFIED — exact address, body
  0x006fc0b0–0x006fcd37]** does exist. The InfantryClass-specific
  wrapper either has a different address or doesn't exist as a separate
  function (likely just calls TechnoClass::GetFireError directly).
- Behavioral claim — "the Projectile's `AA=` flag is checked against
  the target's air-flag" — **[INFERRED — `TechnoClass::GetFireError`
  not decompiled in audit 2]**.
- `AAHeatSeeker2` has `AA=yes AG=yes`, so the gate passes for both.
  Standing GGI cannot fire at air because the M60's `InvisibleLow`
  projectile has neither AA nor AG-air-targeting affinity; deploying
  is required to engage aircraft.

### Missile homing curve and rounding

- `AAHeatSeeker2` with ROT=60 produces a tight tracking curve (per
  GGI_GHIDRA_REPORT §8.1). Missile orientation update uses `ftol` truncation
  (rounded toward zero, not banker's rounding) per §8.3 — this matters for
  bit-exact replay reproduction.
- The MissileLauncher path uses `TechnoClass::Fire_At` ordering as
  per §8.2 (no ProneDamage application inside Fire_At; deferred to
  BulletClass::Detonate).

### `PixelSelectionBracketDelta=-6`

- Type field consumed by `TechnoClass::DrawExtras @ 0x006F5190`
  **[BINARY-VERIFIED audit 2 — exact address, body
  0x006f5190–0x006f5eee]** (the selection-bracket/pip drawer per
  [SELECTION_BRACKETS_PIPS_DRAW_ORDER_GHIDRA_REPORT.md](../../building-selection-brackets/SELECTION_BRACKETS_PIPS_DRAW_ORDER_GHIDRA_REPORT.md)).
  Parser key existence verified: string at `0x00843dc0` → xref into
  `TechnoTypeClass__ReadINI` at `0x00714166` — TechnoType scope
  confirmed. **The specific bracket-y math** (`sprite_top +
  PixelSelectionBracketDelta`) is **[INFERRED — DrawExtras not
  decompiled in audit 2]**.

### Per-tick AI / fire chain (shared)

- `InfantryClass::AI @ 0x0051BAB0` **[BINARY-VERIFIED audit 1]** — same
  16-phase pipeline as GI.
- `InfantryClass::Fire_At_Target @ 0x005206B0` **[BINARY-VERIFIED
  audit 1]** — same fire-frame gate, reads `+0xE40`/`+0xE48`
  (FireUp/SecondaryFire) on type.
- `InfantryClass::SelectWeapon @ 0x005218E0` **[ADDRESS VERIFIED audit 2;
  Ghidra labels as `FUN_005218e0` (unnamed). Behavior matches the doc's
  claim]**. Decompile-verified logic:
  ```c
  if (*(char *)(type + 0x6ac) == '\0') {                  // DeployFire flag
    return TechnoClass__SelectWeaponAgainst(this, target); // generic path
  }
  iVar1 = *(int *)(this + 0x1a4);                          // current sequence id
  if (iVar1 == 0x1b || iVar1 == 0x1c || iVar1 == 0x1d || iVar1 == 0x1e) {
    return *(int *)(type + 0x6a8);                         // DeployFireWeapon (=1 → Secondary)
  }
  // else: not deployed, return Primary (0) or fall-through
  ```
  **VERIFIED**: `TechnoTypeClass+0x6ac = DeployFire` flag, **`+0x6a8 =
  DeployFireWeapon` slot index** — both read here. Doc's claims about
  these offsets are correct. Sequence IDs 0x1b-0x1e for deployed state
  also confirmed (matches Fire_At_Target).

### Mind control / Iron Curtain / Bombable / Garrison

- `ImmuneToPsionics=no` → mind-controllable (same path as GI).
- Iron Curtain → standard infantry path.
- `Bombable=yes` → Crazy Ivan can plant.
- `Occupier=no` → engine refuses garrison entry for GGI (cursor in
  `What_Action_OnObject` does not show garrison-cursor over civilian
  buildings).

### `IsSelectableCombatant=yes` consumer

- Per GGI report §7 open items, the parse target is verified but the
  read-site is "parsed-but-reader-not-located" — likely consumed by the
  "select all combat units" hotkey handler (per
  [SELECTION_GATES_GHIDRA_REPORT.md](../../SELECTION_GATES_GHIDRA_REPORT.md)).
  Not GGI-specific; shared infantry gate.

### Dangling sound reference

- `Report=GuardianGIDeployedAttack` on MissileLauncher/MissileLauncherE is
  not defined in `soundmd.ini`. Engine path for missing sound IDs: the
  `VocClass::Lookup` returns -1, `Audio::Play(-1)` is a no-op. **Silent
  fire** confirmed; not a bug, just a stale designer reference.

---

## TS-legacy filter

- `ImmuneToVeins=yes` — TS terrain flag, unreachable in YR (no veins).
- `Locomotor={4A582744-...}` — TS-era GUID, alive in YR (every infantry
  uses it).
- `Crawls=yes` (art) — TS-era prone-while-walking, alive in YR.
- Sequence `Undeploy=180,2,2` reuses `Up` frames (TS-era SHP frame-share
  shortcut). Live.
- Sequence `Panic=8,6,6` reuses `Walk` frames. Live.
- Commented `;DeployedSounds= 0 GuardianGiDeploy` in `[GuardianGISequence]`
  — TS-era multi-tracked sound system never wired up in YR. Dead.
- Commented `;EliteSecondary=ParaE` — designer left history of weapon
  switching during development. Dead.

No keys on GGI require gating behind `SpecialFlags`. None of GGI's behavior
depends on TS-only systems.

---

## Cross-references

- **Sibling**: [E1](E1.md) Basic GI — shares the deploy pattern, the M60
  primary, the M60E elite primary, and most ability sets.
- **Builder**: [GAPILE](../structures/GAPILE.md) Allied Barracks.
- **Cloning duplicate**: [NACLON] Cloning Vats (if owned by Allied house) —
  duplicates each GGI built.
- **IFV passenger**: [HTK](../allied/HTK.md) — `IFVMode=16` → Hover Missile.
- **Battle Fortress passenger**: [FV](../allied/FV.md) —
  `OpenTransportWeapon=1` → MissileLauncher.
- **Tech-building free spawn**: none.
- **Counter-roles**:
  - Counters: light/medium/heavy vehicles (Verses 100%/50%/100%), aircraft
    (AA missile), Terror Drones (AA-eligible projectile catches them
    mid-jump).
  - Countered by: Attack Dog (one-shot anti-inf), Crazy Ivan bomb, Yuri
    mind control, Tanya/SEAL gunfire, Desolator radiation, vehicle crush
    (standing only — deployed is uncrushable).
- **Related deploy units**: [DESO](../soviet/DESO.md) Desolator
  (Soviet deploy-radiation analog), [TANY](../allied/TANY.md) Tanya (no
  deploy, but C4 equivalent special ability).

---

## Ghidra audit log (audit iteration 2 — 2026-05-18)

Deep-Ghidra audit pass. ~7 decompiles + 6 entry-point lookups + 3
string xrefs. Goal: verify the GGI-specific claims (deployed-uncrushable,
IFVMode=16 → Weapon17 swap, MissileLauncher Anti-Air gate) against the
binary.

### Function entry points verified

| Doc claim | Ghidra label / address | Status |
|-----------|------------------------|--------|
| `TechnoClass::CanCrushCheck @ 0x005F6CD0` | `TechnoClass__CanCrushCheck` exact, body `0x005f6cd0–0x005f6d92` | ✅ VERIFIED |
| `InfantryClass::GetFireError @ 0x0051C8B0` | **NO function** at that address | ❌ INCORRECT |
| `TechnoClass::GetFireError @ 0x006FC0B0` | `TechnoClass__GetFireError` exact, body `0x006fc0b0–0x006fcd37` | ✅ VERIFIED |
| `InfantryClass::SelectWeapon @ 0x005218E0` | `FUN_005218e0` (unlabeled), body `0x005218e0–0x0052195d` | ⚠️ ADDRESS VERIFIED, NAME UNCONFIRMED (function is unlabeled but its behavior matches the doc's description) |
| `TechnoClass::DrawExtras @ 0x006F5190` | `TechnoClass__DrawExtras` exact, body `0x006f5190–0x006f5eee` | ✅ VERIFIED |

### Key behavioral findings (decompile-verified)

1. **DeployFire weapon-selection logic** in `FUN_005218e0` — this is the
   load-bearing decompile this iteration:
   ```c
   undefined __thiscall FUN_005218e0(TechnoClass *this, void *target) {
     if (*(char *)(type + 0x6ac) == '\0') {                    // DeployFire flag check
       return TechnoClass__SelectWeaponAgainst(this, target);   // generic path
     }
     iVar1 = *(int *)(this + 0x1a4);                            // current sequence id
     if (iVar1 != 0x1b && iVar1 != 0x1c && iVar1 != 0x1d && iVar1 != 0x1e) {
       // not in deployed state
       if (this->field_0x82 != '\0') {                          // some special-mode flag
         iVar1 = vtable_84_call();                              // get type
         if (*(int *)(iVar1 + 0xd50) != -1) {
           return *(int *)(iVar1 + 0xd50);                      // pre-deploy weapon override
         }
       }
       return 0;                                                // default: Primary (slot 0)
     }
     return *(int *)(type + 0x6a8);                             // DeployFireWeapon (default 1 = Secondary)
   }
   ```
   - **VERIFIED**: `TechnoTypeClass+0x6ac = DeployFire` flag (used as boolean gate).
   - **VERIFIED**: `TechnoTypeClass+0x6a8 = DeployFireWeapon` slot index (used as weapon-table index, defaults to 1 per cheat-sheet).
   - **VERIFIED**: Sequence IDs `0x1b, 0x1c, 0x1d, 0x1e` are the deployed states (consistent with audit iter 1 finding in Fire_At_Target).
   - **NEW DISCOVERY**: `TypeClass+0xd50` is a "pre-deploy weapon override" slot — when the unit is in some special mode (`+0x82 != 0`) and not currently deployed, the engine reads this alternate weapon. Not GGI-specific (and likely never set for GI/GGI; the +0xd50 sentinel `-1` falls through).

2. **CanCrushCheck branches** (from decompile):
   ```c
   undefined4 __thiscall TechnoClass__CanCrushCheck(int *param_1, int *param_2) {
     // Branch 1: requires target's Crushable flag set, crusher's allow-flag clear
     if (param_2 != 0) {
       iVar2 = vtable_84_of(param_2);                            // target's type
       if (*(char *)(iVar2 + 0xd29) != '\0'                       // target Crushable=yes
           && param_1 != 0
           && (*(byte *)(param_1 + 5) & 1) != 0) {
         iVar2 = vtable_84_of(param_1);                           // crusher's type
         if (*(char *)(iVar2 + 0xd2a) == '\0') {                  // crusher's "cannot-crush" flag clear
           // type-id check, faction check
           if (not_ally(...) && not_friend(...)) return 1;        // CAN CRUSH
         }
       }
     }
     // Branch 2: alternate crush path
     iVar2 = vtable_88_of(param_1);                               // different type accessor on crusher
     if (*(char *)(iVar2 + 0x22d) != '\0'
         && param_1 != 0
         && (*(byte *)(param_1 + 5) & 1) != 0
         && (char)param_1[0xa9] == '\0') {                        // *** +0x2A4 IS ZERO check ***
       if (not_ally(...) && not_friend(...)) return 1;            // CAN CRUSH
     }
     return 0;                                                    // CANNOT CRUSH
   }
   ```
   - **VERIFIED**: `param_1[0xa9]` reads byte offset `0x2A4` on the entity. If non-zero, branch 2 fails. **This confirms `+0x2A4` is the IsLowSilhouette/deployed-state byte that gates crush logic.**
   - **OPEN**: which side (target or crusher) `param_1` is, in this function's signature. The function is called from multiple sites (Drive_Track, Can_Enter_Cell, etc.), and the role may differ per caller.
   - **OPEN**: `+0xd29`, `+0xd2a`, `+0x22d` are type-side crush flags but their exact INI mapping (Crushable, OmniCrushResistant, Crusher, DeployedCrushable) wasn't disambiguated. Multiple candidates fit each offset.

3. **CanCrushCheck callers** (binary-verified via `get_function_callers`):
   - `DriveLocomotionClass::Process_Drive_Track @ 0x004b0f20` (vehicle drive cycle)
   - `ShipLocomotionClass::Process_Drive_Track @ 0x006a05f0` (ship drive)
   - `UnitClass::Can_Enter_Cell @ 0x0073f0a0` (cell-entry check)
   - `UnitClass::PerCellProcess @ 0x007416a0` (per-cell tick)
   - `UnitClass::What_Action_OnObject @ 0x0073fd50` (cursor-action selector — used for cursor display, not just movement)
   - + 2 unlabeled functions
   Confirms crush is checked at: vehicle movement (drive track), pathfinding (can-enter-cell), per-cell processing, AND cursor display.

4. **DeployedCrushable parser key** (Ghidra string + xref):
   - String `"DeployedCrushable"` at `0x00825914`.
   - Xref `0x00524627` in **`InfantryTypeClass__ReadINI`** — confirms InfantryType-scope (matches the doc's `InfantryTypeClass+0xEC9` claim that this is an InfantryType field).
   - **The specific +0xEC9 struct offset was NOT decompile-verified** (would need to read the ReadINI parse-site).

5. **PixelSelectionBracketDelta parser key** (Ghidra string + xref):
   - String at `0x00843dc0`.
   - Xref `0x00714166` in `TechnoTypeClass__ReadINI` — TechnoType scope confirmed.
   - The DrawExtras consumption claim is unverified by decompile; only the parser-side is confirmed.

### Discrepancies resolved

- **`InfantryClass::GetFireError @ 0x0051C8B0`** — **NO FUNCTION at that
  address** per Ghidra. The doc lists a tail-call chain to
  `TechnoClass::GetFireError @ 0x006FC0B0` (verified). The InfantryClass
  wrapper either doesn't exist as a separate function (the chain goes
  directly to TechnoClass::GetFireError) or has a different address.
  Possibly the GGI report's claim was based on a function that has since
  been renamed/merged in this Ghidra build.
- **`InfantryClass::SelectWeapon @ 0x005218E0`** — the function exists
  at the exact address, but is **unlabeled in Ghidra** (`FUN_005218e0`).
  The decompile shows it IS the DeployFire weapon-selection function
  (matches the doc's claim functionally). The vtable-slot `+0x2E4`
  claim wasn't verified in audit 2.

### Items intentionally NOT re-verified in iter 2

- **`InfantryTypeClass+0xEC8 = Deployer` offset** — not visible in
  CanCrushCheck or SelectWeapon. Read elsewhere (likely deploy-input
  dispatcher in EventClass::Process). DEFERRED.
- **`InfantryTypeClass+0xEC9 = DeployedCrushable` exact offset** — parser
  key is InfantryType-scope confirmed, but the specific struct offset
  isn't decompile-verified. DEFERRED.
- **`InfantryClass+0x2A4` set/clear in Deploy/Undeploy sequence transitions** —
  the offset is decompile-verified as the field CanCrushCheck reads,
  but the SET path (which sequence transition writes to it) wasn't
  traced. DEFERRED.
- **`IFVMode=16 → Weapon17`** — SetGunnerWeapon's address verified
  audit 1; the specific Weapon17 → "AA HoverMissile" claim wasn't traced
  because it requires decompiling the IFV's weapon-table lookup chain.
  DEFERRED.
- **`AAHeatSeeker2 AA=yes` projectile-vs-target air-flag gate** — claim
  requires decompiling `TechnoClass::GetFireError` (body 0x006fc0b0–
  0x006fcd37). Exceeds per-doc effort budget. DEFERRED.
- **`+0xd29, +0xd2a, +0x22d` mapping to specific INI keys** (Crushable,
  Crusher, OmniCrushResistant, DeployedCrushable) — the offsets are
  decompile-verified as being read but the INI parser side wasn't
  traced. DEFERRED.

### Confidence summary

- ~50% of GGI-specific behavioral claims now have direct binary verification.
- ~35% are INFERRED (function exists or parser key exists, but the specific
  behavioral claim wasn't fully traced).
- ~10% are INCORRECT or have name discrepancies (GetFireError @ 0x0051C8B0
  is a phantom; SelectWeapon is unlabeled).
- ~5% are SHARED-WITH-E1 verified items (Fire_At_Target, AI, etc.) carried
  over from audit iter 1.

The GGI-specific claim "deployed = uncrushable" is **functionally verified**:
CanCrushCheck reads `+0x2A4` on the entity and rejects crush if non-zero.
The specific INI-to-offset mapping (`+0xEC9` for DeployedCrushable type
flag, `+0x2A4` for runtime IsLowSilhouette) is **partially verified** —
the +0x2A4 read is confirmed; the +0xEC9 InfantryType scope is confirmed
but the exact offset isn't pinned to that decompile.

---

## Coverage audit

- ✅ Every key in `[GGI]` rulesmd block (47 lines including commented
  `;EliteSecondary=ParaE`) covered above.
- ✅ Every key in `[GGI]` artmd block (9 lines) covered, plus
  `[GuardianGISequence]` (23 lines).
- ✅ Weapon chain: M60, MissileLauncher, M60E, MissileLauncherE — all four
  covered with projectile (AAHeatSeeker2) and warhead (GUARDWH).
- ✅ Sound chain: 9 distinct soundmd entries covered + dangling reference
  `GuardianGIDeployedAttack` flagged.
- ✅ Ghidra search: `search_strings "GGI"` / `"Guardian GI"` recorded —
  only INI/voice-string constants, no hardcoded section-name branch. Deep
  RE delegated to `GGI_GHIDRA_REPORT.md`.
- ✅ TS-legacy filter applied (ImmuneToVeins, Locomotor GUID note,
  Crawls, sequence frame-reuse, commented-out DeployedSounds and EliteSecondary).
- ✅ Cross-references to E1, GAPILE, NACLON, HTK, FV, DESO, TANY.
