# RMG Start Generation 0x00594B50 / 0x005A1FB0 - Ghidra Research Report

**Address(es):** `0x00598960`, `0x00594B50`, `0x00594870`, `0x00594F40`, `0x005A1FB0`, `0x0068BCC0`, `0x0068BF50`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** random-map generated start waypoint contract in `0x00598960`: liveness, retry-shaped caller loop, `0x00594B50` waypoint generation count, `0x005A1FB0` `.SED NumPlayers` consumer, and Rust metadata/test handoff.  
**Non-Scope:** full terrain, region, tiberium, bridge, hill, LAT, start-scoring, and passability formulas outside the generated-start count/success contract.  
**Confidence:** High for path liveness, caller loop shape, return semantics, `NumPlayers` loop bound, scenario waypoint storage offsets, and current Rust delta; Medium for the full meaning and lifecycle of `DAT_00ABE028` because this slot did not exhaust every UI/random-dialog caller.  
**Active in YR:** Conditional. Active in standard YR when a `.SED` random map is selected/launched; inactive for ordinary concrete map files.

## Working Notes

- Target question: In random-map launch generation, how do `0x00594B50` and `0x005A1FB0` decide generated start count, success/failure, retry behavior, relation to `.SED NumPlayers`, and later Skirmish spawn metadata?
- Non-goals: Do not investigate full terrain/region/tiberium formulas; do not reopen `.SED` key layout, `RandMap.img`, selected-map file priority, or selected-mode MCV placement.
- Evidence needed to mark COMPLETE: `.SED` path liveness into `0x00598960`; assembly/decompile for the `0x00594B50`/`0x005A1FB0` loop; direct evidence for `MapSeed+0x50` loop bound; direct evidence for scenario waypoint read/write offsets; return/success semantics for both callees; Rust scan and implementation handoff.
- Stop conditions: stop after the generated-start count and metadata contract is implementation-ready; defer lower-level candidate scoring/passability and full `DAT_00ABE028` UI lifecycle if they exceed this slice.

## 1. Overview

On the `.SED` random-map launch path, `ScenarioClass__Read_Scenario @ 0x00684960` calls the seed reader and then `FUN_00598960(0,0)` before `ScenarioClass__Post_Map_Init`. Inside `0x00598960`, the "Creating starting points" stage has a retry-shaped loop:

1. call `0x00594B50`;
2. if `AL == 0`, retry `0x00594B50`;
3. pass the `MapSeedClass` pointer in `ECX` to `0x005A1FB0`;
4. if `AL == 0`, retry from `0x00594B50`.

Active in YR: Yes, conditional on `.SED` random-map launch. Evidence: `ScenarioClass__Read_Scenario @ 0x00684960` decompile; assembly context `0x00598EAB..0x00598EBD`.

Important correction to prior shorthand: the caller is shaped like "loop until both return nonzero", but the scoped callee bodies observed here return success (`AL=1`) unconditionally on their normal exits. This means current static evidence does not prove a robust "try new starts until valid" runtime loop. It proves a defensive retry-shaped caller plus two callees that, in the inspected bodies, do not report failure for ordinary invalid/no-candidate outcomes.

## 2. Key State / Offsets

| State | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `MapSeed+0x50` | `.SED [RandomMap] NumPlayers`; used by `0x005A1FB0` as signed loop bound | `0x005A1FB9..0x005A1FC8`, `0x005A2384..0x005A238F`; prior reader evidence `0x00597B42..0x00597B5C` | Conditional |
| `DAT_00ABE028` | generated-start quota consumed by `0x00594B50`/`0x00594F40`; not read from `MapSeed+0x50` inside those functions | `0x00594DF0..0x00594E40`, `0x00594F7D`; retail byte scan found direct writes at `0x00596018` and `0x007A2494` | Conditional |
| `ScenarioClass+0x632 + index*4` | waypoint packed cell storage used for random start slots | readers/writer `0x0068BCC0`, `0x0068BF50` | Yes |
| `ScenarioClass+0x11C0..0x11DC` | generated start metadata mirror populated by `0x00594870` for written start slots | `0x00594870` decompile, writes `g_ScenarioClass_Instance + 0x11BC + (i+1)*4` | Conditional |
| `RMG scratch cell +0x3C` | start flood-fill ownership/index marker used by `0x005A1FB0` | `0x005A2000`, `0x005A2100..0x005A2200` decompile | Conditional |
| `RMG scratch cell +0x45` | byte marked during each start flood-fill expansion | `0x005A21B0..0x005A21CE` decompile | Conditional |

