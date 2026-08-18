# Rim-Refresh Parity Scan — `update_adjacent_bridges`

Facet: rim refresh (`UpdateAdjacentBridges` + edge-tile re-selection + stub reset).
Rust under test: `src/sim/world/bridge_orchestrator.rs::update_adjacent_bridges` (lines 1024–1109).
gamemd anchors (all re-confirmed live this session via `get_function_by_address`):
- `MapClass__UpdateAdjacentBridges` @ `0x00571050` (Low caller) — body `0x00571050–0x0057148f`.
- `MapClass__UpdateAdjacentBridges_High` @ `0x00576770` — body `0x00576770–0x00576b99`.
- `MapClass__UpdateBridgeEdgeTiles_High` @ `0x00576200` — body `0x00576200–0x00576764`.
- `MapClass__UpdateBridgeEdgeTiles_Low` @ `0x00570ae0` — body `0x00570ae0–0x00571044`.
- `CellClass__SetBridgeDirection_NESW` @ `0x0047e040`, `RepairBridgeSegment` @ `0x00575ee0`.

Bottom line: the Rust `update_adjacent_bridges` is a **rewritten approximation that shares
nothing with the binary's algorithm except "walk near the collapse."** It performs a
*destroy-stub-blanking* pass (overlay→0xFF, deck_present→false) that the binary NEVER does in
this function. The binary instead does a two-stage **edge-tile re-evaluation** (`UpdateAdjacentBridges_*`
finds a bridge-head, classifies it, then calls `UpdateBridgeEdgeTiles_*` which walks the ramp,
re-stamps the cap tile via `RepairBridgeSegment`, and on a vanished ramp clears exactly ONE
cell). This is wholesale DRIFT, enumerated below.

---

### D1: Rust rim refresh implements the WRONG algorithm entirely (stub-blank vs edge-tile re-eval)

- Rust now: `update_adjacent_bridges` (bridge_orchestrator.rs:1035–1109) does — Phase A: 8-dir
  walk for a neighbor whose `role == Bridgehead` OR `damage_state == Destroyed`; Phase B: walk
  up to 30 cells in that direction and, for every cell whose `anchor_span_id` no longer resolves
  to a live anchor span, set `overlay_byte = 0xFF`, `damage_state = Healthy{variant:0}`,
  `bridge_group_id = None`, `deck_present = false`. It mutates an arbitrary number of cells.
