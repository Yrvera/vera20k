# RandomClass + ScenarioClass as an Engine Substrate Service — Study & Replacement-Boundary Design

**Date:** 2026-05-29
**Mode:** study/design only — no Rust written. Authority order binary → Ghidra (live) → docs.
Every load-bearing native claim was re-verified live this session; citations inline. Default
verdict on any difference is **DRIFT/UNKNOWN** until proven (CLAUDE.md burden of proof).
**Bar:** active in a standard **local skirmish** (`g_GameMode == 5`) / campaign (`== 0`);
MP-only (`== 3/4`), SpecialFlags, and TS-legacy paths are flagged DORMANT.
**Builds on (does not re-decide):** `RNG_SYSTEM_GHIDRA_REPORT.md`,
`RNG_MP_SEED_HANDSHAKE_AND_GAMEPLAY_INSTANCE_GHIDRA_REPORT.md`, the
`*_RNG_CLASSIFICATION_GHIDRA_REPORT.md` family, `TWO_STREAM_RNG_DESIGN_20260529.md`,
`SUBSTRATE_PARITY_LEDGER_20260529.md`. Where those decided something this cites it; where this
session's live disassembly **contradicts** them the correction is called out in §9.

---

## 0. Executive summary

"RandomClass + ScenarioClass" is the engine's **session substrate**: the deterministic clock,
identity, and randomness that every other system reads but none owns. In gamemd it is **three
cooperating pieces**, not one class:

1. **`RandomClass`** — a Westwood **R(250,103) XOR lagged-Fibonacci** PRNG primitive (0x3F4 bytes:
   `disabled@0 / index_a@4 / index_b@8 / state[250]@0xC`). Three *instances* exist, each a separate
   draw stream that advances independently.
2. **`ScenarioClass`** — the heap-allocated **session singleton** (0x3740 bytes, pointer at
   `g_ScenarioClass_Instance @ 0x00A8B230`). It **owns one of the three RNG streams** (`Scen+0x218`)
   plus scenario identity, map metadata, multiplayer start waypoints, and the per-map session flags.
3. **A ring of standalone session globals** that gamemd keeps *outside* the struct but are
   conceptually the same substrate: the shared seed `g_RngSeed @ 0x00A8ED94`, the lockstep frame
   counter `g_CurrentFrameCounter @ 0x00A8ED84`, the mode selector `g_GameMode @ 0x00A8B238`, replay
   flags, and the other two RNG instances (`g_MainRng`, `g_MapGenRng`).

**The substrate's whole job is lockstep determinism:** given one shared `u32` seed, every client
reproduces the same RNG streams in the same draw order, so every damage roll, scatter facing,
ore-growth pick, debris angle, and lightning bolt lands identically.

**State of the Rust port — the headline:** the **PRNG primitive is done and byte-exact**, and the
**two-stream split (`scenario_rng` + `main_rng`) is already implemented, hashed, snapshotted, and
routing-tested** (this supersedes RNG_SYSTEM §6's "no dual-stream — highest-priority gap", which is
stale). The algorithm, the rejection-sampling `RandomRanged`, and the seed-derivation tables are all
verified bit-identical to gamemd. The **real remaining work is the *session-substrate* layer**, not
the RNG math:

- **No `ScenarioClass`-equivalent session object exists.** Session state (seed, two RNG streams, frame
  counter, map dims, identity, options, waypoints) is **scattered** across the `Simulation`
  god-struct and the app layer, with the same facts duplicated in 3+ places.
- **The per-match seed is never wired.** Every real skirmish runs the hardcoded `DEFAULT_SIM_SEED`;
  there is no entropy/MP-handshake/replay seed injection. Replays record the seed but never re-apply it.
- **Three latent routing DRIFTs** for not-yet-ported consumers (radiation eruption, warhead detonate,
  bridge walker-variant) that the existing accessor names would misroute.

**Headline verdict:** the RNG substrate is essentially complete and faithful; the **Scenario session
substrate is the gap**. The work is (a) a small `ScenarioSession` aggregate inside `sim/` that owns
seed + streams + frame clock + identity + map bounds + waypoints, (b) wiring the negotiated seed
through it at launch/MP/replay, (c) modelling the third RNG instance (`g_MapGenRng`) for bridge
walker-variant + random-map paths, and (d) fixing the three latent routing DRIFTs before their
consumers are ported.

---

## 1. Verified active-YR responsibilities

### 1.1 RandomClass (the PRNG primitive)

| # | Responsibility | Evidence | Active in skirmish |
|---|---|---|---|
| RC1 | **Single-step draw** — R(250,103) XOR lagged-Fibonacci `state[a]^=state[b]; ret state[a]; a++,b++` wrap at 250 | `decompile_function 0x0065C780` | Yes |
| RC2 | **Range-bounded draw** — `RandomRanged(low,high)` inclusive both ends, swap on reversed, no-draw on `low==high`, rejection mask `(1<<(msb+1))-1`, disabled→`low` | `decompile_function 0x0065C7E0` | Yes |
| RC3 | **Seed → 250-word state** — 4-round Feistel mixer over two 4-dword constant tables; sets `index_a=0`, `index_b=0x67` (the lag), `disabled=0` | `decompile_function 0x0065C6D0` | Yes |
| RC4 | **Three independent draw streams** seeded identically, advancing independently | `decompile_function 0x0052FC20` | Yes |

### 1.2 ScenarioClass (the session singleton)

