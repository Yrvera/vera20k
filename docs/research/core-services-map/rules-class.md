# Core Service Profile — RulesClass (slug: `rules-class`)

> The parsed-INI gameplay-tunables global. How parsed rules reach the sim.
> Defers RNG/scenario state to `random-scenario`; defers raw INI byte reading to
> `ini-parsing`.

**Evidence base:** `docs/research/RULESCLASS_GHIDRA_REPORT.md` (Ghidra-verified,
addresses cited) + `docs/plans/2026-06-10-scenarioclass-rulesclass-substrate-plan.md`
(Rust-port substrate). Authority order binary → Ghidra → docs; the report's
load-bearing addresses were re-verified live in its own session.

---

## Purpose

`RulesClass` is the single global that holds all *game-wide* tuning data read
from the top-level sections of `rulesmd.ini` (base `rules.ini` as fallback,
then a second pass over the map INI). It is the engine's root source-of-truth
for non-per-type gameplay constants: veterancy/repair/build-speed globals,
combat globals (Iron Curtain, occupy/bunker/open-topped multipliers, overload,
DeathWeapon), AI parameters, IQ thresholds, three embedded difficulty multiplier
sets, the speed×land-type movement table, special-weapon warhead/projectile
bindings, crate/radiation/elevation/wall models, multiplayer-dialog defaults,
the ColorAdd remap, and global anim/sound bindings.

It is NOT per-type data — weapons, infantry, buildings, warheads live on
`WeaponTypeClass`/`InfantryTypeClass`/etc. RulesClass holds the type-class
*arrays* in a sense only through its loader (`Read_INI` allocates them), but the
typed data is owned elsewhere.

**Lifecycle:** allocated once `operator new(0x18C0)` in `Init_Game`
(`0x0052BAD8`); populated by `ScenarioClass::Full_Init` (`0x00686B20`) before the
tick loop; freed in `Game_Shutdown` (`0x006BE1C0`). No per-tick re-read.

**Vtable:** NONE — plain non-polymorphic singleton (`this+0` is an int field
default `0x0F`, not a vtable pointer; report §1, ctor bytes at `0x00665650`).

---

## Owns

- `g_RulesClass_Instance` — global pointer at `0x008871E0` (stores `RulesClass*`).
- The `RulesClass` instance — `0x18C0` bytes (6336), flat struct, no vtable.
- Embedded state (offsets from `RulesClass*`, all report §2):
  - `[Maximums] Players` cap — `0x14D0` (also mirrored into `DAT_00A8B548`).
  - `[IQ]` thresholds — `0x1434`–`0x145C`.
  - `[CombatDamage]` globals — Iron Curtain `0xFE8`, MaxDamage `0x16C8`,
    C4Delay `0x1750`, occupy/bunker/open-topped multipliers `0xF40`–`0xF60`,
    overload triples `0xEF8`–`0xF38`, DeathWeapon `0xFDC`, warhead bindings
    `0xFA8`–`0xFC4`, etc.
  - `[CrateRules]` — `0x40`, `0xF8`–`0x100`, `0x1140`–`0x172C`.
  - `[Radiation]` — `0x1804`–`0x1834`.
  - `[ElevationModel]` `0x1838`–`0x1848`; `[WallModel]` `0x1850`–`0x1858`.
  - `[SpecialWeapons]` Nuke/EMP/Mutate warhead+projectile bindings —
    `0xF8C`–`0xFA4`.
  - `[JumpjetControls]` defaults — `0x40C`–`0x438`.
  - `[MultiplayerDialogSettings]` defaults — `0x1480`–`0x14BB` (overwritten by
    the MP dialog before `Full_Init`).
  - `[AI]` parameters — `0x8AC`–`0xB1C`, `0x10A0`–`0x10C0`, `0x1100`–`0x1768`.
  - Three embedded `DifficultyClass` slots (Easy/Normal/Difficult) —
    `0x1538`/`0x1588`/`0x15D8`, each `0x50 B`.
  - ColorAdd 16-entry RGB remap — `0x1874`.
