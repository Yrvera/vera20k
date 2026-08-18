# Bridgehead Slot +3 Collapse — Parity Scan Findings

Facet: bridgehead-slot3 — Bridgehead direct-damage tile-class slot +3 collapse branch of
`ProcessBridgeDamageStateMachine_High@0x00576ba0` / `_Low@0x00571490`.

Rust under test: `src/sim/bridge_state/mod.rs::bridgehead_advance_state` (line 1405),
`src/sim/bridge_specs.rs::{bridgehead_walk_to_anchor@702, bridgehead_blow_up_row@788,
update_ramp_perpendicular@537, apply_anchor_class_transition@654}`,
`src/sim/world/bridge_orchestrator.rs` routing @1436-1455.

Authority: live decompiles of `0x00576ba0`, `0x00571490`, `0x00572230` (UpdateRamp_NS_DamageA_High),
`0x00572330` (UpdateRamp_NS_DamageB_High). All anchor addresses re-confirmed via
`get_function_by_address` this session.

NOTE: The current Rust is much further along than the cited research doc
`BRIDGEHEAD_DIRECT_DAMAGE_SLOT3_COLLAPSE_GHIDRA_REPORT.md` describes. That doc's headline
DRIFT ("Rust absorbs every repeat hit forever, never collapses") is STALE — Rust now has a
collapse path. The findings below are drifts in the CURRENT collapse implementation versus
the live binary.

---

### D1: Collapse trigger is a fabricated two-hit progression; binary collapses only on a cell that is ALREADY slot +3

- Rust now: `bridgehead_advance_state` collapses when `input_is_final || anchor_is_final`,
  where `is_final` ≡ `bridgehead_anchor_class == AboutToFall` (mod.rs:1449-1458). On a fresh hit
  of a healthy bridgehead (slots +0/+1/+2), the **absorb** path explicitly writes the *anchor*
  cell to `AboutToFall` (= slot +3) at mod.rs:1543-1545, then returns `Absorbed`. A **second**
  hit on the same bridgehead then sees `anchor_is_final == true` and runs the collapse. Net:
  a healthy bridgehead collapses in exactly TWO direct state-machine hits.
- gamemd: The collapse-vs-absorb decision is a **single-hit** read of the hit cell's OWN tile
  class. In `ProcessBridgeDamageStateMachine_High@0x00576ba0`, `iVar2 = (puVar9[0x38] - DAT_00aa0e28) + 1`
  is computed from the **input** cell (`param_1`) before the height-walk and is never re-read.
  NS: `if (iVar2 != DAT_00abad30 + 3)` → slots +0/+1/+2 take the DamageA/DamageB absorb and
  `return 0`; only `iVar2 == DAT_00abad30 + 3` runs the collapse and `return 1`. Crucially, the
  absorb path writes the *anchor* to **slot +2** (`SetOverlayAndPropagate(..., DAT_00abad30 + 2 + base ...)`),
  NOT slot +3, and `UpdateRamp_NS_DamageB_High@0x00572330` advances perpendicular neighbors
  only `slot+0→+1` and `slot+1→+2` (verified: the function's two `SetOverlayAndPropagate` calls
  target `DAT_00abad30 + 1` and `DAT_00abad30 + 2`). `UpdateRamp_NS_DamageA_High@0x00572230`
  writes `slot+0→+0` and `slot+2→+2` (preserve). **No path in the bridgehead state machine ever
  writes a cell to slot +3.** Slot +3 bridgehead tiles only come from map placement (a tile
  authored in the maximally-damaged class).
- Fixture: Healthy NS bridgehead column X=2 (heights (2,4)=8 bridgehead, (2,3)=6, (2,2)=4 anchor),
  all `bridgehead_anchor_class = Variant0` (slot +0). Direct state-machine hit on (2,4):
  - Binary: input cell (2,4) tile class = slot +0 → `iVar2 != slot+3` → walk to anchor (2,2),
    write anchor tile class to **slot +2**, DamageA on E neighbor (3,2), DamageB on W neighbor
    (1,2) advancing it +0→+1, `return 0`. NO collapse, NO BlowUpBridge. A second identical hit:
    input (2,4) still slot +0 (its own class never changed) → again absorb to slot +2, again
    `return 0`. The bridgehead NEVER collapses from this path.
  - Rust: first hit writes anchor (2,2) `bridgehead_anchor_class = AboutToFall` (slot +3),
    returns `Absorbed`. Second hit sees `anchor_is_final` → `Collapsed{...}`, BlowUpBridge on
    (2,1),(2,2),(2,3), `destroyed_cells = [(2,1),(2,2),(2,3)]`, zones rebuilt. Bridge endpoint
    severed. (This is exactly what test `bridgehead_advance_repeat_high_hit_collapses_about_to_fall_slot`
    @tests.rs:1282 asserts.)