- gamemd: `UpdateAdjacentBridges_High` @ `0x00576770` — Phase A: 8-dir walk stopping on first
  cell with `flags & 0x500 != 0` (head/destroyed). Phase B: pick a *coord* (not a "direction to
  walk and blank") from the matched cell's flag bits (`0x100`/`0x400`/`0x80`/`0x800`). Phase C:
  walk that coord forward, and for the FIRST cell matching one of four `(normalized_tile_idx,
  cell+0x11A)` ramp-class patterns, call `UpdateBridgeEdgeTiles_High(coord, mode∈{2,4}, &rect)`
  then `DirtyScreenRect`, and **return**. `UpdateAdjacentBridges` itself writes ZERO cell fields
  — only the local rect. ALL cell mutation happens inside `UpdateBridgeEdgeTiles_*`.
- Fixture: HIGH bridge collapse at anchor (10,10), NS axis. Rust gets rim_cells = perpendiculars
  {(11,10),(9,10)} (compute_adjacent_bridges_dirty, mod.rs:2028). For rim cell (11,10): Phase A
  finds the Destroyed anchor at (10,10) → head_dir=(-1,0). Phase B walks W from (11,10): (10,10)
  is Destroyed → `continue` (skip); (9,10),(8,10)… if any still carry `deck_present` and a dead
  `anchor_span_id`, Rust blanks them to overlay 0xFF. The binary, given the same collapse,
  instead steps to the ramp-edge cell, calls `UpdateBridgeEdgeTiles_High`, which (if it finds the
  ramp transition) clears exactly ONE cell and re-stamps the cap tile — leaving the deck overlay
  bytes on every intermediate cell intact.
- Player sees: after a high-bridge span is destroyed, the surviving bridge cells either keep the
  wrong (un-recapped) edge tile (binary path not done) OR get blanked to no-overlay when they
  should retain their destroyed-deck art (Rust over-blanks). Different surviving-stub sprite at
  the break. Triggers every time a multi-cell bridge span collapses (combat, IC, demo truck).
- Severity: HIGH (visible bridge-edge art divergence on every span collapse).
- Confidence: PROVEN-DRIFT
- Verify-call: `decompile_function 0x00576770`, `decompile_function 0x00576200`.

---

### D2: Rust does no edge-tile re-selection at all — `UpdateBridgeEdgeTiles_*` callee is entirely missing

- Rust now: there is no analog of `UpdateBridgeEdgeTiles_High/_Low`. `update_adjacent_bridges`
  blanks stub cells and stops; it never re-classifies the ramp-edge tile, never re-stamps a cap.
  The orchestrator header comment (line 8, 122) literally calls it "Stub today."
- gamemd: `UpdateBridgeEdgeTiles_High` @ `0x00576200` is the real state writer. It (a) walks up
  to `0x1e`=30 cells looking for a ramp-edge tile class (`param_3==2` checks against
  `DAT_00abc1e8`/`DAT_00aa0e38`/`DAT_00abad30[..+3]` with `cell+0x11A==4`; `param_3==4` checks
  `DAT_00abc1d0`/`DAT_00aa1540`/`DAT_00aa1028[..+3]` with `cell+0x11A==2`); (b) accumulates a
  dirty rect via `CoordsToClient2`; (c) walks back detecting `flags & 0x80` transitions —
  was-set→now-clear ⇒ `SetBridgeDirection_NESW(dir,0)`, clear `+0x11E`, write `+0x44 = -1`
  (overlay −1), `MarkTerrainDirty`, recurse; was-clear→now-set ⇒ one `RepairBridgeSegment` call
  (cap re-stamp), latched once per walk via `bVar2`.
- Fixture: HIGH ramp at the edge of a collapsed span, `param_3=2` (S-walk). Binary walks N from
  the head until it hits the ramp-class tile (e.g. `cell+0x38 - DAT_00aa0e28 + 1 == DAT_00abad30`
  with `cell+0x11A==4`), unions the rect, then on the back-walk where the `flags&0x80` went
  set→clear it clears exactly that boundary cell's overlay to −1 and re-stamps the cap. Rust does
  NONE of this; it instead blanks every dead-span cell it walks over. The two produce different
  overlay bytes on different cells.
- Player sees: the bridge ramp/cap tile is not re-evaluated after a collapse — the ramp keeps its
  pre-collapse art instead of switching to the open-end cap art. Every span collapse adjacent to
  a ramp.
- Severity: HIGH.
- Confidence: PROVEN-DRIFT
- Verify-call: `decompile_function 0x00576200`, `decompile_function 0x00570ae0`.

---

### D3: Stub-reset condition + written field set diverge (overlay 0xFF & deck_present=false vs SetBridgeDirection group-clear)

- Rust now: per-cell reset writes (bridge_orchestrator.rs:1101–1106): `overlay_byte = 0xFF`,
  `damage_state = Healthy{variant:0}`, `bridge_group_id = None`, `deck_present = false`. Gate:
  `cell.deck_present && damage_state != Destroyed && anchor_span_id resolves to a dead span`.
- gamemd: when `UpdateBridgeEdgeTiles_High` clears a cell it writes `puVar15[0x11e] = 0`
  (damage_state) and `*(puVar15+0x44) = 0xffffffff` (overlay = −1) and calls
  `SetBridgeDirection_NESW(uVar17,0)` where `uVar17 = (uVar3==2 ? 0 : 6)` BEFORE the clear. That
  `SetBridgeDirection_NESW(dir,0)` (`0x0047e040`) does NOT just touch one cell — with param_2≠0,
  param_3=0 it walks the anchor + 3 forward neighbors + the (dir−4)&7 opposite neighbor,
  AND-masks each cell's `Flags` (clearing bits 0x80/0x100/0x200/0x400/0x1000/0x10000/0x800),
  sets each `+0x11E = (param_2!=0 ? 9 : 0)`. The clear is a multi-cell flag/state group-edit
  with `RadarClass__MarkTerrainDirty` per cell — not a per-cell overlay blank. gamemd never sets
  a "deck_present=false" field; it clears the structural/head flag bits via the AND-mask. gamemd
  writes overlay = −1 (0xFFFFFFFF, a full i32), Rust writes `overlay_byte = 0xFF` (a single byte
  sentinel) — these only coincide if Rust's overlay sentinel-byte maps to the same -1 the
  renderer reads; the rest of the SetBridgeDirection flag group-clear is absent.
- Fixture: clear at HIGH cell (8,10), `param_3=2` ⇒ `SetBridgeDirection_NESW(0,0)`. Binary edits
  (8,10) + the 3 cells stepped via direction 0 (N: (8,9),(8,8),(8,7)) + the (0−4)&7=4 opposite
  cell (S: (8,11)) — five cells get flag-mask clears + `+0x11E=0` + MarkTerrainDirty. Rust edits
  only (8,10) (overlay byte + deck_present), leaving the structural-flag bits on the four
  neighbor cells untouched, then keeps walking and blanking more cells. The neighbor cells'
  bridge-structural flags are left set in Rust → downstream `IsBridge`/passability reads diverge.
- Player sees: residual bridge-structural flags on cells adjacent to the cleared stub → those
  cells may still be treated as walkable bridge by pathfinding when the binary has cleared them
  (or vice-versa). Every collapse that reaches the clear branch.
- Severity: MED (pathing/placement divergence on cells near the break; not always reached because
  Rust's whole branch is wrong).
- Confidence: PROVEN-DRIFT
- Verify-call: `decompile_function 0x0047e040`, `decompile_function 0x00576200`.

---

### D4: No `RepairBridgeSegment` cap re-stamp during rim refresh (and it fires TriggerEvent 31)

- Rust now: `update_adjacent_bridges` never calls anything resembling `RepairBridgeSegment`. The
  only `RepairBridgeSegment` analog in the orchestrator is `notify_bridge_span_collapse`
  (line 1120) which is an intentional **no-op** (`let _ = (sim, cells);`).
- gamemd: inside `UpdateBridgeEdgeTiles_High`'s back-walk, `RepairBridgeSegment(uVar1, local_40)`
  (`0x00575ee0`) is called once (latched by `bVar2`) when a `flags&0x80` was-clear→now-set
  transition is detected — i.e. on a newly-valid ramp segment. `RepairBridgeSegment` walks the
  span between two endpoints and, for every cell whose `+0x3c != 0` (occupant present), fires
  `TechnoClass__ProcessCellAction(0x1f, ...)` — that 0x1f = TriggerEvent 31. So in gamemd the
  rim-refresh path is one of the call sites that emits event 31; Rust both (a) never re-stamps
  the cap and (b) routes event 31 only through the no-op stub.
- Fixture: a span collapse that leaves a ramp re-validated (flags&0x80 clear→set during the
  back-walk). gamemd calls `RepairBridgeSegment` which iterates the span and, on any cell holding
  a unit, fires event 31; Rust does neither. On a skirmish map event 31 is unbound so the trigger
  itself is invisible — but the cap-tile re-stamp (the rest of `RepairBridgeSegment`'s walk over
  the endpoint columns) is missing, so the re-validated ramp keeps damaged art.
- Player sees: a ramp that should "heal" its end-cap after an adjacent collapse re-validates the
  span keeps its damaged/destroyed art instead of the pristine cap. Rare in skirmish (needs a
  collapse that re-validates a neighbor ramp), but visible when it occurs. Event-31 firing itself
  is TS/campaign-only — not a skirmish disparity (see UNCHECKED/TS-legacy note).
- Severity: LOW (narrow trigger: only when a collapse re-validates an adjacent ramp).
- Confidence: PROVEN-DRIFT (re-stamp absence) / TS-legacy (event-31 broadcast).
- Verify-call: `decompile_function 0x00575ee0`, `decompile_function 0x00576200`.

---

### D5: Rust skips Destroyed cells during the walk; binary's walk does not "skip and continue"

- Rust now: bridge_orchestrator.rs:1091–1093 — when a walked cell `damage_state == Destroyed`,
  Rust `continue`s (keeps walking past it). It also `break`s when `!cell.deck_present`
  (line 1086–1088).
- gamemd: neither `UpdateAdjacentBridges_*` nor `UpdateBridgeEdgeTiles_*` has a "skip destroyed,
  keep going" rule. `UpdateAdjacentBridges_High`'s Phase C walks via the `g_DirectionOffsets`
  direction and at each cell either matches a ramp pattern (→ call edge-tiles, return) or
  `goto LAB_00576a74` (advance one cell) with bounds checks against the map rect
  (`DAT_0087f8dc`/`DAT_0087f8e0`) — there is no `deck_present` break and no Destroyed-skip; it
  terminates by leaving the map bounds or matching a pattern. `UpdateBridgeEdgeTiles_*`'s forward
  walk terminates at the `0x1e`=30 cap or on a ramp-class tile match; its back-walk terminates by
  the `flags&0x80` transition. The Rust loop's `deck_present`/Destroyed control flow has no binary
  counterpart.
- Fixture: HIGH collapse, walking W from rim (11,10) over Destroyed (10,10): Rust `continue`s past
  (10,10) and continues to (9,10),(8,10)…; the binary's Phase-C walk from the head coord advances
  via direction offset and stops at the first ramp-pattern match or map edge — it does not pass
  over a destroyed cell looking for stubs to blank. Different set of cells visited and mutated.
- Player sees: subsumed by D1 — different cells get touched, producing different surviving art.
  Every span collapse.
- Severity: HIGH (part of the D1 wrong-algorithm; same trigger frequency).
- Confidence: PROVEN-DRIFT
- Verify-call: `decompile_function 0x00576770`.

---

### D6: Walk-length cap of 30 is on the WRONG walk

- Rust now: `WALK_LIMIT = 30` (bridge_orchestrator.rs:1040) bounds the stub-blanking walk in
  `update_adjacent_bridges`.
- gamemd: the `0x1e`=30 cap is the loop bound (`local_44 < 0x1e`) of the FORWARD ramp-search walk
  inside `UpdateBridgeEdgeTiles_High`/`_Low` (the `do { … } while (local_44 < 0x1e)` at the top of
  each, verified at `0x00576200`/`0x00570ae0`). `UpdateAdjacentBridges_High`'s own Phase-C walk is
  NOT capped at 30 — it's bounded by the map-rect tests (`DAT_0087f8dc`, `DAT_0087f8e0*2`) and by
  finding a ramp pattern. So the 30 cap belongs to a function (edge-tiles) that Rust doesn't have,
  applied to a different walk (ramp search, not stub blanking). The constant is numerically right
  but attached to the wrong loop.
- Fixture: a >30-cell linear bridge. Binary: `UpdateAdjacentBridges_High` Phase-C can walk the
  whole length (no 30 cap) until it finds a ramp or leaves the map; `UpdateBridgeEdgeTiles_High`'s
  inner ramp search caps at 30 and returns 0 (no rebuild) if no ramp within 30. Rust caps its
  stub-blank walk at 30 cells regardless. On a 35-cell bridge the cell-coverage differs.
- Player sees: subsumed by D1; on very long bridges the touched region differs. Rare (bridges
  >30 cells are uncommon).
- Severity: LOW.
- Confidence: PROVEN-DRIFT
- Verify-call: `decompile_function 0x00576200`, `decompile_function 0x00576770`.

---

### D7: Rust runs rim refresh on direct-overlay + hut paths; binary runs it from state-machine + DestroyBridge_*_MapInit

- Rust now: `update_adjacent_bridges` is called from `apply_bridge_damage_events`
  (bridge_orchestrator.rs:123) and `apply_hut_bridge_execution` (line 333) using `rim_cells`
  built from `StateOutcome::Collapsed.adjacent_bridges_dirty`. Those rim cells are the two
  perpendicular neighbors of the collapsed anchor (`compute_adjacent_bridges_dirty`, mod.rs:2028).
- gamemd callers (verified, BRIDGE_DISPLAY_TABLE §7.3 + HIGH §5/§11.x): `UpdateAdjacentBridges_High`
  ← `ProcessBridgeDamageStateMachine_High @ 0x576BA0` (the body-cell collapse path,
  "`UpdateAdjacentBridges_High × 2`" on two perpendicular neighbors — HIGH §5 line 196), plus
  `DestroyBridge_High_MapInit @ 0x574000` and `DestroyBridge_Low_MapInit @ 0x574C20` (hut-death).
  `UpdateAdjacentBridges_Low @ 0x571050` ← `ProcessBridgeDamageStateMachine_Low @ 0x571490` only.
  The "× 2 perpendicular neighbors" matches Rust's rim_cells source, so the SEED cells are right.
- Note — vanilla bug to preserve (BRIDGE_REPAIR §13.4): both `DestroyBridge_Low_MapInit` AND
  `DestroyBridge_High_MapInit` call `UpdateAdjacentBridges_High` (there is no `_Low` MapInit
  caller). So a LOW bridge collapsed via CABHUT death runs the HIGH rim-refresh in vanilla. Rust's
  `update_adjacent_bridges` is family-agnostic (no high/low split at all), which masks rather than
  reproduces this — but since the whole function is the wrong algorithm (D1), the family question
  is moot until D1/D2 are fixed. Flagged so the eventual fix reproduces the High-on-Low quirk for
  hut-death LOW collapses.
- Fixture: CABHUT serving a LOW bridge is destroyed → vanilla calls `UpdateAdjacentBridges_High`
  (the High edge-tile classifier, High DAT_ constants) on the LOW span. Rust calls its single
  family-agnostic blanker. Observably the LOW span's edge tiles are re-classified by the High
  table in vanilla (often a visible mis-cap on wood bridges); Rust produces neither.
- Player sees: only matters once D1/D2 are implemented; flagged as a fix-time constraint.
- Severity: LOW (latent; depends on D1/D2 fix).
- Confidence: PROVEN-DRIFT (caller set + High-on-Low quirk verified).
- Verify-call: `decompile_function 0x00574c20` (Low MapInit) + BRIDGE_REPAIR §13.4 cross-ref.

---

## PARITY-CONFIRMED

- **Rim-cell seed = 2 perpendicular neighbors of the collapsed anchor.** `compute_adjacent_bridges_dirty`
  (mod.rs:2028) emits exactly the {E,W} (NS axis) / {S,N} (EW axis) perpendiculars, matching the
  binary's "`UpdateAdjacentBridges_High × 2`" two-perpendicular-neighbor seed (HIGH §5 line 196).
  The SEED is correct even though the body is wrong.
- **8-direction Phase-A walk uses the same 8 offsets in the same order.** Rust `DIRECTIONS`
  (bridge_orchestrator.rs:1041) = `(0,-1),(1,-1),(1,0),(1,1),(0,1),(-1,1),(-1,0),(-1,-1)`, the
  standard `g_DirectionOffsets` N,NE,E,SE,S,SW,W,NW order the binary indexes via `uVar9 & 7`.
  Phase-A direction enumeration order matches; only the stop condition differs (Rust uses
  role==Bridgehead||Destroyed, binary uses `flags & 0x500` — bit 8 OR bit 10, the head/destroyed
  union — which is the same intent, NOT separately bit-verified equal across all flag states, so
  noted but not raised as its own DRIFT).
- **Walk-cap numeric value 30 (0x1e) is correct as a number** — verified `local_44 < 0x1e` in both
  `UpdateBridgeEdgeTiles_High/_Low`. (It's attached to the wrong loop — see D6.)
- **`SetBridgeDirection_NESW` (NESW) vs `_NWSE` (NWSE) split is High-vs-Low correct** — High edge
  tiles call `_NESW @ 0x47e040`, Low call `_NWSE @ 0x47e470` (verified `0x00570ae0` calls
  `CellClass__SetBridgeDirection_NWSE`). Rust has neither, so nothing to mis-match yet.

## UNCHECKED

- **TS-legacy / event-31:** `RepairBridgeSegment` fires `ProcessCellAction(0x1f=31)` on occupied
  span cells. On RA2/YR skirmish, event 31 has no bound trigger, so the broadcast is invisible
  (matches CLAUDE.md TS-legacy filter and the orchestrator's deliberate `notify_bridge_span_collapse`
  no-op). NOT raised as a skirmish DRIFT; only the cap-tile re-stamp part of `RepairBridgeSegment`
  is (D4). Whether the cap-restamp half is reachable in a normal skirmish collapse is bounded by
  the `flags&0x80` clear→set transition — could not construct a guaranteed skirmish fixture that
  hits it without a live trace.
- **`UpdateBridgeEdgeTiles_*` ramp-class globals** (`DAT_00abc1e8`, `DAT_00aa0e38`, `DAT_00abad30`,
  `DAT_00abc1d0`, `DAT_00aa1540`, `DAT_00aa1028`) are theater-loaded (zero in static binary, per
  BRIDGE_DISPLAY_TABLE §8). The exact tile-index match values can't be pinned statically — the
  pattern STRUCTURE (which DAT_ + which `cell+0x11A` literal: 4 for mode 2, 2 for mode 4) IS
  verified from the decompile, but the runtime values need a live capture to fixture-walk the
  exact cell matched. This blocks a bit-identical "which cell gets cleared" check, but does not
  change the D1/D2 verdict that the algorithm is wholly different.
- **Whether Rust's `overlay_byte = 0xFF` (u8 sentinel) renders identically to the binary's
  `+0x44 = 0xFFFFFFFF` (i32 −1).** The renderer-side equivalence of the two "no overlay" encodings
  was not traced this session; flagged in D3.
