# Navy SEAL / Tanya C4 Demolition — Ghidra Research Report

**Topic:** The walk-up-and-detonate building-demolition mechanic used by SEAL,
Tanya, Yuri/Psi-Corp Trooper, and (with a caveat) Chrono Commando. *Distinct
from* Crazy Ivan's timed BombClass — see `BOMB_CLASS_GHIDRA_REPORT.md`.

**Primary addresses:**
- `0x005196a0` — `InfantryClass::Mission_Enter` (the detonation site)
- `0x0051f3e0` — `InfantryClass::Mission_Attack` (the dispatcher to Enter/Capture)
- `0x004d4b20` — `FootClass::Mission_Capture` (AI walk-up wrapper)
- `0x00524400` (entry `0x005240a0`) — `InfantryTypeClass::ReadINI` (flag parser)
- `0x00460050` — `BuildingTypeClass::ReadINI_Water` (CanC4 parser)
- `0x0066bbd1` — `RulesClass::ReadCombatDamage` (C4Warhead/C4Delay parser)
- `0x0051d6f0` — `InfantryClass::Do_Action` (DoType setter)
- `0x00520ae0` — `InfantryClass::DoType_Sequencer` (per-frame anim tick)
- `0x0051df70` — `InfantryClass::Fire_At_Override` (NOT a self-destruct path)
- `0x005206b0` — `InfantryClass::Fire_At_Target` (firing-frame matcher)
- `0x0051e3b0` — `InfantryClass::What_Action_OnObject` (cursor picker)
- `0x00489280` — `Apply_area_damage` (the detonation routine)

**Confidence:** HIGH on all offset reconciliations; HIGH on the dispatch graph;
HIGH on the cursor path; MEDIUM on the precise frame-counter mechanics of the
plant timer (the C4Delay double at `Rules+0x1750` is parsed but its consumer
was not traced to a single read site).

**Active in YR:** Yes for the player C4 path. The AI Sabotage branch is also
active. No SpecialFlags gate observed.

---

## 0. Three Offset Conflicts — Resolved

The investigation plan flagged three pre-existing offset conflicts in the
research archive. **All three are now settled by direct binary reads:**

| Conflict | Resolution | Evidence |
|----------|------------|----------|
| `InfantryType` C4 flag — `+0xEBE` vs `+0xEC2` vs `+0xEC8`? | **`+0xEC2` is `C4=`.** `+0xEBE` is `Infiltrate=` (auto-derived). `+0xEC8` is `Deployer=`. | Disassembly of `0x005240a0`: `PUSH 0x825978; CALL ReadBool; MOV byte ptr [ESI + 0xec2], AL`. Memory at `0x825978` reads `"C4\0"`. |
| `BuildingType` `CanC4` — `+0x1577` vs `+0x16A9`? | **`+0x1577` is `CanC4=`.** `+0x16A9` is `UnitRepair=` (a totally different flag). | Decompile of `BuildingTypeClass::ReadINI_Water`: `CCINIClass__ReadBool(.., s_CanC4_0081adfc, ..); *(param_1 + 0x1577) = ..`. Memory at `0x81adfc` reads `"CanC4\0"`. |
| `Rules` `C4Warhead` — `+0xFA8` vs `+0xFAC`? | **`+0xFA8` is `C4Warhead=`.** `+0xFAC` is `CrushWarhead=`. | Decompile of `RulesClass::ReadCombatDamage`: ReadString for `s_C4Warhead_0083b1d4` → stores at `param_1+0xfa8`. Next read is `s_CrushWarhead_0083b1c4` → `+0xfac`. Mission_Enter detonation also reads `g_RulesClass_Instance + 0xfa8`. |

The `WARHEAD_DETONATE_GHIDRA_REPORT.md` claim of `+0xfac` was a typo /
mislabeling — that field is CrushWarhead, not C4Warhead. The
`BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md` claim of `+0x16A9` for CanC4
was wrong — that's UnitRepair (Service Depot flag).

---

## 1. End-to-End Pipeline (Player Path)

```
Player right-clicks an enemy structure with SEAL/Tanya selected
   │
   ▼
ActionClass / EventClass dispatch (player-input plumbing)
   │
   ▼
SEAL.TarCom = building, SEAL.Mission = ATTACK (2)
   │
   ▼  (every tick)
InfantryClass::Mission_Attack @ 0x0051f3e0
   │
   ├─ Test 1: Type[+0xec2 /* C4 */] != 0 OR HasWeaponAbility(0xe)
   │  ├─ TarCom is Building, BldgType[+0x1577 /* CanC4 */] != 0,
   │  │  BldgType[+0x1701 /* InvisibleInGame */] == 0
   │  │   → vtable[+0x480](TarCom, 1)        // Set_Target
   │  │   → vtable[+0x1e8](0x11, 0)          // SetMission(Enter = 17)
   │  │   → return 1
   │  └─ Else: fall through
   │
   ▼
InfantryClass::Mission_Enter @ 0x005196a0  (every tick while mission==0x11)
   │
   ├─ mission == 0x11 AND Type[+0xec2 /* C4 */] != 0
   │  │
   │  ├─ Building at SEAL's cell == NavCom target?
   │  │  ├─ Yes → record plant state on building:
   │  │  │  • Building+0x6df = 1            // "being C4'd" marker
   │  │  │  • Building+0x150 = SEAL ptr      // attacker
   │  │  │  • Building+0x14a = g_CurrentFrameCounter   // start frame
   │  │  │  • Building+0x14b/14c = saved coords
   │  │  │  • Building.vtable[+0x148](math_result)    // pre-detonation hook
   │  │  │  ...then FootClass::Stop_Moving(); SEAL.vtable[+0x45c](0); queue approach
   │  │  │     and RETURN (no detonation this tick)
   │  │  └─ No → walk toward target
   │  │
   │  └─ Else (SEAL's cell == NavCom dest cell, i.e. arrival):
   │     ┌──── DETONATION BLOCK ────┐
   │     • Apply_area_damage(SEAL, Rules[+0xfa8 /* C4Warhead */], 1, 0)
   │     • SEAL.vtable[+0x500]()                  // post-damage hook
   │     • Random scatter dir = (RateTimer >> 12 + 1) >> 1 & 7
   │     • SEAL.vtable[+0x174](&offset_cell, 1, 1) // walk away in scatter dir
   │     • SEAL.vtable[+0x1e8](2, 0)              // SetMission(Move = 2)
   │     • If NavCom == NULL OR cell.field_0xec == 2: vtable[+0x124](1)  // unmark/cleanup
   │     • Apply_area_damage(0, C4Warhead, 1, 0)  // chain hit (debris/overlay)
   │     • Apply_area_damage(0, C4Warhead, 1, 0)  // chain hit (debris/overlay)
   │     └────────────────────────────┘
```