- Player sees: A healthy high/low bridgehead can be destroyed by two ordinary shots in Rust;
  in gamemd those same shots only chip the bridgehead's ramp/anchor to its Damaged class and it
  stays standing (the only way to drop a bridgehead via this path is if the map authored a
  slot +3 bridgehead tile). Triggers every time a player concentrates direct fire on a bridge
  ramp/endpoint cell — a common tactic to deny a crossing. Wrong in BOTH directions: Rust
  destroys bridges that should survive, and Rust would still mishandle a genuine pre-authored
  slot +3 tile (D2).
- Severity: HIGH (player can collapse a bridge endpoint that gamemd leaves standing; fires
  whenever direct fire hits a bridgehead/ramp cell, which is routine).
- Confidence: PROVEN-DRIFT
- Verify-call: `decompile_function 0x00576ba0` (NS `iVar2 == DAT_00abad30 + 3` gate + slot+2
  absorb write); `decompile_function 0x00572330` (DamageB caps neighbor at slot+2);
  `decompile_function 0x00572230` (DamageA preserves slot+0/+2).

---

### D2: A map-authored slot +3 bridgehead never collapses on first hit in Rust (mirror of D1)

- Rust now: A bridgehead cell loaded with `bridgehead_anchor_class = AboutToFall` would collapse
  on first hit (because `input_is_final` is checked, mod.rs:1449-1452). BUT the normal map-load
  default is `Variant0` (mod.rs:611, 750), and there is no parser path observed that sets a
  bridgehead cell's `bridgehead_anchor_class` to `AboutToFall` from map tile data — the only
  writer of `AboutToFall` outside collapse is the absorb path (D1). So in practice a genuinely
  pre-damaged (slot +3) bridgehead tile authored in the map is loaded as `Variant0` and follows
  the two-hit path of D1 instead of collapsing on the first hit.
- gamemd: A bridgehead cell whose authored tile class is slot +3 (`Cell+0x38` resolves to
  `DAT_00abad30 + 3` NS or `DAT_00aa1028 + 3` EW) collapses on the **very first** direct
  state-machine hit — `return 1` (High) / collapse side effects then `return 0` (Low).
- Fixture: Map authors a bridge ramp endpoint already at the most-damaged SHP variant (slot +3
  tile class). gamemd: first shell → BlowUpBridge ×3 + collapse. Rust: the tile is loaded as
  Variant0, so first shell absorbs (writes anchor to AboutToFall), second shell collapses — and
  the Rust three-cell footprint/anchor selection is derived from the height-walk, not from the
  authored tile, so even the resulting geometry can differ.
- Player sees: Pre-damaged bridge endpoints (used by mapmakers for "already crumbling" bridges)
  take one extra hit to drop in Rust. Frequency depends on map authoring — uncommon in stock
  YR multiplayer maps, more likely in campaign/custom maps. (The doc deferred a stock-map
  prevalence scan; not resolvable in Ghidra.)
- Severity: MED (correctness gap on a real binary path; trigger frequency depends on map data).
- Confidence: PROVEN-DRIFT (binary single-hit slot+3 collapse verified) for the binary side;
  LIKELY-DRIFT on the Rust parser-default claim — see UNCHECKED about whether any loader maps
  slot+3 tiles to `AboutToFall`.
- Verify-call: `decompile_function 0x00576ba0` (NS: `iVar2 == DAT_00abad30 + 3` directly enters
  collapse with no prior-state requirement).

---

### D3: Absorb path writes anchor to slot +3 (AboutToFall) instead of slot +2 (Damaged)

