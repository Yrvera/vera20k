# AStar_compute_edge_cost 0x00429830 Marker Stacking - Ghidra Research Report

**Address(es):** `0x00429830` primary edge-cost helper; caller cost-add site `0x00429F7C..0x00429F9D`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** exact placement/order of the temporary `CellClass+0x140 & 0x40000` cost marker within `AStar_compute_edge_cost` inputs and caller-side final edge add.
**Non-Scope:** `PathfinderClass::UpdateBridgePassability @ 0x0042ACF0` marker writer geometry, peer-path selection, and broad A* correctness outside this cost-stacking slice.
**Confidence:** High for formula/order; evidence is existing Ghidra decompile reports plus direct read-only disassembly/byte-table verification from local `gamemd.exe`.
**Active in YR:** Yes - reached from standard A* neighbor expansion in `AStar_main_loop`, which is on the live ground/bridge movement path.

## 0. Working Notes

Target question: exact stacking/order of temporary `CellClass+0x140` bit `0x40000` inside `AStar_compute_edge_cost @ 0x00429830`.

Non-goals: marker writer geometry, broad bridge passability, Rust edits, INI edits, and unrelated pathfinder fields.

Evidence needed to mark COMPLETE: binary-backed formula/order evidence covering base/Can_Enter cost, code-2/entity costs, `0x40000`, bridge flank multipliers, `Pathfinder+0x3C`, `Pathfinder+0x04`, direction epsilon, and current Rust delta.

Stop conditions: every scoped input resolved/deferred with evidence; a zero-add pass over `0x00429830` and the caller cost site adds no new scoped questions; report written to the required path.

## 1. Overview

`AStar_compute_edge_cost @ 0x00429830` computes the multiplicative part of a normal 8-direction compass A* edge. The caller then multiplies the returned edge cost by `PathfinderClass+0x04` and adds the direction epsilon. Direction `8` tube/bridge-crossing edges bypass this helper and do not receive the `0x40000` marker cost or direction epsilon.

The temporary marker is not a separate pre-filter and not a static terrain type. It multiplies the current edge-cost accumulator after base/Can_Enter/code-2 adjustment, before optional bridge flank multiplier selection, and before caller-side `+0x04` and direction epsilon.

## 2. Class Layout / Key Offsets

| Offset / address | Type | Purpose | Active in YR | Evidence |
|---|---:|---|---|---|
| `CellClass+0x140` bit `0x40000` | flag bit | Temporary marker cost multiplier input; cost x4 when set on destination cell | Yes, conditional on search-scoped marker being present | `0x004299AA..0x004299C2`; writer/cleanup liveness in `ASTAR_0X40000_CLEANUP_TAILS_GHIDRA_REPORT.md` |
| `CellClass+0x140` bit `0x100` | flag bit | Structural bridge-cell/flank test inside bridge cost branch | Yes on bridge cells | `0x00429A41..0x00429A75`; bridge report section 3.5 |
| `CellClass+0x140` bit `0x800` | flag bit | Selects one of the bridge flank offset tables | Yes on bridge cells | `0x00429A02..0x00429A37` |
| `PathfinderClass+0x01` | byte | Enables bridge flank cost branch when entering bridge layer | Yes, conditional | `0x004299D2..0x004299D7`; prior bridge A* report |
| `PathfinderClass+0x04` | float | Caller-side cost multiplier; constructor writes `1.0f` | Yes; value is a runtime no-op in stock YR | write `0x0042A6EB`; read `0x00429F8F` |
| `PathfinderClass+0x3C` | DWORD | Per-search urgency for Can_Enter code 2 | Yes for standard Find_Path calls | write `0x0042C927..0x0042C92F`; reads `0x00429878`, `0x00429995` |
| `0x0081870C` | 8 floats | Can_Enter return-code base cost table | Yes | local byte read; helper load `0x00429845..0x00429854` |
| `0x0081872C` | 9 floats | Caller-side direction epsilon table | Yes for directions 0-7; direction 8 value is not used by helper path | local byte read; caller add `0x00429F96` |
| `0x007E37BC` | float `4.0` | `0x40000` marker multiplier | Yes, conditional | local byte read; helper multiply `0x004299B8..0x004299C2` |

