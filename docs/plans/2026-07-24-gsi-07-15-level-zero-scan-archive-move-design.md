# GSI-07.15 Level-Zero Scan, Archive, and Move Design

Date: 2026-07-24  
Design status: **AUTONOMOUSLY APPROVED FOR IMPLEMENTATION PLANNING**  
Contract:
`docs/contracts/2026-07-24-gsi-07-15-level-zero-scan-archive-move-implementation-contract.md`  
Base inspected: `dev` `4910e8ffe3d5ef9559b81b98a68b9b9d7bab18f9`

## Goal

Let standard stock War and Chrono Miners consume the existing Rust projection
of a still-present level-zero tiberium cell without widening unverified Slave
Miner behavior or absorbing the parent reducer/cell-authority work.

The player-visible result is that a standard miner can select, archive, retain,
and physically move toward a level-zero ore/gem cell instead of declaring the
field empty or abandoning the target.

## Scope Decision

The goal contract authorizes autonomous design and approval after adversarial
review. This design therefore treats the frozen implementation contract as the
scope decision that the generic brainstorm workflow would otherwise ask the
user to approve.

The design begins with the parent-owned staged invariant:

```text
present ResourceNode { remaining: 0 }
+ surviving mapped tiberium overlay with overlay_data == 0
```

It ends at the existing `Harvest` boundary or, for the archive sub-loop, after
the archive survives return/unload and starts outbound movement. It does not
perform the later zero cleanup.

## System Card

| Field | Value |
|---|---|
| GSI / name | GSI-07.15 / standard level-zero tiberium scan/archive/move |
| Core owner | `src/sim/miner/miner_system.rs` |
| State owner | `Miner` inside `GameEntity`; resource presence in `ProductionState.resource_nodes` |
| Production entry | `Simulation::advance_tick` -> production -> resource economy -> standard miner dispatcher |
| Native anchors | `0x004DD0A0`, `0x004DCE80`, `0x00485020`, `0x0073E5E0` |
| Stock actors | `HARV`, `CMIN` |
| Parent producer | suspended GSI-04.09 |
| Canonical cell dependency | suspended GSI-04.04/GSI-04.06 |
| Scheduler dependency | suspended GSI-01.05 |
| Neighbor | Slave Miner/GSI-09.06, behavior preserved only |
| Intended writes | `miner_system.rs`, `miner_tests.rs`; test-only `slave_miner.rs` only if needed |
| No-write surfaces | `production_economy.rs`, reducer/growth/cell authority, UI, render, snapshot/hash, protected damage worktree |

## Architecture Context

### Ownership and producers

`Simulation` owns `ProductionState`, whose deterministic
`BTreeMap<(u16, u16), ResourceNode>` is the current Rust resource-node
projection. `OverlayGrid` separately retains the overlay id/data byte.

Current stock producers do not create a persistent zero node:

- map seeding stores `(frame + 1) * base`;
- spread, growth, and TIBTRE insert positive values;
- current exact extraction removes the node;
- current density-zero reduction is a no-op.

The future parent reducer is therefore the natural producer for this design's
staged input.

### Production order

The real tick is:

```text
ground movement
-> teleport/special movement
-> combat and cell reductions
-> production/resource economy
-> standard HARV/CMIN miner dispatch
-> Slave Miner dispatch
-> ore growth/spread
```

The common `tick_miners_n` test helper is not a production oracle because it
runs teleport -> miner -> ground movement. New acceptance tests must call
`Simulation::advance_tick`.

### Standard miner path

The standard miner dispatcher skips `MinerKind::Slave`, snapshots HARV/CMIN in
live-object order, processes each snapshot against the shared live resource
map, writes miner state back, then updates harvest visuals.

Relevant standard calls:

- `handle_search_ore` long bounded scan;
- `handle_move_to_ore` per-tick rescan;
- `handle_harvest` short continuation scan;
- `save_archive_via_short_scan`.