**Notes on the detonation block:**

- The `Apply_area_damage` call iterates objects on/around the target cell and
  invokes `vtable[+0x16c]` (Take_Damage) on each. With `C4Warhead == Super`
  (Verses=100% across all armors) and the building's full HP, this kills the
  building outright. **InfDeath=2** on `[Super]` means survivors are gibbed.
- The 2nd and 3rd `Apply_area_damage` calls with `param_1 == 0` are NOT
  duplicate damage — they are how the routine handles **destructible overlay
  on the same cell** (sandbags/wood crates/barrels). `Apply_area_damage`
  itself recurses into a 4th-level `Apply_area_damage(0, C4Warhead, 1, 0)` if
  it destroys an overlay whose `OverlayType[+0x2b0] != 0`. This is the barrel
  chain-reaction code.
- After the explosion, SEAL transitions to mission state **2** (typically
  Move/Idle) and walks one cell in a random direction (scatter). The scatter
  direction comes from RateTimer bits 13-15 — deterministic given the global
  frame counter, so multiplayer-safe.
- **No self-destruct on the SEAL.** There is no `vtable[+0x1e8](self_destruct, ...)`
  call in this path. SEAL/Tanya survive C4 plant. Confirmed against gamemd's
  observable behavior.

---

## 2. AI Path (Sabotage = Mission 8 internally)

```
AI's TaskForce assigns a SEAL/Tanya/Engineer to attack a building
   │
   ▼
InfantryClass::Mission_Attack @ 0x0051f3e0
   │
   ├─ Skip Test 1 (player check fails)
   ├─ Test 2: !IsPlayerControl AND TarCom is Building
   │  ├─ Type[+0xebe /* Infiltrate */] != 0
   │  │   → SetMission(8 = Capture)  via vtable[+0x1f0](8)
   │  │   → return 1
   │  └─ Type[+0xeb4 /* Occupier */] OR Type[+0xeb5 /* Assaulter */],
   │     AND BuildingClass::CanDock(this)
   │     → SetMission(8) likewise
   │
   ▼
FootClass::Mission_Capture @ 0x004d4b20  (AI walk-up wrapper)
   │
   ├─ Re-asserts TarCom is set if missing
   ├─ Walks toward target (vtable[+0x174])
   ├─ If completely no target after the call, fallback to Guard (0xf)
   └─ Returns mission timer + Random(0..2)  // small per-tick jitter
   │
   ▼  (on arrival, when SEAL's cell is the target's cell)
InfantryClass::Mission_Enter @ 0x005196a0  with mission state 8
   │
   ├─ mission ∈ {8, 0xb, 0x19} branch:
   │  ├─ If !Type[+0xec3 /* Engineer */] AND !Type[+0xec4 /* Agent */]:
   │  │   → walk-toward-target setup, return  (this is where C4-capable AI lands)
   │  ├─ If !Type[+0xec3] AND Type[+0xec4]:
   │  │   → BuildingClass::OnSpyInfiltrate(target)   // Spy/Agent path
   │  └─ If Type[+0xec3] (Engineer):
   │     ├─ If target.Type[+0x16b6 /* BridgeRepairHut */]: bridge destruction
   │     └─ Else: Engineer capture / damage path (uses C4Warhead for the damage path
   │              when health > Rules[+0x1708] threshold via vtable[+0x16c])
```

**Key insight:** In the AI path, **mission state 8 (Capture/Sabotage)** is
the dispatcher slot, but `InfantryClass::Mission_Enter` is the actual handler
function — it switches internally on mission state. C4-capable AI infantry
fall through the `!Engineer && !Agent` branch and approach the building, then
the system transitions to mission state 0x11 (Enter) when the unit arrives,
at which point the same Mission_Enter function executes the C4 detonation
block via the mission==0x11 branch documented in §1.

The plan's claim that "Mission_Capture handles BOTH Capture (8) AND Sabotage
(17)" was almost right but slightly off: the dispatcher slot for both is
served by Mission_Capture (walk-up phase), but the actual sabotage logic
lives in Mission_Enter. **Mission state 17 (= 0x11) IS the Enter mission**;
that's the same path used for Engineer capture, Spy infiltrate, garrison
entry, and SEAL C4 plant — the function dispatches internally based on
infantry-Type flags.

---

## 3. InfantryTypeClass Flag Map (verified from disassembly at `0x005240a0`)

The plan asked for the exact `+0xEBE..+0xEC8` band. Read directly from the
PUSH-then-ReadBool sequence in the disassembly. **Every entry verified by
reading the literal INI key string at the pushed address:**

