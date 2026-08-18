# Coord Transform Audit — Ghidra vs Rust (2026-04-24)

Audits the isometric world↔screen↔cell coordinate math in gamemd.exe
against the Rust equivalents. Flags where the earlier
`MAPCLASS_GHIDRA_REPORT_FOLLOWUP.md` (§4) mis-identified the relevant
gamemd functions.

**Confidence:** HIGH — every gamemd function below was freshly
decompiled in this pass.

**Active in YR:** Yes — these transforms run on every tick, frame,
click, and cursor move.

**Patch note 2026-05-22:** verify-doc-swarm corrected the tactical
inverse details below: `0x006D6590` subtracts `g_RadarViewportOffsetX/Y`
internally, the height scan compares against `0xB4` for an effective
180 failed attempts, bridge edge tests are strict `> 15`, and
`0x006D1FE0` is not a raw alias of `0x006D1EB0` because it right-shifts
from lepton space to pixel space.

**Patch note 2026-07-18:** verify-doc-fix-swarm w1 slot 13 re-verified
all §1–§2 gamemd claims byte-for-byte against live decompiles — all
CONFIRMED, no drift since 2026-05-28. Found and corrected significant
Rust-side staleness in §3/§5/§6/§7: all six Rust file:line cross-refs
had drifted (file growth), and — more importantly — the production
click→cell path has been rewritten since the last patch. It no longer
goes through the 3-iteration/7×7-bbox functions this doc previously
flagged as parity mismatches; a new `screen_to_cell_tactical_inverse`
(src/map/terrain.rs) now uses the same iteration cap (180) and
cardinal-only bridge-neighbor strategy as gamemd's `0x6D6590`. This is
a constant/architecture match confirmed by reading current source, NOT
a certified bit-exact parity result — the doc now flags this as
"believed addressed, unverified" rather than "known mismatch."

---

## 1. Correction to the prior follow-up

`MAPCLASS_GHIDRA_REPORT_FOLLOWUP.md` §4 claimed that `FUN_005654A0`,
`FUN_00565520`, `FUN_00565660` are "the world↔cell coordinate
transforms used by every on-screen rendering path and mouse-hit
test." That attribution is wrong.

Tracing the callers:

- `FUN_005654A0` has only **10 xrefs**: 6 in a single function
  `FUN_004AA440` (a placement-search routine) and 4 in
  `HouseClass::DetermineEdge`.
- `FUN_00565520` has **1 xref**: the same `FUN_004AA440`.
- `FUN_00565660` has **1 xref**: `FlyLocomotionClass::Process`.

Nothing in the tactical render path, pick, or band-box code calls
them. Reading the arithmetic confirms why: the math operates on
**cell-space integers and LocalSize offsets**, not lepton or pixel
coords. These are really *local-grid ↔ cell-diamond* skew
transforms, used to iterate rectangular ranges of cells along the
map's LocalSize edge. They have no connection to camera/cursor math.

### What 0x5654A0 / 0x565520 / 0x565660 actually do

With `W = map.size_width (+0xF4)`, `L = map.local_left (+0xFC)`,
`T = map.local_top (+0x100)`:

**0x5654A0** — `(local_idx_x, local_idx_y) → (cell_x, cell_y)`
applies an iso skew:
```
t1 = in_x + L
t2 = in_y + T
cell_x = ((t2 + 1) >> 1) + t1
cell_y = W + (t2 >> 1) - t1
```

**0x565520** — cell → local-grid index (parity-aware inverse):
```
parity = W & 1
diff = cell_x - cell_y + parity
local_x = (diff >> 1) + W/2 - L
local_y = cell_x + cell_y - W - T
```

**0x565660** — same formula as 0x565520, packed into a 32-bit
`CONCAT22(short_y, short_x)` result.

Use cases confirmed from the callers:
- `HouseClass::DetermineEdge` uses them to walk along each of the
  four playfield edges when computing a house's starting edge for
  AI and ally logic.
- `FUN_004AA440` (placement search) uses them to iterate a
  rectangular window of valid placement positions and skew onto the
  diamond.

---

## 2. The real tactical coord transforms

