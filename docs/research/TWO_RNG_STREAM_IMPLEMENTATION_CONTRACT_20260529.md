# Two-RNG-Stream Implementation Contract - 2026-05-29

Status: RESEARCH/DESIGN ONLY. No Rust was edited producing this doc. It describes
required deltas; it does not apply them. Authority order: live `gamemd.exe` binary
(Ghidra MCP) > `docs/research/` > docs. Every binary claim below names its
verification call inline. Burden of proof defaults to DRIFT.

---

## Verified gamemd behavior - both streams

### Algorithm (shared by both gameplay streams)

**VERIFIED.** Both gameplay RNG streams are `RandomClass` instances sharing ONE
algorithm: an R(250,103) XOR lagged-Fibonacci generator (NOT an LCG). Per draw:
`state[index_a] ^= state[index_b]; result = state[index_a]; index_a++; index_b++`,
each index wrapping to 0 when it exceeds 249. If `disabled_flag != 0` the draw
returns 0 and does not advance. Layout: `disabled_flag@0` (u8), `index_a@4` (i32),
`index_b@8` (i32, lag seeded to 0x67=103), `state[250]@0xC`; total 0x3F4 bytes.

- Trace-cited via `decompile_function 0x0065C780` (Random__Next): XOR-lag body,
  index wrap at 0xF9, disabled-flag short-circuit as first statement.
- This session re-confirmed the Rust mirror is byte-exact at the algorithm level:
  `src/sim/rng.rs:98-118` (`next_u32`) implements the same XOR-lag + wrap, and the
  pinned seed=1 sequence test (`rng.rs:218-223`: `0x78B76ED5, 0x275D74AE,
  0xDA63B931`) holds. The defect is one instance vs two, not the math.

### Seeder + identical-seed proof

**VERIFIED this session** via `decompile_function 0x0052FC20`
(`Init_Random_Number_System`). The decompile shows, in BOTH the
SP-entropy branch (`g_GameMode==0 || g_GameMode==5`) and the MP/replay branch:

```
FUN_0065c6d0(DAT_00a8ed94);                      // seed Scen->Random
puVar4 = g_ScenarioClass_Instance + 0x218;        // copy 0xFD dwords (0x3F4 bytes)
...copy loop...
FUN_0065c6d0(DAT_00a8ed94);                      // re-seed from SAME DAT_00a8ed94
puVar4 = &DAT_00886b88;                            // copy 0xFD dwords into g_MainRng
...copy loop...
```

The same `DAT_00a8ed94` (`g_SeedU32`, 0x00A8ED94) is passed to the seeder
`FUN_0065c6d0` (`RandomClass_Seed`, 0x0065C6D0) TWICE. Therefore both streams
**start in byte-identical state** and diverge ONLY by independent consumption
order. The SP path mixes `GetSystemTime`/`GetTickCount` entropy into
`DAT_00a8ed94` *before* the dual seed; MP/replay skip entropy and use the
pre-set shared seed. Either way, both streams are seeded from one value.

- Seeder internals (index_a=0, index_b=0x67, 4-round per-word mixer over the two
  tables at 0x00839644 / 0x00839694): trace-cited via `decompile_function
  0x0065C6D0`; **VERIFIED** against the Rust mirror — `src/sim/rng.rs:11-12`
  declare `INIT_TABLE_1 = [0xBAA96887,0x1E17D32C,0x03BCDC3C,0x0F33D1B2]` and
  `INIT_TABLE_2 = [0x4B0F3B58,0xE874F0C3,0x6955C5A6,0x55A7CA46]`, byte-exact to
  the trace `read_memory` dumps of both table addresses.

### Stream 1 = g_MainRng

**VERIFIED.** Static BSS `RandomClass` at absolute `0x00886B88`. Dominant per-tick
stream (combat/damage, particles/effects, sound/voice/EVA, music, missile/bolt
jitter, several per-unit AI rolls, two of three `HouseClass::Update` rolls). Reached
by `MOV ECX,0x00886B88` (bytes `b9 88 6b 88 00`) before calling the draw primitive.