| Offset | INI Key | String Address | Type | Notes |
|--------|---------|----------------|------|-------|
| `+0xeac` | `Cyborg` | `0x825a0c` | bool | TS-legacy (sets `+0xc8f=1` if true; legacy "is cyborg" predicate) |
| `+0xead` | `NotHuman` | `0x825a00` | bool | TS-legacy aesthetic flag |
| `+0xeae` | `Ivan` | `0x8259b4` | bool | Crazy Ivan flag — gates IvanBomb plant logic. **NOT** the C4 flag. |
| `+0xeb0` | `DetectionDistance` | `0x825988` | int | TS-legacy (sub detection?). Not C4-related. |
| `+0xeb4` | `Occupier` | `0x8259a8` | bool | Garrison occupant flag |
| `+0xeb5` | `Assaulter` | `0x82599c` | bool | Grants weapon-ability `0xe`; gates UC-building clearing |
| `+0xeb8` | `HarvestRate` | `0x82597c` | int | TS-legacy harvester field on infantry |
| `+0xebc` | `Fearless` | `0x8259d4` | bool | Won't run from combat |
| `+0xebd` | `Crawls` | `0x8258f4` | bool | Required for `Do_Action(5)` — controls prone-mode (`InfantryClass+0x6db`) |
| `+0xebe` | `Infiltrate` | `0x8259bc` | bool | **Derived: auto-set to 1 if any of `+0xec2`/`+0xec3`/`+0xec4` is set.** Gates AI infiltrate-class behavior. |
| `+0xebf` | **`Fraidycat`** | `0x8259c8` | bool | **Cowardice flag — NOT "Suicide". The plan's hypothesis was wrong.** Used in `Fire_At_Override` for fraidycat-flee timer. |
| `+0xec0` | `TiberiumProof` | `0x82595c` | bool | Immune to tiberium damage |
| `+0xec1` | `Civilian` | `0x818164` | bool | Civilian unit (no kill-credit) |
| `+0xec2` | **`C4`** | `0x825978` | bool | **The C4 flag.** Gates the SEAL/Tanya plant path. |
| `+0xec3` | `Engineer` | `0x82596c` | bool | Engineer capture/repair path |
| `+0xec4` | `Agent` | `0x825954` | bool | Spy class |
| `+0xec5` | `Thief` | `0x82594c` | bool | TS-legacy money-thief role |
| `+0xec6` | `VehicleThief` | `0x82593c` | bool | Hijacker (Yuri Hijacker) |
| `+0xec7` | `Doggie` | `0x825934` | bool | Dog units (auto-attack infantry) |
| `+0xec8` | `Deployer` | `0x825928` | bool | Can deploy (e.g., GI, Tank Bunker) |
| `+0xec9` | `DeployedCrushable` | `0x825914` | bool | When deployed, becomes crushable |
| `+0xeca` | `UseOwnName` | `0x825908` | bool | Use unit's own name in EVA |
| `+0xecb` | `JumpJetTurn` | `0x8258fc` | bool | TS-legacy jumpjet steering |

**Critical "auto-derived" aggregation in the parser** (lines `0x52466e..0x52469a`):

```
if (Type[+0xec2 /* C4 */]      != 0) Type[+0xebe /* Infiltrate */] = 1;
if (Type[+0xec3 /* Engineer */] != 0) Type[+0xebe /* Infiltrate */] = 1;
if (Type[+0xec4 /* Agent */]    != 0) Type[+0xebe /* Infiltrate */] = 1;
```

This means **any C4/Engineer/Agent unit is automatically marked as Infiltrate=yes
even if the INI doesn't say so**. This is what gates the AI's "send this unit
to a building via Mission(8)" branch in Mission_Attack. Per `rulesmd.ini`:

- `[GHOST]` (SEAL): `C4=yes` → `+0xec2=1` → `+0xebe=1` (auto)
- `[TANY]` (Tanya): `C4=yes` → `+0xec2=1` → `+0xebe=1` (auto)
- `[PTROOP]` (Psi-Corp Trooper): `C4=yes` → `+0xec2=1` → `+0xebe=1` (auto)
  - Note: PTROOP has `Primary=MindControl` and **no Secondary** weapon. It
    relies entirely on the C4 mission path; force-firing through a weapon is
    not an option.
- `[CCOMAND]` (Chrono Commando): `;C4=yes` (commented out!) → `+0xec2=0`. Uses
  `Secondary=FakeC4` for a normal-weapon-fire path with the FakeC4 warhead.
  **Chrono Commando does NOT use the C4 plant Mission_Enter path.**

---

## 4. BuildingTypeClass Flag Map (relevant subset)

Verified by direct decompile of `BuildingTypeClass::ReadINI_Water` at
`0x00460050`:

| Offset | INI Key | Type | Default | Used For |
|--------|---------|------|---------|----------|
| `+0x1572` | `Capturable` | bool | no | Engineer capture eligibility |
| `+0x1576` | `Spyable` | bool | no | Spy infiltrate eligibility |
| `+0x1577` | **`CanC4`** | **bool** | **yes** | **C4-target gate (the flag)** |
| `+0x157b` | `CanBeOccupied` | bool | no | Garrison eligibility |
| `+0x157c` | `CanOccupyFire` | bool | no | Garrisoned units can fire |
| `+0x16ad` | `Grinding` | bool | no | Grinder building |
| `+0x16ae` | `UnitAbsorb` | bool | no | Unit-absorb (Genetic Mutator-like) |
| `+0x16af` | `InfantryAbsorb` | bool | no | Infantry-absorb |
| `+0x16a9` | `UnitRepair` | bool | no | Service Depot — **NOT CanC4** |
| `+0x16b6` | `BridgeRepairHut` | bool | no | Engineer bridge-repair logic |
| `+0x16c1` | `Hospital` | bool | no | Garrison heal cursor |
| `+0x16c2` | `Armory` | bool | no | Veterancy upgrade cursor |
| `+0x1701` | **`InvisibleInGame`** | **bool** | **no** | **C4-exclusion (and other "logical-only" gates)** |

**The C4 cursor / Mission_Attack gate is:**

```
target.Type[+0x1577 /* CanC4 */] != 0
AND target.Type[+0x1701 /* InvisibleInGame */] == 0
```

Per `rulesmd.ini` (verified):

- `[CAMISC01]` Oil Derrick: `CanC4=no`
- `[CAMISC02]` Barrel: `CanC4=no`
- `[CAMSC09]`, `[CAMSC10]` McBurger Kong: `CanC4=no`
- All other buildings inherit the default `CanC4=yes`.

Buildings with `InvisibleInGame=yes` (logical-only nodes like victim bridge
anchors) also reject C4 — but no normal targetable building has this flag set,
so it's effectively a defensive gate.

---

## 5. RulesClass C4 Fields (verified from `RulesClass::ReadCombatDamage` decompile)

