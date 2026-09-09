# Bridge movement clicks near railings: diagnosis and correction

## Finding and scope

On the prior code (`1e064efd`, merged through PR 314), the bridge-edge
calculation uses the deck artwork's bounding-box origin instead of the native
cell corner at base terrain height. On an ordinary high bridge, the reference
is displaced **30 world pixels left and 60 world pixels up**. This changes which
side of the bridge-edge decision a click occupies.

The user reported movement destinations resolving to ground when clicking near
one railing. This is a demonstrated code mismatch and a plausible cause; their
exact cursor position was not captured. It is separate from object selection and
from the recently accepted Grizzly/GI rendering comparison.

The production correction now uses the native reference and signed edge
arithmetic in the shared tactical inverse. Camera/zoom fractions are quantized
once to the containing world pixel before the scan. The executable comparisons
and integration coverage below distinguish native parity from VERA regressions.

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
The correction preserves those legitimate ground destinations; it does not make
every visible pixel of railing artwork a bridge destination.

## Rust path and confirmed mismatch

The ordinary input path is:

`context_order::try_queue_context_order_at_screen_point`
→ `sim_tick::screen_point_to_world_cell`
→ `world_point_to_cell`
→ `terrain::screen_to_cell_tactical_inverse`
→ `apply_tactical_bridge_inverse`.

The prior [`terrain.rs`](../../src/map/terrain.rs) used
`iso_to_screen(cell_rx, cell_ry, bridge.deck_z)` for the bridge reference.
[`resolved_terrain.rs`](../../src/map/resolved_terrain.rs) supplied the actual
deck level in `build_tactical_bridge_inverse_map`. That unused presentation
field has been removed from the click metadata; structural/orientation flags
remain owned by the resolved terrain.

After expressing both implementations in VERA's existing world-pixel frame:

| Reference | X | Y |
|---|---|---|
| Native equivalent | `(rx-ry)*30` | `(rx+ry)*15 + 15 - base_level*15` |
| Prior Rust | `(rx-ry)*30 - 30` | `(rx+ry)*15 + 15 - deck_level*15` |

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

## Implemented correction

1. Compute the bridge reference from the raw cell corner at **base** terrain
   height. A minimal equivalent in the current frame is
   `iso_to_screen(cell_rx, cell_ry, terrain_z)` followed by `reference_x += 30`.
   This operation remains local to its one bridge-edge consumer.
2. Preserve native orientation, neighbor checks, signed level comparisons,
   direct-neighbor returns, strict `>15`, and signed integer half-X arithmetic.
3. Floor world-pixel coordinates once after viewport subtraction at the shared
   inverse boundary. Scan, direct-neighbor and edge decisions use that same
   point, including negative world X. Native inputs are integer pixels;
   fractional zoom is a VERA extension, native equivalent UNCHECKED.
4. Do not blindly replace cell rounding with floor. Rust's inverse currently
   produces center-relative coordinates; its positive-coordinate rounding
   already corresponds mathematically to native uncentered cell truncation.

Use zero Z for the raw reference projection and the explicit native
`base_level*15` step. Replacing it with arbitrary object-height projection would
introduce the separate `AdjustForZ` rounding rules into this calculation.

## Original diagnostic and delivery bar

Diagnostic files are under `.local/depth-occlusion/bridge-click-research/`.
`build_rust_probe.py` extracted the prior pure inverse and helper bodies verbatim
into a standalone Rust executable, recording the source hash. It also generates
two isolated candidates: reference-only and reference plus signed edge arithmetic.
These are not app integration tests or production changes.

Production acceptance checks:

- Compare native executed cases for both orientations, open edges, direct
  neighbor returns, height compatibility, odd signed X offsets, and equality.
- Check the complete shared inverse on the prepared Hills bridge: deck interior,
  both railings, entrances, and surrounding ground.
- Pass representative points through the ordinary Move command and check the
  resulting destination cell and layer.
- Cover flat-ground controls and camera/zoom if shared input quantization changes.
- Ask for the same railing movement comparison in YR and VERA after integration.

The old `tactical_bridge_edge_threshold_is_strict` used the artwork origin with
`deck_z=0`; it has been corrected to the cell corner but alone does not certify
the real reference alignment.
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
| Prior production helper | 450 |
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
- A direct-south fixture at native pixel `(389,2310)` makes prior Rust return
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

