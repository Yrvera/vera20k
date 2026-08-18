# AStar Compute Edge Cost Bridge Costs - Ghidra Research Report

**Address(es):** `0x00429830` primary helper; caller add site `0x00429F6B..0x00429F9D`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** bridge-relevant effects inside `AStar_compute_edge_cost @ 0x00429830`: `0x40000` temporary marker multiplier, bridge structural/orientation flank flags, `PathfinderClass+0x3C` moving-friendly urgency, direction-8/tube bypass, exact constant/order facts, and Rust-facing acceptance tests.  
**Non-Scope:** marker writer geometry in `UpdateBridgePassability @ 0x0042ACF0`, full A* main loop, zone precheck ordering, alternate object list lookup, live stock-map route traces, and Rust implementation.  
**Confidence:** High for scoped helper formula/order; Ghidra decompile was checked against assembly contexts and constant byte reads.  
**Active in YR:** Yes, conditional on normal ground/bridge A* expansion. The helper has one caller from live `AStar_main_loop`; bridge branches depend on live cell flags/layer inputs.

## 0. Working Notes

Target question: Does `AStar_compute_edge_cost @ 0x00429830` have any bridge-specific cost effects beyond the already-known marker x4, and what exact order/constants must Rust preserve?

Non-goals: Do not re-investigate `0x0042ACF0` writer geometry, zone precheck, dual closed-list reopening, stock-map route traces, or modify Rust.

Evidence needed to mark COMPLETE: decompile plus assembly/disassembly address ranges for base table, code-2 urgency, `0x40000`, bridge flank/orientation flags, direction-8 bypass, caller-side epsilon/add order, plus current Rust surface scan and acceptance-test handoff.

Stop conditions: every scoped input resolved/deferred with evidence; zero-add re-read of `0x00429830` and `0x00429F6B..0x00429F9D`; report saved to this path; slot-2 row appended to `.swarm-claims.md`.

## 1. Overview

`AStar_compute_edge_cost` computes the multiplicative cost for normal compass A* edges. The caller handles direction `8` separately, so tube edges bypass this helper and do not receive marker, bridge flank, `Pathfinder+0x04`, or normal direction-epsilon costs.

The bridge-relevant order is fixed: base `Can_Enter_Cell` code cost, optional code-2 friendly-moving adjustment, optional destination `0x40000` x4 marker, optional bridge flank multiplier, then caller-side `Pathfinder+0x04` multiply and direction epsilon add.

## 2. Key Offsets And Constants

| Offset / address | Meaning in this slice | Active in YR | Evidence |
|---|---|---|---|
| `CellClass+0x140 & 0x40000` | Temporary destination-cell A* cost marker; multiplies current edge accumulator by `4.0` | Conditional: yes when `UpdateBridgePassability` marks cells | `0x004299AA..0x004299C2`; bytes `0x007E37BC = 00 00 80 40` |
| `CellClass+0x140 & 0x100` | Structural bridge-cell test for flank cells | Yes on bridge cells | `0x00429A41..0x00429A75` |
| `CellClass+0x140 & 0x800` | Destination bridge orientation selector: chooses NS vs EW flank table | Yes on bridge cells | decompile branch before `0x00429A1E/0x00429A27` |
| `PathfinderClass+0x01` | Enables bridge flank multiplier branch after layer gate | Conditional; constructor clears it, live setter not re-found in this narrow slice | read at `0x004299D2..0x004299D7`; constructor write `0x0042A6E2` |
| `PathfinderClass+0x04` | Caller-side multiplier after helper return; constructor writes `1.0f` | Yes, normally no-op in stock pathfinding | write `0x0042A6D0`; read `0x00429F8F` |
| `PathfinderClass+0x3C` | Per-search urgency for code-2 friendly-moving blockers | Yes | setter `0x0042C900`; reads `0x00429878`, `0x00429995` |
| `0x0081870C` | 8-float base table indexed by `Can_Enter_Cell` code | Yes | bytes `0000803f...00401c46`; load `0x00429848` |
| `0x0081872C` | caller-side direction epsilon table for dirs `0..7`; dir 8 table value is not used on this helper path | Yes for compass dirs | bytes `6f12833a...00000000`; add `0x00429F96` |
| `0x007E37B4/B8/BC` | `2.0`, `10.0`, `4.0` float constants | Yes in scoped branches | byte reads at those addresses |
| `0x007E2AC8` | `1.0` bridge one-flank multiplier | Yes in scoped branch | byte read `00 00 80 3f`; load `0x004299F1` |

