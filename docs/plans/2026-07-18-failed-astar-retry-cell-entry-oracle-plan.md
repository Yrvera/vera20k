# Failed-A* Retry Cell-Entry Oracle Implementation Plan

> **For Codex:** Execute this plan task-by-task. Each task is self-contained. Use the smallest sufficient Rust-native solution that preserves the cited active `gamemd.exe` semantics.

**Goal:** Add a search-local exact Unit/Infantry retry cell-entry oracle and use it to activate the verified five-attempt flat hierarchy retry lifecycle without cloning the world or reusing approximate path-grid legality.

**Architecture:** `OccupancyGrid` first gains the transient native movement-occupation
bits/Infantry-owner state that CellClass policy reads, while `GameEntity` gains the
durable timer-backed locomotor-facing state corresponding to native `+0x388`.
`retry_cell_entry.rs` then interprets those authoritative sources and returns the
full native code `0..7` through a narrow read-only trait. `zone_retry.rs` owns only
the pure hierarchy retry producer. Movement constructs the borrowed oracle at the
synchronous search boundary after earlier live-order mutations and outside mutable
mover borrows.

**Design Doc:** `docs/plans/2026-07-18-failed-astar-retry-cell-entry-oracle-design.md`

---

## Grounding Summary

- The active retry entry is `AStar_pathfind_search @ 0x0042C900`: it clears exclusions once, permits five total A* attempts, updates hierarchy edges after failed hierarchical attempts, and may spend the next remaining attempt without hierarchy when update invalidates it.
- `ZoneMap::FloodFillReachableZones @ 0x005840C0` is a retry-local `2/4/8` block helper, not a persistent zone rebuild. Its corrected bookkeeping branch is `cell_entry_code == 0 || passability_matrix_value != 1`.
- On helper return `1`, `InvalidateZoneEdge @ 0x0042CF80` selects one adjacent stored-path edge, appends it first, then appends asymmetric common-neighbor edges in reverse adjacency encounter order. On return `0`, the caller appends current-zone edges to graph neighbors absent from the locally observed vector.
- `UnitClass::Can_Enter_Cell @ 0x0073F0A0` and `InfantryClass::Can_Enter_Cell @ 0x0051BF90` are the active runtime-class policies. The retry calls them with candidate cell, direction `0..7`, signed candidate `Level`, null parent, and final flag `1`.
- The retry consumer needs the full native code even though the flood branch subsequently reduces it to zero/nonzero. MovementZone and locomotor kind must not replace Unit-versus-Infantry dispatch.
- The fixed retry tuple ordinarily reads ground terrain, occupancy bits, and owner. The verified null-parent diff-4 bridgehead case can select the bridge object list while terminal occupancy bits and owner remain ground.
- `OccupancyGrid::iter_layer` already preserves CellClass-style list order reconstructed from `(occupancy_enter_order, stable_id)`. It is the ordered object source; `EntityStore` key order is not.
- Current occupants cannot reconstruct the native movement-occupation state. Infantry mark overwrites the per-layer owner and unmark retains that owner while any `0x1C` subcell bit remains; Unit and Infantry bits clear without reference counting. The mover's Infantry subcell bit also remains sampled when self is skipped from the object scan.
- gamemd does not serialize these CellClass attributes. Post-load object Unlimbo order rebuilds them, matching Rust's persisted `occupancy_enter_order`; the new per-layer shadow therefore belongs in transient `OccupancyGrid`, not snapshot state.
- Fresh live Ghidra corrected a stale field label: active Drive/Hover locomotors and `TechnoClass::Unlimbo` write object `+0x388`, and `UnitClass::Can_Enter_Cell` compares `Current(+0x388)` for mover/blocker. It is the locomotor/body-facing timer for this consumer, distinct from the `+0x3A0` aim timer represented by Rust `barrel_facing`.
- Rust already has the exact timer primitive in `FacingClass`, but body `facing: u8`/`facing_target` currently lose the 16-bit animated state. A distinct serialized/hashed `locomotor_facing` instance is required before the deadlock branch can be exact.
- `HierarchyProgressTracker` already records source-or-furthest-next-zone progress, but `find_path_with_costs_hierarchy_marker_progress` currently discards it on failure through `?`.
- `ZonePrecheckExclusions` already has set-like membership plus an ordered duplicate-preserving producer ledger. It must remain search-local and survive retry resets.
- The exact isometric playfield predicate already exists in `cell_rect.rs`; expose a crate-private single-cell wrapper rather than substituting rectangular `PathGrid` bounds.
- Standard movement currently performs some path searches while holding `&mut GameEntity`. The existing deferred cell/drive-track pattern is the model for recording a small request, ending the borrow, searching synchronously, and then committing the result in the same logical operation.
- Stock YR data proves the wall path is live: `rulesmd.ini` has `[GAWALL] Wall=yes`, `[NAWALL] Wall=yes`, `[GASAND] Wall=yes`, and `[PrismWarhead] Wall=yes`; `artmd.ini` has `DamageLevels=3`, `3`, and `2` respectively.
- The needed wall, overlay, weapon, and warhead fields are already parsed. No parser task or hardcoded wall/type-name table is required.
- Research-index validation was clean for the focused pathfinding corpus. The older broad `PATHFINDING_ASTAR_GHIDRA_REPORT.md` and older positive flood-expansion prose are superseded for retry polarity.
- No TS-only mechanism is being activated: both class policies, the hierarchy retry, stock wall overlays, and wall-capable warheads are active in standard Yuri's Revenge.

## Key Technical Decisions

- **Use a crate-private query trait, not a world-shaped parameter bundle:** `RetryCellEntryQuery::classify_retry(candidate, direction) -> u8` keeps zone code independent of entities/rules and permits injected pure tests. **Confidence: high.**
  - **Source:** approved design; adapter report; current `zone_search.rs` boundary.
- **Borrow authoritative state for one synchronous search and own only mover facts:** this prevents a map/world clone while Rust's immutable borrow guarantees no state changes during a query sequence. **Confidence: high.**
  - **Source:** approved design; current movement deferred-borrow pattern.
- **Keep exact retry optional in low-level callers but mandatory for eligible standard movement:** absence retains the existing one-attempt hierarchy behavior and never fabricates retry classifications from `PathGrid` or `LayeredEntityBlockMap`. **Confidence: high.**
  - **Source:** approved design interfaces; oracle contract production gate.
- **Expose the existing diamond predicate:** `zone_retry` uses `PlayfieldBounds` plus `ResolvedTerrainGrid` and does not invent another bounds formula. **Confidence: high.**
  - **Source:** `cell_rect.rs:466-510`; `MapClass::Is_Cell_In_Playfield @ 0x00578460`.
- **Return progress on both A* success and failure:** replace the success-only result with `path: Option<Vec<_>>` while retaining progress cell/index. **Confidence: high.**
  - **Source:** retry contract T2/T3; `core.rs:2361-2406`.
- **Preserve inverse retry-flood bookkeeping polarity exactly:** bookkeeping runs for code zero or matrix value other than one; nonzero plus matrix one skips it. **Confidence: high.**
  - **Source:** live assembly `0x00584271..0x00584286`; corrected producer report.
- **Store native movement-occupation history in `OccupancyGrid`:** each layer owns
  bits `0x1C/0x20` plus the last-marked Infantry owner, updated atomically with list
  insertion/removal and rebuilt in `occupancy_enter_order`. Do not fold it from the
  current list. **Confidence: high.**
  - **Source:** `InfantryClass::MarkCellOccupancy @ 0x005217C0`, unmark
    `0x00521850`, Unit mark/clear `0x007441B0/0x00744210`,
    `CELLCLASS_MAPCLASS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` section 8 item 14.
- **Model native `+0x388` with a distinct `FacingClass`:** the oracle samples the
  full 16-bit locomotor-facing value at the shared binary frame; it never reuses
  `barrel_facing` or only `facing: u8`. **Confidence: high.**
  - **Source:** `FOOTCLASS_0X388_LOCOMOTOR_FACING_GHIDRA_REPORT.md`; live
    `DriveLocomotionClass::Do_Turn @ 0x004B0EF0`, Unlimbo `0x006F6CA0`, and
    Unit deadlock reads `0x0073F8EB/0x0073F906`.
- **Serialize only the entity timer:** attempt scratch, oracle references, and
  CellClass shadow remain transient; `locomotor_facing` is durable entity state and
  requires one snapshot-version bump plus hash coverage. **Confidence: high.**
  - **Source:** native Unlimbo reconstruction; current `snapshot.rs` and
    `world_hash.rs` entity-state patterns.

## Open Questions

### Resolved During Planning

- **Does the helper use normal passable-neighbor polarity?** No. The newest direct assembly check proves local bookkeeping on `code == 0 || matrix != 1`; the older opposite prose is stale.
- **Does retry dispatch by locomotor or MovementZone?** No. MovementZone supplies only the matrix row; runtime Unit/Infantry class supplies the virtual policy.
- **Can retry select bridge objects while using bridge terminal occupancy?** It can select bridge objects in the rare diff-4 bridgehead reconstruction, but the fixed candidate-self height keeps terminal occupancy bits and owner ground.
- **Is rectangular grid containment sufficient for the helper scan?** No. The repository already has the exact isometric diamond test and current `Simulation::playfield_bounds`; thread those values.
- **Are new INI parsers needed?** No. Overlay `wall/crate_type/damage_levels`, weapon `warhead/range`, and warhead `wall` are already parsed.
- **Can the exact producer run without an oracle?** No. Low-level callers may retain one attempt, but the five-attempt producer is activated only with a complete query.
- **Do command-time searches need this wiring?** Current `movement_commands.rs` passes no blocker-neighbor counts, so it cannot enter the eligible hierarchy branch. Its query remains absent without changing behavior.
- **Do queued drive-arrival searches need this wiring?** `process_pending_drive_arrivals` passes `zone_grid: None`; it remains outside the exact hierarchy retry.
- **Can current occupants reconstruct the sampled owner/byte?** No. Mark/unmark
  history produces reachable states that current-list folding cannot reproduce;
  store the native shadow transiently.
