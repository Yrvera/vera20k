# Bridge Presentation Radar Dirty - Ghidra Research Report

**Address(es):** `0x00519630`, `0x0065FA70`, `0x0065FB80`, `0x006551C0`, `0x00655250`, `0x00655C50`, `0x0047E040`, `0x0047E470`, `0x0057F6A0`, `0x0057FBC0`, `0x005800D0`, `0x00580600`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Bridge repair/collapse radar/minimap dirty behavior: repair `RadarEvent` type 14 timing, terrain-dirty cells for repair/collapse mutation paths, and whether repair and collapse use distinct update paths.
**Non-Scope:** Audio mixer parity, exact bridge SHP visual frame math, TWLT/debris RNG, and full first-frame tactical render equality except where it gates radar/minimap dirty output.
**Confidence:** High for repair event timing, repair dirty-cell gating, radar dirty queue mechanics, and collapse `SetBridgeDirection` dirty calls. Medium for complete collapse caller breadth because C4/damage walkers have many callers and this slice spot-checked the shared state-stamp primitive rather than every collapse entry.
**Active in YR:** Yes. Evidence: standard bridge repair hut branch in `InfantryClass__PerCellProcess`; standard collapse/damage paths call `SetBridgeDirection_*`; `DestroyableBridges=yes` and standard bridge overlays exist in stock `rulesmd.ini`.

## Target Question

Does retail `gamemd.exe` update bridge repair/collapse radar/minimap state by event-only behavior, terrain dirty cells, or separate paths; and what should Rust carry forward?

## Non-Goals

- Do not re-audit `RepairBridgeSound=BridgeRepaired` mixer volume/sample onset.
- Do not re-audit TWLT animation `Report=` sound or debris placement.
- Do not implement Rust changes in this report.
- Do not treat tactical dirty rectangles as sim responsibilities.

## Evidence Needed To Mark COMPLETE

- Repair branch decompile plus assembly range around `CreateRadarEvent(14, cell)`.
- `CreateRadarEvent` / `InitRadarEvent` decompile proving type config, dedup return, and ring-buffer side effect.
- Radar terrain dirty function decompile proving dirty-list semantics and processing path.
- Repair walker decompile proving which repaired cells call `MarkTerrainDirty`.
- Collapse shared stamp decompile proving `BlowUpBridge` and terrain dirty order.
- Rust scan proving current delta and test surfaces.

## Stop Conditions

- Stop before audio mix/sample parity.
- Stop before full damage/collapse visual RNG.
- Stop if Ghidra function boundaries are missing; record uncertainty instead of mutating Ghidra.
- Stop before modifying Rust or repo docs other than `.swarm-claims.md`.

## 1. Overview

Bridge repair has two separate presentation outputs. First, the engineer/CABHUT branch creates radar event type `14` and only plays `EVA_BridgeRepaired` when that event is not suppressed. Second, the repair walkers mark terrain dirty only for destroyed-anchor restoration, not for every repaired/damaged bridge tile.

Bridge collapse uses different update plumbing. Collapse state stamping calls `CellClass::BlowUpBridge` and then `RadarClass::MarkTerrainDirty` for the affected anchor/body/opposite cells; collapse tails separately set tactical redraw state and zone rebuild work. There is no `BridgeRepaired` radar event on collapse.

## 2. Class Layout / Key Offsets

| Struct / object | Offset | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| `CellClass` | `+0x24/+0x26` | packed map coord passed to `MarkTerrainDirty` | `0x006551C0`, `0x0047E040`, walkers | Yes |
| `CellClass` | `+0x44` | overlay id; repair walkers key destroyed anchors `0x64/0x65/0xE7/0xE8` | `0x0057F6A0`, `0x0057FBC0`, `0x005800D0`, `0x00580600` | Yes |
| `CellClass` | `+0x11E` | bridge state byte updated by `SetBridgeDirection` | `0x0047E040`, `0x0047E470` | Yes |
| `CellClass` | `+0x140` | bridge flags; collapse state sets `0x400`, clears/sets bridge bits | `0x0047E040`, `0x0047E470` | Yes |
| `RadarClass` | `+0x1228` | terrain dirty cell vector data | `0x006551C0`, `0x00655250` | Yes |
| `RadarClass` | `+0x1234` | terrain dirty count | `0x006551C0`, `0x00655250` | Yes |
| `RadarClass` | `+0x1260/+0x126C` | pixel/object dirty list and count | `0x006562D0`, `0x00655C50` | Yes |
| `RadarEvent` | `+0x00` | event type | `0x0065FB80` | Yes |
| `RadarEvent` | `+0x20` | source cell used by ring/spacebar cycling | `0x0065FB80` | Yes |

