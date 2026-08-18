# frontier-super — SuperClass (superweapon charge / launch)

**Slug:** `frontier-super`  **Status:** promoted from catalog stub (`_frontier.md` §G1) to full
profile. **Layer:** object-satellite (per-house object-AI satellite ticked inside the
HouseClass per-frame update). **Active in YR:** Yes — core gameplay; all 12 superweapon types
are live in stock YR.

**Verification note (read first):** the Ghidra MCP instance was **offline this entire session**
(`list_instances` → 0 instances; TCP 127.0.0.1:8089 refused on every `connect_instance` /
monitor attempt). Every address below is therefore **carried from the existing verified
research corpus** (5 dedicated SuperClass/SuperWeapon docs, the EVA deep-dive, the HouseClass
report, and the verified per-tick spine spec), cross-checked for internal agreement, **not**
re-decompiled live this session. One genuine inter-doc address conflict is flagged in §2 and
must be settled with one `get_function_by_address` call when Ghidra is back. All other
representative addresses are mutually consistent across ≥2 independent docs.

---

## 1. PURPOSE

The superweapon state machine. Two classes:

- **`SuperWeaponTypeClass`** — static per-`[SWType]` INI definition (recharge time, type enum,
  cursor action, sounds, aux-building requirement, power/charge-drain flags). One per section.
- **`SuperClass`** — runtime instance, **one per SuperWeaponType per HouseClass**. Owns the
  charge timer, ready/suspended/enabled flags, the targeting cell, the ChargeDrain
  state machine, and the readiness/launch EVA cues.

Lifecycle: a building with `SuperWeapon=`/`SuperWeapon2=` completing **grants** the matching
SuperClass on its house → the instance **charges** over `RechargeTime` frames (paused when the
house is under-powered) → goes **ready** (EVA "X ready", sidebar cameo flash) → player or AI
clicks/targets → **Launch** dispatches one of 12 type-specific effects → recharge restarts.

---

## 2. KEY FUNCTIONS + GLOBALS (re-verified against the doc corpus this session; not live-Ghidra this session)

### Representative function

`SuperClass::Launch @ 0x006CC390` — the master fire handler; a switch on `SWType->Type`
(`SuperWeaponTypeClass+0xB4`) into 12 case handlers (Nuke, IronCurtain, LightningStorm,
ChronoSphere, ChronoWarp, ParaDrop, AmerParaDrop, PsychicDominator, SpyPlane, GeneticConverter,
ForceShield, PsychicReveal).

- **Address status: located / cross-confirmed, one conflict pending live Ghidra.**
  - `0x006CC390` is the entry given by `SUPERCLASS_SYSTEM_GHIDRA_REPORT.md` (line 14, line 676),
    `SUPERWEAPON_TYPE_CLASS_GHIDRA_REPORT.md` (line 355), and `EVA_SYSTEM_DEEP_DIVE` (the launch
    EVA call-sites `0x006ccd03 / 0x006ccd81 / 0x006ccdfa / 0x006ccf21 / 0x006cd8bd / 0x006cdc98 /
    0x006cde01` all sit inside a function whose entry is ≤ 0x6cc390 and well below the next labeled
    fn) — three independent agreements.
  - **Conflict:** `CHRONOSPHERE_SUPERWEAPON_GHIDRA_REPORT.md` (lines 46, 881) labels
    `SuperClass::Launch` as **`0x006CC200`**. Two possibilities, both plausible without a live
    read: (a) `0x006CC200` is a stale/earlier label for the same function, or (b) `0x006CC200`
    is a small predecessor block / thunk that tail-calls into `0x006CC390`. The 3-doc majority
    plus the in-range launch-EVA call-sites make **`0x006CC390` the entry of record**; settle
    with a single `get_function_by_address 0x006CC390` + `get_function_by_address 0x006CC200`
    (does 0x6CC200 fall inside the 0x6CC390 function body?) when Ghidra is reachable.

### Per-tick / lifecycle functions (all from the SuperClass doc family)

