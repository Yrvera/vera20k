# CIVAN — Chrono Ivan

**Side classification:** Soviet-themed (Ivan + Soviet tech-origin). Universally
buildable via tech-steal.
**Role:** Tech-steal demolition specialist — non-Soviet houses gain a teleporting
Crazy Ivan by infiltrating a Soviet Battle Lab with a Spy.
**Tech-steal triplet:** CCOMAND (Allied tech) / **CIVAN (Soviet tech)** / PTROOP (Yuri tech).

> Output bar: indistinguishable from gamemd.exe for the player. INI is the source of
> truth; gamemd contains **no** `"CIVAN"` / `"ChronoIvan"` strings (verified — see §7),
> so every behavior described below is driven by generic TechnoType / WeaponType /
> WarheadType / locomotor handling combined with the global `Ivan*` rules and the
> BombClass system. The full bomb-plant mechanics are reverse-engineered in
> [BOMB_CLASS_GHIDRA_REPORT.md](../../BOMB_CLASS_GHIDRA_REPORT.md) (941 lines) — this
> doc cross-references but does not duplicate that report.

---

## 1. `rulesmd.ini` — `[CIVAN]` verbatim

```ini
[CIVAN] ; anybody gets into a soviet tech center
UIName=Name:CIVAN
Name=Chrono Ivan
Category=Soldier
Prerequisite=BARRACKS
RequiresStolenSovietTech=yes
Primary=IvanBomber
CrushSound=InfantrySquish
Crushable=no
TiberiumProof=yes
Strength=100
Armor=none
TechLevel=9
Pip=red
Sight=8
Speed=6
Owner=Russians,Confederation,Africans,Arabs,YuriCountry,British,French,Germans,Americans,Alliance
AllowedToStartInMultiplayer=no
Cost=1750
Soylent=500
Points=50
IsSelectableCombatant=yes
VoiceSelect=CrazyIvanSelect
VoiceMove=CrazyIvanMove
VoiceAttack=CrazyIvanAttackCommand
VoiceFeedback=CrazyIvanFear
VoiceSpecialAttack=CrazyIvanAttackCommand
DieSound=CrazyIvanDie
ChronoInSound=ChronoLegionTeleport
ChronoOutSound=ChronoLegionTeleport
Locomotor={4A582747-9839-11d1-B709-00A024DDAFD1}
;Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}; <-Walk  teleport->{4A582747-9839-11d1-B709-00A024DDAFD1}
PhysicalSize=1
MovementZone=Infantry
ThreatPosed=25	; This value MUST be 0 for all building addons
SpecialThreatValue=1
ImmuneToVeins=yes
Ivan=yes;needed to differentiate from Bomber, which is C4, and engineer
;Bombable=no
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
;Deployer=yes
;UndeployDelay=20
AttackCursorOnFriendlies=yes
MoveToShroud=no
IFVMode=7
Teleporter=yes;
Trainable=no ;gs like regular ivan, since can't gain experience
```

### Key-by-key explanation

