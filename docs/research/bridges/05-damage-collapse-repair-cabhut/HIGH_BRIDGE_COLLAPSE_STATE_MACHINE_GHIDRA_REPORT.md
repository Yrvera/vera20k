# High Bridge Collapse State Machine - Ghidra Research Report

**Address(es):** `0x00576BA0` primary; `0x0047E040`, `0x0047E470`, `0x00587180`, `0x0057CCF0`, `0x0057CF60`, `0x0057D530` supporting.
**Investigation Mode:** exhaustive-slice.
**Claimed Scope:** high bridge collapse state machine around `ProcessBridgeDamageStateMachine_High @ 0x00576BA0`, state-byte decoding, body vs bridgehead branches, partial-collapse transitions, `SetBridgeDirection_NESW/NWSE` destroy stamping, blow-up slot selection, direct high walker cascade deltas needed to hand off Rust changes.
**Non-Scope:** low TubeClass internals, full CABHUT/C4 entry, area-damage RNG gate, bridge repair, render atlas frame selection, and bridge-debris/audio ordering except where the high state-machine return shape exposes cascade cells.
**Confidence:** High for the primary state machine, caller, and destroy stamping; Medium for direct walker side-cascade deltas because only high walkers were re-opened, not every `ApplyBridgeDestruction_*` callee.
**Active in YR:** Yes, conditional. `ApplyDamageToCell @ 0x00587180` calls this path on live bridge damage. Area-damage entry is gated by `DestroyableBridges=yes` and warhead bridge damage in standard YR; direct map/hut paths can reach high walkers separately.

## 0. Investigation Framing

**Target question:** What exact state transitions and collapse side effects does `ProcessBridgeDamageStateMachine_High @ 0x00576BA0` perform for high bridge body and bridgehead cells, and what must Rust preserve?

**Non-goals:** Do not redo low TubeClass, CABHUT search, area-damage probability, or full fallout ordering. Those are sibling swarm slots.

**Evidence needed to mark COMPLETE:** decompile plus assembly evidence for the primary body switch, bridgehead branch final-collapse branch, caller dispatch from `ApplyDamageToCell`, `SetBridgeDirection_*` destroy stamping and blow-up slots, and enough Rust scan to name exact deltas and tests.

**Stop conditions:** Stop when the high state-machine body/bridgehead branches and destroy-stamp slots are resolved, every open question is resolved or explicitly deferred, and any low/direct walker material is limited to handoff-critical high contrast.

## 1. Overview

High bridge damage has two state-machine branches inside `0x00576BA0`. A structural body cell follows its anchor pointer when needed and switches on `CellClass+0x11E`; healthy states absorb a hit, damaged/partial-collapse states clear the high bridge stamp and return collapse. A bridgehead/ramp class cell instead walks by height back to the anchor tile class; slots 0-2 absorb into the "damaged" bridgehead class, while slot 3 immediately runs a collapse branch with three `BlowUpBridge` calls, ramp collapse helpers, adjacency refresh, zone invalidation, and a 10-slot recalculation list.

Active in YR: Yes. `ApplyDamageToCell @ 0x00587180` dispatches to `ProcessBridgeDamageStateMachine_High` when a body anchor overlay is `0x18/0x19` or a high bridgehead class matches the runtime theater globals.

## 2. Class Layout / Key Offsets