| Offset | INI Key | Type | Default | Notes |
|--------|---------|------|---------|-------|
| `+0xfa8` | **`C4Warhead`** | WarheadType* | **`Super`** | Damage warhead used by C4 detonation. Super = 100% verses all armor classes. |
| `+0xfac` | `CrushWarhead` | WarheadType* | `Crush` | NOT C4 — distinct field. |
| `+0xfb0` | `V3Warhead` | WarheadType* | `V3WH` | (adjacent) |
| `+0xfb4` | `DMislWarhead` | WarheadType* | `DMISLWH` | (adjacent) |
| `+0xfb8` | `V3EliteWarhead` | WarheadType* | `V3EWH` | (adjacent) |
| `+0xfbc` | `DMislEliteWarhead` | WarheadType* | `DMISLEWH` | (adjacent) |
| `+0xfc0` | `CMislWarhead` | WarheadType* | `CMISLWH` | (adjacent) |
| `+0xfc4` | `CMislEliteWarhead` | WarheadType* | `CMISLEWH` | (adjacent) |
| `+0xfc8` | `IvanWarhead` | WarheadType* | `IvanWH` | Crazy Ivan TimedBomb — distinct from C4. |
| `+0xfcc` | `IvanDamage` | int | 450 | (adjacent) |
| `+0xfd0` | `IvanTimedDelay` | int | 450 frames | (adjacent) |
| `+0xfd4` | `CanDetonateTimeBomb` | bool | no | Double-click enemy bombs |
| `+0xfd5` | `CanDetonateDeathBomb` | bool | no | Double-click own bombs |
| `+0xfd8` | `IvanIconFlickerRate` | int | 8 | (adjacent) |
| `+0xfdc` | `DeathWeapon` | WeaponType* | — | (adjacent) |
| `+0xff0` | `IonCannonWarhead` | WarheadType* | — | Special-cased by Apply_area_damage |
| `+0x1740` | `BridgeStrength` | int | 1500 | (Apply_area_damage uses this for bridge collapse RNG) |
| `+0x1750` | **`C4Delay`** | **double (minutes)** | **`0.03`** ≈ **27 frames @ 15 fps** | **Time between plant start and detonation** |
| `+0x16c0` | `Incoming` | speed | — | NOT a C4 health threshold (the plan suspected this; it's "Incoming" projectile speed) |

**On C4Delay:** the field is parsed as a `double` representing **minutes**.
At 15 ticks/sec (the simulation rate) this is `0.03 × 60 × 15 = 27 ticks`.
The consumer is the building-side update tick, which checks
`g_CurrentFrameCounter - target+0x14a >= C4Delay_in_ticks` — but this exact
read site was not nailed down in this pass (see Open Questions §10).

---

## 6. The DoType Animation Cycle (Fire1–Fire4 = `0x1b`–`0x1e`)

`InfantryClass::Do_Action @ 0x0051d6f0` is the DoType setter; `DoType_Sequencer
@ 0x00520ae0` advances per frame. Findings:

### 6.1 The cycle

The C4 plant uses **DoType `0x1b` → `0x1c` → loop on `0x1c`**:

- **`0x1b` (Fire1):** Entry frame. The sequencer's `case 0x1b` runs:
  ```
  Do_Action(0x1c, 1, 0)              // immediate transition to 0x1c (Fire2)
  if (Type[+0xec9 /* DeployedCrushable */] == 0)
      InfantryClass+0xa9 = 1          // sets some "firing" flag
  FUN_0070f770(this)                  // sets a 4-8 frame timer at +0x180/+0x188
  ```
  So `0x1b` is a one-frame transient that immediately bounces to `0x1c`.

- **`0x1c`/`0x1d`/`0x1e` (Fire2/Fire3/Fire4):** Default-case path. When the
  current animation completes, the sequencer transitions back to `0x1c`:
  ```
  default:
      ...
      if (this->DoType ∈ {0x1b, 0x1c, 0x1d, 0x1e}):
          Do_Action(0x1c, 1, 0)       // re-enter 0x1c (loop)
  ```
  This produces a **continuous 0x1c loop** until something external breaks it
  (mission change, target destroyed, SEAL killed).

- **Loop terminator:** Mission_Attack's branch-3 detects DoType ∈ {0x1b..0x1e}
  for the player and short-circuits the dispatcher — returning a per-tick
  jitter `MissionTimer + Random(0..2)`. Meanwhile Mission_Enter is what
  actually advances state when the SEAL is in mission 0x11 + on-target-cell.

- **The "fire" event** comes from `Fire_At_Target @ 0x005206b0`. That function
  compares `InfantryClass+0xf8` (current animation frame counter) to one of
  `Type+0xe40` (`FireUp`), `+0xe44` (`FireProne`), `+0xe48` (`SecondaryFire`),
  or `+0xe4c` (`SecondaryProne`). When the frame matches, it calls
  `vtable[+0x3cc]` (FireWeapon). For SEAL/Tanya these per-Type firing-frame
  ints are NOT explicitly set in stock INI (default 0), so the actual weapon
  fire happens on frame 0 of the animation.

### 6.2 What CHARGE.SHP / CHARGEN.SHP actually are

The strings `"CHARGE"`, `"CHARGEN"` do **not** appear as string literals
in `gamemd.exe` (only `"ChargeAnim"`, `"ChargedAnimTime"`, `"ChargeToDrainRatio"`,
`"Charges"` exist — none are SHP filenames). **The CHARGE.SHP family is loaded
through the generic SHP-loading pipeline using filename construction**, not
through any C4-specific code. The "C4 plant" visual you see in-game is the
unit's own SHP playing the FireUp sequence range (`164,6,6` for SEAL/Tanya);
the small dynamite anim spawn we see is from the WarheadType's `AnimList`
when `Apply_area_damage` fires (e.g., `[Super] AnimList=...,TWLT070`).

**There is no dedicated "CHARGE.SHP placement animation" wired into the C4
mission path.** The pre-detonation animation IS the FireUp sequence playing
on the SEAL's own SHP. CHARGE.SHP if it exists in the asset bundle is a
visual-only asset loaded by a different path (likely the FireUp particle anim
spawn).

### 6.3 The InfantryClass `+0x6db` flag (NOT C4-related)

`+0x6db` is set when `Do_Action(5)` (Crawl) is called and cleared by
`Do_Action(7)` (Up) or `Do_Action(0x1b)` (Fire1). Read by DoType_Sequencer
to decide between `0x28`/`0x29` (prone walk variants) and `0/3` (stand
variants). This is the **prone-mode flag**, not a C4-plant flag. The plan's
hypothesis ("set on action 5 [for C4 marker]") was misleading — the sequencer
USES this flag, but it's the generic prone/stand state.

The building-side `+0x6df` (different field, on BuildingClass) IS the C4
"being-planted" marker — set in Mission_Enter when the SEAL arrives and
checked on subsequent ticks to prevent a second SEAL from re-planting.

---

## 7. Cursor / Targeting (`What_Action_OnObject @ 0x0051e3b0`)

The cursor for SEAL/Tanya hovering an enemy structure is **action enum
`0x10` (DEMOLISH)**. The exact gate:

```
HouseClass::IsHumanPlayer(this->Owner)
AND (Type[+0xec2 /* C4 */] != 0 OR HasWeaponAbility(0xe /* Assaulter */))
AND iVar7 == 5                              // FootClass returned "attack/force-fire"
AND target != NULL
AND target.GetType() == 6                   // Building
AND target.vtable[+0x80]() == 0             // not iron-curtained
AND target.Type[+0x1577 /* CanC4 */] != 0
AND target.Type[+0x1701 /* InvisibleInGame */] == 0
   → return 0x10        // DEMOLISH cursor
```

If the C4-capable infantry is force-firing (Ctrl-click) on something but the
target fails one of the post-`iVar7 == 5` gates, the function returns `5` —
the standard force-fire/attack cursor, which routes through normal weapon
fire (Sapper), not the C4 plant path. **This is how the FakeC4 weapon on
Chrono Commando works** — the unit is NOT C4-flagged, so What_Action returns
5 (normal attack), the Sapper weapon's `SabotageCursor=yes` flag promotes the
display cursor visually, but the firing path is the generic
`TechnoClass::Fire_At` → FireWeapon → `Apply_damage(FakeC4WH)` flow.

**Other cursor returns observed in this function** (relevant subset):

| Return | Meaning | Trigger |
|--------|---------|---------|
| `0x10` | DEMOLISH | C4 plant target (above) |
| `0x39` | INFILTRATE | Engineer + (target+0x38 != 0 && target+0x68 != 0) |
| `0x3b` | NOATTACK | Force-fire on non-ally with no weapon |
| `9` | ENTER | Garrison/capture target |
| `0x35`/`0x36` | IVAN_BOMB / IVAN_BOMB_ATTACK | Crazy Ivan + valid target |
| `0x40`/`0x47` | (specialized weapon abilities — e.g. Mind Control variants) |
| `3` | REPAIR | Engineer + ally damaged building below threshold |

---

## 8. Apply_area_damage (`0x00489280`) — How the Damage Lands

Function signature (recovered): `Apply_area_damage(CoordStruct *coords, ?,
TechnoClass *source, WarheadType *warhead, char destroy_overlay, ?)`. Key
behaviors observed:

- **Cell-spread iteration** based on warhead's `CellSpread` (Super has
  CellSpread default; the function uses `DAT_007ed3d0[iStack_bc]` where
  `iStack_bc = ftol(warhead.CellSpread)` — a static lookup table of cell
  offsets per spread radius).