| # | Responsibility | Evidence | Active in skirmish |
|---|---|---|---|
| SC1 | **Own the persisted gameplay RNG stream** (`Scen->Random` @ `Scen+0x218`, 0x3F4 bytes) | `disassemble_function 0x006832C0 @0x006832C9`; `decompile_function 0x0052FC20` | Yes |
| SC2 | **Hold scenario identity** — filename `+0x125C` (0x104), scenario index `+0x34CC`, IsRandom flag `+0x34BD` | `decompile_function 0x00683AB0`, `0x00684620` | Yes |
| SC3 | **Hold map metadata** — theater `+0x1258`, StartX/Y/Width/Height `+0x112C..+0x1138`, NumberStartingPoints `+0x113C` | `decompile_function 0x00686B20`, `0x00689E90` | Yes |
| SC4 | **Own MP start waypoints** — `+0x632` (702 packed `{i16 cellX, i16 cellY}` slots, sentinel `DAT_00B05458/5A`); per-map Waypoints `+0x1140`; start-slot→house table `+0x1180` | `disassemble_function 0x006832C0 @0x00683410`; `decompile_function 0x00688380`, `0x005EE9D0`, `0x00689E90` | Yes |
| SC5 | **Own per-map session flags** — scenario flags dword `+0x0` (FogOfWar bit 0x1000), MultiplayerOnly `+0x34BC`, TiberiumGrowthEnabled `+0x34A6`, FreeRadar/crate flags `+0x34A4..+0x34B4`, loading-in-progress `+0x3598` | `decompile_function 0x00689E90`, `0x00686B20`, `0x00684620` | Yes |
| SC6 | **Drive scenario lifecycle** — construct → seed → read INI/map → create houses → assign starts → read units → post-map-init | `decompile_function 0x0052ba60`, `0x0052d9a0`, `0x00683AB0`, `0x00684620`, `0x00686730`, `0x00686B20` | Yes |
| SC7 | **Be a setup-phase RNG consumer** — `Gather_Start_Positions` draws `Random__RandomRanged` (Scen stream) to fill start-position deficits, so setup draws are part of the lockstep contract | `decompile_function 0x00688380` | Conditional (deficit) |

### 1.3 Session globals (substrate, but standalone — NOT ScenarioClass fields)

| # | Global | Role | Evidence |
|---|---|---|---|
| G1 | `g_RngSeed @ 0x00A8ED94` | The single shared `u32` seed; seeds both `Scen->Random` and `g_MainRng` identically | `decompile_function 0x0052FC20`; `list_globals g_RngSeed` |
| G2 | `g_CurrentFrameCounter @ 0x00A8ED84` | Lockstep frame index (a plain counter, **not** an RNG); reset in `Read_Scenario`, incremented in `Main_Tick` | `get_xrefs_to 0x00A8ED84` (WRITE `0x0055DE81`/`0x0052DA08`) |
| G3 | `g_GameMode @ 0x00A8B238` | Mode selector: 0 campaign / 3 LAN / 4 Internet / 5 skirmish (numeric branches verified; LAN-vs-Internet label is doc-sourced) | `list_globals g_GameMode`; branches in `0x0052FC20`/`0x00683AB0`/`0x00686B20` |
| G4 | `DAT_00A8D5F8` | Replay flag bitfield (`&2` = playback gate in seed pipeline) | `decompile_function 0x0052FC20` |
| G5 | `DAT_00A8B8B8` | MP "seed already set" gate — non-zero skips SP entropy so a handshaked seed survives | `decompile_function 0x0052FC20`; `get_xrefs_to 0x00A8B8B8` |
| G6 | `g_MainRng @ 0x00886B88` | Second gameplay RNG stream (combat/weapon/AI/visual); seeded identically to `Scen->Random`, **not** serialized with the scenario | `decompile_function 0x0052FC20` |
| G7 | `g_MapGenRng @ 0x00ABE890` | Random-map-generator RNG; seeded from the RandomMap config object's `+0x74`, **separate** from `g_RngSeed`. Also consumed by the bridge walker-variant selector (§3) | `decompile_function 0x00598960`; `get_xrefs_to 0x00ABE890` |

What this substrate is **not**:
- **Not a state-hash carrier.** The claimed `Scen+0xD64/0xD68` "state hash" is **REFUTED** — it is the
  **tactical-view scroll/camera position** on `g_Tactical @ 0x00887324`, not ScenarioClass, and no
  hash is computed there (`decompile_function 0x006d6000`/`0x006d6170`/`0x006d8640`/`0x006d8b30`;
  ECX = `[0x00887324]`, viewport-clamp + view-center math). gamemd has **no** live per-frame
  state-hash on ScenarioClass; lockstep is command-gate sync. (Five independent verifiers converged.)
- **Not the lobby/session object.** `g_GameMode`, the MP packet handlers, and the lobby `SessionClass`
  are a separate concern (`SESSIONCLASS_GHIDRA_REPORT.md`); ScenarioClass is the *loaded scenario*.

---

## 2. Surface inventory

### 2.1 RandomClass layout (0x3F4 bytes = 1012)

| Off | Type | Field | Notes (evidence: `decompile_function 0x0065C780`/`0x0065C6D0`) |
|-----|------|-------|------|
| +0x000 | u8 | `disabled` | If ≠0: `Next` returns 0, `RandomRanged` returns `low`, **no state advance**. Set 0 (active) at end of `Seed`. |
| +0x001 | u8[3] | padding | alignment |
| +0x004 | i32 | `index_a` | first cursor; seeded 0; ++ per draw; wraps to 0 when `>0xF9` (≥250) |
| +0x008 | i32 | `index_b` | lagged cursor; seeded **`0x67`=103** (this sets the R(250,103) lag); ++/wrap as above |
| +0x00C | u32[250] | `state[]` | the ring buffer; filled by the seed mixer; XOR-mutated in place by `Next` |

`struct_size = 0xC + 250*4 = 0x3F4`; `Init_Random_Number_System` clones a seeded instance with a
**253-dword (0xFD) copy** = `index_a`+`index_b`+250 words (`decompile_function 0x0052FC20`).

### 2.2 RandomClass methods / global helpers

| Symbol | Addr | Contract | Verified |
|--------|------|----------|----------|
| `Random__Next` | `0x0065C780` | `disabled→0`; `state[a]^=state[b]; r=state[a]; a++,b++` wrap 250; return r | `decompile_function 0x0065C780` |
| `Random__RandomRanged` | `0x0065C7E0` | `__thiscall(this,low,high)`; inclusive; `low==high` no-draw; swap reversed; rejection mask `(1<<(msb+1))-1`; accept-first `<=span`; disabled→`low` | `decompile_function 0x0065C7E0` |
| `RandomClass__Seed` | `0x0065C6D0` | `__thiscall(this,u32)`; `index_a=0,index_b=0x67`; 250-word 4-round mixer; `disabled=0` | `decompile_function 0x0065C6D0` |
| `Init_Random_Number_System` | `0x0052FC20` | SP entropy or MP/replay seed → `g_RngSeed`; seed `Scen+0x218`, then seed `g_MainRng` from the same value | `decompile_function 0x0052FC20` |
| `RandomClass__DrawRanged` (LCG) | `0x0065C660` | **DEAD** TS-legacy 15-bit LCG (`*this*0x41C64E6D+0x3039`, `>>10 &0x7FFF`); **no callers** | `decompile_function 0x0065C660`; `get_function_callers 0x0065C660` = none |