## 3. Core Logic

### 3.1 Engineer Repair Event Timing

In `InfantryClass__PerCellProcess @ 0x00519630`, the BridgeRepairHut branch checks the engineer's owner with `HouseClass__IsHumanPlayer`. If true, it gets the engineer/cell coordinate via vtable `+0x1B8`, loads `ECX=0xE`, and calls `CreateRadarEvent @ 0x0065FA70`.

Assembly range `0x00519BB0..0x00519BC9` verifies:

- `MOV ECX,0xE`; `CALL 0x0065FA70`
- `TEST AL,AL`; if zero, skip EVA
- otherwise `MOV ECX,0x825538`; `CALL 0x00752700` for `EVA_BridgeRepaired`

Then `0x00519BD3..0x00519C02` checks `RulesClass+0x248 != -1` and plays `RepairBridgeSound`, then `0x00519C07..0x00519D12` performs the 5x5 bridge scan and calls `ProcessBridgeDestruction_Low` or `_High`. Therefore repair radar event/EVA and optional repair sound occur before bridge mutation.

**Active in YR:** Yes. Standard CABHUT `BridgeRepairHut=yes`; stock `[AudioVisual] RepairBridgeSound=BridgeRepaired`; branch is reached by engineer repair.

### 3.2 Radar Event Type 14 Is Real But Non-Drawing

`CreateRadarEvent @ 0x0065FA70` deduplicates only when the per-type unique flag is set, allocates a 64-byte event, then calls `InitRadarEvent @ 0x0065FB80`. `InitRadarEvent` stores the event type, radar pixel, source cell, current frame, and pushes source cell into the 8-entry ring buffer.

Prior `RADAR_EVENT_CLASS_GHIDRA_REPORT.md` plus fresh decompile show type `14 = BridgeRepaired`, with config `{dedup=8, vis=0, blink=200, unique=yes}`. The same report verifies `DrawRadarEvent @ 0x00660050` default-color types, including 14, do not draw a minimap diamond. They still create queue/ring entries and gate the EVA return path.

**Active in YR:** Yes. The repair branch hardcodes `ECX=0xE` and uses `CreateRadarEvent` result.

### 3.3 Radar Terrain Dirty Queue

`RadarClass__MarkTerrainDirty @ 0x006551C0` scans the terrain dirty cell vector for duplicate packed coords and returns early on duplicate. If unique, it appends the 4-byte coord to `RadarClass+0x1228`, increments `+0x1234`, and sets `RadarClass+0x14D9 = 1`.

`RadarClass__ClearBackground @ 0x00655250` processes that terrain dirty list. For each in-playfield cell it calls `CellClass__GetRadarColor @ 0x0047C060`, writes the updated terrain RGB into the raw radar terrain buffer, computes a dirty screen rect, and clears/updates the list state. `RadarClass__RenderCellPixel @ 0x00655C50` is the separate pixel/object/shroud path.

**Active in YR:** Yes. Radar terrain dirty calls are reached by bridge walkers and collapse state stamping.

### 3.4 Repair Terrain Dirty Cells

The four repair walkers share the same shape:

- Low NS `0x0057F6A0`: only when prior overlay is `0x64`, call `MarkTerrainDirty` for main cell and its `y-1` / `y+1` neighbors.
- Low EW `0x0057FBC0`: only when prior overlay is `0x65`, call `MarkTerrainDirty` for main cell and its `x-1` / `x+1` neighbors.
- High NS `0x005800D0`: only when prior overlay is `0xE7`, call `MarkTerrainDirty` for main cell and its `y-1` / `y+1` neighbors.
- High EW `0x00580600`: only when prior overlay is `0xE8`, call `MarkTerrainDirty` for main cell and its `x-1` / `x+1` neighbors.