## 3. Path Liveness

Active in YR: Yes, conditional. The `.SED` branch in `ScenarioClass__Read_Scenario @ 0x00684960` tests `ScenarioClass+0x34BD`, calls `FUN_00597A10(local_210)`, then on success calls `FUN_00598960(0,0)` and `ScenarioClass__Post_Map_Init()`. This is the standard random-map launch path, not a TS-only or editor-only path.

Evidence: decompile `0x00684960`; random branch around `0x0068496B..0x00684990`.

Active in YR: Yes, conditional. `0x00598960` receives `MapSeedClass` in `ECX`, saves it in `EBP`/later `ESI`, uses `MapSeed+0x74` for RMG RNG setup, and reaches "RMG: Creating starting points" after cell-attribute recalculation. At the start stage, assembly shows:

- `0x00598EAB CALL 0x00594B50`
- `0x00598EB0 CMP AL,BL`
- `0x00598EB2 JZ 0x00598EAB`
- `0x00598EB4 MOV ECX,ESI`
- `0x00598EB6 CALL 0x005A1FB0`
- `0x00598EBB CMP AL,BL`
- `0x00598EBD JZ 0x00598EAB`

Evidence: Ghidra decompile `0x00598960`; Ghidra assembly context `0x00598E9E..0x00598EBF`.

## 4. `0x00594B50` Generates Waypoint Slots From `DAT_00ABE028`

Active in YR: Conditional. `0x00594B50` is the first random-map start-stage callee. It selects candidate regions/cells and calls `0x00594870` once per selected region with a running start index. It does not take the `MapSeedClass` pointer and does not read `MapSeed+0x50` directly.

Evidence: `0x00598960` calls `0x00594B50` with no setup argument at `0x00598EAB`; `0x00594B50` decompile has no parameter and reads `DAT_00ABE028`.

Active in YR: Conditional. `DAT_00ABE028` is the total quota that `0x00594B50` distributes across selected region buckets. The last selected bucket receives the remainder exactly:

- `ECX = [0x00ABE028]`
- each non-last bucket computes a rounded proportional count;
- last bucket assigns `DAT_00ABE028 - already_assigned`;
- each bucket count is stored at region object `+0x20`;
- `0x00594870` is called with the running start-index offset; that offset is incremented by the bucket count.

Evidence: `0x00594B50` decompile; retail/Ghidra-aligned disassembly around `0x00594DF0..0x00594E40` and call loop in `0x00594E50..0x00594EE3`.

Active in YR: Conditional. `0x00594870` writes concrete scenario waypoint slots. For each region quota `param_1[8]` it calls `FUN_0068BF50(index, candidate_cell)`, where `index` starts at the running offset supplied by `0x00594B50` and increments for each generated slot. It also marks the chosen `CellClass+0x140` bit `4` and mirrors the candidate into `ScenarioClass+0x11C0..`.

Evidence: `0x00594870` decompile; `0x0068BF50` writes `ScenarioClass+0x632 + index*4`; `0x0068BCC0` reads the same offset.

Active in YR: Conditional. `0x00594B50` returns `AL=1` on its observed exit regardless of whether all region/start candidates were accepted. Failure in nested selector `0x00594F40` can cause a bucket to write no starts, but `0x00594B50` still reaches `MOV AL,1; RET`.

Evidence: `0x00594B50` decompile returns `CONCAT31(...,1)`; assembly tail `0x00594F30 MOV AL,0x1; 0x00594F35 RET`; `0x00594870` branch where `0x00594F40` returns zero exits without waypoint writes.

## 5. `0x005A1FB0` Consumes `.SED NumPlayers` And Flood-Fills Around Existing Starts

Active in YR: Conditional. `0x005A1FB0` receives the `MapSeedClass` pointer in `ECX`. It reads `MapSeed+0x50` once at entry and uses a signed `> 0` guard before looping. Its outer loop increments `iVar13` and continues while `iVar13 < *(MapSeed+0x50)`.