| Field | Type | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|
| `CellClass+0x24` | packed coord | Input/anchor coordinate copied into local coords for helper calls. | `0x00576BA0` decompile; `0x0057778a`, `0x005778a6` assembly contexts | Yes |
| `CellClass+0x2C` | pointer | Non-anchor structural cells follow this anchor pointer when `flags & 0x80 == 0`. | `0x00576BA0`; `0x00587180` | Yes |
| `CellClass+0x38` | int | IsoTileTypeIndex; bridgehead branch computes `(tile - BridgeSet) + 1`. | `0x00576BA0` entry filter | Yes |
| `CellClass+0x44` | int | OverlayTypeIndex; collapsed body branch writes `-1`; direct high walker uses `0xCD..0xE8`. | `0x0057779f`, `0x005778bb`; `0x0057CCF0` | Yes |
| `CellClass+0x11A` | byte | Height/ramp step byte used by bridgehead branch to walk to anchor and choose offset shape. | `0x00576BA0` decompile | Yes |
| `CellClass+0x11B` | byte | Level byte; bridgehead collapse passes `level - 4` as SetOverlay z argument. | `0x00576BA0` decompile | Yes |
| `CellClass+0x11E` | byte | Body state byte: `0..8` NS side, `9..17` EW side. | `0x00576BA0` switch; `DamageState` Rust scan | Yes |
| `CellClass+0x140` | u32 flags | `0x100` structural body gate, `0x80` anchor-self, destroy stamp writes `0x400` to blow-up slots. | `0x00576BA0`; `0x0047E040` | Yes |

## 3. Core Logic

### 3.1 Entry and Caller

`ApplyDamageToCell @ 0x00587180` first routes raw overlay ranges to direct walkers: low `0x4A..0x63`, high `0xCD..0xE6`. If not direct high/low, it checks high structural body or high bridgehead class and calls `0x00576BA0`. For a structural body cell, it resolves the anchor candidate and requires anchor overlay `0x18` or `0x19`; for non-body bridgehead cells it compares the BridgeSet-relative tile class against `DAT_00ABAD30..+3` and `DAT_00AA1028..+3`.

Evidence: decompile `0x00587180`; assembly `0x005872a4..0x005872b3` compares anchor overlay `0x18/0x19`, `0x005872b9..0x005872e8` compares high bridgehead classes, and `0x005873fe..0x00587401` calls `0x00576BA0`.

Active in YR: Yes, conditional on a live bridge-damage event reaching `ApplyDamageToCell`.

### 3.2 Body Branch State Byte Switch

If `flags & 0x100` is set, the function treats the cell as high structural body. If `flags & 0x80` is clear, it replaces the working cell with `*(cell+0x2C)`, the anchor pointer. It then reads anchor `+0x11E`.

| State byte | Binary behavior | Return | Evidence | Active in YR |
|---:|---|---:|---|---|
| `0..5` | Write `+0x11E = 6`; call `UpdateRamp_NS_DamageA_High(anchor, 2)` and `UpdateRamp_NS_DamageB_High(anchor, 6)`. | `0` | `0x00576BA0` decompile | Yes |
| `6` | Call NS CollapseA and CollapseB, then `SetBridgeDirection_NESW(0,0)`, write `+0x11E=0`, `+0x44=-1`, update adjacent, maybe zones. | `1` | `0x00577782..0x005777a6` assembly | Yes |
| `7` | Call NS CollapseA only, then same `SetBridgeDirection_NESW(0,0)` clear/finalize. | `1` | `0x005777c4..0x005777d0`; shared finalize at `0x0057778a` | Yes |
| `8` | Call NS CollapseB only, then same clear/finalize. | `1` | `0x00577782..0x00577790` | Yes |
| `9..14` | Write `+0x11E = 0x0F`; call `UpdateRamp_EW_DamageA_High(anchor, 4)` and `UpdateRamp_EW_DamageB_High(anchor, 0)`. | `0` | `0x00576BA0` decompile | Yes |
| `0x0F` | Call EW CollapseA and CollapseB, then `SetBridgeDirection_NESW(6,0)`, write `+0x11E=0`, `+0x44=-1`, update adjacent, maybe zones. | `1` | `0x0057789a..0x005778b1` | Yes |
| `0x10` | Call EW CollapseB only, then same `SetBridgeDirection_NESW(6,0)` clear/finalize. | `1` | `0x00576BA0`; `0x005778a6..0x005778b1` | Yes |
| `0x11` | Call EW CollapseA only, then same `SetBridgeDirection_NESW(6,0)` clear/finalize. | `1` | `0x00576BA0`; `0x005778a6..0x005778b1` | Yes |
| other | No state change. | `0` | switch default `0x00576BA0` | Yes |

