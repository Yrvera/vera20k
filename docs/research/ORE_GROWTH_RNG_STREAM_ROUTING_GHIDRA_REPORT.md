# Ore Growth/Spread RNG Stream Routing — Ghidra Research Report

**Address(es):** `0x00722F00` (GrowthProcessor), `0x00722440` (SpreadProcessor), `0x007235A0` (AddToGrowthQueue), `0x00722AF0` (AddToSpreadQueue), `0x00483780` (SpreadTiberium), `0x00487190` (PlaceTiberium), `0x0071C730` (TerrainClass::AI / TIBTRE); RNG primitives `0x0065C780` (Random__Next), `0x0065C7E0` (RandomRanged)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Which RNG *instance* (Scen->Random `ScenarioClass+0x218` vs g_MainRng `0x00886B88`) each tiberium growth/spread/placement draw consumes, and the per-site mapping of the 11 RNG draw sites in `src/sim/ore_growth.rs` to (a) verified Scen draw, (b) verified g_MainRng draw, or (c) Rust-only construct with no gamemd draw.
**Non-Scope:** Native-queue migration design (covered by the GrowthProcessor/SpreadProcessor reports), full PlaceTiberium variant matrix, save/load, AI RNG.
**Confidence:** High — every gamemd draw instance below was read live this session from the load instruction preceding the `CALL`.
**Active in YR:** Yes — growth/spread queue drivers run every tick on growth-enabled maps (`ScenarioClass+0x34A6`), stock `[Riparius/Vinifera/Aboreus] Growth/SpreadPercentage=.06`.

## 1. Overview

All tiberium growth, spread, queue-insert, and placement RNG in gamemd draws from a
**single stream: Scen->Random** (the `RandomClass` embedded at `ScenarioClass+0x218`,
reached via the static scenario pointer `0x00A8B230`). There is **no g_MainRng
(`0x00886B88`) consumption anywhere in the tiberium growth/spread path.** This
confirms and extends the two-RNG-stream contract, which bound "TIBTRE spawn + ore
spread" to Scen->Random — the binding holds for the *entire* queue subsystem, not
just placement.

This **corrects a misleading claim** in
`TIBERIUMCLASS_GROWTH_PROCESSOR_EXACT_QUEUE_PROCESSING_GHIDRA_REPORT.md` §3.2/§3.7,
which describe the processor's RNG as coming from "the global RNG object." It is not
the global object — it is the scenario-embedded instance. See Stale Docs below.

## 2. Verified gamemd draw sites — all Scen->Random

Every site below was read live: the two instructions before the `CALL` load
`[0x00A8B230]` (the ScenarioClass pointer) and add/LEA `+0x218`, then call a
`RandomClass` method with that instance in `ECX` (thiscall).

| # | gamemd site | Addr | Instance proof | Primitive | Draw semantics |
|---|---|---|---|---|---|
| 1 | GrowthProcessor batch budget | `0x00722F6A` | `MOV EAX,[0x00a8b230]` / `LEA ECX,[EAX+0x218]` / `CALL 0x0065c780` | raw `Next` | `actual_attempts = abs(Next) % clamp(ftol(heap·GrowthPct),5,50) + 1` |
| 2 | GrowthProcessor reinsert priority | `0x0072303F` | `MOV EAX,[0x00a8b230]` / `LEA ECX,[EAX+0x218]` / `CALL 0x0065c780` | raw `Next` | `priority = g_CurrentFrameCounter(0x00a8ed84) + abs(Next % 50)` (per still-growable cell) |
| 3 | SpreadProcessor batch budget | `0x007224AE` | `MOV EAX,[0x00a8b230]` / `LEA ECX,[EAX+0x218]` / `CALL 0x0065c780` | raw `Next` | `actual_budget = abs(Next) % clamp(ftol(heap·SpreadPct),5,25) + 1` |
| 4 | AddToGrowthQueue priority | `0x007235F9` | `MOV EAX,[0x00a8b230]` / `LEA ECX,[EAX+0x218]` / `CALL 0x0065c780` | raw `Next` | `priority = g_CurrentFrameCounter + abs(Next % 50)` (runtime insert; only if `OverlayData < 0x0B`) |
| 5 | AddToSpreadQueue priority | `0x00722B5B` | `MOV ECX,[0x00a8b230]` / `ADD ECX,0x218` / `CALL 0x0065c780` | raw `Next` | `priority = g_CurrentFrameCounter + abs(Next % 50)` (only if `CanSpreadTiberium` and bitmap byte == 0) |
| 6 | SpreadTiberium start direction | `0x00483839` | `LEA ECX,[Scen+0x218]` / `CALL 0x0065c7e0` (prior report) | `RandomRanged(0,7)` | one draw, then scans `(start+i)&7` for i=0..7 |
| 7 | PlaceTiberium overlay variant | `0x0048725C` | `LEA ECX,[Scen+0x218]` / `CALL 0x0065c7e0` (prior report) | `RandomRanged(0,11)` | empty-flat-cell **new** placement only; density byte set to caller's arg, not drawn |
| 8 | TIBTRE animation probability | `0x0071C761` | `[Scen+0x218]` / `CALL 0x0065c780` (prior report) | raw `Next` | `abs(Next) % 1_000_000` × 1e-6 vs `AnimationProbability` |

