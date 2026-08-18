# ApplyDamageToCell Overlay-First Routing - Ghidra Research Report

**Address(es):** `0x00587180` (`ApplyDamageToCell`), bounded caller context `0x00489280` (`Apply_area_damage`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Inner dispatcher branch order, direct-overlay short-circuit ranges, duplicate outer C/D draw opportunity, and High/Low state-machine classifier inputs.
**Non-Scope:** Full `Apply_area_damage` AoE mechanics, direct bridge walker algorithms, ramp refresh, railing data, debris, zone rebuild internals, BSS constant recovery.
**Confidence:** High for branch order, draw-gate placement, signedness/ranges, fields/offsets, and "not deck_level>=4"; Medium for post-mutation duplicate-draw frequency because that depends on direct walker side effects outside this slice.
**Active in YR:** Yes. Reached from `Apply_area_damage` when `ScenarioClass.SpecialFlags & 0x8000` (`DestroyableBridges`) and `WarheadType+0x144` (`Wall`) are true; also reached from bridge hut death and other live destruction paths.

## Target Question

Does `ApplyDamageToCell @ 0x00587180` dispatch direct overlay bridge damage before state-machine routing, can `Apply_area_damage` perform a second direct-overlay Block C/D `RandomRanged(1, BridgeStrength)` draw after an A/B `ApplyDamageToCell` call, and does High/Low state-machine routing use tile-type/neighbor-overlay inputs rather than `deck_level >= 4`?

## Non-Goals

- Do not re-investigate all four `Apply_area_damage` blocks beyond the call ordering and draw gates needed for this dispatcher.
- Do not decode `DestroyBridge_*` walker side effects except to identify direct-overlay callee targets.
- Do not recover runtime-initialized BSS constants or railing tables.
- Do not edit Rust, INI, or prior research docs.

## Evidence Needed To Mark COMPLETE

- `ApplyDamageToCell` decompile and assembly show direct-overlay tests before state-machine tests.
- Assembly-level ranges and jumps prove inclusive/exclusive overlay bands and no state-machine execution after a direct-overlay match.
- `Apply_area_damage` assembly shows Block A/B `ApplyDamageToCell` calls fall through to Block C/D gates, with independent `RandomRanged(1, BridgeStrength)` calls.
- `ApplyDamageToCell` state-machine classifier reads `CellClass+0x38`, `+0x44`, `+0x140`, `+0x24`, `+0x2c`, and not `CellClass+0x11b`/deck height.
- Current Rust scan identifies where `deck_level >= 4` is still used as the High/Low discriminator.

## Stop Conditions

Stop once the dispatcher branch order, draw opportunities, classifier inputs, and Rust-facing deltas are proven. Stop before decoding direct bridge walkers, ramp algorithms, BSS init sites, or visual side effects.

## 1. Overview

`ApplyDamageToCell` is an inner bridge damage dispatcher. It first checks the visible overlay byte (`CellClass+0x44`) and immediately routes raw low/high bridge body overlays to `DestroyBridge_Low` or `DestroyBridge_High`. Only cells that miss those raw overlay bands proceed to state-machine routing.

The High/Low state-machine classifier is not a height heuristic. It uses the cell's isometric tile-type index relative to runtime bridge base globals, plus a perpendicular/anchor neighbor overlay check when `Flags & 0x100` is set.

## 2. Key Offsets And Inputs

| Field / global | Evidence | Use in this slice |
|---|---|---|
| `CellClass+0x24` | `0x0058723a`, `0x00587267` paths | Map coordinate for self or linked neighbor lookup. |
| `CellClass+0x2c` | `0x00587267` | Linked/parent cell pointer used when `Flags&0x100` set and `Flags&0x80` clear. |
| `CellClass+0x38` | `0x00587218`, `0x00587327` | Iso tile-type index for High/Low state-machine set membership. |
| `CellClass+0x44` | `0x005871db`, `0x005872a4`, `0x00587337` | Visible overlay byte; direct-overlay dispatch and neighbor overlay routing. |
| `CellClass+0x140` | `0x00587223`, `0x00587235` | Bridge flags; `0x100` gates neighbor lookup, `0x80` chooses self coord vs linked coord. |
| `DAT_00aa0e28` | `0x0058721b` | High bridge tile-type base subtracted from `+0x38`. |
| `DAT_00abad1c` | `0x0058732a` | Low bridge tile-type base subtracted from `+0x38`. |
| `DAT_00abad30`, `DAT_00aa1028` | `0x005872b9..0x00587321`, `0x0058734a..0x00587370` | Tile-index set heads; each tested as `base..base+3`. |
| `Rules+0x1740` | `0x00489fe0`, `0x0048a165`, `0x0048a231`, `0x0048a28b` | `BridgeStrength` upper bound for independent outer RNG gates. |

Negative offset fact: this dispatcher does not read `CellClass+0x11b` or any `deck_level` equivalent for High/Low routing. `+0x11b` appears in the caller's Z-window blocks, not in this classifier.

## 3. Core Logic

### 3.1 Entry and Cell Fetch

The function is a `thiscall`-style MapClass helper: `ECX` is the map/context object and the single stack argument is a pointer to a packed cell coordinate. It fetches the target cell as:

`index = signed_y * 512 + signed_x`, valid only for `0 <= index < 0x40000` and non-null `g_CellArray_Base[index]`.

On out-of-range or null cell, it writes the requested coordinate to `DAT_00abdc74` and uses fallback cell `DAT_00abdc50`. This fallback path still continues through the same dispatcher.

### 3.2 Overlay-First Dispatch

The first real branch reads `CellClass+0x44` and tests it before any tile-index state-machine logic:

| Order | Range test | Signed branch shape | Callee | Exit behavior |
|---|---|---|---|---|
| 1 | `0x4A..=0x63` | `CMP 0x4A; JL miss`, `CMP 0x63; JG miss` | `DestroyBridge_Low @ 0x0057BAA0` | Store return byte, jump to tail cleanup. |
| 2 | `0xCD..=0xE6` | `CMP 0xCD; JL miss`, `CMP 0xE6; JG miss` | `DestroyBridge_High @ 0x0057CCF0` | Store return byte, jump to tail cleanup. |

Consequences:

- Raw low/high bridge body overlays never reach the state-machine classifier in this call.
- A cell with overlay `-1`, `0x64`, `0x65`, `0xE7`, or `0xE8` misses the inner direct-overlay checks.
- The range tests are signed x86 conditional jumps (`JL`/`JG`), but all live positive overlay IDs in these bands behave as the listed inclusive ranges.

### 3.3 High State-Machine Routing

If direct-overlay tests miss, the dispatcher computes:

`high_rel = (CellClass+0x38 - DAT_00aa0e28) + 1`

Then:

- If `Flags & 0x100` is clear, skip neighbor lookup and test `high_rel` against `DAT_00abad30..DAT_00abad30+3` and `DAT_00aa1028..DAT_00aa1028+3`.
- If `Flags & 0x100` is set:
  - If `Flags & 0x80` is set, lookup the current cell's `+0x24` coordinate.
  - Otherwise lookup `(*(CellClass+0x2c)+0x24)`.
  - If that looked-up neighbor cell has overlay `0x18` or `0x19`, route High immediately.
  - Otherwise fall back to the same `high_rel` set membership tests.

On High match, it calls `ProcessBridgeDamageStateMachine_High @ 0x00576BA0`.

### 3.4 Low State-Machine Routing

If High does not match, the dispatcher computes:

`low_rel = (CellClass+0x38 - DAT_00abad1c) + 1`

Then Low matches if either:

- the previously looked-up neighbor cell exists and has overlay `0xED` or `0xEE`, or
- `low_rel` is in `DAT_00abad30..DAT_00abad30+3` or `DAT_00aa1028..DAT_00aa1028+3`.

On Low match, it calls `ProcessBridgeDamageStateMachine_Low @ 0x00571490`.

If no direct-overlay, High SM, or Low SM match occurs, the function returns false after tail cleanup.

### 3.5 Explicit Deck-Level Negative Test

No `deck_level >= 4` or equivalent threshold exists in `ApplyDamageToCell`. The dispatcher does not read the bridge deck-level byte (`CellClass+0x11b`) while choosing High vs Low. It uses:

- overlay byte `+0x44` for direct Low/High dispatch,
- tile-type index `+0x38` relative to High/Low runtime base globals,
- flags `+0x140`,
- linked/self coordinate selection through `+0x2c/+0x24`,
- neighbor overlay `0x18/0x19` for High and `0xED/0xEE` for Low.

## 4. Caller Ordering And Duplicate C/D Draw Opportunity

`Apply_area_damage @ 0x00489280` contains four sequential bridge blocks. For this report, only the call ordering and RNG gates were checked:

| Block | Address range | Match kind | RNG call | Callee |
|---|---|---|---|---|
| A | `0x00489f27..0x0048a099` | High state-machine candidate | `0x00489FE0..0x00489FFE` | `ApplyDamageToCell @ 0x0048A00E`; Ion retry at `0x0048A032`. |
| B | `0x0048a0a5..0x0048a1cd` | Low state-machine candidate | `0x0048A165..0x0048A182` | `ApplyDamageToCell @ 0x0048A192`; Ion retry at `0x0048A1B6`. |
| C | `0x0048a214..0x0048a26a` | Low direct overlay `0x4A..=0x63` | `0x0048A231..0x0048A24E` | `DestroyBridge_Low @ 0x0048A25A`. |
| D | `0x0048a26a..0x0048a2c4` | High direct overlay `0xCD..=0xE6` | `0x0048A28B..0x0048A2A8` | `DestroyBridge_High @ 0x0048A2B4`. |

The A and B blocks do not return or break after `ApplyDamageToCell`; they fall through to C and D. Therefore a raw overlay cell can consume an A/B `BridgeStrength` draw, enter `ApplyDamageToCell`, be routed by the overlay-first short-circuit to `DestroyBridge_*`, then reach the C/D direct-overlay gate, which has its own second `RandomRanged(1, BridgeStrength)` call if the cell's current overlay still satisfies the C/D range.

This proves the duplicate draw site and order. The exact runtime frequency of a second draw after the first direct destroy attempt is post-mutation dependent and belongs to the direct walker slice.

IonCannon exception: A/B bypass the RNG gate and may retry `ApplyDamageToCell` up to three extra times on false return. C/D are single-shot and have no Ion retry loop.

## 5. INI Keys

| Key / setting | Evidence | Effect |
|---|---|---|
| `SpecialFlags::DestroyableBridges` | `Apply_area_damage` gate `ScenarioClass & 0x8000`; parent context says stock YR-live | Enables bridge damage blocks. |
| `WarheadType.Wall` | `warhead+0x144` in `Apply_area_damage` | Required for bridge damage blocks. |
| `[CombatDamage] BridgeStrength` | `Rules+0x1740` passed to `RandomRanged(1, BridgeStrength)` | Per-block RNG gate upper bound. |
| `IonCannonWarhead` | `Rules+0xFF0` identity comparisons | Bypasses per-block RNG and enables A/B retry loop. |

## 6. Current Rust Implementation Status

Current Rust has moved toward a four-path bridge dispatcher, but still has two relevant mismatches for this slice:

- `src/sim/world/bridge_orchestrator.rs` documents and implements paths as sibling choices and stops after the first path that produces a non-`NoChange` outcome (`run_dispatch_loop`, lines around `1443..1495`). The binary does not model A/B and C/D as mutually exclusive when A/B calls `ApplyDamageToCell`; it falls through to later direct gates.
- `src/sim/bridge_state/mod.rs::path_matches_cell` still uses `let is_high = cell.deck_level >= 4` to discriminate `HighStateMachine` vs `LowStateMachine` (lines around `898..904`). The binary classifier uses tile-type set membership plus neighbor overlay, not deck level.

Related mismatch outside this slot but adjacent: `BridgeDamageEvent.impact_z` comments describe a level-unit window, while the caller uses lepton constants and a `Flags&0x100` gate. That is covered by the BR-01/17/18 slot.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `ApplyDamageToCell @ 0x00587180` entry/cell fetch | verified | decompile + disassembly `0x005871a5..0x005871db` | none |
| Direct-overlay Low branch | verified | `0x005871db..0x005871f4` | none |
| Direct-overlay High branch | verified | `0x005871f9..0x00587213` | none |
| High SM classifier | verified | `0x00587218..0x00587321`, `0x005873fe..0x00587406` | exact runtime values of BSS tile-base globals are out-of-scope |
| Low SM classifier | verified | `0x00587327..0x00587384` | exact runtime values of BSS tile-base globals are out-of-scope |
| No `deck_level>=4` classifier | verified | absence in dispatcher plus offset read inventory; only `+0x38/+0x44/+0x140/+0x24/+0x2c` read for routing | none |
| Outer duplicate C/D draw sites | verified | `Apply_area_damage` disassembly `0x0048A231..0x0048A24E`, `0x0048A28B..0x0048A2A8` | post-first-call overlay mutation frequency requires direct walker slice |
| Rust four-path sibling break | verified from source scan | `src/sim/world/bridge_orchestrator.rs:1443..1495` | implementation patch, not in this report |
| Rust deck-level classifier | verified from source scan | `src/sim/bridge_state/mod.rs:898..904` | implementation patch, not in this report |

## 8. Open Questions - Final State

- `[RESOLVED] Q1 - Is `ApplyDamageToCell` active in YR? -> Yes, called by live `Apply_area_damage`, bridge hut death, and collapse paths.` (evidence: xrefs to `0x00587180`; `Apply_area_damage` gate `0x00489eab..0x00489ec3`)
- `[RESOLVED] Q2 - Does direct overlay dispatch happen before state-machine dispatch? -> Yes, `+0x44` low/high ranges are tested before any `+0x38` tile-index routing.` (evidence: `0x005871db..0x00587213`)
- `[RESOLVED] Q3 - What are inner direct overlay ranges? -> Low `0x4A..=0x63`; High `0xCD..=0xE6`; signed `JL/JG` bounds.` (evidence: `0x005871de..0x00587205`)
- `[RESOLVED] Q4 - Does a direct-overlay match continue into state-machine routing? -> No, both direct branches jump to tail cleanup.` (evidence: `0x005871f4`, `0x00587213`)
- `[RESOLVED] Q5 - What selects High SM? -> High tile-base relative set or neighbor overlay `0x18/0x19` after `Flags&0x100` neighbor lookup.` (evidence: `0x00587218..0x00587321`, `0x005873fe..0x00587406`)
- `[RESOLVED] Q6 - What selects Low SM? -> Low tile-base relative set or neighbor overlay `0xED/0xEE` from the same looked-up neighbor.` (evidence: `0x00587327..0x0058737b`)
- `[RESOLVED] Q7 - Is routing `deck_level>=4`? -> No; dispatcher does not read `+0x11b`/deck height for High/Low choice.` (evidence: dispatcher offset inventory `0x00587180..0x00587384`)
- `[RESOLVED] Q8 - Are C/D direct gates independent RNG gates? -> Yes, each has its own `RandomRanged(1, BridgeStrength)` call and callee.` (evidence: `0x0048A231..0x0048A25A`, `0x0048A28B..0x0048A2B4`)
- `[RESOLVED] Q9 - Do A/B return before C/D? -> No, A/B continue to later blocks after call/dirty-rect tail.` (evidence: fall-through to `0x0048A0A5`, then `0x0048A214`, then `0x0048A26A`)
- `[RESOLVED] Q10 - Is the duplicate C/D second draw unconditional after A/B? -> No; the draw site is unconditional in control flow, but it fires only if the post-A/B current overlay still matches the C/D direct range and warhead is not IonCannon.` (evidence: C/D re-read `CellClass+0x44` at `0x0048A214`, `0x0048A26A`)
- `[RESOLVED] Q11 - Do C/D have Ion retry loops? -> No; retry loops exist only around A/B `ApplyDamageToCell` calls.` (evidence: `0x0048A015..0x0048A03E`, `0x0048A199..0x0048A1C2`; no loop around C/D)
- `[DEFERRED] Q12 - How often does a first direct walker leave the same cell overlay in C/D range?` (category: out-of-scope; reason: requires direct walker side-effect slice; next-step-if-pursued: inspect `DestroyBridgeWalker_*` writes for the hit cell before C/D re-read)
- `[DEFERRED] Q13 - What are concrete runtime values of `DAT_00aa0e28`, `DAT_00abad1c`, `DAT_00abad30`, `DAT_00aa1028`?` (category: out-of-scope; reason: BSS constant sweep is slot 5; next-step-if-pursued: xref runtime initializer sites or capture post-map-load)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `ApplyDamageToCell` checks direct overlay ranges before any state-machine routing. | `0x005871db..0x00587213` | Partial/mismatched model: Rust has direct paths but treats outer paths as siblings. | `src/sim/world/bridge_orchestrator.rs`, `src/sim/bridge_state/mod.rs` | Model A/B calls as "call inner dispatcher", whose first operation can be direct overlay. | A raw high overlay cell that also matches the A high tile set routes first to `DestroyBridge_High`, not `body_cell_advance_state(true)`. | Do not call state-machine drivers before checking overlay byte. |
| `Apply_area_damage` can reach a later C/D direct-overlay RNG gate after an A/B `ApplyDamageToCell` call. | A/B calls `0x0048A00E`/`0x0048A192`; C/D RNG calls `0x0048A245`/`0x0048A29F` after fall-through. | Rust stops after first non-`NoChange` path, suppressing later draw opportunities. | `src/sim/world/bridge_orchestrator.rs::run_dispatch_loop` | Preserve outer block order and independent draw consumption; do not `break` solely because A/B did work. | Oracle fixture where post-A/B overlay remains in direct range consumes two `BridgeStrength` draws in A then D or B then C. | Do not collapse four binary blocks into one winner-takes-all path. |
| High/Low SM routing uses tile-index set membership and neighbor overlay, not `deck_level >= 4`. | `0x00587218..0x00587384`; no `+0x11b` read in classifier. | Rust uses `cell.deck_level >= 4`. | `src/sim/bridge_state/mod.rs::path_matches_cell`; resolved terrain needs bridge tile-class/routing facts. | Replace deck-level proxy with resolved tile-type-set and neighbor-overlay classifier. | A low-family bridge cell at deck level 5 still routes Low; a high-family cell at deck level 2 still routes High. | Do not infer bridge family from visual height. |

### Concrete Proposed Tests

- `apply_damage_to_cell_overlay_first_high_direct_before_sm`
- `apply_damage_to_cell_overlay_first_low_direct_before_sm`
- `bridge_aoe_outer_blocks_do_not_break_after_inner_direct_dispatch`
- `bridge_sm_routing_uses_tile_index_not_deck_level_high_low_crossed`
- `bridge_sm_neighbor_overlay_18_19_vs_ed_ee_routes_family`

## Negative Facts / Do Not Do

- Do not classify High vs Low state-machine routing with `deck_level >= 4`.
- Do not run state-machine drivers before checking raw overlay byte ranges.
- Do not treat outer A/B/C/D paths as mutually exclusive after the first successful outcome.
- Do not add C/D IonCannon retry loops; retries are only around A/B `ApplyDamageToCell`.
- Do not use direct-overlay ranges `0x4A..=0x65` or `0xCD..=0xE8` for the inner dispatcher; those wider bands belong to direct walker internals, not `ApplyDamageToCell` entry tests.

## Remaining Uncertainty

The control-flow and draw sites prove the duplicate C/D second draw opportunity. This report does not prove how often the second draw fires after an A/B call mutates overlays through `DestroyBridge_*`; that requires a direct walker mutation pass. Runtime values of the bridge tile-base globals are also out-of-scope and should be recovered by the BSS constant sweep.

## Stale Docs / Follow-Up Wording

Replace any stale wording that says:

> `ApplyDamageToCell` routes bridge damage by state-machine first, or classifies High/Low by `deck_level >= 4`.

With:

> `ApplyDamageToCell @ 0x00587180` is overlay-first: raw low overlays `0x4A..=0x63` call `DestroyBridge_Low`, raw high overlays `0xCD..=0xE6` call `DestroyBridge_High`, and only misses continue to state-machine routing. High/Low state-machine routing is by `CellClass+0x38` tile-type set membership plus the `Flags&0x100` neighbor overlay check (`0x18/0x19` High, `0xED/0xEE` Low), not by deck level.

Replace any stale wording that says:

> The four bridge damage paths choose one winning Rust path and then stop.

With:

> `Apply_area_damage @ 0x00489280` evaluates bridge blocks sequentially. A/B may call `ApplyDamageToCell`, then execution falls through to C/D direct-overlay gates; each matching non-Ion block has its own `RandomRanged(1, BridgeStrength)` draw.

## Sources

- Ghidra `decompile_function 0x00587180`; `disassemble_function 0x00587180`
- Ghidra `decompile_function 0x00489280`; `disassemble_function 0x00489280`
- Ghidra `decompile_function 0x0057BAA0`, `0x0057CCF0`, `0x00576BA0`, `0x00571490`
- Ghidra xrefs to `0x00587180`, `0x0057BAA0`, `0x0057CCF0`
- Rust scan: `src/sim/world/bridge_orchestrator.rs`, `src/sim/bridge_state/mod.rs`, `src/sim/combat/combat_aoe.rs`
- Prior docs read as navigation/stale-warning inputs: `docs/research/bridges/00-system-models/PHASE_F_BRIDGE_DAMAGE_DISPATCH_VERIFICATION.md`, `docs/research/bridges/BRIDGE_PARITY_IMPLEMENTATION_CONTRACT.md`