| Key | Value | Scope | Effect |
|-----|-------|-------|--------|
| `;` (comment) | `anybody gets into a soviet tech center` | — | Author-note: any house can build CIVAN after Spy infiltrates a Soviet Battle Lab. |
| `UIName` | `Name:CIVAN` | AbstractType | CSF key for the sidebar/cursor label. |
| `Name` | `Chrono Ivan` | AbstractType | Dev/fallback name. |
| `Category` | `Soldier` | TechnoType | AI targeting classifier (overridden in practice by `Crushable=no`). |
| `Prerequisite` | `BARRACKS` | TechnoType | Generic barracks token — resolves per-house: Allied→GAPILE, Soviet→NAHAND, Yuri→YABRCK. |
| `RequiresStolenSovietTech` | `yes` | TechnoType (verified — see §7) | Build gate. Set on the house when its Spy successfully infiltrates a Soviet tech building. Verified string at 0x00843be0, read by TechnoTypeClass__ReadINI @ 0x007144f7. |
| `Primary` | `IvanBomber` | TechnoType | Main weapon — see §3. Different from regular Crazy Ivan's `IvanBomber` (same name, but the weapon definition is shared). The "place a bomb" mechanic comes from the warhead, not from `Ivan=yes`. |
| `CrushSound` | `InfantrySquish` | TechnoType | Squish sound (n/a because `Crushable=no`). |
| `Crushable` | `no` | TechnoType | Cannot be crushed by tanks. |
| `TiberiumProof` | `yes` | InfantryType | **TS-LEGACY** — no tiberium in YR. Dormant. |
| `Strength` | `100` | AbstractType | Hitpoints. |
| `Armor` | `none` | TechnoType | Slot 1 in target warheads. |
| `TechLevel` | `9` | TechnoType | Endgame tech-tree slot (combined with the stolen-tech gate). |
| `Pip` | `red` | InfantryType | Carry-passenger pip colour. |
| `Sight` | `8` | TechnoType | Reveal radius. |
| `Speed` | `6` | TechnoType | Move speed — **higher than regular IVAN (4)**. The teleport locomotor uses this for the warp cadence rather than walk-pace. |
| `Owner` | full 10-country list + `Alliance` | TechnoType | All countries can own CIVAN. |
| `AllowedToStartInMultiplayer` | `no` | TechnoType | Not in starting tech tree; requires tech-steal unlock. |
| `Cost` | `1750` | TechnoType | Build cost — **the most expensive tech-steal infantry** (CCOMAND $1500, PTROOP $1000). |
| `Soylent` | `500` | TechnoType | Grinder refund — only ~28% of Cost (CCOMAND 50%, PTROOP 50%). Yuri loses money grinding CIVAN. |
| `Points` | `50` | TechnoType | Score on kill. |
| `IsSelectableCombatant` | `yes` | TechnoType | Counted in select-all-combat. |
| `VoiceSelect` | `CrazyIvanSelect` | TechnoType | Reuses regular Crazy Ivan voice set (7 select clips). See §5. |
| `VoiceMove` | `CrazyIvanMove` | TechnoType | 5 clips. |
| `VoiceAttack` | `CrazyIvanAttackCommand` | TechnoType | 4 clips. |
| `VoiceFeedback` | `CrazyIvanFear` | TechnoType | 2 clips ("under attack" lines). Regular IVAN shares these. |
| `VoiceSpecialAttack` | `CrazyIvanAttackCommand` | TechnoType | Same as primary attack — no unique special voice. |
| `DieSound` | `CrazyIvanDie` | TechnoType | 2 clips. |
| `ChronoInSound` | `ChronoLegionTeleport` | TechnoType (paired with Teleporter) | Played when CIVAN warps **into** a cell. Reuses Chrono Legionnaire's teleport sound (single clip `ichrmova`, `Limit=1` to prevent spam). |
| `ChronoOutSound` | `ChronoLegionTeleport` | TechnoType | Played when CIVAN warps **out** of a cell. Same sound def. |
| `Locomotor` | `{4A582747-9839-11d1-B709-00A024DDAFD1}` | TechnoType | **Teleport locomotor** (TeleportLocomotionClass) — same CLSID as Chrono Legionnaire and Chrono Commando. CIVAN does **not** walk; it warps cell-to-cell. |
| `;Locomotor=...` (commented) | walk CLSID + author note | — | Author-comment preserves the walk-locomotor CLSID and labels both options. The shipped unit uses teleport. |
| `PhysicalSize` | `1` | TechnoType | Sub-cell footprint. |
| `MovementZone` | `Infantry` | TechnoType | Pathing zone — infantry-walkable cells; teleport still respects zone, but skips intermediate path cells. |
| `ThreatPosed` | `25` | TechnoType | Mid-high AI threat. |
| `SpecialThreatValue` | `1` | TechnoType | High-value special-threat marker. |
| `ImmuneToVeins` | `yes` | TechnoType | **TS-LEGACY** dormant. |
| `Ivan` | `yes` | (likely InfantryType — see §7.1 caveat) | INI-comment: "needed to differentiate from Bomber, which is C4, and engineer". **Caveat**: no `"Ivan"` plain string exists in `gamemd.exe` (only `IvanBomb`, `IvanDamage`, `IvanWarhead`, `IvanTimedDelay`, `IvanIconFlickerRate`). The bomb-plant behavior is driven entirely by the **warhead's `IvanBomb=yes`**, not by this unit flag. The unit-level `Ivan=yes` is therefore either dormant or read into a field that no live code path tests against. Documented but flagged as unverified-as-live in §7.1. |
| `;Bombable=no` | *(commented)* | — | Inert. Default `Bombable=yes` applies — meaning **CIVAN can himself be Ivan-bombed by another Crazy Ivan** (or by another CIVAN). |
| `VeteranAbilities` | `STRONGER,FIREPOWER,ROF,SIGHT,FASTER` | TechnoType | But see `Trainable=no` below — these never trigger in normal play. |
| `EliteAbilities` | `SELF_HEAL,STRONGER,FIREPOWER,ROF` | TechnoType | Same caveat. |
| `;Deployer=yes / ;UndeployDelay=20` | *(commented)* | — | Inert design churn. |
| `AttackCursorOnFriendlies` | `yes` | TechnoType (verified at TechnoTypeClass scope per cheat sheet) | The targeting cursor shows the "attack" reticle when hovered over friendly units — CIVAN can plant bombs on friendlies (e.g., on a transport to use it as a delivery bomb), so the cursor must indicate this is intentional. Shared with regular IVAN, CCOMAND. |
| `MoveToShroud` | `no` | TechnoType (cheat sheet) | Cannot be ordered to move into unrevealed shroud. Prevents teleport-into-shroud exploits (CIVAN can't teleport-scout). |
| `IFVMode` | `7` | TechnoType (verified 0x00714787) | When boarded into Allied IFV (`[FV]`), this index selects **`ExplodeTurretWeapon=7`** → `Weapon8=CRNuke` (Crazy Ivan IFV slot). CIVAN in an IFV fires the same Ivan-bomb projectile as the standard IVAN passenger — but lacks the teleport (the IFV vehicle's locomotor takes over). |
| `Teleporter` | `yes` | TechnoType (verified 0x00843e60) | Hardcoded behavioral flag — enables the teleport-locomotor's destination-pick logic. Combined with the `Locomotor=` CLSID above this is the Chrono-warp gate. |
| `Trainable` | `no` | TechnoType (cheat sheet) | INI-comment: "like regular ivan, since can't gain experience". CIVAN never gains veterancy — the `VeteranAbilities` / `EliteAbilities` lists are effectively dead. Reason (per RA2 design): Ivan-style bombing instakills carriers, so combat XP is impossible to balance; both regular IVAN and CIVAN are locked at rookie rank. |

---

## 2. `artmd.ini` — `[CIVAN]` section and animation sequence

### `[CIVAN]` art block

```ini
[CIVAN] ; Chrono Ivan
Cameo=IVNCICON
Sequence=CIvanSequence
Crawls=yes
Remapable=yes
FireUp=6
PrimaryFireFLH=80,0,85
```

| Key | Value | Effect |
|-----|-------|--------|
| `Cameo` | `IVNCICON` | Build cameo (regular IVAN uses `IVANICON`). |
| (no `AltCameo`) | — | Unusual — most tech-tier infantry have a Yuri-skin alt cameo. CIVAN does not. |
| `Sequence` | `CIvanSequence` | Frame table — distinct from `IvanSequence` (the regular Crazy Ivan's). See below. |
| `Crawls` | `yes` | Has crawl/prone anims. |
| `Remapable` | `yes` | House-color remap applied. |
| `FireUp` | `6` | Frames into FireUp anim before the projectile spawns. |
| `PrimaryFireFLH` | `80,0,85` | Firing-pixel offset (X=80 forward, Y=0, Z=85 — chest-height bomb-throw). Identical to regular IVAN's FLH (since both use the same throw motion). |

### `[CIvanSequence]` referenced sequence

```ini
[CIvanSequence]
Ready=0,1,1
Guard=0,1,1
Walk=8,6,6
Idle1=56,15,0,S
Idle2=71,15,0,E
Prone=86,1,6
Crawl=86,6,6
Die1=134,15,0
Die2=149,15,0
FireUp=164,6,6
FireProne=164,6,6
Down=212,2,2
Up=228,2,2
Deploy=244,15,0 ; ### Bad/missing frames in Ivan
Deployed=257,1,0
Undeploy=257,1,0
;Deploy=164,6,0
;Deployed=169,1,0
;Undeploy=169,1,0
Cheer=259,8,0,E
Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
Panic=8,6,6
```

| Row | Notes |
|-----|-------|
| `Ready/Guard=0,1,1` | Stationary stand pose. |
| `Walk=8,6,6` | 6-frame walk × 6 facings. **Important**: because `Locomotor=Teleport`, this sequence is normally invisible — the unit teleport-warps cell-to-cell rather than animating walk. The walk frames may briefly show during entry/exit transitions or when the teleport cadence requires a step. |
| `Idle1=56,15,0,S` / `Idle2=71,15,0,E` | Two idle anims, south- and east-locked. |
| `Crawl/Prone=86,...` | Crawl/prone frames share start 86. |
| `Die1=134 / Die2=149` | Two death animations. |
| `FireUp=164,6,6` | 6-frame fire × 6 facings. **Note**: `FireProne=164,6,6` is identical to FireUp (both prone and standing throw use the same anim — no distinct prone-throw frames). |
| `Down=212 / Up=228` | Lay-down / get-up transitions. |
| `Deploy=244,15,0` (with comment "Bad/missing frames in Ivan") | Deploy animation — but the **unit's `;Deployer=yes` is commented out**, so these frames are dead data. The comment "Bad/missing frames in Ivan" is a hand-written engine note acknowledging incomplete art. |
| Commented alt-Deploy rows at 164,6,0 | Author tried an alternate set; unused. |
| `Cheer=259,8,0,E` | Victory cheer. |
| `Die3-5=0,1,1` | Stub entries (Die1/Die2 are the only real deaths). |
| `Panic=8,6,6` | Reuses walk frames as panic-flee. |

---

## 3. Weapon — `[IvanBomber]`

CIVAN uses the same weapon definition as regular IVAN (and as the IFV "ExplodeTurret"
gunner-mode). There is **no `IvanBomberE` elite variant** — see §9 for why this doesn't matter.

```ini
[IvanBomber]
Damage=400 ; Damage is used only for death explosion
ROF=50
Range=1.5 ; you can't change the target, but you can change yourself for CellRangefinding, so target could still be far side infantry
CellRangefinding=yes
FireOnce=yes ; Only fire once; don't stay in attack mission
Projectile=Invisible
Warhead=IvanBomb
FireInTransport=no;can't fire out of the BattleFortress
```

| Key | Value | Effect |
|-----|-------|--------|
| `Damage` | `400` | INI comment: "used only for death explosion". The 400 is the **bomb-death-event damage** (when the bomb-carrier dies before the timer expires, the bomb-induced area damage uses this); the timed-fuse path instead reads `[CombatDamage] IvanDamage=450` (a global). |
| `ROF` | `50` | 50-tick cooldown after a bomb is planted. CIVAN cannot plant a second bomb during this window. |
| `Range` | `1.5` | INI comment: "you can't change the target, but you can change yourself for CellRangefinding". 1.5-cell short range — CIVAN must walk (teleport) right up to the target. |
| `CellRangefinding` | `yes` | WeaponType-scoped (cheat sheet). The targeting code measures range from the unit's current cell to the target's cell, **not** from the unit's exact position; this lets the unit fire on far-side-infantry-in-the-cell from the edge. |
| `FireOnce` | `yes` | INI comment: "don't stay in attack mission". After one plant, CIVAN drops the attack mission and goes idle — the player must re-issue an attack for the next bomb. |
| `Projectile` | `Invisible` | See §3.1. |
| `Warhead` | `IvanBomb` | See §4 — this is what actually triggers the bomb-plant. |
| `FireInTransport` | `no` | INI comment: "can't fire out of the BattleFortress". WeaponType-scoped (cheat sheet). Prevents Ivan-bombing while a passenger of Battle Fortress (`FV`). |

### 3.1 `[Invisible]` projectile

```ini
[Invisible]
Inviso=yes
Image=none
```

Inviso/no-image — the bomb-plant has no visible projectile travel. The `BombClass` attach (the visible clock icon on the target) is what the player sees.

---

## 4. Warhead — `[IvanBomb]` / `[IvanWH]`

CIVAN's weapon uses **`IvanBomb`** as warhead. This is the *placement* warhead — it does
no damage but attaches a BombClass instance to the target. The eventual *explosion*
uses a different warhead (`IvanWH`) when the fuse expires.

### `[IvanBomb]` — placement (no damage; attaches BombClass)

```ini
[IvanBomb] ; Placing
IvanBomb=yes
```

| Key | Effect |
|-----|--------|
| `IvanBomb` | `yes` | WarheadType-scoped (verified — see §7). Sets bit at `WarheadType+0x157`. Per [BOMB_CLASS_GHIDRA_REPORT.md](../../BOMB_CLASS_GHIDRA_REPORT.md) §4.1, `WarheadTypeClass::Detonate` checks this bit and branches to `BombClass::Attach`, which creates a bomb structure on the target carrier. No INI `Verses=`, no damage, no AnimList — the warhead is purely a behavioral switch. |

### `[IvanWH]` — explosion (used when fuse expires)

```ini
[IvanWH] ;Explosion
Verses=100%,100%,100%,100%,100%,100%,100%,250%,20%,100%,100%
InfDeath=6;3
CellSpread=1.5
PercentAtMax=.25
AnimList=CRIVEXP
```

| Key | Effect |
|-----|--------|
| `Verses` | 100% against most armors; **250% against `steel`** (slot 8 — most non-concrete buildings); **20% against `concrete`** (slot 9). Designed to wreck buildings; under-effective against concrete-armored structures. |
| `InfDeath` | `6` (overrides earlier "3" — INI keeps both for clarity). Infantry caught in the blast use **blown-to-bits** death animation (per InfDeath table compiled across docs: 6=blown-to-bits, IvanWH/PsiPulse/SuperPsiPulse). |
| `CellSpread` | `1.5` | 1.5-cell area damage radius. |
| `PercentAtMax` | `.25` | Damage at the edge of the spread is 25% of centre. |
| `AnimList` | `CRIVEXP` | Crazy Ivan Explosion animation (verified entry in artmd at line 19412). |

### `[CRIVEXP]` impact animation

```ini
[CRIVEXP]  ; Crazy Ivan Explosion
Report=ExplosionCrazyIvan
Crater=yes
Normalized=yes
Translucent=yes
UseNormalLight=yes
```

- `Report=ExplosionCrazyIvan` — explosion sound.
- `Crater=yes` — leaves a permanent crater on terrain.
- `Normalized=yes` — frame timing FPS-normalized.
- `Translucent=yes` — alpha-blended sprite.
- `UseNormalLight=yes` — uses scene's ambient light instead of a fixed bright tint.

### Full bomb-plant lifecycle

Summarised here for completeness; full RE in [BOMB_CLASS_GHIDRA_REPORT.md](../../BOMB_CLASS_GHIDRA_REPORT.md):

1. CIVAN fires `IvanBomber` weapon at target → `InfantryClass::Fire_At` creates a bullet with `Warhead=IvanBomb`.
2. Bullet impacts → `WarheadTypeClass::Detonate` sees `IvanBomb=yes` → branches to `BombClass::Attach(target, attacker=CIVAN)`.
3. `BombClass::Attach` (see report §4.1) sets `EndFrame = CurrentFrame + Rules->IvanTimedDelay` (global from `[CombatDamage]`), plays `BombAttachSound` if defined, draws clock-icon overlay on target.
4. On every sim tick, `BombClass` checks `EndFrame`. When `CurrentFrame >= EndFrame`:
   - `Apply_area_damage(target.coords, Rules->IvanDamage=450, attacker=CIVAN, Rules->IvanWarhead=IvanWH)` is called.
   - If the target is a building on a bridge, the bridge under the building is destroyed too (the "Ivan-bomb-on-bridged-building destroys the bridge" mechanic).
5. If the carrier dies before fuse expiry (e.g., player shoots it), the bomb still detonates at the carrier's death coords with the same `IvanDamage`/`IvanWarhead` parameters — the "shoot the Ivan'd unit to spread the damage" tactic.
6. If a `BombDisarm=yes` warhead hits the bomb carrier, the bomb is removed without detonation. CCOMAND has `BombDisarm=yes` on its primary — it can defuse Ivan bombs by attacking the carrier.

---

## 5. Voices / sounds

```ini
[CrazyIvanSelect]
Sounds= $icrasea $icraseb $icrasec $icrased $icrasee $icrasef $icraseg
Control= random
Volume=85

[CrazyIvanMove]
Sounds= $icramoa $icramob $icramoc $icramod $icramof
Control= random
Volume=85

[CrazyIvanAttackCommand]
Sounds= $icraata $icraatb $icraatc $icraatd
Control= random
Volume=85

[CrazyIvanFear]
Sounds= $icraseg $icrasea
Control= random
Priority=Low
Volume=90

[CrazyIvanDie]
Sounds= $icradia $icradib
Control= random
Volume=85
```

```ini
[ChronoLegionTeleport]
Sounds=ichrmova
Control= interrupt
Limit=1
Range=20
Priority=high
```

```ini
[InfantrySquish]
Sounds=igensqua
FShift= -10 10
Volume=65
```

| Hook | Sound def | Trigger |
|------|-----------|---------|
| `VoiceSelect=CrazyIvanSelect` | 7 clips (sea..seg) | Click-select |
| `VoiceMove=CrazyIvanMove` | 5 clips (moa..mof, skipping moe) | Move order |
| `VoiceAttack=CrazyIvanAttackCommand` | 4 clips (ata..atd) | Attack order |
| `VoiceFeedback=CrazyIvanFear` | 2 clips (reuses seg, sea from select) | Under attack |
| `VoiceSpecialAttack=CrazyIvanAttackCommand` | reuses attack | No unique secondary voice |
| `DieSound=CrazyIvanDie` | 2 clips (dia, dib) | Death |
| `ChronoInSound=ChronoLegionTeleport` | `ichrmova`, `Limit=1`, `Range=20`, `Priority=high` | Warp into a cell |
| `ChronoOutSound=ChronoLegionTeleport` | (same def) | Warp out of a cell |
| `CrushSound=InfantrySquish` | `igensqua`, FShift ±10, Vol 65 | n/a (Crushable=no) |

The `Limit=1` on `ChronoLegionTeleport` means at most one concurrent warp-sound plays even if multiple chrono-units teleport simultaneously — prevents audio spam. `Range=20` limits hearing distance (cells); a warp 20+ cells from the camera is silent.

---

## 6. Prerequisites / owners / tech-steal gate

### Build-tree gate
- **`Prerequisite=BARRACKS`** — generic barracks token (per-house resolution).
- **`RequiresStolenSovietTech=yes`** — house-flag gate set by Spy infiltrating any Soviet tech building (NARADR, NATECH, NAWEAP, NAREFN, NACNST, etc., wherever `Spyable=yes`).
- **`AllowedToStartInMultiplayer=no`** — never preplaced; only accessible after infiltration.
- **`TechLevel=9`** — endgame slot.

### Owner / RequiredHouses
- **`Owner=`** — all 10 countries + `Alliance`. No `RequiredHouses=` → any country can build CIVAN once tech-stolen.
- The author-comment "anybody gets into a soviet tech center" makes the intent explicit.

### Comparison to the tech-steal triplet

| Unit | Tech gate flag | Stolen tech building | Theme | Cost | Soylent |
|------|----------------|----------------------|-------|------|---------|
| CCOMAND (Allied tech) | `RequiresStolenAlliedTech=yes` | Allied Battle Lab | Chrono SEAL | $1500 | $750 (50%) |
| **CIVAN (Soviet tech)** | **`RequiresStolenSovietTech=yes`** | **Soviet Battle Lab** | **Chrono Ivan** | **$1750** | **$500 (28%)** |
| PTROOP (Yuri tech) | `RequiresStolenThirdTech=yes` | Yuri Battle Lab | Psi-Corp Trooper | $1000 | $500 (50%) |

CIVAN is the **most expensive** of the three and has the **worst grinder refund ratio** — design treats it as a high-impact specialist worth gating.

> The Spy-infiltration → tech-flag mechanic is reverse-engineered in
> [SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md](../../SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md).

---

## 7. Hardcoded behavior (Ghidra-verified)

### 7.1 CIVAN-specific code in `gamemd.exe`

| Query (search_strings) | Result |
|------------------------|--------|
| `CIVAN` | 0 matches |
| `ChronoIvan` | 0 matches |
| `CrazyIvan` | 0 matches |
| Plain `Ivan` (exact-match) | 0 matches |
| `Ivan` (substring) | 6 matches — all of: `NoIvanBomb`, `IvanBomb`, `IvanIconFlickerRate`, `IvanTimedDelay`, `IvanDamage`, `IvanWarhead` |

⇒ **No CIVAN-specific function or hardcoded ID** in the binary. The unit-level `Ivan=yes` INI flag has **no string-literal match** in `gamemd.exe`. Possible explanations:

1. The flag is read into InfantryType via a fixed-offset reader that uses a different name string in the binary, or
2. The flag is parsed but the field is never tested (dormant).

**Either way, the live bomb-plant mechanic is driven by the warhead's `IvanBomb=yes`** (verified at 0x0081bd60 read by WarheadTypeClass__ReadINI @ 0x0075d807), **not by the unit's `Ivan=yes` flag**. The INI comment "needed to differentiate from Bomber, which is C4, and engineer" is therefore misleading by today's binary state — the actual differentiation between Crazy Ivan / SEAL-style C4 / Engineer happens on the weapon+warhead pairing, not the unit-class boolean.

### 7.2 Flag-scope verification

| Key | String at | Read by | Class scope |
|-----|-----------|---------|-------------|
| `RequiresStolenSovietTech` | 0x00843be0 | TechnoTypeClass__ReadINI @ 0x007144f7 | TechnoType |
| `Teleporter` | 0x00843e60 | TechnoTypeClass__ReadINI (cheat sheet) | TechnoType |
| `IvanBomb` (warhead key) | 0x0081bd60 | WarheadTypeClass__ReadINI @ 0x0075d807 (+ second runtime ref at 0x007e4d24) | WarheadType |
| `IFVMode` | 0x00843ae4 | TechnoTypeClass__ReadINI @ 0x00714787 | TechnoType |
| `AttackCursorOnFriendlies` | cheat sheet | TechnoTypeClass__ReadINI | TechnoType |
| `MoveToShroud` | cheat sheet | TechnoTypeClass__ReadINI | TechnoType |
| `Trainable` | cheat sheet | TechnoTypeClass__ReadINI | TechnoType |
| `IvanDamage` / `IvanWarhead` / `IvanTimedDelay` / `IvanIconFlickerRate` | 0x0083b110 / 0x0083b154 / 0x0083b100 / 0x0083b0cc | `RulesClass::ReadCombatDamage` (cheat sheet 0x0066B000-range) | Global `[CombatDamage]` |

### 7.3 Live behaviors driven by these flags

| Behavior | Driver | Notes |
|----------|--------|-------|
| Build-tree gate (must infiltrate Soviet lab) | `RequiresStolenSovietTech` checked in HouseClass build-availability path | Same path as CCOMAND/PTROOP, different flag bit. |
| Teleport locomotion | `Teleporter=yes` + `Locomotor={4A582747-...}` (TeleportLocomotionClass) | CIVAN warps cell-to-cell. See [TELEPORT_LOCOMOTION_DEEP_DIVE.md](../../TELEPORT_LOCOMOTION_DEEP_DIVE.md) and [TELEPORT_LOCOMOTION_IMPLEMENTATION_REFERENCE.md](../../TELEPORT_LOCOMOTION_IMPLEMENTATION_REFERENCE.md). |
| Teleport entry/exit sound | `ChronoInSound` / `ChronoOutSound` keys read by TechnoType — fires through `VocClass::Play` when locomotor reports warp-in/warp-out | Both set to same `ChronoLegionTeleport` sound. |
| Bomb plant on fire | Weapon `IvanBomber` (Warhead=`IvanBomb`, `IvanBomb=yes` flag at WarheadType+0x157) → `WarheadTypeClass::Detonate` → `BombClass::Attach` | Full mechanic in BOMB_CLASS_GHIDRA_REPORT. |
| Bomb fuse timer | `Rules->IvanTimedDelay` global from `[CombatDamage]` | Per-rules tunable; same delay as regular IVAN. |
| Bomb area damage on detonation | `Rules->IvanDamage=450` + `Rules->IvanWarhead=IvanWH` | Identical to regular IVAN's bomb. |
| Bomb-on-bridged-building collapses bridge | `BombClass` detonation path checks if target is building on a bridge | See BOMB_CLASS_GHIDRA_REPORT §4.4. |
| No combat experience | `Trainable=no` short-circuits the veterancy-XP accumulator on the unit | `VeteranAbilities` / `EliteAbilities` lists exist but never trigger. INI comment correctly notes this. |
| Cannot move-attack into shroud | `MoveToShroud=no` | Prevents teleport-into-shroud exploits. |
| Targeting cursor flips on friendlies | `AttackCursorOnFriendlies=yes` | Allows attaching bombs to allied units. |
| IFV explode-mode passenger | `IFVMode=7` → `ExplodeTurretWeapon=7` → IFV's `Weapon8=CRNuke` | Same as regular IVAN; the IFV locomotor takes over (no teleport while embarked). |

### 7.4 Behaviors NOT present in CIVAN

- **Veterancy** — `Trainable=no` disables gain. The `VeteranAbilities`/`EliteAbilities` lists are dead.
- **Deploy** — `;Deployer=yes` commented; deploy frames in CIvanSequence are dead data.
- **Elite weapon swap** — no `ElitePrimary=` key. CIVAN uses `IvanBomber` regardless of (non-existent) rank.
- **Self-heal at any rank** — `SELF_HEAL` is in EliteAbilities, but CIVAN never reaches elite.
- **Disguise / disguise-detect** — no `DetectDisguise` / `Disguise` flags.
- **Anti-aircraft** — `IvanBomber` has no AA.

---

## 8. TS-legacy filter

| INI line | TS-LEGACY? | Status in YR |
|----------|-----------|--------------|
| `TiberiumProof=yes` | YES (no tiberium in YR) | Dormant. |
| `ImmuneToVeins=yes` | YES (no veinholes in YR) | Dormant. |
| `Ivan=yes` (unit flag) | Possibly — no binary string match | See §7.1 — likely dormant; live bomb behavior comes from warhead. |
| `;Deployer / ;UndeployDelay / alt Locomotor / ;Bombable / ;Deploy art rows` | n/a (commented/dead) | Inactive. |

All other flags are live in YR.

---

## 9. Veterancy

`Trainable=no` ⇒ **CIVAN cannot gain experience**. The unit is permanently locked at
rookie rank for its entire lifetime. Consequently:

- The `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` list is **inert**.
- The `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` list is **inert**.
- There is no `ElitePrimary=` key — the weapon never changes.

This mirrors the regular Crazy Ivan (IVAN), which is also `Trainable=no` for the same
balance reason: a single bomb instakills most targets, so attaching XP gain would let
the unit become quickly broken.

> Veteran-crate pickups: a veteran-promotion crate gives a "veteran rank" status to a
> non-`Trainable` unit only if the engine's hardcoded crate handler doesn't gate on
> `Trainable=` (the cheat sheet implies it does gate). In standard play CIVAN/IVAN
> are not observed gaining rank from crates.

---

## 10. Cross-references

### Direct dependencies (must exist in `rulesmd.ini` / `artmd.ini`)
- `[IvanBomber]` — weapon (§3)
- `[Invisible]` — projectile (§3.1)
- `[IvanBomb]` — placement warhead (§4)
- `[IvanWH]` — explosion warhead (§4) — uses globals `IvanDamage` & `IvanWarhead`
- `[CRIVEXP]` (artmd) — explosion animation (§4)
- `[CIvanSequence]` (artmd) — frame table (§2)
- `[CrazyIvanSelect/Move/AttackCommand/Fear/Die] / [ChronoLegionTeleport] / [InfantrySquish]` (soundmd) — voices (§5)
- `[CombatDamage] IvanWarhead=IvanWH / IvanDamage=450 / IvanTimedDelay / IvanIconFlickerRate` (rulesmd globals — line 828+) — fuse and damage parameters
- `BARRACKS` token resolution — per-side build prereq resolver

### Conceptual companions
- **IVAN** (`soviet/IVAN.md`) — the base Crazy Ivan; shares weapon/warhead/voices, but walks instead of teleports, has `RequiredHouses=Russians` (Soviet-only build), and lower Cost ($600 vs $1750).
- **CLEG** (`allied/CLEG.md`) — Chrono Legionnaire; shares `Locomotor=TeleportLocomotionClass` and `ChronoLegionTeleport` sound. Different weapon (`NeutronRifle`) and warhead.
- **CCOMAND** (`allied/CCOMAND.md`) — Allied tech-steal sibling; teleporting C4-planter (note the spelling: SEAL with chrono + C4). Has `BombDisarm=yes` on its weapon → can defuse CIVAN's bombs.
- **PTROOP** (`yuri/PTROOP.md`) — Yuri tech-steal sibling; psi mind-control trooper.

### Deep-RE docs (cross-reference, do not re-derive)
- **[BOMB_CLASS_GHIDRA_REPORT.md](../../BOMB_CLASS_GHIDRA_REPORT.md)** — 941-line full reverse-engineering of the Crazy Ivan bomb mechanic. Field offsets, fuse timer, area-damage formula, bridge-collapse path, BombDisarm interaction. CIVAN inherits all of this verbatim.
- **[TELEPORT_LOCOMOTION_DEEP_DIVE.md](../../TELEPORT_LOCOMOTION_DEEP_DIVE.md)** — teleport-locomotor mechanics (destination pick, cooldown, sound triggers).
- **[TELEPORT_LOCOMOTION_IMPLEMENTATION_REFERENCE.md](../../TELEPORT_LOCOMOTION_IMPLEMENTATION_REFERENCE.md)** — implementation reference for teleport locomotor.
- **[SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md](../../SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md)** — Spy → tech-flag flip mechanic that unlocks CIVAN.

---

## 11. Coverage audit

| Section | Status |
|---------|--------|
| Every `[CIVAN]` rulesmd key explained | ✅ §1 |
| Every `[CIVAN]` artmd key explained | ✅ §2 |
| `Sequence=CIvanSequence` fully expanded | ✅ §2 |
| Primary weapon + projectile + both warheads + impact anim | ✅ §3–§4 |
| Bomb-plant lifecycle documented (cross-ref to BOMB_CLASS_GHIDRA_REPORT) | ✅ §4 |
| All voices + crush + chrono-warp sounds expanded with verbatim sound defs | ✅ §5 |
| Prereqs / owners / tech-steal gate analysed | ✅ §6 |
| Tech-steal triplet comparison table | ✅ §6 |
| Hardcoded behavior — Ghidra searches for CIVAN ID + all gating flags | ✅ §7 (six string searches; CIVAN-string returned 0; `Ivan=yes` unit-flag returned no string match — flagged in §7.1) |
| Veterancy treated correctly (Trainable=no → dead lists) | ✅ §9 |
| TS-legacy filter applied | ✅ §8 |
| Cross-refs to weapon/warhead/anim/voice sections + deep-RE reports | ✅ §10 |
| Doc placed in `soviet/` per theme-aligned convention | ✅ |

**Open follow-ups (none load-bearing):**
- **Verify whether `Ivan=yes` unit-flag has any live consumer.** No string match in binary; suspect dormant. Could decompile `InfantryType::ReadINI` (cheat sheet 0x00524xxx range) to confirm whether the field is parsed at all, and grep runtime code for `*(infantryType + 0xN)` reads at the offset where `Ivan` would land. Worth a dedicated audit if a future parity bug surfaces.
- **Confirm IFV interaction.** CIVAN with `IFVMode=7` boards an Allied IFV — the IFV's gunner becomes the "ExplodeTurret" mode firing `CRNuke` (Weapon8). Need to confirm whether the IFV's `CRNuke` is the same as `IvanBomber` or a parallel definition; brief mention in §1 / §7.3 should be confirmed by reading `[CRNuke]` from rulesmd in a future iteration.
- **Crate-rank interaction.** The veteran-crate handler may or may not gate on `Trainable=`. Document if this becomes load-bearing.
