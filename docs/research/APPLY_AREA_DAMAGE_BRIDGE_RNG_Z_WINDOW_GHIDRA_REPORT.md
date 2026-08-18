# Apply_area_damage Bridge RNG/Z Window - Ghidra Research Report

**Address(es):** `0x00489280` (`Apply_area_damage`) primary; `0x0065C7E0` (`Random__RandomRanged`) RNG callee; init stubs around `0x00489060` / `0x00489100` for `DAT_0089E870` / `DAT_0089E864`.
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** only the four bridge-related blocks inside `Apply_area_damage`: block order, absence of inter-block early-out, per-block `BridgeStrength` RNG draw order/count, structural Z-window bounds, `Flags & 0x100` gating, and static init-site recovery for `DAT_0089E870` / `DAT_0089E864`.
**Non-Scope:** `ApplyDamageToCell @ 0x00587180` internals, bridge state-machine mutation, rim refresh, ramp perpendicular helpers, object AoE target collection, debris/explosion RNG, and CABHUT/C4 bridge collapse.
**Confidence:** High for block order, RNG calls, Z-window branch predicates, and `DAT_0089E864` derivation; Medium for the exact numeric runtime value of `DAT_0089E870` because no live debugger/post-map-load memory capture was available in this slot.
**Active in YR:** Yes, conditional on standard bridge tile damage gates: `Scenario.SpecialFlags & 0x8000` (`DestroyableBridges`) and `WarheadType+0x144` (`Wall=yes`). Stock YR has `DestroyableBridges=yes` and `BridgeStrength=1500` in retail rules; the active consumer is the scenario special flag and the warhead `Wall` byte.

## 0. Working Notes

Target question: confirm whether `Apply_area_damage @ 0x00489280` runs all four bridge blocks with independent `BridgeStrength` RNG draws, and recover the structural Z-window constants/bounds.

Non-goals: do not analyze `ApplyDamageToCell` internals, bridge walkers, rim refresh, low-family ramp helpers, object AoE collection, or debris RNG.

Evidence needed to mark COMPLETE: decompile plus read-only assembly context for the four block entries/RNG call sites/Z-window branches; stock INI/default evidence for `BridgeStrength` and bridge activation; init-site assembly for `DAT_0089E870` / `DAT_0089E864`, or explicit live-capture deferral.

Stop conditions: stop after the four-block order, RNG call count/order, Z-window bounds/gate, constants status, Rust-facing delta, and stale-doc replacement wording are resolved or deferred with reason.

## 1. Overview

The bridge section of `Apply_area_damage` is a sequential four-block dispatcher, not a first-match dispatcher. Blocks A, B, C, and D run in fixed order; each block that passes its own identity/Z/warhead gates either bypasses the `BridgeStrength` roll for `IonCannonWarhead` or consumes its own `Random__RandomRanged(1, Rules+0x1740)` draw from `Scenario+0x218`.

The structural Z-window is lepton-based and guarded by `CellClass.Flags & 0x100`. It accepts `((Level - 2) * DAT_0089E870 + DAT_0089E864) < impact_z <= ((Level + 1) * DAT_0089E870 + DAT_0089E864)`. The lower bound is exclusive; the upper bound is inclusive. Direct overlay blocks C/D do not apply this structural Z-window in `Apply_area_damage`.

## 2. Key Offsets / Globals

| Item | Binary location | Meaning | Active in YR |
|---|---:|---|---|
| `Scenario+0x218` | `0x00489FEF`, `0x0048A173`, `0x0048A23F`, `0x0048A299` | Scenario RNG object passed as `ECX` to `Random__RandomRanged` | Yes; live bridge damage RNG stream |
| `Rules+0x1740` | `0x00489FE0`, `0x0048A165`, `0x0048A231`, `0x0048A28B` | `[CombatDamage] BridgeStrength` upper bound | Yes; stock YR `1500` |
| `Rules+0xFF0` | compared before each RNG call | `IonCannonWarhead`; matching warhead bypasses the `BridgeStrength` roll | Yes; stock bridge-damage special identity |
| `WarheadType+0x144` | `0x00489FC5`, `0x0048A14A`; outer bridge gate also in decompile | `Wall=yes`; required before structural bridge blocks run | Yes |
| `CellClass+0x140 & 0x100` | `0x00489F77..0x00489FC2`, `0x0048A102..0x0048A147` | structural bridge flag; gates whether the Z-window is evaluated | Yes |
| `CellClass+0x11B` | `0x00489F82`, `0x0048A10D` | signed terrain level byte used in structural Z-window | Yes |
| `CellClass+0x44` | `0x0048A214`, `0x0048A26A` | direct overlay index for low/high overlay blocks | Yes |
| `DAT_0089E870` | write at `0x0048908B`; reads at `0x00489F90`, `0x0048A114` | runtime/theater-initialized level-height/lepton step | Yes, but exact runtime value needs capture |
| `DAT_0089E864` | write at `0x00489120`; reads at `0x00489F97`, `0x0048A127` | derived base/intercept: `2 * DAT_0089E870` | Yes |