- **Is native `+0x388` Rust `barrel_facing`?** No. Live locomotor and constructor
  disassembly proves distinct storage and writer roles; add `locomotor_facing`.
- **Does the CellClass shadow need snapshot serialization?** No. gamemd rebuilds it
  through post-load Unlimbo order, and Rust already persists the equivalent
  `occupancy_enter_order`.

### Deferred to Implementation Verification

- **Which existing Unit policy facts are not yet represented by `GameEntity`/`ObjectType`?** Task 6 begins with a compile-time fact inventory against every cited phase. A missing active input is a hard stop for that phase, not a default value.
- **What route changes occur on retail maps?** Pure tests prove mechanisms only. The final fidelity task must capture a gamemd-derived retry case before claiming parity.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/sim/occupancy.rs` | Own per-layer native movement bits/Infantry owner and update them atomically with ordered list membership. |
| Modify | `src/sim/game_entity.rs` | Add durable `locomotor_facing: FacingClass`, distinct from aim `barrel_facing`. |
| Modify | `src/sim/movement/facing_class.rs` | Expose only the exact projection/completion helpers needed by body-facing owners. |
| Create | `src/sim/pathfinding/retry_cell_entry.rs` | Borrowed world view, owned mover facts, fixed retry tuple/layer derivation, and exact Unit/Infantry policies. |
| Create | `src/sim/pathfinding/zone_retry.rs` | Pure `2/4/8` block producer, stored-path invalidation, exclusion updates, and retry constants/types. |
| Modify | `src/sim/pathfinding/mod.rs` | Declare the two focused modules and expose only crate-private seams. |
| Modify | `src/sim/pathfinding/core.rs` | Preserve hierarchy progress on failed A* attempts. |
| Modify | `src/sim/pathfinding/zone_hierarchy.rs` | Add read-only edge-key accessors needed by tests; retain ordered duplicate exclusions. |
| Modify | `src/sim/pathfinding/zone_search.rs` | Coordinate the bounded exact loop in the eligible flat hierarchy branch. |
| Modify | `src/sim/cell_rect.rs` | Expose the existing exact single-cell playfield predicate crate-privately. |
| Modify | `src/sim/movement/mod.rs` | Extend `PathfindingContext`; define small owned pending-search/result types. |
| Modify | `src/sim/movement/movement_commands.rs` | Initialize absent retry context for command-time searches and retarget locomotor-facing state through the native timer helper. |
| Modify | `src/sim/movement/movement_path.rs` | Thread the query/bounds to flat zone search and split search calculation from target mutation. |
| Modify | `src/sim/movement/movement_blocked.rs` | Prepare blocked-repath requests and apply completed results without searching inside a mover borrow. |
| Modify | `src/sim/movement/movement_step.rs` | Bubble a pending blocked search out of the cell-crossing mutable borrow. |
| Modify | `src/sim/movement/movement_occupancy.rs` | Return pending blocked searches from deferred occupancy handling. |
| Modify | `src/sim/movement/movement_tick.rs` | Construct/drop the oracle at each eligible synchronous search point and preserve live-order timing. |
| Modify | `src/sim/world/mod.rs` | Pass existing fog, overlay, registry, and playfield sources into movement. Preserve unrelated dirty animation/substrate work. |
| Modify | `src/sim/world/world_spawn.rs` | Initialize locomotor-facing from spawn facing and parsed `ROT`. |
| Modify | `src/sim/world/world_hash.rs` | Hash the full locomotor-facing timer state. |
| Modify | `src/sim/snapshot.rs` | Advance `SNAPSHOT_VERSION` once and verify round-trip preservation/rebuild behavior. |
| Modify | `src/sim/pathfinding/core_tests.rs` | Update changed pathfinding signatures and keep focused retry fixtures compiling. |
| Modify | `src/sim/movement/prone_speed_tests.rs` | Update movement entry-point call signature. |
| Modify | `src/sim/movement/movement_tests.rs` | Update movement entry-point call signatures and add body-facing/retry integration coverage. |

The two new pathfinding files prevent further growth in `core.rs`, `zone_search.rs`, `zone_hierarchy.rs`, and `cell_entry.rs`, all of which are already near or above the project's normal split threshold. The movement files stay within their existing responsibilities; no gameplay logic moves into `world/mod.rs`.

## Interface Changes

- New crate-private trait:

```rust
pub(crate) trait RetryCellEntryQuery {
    fn classify_retry(&self, candidate: (u16, u16), direction: u8) -> u8;
}
```

- `PathfindingContext<'a>` gains:

```rust
pub retry_cell_entry: Option<&'a dyn RetryCellEntryQuery>,
pub playfield_bounds: Option<PlayfieldBounds>,
```

Every existing constructor receives explicit `None` values. Standard eligible movement replaces them only for the duration of an exact search.

- `find_path_zoned_marker` and its private inner function gain the same query and playfield inputs. Layered, non-hierarchy, compatibility, command-time, and headless paths do not consume them.
- `find_path_with_costs_hierarchy_marker_progress` changes from `Option<HierarchyMarkerPathResult>` to an always-returned `HierarchyMarkerAttemptOutcome { path: Option<Vec<_>>, progress_cell, progress_index }`.
- `cell_in_playfield_diamond` remains the single implementation; a crate-private `is_cell_in_playfield` wrapper exposes it to `zone_retry`.
- Movement blocked/exhaustion helpers return small owned requests/results rather than invoking path search while a mover is mutably borrowed. These are module-private, un-serialized, and un-hashed.
- `tick_movement_with_grids` gains read-only `FogState`, `OverlayGrid`, `OverlayTypeRegistry`, `PlayfieldBounds`, house-state, and pre-increment `binary_frame` inputs. Legacy wrappers pass absence/empty data and therefore cannot activate exact retry.
- `CellOccupant` retains category and mark-time subcell facts. `OccupancyGrid::add`
  also receives the entrant owner; remove recovers the stored facts before unlinking
  and applies the non-reference-counted native clear.
- `CellOccupancy` exposes a read-only per-layer
  `movement_occupation(layer) -> NativeMovementOccupation` containing bits and
  Infantry owner.
- `GameEntity` adds `locomotor_facing: FacingClass` plus crate-private snap,
  retarget, and `current_u8(binary_frame)` helpers. The existing `facing` byte is a
  compatibility projection, not oracle authority.

## Sim Checklist

- [ ] No new `f32`/`f64` appears in retry, oracle, or movement logic; all existing terrain/weapon values retain their parsed integer/fixed representations.
- [ ] `locomotor_facing` is serialized, hashed, and covered by the single coordinated snapshot-version bump; transient occupancy shadow is rebuilt, not serialized.
- [ ] No dependency from `sim/` to `render/`, `ui/`, `sidebar/`, `audio/`, or `net/` is added.
- [ ] Tick order remains command application → object AI → live-order movement → vision refresh. Searches see prior-tick fog and all earlier same-tick movement mutations, matching their existing call point.
- [ ] `EntityStore` key order is never used as CellClass object order; oracle scans `OccupancyGrid::iter_layer` order.
- [ ] Oracle reads stored movement bits/owner and full 16-bit locomotor-facing timers; it never refolds current occupants or substitutes `barrel_facing`/high-byte facing.
- [ ] Retry directions are exactly `0..7` in the existing gamemd direction table order.
- [ ] Query, visited scratch, observed zones, attempt count, paths, markers, and exclusions are scoped to one synchronous path call.
- [ ] Oracle classification consumes no RNG and mutates no entity, occupancy, overlay, fog, rule, or pathfinding state.

## Risk Areas

- **Unit policy breadth:** `UnitClass::Can_Enter_Cell` has many early returns and state-dependent building/mission branches. Each phase must land with its fixtures before production wiring.
- **Occupancy lifecycle blast radius:** every production add/remove/subcell path must
  update ordered list and native shadow together. Missing one path creates stale
  retry results; caller enumeration and debug rebuild assertions are mandatory.
- **Body-facing migration:** current `facing`/`facing_target` writers are spread across
  movement commands, drive tracks, deployment, and spawn. Central helpers plus a
  same-frame full-16-bit fixture prevent timer/projection divergence.
- **Snapshot coordination:** `SNAPSHOT_VERSION` must be bumped only after confirming
  no other session owns a rebaseline; do not overwrite an independently advanced
  version.