- Re-confirmed this session: `read_memory 0x004f887d` = `b9 88 6b 88 00` (MOV
  ECX,0x886B88) — the first HouseClass::Update roll. Confirms g_MainRng binding and
  the in-function stream split.

### Stream 2 = Scen->Random

**VERIFIED.** A `RandomClass` embedded inside the heap `ScenarioClass` at offset
`+0x218`, reached indirectly: load the ScenarioClass pointer from static slot
`0x00A8B230`, then `LEA/ADD ECX,+0x218` before the draw call. It is the ONLY
gameplay stream serialized with `ScenarioClass` save/replay state.

- Re-confirmed this session at two sites:
  - `read_memory 0x0051D29A` (InfantryClass::Scatter): `a1 30 b2 a8 00`
    (MOV EAX,[0x00A8B230]) / `6a 04` (PUSH 4) / `55` (PUSH EBP) /
    `8d 88 18 02 00 00` (LEA ECX,[EAX+0x218]) / `e8 21 f5 13 00` → 0x0065C7E0
    = `RandomRanged(0,4)` on Scen->Random.
  - `read_memory 0x004F88E8` (HouseClass::Update third roll): `a1 30 b2 a8 00` /
    `6a 02` / `55` / `8d 88 18 02 00 00` (LEA ECX,[EAX+0x218]) = `RandomRanged(0,2)`
    on Scen->Random. Confirms the same function consumes BOTH streams.

### Draw primitives + consumption

**VERIFIED** (from trace, mirror confirmed in Rust):

- `RandomRanged` (0x0065C7E0): inclusive on both ends; returns `low` with NO draw
  when `low==high`; swaps if `high<low`; rejection-samples masked `Random__Next`
  draws to `2^(msb+1)-1` retrying while masked > span. Consumes one OR MORE raw
  draws. Rust mirror: `next_range_u32_inclusive` (`rng.rs:131-162`), with the
  `leading_zeros` mask and the explicit pinned 3-draw rejection test
  (`rng.rs:226-250`).
- `Random__Next` (0x0065C780): single raw step, one draw. Rust mirror: `next_u32`.

### YELLOW - Unverified (do NOT mix into the verified body above)

- **Third RandomClass `g_MapGenRng` @ 0x00ABE890**: INFERRED (trace) to be the
  random-map generator's RNG, seeded from a map-seed (not `g_SeedU32`), consumed
  only by `FUN_00598960` map gen — NOT a per-tick sim stream. Not independently
  re-decompiled this session. Do NOT model it as one of the two gameplay streams.
  Action: leave out of scope; if random-map gen is implemented later, decode
  separately.
- **AnimClass bouncer/meteor debris raw `Random__Next` calls (~0x00422xxx)**:
  trace flagged as likely g_MainRng but NOT byte-confirmed per call. Treat the
  stream identity as UNVERIFIED until each is read at its load point. Do not route
  the corresponding Rust particle/bounce code without per-call verification.
- **AnimClass::Middle tiberium chain-reaction 1-in-3 `Random__Next()` gate
  (0x00424E…)**: the RandomRanged variant-pick at 0x00424E46 is Scen->Random
  (trace), but the preceding raw 1-in-3 gate's instance is UNVERIFIED.

---

## Per-caller stream assignment (live YR only)

Two tables. Stream identity is from the trace's byte-level `read_memory` at each
call's load point; the two spot-checks re-verified this session (Scatter,
HouseClass third roll) agree. AI sites are gamemd-Scen but DEFERRED in Rust (AI
not implemented) — listed for completeness so they route correctly when AI ships.

### Draw from Scen->Random (ScenarioClass+0x218)