- **For each cell-occupant in range**: calls `vtable[+0x16c]`
  (`TakeDamage`/`ReceiveDamage`) with `(coords, computed_damage, warhead,
  source, 0, 0, ?)`.
- **Damage halving** for armored/heavy infantry per their FireSupress
  modifier (`piVar20[+0x1b]` and a `vtable[+0x54]` predicate halve the
  damage).
- **Bridge-cell handling**: if the cell is a high-bridge anchor
  (`OverlayType` ranges `0x4a..0x63` or `0xcd..0xe6`) and the warhead has
  Wall=yes (`+0x144`) AND the damage roll wins
  `Random(1, BridgeStrength) < damage`, calls `DestroyBridge_Low` /
  `DestroyBridge_High`. (This is the same path the engine uses for
  C4Warhead-on-bridge from `BlowUpBridge`.)
- **Recursive overlay-destroy**: if the cell has a destructible overlay
  (`OverlayType[+0x2b0] != 0`), it spawns the overlay's death anim and
  **recursively** calls
  `Apply_area_damage(0, Rules[+0xfa8 /* C4Warhead */], 1, param_6)` — the
  barrel chain-reaction. So **C4Warhead is the canonical "absolute damage"
  warhead reused for chain-detonation of nearby barrels**, regardless of
  what warhead initially fired.
- **Special-cases** `+0xfac` (CrushWarhead — early return value 2) and
  `+0xff0` (IonCannonWarhead — bypasses the bridge RNG check).
- **For radius=0 warheads** (CellSpread<minimum threshold via
  `DAT_007e5168`), only the impact cell is processed. C4Warhead = Super has
  no CellSpread set (default 0), so it's effectively a single-cell
  detonation that hits everything on the building's center cell —
  realistically just the building.

---

## 9. Fire_At_Override (`0x0051df70`) — NOT a Self-Destruct Path

The plan suspected this was a "Suicide-on-fire" path for SEAL/Tanya. **It is
not.** Decompiled body:

```c
int Fire_At_Override(this, TechnoClass *target) {
    this[+0x68d] = 0;
    int result = TechnoClass::Fire_At(target);
    if (result != 0
        && this[+0x81] == 0
        && this->Type[+0xebf /* Fraidycat */] != 0     // NOT Suicide
        && this[+0xbf] == 0) {
        this[+0x1b5] = 300;
        int mission = this->vtable[+0x184]();          // GetMission
        if (mission != 1 /* Sleep */ && mission != 0xf /* Guard */) return result;
        this->vtable[+0x1e8]();                        // SetMission to flee state
    }
    return result;
}
```

The trigger is **`Fraidycat=yes`**, not `Suicide=yes`. Per `rulesmd.ini`
inspection:

- `[GHOST]` SEAL: no `Fraidycat=yes` → never triggers.
- `[TANY]` Tanya: no `Fraidycat=yes` → never triggers.
- `[PTROOP]` Psi-Corp Trooper: no `Fraidycat=yes`.
- `[CCOMAND]` Chrono Commando: no `Fraidycat=yes`.

So **none of the C4-capable units run this code**. SEAL and Tanya correctly
survive C4 plant — there is no engine-level self-destruct, just the explicit
`vtable[+0x174]` walk-away call in the Mission_Enter detonation block.

Confirmed against the original game's behavior: SEAL and Tanya do not die on
C4.

---

## 10. INI Reference (Verified Parsing)

### A. Per-infantry C4 demolitionist flags (parsed via `InfantryTypeClass::ReadINI` at `0x005240a0`)

