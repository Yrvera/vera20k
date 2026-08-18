# Overlay-Direct Destroy Walkers — Adversarial Verdicts

**Facet:** overlay-direct-walkers (NS/EW × High/Low direct destroy walkers + sibling cascade).
**Auditor stance:** adversarial skeptic; default DRIFT, downgrade only on proof.
**Rust audited:** `src/sim/bridge_state/walker.rs` (walker bodies, cascade leaves, start-shift, classifiers),
`src/sim/bridge_state/mod.rs` (`path_matches_cell`, `StateOutcome`, role assignment),
`src/sim/world/bridge_orchestrator.rs` (dispatch + outcome consumption).

**Live decompiles this session (all re-confirmed to resolve to the named function):**
- `ApplyDamageToCell @ 0x00587180` — dispatch gate.
- `DestroyBridge_High @ 0x0057ccf0` — forward-start `CONCAT22(psVar1[1]+1,*psVar1)`.
- `DestroyBridgeWalker_NS_High @ 0x0057cf60`, `EW_High @ 0x0057d530`, `NS_Low @ 0x0057bcf0`, `EW_Low @ 0x0057c2b0`.
- `ApplyBridgeDestruction_NS_High @ 0x0057e7a0` (cascade leaf).

---

## D1: Walker skips Bridgehead-role cells — VERDICT=REAL

**REAL.** The finder's gamemd reading holds verbatim and is reachable.

- `DestroyBridgeWalker_NS_High @ 0x0057cf60`: in EVERY branch the three triple cells are written
  unconditionally — `local_a0->OverlayTypeIndex = X; local_a4->OverlayTypeIndex = X;
  this->OverlayTypeIndex = X;` — with **no role/bridgehead/anchor/flag guard of any kind**. Verified
  identical in `EW_High @ 0x0057d530`, `NS_Low @ 0x0057bcf0`, `EW_Low @ 0x0057c2b0`, and in the cascade
  leaf `ApplyBridgeDestruction_NS_High @ 0x0057e7a0` (`local_c4/b8/cc->OverlayTypeIndex = iVar2`, no
  role check). gamemd has **no concept analogous to the Rust `BridgeCellRole`**; it dispatches and writes
  purely on `OverlayTypeIndex` band (`0xCD..=0xE8` high / `0x4A..=0x65` low).
- Rust adds `if matches!(c.role, BridgeCellRole::Bridgehead) { continue; }` before the overlay/damage_state
  write in all 4 walker bodies (`walker.rs:879, 972, 1236, 1325`) AND all 4 cascade leaves
  (`:753, :806, :1118, :1170`). Any triple/cascade cell tagged `Bridgehead` is skipped.
- **Reachability proven two ways:**
  1. *Neighbor skip:* the walker enters on a body cell with in-band overlay; its length-axis neighbor
     toward shore is the Rust pass-3 bridgehead (ramp) cell (`mod.rs:706-710`, axis=Some). gamemd writes
     the destroy overlay into that neighbor; Rust skips it. Span-end is hit by every full collapse.
  2. *Center skip:* `path_matches_cell` HighDirect/LowDirect routes on overlay band ONLY, with **no role
     gate** (`mod.rs:848-849`). A pass-3 Bridgehead cell whose overlay is in the destroy band enters
     `destroy_bridge_high/low` and the triple loop then `continue`s on its OWN center cell — gamemd would
     write it.
- **Corrected delta:** Rust `if role==Bridgehead { continue }` (no overlay byte / damage_state write, cell
  omitted from `destroyed`) → gamemd writes the per-case overlay (`0xD3/0xE7/...`) to ALL three triple
  cells and ALL cascade-triple cells unconditionally, marks them dirty/recalc, and BlowUpBridge fires on
  the bridgehead cell when the write is the final byte. Player-visible: a bridgehead/ramp tile left
  standing or half-damaged after a collapse gamemd renders fully destroyed; no BlowUpBridge on it.