| gamemd site (addr) | Subsystem | Draw |
|---|---|---|
| InfantryClass::Scatter 0x0051D2AC | infantry scatter direction | RandomRanged(0,4) |
| InfantryClass::Scatter 2nd 0x0051D36D | infantry scatter (alt path) | RandomRanged(0,4) |
| UnitClass scatter 0x00743DC5 | vehicle scatter jitter | RandomRanged(0,2)-1 |
| UnitClass scatter 0x00743D2B | vehicle scatter (secondary) | RandomRanged(1,4) |
| CellClass::PlaceInfantryInCell 0x0048139A | infantry sub-cell quadrant | RandomRanged(0,3) |
| HouseClass::Update 3rd roll 0x004F88FA | house cell-state roll | RandomRanged(0,2) |
| TerrainClass::AI TIBTRE 0x0071C761 | TIBTRE ore-tree spawn prob | raw Next %1e6 |
| CellClass::SpreadTiberium 0x00483839 | ore spread direction/variant | RandomRanged |
| CellClass::DestroyOverlay 0x00480cb0 | **wall damage** RandomRanged(0,Strength) | RandomRanged |
| AnimClass::Start 0x0042508D | scorch-vs-crater 50/50 | RandomRanged(0,0x7FFFFFFE) |
| AnimClass::Constructor 0x004221F5 | random anim start-frame/loop-delay | RandomRanged |
| AnimClass::Middle 0x00424E46 | tiberium chain-reaction variant | RandomRanged |
| BuildingClass::SpawnSurvivors 0x004432BF | per-cell survivor/smudge prob | RandomRanged(.,99) |
| BuildingClass::DestructionEffects 0x00441805/0x00441819 | center smudge (2 discard + 0..99) | RandomRanged |
| CellClass::BlowUpBridge 0x0047DE54 (+6 more) | bridge debris jitter | RandomRanged(0,0x7FFFFFFE) |
| MapClass::CollapseBridge_* (EW/NS Low/High, ~24 sites) | bridge-piece debris jitter | RandomRanged |
| HouseClass::AI_Choose_Building 0x004FE59E | AI build-choice (DEFERRED) | RandomRanged(0,99) |
| HouseClass::AI_ChooseNextProduction 0x00507759 | AI production (DEFERRED) | RandomRanged(1,EAX) |
| HouseClass::AI_* cluster (RecalcBuildOptions, Building_Strategy, UpdateEnemyThreatRatios, FindBestRallyTarget, EconomyStateMachine) | AI decisions (DEFERRED; pattern-classified, verify each before AI ships) | RandomRanged |

Two doc-drift corrections the trace resolved (both bind Scen->Random, contradicting
older docs): **bridge collapse** (PER_FRAME doc said g_MainRng — WRONG) and the
**building center-smudge discard rolls** (SMUDGE doc YELLOW — resolved to Scen).
The **wall-damage CellClass::DestroyOverlay** site is Scen->Random
(`disassemble_function 0x00480cb0` shows MOV EDX,[0x00A8B230]/LEA ECX,[EDX+0x218]),
NOT g_MainRng — PER_FRAME doc's YELLOW guess was wrong.

### Draw from g_MainRng (0x00886B88)