- **Borrow/timing refactor:** blocked and segment repaths currently occur inside mutable mover paths. Tests must prove same-tick request/commit order, timer changes, debug events, and no extra movement step.
- **Corrected flood polarity:** two research documents contain historical opposite prose. Tests must name the inverse bookkeeping rule so future cleanup cannot flip it.
- **Playfield shape:** rectangular bounds would alter block scans near the isometric map edge. Only the existing diamond function is permitted.
- **Retry budget transitions:** invalidating hierarchy with budget left runs one unrestricted attempt; invalidating on attempt five produces no sixth attempt.
- **Dirty worktree collision:** `movement_tick.rs`, `occupancy.rs`, `game_entity.rs`, and `world/mod.rs` already contain unrelated edits. Re-read each immediately before patching, preserve those hunks, format only edited files, and never revert unrelated work.
- **Hot path:** do not clone `EntityStore`, `RuleSet`, grids, fog, or occupancy. Reuse fixed `[u8; 64]` visited scratch and small per-level `Vec<ZoneId>` buffers inside one failed search only.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|---|---|---|---|
| 0A | Native mark/unmark bits and Infantry owner retention | Current-list folding loses self bits, non-reference-counted clears, and last-marker owner history. | `0x005217C0/0x00521850`, `0x007441B0/0x00744210`; lifecycle fixtures. |
| 0B | Full 16-bit locomotor-facing timer at native `+0x388` | Same high byte can hide unequal native animated values and change moving-ally deadlock results. | `0x004B0EF0`, `0x006F6CA0`, `0x0073F8EB/0x0073F906`; same-high-byte fixture. |
| 2 | Failure progress survives a failed A* | Retry exclusions use source-or-furthest accepted next-zone cell, not the last popped node. | Contract T2/T3 plus focused `core.rs` tests. |
| 3 | `2/4/8` block math and fixed stride `8` | A one-cell visited-index difference selects different exclusions. | `0x005840C0`; per-level mask fixtures. |
| 3 | Inverse flood bookkeeping polarity | Flipping zero/nonzero changes the producer branch and subsequent route. | Assembly `0x00584271..0x00584286`; T14. |
| 3 | Local observed vector and reverse graph scans | Output is the complement of observed graph neighbors, not the observed vector. | Producer report; T8. |
| 3 | Direct/common exclusion order and duplicates | Precheck consumes edge identity while exact producer order/multiplicity persists. | `0x0042CF80`; T9-T11. |
| 4 | Five total attempts and invalid-hierarchy fallback | Off-by-one behavior creates a sixth route attempt or suppresses the allowed fallback. | `0x0042C900`; T4-T6/T12. |
| 5 | Fixed retry tuple, stored occupancy, and split layers | Wrong height/parent/layer reads or refolded bits/owner change bridgehead and terminal occupancy results. | Adapter report; oracle T3/T4 plus Task 0A state fixtures. |
| 5 | Native object-list order | Reversing two occupants can change an early return. | Occupancy order fixture T5/T6. |
| 6 | Unit phases and accumulator precedence | Codes `0..7` drive attack, wait, scatter, crush, and hard-block behavior. | `0x0073F0A0`; T7-T10. |
| 6 | Player-control and moving-ally frame phase | Crate gates and deadlock-yield decisions change on these exact inputs. | Unit report phases 9-10; player/nonplayer and frame-octant fixtures. |
| 7 | Infantry wall state nibble, weapon/warhead, and alliance | Stock walls produce distinct allied `4`, hostile `5`, or hard-block `7`. | `0x0051C17C..0x0051C225`; stock INI T11-T14. |
| 7 | Infantry exact `0x1C`, bit 5, range, and stationary count `==3` | Free-subcell and full-cell outcomes differ from the current shortcut. | `0x0051C78B..0x0051C880`; T15-T20. |
| 8 | Search occurs after earlier live-order mutations but before later mover commits | A one-tick visibility/occupancy shift changes routes and RNG-independent state. | Movement integration T1/T2/T21. |
| 9 | No approximate fallback | A coarse grid result must never masquerade as exact retry input. | Integration T22 and absent-query regression. |

---

## Tasks

### Task 0A: Add the native per-layer movement-occupation shadow

**Why:** The oracle cannot reconstruct native CellClass bits or Infantry owner
history from the current object list. This state foundation must exist before any
Unit/Infantry policy is implemented.

**Files:**
- Modify: `src/sim/occupancy.rs`
- Modify production Unit/Infantry occupancy call sites in:
  `src/sim/world/mod.rs`, `src/sim/world/world_spawn.rs`,
  `src/sim/world/bridge_orchestrator.rs`, `src/sim/movement/movement_tick.rs`,
  `src/sim/movement/movement_step.rs`, `src/sim/movement/movement_occupancy.rs`,
  `src/sim/movement/tube_movement.rs`, `src/sim/movement/tunnel_movement.rs`,
  `src/sim/movement/teleport_movement.rs`, `src/sim/passenger.rs`,
  `src/sim/production/production_sell.rs`, `src/sim/aircraft/drop_payload.rs`,
  `src/sim/superweapon/genetic_converter.rs`, and
  `src/sim/superweapon/lightning_storm.rs`.

**Pattern:** Keep `OccupancyGrid` as the Rust-native owner, but co-locate native
mark/unmark shadow writes with its existing ordered-list mutation, matching the
`AddContent -> Mark_Occupation` / `RemoveContent -> Clear_Occupation` lifecycle.

**Step 1: Define only the state consumed by this task.**

```rust
const INFANTRY_FUNCTIONAL_BITS: u8 = 0x1c;
const UNIT_OCCUPATION_BIT: u8 = 0x20;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub(crate) struct NativeMovementOccupation {
    pub(crate) bits: u8,
    pub(crate) infantry_owner: Option<InternedId>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum NativeOccupationMark {
    None,
    Unit,
    Infantry { sub_cell: u8, owner: InternedId },
}
```

Add `ground_movement` and `bridge_movement` to `CellOccupancy`. Add
`native_mark: NativeOccupationMark` to `CellOccupant`; this captures mark-time
owner and subcell state so owner changes do not rewrite the cell cache retroactively.
Do not add unrelated CellClass flags.

**Step 2: Implement literal mark/unmark helpers.**

```rust
impl NativeMovementOccupation {
    fn mark(&mut self, mark: NativeOccupationMark) {
        match mark {
            NativeOccupationMark::None => {}
            NativeOccupationMark::Unit => self.bits |= UNIT_OCCUPATION_BIT,
            NativeOccupationMark::Infantry { sub_cell: 2..=4, owner } => {
                self.bits |= 1u8 << sub_cell;
                self.infantry_owner = Some(owner);
            }
            NativeOccupationMark::Infantry { .. } => {
                debug_assert!(false, "native Infantry subcell must be 2..=4");
            }
        }
    }

    fn unmark(&mut self, mark: NativeOccupationMark) {
        match mark {
            NativeOccupationMark::None => {}
            NativeOccupationMark::Unit => self.bits &= !UNIT_OCCUPATION_BIT,
            NativeOccupationMark::Infantry { sub_cell: 2..=4, .. } => {
                self.bits &= !(1u8 << sub_cell);
                if self.bits & INFANTRY_FUNCTIONAL_BITS == 0 {
                    self.infantry_owner = None;
                }
            }
            NativeOccupationMark::Infantry { .. } => {
                debug_assert!(false, "native Infantry subcell must be 2..=4");
            }
        }
    }
}
```

These operations are intentionally non-reference-counted. Two objects sharing one
native bit followed by one unmark leaves the bit clear, matching gamemd.

**Step 3: Make list and shadow updates atomic.** Add
`OccupancyGrid::add_with_native_mark(...)`; keep the existing `add(...)` as an
unmarked convenience for non-Techno/test-only occupants. `remove`,
`remove_on_layer`, `move_entity[_layered]`, and `update_sub_cell` must recover the
stored mark before unlinking, unmark the old layer, then mark the new layer/subcell
in native order. Expose only:

```rust
pub(crate) fn movement_occupation(
    &self,
    rx: u16,
    ry: u16,
    layer: MovementLayer,
) -> NativeMovementOccupation;
```

Do not delete a `CellOccupancy` until both its list and both native layer states are
empty/default.

**Step 4: Rebuild through native mark order.** `OccupancyGrid::rebuild` already sorts
by `(occupancy_enter_order, stable_id)`. For each surviving entity, construct
`NativeOccupationMark::Unit` or `Infantry { sub_cell, owner }` and call the marked
add. This reproduces gamemd post-load Unlimbo reconstruction; do not serialize the
cell shadow. Keep `debug_assert_matches` list comparison and add an explicit shadow
comparison only for freshly rebuilt grids—runtime non-reference-counted collisions
need not equal a fold of surviving entities.

**Step 5: Switch every production Unit/Infantry call site.** At each file listed
above, form the mark from the actual entity before mutation. Terrain, structures,
and presentation-only occupants may keep unmarked add only when the retry never
consumes a Unit/Infantry mark from that path. Re-run:

```powershell
rg -n "occupancy\.(add|remove|remove_on_layer|move_entity|move_entity_layered|update_sub_cell)" src/sim
```

Classify every result in the task notes; no production Unit/Infantry path may remain
on unmarked add.

**Step 6: Add exact lifecycle fixtures in `occupancy.rs`.** Cover:

- Infantry mover plus subcells `2/3/4`: sampled bits include the mover's bit;
- two allied Infantry marks followed by removal of the last marker: retained owner
  remains the departed marker's owner while a functional bit remains;
- owner change while occupied does not rewrite mark-time owner;
- two Units sharing bit `0x20`, then one removal: bit clears despite one list entry;
- two Infantry sharing one subcell, then one removal: that subcell bit clears;
- final Infantry removal clears owner to `None`;
- ground/bridge state remains independent;
- rebuild replays owner overwrites in `occupancy_enter_order`.

**Step 7: Verify serially.**

```powershell
cargo test occupancy_native_movement_ -- --nocapture
cargo test occupancy -- --nocapture
```

Expected: each command prints a passing literal `test result:` line.

### Task 0B: Add the authoritative native `+0x388` locomotor-facing timer

**Why:** Unit moving-ally deadlock compares full animated 16-bit values. Rust's
8-bit `facing` and separate `barrel_facing` cannot produce that result.

**Files:**
- Modify: `src/sim/game_entity.rs`
- Modify: `src/sim/movement/facing_class.rs`
- Modify body-facing writers in `src/sim/movement/movement_commands.rs`,
  `movement_path.rs`, `movement_step.rs`, `movement_tick.rs`, `air_movement.rs`,
  `rocket_movement.rs`, `tube_movement.rs`, and `tunnel_movement.rs`
- Modify non-movement writers in `src/sim/combat/combat_targeting.rs`,
  `src/sim/miner/miner_dock_sequence.rs`, `src/sim/docking/bunker_link.rs`,
  `src/sim/docking/bunker_install.rs`, `src/sim/world/world_commands.rs`, and
  `src/sim/world/world_spawn.rs`
