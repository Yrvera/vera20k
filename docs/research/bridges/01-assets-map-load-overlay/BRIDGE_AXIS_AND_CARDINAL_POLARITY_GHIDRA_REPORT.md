# Bridge Axis and Cardinal Polarity - Ghidra Research Report

**Address(es):** `0x0047E040`, `0x005FC570`, `0x00573540`, `0x00576BA0`, `0x0056D6E0`, `0x005851B0`, `0x00582D70`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** NS/EW bridge axis polarity, anchor overlay `0x18/0x19` direction constants, bit `0x800`, bridge direction table use, direction wrapping, high-bridge state byte axis split, and active YR liveness for those paths.  
**Non-Scope:** full bridge damage lifecycle, low-bridge TubeClass internals beyond direct table/record consumers, render-frame Latin-square variety, tactical screen inverse, and Rust implementation changes.  
**Confidence:** High for polarity, direction wrapping, and active liveness; Medium for current Rust delta because only relevant surfaces were scanned, not tested.  
**Active in YR:** Yes. The verified paths are standard map-load, bridge-zone build/update, bridge damage/collapse, and repair/hut fallback paths.

## 0. Investigation Framing

**Target question:** what is the exact YR contract for "north/south/east/west" bridge axis and reference cells, especially `0x18/0x19`, `Flags & 0x800`, `dir & 7`, `(dir - 4) & 7`, bridge stamp cells, and the bridge direction tables?

**Non-goals:** do not re-investigate all bridge damage states, low-bridge tube movement, bridge rendering art selection, or write Rust.

**Evidence needed to mark COMPLETE:** binary proof for overlay `0x18/0x19` constants, `0x800` writer polarity, direction table reads and wrap math, active callers, and enough Rust surface scan to hand off parity tests.

**Stop conditions:** stop after the polarity/table/reference-cell contract is reconciled and no un-deferred open question remains inside this slice; defer full damage/repair lifecycle and data-dump validation.

## 1. Overview

The bridge polarity contract is: standard direction indices are `0=N, 2=E, 4=S, 6=W`; `dir & 7` wraps a raw direction into that table; `(dir - 4) & 7` gives the opposite direction. Overlay `0x18` calls `SetBridgeDirection_NESW(dir=0,state=1)`, producing an N-S anchor/body stamp and setting `CellClass+0x140 bit 0x800`; overlay `0x19` calls `SetBridgeDirection_NESW(dir=6,state=1)`, producing an E-W stamp and clearing `0x800`.

State bytes agree with that contract: bytes `0..=8` are the N-S half and bytes `9..=17` are the E-W half. The stale polarity to remove is "bit `0x800` set means E-W"; the verified replacement is "bit `0x800` set means the direction-zero/N-S stamp."

## 2. Class Layout / Key Offsets

| Field | Type | Verified purpose in this slice | Evidence | Active in YR |
|---|---:|---|---|---|
| `CellClass+0x24` | packed coord | Anchor/reference cell coordinate used by stamp, damage, bridge-zone, and fallback walks. | `0x0047E040`, `0x00573540`, `0x005851B0` | Yes |
| `CellClass+0x2C` | pointer | Non-anchor stamped cells receive the anchor pointer when intact; damage paths can dereference it when `0x80` is absent. | `0x0047E040`, `0x00576BA0` | Yes |
| `CellClass+0x38` | int | IsoTileTypeIndex; bridge-zone/table code subtracts high/wood bridge base to index 16-entry tables. | `0x0056D6E0`, `0x005851B0`, `0x00582D70` | Yes |
| `CellClass+0x44` | int | Overlay type id; `OverlayClass::Mark` reads overlay type metadata id and bridge damage readers classify overlay ids. | `0x005FC570`, `0x00573540` | Yes |
| `CellClass+0x11A` | byte | Subtile/ramp slot used by bridge state machine branches; not the `0x18/0x19` axis source. | `0x00576BA0` | Yes |
| `CellClass+0x11E` | byte | Bridge state/damage byte: `0..=8` N-S, `9..=17` E-W. | `0x0047E040`, `0x00576BA0` | Yes |
| `CellClass+0x140` | u32 flags | `0x800` is set only for direction-zero stamps; destruction fallback reads it for cardinal walker choices. | `0x0047E09F..0x0047E0AC`, `0x00573540` | Yes |