## 3. Core Logic

### 3.1 Exact Formula For Compass Directions 0..7

For normal directions `0..7`, the binary does:

1. `edge = base_table[can_enter_code]`.
2. If `can_enter_code == 2`, run the friendly-moving branch:
   - urgency `0`: prediction may leave `edge = 1.0` or fall through to `edge = 4.0`;
   - urgency `1`: skip prediction and write `edge = 4.0`;
   - urgency `2`: write `edge = 4.0`, then override to `edge = 1000.0`.
3. If destination `CellClass+0x140 & 0x40000`, `edge *= 4.0`.
4. If entering the bridge layer and `PathfinderClass+0x01 != 0`, apply bridge flank multiplier:
   - first flank not structural bridge: `edge *= 10.0`;
   - first flank bridge, second flank not bridge: `edge *= 1.0`;
   - both flanks bridge: `edge *= 2.0`.
5. Otherwise return `edge`.
6. Caller computes `edge * PathfinderClass+0x04 + DirectionEpsilon[dir]`.

Active in YR: Yes. Evidence: decompile `0x00429830`; caller `0x00429F8A..0x00429F9D`; assembly context shows `CALL 0x00429830`, then `FMUL [ESI+0x4]`, then `FADD [0x81872c + dir*4]`.

### 3.2 Base Table Is Can-Enter Code Cost

The helper compares the incoming code to `2`, then loads `0x0081870C[code]`:

| Code | Base cost | Scoped meaning from prior reports |
|---:|---:|---|
| 0 | `1.0` | clear / OK |
| 1 | `1000.0` | crushable |
| 2 | `1.0` | friendly moving; branch may replace |
| 3 | `1.0` | passable special / bridge ramp |
| 4 | `60.0` | friendly wall |
| 5 | `20.0` | enemy block |
| 6 | `8.0` | friendly stationary |
| 7 | `10000.0` | impassable, normally rejected by caller before opening |

Active in YR: Yes. Evidence: assembly `0x00429845 CMP EAX,0x2`; `0x00429848 FLD [EAX*4 + 0x81870c]`; byte read `0x0081870C`.

### 3.3 Code-2 Urgency Is Before Marker And Only Affects Code 2

`PathfinderClass+0x3C` is read only inside the code-2 branch in this helper. Value `0` runs the blocker-prediction loop; value `1` skips prediction and leaves the jam cost; value `2` overrides to `1000.0`.

The blocker path prediction chooses `dest+0xE4` or `dest+0xE8` by the bridge-layer argument, walks at most 10 hops, and uses the asymmetric level test already documented in prior reports. This slice rechecked placement, not the whole blocker-list lifecycle.

Active in YR: Yes. Evidence: branch starts after `0x0042985C JNZ 0x004299AA`; list selection at `0x00429862..0x00429878`; urgency reads at `0x00429878` and `0x00429995`; setter in `AStar_pathfind_search @ 0x0042C900` writes `*(this+0x3C)=param_8`.

### 3.4 Temporary `0x40000` Marker Multiplies The Current Edge Accumulator

The marker multiply is not a passability gate and not a terrain/cliff rule:

`if (dest.flags & 0x40000) edge *= 4.0`

It runs after code-2 adjustment joins at `0x004299AA`, so it stacks with code-2 and other base-table costs:

| Prior edge | Marked edge before bridge flank |
|---:|---:|
| clear `1.0` | `4.0` |
| code-2 jam `4.0` | `16.0` |
| code-2 urgency 2 `1000.0` | `4000.0` |
| enemy block `20.0` | `80.0` |
| stationary friendly `8.0` | `32.0` |

Active in YR: Conditional, yes when the marker writer toggles the destination. Evidence: `0x004299AA MOV EDX,[EBX+0x140]`; `0x004299B0 TEST EDX,0x40000`; `0x004299BC FMUL [0x007e37bc]`; byte read `0x007E37BC = 4.0`.