## 3. Core Logic

### 3.1 Four Bridge Blocks Run Sequentially

`Apply_area_damage` reaches the bridge section only after the standard bridge tile gates. Once inside, the block order is:

1. Block A, high/structural tile-set candidate, enters at the tile-set match path ending at `0x00489F77`, calls `ApplyDamageToCell @ 0x00587180` after its gate.
2. Block B, low/structural tile-set candidate, starts at `0x0048A0A5`, calls `ApplyDamageToCell @ 0x00587180` after its gate.
3. Block C, direct low overlay candidate, starts at `0x0048A214`, calls `DestroyBridge_Low @ 0x0057BAA0` after its gate.
4. Block D, direct high overlay candidate, starts at `0x0048A26A`, calls `DestroyBridge_High @ 0x0057CCF0` after its gate.

There is no `break` or success early-out from A to B, B to C, or C to D. The assembly fall-through/skip labels are sequential: A's skip/final tail lands at B (`0x0048A0A5`), B's skip/final tail lands at C (`0x0048A214`), C's skip/final tail lands at D (`0x0048A26A`), and only D exits to the non-bridge tail at `0x0048A2C4`.

**Active in YR:** Yes, conditional on the bridge section gate. Evidence: `Apply_area_damage @ 0x00489280` decompile plus read-only assembly contexts at `0x00489F77`, `0x0048A0A5`, `0x0048A214`, `0x0048A26A`; stock `BridgeStrength=1500` in `ini/rulesmd.ini:816`.

### 3.2 Exact RNG Draw Count / Order

For non-Ion warheads, each eligible block independently performs:

```text
Random__RandomRanged(1, Rules.BridgeStrength)
pass only if roll < effective_damage
```

Draw order is fixed:

| Order | Block | RNG assembly evidence | Success callee | Active in YR |
|---:|---|---|---|---|
| 1 | A high structural / state-machine | `0x00489FE0` read `Rules+0x1740`; `0x00489FED PUSH 1`; `0x00489FEF LEA ECX,[Scenario+0x218]`; `0x00489FF5 CALL 0x0065C7E0`; `0x00489FFA CMP`; `0x00489FFE JGE skip` | `ApplyDamageToCell` | Yes |
| 2 | B low structural / state-machine | `0x0048A165` read `Rules+0x1740`; `0x0048A171 PUSH 1`; `0x0048A173 LEA ECX,[Scenario+0x218]`; `0x0048A179 CALL`; `0x0048A17E CMP`; `0x0048A182 JGE skip` | `ApplyDamageToCell` | Yes |
| 3 | C direct low overlay | `0x0048A231` read `Rules+0x1740`; `0x0048A23D PUSH 1`; `0x0048A23F LEA ECX,[Scenario+0x218]`; `0x0048A245 CALL`; `0x0048A24A CMP`; `0x0048A24E JGE skip` | `DestroyBridge_Low` | Yes |
| 4 | D direct high overlay | `0x0048A28B` read `Rules+0x1740`; `0x0048A297 PUSH 1`; `0x0048A299 LEA ECX,[Scenario+0x218]`; `0x0048A29F CALL`; `0x0048A2A4 CMP`; `0x0048A2A8 JGE skip` | `DestroyBridge_High` | Yes |

The `JGE` skip proves equality fails. A roll equal to effective damage does not damage the bridge. `IonCannonWarhead` pointer equality to `Rules+0xFF0` jumps around each RNG call and enters the block's damage callee without consuming that block's `BridgeStrength` draw.

