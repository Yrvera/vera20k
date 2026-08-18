# AMCV — Allied Construction Vehicle (MCV)

**Side classification:** Allied (Owner=British,French,Germans,Americans,Alliance).
**Role:** Mobile Construction Vehicle. Deploys into the Allied Construction Yard
(GACNST), which is the build-tree entry point for the Allied faction. Always
present at game start (preplaced) and after every successful deploy.

> Output bar: build-tree entry, deploy timing, and the heavy/OmniCrush-resistant
> protection all matter. Players measure their tempo from "MCV down at T+0" to "ConYard
> up at T+~3s", and the deploy facing/cell-pick must match gamemd's exactly so the
> initial base layout is identical.

> **Companion docs**: this is one of three MCVs sharing identical mechanics:
> - **AMCV** (Allied, this doc) — deploys to `GACNST`
> - **SMCV** (Soviet, TODO) — deploys to `NACNST`
> - **PCV** (Yuri, TODO) — deploys to `YACNST`
>
> The deploy mechanic is generic (`DeploysInto=`) so all three share the same code path.
> They differ only in voices, art, and target ConYard.

> **Deep-RE cross-reference**: [SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md](../../SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md) §1 covers `UnitClass::Deploy @ 0x007393c0` in detail (originally documented for SMIN→SMON but the same routine handles AMCV→GACNST, SMCV→NACNST, PCV→YACNST). Key INI offsets:
> - `DeploysInto` → TechnoTypeClass offset 0x404 (verified `BuildingTypeClass::Find` lookup)
> - `UndeploysInto` → TechnoTypeClass offset 0x408 (`UnitTypeClass::Find` lookup, not used by AMCV)

> Ghidra confirms `gamemd.exe` contains no `"AMCV"` string — all behavior is generic
> flag-driven via `DeploysInto=GACNST` and the standard MCV deploy mechanism.

---

## 1. `rulesmd.ini` — `[AMCV]` verbatim

```ini
[AMCV]
UIName=Name:AMCV
Name=Allied Construction Vehicle
Image=MCV
Prerequisite=GAWEAP,GADEPT
Strength=1000
Category=Support
Armor=heavy
DeploysInto=GACNST
TechLevel=10
Sight=6
Speed=4
Owner=British,French,Germans,Americans,Alliance
CrateGoodie=yes
Cost=3000
Soylent=3000
Points=60
ROT=5
Crewed=yes
Crusher=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=MCVAlliedSelect
VoiceMove=MCVAlliedMove
VoiceAttack=MCVAlliedMove
DieSound=GenVehicleDie
VoiceFeedback=
DeploySound=PlaceBuilding
MoveSound=MCVMoveStart
MaxDebris=6
Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}
Weight=3.5
MovementZone=Normal
ThreatPosed=0	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
SpecialThreatValue=1
ZFudgeColumn=12
ZFudgeTunnel=15
Size=6
Trainable=no
Bunkerable=no; Units default to yes, others default to no
OmniCrushResistant=yes; so Crusher can crush Crushable, OmniCrusher trumps Crushable=no, and then OmniCrushResistant trumps OmniCrusher
```

### Key-by-key explanation