### 2.3 Seed-derivation static tables (must reproduce byte-for-byte)

| Addr | First 16 bytes (LE) | u32 dwords | Rust mirror |
|------|---------------------|------------|-------------|
| `0x00839644` | `87 68 a9 ba 2c d3 17 1e 3c dc bc 03 b2 d1 33 0f` | `0xBAA96887,0x1E17D32C,0x03BCDC3C,0x0F33D1B2` | `INIT_TABLE_1` (`rng.rs:11`) — **EXACT MATCH** |
| `0x00839694` | `58 3b 0f 4b c3 f0 74 e8 a6 c5 55 69 46 ca a7 55` | `0x4B0F3B58,0xE874F0C3,0x6955C5A6,0x55A7CA46` | `INIT_TABLE_2` (`rng.rs:12`) — **EXACT MATCH** |

(`read_memory 0x00839644`/`0x00839694`.) Only the first 4 dwords of each are consumed (loop bound
`0x10` step 4). All-seeds parity is **provable**: identical tables + identical mixer + identical
draw step ⇒ bit-identical stream for every `u32` seed (pinned empirically at seed=1 by
`test_gamemd_raw_sequence_seed_one`).

### 2.4 ScenarioClass singleton — substrate field map (0x3740 bytes, ptr `0x00A8B230`)

Allocated once at boot: `operator_new(0x3740)` in `CCFileClass__Constructor @ 0x0052BA60`
(`decompile_function 0x0052ba60`); never re-allocated per skirmish, only field-reset. All offsets
re-verified this session; **substrate-relevant** fields only (incidental mission/UI strings, timers,
embedded vectors summarized).

| Off | Field | Meaning | Evidence | Active |
|-----|-------|---------|----------|--------|
| +0x0 | `scenario_flags` | SpecialFlags dword; **bit 0x1000 = FogOfWar** (gates `FUN_005866C0`, default OFF in YR) | `decompile_function 0x00686B20` (gate confirmed by 5 verifiers @ `0x00687c75`) | flag yes / fog dormant |
| +0x218 | `Random` | embedded `RandomClass` (0x3F4); the persisted gameplay RNG stream | `disassemble_function 0x006832C0 @0x006832C9`; `decompile_function 0x0052FC20` | **Yes** |
| +0x632 | `MP_Waypoints[702]` | packed `{i16 cellX,i16 cellY}` per slot (`+0x632+i*4`); first 8 = player starts; sentinel `DAT_00B05458/5A` | `disassemble_function 0x006832C0 @0x00683410`; `decompile_function 0x00688380` | Yes |
| +0x112C | `StartX` | map `[Header]` StartX (i32) | `decompile_function 0x00689E90` (verifier: section is `[Header]`, not `[Basic]`) | Yes |
| +0x1130 | `StartY` | map `[Header]` StartY (i32) | `decompile_function 0x00689E90` | Yes |
| +0x1134 | `Width` | map `[Header]` Width (i32) | `decompile_function 0x00689E90` | Yes |
| +0x1138 | `Height` | map `[Header]` Height (i32) | `decompile_function 0x00689E90` | Yes |
| +0x113C | `NumberStartingPoints` | `[Header]` start-spot count (i32) | `decompile_function 0x00689E90` | Yes |
| +0x1140 | `Waypoints[]` | per-map `Waypoint%d` cell array (2-dword stride, `ReadMinMax`), **all-modes** (not SP-only) | `decompile_function 0x00689E90` (verifier corrected SP→all-modes) | Yes |
| +0x1180 | `StartSlotHouse[16]` | start-slot→house index table (init 0xFFFFFFFF) | `decompile_function 0x00686B20`, `0x005EE9D0` | Yes |
| +0x11C0 | `StartStaging[8]` | random-map start slots, copied into `+0x632` when IsRandom | `decompile_function 0x00686B20` | Conditional |
| +0x1258 | `Theater` | theater index (0 Temperate, 1 Snow, …) | `decompile_function 0x00686B20` (`[0x496]`); TerrainClass consumer | Yes |
| +0x125C | `ScenarioFilename` | retained `.map`/`.SED` filename (0x104) | `decompile_function 0x00683AB0`, `0x00684620` | Yes |
| +0x34A6 | `TiberiumGrowthEnabled` | per-map ore-growth gate (the one `CellClass::GrowTiberium` checks) | `decompile_function 0x00689E90` | Yes |
| +0x34BC | `MultiplayerOnly` | `[Basic]` MultiplayerOnly bool | `decompile_function 0x00689E90` | Yes |
| +0x34BD | `IsRandom` | random-map (`.SED` suffix) flag → selects generator vs normal-INI branch | `decompile_function 0x00684620` | Conditional |
| +0x34CC | `ScenarioIndex` | campaign/scenario house index (-1 sentinel); set by `Start_Scenario` | `decompile_function 0x00683AB0`, `0x00686B20` | Yes |
| +0x3598 | `LoadingInProgress` | set 1 during `Read_Scenario`, 0 on completion | `decompile_function 0x00684620` | Yes |
| +0xD64..+0xD8C | **NOT ScenarioClass** | tactical-view scroll/camera block on `g_Tactical @ 0x00887324` — **refuted** as a Scen field | `decompile_function 0x006d6000`/`0x006d8640`/`0x006d8b30` | n/a |

(Real-time radar timers `+0x614`/`+0x620`, frame timers `+0x11E8..+0x1250` seeded from
`g_CurrentFrameCounter`, embedded vectors `+0x34D4/+0x34F0/+0x350C` (AllowableUnits/Maximums),
lighting `+0x3528..+0x3594`, and the campaign-only mission strings/movies are documented in
`scenLayout`/`scenLegacy` notes; out of the substrate core.)

### 2.5 Lifecycle ordering (the substrate's init contract — RNG draw order depends on it)