State-machine blocks A/B may retry `ApplyDamageToCell` up to three more times for Ion when the call returns false. Those retries do not add `BridgeStrength` RNG draws because the RNG gate is before the retry loop. Direct overlay blocks C/D are single-shot.

**Active in YR:** Yes. Evidence: decompile of `Apply_area_damage @ 0x00489280`, assembly ranges above, stock `BridgeStrength=1500` in `ini/rulesmd.ini:816`, and `IonCannonWarhead=IonCannonWH` in `ini/rulesmd.ini:874`.

### 3.3 Structural Z-Window

Blocks A and B evaluate the Z-window only when `CellClass.Flags & 0x100` is set. The assembly tests `CellClass+0x140` high byte with `TEST AH,0x1`; if the test is zero, execution jumps around the window to the warhead/RNG gate.

When active, the skip predicates are:

```text
if impact_z >  (Level + 1) * DAT_0089E870 + DAT_0089E864: skip
if impact_z <= (Level - 2) * DAT_0089E870 + DAT_0089E864: skip
```

Therefore the accepted range is:

```text
(Level - 2) * DAT_0089E870 + DAT_0089E864 < impact_z
impact_z <= (Level + 1) * DAT_0089E870 + DAT_0089E864
```

The lower bound is `-2` relative levels and exclusive (`JLE skip` at `0x00489FBC` / `0x0048A141`). The upper bound is `+1` relative levels and inclusive (`JG skip` at `0x00489FA2` / `0x0048A131`). The units are leptons, not abstract deck-level units.

**Active in YR:** Yes, conditional on structural bridge flag `Flags & 0x100`. Evidence: block A assembly `0x00489F77..0x00489FC2`; block B assembly `0x0048A102..0x0048A147`; decompile of `Apply_area_damage @ 0x00489280`.

### 3.4 Constant Init-Site Recovery

`DAT_0089E864` is statically recoverable as a derivation from `DAT_0089E870`:

```text
0x00489101  read DAT_0089E870
0x00489106  compute 4 * DAT_0089E870
0x00489111  FILD that integer
0x00489115  multiply by static double 0.5 at 0x007E1738
0x0048911B  ftol
0x00489120  write DAT_0089E864
```

So `DAT_0089E864 = ftol(DAT_0089E870 * 4 * 0.5) = 2 * DAT_0089E870` for the integer level-height values used here.

`DAT_0089E870` has a write at `0x0048908B` after FPU arithmetic that reads runtime/theater geometry globals including `DAT_0089E7F8`, `DAT_0089E820`, and `DAT_0089E818`, then calls helper `0x004CAD50`, multiplies by `DAT_0089E818`, multiplies by static double `0.5` at `0x007E1738`, and calls `Math__ftol @ 0x007C5F00`. The init formula site is recovered, but the exact numeric post-map value was not recoverable from static data in this read-only slot because those operands are themselves runtime-initialized.

**Constant status:** `DAT_0089E864` does not need a separate capture once `DAT_0089E870` is known; capture `DAT_0089E870` post-map-load and compute `DAT_0089E864 = 2 * DAT_0089E870`, or capture both to verify the invariant. Prior docs report nominal `DAT_0089E870 = 104` and `DAT_0089E864 = 208`; this slot did not re-capture those numeric values live.

**Active in YR:** Yes. Evidence: read-only assembly contexts at `0x0048908B` and `0x00489100..0x00489120`; use sites inside active `Apply_area_damage` structural Z-window.

## 4. INI / Activation

| Key / flag | Retail YR source | Binary role | Active in YR |
|---|---|---|---|
| `BridgeStrength=1500` | `ini/rulesmd.ini:816`; same in base `rules.ini:676` | parsed to `Rules+0x1740`; upper bound for each non-Ion block's independent RNG draw | Yes |
| `IonCannonWarhead=IonCannonWH` | `ini/rulesmd.ini:874` per prior bridge entry report | `Rules+0xFF0`; bypasses `BridgeStrength` RNG in each block | Yes |
| `DestroyableBridges=yes` | stock rules line exists, but active consumer is scenario `SpecialFlags & 0x8000` per prior verified report | outer bridge section gate | Yes in stock skirmish default; map/scenario override conditional |
| Warhead `Wall=yes` | warhead sections vary | `WarheadType+0x144`; required for bridge tile damage | Yes for wall-capable warheads |

