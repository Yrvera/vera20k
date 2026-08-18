# Ore / Tiberium RNG Classification - Ghidra Research Report

**Address(es):** `0x0071C730`, `0x00483780`, `0x00487190`, `0x004838E0`, `0x00423AC0`, `0x0065C7E0`, `0x0065C780`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** RNG call classification for current Rust `src/sim/ore_growth.rs` and `src/sim/terrain_spawn.rs` against verified YR TIBTRE ore spawning and the verified AnimClass bouncer ore-spawn block.
**Non-Scope:** Full TiberiumClass queue scheduling, full natural ore growth/spread parity outside the RNG calls visible in the scoped Rust files, rendering of TIBTRE animation frames, and implementation patches.
**Confidence:** High for TIBTRE and AnimClass RNG bounds/order listed below; Medium for current Rust natural `ore_growth.rs` being non-gamemd because this report did not exhaust all TiberiumClass queue tick functions.
**Active in YR:** Yes. TIBTRE terrain spawning is active in standard YR skirmish; AnimClass bouncer ore spawning is active conditionally for meteor/crystal debris content.

## Target Question

Classify ore/tiberium RNG in current Rust against gamemd/YR behavior, focusing on `ore_growth.rs`, `terrain_spawn.rs`, `TIBTRE_ANIMCLASS_ORE_SPAWN_TICK_GHIDRA_REPORT.md`, and `TIBTRE_ORE_SPAWNING_GHIDRA_REPORT.md`.

Specifically:

- Which current calls match `RandomRanged(0,3)`?
- Which current calls match `RandomRanged(0,2)`?
- Which direction picks match YR?
- Which probability denominators match or drift?
- Which current calls are inferred/non-gamemd?

## Non-goals

- No Rust edits.
- No INI edits.
- No Ghidra mutation.
- No claim that natural ore growth/spread has been fully reverse-engineered.
- No attempt to merge TIBTRE and AnimClass bouncer ore spawning; they are separate systems.

## Evidence Needed To Mark COMPLETE

- Decompile `TerrainClass::AI @ 0x0071C730` and confirm TIBTRE probability/timing path.
- Decompile and assembly-check `CellClass::SpreadTiberium @ 0x00483780` for direction range, caller arguments, and placement handoff.
- Decompile and assembly-check `CellClass::PlaceTiberium @ 0x00487190` for variant RNG ranges and density handling.
- Decompile and assembly-check `AnimClass::AI @ 0x00423AC0` for `RandomRanged(0,3)` and `RandomRanged(0,2)`.
- Scan current Rust call sites in `src/sim/ore_growth.rs` and `src/sim/terrain_spawn.rs`.

## Stop Conditions

- Stop after all scoped Rust RNG calls are classified GREEN/YELLOW/RED.
- Stop if a candidate requires the broader TiberiumClass natural queue system; mark it deferred rather than expanding scope.
- Stop before implementation.

## 1. Overview

TIBTRE terrain objects in YR do not use `RandomRanged(0,3)` or `RandomRanged(0,2)`. Their live RNG path is: raw `Random::Next() % 1_000_000` for animation start probability, then later `CellClass::SpreadTiberium(true)`, which uses `RandomRanged(0,7)` for a random adjacent start direction, then `CellClass::PlaceTiberium(type, 3)`.

The `RandomRanged(0,3)` and `RandomRanged(0,2)` calls are live, but in the separate AnimClass bouncer/meteor/crystal ore-spawn block. Current Rust `ore_growth.rs` and `terrain_spawn.rs` do not implement that AnimClass path.

## 2. Class Layout / Key Offsets