- Modify: `src/sim/world/world_hash.rs`
- Modify: `src/sim/snapshot.rs`
- Modify affected fixtures in `src/sim/movement/movement_tests.rs` and existing
  module-local tests beside each writer

**Pattern:** Reuse the existing verified `FacingClass` primitive. Keep the 8-bit
field only as a compatibility projection while central helpers own every snap,
retarget, and current-frame projection.

**Step 1: Add the durable field and helpers.**

```rust
pub locomotor_facing: crate::sim::movement::FacingClass,

impl GameEntity {
    pub(crate) fn snap_locomotor_facing(&mut self, facing: u8, frame: u32) {
        self.locomotor_facing.snap(u16::from(facing) << 8, frame);
        self.facing = facing;
        self.facing_target = None;
    }

    pub(crate) fn retarget_locomotor_facing(&mut self, facing: u8, frame: u32) {
        self.locomotor_facing.set(u16::from(facing) << 8, frame);
        self.facing_target = Some(facing);
        self.facing = (self.locomotor_facing.current(frame) >> 8) as u8;
    }

    pub(crate) fn project_locomotor_facing(&mut self, frame: u32) {
        self.facing = (self.locomotor_facing.current(frame) >> 8) as u8;
        if !self.locomotor_facing.is_rotating(frame) {
            self.facing_target = None;
        }
    }
}
```

`GameEntity::new` initializes the timer at `facing << 8` with ROT zero. Spawn/type
initialization immediately applies parsed `ROT`; this mirrors native Unlimbo snap
followed by Unit constructor `SetROT`. `barrel_facing` remains untouched and
distinct.

**Step 2: Replace per-tick byte stepping.** `handle_vehicle_rotation` receives
`&mut GameEntity` or `&mut FacingClass` plus `binary_frame`; it retargets once and
projects `current(binary_frame)` rather than using `rot_to_facing_delta`. The
same-frame call must observe elapsed zero. Infantry instant turns use snap. Drive
track and each active locomotor writer listed above must choose `set` versus `snap`
from the existing native role: normal turn requests use `set`; reveal/teleport or
explicit instant-placement paths use `snap`.

**Step 3: Exhaustively eliminate unsynchronized production writes.** Run:

```powershell
rg -n "(entity|entity_mut|ge|unit|u)\.facing(_target)?\s*=|\*facing(_target)?\s*=" src/sim --glob '!**/*tests.rs'
```

Each result must either call a locomotor-facing helper or contain a cited reason it
is not a Techno/body-facing writer. Do not leave direct Unit/Infantry projection
writes.

**Step 4: Hash and snapshot it.** Hash `entity.locomotor_facing` beside
`facing/facing_target`. Coordinate ownership of `SNAPSHOT_VERSION`; if it is still
`27`, advance to `28`, otherwise advance exactly once from the then-current value.
Update the literal version test. Round-trip must preserve a mid-rotation timer and
rebuild occupancy shadow from entities rather than serialize it.

**Step 5: Add exact fixtures.** Cover:

- Unlimbo-style snap at `facing << 8`;
- ROT=5 retarget and same-frame `Current == Prev`;
- two mid-rotation timers with the same high byte but unequal full values;
- `barrel_facing` changes do not affect `locomotor_facing`;
- projection and timer remain synchronized across command, drive-track, docking,
  and owner-change-independent paths;
- snapshot round-trip and world hash change for different timer internals.

**Step 6: Verify serially.**

```powershell
cargo test locomotor_facing_ -- --nocapture
cargo test snapshot -- --nocapture
cargo test world_hash -- --nocapture
```

Expected: each command prints a passing literal `test result:` line.

### Task 1: Define the retry interfaces and exact playfield seam

**Why:** Establish the smallest dependency boundary before either policy or retry logic consumes it.

**Files:**
- Create: `src/sim/pathfinding/retry_cell_entry.rs`
- Create: `src/sim/pathfinding/zone_retry.rs`
- Modify: `src/sim/pathfinding/mod.rs`
- Modify: `src/sim/cell_rect.rs:466-510`
- Modify: `src/sim/movement/mod.rs:123-130`
- Modify: `src/sim/movement/movement_commands.rs:386-439`
- Modify: `src/sim/movement/movement_path.rs:625-680`
- Modify: `src/sim/movement/movement_tick.rs:433-520,820-860`

**Pattern:** Crate-private focused pathfinding submodules, matching `zone_hierarchy.rs`; `PathfindingContext` remains a Copy bundle of borrowed search inputs.

**Step 1: Declare the exact query and mover facts.** Use InternedId/TypeHandle-style identities already present in sim/rules; do not store strings.

```rust
// retry_cell_entry.rs
//! Exact read-only Unit/Infantry Can_Enter_Cell policy for failed hierarchy retries.

pub(crate) const CELL_ENTRY_CLEAR: u8 = 0;
pub(crate) const CELL_ENTRY_MAX: u8 = 7;

pub(crate) trait RetryCellEntryQuery {
    fn classify_retry(&self, candidate: (u16, u16), direction: u8) -> u8;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum RetryMoverClass {
    Unit,
    Infantry,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RetryMoverFacts {
    pub entity_id: u64,
    pub class: RetryMoverClass,
    pub owner: InternedId,
    pub type_ref: InternedId,
    pub movement_zone: MovementZone,
    pub speed_type: SpeedType,
    pub current_cell: (u16, u16),
    pub current_level: i8,
    pub on_bridge: bool,
    pub regular_crusher: bool,
    pub omni_crusher: bool,
    pub binary_frame: u32,
    pub locomotor_facing: FacingClass,
}
```

`RetryMoverFacts::from_entity` returns `None` unless category is Unit or Infantry and a locomotor is present. It copies only scalar/Copy facts; policy-specific reads remain borrowed through the oracle.

**Step 2: Declare the pure retry constants/types.**

```rust
// zone_retry.rs
//! Search-local failed-A* hierarchy retry producer.

pub(crate) const MAX_ASTAR_ATTEMPTS: u8 = 5;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum HierarchyValidity {
    Valid,
    Invalid,
}

pub(crate) struct RetryProducerContext<'a> {
    pub hierarchy: &'a ZoneHierarchy,
    pub movement_zone: MovementZone,
    pub resolved_terrain: &'a ResolvedTerrainGrid,
    pub playfield_bounds: PlayfieldBounds,
    pub query: &'a dyn RetryCellEntryQuery,
}
```

**Step 3: Expose the existing diamond predicate without copying its formula.**

```rust
pub(crate) fn is_cell_in_playfield(
    cell: (i32, i32),
    bounds: PlayfieldBounds,
    terrain: Option<&ResolvedTerrainGrid>,
) -> bool {
    cell_in_playfield_diamond(cell.0, cell.1, &bounds, terrain)
}
```

Add tests that the wrapper matches existing diamond edge fixtures, including the strict low/right/left and inclusive high boundaries.

**Step 4: Extend `PathfindingContext`.**

```rust
pub(super) struct PathfindingContext<'a> {
    pub path_grid: Option<&'a PathGrid>,
    pub zone_grid: Option<&'a ZoneGrid>,
    pub resolved_terrain: Option<&'a ResolvedTerrainGrid>,
    pub blocker_neighbor_counts: Option<&'a BlockerNeighborCounts>,
    pub retry_cell_entry: Option<&'a dyn RetryCellEntryQuery>,
    pub playfield_bounds: Option<PlayfieldBounds>,
}
```

Update every existing literal in `movement_path.rs`, `movement_commands.rs`, `movement_tick.rs`, and tests with `retry_cell_entry: None, playfield_bounds: None`. This is compile-only plumbing; exact activation comes in Task 8.

**Step 5: Add module declarations.** Keep both modules `pub(crate)` only where callers require them; do not export through the library API.

**Step 6: Verify.**

Run serially:

```powershell
cargo test cell_in_playfield -- --nocapture
cargo check -q
```

Expected: each command exits zero; the test command prints a literal passing `test result:` line.

### Task 2: Preserve hierarchy progress on failed A*

**Why:** The retry producer cannot choose its current cell if `core.rs` erases progress when A* returns no path.

**Files:**
- Modify: `src/sim/pathfinding/core.rs:2320-2406`

**Pattern:** Extend the existing `HierarchyProgressTracker`; do not expose frontier/closed-list internals.