| gamemd site (addr) | Subsystem |
|---|---|
| WarheadTypeClass::Detonate 0x00469121 | warhead detonation spread |
| TechnoClass::ReceiveDamage 0x00702169 (+3) | damage response rolls |
| TechnoClass::IncreaseGattlingStage 0x0070dfc1 | gattling FX |
| TechnoClass::SpawnRadEruption 0x006fd8f1 (+4) | rad eruption particles |
| EBolt::DrawRecursiveBolt 0x004c21d1 (12) / EBolt::Init 0x004c2a9b | Tesla bolt jitter |
| LaserDrawClass::Draw 0x0055049a (+2) | laser beam wobble |
| RadBeam::DrawAndTickAll 0x006592de | rad beam wobble |
| FUN_00705860 0x00705900 | EMP/lightning bolt render jitter |
| BuildingClass::Mission_Missile 0x0044d4d4 | missile launch jitter |
| HouseClass::Update 1st+2nd 0x004f887d / 0x004f8895 | 2 of 3 per-house rolls |
| FootClass::AI 0x004daac0 | per-unit AI tick roll |
| InfantryClass::PerCellProcess 0x005197da / 0x00519842 | per-cell + infiltrate voice |
| UnitClass::PerCellProcess 0x0073a010 / 0x0073a07d | per-cell vehicle rolls |
| InfantryClass::UpdateIdleAction 0x0051cf8b | idle anim/facing/voice |
| FUN_006ffbe0 0x006ffd5b | unit voice-response selection |
| FUN_00708eb0 / FUN_00708fc0 / FUN_007090a0 | voice-set variant selection |
| FUN_007712c0 / FUN_007714b0 / FUN_00770c10 | EVA/speech variant selection |
| FUN_00720a80 0x00720ab5 | music theme next-track |
| SoundEvent::* cluster (0x004047e5..0x00405b14, 12 refs) | sound sample variant/playlist |
| FUN_004814f0 0x00481521 | one-time tile-variant table fill |
| bare g_MainRng loads 0x005dec42/0x004b5f7e/0x0040d9f2/0x007a4b86/0x007b161a/0x007b3f5f | voice/EVA/UI variant picks |

EXCLUDED from the live per-tick stream (not sim-tick draws): LAN-host seed mint
(CDFileClass::Constructor 0x005b8ae0 / 0x006981a6), random-map/skirmish-options
randomizers (FUN_00596300/00597260/00597380/00597430), and the two seeding WRITEs
in 0x0052fc20.

---

## Current Rust state

**VERIFIED this session.** The Rust port collapses both gamemd streams into ONE
cursor.

- `Simulation.rng: SimRng` — single field declared at `src/sim/world/mod.rs:289`,
  built once at `src/sim/world/mod.rs:459` (`rng: SimRng::new(seed)` inside
  `with_seed`). No second RNG field. The `rng.rs` module doc-comment
  (`src/sim/rng.rs:1-5`) explicitly states "one auditable stream."
- Every sim caller of `sim.rng` / `rng` draws from this one cursor regardless of
  whether gamemd would draw from g_MainRng or Scen->Random.