The original isolated diagnostic did not execute the full native search or app
integration. The tracked implementation checks below supersede that limitation
within their stated coverage; these older local artifacts remain historical.

The final independent review checked all 1,782 records, executable/source hashes,
the isolated candidate differences, and retail disassembly for fixture entry
state and branch endpoints. It found no blocker to this bounded diagnosis.

## Tracked native oracle and production regression coverage

Run `python -m tools.bridge_click_oracle --check` from the repository. Default
operation compares native outputs read-only; `--write` explicitly regenerates
the fixtures. No downloads or external game modifications are involved.

- [Generator](../../tools/bridge_click_oracle.py)
- [Expected native outputs](../../tools/bridge_click_oracle/vectors.json)
- [Executable identity, assumptions and runtime provenance](../../tools/bridge_click_oracle/vectors.meta.json)

The generator executes the 1,782 candidate cases above and **572 complete calls
to `0x006D6590`**. Complete calls cover two bridge orientations, both edges,
interior and approach points, integer camera and viewport offsets. Their terrain
is synthetic flat level 2 at Hills coordinates, not the actual retail map.
There are 286 world-point cases (260 distinct points); the second camera
reproduces the same normalized destinations.

The original direction initializer `0x0049F2F0`, inverse-matrix constructor block
`0x006D1DC5..0x006D1E1E`, matrix multiply `0x005AFB80`, x87 conversion
`0x007C5F00`, projector and cell lookups execute unchanged. The constructor's
stored matrix constants are used, not an invented analytic substitute. Output
guards, native stack cleanup and mapped lookup bounds are checked. No hooks
replace function outcomes. Metadata records the supplied cells, flags, x87 state
and zero-Z projection assumption.

[`terrain_bridge_click_tests.rs`](../../src/map/terrain_bridge_click_tests.rs)
compares the production candidate helper and complete inverse against those
native outputs. Separate flat/elevated controls cover the fractional-pixel policy.

[`bridge_click_tests.rs`](../../src/app/input/bridge_click_tests.rs) exercises
the actual app camera transform, shared inverse, ordinary Move serialization
and command admission/drain for MTNK and E1 across the 286 world-point cases, four
camera/zoom settings and fractional offsets, including negative world X. These
are production-owner integration tests, not window-event automation. The
context-order selection and unwalkable-goal fallback branches are unchanged.

Two separately runnable retail tests load Hills.mmx and its real rules/terrain,
issue ordinary tank/GI Move commands to deck and approach cells, and check
arrival on the intended movement layer. They are Rust integration checks, not
native path-sequence goldens. The user will repeat the original railing clicks
in the release preview; their exact original cursor pixels were not recorded.

## Final implementation validation

`python -m tools.bridge_click_oracle --check` reproduced all 2,354 native
outputs with no fixture changes. Independent review also replayed 16 complete
native calls and found no implementation or oracle blocker.

`cargo test -p vera20k --lib bridge_click -- --include-ignored --nocapture`:

```text
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 8633 filtered out; finished in 21.66s
```

Real Hills arrivals for MTNK were `(87,74)`/124 frames and `(87,76)`/31 frames
on the bridge, followed by east `(98,74)`/138 and west `(75,74)`/254 on ground.
E1 completed the same orders in 326, 42, 321 and 570 frames, respectively.
A separate ground E1 reached `(87,78)` in 178 frames on the ground layer.
These are arrival checks; the existing underpass movement regression separately
checks continuous ground-layer traversal beneath the span.

`cargo test -p vera20k --lib`:

```text
test result: ok. 8549 passed; 0 failed; 91 ignored; 0 measured; 0 filtered out; finished in 22.00s
```

The initial test run exposed two harness assumptions, both corrected without
changing gameplay: flat level-zero inverse results legitimately use the scan
fallback, and a headless empty-batch tick does not drain queued player orders.
The retail checks now perform the production app's due-command drain followed
by `advance_frame(..., Ordinary)` and assert initial movement-target admission.
No snapshot/replay baseline, retail asset, INI or MIX file was changed.

`cargo clippy -p vera20k --lib` completed successfully in 45.03 seconds with
the existing 1,147 library warnings. Independent final review cleared the
bounded click correction after inspecting the focused and retail results.