- Rust now: mod.rs:1543-1545 writes `anchor_cell.bridgehead_anchor_class = AboutToFall` (slot +3)
  on the absorb path. The enum order is `Variant0=0, Variant1=1, Damaged=2, AboutToFall=3`
  (mod.rs:172-177), so `AboutToFall` is slot +3. The doc-comment at mod.rs:161-163 even states
  the absorb path "writes the anchor straight to this slot (skipping Variant1/Damaged)" — an
  acknowledged divergence.
- gamemd: The absorb path (`iVar2 == DAT_00abad30 + 0/+1/+2`, NS High) calls
  `SetOverlayAndPropagate(anchor, DAT_00abad30 + 2 + DAT_00aa0e28, ...)` — it writes the anchor's
  tile class to **slot +2** (Damaged), the second-most-damaged class. It never writes slot +3.
- Fixture: First hit on healthy NS bridgehead → anchor (2,2). Binary: anchor tile class becomes
  slot +2. Rust: anchor `bridgehead_anchor_class` becomes `AboutToFall` (slot +3). Observable as
  a different bridge-deck/anchor SHP frame after one hit (the anchor renders one damage stage
  more advanced than gamemd shows), AND it is the mechanism that causes the spurious second-hit
  collapse in D1.
- Player sees: After a single hit, the bridge anchor tile in Rust shows the most-damaged
  ("about to fall") frame; gamemd shows the Damaged frame, one stage less. Visible every time a
  bridgehead is hit once. This is the root cause of D1.
- Severity: HIGH (root cause of D1; visible 1-frame/1-stage SHP difference on every bridgehead
  hit, plus drives the wrong collapse).
- Confidence: PROVEN-DRIFT
- Verify-call: `decompile_function 0x00576ba0` (absorb branch
  `SetOverlayAndPropagate(&param_1, DAT_00abad30 + 2 + DAT_00aa0e28, 0xffffffff, 0xffffffff, 0)`).

---

### D4: Collapse SetOverlayAndPropagate(slot+3) and its `level = Cell+0x11B - 4` not modeled