**Caller sites that currently use the single `rng` and must be re-classified**
(file:line, from this session's grep):

Scen->Random callers (must be re-routed to a new ScenRng):
- `src/sim/movement/scatter.rs:123` — `rng.next_range_u32(8)` infantry scatter
  start direction (Rust models scatter as an 8-neighbor scan seeded by one draw;
  gamemd draws RandomRanged(0,4) on Scen — the *stream* must match even though the
  internal direction mechanism differs).
- `src/sim/movement/bump_crush.rs:383` — `rng.next_range_u32(4)` sub-cell rotation
  (infantry placement quadrant; maps to CellClass::PlaceInfantryInCell Scen draw).
  Also review bump_crush.rs:740 (`next_range_u32(8)`) for scatter-direction origin.
- `src/sim/combat/smudge_dispatch.rs:239-241` — building center smudge: two
  discarded draws + `next_range_u32(100)` (BuildingClass::DestructionEffects, Scen).
- `src/sim/combat/smudge_dispatch.rs:212` — `rng_below_half_normalized`
  scorch/crater 50/50 (AnimClass::Start, Scen).
- `src/sim/combat/smudge_dispatch.rs:297` — `next_range_u32(100)` survivor-cell
  smudge (BuildingClass::SpawnSurvivors, Scen).
- `src/sim/combat/smudge_dispatch.rs:38` — `rng.next_u32()` smudge variant byte;
  verify against the matching gamemd site before routing (likely Scen, in the
  destruction-effects family — confirm load point).
- `src/sim/overlay_grid.rs:359` — `next_range_u32(flags.strength)` **wall damage**
  (CellClass::DestroyOverlay, Scen — corrected from the old g_MainRng guess).
- `src/sim/terrain_spawn.rs:78` — `raw_probability_sample(rng.next_u32())` TIBTRE
  spawn probability (TerrainClass::AI, Scen, raw Next).
- `src/sim/terrain_spawn.rs:382` — `next_range_u32(8)` ore spread start direction
  (CellClass::SpreadTiberium, Scen).
- `src/sim/terrain_spawn.rs:587` — `next_range_u32(ids.len())` spread variant pick
  (Scen).
- `src/sim/bridge_state/mod.rs:1335` and `walker.rs:420` — bridge collapse variant
  / debris (CellClass::BlowUpBridge + MapClass::CollapseBridge_*, Scen — corrected).
- `src/sim/world/bridge_orchestrator.rs:1203/1227/1228/1262/1294/1295/1419/1894-1900`
  — bridge collapse debris jitter, delay, strength roll, and the normalized X/Y
  draws (all bridge-collapse family → Scen).
- `src/sim/smudge_grid.rs:285` — `next_range_u32(filtered.len())` smudge variant
  pick; classify against its gamemd origin (smudge family → likely Scen; confirm).

g_MainRng callers (stay on the main cursor):
- `src/sim/particles/*` (fire.rs, gas.rs, smoke.rs, spawn.rs) — particle offsets.
- `src/sim/superweapon/lightning_storm.rs:190-213` — lightning bolt jitter/anim.
- `src/sim/passenger.rs:887/1042` — `next_u32() % 8` start dir (per-unit AI/voice
  family; confirm origin but defaults to main).
- `src/sim/production/production_sell.rs:394` — sell jitter (confirm origin).
- Combat/damage/voice draws as they are added.
- `src/sim/ore_growth.rs` draws: these are the ore-GROWTH queue priority/budget
  draws. NOTE: gamemd ore *spread* (CellClass::SpreadTiberium / TerrainClass::AI)
  is Scen; ore *growth* queue draws here must be matched to their specific gamemd
  origin before assignment — see Open Questions. Do NOT bulk-assign.

**World-hash / snapshot / replay touchpoints (VERIFIED this session):**
- `src/sim/world/world_hash.rs:39` — `self.rng.hash_state(&mut hasher)` folds only
  the single stream. A second stream's divergence would be invisible to desync
  detection until both are hashed.
- `src/sim/snapshot.rs:35` — `GameSnapshot.sim: Simulation` serializes the whole
  sim; `SimRng` derives Serialize/Deserialize (`rng.rs:15`), so a new field would
  auto-serialize, but it must be added explicitly to the struct AND
  `SNAPSHOT_VERSION` (currently `11`, `snapshot.rs:16`) bumped — load rejects on
  version mismatch (`snapshot.rs:109,122`).
- `src/sim/world/mod.rs:459` (`with_seed`) seeds the single cursor once.

---

## Required Rust deltas (minimum)

These describe the change; they are NOT applied here.

1. **Struct shape — two cursors.** In `src/sim/world/mod.rs` (~line 289), add a
   second field beside `rng`. Suggested: keep `pub rng: SimRng` as the g_MainRng
   stream and add `pub scen_rng: SimRng` as the Scen->Random stream. Document each
   in a `///` comment naming which gamemd instance it mirrors (g_MainRng 0x00886B88
   vs ScenarioClass+0x218). Do not rename `rng` (minimizes churn at main-stream call
   sites).

2. **Seeding — dual seed from one value.** In `with_seed` (`mod.rs:459`), seed BOTH
   from the same `seed`: `rng: SimRng::new(seed), scen_rng: SimRng::new(seed)`. This
   reproduces the gamemd dual-`RandomClass_Seed(DAT_00a8ed94)`: both start
   byte-identical, then diverge by independent draw order. Audit every other
   `Simulation` constructor / seed-reset path for the same dual-seed (grep
   `SimRng::new` and any `with_seed` callers).

3. **Draw API — no algorithm change, route by field, not by selector.** The draw
   primitives in `rng.rs` are correct and stay as-is. Routing is done by which field
   the caller borrows (`sim.rng` vs `sim.scen_rng`), NOT by adding a stream-enum
   parameter to `SimRng`. Helper systems that take `rng: &mut SimRng` keep that
   signature; the *caller* decides which field to pass. Where a function currently
   receives `&mut sim.rng`, change the call site to pass `&mut sim.scen_rng` for
   Scen consumers.

4. **Exact caller routing — pass `scen_rng` to these `src/sim/` sites:**
   - `src/sim/movement/scatter.rs:123`
   - `src/sim/movement/bump_crush.rs:383` (sub-cell rotation); review :740
   - `src/sim/combat/smudge_dispatch.rs:212, 239-241, 297` (and :38 after origin
     confirm)
   - `src/sim/overlay_grid.rs:359` (wall damage)
   - `src/sim/terrain_spawn.rs:78, 382, 587`
   - `src/sim/bridge_state/mod.rs:1335`, `src/sim/bridge_state/walker.rs:420`
   - `src/sim/world/bridge_orchestrator.rs:1203, 1227, 1228, 1262, 1294, 1295,
     1419, 1894-1900`
   - `src/sim/smudge_grid.rs:285` (after origin confirm)

   All OTHER current `sim.rng` consumers (particles, lightning_storm, combat/damage,
   voice/sound, the two HouseClass main rolls when AI ships) stay on `rng`. The
   ore_growth.rs draws need per-site origin confirmation before assignment (see
   Open Questions) — do not move them speculatively.

5. **World hash — fold BOTH streams.** In `src/sim/world/world_hash.rs` (~line 39),
   add `self.scen_rng.hash_state(&mut hasher)` immediately after the existing
   `self.rng.hash_state` so a divergence in either stream is visible to desync
   detection. Order matters for hash stability: append after `rng`, do not reorder.

6. **Snapshot — add field + version bump.** Adding `scen_rng` to `Simulation`
   auto-serializes via the existing derive, but bump `SNAPSHOT_VERSION` 11 → 12 in
   `src/sim/snapshot.rs:16`. Old saves (v11) will be rejected on load by the existing
   version guard — that is correct (a v11 save has no second stream and would
   desync).

---

## Determinism / parity risk

- **World hash changes for every game.** Folding `scen_rng` into the hash
  (`world_hash.rs`) changes `state_hash()` output even before any draw is rerouted,
  because a second 0x3F4-byte state is now hashed. Every existing replay/desync
  fixture that pins a hash value will mismatch and must be regenerated.
- **Existing replays WILL mismatch — and that is correct.** Rerouting scatter,
  smudge, wall-damage, TIBTRE/ore-spread, and bridge-collapse draws to a separate
  cursor changes the value each of those callers observes AND changes how the main
  cursor advances (those draws no longer consume from it). Both streams now advance
  in gamemd's actual interleave. Any replay recorded under the single-stream engine
  encodes the wrong draw sequence and will diverge. This is a **corrective drift
  toward retail**: the new behavior matches gamemd's two-stream consumption; the old
  single-stream behavior was the bug. Bump the snapshot/replay version, regenerate
  fixtures, and treat the break as intentional.
- **Cross-stream interleave is the whole point.** Because `HouseClass::Update`
  consumes both streams within one function (two g_MainRng + one Scen), and ore
  spread/growth interleaves Scen and main draws across a tick, getting EITHER the
  per-caller routing OR the tick draw-order wrong reproduces a different sequence.
  Routing fixes stream identity; it does NOT fix intra-tick draw ORDER.
- **Separate, compounding parity axis (out of scope here, flag for follow-up):**
  the trace notes Rust runs ore growth/spread in Phase 7 (after combat), whereas
  gamemd runs orders 9-10 before the object vector. That changes intra-stream draw
  order independently of stream identity. Two-stream split + wrong phase order
  compound; fixing the split alone will not be byte-exact if the phase order is also
  off. Verify tick order in a follow-up.

---

## Acceptance tests

1. **Identical-seed start (both streams).** New seed N: assert
   `sim.rng.state() == sim.scen_rng.state()` immediately after `with_seed(N)` and
   before any draw — proves the dual-seed produces byte-identical initial state.
2. **Per-stream draw-sequence fixtures.** Pin the first K raw draws of each stream
   independently for a fixed seed (extend the existing seed=1 pin in `rng.rs:218`).
   Drive a fixture scenario that triggers a known Scen consumer (e.g. infantry
   scatter) and a known main consumer (e.g. a particle spawn) and assert each
   advanced its own cursor by the expected count while the other is untouched.
3. **Stream-isolation cross-check (per routed caller).** For each rerouted site
   (scatter, sub-cell rotation, center/survivor smudge, scorch/crater, wall damage,
   TIBTRE spawn, ore spread, bridge collapse): snapshot both `rng.state()` and
   `scen_rng.state()` before the action, run it, and assert ONLY `scen_rng`
   advanced. Symmetric test for a main-stream caller asserting ONLY `rng` advanced.
   This is the concrete proof that each caller consumes from the correct stream.
4. **Replay reproducibility.** Record a short fixture match, snapshot at tick T,
   restore, replay to T again, assert identical `state_hash()` (now folding both
   streams). Then assert the v11→v12 version guard rejects a stored v11 save.
5. **World-hash sensitivity.** Force a single extra draw on `scen_rng` only; assert
   `state_hash()` changes. Same for `rng`. Proves both streams are folded into the
   hash (guards against silently hashing only one).

---

## Open questions / blockers (must resolve before implementation)

1. **ore_growth.rs draw origins (BLOCKER for those sites).** The ore-GROWTH queue
   priority/budget/spiral draws in `src/sim/ore_growth.rs` (lines 363, 435, 525,
   559, 583, 693, 863, 1192, 1274, 1463, 1499) are NOT mapped to a verified gamemd
   site in the trace. gamemd ore *spread* (CellClass::SpreadTiberium, TerrainClass::
   AI) is Scen, but the growth-queue mechanism is a Rust-side construct — each draw
   must be traced to its gamemd origin (Scen vs main) before routing. Do not
   bulk-assign. Resolve via `/re-investigate` on ore growth+spread RNG.
2. **smudge_dispatch.rs:38 and smudge_grid.rs:285 origins.** Confirm the matching
   gamemd load point (smudge-variant byte and variant pick) binds Scen vs main
   before routing. Likely Scen (destruction-effects family) but UNVERIFIED.
3. **bump_crush.rs:740 / passenger.rs:887,1042 / production_sell.rs:394 origins.**
   Each `next_range_u32(8)` / `next_u32() % 8` / sell-jitter draw needs its gamemd
   site identified before deciding the stream. Defaulting to main is a guess, not a
   verification.
4. **AnimClass bouncer raw Next + chain-reaction 1-in-3 gate (YELLOW above).**
   Stream identity UNVERIFIED; do not route any corresponding Rust code until
   byte-confirmed at the load point.
5. **AI cluster (deferred, not a current blocker).** When AI ships, each
   HouseClass::AI_* site must be byte-verified (most are pattern-classified, not
   individually read) before routing to Scen.
6. **Tick draw-ORDER parity (separate axis).** Stream split does not fix the Phase 7
   vs orders 9-10 ore-phase ordering disparity. Confirm whether that must land in the
   same change or as a follow-up — byte-exact replay parity needs both.
7. **All Simulation seed paths.** Confirm `with_seed` is the only seeding entry; any
   other constructor or seed-reset must dual-seed too.