Archive consumption first checks node key presence, but then applies the
current scan filter. In the bounded reachable fixture that filter passes; the
later `MoveToOre` validation is the rejection that prevents the zero archive
from reaching movement. A present archive that became unreachable remains a
separate named DRIFT.

### Neighbor path

The exported `search_local_ore` helper has four Slave Miner consumers:

- initial search;
- idle rescan;
- short scan correction;
- long scan correction.

Slave harvesting separately requires `remaining > 0`. Changing the exported
helper globally would create an internally inconsistent Slave loop without
binary evidence for GSI-09.06.

## Tiny-Detail Ledger

| Detail | Required treatment |
|---|---|
| Ring 0 | Accept present zero for standard miners; never apply filter/value |
| Rings 1+ | Admit present zero before existing filter/value logic |
| Value at zero | Preserve `base * (remaining + 1)`; exact stock base at zero |
| Positive values | Do not certify or redesign the current `remaining` projection |
| Tie | Preserve strict `old < new`; first seen wins |
| Ring exit | Preserve first productive ring |
| Ring traversal | Preserve current arm/corner visitation order |
| Bounds | Preserve signed intermediate and `u16` bounds checks |
| Filter | Preserve current ring-1+ zone/path/occupancy filter exactly |
| Archive creation | Standard short scan uses zero-eligible policy |
| Archive consume | Preserve key-presence check, clear order, and state transition |
| Move validity | Present node valid for standard miner regardless of remaining |
| Missing target | Preserve existing invalid-target branch; do not repair stale movement here |
| Retarget | Preserve per-tick standard rescan and movement-target clear on target change |
| Direct/A* movement | No change |
| Chrono outbound | Preserve ordinary drive behavior |
| Timer | No change; search-success/arrival timing remains DRIFT |
| Global fallback | Keep zero rejection; do not expand beyond native bounded scan |
| Slave behavior | Preserve positive-only helper semantics and current harvest gate |
| Live order | No change |
| Same-tick visibility | No change |
| RNG | No draw added, removed, or reordered |
| Hash | Node zero already hashed; no hash implementation edit |
| Archive hash | Record existing omission; do not touch protected `world_hash.rs` |
| Snapshot | No schema/version change |
| Overlay | Test stages and asserts it; feature does not mutate it |
| LandType | No reconstructed or shadow authority |
| UI/manual order | Excluded |
| Docs | Cite corrected live evidence and retain residuals |

## Design Approaches

### Approach A — policy-bearing internal scan core with two wrappers

Factor the current scan body into one internal function taking a small explicit
eligibility policy:

```text
StandardHarvester: key presence is eligible
PositiveOnlyLegacy: remaining > 0 is eligible
```

Route all four standard HARV/CMIN callsites through a private standard wrapper.
Keep the existing exported Slave-facing helper signature and positive-only
result by delegating it to the same core.

Change standard `MoveToOre` validity from positive remaining to node presence.

Advantages:

- one traversal/value/filter implementation;
- no duplicated ring semantics;
- standard scope is explicit;
- Slave callers remain source- and behavior-compatible;
- no new public API, state, allocation, RNG, or layer edge;
- smallest implementation surface.

Risks:

- the compatibility wrapper can hide the known Slave uncertainty if poorly
  named or undocumented;
- a future caller could select the wrong policy.

Mitigation:

- policy and wrappers have semantic names;
- the internal core is not exported;
- a neighbor regression pins Slave behavior;
- comments state that positive-only is preservation, not native parity.

### Approach B — duplicate a standard-only scan implementation

Copy the current scan into a private standard function and remove zero checks
from the copy. Leave the exported helper untouched.

Advantages:

- no behavioral risk to Slave callers;
- very local callsite change.

Rejected because:

- ring order, filter order, tie behavior, bounds, and value calculation would
  have two authorities;
- later fixes could drift between copies;
- duplicate parity-sensitive traversal is worse than an explicit policy seam.