Tiny details:

- For NS states, `uVar6 = (8 < state) - 1` becomes `0xFFFFFFFF`, so `(uVar6 & ~1) + 4` is `2` and `uVar6 & 6` is `6`.
- For EW states, the same formula yields `4` and `0`.
- Final collapse writes `+0x44 = 0xFFFFFFFF`, not `+0x38`; this is an overlay clear, not a base-tile clear.
- After either axis finalizes, `InvalidateBridgeZones(anchor_coord)` gates `UpdateBridgeZonesHelper`; it is not called unconditionally.

### 3.3 Bridgehead Branch

If `flags & 0x100` is clear but the BridgeSet-relative class matches one of the high bridgehead globals, the function runs a tile-class branch. This branch uses `+0x38`, not `+0x11E`, as the state source.

NS-class branch: `iVar2 in DAT_00ABAD30..DAT_00ABAD30+3`.

- If `cell+0x11A` is odd, return `0`.
- Walk in the cardinal direction chosen by whether height is above or below `4` until the working cell height is exactly `4`.
- For class slots `+0`, `+1`, or `+2`, set the anchor overlay class to `DAT_00ABAD30 + 2 + BridgeSet`, call NS DamageA/DamageB with `2`/`6`, and return `0`.
- For class slot `+3`, run full bridgehead collapse: call `BlowUpBridge` on three axial cells, set overlay class to `DAT_00ABAD30 + 3 + BridgeSet` with z `level - 4`, call NS CollapseA/CollapseB, update two adjacent bridge cells, conditionally rebuild zones, build a 10-slot cell list, call `RecalcCellsAndRebuildZones`, and return `1`.

EW-class branch: `iVar2 in DAT_00AA1028..DAT_00AA1028+3`.

- If `height > 4`, return `0`; otherwise walk to height `2`.
- Slots `+0..+2` set class to `DAT_00AA1028 + 2 + BridgeSet`, call EW DamageA/DamageB with `4`/`0`, return `0`.
- Slot `+3` runs the EW mirror full-collapse branch: three `BlowUpBridge` cells, class `DAT_00AA1028 + 3 + BridgeSet`, EW CollapseA/CollapseB, two adjacency updates, conditional zone rebuild, 10-slot recalculation list, return `1`.

Active in YR: Yes. The bridgehead globals are theater/tile-set runtime values loaded for high bridges and are checked directly by `ApplyDamageToCell` and `0x00576BA0`.

Rust-facing correction: current `BridgeRuntimeState::bridgehead_advance_state` says it never returns `Collapsed`; this contradicts the slot `+3` branch in `0x00576BA0`.

### 3.4 SetBridgeDirection Destroy Stamping and Blow-Up Slots

`CellClass::SetBridgeDirection_NESW @ 0x0047E040` and `CellClass::SetBridgeDirection_NWSE @ 0x0047E470` are byte-identical for the destroy-stamp contract. The high state-machine body branch calls only `NESW`, with direction `0` for NS states and `6` for EW states. `NWSE` is included here because the requested destroy-stamp slot contract is shared and byte-identical.

Destroy call means `param_3 == 0`. The helper clears intact bits, writes state byte `0`, sets `0x400` on the four material cells, clears anchor pointers on non-anchor material cells, and calls `CellClass::BlowUpBridge` only on:

- slot 0: anchor cell,
- slot 1: one forward step,
- slot 2: two forward steps,
- slot 4: one opposite step.

It does not call `BlowUpBridge` on:

- slot 3: three forward steps, flag-only `0x1000` update,
- slot 5: the extra direction-6 cell, flag-only `0x10000` update.