Evidence: decompile `0x005A1FB0`; assembly `0x005A1FB9 MOV EAX,[ECX+0x50]`, `0x005A1FC2 TEST EAX,EAX`, `0x005A1FC8 JLE 0x005A2395`, and loop tail `0x005A2384..0x005A238F`.

Active in YR: Conditional. For each `i` in `0..NumPlayers-1`, `0x005A1FB0` reads existing waypoint slot `i` via `FUN_0068BCC0`, seeds a per-start frontier at that packed cell, marks `RMG scratch +0x3C` with `i+1`, and flood-fills up to 400 popped cells while checking map diagonal bounds, unclaimed scratch `+0x3C == 0`, and `CellClass__IsClearTile()`.

Evidence: `0x005A1FB0` decompile; waypoint read call `0x005A1FD8..0x005A1FDF`; initial scratch mark near `0x005A2000`; clear-tile branch and ownership mark near `0x005A2150..0x005A2220`; 400 cap in `local_3c < 400`.

Active in YR: Conditional. `0x005A1FB0` does not create the start waypoints itself. It reads waypoint slots produced earlier, then builds per-start clear-area/ownership scratch data. The only direct waypoint writer in this slice is `0x0068BF50`, reached from `0x00594870`.

Evidence: `0x005A1FB0` calls `0x0068BCC0` but not `0x0068BF50`; `0x00594870` calls `0x0068BF50`; `0x0068BCC0/0x0068BF50` offset bodies.

Active in YR: Conditional. `0x005A1FB0` also returns `AL=1` on the observed exit. If `MapSeed+0x50 <= 0`, it skips all per-start work and still returns success. Therefore no native static evidence in this slice supports "bad `NumPlayers` makes `0x005A1FB0` return failure and retry."

Evidence: assembly entry/exit `0x005A1FB9..0x005A1FC8` and `0x005A2395 MOV AL,0x1; RET`; decompile returns `1`.

## 6. Count Contract And `.SED NumPlayers` Relationship

Active in YR: Conditional. Native has two count-like inputs in this slice:

1. `DAT_00ABE028` is the waypoint-generation quota used by `0x00594B50` and `0x00594870`.
2. `MapSeed+0x50` / `.SED NumPlayers` is the post-generation per-start processing loop bound used by `0x005A1FB0`.

Evidence: `0x00594DF0..0x00594E40` reads `DAT_00ABE028`; `0x005A1FB9..0x005A238F` reads and loops on `MapSeed+0x50`.

Active in YR: Conditional. Standard Create Random Map setup initializes `DAT_00ABE028` to `4` in observed setup bytes. Direct writes found in the retail binary are the static/default setup write at `0x007A2494` and the dialog/preview helper write at `0x00596018`, both writing `4`; direct reads are in `0x00594B50` and `0x00594F40`. This supports the earlier Choose Map sentinel min/max `2..4` result, but this slot does not claim the full UI lifecycle for all network/custom paths.

Evidence: retail PE direct-reference scan for little-endian `28 E0 AB 00`; disassembly at `0x007A2494 MOV dword ptr [0xABE028],4` and `0x00596018 MOV dword ptr [0xABE028],EDI` with `EDI=4` from `0x00595FCC`; readers at `0x00594DF0` and `0x00594F7D`.

Active in YR: Conditional. Native-created `RandMap.Sed` values are expected to be normalized by dialog/setup before writing, but external `.SED` reader-side clamping remains unresolved from the prior layout report. For this slice, `0x005A1FB0` uses `MapSeed+0x50` as loaded; `0x00594B50` uses `DAT_00ABE028` as global state. If those diverge in a malformed/external `.SED` scenario, this slot does not prove a clean rejection path.

Evidence: prior `0x00597A30`/`0x005975E0` layout report; current `0x005A1FB0` decompile; `DAT_00ABE028` direct-reference scan.

## 7. Current Rust Implementation Status

Rust currently has no random-map generator and no generated-start metadata model:

- `src/skirmish_scenarios.rs` defines `RANDMAP_SED` and a `RandomMapSentinel`, but the sentinel has `multiplayer_start_waypoints = Vec::new()`, `min_players = None`, `max_players = None`, and `official = false`.
- `src/app_init.rs` routes launch map names through concrete map loading; there is no `.SED` random-generation branch.
- `src/app_skirmish.rs::assign_launch_starts` only consumes `MapFile.waypoints`; deficient starts are marked unsupported by current Rust, while random maps currently provide no waypoints at all.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `.SED` launch liveness into `0x00598960` | verified | `0x00684960`, `0x0068496B..0x00684990` | none for this slice |
| caller loop around `0x00594B50` / `0x005A1FB0` | verified | `0x00598EAB..0x00598EBD` | none |
| `0x00594B50` generated waypoint quota | verified | decompile; `0x00594DF0..0x00594E40` | full candidate scoring deferred |
| `0x00594870` scenario waypoint writes | verified | decompile; `0x0068BF50` | full candidate scoring deferred |
| `0x00594F40` selector failure behavior | touched-not-exhausted | decompile/disassembly shows zero return can make `0x00594870` skip writes | full scoring deferred |
| `0x005A1FB0` `NumPlayers` loop and flood-fill | verified | decompile; `0x005A1FB9..0x005A2395` | lower-level clear-tile semantics deferred |
| return semantics of scoped callees | verified | `0x00594F30`, `0x005A2395` | none |
| `DAT_00ABE028` full lifecycle | touched-not-exhausted | direct-reference scan; setup writes `4` | network/custom/UI lifecycle follow-up if needed |
| Rust random-map support | verified | focused `rg` and file reads | implementation not performed |

## 9. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is this path active in standard YR? -> Yes, conditionally for `.SED` random-map launch.` (evidence: `0x00684960`; `0x0068496B..0x00684990`)
- `[RESOLVED] OQ-02 - What is the caller loop shape? -> `0x00598960` retries from `0x00594B50` if either callee returns `AL==0`.` (evidence: `0x00598EAB..0x00598EBD`)
- `[RESOLVED] OQ-03 - Does `0x00594B50` read `.SED NumPlayers` directly? -> No; it has no `MapSeedClass` parameter and uses `DAT_00ABE028`.` (evidence: `0x00594B50` decompile; `0x00594DF0`)
- `[RESOLVED] OQ-04 - What writes generated scenario waypoint slots? -> `0x00594870` calls `0x0068BF50`, which writes `ScenarioClass+0x632+index*4`.` (evidence: `0x00594870`, `0x0068BF50`)
- `[RESOLVED] OQ-05 - How many slots does `0x00594B50` attempt to distribute? -> `DAT_00ABE028`, with the final bucket receiving the remainder.` (evidence: `0x00594DF0..0x00594E40`)
- `[RESOLVED] OQ-06 - Does `0x005A1FB0` use `.SED NumPlayers`? -> Yes, signed loop `i < MapSeed+0x50`.` (evidence: `0x005A1FB9..0x005A238F`)
- `[RESOLVED] OQ-07 - Does `0x005A1FB0` create waypoints? -> No; it reads existing waypoints and flood-fills scratch ownership around them.` (evidence: `0x005A1FD8`; no `0x0068BF50` call in `0x005A1FB0`)
- `[RESOLVED] OQ-08 - What do scoped callee returns mean? -> In observed bodies, both return `AL=1`; invalid/no-candidate cases do not visibly report failure to the caller.` (evidence: `0x00594F30`; `0x005A2395`)
- `[RESOLVED] OQ-09 - Does `NumPlayers<=0` fail? -> Not inside `0x005A1FB0`; it skips the loop and returns `1`.` (evidence: `0x005A1FC2..0x005A1FC8`, `0x005A2395`)
- `[RESOLVED] OQ-10 - Does Rust currently model this? -> No; only a sentinel exists and launch routes through concrete map loading.` (evidence: `src/skirmish_scenarios.rs`, `src/app_init.rs`, `src/app_skirmish.rs`)
- `[DEFERRED] OQ-11 - Exact candidate scoring/passability in `0x00594870` / `0x00594F40`.` (category: out-of-scope; reason: this slot only claims count/success/metadata contract; next-step-if-pursued: dedicated start candidate scoring report)
- `[DEFERRED] OQ-12 - Full `DAT_00ABE028` lifecycle across online/custom random-map paths.` (category: bounded-cost-too-high; reason: direct refs were identified, but this slot did not exhaust every UI/network caller; next-step-if-pursued: dedicated `DAT_00ABE028` lifecycle report)
- `[DEFERRED] OQ-13 - Malformed external `.SED` behavior when `MapSeed+0x50` diverges from `DAT_00ABE028`.` (category: needs-runtime-debugger; reason: static code suggests possible mismatch but not player-facing outcome; next-step-if-pursued: launch crafted `.SED` files)

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Random-map launch must produce concrete multiplayer waypoint slots before Skirmish spawn setup; waypoint slots are `ScenarioClass+0x632+index*4` written by `0x0068BF50` from `0x00594870`. | `0x00594870`, `0x0068BF50`, caller `0x00598EAB..0x00598EBD` | missing | future random-map generator/map metadata module; `src/app_init.rs`; `src/app_skirmish.rs` | Generated random map must expose real `MapFile.waypoints` / multiplayer start waypoints before `apply_skirmish_launch_session`. | Launch a generated random map and verify local+AI spawn assignment consumes generated waypoint slots instead of empty sentinel metadata. Proposed test: `skirmish_randmap_generation_populates_multiplayer_waypoints_before_spawn_setup` | Do not leave `RandomMapSentinel.multiplayer_start_waypoints` empty at launch time. |
| `0x005A1FB0` consumes `.SED NumPlayers` as a signed loop bound for per-start scratch/flood-fill processing, but the scoped body returns success even for `NumPlayers<=0`. | `0x005A1FB9..0x005A1FC8`, `0x005A2384..0x005A2395` | missing | `.SED` parser/validator and generator entry | Rust should validate/sanitize native-created random-map player counts at the seed/options layer, not assume the flood-fill helper rejects bad counts. | A native-created `.SED` with `NumPlayers=4` drives four per-start metadata passes; malformed `NumPlayers=0` is handled by explicit Rust policy, not by expecting native-style failure. Proposed test: `skirmish_randmap_numplayers_drives_start_metadata_passes` | Do not model `0x005A1FB0` return false as the malformed-count guard. |
| `0x00594B50` uses `DAT_00ABE028` for generated waypoint quota, while `0x005A1FB0` uses `MapSeed+0x50`; standard setup evidence initializes the quota global to `4`. | `0x00594DF0..0x00594E40`; `0x005A1FB9`; setup writes `0x007A2494` and `0x00596018` | missing | random-map seed/options model and generated-map metadata | Keep a distinct generated-start quota in the Rust random-map generator, at least for standard Create Random Map/offline sentinel behavior; do not collapse it blindly into `NumPlayers` without resolving the lifecycle. | Standard Create Random Map sentinel/generation exposes the native four-start map capacity while `.SED NumPlayers` still controls per-start processing. Proposed test: `skirmish_randmap_standard_setup_uses_four_generated_start_slots` | Do not claim `.SED NumPlayers` alone controls how many waypoint slots `0x00594B50` writes. |