| Address | Name | Role |
|---------|------|------|
| `0x006CAEC0` | `SuperClass::Constructor` (0-param) | basic field init |
| `0x006CAF90` | `SuperClass::Constructor` (Type, House) | full init + global-array register |
| `0x006CB120` | `SuperClass::Destructor` | |
| `0x006CB560` | `SuperClass::Activate` | building grants SW → enable + start/resume timer |
| `0x006CB4D0` | `SuperClass::Suspend` | pause/resume charge on power transition (saves remaining frames) |
| `0x006CB7B0` | `SuperClass::Deactivate` | granting building lost / house defeated → fully disable |
| `0x006CC080` | `SuperClass::AI_Charging` | per-tick charge step; ready transition; readiness EVA |
| `0x006CBCA0` | `SuperClass::AI_Ready` | per-tick ready/anim-stage update; ready-countdown sound; building flash |
| `0x006CBEE0` | `SuperClass::AnimStage` | 0–54 sidebar charge-pip stage |
| `0x006CC2B0` | `SuperClass::NameReadiness` | CSF readiness text ("Charging"/"Ready"/"Active"/"Offline") |
| `0x006CB3A0` | `SuperClass::SetTargetData` | stores target cell (ChronoSphere first click) |

### SuperWeaponTypeClass functions

| Address | Name | Role |
|---------|------|------|
| `0x006CE5B0` | `SuperWeaponTypeClass::Constructor` | |
| `0x006CEA20` | `SuperWeaponTypeClass::ReadINI` | reads all `[SWType]` keys; RechargeTime = `ftol(minutes * 900.0f)` |
| `0x006CE800` | `::Load` (save-game ctor) | |
| `0x006CE8D0` | `::Save` | delegates to AbstractTypeClass::Save |
| `0x006CE910` | `::ComputeChecksum` | CRC of gameplay fields |
| `0x006CEEF0` | `::FindOrAllocate` | find-by-name / create |
| `0x006CEF80` | `::GetAction` | returns Action enum (ForceShield special case) |

### Owned globals / structs

- `SuperWeaponTypeClass` table — `DynamicVectorClass<SuperWeaponTypeClass*> @ 0x00A8E328`
  (data ptr `+0xC = 0x00A8E334`, count `+0x18 = 0x00A8E340`).
- `SuperClass` instance registry — `DynamicVectorClass<SuperClass*> @ 0x00A83CB8`
  (data ptr `0x00A83CBC`, count `0x00A83CC8`) — all instances across all houses.
- Per-house ownership: `HouseClass+0x258` (SuperClass* array data) / `HouseClass+0x264` (count)
  — the per-house slice the HouseClass tick walks.
- Type-enum string table — `0x008425C0` (12 entries, ends `0x008425F0`).
- Action-enum string table — `0x007E4C50` (73 entries, ends `0x007E4D74`).
- `RechargeTime` minutes→frames factor — `900.0f` const at `0x007F4100`.
- 3×3 cell adjacency table (used by IronCurtain/ChronoWarp/GeneticMutator launch cases) —
  `0x00B0C038` (9 × `short[2]`, ends `0x00B0C05C`).

**SuperClass struct (0x80 bytes), gameplay-load-bearing fields** (from
`HOUSECLASS_GHIDRA_REPORT.md` + `SUPERWEAPON_TYPE_CLASS_GHIDRA_REPORT.md`, mutually consistent):
`+0x24` TimerOverride (-1=use type default), `+0x28` SWType*, `+0x2C` OwnerHouse*,
`+0x30` CDTimerClass {StartFrame,_,Duration} (ChargeStartFrame at +0x30, RemainingFrames at +0x38),
`+0x50` ready-countdown sound timer, `+0x62` packed target cell, `+0x68` AssociatedBuilding*,
`+0x6D` IsEnabled, `+0x6E` IsPostClicked, `+0x6F` IsReady, `+0x70` IsSuspended,
`+0x78` LastAnimStage (sidebar change-detect), `+0x7C` ChargeDrain state (0/1/2).

