# Failed-A* Retry Cell-Entry Oracle Design

## Goal

Add the smallest Rust-native, search-local Unit/Infantry cell-entry oracle that preserves the verified `gamemd.exe` retry semantics and enables the exact failed-A* retry lifecycle without cloning the world or reusing approximate path-grid legality.

## Architecture Context

`Simulation` owns the authoritative mutable state needed by the query: `EntityStore`, live object order, `OccupancyGrid`, fog, alliances, resolved terrain, bridge and overlay state. `advance_tick` also receives the active `RuleSet` and `OverlayTypeRegistry`. Movement runs before the later fog recompute and processes movers in live-object order, so a path search must observe mutations committed by earlier movers in the same tick.

`tick_movement_with_grids` already follows a snapshot/defer pattern around Rust borrow boundaries. It creates `MoverSnapshot` before the inner mutable mover scope and defers cell-entry and drive-track checks until the mutable entity borrow has ended. Path searches currently receive a small `PathfindingContext` containing path/zone/terrain grids and blocker-neighbor counts.

`movement_path.rs` funnels flat searches into `zone_search::find_path_zoned_marker`. In the eligible flat hierarchy branch, `zone_search.rs` performs one precheck and one marker-gated A* attempt. `core.rs` has a `HierarchyProgressTracker`, but its success-only wrapper discards the progress cell when A* fails. The exact retry loop and exclusion producer are not yet active.

`cell_entry.rs` contains useful shared types such as `CanEnterLayerContext`, but its production occupied-cell classifier explicitly approximates the binary: it crushes first, selects one primary blocker, applies coarse friendship results, and blanket-overrides non-code-7 results for JumpJet. Its Infantry terrain fast path also returns clear as soon as it finds a free subcell and no selected-list non-Infantry blocker. These mechanisms cannot supply the verified retry result.

The approved design therefore adds one focused cell-entry module rather than extending the already large `cell_entry.rs`. The independent retry lifecycle remains a separate pure zone-pathfinding owner because it consumes only cell-entry codes and hierarchy data, not world objects.

Relevant architecture boundaries:

- `sim/` remains independent of render, UI, sidebar, audio, and net.
- `EntityStore` remains the durable entity-state owner. `OccupancyGrid` remains the
  transient CellClass analogue, but it owns both selected-list order and the
  native movement-occupation bits/Infantry-owner cache that cannot be reconstructed
  from the current list at query time.
- `LogicScheduler`/live object order determines when a mover's synchronous search occurs.
- `RuleSet`, `OverlayTypeRegistry`, resolved terrain, fog, and alliance maps remain authoritative sources; the oracle does not duplicate their durable ownership.
- Retry state and the oracle are derived call-local state. The CellClass-style
  occupation shadow is transient and rebuilt after load; the newly required
  per-entity `+0x388` locomotor-facing timer is durable, serialized, and hashed.

Primary sources:

- `docs/contracts/2026-07-18-failed-astar-retry-can-enter-oracle-implementation-contract.md`
- `docs/contracts/2026-07-18-pathfinding-failed-astar-retry-implementation-contract.md`
- `docs/research/pathfinding/PATHFINDING_FAILED_ASTAR_RETRY_CAN_ENTER_CELL_ADAPTER_GHIDRA_REPORT.md`
- `docs/research/pathfinding/INFANTRYCLASS_CAN_ENTER_CELL_TERMINAL_OCCUPANCY_AND_WALL_GHIDRA_REPORT.md`
- `docs/research/pathfinding/UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md`
- `docs/research/LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md`
- `docs/research/CELLCLASS_MAPCLASS_ENGINE_SUBSTRATE_SERVICE_STUDY.md`
- `docs/research/FOOTCLASS_0X388_LOCOMOTOR_FACING_GHIDRA_REPORT.md`

## Impact Analysis

### Touched modules