### Negative Facts / Do Not Do

- Do not state that `0x00594B50` directly uses `.SED NumPlayers`; it uses `DAT_00ABE028`. Evidence: `0x00594DF0`, no `MapSeedClass` parameter.
- Do not state that the retry loop guarantees valid starts. The caller retries on zero return, but both scoped callee bodies return `AL=1` on observed exits. Evidence: `0x00598EAB..0x00598EBD`, `0x00594F30`, `0x005A2395`.
- Do not have `0x005A1FB0` create the waypoints in Rust. Native reads existing slots via `0x0068BCC0`; waypoint writes are through `0x00594870 -> 0x0068BF50`. Evidence: `0x005A1FD8`, `0x00594870`, `0x0068BF50`.
- Do not leave random-map launch metadata empty and rely on selected-map deficient-start fallback. Native has a generated-start stage before post-map init. Evidence: `0x00598EAB..0x00598EBF`.
- Do not silently treat malformed external `.SED NumPlayers` as proven native rejection. `0x005A1FB0` skips `<=0` and returns success. Evidence: `0x005A1FC2..0x005A1FC8`, `0x005A2395`.

### Remaining Uncertainty

- Exact region/candidate scoring and passability in `0x00594870` / `0x00594F40` remain deferred.
- Full `DAT_00ABE028` lifecycle across online/custom random-map paths remains deferred.
- Malformed external `.SED` behavior when `MapSeed+0x50` and `DAT_00ABE028` diverge needs runtime testing.