**SuperWeaponTypeClass key offsets:** `+0xB0` RechargeTime (frames), `+0xB4` Type enum,
`+0xBC` Action, `+0xC8` AuxBuilding*, `+0xE5` UseChargeDrain, `+0xE6` IsPowered (default **true**),
`+0xEE` PostClick, `+0xF5` ManualControl. (Note: `[SWType] RechargeVoice/ChargingVoice/
ImpatientVoice/SuspendVoice` are **not** in stock gamemd — EVA per type is hardcoded, not INI.)

---

## 3. PLUG POINT (per-tick spine rung)

**Rung AA — HouseClass tick @ `0x004F8440`** (`HouseClass::AI/Update`, vt+0x5c), rung 27 of 28
in `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md`. The HouseClass tick walks `g_HouseClass_Array
@ 0x00A8022C` (count `0x00A80238`) forward and, per house, runs the superweapon update over
that house's SuperClass slice (`+0x258`/`+0x264`) — `AI_Ready` each tick, which itself calls
`AI_Charging` while charging. This is **inside Rung AA**, not a separate rung.

> Correction to the seed stub: `_frontier.md` §G1 said "rung U". In this spec rung **U** is the
> AnimClass-subset vector; the **HouseClass** rung is **AA** (`0x004F8440`). The stub's
> "exact sub-position UNVERIFIED" resolves to: ticked **within** the HouseClass per-house loop
> body of Rung AA.

- **Power-transition coupling (not the per-tick spine):** `HouseClass::AI_ResumeProduction`
  / `HandlePowerTransition @ 0x0050AF10` (called from the power system on low-power state change)
  iterates the house's supers and calls `Suspend(true/false)` or `Deactivate`. This is the
  grant/enable + power-gate path, distinct from the Rung-AA charge tick.
- **Launch** is **out-of-band of the charge tick**: it fires from the executed player command
  (target-cell click → `EventClass` → execution) or from an AI super-launch decision
  (`AI::SuperLaunchCheck_SingleSW @ 0x006EFC70`, `_DualSW @ 0x006EFE60`), then dispatches its
  type-specific effect immediately.
- **RNG:** the Rung-AA HouseClass record in the spine notes its synchronized `Scen->Random` draw
  is **local-player-gated** (0 synchronized draws on AI/remote houses). SuperClass charge/ready
  ticking itself is deterministic (frame arithmetic); per-launch RNG (e.g. LightningStorm scatter,
  nuke trajectory jitter) is drawn by the *effect* systems the launch case calls (Rung P for the
  storm process), not by SuperClass charge logic. Effect-side draws must stay on `Scen->Random`.

---

## 4. OUTGOING EDGES (this service depends on → other services)