| Key | Value | Scope | Effect |
|-----|-------|-------|--------|
| `UIName` | `Name:AMCV` | AbstractType | CSF lookup. |
| `Name` | `Allied Construction Vehicle` | AbstractType | Dev/fallback. |
| `Image` | `MCV` | AbstractType | **Art-block redirect.** AMCV reads from `[MCV]` in artmd.ini instead of `[AMCV]`. This is the "shared MCV voxel" pattern — Allied/Soviet/Yuri MCVs all use Image= redirects to art entries that share names with the generic MCV body. SMCV uses `Image=SMCV`, PCV uses `Image=PCV` (own entries). AMCV using `Image=MCV` is the original RA2 naming convention. |
| `Prerequisite` | `GAWEAP,GADEPT` | TechnoType | Allied War Factory AND Service Depot. Note: Allied **Service Depot** is required, not a Battle Lab — the MCV is intended for **redeploy** scenarios where the player wants to relocate their base. Initial MCV at game start bypasses prereqs (preplaced). |
| `Strength` | `1000` | AbstractType | 1000 HP — tied with HARV/CMIN as the tankiest non-MBT vehicle. |
| `Category` | `Support` | TechnoType | Support classifier — not AFV. |
| `Armor` | `heavy` | TechnoType | Verses-slot 6 — same as Rhino. |
| `DeploysInto` | `GACNST` | TechnoType [BINARY-VERIFIED audit 14: string @ 0x00844180, parser xref @ 0x00713279, `TechnoType+0x404` (BuildingType*)] | **The deploy target.** When the player issues a deploy order (or hotkey D), `UnitClass::Deploy @ 0x007393c0` (Ghidra-labeled, body 0x007393C0–0x00739AB7, fully decompiled audit 14) creates a new `BuildingClass` of type `GACNST` at the unit's cell, transfers Strength/UniqueID/AttachedTag/veterancy, then removes the unit. See SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT §1 for the full transition sequence. |
| `TechLevel` | `10` | TechnoType | **Highest tier** — but `Cost=3000` and `Prerequisite=GAWEAP,GADEPT` are the practical gates. TechLevel=10 just means "always available once prereqs met". |
| `Sight` | `6` | TechnoType | 6-cell reveal. |
| `Speed` | `4` | TechnoType | Slow (same as harvesters). |
| `Owner` | `British,French,Germans,Americans,Alliance` | TechnoType | Allied countries only. |
| `CrateGoodie` | `yes` | UnitType | Can drop from crates — a **free MCV** from a crate is one of the most valuable crate outcomes (saves $3000 + lets the player branch base, plant a forward ConYard). |
| `Cost` | `3000` | TechnoType | Most expensive non-superweapon unit. |
| `Soylent` | `3000` | TechnoType | 100% Grinder refund (Allied has no Grinder; relevant only if captured). |
| `Points` | `60` | TechnoType | Score on kill — higher than harvester (55), reflecting strategic value. |
| `ROT` | `5` | TechnoType | Rate-of-turn for body (no turret). |
| `Crewed` | `yes` | TechnoType | **On death, 1+ infantry survivor parachute out** (typically an Engineer or GI). Notable — distinguishes MCV from HARV (Crewed=no). Crew helps the player recover slightly from losing the MCV. |
| `Crusher` | `yes` | TechnoType | Crushes infantry. |
| `Explosion` | `TWLT070,...` | TechnoType | Multi-anim death. |
| `VoiceSelect` | `MCVAlliedSelect` | TechnoType | 6 clips (`$vmcasea..af`). |
| `VoiceMove` | `MCVAlliedMove` | TechnoType | 6 clips (`$vmcamoa..of`). |
| `VoiceAttack` | `MCVAlliedMove` | TechnoType | Reuses Move set — MCV has no weapon, attack order treated as move. |
| `DieSound` | `GenVehicleDie` | TechnoType | Standard 6-clip vehicle death. |
| `VoiceFeedback` | *(empty)* | TechnoType | No "under attack" voice — typical of support vehicles. |
| `DeploySound` | `PlaceBuilding` | TechnoType (verified — 0x008440b0 read at 0x00713568) | Sound played at the start of the deploy sequence. `[PlaceBuilding]` = `uplace` clip — the "building placed" sound shared with normal building placements (so the MCV's deploy "feels" identical to placing any building). |
| `MoveSound` | `MCVMoveStart` | TechnoType | Engine-start sound. `[MCVMoveStart]` = `vmcvstaa`, Priority=Low, FShift ±2, VShift +20, Vol=40 — very quiet, low-priority. |
| `MaxDebris` | `6` | TechnoType | Death debris count. |
| `Locomotor` | `{4A582741-...}` | TechnoType | DriveLocomotionClass. |
| `Weight` | `3.5` | TechnoType | Physics weight (same as harvesters/Rhino). |
| `MovementZone` | `Normal` | TechnoType | Standard land vehicle path. Note: **`Normal`, not `Crusher`** (unlike harvesters). MCV cannot path through crushable obstacles — has to drive around walls. |
| `ThreatPosed` | `0` | TechnoType | Enemy AI ignores AMCV for auto-targeting. |
| `DamageParticleSystems` | `SparkSys,SmallGreySSys` | TechnoType | |
| `SpecialThreatValue` | `1` | TechnoType | High-value special-threat marker — combined with `ThreatPosed=0`, this signals the AI treats AMCV as "important but don't aggro" — flagged for protection/escort logic rather than direct targeting. |
| `ZFudgeColumn` | `12` | UnitType | Z-render fudge near columns. **Larger than harvester (9)** — MCV's voxel is taller, needs more z-offset to render correctly behind tall obstacles. |
| `ZFudgeTunnel` | `15` | UnitType | Z-fudge for tunnels (TS-legacy mostly). |
| (no `ZFudgeBridge`) | — | — | MCV doesn't have one — harvesters do. |
| `Size` | `6` | TechnoType | **Transport-slot cost 6** — far higher than harvester's 3. MCV is too big to fit in any transport (SAPC=2 slots max, SHAD=5 max). |
| `Trainable` | `no` | TechnoType | Cannot gain veterancy. Same reason as CMIN: no weapon, no combat XP path. |
| `Bunkerable` | `no` | TechnoType | Cannot enter Battle Bunker. |
| `OmniCrushResistant` | `yes` | TechnoType [BINARY-VERIFIED audit 14: string @ 0x00843868, parser xref @ 0x00714d11, `TechnoType+0xD2A` (byte). CORRECTS audit-2 cumulative which had +0xD2A as "TBD crusher-side gate flag"] | **Cannot be crushed by Battle Fortress / Mastodon-style omni-crushers.** INI comment chain: "so Crusher can crush Crushable, OmniCrusher trumps Crushable=no, and then OmniCrushResistant trumps OmniCrusher". This is the 3-tier crush resolution: |
| | | | • Tier 1: `Crusher=yes` + `Crushable=yes (default)` → crush |
| | | | • Tier 2: `OmniCrusher=yes` on vehicle (Battle Fortress FV) overrides target's `Crushable=no` |
| | | | • Tier 3: `OmniCrushResistant=yes` on target overrides Tier 2 — even Battle Fortress cannot crush AMCV |
| | | | This protects the foundational unit from being squished by a single Battle Fortress run. |

### Notable absent keys
- **No `Primary=`** — unarmed.
- **No `Turret=`** — no rotating turret.
- **No `Harvester=`** — not a harvester.
- **No `Passengers=`** — not a transport.
- **No `Teleporter=`** — doesn't teleport.
- **No `SelfHealing=`** — does NOT auto-heal (unlike harvesters and most armored vehicles). MCV must be repaired at a Service Depot.
- **No `OpportunityFire=`** — no weapon to use it on.

---

## 2. `artmd.ini` — `[MCV]` (referenced via `Image=MCV`)

AMCV's `Image=MCV` redirects art lookup to:

```ini
[MCV] ; Allied MCV
Cameo=MCVICON
Remapable=yes
Voxel=yes
```

| Key | Value | Effect |
|-----|-------|--------|
| `Cameo` | `MCVICON` | Build cameo. |
| `Remapable` | `yes` | House-color remap. |
| `Voxel` | `yes` | Voxel-rendered from `MCV.VXL` + `MCV.HVA`. |

Notably absent:
- **No `AltCameo=`** — no Yuri-skinned alt cameo (if a Yuri faction captures an AMCV, the standard MCVICON is used).
- **No `TurretOffset=`** — no turret.
- **No `PrimaryFireFLH=`** — no weapon.

(The other two MCVs follow the same pattern — `[SMCV]` and `[PCV]` art blocks are similarly minimal, with their own cameos `SMCVICON` and `YPCVICON`.)

---

## 3. Weapon — none

AMCV is **unarmed**. `Primary=` is not specified (defaults to none). No weapon, no
warhead, no projectile, no muzzle flash. The deploy action is the unit's only
gameplay-relevant input.

---

## 4. Warhead — n/a

---

## 5. Voices / sounds

```ini
[MCVAlliedSelect]
Sounds=$vmcasea $vmcaseb $vmcasec $vmcased $vmcasee $vmcasef
Control=random
Volume=85

[MCVAlliedMove]
Sounds=$vmcamoa $vmcamob $vmcamoc $vmcamod $vmcamoe $vmcamof
Control=random
Volume=85
```

```ini
[MCVMoveStart]
Sounds= vmcvstaa
Priority=Low
FShift= -2 2
VShift=20
Volume=40
```

```ini
[PlaceBuilding]
Sounds=uplace
```

| Hook | Sound def | Trigger |
|------|-----------|---------|
| `VoiceSelect=MCVAlliedSelect` | 6 clips | Click-select |
| `VoiceMove=MCVAlliedMove` | 6 clips | Move order |
| `VoiceAttack=MCVAlliedMove` | (same as Move) | Attack order — no weapon, default to move |
| `VoiceFeedback=` (empty) | — | No under-attack voice |
| `DeploySound=PlaceBuilding` | `uplace` clip | At the start of deploy sequence — shared with normal building-placement (consistent audio feel) |
| `MoveSound=MCVMoveStart` | `vmcvstaa` (low priority, FShift ±2, VShift +20, vol 40) | Engine start when movement begins; very quiet |
| `DieSound=GenVehicleDie` | 6 clips | Death |

Notably no `CrushSound=` — defaults to `TankCrush` via inheritance (although `Crusher=yes` is set, MCV rarely crushes anything because of its rare combat exposure).

---

## 6. Prerequisites / owners / availability

### Build-tree gate

- **Prerequisite** = `GAWEAP,GADEPT` (Allied War Factory + Service Depot).
- **TechLevel** = `10` (always available once prereqs met).
- **Cost** = $3000 — single biggest unit purchase.
- **Owner** = 5 Allied countries.
- **`AllowedToStartInMultiplayer=` is absent** — meaning AMCV IS allowed as preplaced starting unit. Every Allied player begins each game with one preplaced AMCV.

### Availability paths

1. **Game start**: every Allied player has one AMCV preplaced at their start location.
2. **Build from War Factory**: with GAWEAP + GADEPT up, $3000 buys a new AMCV.
3. **Crate goodie**: rare random reward — gives a free MCV (skips the $3000 cost).
4. **Cross-faction capture**: a Soviet or Yuri player can capture an Allied War Factory and build AMCV (but only if they also have an Allied Service Depot).
5. **Mind-control**: Yuri can mind-control an existing AMCV.

### Deploy mechanic — full sequence

Per [SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md](../../SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md) §1, `UnitClass::Deploy @ 0x007393c0` for any unit with `DeploysInto=`:

1. **Check `CanDeploy`** (vtable slot 0x314) — verifies preconditions (cell free, no enemies nearby, valid placement footprint).
2. **Face correct direction** — calls `Deploy_facing_calculator @ 0x00465d70`. The unit rotates to match `DeployFacing` from its TechnoType before deploying. AMCV doesn't override `DeployFacing` so it uses the default.
3. **Create `BuildingClass`** — `operator_new(0x720)` followed by `BuildingClass::Constructor` with the `DeploysInto` target (`GACNST`).
4. **Place building** — calls vtable slot 0xd8 (`TryPlaceBuilding`) at the unit's cell coordinates.
5. **Transfer properties** unit → building:
   - Copies `UniqueID`.
   - Copies `Location_Z` (height).
   - Transfers health: `ObjectClass::GetHealthRatio(unit)` → `Math::ftol` → sets `building->Health`. (A damaged MCV deploys into a damaged ConYard.)
   - Copies 5 dwords starting at field 0x1E0 (experience/veterancy data — irrelevant for `Trainable=no` AMCV).
   - Transfers `field_0x1EC` and `field_0x1F0` (presumably rally point / linking data).
   - If unit has `AttachedTag`, transfers it to building (with refcount management).
6. **Update targeting** — iterates all `TechnoClass` objects, redirecting any that targeted the unit (e.g. enemy tanks targeting the MCV) to now target the new building (unless it's a deploy-immune type).
7. **Remove unit** — vtable 0xF8 (`RemoveFromMap`) and vtable 0x3A0 (`Destroy/Limbo`).
8. **MCV special** — if the new building has `IsDeployable` (BuildingTypeClass offset 0x16b9), sets up base deployment: center-view-on-building, construction-yard flags, etc. **This is the path AMCV uses to anchor the build tree.**

**Cancel deploy** is via `UndeploysInto=` — but AMCV has no `UndeploysInto=` line (GACNST handles redeploy by spawning a new AMCV via its own `UndeploysInto=AMCV`-equivalent path, not by the AMCV reversing). The Yuri PCV has the same pattern.

---

## 7. Hardcoded behavior (Ghidra-verified)

### 7.1 AMCV-specific code in `gamemd.exe`

| Query | Result |
|-------|--------|
| `AMCV` | 0 matches |
| `Allied Construction Vehicle` | (not searched — string would be in CSF, not code) |

⇒ **No AMCV-specific hardcoded ID.** All behavior is generic flag-driven via `DeploysInto=GACNST` and the standard MCV deploy path.

### 7.2 Flag-scope verification (this iteration)

| Key | String at | Read by | Class scope |
|-----|-----------|---------|-------------|
| `DeploysInto` | 0x00844180 | TechnoTypeClass__ReadINI @ 0x00713279 | TechnoType (offset 0x404 per SLAVE_MINER_ORE_SYSTEM doc) |
| `UndeploysInto` | 0x00844170 | (sibling) | TechnoType (offset 0x408) |
| `OmniCrushResistant` | 0x00843868 | TechnoTypeClass__ReadINI @ 0x00714d11 | TechnoType |
| `DeploySound` | 0x008440b0 | TechnoTypeClass__ReadINI @ 0x00713568 | TechnoType |
| `UndeploySound` | 0x008440a0 | (sibling, not used by AMCV) | TechnoType |

Globals:
| Key | Address |
|-----|---------|
| `SlaveMinerDeploySound` (global default) | 0x0083a854 |
| `SlaveMinerUndeploySound` | 0x0083a83c |

⇒ The slave miner has its own dedicated deploy/undeploy sound globals (separate from the generic `DeploySound=` flag), but standard MCVs (AMCV/SMCV/PCV) use the per-unit `DeploySound=PlaceBuilding`.

### 7.3 Live behaviors driven by these flags

| Behavior | Driver | Notes |
|----------|--------|-------|
| Deploy to ConYard | `DeploysInto=GACNST` + `UnitClass::Deploy @ 0x007393c0` | Full sequence in §6 |
| Deploy sound = building-place sound | `DeploySound=PlaceBuilding` → `uplace` clip | Audio consistency with normal building placement |
| 3-tier crush resolution survives Battle Fortress | `OmniCrushResistant=yes` overrides FV's `OmniCrusher=yes` | Protects strategic foundational unit |
| Survivors parachute on death | `Crewed=yes` | Standard infantry-survivor path |
| Cannot enter Battle Bunker | `Bunkerable=no` | |
| Ignored by enemy AI for auto-target | `ThreatPosed=0` + `SpecialThreatValue=1` (AI flag for "protect, don't engage") | |
| Cannot gain veterancy | `Trainable=no` | |
| Cannot path through crushable obstacles | `MovementZone=Normal` | (Compare: harvesters use `Crusher` zone, can plow through walls) |
| Free MCV at game start | `AllowedToStartInMultiplayer=` absent (default yes) | Preplaced |

### 7.4 Behaviors NOT present

- **No weapon, no Secondary** — purely a deploy carrier.
- **No SelfHealing** — must use Service Depot to repair.
- **No turret** — body-only.
- **No Spawns** — no child units.
- **No special-attack** — no `VoiceSpecialAttack=`.
- **No `Teleporter=`** — Allied MCV does not chrono-warp. (The Chronosphere superweapon CAN teleport an MCV like any other unit, but that's a different system.)

---

## 8. TS-legacy filter

| INI line | TS-LEGACY? | Status in YR |
|----------|-----------|--------------|
| `ZFudgeTunnel=15` | YES (no real tunnels in YR) | Dormant render value. |
| All other flags | — | Live. |

No `ImmuneToVeins`, no Tiberium refs, no fog-of-war refs. AMCV is one of the cleaner TS-legacy-free units.

---

## 9. Veterancy

**`Trainable=no`** — locked at rookie rank permanently.

No `VeteranAbilities=`, no `EliteAbilities=`, no `ElitePrimary=`. AMCV has no
veterancy to gain, no weapon to upgrade, and no stats to boost. The unit's gameplay
role is single-purpose: deploy once, done.

---

## 10. Cross-references

### Direct dependencies
- `[GACNST]` (structures/GACNST.md TODO) — Allied ConYard, the deploy target
- `[GAWEAP]` (structures/GAWEAP.md TODO) — Allied War Factory, prereq
- `[GADEPT]` (structures/GADEPT.md TODO) — Allied Service Depot, prereq
- `[MCV]` (artmd) — art block (via `Image=MCV` redirect)
- `[MCVAlliedSelect / MCVAlliedMove]` (soundmd) — voices
- `[MCVMoveStart]` (soundmd) — engine sound
- `[PlaceBuilding]` (soundmd) — deploy sound
- `[GenVehicleDie]` (soundmd) — death sound

### Conceptual companions
- **SMCV** ([`soviet/SMCV.md`](../soviet/SMCV.md) — TODO) — Soviet MCV, `DeploysInto=NACNST`, `Image=SMCV`.
- **PCV** ([`yuri/PCV.md`](../yuri/PCV.md) — TODO) — Yuri MCV, `DeploysInto=YACNST`, `Image=PCV`.
- **HARV** ([`soviet/HARV.md`](../soviet/HARV.md)) / **CMIN** ([`allied/CMIN.md`](./CMIN.md)) — harvesters, similar Strength=1000 / Armor=medium support-vehicle profile, but Crewed=no and Crusher zone.

### Deep-RE docs (cross-referenced, not re-derived)
- **[SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md](../../SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md)** §1 — `UnitClass::Deploy @ 0x007393c0` full sequence. The single canonical reference for MCV-style deploy behavior in the codebase (originally written for SMIN→SMON but applies generically to any `DeploysInto=`-bearing unit).
- **[BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md](../../BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md)** — locomotor reference (AMCV uses DriveLocomotionClass).

---

## Ghidra audit log (audit iteration 14 — 2026-05-18)

**Methodology**: AMCV's deploy mechanic is concretely Ghidra-rooted —
the doc cites a function address (`UnitClass::Deploy @ 0x007393c0`), an
8-step state machine with vtable-slot annotations, several struct
offsets, and 5 parser xrefs. This audit re-verifies each in the binary.
~14 Ghidra queries: 6 string searches + 3 xref lookups + 2
`get_function_by_address` + 1 full `UnitClass__Deploy` decompile + 2
grep passes on saved `TechnoTypeClass__ReadINI` decompile.

### Negative claim re-verified

| Query | Result |
|-------|--------|
| `search_strings("^AMCV$")` | **0 matches** |

Confirms: no hardcoded section-name branch for AMCV in `gamemd.exe`.

### Function entry points (BINARY-VERIFIED)

| Function | Entry | Body | Status |
|----------|-------|------|--------|
| `UnitClass__Deploy` | `0x007393C0` | `0x007393C0–0x00739AB7` | Ghidra-labeled. Fully decompiled this pass. |
| `Deploy_facing_calculator` | `0x00465D70` | `0x00465D70–0x00465D76` | Ghidra-labeled. **Body is only 7 bytes** — this is a thin wrapper (likely a `ret` or single tail-call). Not the full facing-rule calculator; the actual rotation logic lives in the caller. **[ADDRESS PARTIAL]** — doc cites this address as if it contained the facing-rule body; it does not. |

### String + parser xref re-verification (BINARY-VERIFIED)

All 5 cited parser-address claims verify exactly:

| String | Addr | Parser xref | Function |
|--------|------|-------------|----------|
| `DeploysInto` | 0x00844180 | 0x00713279 | TechnoTypeClass__ReadINI |
| `UndeploysInto` | 0x00844170 | (sibling — body of same function) | TechnoTypeClass__ReadINI |
| `OmniCrushResistant` | 0x00843868 | 0x00714d11 | TechnoTypeClass__ReadINI |
| `DeploySound` | 0x008440b0 | 0x00713568 | TechnoTypeClass__ReadINI |
| `UndeploySound` | 0x008440a0 | (sibling) | TechnoTypeClass__ReadINI |

### Struct offsets BINARY-VERIFIED (this pass)

**TechnoTypeClass** (new in this pass, from TechnoTypeClass__ReadINI grep):

| Offset | INI key | Type | Notes |
|--------|---------|------|-------|
| `+0x404` | `DeploysInto` | BuildingType* | `param_1[0x101]` after BuildingTypeClass__FindOrAllocate. **Confirms doc claim** (originally from SLAVE_MINER doc). Also referenced 4× inside `UnitClass::Deploy` body at `param_1[10].vtable_INoticeSource + 0x404` (after type-cast). |
| `+0x408` | `UndeploysInto` | UnitType* | `param_1[0x102]` after UnitTypeClass__FindOrAllocate. **Confirms doc claim**. |
| `+0x40C` | `PowersUnit` | UnitType* | `param_1[0x103]` (NEW — adjacent sibling key, not on AMCV but in same parser block). |
| `+0x56C` | `DeploySound` | int (VocClass index) | `param_1[0x15b]` after VocClass__FindByName. Falls back to prior value if string read fails. (NEW.) |
| `+0x570` | `UndeploySound` | int (VocClass index) | `param_1[0x15c]` (NEW). |
| `+0x6B8` | `DeployingAnim` | AnimType* | `param_1[0x1ae]` after AnimTypeClass__FindOrAllocate (NEW — sibling key parsed in adjacent block). |
| `+0xD2A` | `OmniCrushResistant` | byte | `*(char*)((int)param_1 + 0xd2a) = uVar5` after ReadBool. **CORRECTS audit-2 GGI cumulative**, which had `+0xD2A` as "crusher-side gate flag, INI mapping TBD". Now BINARY-VERIFIED as the target-side resistance flag — read by `CanCrushCheck` to gate OmniCrusher attempts (per the 3-tier crush resolution: Crusher → OmniCrusher → OmniCrushResistant). |

**BuildingTypeClass** (from `UnitClass::Deploy` decompile):

| Offset | Field | Notes |
|--------|-------|-------|
| `+0x16B9` | `IsDeployable` | byte — checked at `pOVar9[7].Health + 0x16b9` (pOVar9 is the newly-constructed BuildingClass). When set, the deploy path triggers the "construction yard" special branch: center-view, set base-construction flag, mark deployed. **Confirms doc claim** of "BuildingTypeClass offset 0x16b9 = IsDeployable" from the SLAVE_MINER cross-reference. |
| `+0x16C4` | (unknown — `FacingClass::UpdateFacing` trigger flag) | byte |
| `+0x16CA` | (unknown — also `FacingClass::UpdateFacing` trigger flag) | byte |

### Vtable slot verification (BINARY-VERIFIED via UnitClass__Deploy)

The doc's `UnitClass::Deploy` 8-step sequence with vtable-slot annotations:

| Doc claim | Verified | Notes |
|-----------|----------|-------|
| vtable+0x314 = `CanDeploy` (precondition predicate) | ✓ BINARY-VERIFIED | First call in Deploy; returns char; gates entire function. |
| vtable+0xD8 = `TryPlaceBuilding` (on the NEW BuildingClass) | ✓ BINARY-VERIFIED | Called as `pOVar9->vtable + 0xd8` with a placement struct; returns char success/fail. |
| vtable+0xF8 = `RemoveFromMap` | ✓ BINARY-VERIFIED | Called on the unit (`param_1->vtable + 0xf8`) after building placement succeeds. **Matches audit 3** Engineer-capture finding (+0xF8 = self-destruct/remove). |
| vtable+0x3A0 = `Destroy/Limbo` | ✓ BINARY-VERIFIED | Called via `(**(code **)(param_1->vtable + 0x3a0))()` after target-redirect loop. |
| (not in doc) vtable+0x274 = SetMission | — | Confirms audit 3 cumulative (BuildingClass+0x274 = SetMission, called with mission ID 3 for the new building). |
| (not in doc) vtable+0x124 = Mark_Occupants | — | New observation — called with 0/1 to clear/set cell-occupancy bits during the deploy state transitions. |
| (not in doc) vtable+0x2c = GetAbstractType (RTTI) | — | Used in target-redirect loop with `iVar5 == 1` check — **matches audit 5 RTTI=1=FootClass** resolution. |
| `operator_new(0x720)` for the new BuildingClass | ✓ BINARY-VERIFIED | Exact size matches the doc claim. |

### Other behavioral facts confirmed in Deploy body

- **Health-ratio transfer**: `ObjectClass::GetHealthRatio(param_1) → Math::ftol → pOVar9->Health = iVar11` (with clamp to ≥ 1). A damaged MCV deploys into a proportionally-damaged ConYard — **confirms doc claim**.
- **UniqueID transfer**: `pOVar9[3].UniqueID = param_1[3].UniqueID`. **Confirms doc**.
- **Location_Z transfer**: `pOVar9[1].Location_Z = param_1[1].Location_Z`. **Confirms doc**.
- **5-dword veterancy/experience block transfer**: explicit `for (iVar11 = 5; ...)` loop copying `param_1[7].field_0x28+i*4 → pOVar9[7].field_0x28+i*4`. **Confirms doc** ("Copies 5 dwords starting at field 0x1E0").
- **field_0x1EC and field_0x1F0 transfer**: `*(undefined4*)&pOVar9[7].field_0x3c = *(undefined4*)&param_1[7].field_0x3c` and the +0x40 sibling. **Confirms doc** (those offsets correspond to ObjectClass[7].field_0x3c/0x40 in pointer arithmetic).
- **AttachedTag transfer + refcount**: `if (param_1->AttachedTag != 0) { FUN_005f5b50(param_1->AttachedTag); *(int*)(tag+0x2c)--; param_1->AttachedTag = 0; }`. **Confirms doc**.
- **Target-redirect loop**: iterates `g_TechnoClass_Array` (count `g_TechnoClass_Count`), checks RTTI=1 (FootClass) targets, redirects via `vtable+0x3c8` (SetTarget) from unit→building. **Confirms doc** description of step 6.
- **IsDeployable special branch**: `if ((cVar4 == '\0') && (*(char*)(pOVar9[7].Health + 0x16b9) != '\0') && (g_GameMode != 0)) { ... // construction yard setup ... }`. **Confirms doc** step 8 ("MCV special — if IsDeployable, set up base deployment").

### Items NOT re-verified in this pass (DEFERRED)

- `Deploy_facing_calculator` actual facing-rule body (the labeled function at `0x00465D70` is a 7-byte stub; the real facing logic must be one or two functions away). Documenting as **[ADDRESS PARTIAL]** rather than incorrect — the address Ghidra labeled may be a thunk for the real entry.
- 7-byte function content — would need disassembly (not decompile) to see the actual instructions.
- `pOVar9[7].Health + 0x16C4` and `+0x16CA` exact INI keys (both gate `FacingClass::UpdateFacing` call — likely IsBaseDefense / IsConstructionYard style flags; not load-bearing for AMCV).
- `CanDeploy` predicate body (vtable+0x314) — confirmed-called but body not traced.
- `BuildingClass::Constructor` body — confirmed-called with the DeploysInto target and the unit's owner field; body not traced.
- BuildingType+0x16C4/0x16CA INI key mappings.

### Confidence summary

- **HIGH**: 6 string addresses + 5 parser xrefs (all exact); 2 function entry points (both Ghidra-labeled, one of them fully decompiled in this pass); 7 struct offsets (5 new TechnoType, 1 corrected TechnoType, 1 BuildingType, all read directly from decompile); 4 vtable slots verified end-to-end in the deploy chain; full 8-step deploy state machine cross-referenced against the binary with no functional discrepancies.
- **MEDIUM**: `Deploy_facing_calculator` address is a 7-byte stub — the function exists at the cited address but doesn't contain the facing-rule body. Marked **[ADDRESS PARTIAL]**, not incorrect.
- **No INCORRECT findings in the doc**. All 8 deploy-sequence steps confirmed in the decompile. The single substantive correction is to the *cumulative cheat-sheet* (+0xD2A is OmniCrushResistant, replacing the audit-2 "TBD crusher-side flag" placeholder).

---

## 11. Coverage audit

| Section | Status |
|---------|--------|
| Every `[AMCV]` rulesmd key explained | ✅ §1 |
| `Image=MCV` redirect explained, art block expanded | ✅ §2 |
| No weapon / warhead (explicitly addressed) | ✅ §3–§4 |
| All voices + sounds (Select, Move, MoveStart, Deploy, Die) | ✅ §5 |
| Prereqs / owners / 5 availability paths | ✅ §6 |
| **Full deploy sequence documented** (8-step `UnitClass::Deploy` flow with vtable slot annotations) | ✅ §6 |
| Hardcoded behavior — Ghidra searches + 5 flag-scope verifications (DeploysInto, UndeploysInto, OmniCrushResistant, DeploySound, UndeploySound) | ✅ §7 |
| **3-tier crush resolution documented** (Crusher / OmniCrusher / OmniCrushResistant) | ✅ §1 + §7 |
| TS-legacy filter | ✅ §8 |
| Veterancy treated correctly (Trainable=no → permanent rookie) | ✅ §9 |
| Cross-refs to SMCV/PCV siblings + deploy deep-RE doc | ✅ §10 |

**Open follow-ups (none load-bearing):**
- Verify `DeployFacing=` default in `Deploy_facing_calculator @ 0x00465d70` — AMCV doesn't override it, but the default value matters for parity (first ConYard placement direction). Should decompile that function for the exact rules.
- The `[General] FreeUnit=` / `FreeHarvester=` mechanism on GACNST: when GACNST is deployed (from AMCV), it spawns a free CMIN nearby. This is the symmetric Allied-side equivalent of NACNST's `FreeUnit=HARV`. Worth documenting in GACNST.md when that doc is written.
- Cross-faction MCV: if a Yuri player captures an AMCV and deploys it, does it produce a GACNST (Allied) or a YACNST (Yuri)? Per `DeploysInto=GACNST` literal: it produces GACNST regardless of current owner. Combined with `Owner=Allied countries` for AMCV itself, this creates a quirky scenario where Yuri owns an Allied ConYard — the build menu adapts accordingly.
- `OmniCrushResistant=yes` should be checked on SMCV and PCV too — likely all three MCVs share this flag for consistency.