## 5. Integration Points

`Apply_area_damage` is reached from standard weapon/superweapon/anim damage paths already documented in prior reports. This slot did not re-audit the caller taxonomy. The four-block bridge section runs after ordinary object AoE processing and before the later overlay/particle tail. The findings here affect deterministic bridge tile damage dispatch and RNG stream advancement, not object splash target collection.

## 6. Current Rust Implementation Status

Current Rust has recently moved toward a four-path loop, but it still stops too early:

| Surface | Current behavior | Binary delta |
|---|---|---|
| `src/sim/world/bridge_orchestrator.rs::run_dispatch_loop` | path order is A/B/C/D-equivalent, and non-Ion paths use `next_range_u32_inclusive(1, bridge_strength)` with strict `<`; however the loop `break`s after the first path whose gate passed, even if the outcome had no effect or later blocks would also match | mismatch: gamemd continues to evaluate all four blocks; a multi-matching cell consumes multiple independent `BridgeStrength` draws in A/B/C/D order |
| `src/sim/bridge_state/mod.rs::path_matches_cell` | state-machine Z gate uses `impact_z < level - 1 || impact_z > level + 1` in level units and applies it broadly to state-machine candidates | mismatch: gamemd uses lepton formula with lower `Level-2` exclusive, upper `Level+1` inclusive, and evaluates it only when `Flags & 0x100` is set |
| `src/sim/combat/combat_aoe.rs::bridge_adjusted_impact_z` | builds a bridge-adjusted impact Z from terrain level units for Rust events | unchecked/mismatch risk: future bridge event construction must pass lepton-Z values compatible with `DAT_0089E870` / `DAT_0089E864`, not just deck-level units |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Apply_area_damage` four-block order | verified | decompile `0x00489280`; assembly contexts `0x00489F77`, `0x0048A0A5`, `0x0048A214`, `0x0048A26A` | none |
| Block A RNG call | verified | `0x00489FE0..0x00489FFE` | none |
| Block B RNG call | verified | `0x0048A165..0x0048A182` | none |
| Block C RNG call | verified | `0x0048A231..0x0048A24E` | none |
| Block D RNG call | verified | `0x0048A28B..0x0048A2A8` | none |
| IonCannon bypass of block RNG | verified | `Rules+0xFF0` compares before each RNG call | retry semantics after state-machine call are out-of-scope except no extra `BridgeStrength` draws |
| Structural Z-window bounds | verified | `0x00489F82..0x00489FBC`, `0x0048A10D..0x0048A141` | none |
| `Flags & 0x100` Z-window gate | verified | `0x00489F77 TEST AH,0x1`; `0x0048A102 TEST AH,0x1` | none |
| `DAT_0089E864` init site | verified | `0x00489100..0x00489120` | none for derivation |
| `DAT_0089E870` init site | touched-not-exhausted | `0x00489060..0x0048908B` | exact numeric value requires post-map-load debugger capture |
| Current Rust dispatcher scan | verified | `src/sim/world/bridge_orchestrator.rs::run_dispatch_loop`; `src/sim/bridge_state/mod.rs::path_matches_cell` | implementation fix not done in this research slot |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Does the bridge section run all four blocks or break after first success? -> It runs A, B, C, D sequentially; no inter-block success break.` (evidence: `0x00489F77 -> 0x0048A0A5 -> 0x0048A214 -> 0x0048A26A -> 0x0048A2C4`)
- `[RESOLVED] OQ-2 - Does each eligible block consume its own BridgeStrength RNG draw? -> Yes for non-Ion; one `RandomRanged(1, Rules+0x1740)` per eligible block in A/B/C/D order.` (evidence: `0x00489FE0..0x0048A2A8`)
- `[RESOLVED] OQ-3 - Does IonCannon consume these RNG draws? -> No; each block compares warhead to `Rules+0xFF0` and jumps around the RNG call on equality.` (evidence: `0x00489FD8`, `0x0048A15D`, `0x0048A229`, `0x0048A283`)
- `[RESOLVED] OQ-4 - Is equality with damage a pass? -> No; `JGE` skips after comparing roll with effective damage, so only `roll < damage` passes.` (evidence: `0x00489FFA..0x00489FFE` and sibling block ranges)
- `[RESOLVED] OQ-5 - What is the structural Z-window? -> `(Level-2)*DAT_0089E870 + DAT_0089E864 < z <= (Level+1)*DAT_0089E870 + DAT_0089E864`.` (evidence: `0x00489F82..0x00489FBC`, `0x0048A10D..0x0048A141`)
- `[RESOLVED] OQ-6 - Is the lower bound `-2` exclusive? -> Yes; `CMP impact, lower; JLE skip`.` (evidence: `0x00489FBA..0x00489FBC`, `0x0048A13F..0x0048A141`)
- `[RESOLVED] OQ-7 - Is the Z-window unconditional? -> No; it is skipped unless `CellClass.Flags & 0x100` is set.` (evidence: `0x00489F77..0x00489FC2`, `0x0048A102..0x0048A147`)
- `[RESOLVED] OQ-8 - Can `DAT_0089E864` be recovered statically? -> Yes; writer computes `2 * DAT_0089E870`.` (evidence: `0x00489100..0x00489120`)
- `[DEFERRED] OQ-9 - What is the exact current runtime value of `DAT_0089E870` after a stock YR map loads?` (category: `needs-runtime-debugger`; reason: writer reads other runtime/theater-initialized geometry globals and no debugger server was available; next-step-if-pursued: after loading a map, read `0x0089E870` and `0x0089E864` from live process memory and verify the `2x` invariant)
- `[DEFERRED] OQ-10 - Which concrete map/theater init table calls the `0x00489060` and `0x00489100` stubs?` (category: `requires-different-system-context`; reason: data/function-pointer table dispatch is outside this bridge AoE slice; next-step-if-pursued: audit theater geometry init table xrefs and load order)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `Apply_area_damage` evaluates bridge blocks A/B/C/D sequentially with no inter-block early-out; a cell matching multiple block predicates can consume multiple block-gate draws. | Decompile `0x00489280`; assembly contexts `0x00489F77`, `0x0048A0A5`, `0x0048A214`, `0x0048A26A` | mismatch: `run_dispatch_loop` breaks after the first path scan | `src/sim/world/bridge_orchestrator.rs::run_dispatch_loop` | remove first-match termination for bridge block scanning; preserve A/B/C/D order and per-block side effects/dirtying | `bridge_damage_multimatch_consumes_all_matching_block_rng_draws`: one event on a cell satisfying two block predicates advances the scenario RNG by two `R(1,BridgeStrength)` draws and attempts both blocks | Do not collapse the four blocks into a single classified route; draw count/order is lockstep-critical |
| Each eligible non-Ion block independently rolls `RandomRanged(1, BridgeStrength)` from `Scenario+0x218`; equality fails (`roll < damage` only). | `0x00489FE0..0x00489FFE`, `0x0048A165..0x0048A182`, `0x0048A231..0x0048A24E`, `0x0048A28B..0x0048A2A8`; `ini/rulesmd.ini:816` | mostly matches for one path, but draw count is wrong when more than one block matches | `src/sim/world/bridge_orchestrator.rs`; RNG tests/oracle fixtures | keep inclusive low/high bounds and strict `<`, but execute them per eligible block in binary order | `bridge_damage_rng_equal_damage_fails_per_block`: configured roll equal to damage skips that block and still lets later matching blocks perform their own gate | Do not use `<=`, `0..BridgeStrength`, or a one-roll shared gate |
| Structural Z-window is lepton-based, lower `Level-2` exclusive, upper `Level+1` inclusive, and evaluated only when `Flags & 0x100` is set. | `0x00489F77..0x00489FC2`, `0x0048A102..0x0048A147`; init stubs `0x0048908B`, `0x00489120` | mismatch: `path_matches_cell` uses level units, `level-1..level+1`, and lacks a structural-flag-gated lepton formula | `src/sim/bridge_state/mod.rs::path_matches_cell`; event Z construction in `src/sim/combat/combat_aoe.rs` / bridge event producers | represent bridge impact Z in native lepton units and apply the Z gate only for structural-flagged candidates | `bridge_damage_z_window_lower_exclusive_upper_inclusive_leptons`: z at lower bound fails, lower+1 passes, upper passes, upper+1 fails; non-`0x100` candidate skips the Z check | Do not hardcode `104`/`208` without sourcing from map/theater geometry or an explicit captured invariant |