- Rust now: The collapse path (mod.rs:1461-1535) sets the anchor's `bridgehead_anchor_class =
  AboutToFall` and, for each BlowUpBridge-row cell, `damage_state = Destroyed` and class =
  AboutToFall. It does NOT write any overlay/tile-class via a `SetOverlayAndPropagate` equivalent
  to the anchor, and there is no Z/level argument carried anywhere.
- gamemd: After the three `CellClass__BlowUpBridge` calls, the collapse calls
  `MapClass__SetOverlayAndPropagate(anchor, DAT_00abad30 + 3 + DAT_00aa0e28 /* slot+3 tile */,
  Z = 0xffffffff (-1), level = cVar1 - 4 where cVar1 = puVar9[0x11b], flag = 0)`. The `level`
  is the anchor's (or odd-height shifted neighbor's) height byte at `Cell+0x11B` minus 4 — i.e.
  the collapse drops the tile by its bridge-deck height so it renders at ground level. The Rust
  collapse never re-skins the anchor tile to the slot+3/collapsed SHP nor adjusts a Z/level.
- Fixture: High NS slot+3 collapse at anchor (2,2), `Cell+0x11B` (deck height) = e.g. 0x18.
  Binary writes the anchor tile to slot+3 overlay at level `0x18 - 4 = 0x14`. Rust leaves
  `overlay_byte` untouched and has no level write — the destroyed anchor tile may render with
  the wrong frame/elevation versus gamemd's dropped collapsed tile.
- Player sees: The collapsed bridgehead tile's appearance/elevation on collapse can differ
  (wrong damage SHP frame or wrong drop height). Fires on every bridgehead collapse. Whether it
  is visible depends on how the Rust renderer derives the bridgehead tile frame from
  `bridgehead_anchor_class`/`damage_state` vs the binary's overlay+level write — needs a render
  cross-check.
- Severity: MED (collapse is less frequent than absorb; visible-impact magnitude depends on the
  Rust renderer's frame derivation — flagged, not triaged out).
- Confidence: PROVEN-DRIFT (binary side: the SetOverlayAndPropagate slot+3 + `cVar1-4` level is
  in the decompile); LIKELY-DRIFT on player-visibility pending a render check.
- Verify-call: `decompile_function 0x00576ba0`
  (`MapClass__SetOverlayAndPropagate(&param_1, iVar2, 0xffffffff, cVar1 + -4, 0)` with
  `iVar2 = DAT_00abad30 + 3 + DAT_00aa0e28`, `cVar1 = puVar9[0x11b]`).

---

### D5: Two BlowUpBridge cells written by the Rust collapse can be a Bridgehead/Anchor and get class overwritten — binary just calls BlowUpBridge

- Rust now: In the collapse loop (mod.rs:1486-1491) for each blow-up-row cell it sets
  `damage_state = Destroyed` and, if the cell role is `Anchor | Bridgehead`, also overwrites
  `bridgehead_anchor_class = AboutToFall`. It then re-scans perpendicular neighbors (E/W/N/S of
  anchor) and appends any already-`Destroyed` cell to `destroyed_cells` (mod.rs:1510-1525).
- gamemd: The collapse calls `CellClass__BlowUpBridge` on exactly the three row/column cells
  (whose internals are out of this facet's scope), then writes the anchor's overlay once via
  SetOverlayAndPropagate (D4). It does NOT additively scan four perpendicular neighbors to build
  `destroyed_cells`; the cascade/recalc list is the fixed 2×5 ten-cell rectangle built by the
  `FUN_0042fcb0 ... RecalcCellsAndRebuildZones` loop. The `destroyed_cells` perpendicular re-scan
  in Rust is a model-specific aggregation with no direct binary analog; it can include or omit
  cells differently than the binary's BlowUpBridge set.
- Fixture: NS even collapse, anchor (2,2). Binary BlowUpBridge cells: (2,1),(2,2),(2,3) — column
  at anchor.X, Y∈{Y-1,Y,Y+1}. Rust BlowUpBridge cells match (verified vs `bridgehead_blow_up_row`
  NS-even). But Rust's `destroyed_cells` may additionally pick up E/W neighbors (3,2)/(1,2) if a
  prior absorb's DamageB had pushed them to Destroyed — the binary does not derive collapse
  destroyed-set from those neighbors here.
- Player sees: Possible extra/missing entries in the destroyed-cell set that drives downstream
  ground-occupant kills / debris / zone severing on collapse. Edge-dependent; fires only when a
  perpendicular neighbor was already Destroyed at collapse time.
- Severity: LOW (the three primary BlowUpBridge cells match; divergence only in the auxiliary
  perpendicular re-scan and only in pre-damaged-neighbor states).
- Confidence: LIKELY-DRIFT (the binary builds the recalc rectangle, not a neighbor-destroyed
  scan; exact downstream consumer equivalence not exhaustively traced).
- Verify-call: `decompile_function 0x00576ba0` (collapse builds `local_18 = &PTR_FUN_007e3890`
  2×5 recalc list via the `iVar8 = -2 .. < 3`, `iVar10 .. < 2` nested loop; three explicit
  `CellClass__BlowUpBridge` calls only).

---

## PARITY-CONFIRMED

These sub-aspects were checked against the live binary and match the current Rust:

- **BlowUpBridge three-cell footprint geometry (NS even / NS odd / EW <5 / EW >=5).**
  Binary High NS even: column at anchor.X, Y∈{Y-1,Y,Y+1}; NS odd: column at anchor.X-1;
  EW <5: row at anchor.Y, X∈{X-1,X,X+1}; EW >=5: row at anchor.Y-1. `bridgehead_blow_up_row`
  (bridge_specs.rs:788-817) reproduces all four exactly. Verified `decompile_function 0x00576ba0`.
- **NS even-vs-odd / EW <5-vs->=5 anchor-shift predicate is dead post-walk on both sides.**
  The NS walk converges to height byte 4 (even) and EW to height byte 2 (<5); the binary's
  odd/>=5 shift branches and Rust's `bridgehead_blow_up_row` odd/>=5 branches are therefore both
  unreachable at the walked anchor. No divergence in the reachable case. Verified
  `decompile_function 0x00576ba0` (NS `if ((puVar9[0x11a] & 1) == 0)` post-walk;
  EW `if ((byte)puVar9[0x11a] < 5)`).
- **Start-cell gate: NS rejects odd height, EW rejects height > 4.** Binary High NS:
  `if ((puVar9[0x11a] & 1) != 0) return 0`; High EW: `if (4 < uVar6) return 0`.
  `bridgehead_walk_to_anchor` (bridge_specs.rs:716-728) matches (NS `start_h & 1 != 0 → None`;
  EW `start_h > 4 → None`). Verified `decompile_function 0x00576ba0`.
- **Walk targets: NS height byte 4, EW height byte 2.** Binary NS loop `do {...} while (iVar8 != 0)`
  with `iVar8 = uVar6 - 4`; EW `while (iVar2 != 0)` with `iVar2 = uVar6 - 2`.
  `bridgehead_walk_to_anchor` uses `target_height = 4 (NS) / 2 (EW)` (bridge_specs.rs:709-712).
- **Low slot +3 returns false; High slot +3 returns true.** Binary High collapse paths
  `return 1`; Low collapse paths fall through to `return 0`. Rust `Collapsed{binary_success:
  is_high_bridge}` (mod.rs:1529) and `apply_damage_success` keying on `binary_success: true`
  (mod.rs:417-425) reproduce the asymmetry. Tests @tests.rs:1297, 1346 assert it. Verified
  `decompile_function 0x00571490` (Low NS/EW collapse blocks fall to `switchD_00572019_default:
  return 0`).
- **IonCannon state-machine retry = 4 attempts; non-state-machine/direct = 1.** Orchestrator
  (bridge_orchestrator.rs:1429-1434) gives state-machine paths 4 attempts on IonCannon, retrying
  while success==false. Matches the follow-up doc's verified `Apply_area_damage` "first attempt +
  up to three retries" and the absorb path returning false (so absorb re-runs on IonCannon).
- **Slots +0/+1/+2 do NOT collapse on first hit.** Binary High: `iVar2 != slot+3` → DamageA/B +
  `return 0`. Rust absorb path returns `Absorbed` (no Collapsed) on a non-`AboutToFall` cell.
  (The DRIFT in D1/D3 is in WHAT the absorb writes and the spurious second-hit collapse, not in
  the first-hit-doesn't-collapse property.)
- **Raw overlay direct-destroy bands route before the bridgehead state machine.** Binary
  `ApplyDamageToCell@0x00587180` checks overlay `Cell+0x44` bands first; orchestrator dispatch
  order is HighStateMachine, LowStateMachine, LowDirect, HighDirect with `path_matches_cell`
  gating — overlay-direct cells route to the Direct walkers. (Dispatch-order detail; covered by a
  sibling facet but spot-confirmed consistent.)

---

## UNCHECKED

- **Whether any map-load / parser path sets a bridgehead cell's `bridgehead_anchor_class` to a
  non-`Variant0` value from authored slot +3 tile data.** D2 assumes the loader always defaults
  bridgeheads to `Variant0` (observed defaults at mod.rs:611, 750), which would make a
  map-authored slot +3 bridgehead behave as a healthy one. I did not exhaustively trace the
  map/overlay loader to confirm no path maps an authored slot+3 tile class to `AboutToFall`.
  If such a path exists, D2's first-hit-collapse would actually work and only D1/D3 (the
  fabricated two-hit progression on Variant0 cells) would remain.
- **Player-visibility magnitude of D4 (overlay slot+3 + `level = Cell+0x11B - 4`).** Whether the
  missing overlay/level write produces a visibly wrong collapsed-tile frame/elevation depends on
  how the Rust bridge renderer derives the bridgehead tile frame from
  `bridgehead_anchor_class`/`damage_state`. The render layer is outside this facet's read scope
  (sim must not depend on render); flagged as DRIFT on the sim-state side, visibility pending a
  render cross-check.
- **Exact equivalence of D5's `destroyed_cells` perpendicular re-scan vs the binary's downstream
  ground-occupant-kill / debris consumers.** I confirmed the three primary BlowUpBridge cells
  match and that the binary uses a fixed 2×5 recalc rectangle rather than a neighbor-destroyed
  scan, but did not trace every Rust consumer of `destroyed_cells` to prove the auxiliary entries
  never change an observable outcome.
- **Stock-map prevalence of pre-authored slot +3 bridgehead tiles** (D2 trigger frequency).
  Deferred by the source doc; needs a map-corpus scan, not Ghidra.