**Step 1: Replace the success-only result.**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HierarchyMarkerAttemptOutcome {
    pub path: Option<Vec<(u16, u16)>>,
    pub progress_cell: (u16, u16),
    pub progress_index: usize,
}
```

**Step 2: Always return the outcome.**

```rust
// Replace only the current `let steps = astar_search(...)?; Some(...)` tail.
{
    let progress = HierarchyProgressTracker::new(start, level0_path);
    let path = astar_search(
        grid,
        start,
        MovementLayer::Ground,
        goal,
        &AStarOptions {
            terrain_costs: costs,
            entity_blocks,
            hierarchy_gate: Some(HierarchyGate {
                level0_zones,
                marked_level0,
                blocker_neighbor_counts,
            }),
            hierarchy_progress: Some(&progress),
            entity_block_map,
            marker_overlay,
            urgency,
            mover_is_crusher,
            movement_zone,
            resolved_terrain,
            ..Default::default()
        },
    )
    .map(|steps| steps.into_iter().map(|step| (step.rx, step.ry)).collect());

    HierarchyMarkerAttemptOutcome {
        path,
        progress_cell: progress.progress_cell(),
        progress_index: progress.progress_index(),
    }
}
```

Change the function return type to `HierarchyMarkerAttemptOutcome` and update `find_path_with_costs_hierarchy_marker` to return `.path` directly.

**Step 3: Add focused tests.**

- `failed_hierarchy_attempt_reports_furthest_selected_zone_progress`: selected path `A-B-C`, accept a B cell, force failure, assert the B coordinate/index.
- `failed_hierarchy_attempt_without_crossing_reports_start`: block every accepted exit from A and assert start/index zero.
- `successful_hierarchy_attempt_still_returns_path_and_progress`: protect the prior success contract.

The fixtures must use the existing hierarchy gate/progress machinery, not manually set the tracker.

**Step 4: Verify.**

```powershell
cargo test failed_hierarchy_attempt -- --nocapture
cargo test successful_hierarchy_attempt_still_returns_path_and_progress -- --nocapture
```

Expected: both commands print passing `test result:` lines.

### Task 3: Implement the pure retry producer

**Why:** Encode the binary-shaped `FloodFillReachableZones` split and `InvalidateZoneEdge` append rules independently of A* and world storage.

**Files:**
- Modify: `src/sim/pathfinding/zone_retry.rs`
- Modify: `src/sim/pathfinding/zone_hierarchy.rs:220-282`

**Pattern:** Pure deterministic transform over hierarchy graphs, terrain, a query trait, and search-local exclusions.

**Step 1: Add read-only edge-key accessors for assertions.**

```rust
impl ZoneEdgeKey {
    pub(crate) fn endpoints(self) -> (ZoneId, ZoneId) {
        (self.a, self.b)
    }
}
```

Do not expose mutable graph/exclusion internals.

**Step 2: Implement one level's block helper.** Use `[u8; 64]`, a stack preallocated to 64 cells, and a local observed-zone vector. The exact control flow is:

```rust
fn flood_level(
    ctx: &RetryProducerContext<'_>,
    progress_cell: (u16, u16),
    level: usize,
) -> FloodLevelResult {
    let graph = ctx.hierarchy.level(level).expect("levels 0..2 exist");
    let seed_zone = graph.zone_at(progress_cell.0, progress_cell.1);
    let block_size = 1u16 << (level + 1);
    let mask = block_size - 1;
    let mut visited = [0u8; 64];
    let mut stack = Vec::with_capacity(64);
    let mut observed_different = Vec::<ZoneId>::new();

    mark_and_push(&mut visited, &mut stack, progress_cell, mask);
    while let Some(current) = stack.pop() {
        for direction in 0u8..8 {
            let Some(neighbor) = checked_neighbor(current, direction) else { continue };
            let code = ctx.query.classify_retry(neighbor, direction);
            debug_assert!(code <= CELL_ENTRY_MAX);
            let matrix = ctx.resolved_terrain.cell(neighbor.0, neighbor.1)
                .and_then(|cell| ctx.movement_zone.matrix_row()
                    .map(|row| passability_value(row, cell.zone_type)))
                .unwrap_or(PASS_IMPASSABLE);

            if code != CELL_ENTRY_CLEAR && matrix == PASS_OK {
                continue;
            }

            let zone = graph.zone_at(neighbor.0, neighbor.1);
            if zone == seed_zone {
                mark_and_push_if_clear(&mut visited, &mut stack, neighbor, mask);
            } else if zone != ZONE_INVALID && !observed_different.contains(&zone) {
                observed_different.push(zone);
            }
        }
    }

    if block_contains_unvisited_seed_zone(
        progress_cell, block_size, mask, seed_zone, &visited, graph,
        ctx.playfield_bounds, ctx.resolved_terrain,
    ) {
        FloodLevelResult::Split
    } else {
        let mut unobserved_graph_neighbors = Vec::new();
        for edge in graph.edges(seed_zone).iter().rev() {
            if !observed_different.contains(&edge.neighbor) {
                unobserved_graph_neighbors.push(edge.neighbor);
            }
        }
        FloodLevelResult::UnobservedGraphNeighbors(unobserved_graph_neighbors)
    }
}
```

`mark_and_push` indexes exactly `((x & mask) * 8 + (y & mask))`. `block_contains_unvisited_seed_zone` reconstructs the aligned block coordinates with the verified masks, calls `is_cell_in_playfield(..., height_flag semantics already embodied by the wrapper)`, and scans in the binary's nested coordinate order.

**Step 3: Implement stored-path invalidation.**

```rust
fn invalidate_stored_path_edge(
    graph: &ZoneLevelGraph,
    path: &[ZoneId],
    current_zone: ZoneId,
    level: usize,
    exclusions: &mut ZonePrecheckExclusions,
) -> HierarchyValidity {
    if path.len() < 2 {
        return HierarchyValidity::Invalid;
    }
    let Some(index) = path.iter().position(|zone| *zone == current_zone) else {
        return HierarchyValidity::Invalid;
    };
    let (early, late) = if index + 1 < path.len() {
        (path[index], path[index + 1])
    } else {
        (path[index - 1], path[index])
    };
    exclusions.append_producer_edge(level, early, late);

    for late_edge in graph.edges(late).iter().rev() {
        let common = late_edge.neighbor;
        if common == early {
            continue;
        }
        for early_edge in graph.edges(early).iter().rev() {
            if early_edge.neighbor == common {
                exclusions.append_producer_edge(level, early, common);
            }
        }
    }
    HierarchyValidity::Valid
}
```

Do not sort adjacency, append the late endpoint to common neighbors, or suppress duplicate producer appends.

**Step 4: Implement all-level update.** For levels `0..3` in ascending order, derive `current_zone` from the same `progress_cell`. `Split` calls stored-path invalidation. `UnobservedGraphNeighbors` is reverse-consumed by the caller and appends `current_zone` pairs. Once invalid, retain already-appended records and stop only as the binary loop does.

**Step 5: Add pure tests.** Implement retry-contract T1 and T7-T14 with these exact names:

- `retry_exclusions_preserve_duplicate_order_until_new_search`
- `retry_update_uses_same_progress_cell_for_all_levels`
- `retry_zero_result_excludes_unobserved_graph_neighbors_in_native_order`
- `retry_invalidation_selects_adjacent_path_edge`
- `retry_invalidation_appends_asymmetric_common_neighbors_in_reverse_order`
- `retry_invalidation_preserves_duplicate_producer_appends`
- `retry_invalidation_without_path_edge_clears_validity`
- `retry_flood_bookkeeping_uses_inverse_cell_entry_matrix_polarity`
- `retry_flood_uses_fixed_stride_eight_for_blocks_two_four_eight`
- `retry_flood_uses_isometric_playfield_gate`

**Step 6: Verify.**

```powershell
cargo test retry_flood -- --nocapture
cargo test retry_invalidation -- --nocapture
cargo test retry_exclusions -- --nocapture
```

Expected: all print passing `test result:` lines.

### Task 4: Wire the five-attempt lifecycle in the eligible flat hierarchy branch

**Why:** Connect the pure producer to A* while preserving initial failure, retry-precheck, budget, and invalid-hierarchy transitions.

**Files:**
- Modify: `src/sim/pathfinding/zone_search.rs:176-330`

**Pattern:** Replace only the current single precheck/single A* eligible branch; leave layered and compatibility paths untouched.

**Step 1: Thread query and playfield parameters to `find_path_zoned_marker` and its inner function.** Append them near `blocker_neighbor_counts` to avoid changing unrelated parameter meaning.

**Step 2: Gate exact retries.** Enter the new loop only when all of these are present: hierarchy counts, hierarchy, query, resolved terrain, and playfield bounds. If the query/bounds are absent, retain the current one-precheck/one-A* behavior byte-for-byte at the Rust level.

**Step 3: Implement the loop.**

```rust
let run_unrestricted = || {
    find_path_with_costs_marker(
        grid,
        start,
        goal,
        costs,
        entity_blocks,
        movement_zone,
        resolved_terrain,
        entity_block_map,
        marker_overlay,
        urgency,
        mover_is_crusher,
    )
};
let run_hierarchical = |result: &ZonePrecheckResult| {
    find_path_with_costs_hierarchy_marker_progress(
        grid,
        start,
        goal,
        costs,
        entity_blocks,
        level0_zones,
        &result.marked[0],
        blocker_neighbor_counts.expect("exact retry requires counts"),
        &result.paths[0],
        movement_zone,
        resolved_terrain,
        entity_block_map,
        marker_overlay,
        urgency,
        mover_is_crusher,
    )
};
let mut exclusions = ZonePrecheckExclusions::default();
let mut validity = HierarchyValidity::Valid;
let mut attempts = 0u8;
let mut first_precheck = true;

loop {
    if attempts >= MAX_ASTAR_ATTEMPTS {
        return None;
    }
    if validity == HierarchyValidity::Invalid {
        attempts += 1;
        return run_unrestricted();
    }

    let precheck = zone_precheck_flat(
        hierarchy, hierarchy_start_zone, hierarchy_goal_zone,
        movement_zone.unwrap_or(mz), &exclusions,
    );
    let result = match precheck {
        ZonePrecheckOutcome::Passed(result) => result,
        ZonePrecheckOutcome::Failed if first_precheck && zones_match => {
            return run_unrestricted();
        }
        ZonePrecheckOutcome::Failed => return None,
    };
    first_precheck = false;
    attempts += 1;

    let outcome = run_hierarchical(&result);
    if let Some(path) = outcome.path {
        return Some(path);
    }

    validity = update_hierarchical_edges(
        &RetryProducerContext {
            hierarchy,
            movement_zone: movement_zone.unwrap_or(mz),
            resolved_terrain,
            playfield_bounds,
            query,
        },
        outcome.progress_cell,
        &result.paths,
        &mut exclusions,
    );
}
```

The fifth failed attempt still runs `update_hierarchical_edges`; the top-of-loop budget check prevents a sixth unrestricted attempt. A retry precheck failure returns before another A* call.

**Step 4: Add lifecycle tests with injected query rows and an A* call counter.** Cover contract T4-T6, T12, and T13:

- exactly five attempts when the fifth succeeds;
- one unrestricted second attempt after first update invalidates hierarchy;
- no second A* after retry precheck failure;
- no sixth attempt when fifth update invalidates hierarchy;
- same-zone initial precheck failure gets one unrestricted A*, cross-zone gets none;
- absent query retains one attempt and never calls the producer.

**Step 5: Verify.**

```powershell
cargo test hierarchy_retry_ -- --nocapture
cargo test initial_precheck_failure_ -- --nocapture
```

Expected: passing `test result:` lines and exact asserted call counts.

### Task 5: Implement fixed-tuple layer and ordered occupancy extraction

**Why:** Build the exact shared oracle inputs before writing either class policy.

**Files:**
- Modify: `src/sim/pathfinding/retry_cell_entry.rs`

**Pattern:** Borrowed read-only facade over existing authoritative owners; fold occupancy in native list order without per-query heap allocation.

**Step 1: Define the borrowed view.**

```rust
pub(crate) struct RetryCellEntryOracle<'a> {
    mover: RetryMoverFacts,
    entities: &'a EntityStore,
    occupancy: &'a OccupancyGrid,
    path_grid: &'a PathGrid,
    terrain: &'a ResolvedTerrainGrid,
    fog: &'a FogState,
    overlay_grid: &'a OverlayGrid,
    overlay_types: &'a OverlayTypeRegistry,
    alliances: &'a HouseAllianceMap,
    houses: &'a BTreeMap<InternedId, HouseState>,
    interner: &'a StringInterner,
    rules: &'a RuleSet,
}
```

`new` takes all sources and returns `None` if the mover/runtime class cannot be represented exactly. It does not clone any source.

**Step 2: Form the fixed tuple.**

```rust
#[derive(Debug, Clone, Copy)]
struct RetryTuple {
    candidate: (u16, u16),
    direction: u8,
    candidate_level: i8,
    parent: Option<(u16, u16)>,
    final_flag: bool,
}