| Field | Offset | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|
| TerrainType `AnimationProbability` | `+0x2A4` | Float probability checked against raw RNG modulo million | `TerrainClass::AI @ 0x0071C730`, `TerrainTypeClass::ReadINI @ 0x0071DEA0` | Yes |
| TerrainType `AnimationRate` | `+0x2A0` | CDTimer duration after animation start | `0x0071C79E..0x0071C7BF`, `0x0071DEA0` | Yes |
| TerrainType `SpawnsTiberium` | `+0x2B1` | Midpoint spawn gate | `0x0071C82F..0x0071C870`, `0x0071DEA0` | Yes |
| TerrainType `IsAnimated` | `+0x2B3` | Probability and midpoint spawn gate | `0x0071C745`, `0x0071C836`, `0x0071DEA0` | Yes |
| Cell `OverlayTypeIndex` | `+0x44` | Must be `-1` for `CanPlaceTiberium` true | `CellClass::CanPlaceTiberium @ 0x004838E0` | Yes |
| Cell `SlopeIndex` | `+0x11C` | Flat vs sloped overlay variant path | `0x00487190`, `0x004838E0` | Yes |
| Cell `OverlayData` | `+0x11E` | Density byte | `0x004872A0`, `0x00424155` | Yes |
| AnimType `TiberiumSpawnType` | `+0x338` | Overlay base for bouncer ore spawn | `AnimClass::AI @ 0x00423AC0`, `AnimTypeClass::ReadINI @ 0x00427D00` | Conditional |
| AnimType `TiberiumSpreadRadius` | `+0x33C` | Radius loop for bouncer ore spawn | `0x00423FF6..0x00424057`, `0x00427D00` | Conditional |
| AnimType `IsTiberium` | `+0x358` | Bouncer ore-spawn gate | `0x00423FD4`, `0x00427D00` | Conditional |

## 3. Core Logic

### 3.1 TIBTRE TerrainClass probability uses raw `Random::Next`, not `RandomRanged`

Verified binary facts:

- `TerrainClass::AI @ 0x0071C730` first calls `ObjectClass::AI`, then if `IsAnimated` and the animation timer field is zero, it calls `Random__Next @ 0x0065C780`.
- Assembly at `0x0071C755..0x0071C781`: loads `ScenarioClass+0x218`, calls `0x0065C780`, applies signed absolute-value transform, divides by `0xF4240` (`1_000_000`), and uses the remainder.
- The remainder is multiplied by double constant `0x007EF918` (`1.0e-6`) and compared with TerrainType `+0x2A4` (`AnimationProbability`).
- Active in YR: Yes. Stock `[TIBTRE01]`, `[TIBTRE02]`, `[TIBTRE03]` in `ini/rulesmd.ini` have `SpawnsTiberium=yes`, `IsAnimated=yes`, `AnimationRate=3`, `AnimationProbability=.003`.

Classification:

- Rust `terrain_spawn.rs:101` uses `rng.next_range_u32(1_000_000)`.
- With the new `SimRng`, this means `RandomRanged(0,999999)`, not raw `Random::Next() % 1_000_000`.
- It consumes at least one draw, but can consume additional rejection draws; gamemd consumes exactly one raw draw.
- Verdict: RED for parity and call-order risk, despite the denominator value being numerically correct.

### 3.2 TIBTRE spawn timing delays direction RNG until animation midpoint

Verified binary facts:

- After a successful probability roll, `TerrainClass::AI` stores current frame counter and loads `param_1[0x30]` from TerrainType `+0x2A0` (`AnimationRate`).
- The spawn only happens later when `CDTimerClass__GetTimeRemaining()` returns zero, the animation frame advances, and current frame equals half the SHP frame count.
- At the midpoint, the code resets current frame and timer to zero, gets the terrain cell, and calls `CellClass::SpreadTiberium(1)`.
- Active in YR: Yes.

Classification:

- Rust `terrain_spawn.rs:106..114` calls `try_spawn_ore` immediately on the probability hit.
- That means the direction draw happens in the hit tick rather than the future midpoint tick.
- Rust also allows a fresh probability roll on the next tick, while gamemd keeps the animation in progress until midpoint/reset.
- Verdict: RED for call order and spawn cadence.

### 3.3 TIBTRE adjacent direction pick is `RandomRanged(0,7)`

Verified binary facts:

- `CellClass::SpreadTiberium @ 0x00483780` with `force=true` defaults missing source tiberium type to index `0`.
- Assembly at `0x00483823..0x00483839`: `PUSH 0x7`, `PUSH 0x0`, `LEA ECX,[Scenario+0x218]`, `CALL 0x0065C7E0`.
- It then tries directions `(start + i) & 7` for `i=0..7`.
- Active in YR: Yes for TIBTRE midpoint spawn.