These are the functions that actually handle camera/cursor/pick math:

| Address | Name | Purpose |
|---------|------|---------|
| `0x6D1EB0` | `Tactical::WorldToScreenSub` | lepton `(x,y)` → world-pixel `(sx, sy)`, iso only, no camera shift |
| `0x6D1F10` | `CoordsToClient` | lepton `(x,y)` → client-pixel `(sx, sy)` with Z from camera matrix |
| `0x6D1FE0` | `TacticalClass::CellToPixel` | lepton `(x,y)` → world-pixel (alias of WorldToScreenSub but 2-arg pattern) |
| `0x6D2140` | `TacticalClass::CoordsToClient2` | lepton → client-pixel; applies camera `+0xB0/+0xB4`, returns visibility bool |
| `0x6D6590` | `(unnamed)` client-pixel → cell | the real inverse: applies `Matrix3x4_TransformPoint`, iterates up to 180 times for height convergence, handles bridge cell shifts |

Patch note for the table: the `0x6D1FE0` row should not be read as a
raw alias of `0x6D1EB0`; it uses the same projection family but includes
its own 256-lepton-to-pixel shift/divide behavior.

### Forward math (gamemd `WorldToScreenSub`, 0x6D1EB0)

Input is lepton coords. `0x3c = 60`, `0x1e = 30`:
```
screen_x = (lepton_x * 60 / 2) + (lepton_y * -60 / 2) = (lepton_x - lepton_y) * 30
screen_y = (lepton_x * 30 / 2) + (lepton_y *  30 / 2) = (lepton_x + lepton_y) * 15
```

To map a cell, the caller first expands cells to lepton center:
`(cx * 256 + 128, cy * 256 + 128)`, producing:
```
screen_x ≈ (cx - cy) * 30       (the +0.5/-0.5 in the center lepton cancels)
screen_y ≈ (cx + cy) * 15 + 15
```

`CoordsToClient` adds the same math then divides by 256 (via `>> 8`
with a sign-fix) and subtracts a Z contribution read from the FPU
(computed by the camera matrix). Camera offset `+0xB0/+0xB4` is NOT
subtracted here — that only happens in `CoordsToClient2` (0x6D2140).
(corrected 2026-05-28: was "then subtracts the camera offset +0xB0/+0xB4 and a Z contribution"; binary for 0x6D1F10 shows no +0xB0/+0xB4 subtraction, only sign-fix >>8 and Z; +0xB0/+0xB4 subtraction confirmed exclusively in 0x6D2140 via decompile_function 0x6D1F10 — MISLEADING)

### Inverse math (gamemd 0x6D6590)

Screen-pixel → cell is **not** a simple iso inverse. It uses:

```
1. Apply camera matrix inverse (Matrix3x4_TransformPoint) to translate
   screen-pixel + camera offset back into world leptons (float).
2. Round to cell (floor-division by 256).
3. Iteratively (up to 180 times):
     a. Look up the resolved cell's height level (+0x11B).
     b. Re-apply matrix with corrected Y = screen_y + level*15.
     c. If the cell has bridge flag 0x100, check 4 cardinal bridge
        neighbors, possibly shift the pick to a neighbor cell that
        better matches the bridge-decking geometry.
     d. Break when converged or neighbor-shift decision finalizes.
```

Notable constants: the loop compares the incremented scan counter
against `0xB3` (not `0xB4`), giving an effective 180 failed attempts
(counter runs 0–179; break fires when `0xB3 < local_58`). The
initial Y-scan offset added before the loop is `+0xB4 = +180` pixels
— these two `0xB3`/`0xB4` constants are distinct and must not be
conflated. The bridge-neighbor edge tests use strict `> 0xF` / `> 15`
pixel comparisons.
(corrected 2026-05-28: was "compare against 0xB4"; binary at FUN_006d6590 shows `if (0xb3 < local_58)` — the compare literal is 0xB3; 0xB4 is the separate initial Y-scan offset; effective count of 180 iterations unchanged; ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT — see decompile_function 0x6D6590)

---

## 3. Rust equivalents