### Stale Docs / Follow-up Docs

Path: `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_RANDOM_MAP_GENERATOR_00598960_GHIDRA_REPORT.md`

Replace:

> `0x00598960` looping until `0x00594B50()` and `0x005A1FB0()` both return nonzero. `0x005A1FB0` loops `i < MapSeed+0x50`, reads/writes scenario waypoint slots through `FUN_0068BCC0/FUN_0068BF50`, and flood-fills clear tiles around each start.

with:

> `0x00598960` has a retry-shaped start stage that calls `0x00594B50`, retries if its `AL` is zero, then calls `0x005A1FB0(MapSeed)` and retries if that `AL` is zero. In the scoped callee bodies observed by `RMG_START_GENERATION_00594B50_005A1FB0_GHIDRA_REPORT.md`, both functions return `AL=1` on normal exits, so static evidence does not prove a robust retry-until-valid-starts loop. `0x00594B50`/`0x00594870` generate scenario waypoint slots using `DAT_00ABE028` as the quota; `0x005A1FB0` then loops `i < MapSeed+0x50` (`[RandomMap] NumPlayers`) and flood-fills scratch ownership around already-written waypoint slots read via `0x0068BCC0`.

Path: `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_RANDMAP_SED_WRITER_00597730_LAYOUT_GHIDRA_REPORT.md`

Add:

> `NumPlayers` is consumed by `0x005A1FB0` as the per-start flood-fill loop bound, but the waypoint-generation quota in `0x00594B50` is the separate global `DAT_00ABE028`; standard Create Random Map setup evidence initializes that quota to `4`. Do not claim `NumPlayers` alone controls the number of generated waypoint slots without resolving the remaining `DAT_00ABE028` lifecycle.

## 11. Addendum 2026-07-20 — Full `0x00594B50` / `0x005A1FB0` body decode (Task 9 implementation)

The "full candidate scoring deferred" gaps above are now closed. Everything below was
read live this session; citations inline.

### 11.1 `0x00594B50` head: threshold, zones, region rebuild

- **Survival threshold** = `ftol(max(genH × genW × 0.03, 400.0))`:
  `FILD [0x00ABE15C](genH); FIMUL [0x00ABE158](genW); FMUL [0x007ED8D0]=0.03;
  FLD [0x007ED8C8]=400.0; FCOMP` — the 400 floor wins only when strictly greater.
  Evidence: `disassemble_bytes 0x00594b50`, `read_memory 0x007ed8c8`.
- **Zone recompute + reference:** `CALL 0x0056C510` (ECX = MapClass) recomputes all
  zones and returns the base-zone id of the largest fill component;
  `zone_kind = {5,5,5,0,0}[DAT_00ABE014]` (Amphibious for map types 0–2, Normal for
  3–4); `reference = derived_table[kind][returned_id]`. Full zone subsystem:
  `RMG_ZONE_SUBSYSTEM_0056C510_GHIDRA_REPORT.md`.
- **Region rebuild:** `FUN_0058CE90` teardown (scratch `+0x38`/`+0x3C` → −1 across the
  full S² array, all regions freed, id counter `DAT_00ABED14` zeroed), then the cell
  iterator seeds `FUN_00594420` at every cell with scratch `+0x38 == −1`,
  `*(cell+0x4C) == 0` (class byte: plain clear ground) and
  `GetZoneID(cell, kind, 0) == reference`. `FUN_00594420` constructs a region object
  (ctor `0x0058BF70`) and LIFO-floods 8-directionally with claim-at-enqueue on
  `+0x3C`, pop-claim on `+0x38`; **each popped cell is appended to the region's
  `+0x2C` DynamicVector** — this is the `region.cells` filler the tiberium/starts
  gatherer reads. Neighbor gates: diamond band, `cell+0x4C == 0`, zone == reference,
  `+0x3C == −1`. Evidence: `decompile_function 0x00594420`, `0x0058ce90`.
- `FUN_005AC230` (the seed-scan "predicate") is only the diamond-band validity test.
  Evidence: `decompile_function 0x005ac230`.

### 11.2 Deletion, score, sort, quota split

- Regions with `count < threshold` are deleted iterating **downward** from the end
  (`0x00594C47..0x00594C80`).