Classification:

- Rust `terrain_spawn.rs:130` uses `rng.next_range_u32(8)`, which maps to `RandomRanged(0,7)`.
- Rust `ore_growth.rs:306` also uses `rng.next_range_u32(8)`.
- Verdict: GREEN for the bound/inclusive semantics only. `terrain_spawn.rs` remains RED in context because the call occurs at the wrong time. `ore_growth.rs` remains YELLOW because this report did not prove its RA1-style natural spread loop is a YR queue equivalent.

### 3.4 TIBTRE placement into a new flat cell consumes `RandomRanged(0,11)`, not `RandomRanged(0,3)` or `(0,2)`

Verified binary facts:

- `SpreadTiberium` calls `CellClass::CanPlaceTiberium` before `PlaceTiberium`.
- Assembly at `0x0048389A..0x004838C5`: target cell in `ECX`, push tiberium class pointer, call `0x004838E0`, and only on true call `CellClass::PlaceTiberium(type, 3)`.
- `CanPlaceTiberium @ 0x004838E0` requires in-playfield, no `+0x140 & 0x500`, no blocking live building, no chain-reaction overlay occupant, land type permits tiberium, `Cell+0x44 == -1`, `Cell+0x11C == 0`, and tile permits tiberium.
- Therefore the TIBTRE `SpreadTiberium` placement path reaches `PlaceTiberium` only for an empty flat cell.
- In `CellClass::PlaceTiberium @ 0x00487190`, the empty-flat branch calls `RandomRanged(0,0xB)` to choose the overlay visual variant. Assembly: `PUSH 0xB`, `PUSH 0x0`, `CALL 0x0065C7E0` at `0x0048725C..0x00487266`.
- It then constructs the overlay and writes `OverlayData = param_3`, so TIBTRE density is exactly `3`.
- Active in YR: Yes for successful TIBTRE spread to an empty flat cell.

Classification:

- Rust `terrain_spawn.rs:146..151` and `place_tiberium_additive` do not consume `RandomRanged(0,11)` for new ore overlay variant selection. They place `default_ore_overlay_id` and derive frame from remaining stock.
- Rust `terrain_spawn.rs:181..184` allows existing ore as acceptable; binary `SpreadTiberium` prefilter requires `Cell+0x44 == -1`, so existing ore is not selected by the TIBTRE spread loop.
- Verdict: RED. Future TIBTRE parity must include the empty-cell visual variant draw and must not add to existing ore cells from this caller.

### 3.5 `RandomRanged(0,3)` and `RandomRanged(0,2)` belong to AnimClass bouncer ore spawn

Verified binary facts:

- `AnimClass::AI @ 0x00423AC0` contains a live bouncer/meteor ore-spawn block gated by AnimClass bouncer state, landing condition, `AnimType+0x358 IsTiberium`, candidate `CellClass::CanPlaceTiberium`, and non-null `AnimType+0x338 TiberiumSpawnType`.
- Assembly at `0x00424102..0x00424110`: `PUSH 0x3`, `PUSH 0x0`, `CALL 0x0065C7E0` selects one of four overlay variants from `TiberiumSpawnType`.
- Assembly at `0x00424146..0x00424155`: `PUSH 0x2`, `PUSH 0x0`, `CALL 0x0065C7E0`, then `MOV byte ptr [cell+0x11E], AL` sets random density `0..2`.
- Active in YR: Conditional. Standard content has `METDEBRI` and `CRYSTAL1..4` with `IsTiberium=true`; `CRYSTAL1..4` set `TiberiumSpreadRadius=0` and `TiberiumSpawnType=TIB2_01`; `METDEBRI` has `TiberiumSpawnType=TIB01`.

Classification:

- No scoped Rust call in `ore_growth.rs` or `terrain_spawn.rs` matches this path.
- These calls should not be added to TIBTRE terrain spawning. They require a separate AnimClass/bouncer ore-deposit implementation surface.
- Verdict: RED/missing for AnimClass bouncer ore spawning; negative fact for TIBTRE.