## 3. Core Logic

### 3.1 Direction table and wrap contract

The direction table used by these bridge paths is `g_DirectionOffsets @ 0x0089F688`, with the standard 8-direction order recovered in prior direction research: `0=N`, `1=NE`, `2=E`, `3=SE`, `4=S`, `5=SW`, `6=W`, `7=NW`.

The binary applies two important wraps:

- `dir & 7`: used for the forward step. Evidence: `SetBridgeDirection_NESW` decompile indexes `g_DirectionOffsets + (param_2 & 7)`; `AddBridgeZoneEdges @ 0x00585248`; `FUN_00582D70 @ 0x00582F4F`.
- `(dir - 4) & 7`: used for the opposite step. Evidence: `SetBridgeDirection_NESW` decompile computes `uVar15 = param_2 - 4 & 7`; `AddBridgeZoneEdges` and `FUN_00582D70` both call `MapCoord_Add` with `uVar3 - 4 & 7`.

Tiny detail: `DAT_0082A944` contains `-1` at indices 2 and 5 per prior verified memory dump. In the direct Add/Remove/FUN_00582D70 readers, `-1` is not a skip sentinel at the read site; it becomes direction `7` via `dir & 7` and opposite `3` via `(dir - 4) & 7`. Reachability is guarded earlier by bridge tile/base/height context, not by a post-read `if dir == -1`.

### 3.2 Overlay `0x18/0x19` constants

`OverlayClass::Mark @ 0x005FC570` is the map overlay mark path. Assembly around the bridge constants proves:

- `0x005FC5EE CMP ESI,0x18; 0x005FC5F1 JZ 0x005FC605`; at `0x005FC605..0x005FC60A`, the call pushes `state=1`, `dir=EDI`, where `EDI` was zeroed at `0x005FC5EC`. Result: overlay `0x18 -> SetBridgeDirection_NESW(dir=0,state=1)`.
- If the `0x18` jump is not taken, `0x005FC5F3 CMP ESI,0x19`; when equal, execution falls through `PUSH 0x1; PUSH 0x6; CALL 0x0047E040` at `0x005FC5F8..0x005FC5FE`. Result: overlay `0x19 -> SetBridgeDirection_NESW(dir=6,state=1)`.

Active in YR: Yes. `OverlayClass::Mark` is reached by the normal `[OverlayPack]` map-load object construction path and by editor/overlay placement paths; standard maps with these bridge anchors hit it.

### 3.3 Bit `0x800` writer polarity

`CellClass::SetBridgeDirection_NESW @ 0x0047E040` writes state and flags from the direction parameter:

- Function start writes `+0x11E = 0` if direction is zero, else `9` (`0x0047E04D..0x0047E05E`).
- `0x0047E09B` loads the direction parameter; `0x0047E09F TEST EDX,EDX`; `0x0047E0A1 SETZ DL`; `0x0047E0AC SHL EDX,0xB`. This produces bit `0x800` only when `direction == 0`.
- The computed bit is ORed into the anchor flags and propagated to the stamped neighbor cells whose masks rewrite this bit.

Therefore:

| Overlay id | Helper | Direction | Stamp axis from cells | Default `+0x11E` | `Flags & 0x800` |
|---:|---|---:|---|---:|---|
| `0x18` | NESW | `0` | anchor + N,N,N + S = N-S body axis | `0` | set |
| `0x19` | NESW | `6` | anchor + W,W,W + E + extra E = E-W body axis | `9` | clear |

### 3.4 Stamp/reference cells

For normal map-load directions:

- Direction `0`: anchor, forward1=N, forward2=Nx2, forward3=Nx3, opposite=S. No extra cell.
- Direction `6`: anchor, forward1=W, forward2=Wx2, forward3=Wx3, opposite=E, plus an extra E step from the opposite slot. Evidence: `SetBridgeDirection_NESW` decompile and the `CMP direction,0x6` branch at `0x0047E3FF..0x0047E452`.