| Module | Design impact |
|---|---|
| `src/sim/occupancy.rs` | Extends each ground/bridge cell layer with native movement-occupation bits and the last-marked Infantry owner; add/remove/rebuild update list and shadow atomically. |
| `src/sim/game_entity.rs` | Adds the serialized, timer-backed locomotor/body-facing state corresponding to active native `+0x388`, distinct from `barrel_facing`. |
| `src/sim/movement/facing_class.rs` | Reuses the existing verified timer primitive; only narrowly exposes helpers needed to project the current 16-bit value into the compatibility 8-bit facing. |
| `src/sim/movement/movement_commands.rs`, `movement_step.rs`, `movement_tick.rs`, and `drive_track.rs` | Route active body-facing snap/retarget/projection writes through the `+0x388`-equivalent timer without changing movement ordering. |
| `src/sim/pathfinding/retry_cell_entry.rs` | New focused owner for the borrowed search view, mover facts, fixed retry tuple, layer derivation, and exact Unit/Infantry policies. |
| `src/sim/pathfinding/zone_retry.rs` | Pure owner for the already-contracted attempt/update/exclusion lifecycle. It consumes a narrow cell-entry query and hierarchy inputs only. |
| `src/sim/pathfinding/mod.rs` | Declares the two focused internal modules. |
| `src/sim/movement/mod.rs` | Extends `PathfindingContext` with an optional search-local exact-query reference. It remains a read-only environment value. |
| `src/sim/movement/movement_tick.rs` | Receives existing fog/overlay sources, constructs the oracle immediately before eligible searches, and defers path requests that currently occur inside a mutable mover borrow. |
| `src/sim/movement/movement_path.rs` | Carries the oracle reference through the flat search call without exposing world systems as individual parameters. |
| `src/sim/pathfinding/zone_search.rs` | Coordinates the exact bounded retry loop and hands `(neighbor, direction)` probes to the query. |
| `src/sim/pathfinding/core.rs` | Returns failed-attempt progress instead of discarding it. It remains unaware of entities, rules, and overlays. |
| `src/sim/world/mod.rs` | Passes existing `FogState`, `OverlayGrid`, and `OverlayTypeRegistry` references into movement alongside existing rules, alliances, entities, occupancy, and terrain. |
| `src/sim/world/world_spawn.rs` | Initializes the locomotor-facing timer from spawn facing and parsed `ROT`, matching native Unlimbo/Unit construction roles. |
| `src/sim/world/world_hash.rs` and `src/sim/snapshot.rs` | Hash the new durable timer and bump the snapshot version once; occupancy shadow remains rebuilt and is not serialized. |

### Dependencies and blast radius

- Existing non-hierarchy, layered bridge, compatibility corridor, and headless path callers must continue to compile without constructing the oracle.
- Exact retry activation is allowed only when the complete query is present. Absence must not synthesize a result from `PathGrid`, `LayeredEntityBlockMap`, or the approximate runtime classifier.
- Movement borrow restructuring can affect path exhaustion and blocked-repath timing. The design preserves the same-tick call point: it defers only the Rust borrow, not the logical operation or commit order.
- `movement_tick.rs`, `occupancy.rs`, `game_entity.rs`, and `world/mod.rs` currently contain unrelated worktree changes. Implementation must patch only the named boundaries and preserve concurrent work.
- Adding the `+0x388`-equivalent timer changes serialized entity state and the
  deterministic hash, so the snapshot version must advance once after coordinating
  with other sessions. The transient occupancy shadow does not change the snapshot
  schema. No INI schema or public app API change is required.

### Determinism

- The oracle holds immutable references for the duration of one synchronous search.
- It is constructed after earlier live-order mutations and dropped before the current mover commits later mutations.
- It consumes no RNG and performs no writes or deferred effects.
- Cell occupants are scanned through `OccupancyGrid::iter_layer`, never `EntityStore` key order.
- Packed movement-occupation state is read directly from `OccupancyGrid`; it is not
  refolded from current occupants. Infantry owner retains the native last-marked
  value while any functional Infantry bit remains.
- Moving-allied deadlock comparison samples both authoritative locomotor-facing
  timers at the same pre-increment binary frame.
- Retry exclusions remain ordered and duplicate-preserving as specified by the separate retry contract.

## Chosen Approach

Use a borrowed, search-local `RetryCellEntryOracle` plus a small owned mover-facts
value, backed by two narrow native-state foundations: transient per-layer
movement-occupation shadow state and a durable per-entity locomotor-facing timer.

The oracle borrows authoritative state only while the path search is running. It does not copy the map or all entities. Movement releases any mutable mover borrow, builds the mover facts and oracle, invokes the search, drops the oracle, then reacquires the mover to commit the path result.

The pathfinding seam is a narrow read-only query of the form `(candidate_cell, direction) -> full native code`. The zone retry owner does not receive `Simulation`, `EntityStore`, rules, or occupancy. The oracle internally forms the verified candidate-self signed height, null-parent, final-flag-1 tuple and dispatches by runtime Unit/Infantry class.

