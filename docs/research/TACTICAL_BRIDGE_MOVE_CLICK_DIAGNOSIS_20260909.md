# Bridge movement clicks near railings: diagnosis and correction

## Finding and scope

On the current task code (`1e064efd`, merged through PR 314), the bridge-edge
calculation uses the deck artwork's bounding-box origin instead of the native
cell corner at base terrain height. On an ordinary high bridge, the reference
is displaced **30 world pixels left and 60 world pixels up**. This changes which
side of the bridge-edge decision a click occupies.

The user reported movement destinations resolving to ground when clicking near
one railing. This is a demonstrated code mismatch and a plausible cause; their
exact cursor position was not captured. It is separate from object selection and
from the recently accepted Grizzly/GI rendering comparison.

This document records research and diagnostic candidates. The production game
has not been changed by this investigation.

## Native contract

Retail executable SHA-256:
`1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c`.

The active tactical inverse is `gamemd.exe 0x006D6590`. The prior live binary
inspection established the following, independently checked against instructions:

- `0x006D6895..0x006D68FF` constructs `(cell_x*256, cell_y*256, 0)`, calls
  `0x006D1F10`, and subtracts the signed **base** cell level multiplied by 15.
- The projector at `0x006D1F27..0x006D1F98` uses the 60:30 isometric projection;
  `0x006D1FAB..0x006D1FC4` applies `AdjustForZ`. This caller supplies zero Z,
  so there is no hidden deck-height or artwork-origin term.
- Orientation and neighboring structural-bridge flags select north/west open
  edges and east/south direct-neighbor returns (`0x006D6793..0x006D6895`).
- Open-edge tests use `dy - trunc(dx/2) > 15` or `dy + trunc(dx/2) > 15`.
  The division is signed integer division and the threshold is strict.
  Passing the edge gate applies an additional 60-pixel adjustment to the scan.
  Direct-neighbor returns and the scan's eventual cell selection remain separate.

Original YR therefore legitimately resolves some clicks beyond the deck to ground.
That does not establish that VERA's currently displaced boundary is correct.

## Rust path and confirmed mismatch

The ordinary input path is:

`context_order::try_queue_context_order_at_screen_point`
→ `sim_tick::screen_point_to_world_cell`
→ `world_point_to_cell`
→ `terrain::screen_to_cell_tactical_inverse`
→ `apply_tactical_bridge_inverse`.

[`terrain.rs`](../../src/map/terrain.rs) uses
`iso_to_screen(cell_rx, cell_ry, bridge.deck_z)` for the bridge reference.
[`resolved_terrain.rs`](../../src/map/resolved_terrain.rs) supplies the actual
deck level in `build_tactical_bridge_inverse_map`.

After expressing both implementations in VERA's existing world-pixel frame:

| Reference | X | Y |
|---|---|---|
| Native equivalent | `(rx-ry)*30` | `(rx+ry)*15 + 15 - base_level*15` |
| Current Rust | `(rx-ry)*30 - 30` | `(rx+ry)*15 + 15 - deck_level*15` |

VERA's common `+15` world-row offset is deliberate and must remain consistent
with other world layers. The mismatch is the artwork's `-30` X term and use of
deck height at this particular reference step. Do not remove the native extra
60-pixel scan adjustment: that is a different operation.

For a walkable clicked cell, the app passes the resolved coordinates into the
ordinary `Command::Move`. In
[`movement_path.rs`](../../src/sim/movement/movement_path.rs), `goal_zone_layer`
derives bridge versus ground from the destination cell's structural-bridge flag.
Choosing a neighboring ground cell can therefore produce a ground destination
before pathfinding computes any route.

## Recommended correction

1. Compute the bridge reference from the raw cell corner at **base** terrain
   height. A minimal equivalent in the current frame is
   `iso_to_screen(cell_rx, cell_ry, terrain_z)` followed by `reference_x += 30`.
   Prefer a clearly named cell-reference helper if sharing this operation.
2. Preserve native orientation, neighbor checks, signed level comparisons,
   direct-neighbor returns, strict `>15`, and signed integer half-X arithmetic.
3. Make fractional camera/zoom coordinates follow one explicit policy at the
   shared inverse boundary. Casting only edge deltas is an integer-input
   diagnostic, not a complete fractional-input contract; scan/direct/edge
   decisions must use a consistent point.