### 3.6 `ore_growth.rs` reservoir sampling is inferred/non-gamemd for YR

Current Rust facts:

- `ore_growth.rs:274..289` reservoir-samples growth/spread candidates using `rng.next_range_u32(seen)`.
- `ore_growth.rs:156..267` scans resource nodes incrementally by map position and executes collected growth/spread at cycle end.
- Module comments explicitly describe a proven RA1 algorithm, not verified YR `gamemd.exe` behavior.

Classification:

- No scoped Ghidra evidence ties this reservoir-sampling RNG stream to YR TiberiumClass growth/spread queues.
- The known YR functions here use per-cell queue-style `CellClass::GrowTiberium`, `CellClass::SpreadTiberium`, `CellClass::PlaceTiberium`, and `TiberiumClass__AddToGrowthQueue/AddToSpreadQueue` side effects rather than the Rust reservoir-sampling model.
- Verdict: YELLOW/RED. The `next_range_u32(seen)` bound may be a valid reservoir algorithm, but it is not a verified gamemd/YR RNG call in this scope.

## 4. INI Keys

| Key | Stock YR value / source | Binary consumer | RNG implication | Active in YR |
|---|---|---|---|---|
| `[TIBTRE01/02/03] SpawnsTiberium` | `yes`, `ini/rulesmd.ini:28111,28126,28141` | TerrainType `+0x2B1`, `TerrainClass::AI` midpoint gate | Enables `SpreadTiberium(true)` | Yes |
| `[TIBTRE01/02/03] IsAnimated` | `yes`, `ini/rulesmd.ini:28113,28128,28143` | TerrainType `+0x2B3` | Enables probability roll and midpoint spawn gate | Yes |
| `[TIBTRE01/02/03] AnimationRate` | `3`, `ini/rulesmd.ini:28119,28134,28149` | TerrainType `+0x2A0` | Delays spawn direction RNG until later animation midpoint | Yes |
| `[TIBTRE01/02/03] AnimationProbability` | `.003`, `ini/rulesmd.ini:28120,28135,28150` | TerrainType `+0x2A4` | Compared against raw modulo-million draw | Yes |
| `[METDEBRI] TiberiumSpawnType` | `TIB01`, `ini/artmd.ini:19121` | AnimType `+0x338` | Enables `RandomRanged(0,3)` variant path if landing cell passes | Conditional |
| `[CRYSTAL1..4] TiberiumSpreadRadius` | `0`, `ini/artmd.ini:19159,19179,19199,19219` | AnimType `+0x33C` | Radius loop only candidate center cell | Conditional |
| `[CRYSTAL1..4] TiberiumSpawnType` | `TIB2_01`, `ini/artmd.ini:19160,19180,19200,19220` | AnimType `+0x338` | Enables `RandomRanged(0,3)` variant path | Conditional |
| Anim `IsTiberium` | true on MET/CRYSTAL entries | AnimType `+0x358` | Gates bouncer ore block before `(0,3)` / `(0,2)` calls | Conditional |

## 5. Integration Points

TIBTRE path:

1. Game tick invokes `TerrainClass::AI`.
2. If not already animating, raw RNG modulo million may start animation.
3. CDTimer advances animation frames.
4. At half SHP frame count, `TerrainClass::AI` resets animation and calls `CellClass::SpreadTiberium(true)`.
5. `SpreadTiberium(true)` uses `RandomRanged(0,7)` once for random neighbor start.
6. First valid empty flat neighbor calls `CellClass::PlaceTiberium(type, 3)`.
7. Empty-flat `PlaceTiberium` uses `RandomRanged(0,11)` for visual overlay variant, then sets density byte to `3`.

AnimClass bouncer path:

1. `AnimClass::AI` runs for active animations.
2. Bouncer/meteor landing branch checks `IsTiberium` and radius candidates.
3. Each valid candidate places an overlay with `RandomRanged(0,3)` variant and `RandomRanged(0,2)` density.
4. This is not the TIBTRE path.

## 6. Current Rust Implementation Status