fn retry_tuple(&self, candidate: (u16, u16), direction: u8) -> Option<RetryTuple> {
    (direction < 8).then_some(RetryTuple {
        candidate,
        direction,
        candidate_level: self.terrain.cell(candidate.0, candidate.1)?.level as i8,
        parent: None,
        final_flag: true,
    })
}
```

**Step 3: Derive independent layers.** Start from ground terrain/object/occupancy/owner. Apply only the verified null-parent diff-4 bridgehead reconstruction to `object_list_layer`; never change `occupancy_bits_layer` or owner source for the fixed tuple. Reuse `CanEnterLayerContext` only for the three layer fields; keep owner derivation explicit in the oracle.

**Step 4: Read stored occupation state and scan the ordered list separately.**

```rust
#[derive(Debug, Clone, Copy, Default)]
struct RetryOccupancyFacts {
    low_bits: u8,
    owner: Option<InternedId>,
    stationary_allied_infantry: u8,
}

fn occupancy_facts(
    &self,
    cell: (u16, u16),
    object_list_layer: MovementLayer,
    occupancy_layer: MovementLayer,
) -> RetryOccupancyFacts {
    let native = self
        .occupancy
        .movement_occupation(cell.0, cell.1, occupancy_layer);
    let mut facts = RetryOccupancyFacts {
        low_bits: native.bits,
        owner: native.infantry_owner,
        stationary_allied_infantry: 0,
    };
    let Some(list) = self.occupancy.get(cell.0, cell.1) else { return facts };
    for occupant in list.iter_layer(object_list_layer) {
        let Some(entity) = self.entities.get(occupant.entity_id) else { continue };
        if entity.stable_id == self.mover.entity_id {
            continue;
        }
        if entity.category == EntityCategory::Infantry
            && self.is_allied(entity.owner)
            && !locomotor_is_moving(entity)
        {
            facts.stationary_allied_infantry =
                facts.stationary_allied_infantry.saturating_add(1);
        }
    }
    facts
}
```

Use the selected CellClass list order directly for object-policy scans. Never clear
the mover's Infantry subcell bit from `low_bits`: self is skipped only from the
object scan. Unit policy performs its native local `low_bits &= !0x20` only when
its own object entry is encountered. Owner always comes from the fixed tuple's
occupation layer, which remains ground even when the rare bridgehead rule selects
the bridge object list.

**Step 5: Implement dispatch.**

```rust
impl RetryCellEntryQuery for RetryCellEntryOracle<'_> {
    fn classify_retry(&self, candidate: (u16, u16), direction: u8) -> u8 {
        let Some(tuple) = self.retry_tuple(candidate, direction) else {
            return 7;
        };
        let result = match self.mover.class {
            RetryMoverClass::Unit => self.classify_unit(tuple),
            RetryMoverClass::Infantry => self.classify_infantry(tuple),
        };
        debug_assert!(result <= CELL_ENTRY_MAX);
        result
    }
}
```

**Step 6: Add contract tests T1-T6 and T21 foundations.** Include fixed tuple
recording, Unit/Infantry dispatch independent of JumpJet/MovementZone, rare split
layer, order reconstruction from `occupancy_enter_order`, reversed-occupant early
return harness, retained Infantry self bit, stored owner after last-marker removal,
non-reference-counted Unit bit clear, no RNG/state-generation changes, and
source-scope/drop/rebuild behavior.

**Step 7: Verify.**

```powershell
cargo test retry_oracle_fixed_tuple -- --nocapture
cargo test retry_oracle_layer -- --nocapture
cargo test retry_oracle_occupancy -- --nocapture
cargo test retry_oracle_no_side_effects -- --nocapture
```

Expected: all pass. No current-list fold remains in the oracle.

### Task 6: Implement the exact Unit policy

**Why:** Unit is one of the two active runtime-class targets and cannot be replaced by the approximate occupied-cell classifier.

**Files:**
- Modify: `src/sim/pathfinding/retry_cell_entry.rs`

**Pattern:** One ordered policy function with small private fact helpers; no runtime action payloads and no reuse of approximate `cell_entry.rs` branches unless tests prove full equivalence.

**Step 1: Inventory every input before coding.** Map each verified Unit phase to an existing field/helper. The required sequence is:

1. ground/bridge snapshot and candidate height;
2. tube land/subtype gates;
3. tube entry/dead-tube result;
4. raw and reverse direction mismatch;
5. bridge traversal height/slope gate;
6. optional bridge occupancy reread (not selected by the ordinary fixed tuple);
7. shroud/RequiresRevealedCells policy;
8. locomotor passability result;
9. crate and wall overlay policy;
10. selected-list scan: self, transport/destination, DontScore foundation, train mutual-ignore, building/refinery/contact/capture/garrison/gate/bunker/repair/laser-fence/mission cases, alliance, crush and weapon cases, moving-ally deadlock;
11. speed-type/land-type zero gate;
12. crush candidate and final packed occupancy resolution.

If an active read has no Rust source, record the exact missing input and stop this task. Do not default a branch.

**Step 2: Implement phase helpers that return `ControlFlow<u8, UnitAccumulator>`.** This makes early returns explicit while retaining the running result/crush flags:

```rust
#[derive(Debug, Clone, Copy, Default)]
struct UnitAccumulator {
    result: u8,
    crush_candidate: bool,
    occupancy_bits: u8,
}

fn classify_unit(&self, tuple: RetryTuple) -> u8 {
    let Some((layers, mut acc)) = self.unit_initial_state(tuple) else { return 7 };
    for phase in [
        Self::unit_tube_and_bridge_phase,
        Self::unit_shroud_and_locomotor_phase,
        Self::unit_overlay_phase,
        Self::unit_object_list_phase,
        Self::unit_speed_land_phase,
    ] {
        match phase(self, tuple, layers, acc) {
            ControlFlow::Break(code) => return code,
            ControlFlow::Continue(next) => acc = next,
        }
    }
    self.unit_terminal_phase(tuple, layers, acc)
}
```

Use named code constants, saturating/ordered comparisons exactly as the report states, and existing fixed/integer range helpers. Do not reproduce gamemd vtables or inheritance.

**Step 3: Implement object iteration as one native-order scan.** Every occupant is visited; do not choose a primary blocker. Initialize `acc.occupancy_bits` from Task 0A's stored shadow. When the scan encounters self, clear only local bit `0x20`, clear the local has-Unit flag, and continue; do not mutate `OccupancyGrid`. Update `acc.result` only under the binary's `<3`/`<5`-style precedence or return immediately. Resolve rules through `rules.object(interner.resolve(type_ref))`, primary weapon through `rules.weapon`, and warhead through `rules.warhead`; missing subobjects take their verified hard result.

For the moving-allied deadlock branch, sample both full timers at the same frame:

```rust
let blocker_facing = blocker
    .locomotor_facing
    .current(self.mover.binary_frame);
let mover_facing = self
    .mover
    .locomotor_facing
    .current(self.mover.binary_frame);