- Score key per surviving region: `(500000 − count) × N + id`, where **N = the region
  count captured BEFORE the deletions** (EBX loaded at `0x00594C47`, never reloaded;
  `IMUL ESI,EBX` at `0x00594CE5`). Keys sort ascending via `qsort` with the plain
  signed compare at `0x005AD9C0` — descending size, unique ids as tiebreak.
- Quota split over the sorted buckets (`0x00594DCB..0x00594E52`): cumulative
  fractions, not per-bucket rounding: non-last bucket
  `q_i = ftol(cum_count/total × quota + 0.5 − assigned)` (`FADD [0x007E1738]=0.5`),
  last bucket `quota − assigned`. `quota = DAT_00ABE028`.
- Dispatch: `FUN_00594870(region, offset)` in sorted order, `offset += region+0x20`
  regardless of whether the bucket wrote waypoints (`0x00594E52..0x00594E80`).

### 11.3 Map-type-0 tech coin flips and the tail

- Map type 0 only (`0x00594E80..0x00594EE3`): each zero-quota region in bucket order
  draws once from the RMG RNG (`ECX=0xABE890`); if `draw × 2⁻³² < 0.5` (FCOMP against
  `[0x007E1738]`) it calls `FUN_00595400` (tech-building placement — the Task 10
  surface). The draw is consumed regardless of placement outcome.
- Tail `FUN_0058B820`: publishes preview metadata into ScenarioClass — per existing
  waypoint (sequential `FUN_0068BD80` probe, max 8): packed cell to `+0x11C0..`, iso
  pixel coords (`Center_Coord`→screen, `x/60`, `y/30`) to `+0x1140..`, plus playfield
  pixel mins/extents to `+0x112C..+0x1138` and the count to `+0x113C`. No RNG. Not yet
  modeled in Rust (emitter task owns it). Evidence: `decompile_function 0x0058b820`.

### 11.4 `0x005A1FB0` clearing floods — exact shape

Per start `i` in `0..MapSeed+0x50`:
1. Read waypoint slot `i` (`FUN_0068BCC0`) — **unconditionally**, even if no bucket
   wrote it (an unwritten slot feeds garbage; no defined output — the Rust port skips
   those with a comment).
2. Zero scratch `+0x3C` for every existing cell (a full iterator sweep before every
   start's flood — this is why the marker survives only within one flood).
3. Seed node `{cell, dist=0.0}`; scratch `+0x3C = i+1`; push into a 1-indexed f32
   min-heap (cap 800, insert only while `count+1 < 800`, strict-< sift — the same
   helper family as the water-blob heap; `FUN_005AD870` sift-down, `FUN_005AC960`
   pop).
4. Up to 400 pops: mark scratch `+0x45 = 1` on the popped cell (the protected flag
   the hill/patch phases honor), then for each of 8 directions: diamond band,
   `+0x3C == 0`, `CellClass__IsClearTile (0x00486380)` → node dist =
   `Sqrt_Approx(dx²+dy²)` **measured from the start waypoint** (f32 key;
   `disassemble_bytes 0x005a2220` — the subtrahend slots are the per-start seed
   coordinates; MEDIUM-HIGH confidence on the slot identification, the ring-growth
   seed key of 0.0 corroborates), stamp `+0x3C = i+1` (taken even when the full heap
   drops the node), heap-insert.

### 11.5 Implementation and open questions

Implemented in `src/map/rmg/phases/starts.rs` + `phases/zones.rs` +
`src/map/rmg/sqrt_approx.rs` (Task 9 of the 2026-07-20 terrain-phases plan), with
these documented divergences, all confined to native-undefined outputs:
- quota entries beyond the selector output and floods from unwritten waypoint slots
  read uninitialized native memory → Rust skips them. **Corrected 2026-07-25:** this
  is only half true, and the untrue half matters. The write loop at
  `0x00594A76..0x00594ADD` is bounded **solely** by the region quota (`region+0x20`,
  reloaded at `0x00594AD8`); it loads the selector array base at `0x00594A83
  MOV EAX,[EDI+4]` and indexes it at `0x00594A86 MOV ECX,[EAX+ESI*4]` without ever
  comparing against the selector's live count at `[EDI+0x10]` (the *following* drain
  loop at `0x00594AEA` does read `[EDI+0x10]` and is count-guarded — the asymmetry is
  in the binary). So: with **1 ≤ selectorCount < quota** the surplus indices read
  uninitialized heap *inside* the vector's own allocation (first push grows capacity
  to 10 dwords, quota ≤ 8) — nondeterministic, as stated. With **selectorCount == 0
  and quota > 0** there is no allocation at all: `0x00594F40` still returns a
  non-NULL vector object whose array pointer is `0` (ctor `0x0042FCB0` called with
  capacity 0 leaves `[+4]=0`; the NULL return — `0x00594FBB XOR EAX,EAX` /
  `0x00594FC3 RET` — is reached only through `0x00594FB7 CMP ESI,EDI` /
  `0x00594FB9 JNZ`, i.e. only when `quota == 0`), and the caller's only guard at
  `0x00594A21` tests the vector
  *object* pointer, not its count — so `0x00594A86` dereferences address `0` and the
  process takes an access violation. The native behavior in that sub-case is a crash,
  not garbage. Verified 2026-07-25: `disassemble_function 0x00594870`;
  `decompile_function 0x00594F40`; `decompile_function 0x0042FCB0`;
  `decompile_function 0x005657A0` (Get_CellClass is range-checked and returns the
  dummy cell `0x00ABDC50`, so the garbage-coordinate sub-case does not wild-write);