| → Service | Via (symbol / mechanism) | Evidence |
|-----------|--------------------------|----------|
| `factory-house` | HouseClass owns the per-house SuperClass slice (`HouseClass+0x258/+0x264`); `Constructor 0x004F54A0` creates one per type; grant/power gate via `AI_ResumeProduction/HandlePowerTransition 0x0050AF10` → `Activate/Suspend/Deactivate`; building `SuperWeapon=`/`SuperWeapon2=` (`BuildingTypeClass+0x16F0/+0x16F4`) grants the SW | HOUSECLASS_GHIDRA_REPORT §SuperClass; SUPERCLASS_SYSTEM §5.1–5.2, §9 |
| `frontier-sidebar` | ready-state flashes the SW cameo / sidebar tab; `AnimStage 0x006CBEE0` feeds the charge pip; `HandlePowerTransition` calls `SidebarClass::Refresh` + sets ProductionChanged when local player; `FlashSidebarTabFrames` (`SWType+0xE8`) | SUPERCLASS_SYSTEM §5.5/§5.8; HOUSECLASS_GHIDRA_REPORT lines 3250–3253 |
| `frontier-audio-eva` | readiness + launch voice cues — `AI_Ready 0x006CBCA0` and `AI_Charging 0x006CC080` switch-on-Type call `VoxClass__PlayEVA 0x00752700` (EVA_…Ready); `Launch 0x006CC390` calls it at `0x006ccd03/…/0x006cde01` (EVA_…Activated/Launched) | EVA_SYSTEM_DEEP_DIVE lines 416–445 (verified call-site table) |
| `frontier-audio-voc` | `StartSound`/`SpecialSound` (`SWType+0xC4/+0xC0`, VocClass indices) played on activate/effect; `AI_Ready` ready-countdown plays a ready sound | SUPERCLASS_SYSTEM §3 (offsets 0xC0/0xC4), §5.5 |
| `random-scenario` | LightningStorm pick-random-cell + scatter (Rung P, `Scen->Random`); nuke trajectory jitter; PsychicDominator/wave effects — all drawn by the launch-case effect systems on the synchronized stream | SUPERCLASS_SYSTEM §6 cases 0/2/7; spine spec Rung P |
| `frontier-bullet` | Nuke case (case 0) spawns the carrier/nuke `BulletClass` (alloc `0x0046B050`, fire `0x00468670`, `NukeMaker::SpawnDownwardNuke 0x0046B310`) | NUKE_SUPERWEAPON_GHIDRA_REPORT; SUPERCLASS_SYSTEM §6 case 0 |
| `frontier-anim` | every launch case creates effect anims (IRONBLST, ChronoBlast, GeneticMutator, ForceShield, NUKEBALL, cloud bolts); GeneticMutator conversion routes through `AnimClass::AI 0x00423AC0` MakeInfantry | SUPERCLASS_SYSTEM §6; EVA_SYSTEM_DEEP_DIVE |
| `damage-helpers` | IronCurtain/ForceShield invulnerability application; ChronoWarp non-chronoshiftable kill (C4 warhead `Rules+0xFA8`); GeneticMutator MutateWarhead; nuke/storm/dominator area damage (`Apply_area_damage`) | SUPERCLASS_SYSTEM §6 cases 1/4/9/10; NUKE/PSYCHIC_DOMINATOR reports |
| `cell-map` | every launch case reads target-cell center coords + bridge height, walks cell occupant lists (`CellClass+0xE4` ground / `+0xE8` bridge) over the 3×3 grid; PsychicReveal calls map reveal (`0x005678E0`) | SUPERCLASS_SYSTEM §6 cases 1/4/9/11, §9 |
| `techno-foot` | ChronoWarp builds a TeleportLocomotion on warped units and piggybacks their existing locomotor; IronCurtain/ForceShield set per-techno invuln fields (StartFrame +0x18C, Duration +0x194, IsForceShield +0x1C4) | CHRONOSPHERE_SUPERWEAPON_GHIDRA_REPORT §3; IRONCURTAIN_FORCESHIELD report |
| `rules-class` | `SuperWeaponTypeClass::ReadINI 0x006CEA20` reads every `[SWType]` key; launch cases read `[General]` keys (IronCurtainDuration `Rules+0xFE8`, ForceShieldRadius/Duration, PsychicRevealRadius, paradrop inf lists, effect anim type ptrs) | SUPERCLASS_SYSTEM §3, §8 |

(AI launch decision via `AI::SuperLaunchCheck_*` ties to the deferred `frontier-ai-house` brain —
project rule defers AI, so not implemented now, but recorded as the AI-side launch trigger.)

---

## 5. INCOMING EDGES (other services → this service)