```
boot:    CCFileClass__Constructor 0x0052BA60
           → operator_new(0x3740); ScenarioClass__Constructor 0x006832C0  (seeds Scen+0x218 with 0; fills +0x632 sentinels; inits flags)
           → Init_Random_Number_System 0x0052FC20  (boot-time)
per game: Main_Game 0x0052D9A0
           → g_CurrentFrameCounter = 0
           → Init_Random_Number_System 0x0052FC20   ← THE per-skirmish seed; BEFORE Start_Scenario
                · g_RngSeed = (SP entropy | MP-handshake | replay-file)
                · Seed(g_RngSeed) → copy 253 dwords → Scen+0x218
                · Seed(g_RngSeed) → copy 253 dwords → g_MainRng   (both byte-identical)
           → Start_Scenario 0x00683AB0   (filename→+0x125C; +0x34CC=index; → Read_Scenario)
                → Read_Scenario 0x00684620   (frame=0; +0x3598=1; .SED? → +0x34BD)
                     · IsRandom: FUN_00597A10 → generator FUN_00598960 (g_MapGenRng) → Post_Map_Init
                     · normal:   Read_Scenario_INI 0x00686730 → Full_Init 0x00686B20
                          → Create_Houses 0x00687F10
                          → (DAT_00A8B244==2) AssignStartingPoints 0x005EE9D0 → Gather_Start_Positions 0x00688380 (RNG: Scen stream)
                          → Read_INI_Basic 0x00689E90 (map dims, waypoints, flags)
                          → map sections / overlay / tiberium queues
                          → Read_Units_Section 0x00743270 → BuildingClass__ReadFromINI
                          → (MP) Post_Map_Init 0x00686890 → Generate_Random_Units 0x006886B0
```

**Invariant:** the seed is fixed (Init_Random_Number_System) **before** any setup-phase RNG draw
(`Gather_Start_Positions`, random-unit generation) and before the first tick. There is **no named
`ScenarioClass::Save`/`Load`**; the embedded RNG is **not** serialized — replays record only the
`u32 g_RngSeed` (+ `+0x1254` settings + `+0x125C` filename) and **re-derive** RNG state by re-seeding
(`decompile_function 0x0052d9a0`; `search_functions` found no Scen serialize method).

---

## 3. Active vs inactive / legacy / dormant

**Active in standard skirmish:** RC1–RC4, SC1–SC7, G1–G7; both gameplay RNG streams; the lifecycle
chain; per-map flags; map metadata; MP/per-map waypoints.

**Conditional (mode/feature-gated, still YR-live when triggered):**
- **Random-map generator** (`FUN_00597A10`→`FUN_00598960`, `g_MapGenRng`) — only when a `.SED`
  random map is chosen (`+0x34BD`). On fixed `.map`/`.mmx` it never runs.
- **`AssignStartingPoints`/`Generate_Random_Units`** — non-campaign (`g_GameMode != 0`) and gated by
  session selectors (`DAT_00A8B244==2`, `g_IsMapEditor==0`, `DAT_00A8B23C==0`). This is the live
  skirmish/MP start-spot + start-unit path, **not** campaign-only (`get_function_callers 0x006886B0`
  / `0x005EE9D0`).
- **Crate seeding** in `Post_Map_Init` — gated `DAT_00A8B261` (`[crates]` session flag).
- **MP seed handshake / replay seed** — only modes 3/4 / replay (G4/G5).

**DORMANT by INI/SpecialFlags default in YR (do NOT implement as always-on):**
- **FogOfWar darkening** (`scenario_flags & 0x1000` → `FUN_005866C0`, the previously-seen-cell
  re-shroud) — `FogOfWar=no` default; the gate is normally not taken. (Matches the project's known
  TS-fog ghost — implement shroud only.) Verified by 5 verifiers.
- Campaign mission metadata (`+0x4`/`+0x108` NextScenario, `+0x1434..+0x144C` movies, `+0x4D8`
  Briefing, `[Ranking] ParTime*`) — read for all modes but consumed only in campaign.

**TS-legacy / dead (do NOT implement):**
- **`RandomClass__DrawRanged @ 0x0065C660`** — the old 15-bit LCG ranged helper; **zero callers**.
  Its existence confirms gamemd carries the TS LCG, but the live YR draw path is exclusively the
  R(250,103) Feistel-seeded generator.

**Out-of-sim (render/UI layer):**
- The tactical-view scroll block (`g_Tactical+0xD64..0xD8C`) — recorded only in the SP replay stream;
  must live in the render/app layer, never in `sim/`.

---

## 4. Current Rust architecture comparison

### 4.1 RNG primitive + two-stream split — built and faithful

| Native contract | Rust | Verdict |
|---|---|---|
| R(250,103) XOR step, `index_b=0x67`, wrap 250 | `SimRng::next_u32` (`rng.rs:106`) | **MATCH** (raw seq pinned seed=1) |
| `RandomRanged` inclusive / no-draw-on-equal / swap / rejection mask | `next_range_u32_inclusive` (`rng.rs:139`) | **MATCH** (except §4.3 extreme-span) |
| 4-round seed mixer + the 8 table constants | `reseed` + `INIT_TABLE_1/2` (`rng.rs:73`) | **MATCH** (byte-exact vs `read_memory`) |
| two streams seeded identically from one seed | `scenario_rng`+`main_rng`, both `SimRng::new(seed)` + `debug_assert_eq!` (`mod.rs:472-520`) | **MATCH** |
| both streams part of lockstep state | hashed in fixed order (`world_hash.rs:39-44`); both serialized, `SNAPSHOT_VERSION=12` | **MATCH** |
| per-callsite stream routing | 12 intent-named accessors (`mod.rs:527-540`) + 1-test-per-accessor (`rng_routing_tests.rs`) | **mostly MATCH** — see §4.4 |

This **supersedes** RNG_SYSTEM §6 ("no dual-stream design — highest-priority gap") — that note is stale.

### 4.2 Routing correctness — disassembly-verified this session

Every **ported** sim consumer routes to the **correct** gamemd instance (each row read at the ECX
setup before the `CALL`, this session):

| Rust accessor | gamemd binding | Function (evidence) | Verdict |
|---|---|---|---|
| `scatter_rng`/`subcell_rng` | Scen->Random | `InfantryClass__Scatter 0x0051D0D0`, `CellClass__PlaceInfantryInCell 0x00481180` | MATCH |
| `wall_damage_rng` | Scen->Random | `CellClass__DestroyOverlay 0x00480cb0` | MATCH |
| `smudge_rng` | Scen->Random | `BuildingClass__DestructionEffects 0x004415f0`, `SpawnSurvivors 0x00442d90` | MATCH |
| `bridge_rng` (collapse/debris) | Scen->Random | `CellClass__BlowUpBridge 0x0047dd70`, `MapClass__CollapseBridge_* 0x00575220…` | MATCH |
| `ore_rng` | Scen->Random | `TiberiumClass__GrowthProcessor 0x00722f00`, `TerrainClass::AI 0x0071C730` (TIBTRE) | MATCH |
| `anim_rng` | Scen->Random | `AnimClass__AI 0x00423ac0` | MATCH |
| `particle_rng` (generic) | Scen->Random | `ParticleClass__Constructor 0x0062b5e0`, `ParticleSystemClass__AI_Smoke 0x0062ed40` | MATCH |
| `superweapon_rng` | Scen->Random | `LightningStorm__GroundStrike 0x0053a300`, `__Process 0x0053a6c0` (6 sites, **zero g_MainRng**) | MATCH |
| `miner_jitter_rng` | Scen->Random | `FootClass__Mission_Enter 0x004d9290` retry jitter | MATCH |
| `weapon_spread_rng` | g_MainRng (partial) | `WarheadTypeClass__Detonate 0x004690b0` warhead-prop rolls | **partial — see §4.4** |
| `house_ai_rng` | g_MainRng | `HouseClass__Update 0x004F887D/0x004F8895` | MATCH (unported) |