| Rust function | File:line | gamemd counterpart |
|---------------|-----------|--------------------|
| `terrain::iso_to_screen(rx, ry, z) -> (sx, sy)` | [src/map/terrain.rs:237](../ra2-rust-game/src/map/terrain.rs#L237) | `TacticalClass::CellToPixel` (0x6D1FE0) |
| `terrain::screen_to_iso(sx, sy) -> (rx, ry)` | [src/map/terrain.rs:285](../ra2-rust-game/src/map/terrain.rs#L285) | one-shot inverse; gamemd equivalent is the matrix transform at the start of 0x6D6590 |
| `terrain::screen_to_cell_tactical_inverse` | [src/map/terrain.rs:296](../ra2-rust-game/src/map/terrain.rs#L296) | **live production counterpart of 0x6D6590** — vertical scan loop, `TACTICAL_INVERSE_MAX_SCAN_ATTEMPTS = 180` (src/map/terrain.rs:33), cardinal-neighbor bridge check via `apply_tactical_bridge_inverse` |
| `terrain::screen_to_iso_with_height` (legacy) | [src/map/terrain.rs:476](../ra2-rust-game/src/map/terrain.rs#L476) | iterative height-correction loop inside 0x6D6590 — **now test-only**, no production callers |
| `terrain::screen_to_iso_with_height_and_bridges` (legacy) | [src/map/terrain.rs:490](../ra2-rust-game/src/map/terrain.rs#L490) | bridge-pick branch of 0x6D6590 — **now test-only**, no production callers |
| `world_point_to_cell` | [src/app_sim_tick.rs:1448](../ra2-rust-game/src/app_sim_tick.rs#L1448) | wraps `screen_to_cell_tactical_inverse`, not the legacy height+bridge variant |
| `screen_point_to_world_cell` | [src/app_sim_tick.rs:1482](../ra2-rust-game/src/app_sim_tick.rs#L1482) | subtracts camera, then world→cell |

(corrected 2026-07-18: all six file:line refs above had drifted from file growth — e.g. `iso_to_screen` was cited at line 187, actually at 237; `world_point_to_cell` was cited at app_sim_tick.rs:890, actually at 1448 — verified by reading current src/map/terrain.rs and src/app_sim_tick.rs this session — RUST_IMPL_SUPERSEDED/line-drift. Additionally, `world_point_to_cell` no longer calls `screen_to_iso_with_height_and_bridges`; a new function `screen_to_cell_tactical_inverse` (added after the 2026-05-22/05-28 patches, per its own doc comment "verified from gamemd.exe `0x006D6590`") is now the live click→cell path. This directly affects the "parity mismatch" findings in §5/§6 below — see corrections there.)

Rust constants: `TILE_WIDTH = 60`, `TILE_HEIGHT = 30`, `HEIGHT_STEP =
15`. These match gamemd's `0x3c = 60` and `0x1e = 30`. ✓

---

## 4. Formula diff — forward (cell→screen)

**Gamemd** (`CoordsToClient` with cell center leptons `(cx*256+128,
cy*256+128)`):
```
screen_center_x = (cx - cy) * 30
screen_center_y = (cx + cy) * 15 + 15 - z_from_matrix
```

**Rust** (`iso_to_screen(rx, ry, z)` returns tile NW corner):
```
screen_NW_x = (rx - ry) * 30 - 30
screen_NW_y = (rx + ry) * 15 + 15 - z * 15

# tile center = NW + (30, 15):
screen_center_x = (rx - ry) * 30
screen_center_y = (rx + ry) * 15 + 30 - z * 15
```

Diff: **Rust tile center Y is 15 px lower than gamemd's** (at
identical rx,ry,z). The X coords match.

This 15-pixel constant offset is absorbable via a world-origin shift
when drawing — the *relative* positions of neighboring tiles match
gamemd exactly (same 60×30 tile-grid spacing, same Y-per-level step
of 15). It only matters if Rust compares absolute world coords
against a gamemd-stored coord, which shouldn't happen anywhere in
sim code.

**Risk level:** LOW for sim; MEDIUM for render/UI if any draw path
bakes an assumption that gamemd's Y = Rust's Y at identical (rx,ry).
Worth grep for `(rx + ry) * 15` and friends outside `iso_to_screen`
to make sure nothing else independently encodes the forward math.

---

## 5. Formula diff — inverse (screen→cell)

### Canonical camera case

Both engines produce `rx = (dx + dy) / 2, ry = (dy - dx) / 2` where
`(dx, dy)` are screen-delta-over-tile-half. Equivalent.

### Elevated terrain

**Gamemd:** effective 180 failed attempts of look-up-cell-level /
correct-Y / re-matrix until convergence. Iteration count unbounded
by `break`, only by the hard cap implemented as an incremented-counter
compare against `0xB4`.

**Rust (historical, pre-2026-07-18):** up to 3 iterations with early
break at `<0.01 delta`. Per file comment: *"Converges in 1-3
iterations on typical RA2 terrain gradients."* This described
`screen_to_iso_with_height_and_bridges`, which was the production
click→cell path at the time of the 2026-05-22 audit.

(corrected 2026-07-18: `screen_to_iso_with_height_and_bridges` is no
longer the production path — `world_point_to_cell` in
src/app_sim_tick.rs now calls `terrain::screen_to_cell_tactical_inverse`,
which loops up to `TACTICAL_INVERSE_MAX_SCAN_ATTEMPTS = 180`
(src/map/terrain.rs:33), matching gamemd's effective 180-attempt cap
(`0xb3 < local_58` in `decompile_function 0x6D6590`) exactly in
constant value. The old 3-iteration function still exists but is
now test-only — see §3. RUST_IMPL_SUPERSEDED. This does NOT
constitute a certified bit-exact parity claim: no gamemd-derived
fixture was run this session to confirm the per-iteration cell/height
resolution matches frame-for-frame, only that the iteration budget
and gating constant now match.)

**Risk level:** Previously LOW/parity-mismatch under the old
3-iteration path. As of 2026-07-18 the live path's iteration cap
matches gamemd numerically; residual risk is UNVERIFIED pending a
dedicated gamemd-derived fixture test on tall terrain stacks (see §7
item 4).

### Bridge handling

**Gamemd:** When the resolved cell has `Flags & 0x100` (is-bridge),
it checks up to 4 cardinal neighbors on the bridge, and may shift
the pick one cell along the bridge direction based on which neighbor
is also a bridge cell and strict `> 15` pixel edge tests.

**Rust (historical, pre-2026-07-18):** When `bridge_height_map` is
provided, `screen_to_iso_with_height_and_bridges` searched a 7×7 cell
neighborhood around the ground-resolved cell for any bridge entries,
tested each at its deck height, picked the closest. This function is
now test-only (see §3) — it is no longer reachable from
`world_point_to_cell`/`screen_point_to_world_cell`.

(corrected 2026-07-18: the live path is now `apply_tactical_bridge_inverse`
in src/map/terrain.rs (called from `screen_to_cell_tactical_inverse`),
which is cardinal-only — it checks exactly the 4 directional neighbors
via `tactical_cardinal_neighbor` (DIR_NORTH=0, DIR_EAST=2, DIR_SOUTH=4,
DIR_WEST=6), matching gamemd's `MapCoord_StepByDir_GetCell(0/2/4/6)`
calls in `decompile_function 0x6D6590`. It also uses
`TACTICAL_BRIDGE_EDGE_THRESHOLD_PX = 15.0` (matches the binary's strict
`0xf <` edge test) and `TACTICAL_BRIDGE_EXTRA_HEIGHT_PX = 60.0` (matches
the binary's `& 0x3c` = 60 Y-shift). The "radial 7×7 bbox vs. gamemd's
directional cardinal scan" divergence described below is
RUST_IMPL_SUPERSEDED — the current implementation is architecturally
directional/cardinal like gamemd, not radial. This is a structural
match on constants and neighbor selection, not a certified bit-exact
parity claim — no gamemd-derived trace comparison was run this session.)

Both the historical Rust radial search and gamemd's cardinal scan are
described below for context; the current Rust implementation is
cardinal, matching gamemd's approach.

**Risk level (historical, superseded):** MEDIUM — divergence scenarios
under the old 7×7-bbox implementation:
- At a bridge endpoint where ramp meets ground, gamemd picks a
  specific ramp-side cell based on direction; the old Rust bbox picked
  closest regardless of direction.
- On a diagonal-looking bridge segment, gamemd's cardinal scan may
  miss a cell that the old Rust bbox caught (or vice versa).
- Edge where the old 7×7 bbox wrapped around a curved bridge run.

These scenarios are believed addressed by the cardinal-only rewrite
but UNVERIFIED pending a dedicated fixture — see suggested test below.

Suggested test: click precisely on the boundary between a bridge's
intact surface and its damaged-ramp segment. Compare gamemd vs Rust
resolved cells.

Audit correction: treat `gamemd.exe` as the parity target for these
bridge-cell sequences. The radial Rust search must not be preserved
just because it feels more plausible near bridge endpoints. (Historical
note, 2026-07-18: this recommendation appears to have already been
acted on — see correction above.)

### Radar viewport offset

**Gamemd:** `0x006D6590` subtracts `g_RadarViewportOffsetX/Y` inside
the inverse after applying tactical camera fields. Some callers may
pre-add the viewport offset before calling, so the end-to-end input
contract is caller-sensitive. This compensates for tactical viewport
origin/clip-rect differences such as the sidebar/radar layout.

**Rust:** `screen_point_to_world` = `screen/zoom + camera`. No
explicit radar-viewport offset applied.

This might be benign if the Rust camera already bakes sidebar
offset into the reported screen coord. Worth checking:
`app_sim_tick.rs::screen_point_to_world` vs the sidebar-clip rect
handling. If Rust reports click coords relative to the
full-window-including-sidebar, it needs an offset equivalent to the
radar viewport.

**Risk level:** MEDIUM — visible as cursor mis-alignment near the
sidebar-tactical boundary. Exactly the 99% parity bar territory.

---

## 6. Constants that must match gamemd exactly

| Constant | Gamemd | Rust | Match? |
|----------|--------|------|--------|
| Tile width (px) | `0x3c = 60` | `TILE_WIDTH = 60.0` | ✓ |
| Tile height (px) | `0x1e = 30` | `TILE_HEIGHT = 30.0` | ✓ |
| Level-step (px) | `0x0F = 15` | `HEIGHT_STEP = 15.0` | ✓ |
| Lepton-per-cell | `0x100 = 256` | implicit `1.0 cell` | ✓ (equivalent) |
| Iterate limit for cell pick | effective 180 failed attempts; break when `0xB3 < counter` (counter runs 0–179) | `TACTICAL_INVERSE_MAX_SCAN_ATTEMPTS = 180` (live path, `screen_to_cell_tactical_inverse`) | ✓ constant matches (corrected 2026-07-18, was `3` / ✗ — that described the now-test-only `screen_to_iso_with_height_and_bridges`; RUST_IMPL_SUPERSEDED, see §3/§5) |
| Bridge-neighbor threshold (px) | `0xF = 15` | `TACTICAL_BRIDGE_EDGE_THRESHOLD_PX = 15.0` (live path, cardinal-only via `apply_tactical_bridge_inverse`) | ✓ constant + strategy match (corrected 2026-07-18, was "7×7 bbox radius / ✗ — different strategy"; RUST_IMPL_SUPERSEDED, see §5) |
| Camera-matrix transform | 3×4 world matrix | fixed-iso assumption | ✗ — equivalent at canonical camera |

Note: the two ✓ corrections above are constant/architecture matches
confirmed by reading current `src/map/terrain.rs` this session, not a
certified bit-exact behavioral match — no gamemd-derived trace/fixture
comparison was run. Do not cite this table as a parity certification;
it downgrades the prior "known mismatch" status to "believed addressed,
unverified."

---

Audit corrections for the constants table above:
- Iterate limit: break condition is `0xB3 < counter` (literal `0xB3` in binary), giving
  180 iterations (0–179). The `0xB4 = 180` that appears in the same function is the
  *initial Y-scan offset*, not the iteration cap. Do not conflate them.
  (corrected 2026-05-28: prior text said "compare against 0xB4, not 0xB3 = 180" which had the
  constants backwards; binary `FUN_006d6590` shows `if (0xb3 < local_58)` — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT)
- Bridge threshold should be read as strict `> 0xF` / `> 15` edge
  tests, not just an unspecified 15-pixel threshold.
- The mismatch is a parity issue, not an acceptable 99% approximation.

## 7. Recommendations (no code changes)

1. **Grep audit**: outside `terrain::iso_to_screen` and
   `terrain::screen_to_iso`, no other Rust code should independently
   encode `(rx - ry) * 30` or `(rx + ry) * 15`. Search and
   consolidate any duplicates into the canonical helpers so a future
   formula fix lands in one place.

2. **Sidebar click boundary test**: move the cursor from inside the
   tactical view → across the sidebar → back. Verify the resolved
   cell at boundary pixels matches gamemd. If Rust drifts by even 1
   cell at the boundary, check whether a radar-viewport-style offset
   is missing in `screen_point_to_world`.

3. **Bridge click divergence test**: construct a scenario with a
   bridge at a known cell. Click at every pixel along the bridge
   span in both engines. Compare resolved cell sequences. Divergence
   at segment joints is expected. Treat `gamemd.exe` as the parity
   target; do not keep the radial Rust behavior because it feels more
   plausible.

   Status update 2026-07-18: the radial 7×7-bbox implementation this
   item warned about has been replaced by a cardinal-only
   `apply_tactical_bridge_inverse` (src/map/terrain.rs) — see §5. This
   test is still recommended to certify bit-exact parity; it has not
   been run this session.

4. **High-cliff click test**: find the tallest terrain stack in a
   stock map. Click at the extreme top. If Rust's 3-iteration budget
   fails, bump to 5–7 iterations (still far below gamemd's 180, but
   handles taller features).

   Audit correction for item 4: replace the approximation with
   gamemd's effective 180-attempt scan semantics instead of merely
   bumping to another small cap.

   Status update 2026-07-18: `screen_to_cell_tactical_inverse` now uses
   `TACTICAL_INVERSE_MAX_SCAN_ATTEMPTS = 180`, matching gamemd's
   effective cap — see §3/§5. This test is still recommended to certify
   bit-exact parity; it has not been run this session.

5. **Document the Y-offset convention**: the 15-px Y delta between
   gamemd and Rust tile-center is fine *if* Rust consistently uses
   its own world origin. Add a terrain-module comment explicitly
   stating the convention so future coord-related code doesn't
   accidentally reintroduce the gamemd offset.

---

## 8. What 0x5654A0 / 0x565520 / 0x565660 should actually be labeled

Proposed Ghidra rename (future session):

| Address | Proposed name |
|---------|---------------|
| `0x5654A0` | `MapClass::LocalIndex_to_Cell` |
| `0x565520` | `MapClass::Cell_to_LocalIndex` |
| `0x565660` | `MapClass::Cell_to_LocalIndex_Packed` |

Do **not** bundle them with "coord transform" in any Rust rewrite —
they are an internal helper for edge iteration, not a coordinate
system primitive.

---

## Sources

### Newly decompiled
- `0x6D1EB0` Tactical::WorldToScreenSub
- `0x6D1F10` CoordsToClient
- `0x6D1FE0` TacticalClass::CellToPixel
- `0x6D2140` TacticalClass::CoordsToClient2
- `0x6D6590` (unnamed) screen-pixel → cell with height/bridge iteration
- `0x6DA380` Tactical::PickObjectAtScreenPoint (for caller context)
- `0x4AA440` placement-search (for caller context of 0x5654A0)

### Re-read
- `0x5654A0`, `0x565520`, `0x565660` (local-index↔cell skews)

### Rust files read
- [src/map/terrain.rs](../ra2-rust-game/src/map/terrain.rs) (lines 20–260)
- [src/app_sim_tick.rs](../ra2-rust-game/src/app_sim_tick.rs) (lines 880–925)

### Xref scans
- `search_functions("Tactical")` — 30+ hits, key ones above
- `search_functions("CoordsToClient")` — 2 hits (0x6D1F10, 0x6D2140)
- function xrefs of 0x5654A0, 0x565520, 0x565660 — confirmed scope