### 3.5 Bridge Flank / Structural Flag Multiplier Is After Marker

The bridge branch is gated by both inputs:

- entering bridge layer: helper argument `param_4 != 0`;
- `PathfinderClass+0x01 != 0`.

It computes direction from `dest - source`, chooses an orientation table from destination `flags & 0x800`, reads two flanking cells using `dir` and `(dir - 4) & 7`, then checks each flank's structural bridge bit `0x100`.

| Flank condition | Multiplier | Evidence |
|---|---:|---|
| first flank is not structural bridge | `10.0` | `0x00429A41..0x00429A58`; constant `0x007E37B8` |
| first flank bridge, second not bridge | `1.0` | initial `FLD 0x007E2AC8` at `0x004299F1`, then falls to `0x00429A75` |
| both flanks bridge | `2.0` | `0x00429A65 TEST [EAX+0x140],0x100`; `0x00429A6F FLD 0x007E37B4` |

Active in YR: Conditional. The code is live in the helper; it only affects A* calls that enter bridge layer with the branch-enable byte set. Evidence: decompile and assembly `0x004299D2..0x00429A79`.

Tiny parity details:

- Orientation flag `0x800` only chooses the flank-offset table; the structural tests are still `0x100`.
- Cardinal moves degenerate because the two flank lookups can be the same cell, so the `2.0` both-flanks case is mainly a diagonal bridge-deck detail.
- A marked bridge edge stacks multiplicatively with this branch. Example: code-2 jam on a marked non-bridge-flank bridge entry is `4.0 * 4.0 * 10.0 = 160.0` before caller epsilon.

### 3.6 Direction 8 / Tube Bypass

The caller checks direction before calling the helper. If `dir == 8`, it jumps to a Chebyshev-distance tube path and never calls `AStar_compute_edge_cost`.

Therefore direction `8` does not receive:

- `Can_Enter_Cell` base table cost from this helper;
- code-2 urgency cost from this helper;
- `0x40000` marker x4;
- bridge flank multiplier;
- `PathfinderClass+0x04`;
- normal direction epsilon.

Active in YR: Conditional on tube/direction-8 expansion. Evidence: `0x00429F6B CMP [ESP+0x18],0x8`; `0x00429F70 JZ 0x00429FA3`; helper call only at `0x00429F8A`.

## 4. INI Keys

No INI key is read directly by `AStar_compute_edge_cost`.

| Input | Effect in this slice | Active in YR | Evidence |
|---|---|---|---|
| `BlockagePathDelay` / locomotor retry state | Upstream influence on the `0/1/2` urgency passed into `PathfinderClass+0x3C`; helper only reads the field | Yes, indirect | `PATHFINDERCLASS_FIELD_3C_GHIDRA_REPORT.md`; setter `0x0042C900` |
| Terrain speed costs | Not read by this helper; current helper uses `Can_Enter_Cell` code table | Terrain speed is live elsewhere, not here | no INI/string read in helper; base table load `0x00429848` |

## 5. Integration Points

| Integration point | Role | Active in YR | Evidence |
|---|---|---|---|
| `AStar_main_loop @ 0x00429A90` | Sole direct caller; computes direction/layer and adds caller-side multiplier/epsilon | Yes | xref `0x00429F8A`; caller context `0x00429F6B..0x00429F9D` |
| `PathfinderClass+0x3C` setter | Stores caller urgency into the helper-readable field | Yes | `AStar_pathfind_search @ 0x0042C900` |
| `UpdateBridgePassability @ 0x0042ACF0` | Writes/clears `0x40000`; not re-investigated here | Conditional | existing writer report; consumer verified here |
| `PathfinderClass` constructor | Initializes `+0x01=0`, `+0x04=1.0`, `+0x3C=0` | Yes | decompile `0x0042A6D0`; assembly `0x0042A6E2`, `0x0042A6E8` |

## 6. Current Rust Implementation Status

Read-only scan only:

| Rust surface | Status vs verified helper |
|---|---|
| `src/sim/pathfinding/core.rs` constants | Has `CODE2_MULT_CLEARING=1`, `CODE2_MULT_JAM=4`, `CODE2_MULT_ROUTE_AROUND=1000`, `SEARCH_MARKER_COST_MULTIPLIER=4`, and binary-shaped direction tiebreak constants. |
| `compute_code2_multiplier` | Models urgency `0/1/2` and 10-hop chain on a selected layer. Exact binary list-order and asymmetric height follow remain dependent on the upstream entity block map. |
| `SearchMarkerOverlay` / `apply_search_marker_cost` | Models search-scoped XOR marker and applies x4 after entity/code cost and before `DIR_TIEBREAK`. This matches the scoped marker order. |
| direction 8 / tube branch | Existing explicit tube branch uses separate cost and bypasses marker overlay. This matches the helper bypass principle for explicit tube edges. |
| bridge flank multiplier | No direct equivalent found for the binary `10.0 / 1.0 / 2.0` bridge flank multiplier keyed by `0x100/0x800`. Current Rust diagonal bridge movement uses a walkability corner-cut check instead. |
| generic height/cliff multiplier | Rust still has `CLIFF_COST_MULTIPLIER=4` for effective height changes. That is a separate Rust rule and must not be conflated with `0x40000`; binary marker x4 is destination-flag-driven, not generic height-change-driven. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `AStar_compute_edge_cost @ 0x00429830` base table | verified | decompile; assembly `0x00429845..0x00429854`; bytes `0x0081870C` | none |
| Code-2 urgency placement | verified | decompile; assembly `0x0042985C..0x004299A6`; field report | upstream blocker-list construction remains separate |
| `0x40000` marker placement | verified | assembly `0x004299AA..0x004299C2`; bytes `0x007E37BC` | writer geometry out of scope |
| Bridge flank multiplier | verified | decompile; assembly `0x004299D2..0x00429A79`; table bytes | exact setter for `Pathfinder+0x01` not resolved in this narrow slice |
| Direction-8 bypass | verified | assembly `0x00429F6B..0x00429FA3` | full TubeClass lifecycle out of scope |
| Caller-side `+0x04` and epsilon order | verified | assembly `0x00429F8A..0x00429F9D`; constructor `0x0042A6D0` | none |
| Rust current status | touched-not-exhausted | `rg` and line scan of `core.rs/core_tests.rs` | future implementation only |

## 8. Open Questions - Final State Of The Investigation Log

- `[RESOLVED] OQ-1 - Is the target bounded enough for exhaustive-slice? -> Yes, one helper plus its immediate caller add/bypass site.` (evidence: `0x00429830`, `0x00429F6B..0x00429F9D`)
- `[RESOLVED] OQ-2 - Is this helper active in YR? -> Yes; sole caller is live `AStar_main_loop`.` (evidence: xref from `0x00429F8A`; Active in YR: Yes)
- `[RESOLVED] OQ-3 - What is the base cost source? -> 8-float table at `0x0081870C`, indexed by `Can_Enter_Cell` code.` (evidence: `0x00429848`; byte read)
- `[RESOLVED] OQ-4 - Does moving-friendly urgency affect all costs? -> No; in this helper it only affects code 2.` (evidence: `0x0042985C` branch; reads at `0x00429878`, `0x00429995`)
- `[RESOLVED] OQ-5 - Does marker x4 happen before or after code-2? -> After code-2 branch joins.` (evidence: code-2 branch `0x0042985C..0x004299A6`; marker starts `0x004299AA`)
- `[RESOLVED] OQ-6 - Does marker x4 happen before bridge flank multipliers? -> Yes.` (evidence: marker `0x004299AA..0x004299C2`; bridge gate starts `0x004299C6/0x004299D2`)
- `[RESOLVED] OQ-7 - Which bridge flags matter inside helper? -> destination `0x800` chooses flank table; flank cells' `0x100` choose multiplier; destination `0x40000` applies marker.` (evidence: `0x004299AA`, `0x004299D2..0x00429A75`)
- `[RESOLVED] OQ-8 - Are there bridge structural flags beyond those in scope? -> No other bridge flag was read inside `0x00429830`.` (evidence: full decompile of helper)
- `[RESOLVED] OQ-9 - Does direction epsilon get multiplied by marker/flank costs? -> No; it is caller-side additive after helper return and `+0x04` multiply.` (evidence: `0x00429F8A..0x00429F9D`)
- `[RESOLVED] OQ-10 - Does direction 8 use this helper? -> No; caller jumps around helper for direction 8.` (evidence: `0x00429F6B..0x00429FA3`)
- `[RESOLVED] OQ-11 - Are INI keys read directly by this helper? -> No.` (evidence: full helper decompile; no string/Rules reads)
- `[RESOLVED] OQ-12 - What is the current Rust delta? -> marker/code2/tube-bypass surfaces exist; bridge flank multiplier is missing/unchecked; height x4 must not be treated as marker parity.` (evidence: `src/sim/pathfinding/core.rs` scan)
- `[DEFERRED] OQ-13 - Where exactly is `PathfinderClass+0x01` set for all live callers?` (category: out-of-scope; reason: this slot targets edge-cost effects, not PathfinderClass lifecycle; next-step-if-pursued: dedicated field-setter xref pass)
- `[DEFERRED] OQ-14 - Exact peer object-list ordering feeding code-2 when multiple blockers share a layer.` (category: out-of-scope; reason: requires object-list construction/alt-list slot or slot-1 findings; next-step-if-pursued: combine with `FUN_0042B080` and object-list insertion reports)
- `[DEFERRED] OQ-15 - Full TubeClass lifecycle for direction 8.` (category: out-of-scope; reason: helper bypass is proven, but tube creation/path semantics are separate; next-step-if-pursued: TubeClass lifecycle investigation)