Damaged but not destroyed repair transitions still update overlays, dirty tactical screen rect, recalc three cells, and may update zones, but they do not call `MarkTerrainDirty` unless the prior overlay is the destroyed-anchor sentinel.

**Active in YR:** Yes. These walkers are called from the engineer repair branch through `ProcessBridgeDestruction_Low/High`.

### 3.5 Collapse Terrain Dirty Path

`CellClass__SetBridgeDirection_NESW @ 0x0047E040` and `CellClass__SetBridgeDirection_NWSE @ 0x0047E470` are byte-identical for this behavior. For `param_3 == 0`, each of anchor, forward1, forward2, and opposite cell performs:

1. Write bridge state/flags (`+0x11E = 0`, destroyed bit state).
2. Call `CellClass__BlowUpBridge`.
3. Call `RadarClass__MarkTerrainDirty(&cell.MapCoord)`.

Forward3 only gets a subset flag update and no terrain dirty call. The `param_2 == 6` special cell receives anchor/flag updates but no `MarkTerrainDirty`.

**Active in YR:** Yes. Collapse/damage paths call this shared state-stamping helper for standard bridge destruction.

### 3.6 Distinct Paths

Repair and collapse do not share one "bridge presentation dirty" path:

- Repair event/EVA: `InfantryClass__PerCellProcess` -> `CreateRadarEvent(14)` before mutation.
- Repair terrain dirty: only inside repair walkers and only for destroyed-anchor restoration.
- Collapse terrain dirty: inside `SetBridgeDirection_*` after `BlowUpBridge`.
- Collapse tactical redraw: collapse tails also set tactical redraw state (covered by prior `BRIDGE_COLLAPSE_FALLOUT_ORDERING_GHIDRA_REPORT.md`); this is not the radar event queue.

## 4. INI Keys

| Key | Default / stock value | Effect | Evidence | Active in YR |
|---|---|---|---|---|
| `[AudioVisual] RepairBridgeSound` | `BridgeRepaired` | Optional SFX only; after radar/EVA check and before bridge mutation | `rulesmd.ini`, `0x00519BD3..0x00519C02` | Yes |
| `EVA_BridgeRepaired` | present in `evamd.ini` | EVA string called only if `CreateRadarEvent(14)` returns true | `0x00519BC4`, string `0x00825538` | Yes |
| `RadarEventSuppressionDistances` arrays | documented six entries | Parsed but not used for type 14; type table has hardcoded row 14 | `RADAR_EVENT_CLASS_GHIDRA_REPORT.md`; `0x0065FA70` | Conditional: row 14 hardcoded |
| `DestroyableBridges` | `yes` | Upstream gate for bridge damage/destruction, not a direct radar dirty key | `rulesmd.ini`; prior destroyability report | Yes by default |

## 5. Integration Points

| Caller / callee | Relationship | Evidence | Active in YR |
|---|---|---|---|
| `InfantryClass__PerCellProcess -> CreateRadarEvent` | Creates type 14 before repair scan | decompile plus asm `0x00519BB0..0x00519BC9` | Yes |
| `CreateRadarEvent -> InitRadarEvent` | Allocates event and pushes ring source cell | `0x0065FA70`, `0x0065FB80` | Yes |
| repair walkers -> `RadarClass__MarkTerrainDirty` | Three-cell terrain dirty only for destroyed anchor sentinels | `0x0057F6A0`, `0x0057FBC0`, `0x005800D0`, `0x00580600` | Yes |
| `SetBridgeDirection_* -> BlowUpBridge -> MarkTerrainDirty` | Collapse/stamp terrain dirty path | `0x0047E040`, `0x0047E470` | Yes |
| `MarkTerrainDirty -> ClearBackground` | Dirty terrain cells recolored from `CellClass__GetRadarColor` | `0x006551C0`, `0x00655250`, `0x0047C060` | Yes |