Per-slot material behavior is inherited from the verified stamp report and re-checked in `0x0047E040`: anchor/forward1/forward2/opposite receive anchor relation/state-byte writes and terrain dirty calls; forward3 is `0x1000`-only; the direction-6 extra cell is `0x10000`-only plus anchor pointer. Destroy-state stamping calls `BlowUpBridge` only on anchor, forward1, forward2, and opposite.

### 3.5 State-machine axis split

`ProcessBridgeDamageStateMachine_High @ 0x00576BA0` reconciles the same axis:

- If the damaged cell is not the `0x80` anchor, the code uses `cell+0x2C` to find the anchor when needed.
- Switch cases `0..=8` call the `UpdateRamp_NS_*_High` family. Collapse cases call `SetBridgeDirection_NESW(0,0)` and clear overlay/state.
- Switch cases `9..=17` call the `UpdateRamp_EW_*_High` family. Collapse cases call `SetBridgeDirection_NESW(6,0)` and clear overlay/state.
- Direction arguments inside the state machine are perpendicular to body axis for ramp update: NS states use E/W (`2/6`); EW states use S/N (`4/0`).

Active in YR: Yes, conditional on `DestroyableBridges`; stock YR default is enabled per existing `DESTROYABLEBRIDGES_INI_GATE_GHIDRA_REPORT.md` and bridge damage reports.

### 3.6 Bridge-zone direction tables

`MapClass::ComputeBridgeZones @ 0x0056D6E0` uses the adjacent static tables in this order:

1. choose bridge base: `DAT_00AA0E28` high bridge or `DAT_00ABAD1C` wood/low;
2. compute `tile_offset = IsoTileTypeIndex - bridge_base`;
3. compare `DAT_0082A734[tile_offset]` to `CellClass.Height`;
4. read `DAT_0082A774[tile_offset]` as span-walk direction;
5. walk using `Pathfinding_update_continued`;
6. compare far candidate height to `DAT_0082A7B4[next_tile_offset]`.

`MapClass::AddBridgeZoneEdges @ 0x005851B0`, `RemoveBridgeZoneEdges @ 0x00584E50`, and `FUN_00582D70` read `DAT_0082A944[tile_offset]` and derive endpoint extension pairs using both `dir & 7` and `(dir - 4) & 7`.

Important table contents from the prior verified memory dump:

| Table | Meaning | Values |
|---|---|---|
| `0x0082A734` | start-height gate | `[7,7,-1,7,7,-1,4,4,4,4,4,2,2,2,2,2]` |
| `0x0082A774` | span walk direction | `[2,2,-1,4,4,-1,2,2,2,2,2,4,4,4,4,4]` |
| `0x0082A7B4` | far endpoint height gate | `[-1,-1,4,-1,-1,2,4,4,4,4,4,2,2,2,2,2]` |
| `0x0082A944` | bridgehead step direction | `[0,0,-1,2,2,-1,0,0,0,0,0,2,2,2,2,2]` |

## 4. INI Keys

No INI key defines the bridge direction tables or the `0x18/0x19 -> dir` constants. They are binary constants.

| INI key / data | Default / role | Effect in this slice | Active in YR |
|---|---|---|---|
| `[CombatDamage] DestroyableBridges` | stock YR `yes` per prior gate report | Enables bridge damage state-machine paths; does not alter axis polarity. | Yes |
| `[CombatDamage] BridgeStrength` | stock YR damage probability threshold | Affects whether damage reaches state machine; no direction effect. | Yes |
| Theater `BridgeSet` / `WoodBridgeSet` | loaded into `DAT_00AA0E28` / `DAT_00ABAD1C` | Base tile index for 16-entry table indexing. | Yes |
| `[OverlayTypes]` bridge entries | map overlay ids | Map data supplies overlay ids; the `0x18/0x19` constants are still hard-coded in `OverlayClass::Mark`. | Yes |
| `[OverlayPack]` / `[OverlayDataPack]` | map file sections | Overlay pack triggers stamping; overlay data pack later overwrites final `+0x11E` bytes. | Yes |

## 5. Integration Points