Adversarial checks answered:

- Marked code-2 urgency-2 bridge edge: `1000 * 4 * bridge_mult + epsilon`, not `1000 + 4`.
- Marked code-5 enemy blocker: `20 * 4 * bridge_mult + epsilon`, not a hard block.
- Direction-8/tube edge into a marked destination: no marker cost from this helper path.
- Flank structural failure: first flank not `0x100` yields `10.0` even if destination is a bridge cell.
- Direction epsilon: remains additive and outside all helper multipliers.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `0x40000` marker multiplies the current edge accumulator by `4.0` after code-2/entity adjustment and before bridge flank multiplier. | `0x0042985C..0x004299C2`; bytes `0x007E37BC` | mostly present for marker/code2 order | `src/sim/pathfinding/core.rs`, `src/sim/pathfinding/core_tests.rs` | Preserve marker as search-scoped destination cost overlay after entity/code cost and before final tiebreak. | `astar_edge_cost_marker_stacks_after_code2_before_bridge_flank`: marked code-2 jam bridge edge has code2 x marker x bridge multiplier. | Do not bake this into static `PathGrid`, cliff/height cost, or walkability. |
| Bridge-layer diagonal/flank cost uses destination orientation `0x800` and flank structural bridge bit `0x100`, producing multipliers `10.0`, `1.0`, or `2.0`. | `0x004299D2..0x00429A79`; bytes `0x007E3710`, `0x007E3730`, `0x007E37B4/B8`, `0x007E2AC8` | missing/unchecked; Rust has bridge diagonal walkability checks but no equivalent multiplier | `src/sim/pathfinding/core.rs`, bridge-layer A* cost tests | Add bridge-layer cost fixtures that distinguish first-flank-not-bridge, one-flank-bridge, both-flanks-bridge, and orientation table selection. | `astar_bridge_flank_cost_penalizes_diagonal_shortcut_to_bridgehead`: diagonal shortcut with first flank off-bridge loses to cardinal bridgehead entry. | Do not replace this with a simple diagonal-blocking rule; binary penalizes, it does not always reject. |
| Direction epsilon is caller-side additive after helper return and `Pathfinder+0x04`; marker and bridge flank multipliers do not scale it. | `0x00429F8A..0x00429F9D`; bytes `0x0081872C` | currently matched for marker tests; must remain true after flank multiplier work | `src/sim/pathfinding/core.rs`, `core_tests.rs` | Keep `DIR_TIEBREAK` final and outside all multiplicative costs. | `astar_bridge_flank_marker_cost_does_not_scale_direction_tiebreak`: two paths with same multiplied edge cost differ only by epsilon. | Do not fold tiebreak into base step before multiplier. |
| Direction 8 bypasses helper completely. | `0x00429F6B..0x00429FA3` | existing explicit tube branch bypasses marker overlay; keep this invariant | `src/sim/pathfinding/core.rs` tube/explicit edge branch | Restrict marker/flank/entity helper parity to normal compass edges unless another binary finding proves a tube-specific cost. | `astar_marker_overlay_does_not_apply_to_direction8_tube_edge` remains valid with bridge flank work. | Do not globally post-process every destination edge with marker or flank costs. |
| `PathfinderClass+0x3C` urgency is code-2-only inside this helper and value `2` overrides to `1000.0`. | `0x00429878`, `0x00429995`; field setter report | Rust has `urgency` input and code2 multipliers; upstream value generation remains broader movement work | `src/sim/pathfinding/core.rs`; movement caller that builds `AStarOptions` | Keep urgency from affecting only moving-friendly code-2 costs in this helper surface. | `astar_urgency_two_routes_around_code2_only`: urgency 2 raises friendly-moving blocker cost without changing a clear marked cell except via marker overlay. | Do not use urgency as a global terrain/bridge multiplier. |