- Verify: `decompile_function 0x0057cf60` (and 0x0057d530 / 0x0057bcf0 / 0x0057c2b0 / 0x0057e7a0).

## D2: Forward-start `saturating_add(1)` vs raw 16-bit `+1` — VERDICT=REAL (boundary-only)

**REAL but unreachable on real maps** (matches finder severity).

- `DestroyBridge_High @ 0x0057ccf0`: forward start is `param_1 = (short*)CONCAT22(psVar1[1] + 1, *psVar1)`
  — a raw signed-16-bit add, wraps at 0xFFFF, no saturation. Same shape in the Low twin.
- Rust `find_walker_start_*` use `ry.saturating_add(1)` / `rx.saturating_add(1)` (`walker.rs:518, 540,
  561, 582`). Diverges only when `ry`/`rx == 0xFFFF`: Rust → 0xFFFF, gamemd → 0x0000 (then off-map sentinel
  routing). Unreachable: WAE/retail map dimensions are far below 0xFFFF, so a bridge body cell at row/col
  65535 cannot exist. No observable divergence in play; correctly surfaced under "no disparity too small."
- Corrected delta: Rust `saturating_add(1)` → gamemd `(short)(coord + 1)` wrapping add. Boundary only.
- Verify: `decompile_function 0x0057ccf0`.

## D3: Cascade-leaf final-cell de-dup (`destroyed.contains`) — VERDICT=REFUTED (output-identical)

**REFUTED — provably output-identical for player-visible state.**

- gamemd (`0x0057e7a0`) re-issues the overlay write + MarkTerrainDirty/RecalcAttributes per cascade call
  with no cross-call de-dup, as the finder says. BUT the Rust de-dup at `walker.rs:902/993/1257/1346` only
  guards pushes into the `destroyed`/`actions` Vec; the actual overlay/damage_state WRITE inside
  `apply_bridge_destruction_*` is unconditional (`:756, :809, :1121, :1173`) — so the final overlay byte
  and damage_state end-state are bit-identical to gamemd regardless of de-dup.
- The de-dup only collapses duplicate `BlowUpBridge` actions. BlowUpBridge is a one-shot cell kill/limbo,
  and the orchestrator independently de-dups again via `destroyed_set: BTreeSet` + `destroyed_set.insert`
  (`bridge_orchestrator.rs:78, 87, 91`) before firing `blow_up_bridge_cell_fallout` once per cell. A cell
  cannot be blown up twice. No observable difference; the only delta is per-cell radar/dirty event
  multiplicity, which folds into D4's render-dirty scope. Not a standalone disparity.

## D4: Per-cell radar MarkTerrainDirty + DirtyScreenRect on direct collapse — VERDICT=REAL

**REAL — upgraded from a prior pass's UNCERTAIN after tracing the orchestrator + minimap radar wiring.**

- gamemd final branch of `0x0057cf60` confirmed live: `RadarClass__MarkTerrainDirty` ×3 on the triple,
  `TacticalClass__DirtyScreenRect`, `CellClass__RecalcAttributes` ×3, then `UpdateBridgeZonesHelper`.
  Confirmed identical in EW_High `0x0057d530`. Intermediate branches (`<0xD3→0xD3`, `0xDF→0xE0`, etc.)
  call DirtyScreenRect + RecalcAttributes ×3 but do NOT MarkTerrainDirty. The cascade leaf
  (`0x0057e7a0`, `iVar2 == 0xe7` block) additionally marks the sibling cell + its 2 perpendicular
  neighbors via `MapCoord_Add`/`FUN_00588c60`.