## 3. Core Logic

### 3.1 Exact stacking formula

For normal compass directions `0..7`, the binary formula is:

1. `edge = EdgeCostBaseTable[can_enter_code]`.
2. If `can_enter_code == 2`, replace `edge` according to the code-2 urgency/prediction branch:
   - urgency `0`: prediction may leave `edge = 1.0` or set `edge = 4.0`;
   - urgency `1`: prediction is skipped and `edge = 4.0`;
   - urgency `2`: after the `4.0` write, override to `edge = 1000.0`.
3. If destination `CellClass+0x140 & 0x40000`, set `edge = edge * 4.0`.
4. If entering bridge layer and `Pathfinder+0x01 != 0`, compute bridge flank multiplier from destination orientation and flank cells, then return `edge * bridge_mult`, where `bridge_mult` is `10.0`, `1.0`, or `2.0`.
5. Otherwise return `edge`.
6. Caller computes `step = returned_edge * *(float *)(Pathfinder+0x04) + DirectionEpsilon[dir]`.

In stock YR, `Pathfinder+0x04` is initialized to `1.0f` at startup and no later pathfinding writer was documented in the cited cost-multiplier report, so the observable formula is normally `step = returned_edge + DirectionEpsilon[dir]`.

Active in YR: Yes. Evidence: `AStar_main_loop` caller sequence at `0x00429F7C..0x00429F9D`; edge helper branch order at `0x00429845..0x00429A86`; constructor write `0x0042A6EB`.

### 3.2 Base/terrain cost is Can_Enter code cost, not terrain speed

`0x00429845..0x00429854` compares the incoming Can_Enter code to `2` and immediately loads `0x0081870C[code]` into the edge-cost slot. The verified table is:

| Code | Cost |
|---:|---:|
| 0 | `1.0` |
| 1 | `1000.0` |
| 2 | `1.0` base, then code-2 branch may replace |
| 3 | `1.0` |
| 4 | `60.0` |
| 5 | `20.0` |
| 6 | `8.0` |
| 7 | `10000.0` |

Active in YR: Yes. Evidence: local byte read of `0x0081870C`; decompile and table in `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md` section 3.2. The caller rejects codes `>= 7` before opening the node, so code 7's table value is not normally added as a reachable compass edge cost.

### 3.3 Code-2 / entity cost happens before marker cost

The code-2 branch is entered only when the incoming Can_Enter code equals `2`. It selects `dest+0xE4` or `dest+0xE8` based on the bridge-layer argument, uses `Pathfinder+0x3C` to decide whether to run prediction, writes `4.0` on jam/urgency, and writes `1000.0` only when `Pathfinder+0x3C == 2`. Only after this branch joins at `0x004299AA` does the helper read destination flags and test `0x40000`.

Active in YR: Yes. Evidence: direct assembly order `0x00429845..0x004299A6`, then marker test/multiply `0x004299AA..0x004299C2`; `PATHFINDERCLASS_FIELD_3C_GHIDRA_REPORT.md` setter/caller chain.

### 3.4 Marker cost placement

The marker multiply is:

`if (dest.flags & 0x40000) edge *= 4.0`

It multiplies the already-adjusted current edge-cost slot. That means the marker stacks with every Can_Enter-code cost and code-2 urgency result:

| Prior edge value | Marker result |
|---:|---:|
| clear/code 0 `1.0` | `4.0` |
| code 2 clearing `1.0` | `4.0` |
| code 2 jam/urgency 1 `4.0` | `16.0` before any bridge flank multiplier |
| code 2 urgency 2 `1000.0` | `4000.0` before any bridge flank multiplier |
| code 5 enemy `20.0` | `80.0` |
| code 6 stationary ally `8.0` | `32.0` |

Active in YR: Yes, conditional on marker presence. Evidence: `0x004299AA` reads `dest+0x140`; `0x004299B0` tests `0x40000`; `0x004299B8..0x004299C2` multiplies the edge slot by `0x007E37BC == 4.0`; marker setup/cleanup liveness in `ASTAR_0X40000_CLEANUP_TAILS_GHIDRA_REPORT.md`.