### Negative Facts / Do Not Do

- Do not call `0x40000` a permanent cliff/ramp/height flag. Active in YR: Yes for A* marker; evidence `0x004299AA..0x004299C2` and writer reports.
- Do not apply marker/flank/entity costs to direction 8. Active in YR: Conditional tube path; evidence `0x00429F6B..0x00429FA3`.
- Do not multiply direction epsilon by marker, code-2, or bridge flank costs. Active in YR: Yes; evidence `0x00429F8A..0x00429F9D`.
- Do not collapse bridge flank logic into "diagonal blocked." Active in YR: Yes; helper returns costs `10.0/1.0/2.0`, not a boolean.
- Do not let `Pathfinder+0x3C` affect enemy/stationary/clear cells inside this helper. Active in YR: Yes; reads are inside the code-2 branch.
- Do not treat `0x800` as the structural bridge bit. Active in YR: Yes; `0x800` selects orientation table, while flank structural checks use `0x100`.

### Stale Docs / Follow-up Docs

No contradiction found with the recent marker-stacking report. Older docs that still call `0x40000` a cliff/ramp penalty remain stale; use the wording from `ASTAR_COMPUTE_EDGE_COST_00429830_MARKER_STACKING_GHIDRA_REPORT.md`: destination `0x40000` is a temporary A* marker multiplier, not terrain height cost.

## 10. Remaining Uncertainty

No remaining uncertainty for the claimed helper formula/order. Deferred items are deliberately outside this slot: `Pathfinder+0x01` full setter lifecycle, object-list ordering feeding code-2 when several blockers share a layer, and full TubeClass behavior after the proven helper bypass.

## Sources

- Ghidra decompile: `AStar_compute_edge_cost @ 0x00429830`
- Ghidra decompile: `AStar_main_loop @ 0x00429A90`
- Ghidra decompile: `AStar_pathfind_search @ 0x0042C900`
- Ghidra decompile: `PathfinderClass__Constructor @ 0x0042A6D0`
- Assembly contexts: `0x00429845`, `0x0042985C`, `0x004299AA`, `0x004299D2`, `0x00429A41`, `0x00429A52`, `0x00429A6F`, `0x00429F6B`, `0x00429F8A`, `0x00429F96`
- Memory reads: `0x0081870C`, `0x0081872C`, `0x007E3710`, `0x007E37B4`, `0x007E37B8`, `0x007E37BC`, `0x007E37C0`, `0x007E2AC8`
- Existing reports: `BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md`, `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md`, `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md`, `ASTAR_COMPUTE_EDGE_COST_00429830_MARKER_STACKING_GHIDRA_REPORT.md`, `BRIDGE_PATH_TIE_ORDER_AFTER_LOW_COLLAPSE_GHIDRA_REPORT.md`, `PATHFINDERCLASS_FIELD_3C_GHIDRA_REPORT.md`
- Rust scan: `C:/Users/enok/Documents/ra2-rust-game/src/sim/pathfinding/core.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/pathfinding/core_tests.rs`