- Rust side traced end-to-end this pass:
  - `StateOutcome::Collapsed` (`mod.rs:381-401`) has NO radar/minimap field — only `binary_success,
    destroyed_cells, set_bridge_direction, adjacent_bridges_dirty, zones_dirty`. The destruction walkers
    return zero radar cells.
  - `bridge_orchestrator.rs::apply_bridge_damage_events` consumes only `destroyed_cells` (fallout),
    `set_bridge_direction` (BlowUpBridge), `adjacent_bridges_dirty` (rim), `zones_dirty` (zones). A grep
    of the entire orchestrator returns **zero** hits for `mark_radar_terrain_dirty` / `radar_terrain_dirty`
    / `radar_cells` — the direct-collapse path never marks radar terrain dirty.
  - The minimap terrain refresh IS driven by `sim.radar_terrain_dirty_cells` +
    `radar_terrain_dirty_generation` (`render/minimap.rs:225-243` → `apply_bridge_terrain_dirty_cells`).
  - `mark_radar_terrain_dirty_cells` (`mod.rs:513`) is fed by the engineer REPAIR path
    (`world_orders.rs:391`, from `RepairOutcome.radar_cells`) and combat smudge — but NOT by bridge
    destruction. The repair path correctly dirties cells; the symmetric destruction path drops it.
- **Conclusion:** the Rust direct-collapse path provably pushes 0 cells into `radar_terrain_dirty_cells`,
  whereas gamemd dirties the collapsed triple (+ cascade-leaf perpendicular neighbors) via MarkTerrainDirty.
  This is the exact dirty-cell mechanism the minimap consumes, and destruction does not feed it — so it is
  a REAL drift, not UNCERTAIN. Corrected delta: Rust marks **no** minimap/radar cells on direct bridge
  collapse → gamemd MarkTerrainDirty on the final triple + cascade-leaf neighbors. Player-visible: stale
  minimap terrain over the collapsed span. Fires every direct collapse. Severity MED.
- **Residual (only reason it is not raised higher):** I did not find an app-layer *blanket* full-minimap
  redraw keyed off `bridge_state_changed`; if such a coarse refresh exists it could cosmetically mask the
  missing per-cell dirties. But the per-cell dirty path gamemd uses is provably unfed by destruction.
- Verify: `decompile_function 0x0057cf60`, `0x0057d530`, `0x0057e7a0`; Rust trace `mod.rs:381-401/513`,
  `bridge_orchestrator.rs` (no radar call), `world_orders.rs:391`, `render/minimap.rs:225-243`.

> **Reconciliation note:** a prior adversarial pass left D4 at UNCERTAIN because the orchestrator radar
> wiring "was not traced." This pass traced it (orchestrator → `radar_terrain_dirty_cells` → minimap) and
> found the destruction path feeds nothing while the repair path does — sufficient evidence to upgrade to
> REAL under burden-of-proof (the equivalence the UNCERTAIN verdict deferred to is disproven).

---

## PARITY-CONFIRMED (independently re-verified live)

- **Dispatch gate.** `ApplyDamageToCell @ 0x00587180`: `(0x49 < ov < 100)` → `DestroyBridge_Low`;
  `(0xcc < ov < 0xe7)` → `DestroyBridge_High`. Rust `path_matches_cell` LowDirect `0x4A..=0x63`,
  HighDirect `0xCD..=0xE6` (`mod.rs:848-849`). Match. Confirms raw-overlay routing to the direct walker.
- **All 4 walker case values + transition overlays** re-read live and match Rust:
  NS_High `0xDF→0xE0 / 0xE1→0xE2 / <0xD3→0xD3 / 0xD3..0xD5→0xE7 (final) / >0xD5 noop` (`walker.rs:852-869`);
  EW_High `0xE3→0xE4 / 0xE5→0xE6 / <0xDC→0xDC / 0xDC..0xDE→0xE8` (`:946-963`);
  NS_Low `0x5C→0x5D / 0x5E→0x5F / <0x50→0x50 / 0x50..0x52→0x64` (`:1210-1227`);
  EW_Low `0x60→0x61 / 0x62→0x63 (=99) / <0x59→0x59 / 0x59..0x5B→0x65` (`:1299-1316`). Match.