### Negative Facts / Do Not Do

- Do not implement `Apply_area_damage` bridge dispatch as first-match-wins. Active in YR: Yes; evidence `0x00489F77 -> 0x0048A0A5 -> 0x0048A214 -> 0x0048A26A`.
- Do not share one `BridgeStrength` RNG roll across multiple matching bridge blocks. Active in YR: Yes; each block has its own `CALL 0x0065C7E0`.
- Do not make the structural Z lower bound inclusive or use `Level-1` as the lower side. Active in YR: Yes; lower skip is `impact_z <= (Level-2)*step + base`.
- Do not apply the structural Z-window when `Flags & 0x100` is clear. Active in YR: Yes; `TEST AH,0x1` skips the window.
- Do not treat `DAT_0089E864` as an independent INI/rules constant. Active in YR: Yes; it is written by theater geometry init as `2 * DAT_0089E870`.

### Proposed Tests

- `bridge_damage_multimatch_consumes_all_matching_block_rng_draws`
- `bridge_damage_rng_equal_damage_fails_per_block`
- `bridge_damage_ion_bypasses_block_strength_draws_but_still_runs_later_blocks`
- `bridge_damage_z_window_lower_exclusive_upper_inclusive_leptons`
- `bridge_damage_non_structural_candidate_skips_z_window`