Evidence: `0x0047E040` decompile; assembly `0x0047e0e7` writes anchor `0x80` bit source, `0x0047e29b..0x0047e2b5` shows state-zero plus `BlowUpBridge`, and `0x0047e3ff..0x0047e452` shows the direction-6 extra-cell branch without a `BlowUpBridge` call. `0x0047E470` decompile is byte-identical.

Active in YR: Yes. Called by normal map-load stamping, repair/damage helpers, and the high body collapse branch.

### 3.5 Direct High Walker Deltas Relevant to Cascade

Direct high walker entry is separate from `0x00576BA0` but matters because `ApplyDamageToCell` routes raw high overlays there before the state machine.

`DestroyBridge_High @ 0x0057CCF0` dispatches:

- `0xCD..0xD5`, `0xDF..0xE2`, `0xE7` to `DestroyBridgeWalker_NS_High @ 0x0057CF60` after a local neighbor-origin adjustment.
- `0xD6..0xDE`, `0xE3..0xE6`, `0xE8` to `DestroyBridgeWalker_EW_High @ 0x0057D530` after a mirrored adjustment.

`DestroyBridgeWalker_NS_High` writes the `(this, y-1, y+1)` triple:

- `0xDF -> 0xE0`, cascade one west sibling,
- `0xE1 -> 0xE2`, cascade one east sibling,
- `<0xD3 -> 0xD3`, cascade both east/west siblings,
- `0xD3..0xD5 -> 0xE7`, mark final, cascade both siblings, find endpoints, update zones, dirty/recalc 3 cells.

`DestroyBridgeWalker_EW_High` writes the `(this, x-1, x+1)` triple:

- `0xE3 -> 0xE4`, cascade one south sibling,
- `0xE5 -> 0xE6`, cascade one north sibling,
- `<0xDC -> 0xDC`, cascade both north/south siblings,
- `0xDC..0xDE -> 0xE8`, mark final, cascade both siblings, find endpoints, update zones, dirty/recalc 3 cells.

Active in YR: Yes. Direct high walker is hit by raw high overlay damage before the state-machine branch.

## 4. INI Keys

| Section | Key | Stock YR/default | Effect in this slice | Evidence | Active in YR |
|---|---|---|---|---|---|
| `[CombatDamage]` | `BridgeStrength` | `1500` | Area-damage RNG gate before `ApplyDamageToCell`; not read inside `0x00576BA0`. | `ini/rulesmd.ini:816`; Rust `BridgeRules` parser | Yes |
| `[CombatDamage]` | `DestroyableBridges` | `yes` | Standard area-damage master gate before bridge damage reaches the state machine. | `ini/rulesmd.ini:804`; prior gate doc | Yes |
| `[CombatDamage]` | `IonCannonWarhead` | `IonCannonWH` | Bypasses outer probability and enables retry in Rust dispatcher; not inside `0x00576BA0`. | `ini/rulesmd.ini:874`; Rust `BridgeWarheads` | Yes |
| `[CombatDamage]` | `C4Warhead` | `Super` | Used by collapse fallout for deck/ground kill semantics, not inside `0x00576BA0`. | `ini/rulesmd.ini:818`; sibling fallout docs | Yes |
| `[AudioVisual]` | `BridgeExplosions` | `TWLT026,TWLT036,TWLT050,TWLT070` | Direct/collapse walkers spawn effects downstream; primary state machine only returns collapse. | `ini/rulesmd.ini:529`; sibling fallout docs | Yes |
| Building type | `BridgeRepairHut` | `CABHUT=yes` | CABHUT collapse can call high walkers/state-machine siblings, but not this slot's entry proof. | `ini/rulesmd.ini:16348`; sibling slot | Yes |

No INI key defines the state bytes `0..17`, bridgehead class slots, direction constants `0/2/4/6`, or destroy-stamp blow-up slots. Those are binary constants.

## 5. Integration Points

