# Bridgehead Direct Damage Slot +3 Collapse - Ghidra Report

Date: 2026-05-23
Investigation mode: exhaustive-slice

## Scope

This report verifies one bounded bridge damage slice: direct cell damage routed by `ApplyDamageToCell` into the non-structural bridgehead branch of `ProcessBridgeDamageStateMachine_High` and `ProcessBridgeDamageStateMachine_Low`, specifically the tile-class slot `+3` collapse behavior.

In scope:

- `ApplyDamageToCell @ 0x00587180`
- `ProcessBridgeDamageStateMachine_High @ 0x00576BA0`
- `ProcessBridgeDamageStateMachine_Low @ 0x00571490`
- High and low bridgehead tile-class slots `+0`, `+1`, `+2`, `+3`
- Rust mismatch surface in `BridgeRuntimeState::bridgehead_advance_state`

Out of scope:

- Bridge repair hut death sweep behavior
- Raw high/low overlay `DestroyBridge_*` walkers, except for dispatch ordering
- Full structural bridge body state-machine behavior, except as contrast
- Exact internals of `CellClass__BlowUpBridge`
- Stock-map frequency scan for pre-existing slot `+3` bridgeheads

## Prior Work Read

- `HIGH_BRIDGE_COLLAPSE_STATE_MACHINE_GHIDRA_REPORT.md`
- `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`
- `LOW_BRIDGE_COLLAPSE_TUBE_ZONES_GHIDRA_REPORT.md`
- Current Rust source under `src/sim/bridge_state`, `src/sim/bridge_specs.rs`, and `src/sim/world/bridge_orchestrator.rs`

Prior reports correctly identified that high bridgehead slot `+3` can collapse. This investigation re-checked the live binary path and extended the check to the low bridgehead function.

## Open Questions Log

Resolved:

- Does `ApplyDamageToCell` route bridgehead tile-class hits into the state-machine functions? Yes, verified at `0x00587180`.
- Does high bridgehead slot `+3` collapse? Yes, verified at `0x00576BA0`.
- Does low bridgehead slot `+3` collapse? Yes, verified side effects at `0x00571490`.
- Are slots `+0..+2` only damage/advance states? Yes for this branch; they write slot `+2` and call DamageA/DamageB helpers, then return no-collapse.
- Does the high/low return value match? No. High bridgehead slot `+3` returns `1`; low bridgehead slot `+3` performs collapse side effects but falls through to return `0` in the decompile.

Resolved by follow-up:

- Whether any external caller relies on low bridgehead slot `+3` returning `0` after side effects. Yes. `BRIDGEHEAD_SLOT3_LOW_RETURN_VALUE_FOLLOWUP_GHIDRA_REPORT.md` verifies that `ApplyDamageToCell` callers use the boolean for retry loops and `TechnoClass__StopAllTargeting` eligibility.

Deferred:

- Stock-map frequency of bridgehead slot `+3` placements. Deferred because it needs a runtime/map corpus scan, not Ghidra.

## Binary Evidence

### 1. `ApplyDamageToCell` dispatch order

Evidence: `ApplyDamageToCell @ 0x00587180`. Confidence: high. Active in YR: yes, conditional on bridge damage reaching this helper.

The function first checks raw overlay byte at `Cell+0x44`:

- `0x4A < overlay < 0x64` routes to `DestroyBridge_Low`.
- `0xCC < overlay < 0xE7` routes to `DestroyBridge_High`.

Only if those raw overlay bands do not match does it inspect tile class at `Cell+0x38`.

For high state-machine dispatch, it computes:

- `relative = (Cell+0x38 - DAT_00aa0e28) + 1`

It accepts either a structural bridge flag path (`Cell+0x140 & 0x100`) or bridgehead tile classes:

- `DAT_00abad30 + 0`
- `DAT_00abad30 + 1`
- `DAT_00abad30 + 2`
- `DAT_00abad30 + 3`
- `DAT_00aa1028 + 0`
- `DAT_00aa1028 + 1`
- `DAT_00aa1028 + 2`
- `DAT_00aa1028 + 3`

For low state-machine dispatch, it recomputes relative tile class against the WoodBridgeSet base:

- `relative = (Cell+0x38 - DAT_00abad1c) + 1`

It then accepts the same two bridgehead class families and slots `+0..+3`, or a low anchor overlay `0xED`/`0xEE` reached through the structural-anchor probe.

Tiny but important details:

- The `+1` after subtracting the BridgeSet base is real. Any Rust comparison against raw relative tile indices must preserve that offset.
- Raw overlay direct-destroy bands win before bridgehead state-machine dispatch.
- High uses `DAT_00aa0e28` as the BridgeSet base; low uses `DAT_00abad1c`.
- The bridgehead family globals are reused across high and low; the base changes.
- Structural bridge cells may route high based on an anchor probe requiring overlay `0x18` or `0x19`.
- Low has an extra accepted anchor-overlay probe for `0xED`/`0xEE`.

### 2. High bridgehead slots `+0..+2`

Evidence: `ProcessBridgeDamageStateMachine_High @ 0x00576BA0`. Confidence: high. Active in YR: yes.

When `Cell+0x140 & 0x100` is clear, the high function takes the bridgehead tile-class branch.

North/south bridgehead family:

- Accepted classes are `DAT_00abad30 + 0..+3`.
- It reads height/ramp byte at `Cell+0x11A`.
- If `height & 1 != 0`, the branch returns `0`.
- It walks along cardinal offsets until it reaches a cell whose byte `0x11A` is `4`.
- For slots `+0`, `+1`, and `+2`, it writes:
  - overlay/tile class `DAT_00abad30 + 2 + DAT_00aa0e28`
  - Z argument `-1`
  - level argument `-1`
- It calls:
  - `MapClass__UpdateRamp_NS_DamageA_High(..., 2)`
  - `MapClass__UpdateRamp_NS_DamageB_High(..., 6)`
- It returns `0`.

East/west bridgehead family:

- Accepted classes are `DAT_00aa1028 + 0..+3`.
- If height is greater than `4`, the branch returns `0`.
- It walks until `Cell+0x11A == 2`.
- For slots `+0`, `+1`, and `+2`, it writes:
  - overlay/tile class `DAT_00aa1028 + 2 + DAT_00aa0e28`
  - Z argument `-1`
  - level argument `-1`
- It calls:
  - `MapClass__UpdateRamp_EW_DamageA_High(..., 4)`
  - `MapClass__UpdateRamp_EW_DamageB_High(..., 0)`
- It returns `0`.

### 3. High bridgehead slot `+3`

Evidence: `ProcessBridgeDamageStateMachine_High @ 0x00576BA0`. Confidence: high. Active in YR: yes.

If the high non-structural bridgehead class is slot `+3`, the branch collapses.

North/south slot `+3` side effects:

- It calls `CellClass__BlowUpBridge` on three cells.
- If anchor height byte is even, the three cells are the same X column at `Y-1`, `Y`, `Y+1`.
- If anchor height byte is odd, the three cells are shifted to `X-1` and `Y-1`, `Y`, `Y+1`.
- It calls `MapClass__SetOverlayAndPropagate` with:
  - tile class `DAT_00abad30 + 3 + DAT_00aa0e28`
  - Z `-1`
  - level `Cell+0x11B - 4` or the shifted neighbor's `0x11B - 4`
  - trailing flag `0`
- It calls:
  - `MapClass__UpdateRamp_NS_CollapseA_High(..., 2)`
  - `MapClass__UpdateRamp_NS_CollapseB_High(..., 6)`
- It updates two adjacent bridge positions through `MapClass__UpdateAdjacentBridges_High`.
- It calls `MapClass__InvalidateBridgeZones`; if non-zero, it calls `MapClass__UpdateBridgeZonesHelper`.
- It builds a ten-cell recalc list as a 2 by 5 rectangle around the collapse footprint and calls `MapClass__RecalcCellsAndRebuildZones`.
- It returns `1`.

East/west slot `+3` side effects:

- It calls `CellClass__BlowUpBridge` on three cells.
- If anchor height byte is less than `5`, the three cells are the same Y row at `X-1`, `X`, `X+1`.
- If anchor height byte is `>= 5`, the three cells are shifted to `Y-1` and `X-1`, `X`, `X+1`.
- It calls `MapClass__SetOverlayAndPropagate` with:
  - tile class `DAT_00aa1028 + 3 + DAT_00aa0e28`
  - Z `-1`
  - level `Cell+0x11B - 4` or shifted neighbor's `0x11B - 4`
  - trailing flag `0`
- It calls:
  - `MapClass__UpdateRamp_EW_CollapseA_High(..., 4)`
  - `MapClass__UpdateRamp_EW_CollapseB_High(..., 0)`
- It updates two adjacent bridge positions through `MapClass__UpdateAdjacentBridges_High`.
- It invalidates/rebuilds bridge zones, builds the same ten-cell recalc list shape, and returns `1`.

Conclusion for high: the earlier statement is correct. A high bridgehead already in the most-damaged class slot `+3` does collapse on direct bridgehead state-machine damage.

### 4. Low bridgehead slots `+0..+2`

Evidence: `ProcessBridgeDamageStateMachine_Low @ 0x00571490`. Confidence: high. Active in YR: yes.

Low uses the same non-structural bridgehead class families, but relative to `DAT_00abad1c`.

North/south slots `+0..+2`:

- Accepted classes are `DAT_00abad30 + 0..+3`.
- It reads `Cell+0x11A`.
- If `height & 1 != 0`, it does nothing and returns `0`.
- It walks to height byte `4`.
- For slots `+0..+2`, it writes:
  - tile class `DAT_00abad30 + 2 + DAT_00abad1c`
  - Z `-1`
  - level `-1`
- It calls:
  - `MapClass__UpdateRamp_NS_DamageA_Low(..., 2)`
  - `MapClass__UpdateRamp_NS_DamageB_Low(..., 6)`
- It returns `0`.

East/west slots `+0..+2`:

- Accepted classes are `DAT_00aa1028 + 0..+3`.
- The branch only proceeds when height byte is less than `5`.
- It walks to height byte `2`.
- For slots `+0..+2`, it writes:
  - tile class `DAT_00aa1028 + 2 + DAT_00abad1c`
  - Z `-1`
  - level `-1`
- The decompile labels the non-collapse ramp updates as:
  - `MapClass__UpdateRamp_EW_DamageA_High(..., 4)`
  - `MapClass__UpdateRamp_EW_DamageB_High(..., 0)`
- It returns `0`.

The `_High` labels inside the low EW non-collapse branch may be inherited/misleading labels, but the call targets are what the binary decompile shows. Do not assume helper naming alone captures the low/high split.

### 5. Low bridgehead slot `+3`

Evidence: `ProcessBridgeDamageStateMachine_Low @ 0x00571490`. Confidence: high for side effects, medium-high for caller return value. Active in YR: yes.

Low slot `+3` performs the same collapse-class side effects as high, but the decompiled function does not return `1` from the non-structural bridgehead `+3` path. After the collapse side effects it falls through to the function's `return 0` path.

North/south low slot `+3`:

- Calls `CellClass__BlowUpBridge` on three cells.
- Even height: same X column at `Y-1`, `Y`, `Y+1`.
- Odd height: shifted to `X-1` and `Y-1`, `Y`, `Y+1`.
- Calls `MapClass__SetOverlayAndPropagate` with:
  - tile class `DAT_00abad30 + 3 + DAT_00abad1c`
  - Z `-1`
  - level `Cell+0x11B - 4` or shifted neighbor's `0x11B - 4`
  - trailing flag `0`
- Calls:
  - `MapClass__UpdateRamp_NS_CollapseA_Low(..., 2)`
  - `MapClass__UpdateRamp_NS_CollapseB_Low(..., 6)`
- Calls `MapClass__UpdateAdjacentBridges` twice, without the `_High` suffix.
- Invalidates bridge zones, optionally updates bridge zones, builds the ten-cell recalc list, and rebuilds zones.
- Falls through to return `0`.

East/west low slot `+3`:

- Calls `CellClass__BlowUpBridge` on three cells.
- Height `< 5`: same Y row at `X-1`, `X`, `X+1`.
- Height `>= 5`: shifted to `Y-1` and `X-1`, `X`, `X+1`.
- Calls `MapClass__SetOverlayAndPropagate` with:
  - tile class `DAT_00aa1028 + 3 + DAT_00abad1c`
  - Z `-1`
  - level `Cell+0x11B - 4` or shifted neighbor's `0x11B - 4`
  - trailing flag `0`
- Calls:
  - `MapClass__UpdateRamp_EW_CollapseA_Low(..., 4)`
  - `MapClass__UpdateRamp_EW_CollapseB_Low(..., 0)`
- Calls `MapClass__UpdateAdjacentBridges` twice.
- Invalidates bridge zones, optionally updates bridge zones, builds the ten-cell recalc list, and rebuilds zones.
- Falls through to return `0`.

This means "does low bridgehead slot `+3` collapse?" and "does the low state-machine function return true?" have different answers:

- Collapse side effects: yes.
- Function return value: decompile shows no; it returns `0` on this non-structural path.

### 6. Structural body branch is separate

Evidence: `ProcessBridgeDamageStateMachine_High @ 0x00576BA0`, `ProcessBridgeDamageStateMachine_Low @ 0x00571490`. Confidence: high. Active in YR: yes.

The body branch is gated by `Cell+0x140 & 0x100`. It uses byte `Cell+0x11E` and has its own staged damage states:

- States `0..5` absorb to `6`.
- States `6`, `7`, `8` collapse NS.
- States `9..14` absorb to `15`.
- States `15`, `16`, `17` collapse EW.

That branch is not the source of this Rust mismatch. The mismatch is the non-structural bridgehead tile-class slot `+3` branch.

## Current Rust Evidence

Evidence: current source tree. Confidence: high.

Affected surfaces:

- `src/sim/bridge_state/mod.rs:1378` defines `BridgeRuntimeState::bridgehead_advance_state`.
- Its comments at `src/sim/bridge_state/mod.rs:1355` and nearby state that the anchor class is written to `AboutToFall` and repeat hits are idempotent.
- Its return documentation at `src/sim/bridge_state/mod.rs:1367` says success returns `StateOutcome::Absorbed`.
- Its comment states it never returns `Collapsed`.
- It writes `anchor_cell.bridgehead_anchor_class = BridgeheadAnchorClass::AboutToFall` at `src/sim/bridge_state/mod.rs:1426`.
- It calls `update_ramp_perpendicular(... DamageA ...)` and `DamageB`.
- It returns `StateOutcome::Absorbed` at `src/sim/bridge_state/mod.rs:1449`.
- `is_high_bridge` is passed into ramp update helpers, but the function has no high/low-specific collapse behavior.
- `src/sim/bridge_state/tests.rs:1280` repeats the hit 100 times.
- `src/sim/bridge_state/tests.rs:1284` explicitly asserts "every hit must return Absorbed, never Collapsed".
- `src/sim/bridge_specs.rs:788` already has `bridgehead_blow_up_row`, matching the binary's three-cell row/column footprint rules.
- `src/sim/world/bridge_orchestrator.rs:1434` routes high bridgehead state-machine events to `bridgehead_advance_state(..., true, ...)`.
- `src/sim/world/bridge_orchestrator.rs:1444` routes low bridgehead state-machine events to `bridgehead_advance_state(..., false, ...)`.
- `src/sim/world/bridge_orchestrator.rs:81` and following collect `StateOutcome::Collapsed` to run `BlowUpBridge` fallout and zone refresh side effects.

Current Rust behavior is therefore not parity-correct for this slice. Rust models bridgehead direct damage as "write AboutToFall forever", while gamemd treats slot `+3` as a collapse state.

## Player-Visible Impact

High bridgehead direct damage:

- gamemd can collapse the bridgehead-side span once the bridgehead tile class is already slot `+3`.
- Rust absorbs every repeat hit forever.
- Player-visible result: a damaged high bridgehead can become effectively indestructible from this damage path in Rust.

Low bridgehead direct damage:

- gamemd performs collapse side effects for low slot `+3`, including `BlowUpBridge`, ramp collapse helpers, adjacent bridge updates, zone invalidation, and cell recalc.
- Rust absorbs every repeat hit forever.
- Player-visible result: low bridgehead collapse effects can be missing, even though binary return value nuance differs from high.

Frequency:

- This path is conditional. It requires direct state-machine damage reaching a bridgehead tile-class cell that is slot `+3`, or a previous hit that wrote the anchor/bridgehead class to the most-damaged slot and a later hit reaching that same collapse-eligible class.
- It is important despite being conditional because it creates an infinite-survival bridge endpoint case.

## Implementation Handoff

Do not implement from this report by copying legacy control flow. Implement the observable state transitions and side effects cleanly.

Required behavior:

1. `bridgehead_advance_state` must become slot-aware.
2. First-hit behavior for bridgehead slots `+0..+2` should remain an absorbed damage transition:
   - write/set the anchor bridgehead class to `AboutToFall`/slot `+3`
   - run DamageA/DamageB perpendicular updates
   - no collapse side effects
3. A later direct state-machine hit on a bridgehead/anchor class that is already slot `+3` must run the collapse path.
4. High slot `+3` must produce collapse side effects and carry binary-success `true`.
5. Low slot `+3` must produce collapse side effects and carry binary-success `false`. Follow-up report `BRIDGEHEAD_SLOT3_LOW_RETURN_VALUE_FOLLOWUP_GHIDRA_REPORT.md` verifies the false return is load-bearing for retries and `TechnoClass__StopAllTargeting` eligibility, so Rust should separate "collapse side effects happened" from "ApplyDamageToCell returned true".
6. The three `BlowUpBridge` target cells should use the existing `bridgehead_blow_up_row` rules:
   - NS even height: same X, `Y-1..Y+1`
   - NS odd height: `X-1`, `Y-1..Y+1`
   - EW height `< 5`: same Y, `X-1..X+1`
   - EW height `>= 5`: `Y-1`, `X-1..X+1`
7. High collapse must use high collapse ramp helpers / high adjacent bridge update semantics.
8. Low collapse must use low collapse ramp helpers / non-high adjacent bridge update semantics.
9. Tests asserting "repeat hits never collapse" must be replaced.

Suggested acceptance scenarios:

- High NS bridgehead: slot `+0` hit absorbs and writes slot `+3`; second hit collapses with three `BlowUpBridge` cells in the NS footprint.
- High EW bridgehead: slot `+0` hit absorbs and writes slot `+3`; second hit collapses with three `BlowUpBridge` cells in the EW footprint.
- Low NS bridgehead: slot `+0` hit absorbs; second hit performs collapse side effects with low helper semantics.
- Low EW bridgehead: slot `+0` hit absorbs; second hit performs collapse side effects with low helper semantics.
- Odd NS starting-height gate still returns no-change/no-collapse.
- EW starting height greater than `4` still returns no-change/no-collapse before the walk.
- Slot `+0..+2` must not collapse on the first state-machine hit.
- Raw overlay direct-destroy bands must continue to route before the bridgehead state-machine path.

## Coverage Ledger

Verified:

- `ApplyDamageToCell` raw overlay and bridgehead dispatch order.
- High bridgehead slots `+0..+3`.
- Low bridgehead slots `+0..+3`.
- High/low three-cell `BlowUpBridge` footprints.
- High/low collapse ramp helper families.
- Rust mismatch surface and tests encoding wrong behavior.

Touched but not fully expanded:

- Structural body state-machine branch. It is separate and already covered by other bridge reports.
- `CellClass__BlowUpBridge` internals. Not needed to prove slot `+3` collapse.

Deferred:

- Stock-map prevalence and exact external meaning of low slot `+3` returning `0`.

## Bottom Line

The original high-bridge claim is correct: high bridgehead direct damage on tile-class slot `+3` collapses in gamemd, while Rust currently never collapses from `bridgehead_advance_state`.

The full extent is broader: low bridgehead slot `+3` also performs collapse side effects, but the low non-structural function falls through with return `0`. Fixing Rust should therefore implement bridgehead slot `+3` collapse side effects for both high and low, while treating high/low caller-return semantics carefully.