This approach was chosen because it:

- observes the exact same-tick world without a whole-world clone;
- follows the movement system's existing snapshot/deferred-work pattern;
- keeps world knowledge out of zone retry and A*;
- preserves full result codes for tests while allowing the retry producer to consume zero/nonzero;
- avoids a new cache-invalidation subsystem;
- permits pure policy fixtures and pure retry-kernel fixtures independently.

Before the oracle policy is wired, the existing occupancy and body-facing lifecycle
owners are extended rather than creating retry-only reconstructions. This keeps the
query pure while preserving the native state history it reads.

## Tiny-Detail Ledger

- Runtime object class chooses Unit `0x0073F0A0` versus Infantry `0x0051BF90`; MovementZone and JumpJet locomotor do not replace class dispatch. [doc: `PATHFINDING_FAILED_ASTAR_RETRY_CAN_ENTER_CELL_ADAPTER_GHIDRA_REPORT.md` sections 2, 4]
- Every retry probe uses candidate cell, direction `0..7`, sign-extended candidate `Level`, null parent, and final argument `1`. [doc: adapter report section 2]
- The retry consumer retains the full code internally but tests `code == 0` separately from the movement-zone matrix. [doc: adapter report section 2]
- Standard search direction order must remain native direction-table order. [doc: adapter report sections 2, 7]
- Candidate terrain and occupancy byte/bit/owner remain ground for the fixed tuple. [doc: adapter report section 3; Infantry terminal report section 2]
- The occupation bits consumed by retry are stateful marks, not a count derived from
  the current object list: Infantry mark ORs its subcell bit and overwrites owner;
  unmark clears that bit and clears owner only when `(bits & 0x1C) == 0`; Unit
  mark/unmark sets/clears bit `0x20` without reference counting. [doc:
  `INFANTRY_SUBCELL_POSITIONING.md`; `LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md`]
- `OccupancyGrid::rebuild` replays surviving objects in persisted
  `occupancy_enter_order`, matching gamemd's post-load Unlimbo reconstruction; the
  CellClass shadow itself is not serialized. [doc:
  `CELLCLASS_MAPCLASS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` section 8 item 14]
- The null-parent reconstructed diff-4 bridgehead case may select bridge objects while occupancy bits and owner remain ground. [doc: adapter report section 3]
- The selected cell object list is scanned in its native list order. [doc: Unit report phase 10; contract T5]
- Unit self-handling clears bit `0x20` from the local sampled value while skipping
  self in the selected-list scan; Infantry skips self only in the object scan and
  retains its already-marked subcell bit in the sampled packed value. [doc: Unit
  report phase 10a; live `InfantryClass::MarkCellOccupancy @ 0x005217C0` and
  `InfantryClass::Can_Enter_Cell @ 0x0051BF90`]
- The query observes the state at the current mover's live-order search point, after earlier movers and before later movers. [doc: adapter report sections 5-7]
- The query performs no writes and consumes no RNG. [doc: adapter report section 7; Infantry terminal report tiny-detail item 20]
- Unit policy preserves the verified tube/bridge, shroud, overlay, object/building, mission/alliance/crush/weapon, speed/land, and final occupancy phase order. [doc: `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` phases 1-12]
- The moving-allied deadlock branch compares
  `FacingClass::Current(blocker+0x388)` and
  `FacingClass::Current(self+0x388)`. Active Drive/Hover locomotors write this
  locomotor/body-facing timer; Rust `barrel_facing` corresponds to a distinct aim
  state and is not a substitute. [doc:
  `FOOTCLASS_0X388_LOCOMOTOR_FACING_GHIDRA_REPORT.md`]