### 3.5 Bridge flank multiplier happens after marker cost

After the marker join, the helper checks the bridge-layer argument and `Pathfinder+0x01`. If both are nonzero, it:

1. computes direction from `dest - source`;
2. selects a flank table based on destination `flags & 0x800`;
3. reads two flank cells using `dir` and `(dir - 4) & 7`;
4. returns `edge * 10.0` when the first flank is not a structural bridge;
5. returns `edge * 1.0` when the first flank is structural bridge and the second is not;
6. returns `edge * 2.0` when both flanks are structural bridge.

Because the branch multiplies the same edge slot that the marker branch already changed, a marked bridge edge stacks multiplicatively with the bridge flank multiplier. Examples: marked clear edge with both flanks bridge is `1.0 * 4.0 * 2.0 = 8.0`; marked code-2 jam with non-bridge first flank is `4.0 * 4.0 * 10.0 = 160.0`, before caller-side `+0x04` and epsilon.

Active in YR: Yes, conditional on entering bridge layer and `Pathfinder+0x01 != 0`. Evidence: marker branch ends before bridge checks at `0x004299C6`; bridge multiplier reads/use at `0x004299D2..0x00429A79`; constants byte-read as `2.0`, `10.0`, `1.0`.

### 3.6 Direction tiebreak epsilon is outside the helper

The caller at `0x00429F8A` calls `0x00429830`, then immediately multiplies the FPU return by `[Pathfinder+0x04]` and adds `0x0081872C[direction]`. This means the direction epsilon does not get multiplied by the marker, code-2, bridge flank multiplier, or `Pathfinder+0x04`.

Direction table values verified from bytes:

`[0.001000000047, 0.004999999888, 0.002000000095, 0.006000000052, 0.003000000026, 0.007000000216, 0.004000000190, 0.008000000380, 0.0]`

Active in YR: Yes for compass directions `0..7`; direction `8` takes a separate caller branch. Evidence: `0x00429F6B..0x00429FA3`; local byte read of `0x0081872C`.

### 3.7 Direction 8 bypass

When the expansion direction is `8`, the caller branches away at `0x00429F6B..0x00429FA3` and does not call `AStar_compute_edge_cost`. Therefore direction-8 tube/bridge-crossing edges do not receive the `0x40000` marker multiply, bridge flank multiplier, `Pathfinder+0x04`, or normal direction epsilon on this path.

Active in YR: Yes, conditional on direction-8 tunnel/tube edge expansion. Evidence: caller branch `cmp [esp+0x18], 8; je 0x00429FA3`.

## 4. INI Keys

No INI key is read by `AStar_compute_edge_cost`.

| Key | Role in this slice | Active in YR | Evidence |
|---|---|---|---|
| `BlockagePathDelay` | Upstream locomotor timing controls when `Pathfinder+0x3C` becomes `1` or `2`; the edge helper only reads the resulting field. | Yes, via movement/repath callers | `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md`; `PATHFINDERCLASS_FIELD_3C_GHIDRA_REPORT.md` |
| Terrain speed costs | Not read by `0x00429830`; current helper uses Can_Enter return code table. | Terrain passability is live elsewhere, but not as an edge-speed multiplier here | no string/INI read in helper; helper load from `0x0081870C` |

## 5. Integration Points

| Integration point | Status | Active in YR | Evidence |
|---|---|---|---|
| `AStar_main_loop @ 0x00429A90` normal compass expansion calls helper | verified | Yes | `0x00429F7C..0x00429F9D` |
| `Can_Enter_Cell` return code supplies base table index | verified | Yes | caller passes code; helper load at `0x00429845..0x00429854`; prior UnitClass report |
| `Pathfinder+0x3C` written per pathfind attempt | verified | Yes | setter `0x0042C927..0x0042C92F`; field report |
| `Pathfinder+0x04` default/source | verified from prior and spot disassembly | Yes but no-op (`1.0`) | constructor `0x0042A6EB`; caller read `0x00429F8F` |
| `UpdateBridgePassability @ 0x0042ACF0` marker lifecycle | touched, not re-investigated | Yes, conditional | cleanup report and update report; writer geometry out of scope |