## 6. Current Rust Implementation Status

Rust is close on collecting repair dirty cells but does not propagate them to the renderer/minimap. `src/sim/bridge_state/walker.rs:365` collects `outcome.radar_cells` for `0x64/0x65/0xE7/0xE8`, matching the destroyed-anchor gate. `src/sim/world/world_orders.rs:354` explicitly drops those cells because there is no render-side dirty channel.

Rust does not represent `RadarEventType::BridgeRepaired`. `src/sim/radar.rs:42` only includes the first six public event categories. `src/app_sim_tick.rs:530` converts `SimSoundEvent::BridgeRepaired` to SFX/EVA sound, but not to a radar event/ring entry.

Rust minimap terrain/overlay pixels are precomputed. `src/render/minimap.rs:43` stores a generated base image; `src/render/minimap.rs:58`/`:60` store static terrain and overlay pixels; `update_unit_dots` rebuilds from those cached arrays rather than applying bridge terrain dirty cells dynamically.

Collapse handling in `src/sim/world/bridge_orchestrator.rs:328..334` applies fallout, adjacent bridge update, span notification, and zone refresh. The scanned surface did not expose any bridge collapse radar-dirty output channel equivalent to `MarkTerrainDirty`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Repair `CreateRadarEvent(14)` timing | verified | `0x00519BB0..0x00519BC9`, `0x00519630` | none |
| `CreateRadarEvent` dedup/allocation/ring | verified | `0x0065FA70`, `0x0065FB80` | none for type 14 use |
| Type 14 draw behavior | verified via prior doc + spot-check dependency | `RADAR_EVENT_CLASS_GHIDRA_REPORT.md`, `0x00660050` | none |
| Terrain dirty queue append/dedupe | verified | `0x006551C0` | none |
| Terrain dirty processing | verified | `0x00655250`, `0x0047C060` | exact screen rect draw timing outside scope |
| Repair walker dirty-cell gate | verified | `0x0057F6A0`, `0x0057FBC0`, `0x005800D0`, `0x00580600` | none |
| Collapse shared terrain dirty stamp | verified | `0x0047E040`, `0x0047E470` | every collapse caller not exhaustively re-listed |
| Collapse tactical redraw flag | touched-not-exhausted | prior `BRIDGE_COLLAPSE_FALLOUT_ORDERING_GHIDRA_REPORT.md` | full tactical draw timing out-of-scope |
| Rust repair dirty cells | verified | `src/sim/bridge_state/walker.rs:365`, `src/sim/world/world_orders.rs:354` | needs implementation |
| Rust radar event type 14 | verified missing | `src/sim/radar.rs:42` | needs implementation |
| Rust minimap dynamic bridge terrain | verified missing/static | `src/render/minimap.rs:43..60`, `:205..272` | needs implementation |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is repair radar event type 14 live in YR? -> Yes, hardcoded `ECX=0xE` before `CreateRadarEvent`, in the BridgeRepairHut engineer branch.` (evidence: `0x00519BB0..0x00519BC9`)
- `[RESOLVED] OQ-02 - Does `CreateRadarEvent` gate EVA? -> Yes; AL return is tested, and `EVA_BridgeRepaired` is skipped when it returns 0.` (evidence: `0x00519BBB..0x00519BC9`)
- `[RESOLVED] OQ-03 - Does type 14 draw a minimap diamond? -> No; default-color event types do not draw, but still enqueue and update ring/dedup state.` (evidence: `RADAR_EVENT_CLASS_GHIDRA_REPORT.md`, `0x00660050`)
- `[RESOLVED] OQ-04 - Does repair terrain dirty run before or after mutation? -> It runs inside the repair walker when overlay changes, after event/EVA/SFX and during mutation.` (evidence: `0x00519630`, repair walker decompiles)
- `[RESOLVED] OQ-05 - Which repair cells are terrain-dirtied? -> Only the three-cell perpendicular strip for prior destroyed anchors `0x64/0x65/0xE7/0xE8`.` (evidence: `0x0057F6A0`, `0x0057FBC0`, `0x005800D0`, `0x00580600`)
- `[RESOLVED] OQ-06 - Do damaged-only repairs mark terrain dirty? -> No `MarkTerrainDirty` unless the prior overlay is the destroyed-anchor sentinel.` (evidence: same repair walkers)
- `[RESOLVED] OQ-07 - Does collapse use radar event type 14? -> No evidence in collapse stamp path; type 14 is repair branch only.` (evidence: `0x0047E040`, `0x0047E470`, `0x00519630`)
- `[RESOLVED] OQ-08 - What collapse dirty path marks minimap terrain? -> `SetBridgeDirection_*` calls `BlowUpBridge` then `MarkTerrainDirty` on anchor/body/opposite cells.` (evidence: `0x0047E040`, `0x0047E470`)
- `[RESOLVED] OQ-09 - Is terrain dirty distinct from object/pixel dirty? -> Yes; terrain dirty is `+0x1228/+0x1234` and pixel dirty is `+0x1260/+0x126C`.` (evidence: `0x006551C0`, `0x006562D0`)
- `[RESOLVED] OQ-10 - Does Rust collect repair dirty cells? -> Yes in walker outcome, but drops them at world order boundary.` (evidence: `src/sim/bridge_state/walker.rs:365`, `src/sim/world/world_orders.rs:354`)
- `[RESOLVED] OQ-11 - Does Rust have bridge repaired radar event type? -> No scanned enum variant or event push exists.` (evidence: `src/sim/radar.rs:42`, `src/app_sim_tick.rs:530`)
- `[RESOLVED] OQ-12 - Does Rust minimap recolor dirty bridge terrain dynamically? -> No, terrain/overlay pixels are cached/precomputed and refreshed from cache.` (evidence: `src/render/minimap.rs:43..60`, `:205..272`)
- `[DEFERRED] OQ-13 - Exact tactical dirty rect dimensions for every bridge repair/collapse draw update.` (category: out-of-scope; reason: target is radar/minimap dirty output, not tactical invalidation geometry; next-step-if-pursued: re-investigate `FUN_0047FDE0/FUN_0047FB90/FUN_0045A130` rect chain)
- `[DEFERRED] OQ-14 - Full C4/damage collapse caller matrix into `SetBridgeDirection_*`.` (category: bounded-cost-too-high; reason: shared dirty primitive is verified and enough for this slot; next-step-if-pursued: caller-matrix report for all bridge collapse entries)
- `[DEFERRED] OQ-15 - Runtime debugger validation of duplicate same-tick type-14 repairs.` (category: needs-runtime-debugger; reason: static evidence proves dedup return behavior; duplicate timing is covered by another swarm slot)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Repair creates `RadarEvent` type 14 before mutation; EVA is gated on successful event creation. | `0x00519BB0..0x00519BC9`, `0x0065FA70` | Missing radar event type/channel; Rust only emits sound/EVA event | `src/sim/radar.rs`, `src/sim/world/world_orders.rs`, `src/app_sim_tick.rs` | Add a bridge-repaired radar event/ring entry with type-14 dedup semantics and preserve event-before-mutation ordering at the sim boundary. | Engineer repairs a CABHUT bridge twice within 8 cells while first event is live; first repair queues type 14/ring/EVA, second suppresses EVA/radar. | Do not model this as a visible minimap diamond; type 14 is non-drawing. Proposed test: `bridge_repair_type14_radar_event_gates_eva_before_mutation`. |
| Destroyed-anchor repair marks exactly the three touched bridge cells terrain-dirty. | `0x0057F6A0`, `0x0057FBC0`, `0x005800D0`, `0x00580600` | Rust collects `radar_cells` but drops them | `src/sim/bridge_state/walker.rs`, `src/sim/world/world_orders.rs`, app/render minimap dirty channel | Propagate `RepairOutcome::radar_cells` to a minimap terrain dirty API and recolor those cells from current bridge state. | Repair high NS destroyed anchor `0xE7`; minimap refreshes only main, north, south bridge cells to healthy bridge color. | Do not dirty every repaired/damaged bridge tile; damaged-only repairs lack `MarkTerrainDirty`. Proposed test: `bridge_repair_destroyed_anchor_marks_three_minimap_cells`. |
| Collapse terrain dirty is separate from repair event path and occurs after `BlowUpBridge` in `SetBridgeDirection_*`. | `0x0047E040`, `0x0047E470` | No collapse radar-dirty output channel found | `src/sim/world/bridge_orchestrator.rs`, bridge state outcomes, minimap renderer | Carry collapse `SetBridgeDirection`/`BlowUpBridge` cells into terrain dirty output after the bridge state mutation. | C4 collapse changes bridge deck to destroyed/water; minimap terrain cell recolors on the next radar refresh without a `BridgeRepaired` event. | Do not reuse `BridgeRepaired` or type 14 for collapse; no repair EVA/SFX on collapse. Proposed test: `bridge_collapse_marks_minimap_terrain_dirty_without_repair_event`. |

## Negative Facts / Do Not Do

- Do not draw a visible radar diamond for `BridgeRepaired`; type 14 exists but default-color types do not draw.
- Do not play or enqueue `EVA_BridgeRepaired` when `CreateRadarEvent(14)` would be suppressed by same-type 8-cell dedup.
- Do not dirty minimap terrain for every damaged-only repair; `MarkTerrainDirty` is only in destroyed-anchor cases.
- Do not reuse `BridgeRepaired` radar/audio/event paths for bridge collapse.
- Do not put radar/minimap dirty queues in `sim` with a dependency on `render`; carry deterministic dirty cell outputs upward.

## Stale Docs / Follow-up Docs

- Replace wording "standard YR radar event type 14 is `BridgeRepaired`" with: "standard YR event type 14 is `BridgeRepaired`; it enqueues/ring-buffers and gates EVA, but it is a non-drawing default-color radar event."
- Replace wording "repair updates bridge terrain/radar dirty state" with: "repair marks radar terrain dirty only for destroyed-anchor sentinels `0x64/0x65/0xE7/0xE8`, using the three touched perpendicular cells."
- Replace wording "collapse minimap/radar terrain update uses bridge terrain dirty paths" with: "collapse uses `SetBridgeDirection_*`: `BlowUpBridge` then `MarkTerrainDirty` on anchor/body/opposite cells, plus separate tactical redraw/zone work."

## Remaining Uncertainty

- Exact tactical dirty rectangle dimensions were not redecoded; not needed for minimap/radar dirty parity.
- Full collapse caller breadth was not re-listed; shared `SetBridgeDirection_*` behavior is verified.

## Sources

- Ghidra decompile: `InfantryClass__PerCellProcess @ 0x00519630`
- Ghidra assembly: `0x00519BB0..0x00519BC9`, `0x00519BD3..0x00519D12`
- Ghidra decompile: `CreateRadarEvent @ 0x0065FA70`, `InitRadarEvent @ 0x0065FB80`
- Ghidra decompile: `RadarClass__MarkTerrainDirty @ 0x006551C0`, `RadarClass__ClearBackground @ 0x00655250`, `RadarClass__MarkCellDirty @ 0x006562D0`, `RadarClass__RenderCellPixel @ 0x00655C50`
- Ghidra decompile: `CellClass__SetBridgeDirection_NESW @ 0x0047E040`, `CellClass__SetBridgeDirection_NWSE @ 0x0047E470`
- Ghidra decompile: `MapClass__RepairBridgeWalker_NS_Low @ 0x0057F6A0`, `EW_Low @ 0x0057FBC0`, `NS_High @ 0x005800D0`, `EW_High @ 0x00580600`
- `C:/Users/enok/Documents/ra2-rust-game-docs/RADAR_EVENT_CLASS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_RADAR_MINIMAP_PIXEL_RENDER_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_COLLAPSE_FALLOUT_ORDERING_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/traces/BRIDGE_DEEP_SLOT5_AUDIO_RENDER_PRESENTATION_TRACE.md`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/world_orders.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/bridge_state/walker.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/radar.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/render/minimap.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app_sim_tick.rs`