| Key | Default | Stock units that set it | Currently Parsed in Rust? |
|-----|---------|-------------------------|----------------------------|
| `C4` | no | `[GHOST]` SEAL, `[TANY]` Tanya, `[PTROOP]` Psi-Corp | **No** — only `Assaulter` is parsed |
| `Assaulter` | no | (none stock; modders use it) | Yes (`object_type.rs:503-504, 892`) |
| `Fraidycat` | no | (TS-legacy candidates) | No |
| `Ivan` | no | `[IVAN]` Crazy Ivan only | No |
| `Engineer` | no | `[ENGINEER]` only | No |
| `Agent` | no | `[SPY]` only | No |
| `Thief` | no | (TS legacy — none in YR) | No |
| `VehicleThief` | no | `[YHIJACK]` Hijacker | No |
| `Doggie` | no | `[ADOG]`/`[SDOG]`/`[YADOG]` | No |
| `Deployer` | no | `[E1]` GI / others | (Yes, in another path) |
| `Civilian` | no | civilian units | (Yes) |

### B. Per-building C4 eligibility (parsed via `BuildingTypeClass::ReadINI_Water` at `0x00460050`)

| Key | Default | Buildings setting `=no` | Currently Parsed in Rust? |
|-----|---------|-------------------------|----------------------------|
| `CanC4` | **yes** | CAMISC01, CAMISC02, CAMSC09, CAMSC10 | **No** |
| `InvisibleInGame` | no | (logical-only nodes) | (verify) |

### C. Global combat damage rules (parsed via `RulesClass::ReadCombatDamage` at `0x0066bbd1`)

| Key | Default | Currently Parsed in Rust? |
|-----|---------|----------------------------|
| `C4Warhead` | `Super` | **Partial** — parsed in `bridge_warheads.rs` for bridge-collapse only. Not exposed for infantry C4. |
| `C4Delay` | `0.03` (minutes ≈ 27 ticks) | **No** |

### D. Weapons (verified contents)

| Section | Damage | ROF | Range | Warhead | Report | SabotageCursor |
|---------|--------|-----|-------|---------|--------|----------------|
| `[Sapper]` | 2500 | 100 | 1.5 | `Mechanical` | `SealPlaceBomb` | `yes` |
| `[FakeC4]` | 5000 | 10 | 1.5 | `FakeC4WH` | `SealPlaceBomb` | `yes` |

**Critical observation:** `[Sapper] Warhead=Mechanical`. Mechanical's
`Verses=0%,0%,0%,100%,100%,100%,0%,0%,0%,100%,100%`. **Verses positions 6,7,8
(building armors wood/steel/concrete) are 0%.** This means the Sapper weapon
itself does NOT damage buildings via the warhead. The actual building damage
comes from `Mission_Enter`'s `Apply_area_damage(self, C4Warhead, 1, 0)` call,
where C4Warhead = Super = 100% verses everything.

So the Sapper weapon's role is:
1. Cursor display (`SabotageCursor=yes`)
2. Sound (`Report=SealPlaceBomb`)
3. The "fire" event in Fire_At_Target that triggers the per-frame animation cycle

The actual damage is delivered by C4Warhead via Apply_area_damage from
Mission_Enter, NOT by Sapper's weapon-fire.

For `FakeC4` (Chrono Commando): `Warhead=FakeC4WH` has
`Verses=0%,0%,0%,0%,0%,0%,100%,100%,100%,0%,100%`. Building armors at
positions 6,7,8 are 100%. So **Chrono Commando's FakeC4 damages buildings
through the regular weapon path**, not through Mission_Enter — this is the
"fire-and-forget" version that doesn't require the unit to plant in place.
Per the comment in `[CCOMAND]`: "otherwise he can teleport into a building
and kill it before he unwarps."

### E. Animation sequences (`artmd.ini`)

| Sequence | Owner | FireUp= | FireProne= |
|----------|-------|---------|------------|
| `[SealSequence]` | SEAL/Tanya/Comando/PsiTrooper (shared SHP layout) | `164,6,6` | `212,6,6` |
| `[TanyaSequence]` | Tanya specifically | `164,6,6` | `212,6,6` |

The `Sequence=` Fire1/Fire2/Fire3/Fire4 fields **were not found in the
inspected sequences for SEAL/Tanya** — these sequences use only the standard
`FireUp=` and `FireProne=` ranges. The DoType `0x1b..0x1e` codes are
**generic firing DoTypes** that map to the FireUp animation range; they're
not "C4-specific" sequence keys. The plan's claim that "DoType 0x1b–0x1e
carry the C4 plant animation" is correct in that those DoType codes ARE
active during the plant, but they map to the generic FireUp sequence frames
164–169, not to a separate C4-specific sequence.

### F. Audio

| Cue | Role | Note |
|-----|------|------|
| `[SealPlaceBomb]` (`soundmd.ini:3937`) | `Sounds=icraatta`, `Volume=60` | Plays when Sapper "fires" |
| `[SealSpecialAttack]` (3942) | `Sounds=$iseaexa $iseaexb`, `Type=global` | EVA voice on C4 plant |
| `VoiceSpecialAttack=SealSpecialAttack` on `[GHOST]` | Triggers SealSpecialAttack | Actually fires when SEAL is told to attack a C4-capable building (per cursor 0x10 dispatch) |

---

## 11. Edge Cases (answered)