- Final argument `1` keeps the locomotor `+0x1C` mode represented; all eleven audited stock locomotor vtables return zero there. No extra locomotor policy may be invented. [doc: adapter report section 4.3]
- Infantry wall classification compares the overlay-state upper nibble with `DamageLevels` using inequality; equality skips the wall branch. [doc: Infantry terminal report section 3]
- Infantry wall handling always checks primary weapon index zero and requires a non-null warhead with `Wall=yes`. [doc: Infantry terminal report section 3]
- Allied/own wall yields code `4`; hostile or owner-`-1` wall yields code `5`; failed action/weapon/warhead gates yield code `7`. [doc: Infantry terminal report section 3]
- Infantry functional fullness is exactly `(occupancy_byte & 0x1C) == 0x1C`; bits 0 and 1 do not participate. [doc: Infantry terminal report sections 4-5]
- Retained occupancy bit 5 returns code `2` only while the accumulated result is zero and before ownership handling. [doc: Infantry terminal report section 4]
- A slave-at-cell branch can clear the retained bit-5 value without clearing the packed occupancy byte. [doc: Infantry terminal report sections 2, 5]
- Enemy owner handling runs even with a free functional subcell: range `<= 0` yields `7`; positive range upgrades a result below `5` to `5`. [doc: Infantry terminal report section 4]
- Allied full occupancy with a prior result below `2` returns `6` only when stationary allied Infantry count equals exactly three; every other count returns `2`. [doc: Infantry terminal report sections 4.1-4.2]
- Stationary count follows selected-list Infantry occupants and the actual locomotor `Is_Moving` predicate, not total occupant count or path-target presence by assumption. [doc: Infantry terminal report section 4.1]
- Owner `-1` skips ownership handling; final zero/full becomes `7`, final zero/nonfull becomes `0`, and any existing nonzero result wins. [doc: Infantry terminal report section 4]
- The pre-terminal Infantry zero-speed rejection is gated by the selected object-list layer, so the rare bridge-object split bypasses the ground-layer speed rejection. [doc: Infantry terminal report section 4]
- Stock walls and `Wall=yes` warheads make code `4` active in standard YR; it is not a TS-legacy design input. [doc: Infantry terminal report section 6; `rulesmd.ini`, `artmd.ini`]
- Retry permits five total A* attempts, preserves ordered duplicate exclusions across retry reset, and clears them only at outer-search entry. [doc: failed-A* retry contract gamemd baseline]
- Failed attempts retain source-or-furthest accepted next-level0-zone progress; update uses that same progress cell for levels `0..2`. [doc: failed-A* retry contract rows T2/T7]
- If retry update invalidates hierarchy and budget remains, one following attempt runs without hierarchy; exhausted budget produces no sixth attempt. [doc: failed-A* retry contract rows T5/T12]

## Design

### Components

#### Native per-layer movement-occupation shadow

Each `CellOccupancy` owns one small state value per ground/bridge layer containing
only the native fields consumed by the retry policies:

- packed movement-occupation bits `0x1C` (Infantry subcells) and `0x20` (Unit);
- the last Infantry owner written on that layer, or the native `-1` equivalent.

The shadow is updated in the same `OccupancyGrid::add`/remove/subcell-transition
operation as the selected-list entry. `CellOccupant` retains enough category,
layer, and subcell information for removal to apply the native non-reference-counted
clear. Add receives the entrant owner so Infantry mark can overwrite the cache.

The state intentionally does not attempt to port unrelated CellClass bytes. It is
transient and rebuilt by replaying entities in `occupancy_enter_order`, which is the
Rust analogue of gamemd's post-load Unlimbo sequence.

#### `GameEntity::locomotor_facing`

One required `FacingClass` instance owns the active native `+0x388` role. It is
initialized from `facing << 8` with parsed `ROT`, retargeted or snapped through
central movement helpers, serialized, and hashed. Existing `facing: u8` remains a
compatibility/render projection of `locomotor_facing.current(binary_frame)` while
callers are migrated; `barrel_facing` remains the distinct aiming timer.

No caller may write the projected byte without updating the timer through the
same helper. This prevents the retry oracle and rendering/movement state from
silently diverging.

#### `RetryMoverFacts`

A small owned value built from the current mover immediately before the search. It contains only mover state used by the verified Unit/Infantry predicates: stable ID, runtime class, owner, type identity/handle, mission/target/contact/dock flags, crush/action capabilities, active locomotor facts, speed type, and weapon-selection inputs.

It does not copy unrelated render state, cargo collections, production state, or the world. Fields are added only when cited predicate branches consume them.

#### `RetryCellEntryOracle<'a>`

A stack-scoped read-only view containing `RetryMoverFacts` and borrowed references to:

- `EntityStore` for ordered occupant facts and cross-object state;
- `OccupancyGrid` for native ground/bridge list order plus the exact sampled
  movement-occupation bits and Infantry-owner cache;