- **Sibling-cascade perpendicular shift.** Intermediate cases dispatch ONE sibling, healthy/final BOTH.
  NS dispatches at `*param_1 ± 1` (same row → west/east); EW at `param_1[1] ± 1` (same col → north/south).
  NS/EW labels in Rust are correct. Match.
- **Cascade-leaf table + two-stage progression** (`0x0057e7a0`): `local_70[16]` =
  `[-1,0xD2,0xD5,-1,0xD1,0xD3,0xD5,-1,0xD4,0xD4,0xE7,-1×5]`; gate `if(0<iVar2){ if cur<0xDF table[idx]
  (return if cur==v); else 0xDF→0xE0; 0xE1→0xE2; else return }`. Rust `apply_bridge_destruction_ns_high`
  table + `cur<0xDF`/`0xDF→0xE0`/`0xE1→0xE2` + `n != cur` no-op guard (`walker.rs:736-748`). Match.
- **`is_final`/`zones_dirty`.** gamemd sets `local_a5=1` + `UpdateBridgeZonesHelper` only on final branch;
  Rust `zones_dirty: is_final` only on the final write. Match.
- **`CheckBridgeNeighbors_NS_Low @ 0x0057b990`** (a prior pass left this UNCHECKED) re-decompiled live:
  north(y-1) `{0x57,0x59,0x5b,0x61}=1, {0x5a,0x65}=2`; south(y+1) `{0x58,0x59,0x5a,99=0x63}=4, {0x5b,0x65}=8`.
  Rust `check_bridge_neighbors_ns_low` (`walker.rs:1066-1072`) matches byte-for-byte; the `return uVar4|4`
  early-return ≡ Rust `idx|=4` by disjointness of the two south sets. Match.

## NEW / MISS

- **MISS (non-observable, do NOT raise): walker return value (`local_a5`) vs Rust `binary_success`.**
  gamemd sets `local_a5=1` ONLY in the final-collapse branch; the `<0xD3→0xD3` (and `0xDF/0xE1`)
  intermediate branches return `0` even when a cascade sibling reaches final (the sibling's success flag
  is local to `ApplyBridgeDestruction` and never propagates to the walker). Rust hardcodes
  `binary_success: true` on every `Collapsed` (`walker.rs:917,1006,1270,1359`), so it returns `true` for
  an intermediate-center + final-sibling case where gamemd returns `0`. **Refuted as observable:** the
  direct path is single-shot in `run_dispatch_loop` (`max_attempts=1`, `bridge_orchestrator.rs:1429-1434`)
  and the outer path loop `break`s unconditionally (`:1473`), so `apply_damage_success()` drives nothing
  for the direct walker. Internal-only; recorded for completeness, not a parity gap.
- **MISS (cross-facet, low novelty): object-on-bridge notify on final collapse.** Both HIGH walkers'
  final branch calls `FindBridgeEndpoints_{NS,EW}_High(*param_1)` then `FUN_005868a0(&local_80)` with a
  `local_80=*param_1-1, local_7c=param_1[1]-1, local_78=3, local_74=3` rect — a 3×3 origin-`(x-1,y-1)`
  scan that notifies objects standing on the collapsing span (verified live in `0x0057cf60` and
  `0x0057d530`). The Rust direct walker returns only `destroyed_cells`/BlowUpBridge actions;
  `blow_up_bridge_cell_fallout` runs per BlowUpBridge cell, but whether its kill/DropIn set equals the
  3×3 `FUN_005868a0` rect (which is NOT the axial overlay triple) is unestablished. Belongs to the
  cascade/fallout facet; restated so it is not lost.
- **No other missed disparity found** in the walker bodies / cascade leaves / start-shift / axis
  classifiers. The intermediate-branch DirtyScreenRect/RecalcAttributes (no MarkTerrainDirty) fidelity
  folds into D4's render-dirty scope.