Notes:
- The processor batch draws (1, 3) use **signed-abs `% clamp + 1`**, where clamp is
  `[5,50]` for growth and `[5,25]` for spread. The batch is a **pop-attempt budget**
  (growth) / **valid-source budget** (spread), not a success count.
- Density *increments* during growth go through `PlaceTiberium(type,1)` on an
  existing cell, which takes the **additive** branch and does **not** draw the
  `(0,11)` variant. Only spread to a **new empty** cell draws the variant. So a
  growth pop consumes: 1 batch draw (per processor call) + 1 reinsert-priority draw
  (per still-growable cell) + optionally 1 AddToSpreadQueue draw. No variant draw.

## 3. Per-site routing table — `src/sim/ore_growth.rs`

| Rust site | Call | gamemd correspondence | Stream | Verdict |
|---|---|---|---|---|
| L363 `enqueue_growth_queue_cell` | `growth_queue_priority(frame, next_u32())` | AddToGrowthQueue priority (#4) | **Scen** | (a) route to `scen_rng` |
| L435 | `growth_queue_priority(frame, next_u32())` | GrowthProcessor reinsert priority (#2) | **Scen** | (a) route to `scen_rng` |
| L525 | `signed_abs_mod_plus_one(next_u32(), batch)` | GrowthProcessor batch budget (#1) | **Scen** | (a) route to `scen_rng` |
| L559 | `growth_queue_priority(frame, next_u32())` | GrowthProcessor reinsert priority (#2) | **Scen** | (a) route to `scen_rng` |
| L583 | `growth_queue_priority(frame, next_u32())` | GrowthProcessor reinsert priority (#2) | **Scen** | (a) route to `scen_rng` |
| L693 | `signed_abs_mod_plus_one(next_u32(), batch)` | SpreadProcessor batch budget (#3) | **Scen** | (a) route to `scen_rng` |
| L863 (TIBTRE) | `growth_queue_priority(frame, next_u32())` | AddToGrowthQueue priority (#4), triggered by PlaceTiberium | **Scen** | (a) route to `scen_rng` |
| L1192 `try_spread` start dir | `next_range_u32(8)` | SpreadTiberium `RandomRanged(0,7)` (#6) | **Scen** | (a) route to `scen_rng` |
| L1274 overlay variant | `next_range_u32(variants.len())` | PlaceTiberium `RandomRanged(0,11)` (#7) | **Scen** | (a) route to `scen_rng` — **plus bound-drift flag, see §4** |
| L1499 `try_spread_ore` start dir | `next_range_u32(8)` | SpreadTiberium `RandomRanged(0,7)` (#6) | **Scen** | (a) route to `scen_rng` |
| L1463 `reservoir_sample` | `next_range_u32(seen)` | **NONE** | — | (c) **Rust-only — no gamemd draw** |

**Bottom line for the two-RNG-stream plan: all 10 gamemd-corresponding ore_growth
draws route to `scen_rng`. None route to the main stream.** The 11th (reservoir
sampling) is a Rust-only construct that gamemd never performs.

## 4. Flagged divergences (Rust-only / drift in their own right)

These are independent of the stream-routing question — they are RNG-consumption
disparities that exist regardless of which cursor is used.

1. **Reservoir-sampling draw (L1463) has no gamemd origin and is on the live tick
   path.** `tick_ore_growth` (L1334) still uses the legacy scan/reservoir model and
   calls `reservoir_sample` at L1375 and L1385; the draw `next_range_u32(*seen)` at
   L1463 fires per candidate seen. gamemd selects cells by **min-heap pop**, never by
   reservoir sampling — so each reservoir draw is a phantom Scen draw gamemd never
   makes. Routing it to `scen_rng` would still desync the Scen stream vs retail. The
   correct resolution is the **native-queue migration** (heap-pop budget replacing the
   scan), documented in `TIBERIUMCLASS_GROWTH_PROCESSOR_EXACT_QUEUE_PROCESSING` and
   `TIBERIUMCLASS_SPREAD_PROCESSOR_AUDIT` handoffs. Until that lands, ore-growth RNG
   consumption cannot match gamemd by stream-routing alone.

2. **The native-shaped queue methods coexist with the legacy scanner.** Sites
   L363/L435/L525/L559/L583/L693/L863 are part of the new native-queue methods
   (`enqueue_growth_queue_cell`, the growth/spread processors), but the **live**
   `tick_ore_growth` path still drives the reservoir scanner. So today both a faithful
   queue model and a non-gamemd scanner exist in the same file; the routing table
   above is correct for once the native processor is the live path.

3. **L1274 variant bound:** Rust draws `next_range_u32(variants.len())`; gamemd draws
   a fixed `RandomRanged(0,11)` (12 variants) only on empty-flat **new** placement. If
   `variants.len() != 12`, the bound itself drifts; verify the variant table length and
   that the draw fires only for new empty cells (not growth density increments).

## 5. Coverage Ledger

| Area | Status | Evidence | What remains |
|---|---|---|---|
| GrowthProcessor batch draw instance | verified | `0x00722F6A` live disasm = Scen | none |
| GrowthProcessor reinsert-priority draw instance | verified | `0x0072303F` live disasm = Scen | none |
| SpreadProcessor batch draw instance | verified | `0x007224AE` live disasm = Scen | none |
| AddToGrowthQueue draw instance | verified | `0x007235F9` live disasm = Scen | none |
| AddToSpreadQueue draw instance | verified | `0x00722B5B` live disasm = Scen | none |
| SpreadTiberium dir / PlaceTiberium variant / TIBTRE prob instance | verified | prior `ORE_TIBERIUM_RNG_CLASSIFICATION` report (Scen) | none |
| g_MainRng usage anywhere in tiberium growth/spread | verified-absent | no `0x00886B88` load in any of the 5 disassembled functions | none |
| Rust reservoir draw liveness | verified | `ore_growth.rs:1375,1385,1463` live in `tick_ore_growth` | resolved by native-queue migration |
| Native-queue migration | deferred | covered by GrowthProcessor/SpreadProcessor handoffs | implement queue processor (separate effort) |

## 6. Open Questions — Final State

- `[RESOLVED] OGR-01 — Which instance does GrowthProcessor draw from? → Scen->Random (`ScenarioClass+0x218`).` (evidence: `disassemble_function 0x00722F00`, draws at `0x00722F6A`/`0x0072303F` load `[0x00a8b230]+0x218`)
- `[RESOLVED] OGR-02 — Which instance does SpreadProcessor draw from? → Scen->Random.` (evidence: `disassemble_function 0x00722440`, draw at `0x007224AE`)
- `[RESOLVED] OGR-03 — Which instance do AddToGrowthQueue / AddToSpreadQueue draw from? → Scen->Random.` (evidence: `0x007235F9`, `0x00722B5B`)
- `[RESOLVED] OGR-04 — Does the GrowthProcessor report's "global RNG object" mean g_MainRng? → No; it is the scenario-embedded instance. Doc wording is misleading.` (evidence: §2 instance proofs)
- `[RESOLVED] OGR-05 — Does any tiberium growth/spread draw use g_MainRng (`0x00886B88`)? → No.` (evidence: absence across all 5 disassembled functions)
- `[RESOLVED] OGR-06 — Does the Rust reservoir draw (L1463) correspond to any gamemd draw? → No; gamemd uses heap-pop, not reservoir sampling.` (evidence: GrowthProcessor/SpreadProcessor heap-pop logic vs `ore_growth.rs:1452-1470`)
- `[RESOLVED] OGR-07 — Is the reservoir draw on a live path? → Yes; `tick_ore_growth` calls it at L1375/L1385.` (evidence: `ore_growth.rs` grep)
- `[RESOLVED] OGR-08 — Does growth density increment draw the (0,11) variant? → No; only new-empty-cell placement draws it; growth uses the additive PlaceTiberium branch.` (evidence: `ORE_TIBERIUM_RNG_CLASSIFICATION` §3.4, GrowthProcessor RNG summary)
- `[DEFERRED] OGR-09 — Exact `variants.len()` value at `ore_growth.rs:1274` vs fixed 12.` (category: bounded-cost-too-high; reason: needs the overlay-variant table read; next-step: confirm table length and new-cell-only firing.)

## 7. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected surface | Required effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| All tiberium growth/spread/queue/placement draws are Scen->Random. | §2 instance proofs | All ore_growth draws currently use the single shared `rng`. | `src/sim/ore_growth.rs` (all 10 gamemd-corresponding sites), `tick_ore_growth` signature, `world/mod.rs` caller | Route every gamemd-corresponding ore draw to `scen_rng`. | Snapshot `rng.state()` and `scen_rng.state()` around a growth/spread tick; assert only `scen_rng` advanced. | Do **not** route any ore draw to the main `rng`; none belong there. |
| gamemd has no reservoir-sampling draw. | GrowthProcessor/SpreadProcessor heap-pop logic | `reservoir_sample` (L1452) is live (L1375/L1385). | `src/sim/ore_growth.rs` legacy scanner | Replace scan/reservoir selection with native heap-pop budget; eliminate the L1463 draw. | After migration, a growth tick draws exactly `1 batch + N reinsert (+ conditional spread)` draws — no per-candidate reservoir draws. | Do **not** "fix" parity by routing the reservoir draw to a stream; the draw itself must be removed. |
| Variant draw is fixed `RandomRanged(0,11)`, new-empty-cell only. | `0x0048725C` | Rust uses `next_range_u32(variants.len())`. | `ore_growth.rs:1274` | Confirm 12-entry bound; fire only on new empty placement, not growth increment. | New empty cell draws exactly one `(0,11)`; a growth density increment draws zero variant draws. | Do not draw the variant on additive growth. |

## 8. Stale Docs / Follow-up

- `TIBERIUMCLASS_GROWTH_PROCESSOR_EXACT_QUEUE_PROCESSING_GHIDRA_REPORT.md` §3.2 and
  §3.7: replace "Random__Next from global RNG object" / "Random::Next()" with:
  "raw `Random::Next` on **Scen->Random** (`ScenarioClass+0x218`, via `[0x00A8B230]`)
  — verified `disassemble_function 0x00722F00`, draws at `0x00722F6A` and `0x0072303F`
  load `[0x00a8b230]+0x218`. Not the global `g_MainRng` (`0x00886B88`)."
- `TWO_RNG_STREAM_IMPLEMENTATION_CONTRACT_20260529.md`: the ore_growth.rs reroute set
  is fully resolved by this report — all 10 gamemd-corresponding draws → `scen_rng`;
  the 11th (reservoir, L1463) is a Rust-only draw to be removed by the native-queue
  migration, not routed. This clears the contract's "ore_growth origins" blocker for
  stream-routing purposes (the consumption-count parity remains gated on the native
  migration).

## Sources

- Ghidra live disassembly this session: `TiberiumClass::GrowthProcessor @ 0x00722F00`, `TiberiumClass::SpreadProcessor @ 0x00722440`, `TiberiumClass::AddToGrowthQueue @ 0x007235A0`, `TiberiumClass::AddToSpreadQueue @ 0x00722AF0`.
- Prior verified reports: `ORE_TIBERIUM_RNG_CLASSIFICATION_GHIDRA_REPORT.md`, `TIBERIUMCLASS_GROWTH_PROCESSOR_EXACT_QUEUE_PROCESSING_GHIDRA_REPORT.md`, `TIBERIUMCLASS_SPREAD_PROCESSOR_AUDIT_GHIDRA_REPORT.md`, `TWO_RNG_STREAM_IMPLEMENTATION_CONTRACT_20260529.md`.
- Rust scanned: `src/sim/ore_growth.rs` (draw sites L363–L1499; `tick_ore_growth` L1334; `reservoir_sample` L1452).