### Approach C — change the shared helper and Slave harvest atomically

Remove zero rejection globally and teach Slave harvest to accept/clean it.

Advantages:

- one apparent tiberium-scan meaning;
- no compatibility mode.

Rejected because:

- the complete active Slave mission is UNCHECKED;
- the cleanup producer is not implemented;
- it widens the feature into GSI-09.06;
- it can create a new select/move/zero-return loop;
- it violates the smallest-prerequisite rule.

### Approach D — consult `OverlayGrid` or resolved LandType in the miner scan

Pass overlay/terrain authority into the scan and decide eligibility from
tiberium classification or the current resolved byte.

Rejected because:

- resolved LandType remains stale until the synchronous recalc dependency;
- reconstructing an effective byte would repeat the already cycle-popped
  shadow-authority design;
- it would widen signatures and couple standard scan to incomplete cell
  authority without closing same-tick consumers.

## Recommended Design

Choose **Approach A**.

### Internal behavior seam

The core receives a private, copyable policy and centralizes:

- node admission;
- ring 0;
- ring traversal;
- filter application;
- value calculation;
- tie handling;
- early ring exit.

The policy controls only whether a present zero node is admitted. It must not
change value, filters, iteration order, or bounds.

The existing exported helper remains the Slave compatibility wrapper.
Standard callsites use the private zero-eligible wrapper.

### Move validity

`handle_move_to_ore` handles only standard miners because the standard
dispatcher excludes `MinerKind::Slave`. Its initial validity check therefore
uses `resource_nodes.contains_key(&current_target)`.

No other invalid-target behavior changes. In particular, the existing stale
physical movement target on true node absence is a separate residual; this
feature only prevents a level-zero target from being misclassified as absent.

### Archive

Both standard short-scan callsites use the zero-eligible wrapper:

- no-bale/not-full continuation;
- full-gate archive save.

Archive consumption remains unchanged for this bounded feature. The declared
fixture keeps the archive reachable, so the standard move-validity change is
what lets that consumed zero archive survive the next tick. Native state 0
does not re-check archive reachability while current Rust does; that broader
archive-predicate mismatch is explicit residual DRIFT, not certified here.

## Test Design

### Test support

Add one test-only retail fixture loader. It must read the checked-in
`ini/rules.ini`, merge `ini/rulesmd.ini` over it, build both `RuleSet` and
`OverlayTypeRegistry` from that same merged rules source, then read
`ini/art.ini`, merge `ini/artmd.ini`, and call `RuleSet::merge_art_data`.
Before any tick, the fixture must assert the load-bearing stock facts it uses:

- `TiberiumShortScan=6` and `TiberiumLongScan=48`;
- HARV/CMIN are parsed standard harvesters with storage 40/20;
- retail ore/gem bale values resolve from the merged rules to 25/50;
- both HARV and CMIN can dock at GAREFN;
- GAREFN is a compatible `Bib=yes` 4x3 refinery with
  `NumberImpassableRows=3` whose art queue cell is `(4,1)`;
- `TIB01` exists at its compact registry ID, is flagged tiberium, and
  `tiberium_type_for_overlay(&rules.tiberium_types, tib01)` is type zero.

Add a production tick helper that calls only `Simulation::advance_tick` with
empty commands, `Some(&rules)`, a real `PathGrid`, the explicit
`OverlayTypeRegistry`, and 67 ms. It must not invoke the miner or movement
helpers directly.

The fixture must also construct a flat `ResolvedTerrainGrid`, set the staged
tiberium cells' `land_type`/`yr_cell_land_type` to the current native
tiberium byte and `terrain_class` to Tiberium, bind it to the simulation, and
call `Simulation::rebuild_zone_grid(&path_grid)`. A ring-1 acceptance case
must include both a blocked zero candidate and a reachable zero candidate so
`build_scan_filter` cannot silently fall back to `None`.