| Rust call site | Classification | Why |
|---|---|---|
| `src/sim/terrain_spawn.rs:101 rng.next_range_u32(1_000_000)` | RED | Correct denominator, wrong helper. gamemd uses raw `Random__Next` absolute/modulo one million; Rust uses `RandomRanged(0,999999)` with possible rejection redraws. |
| `src/sim/terrain_spawn.rs:106..114 immediate try_spawn_ore on hit` | RED | gamemd delays direction RNG until animation midpoint; Rust consumes direction RNG immediately. |
| `src/sim/terrain_spawn.rs:130 rng.next_range_u32(8)` | GREEN for bound, RED in context | Bound equals `RandomRanged(0,7)`, but call is at wrong tick in current Rust. |
| `src/sim/terrain_spawn.rs:181..184 accept existing ore` | RED | TIBTRE `SpreadTiberium` prefilters through `CanPlaceTiberium`, which requires empty overlay cell. |
| `src/sim/terrain_spawn.rs:193..229 place_tiberium_additive` | RED | Missing empty-cell `RandomRanged(0,11)` overlay variant draw; incorrectly additive for TIBTRE selected cell. |
| `src/sim/ore_growth.rs:285 rng.next_range_u32(seen)` | YELLOW/RED | Reservoir sampling is not verified against YR gamemd in this scope. |
| `src/sim/ore_growth.rs:306 rng.next_range_u32(8)` | GREEN for bound only | Matches `RandomRanged(0,7)` direction bound if modeling `SpreadTiberium`, but surrounding RA1-like scanner and placement are not verified YR. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TerrainClass::AI` TIBTRE probability | verified | `0x0071C730`, assembly `0x0071C755..0x0071C781` | none for scoped RNG |
| `TerrainClass::AI` midpoint spawn call | verified | `0x0071C7C2..0x0071C870` decompile | none for scoped RNG |
| `CellClass::SpreadTiberium(true)` direction draw | verified | `0x00483780`, assembly `0x00483823..0x00483839` | none |
| `CellClass::SpreadTiberium` candidate prefilter | verified | assembly `0x00483896..0x004838C5`; `CanPlaceTiberium @ 0x004838E0` | none |
| `CellClass::PlaceTiberium` empty-flat variant | verified | `0x00487190`, assembly `0x0048725C..0x00487266` | none |
| `AnimClass::AI` `(0,3)` / `(0,2)` calls | verified | `0x00423AC0`, assembly `0x00424102..0x00424155` | none for scoped RNG |
| `terrain_spawn.rs` current RNG calls | verified | `src/sim/terrain_spawn.rs:101,130` | implementation follow-up |
| `ore_growth.rs` current RNG calls | touched-not-exhausted | `src/sim/ore_growth.rs:285,306` | full YR TiberiumClass natural growth/spread queue investigation |
| Natural TiberiumClass queue scheduler | deferred | out of scope | investigate `TiberiumClass` queue tick/readers |

## 8. Open Questions - Final State

- `[RESOLVED] ORE-RNG-001 - Does TIBTRE probability use RandomRanged? -> No; it calls raw Random__Next and `% 1_000_000`.` (evidence: `0x0071C755..0x0071C781`)
- `[RESOLVED] ORE-RNG-002 - Does TIBTRE direction use RandomRanged(0,7)? -> Yes, once in SpreadTiberium before the neighbor loop.` (evidence: `0x00483823..0x00483839`)
- `[RESOLVED] ORE-RNG-003 - Does TIBTRE use RandomRanged(0,3)? -> No; `(0,3)` is in AnimClass bouncer ore spawn.` (evidence: `0x00424102..0x00424110`)
- `[RESOLVED] ORE-RNG-004 - Does TIBTRE use RandomRanged(0,2)? -> No; `(0,2)` is in AnimClass bouncer ore spawn density.` (evidence: `0x00424146..0x00424155`)
- `[RESOLVED] ORE-RNG-005 - Does TIBTRE add density to existing ore? -> The verified SpreadTiberium caller prefilters candidates with CanPlaceTiberium, which requires empty overlay cell, then calls PlaceTiberium(type,3).` (evidence: `0x00483896..0x004838C5`, `0x004838E0`)
- `[RESOLVED] ORE-RNG-006 - Does successful TIBTRE empty-cell placement consume a visual variant draw? -> Yes, RandomRanged(0,11) before OverlayClass construction.` (evidence: `0x0048725C..0x00487266`)
- `[RESOLVED] ORE-RNG-007 - Does current Rust terrain_spawn probability match? -> No; it uses next_range_u32(1_000_000), i.e. RandomRanged(0,999999), not raw modulo.` (evidence: `src/sim/terrain_spawn.rs:101`, `0x0071C755..0x0071C781`)
- `[RESOLVED] ORE-RNG-008 - Does current Rust terrain_spawn direction bound match? -> Yes for bound only: next_range_u32(8) equals RandomRanged(0,7).` (evidence: `src/sim/terrain_spawn.rs:130`, `0x00483823..0x00483839`)
- `[RESOLVED] ORE-RNG-009 - Does current Rust implement AnimClass bouncer ore `(0,3)/(0,2)`? -> No scoped call exists in ore_growth.rs or terrain_spawn.rs.` (evidence: `rg` scan; `src/sim/terrain_spawn.rs`, `src/sim/ore_growth.rs`)
- `[DEFERRED] ORE-RNG-010 - Does Rust ore_growth.rs match YR natural TiberiumClass growth/spread queue RNG?` (category: `out-of-scope`; reason: this report classified current calls but did not exhaust TiberiumClass queue tick functions; next-step-if-pursued: `/re-investigate YR TiberiumClass natural growth spread queues`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| TIBTRE probability consumes exactly one raw RNG draw, absolute/modulo `1_000_000`, not `RandomRanged`. | `0x0071C755..0x0071C781`; `Random__Next @ 0x0065C780` | mismatch | `src/sim/terrain_spawn.rs:101`; likely `SimRng` needs a raw-mod helper or local raw draw use | Preserve exact one-draw probability roll and denominator. | A deterministic terrain spawner with a seed whose `RandomRanged(0,999999)` rejects must not consume extra draws relative to gamemd. Proposed test: `test_tibtre_probability_uses_raw_modulo_one_million_one_draw`. | Do not use `next_range_u32(1_000_000)` for this probability gate. |
| TIBTRE direction draw is `RandomRanged(0,7)`, but only after animation reaches midpoint. | `0x00483823..0x00483839`; `TerrainClass::AI @ 0x0071C7C2..0x0071C870` | bound matches; timing mismatch | `src/sim/terrain_spawn.rs:72..115` and `TerrainSpawnerState` | Track animation-in-progress/rate/frame state so direction RNG occurs at midpoint, not hit tick. | With `AnimationRate=3`, a successful probability roll should consume no direction draw until midpoint; proposed test: `test_tibtre_direction_rng_delayed_until_animation_midpoint`. | Do not collapse visual animation timing into immediate spawn; it changes RNG stream and spawn cadence. |
| Successful TIBTRE spread selects an empty flat cell, then `PlaceTiberium(type,3)` consumes `RandomRanged(0,11)` for the overlay variant and sets density `3`. | `0x00483896..0x004838C5`; `0x0048725C..0x004872A0` | mismatch | `src/sim/terrain_spawn.rs:142..229` | Reject existing ore as a candidate for this caller; on empty-cell placement consume overlay variant draw and place the chosen variant with density 3. | A tree surrounded by existing ore and one empty valid cell should skip existing ore and draw one `(0,11)` variant only for the empty target; proposed test: `test_tibtre_spread_requires_empty_cell_and_draws_variant_0_11`. | Do not make TIBTRE additive on existing ore; additive `PlaceTiberium` branch is not reached through this prefiltered SpreadTiberium path. |
| AnimClass bouncer ore spawn uses `RandomRanged(0,3)` for variant and `RandomRanged(0,2)` for density. | `0x00424102..0x00424155` | missing in scoped Rust | new AnimClass/bouncer sim surface, not `terrain_spawn.rs` TIBTRE path | Implement separately when AnimClass bouncer/meteor/crystal ore deposition is modeled. | A METDEBRI/CRYSTAL landing candidate should consume `(0,3)` then `(0,2)` and write density 0..2; proposed test: `test_bouncer_tiberium_spawn_draws_variant_0_3_then_density_0_2`. | Do not graft `(0,3)` or `(0,2)` onto TIBTRE terrain spawning. |

### Negative Facts / Do Not Do

- Do not use `RandomRanged(0,999999)` for TIBTRE animation probability. Evidence: `TerrainClass::AI` calls `Random__Next @ 0x0065C780` and then `IDIV 0xF4240` at `0x0071C761..0x0071C771`.
- Do not spawn TIBTRE ore immediately on probability success. Evidence: `TerrainClass::AI` starts a timer at `0x0071C798..0x0071C7BF`; `SpreadTiberium` call is in the later midpoint branch.
- Do not add `RandomRanged(0,3)` or `RandomRanged(0,2)` to TIBTRE. Evidence: these calls are in `AnimClass::AI @ 0x00424102..0x00424155`, not `TerrainClass::AI` or `SpreadTiberium`.
- Do not treat TIBTRE spread as additive onto existing ore. Evidence: `SpreadTiberium` requires `CanPlaceTiberium` true before `PlaceTiberium`, and `CanPlaceTiberium` requires `Cell+0x44 == -1`.
- Do not remove the empty-cell overlay variant draw. Evidence: `PlaceTiberium` empty-flat branch calls `RandomRanged(0,0xB)` at `0x0048725C..0x00487266`.

### Remaining Uncertainty

- Full YR natural ore/tiberium growth/spread queue parity remains unverified for `ore_growth.rs`. The current reservoir-sampling RNG should be treated as inferred until a TiberiumClass queue investigation proves or replaces it.
- This report did not verify practical frequency of METDEBRI/CRYSTAL ore deposition; only the RNG bounds/order inside the live AnimClass block.

### Stale Docs / Follow-up Docs

- `docs/research/TIBTRE_ORE_SPAWNING_GHIDRA_REPORT.md` says `PlaceTiberium` "increases density by 3 if ore already exists" under `SpreadTiberium`. Replacement wording: `CellClass::PlaceTiberium has an additive branch for existing matching ore, but the verified TIBTRE SpreadTiberium caller prefilters candidates through CanPlaceTiberium, which requires an empty flat overlay cell; TIBTRE spread therefore does not intentionally choose existing ore cells in the verified path.`
- `src/sim/terrain_spawn.rs` comments say the two-phase model is collapsed because spawn-rate average is identical. Replacement wording for future Rust patch: `Do not collapse the two-phase TIBTRE animation model for parity; gamemd consumes the probability draw on animation start and the direction/placement draws only at animation midpoint, so collapse changes RNG order and cadence.`

## Sources

- Ghidra decompilation: `TerrainClass::AI @ 0x0071C730`
- Ghidra decompilation: `CellClass::SpreadTiberium @ 0x00483780`
- Ghidra decompilation: `CellClass::CanPlaceTiberium @ 0x004838E0`
- Ghidra decompilation: `CellClass::PlaceTiberium @ 0x00487190`
- Ghidra decompilation: `AnimClass::AI @ 0x00423AC0`
- Ghidra decompilation: `Random__RandomRanged @ 0x0065C7E0`
- Ghidra decompilation: `Random__Next @ 0x0065C780`
- Ghidra decompilation: `TerrainTypeClass::ReadINI @ 0x0071DEA0`
- Ghidra decompilation: `AnimTypeClass::ReadINI @ 0x00427D00`
- Assembly contexts: `0x0071C755..0x0071C781`, `0x00483823..0x00483839`, `0x00483896..0x004838C5`, `0x0048725C..0x00487266`, `0x00424102..0x00424155`
- `docs/research/TIBTRE_ORE_SPAWNING_GHIDRA_REPORT.md`
- `docs/research/TIBTRE_ANIMCLASS_ORE_SPAWN_TICK_GHIDRA_REPORT.md`
- `src/sim/terrain_spawn.rs`
- `src/sim/ore_growth.rs`
- `ini/rulesmd.ini`
- `ini/artmd.ini`