- the native 800-node scratch array in `0x005A1FB0` can overrun for pathological
  flood shapes (no bounds check) → Rust uses a growable list (heap cap kept at 800).

OPEN (flagged for the mode-3/4 and emit tasks):
- On map types 3/4 (kind Normal) the reference zone is the largest base component; if
  that is the ocean, its derived value is the shared "impassable" 1 and NO start
  regions can seed (class-0 cells always map to ids ≥ 2). Either the mode-3/4
  bridge/connector passes split the ocean before starts, or those types genuinely
  generate no starts — must be resolved when Task 10/mode-34 lands.
- Whether shoreline zone merging actually occurs depends on water/shore cell LEVELS
  at starts time (edges need |Δlevel| ≤ 1): the water-variant placement helper's
  level write (absolute TMP height vs additive) and the real WaterSet TMP height
  bytes must be re-verified when real TMP blocks are wired (Task 15).

### AUDIT_LOG

- 2026-07-25: §11.5 bullet 1 corrected — the "reads uninitialized native memory"
  shorthand hid a second, harder sub-case (selector count 0 with quota > 0 → NULL
  array pointer → access violation). Also re-confirmed from assembly that the
  `0x00598EAB`/`0x00598EB6` retry loop is unreachable: `disassemble_function
  0x00594B50` (single RET `0x00594F35`, `MOV AL,0x1` at `0x00594F30`) and
  `disassemble_function 0x005A1FB0` (single RET `0x005A239E`, `MOV AL,0x1` at
  `0x005A2396`). Starvation is unhandled, not retried.
- 2026-07-20: §2.3/§3.1/§5/§8 of the sibling scoring doc corrected (PavedRoadEnds
  mislabel, pave-polarity, constants; see that doc). This doc: added §11 full-body
  addendum. Verified live: `0x00594B50`, `0x00594420`, `0x0058CE90`, `0x0058C070`,
  `0x0058B820`, `0x00594F40`, `0x005A7250`, `0x005A1FB0`, `0x0056C510` (agent doc),
  `0x004CAC40` + table `0x008650BC`, `0x00578460`, `0x00578640`, `0x005AD9C0`.

## Sources

- Ghidra read-only decompile / assembly: `0x00684960`, `0x00598960`, `0x00594B50`, `0x00594870`, `0x00594F40`, `0x005A1FB0`, `0x0068BCC0`, `0x0068BF50`.
- Retail PE direct-reference scan for `DAT_00ABE028`: readers around `0x00594DF0`, `0x00594F7D`; setup writes around `0x00596018`, `0x007A2494`.
- Prior docs: `SKIRMISH_RANDOM_MAP_GENERATOR_00598960_GHIDRA_REPORT.md`, `SKIRMISH_RANDMAP_SED_WRITER_00597730_LAYOUT_GHIDRA_REPORT.md`.
- Rust scan: `src/skirmish_scenarios.rs`, `src/app_init.rs`, `src/app_skirmish.rs`.