Spawn HARV, CMIN, and GAREFN through `Simulation::spawn_object`; never insert
them directly into `EntityStore`. For every spawned miner, assert all of:

- the entity is revealed/non-limbo;
- `lifecycle.object_alive` and `lifecycle.cell_marked` are both true;
- `in_logic_vector` is true and the stable ID occurs in
  `live_object_order_snapshot()`;
- the production-created `Miner.kind` matches the requested stock type.

Assert the same object-alive, cell-marked, non-limbo, LogicVector, and live
order facts for the spawned GAREFN.

Stage a level-zero cell with all three companion facts together:

```text
resource_nodes[cell] = ResourceNode { remaining: 0, expected resource type }
overlay_grid[cell] = (TIB01 compact id, overlay_data 0)
resolved terrain cell = native tiberium LandType byte
```

At every named checkpoint, reassert that the node key still exists with
`remaining == 0`, the mapped overlay ID is still present, and its data byte is
zero. Ore growth/spread remains disabled so no later phase rewrites the
fixture.

Do not replace existing tests wholesale; this helper is only for the new
production-order oracle.

### Red-first tests

Before implementation, the production tests must fail for the expected reason:

- no standard target selected from a zero node; or
- a restored zero archive is discarded in `MoveToOre`.

Compilation or fixture failures are not valid red evidence.

### Test 1 — both stock miner kinds cross ring 0 and ring 1

Parameterize HARV and CMIN over two production-spawned cases:

- **ring 0:** stage the present zero at the miner's own cell, also block that
  cell in `PathGrid`, and make a supplied filter reject it. Selection must
  still succeed, proving the native ring-0 filter/value bypass.
- **ring 1:** stage a blocked present-zero candidate earlier in scan order and
  a reachable present-zero candidate later in the same ring. The reachable
  candidate must be selected, proving the real zone/path filter ran.

For each actor/case, assert across real ticks:

1. `SearchOre` selects the expected zero cell;
2. the following dispatch retains the target;
3. when the target is remote, a movement target is issued;
4. a later production movement phase changes physical position toward it; and
5. the miner reaches `Harvest`.

Assert the companion invariant at each checkpoint. Capture
`Simulation::rng_state()` immediately before and after the acquisition-only
ticks and assert all three streams are unchanged. CMIN must not emit an
outbound teleport.

### Test 2 — value, tie, and ring exit

Use the policy-bearing core through the standard wrapper:

- level-zero gem beats level-zero ore in the same ring under stock base values;
- equal-value candidates retain the first native scan-order candidate;
- a nearer level-zero ring beats a richer farther ring.

This may be a focused scan test because production movement does not add
evidence to ranking order. It must still call the standard wrapper, not the
policy core directly.

### Test 3 — archive round trip

Use one exact stock HARV/GAREFN production fixture:

- merge retail rules and art as described above;
- spawn GAREFN at `(10,10)`, whose CAN_DOCK accepted/pad cell is
  `anchor + (3,1) == (13,11)` and whose art wait queue is `(14,11)`;
- derive the foundation and `Bib` flag from the merged retail object, apply
  `PathGrid::block_building_movement_cells` before rebuilding zones, and prove
  an interior foundation cell is blocked while pad `(13,11)` and queue
  `(14,11)` remain walkable;
- assert the spawned GAREFN is alive, revealed, cell-marked, registered in
  LogicVector, and present in occupancy for every 4x3 foundation cell;
- spawn HARV at `(20,11)`, physically distinct from the refinery footprint,
  wait queue, and accepted/pad handoff cell;
- stage the archive candidate at that same `(20,11)` ring-0 cell;
- fill the stock HARV with 40 retail-value ore bales;
- set only the pre-existing full-gate inputs (`Harvest`, target current cell,
  harvest timer due), leaving `last_harvest_cell` unset.

Then drive only real production ticks and assert:

1. the due full gate discovers `(20,11)`, writes it to
   `last_harvest_cell`, and enters return;
2. the HARV physically reaches the stock CAN_DOCK/pad cell `(13,11)`, and the
   exact phase-change trace is `Approach -> MissionEnter ->
   AwaitingAcceptedCell -> MissionEnter -> FaceSync -> MissionQueued ->
   Pivoting -> Unloading -> Departing`;
3. the archive and staged-zero companion invariant survive every return/dock
   phase;
4. `reserved_refinery` remains selected throughout Dock; the accepted HELLO
   contact and both sender/receiver radio mirrors exist from the first
   `MissionEnter`; contact-entered exists from `FaceSync` through `Departing`;
   and the stock zero-link path never creates an `on_pad` link;
5. unloading empties cargo, credits the GAREFN owner by the 40 stock ore
   bales, and reaches the zero-link Departing handoff without a fabricated
   exit destination;
6. Departing returns to `SearchOre`, keeps the HARV physically at the
   `(13,11)` pad handoff, clears both the miner field and dock-contact
   authority, and consumes exactly the already-documented resume-jitter RNG
   draw;
7. when that jitter expires, SearchOre consumes and clears the archive,
   retains `(20,11)` as `target_ore_cell`, and enters `MoveToOre`;
8. the next standard dispatch keeps that present-zero target and issues the
   outbound drive; and
9. a later movement phase changes the HARV's physical cell away from
   `(13,11)` toward `(20,11)`.

Record all three RNG streams before and after every tick in the dock loop.
Advance a cloned scenario RNG by exactly one draw on each due Approach,
MissionEnter, and FaceSync dispatch, on Pivoting -> Unloading, and on
Departing -> SearchOre; require no scenario draw on every other tick and no
main/mapgen draw anywhere in the loop. The selection/retention policy itself
must add no draw.

The test stops before `Harvest` can invoke the parent-owned level-zero cleanup.
If this exact fixture cannot reach every checkpoint without mutating unrelated
production owners, implementation planning must stop and narrow the contract;
it may not substitute a direct miner helper.

### Test 4 — Slave preservation

Add the regression beside the private Slave machinery in
`src/sim/slave_miner.rs`. Construct the smallest real Slave snapshot/master
fixture and call the existing Slave search-state dispatcher (`process_slave`
or its already-private search handler), not `search_local_ore` directly.

Expected: a lone present zero node remains rejected. The test name and comments
must say `preserves_current_unverified_behavior`. No new production API or
test-only public seam is permitted.

### Test 5 — deterministic node identity

Clone two otherwise identical simulations. Insert a present zero node in only
one and assert `state_hash` differs. Do not add an archive-only hash test that
would require the protected `world_hash.rs` fix.

## Validation Matrix

Run serially under the single Cargo lease:

1. each new production test by exact name;
2. existing:
   - `scan_ring_0_allows_harvesters_own_cell`;
   - `move_to_ore_target_stable_when_world_unchanged`;
   - `exit_pad_preserves_archive_on_arrival`;
   - `harvester_continues_to_short_scan_when_partial_then_empty`;
3. exact miner module filter;
4. exact Slave test/module filter if `slave_miner.rs` is touched;
5. `cargo check -q`.

Every test command must report the literal `test result:` line. Rust tests are
regression and integration evidence, not a native parity certificate.

## Ownership And Integration

- Create a unique `feature/gsi-07-15-level-zero-scan-move-<timestamp>` branch
  and linked worktree from the exact clean `dev` SHA after approval.
- Hydrate ignored `ini/` as a verified physical copy, never staged.
- Primary coordinator owns the implementation paths.
- Supporting reviewers remain read-only.
- Format only edited Rust files with edition 2024.
- Commit one coherent feature milestone.
- Run a clean-dev baseline, then a guarded `--no-ff --no-commit` merge.
- Inspect exact staged paths, run the combined validation matrix, create the
  merge commit, and do not push.