```

Apply the verified native quantization/comparison after these reads. Never compare
`barrel_facing`, `facing_target`, or only `facing: u8`.

**Step 4: Add Unit fixtures.** Cover contract T7-T10 plus an explicit reversed-list test:

- wrong/dead tube and direction mismatch;
- diff-0/diff-1-slope/diff-4 bridge cases and rejected height;
- revealed/unrevealed branches;
- crushable and non-crushable wall with/without weapon ability;
- refinery/contact/bib, gate state, bunker/repair, garrison, capture/enter, own cargo/target;
- allied moving/stationary and hostile Unit/Infantry/building;
- moving allies whose full 16-bit values differ while their high bytes match;
- moving allies whose full values match, with distance immediately below and at
  the native `0x200` threshold;
- crusher and weapon/warhead outcomes;
- speed/land zero;
- empty terminal result and every produced code `0..7` reachable in the scoped policy.

**Step 5: Verify.**

```powershell
cargo test retry_unit_ -- --nocapture
```

Expected: a passing `test result:` line. Do not proceed if any active phase remains represented by a guessed default.

### Task 7: Implement the exact Infantry wall and terminal policy

**Why:** Close the verified free-subcell, wall-code, owner/range, and stationary-count disparities with stock-active data.

**Files:**
- Modify: `src/sim/pathfinding/retry_cell_entry.rs`

**Pattern:** Preserve the existing Infantry pre-terminal object policy from verified research, then apply the focused wall and terminal slices in their exact order.

**Step 1: Implement wall classification before terminal occupancy.**

```rust
fn infantry_wall_result(&self, tuple: RetryTuple, accumulated: u8) -> Result<u8, u8> {
    let cell = self.overlay_grid.cell(tuple.candidate.0, tuple.candidate.1);
    let Some(overlay_id) = cell.overlay_id else { return Ok(accumulated) };
    let Some(flags) = self.overlay_types.flags(overlay_id) else { return Ok(accumulated) };

    if flags.crate_type && !self.mover_is_player_controlled() {
        return Err(7);
    }
    if !flags.wall || u16::from(cell.overlay_data >> 4) == flags.damage_levels {
        return Ok(accumulated);
    }
    if !self.infantry_action_gate(tuple.candidate) {
        return Err(7);
    }
    let Some(weapon) = self.primary_weapon() else { return Err(7) };
    let Some(warhead) = weapon.warhead.as_deref().and_then(|id| self.rules.warhead(id)) else {
        return Err(7);
    };
    if !warhead.wall {
        return Err(7);
    }
    let wall_owner = self.wall_owner(tuple.candidate);
    Ok(if wall_owner.is_some_and(|owner| self.is_allied(owner)) { 4 } else { 5 })
}
```

Use the actual parsed field names (`crate_type`, `damage_levels`, `warhead`, `wall`) and the paired wall entity's owner. Equality with DamageLevels skips classification; do not use a less/greater comparison.

**Step 2: Implement the terminal ladder literally.**

```rust
fn infantry_terminal_result(
    &self,
    accumulated: u8,
    occupancy: RetryOccupancyFacts,
    primary_range: SimFixed,
) -> u8 {
    let full = occupancy.low_bits & 0x1c == 0x1c;
    if accumulated == 0 && occupancy.low_bits & 0x20 != 0 {
        return 2;
    }
    if let Some(owner) = occupancy.owner {
        if self.is_allied(owner) {
            if full && accumulated < 2 {
                return if occupancy.stationary_allied_infantry == 3 { 6 } else { 2 };
            }
        } else if accumulated < 5 {
            return if primary_range <= SIM_ZERO { 7 } else { 5 };
        }
    }
    if accumulated != 0 {
        accumulated
    } else if full {
        7
    } else {
        0
    }
}
```

Apply the verified ground-only speed-table hard gate immediately before this ladder when the selected object-list layer is ground. Preserve the slave-at-cell bit-5 clearing branch if its required state is represented; otherwise stop and report the missing active input.

**Step 3: Count stationary Infantry from locomotor state.** Use the existing locomotor movement predicate/phase that corresponds to Walk `Is_Moving`; do not equate entity count, movement_target presence, or speed with stationarity without a fixture proving equivalence.

**Step 4: Add stock-shaped parser fixture and contract T11-T20.** Tests must cover:

- GAWALL/NAWALL/GASAND-style flags and DamageLevels from INI registries;
- allied wall `4`, hostile/unowned wall `5`;
- no weapon/warhead, `Wall=no`, failed action gate, and equal state nibble;
- bit 5 only at accumulated zero;
- hostile free subcell with nonpositive versus positive primary range;
- allied full exactly three stationary `6`;
- two stationary plus one moving, and all other counts, `2`;
- anonymous full `7`, anonymous nonfull `0`;
- accumulated `4` survives full occupancy and hostile ownership upgrades only values below `5`;
- bridge object list plus ground terminal inputs.

**Step 5: Verify.**

```powershell
cargo test retry_infantry_ -- --nocapture
cargo test retry_wall_ -- --nocapture
```

Expected: passing `test result:` lines for both commands.

### Task 8: Move production searches across the mutable-borrow boundary and construct the oracle

**Why:** Exact policy must read the full world immutably at the existing logical search point; a world clone or approximate fallback is forbidden.

**Files:**
- Modify: `src/sim/movement/mod.rs`
- Modify: `src/sim/movement/movement_path.rs`
- Modify: `src/sim/movement/movement_blocked.rs`
- Modify: `src/sim/movement/movement_step.rs`
- Modify: `src/sim/movement/movement_occupancy.rs`
- Modify: `src/sim/movement/movement_tick.rs`
- Modify: `src/sim/world/mod.rs:2086-2163`
- Modify signature callers: `src/sim/pathfinding/core_tests.rs`,
  `src/sim/movement/prone_speed_tests.rs`, and
  `src/sim/movement/movement_tests.rs`

**Pattern:** Existing `DeferredCellCheck`/`DeferredDriveTrackChain` approach: record Copy/owned inputs, end the mover borrow, read the world, then reborrow only to commit.

**Step 1: Define one pending request and one completed result.**

```rust
#[derive(Debug, Clone, Copy)]
pub(super) struct PendingPathSearch {
    pub start: (u16, u16),
    pub start_layer: MovementLayer,
    pub goal: (u16, u16),
    pub layered_pathing: bool,
    pub movement_zone: MovementZone,
    pub too_big_to_fit_under_bridge: bool,
    pub urgency: u8,
    pub mover_is_crusher: bool,
    pub reason: PendingPathReason,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) enum PendingPathReason {
    ExhaustedSegment,
    Blocked { is_infantry: bool },
}

pub(super) struct CompletedPathSearch {
    pub request: PendingPathSearch,
    pub effective_goal: (u16, u16),
    pub path: Option<(Vec<(u16, u16)>, Vec<MovementLayer>)>,
}
```

These types are never serialized or stored beyond one mover iteration.

**Step 2: Split calculation from mutation in `movement_path.rs`.** `run_pending_path_search` performs `resolve_requested_move_goal` and `find_move_path`; `apply_exhausted_segment_result` and `apply_blocked_repath_result` mutate target/facing/timers using the existing code verbatim. Failed blocked results retain urgency-specific stuck/delay behavior; successful results retain same debug event and path fields.

**Step 3: Prepare exhaustion before the main mutable scope.** Inspect the mover immutably. If its segment is exhausted and final goal differs, build a request, construct the oracle, run the search, drop the oracle, then mutably apply the result before entering normal movement processing. If already at the final goal, preserve the current subcell/finished logic.

**Step 4: Make blocked handling return a request.** `handle_blocked_tick` performs only pre-search timer/stat/event mutations and returns:

```rust
pub(super) struct BlockedTickOutcome {
    pub debug_events: Vec<(u32, DebugEventKind)>,
    pub pending_search: Option<PendingPathSearch>,
    pub aborted_for_stuck: bool,
}
```

`movement_step::CrossingOutput` and the deferred occupancy result each gain `pending_search`. Since existing blocked branches already stop crossing/advancement after the search call, bubbling the request out does not add a movement step or delay the commit to another tick.

**Step 5: Add a helper that scopes the oracle to one search.**

```rust
#[derive(Clone, Copy)]
struct PendingSearchInputs<'a> {
    base_ctx: PathfindingContext<'a>,
    terrain_costs: Option<&'a TerrainCostGrid>,
    entity_blocks: Option<&'a BTreeSet<(u16, u16)>>,
    entity_block_map: Option<&'a LayeredEntityBlockMap>,
}

fn execute_pending_search(
    request: PendingPathSearch,
    mover: RetryMoverFacts,
    sources: RetryOracleSources<'_>,
    inputs: PendingSearchInputs<'_>,
) -> CompletedPathSearch {
    let oracle = RetryCellEntryOracle::new(mover, sources);
    let ctx = PathfindingContext {
        retry_cell_entry: oracle.as_ref().map(|value| value as &dyn RetryCellEntryQuery),
        playfield_bounds: sources.playfield_bounds,
        ..inputs.base_ctx
    };
    run_pending_path_search(
        request,
        ctx,
        inputs.terrain_costs,
        inputs.entity_blocks,
        inputs.entity_block_map,
    )
}
```

`RetryOracleSources` is a Copy bundle of references to entities, occupancy, path grid, resolved terrain, fog, overlay grid/registry, alliances, interner, and rules. Construction returns no exact query if any required top-level source is absent.

**Step 6: Extend the movement entry point and world call.** Pass `&self.fog`, `&self.overlay_grid`, `overlay_registry`, `self.playfield_bounds`, `&self.houses`, and the pre-increment `self.session.binary_frame` alongside existing sources. Preserve the current Phase 1 location before fog refresh. Legacy wrappers pass `None`/empty house data for exact-only sources.

Update every direct signature caller, including the focused files listed above;
verify exhaustively with `rg -n "tick_movement_with_grids\(|find_path_zoned_marker\(" src/sim`.

**Step 7: Thread query/bounds through `movement_path.rs`.** Only the flat call to `zone_search::find_path_zoned_marker` consumes them. Layered search and smoothing do not.

**Step 8: Add timing/borrow regression tests.**

- earlier live-order mover changes occupancy; later mover's newly constructed oracle observes it;
- one oracle/search sees a stable state and consumes no RNG;
- exhausted-segment repath still commits before same-tick movement processing;
- blocked success/failure preserves timers, stuck counter, stats, events, and next-tick path consumption;
- no source causes a world/grid clone;
- legacy/headless paths compile and retain absent query;
- standard eligible Unit/Infantry movement asserts the query is present;
- unrelated dirty `world/mod.rs` animation/object-ID changes remain in the diff.

**Step 9: Verify focused movement tests.** Before Cargo, check for another active build:

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue | Select-Object ProcessName,Id,CPU
cargo test pending_path_search -- --nocapture
cargo test retry_oracle_live_order -- --nocapture
cargo test movement -- --nocapture
```

Expected: no conflicting active Cargo owner; each test command prints a passing `test result:` line.

### Task 9: Activate and prove the exact integration