| Function / path | Role | Evidence | Active in YR |
|---|---|---|---|
| `OverlayClass::Mark @ 0x005FC570` | Converts overlay ids `0x18/0x19/0xED/0xEE` into bridge stamp helper calls. | Decompile plus assembly `0x005FC5EE..0x005FC62C` | Yes |
| `SetBridgeDirection_NESW @ 0x0047E040` | Stamps reference cells, default state bytes, anchor pointers, and bit `0x800` polarity. | Decompile plus assembly `0x0047E04D..0x0047E0AC`, `0x0047E3FF` | Yes |
| `SetBridgeDirection_NWSE @ 0x0047E470` | Compiled twin used for `0xED/0xEE`; not deeply re-decompiled here because slot target is `0x18/0x19` and NESW polarity. | Prior stamp report; OverlayClass caller constants | Yes |
| `ProcessBridgeDestruction_High @ 0x00573540` | Reads bit `0x800` to choose fallback walker directions. | Decompile: destroyed/ramp branch uses `((flags & 0x800) ? 2 : 0) + 2`; later hop uses `-(flags&0x800!=0) & 6`. | Yes |
| `ProcessBridgeDamageStateMachine_High @ 0x00576BA0` | Reads `+0x11E` and dispatches `0..8` to NS, `9..17` to EW; collapse calls `SetBridgeDirection_NESW(0/6,0)`. | Decompile switch cases | Yes |
| `MapClass::ComputeBridgeZones @ 0x0056D6E0` | Builds endpoint records from bridge tile tables. | Decompile table reads `0x0082A734/774/7B4` | Yes |
| `AddBridgeZoneEdges @ 0x005851B0`, `RemoveBridgeZoneEdges @ 0x00584E50`, `FUN_00582D70` | Use `DAT_0082A944` and wrapped direction/opposite direction to add/remove or temp-inject zone edges. | Assembly reads at `0x0058523F`, `0x00584EEB`, `0x00582F42` | Yes |

## 6. Current Rust Implementation Status

Current Rust already carries most of the verified contract:

- `src/map/bridge_facts.rs` has `BRIDGE_FLAG_DIRECTION_ZERO = 0x800`, classifies `0x18 -> (Nesw,0)` and `0x19 -> (Nesw,6)`, stamps forward/opposite cells, and uses `direction.wrapping_sub(4) & 7`.
- `src/map/resolved_terrain.rs` runs a bridge-fact pass after initial terrain construction, calls `stamp_set_bridge_direction`, then applies `OverlayDataPack` bytes to `state_byte`, matching the load-order contract.
- `src/sim/bridge_state/mod.rs` defines `Direction` as `N=0, NE=1, E=2, SE=3, S=4, SW=5, W=6, NW=7`, maps stamp `direction 0` to `Axis::NS` and `direction 6` to `Axis::EW`, and preserves state byte `0..8`/`9..17`.
- `src/sim/world/bridge_orchestrator.rs` has hut fallback logic using `BRIDGE_FLAG_DIRECTION_ZERO`; this matches the verified high-level polarity but should receive explicit regression tests because the binary has two separate walker formulas for destroyed/ramp fallback.