| Function / path | Role | Evidence | Active in YR |
|---|---|---|---|
| `ApplyDamageToCell @ 0x00587180` | Live caller; routes raw overlays to direct high walker first, then high body/bridgehead classes to `0x00576BA0`. | decompile; assembly `0x005873fe..0x00587401` | Yes |
| `ProcessBridgeDamageStateMachine_High @ 0x00576BA0` | Primary high body/bridgehead state machine. | decompile; assembly `0x0057778a`, `0x005778a6`, `0x005778cc` | Yes |
| `SetBridgeDirection_NESW @ 0x0047E040` | Destroy-stamp called by body collapse with directions `0` and `6`. | decompile; assembly `0x0047e0e7`, `0x0047e3ff` | Yes |
| `SetBridgeDirection_NWSE @ 0x0047E470` | Byte-identical stamp twin, included for shared slot contract. | decompile `0x0047E470` | Yes |
| `UpdateRamp_*_High` helpers | Perpendicular damage/collapse side writes; called with `NS: 2/6`, `EW: 4/0`. | `0x00576BA0` decompile | Yes |
| `DestroyBridge_High @ 0x0057CCF0` | Direct raw-overlay high entry before state machine. | decompile | Yes |
| `DestroyBridgeWalker_NS/EW_High @ 0x0057CF60/0x0057D530` | Direct high overlay transition and final collapse triples. | decompile | Yes |

## 6. Current Rust Implementation Status

Scanned surfaces:

- `src/sim/bridge_state/mod.rs`
- `src/sim/bridge_state/walker.rs`
- `src/sim/world/bridge_orchestrator.rs`
- `src/sim/combat/mod.rs`
- `src/rules/ruleset.rs`
- bridge-collapse tests in `src/sim/world/world_tests.rs`

Current matches:

- `DamageState::from_state_byte` and `to_state_byte` encode `0..5`, `6`, `7`, `8`, `9..14`, `0x0F`, `0x10`, `0x11` in the verified split.
- `body_cell_advance_state` follows anchor spans for non-anchor body cells and implements Healthy -> Damaged, Damaged -> Destroyed, PartialCollapseA/B -> Destroyed.
- `set_bridge_direction` data model already separates blow-up slots from flag-only slots through `AnchorSpan::BLOW_UP_SLOTS = [0,1,2,4]`.
- `bridge_orchestrator` already drains cascade stages separately from the state-machine return.

Current deltas:

- `bridgehead_advance_state` explicitly says it never returns `Collapsed`; binary bridgehead slot `+3` returns `1` and runs collapse. This is the highest-priority mismatch from this slot.
- `path_matches_cell` rejects high state-machine damage when `overlay_byte` is in `0xCD..0xE6`; that is correct for direct raw body overlays, but bridgehead state-machine routing must be based on BridgeSet-relative tile class/role rather than "not raw overlay" alone.
- Body collapse clears binary overlay `+0x44 = -1`; Rust sets `DamageState::Destroyed` and relies on render/effective state. Confirm callers that need raw overlay clear see `overlay_byte = 0xFF` or equivalent after state-machine collapse.
- Direct high walker implementation in `walker.rs` is close to the verified high walker transition table, but it should be pinned with tests named from the exact binary branches rather than only integration replay tests.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Target question / non-goals / stop conditions | verified | Section 0 | none |
| `ApplyDamageToCell` high state-machine caller | verified | `0x00587180`; assembly `0x005873fe..0x00587401` | none |
| Primary body branch entry and anchor pointer follow | verified | `0x00576BA0` decompile | none |
| Body states `0..5`, `6`, `7`, `8`, `9..14`, `0x0F`, `0x10`, `0x11`, default | verified | `0x00576BA0`; assembly `0x0057778a`, `0x005778a6`, `0x005778cc` | none |
| Bridgehead class slots `DAT_00ABAD30..+3` and `DAT_00AA1028..+3` | verified | `0x00576BA0` decompile | none |
| Bridgehead slot `+3` collapse branch | verified | `0x00576BA0` decompile | exact runtime map examples not scanned |
| `SetBridgeDirection_NESW` destroy slots | verified | `0x0047E040` decompile; assembly `0x0047e0e7`, `0x0047e3ff` | none |
| `SetBridgeDirection_NWSE` twin | verified | `0x0047E470` decompile | none |
| Direct high walker dispatch | verified | `0x0057CCF0` decompile | none |
| Direct high walker transition triples | verified | `0x0057CF60`, `0x0057D530` decompile | `ApplyBridgeDestruction_*` internal table details deferred |
| INI defaults | verified | `ini/rulesmd.ini`, `ini/rules.ini`, Rust parser scan | none |
| Current Rust body state-machine surface | verified-by-source-scan | `src/sim/bridge_state/mod.rs` | focused tests still needed |
| Current Rust bridgehead surface | verified-by-source-scan | `src/sim/bridge_state/mod.rs` | implementation task should fix collapse slot |
| Low TubeClass contrast | deferred | out-of-scope | slot 4 owns this |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is `0x00576BA0` live in YR? -> Yes, `ApplyDamageToCell` calls it for high structural/body and high bridgehead cases.` (evidence: `0x00587180`, `0x005873fe..0x00587401`)
- `[RESOLVED] OQ-02 - What gates entry? -> raw high overlays go direct first; otherwise high body requires anchor overlay `0x18/0x19`, high bridgeheads require BridgeSet-relative class slots.` (evidence: `0x00587180`, `0x00576BA0`)
- `[RESOLVED] OQ-03 - How does a non-anchor body cell resolve state? -> it follows `CellClass+0x2C` when `flags & 0x80` is clear.` (evidence: `0x00576BA0`)
- `[RESOLVED] OQ-04 - What do state bytes `0..5` do? -> write `6`, call NS DamageA/DamageB, return `0`.` (evidence: `0x00576BA0`)
- `[RESOLVED] OQ-05 - What does state `6` do? -> call NS CollapseA/CollapseB, `SetBridgeDirection_NESW(0,0)`, clear state and overlay, return `1`.` (evidence: `0x00577782..0x005777a6`)
- `[RESOLVED] OQ-06 - What do states `7` and `8` do? -> collapse only one remaining NS side, then same final clear.` (evidence: `0x00576BA0`, `0x0057778a`)
- `[RESOLVED] OQ-07 - What do states `9..14` do? -> write `0x0F`, call EW DamageA/DamageB, return `0`.` (evidence: `0x00576BA0`)
- `[RESOLVED] OQ-08 - What do states `0x0F`, `0x10`, `0x11` do? -> collapse both or one EW side, `SetBridgeDirection_NESW(6,0)`, clear state and overlay, return `1`.` (evidence: `0x005778a6..0x005778c2`)
- `[RESOLVED] OQ-09 - Does body collapse clear IsoTileTypeIndex? -> No, it writes `CellClass+0x44 = -1`.` (evidence: `0x0057779f`, `0x005778bb`)
- `[RESOLVED] OQ-10 - Does bridgehead direct fire ever collapse? -> Yes, class slot `+3` collapses and returns `1`.` (evidence: `0x00576BA0`)
- `[RESOLVED] OQ-11 - Which bridgehead classes only absorb damage? -> slots `+0..+2` set class `+2`, call DamageA/DamageB, return `0`.` (evidence: `0x00576BA0`)
- `[RESOLVED] OQ-12 - What cells does destroy stamping blow up? -> anchor, forward1, forward2, and opposite only; forward3 and extra direction-6 are flag-only.` (evidence: `0x0047E040`, `0x0047e3ff`)
- `[RESOLVED] OQ-13 - Is `NWSE` different for destroy slot purposes? -> No, `0x0047E470` is byte-identical for the shared stamp contract.` (evidence: `0x0047E470`)
- `[RESOLVED] OQ-14 - Is direct high walker before state machine relevant? -> Yes, raw high overlays `0xCD..0xE6` dispatch to `DestroyBridge_High` before state machine.` (evidence: `0x00587180`, `0x0057CCF0`)
- `[RESOLVED] OQ-15 - What Rust surface mismatches the binary? -> `bridgehead_advance_state` never collapses, unlike binary slot `+3`.` (evidence: source scan `src/sim/bridge_state/mod.rs`; binary `0x00576BA0`)
- `[RESOLVED] OQ-16 - Are the INI keys inside the primary state machine? -> No; they gate or decorate callers/fallout, not `0x00576BA0` itself.` (evidence: `0x00576BA0`; `ini/rulesmd.ini`)
- `[DEFERRED] OQ-17 - Exact `ApplyBridgeDestruction_NS/EW_High` internal neighbor table semantics.` (category: requires-different-system-context; reason: direct walker callee internals exceed this state-machine slot; next-step-if-pursued: focused high walker cascade leaf investigation)
- `[DEFERRED] OQ-18 - Runtime stock-map examples that place bridgehead class slot `+3`.` (category: out-of-scope; reason: requires retail map/theater data scan, not binary behavior proof; next-step-if-pursued: dump BridgeSet-relative classes from stock map cells)
- `[DEFERRED] OQ-19 - Low TubeClass state-machine contrast.` (category: requires-different-system-context; reason: slot 4 owns low collapse and zones; next-step-if-pursued: use LOW_BRIDGE_TUBECLASS reports)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Body state bytes `0..5` absorb into `6`; `9..14` absorb into `0x0F`; no collapse return. | `0x00576BA0` decompile | none observed | `src/sim/bridge_state/mod.rs::body_cell_advance_state` | Preserve Healthy -> Damaged and return `Absorbed` without blow-up actions. | `high_body_healthy_state_absorbs_and_sets_axis_damage_byte` | Do not collapse on the first body hit. |
| Body state `6` collapses with NS CollapseA+B then `SetBridgeDirection_NESW(0,0)`; `0x0F` collapses with EW CollapseA+B then `(6,0)`. | `0x00577782..0x005777a6`, `0x005778a6..0x005778c2` | likely mostly implemented; verify overlay clear | `src/sim/bridge_state/mod.rs`, `src/sim/world/bridge_orchestrator.rs` | On Damaged -> Destroyed, clear runtime overlay equivalent and emit destroy-stamp actions for the correct anchor span. | `high_body_damaged_collapse_clears_overlay_and_emits_destroy_stamp_slots` | Do not mark every span slot as `BlowUpBridge`; use slots 0/1/2/4 only. |
| Partial states `7/8/0x10/0x11` call only one collapse helper and then finalize like full collapse. | `0x00576BA0`; assembly shared finalize at `0x0057778a`/`0x005778a6` | implemented as enum states; focused tests needed | `src/sim/bridge_state/mod.rs::body_cell_advance_state` | Preserve `PartialCollapseA` and `PartialCollapseB` as distinct one-side-collapse states. | `high_body_partial_collapse_a_fires_only_collapse_a_then_finalizes`; `high_body_partial_collapse_b_fires_only_collapse_b_then_finalizes` | Do not normalize partial states to `Damaged`; it changes side-effect count. |
| Bridgehead class slots `+0..+2` absorb into class `+2`, while slot `+3` collapses immediately and returns `1`. | `0x00576BA0` bridgehead branch | mismatch: Rust `bridgehead_advance_state` never returns `Collapsed` | `src/sim/bridge_state/mod.rs::bridgehead_advance_state`; `src/sim/bridge_specs.rs` | Add slot-aware bridgehead state so an already-AboutToFall bridgehead/anchor class triggers collapse with three axial blow-up cells, ramp collapse helpers, adjacency, and zones. | `high_bridgehead_about_to_fall_hit_collapses_and_returns_destroyed_cells`; `high_bridgehead_variant_zero_to_damaged_absorbs` | Do not model sustained bridgehead fire as permanently non-collapsing. |
| `SetBridgeDirection_*` destroy stamps call `BlowUpBridge` only on anchor, forward1, forward2, opposite. | `0x0047E040`, `0x0047E470`, assembly `0x0047e3ff` | none observed; `AnchorSpan::BLOW_UP_SLOTS=[0,1,2,4]` matches | `src/sim/bridge_state/mod.rs::AnchorSpan`; `src/sim/bridge_specs.rs::set_bridge_direction` | Keep flag-only forward3 and direction-6 extra slot out of ground-kill/drop-in blow-up cells. | `set_bridge_direction_destroy_skips_forward3_and_dir6_extra_blowup` | Do not use "all stamped cells" as fallout cells. |
| Raw high overlays route to direct walkers before state-machine. | `0x00587180`, `0x0057CCF0` | implemented in dispatcher ordering; tests exist but should pin direct high table names | `src/sim/world/bridge_orchestrator.rs`; `src/sim/bridge_state/walker.rs` | Preserve HighSM rejection for raw overlay and HighDirect walker route. | `raw_high_overlay_routes_high_direct_before_high_state_machine` | Do not let raw body overlays burn state-machine retry attempts first. |

### Stale Docs / Follow-up Docs

Replace any claim that "bridgehead direct fire cannot collapse" with:

> In `ProcessBridgeDamageStateMachine_High @ 0x00576BA0`, high bridgehead class slots `+0..+2` absorb damage into the damaged class and return `0`, but class slot `+3` runs the collapse branch and returns `1`.

## 10. Negative Facts / Do Not Do

- Do not infer collapse state from labels `NS`/`EW` alone; cite state-byte ranges and direction constants (`0`, `2`, `4`, `6`).
- Do not clear `CellClass+0x38` on body collapse; the binary clears overlay `+0x44`.
- Do not collapse bridgehead hits only through the body driver; the bridgehead slot `+3` branch is live and returns collapse.
- Do not apply `BlowUpBridge` to forward3 or the direction-6 extra stamp cell.
- Do not treat direct high overlay damage and high state-machine damage as the same path. Raw overlays go to `DestroyBridge_High` first.
- Do not call `UpdateBridgeZonesHelper` unconditionally after the body state machine; the body branch checks `InvalidateBridgeZones`.

## 11. Remaining Uncertainty

- The exact internal tables inside `ApplyBridgeDestruction_NS_High` and `ApplyBridgeDestruction_EW_High` were not re-drained in this slot. Direct walker top-level effects were verified, but a separate high walker cascade-leaf report should own those helper internals if needed.
- I did not scan retail maps for actual bridgehead class slot `+3` placements; binary behavior is verified, data frequency remains unknown.
- Low bridge TubeClass behavior is intentionally deferred to slot 4.
- No Rust tests were run because this was a read-only research subagent task.

## Sources

- Ghidra decompiled this pass: `0x00576BA0`, `0x00587180`, `0x0047E040`, `0x0047E470`, `0x0057CCF0`, `0x0057CF60`, `0x0057D530`.
- Ghidra assembly context this pass: `0x00587293`, `0x005872b9`, `0x005873fe`, `0x0057778a`, `0x005778a6`, `0x005778b1`, `0x005778cc`, `0x0047e0e7`, `0x0047e29b`, `0x0047e3ff`.
- Existing docs referenced: `BRIDGE_SETBRIDGEDIRECTION_STAMPING_GHIDRA_REPORT.md`, `BRIDGE_AXIS_AND_CARDINAL_POLARITY_GHIDRA_REPORT.md`, `BRIDGE_COLLAPSE_CHAIN_MECHANISM_GHIDRA_REPORT.md`, `DESTROYABLEBRIDGES_INI_GATE_GHIDRA_REPORT.md`.
- INI files checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust surfaces scanned: `src/sim/bridge_state/mod.rs`, `src/sim/bridge_state/walker.rs`, `src/sim/world/bridge_orchestrator.rs`, `src/sim/combat/mod.rs`, `src/rules/ruleset.rs`, `src/rules/bridge_warheads.rs`, `src/sim/world/world_tests.rs`.