- Preserve the feature reference until the journal records cleanup and parent
  unwind.

## Why This Should Be Approved

- It repairs the earliest proven standard consumer rejection.
- It reuses the existing deterministic traversal rather than duplicating it.
- It makes the shared-neighbor boundary explicit and testable.
- It does not create a second LandType authority.
- It does not absorb producer, cleanup, scheduler, UI, or Slave work.
- It validates actual movement-before-miner production cadence.
- It binds tests to merged retail rules/art, canonical lifecycle registration,
  a non-vacuous zone filter, and the real HARV dock loop.
- It changes no schema, hash implementation, RNG, public API, or layer edge.

## Independent Challenge And Repairs

Two independent read-only reviews rejected the first draft. Their load-bearing
objections and the resolved design changes are:

| Objection | Resolution |
|---|---|
| Direct insertion would leave the production LogicVector empty | Every stock actor now uses `spawn_object`; membership and miner kind are asserted before ticking |
| “Stock-shaped” rules and a merely supplied overlay registry were vacuous | The oracle now reads merged checked-in retail rules/art and makes an explicit `TIB01 -> type 0` classifier assertion |
| A `PathGrid` without `zone_grid` silently disables the scan filter | The fixture binds resolved terrain, rebuilds zones, and presents blocked plus reachable zero candidates |
| Ring 0 was absent | HARV and CMIN are both exercised at ring 0 and ring 1 |
| Archive/dock feasibility was deferred | Exact GAREFN `(10,10)`, pad `(13,11)`, queue `(14,11)`, HARV/archive `(20,11)`, full cargo, and every dock/unload/outbound checkpoint are now fixed |
| “Movement begins” did not prove travel | The archive oracle requires a later physical-cell change from the pad toward the distinct archive |
| Companion state and RNG were underasserted | Node/overlay/data/terrain invariants and RNG checkpoints are mandatory throughout |
| Slave preservation tested only a shared wrapper | The regression must invoke the actual private Slave search dispatcher |

Both reviewers agreed that the single policy-bearing traversal core is the
smallest architecture-safe implementation once those oracle defects are
repaired.

## What Could Still Make It Wrong

- A native standard callsite could apply an additional density-zero check after
  the verified scan result.
- The parent reducer could choose a representation other than a present zero
  node, invalidating the staged seam.
- A stock Slave path could be proven to require the same atomic zero behavior.
- The exact merged-retail archive dock fixture could expose an unrelated
  production blocker; that is a hard stop, not permission to weaken the test.
- The internal policy could accidentally alter positive-node behavior, filter
  order, or tie order.

Current ordinary producers do not make the staged invariant, so this remains a
bounded parent prerequisite rather than a standalone player-reachable fix.
The parent reducer/cell-authority work must supply the natural producer before
the loop can be called complete.

## Autonomous Approval Decision

**Approved for implementation planning.** The design should be approved
because it changes only the earliest proven standard-consumer boundary, keeps
one traversal authority, and now has non-vacuous production oracles for every
scope claim. It could still be wrong if new live-binary evidence proves a
post-scan zero gate, the parent changes representation, the actual Slave path
must move atomically, or the exact dock fixture cannot run. None is presently
load-bearing; the first three are explicitly residual/unchecked boundaries and
the fourth is now an implementation-plan stop gate.

## Residuals After This Design

- No natural zero producer or cleanup.
- No canonical synchronous LandType/zone agreement.
- Growth remains after miners in Rust.
- Positive-density value projection remains non-native.
- Global unbounded fallback remains non-native.
- A saved archive is still filtered for Rust reachability even though native
  state 0 sends it directly to `Set_Destination`.
- Native search-success state/timer/destination timing remains divergent.
- UI/manual order paths still reject zero.
- `last_harvest_cell` remains omitted from the manual world hash.
- Slave zero behavior remains UNCHECKED and intentionally unchanged.
- No parity certification is made.