- **Side-tables NOT on the instance but owned by the same loader** (report §3,
  §5, §12):
  - Speed×LandType movement table — global `0x0089EA44` (0x180 B), every
    multiplier clamped ≤ 1.0; Winged/Flying row hardcoded 1.0 (INI ignored).
  - `[Colors]` per-house scheme palette — globals `0x00886380`/`0x00885780`.
  - `[Powerups]` crate-bonus table — four parallel globals (`DAT_0081DA8C`
    weight, `DAT_0081DAD8` anim, `DAT_0089ECC0` enabled, `DAT_0089EC28` value),
    19 fixed slots.
  - `[AdvancedCommandBar]` button array — `DAT_00B0CB78` (no-op in stock YR).

**Rust-port ownership** (substrate plan): the parsed result lives in
`RuleSet` (`src/rules/ruleset.rs`, `RuleSet::from_ini`), `GameOptions`
(`src/sim/game_options.rs`), `terrain_rules.rs`, `color_scheme.rs`,
`radiation` rules. SC-2 added a separate `ScenarioSession`
(`src/sim/scenario_session.rs`) owning seed/clock/options/identity/bounds — that
is the `random-scenario`/scenario aggregate, NOT RulesClass; the two are
distinct in both binary and port.

---

## Key functions & globals (addresses)

| Symbol | Address | Role |
|---|---|---|
| `g_RulesClass_Instance` | `0x008871E0` | global pointer to the singleton |
| `RulesClass::Constructor` | `0x00665650` (→`0x00667A26`) | sets all ~1085 defaults |
| `RulesClass::Destructor` | `0x00667A30` | teardown |
| `RulesClass::Process` (outer) | `0x006686C0` | clears type arrays, reads `[Maximums]`, calls inner, ColorAdd, LANGRULE merge, **second pass over map INI** |
| `RulesClass::Read_INI` (inner) | `0x00668BF0` | section-by-section dispatch (33 steps, report §5) |
| `ReadGeneral` | `0x0066D530` (→`0x00671E98`) | `[General]` ~240 keys |
| `ReadAudioVisual` | `0x006691E0` (→`0x0066B8FF`) | `[AudioVisual]` ~100 keys |
| `ReadCombatDamage` | `0x0066BBB0` | `[CombatDamage]` ~60 keys |
| `ReadAI` (FUN_00672AE0) | `0x00672AE0` | `[AI]` ~45 keys |
| `ReadIQ` | `0x00674240` | `[IQ]` |
| `ReadMultiplayerDialogSettings` | `0x00671EA0` | MP dialog defaults |
| `ReadSpeedTypeLandTypeTable` | `0x00674000` | → global `0x0089EA44` |
| `ReadSpecialWeapons` | `0x00668FB0` | Nuke/EMP/Mutate bindings |
| `ReadCrateRules` | `0x0066B900` | `[CrateRules]` |
| `ReadRadiation` | `0x0066CF70` | `[Radiation]` |
| `DifficultyClass::Read_INI` | `0x0066D270` | three embedded slots |
| `Type_Read_INI_All` (FUN_00679A10) | step 21 of inner | iterates all type-class arrays, calls each `vtable+0x64` (ReadINI), + `MissionClass::Read_INI` |
| `Init_Game` (ctor site) | `0x0052BAD8` | `operator_new(0x18C0)` |
| `ScenarioClass::Full_Init` (populate site) | `0x00686B20` | the single caller of `Process` |
| `Game_Shutdown` (dtor site) | `0x006BE1C0` | frees singleton |

---

## Tick / render position

**Not in the per-tick spine.** RulesClass is a load-time service: populated once
in `ScenarioClass::Full_Init` before the tick loop, then read-only at runtime.
It has no `LogicClass::PerTickUpdate` step of its own. Its outputs are *consumed*
during the tick (combat, vision, ore, crates, diplomacy) and during render
(damage-state frames, color schemes, ColorAdd tint), but the class itself does
no per-tick work.

---

## Depends-on (outgoing edges)

These are the services RulesClass *calls into / writes* during its load pass.