| ← Service | Via (symbol / mechanism) | Evidence |
|-----------|--------------------------|----------|
| `factory-house` (Rung AA, HouseClass tick `0x004F8440`) | per-house loop runs `AI_Ready` (→ `AI_Charging`) each tick over the house's SuperClass slice; HouseClass::Constructor creates the instances; `HandlePowerTransition 0x0050AF10` drives Activate/Suspend/Deactivate on power/grant changes | spine spec Rung AA; HOUSECLASS_GHIDRA_REPORT §SuperClass; SUPERCLASS_SYSTEM §9 |
| `logicclass` (per-tick spine) | LogicClass::PerTickUpdate Rung AA is the driver that ultimately ticks SuperClass charge/ready; the launch effect systems (storm/dominator) run at Rung P; lockstep RNG order constrains effect-side draws | LOGICCLASS_PERTICKUPDATE_SPINE_SPEC Rungs AA, P |
| `frontier-net-eventqueue` | player target-cell command is wrapped into an `EventClass` and executed at the scheduled frame → triggers `SuperClass::Launch`; lockstep-critical (launch must execute at the same frame on all peers) | SUPERCLASS_SYSTEM §9 ("Player input / AI decision → Launch"); PARADROP_SUPERWEAPON §5 |
| `frontier-input-command` | sidebar cameo click + target-cursor action (`Action` enum `0x007E4C50`, e.g. Nuke=20, IronCurtain=37, ChronoSphere=39) resolve the click into the SW-target command | SUPERCLASS_SYSTEM §11 (Action enum); SUPERWEAPON_TYPE_CLASS |
| `mission-radio` / spy / triggers | spy infiltration resets a SW recharge timer (BuildingClass::OnSpyInfiltrate BRANCH 4); a map TriggerAction can force-`Activate`; building destroy/sell → `Deactivate` | SPY_INFILTRATION_SYSTEM BRANCH 4; SUPERCLASS_SYSTEM §9 ("TriggerAction::Execute → Activate") |
| `frontier-saveload` | `SuperWeaponTypeClass::Load 0x006CE800` / `::Save 0x006CE8D0`; SuperClass instances persist as part of the per-house save walk | SUPERWEAPON_TYPE_CLASS §6 |

---

## 6. ACTIVE-IN-YR / TS-LEGACY

- **Active in stock YR:** Yes — the whole system. All 12 type enum entries map to live stock
  superweapons (Nuke, Iron Curtain, Lightning/Weather, Chronosphere+Warp, both Paradrops,
  Psychic Dominator, Spy Plane, Genetic Mutator, Force Shield, Psychic Reveal). The charge/ready
  tick runs every match for every house that owns a granting building; cameo flash + EVA cues are
  player-visible every game where a superweapon is built.
- **TS-legacy / not stock:** none of the SuperClass control path is TS-dead. Caveats:
  - The Action enum (`0x007E4C50`) contains some `DontUse`/`TibSunBug` placeholder entries — TS
    legacy slots, not used by stock YR superweapons.
  - `SuperWeaponTypeClass+0xA0–0xAC` are vestigial (never read; in checksum only) — dead fields,
    not behavior. (Confidence 85% per the type-class report.)
  - `RechargeVoice=`/`ChargingVoice=`/`ImpatientVoice=`/`SuspendVoice=` INI keys do **not** exist
    in stock gamemd (mod-extension keys); EVA voices are hardcoded per Type — do not parse them
    for stock parity.
  - One launch effect, **EMPulse** (a SuperClass::Launch case in some reports), routes to the
    EMPulseClass family which is effectively TS-dead in stock (EMP warhead `;gs disabled`, Rung S
    list stays empty) — the *launch case* may exist but produces no stock effect.

---

## 7. RUST IMPLEMENTATION STATUS (informational)

Per SUPERCLASS_SYSTEM §10: **not implemented** beyond the `GameOptions::super_weapons` on/off flag,
cursor enum variants + cameo sprites, and the *separate* chrono-miner teleport. SuperWeaponTypeClass
INI parsing, SuperClass runtime state/timers, building grant/enable, all 12 launch handlers, sidebar
cameo integration, AI auto-fire, and power suspend/resume are unbuilt. This profile is the map node;
promotion to an implementation plan would start from SUPERCLASS_SYSTEM_GHIDRA_REPORT.md.

---

## 8. SESSION CAVEATS (honesty ledger)

- **Ghidra was offline this session** — no address was re-decompiled live. Status of every address
  is "carried from prior verified docs, cross-checked for inter-doc agreement," not "re-verified in
  Ghidra 2026-06-29."
- **One unresolved address conflict** (§2): `SuperClass::Launch` = `0x006CC390` (3 docs, majority +
  in-range EVA call-sites) vs `0x006CC200` (Chronosphere doc). Default verdict per project rule =
  treat as unresolved until a live `get_function_by_address` settles whether 0x6CC200 is the same
  function, a stale label, or a predecessor thunk. Entry-of-record here = **0x006CC390**.
- The plug-point rung was corrected from the stub's "rung U" to **rung AA** (HouseClass
  `0x004F8440`) using the verified spine spec — high confidence (spine spec is binary-verified).
