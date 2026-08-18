# Core Service Profile: random-scenario (RandomClass + ScenarioClass)

**Slug:** `random-scenario`
**Primary doc:** `docs/research/RANDOM_SCENARIO_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (2026-05-29, live-Ghidra verified)
**One-line:** The engine's session substrate — the deterministic RNG streams plus the loaded-scenario singleton (identity, map metadata, waypoints, per-map flags) that every other system reads but none owns.

---

## Purpose

Provides lockstep determinism for the whole engine. Given one shared `u32` seed, every client reproduces the same RNG streams in the same draw order so every damage roll, scatter facing, ore-growth pick, debris angle, and lightning strike lands identically. Three cooperating pieces (not one class):

1. **`RandomClass`** — a Westwood R(250,103) XOR lagged-Fibonacci PRNG primitive (0x3F4 bytes). Three independent instances, each its own draw cursor.
2. **`ScenarioClass`** — the heap-allocated session singleton (0x3740 bytes, ptr `g_ScenarioClass_Instance @ 0x00A8B230`). Owns one of the three RNG streams (`Scen+0x218`) plus scenario identity, map metadata, MP start waypoints, and per-map session flags.
3. **A ring of standalone session globals** kept outside the struct but conceptually the same substrate: shared seed, lockstep frame counter, mode selector, replay flags, and the other two RNG instances.

---

## Owns

**RNG primitive + streams:**
- `RandomClass` instance state (0x3F4 each): `disabled@0 / index_a@4 / index_b@8 (seeded 0x67=lag 103) / state[250]@0xC`.
- `Scen->Random @ Scen+0x218` — the persisted gameplay RNG stream (serialized? NO — re-derived from seed).
- `g_MainRng @ 0x00886B88` — second gameplay stream (combat/weapon/AI/visual); seeded identically, NOT serialized with scenario.
- `g_MapGenRng @ 0x00ABE890` — random-map-generator stream; seeded from the RandomMap config object's `+0x74`, separate from `g_RngSeed`; also drives the bridge walker-variant selector.

**Session globals:**
- `g_RngSeed @ 0x00A8ED94` — the single shared `u32` seed (seeds both gameplay streams identically).
- `g_CurrentFrameCounter @ 0x00A8ED84` — lockstep frame index (plain counter, not RNG); reset in Read_Scenario, incremented in Main_Tick.
- `g_GameMode @ 0x00A8B238` — 0 campaign / 3 LAN / 4 Internet / 5 skirmish.
- `DAT_00A8D5F8` — replay flag bitfield (`&2` = playback gate).
- `DAT_00A8B8B8` — MP "seed already set" gate.

**ScenarioClass session fields (substrate-relevant):**
- Identity: `ScenarioFilename +0x125C` (0x104), `ScenarioIndex +0x34CC`, `IsRandom +0x34BD`.
- Map metadata: `Theater +0x1258`, `StartX/Y/Width/Height +0x112C..+0x1138`, `NumberStartingPoints +0x113C`, `Waypoints[] +0x1140`.
- Start positions: `MP_Waypoints[702] +0x632` (packed `{i16 cellX,i16 cellY}`, sentinel `DAT_00B05458/5A`), `StartSlotHouse[16] +0x1180`, `StartStaging[8] +0x11C0`.
- Per-map flags: `scenario_flags +0x0` (FogOfWar bit 0x1000, dormant in YR), `TiberiumGrowthEnabled +0x34A6`, `MultiplayerOnly +0x34BC`, `LoadingInProgress +0x3598`.

**Does NOT own:** any per-frame state-hash. The claimed `Scen+0xD64/0xD68` "state hash" is REFUTED — that block is the tactical-view scroll/camera on `g_Tactical @ 0x00887324`. gamemd lockstep is command-gate sync, not state-hash. Also not the lobby `SessionClass` (separate concern).

---

## Key functions & globals (addresses)

| Symbol | Addr | Contract |
|--------|------|----------|
| `Random__Next` | `0x0065C780` | `disabled→0`; `state[a]^=state[b]; r=state[a]; a++,b++` wrap 250; return r |
| `Random__RandomRanged` | `0x0065C7E0` | `__thiscall(this,low,high)`; inclusive; `low==high` no-draw; swap reversed; rejection mask `(1<<(msb+1))-1`; disabled→`low` |
| `RandomClass__Seed` | `0x0065C6D0` | `index_a=0,index_b=0x67`; 250-word 4-round Feistel mixer over 8 table constants; `disabled=0` |
| `Init_Random_Number_System` | `0x0052FC20` | SP entropy or MP/replay seed → `g_RngSeed`; seed `Scen+0x218` then `g_MainRng` from same value (253-dword copy) |
| `RandomClass__DrawRanged` (LCG) | `0x0065C660` | **DEAD** TS-legacy 15-bit LCG; zero callers |
| Seed tables | `0x00839644`, `0x00839694` | 4 dwords each consumed by mixer (byte-exact in Rust `INIT_TABLE_1/2`) |
| `ScenarioClass__Constructor` | `0x006832C0` | seeds `Scen+0x218` with 0; fills `+0x632` sentinels; inits flags |
| `CCFileClass__Constructor` (allocator) | `0x0052BA60` | `operator_new(0x3740)`; boot-time singleton alloc |
| `Main_Game` | `0x0052D9A0` | per-game driver: frame=0 → Init_Random_Number_System → Start_Scenario |
| `Start_Scenario` | `0x00683AB0` | filename→`+0x125C`; `+0x34CC`=index; → Read_Scenario |
| `Read_Scenario` | `0x00684620` | frame=0; `+0x3598`=1; `.SED`? → `+0x34BD`; dispatch generator vs INI |
| `Read_Scenario_INI` | `0x00686730` | normal-map INI read |
| `Full_Init` | `0x00686B20` | Create_Houses → AssignStartingPoints → Read_INI_Basic → units |
| `Read_INI_Basic` | `0x00689E90` | map dims, waypoints, per-map flags |
| `Gather_Start_Positions` | `0x00688380` | setup-phase RNG draws (Scen stream) to fill start-position deficits |
| `AssignStartingPoints` | `0x005EE9D0` | start-slot→house assignment |
| `Generate_Random_Units` | `0x006886B0` | MP start-unit generation |
| random-map generator | `0x00598960` | uses `g_MapGenRng` |
| bridge walker-variant selector | `0x00598030` | draws `g_MapGenRng` (`MOV ECX,0x00ABE890` @ `0x0059805e`) |

---

## Tick / render position

**Not in the per-tick `LogicClass::PerTickUpdate` spine as an actor.** RandomClass/ScenarioClass are a passive substrate read by the tick, not a phase of it. Two timing-relevant facts:

- **Seed pipeline runs once at session start, before any draw or the first tick.** Lifecycle order (RNG draw order depends on it): `CCFileClass__Constructor` (boot alloc) → `Init_Random_Number_System` → `Start_Scenario` → `Read_Scenario` → `Full_Init` (Create_Houses → AssignStartingPoints → `Gather_Start_Positions` setup draws → Read_INI_Basic → Read_Units → Post_Map_Init/Generate_Random_Units). The seed is fixed BEFORE the first setup-phase RNG draw and before tick 1.
- **`g_CurrentFrameCounter` is incremented late in `Main_Tick`** (write `0x0055DE81`); the whole tick reads the pre-increment value (late-increment, already MATCH in Rust).

During each tick, RNG streams are *consumed* by other services in their own phases (movement scatter, combat/damage, ore growth, anims, particles); the substrate just supplies the cursor.

---

## Depends-on (outgoing edges)

This service sits at the bottom of the layering — it reads almost nothing. The only outgoing edges are setup-phase draws against its own streams and the structural alloc/lifecycle plumbing.

- **ini-parsing** — via `Read_Scenario_INI 0x00686730` / `Read_INI_Basic 0x00689E90` reading `[Header]` (StartX/Y/Width/Height) and `[Basic]` (MultiplayerOnly) and per-map flags through CCINIClass accessors. Evidence: `decompile_function 0x00689E90` (section reads). The scenario load is the consumer of the INI parser to populate ScenarioClass fields.
- **factory-house** — via `Create_Houses 0x00687F10` invoked inside `Full_Init 0x00686B20`; scenario lifecycle constructs the HouseClass instances. Evidence: lifecycle chain §2.5, `decompile_function 0x00686B20`. (Construction edge, runs once at load — scenario drives house creation, not the reverse.)
- **cell-map** — `AssignStartingPoints 0x005EE9D0` / `Gather_Start_Positions 0x00688380` read/validate start cells against the map grid while filling start positions (drawing the Scen stream). Evidence: `decompile_function 0x00688380`, `0x005EE9D0`. (Setup-phase, load-time.)

Note: these are load-time/construction edges. At per-tick steady state the substrate has **no outgoing edges** — it is a pure source.

---

## Used-by (incoming edges)

The substrate is read by nearly every gameplay system. The RNG-stream consumers below are disassembly-verified (ECX binding read at the CALL site, §4.2/§4.4 of the primary doc). Edge form: `consumer-slug → via gamemd function (stream)`.

- **techno-foot** — `InfantryClass__Scatter 0x0051D0D0` (scatter facing, Scen->Random); `FootClass__Mission_Enter 0x004d9290` retry jitter (Scen->Random). Locomotion/object-AI scatter draws.
- **damage-helpers** — `WarheadTypeClass__Detonate 0x004690b0`: top-of-function warhead-property rolls (`+0x17c..+0x188` → `DAT_0087f7ec/f0`) are **g_MainRng**; all scatter/debris/shrapnel draws (VoxelAnim/AnimClass debris, two `RandomRanged(0,0x20)` shrapnel-bolt angles, `FUN_0049F420` explosion scatter) are **Scen->Random**. Per-draw mixed routing (§4.4).
- **cell-map** — `CellClass__PlaceInfantryInCell 0x00481180` sub-cell pick (Scen->Random); `CellClass__DestroyOverlay 0x00480cb0` wall-damage (Scen->Random); `TiberiumClass__GrowthProcessor 0x00722f00` ore growth (Scen->Random).
- **bridge-helpers** — `CellClass__BlowUpBridge 0x0047dd70`, `MapClass__CollapseBridge_* 0x00575220…` collapse/debris (Scen->Random, all 11 callsites); bridge **walker-variant selector `0x00598030`** draws **g_MapGenRng** (NOT scenario — corrects prior docs).
- **abstract-object / world objects** — `AnimClass__AI 0x00423ac0` (anim, Scen->Random); `ParticleClass__Constructor 0x0062b5e0` / `ParticleSystemClass__AI_Smoke 0x0062ed40` (generic particles, Scen->Random); `TechnoClass__SpawnRadEruption 0x006fd800` (5 draws, **g_MainRng**) and `IncreaseGattlingStage 0x0070de70` (**g_MainRng**).
- **factory-house** — `HouseClass__Update 0x004F8440` (AI/economy draws, **g_MainRng**); also created by scenario lifecycle (bidirectional: house creation is a depends-on, house AI draws are used-by).
- **frontier-objects (terrain/superweapon)** — `TerrainClass::AI 0x0071C730` TIBTRE growth (Scen->Random); `LightningStorm__GroundStrike 0x0053a300` / `__Process 0x0053a6c0` (6 sites, Scen->Random, zero g_MainRng); `BuildingClass__DestructionEffects 0x004415f0` / `SpawnSurvivors 0x00442d90` smudge/survivors (Scen->Random).
- **logicclass** — reads `g_CurrentFrameCounter` as the lockstep clock; the tick spine increments it. (Frame-clock dependency, not an RNG draw.)
- **rules-class** — indirect: ScenarioClass per-map flags (FogOfWar, TiberiumGrowthEnabled) gate behaviors that RulesClass tunables also feed; consumers read both. (Per-map flag override of global rules.)

Effectively every active sim system that rolls a random value is a used-by; ScenarioClass identity/map-bounds/waypoints are read-by cell-map, factory-house, and the render/UI layer at load.

---

## Open / unverified edges

- **`g_MapGenRng` reachability beyond bridge repair + random-map gen** — bridge walker-variant (`0x00598030`) and random-map generator (`0x00598960`) verified; any other live-skirmish third-stream consumers UNCHECKED.
- **True projectile-spread X/Y binding** — `TechnoClass::Fire` (the real Fire, NOT `WarheadTypeClass__Detonate 0x004690b0`) was not examined; its spread-X/Y stream is UNVERIFIED. The `TWO_STREAM_RNG_DESIGN` doc mislabels `0x004690b0` as `TechnoClass::Fire` — corrected here.
- **MP `DAT_00A8B8B8` positive-setter** — who sets the "seed already set" gate during connect is unfound (only zero-writers seen).
- **`u64`→`u32` seed truncation** — confirm gamemd `g_RngSeed` is `u32` and discarding high bits is intended.

## Doc-staleness corrected by the primary study (authority = binary)

- `RNG_SYSTEM §3.1/§3.2`: LightningStorm + generic particles are **Scen->Random**, NOT g_MainRng. Only rad-eruption/Gattling are g_MainRng.
- `PER_FRAME_RNG_CONSUMPTION_ORDER`: bridge collapse is **Scen->Random** (all 11 sites), NOT g_MainRng.
- `TWO_STREAM_RNG_DESIGN §3`: `0x004690b0` is `WarheadTypeClass__Detonate` (not `TechnoClass::Fire`); its g_MainRng draws are warhead-property rolls, not spread X/Y.
- `Scen+0xD64/0xD68` "state hash" REFUTED — it's the tactical-view scroll on `g_Tactical`.
- `RNG_SYSTEM §6` "no dual-stream (highest-priority gap)" RESOLVED — two-stream split is built, hashed, serialized, routing-tested.