### 4.3 What drifts in the RNG layer

1. **Per-match seed never wired (HIGH).** Every real skirmish uses `DEFAULT_SIM_SEED` (`mod.rs:75`);
   `with_seed`/`reseed_both` are called only from tests. `spawn_entities` builds the sim via
   `Simulation::new()` (`app_init_helpers.rs:360`). No entropy / MP-handshake / replay seed injection
   exists. **DRIFT for MP lockstep and replay-by-seed** (every match is accidentally deterministic on
   one constant). Mirrors gamemd's G1 pipeline being unimplemented on the Rust side.
2. **Replay does not re-apply the recorded seed (HIGH for replay).** `ReplayHeader.seed` is written
   (`app_sim_tick.rs:260`) but `ReplayRunner::run` (`replay.rs:76-91`) replays commands only and never
   re-seeds from `header.seed`. Determinism silently rests on the default constant.
3. **`g_MapGenRng` (third stream) unmodeled.** Rust has two streams. gamemd's bridge **walker-variant
   selector `FUN_00598030` draws `g_MapGenRng`** (`MOV ECX,0x00ABE890`, verified `0x0059805e`), as does
   the random-map generator. Rust routes bridge walker-variant through `bridge_rng`→scenario ⇒ **DRIFT**
   on a third stream (advances scenario when gamemd doesn't, and produces different variants).
4. **Extreme-span `RandomRanged` early-out (boundary DRIFT, unreachable).** `rng.rs:150-152` returns
   `lo+0x8000_0000` for `span>=0x7FFF_FFFF` without sampling; gamemd rejection-samples with mask
   `0xFFFFFFFF`. No YR caller passes such a span — flagged, not active.
5. **`u64`→`u32` seed truncation** (`rng.rs:32`) — only the low 32 bits drive state; the replay header
   stores the full `u64`. gamemd's seed is `u32`; confirm intent before relying on the high bits.
6. **`f64`/`f32` in two sim draws** — `terrain_spawn` probability (`%1_000_000` ×1e-6 f64) and
   `ore_growth` priority (f32 `total_cmp`). Integer-derived but a cross-platform determinism surface
   (CLAUDE.md forbids float in game logic) — confirm bit-stable across the AMD/Vulkan + headless targets.

### 4.4 Latent routing DRIFTs (unported consumers — accessor names would misroute)

These are **not bugs today** (the consumers aren't ported, so the streams aren't advanced), but the
existing accessor intent-names would route them to the **wrong** stream when implemented:

- **Radiation eruption / Gattling stage = g_MainRng**, not scenario. `TechnoClass__SpawnRadEruption
  0x006fd800` (5 draws, all `MOV ECX,0x886b88`) and `IncreaseGattlingStage 0x0070de70` are g_MainRng
  (confirms RNG_SYSTEM §3.1 *for these*). But `particle_rng()→scenario` backs the whole particle
  family — if rad-eruption is wired through it, **DRIFT**. Needs a `main`-routed accessor.
- **Warhead detonate is per-draw mixed.** `WarheadTypeClass__Detonate 0x004690b0`:
  the two top-of-function rolls (WarheadType `+0x17c/+0x180`, `+0x184/+0x188` → globals
  `DAT_0087f7ec/f0`) are **g_MainRng**; **all the scatter/debris draws** (VoxelAnim debris pick/count,
  AnimClass debris pick, two `RandomRanged(0,0x20)` shrapnel-bolt angles, `FUN_0049F420` explosion
  scatter) are **Scen->Random** (verified live, `decompile_function 0x004690b0`). The
  `weapon_spread_rng()→main` accessor's "warhead detonate scatter" intent comment is therefore
  **wrong** — detonate scatter is scenario; only the warhead-property rolls are main.
- **Bridge walker-variant = g_MapGenRng** (see §4.3 #3).

### 4.5 What drifts in the scenario/session layer

| Native (substrate) | Rust | Verdict |
|---|---|---|
| One `ScenarioClass` session object owns seed+RNG+identity+map+waypoints+flags | **No aggregate** — session state inline on the `Simulation` god-struct (`seed`, `scenario_rng`, `main_rng`, `tick`, `total_sim_ms`, `binary_frame`, `fog`, `houses`, `game_options`) interleaved with caches/queues; rest in app layer (`SkirmishLaunchSession`, `SkirmishScenarioRecord`, `AppState`, `ReplayHeader`) | **scattered** |
| Scenario identity (filename `+0x125C`, index `+0x34CC`) sim-resident | Map name lives only in `ReplayHeader`/`GameSnapshot`/`AppState.theater_name`; **the sim never records "which scenario"** | DRIFT (identity not sim-owned) |
| Map dims authoritative at load (`+0x112C..+0x1138`) | Derived **lazily** into `FogState.width/height` during the first vision tick (`vision/mod.rs:464`); sim has no dims pre-first-tick | DRIFT (lazy, zero-dim edge case) |
| MP start waypoints sim-resident (`+0x632`) | Full `Vec<Waypoint>` lives only in app-layer `SkirmishScenarioRecord`; only derived `base_center`/`waypoint_edge` survive per-house | DRIFT (table not sim-resident) |
| Frame counter is one global `g_CurrentFrameCounter` | Three loose fields `tick`/`total_sim_ms`/`binary_frame` (committed late — timing MATCH) | structural only |
| No state-hash on the session object | Rust `state_hash`/`world_hash` is a correct **internal** determinism tool (keep) — not a gamemd parity surface | OK (internal) |

---

## 5. The gamemd-native behavior contract (what the substrate must reproduce)

Reproduce the *outputs*, with clean Rust internals.

**C-RNG (the PRNG primitive):**
1. R(250,103) XOR step exactly (§2.1); `index_b` starts 103 ahead of `index_a` via seed `0x67`; wrap at 250.
2. Seed→state via the 4-round mixer with the **exact 8 table constants** (§2.3), 250 words, in order.
3. `RandomRanged` inclusive both ends; `low==high` consumes **no** draw; swap reversed; rejection mask
   `(1<<(msb+1))-1`; accept-first `<=span`; disabled→`low`/no-advance.

**C-STREAMS (the three instances):**
4. **Two gameplay streams** (`Scen->Random`, `g_MainRng`) start byte-identical from one `g_RngSeed`
   and diverge **only by consumption**; both are lockstep state and both must be hashed/serialized.
5. **Binding is per-callsite, hardcoded**, and a single function may draw from both — never collapse
   to a per-function or global rule. The verified routing table is §4.2 + §4.4.
6. **A third stream `g_MapGenRng`** exists, seeded from the *map seed* (not `g_RngSeed`), consumed by
   the random-map generator **and** the bridge walker-variant selector. It is deterministic across
   clients (same map seed) but is a **separate cursor** — model it, don't fold it into the others.

**C-SEED (the seed pipeline):**
7. Seed source by mode: SP/skirmish = local entropy (we are not bound to reproduce gamemd's exact
   entropy — only to reach the same `u32` eventually); MP = the handshaked `u32` travels **verbatim**
   (no scramble); replay = the `u32` from the recording header. One seed in ⇒ identical dual-stream
   state out, **before** any setup-phase or tick draw.
8. The seed is fixed by `Init_Random_Number_System`-equivalent **before** `Start_Scenario`-equivalent;
   setup-phase draws (start-position deficit fill, random-unit gen) are part of the lockstep draw order.

**C-SESSION (the ScenarioClass singleton):**
9. One session object owns: the persisted RNG stream, scenario identity (filename + index + IsRandom),
   map metadata (theater + dims + start count), MP/per-map waypoints + start-slot→house table, and the
   per-map flags (FogOfWar off, MultiplayerOnly, growth gates).
10. Lifecycle order (§2.5) is itself a contract: construct → seed → read identity/map → create houses →
    assign starts (RNG) → read units → post-map-init. RNG draw order across setup must match.
11. **No ScenarioClass state-hash** — do not invent one to match gamemd; the `+0xD64` block is the
    tactical view, render-layer only.
12. The frame counter is a single monotonic session clock reset at scenario load; the whole tick reads
    the **pre-increment** value (late increment) — already MATCH in Rust.

---

## 6. Rust-native replacement boundary

**Principle:** *Rust-native structure, gamemd-native semantics.* Do **not** port the 0x3740-byte
struct, the embedded-RandomClass layout, or the standalone-globals sprawl. Model the **behavior
contract** behind two small owners inside `sim/`.

```
                 ┌──────────────────────────────────────────────────────────┐
  app/net/launch │  SEED + SCENARIO DESCRIPTOR  (app layer → sim, one-way)   │
  (host/handshake│   negotiated u32 seed ; map id+dims+theater ; waypoints   │
   / replay file)│   ; game options ; IsRandom/mapgen seed                   │
                 └───────────────┬──────────────────────────────────────────┘
                                 │ injected once at construction (never reaches back up)
                                 ▼
   ┌────────────────────────────────────────────────────────────────────────┐
   │  ScenarioSession  (new sim/ aggregate — the ScenarioClass analog)        │
   │   seed:u32 ; scenario_id/map_name ; theater ; map_bounds(w,h)            │
   │   mp_start_waypoints:Vec<Cell> ; start_slot_house:[..] ; flags           │
   │   frame clock (tick / sim_ms / binary_frame)                             │  ← consolidate scattered fields
   │   owns ─────────────┐                                                    │
   └─────────────────────┼────────────────────────────────────────────────────┘
                         ▼
   ┌────────────────────────────────────────────────────────────────────────┐
   │  SessionRng  (the RNG substrate — already built, keep + extend)          │
   │   scenario_rng : SimRng   (gamemd Scen->Random)        ← built, faithful │
   │   main_rng     : SimRng   (gamemd g_MainRng)           ← built, faithful │
   │   mapgen_rng   : SimRng   (gamemd g_MapGenRng)         ← NEW: third stream│
   │   intent-named accessors own per-callsite routing (the routing record)   │
   │   all three seeded at construction, all three hashed + serialized        │
   └────────────────────────────────────────────────────────────────────────┘
```

**Two owners, clean responsibilities:**

- **`SessionRng`** = the existing `scenario_rng`/`main_rng` pair **plus a third `mapgen_rng`** for
  `g_MapGenRng`. Keep the intent-named accessors (they are the per-consumer routing record and the
  grep anchor); add `mapgen_*` accessors for bridge walker-variant + random-map. The math primitive
  (`SimRng`) is **unchanged** (already byte-exact). This is the *randomness* substrate.
- **`ScenarioSession`** = a new aggregate that gathers the session-scoped fields currently scattered on
  `Simulation` + the app layer: seed, scenario id/map name, theater, **authoritative** map bounds, the
  MP start-waypoint table + start-slot→house map, game options, IsRandom + mapgen seed, and the frame
  clock. It is fed once at construction by an **app-layer descriptor** (preserving `sim/ ⊥ net/ui`),
  and owns `SessionRng`. This is the *session identity/clock* substrate — the ScenarioClass analog.

**Decided shape:** keep `SimRng` as-is; keep the two existing streams and their accessors; the migration
is **additive** (third stream + a session aggregate + seed wiring), not a rewrite. The app→sim boundary
stays one-way: the net/launch layer computes the negotiated seed and descriptor and hands them to the
constructor; the sim never reaches up.

---

## 7. Ad hoc Rust logic to retire / demote

1. **`DEFAULT_SIM_SEED` as the de-facto match seed** (`mod.rs:75`, `Simulation::new()` at
   `app_init_helpers.rs:360`) — demoted to a *dev/test fallback only*; real matches must inject the
   negotiated seed via the `ScenarioSession` descriptor. Not deleted (tests use it).
2. **`ReplayHeader.seed = sim.seed` with no playback re-seed** (`app_sim_tick.rs:260`, `replay.rs`) —
   replace with: construct the sim from `header.seed` before `ReplayRunner::run`. The record-but-ignore
   path is retired.
3. **Map-name/identity duplicated across `ReplayHeader`/`GameSnapshot`/`AppState.theater_name`** — the
   sim-resident `ScenarioSession.scenario_id`/`map_name` becomes the single source; the header/snapshot
   copies derive from it instead of from `state.theater_name`.
4. **Lazy `FogState.width/height` as the map-dimension source** (`vision/mod.rs:464`) — demoted; map
   bounds become authoritative session metadata set at construction, eliminating the `FogState::default()`
   zero-dimension pre-first-tick edge case. `FogState` keeps a copy for the vision pass.
5. **`bridge_rng()` for the walker-variant draw** — re-route to the new `mapgen_rng` (g_MapGenRng), not
   the scenario stream. Collapse/debris draws stay on `bridge_rng`→scenario (correct).
6. **`particle_rng()→scenario` as the catch-all for *all* particle randomness** — keep for generic
   particle/smoke/gas/fire (correct), but rad-eruption/Gattling draws must route `main` when ported.
7. **`weapon_spread_rng()→main` intent comment "warhead detonate scatter"** — correct the comment;
   detonate scatter/debris is scenario. (No code consumer yet.)

**Not retired (correct as-is):** `SimRng` math/mask/seed-tables; the two-stream split + identical
seeding + `debug_assert_eq!`; hashing both streams in fixed order; serializing both; the 12 intent
accessors and their one-test-per-accessor routing guard; the late frame-counter commit.

---

## 8. Migration slices + acceptance tests

Sequenced lowest-risk first; each gated on a **full-skirmish replay state-hash regression** (unchanged,
or changed only in the documented parity-improving direction).

### Slice 0 — RNG primitive + two-stream split (DONE)
`SimRng` (R(250,103), byte-exact tables, rejection `RandomRanged`), `scenario_rng`+`main_rng` identical
seeding, both hashed/serialized, 12 accessors + routing tests, seed-1 value pins. *Baseline — complete.*

### Slice 1 — Wire the negotiated per-match seed (HIGH, structural)
Add `seed:u32` (or reuse the existing `seed` field) on the session and inject it from an app-layer
descriptor at construction; route SP (local entropy), MP (handshake `u32`), and replay (header `u32`)
through the same `with_seed`. Fix replay playback to construct from `header.seed`.
- **Acceptance:** `mp_sibling_rng_state_matches_after_seed_sync` (two headless clients, same seed →
  identical dual-stream state after 300 ticks; different seeds → diverge); `replay_reapplies_header_seed`
  (replay reconstructs the seed, not the default constant); `default_seed_only_in_tests`.

### Slice 2 — Model the third stream `g_MapGenRng` (`mapgen_rng`)
Add a third `SimRng` + `mapgen_*` accessors; re-route the bridge **walker-variant** draw from
`bridge_rng` to `mapgen_rng`; seed it from the map/mapgen seed (not `g_RngSeed`); hash + serialize it.
- **Acceptance:** `bridge_walker_variant_uses_mapgen_not_scenario` (drawing the walker variant advances
  `mapgen_rng`, leaves `scenario_rng`/`main_rng` untouched); `mapgen_seed_independent_of_game_seed`;
  hash regression (expect a documented change — bridge-repair walker variants shift to the correct stream).

### Slice 3 — `ScenarioSession` aggregate (consolidate scattered session state)
Introduce the `sim/` aggregate owning seed + `SessionRng` + scenario id/map name + theater +
authoritative map bounds + MP waypoint table + start-slot→house + options + frame clock; fed by an
app-layer descriptor at construction. Make map bounds authoritative (retire lazy `FogState` derivation).
Promote the MP start-waypoint table into the sim.
- **Acceptance:** `scenario_identity_is_sim_resident` (map name/index live in the sim, not re-derived
  from `theater_name`); `map_bounds_known_before_first_tick` (no zero-dim edge case); `mp_waypoints_round_trip`
  (full table survives snapshot/replay); pure-refactor hash-neutrality on a full skirmish.

### Slice 4 — Fix the latent routing DRIFTs as their consumers are ported
When radiation eruption, Gattling stage, and warhead detonate are implemented, route the **g_MainRng**
draws (rad-eruption offset/duration/intensity; Gattling stage; warhead-property rolls) to `main_rng`,
and the **Scen->Random** draws (detonate scatter/debris/shrapnel) to the scenario stream — per the §4.4
per-draw split. Add a `main`-routed rad/eruption accessor; do not funnel them through `particle_rng`.
- **Acceptance, per consumer:** `rad_eruption_draws_main_not_scenario`; `warhead_detonate_scatter_is_scenario_warhead_rolls_are_main`
  (per-draw routing matches §4.4); RNG-cursor parity test vs the gamemd draw sequence; behavior + hash regression.

### Slice 5 — Determinism hardening of the float draws (verify, then fix if needed)
Confirm `terrain_spawn` (`%1e6` f64) and `ore_growth` priority (f32 `total_cmp`) are bit-stable across
the AMD/Vulkan + headless/CI targets; if not, move to fixed-point (CLAUDE.md). Pin draw **counts per
tick**, not per wall-clock ms (survives the tick-rate effort).
- **Acceptance:** `terrain_spawn_probability_bit_stable_cross_target`; `ore_priority_ordering_deterministic`.

---

## 9. Open questions / deferred DRIFTs + doc-staleness fixed by this study

**Open / deferred:**
- **`g_MapGenRng` reachability beyond bridge repair** — confirm every live-skirmish consumer of the
  third stream (bridge walker-variant is verified; random-map gen is verified; any others?).
- **True projectile-spread X/Y binding** — `TechnoClass::Fire` (the *real* Fire function, not
  `WarheadTypeClass__Detonate 0x004690b0`) was **not** examined; its spread-X/Y stream is UNVERIFIED.
  Verify before porting weapon spread.
- **MP `DAT_00A8B8B8` positive-setter** — who sets the "seed already set" gate during connect (only
  zero-writers found; matches prior open question). Not blocking the seed contract.
- **`u64`→`u32` seed truncation** — confirm gamemd `g_RngSeed` is `u32` and discarding the high bits
  is intended (replay header stores the full `u64`).
- **Extreme-span `RandomRanged` early-out** (`rng.rs:150-152`) — unreachable in YR, kept flagged.

**Doc staleness this study corrects (binary is authority):**
- **`RNG_SYSTEM_GHIDRA_REPORT.md` §3.1/§3.2** lists **LightningStorm and particle generation under
  `g_MainRng`** — **WRONG**. Live disassembly this session: `LightningStorm__GroundStrike/Process`
  (6 sites) and `ParticleClass__Constructor`/`AI_Smoke` are **Scen->Random**; the only g_MainRng
  particle-adjacent consumer is `SpawnRadEruption`/`IncreaseGattlingStage`. §3.1 should be split:
  generic particles + lightning = Scen->Random; rad-eruption/Gattling = g_MainRng.
  (`PER_FRAME_RNG_CONSUMPTION_ORDER.md` carries the same stale "bridge collapse / lightning under
  g_MainRng" — also WRONG; bridge collapse is Scen->Random at all 11 callsites.)
- **`TWO_STREAM_RNG_DESIGN_20260529.md` §3** labels `0x004690b0` as **`TechnoClass::Fire`** and its
  g_MainRng draws as **"projectile spread X/Y"** — `0x004690b0` is **`WarheadTypeClass__Detonate`**
  and those two g_MainRng draws are **warhead-property rolls** (WarheadType `+0x17c..+0x188` →
  `DAT_0087f7ec/f0`), not spread X/Y; the function's actual scatter/debris draws are Scen->Random.
  The `weapon_spread_rng()` "warhead detonate scatter → main" routing is therefore mislabeled
  (detonate scatter is scenario). Latent (unported), but correct before weapon RNG lands.
- **`RNG_SYSTEM_GHIDRA_REPORT.md` §3.2 / multiple skirmish docs** assert a ScenarioClass **state-hash
  at `+0xD64/+0xD68`** — **REFUTED**: that block is the tactical-view scroll/camera position on
  `g_Tactical @ 0x00887324`, no hash computed. gamemd has no ScenarioClass state-hash.
- **RNG_SYSTEM §6** "no dual-stream design (highest-priority gap)" — **resolved**: the two-stream
  split is implemented, hashed, serialized, and routing-tested.

---

## 10. Sources

**Live Ghidra this session (gamemd.exe, read-only):**
- `decompile_function` — `0x0065C780` (Next), `0x0065C7E0` (RandomRanged), `0x0065C6D0` (Seed),
  `0x0065C660` (dead LCG), `0x0052FC20` (Init_Random_Number_System), `0x00598960` (random-map gen),
  `0x006832C0` (ScenarioClass ctor), `0x0052BA60` (CCFileClass ctor / allocator), `0x0052D9A0` (Main_Game),
  `0x00683AB0` (Start_Scenario), `0x00684620` (Read_Scenario), `0x00686730` (Read_Scenario_INI),
  `0x00686B20` (Full_Init), `0x00686890` (Post_Map_Init), `0x00689E90` (Read_INI_Basic),
  `0x00687F10` (Create_Houses), `0x00688380` (Gather_Start_Positions), `0x005EE9D0` (AssignStartingPoints),
  `0x006886B0` (Generate_Random_Units), `0x00683610` (ctor reset helper), `0x006d6000`/`0x006d6170`/
  `0x006d8640`/`0x006d8b30` (tactical-view scroll, refuting the "Scen state-hash"),
  `0x004690b0` (WarheadTypeClass__Detonate), `0x006fd800` (SpawnRadEruption), `0x0070de70` (Gattling).
- `disassemble_function` (ECX-binding reads) — `0x004F8440` (HouseClass__Update, mixed),
  `0x0051D0D0` (Infantry scatter), `0x00481180` (sub-cell), `0x00480cb0` (overlay/wall),
  `0x004415f0`/`0x00442d90` (smudge/survivors), `0x0047dd70`/`0x00575220…` (bridge collapse),
  `0x00722f00`/`0x0071C730` (ore/TIBTRE), `0x00423ac0` (anim), `0x0062b5e0`/`0x0062ed40` (particles),
  `0x0053a300`/`0x0053a6c0` (lightning storm), `0x004d9290` (Mission_Enter jitter),
  `0x00598030` (bridge walker-variant → g_MapGenRng), `0x0049f420` (detonate scatter helper).
- `read_memory` — `0x00839644`/`0x00839694` (seed tables), `0x00A8B230`/`0x00887324` (instance vs
  tactical pointers), ECX-load byte patterns at every routing callsite.
- `get_function_callers` / `get_xrefs_to` — `0x0052FC20`, `0x006832C0`, `0x005EE9D0`, `0x006886B0`,
  `0x00A8ED84`, `0x00A8ED94`, `0x00A8B8B8`, `0x00886B88`, `0x00ABE890`, `0x0065C660`.
- `list_globals` — `g_RngSeed`, `g_CurrentFrameCounter`, `g_GameMode`, `g_ScenarioClass_Instance`,
  `g_MainRng`, `g_MapGenRng`.

**Research docs digested:** `RNG_SYSTEM_GHIDRA_REPORT`, `RNG_MP_SEED_HANDSHAKE_AND_GAMEPLAY_INSTANCE`,
`RANDOM_RANDOMRANGED_0065C7E0`, `TWO_STREAM_RNG_DESIGN_20260529`, `TWO_RNG_STREAM_IMPLEMENTATION_CONTRACT_20260529`,
`SUBSTRATE_PARITY_LEDGER_20260529`, the `*_RNG_CLASSIFICATION_GHIDRA_REPORT` family (lightning/particle/
wall/smudge/ore-tiberium/scatter-bump/passenger-ejection/bridge), `BUILDINGCLASS_DAMAGE_FIRE_SELECTOR_RNG`,
`PER_FRAME_RNG_CONSUMPTION_ORDER`, `SCENARIO_INIT_DEEP_DIVE`, `LOGICCLASS_ENGINE_SUBSTRATE_SERVICE_STUDY`
(template), `SESSIONCLASS_GHIDRA_REPORT`.

**Rust source mapped:** `src/sim/rng.rs`, `src/sim/world/mod.rs` (Simulation 266, RNG fields 293-302,
accessors 527-540, with_seed 464-520, advance_tick), `src/sim/world/rng_routing_tests.rs`,
`src/sim/world/world_hash.rs:39-44`, `src/sim/snapshot.rs`, `src/sim/replay.rs`, `src/app_sim_tick.rs:256-263`,
`src/sim/world/world_spawn.rs`, `src/sim/vision/mod.rs:455-484`, `src/skirmish_launch.rs`,
`src/skirmish_scenarios.rs`, `src/app_skirmish.rs:162-275`, `src/app_init_helpers.rs:360`, `src/sim/game_options.rs`.
```