- `ResolvedTerrainGrid` and current bridge/tube facts;
- `OverlayGrid` and `OverlayTypeRegistry` for live overlay ID/data and parsed flags;
- `RuleSet` for ObjectType, weapon, warhead, land/speed, and building policy;
- `FogState` for the mover owner's current reveal state;
- `HouseAllianceMap` and `StringInterner` for exact owner/alliance resolution.

The oracle exposes one crate-private operation: classify the retry candidate and direction and return a full native code `0..7`. Named internal constants document codes; no new runtime action/result hierarchy is introduced.

#### Unit and Infantry policies

Both policies live in the focused oracle module and share only proven-equivalent primitives: fixed-tuple layer derivation, ordered list access, alliance lookup, weapon/warhead resolution, and terrain/overlay access.

They do not share branch order merely because both return the same code type. Unit-only building exceptions stay in Unit; Infantry wall and terminal occupancy behavior stay in Infantry.

#### Retry query seam

The zone retry layer receives a read-only callable/reference accepting `(candidate, direction)` and returning the native code. This makes the pure retry kernel testable with injected result rows while keeping it independent of world storage.

#### Deferred path request

Any eligible path search currently invoked inside a mutable mover borrow is represented as a small pending request. The current mutable scope records the inputs and ends. The caller then constructs the oracle, performs the synchronous search, and reacquires the mover to apply the result in the same logical operation.

This follows the existing `DeferredCellCheck` and deferred drive-track-chain pattern. It must not delay the search to another tick or reorder another mover around it.

### Interfaces / Contracts

- `PathfindingContext` gains one optional exact retry query reference. Existing callers default to absent.
- `OccupancyGrid::add` receives the entrant's category/owner mark facts, and remove
  operations recover the stored mark facts before unlinking. List mutation and
  packed-state mutation are one operation.
- `GameEntity` exposes crate-private locomotor-facing snap, retarget, and current
  projection helpers. The oracle receives the current binary frame and reads the
  16-bit timer directly.
- Exact hierarchy retry is activated only when that reference is present.
- Standard YR movement paths that are eligible for the hierarchy retry must always provide it.
- When absent in headless/minimal callers, zone search retains its non-exact one-attempt behavior; it must not fabricate retry results from coarse grids.
- Oracle construction fails closed for missing required top-level sources: exact retry is not activated. Missing native subobjects such as weapon or warhead are classified by the verified policy rather than treated as construction errors.
- Result values are asserted to remain in `0..7`; the retry producer owns zero/nonzero and matrix polarity.

### Data Flow

```text
Simulation::advance_tick
  -> tick_movement_with_grids
      -> live-order mover reaches synchronous path request
          -> release mutable mover borrow
          -> RetryMoverFacts::from current mover
          -> RetryCellEntryOracle borrows current world sources
          -> find_move_path(PathfindingContext + exact query)
              -> zone_search eligible hierarchy attempt
                  -> A* success: return path
                  -> A* failure: retain progress
                  -> zone_retry probes query(candidate, direction)
                      -> oracle derives fixed tuple/layers
                      -> reads stored ground occupation bits/owner
                      -> samples mover/blocker locomotor-facing timers at this frame
                      -> Unit or Infantry exact policy
                      -> full code 0..7
                  -> zone_retry applies code/matrix rule and exclusions
                  -> bounded next attempt
          -> drop oracle/shared borrows
          -> reacquire mover and commit result
```

### Error Handling

- Out-of-bounds or invalid candidate state returns the verified class result; it does not panic.
- Missing weapon, warhead, target, contact, or optional object state follows native null/default branches.
- Missing `RuleSet`, overlay registry, fog, or other required construction source disables exact retry for that call and emits no approximate result.
- Debug assertions catch codes outside `0..7`, layer mismatches, and attempts to retain the oracle beyond the synchronous call.
- No gameplay mutation is used as error recovery.

### Testing Strategy

Testing is divided into three layers:

1. Occupancy lifecycle fixtures prove native mark/unmark owner retention, Unit
   non-reference-counted bit clearing, Infantry self-bit retention, and rebuild order.
2. Locomotor-facing fixtures prove Unlimbo-style initialization, same-frame
   retarget sampling, full 16-bit equality, and snapshot/hash coverage.
3. Pure oracle-policy fixtures cover the Unit phases and Infantry wall/terminal ledger, including exact code values and ordered occupant permutations.
4. Pure zone-retry fixtures inject code rows to prove five-attempt lifecycle, progress handoff, polarity, exclusion order, duplicates, and invalid-hierarchy fallback.
5. Movement integration fixtures prove oracle construction at the live-order search point, same-tick visibility of earlier mover changes, no state/RNG mutation, and no approximate fallback.

