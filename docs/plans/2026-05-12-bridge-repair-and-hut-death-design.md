# Bridge Repair + Hut-Death-Destroys-Bridge — Design

**Date:** 2026-05-12
**Status:** Approved (Approach A) — ready for `/write-plan`
**Source research:** [`BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`](../../../../ra2-rust-game-docs/BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md) (Phases 1 + 2)
**Companion plan that motivated this:** [`2026-05-12-bridge-repair-system-investigation-plan.md`](2026-05-12-bridge-repair-system-investigation-plan.md)

## Goal

Wire runtime `bridge_walkable` invalidation in both directions: engineer
entering a CABHUT (`BridgeRepairHut=yes`) triggers Destroyed→Healthy
across the affected bridge span(s); C4 or demo-truck detonation on a
CABHUT triggers Healthy→Destroyed; both paths flow through the existing
`zones_dirty → refresh_bridge_zones_if_dirty` rebuild so PathGrid
walkability flips correctly.

## Architecture Context

The Rust sim already has most of the load-bearing infrastructure for
bridge state and zone refresh. What's missing is the **trigger**
wiring on both sides.

**Existing — uses unchanged:**
- [`BridgeRuntimeState`](../../src/sim/bridge_state/mod.rs) — per-cell
  `damage_state` (Healthy{variant 0..5} / Damaged / PartialCollapseA /
  PartialCollapseB / Destroyed) keyed by `(rx, ry)`, anchor-span
  registry in `anchor_spans: BTreeMap<AnchorSpanId, AnchorSpan>`,
  and `is_bridge_walkable(rx, ry)` read query at line 732. Forward
  state machine `body_cell_advance_state` (line 756) covers
  Healthy→Damaged and Damaged→Destroyed transitions.