Potential Rust delta / review point: `src/map/resolved_terrain.rs` still has a legacy `BridgeDirection` overlay-layer classifier that maps ids `24|237` as `EastWest` and `25|238` as `NorthSouth`. This is separate from the new `bridge_facts` pass, but future code must not treat that legacy view as the authoritative source for high-bridge axis when `bridge_facts` exists.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Target question / non-goals / stop conditions | verified | Section 0 | none |
| `OverlayClass::Mark` `0x18/0x19` constants | verified | `0x005FC5EE..0x005FC60A` assembly/decompile | none |
| `SetBridgeDirection_NESW` `0x800` polarity | verified | `0x0047E09B..0x0047E0AC`; decompile `uVar13=(direction==0)<<0xb` | none |
| `SetBridgeDirection_NESW` stamp slots | verified | `0x0047E040` decompile; `0x0047E3FF` extra-dir6 branch | none |
| `SetBridgeDirection_NWSE` twin | touched-not-exhausted | prior `BRIDGE_SETBRIDGEDIRECTION_STAMPING_GHIDRA_REPORT.md`; OverlayClass calls `0x0047E470` | full twin re-decompile if diagonal helper becomes its own target |
| High state-machine NS/EW split | verified | `0x00576BA0` decompile switch; calls `SetBridgeDirection_NESW(0,0)` / `(6,0)` | none for axis split |
| High destruction fallback `0x800` reads | verified | `0x00573540` decompile | full hut/death lifecycle remains out-of-scope |
| Bridge-zone static table contents | verified-by-prior-doc | `BRIDGE_DIRECTION_TABLES_GHIDRA_REPORT.md` memory dumps; current assembly reads | live memory read unavailable in this session; prior dump stands |
| Bridge-zone table direct readers | verified | `0x0056D6E0`, `0x005851B0`, `0x00584E50`, `0x00582D70`; assembly `0x0058523F`, `0x00584EEB`, `0x00582F42` | none for direction use |
| Direction `-1` entries not post-read skips | verified | decompile/assembly wraps with `&7`; prior table dump | actual normal-map reachability of offsets 2/5 remains data validation |
| INI ownership of constants | verified | no table population from INI in decompiled readers; prior direction table report | none |
| Current Rust surfaces | touched-not-exhausted | scanned `bridge_facts.rs`, `resolved_terrain.rs`, `bridge_state/mod.rs`, `bridge_orchestrator.rs` | run focused tests in implementation task |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Which overlay id maps to direction zero? -> overlay 0x18 jumps to the `PUSH EDI`/`dir=0` call.` (evidence: `0x005FC5EE..0x005FC60A`)
- `[RESOLVED] OQ-02 - Which overlay id maps to direction six? -> overlay 0x19 falls through to `PUSH 0x6` before `CALL 0x0047E040`.` (evidence: `0x005FC5F3..0x005FC5FE`)
- `[RESOLVED] OQ-03 - What bit polarity does `0x800` carry? -> `0x800` is set only when direction parameter is zero.` (evidence: `0x0047E09B..0x0047E0AC`)
- `[RESOLVED] OQ-04 - What state byte does direction zero write? -> default `+0x11E=0`.` (evidence: `0x0047E04D..0x0047E05E`)
- `[RESOLVED] OQ-05 - What state byte does nonzero direction write? -> default `+0x11E=9`.` (evidence: `0x0047E04D..0x0047E05E`; decompile later writes `-(direction!=0)&9`)
- `[RESOLVED] OQ-06 - What cells does direction zero stamp? -> anchor, N, N2, N3, S.` (evidence: `0x0047E040` decompile using `dir&7` and `(dir-4)&7`)
- `[RESOLVED] OQ-07 - What cells does direction six stamp? -> anchor, W, W2, W3, E, extra E.` (evidence: `0x0047E040`; extra branch `0x0047E3FF..0x0047E452`)
- `[RESOLVED] OQ-08 - Does the damage state machine agree with that axis? -> yes; bytes `0..8` call NS helpers and collapse with `dir=0`; bytes `9..17` call EW helpers and collapse with `dir=6`.` (evidence: `0x00576BA0`)
- `[RESOLVED] OQ-09 - Does the destruction fallback read `0x800` live? -> yes; it chooses cardinal walker directions from `flags & 0x800`.` (evidence: `0x00573540`)
- `[RESOLVED] OQ-10 - Are bridge-zone direction table readers active? -> yes; map-load/full build and update helpers read them on bridge records.` (evidence: `0x0056D6E0`, `0x005851B0`, `0x00584E50`, `0x00582D70`)
- `[RESOLVED] OQ-11 - Are `dir & 7` and `(dir - 4) & 7` both present in active code? -> yes in stamp and zone edge readers.` (evidence: `0x0047E040`, `0x005851B0`, `0x00582D70`)
- `[RESOLVED] OQ-12 - Do `DAT_0082A944` `-1` values mean skip? -> not in direct Add/Remove/FUN_00582D70 readers; they wrap through `&7`.` (evidence: `BRIDGE_DIRECTION_TABLES_GHIDRA_REPORT.md`; `0x0058523F..0x0058524B`, `0x00582F42..0x00582F52`)
- `[RESOLVED] OQ-13 - What is Active in YR? -> map-load, bridge-zone, damage, collapse/repair paths are active in normal YR when maps contain bridges and DestroyableBridges is enabled.` (evidence: `0x005FC570`, `0x0056D6E0`, `0x00576BA0`; prior DestroyableBridges report)
- `[RESOLVED] OQ-14 - Is `BRIDGE_SYSTEM.md` stale on `0x800` polarity? -> current copy already has corrected wording; older stale wording should be replaced wherever found.` (evidence: `BRIDGE_SYSTEM.md` current flag table; `BRIDGE_ANCHOR_OVERLAY_18_19_AXIS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-15 - Does Rust have an authoritative bridge-fact pass? -> yes, `bridge_facts.rs` plus `resolved_terrain.rs` stamp map overlays and apply overlay data after stamping.` (evidence: `src/map/bridge_facts.rs`, `src/map/resolved_terrain.rs`)
- `[DEFERRED] OQ-16 - Which stock maps place every table offset that contains `-1`?` (category: out-of-scope; reason: requires retail map data scan, not binary polarity proof; next-step-if-pursued: dump `BridgeSet/WoodBridgeSet` tile offsets from stock maps and compare reachability)
- `[DEFERRED] OQ-17 - Full low-bridge TubeClass axis semantics.` (category: requires-different-system-context; reason: this slice only checked bridge-zone/table polarity; next-step-if-pursued: extend `LOW_BRIDGE_TUBECLASS_*` reports with cardinal naming audit)
- `[DEFERRED] OQ-18 - Exact visual render frame for every anchor/ramp state.` (category: requires-different-system-context; reason: render table is a different consumer; next-step-if-pursued: verify `DrawOverlay_Body` frame math against `+0x11E` and tile ids)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `0x18` is direction-zero/N-S, and sets bit `0x800`; `0x19` is direction-six/E-W and clears it. | `0x005FC5EE..0x005FC60A`; `0x0047E09B..0x0047E0AC` | none observed in `bridge_facts.rs` | `src/map/bridge_facts.rs` | Preserve `0x18 -> (Nesw,0)` and `0x19 -> (Nesw,6)` as the authoritative map-load axis source. | `bridge_overlay_18_stamps_ns_and_sets_direction_zero_flag`; `bridge_overlay_19_stamps_ew_and_clears_direction_zero_flag` | Do not infer `0x800 set = EW`; do not let legacy overlay-layer labels override bridge facts. |
| Direction indices are the standard 8-way table and opposite is `(dir - 4) & 7`. | `0x0047E040`; `0x005851B0`; prior `GDIRECTIONOFFSETS_0089F688...` | none observed | `src/map/bridge_facts.rs`, `src/sim/bridge_state/mod.rs` | Keep one shared direction contract: `0=N,2=E,4=S,6=W`; wrap with `&7`; opposite via wrapping subtract. | `bridge_direction_opposite_matches_binary_wrapping`; `bridge_stamp_uses_wrapped_opposite_for_tail_cell` | Do not use enum ordinal order that differs from binary table indices. |
| Direction `0` stamp cells are anchor/N/N2/N3/S; direction `6` stamp cells are anchor/W/W2/W3/E/extraE. | `0x0047E040`, `0x0047E3FF..0x0047E452` | none observed in unit tests | `src/map/bridge_facts.rs`, `src/sim/bridge_specs` | Preserve forward3 and extra-dir6 as flag-only slots, not structural bridge deck cells. | `bridge_stamp_dir0_reference_cells_match_binary`; `bridge_stamp_dir6_extra_east_cell_is_extra_side_only` | Do not broaden the stamp into side expansion, gap fill, or component-wide bridge inference. |
| State bytes `0..8` are N-S; `9..17` are E-W; collapse calls re-stamp with `dir=0`/`dir=6` respectively. | `0x00576BA0` | none observed | `src/sim/bridge_state/mod.rs` | State-byte decoding, axis, render state byte, and collapse state should stay tied to this split. | `bridge_state_byte_zero_to_eight_maps_ns`; `bridge_state_byte_nine_to_seventeen_maps_ew` | Do not classify axis by visual tile family after a valid bridge fact/state byte is present. |
| Bridge-zone table `DAT_0082A944` uses `dir & 7` and `(dir - 4) & 7`; `-1` is not a skip at Add/Remove readers. | `0x0058523F`, `0x00584EEB`, `0x00582F42`; prior table dump | partial/unchecked; Rust zone graph is not a direct port | `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_map.rs` | If/when binary-like bridge zone edges are implemented, represent the exact signed table and wrap behavior. | `zone_bridge_direction_table_minus_one_wraps_to_nw_and_se_when_reached` | Do not translate table `-1` entries to `None` without proving the same caller-side guard. |
| High destruction fallback reads `0x800` for walker directions: set chooses S in one branch and W in the forward hop; clear chooses E and N. | `0x00573540` | needs focused tests | `src/sim/world/bridge_orchestrator.rs` hut fallback, bridge repair/hut collapse paths | Keep `BRIDGE_FLAG_DIRECTION_ZERO` semantics as direction-zero/N-S and mirror the separate branch formulas. | `hut_fallback_direction_zero_scans_south_then_hops_west`; `hut_fallback_direction_six_scans_east_then_hops_north` | Do not collapse the two fallback walker formulas into a generic "axis forward" without matching branch context. |

### Stale Docs / Follow-up Docs

Use this replacement wording anywhere stale polarity remains:

> `CellClass+0x140 bit 0x800` is set by `SetBridgeDirection_*` only when the stamp direction argument is zero. For the `0x18/0x19` NESW anchors, that means `0x18 -> dir 0 -> N-S -> bit set`, while `0x19 -> dir 6 -> E-W -> bit clear`. The bit is therefore a direction-zero/N-S stamp marker in this axis context, not an E-W marker.

Also avoid this stale shorthand:

> "0x18/0x19 are high/low by themselves."

Better:

> `0x18/0x19` are the NESW anchor overlay ids consumed by `OverlayClass::Mark`; their axis is not re-read from the id after stamping. Runtime consumers use stamped `+0x11E`, `+0x140` flags, and anchor pointers.

## 10. Negative Facts / Do Not Do

- Do not treat `Flags & 0x800` as "E-W". It is set for `direction == 0`, which is the N-S stamp for `0x18`.
- Do not treat `DAT_0082A944` `-1` entries as automatic "no bridgehead direction" in Add/Remove/FUN_00582D70; direct readers wrap them.
- Do not derive high-bridge deck cells by expanding side cells, normalizing connected components, or gap filling when stamped bridge facts exist.
- Do not conflate body axis with ramp-update direction. NS body states use E/W ramp update directions; EW body states use S/N ramp update directions.
- Do not use overlay id `0x18/0x19` as the live runtime axis discriminator after stamping; use `+0x11E`, `0x800`, and anchor relation.

## 11. Remaining Uncertainty

- Stock-map data validation for rare `DAT_0082A944` offsets `2` and `5` was not performed.
- The `SetBridgeDirection_NWSE` diagonal helper was not re-decompiled in this pass beyond prior report reliance and callsite confirmation; the target scope was `0x18/0x19` NESW polarity.
- Full low-bridge TubeClass cardinal naming remains a separate low-bridge investigation.
- Current Rust was scanned for relevant surfaces but not tested in this research-only task.

## Sources

- Ghidra decompiled/read this session: `0x0047E040`, `0x005FC570`, `0x00573540`, `0x00576BA0`, `0x0056D6E0`, `0x005851B0`, `0x00582D70`.
- Ghidra assembly context this session: `0x005FC5FE`, `0x005FC60A`, `0x0047E09F`, `0x0047E0A1`, `0x0047E0AC`, `0x0047E3FF`, `0x0058523F`, `0x00584EEB`, `0x00582F42`.
- Prior reports referenced: `BRIDGE_ANCHOR_OVERLAY_18_19_AXIS_GHIDRA_REPORT.md`, `BRIDGE_DIRECTION_TABLES_GHIDRA_REPORT.md`, `BRIDGE_SETBRIDGEDIRECTION_STAMPING_GHIDRA_REPORT.md`, `GDIRECTIONOFFSETS_0089F688_BRIDGE_MARKER_PATH_GHIDRA_REPORT.md`, `DESTROYABLEBRIDGES_INI_GATE_GHIDRA_REPORT.md`, `BRIDGE_SYSTEM.md`.
- Rust surfaces scanned: `src/map/bridge_facts.rs`, `src/map/resolved_terrain.rs`, `src/sim/bridge_state/mod.rs`, `src/sim/world/bridge_orchestrator.rs`, `src/app_target_lines.rs` via text search.