4. Do not blindly replace cell rounding with floor. Rust's inverse currently
   produces center-relative coordinates; its positive-coordinate rounding
   already corresponds mathematically to native uncentered cell truncation.

Use zero Z for the raw reference projection and the explicit native
`base_level*15` step. Replacing it with arbitrary object-height projection would
introduce the separate `AdjustForZ` rounding rules into this calculation.

## Validation and delivery bar

Diagnostic files are under `.local/depth-occlusion/bridge-click-research/`.
`build_rust_probe.py` extracts the current pure inverse and helper bodies verbatim
into a standalone Rust executable, recording the source hash. It also generates
two isolated candidates: reference-only and reference plus signed edge arithmetic.
These are not app integration tests or production changes.

Before accepting a production fix:

- Compare native executed cases for both orientations, open edges, direct
  neighbor returns, height compatibility, odd signed X offsets, and equality.
- Check the complete shared inverse on the prepared Hills bridge: deck interior,
  both railings, entrances, and surrounding ground.
- Pass representative points through the ordinary Move command and check the
  resulting destination cell and layer.
- Cover flat-ground controls and camera/zoom if shared input quantization changes.
- Ask for the same railing movement comparison in YR and VERA after integration.

Existing `tactical_bridge_edge_threshold_is_strict` uses the current artwork
origin with `deck_z=0`; it does not certify the real reference alignment.
`world_point_to_cell_forwards_tactical_bridge_inverse_map` derives its expectation
from the same inverse and proves forwarding, not native equivalence.

A separate read-only critic confirmed the production ownership and correction
direction, and identified the fractional-input and existing-test limitations.

## Executed comparison: 1,782 edge cases

`native_edge_probe.py` executed original instructions starting at `0x006D6751`,
stopping at the scan continuation or direct-neighbor branch. Original neighbor
lookup (`0x00481810`), map lookup (`0x005657A0`), and projection (`0x006D1F10`)
ran without substituted function results. The original initializer `0x0049F2F0`
populated the direction table. Fixtures supplied five adjacent cell objects and
the registers/stack for an already-selected scan candidate.

The runner compared native outputs against the compiled, extracted Rust helper
and both diagnostic variants:

| Diagnostic variant | Differences from native / 1,782 |
|---|---:|
| Current production helper | 450 |
| Correct reference only | 6 |
| Correct reference and signed integer edge arithmetic | 0 |

Nine scenarios cover north-open and west-open edges, an enclosed cell, direct
south/east returns, and each orientation's compatible/incompatible neighboring
height condition. Each uses 198 integer points, including odd positive/negative
X offsets and equality boundaries. Base terrain is level 2, deck level 6, and
neighbor heights are 2 or 4. This is diagnostic sampling, not exhaustive parity.

Examples:

- On the north-open edge, `dx=-1, dy=15`: native signed division produces zero
  for `dx/2`, so the strict threshold does not pass. Reference-only float math
  produces `15.5` and wrongly applies the additional 60-pixel scan adjustment.
- On the west-open edge, `dx=1, dy=15` exposes the equivalent float discrepancy.
- A direct-south fixture at native pixel `(389,2310)` makes current Rust return
  the southern ground neighbor `(87,74)` immediately, while native continues the
  scan. Both reference-corrected variants agree with that native branch outcome.
  This does not by itself establish the final cell of the complete native scan.

Reproduction files:
[`native_edge_probe.py`](../../.local/depth-occlusion/bridge-click-research/native_edge_probe.py),
[`results`](../../.local/depth-occlusion/bridge-click-research/native_edge_results.json),
[`provenance`](../../.local/depth-occlusion/bridge-click-research/native_edge_results.meta.json).
The provenance identifies the retail executable, Unicorn 2.1.4, supplied state,
script/probe hashes, and production source hash
`0fa63760a865111727ce3904db882182c360b9cab6e07b9b0f0a5606ba096632`.

The full native inverse search, live mouse routing, fractional zoom, and actual
unit movement were not executed by this diagnostic. The isolated correction is
supported for the tested integer-input block; production integration and the
whole-path acceptance checks above remain necessary.

The final independent review checked all 1,782 records, executable/source hashes,
the isolated candidate differences, and retail disassembly for fixture entry
state and branch endpoints. It found no blocker to this bounded diagnosis.
