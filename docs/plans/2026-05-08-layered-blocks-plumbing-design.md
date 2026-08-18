# Layered A* Entity-Block Plumbing Design

## Goal

Pass `Some(&combined_blocks)` for both `ground_blocks` and `bridge_blocks` at
the four `find_move_path` call sites that currently pass `None`, so the
layered A* sees the same hard-block cells (chiefly building footprints) the
flat A* already sees. Closes G1+G2 of the 2026-05-08 pathfinding disparity
scan and the visible portion of G3.

## Architecture Context

`find_move_path` ([src/sim/movement/movement_path.rs:153-280](src/sim/movement/movement_path.rs#L153-L280))
dispatches between two A* implementations based on `layered_pathing`:

- `layered_pathing=true` → `zone_search::find_layered_path_zoned` →
  `astar_search` with dual ground/bridge closed lists. Consults
  `ground_blocks` and `bridge_blocks` (separately) for hard blocks at
  [core.rs:471-483](src/sim/pathfinding/core.rs#L471-L483).
- `layered_pathing=false` → `zone_search::find_path_zoned` → `astar_search`
  with the merged `entity_blocks` set.

`supports_layered_bridge_pathing`
([movement_path.rs:47-59](src/sim/movement/movement_path.rs#L47-L59))
returns `true` for every `Drive`/`Walk`/`Mech` locomotor — i.e., **every
vehicle and infantry unit** uses the layered branch.

The four call sites of `find_move_path`:

| # | Site | Today: `entity_blocks` | Today: `ground_blocks` | Today: `bridge_blocks` |
|---|---|---|---|---|
| 1 | [movement_commands.rs:236](src/sim/movement/movement_commands.rs#L236) (queued append) | `merged_entity_blocks_ref` | `None` | `None` |
| 2 | [movement_commands.rs:278](src/sim/movement/movement_commands.rs#L278) (initial fresh path) | `merged_entity_blocks_ref` | `None` | `None` |
| 3 | [movement_path.rs:371](src/sim/movement/movement_path.rs#L371) (`try_repath_after_block`) | `Some(&combined_blocks)` | `Some(&combined_blocks)` ✅ | `None` |
| 4 | [movement_tick.rs:166](src/sim/movement/movement_tick.rs#L166) (segment-exhaustion auto-repath) | `mover_entity_blocks` | `None` | `None` |

Site 3 is the only one commit 7e35fef fixed (`ground_blocks` only). Sites
1, 2, 4 are entirely unfixed. `bridge_blocks` is `None` at every site.

`bump_crush::build_entity_block_sets`
([src/sim/movement/bump_crush.rs:112-181](src/sim/movement/bump_crush.rs#L112-L181))
returns `(ground_blocked, bridge_blocked, entity_block_map)`. As of
7e35fef:

- `ground_blocked` carries structure footprints (full foundation when
  `rules` is provided; anchor-only fallback otherwise).
- `bridge_blocked` is declared and never written to.
- `entity_block_map` carries soft-cost entries (codes 2/5/6) keyed only
  by `(rx, ry)` — un-layered.

`build_entity_block_set` (singular)
([bump_crush.rs:185-195](src/sim/movement/bump_crush.rs#L185-L195))
returns `(ground ∪ bridge, entity_block_map)`. Since `bridge` is empty,
this is functionally `(ground_blocked, entity_block_map)`.

`merge_path_blocks`
([movement_path.rs:27-45](src/sim/movement/movement_path.rs#L27-L45))
takes the entity_blocks set and optionally adds under-bridge cells when
`too_big_to_fit_under_bridge && !water_mover`. Result is the
`combined_blocks` local used by sites 3 and 4 (and constructed inline by
sites 1/2 as `merged_entity_blocks_ref`).

Verified gamemd parity claim (Ghidra this session, `0x73f0a0`
LAB_0073f4f9): the binary uses `cell+0xE4` (FirstObject ground list) vs
`cell+0xE8` (AltObject bridge list) for layer-aware passability checks,
with bridge-level occupancy re-read at `cell+0x128` triggered when
`prevFacing == cell+0x11B + 4` (deck height = ground+4). Strict per-layer
separation IS what gamemd does.

## Impact Analysis

**Touched code:** four call sites in `src/sim/movement/`, ~6 lines of
real code change plus comment cleanup. No signature changes, no new
types, no `bump_crush` change.

**Dependencies / consumers:** every caller of `find_move_path` is in
`src/sim/movement/`. No public API touched. `astar_search` already
consults `bridge_blocks` correctly per-layer; the fix only feeds it a
non-None value.

**Blast radius:** sim/ only. No render/UI/audio/net coupling. No save
format change. State hash unaffected.

**Determinism:** unchanged. `combined_blocks` is `BTreeSet<(u16,u16)>`
(sorted iteration). Tick ordering unaffected. Replays from before the
fix may produce different paths only on scenarios where the buggy paths
hit a building (forcing a re-route) vs. now correctly routing around
the first time.

**Risk areas:**

1. Vehicles on the bridge layer will now hard-block on cells in
   `combined_blocks` (mostly building footprints). Vanilla YR has no
   building under any bridge — no observable regression. Modded maps
   with bridges over buildings would get *more* parity (vehicles refuse
   to path along a bridge above a building, matching gamemd's strict
   per-layer object-list semantics).
2. The under-bridge cells added by `merge_path_blocks` (when
   `too_big_to_fit_under_bridge`) will now also hard-block bridge-layer
   pathing. This is harmless: the cells are bridge cells, and any
   vehicle pathing on the bridge layer through them obviously must
   already be small enough (otherwise it wouldn't be there) — but a
   too-big vehicle on the bridge wouldn't hit this path. Worth
   mentioning so a future reader doesn't get confused.

## Chosen Approach

Mirror the 7e35fef fix shape exactly: at each of the four call sites,
pass `Some(&combined_blocks)` (or its local equivalent) for both
`ground_blocks` and `bridge_blocks`. No changes to data construction,
types, signatures, or `bump_crush`.

This is the simplest fix that closes G1 and G2 fully and addresses
the most-visible part of G3 (bridge-layer A* now sees the same
structures the ground layer sees, even though `bridge_blocked` itself
remains empty — the union is reached via passing `combined_blocks`
to the bridge slot too).

The cross-layer soft-cost leak in `entity_block_map` (G3 deeper part,
ledger item L9) is intentionally NOT fixed in this design and is
carried as known drift.

## Tiny-Detail Ledger

| # | Detail | Source | How this design preserves it |
|---|---|---|---|
| L1 | `cell+0x140 & 0x100` is bridge flag; `cell+0xE4` (ground) vs `cell+0xE8` (bridge) selects per-layer object list | [GHIDRA 0x73f0a0 LAB_0073f4f9 — verified this session]; `PATHFINDING_ASTAR_GHIDRA_REPORT.md` §7.1 | Approximated: both layers receive the same `combined_blocks`. Strict separation deferred (L9 drift) |
| L2 | Bridge-level occupancy re-read at `cell+0x128` when `prevFacing == cell+0x11B + 4` (deck = ground+4) | [GHIDRA 0x73f0a0 — verified this session] | `astar_search` already determines bridge-vs-ground per-cell at [core.rs:116-118](src/sim/pathfinding/core.rs#L116-L118) using the same `height + 4` invariant via `is_at_bridge_level`. Unchanged by this fix |
| L3 | Bridge-layer entity at (X,Y) must NOT block ground-layer pathing through (X,Y) below, and vice versa | [GHIDRA 0x73f0a0 dual-list selection]; `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` Phase 5 | NOT fully preserved. In vanilla, the only blocker type that goes into `combined_blocks` is structures (code 7). No building exists under a bridge in any stock YR map → no observable divergence. Documented as accepted drift |
| L4 | Stationary unit on bridge cell = code 6 (cost ×8). Vehicles route around but don't hard-block | `PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md` §1.1; `PATHFINDING_ASTAR_GHIDRA_REPORT.md` §6.2 | Unchanged. `entity_block_map` already applies code-6 cost via [core.rs:558-570](src/sim/pathfinding/core.rs#L558-L570). Layer is un-distinguished, but the cost effect is correct on the layer the unit actually occupies |
| L5 | Buildings (allied/enemy/laser-fence) = code 7 (impassable, never expanded) | `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` Phase 9, §10e (LaserFence return 7); `PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md` §1.1 | Preserved on both layers post-fix. Structures live in `ground_blocked` ⊆ `combined_blocks`, which is now passed to both `ground_blocks` and `bridge_blocks` arguments → astar_search hard-blocks them on whichever layer it's expanding |
| L6 | `combined_blocks = ground_blocks ∪ bridge_blocks`. Today bridge_blocks is empty, so combined = ground (= structure footprints when rules present, anchor-only otherwise) | [movement_path.rs:32-45](src/sim/movement/movement_path.rs#L32-L45); [bump_crush.rs:124, 191-195](src/sim/movement/bump_crush.rs#L124) | Unchanged construction. The fix consumes the existing `combined_blocks` value at the layered-blocks slots |
| L7 | `merge_path_blocks` adds under-bridge cells (`is_elevated_bridge_cell`) when `too_big_to_fit_under_bridge && !water_mover`. Layer-agnostic addition into the merged set | [movement_path.rs:27-45](src/sim/movement/movement_path.rs#L27-L45) | Preserved. After fix, those cells will also hard-block bridge-layer A*, which is harmless (a too-big vehicle isn't pathing on the bridge anyway) |
| L8 | Flat A* fallback (`find_path_zoned`) uses `entity_blocks`. Layered branch (`find_layered_path_zoned`) uses `ground_blocks` + `bridge_blocks` separately. Not interchangeable in signature | [movement_path.rs:180-279](src/sim/movement/movement_path.rs#L180-L279); [core.rs:471-483](src/sim/pathfinding/core.rs#L471-L483) | Preserved. Signature unchanged. The fix passes the same data to both arg slots; the function still routes per-branch correctly |
| L9 | `entity_block_map` (soft costs codes 2/5/6) is keyed only by (rx, ry). Layer not represented → cross-layer soft-cost leak | [core.rs:558-570](src/sim/pathfinding/core.rs#L558-L570); [bump_crush.rs:112-181](src/sim/movement/bump_crush.rs#L112-L181) | NOT fixed. Accepted drift: a ground-layer unit pathing under a bridge will see one cell at ×8 cost when a stationary friendly is on the bridge above. Single-cell barely-perceptible detour. Defer to a future Option-C refactor |
| L10 | Fix shape established by 7e35fef item 3: pass `Some(&combined_blocks)` for ground_blocks at `try_repath_after_block` | git show 7e35fef -- src/sim/movement/movement_path.rs | This design extends that pattern verbatim to `bridge_blocks` at the same site, and to both arg slots at the three remaining sites |
| L11 | All four call sites already construct or have access to `combined_blocks` / `merged_entity_blocks_ref`. No new borrow lifetimes | [movement_commands.rs:166-220](src/sim/movement/movement_commands.rs#L166-L220); [movement_tick.rs:382-388](src/sim/movement/movement_tick.rs#L382-L388) | The fix only re-uses an existing reference — no lifetime/borrow changes |

## Design

### Components

None added. Four call sites changed in three files:
- [src/sim/movement/movement_commands.rs](src/sim/movement/movement_commands.rs) (sites 1, 2)
- [src/sim/movement/movement_path.rs](src/sim/movement/movement_path.rs) (site 3)
- [src/sim/movement/movement_tick.rs](src/sim/movement/movement_tick.rs) (site 4)

### Interfaces / Contracts

Unchanged. `find_move_path`, `find_layered_path_zoned`, and
`astar_search` keep their signatures. `bump_crush::build_entity_block_set`
unchanged. No public API touched.

### Data Flow

```
build_entity_block_sets
  → (ground_blocked, bridge_blocked = ∅, entity_block_map)
                ↓
build_entity_block_set wraps:
  combined = ground_blocked ∪ ∅ = ground_blocked
                ↓
merge_path_blocks may add under-bridge cells when too_big_to_fit_under_bridge
                ↓ &combined_blocks
find_move_path(
  entity_blocks  = Some(&combined),     // flat A* (unchanged)
  ground_blocks  = Some(&combined),     // was None at sites 1, 2, 4
  bridge_blocks  = Some(&combined),     // was None at all 4 sites
  ...
)
                ↓
astar_search per-layer hard-block check at core.rs:471-483 (unchanged code)
```

### Error Handling

Unchanged. Search exhaustion / no-path returns `None` as before.

### Testing Strategy

**Unit tests** in [src/sim/movement/movement_tests.rs](src/sim/movement/movement_tests.rs):

1. `test_initial_layered_path_avoids_friendly_building_footprint`
   - Setup: 12×12 grid; place a 2×2 friendly refinery at (5,5) so its
     footprint covers (5,5), (6,5), (5,6), (6,6); spawn a Grizzly tank
     at (1,5) with Drive locomotor.
   - Action: issue an initial move command to (10,5).
   - Assert: returned path does not contain any of (5,5), (6,5), (5,6),
     (6,6). Path length is finite.
   - This pins site 2 (initial fresh path).

2. `test_queued_append_layered_path_avoids_friendly_building_footprint`
   - Same setup as above, but the tank already has a movement target to
     (3,5); issue a `queue=true` move to (10,5).
   - Assert: appended portion of the path does not contain any
     foundation cells.
   - This pins site 1 (queued append).

3. `test_segment_exhaustion_repath_avoids_friendly_building_footprint`
   - Setup: 40×12 grid; friendly refinery at (20,5); tank at (1,5);
     issue a move to (38,5) (>24 steps so segment exhaustion fires).
   - Action: tick movement until the path segment is exhausted (~24
     ticks of progress) and the auto-repath at line 166 fires.
   - Assert: the new segment's path does not contain any foundation
     cells.
   - This pins site 4 (segment-exhaustion auto-repath).

4. (Optional, low priority) `test_bridge_layer_path_hard_blocks_under_bridge_building`
   - Synthetic map with a bridge cell at (X,Y) and a building footprint
     at (X,Y) below.
   - Layered A* on the bridge layer must refuse (X,Y).
   - Pins the side-effect behavior of the fix (bridge-layer hard-block
     for cells in `combined_blocks`) even though no vanilla map
     triggers it.

The existing `try_repath_after_block` regression tests (the 7e35fef
trio) continue to pass unchanged — the fix only adds non-None args
where None was passed.

**Smoke check** (manual): start a skirmish, build a refinery, order a
Grizzly across the base. With the fix, the tank routes around the
refinery cleanly on the first plan. Without the fix, the tank draws a
straight line through the refinery, bumps, and re-routes.

### Determinism

`combined_blocks` is `BTreeSet<(u16,u16)>` — sorted iteration order,
stable across platforms. Tick ordering unchanged. Replays from before
the fix may diverge wherever the buggy paths previously crossed a
building (they will now route around the first time instead of
bumping-and-rerouting), but that is the desired behavioral change.

## Architectural Decisions

- **Mirror commit 7e35fef item 3 exactly.** Same pattern in 4 sites
  instead of 1. No new abstractions.
- **Pass `combined_blocks` to `bridge_blocks` slot** even though the
  data is technically a ground-layer set. In vanilla YR no building is
  under a bridge, so this is observably equivalent to strict
  per-layer separation.
- **Defer per-layer `entity_block_map` split (L9 drift).** Cross-layer
  soft-cost leak is barely perceptible and would require a function
  signature change. Tracked in the design doc; revisit if a player-
  visible case is reported. Per the parity rule, this is a known and
  user-accepted drift, not a silently-skipped item.

## Alternatives Considered

**Option B — Layer-strict block sets.** Update
`bump_crush::build_entity_block_sets` to populate `bridge_blocked` by
checking each entity's `movement_layer_or_ground()`; pass `Some(&ground)`
and `Some(&bridge)` separately at all call sites. Rejected: in vanilla
YR, no entity that goes into a hard-block set ever sits on the bridge
layer (only structures hard-block, structures are ground-layer-only).
Identical observable output to Option A in vanilla; ~10 extra lines of
code for zero parity gain.

**Option C — Full per-layer split incl. `entity_block_map`.** Option B
plus split `entity_block_map` into `ground_block_map` /
`bridge_block_map`. Closes the cross-layer soft-cost leak. Rejected:
~40 lines, signature change, marginal observable improvement (single-
cell detour avoidance for ground units pathing under bridges with
units above). Revisit if a real bug surfaces.

**Bundling with G4 (cliff cost trigger).** G4 is independent (different
mechanism: needs a `cliff_ramp` flag on `ResolvedTerrainCell` driven
from tile attributes). Kept as a separate brainstorm to avoid scope
creep; per-fix verification stays focused.