### Remaining Uncertainty

- Exact numeric `DAT_0089E870` after stock map load requires a live post-map-load debugger capture. Existing docs cite nominal `104`; this slot verified the writer formula site but did not capture live memory.
- The indirect caller/table that schedules the `DAT_0089E870` and `DAT_0089E864` init stubs was not traced; only the write sites and use sites were in scope.
- `ApplyDamageToCell` internal duplicate dispatch/second draw behavior is assigned to another swarm slot and is not claimed by this report.

### Stale Docs / Follow-up Docs

- Replace wording that says Rust's bridge dispatcher is green or binary-equivalent for path order with: "Gamemd `Apply_area_damage @ 0x00489280` evaluates bridge blocks A/B/C/D sequentially with no inter-block early-out. Current Rust path order is similar, but any first-match `break` is drift because multi-matching cells must consume one `RandomRanged(1,BridgeStrength)` draw per eligible non-Ion block."
- Replace wording that describes the structural Z gate as `level-1 <= impact_z <= level+1` or as deck-level units with: "The structural Z gate is lepton-based and only runs when `CellClass.Flags & 0x100` is set: `(Level-2)*DAT_0089E870 + DAT_0089E864 < impact_z <= (Level+1)*DAT_0089E870 + DAT_0089E864`."
- Replace wording that treats `DAT_0089E864` and `DAT_0089E870` as ordinary rules constants with: "`DAT_0089E864` is derived at `0x00489120` as `2 * DAT_0089E870`; `DAT_0089E870` is runtime/theater-initialized at `0x0048908B` and exact numeric values should be captured post-map-load."

## Sources

- Ghidra read-only decompile: `Apply_area_damage @ 0x00489280`.
- Ghidra read-only assembly contexts: `0x00489F77`, `0x00489FEF`, `0x0048A102`, `0x0048A171`, `0x0048A214`, `0x0048A26A`, `0x0048908B`, `0x00489100`, `0x00489120`.
- Prior research read for reconciliation: `docs/research/bridges/01-assets-map-load-overlay/DAT_0089E864_BRIDGE_THRESHOLD_IDENTITY_GHIDRA_REPORT.md`; `docs/research/bridges/05-damage-collapse-repair-cabhut/BRIDGE_RNG_CALL_ORDER_CLASSIFICATION_GHIDRA_REPORT.md`; `docs/research/bridges/05-damage-collapse-repair-cabhut/WEAPON_AOE_BRIDGE_DAMAGE_ENTRY_GHIDRA_REPORT.md`; `docs/research/bridges/BRIDGE_PARITY_IMPLEMENTATION_CONTRACT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.

## Status

COMPLETE for the requested four-block/RNG/Z-window/static-init-site slice. Exact numeric `DAT_0089E870` remains a documented live-capture requirement, not an unresolved logic question.