- **`ini-parsing`** — via `CCINIClass`/`INIClass` accessors
  (`ReadInt`/`ReadBool`/`ReadDouble`/`ReadString`/`ReadRange`/`ReadSpeed`/
  `ReadColorRGB`/`ReadPercent`). Every Read* method reads through the
  `CCINIClass*` passed to `Process`/`Read_INI`. Evidence: report §2 ("standard
  pattern … each into a direct offset"), §5 signature `(RulesClass*, CCINIClass*)`.
  This is the foundational edge — RulesClass is the structured consumer of the
  raw INI service. **(strong)**

- **`abstract-object`** (TypeClass allocation) — inner `Read_INI` steps 3–13
  run find-OR-allocate loops over every type-class array
  (`[Countries]`/`[OverlayTypes]`/`[SuperWeaponTypes]`/`[Warheads]`/
  `[SmudgeTypes]`/`[TerrainTypes]`/`[VehicleTypes]`/`[AircraftTypes]`/
  `[InfantryTypes]`/`[BuildingTypes]`/`[Animations]`/`[VoxelAnims]`/
  `[Particles]`/`[ParticleSystems]`), e.g. `UnitTypeClass::FindOrAllocate`
  (`0x007480d0`) → `operator_new(0xe78)` + ctor on a name miss. Evidence:
  report §5 steps 3–13, §9.3. RulesClass is the *creator/owner of the type
  registry population* — the strongest structural outgoing edge. **(strong)**

- **`techno-foot` / per-type ReadINI** — step 21 `Type_Read_INI_All`
  (FUN_00679A10) iterates every type-class array and calls each instance's
  `vtable+0x64` (`ReadINI`), so the per-type field data (weapon stats, armor,
  speed, locomotor) is loaded *by RulesClass's loader*. Evidence: report §5
  step 21, §9.2. (Named `techno-foot` as the nearest object-AI/type slug; the
  actual per-type records are `abstract-object`-rooted.) **(strong)**

- **`mission-radio`** — step 21 also runs `MissionClass::Read_INI` for each map
  mission script as part of `Type_Read_INI_All`. Evidence: report §5 step 21.
  **(medium — sub-sequence inside FUN_00679A10 not fully decomposed, report §9.2)**

- **`random-scenario` (ScenarioClass)** — RulesClass does not *call* scenario
  state, but its entire load is *driven by* `ScenarioClass::Full_Init`
  (`0x00686B20`), and the second map-INI override pass uses the map `CCINIClass`
  built by `Read_Scenario_INI` (`0x00686730`). This is primarily an *incoming*
  control edge (scenario orchestrates rules load); listed here only because the
  map-override pass reads a scenario-owned INI object. Evidence: report §9.3.
  **(medium — control flows scenario→rules; data edge is the map INI handle)**

- **`drawing-helpers` / palette** (data-producing edge) — `[Colors]` reader
  `FUN_0066D3A0` (`0x0066D3A0`) populates the global palette/scheme table at
  `0x00886380`/`0x00885780`, and `[ColorAdd]` reader `FUN_0066D480` writes the
  16-slot RGB remap at `+0x1874`. These feed the render color path. Evidence:
  report §5 step 1–2, §10. **(medium — produces tables render reads; not a call
  into a render function)**

- **`lookup-tables`** — `ReadSpeedTypeLandTypeTable` writes the static
  read-only speed×land-type table to global `0x0089EA44`; `[Powerups]` writes
  four parallel static tables. These are exactly the "static read-only table
  substrate." Evidence: report §3, §5 steps 17–18. **(strong)**

> Note: RulesClass has NO outgoing edge to `logicclass` (it is not scheduled),
> nor to `cell-map`, `factory-house`, `pathfinding-helpers`, `target-scoring`,
> `damage-helpers` — those services *read* RulesClass, the dependency is
> incoming, not outgoing.

---

## Used-by (incoming edges)

Subsystems that dereference `g_RulesClass_Instance` (`0x008871E0`) at runtime.
All verified callers from report §6 unless noted.

- **`logicclass`** — `LogicClass::PerTickUpdate` reads RulesClass (DamageDelay,
  lightning/storm cadence). **(report §6)**
- **`damage-helpers`** — `Apply_area_damage`, `Warhead::SelectExplosionAnim`,
  combat-damage resolution read MaxDamage/MinDamage/warhead/overload/occupy
  multipliers. **(report §6)**
- **`cell-map`** — `CellClass::RecalcAttributes`, `CellClass::BlowUpBridge`,
  `CellClass::IsWallConnectableInDirection`, `MapClass::RevealAroundCell`,
  `MapClass::ParanoidRevealAll/UnrevealAll` read terrain/bridge/sight rules.
  **(report §6)**
- **`bridge-helpers`** — bridge strength/collapse via `CellClass::BlowUpBridge`
  reading BridgeStrength (`0x1740`)/CollapseChance (`0x17CC`). **(report §2, §6)**
- **`factory-house`** — `HouseClass::Recalculate_Alliances`, `MakeAlly`,
  `BreakAlliance`, `ComputerTakeover` read diplomacy/AI-takeover rules; build
  speed + survivor divisor on building sell. **(report §6, §8)**
- **`abstract-object` / `techno-foot`** — `ObjectClass::Reveal` reads sight
  bonuses; `BuildingClass::GetCurrentFrame` reads damage-state frame thresholds;
  `AnimClass::Constructor`/`Middle` read damage-fire anim selection. **(report §6)**
- **render (`frontier-render`/`drawing-helpers`)** — `BSurface::Constructor`,
  health-bar/condition colors (ConditionYellow/Red), ColorAdd tint consumers.
  **(report §6, §8)**
- **crate system** — `CrateSlot::Place/Validate/Remove` read crate-regen cadence
  (CrateRegen `0x1678`). **(report §6)**
- **lightning SW** — `LightningStorm::*` reads all `[General]` lightning keys
  directly from the singleton. **(report §2 §6)**
- **water/impact** — `Wave_splash_forces`. **(report §6)**

Rust-port consumers (substrate report §8, verified): ore growth, production
speed, chrono SW, miner purifier, garrison combat, power damage-delay,
pathfinding retry cadence, aircraft reload/flight-level, building-sell survivor
divisor — all read `RuleSet`/`GameOptions`.

---

## Open / unverified edges

- **`Type_Read_INI_All` (FUN_00679A10) internal order** — step 21 is known to
  iterate every type array + run `MissionClass::Read_INI`, but the exact
  sub-sequence (which type loads first, `[AITriggerTypes]`/`[ScriptTypes]`/
  `[TeamTypes]`/`[TaskForces]` ordering) was not decomposed. Affects the
  precise shape of the `abstract-object`/`techno-foot`/`mission-radio` outgoing
  edges. (report §9.2) **UNCHECKED.**
- **ColorAdd per-slot consumers** — the 14-slot remap is written, but *which*
  render subsystem indexes each slot (Iron Curtain flash, Dominator tint, chrono
  blend, health-bar mix) is not traced. The `drawing-helpers` incoming edge is
  inferred at the table level, not per-slot. (report §9.1) **UNCHECKED.**
- **Map-INI TypeClass / `[Colors]` allocation-from-map** — binary DOES allocate
  new type records / color schemes from a map (find-OR-allocate, report §9.3),
  but the Rust port deliberately keeps this OFF (`merge_rules_overrides` is
  value-override-only). Known DRIFT, fires only on maps embedding new
  type/color definitions; stock retail maps do not. (substrate plan §RC-1
  BLOCKED, report §9.3) **DRIFT, deferred.**
- **`[General]`/`[AudioVisual]` individual field offsets** — enumerated in
  `RULESCLASS_FIELDS.csv` but the two large readers were not decompiled
  end-to-end in a single pass (MEDIUM confidence per report §2). Does not change
  the service-edge shape, only per-key offsets.
- **Value-sensitive `rules_hash` (port-side)** — `app_sim_tick.rs:1356` hashes
  only the four ID-list registries, not rule *values*; map value overrides do
  not change `rules_hash`. Pre-existing port gap, deferred (substrate plan
  Key Decisions). Not a gamemd edge.