**Why:** The feature is complete only when a failed eligible hierarchy search probes the exact class oracle and never substitutes coarse legality.

**Files:**
- Modify: `src/sim/pathfinding/zone_search.rs` tests
- Modify: `src/sim/pathfinding/retry_cell_entry.rs` tests
- Modify: `src/sim/movement/movement_tick.rs` tests

**Pattern:** Cross-module mechanism fixture in existing unit-test modules; no hand-computed route may be labeled retail parity.

**Step 1: Add the contract T22 integration fixture.** Build an eligible flat hierarchy search whose first marker-gated A* fails. Record every `(candidate, direction, full_code)` query. Assert:

- directions are generated in `0..7` table order for each popped cell;
- fixed tuple uses candidate signed level/null parent/final `1`;
- producer applies `code == 0 || matrix != 1` outside the oracle;
- exclusions feed the next precheck;
- at most five A* calls occur;
- no call reaches approximate `classify_occupied_cell*`, `PathGrid::is_walkable` as retry classification, or one-blocker `LayeredEntityBlockMap` classification.

**Step 2: Add absence/fail-closed fixture.** Remove rules, fog, registry, or playfield bounds one at a time. Assert exact retry does not activate, only the prior one-attempt behavior runs, and no coarse query is synthesized.

**Step 3: Add full-code preservation fixture.** Make the oracle return each `0..7`; assert the recorder retains the exact code and only the producer performs zero/nonzero collapse.

**Step 4: Run focused and final checks serially.** Format only files edited by the implementation:

```powershell
rustfmt --edition 2024 src/sim/occupancy.rs src/sim/game_entity.rs src/sim/pathfinding/retry_cell_entry.rs src/sim/pathfinding/zone_retry.rs src/sim/pathfinding/core.rs src/sim/pathfinding/zone_hierarchy.rs src/sim/pathfinding/zone_search.rs src/sim/cell_rect.rs src/sim/movement/facing_class.rs src/sim/movement/mod.rs src/sim/movement/movement_commands.rs src/sim/movement/movement_path.rs src/sim/movement/movement_blocked.rs src/sim/movement/movement_step.rs src/sim/movement/movement_occupancy.rs src/sim/movement/movement_tick.rs src/sim/movement/air_movement.rs src/sim/movement/rocket_movement.rs src/sim/movement/tube_movement.rs src/sim/movement/tunnel_movement.rs src/sim/combat/combat_targeting.rs src/sim/miner/miner_dock_sequence.rs src/sim/docking/bunker_link.rs src/sim/docking/bunker_install.rs src/sim/world/world_commands.rs src/sim/world/world_spawn.rs src/sim/world/world_hash.rs src/sim/world/mod.rs src/sim/snapshot.rs
git diff --check
Get-Process cargo,rustc -ErrorAction SilentlyContinue | Select-Object ProcessName,Id,CPU
cargo test retry_ -- --nocapture
cargo test pathfinding -- --nocapture
cargo test movement -- --nocapture
cargo check -q
```

Inspect the formatted diff for unrelated churn, especially `occupancy.rs`,
`game_entity.rs`, `movement_tick.rs`, and `world/mod.rs`. Expected:
`git diff --check` and `cargo check -q` exit zero; every test command prints a
passing literal `test result:` line.

### Task 10: Verify against gamemd-derived evidence and update research status

**Why:** Rust fixtures prove the mechanism implementation but cannot certify retail parity without an executable gamemd-derived comparison.

**Files:**
- Modify only if evidence changes: `docs/contracts/2026-07-18-pathfinding-failed-astar-retry-implementation-contract.md`
- Modify only if evidence changes: `docs/contracts/2026-07-18-failed-astar-retry-can-enter-oracle-implementation-contract.md`
- Add generated evidence under the existing parity/fidelity evidence location selected by `/fidelity-check`; do not hand-edit a parity ledger.

**Pattern:** Project fidelity workflow; active binary evidence outranks prior prose.

**Step 1: Re-open the four load-bearing binary bodies.** Confirm `0x0042C900`, `0x005840C0`, `0x0073F0A0`, and `0x0051BF90` still match the implemented attempt loop, corrected flood polarity, fixed tuple, and class policies. Record any label drift by address/body role.

**Step 2: Capture one gamemd-derived failed hierarchy retry.** The capture must include attempt count, failed progress cell, per-level current zones, ordered exclusions including duplicates, and every retry cell-entry probe/code. Feed the same map/mover/state to Rust and compare the sequence exactly.

**Step 3: Run `/fidelity-check` for the named pathfinding retry case.** A pass requires identical probe order/codes, attempt/precheck/update order, exclusions, chosen path/failure, and no extra RNG consumption. A Rust-vs-old-Rust replay is only a regression ratchet.

**Step 4: Update contracts based on evidence.** Promote the old runtime-cell-entry blocker only after the executable comparison passes. If it fails, record each disparity explicitly; do not soften it as an edge case. Reindex research after intentional doc edits:

```powershell
python tools/research_index/index.py
python tools/research_index/validate.py --system pathfinding "failed AStar retry cell entry oracle"
```

Expected: validation reports no broken links or stale checksums. No commit or push is included because the user did not request either.

## Sources & References

- **Design:** `docs/plans/2026-07-18-failed-astar-retry-cell-entry-oracle-design.md`
- **Contracts:**
  - `docs/contracts/2026-07-18-failed-astar-retry-can-enter-oracle-implementation-contract.md`
  - `docs/contracts/2026-07-18-pathfinding-failed-astar-retry-implementation-contract.md`
- **Primary/focused research:**
  - `docs/research/pathfinding/PATHFINDING_FAILED_ASTAR_RETRY_CAN_ENTER_CELL_ADAPTER_GHIDRA_REPORT.md`
  - `docs/research/MAPCLASS_FLOODFILLREACHABLEZONES_005840C0_GHIDRA_REPORT.md` (use corrected polarity only through the newer producer report)
  - `docs/research/ZONEMAP_FLOODFILLREACHABLEZONES_RETRY_PRODUCER_GHIDRA_REPORT.md`
  - `docs/research/pathfinding/PATHFINDER_INVALIDATEZONEEDGE_COMMON_NEIGHBORS_GHIDRA_REPORT.md`
  - `docs/research/pathfinding/UPDATEHIERARCHICALEDGES_FAILED_ASTAR_EDGE_SELECTION_GHIDRA_REPORT.md`
  - `docs/research/pathfinding/ASTAR_RETRY_RESET_EXCLUSION_LIFETIME_GHIDRA_REPORT.md`
  - `docs/research/pathfinding/PATHFINDER_FAILED_ASTAR_CURRENT_ZONE_SOURCE_GHIDRA_REPORT.md`
  - `docs/research/pathfinding/ASTAR_PATHFIND_SEARCH_0042C900_RETRY_SEMANTICS_GHIDRA_REPORT.md`
  - `docs/research/pathfinding/UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md`
  - `docs/research/pathfinding/INFANTRYCLASS_CAN_ENTER_CELL_TERMINAL_OCCUPANCY_AND_WALL_GHIDRA_REPORT.md`
  - `docs/research/INFANTRY_SUBCELL_POSITIONING.md`
  - `docs/research/LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md`
  - `docs/research/CELLCLASS_MAPCLASS_ENGINE_SUBSTRATE_SERVICE_STUDY.md`
  - `docs/research/FOOTCLASS_0X388_LOCOMOTOR_FACING_GHIDRA_REPORT.md`
  - `docs/research/CELLCLASS_PLAYFIELD_BOUNDS_FROM_LOCALSIZE_GHIDRA_REPORT.md`
- **Live gamemd anchors rechecked during planning:**
  - `AStar_pathfind_search @ 0x0042C900`
  - `ZoneMap::FloodFillReachableZones @ 0x005840C0`
  - `UnitClass::Can_Enter_Cell @ 0x0073F0A0`
  - `InfantryClass::Can_Enter_Cell @ 0x0051BF90`
  - `InfantryClass::MarkCellOccupancy @ 0x005217C0`
  - `InfantryClass::UnmarkCellOccupancy @ 0x00521850`
  - `ObjectClass::Mark_Occupation @ 0x007441B0`
  - `ObjectClass::Clear_Occupation @ 0x00744210`
  - `DriveLocomotionClass::Do_Turn @ 0x004B0EF0`
  - `TechnoClass::Unlimbo @ 0x006F6CA0`
  - `UnitClass::Constructor @ 0x007353C0`
  - `UnitClass::Facing_Update @ 0x00736990`
- **Stock INI:**
  - `ini/rulesmd.ini`: `[GAWALL] Wall=yes`, `[NAWALL] Wall=yes`, `[GASAND] Crushable=yes, Wall=yes`, `[PrismWarhead] Wall=yes`
  - `ini/artmd.ini`: `[GAWALL] DamageLevels=3`, `[NAWALL] DamageLevels=3`, `[GASAND] DamageLevels=2`
- **Current Rust patterns:**
  - `src/sim/pathfinding/core.rs:692-737,2320-2406`
  - `src/sim/pathfinding/zone_hierarchy.rs:113-290`
  - `src/sim/pathfinding/zone_search.rs:176-330`
  - `src/sim/pathfinding/cell_entry.rs:190-209,415-424,449-577`
  - `src/sim/occupancy.rs:20-139`
  - `src/sim/game_entity.rs:198-208,292-294`
  - `src/sim/movement/facing_class.rs:22-175`
  - `src/sim/movement/movement_step.rs:222-275`
  - `src/sim/cell_rect.rs:179-201,466-510`
  - `src/sim/movement/mod.rs:123-161`
  - `src/sim/movement/movement_tick.rs:978-1646`
  - `src/sim/world/mod.rs:2086-2163,2327-2339`
- **Relevant git history:** `80e45210`, `52c712cf`, `e0206d00`, `1ee888a4`, `684d9d22`, `016ae152`.