- [`bridge_orchestrator::refresh_bridge_zones_if_dirty`](../../src/sim/world/bridge_orchestrator.rs#L312)
  — rebuilds `BridgeEndpointRecord.active` flags, `PathGrid`, and
  zone grid when `any_zones_dirty` is true. **Direction-agnostic** —
  the snapshot it reads is post-mutation, so a Destroyed→Healthy
  flip in `BridgeRuntimeCell.damage_state` propagates through the
  rebuild correctly.
- [`tick_capture_orders`](../../src/sim/world/world_orders.rs#L151) and
  [`tick_c4_plants`](../../src/sim/world/world_orders.rs#L228) — the
  Phase-5 trigger zone in `World::advance_tick`. Both fire after
  movement settles, before combat. The bridge-repair trigger lives in
  the same zone.
- [`apply_c4_damage_to_building`](../../src/sim/world/world_orders.rs#L469)
  — entry point for C4 detonation. Hook lives inside.
- INI parses: `ObjectType.bridge_repair_hut` (object_type.rs:483),
  `BridgeRules.repair_sound` (ruleset.rs:700). Both unwired today.

**Missing — added in this design:**
- Reverse state machine: `BridgeRuntimeState::body_cell_repair_state`
  (Destroyed/Damaged → Healthy{variant}).
- Trigger: `Simulation::tick_bridge_repair_orders` (Phase 5).
- Trigger field on engineer entity: re-use `capture_target` with a
  branch on `target.bridge_repair_hut` (Approach A).
- Destroy hook: branch in `apply_c4_damage_to_building` when target
  is BridgeRepairHut.
- Sound event: `SimSoundEvent::BridgeRepaired { rx, ry, owner }`.

**Dependency rules unchanged:** sim/ never depends on render / ui /
audio / net. The design adds only sim-internal calls and one new
`SimSoundEvent` variant; the app layer resolves audio + EVA.

## Impact Analysis

**Files added or modified:**

| File                                       | Change                                                                                       |
|--------------------------------------------|----------------------------------------------------------------------------------------------|
| `src/sim/bridge_state/mod.rs`              | New: `body_cell_repair_state` reverse state machine. New: `cells_in_5x5_scan` helper. New: `RepairOutcome` struct. Per-strip iteration over anchor-span cells. |
| `src/sim/world/world_orders.rs`            | New: `tick_bridge_repair_orders`. Branch in `tick_capture_orders` to skip ownership-transfer when target is BridgeRepairHut. Branch in `apply_c4_damage_to_building` to dispatch hut-death-bridge-collapse instead of damaging the hut. |
| `src/sim/world/mod.rs`                     | New: `SimSoundEvent::BridgeRepaired { rx, ry, owner }`. Wire `tick_bridge_repair_orders` into `advance_tick` at line 1204. Set `TickResult.bridge_state_changed = true` on any repair-side mutation. |
| `src/sim/bridge_state/tests/repair_tests.rs` (or similar) | New unit tests for `body_cell_repair_state` covering each transition class + RNG determinism. |
| `src/sim/world/world_orders_tests.rs` (or similar) | New integration tests: engineer-arrives-CABHUT triggers repair; C4-on-CABHUT triggers collapse (marked `#[ignore]` until upstream Immune fix lands). |

**Blast radius:**
- **Sim → render**: `BridgeRuntimeCell.damage_state` flipping from
  Destroyed to Healthy will trigger the existing dirty-cell render
  refresh (atlas rebuild, sprite re-derive). No new render code; verify
  the existing pipeline handles reverse transitions (it should, since
  it reads current `damage_state` and doesn't cache a "max-damaged"
  snapshot).
- **Sim → PathGrid**: zones rebuild already covers the reverse direction.
  Adding `bridge_state_changed=true` for repair causes one extra PathGrid
  rebuild per tick where a repair fired — same cost as a destruction
  tick, infrequent.
- **Determinism**: RNG draws happen during repair (`sim.rng.next_range_u32(4)`
  per main-deck strip). All Rust clients must advance RNG by the same
  count at the same tick. Driven by deterministic engineer iteration
  order (`keys_sorted`) and deterministic anchor-span strip iteration
  (locked by test).
- **State hash**: `BridgeRuntimeCell.damage_state` is already part of
  the hash. Reverse transitions are hashed identically to forward.

**Risk areas:**
1. **Strip-iteration order for RNG parity.** Lock with a test that pins
   the exact RNG-draw sequence for a known span repair.
2. **C4-on-CABHUT path is dead code in vanilla YR** (Immune blocks at
   upstream gate, out of brainstorm scope). Test `#[ignore]` with a
   reference to `project_c4_bridge_hut_followup` until that fix lands.
3. **Variant byte may be unobserved at render time** (current renderer
   derives jitter from `(x, y)` per the comment at bridge_state/mod.rs:96).
   Phase 3 RE follow-up to verify whether gamemd's renderer reads the
   stored variant. If not, our RNG draws are unobservable but still
   parity-correct on RNG state. Documented; not a blocker.
4. **Tag-trigger fire on engineer consumption** (`ProcessCellAction(0x30)`)
   has no Rust analog yet (no tag system). Stubbed with a TODO; no
   parity impact for vanilla skirmish maps (skirmish maps don't define
   triggers).
5. **Vanilla `UpdateAdjacentBridges_High` copy-paste bug** — our zone
   rebuild is band-agnostic, so the bug becomes a no-op in Rust.
   Documented; no implementation impact.

## Chosen Approach

**Approach A: minimal new state, fold into existing capture intent.**

- The engineer's `capture_target` field is reused: when the target is a
  `BridgeRepairHut`, `tick_capture_orders` short-circuits and delegates
  to `tick_bridge_repair_orders` (or its inline branch). No new field on
  the entity struct.
- A single new tick function `tick_bridge_repair_orders` handles the
  trigger: for each engineer-with-capture-target-at-CABHUT that is
  Chebyshev-≤-1 adjacent, do the EVA + sound + 5×5 scan + bridge state
  mutation + engineer consume.
- A single destroy hook lives in `apply_c4_damage_to_building`: when the
  target is `BridgeRepairHut`, skip damaging the hut and dispatch the
  bridge-collapse cascade for cells in the 5×5 around the hut. The
  demo-truck path stubs the same dispatch (no demo-truck unit yet in
  Rust — a TODO comment marks where the hook will go).
- The cell-state mutation core lives in `BridgeRuntimeState::body_cell_repair_state`,
  which takes the 5×5 cell list, looks up unique anchor spans, iterates
  per-strip, picks RNG variants for main-deck strips, and returns a
  `RepairOutcome` carrying the zones-dirty flag, the radar-dirty cells,
  and the repaired-cell count.

**Why not B or C** (sibling separate field; generic intent enum):
neither offers observable benefit over A; both add Rust state for
internal cleanness that doesn't move the parity needle. A mirrors
gamemd's "mission + target-type branch" dispatch directly.

## Tiny-Detail Ledger

Constraint set carried into implementation. Each item must have a clear
home in the design; un-homed items mean the design is incomplete.

| # | Detail                                                              | Coverage in design                                                                          | Source |
|---|---------------------------------------------------------------------|---------------------------------------------------------------------------------------------|--------|
| 1 | 5×5 inclusive `[-2..=+2]` scan, 25 cells                            | `cells_in_5x5_scan(center)` helper in `bridge_state/mod.rs`; called from both repair and destroy triggers | RE §3.1 step C, §3.2, §3.7 |
| 2 | Scan match: cell has anchor-span membership                         | Inside `body_cell_repair_state`, `BridgeRuntimeCell.anchor_span_id.is_some()` filter         | RE §12 + Rust data model |
| 3 | Low vs high dispatcher decision                                     | Implicit in `BridgeRuntimeCell.is_high_bridge` (per cell); no separate Low/High function pair needed in Rust since cell carries the band info | RE §3.1 step C |
| 4 | EVA → sound → scan → state mutation ordering                        | Literal sequence inside `tick_bridge_repair_orders`                                          | RE §3.1 steps A→B→C→D |
| 5 | Audio fires unconditionally on trigger (even if no bridge in scan)  | Sound/EVA emit before the scan call; emits regardless of cell-mutation count                | RE §3.1 |
| 6 | EVA gated on local human (handled at app layer)                     | `SimSoundEvent::BridgeRepaired` carries `owner: InternedId`; app layer compares against local player | RE §3.1 step A |
| 7 | Sound at building location (not engineer)                            | `SimSoundEvent::BridgeRepaired { rx: building.rx, ry: building.ry, ... }`                    | RE §3.1 step B |
| 8 | Sound gated on `repair_sound.is_some()`                              | If-let around the SimSoundEvent emit                                                          | RE §3.1 step B |
| 9 | RNG draw per main-deck strip (not per cell)                          | `body_cell_repair_state` iterates strips; one RNG draw per strip; all 3 cells in strip share variant | RE §12.3 + §12.5 |
| 10 | Bridgehead repair uses fixed base, no RNG draw                       | Match branch in `body_cell_repair_state`: `BridgeCellRole::Bridgehead` writes Healthy{variant: 0} with no RNG | RE §12.4 + §12.5 |
| 11 | Already-healthy cells skipped, no RNG draw                           | Match branch: `DamageState::Healthy { .. }` continues without mutation                       | RE §12.3 |
| 12 | Zones-rebuild gating: main-deck only (NOT bridgehead-only)            | `RepairOutcome.zones_dirty = true` only if a main-deck damaged or destroyed strip was repaired | RE §12.6 |
| 13 | Radar dirty: Destroyed→Healthy transitions only                      | `RepairOutcome.radar_cells` populated only when transitioning from `DamageState::Destroyed`  | RE §12.7 |
| 14 | 3 cells marked dirty per Destroyed→Healthy transition                 | Strip iteration writes to all 3 cells; radar_cells accumulates all 3 when transition fires    | RE §12.7 |
| 15 | Engineer despawn on dispatch completion                              | `despawn_entity(engineer_id)` at end of `tick_bridge_repair_orders` per-engineer block        | RE §3.1 step G |
| 16 | Cell-attribute recalc per mutated cell                                | `body_cell_repair_state` calls `BridgeRuntimeCell` invariant-recompute per mutated cell (existing pattern) | RE §12.8 |
| 17 | Hut survives the C4/demo-truck explosion                             | Early-return in `apply_c4_damage_to_building` before damage application when `target.bridge_repair_hut` | RE §3.2 |
| 18 | Bridge cells go through forward state machine (Damaged → Destroyed)   | Destroy hook calls `body_cell_advance_state` per cell in 5×5; reuses existing forward path  | RE §13.2 |
| 19 | Vanilla `UpdateAdjacentBridges_High` copy-paste bug becomes a no-op   | Our zone rebuild is band-agnostic; comment in destroy hook documents the no-op equivalence  | RE §13.4 |
| 20 | `bridge_state_changed=true` for both repair and destroy ticks         | Both branches set the `TickResult` flag                                                       | Rust TickResult docs |
| 21 | Multiple spans in 5×5 — all processed                                 | `body_cell_repair_state` collects unique span ids into BTreeSet; iterates all                | RE §3.1 step C/D (slight over-coverage vs gamemd's first-match; flagged for verification) |
| 22 | No-bridge in scan → EVA+sound fire, no cell mutation                  | EVA+sound emit unconditionally; `body_cell_repair_state` early-returns on empty scan          | RE §3.1 |
| 23 | Two engineers same tick → both fire repair                            | `tick_bridge_repair_orders` iterates all engineers in deterministic order; each dispatches    | Implied by gamemd's stateless per-engineer dispatch |
| 24 | C4-on-CABHUT in vanilla YR is unreachable (Immune blocks upstream)    | Destroy hook is present but currently dead code; `#[ignore]`'d integration test               | RE §15.2 + project_c4_bridge_hut_followup |
| 25 | Demo-truck path stubbed (no Rust demo-truck unit yet)                  | TODO comment in the destroy helper marking the demo-truck hook location                       | Rust scan |
| 26 | Tag-trigger fire on engineer consume (ProcessCellAction 0x30)         | **Stub** — no Rust tag system yet. No-op with TODO comment.                                   | RE §3.1 step F (deferred per RE §19 Q4) |
| 27 | Repair-listener registry (DAT_00a83dec)                               | **Deferred** — no known consumers affecting observable output in vanilla YR                   | RE §3.1 step E + §19 Q6 |
| 28 | Variant byte stored but possibly unobserved at render time             | Stored regardless (RNG state advance is locked for lockstep); Phase 3 RE verifies observability | bridge_state/mod.rs:96 + RE §19 |

## Design

### Components

**1. `bridge_state::RepairOutcome`** (new struct)
```rust
pub struct RepairOutcome {
    /// At least one main-deck or destroyed cell was repaired — caller
    /// should trigger zones_dirty + refresh_bridge_zones_if_dirty.
    pub zones_dirty: bool,
    /// Cells that transitioned from Destroyed → Healthy — caller
    /// should mark these in the radar minimap dirty list.
    pub radar_cells: Vec<(u16, u16)>,
    /// Total cells whose damage_state changed this call.
    pub repaired_cells: u32,
}
```

**2. `bridge_state::body_cell_repair_state`** (new function)
```rust
pub fn body_cell_repair_state(
    &mut self,
    scan_cells: &[(u16, u16)],
    rng: &mut DeterministicRng,
) -> RepairOutcome
```

Behavior:
1. Collect unique `anchor_span_id`s by looking up each scan cell.
2. For each unique span, iterate per-strip (a strip = the cell + its
   two perpendicular neighbors per the gamemd 3-wide walker pattern).
   The strip iteration order is the span's `body_cells` natural order,
   which must be deterministic across clients.
3. For each strip, classify by the **anchor cell's** current
   `damage_state` (Healthy/Damaged/PartialCollapse{A,B}/Destroyed):
   - `Healthy { .. }` → skip (no RNG, no mutation)
   - `Damaged` or `Destroyed` (main-deck) → draw `variant = rng.next_range_u32(4)`,
     write `DamageState::Healthy { variant: variant as u8 }` to all 3
     strip cells. Add strip cells to `radar_cells` iff the prior state
     was `Destroyed`. Set `zones_dirty = true`.
   - `PartialCollapseA` / `PartialCollapseB` → treat as main-deck damage
     (same as `Damaged`, RNG-pick variant, restore Healthy). gamemd
     doesn't see PartialCollapse in the overlay encoding, but our Rust
     state can be in PartialCollapse if a bridgehead failed mid-cascade.
     Restoring to Healthy is the parity-equivalent of "fully repaired."
4. For Bridgehead-role cells (`BridgeCellRole::Bridgehead`), write
   `DamageState::Healthy { variant: 0 }` with **no RNG draw**. Do NOT
   set `zones_dirty`. Do NOT add to `radar_cells`.
5. Return the accumulated `RepairOutcome`.

The function is pure-on-`bridge_state` (no Simulation borrow) and
testable in isolation with seeded RNG.

**3. `bridge_state::cells_in_5x5_scan`** (new helper)
```rust
pub fn cells_in_5x5_scan(center: (u16, u16)) -> impl Iterator<Item = (u16, u16)>
```

Yields the 25 cells in `[-2..=+2] × [-2..=+2]` around `center`, clamped
to non-negative coordinates. Used by both the repair trigger and the
destroy hook.

**4. `Simulation::tick_bridge_repair_orders`** (new function in `world_orders.rs`)
```rust
pub(crate) fn tick_bridge_repair_orders(&mut self, rules: &RuleSet) -> bool {
    // Iterate engineers with capture_target pointing at a BridgeRepairHut.
    // For each Chebyshev-≤-1 engineer:
    //   1. Emit SimSoundEvent::BridgeRepaired { rx: bld.rx, ry: bld.ry, owner }
    //      (gated on rules.bridge_rules.repair_sound.is_some())
    //   2. scan = cells_in_5x5_scan(engineer.position)
    //   3. outcome = self.bridge_state.body_cell_repair_state(&scan, &mut self.rng)
    //   4. If outcome.zones_dirty: bridge_state_changed |= true (caller sets TickResult)
    //   5. Propagate outcome.radar_cells through existing radar-dirty mechanism
    //   6. despawn_entity(engineer_id)
    // Returns true if any repair fired.
}
```

The function returns a bool that callers OR into `TickResult.bridge_state_changed`.

**5. `Simulation::tick_capture_orders` modification**

Add a branch at the top of the per-engineer loop:
```rust
// If target is a BridgeRepairHut, this engineer's intent is bridge-repair,
// not ownership-transfer. Delegate; do NOT change ownership.
let target_is_bridge_hut = self
    .entities
    .get(building_id)
    .and_then(|b| self.rules.object_type(b.type_id))
    .map_or(false, |t| t.bridge_repair_hut);
if target_is_bridge_hut {
    continue;  // tick_bridge_repair_orders handles this engineer
}
// ... existing capture logic
```

The capture-vs-repair decision happens at tick time (just like gamemd's
PerCellProcess mission-8 branch).

**6. `Simulation::apply_c4_damage_to_building` modification**

```rust
fn apply_c4_damage_to_building(&mut self, building_id, dmg, warhead, attacker, rules) -> bool {
    let bridge_hut = self
        .entities
        .get(building_id)
        .and_then(|b| self.rules.object_type(b.type_id))
        .map_or(false, |t| t.bridge_repair_hut);

    if bridge_hut {
        // Gamemd parity: skip damage to the hut (BuildingClass::Update does
        // NOT call vtable[0x16C] for BridgeRepairHut). Instead, trigger
        // bridge collapse for the 5×5 around the hut.
        let bld_cell = self.entities.get(building_id).map(|b| (b.position.rx, b.position.ry));
        if let Some(center) = bld_cell {
            self.dispatch_bridge_collapse_from_hut(center);
        }
        return false;  // hut not killed; vanilla copy-paste-bug-equivalent (zone rebuild handles both bands)
    }

    // existing logic: damage the hut
}
```

**7. `Simulation::dispatch_bridge_collapse_from_hut`** (new helper)
```rust
fn dispatch_bridge_collapse_from_hut(&mut self, center: (u16, u16)) -> bool {
    // 5×5 scan; for each cell with an anchor_span_id, drive forward state
    // machine to push Healthy → Damaged → Destroyed (loops body_cell_advance_state
    // until the cell reaches Destroyed or NoChange).
    // Sets bridge_state_changed=true on outcome.
    // Unconditionally fires UpdateBridgeZonesHelper-equivalent
    // (refresh_bridge_zones_if_dirty with any_zones_dirty=true).
}
```

Note: gamemd's destroy walker drives the cells in a single pass (it's
not a state machine loop — the destroy walker writes Destroyed
directly). Our Rust state machine takes 2 transitions to reach
Destroyed (Healthy→Damaged→Destroyed). We loop until convergence;
within a single tick, this is the same observable end state.

For PartialCollapseA/B states, one transition is enough to reach
Destroyed (per the existing `body_cell_advance_state` line 877-919).

**8. `SimSoundEvent::BridgeRepaired`** (new variant)
```rust
/// Bridge was just repaired by an engineer. Played at the BUILDING's
/// (CABHUT's) cell, not the engineer's. Owner is the engineer's
/// house — app layer plays EVA_BridgeRepaired if owner is local human,
/// and plays [BridgeRepaired] from soundmd.ini at the position regardless.
BridgeRepaired { rx: u16, ry: u16, owner: InternedId },
```

### Interfaces / Contracts

**Public API on `BridgeRuntimeState`:**
- `body_cell_repair_state(scan: &[(u16, u16)], rng: &mut DeterministicRng) -> RepairOutcome`
- `cells_in_5x5_scan(center: (u16, u16)) -> impl Iterator<...>` (free function or method)
- Existing: `is_bridge_walkable`, `body_cell_advance_state`, `cell`, `cell_mut`,
  `anchor_span`, `anchor_spans` (no signature changes)

**Public API on `Simulation`:**
- `tick_bridge_repair_orders(&mut self, rules: &RuleSet) -> bool`
- `apply_c4_damage_to_building` — signature unchanged; behavior branches
- Existing helpers reused: `despawn_entity`, `keys_sorted`, `rng`, `bridge_state`

**Data flow on engineer-repair trigger:**
```
movement settles engineer at CABHUT cell (Phase 4 — ground movement)
  → tick_capture_orders sees target.bridge_repair_hut=true, skips
  → tick_bridge_repair_orders picks up the engineer
    → SimSoundEvent::BridgeRepaired emitted (drained by app for sound + EVA)
    → body_cell_repair_state(5×5, rng) executes
      → returns RepairOutcome { zones_dirty, radar_cells, repaired_cells }
    → if zones_dirty: TickResult.bridge_state_changed |= true
    → radar_cells fed into render dirty list
    → despawn_entity(engineer)
  → Phase 5+ continues with combat etc.
  → end of tick: refresh_bridge_zones_if_dirty rebuilds PathGrid (existing path)
  → next tick: movement sees updated PathGrid → bridge cells walkable again
```

**Data flow on C4-on-CABHUT destroy hook:**
```
Phase 5 tick_c4_plants: C4 detonation timer expires
  → apply_c4_damage_to_building called
    → branch: target.bridge_repair_hut → dispatch_bridge_collapse_from_hut
      → 5×5 scan
      → For each cell with anchor_span_id:
          loop body_cell_advance_state until Destroyed or NoChange
      → unconditionally set TickResult.bridge_state_changed = true
      → return false (hut survives)
  → next tick: PathGrid rebuilt, bridge cells now non-walkable
```

### Error Handling

- Engineer points at non-existent building: existing `tick_capture_orders`
  logic clears `capture_target` and continues. Same for repair branch.
- Engineer not adjacent: skip this tick (movement still in progress).
- 5×5 scan hits map edge: `cells_in_5x5_scan` clamps to non-negative
  coords; cells outside `BridgeRuntimeState.cells` Vec return `None`
  from `cell()` — `body_cell_repair_state` skips silently.
- Anchor span missing for a cell: skip the cell (consistent with
  existing `body_cell_advance_state` error handling at lines 781-803).

All error paths are silent skips — no panic, no log. Matches gamemd's
sentinel-cell fallback pattern.

### Testing Strategy

**Unit tests for `body_cell_repair_state`** (in `bridge_state/`):

1. **`repair_destroyed_main_deck_cell`** — seed a span with one
   `Destroyed` body cell, call repair, assert:
   - cell's `damage_state` is `Healthy { variant: <known RNG output> }`
   - `outcome.zones_dirty == true`
   - `outcome.radar_cells == vec![(rx, ry), (rx, ry-1), (rx, ry+1)]`
     (strip of 3 — actual perpendicular axis depends on span axis)
   - `outcome.repaired_cells == 3`
   - RNG advanced exactly 1 draw

2. **`repair_damaged_main_deck_cell`** — seed Damaged, assert
   transition to Healthy, `zones_dirty=true`, **no radar_cells**
   (radar only fires on Destroyed→Healthy), 1 RNG draw.

3. **`repair_bridgehead_no_rng_draw`** — seed Damaged bridgehead,
   assert transition to `Healthy { variant: 0 }`, `zones_dirty=false`,
   `radar_cells.is_empty()`, **0 RNG draws** (RNG state unchanged).

4. **`repair_healthy_cell_is_noop`** — seed Healthy, assert no
   mutation, no RNG draw, `zones_dirty=false`.

5. **`repair_full_span_destroyed_to_healthy`** — destroy an entire
   span, repair, assert all cells Healthy, `zones_dirty=true`,
   `radar_cells` matches the destroyed set, RNG draws = number of
   strips (not number of cells).

6. **`repair_determinism_two_runs_same_seed`** — repair the same
   destroyed span twice from the same seed, assert identical variant
   bytes in all cells.

7. **`repair_no_cells_in_scan_is_empty_outcome`** — pass a scan that
   doesn't intersect any anchor span; assert `RepairOutcome::default()`
   (zones_dirty=false, radar_cells empty, repaired=0). RNG unchanged.

8. **`repair_strip_iteration_order_pin`** — repair a known span,
   capture the sequence of RNG draws, assert byte-equal to a pinned
   reference. **Prevents accidental iteration-order regressions.**

9. **`repair_partial_collapse_to_healthy`** — seed PartialCollapseA
   and PartialCollapseB body cells, assert each transitions to
   Healthy{variant} with 1 RNG draw each.

**Integration tests** (in `world/world_orders_tests.rs` or similar):

10. **`engineer_enters_cabhut_repairs_bridge`** — spawn engineer with
    `capture_target` pointing at a CABHUT adjacent to a destroyed
    bridge. Tick once. Assert:
    - engineer despawned
    - bridge cells now Healthy
    - `SimSoundEvent::BridgeRepaired` in `sound_events`
    - `TickResult.bridge_state_changed == true`
    - `is_bridge_walkable(rx, ry)` returns true for the repaired cells

11. **`engineer_enters_intact_cabhut_emits_sound_but_no_mutation`** —
    spawn engineer + CABHUT with healthy bridge. Tick once. Assert
    engineer despawned, `SimSoundEvent::BridgeRepaired` emitted,
    bridge state unchanged, `bridge_state_changed == false`.

12. **`c4_on_cabhut_destroys_bridge`** *(marked `#[ignore]` — blocked
    on `project_c4_bridge_hut_followup` upstream Immune fix)* —
    spawn SEAL/Tanya, plant C4 on CABHUT, run detonation tick. Assert:
    - CABHUT still alive (Immune in INI, but more importantly the
      bridge-repair-hut branch skips damage)
    - adjacent bridge cells Destroyed
    - `TickResult.bridge_state_changed == true`

13. **`two_engineers_both_repair_same_tick`** — spawn two engineers
    pointing at the same CABHUT, both adjacent. Tick once. Assert both
    despawned, both emit BridgeRepaired sound (2 events), bridge
    repaired (idempotent, no RNG-state corruption).

14. **`engineer_at_cabhut_with_no_bridge_in_5x5`** — CABHUT placed far
    from any bridge. Engineer arrives. Assert: engineer despawned,
    sound emitted, `bridge_state_changed == false`, no panic.

**Test isolation:** All tests use the test seam (`test_seed_cell`,
`test_seed_anchor_span`) plus a fixed-seed RNG. No PathGrid build
required for unit tests; integration tests use the existing
`Simulation::test_*` helpers.

### Determinism Considerations

Critical: every Rust client must execute identical RNG state advances
at identical ticks.

- Engineer iteration order: `entities.keys_sorted()` — deterministic.
- Engineer-adjacency check: deterministic Chebyshev arithmetic on `(rx, ry)`.
- 5×5 scan iteration: fixed nested loop `(dy in -2..=2, dx in -2..=2)`.
- Anchor-span collection: `BTreeSet<AnchorSpanId>` — sorted insertion.
- Strip iteration within span: span's `body_cells.iter()` — Vec order,
  set at scenario load. **Locked by test #8 above.**
- RNG calls per strip: `sim.rng.next_range_u32(4)` — single draw per
  main-deck/destroyed strip, zero draws for bridgehead/healthy.

State-hash inclusion: `bridge_state.cells[i].damage_state` is already
hashed. Reverse transitions show up in the hash identically to forward.

Tick ordering: `tick_bridge_repair_orders` runs in Phase 5 alongside
`tick_capture_orders` and `tick_c4_plants`, all BEFORE `tick_combat`.
This places the bridge state mutation BEFORE this tick's combat reads
of `is_bridge_walkable`, so e.g. a unit that just got walkable terrain
restored beneath it sees the updated grid on the *next* tick's
movement (matching gamemd's one-tick-delayed visibility — see comment
on `TickResult.bridge_state_changed` at world/mod.rs:85).

## Architectural Decisions

**Patterns followed:**
- Pure-state-machine for the cell mutation (`body_cell_repair_state`),
  mirroring `body_cell_advance_state`. Same callsite shape, same RNG
  injection, same `RepairOutcome`/`StateOutcome` return-pattern.
- Same-Phase trigger placement next to capture and C4 plants —
  matches gamemd's PerCellProcess location AND keeps the Rust
  trigger-zone semantically coherent.
- `SimSoundEvent` variant for the bridge-repaired sound, drained by
  the app layer — matches every other sim audio dispatch.
- 5×5 scan as a free helper in `bridge_state/mod.rs` — reusable by
  both repair and destroy, matches gamemd's shared scan pattern.

**Patterns deviated from (and why):**
- gamemd uses three function pairs (RepairBridge / ProcessBridgeDestruction
  / DestroyBridge_*_MapInit) where Rust uses one. Justified because
  Rust's anchor-span model captures the bridge as a logical unit,
  removing the need for the gamemd walker's per-cell traversal. The
  observable output is the same; the internal mechanism is cleaner.
- gamemd's `MapClass::UpdateAdjacentBridges_High` neighbor refresh is
  not separately implemented — our zone rebuild is band-agnostic and
  covers the same observable surface. The vanilla copy-paste bug
  (§13.4 of the RE report) becomes a no-op.
- gamemd writes overlay bytes; Rust writes `DamageState` enum values.
  The renderer derives overlay from state. Variant byte storage on
  Healthy may be unobserved at render time — flagged as Phase 3 RE
  follow-up but not blocking.

**Tech debt introduced:**
- The `capture_target` field is overloaded to mean "engineer's
  destination building, whatever action that resolves to." This is
  fine as long as the resolution-time branch is well-named and the
  intent is documented in the field's docstring. Plan to add a
  docstring on `capture_target` clarifying this.
- The destroy hook in `apply_c4_damage_to_building` is dead code in
  vanilla YR until `project_c4_bridge_hut_followup` lands. Marked
  with a TODO comment referencing the memory entry.
- No demo-truck unit yet in Rust; the destroy helper is callable but
  has no caller from the demo-truck side. When demo-truck is
  implemented, its damage path calls
  `dispatch_bridge_collapse_from_hut` directly.

**Determinism preservation:**
- All ordering deterministic (see "Determinism Considerations" above).
- State hash unchanged in mechanism; new mutations show up
  automatically.
- RNG advance is locked by the strip-iteration-order test.

## Alternatives Considered

**Approach B (separate `bridge_repair_target` field on engineer):**
Cleaner separation but adds entity state for no observable benefit.
Gamemd doesn't have a separate field for this — it's a target-type
branch on a shared mission. Rejected for not moving the parity needle
and adding state.

**Approach C (generic `BuildingArrivalIntent` enum):**
Would unify capture / C4 / bridge-repair / future spy-infiltrate into a
single intent enum dispatched by one tick function. Premature
consolidation — speculative for the spy slot, and refactor risk on the
existing capture and C4 systems. Rejected per YAGNI for the unrelated
refactor; can be revisited when spy-infiltrate lands.

**Direct walker port** (per-cell traversal matching gamemd byte-for-byte):
Would replicate `RepairBridgeWalker_*_*` literally — find-start walk
backward, then forward, 3-cell-strip writes, etc. Doesn't fit our
anchor-span data model (we don't have a continuation-by-overlay-band
concept; we have explicit span boundaries). Would add complexity for
no parity benefit since the final cell state is identical. Rejected.

**Pre-built `hut → span` registry at scenario load:**
O(1) lookup at trigger time, but doesn't recompute on mid-bridge
mutation. Could miss edge cases that gamemd's per-trigger 5×5 scan
catches. Rejected for parity-preservation.

---

## Next Steps

Hand off to `/write-plan` for task decomposition. The plan should
break this design into these milestones (rough sketch):

1. **`bridge_state::body_cell_repair_state`** core + unit tests 1–9.
   Standalone, no Simulation borrow, no UI/render integration. ~200
   lines + tests.
2. **`SimSoundEvent::BridgeRepaired` variant + app-layer wire-up**
   for the sound and EVA. ~30 lines in sim + app layer.
3. **`tick_bridge_repair_orders` + `tick_capture_orders` branch**
   + integration tests 10, 11, 13, 14. ~100 lines + tests.
4. **`apply_c4_damage_to_building` branch + `dispatch_bridge_collapse_from_hut`**
   + integration test 12 (`#[ignore]`). ~80 lines + test.
5. **Demo-truck stub:** TODO comment in the dispatch helper marking
   where the unit's damage path will call in.
6. **Docstring updates:** clarify `capture_target` overload;
   reference `project_c4_bridge_hut_followup` in the destroy hook;
   note vanilla `UpdateAdjacentBridges_High` no-op equivalence.