## 6. Current Rust Implementation Status

Current Rust path cost is in `src/sim/pathfinding/core.rs`.

Observed surfaces:

- `DIR_TIEBREAK` at lines 125-139 matches the binary epsilon ordering under the Rust integer scale.
- `CODE2_*`, `CODE5_MULT_ENEMY`, and `CODE6_MULT_STATIONARY_ALLY` at lines 51-65 represent current entity cost multipliers.
- Normal compass edge cost construction at lines 804-868 applies terrain cost, then a height-change `CLIFF_COST_MULTIPLIER`, then entity multiplier, then direction tiebreaker.
- No `0x40000`, `BridgeApproach`, temporary marker overlay, or equivalent per-search marker cost surface was found under `src/sim/pathfinding`, `src/map`, or `src/sim`.

Rust delta: current Rust has no search-scoped marker overlay and currently places a generic height-change/cliff multiplier before entity cost. The binary's `0x40000` multiplier is not a generic height-change cost; it is a destination-cell temporary marker multiplier applied after Can_Enter/code-2 adjustment and before bridge flank multiplier / caller epsilon.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `AStar_compute_edge_cost @ 0x00429830` base table load | verified | `0x00429845..0x00429854`; byte read `0x0081870C` | none |
| Code-2 urgency/prediction placement | verified | `0x0042985C..0x004299A6`; `PATHFINDERCLASS_FIELD_3C_GHIDRA_REPORT.md` | exact marker-writer geometry out of scope |
| `0x40000` marker multiply placement | verified | `0x004299AA..0x004299C2`; byte read `0x007E37BC` | none |
| Bridge flank multiplier placement | verified | `0x004299C6..0x00429A79`; table byte reads | none for stacking; writer/orientation semantics beyond branch input out of scope |
| Caller `Pathfinder+0x04` and direction epsilon | verified | `0x00429F8A..0x00429F9D`; constructor write `0x0042A6EB`; byte read `0x0081872C` | none |
| Direction 8 bypass | verified | `0x00429F6B..0x00429FA3` | none |
| `Pathfinder+0x3C` setter and value range | verified via prior report + spot disassembly | `0x0042C927..0x0042C92F`; `PATHFINDERCLASS_FIELD_3C_GHIDRA_REPORT.md` | none |
| Marker writer geometry | deferred | non-scope per parent constraints | separate slot/reports cover `0x0042ACF0` |
| Rust implementation status | verified by source scan | `core.rs` lines 47-65, 804-868; `rg` for marker terms | implementation intentionally not changed |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Is the target slice bounded enough for exhaustive mode? -> Yes, one helper plus caller-side final add.` (evidence: target address and caller `0x00429F7C..0x00429F9D`)
- `[RESOLVED] OQ-2 - What is the exact base cost source? -> `0x0081870C[Can_Enter_Cell_code]`, not an INI terrain-speed lookup.` (evidence: `0x00429845..0x00429854`; byte read `0x0081870C`)
- `[RESOLVED] OQ-3 - Does code-2 entity adjustment happen before or after marker? -> Before marker.` (evidence: code-2 branch `0x0042985C..0x004299A6`; marker starts `0x004299AA`)
- `[RESOLVED] OQ-4 - Does `Pathfinder+0x3C` affect only code 2? -> In this helper, yes; reads are inside the code-2 branch.` (evidence: `0x00429878`; `0x00429995`; field report)
- `[RESOLVED] OQ-5 - Does marker multiply every current edge value or only clear terrain? -> Every current edge accumulator value reaching `0x004299AA`.` (evidence: single join into marker branch `0x004299AA..0x004299C2`)
- `[RESOLVED] OQ-6 - Does marker run before bridge flank multipliers? -> Yes.` (evidence: marker `0x004299B0..0x004299C2`; bridge gate starts `0x004299C6`)
- `[RESOLVED] OQ-7 - Does direction epsilon get multiplied by marker/bridge costs? -> No; epsilon is caller-side add after helper return and `+0x04` multiply.` (evidence: `0x00429F8A..0x00429F9D`)
- `[RESOLVED] OQ-8 - What is `Pathfinder+0x04` placement and default? -> Caller multiplies helper return by it; constructor writes `1.0f`.` (evidence: read `0x00429F8F`; write `0x0042A6EB`)
- `[RESOLVED] OQ-9 - Does direction 8 use marker stacking? -> No, caller branches around helper for direction 8.` (evidence: `0x00429F6B..0x00429FA3`)
- `[RESOLVED] OQ-10 - Is `0x40000` static terrain/cliff in this slice? -> No; it is read as a destination flag multiplier, with temporary lifecycle documented elsewhere.` (evidence: `ASTAR_0X40000_CLEANUP_TAILS_GHIDRA_REPORT.md`; marker read `0x004299AA`)
- `[DEFERRED] OQ-11 - Which exact cells are written by `0x0042ACF0`?` (category: out-of-scope; reason: parent explicitly excluded marker writer geometry; next-step-if-pursued: use/update the dedicated `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-12 - What is the Rust-facing surface? -> future per-search pathfinding overlay/cost input in `src/sim/pathfinding/core.rs`, not static `PathGrid`.` (evidence: Rust scan; cleanup-tail report)
- `[RESOLVED] OQ-13 - Are there INI defaults needed inside the helper? -> No direct INI read in helper; only upstream `BlockagePathDelay` affects `+0x3C`.` (evidence: helper disassembly; entity/field reports)
- `[RESOLVED] OQ-14 - Is TS legacy gating present for this helper? -> No TS-only gate found in this slice; branch is reached from live standard A* path, with bridge behavior conditional on live layer/flags.` (evidence: caller `0x00429F7C..0x00429F9D`; bridge docs)
- `[RESOLVED] OQ-15 - Could a same-value shortcut use a flat x4 anywhere? -> No; placement affects stacking with code-2 urgency, bridge flank multiplier, and epsilon non-multiplication.` (evidence: formula order above)

Adversarial checks answered:

- Marked code-2 urgency-2 bridge edge? `1000 * 4 * bridge_mult + epsilon`, not `1000 + marker`.
- Marked enemy blocker? `20 * 4` before bridge flank multiplier, not hard-block.
- Direction-8 marked destination? No helper call on that branch.
- `Pathfinder+0x04` non-1 hypothetical? It would multiply the already marker/bridge-adjusted helper return, not epsilon.
- Generic height change without marker? No marker cost in binary helper unless destination `0x40000` bit is set.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Temporary `0x40000` marker multiplies the current edge accumulator by `4.0` after Can_Enter/code-2 adjustment and before bridge flank multiplier. | `0x00429845..0x004299C2`; `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md` section 3.4 | missing | `src/sim/pathfinding/core.rs` normal compass edge cost | Add a per-search marker overlay/input whose destination-cell hit multiplies the edge after entity/code cost and before bridge flank cost, using the Rust integer scale. | A marked destination occupied by a moving-friendly code-2 jam on a bridge flank route costs code2 jam x marker x bridge flank, while an unmarked equivalent lacks only the marker factor; proposed test name: `astar_edge_cost_marker_stacks_after_code2_before_bridge_flank`. | Do not store this in persistent `PathGrid` or make it a terrain-height/cliff rule. |
| Direction epsilon is added after helper return and `Pathfinder+0x04`; it is not multiplied by marker, entity, or bridge multipliers. | caller `0x00429F8A..0x00429F9D`; byte read `0x0081872C` | mostly matched for existing integer tiebreaker, but marker path must preserve placement | `core.rs` cost addition around direction tiebreak | Keep `DIR_TIEBREAK` as a final additive term outside all multiplicative marker/entity/bridge costs. | Two equal routes differing only by direction order preserve N/E/S/W lower epsilon even when one candidate is marker-adjusted; proposed test name: `astar_marker_cost_does_not_scale_direction_tiebreak`. | Do not fold epsilon into `step_cost` before multiplying marker/bridge cost. |
| Direction 8 bypasses `AStar_compute_edge_cost`; no marker, bridge flank, `+0x04`, or normal epsilon applies on that branch. | caller branch `0x00429F6B..0x00429FA3`; prior cell-entry report section 1.5 | current Rust tube edge is separate; marker future work must not affect it | `core.rs` explicit tube edge branch | Keep future marker overlay restricted to normal compass neighbor expansion unless a separate binary finding proves tube-edge marker behavior elsewhere. | A marked cell reached by explicit tube edge keeps tube cost unchanged while a normal compass edge into the same marked cell gets marker cost; proposed test name: `astar_marker_overlay_does_not_apply_to_direction8_tube_edge`. | Do not globally post-process every destination cell cost with marker. |

### Negative Facts / Do Not Do

- Do not implement `0x40000` as generic height-change or cliff terrain cost. Evidence: helper tests only `dest+0x140 & 0x40000` at `0x004299B0`, and height/layer checks are separate caller/Can_Enter concerns.
- Do not apply marker after adding `DIR_TIEBREAK`. Evidence: epsilon add occurs only in caller at `0x00429F96` after helper return.
- Do not apply marker to direction-8 tube/bridge-crossing branch. Evidence: caller jumps to `0x00429FA3` when direction is `8`, bypassing the helper call.
- Do not replace entity costs with a flat marker-only penalty. Evidence: code-2 urgency and base table selection complete before marker at `0x004299AA`; marker multiplies those results.
- Do not treat bridge flank multiplier as an alternative to marker. Evidence: marker branch writes the edge slot before the bridge branch multiplies that same slot at `0x00429A52`, `0x00429A75`.

### Stale Docs / Follow-up Docs

- `docs/research/PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md`: replace heading/text "`Cliff Ramp Multiplier` / `If cell+0x140 has bit 0x40000 set`" with "`Temporary marker multiplier` / `If destination CellClass+0x140 has bit 0x40000 set, the current edge accumulator is multiplied by 4.0 after Can_Enter/code-2 adjustment and before bridge flank multipliers.`"
- `docs/research/PATHFINDING_ASTAR_GHIDRA_REPORT.md`: replace "`Cliff ramp penalty: If cell flags contain 0x40000 (cliff ramp)`" with "`Temporary marker penalty: If the destination cell flags contain search-scoped bit 0x40000, multiply the current edge accumulator by 4.0; this is not a generic cliff/height transition rule.`"
- `docs/research/ADDRESS_MAP.md`: replace "`0x007E37BC | CliffRampMultiplier | 4.0f (cell_flags & 0x40000)`" with "`0x007E37BC | AStar marker cost multiplier | 4.0f applied when destination CellClass+0x140 & 0x40000`."

## 10. Remaining Uncertainty

None for the claimed stacking/order slice. Marker writer geometry remains intentionally out of scope and should be handled only by the dedicated `0x0042ACF0` reports.

## Sources

- Direct read-only disassembly/byte reads from local `<ra2-install>/gamemd.exe`:
  - `0x00429830..0x00429A90` - full helper branch order.
  - `0x00429F60..0x00429FB0` - caller call/multiply/epsilon and direction-8 bypass.
  - `0x0042A6D0..0x0042A700` - `Pathfinder+0x04 = 1.0f` constructor write.
  - `0x0042C900..0x0042C940` - `Pathfinder+0x3C` setter.
  - `0x0081870C`, `0x0081872C`, `0x007E37B4`, `0x007E37B8`, `0x007E37BC`, `0x007E2AC8`, `0x007E3710`, `0x007E3730`, `0x007E3750` - constant/table bytes.
- `docs/research/BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md`
- `docs/research/ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md`
- `docs/research/PATHFINDERCLASS_FIELD_3C_GHIDRA_REPORT.md`
- `docs/research/PATHFINDERCLASS_COST_MULTIPLIER_GHIDRA_REPORT.md`
- `docs/research/ASTAR_0X40000_CLEANUP_TAILS_GHIDRA_REPORT.md`
- `src/sim/pathfinding/core.rs`