The 22 acceptance tests in the implementation contract are mandatory. Tests use repo INI-backed registries for wall/weapon data rather than hardcoded stock names. Rust-vs-Rust fixtures are regression evidence, not gamemd parity certification; final cutover still needs a gamemd-derived executable comparison or exhaustive proof.

## Architectural Decisions

- **Borrow current state instead of cloning it.** This preserves same-tick semantics and the 20,000-unit scale target without a full-world allocation per search.
- **Own only mover facts.** This crosses the mutable mover boundary without creating a general snapshot framework.
- **Keep query and retry lifecycle separate.** Cell-entry policy owns world interpretation; zone retry owns hierarchy attempts and exclusions.
- **Keep native cell history with the cell owner.** `OccupancyGrid` owns the
  transient movement-occupation bits and Infantry-owner cache because current
  occupants cannot reconstruct native mark/unmark history.
- **Keep object-list and packed-state self handling distinct.** Unit clears its
  sampled bit locally; Infantry retains its marked subcell bit while skipping its
  object pointer.
- **Model `+0x388` as locomotor-facing, not turret aim.** Reuse the verified
  `FacingClass` primitive in a distinct durable `GameEntity` field; do not reuse
  `barrel_facing` or an 8-bit projection.
- **Reuse ordered occupancy.** `OccupancyGrid` remains the CellClass-list analogue;
  EntityStore order is never substituted during runtime scans, while
  `occupancy_enter_order` is used only for native-style post-load rebuild.
- **Keep the exact feature optional at low-level APIs but mandatory in standard production wiring.** This preserves focused tests and compatibility callers without authorizing an approximate retry.
- **Serialize only genuinely durable state.** Oracle/retry scratch and CellClass
  shadow remain outside serialization/hash surfaces. The `+0x388`-equivalent
  entity timer is serialized and hashed, requiring one snapshot-version bump.
- **One contained borrow-flow adjustment.** Existing deferred-work patterns are extended only where an eligible search currently occurs inside a mutable mover scope.

No intentional parity debt is introduced. Stale parent research wording about Infantry code `4` and retry polarity is a documentation follow-up, not a design input.

## Alternatives Considered

### Full owned world snapshot per search

This would copy all potentially queried cells and entity facts before every path search. It avoids borrow restructuring and can preserve semantics, but scales poorly, duplicates authoritative data, and adds allocation/copy cost proportional to the world rather than probed cells. Rejected for the 20,000-unit target.

### Generation-cached snapshot

This would reuse a snapshot until tracked generations change. Exact invalidation would require generations for occupancy, missions, targets, contacts, locomotor motion, fog, overlays, alliances, weapons, and building state. Current occupancy generation alone is insufficient, so this approach creates broad hidden coupling and same-tick drift risk. Rejected.

### Pass world systems directly into zone retry

This would let hierarchy code inspect entities, rules, and overlays. It violates subsystem ownership, makes pure retry tests difficult, and couples zone algorithms to gameplay storage. Rejected in favor of the narrow code-query seam.

### Extend the approximate `cell_entry.rs` classifier

The current classifier returns runtime action payloads and has different ordering and approximation boundaries. Adding retry-specific exact semantics there would mix two contracts, grow an already large file, and risk accidentally activating exact-looking partial behavior elsewhere. Rejected in favor of the approved focused module.

### Derive packed state and owner from current occupants

This is superficially smaller but cannot reproduce native state. Infantry owner is
overwritten on mark and retained when one of several Infantry leaves; Unit and
Infantry occupation bits are cleared without reference counting. It also drops the
mover's Infantry subcell bit when self is skipped. Rejected as confirmed drift.

### Reuse `barrel_facing` or compare only `facing: u8`

Live Ghidra proves active locomotors write native `+0x388`, while Unit aiming uses
distinct `+0x3A0` storage. Comparing only the high byte also collapses different
mid-rotation 16-bit values. Rejected as confirmed drift.

### Port a broad CellClass substrate and all three native facing fields

This could support future systems, but the retry consumes only selected-list order,
movement-occupation bits/owner, and the `+0x388` locomotor-facing timer. Porting
unrelated CellClass bytes and neighboring facing roles now expands risk without
closing an additional requirement in this task. Rejected in favor of the narrow
state additions above.