| Question | Answer |
|----------|--------|
| **Iron-curtained target?** | What_Action_OnObject excludes (returns 5 instead of 0x10). Mission_Attack also rejects via the building.vtable[+0x80]() check inside the iron-curtain predicate path. **C4 plant is rejected.** |
| **Target on a bridge?** | Apply_area_damage's bridge-collapse path can additionally destroy the bridge cell if the warhead has Wall=yes — but `[Super]` does not by default. So the building takes the C4 hit; the bridge survives unless the warhead explicitly damages it. (`BridgeStrength=1500` RNG roll only gates if Wall=yes.) |
| **SEAL killed mid-plant?** | Mission_Enter aborts on the SEAL's death (object cleanup). The building's `+0x6df` marker stays set (set on plant arrival) but no detonation fires because Apply_area_damage is only called from the SEAL's Mission_Enter dispatch. Building-side cleanup of `+0x6df` was not located in this pass — see Open Questions. |
| **Building destroyed by another source mid-plant?** | The SEAL's TarCom is invalidated when the building dies; Mission_Enter falls through the dispatch table without entering the C4 path. SEAL transitions back to Guard via FootClass fallback. |
| **Two SEALs target same building?** | First SEAL to arrive sets `target+0x6df = 1`. Second SEAL on arrival sees the marker, calls `vtable[+0x480](0, 1)` (clear target), `vtable[+0x174]` (re-approach). It loops re-approaching forever until the marker clears (which it does only on detonation completion or building death). **In practice: second SEAL hovers near the building doing nothing useful.** |
| **Force-fire (Ctrl-click) on non-CanC4 building?** | What_Action returns 5 (force-fire), Mission_Attack falls through Test 1 (CanC4 fails), routes to FootClass::Mission_Attack → normal Sapper weapon fire. Sapper.Warhead=Mechanical → 0% damage to building → effectively a no-op except sound. |
| **Force-fire on a unit (not building)?** | Same path: normal Sapper fire. Mechanical does 100% damage to vehicles. So **Ctrl-click Sapper on a Rhino tank deals 2500 raw → 2500 to medium armor = full kill**. (This IS player-reachable behavior in stock YR.) |
| **Target inside fog/shroud?** | What_Action_OnObject's `iVar7` returns from FootClass — fog targeting uses `vtable[+0x2ac]` predicate. If invisible, returns 0x3b (no-attack) instead of 5, so the C4 path never triggers. |

---

## 12. Currently Implemented in Rust (gap analysis)

| Stage | Rust File | Status |
|-------|-----------|--------|
| Parse `C4=` on InfantryType | `src/rules/object_type.rs` | **Missing** (only `Assaulter` parsed) |
| Parse `CanC4=` on BuildingType | `src/rules/object_type.rs` (or building_type) | **Missing** |
| Parse `InvisibleInGame=` on BuildingType | (BuildingType parser) | **Missing** |
| Parse `C4Warhead=` global | `src/rules/bridge_warheads.rs:29,46-48` | **Partial** — bridge-collapse only |
| Parse `C4Delay=` global | (RulesClass struct) | **Missing** |
| Cursor for C4 target | `src/app_cursor.rs:154,214-219` | **Reuses Enter cursor** — ungated; any SabotageCursor weapon shows it. Should gate on `c4 && target.can_c4 && !target.invisible_in_game` |
| Mission state for C4 plant | `src/sim/components.rs::OrderIntent`, `src/sim/command.rs::Command` | **Missing** — no Sabotage/PlantC4 variant |
| Walk-up adjacency + plant timer | (combat / mission dispatch) | **Missing** |
| DoType 0x1b–0x1e equivalent for plant animation | `src/sim/animations/`, `src/render/` | **Missing** |
| Plant-cancellation paths (death/IC/destruction) | (combat) | **Missing** |
| `Apply_area_damage`-equivalent for C4 detonation | `src/sim/combat/` | **Missing** for infantry C4 |
| Building marker (`+0x6df` equivalent) | (building component) | **Missing** |
| Tests for C4 plant flow | `src/sim/world/world_tests.rs:836` (bridge-only) | **Missing** for infantry |

The simplest possible Rust shape (per `feedback_brainstorm_before_implement.md`,
this is for follow-on `/brainstorm`, not for this report to prescribe):
- Per-Type bool `c4` parsed from `C4=`
- Per-BuildingType bool `can_c4` parsed from `CanC4=` (default true)
- New mission state `PlantingC4 { target_id, plant_start_tick }`
- Single-tick detonation when `current_tick - plant_start_tick >= rules.c4_delay_ticks`
  applying a `c4_warhead_id`-attributed damage instance to the target
- Cancellation on attacker death, target death, IC application

Internals don't need to mirror gamemd's three-Apply_area_damage chain (the
2nd/3rd are overlay-chain handlers; for player-visible parity in a stock map
they don't matter unless a barrel happens to be on the building's footprint
cell, which is impossible).

---

## 13. Active-in-YR Classification

| Finding | Active in YR? | Notes |
|---------|---------------|-------|
| C4 plant via Mission_Enter (mission==0x11 + Type[+0xec2]) | **Yes** | Player-reachable every match where SEAL/Tanya are built |
| AI Sabotage via Mission_Capture → Mission_Enter mission=8 | **Yes** | AI uses this for Engineer + Agent + C4 units |
| Fraidycat self-flee (Fire_At_Override) | **Yes but irrelevant** | Active in YR but NO C4-capable unit is Fraidycat=yes |
| TiberiumProof flag (`+0xec0`) | **Yes** | Set on SEAL/Tanya/Comando/PTROOP/etc. |
| Cyborg flag (`+0xeac`) | **Probably TS-legacy** | Sets `+0xc8f=1` if true; no YR unit observed with `Cyborg=yes` |
| HarvestRate on infantry (`+0xeb8`) | **TS-legacy** | Infantry with Harvester role — none in YR |
| DetectionDistance on infantry (`+0xeb0`) | **TS-legacy** | Sub-detection style; not used by YR units |
| Thief flag (`+0xec5`) | **TS-legacy** | No YR unit sets `Thief=yes` |
| C4 → Infiltrate auto-derivation | **Yes** | Per the parser at `0x52466e..0x52469a` |
| Apply_area_damage barrel chain reaction | **Yes** | Active wherever destructible overlays are placed |
| RING1 anim spawn from C4Warhead | **Indirect** | RING1 string exists at `0x008182f0`. Per prior research it spawns when Apply_area_damage hits via Super warhead's `AnimList`. It IS the visual blast for C4 detonation. |
| CHARGE.SHP / CHARGEN.SHP as separate animation | **No** | These strings do not appear in the binary; not loaded by C4-specific code. The "plant animation" IS the unit's FireUp sequence range. |

---

## 14. Open Questions

1. **C4Delay double consumer site.** The `Rules+0x1750` value is parsed but
   the read site that compares it to elapsed frames was not located in this
   pass. Likely in BuildingClass::AI_Update around the `+0x6df` marker check.
   Worth a 30-minute follow-up if exact tick semantics matter.
2. **Building-side cleanup of `+0x6df` on attacker death.** The marker is set
   in Mission_Enter on plant start; the clear path was not traced. Possibly
   building's update tick clears it when `+0x150` (attacker ptr) becomes
   invalid. Worth confirming.
3. **The recursive `Apply_area_damage` calls in Mission_Enter.** Two of the
   three are post-walk-away calls with `param_1==0`. Their exact role
   (chain-damage to surrounding cells vs. defensive duplicate fire) wasn't
   nailed down. Likely chain-overlay handling per §8.
4. **`InfantryClass+0xa9` set in Sequencer case 0x1b** — purpose unverified.
   Likely "currently firing/animating" predicate read elsewhere.
5. **Per-Type firing-frame ints `+0xe40..+0xe4c`**: parsed as INI keys
   `FireUp`/`FireProne`/`SecondaryFire`/`SecondaryProne`. None of these are
   set in stock SEAL/Tanya INI, so they default to 0. Confirm by tracing
   readers in Fire_At_Target — it compares `+0xf8` to one of them, but the
   semantics (frame index vs absolute frame) need verification.
6. **The `target.Mission != 0x13` exclusion** in Mission_Enter's mission==0x11
   path — `0x13` is Selling/Deconstruction. Confirms you can't C4 a
   selling-in-progress building, which matches gamemd behavior, but the
   exact semantics of the other states (`0xb`=Selling, `0x19`=??) weren't
   exhaustively mapped.
7. **`Type+0x6ac`, `+0x6c4`, `+0xd6a`, `+0xd94` on InfantryType** — referenced
   in What_Action_OnObject branches; their INI key bindings weren't traced
   (these are Deployer / undeploy-fire / passenger-related fields, not C4).
8. **TActionClass dispatch table for missions 8 vs 0x11** — confirmed via
   xref existence (4 data refs to Mission_Capture from
   `0x007e24b8..0x007f5e84`) but the exact slot numbers in the dispatch
   array weren't enumerated. Not load-bearing for parity.
9. **`SpecialFlags & ?` C4 gate.** None observed in any of the 9 functions
   decompiled. The plan asked for a quick byte-pattern check; nothing fired.
   **Conclusion: C4 path has no SpecialFlags gate. Always active in YR.**
10. **Phase 3 functions deferred:** `FootClass::Mission_Attack @ 0x004d4dc0`
    (#13), `TechnoClass::Fire_At @ 0x006fdd50` (#14), and
    `TechnoClass::ReceiveDamage` (#17) were not decompiled in this session —
    they are well-documented in other reports
    (`FOOTCLASS_MISSION_ATTACK_GHIDRA_REPORT.md`,
    `RECEIVE_DAMAGE_PIPELINE.md`) and add no C4-specific logic.

None of these blocks implementation of the C4 system at parity quality —
they're nice-to-haves for completeness.

---

## Sources

**Ghidra addresses decompiled in this pass:**
`0x005196a0`, `0x0051f3e0`, `0x004d4b20`, `0x00524400`/`0x005240a0`,
`0x0066bbd1`, `0x00460050`, `0x0051d6f0`, `0x00520ae0`, `0x0051df70`,
`0x005206b0`, `0x0051e3b0`, `0x00489280`, `0x0070f770`.

**Memory reads (string literals at offset push sites):**
`0x825978` ("C4"), `0x82596c` ("Engineer"), `0x825954` ("Agent"),
`0x82594c` ("Thief"), `0x82593c` ("VehicleThief"), `0x825934` ("Doggie"),
`0x825928` ("Deployer"), `0x825914` ("DeployedCrushable"),
`0x825908` ("UseOwnName"), `0x8258fc` ("JumpJetTurn"),
`0x8258f4` ("Crawls"), `0x82595c` ("TiberiumProof"),
`0x818164` ("Civilian"), `0x8259bc` ("Infiltrate"),
`0x8259c8` ("Fraidycat"), `0x8259d4` ("Fearless"),
`0x8259a8` ("Occupier"), `0x82599c` ("Assaulter"),
`0x8259b4` ("Ivan"), `0x825988` ("DetectionDistance"),
`0x82597c` ("HarvestRate"), `0x825a00` ("NotHuman"),
`0x825a0c` ("Cyborg"),
`0x8257d8` ("FireUp"), `0x8257b8` ("FireProne"),
`0x825680` ("SecondaryFire"), `0x825670` ("SecondaryProne").

Building-side strings: `0x81adfc` ("CanC4"), `0x81a8cc` ("InvisibleInGame"),
`0x81ae34` ("Capturable"), `0x81adbc` ("CanBeOccupied"), `0x81a898`
("BridgeRepairHut"), `0x81aa14` ("Hospital"), `0x81aa0c` ("Armory"),
`0x81aac8` ("Grinding"), `0x81aaf0` ("UnitRepair").

Rules strings: `0x83b1d4` ("C4Warhead"), `0x83ad88` ("C4Delay").

**Prior reports cross-referenced:**
`MISSION_GUARD_AREAGUARD_GHIDRA_REPORT.md`,
`FOOTCLASS_MISSION_HANDLERS_GHIDRA_REPORT.md`,
`FOOTCLASS_MISSION_ATTACK_GHIDRA_REPORT.md`,
`BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2.md` / `_V3.md`,
`BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md`,
`ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md`,
`ANIM_CLASS_DEEP_DIVE.md`,
`WARHEAD_DETONATE_GHIDRA_REPORT.md`,
`BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md`,
`BOMB_CLASS_GHIDRA_REPORT.md`,
`READINI_FIELD_MAPS.md`,
`MouseClass_research.md`,
`ENGINEER_CAPTURE_GHIDRA_REPORT.md`,
`RULESCLASS_GHIDRA_REPORT.md`,
`BRIDGE_SYSTEM.md`,
`BUILDINGTYPECLASS_FIELDS.csv`.

**INI files checked:**
`ini/rulesmd.ini` (sections `[GHOST]`, `[TANY]`, `[CCOMAND]`, `[PTROOP]`,
`[VIRUS]`, `[YURI]`, `[YURIPR]`, `[Sapper]`, `[FakeC4]`, `[Super]`,
`[Mechanical]`, `[FakeC4WH]`, `[CombatDamage]`, `[CAMISC01]`, `[CAMISC02]`,
`[CAMSC09]`, `[CAMSC10]`).
`ini/artmd.ini` (sections `[SealSequence]`, `[TanyaSequence]`,
`[ComandoSequence]`, `[PsiTroopSequence]`).
`ini/soundmd.ini` (sections `[SealPlaceBomb]`, `[SealSpecialAttack]`).

**Plan executed:**
`docs/plans/2026-05-10-navy-seal-c4-demolition-investigation-plan.md`
