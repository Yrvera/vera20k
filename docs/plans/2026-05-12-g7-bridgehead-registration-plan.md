# G7 Bridgehead Registration — Implementation Plan

Design doc: [2026-05-12-g7-bridgehead-registration-design.md](./2026-05-12-g7-bridgehead-registration-design.md).

Scope is **bridgehead registration only**. Two G7 follow-ups stay deferred and
MUST NOT be bundled into this PR:

- `production_placement.rs:391` — pending RE on whether gamemd allows naval-yard
  placement under destroyed bridge gaps.
- `movement_path.rs:23-24` — pending `/re-investigate` on
  `TooBigToFitUnderBridge` destroyed-bridge semantics.

---

## Walker-safety audit (verdicts, sourced 2026-05-12)

Every site that iterates `BridgeRuntimeState.cells` or otherwise enumerates
bridge cells, with its verdict post-registration. **Two sites need a code
change** to keep determinism + the design contract intact; the rest are safe
as-is.

| Site | What it does | Verdict |
| --- | --- | --- |
| [src/sim/bridge_state/mod.rs:1386](../../src/sim/bridge_state/mod.rs#L1386) `refresh_endpoint_active_flags` | Iterates `self.cells`; collects `bridge_group_id`s of `Destroyed` cells | **SAFE** — pass-4 bridgeheads have `bridge_group_id = None`, so they're filtered by the `if let Some(gid)` guard at line 1391. Also, pass-4 bridgeheads stay `Healthy` permanently, so they never reach the `Destroyed` outer check. |
| [src/sim/bridge_state/mod.rs:797](../../src/sim/bridge_state/mod.rs#L797) `body_cell_advance_state` | Body state machine | **SAFE** — input gate at lines 811-815 rejects `Bridgehead`; all writes target the resolved anchor (Anchor role); `update_ramp_perpendicular` (gated on Anchor role at [bridge_specs.rs:560](../../src/sim/bridge_specs.rs#L560)) cannot reach bridgeheads. |
| [src/sim/bridge_state/mod.rs:1078](../../src/sim/bridge_state/mod.rs#L1078) `body_cell_repair_state` | Repair state machine | **SAFE** — pass-4 bridgeheads have `anchor_span_id = None`, so they're never added to the `spans` set at line 1090 and the per-span `cells_list` loop never reaches them. The explicit `Bridgehead` arm at line 1121 (which would write `Healthy { variant: 0 }` with 0 RNG draws) is defense-in-depth — it covers pass-3 bridgeheads (which DO have `anchor_span_id`) without breaking the deterministic RNG ordering. |
| [src/sim/bridge_state/mod.rs:1213](../../src/sim/bridge_state/mod.rs#L1213) `bridgehead_advance_state` | Bridgehead state machine (Task 14) | **SAFE under scoped Task 3** — function bails on `axis = None` at line 1230; pass-4 bridgeheads will have `axis = None`. After Task 3's `Bridgehead && axis.is_none()` exclusion, the dispatcher never routes pass-4 cells here, so no RNG burn. Pass-3 bridgeheads (axis = Some) keep their existing routing. Existing Task 14 unit tests call this directly via `test_seed_cell` with explicit axis — those keep passing. |
| [src/sim/bridge_state/mod.rs:681](../../src/sim/bridge_state/mod.rs#L681) `path_matches_cell` (HighSM / LowSM) | Dispatcher entry gate | **NEEDS FIX (Task 3)** — lines 709-717 include `Bridgehead` in the allowed role list. Today the gate is harmless because no pass-4 bridgeheads exist (all live `Bridgehead` cells are pass-3, with axis = Some). Post-pass-4-fix, pass-4 cells (axis = None) match the gate → dispatcher rolls the BridgeStrength RNG → `bridgehead_advance_state` returns `NoChange` (axis None) → one wasted RNG draw per damage event landing on a bridgehead → **lockstep desync**. Reject only `Bridgehead && axis.is_none()` to preserve pass-3 routing. |
| [src/sim/bridge_state/walker.rs:279, 334, 635, 684](../../src/sim/bridge_state/walker.rs#L279) `apply_bridge_destruction_*` (NS/EW × HIGH/LOW) | Overlay-direct sibling-cascade leaves; triple-writes (this, north, south) or (this, west, east) | **NEEDS FIX (Task 2)** — input cell is gated by body-overlay range, but the **triple-write loop is not**. If a body cell is adjacent to a pass-4 bridgehead along the body axis, the triple write corrupts the bridgehead's `overlay_byte` and flips its `damage_state` to `Damaged`/`Destroyed`. Pre-fix this was a no-op (`cell_mut` returned `None`); post-fix it's a regression. Add a `role == Bridgehead` skip to each `cell_mut` write. (Pass-3 bridgeheads were already reachable here pre-fix — the skip is a behavior change for them; see note below the audit table.) |
| [src/sim/bridge_state/walker.rs:396, 487, 741, 826](../../src/sim/bridge_state/walker.rs#L396) `destroy_bridge_walker_*_*` (NS/EW × HIGH/LOW) | Overlay-direct walker bodies; same triple-write pattern | **NEEDS FIX (Task 2)** — same issue as the sibling-cascade leaves. Same role-skip patch. |
| [src/sim/bridge_state/mod.rs:1404](../../src/sim/bridge_state/mod.rs#L1404) `iter_cells` (consumed by [bridges.rs:118,237](../../src/app_instances/bridges.rs#L118) — body / shadow — and [bridges.rs:316](../../src/app_instances/bridges.rs#L316) — railing) | Render sprite instance builders | **SAFE** — body + shadow sites short-circuit on `!is_high_bridge_body_name(name)`. The railing site uses `resolve_bridge_kind_and_sub_idx` (different filter), which returns None when `overlay_names.get(&overlay_byte)` is None. Pass-4 bridgeheads have `overlay_byte = 0` and no entry in `overlay_names` → all three sites `continue`. Bridgeheads never reach the body atlas. |
| [src/sim/world/world_hash.rs:213](../../src/sim/world/world_hash.rs#L213) `hash_bridge_state` | Determinism hash | **HASH BUMP** — `iter_cells()` grows by the bridgehead count; the hash changes once. No existing test hardcodes a hash value (all comparison-based), so no test edit is required. Replay protocol does not exist yet; acceptable per design doc §"Determinism Considerations". |
| [src/sim/world/bridge_orchestrator.rs:174](../../src/sim/world/bridge_orchestrator.rs#L174) `dispatch_bridge_collapse_from_hut` (5×5 scan around hut) | Drives every bridge cell in scan to convergence via `body_cell_advance_state` | **SAFE** — line 197 filters `anchor_span_id.is_some()`; pass-4 bridgeheads have `anchor_span_id = None` and are skipped before any state-machine call. |
| [src/sim/world/world_orders.rs:258](../../src/sim/world/world_orders.rs#L258) `tick_bridge_repair_orders` (5×5 scan around engineer, calls `body_cell_repair_state` at line 342) | Calls `body_cell_repair_state(&scan, …)` | **SAFE** — see `body_cell_repair_state` row above; pass-4 bridgeheads never enter that loop's per-span `cells_list`. |
| [src/sim/world/bridge_orchestrator.rs:326](../../src/sim/world/bridge_orchestrator.rs#L326) `update_adjacent_bridges` rim refresh | Walks from rim cell toward a bridgehead candidate; resets dangling stubs | **SAFE** — `is_head_candidate` check at line 355 already accepts `BridgeCellRole::Bridgehead` as a beacon (intended). Reset writes at line 392 only fire when `stub_now` is true (anchor_span_id pointing to a vanished span); bridgeheads have `anchor_span_id = None`, so `stub_now` evaluates to `false` and the reset is skipped. **Determinism note:** post-fix, more rim cells will find a head candidate (any pass-4 bridgehead, not just `Destroyed` cells), which can change which DIRECTIONS slot wins `head_dir`. Walking past a healthy bridgehead is a no-op (`stub_now = false`), so observable bridge-state output is unchanged; only the walker's iteration trace differs. |

**Summary:** Tasks 2 and 3 are walker-safety fixes that MUST land alongside the
registration in Task 1. Task 4 fixes the test fixture that papers over the
bug. Tasks 5–7 add coverage and integration assertions.

`state_hash` bump: expected and one-shot. No replay protocol exists yet.

### Two kinds of `Bridgehead` role after the fix

The `Bridgehead` role enum now has two populations with different field
shapes. Tasks 2 and 3 are written to be safe for both, but the distinction
is load-bearing for future readers:

| Property | Pass-3 bridgeheads (existing) | Pass-4 bridgeheads (new, this PR) |
| --- | --- | --- |
| Source | [mod.rs:575-598](../../src/sim/bridge_state/mod.rs#L575) — RE-roles existing pass-1 cells whose `bridge_layer.overlay_id` is not anchor (0x18 / 0x19 / 0xED / 0xEE) | [mod.rs Task 1, new pass] — creates new cells where `ResolvedTerrainCell.bridge_walkable && !has_bridge_deck` |
| `has_bridge_deck` of source terrain | `true` (registered by pass 1) | `false` (skipped by pass 1) |
| `axis` | `Some(...)` from `bridge_direction_to_axis(bl.direction)` | `None` |
| `anchor_span_id` | `None` (cleared by pass 3) | `None` |
| `bridge_group_id` | `Some(...)` (set by pass 1) | `None` |
| `overlay_byte` | `bl.overlay_id` | `0` |
| `damage_state` | mutable | permanently `Healthy { variant: 0 }` |

**Task 2's walker role-skip** changes behavior for pass-3 bridgeheads too:
pre-fix the triple-write would hit them; post-fix it skips. This is the
correct direction (pass-3 cells were never supposed to be overlay-write
targets of body-axis walkers — they're ramp connectors), but it IS a
behavior change beyond the pass-4 use case. The Task 5 regression test
exercises both populations.

**Task 3's scoped exclusion** (`Bridgehead && axis.is_none()`) keeps pass-3
bridgeheads in `path_matches_cell`'s allowed list so the dispatcher continues
to route them to `bridgehead_advance_state` (where their axis = Some lets
the state machine actually fire). Only pass-4 cells get rejected.

---

## Task 1 — Add pass 4 (bridgehead registration) to `from_resolved_terrain`

**File:** [src/sim/bridge_state/mod.rs](../../src/sim/bridge_state/mod.rs)

**Edit:** After pass 3 (closes at line 598), just before `let endpoint_records = …`
at line 600, insert a new pass that creates `BridgeRuntimeCell` records for
every terrain cell with `bridge_walkable && !has_bridge_deck` — the
bridgehead-exclusive signal per design doc §"Tiny-Detail Ledger" item #12.

```rust
        // Pass 4: register bridgehead cells. ResolvedTerrainCell sets
        // bridge_walkable=true and has_bridge_deck=false at every bridgehead
        // (see resolved_terrain.rs bridgehead pass). Bridgeheads are NOT
        // created in pass 1 (no deck) and NOT touched by pass 3 (no
        // bridge_layer). Without this pass the rebuild silently flips
        // PathCell.bridge_walkable to false on every rebuild_dynamic_path_grid.
        //
        // Contract (load-bearing — see design doc §"Interfaces / Contracts"):
        //   deck_present=true permanently, damage_state=Healthy permanently,
        //   bridge_group_id=None, anchor_span_id=None, axis=None,
        //   overlay_byte=0. The dispatcher (`path_matches_cell` HighSM/LowSM)
        //   rejects `Bridgehead && axis.is_none()` so no damage-event RNG
        //   fires on these cells. Pass-3 bridgeheads (axis=Some) remain in
        //   the gate's allowed set — see the "Two kinds of Bridgehead role"
        //   note above the tasks.
        for cell in terrain.iter() {
            if !cell.bridge_walkable || cell.has_bridge_deck {
                continue;
            }
            let Some(idx) = index_of(width, height, cell.rx, cell.ry) else {
                continue;
            };
            if cells[idx].is_some() {
                // Defensive: a cell registered in pass 1 (has_bridge_deck)
                // also satisfying bw && !has_deck would be a contradiction;
                // pass 1 wins to avoid clobbering body metadata.
                continue;
            }
            cells[idx] = Some(BridgeRuntimeCell {
                deck_present: true,
                destroyable,
                deck_level: cell.bridge_deck_level,
                bridge_group_id: None,
                damage_state: DamageState::Healthy { variant: 0 },
                axis: None,
                role: BridgeCellRole::Bridgehead,
                anchor_span_id: None,
                overlay_byte: 0,
                damaged_variant: false,
            });
        }

```

**Verification:**

1. `cargo check -p ra2 --lib` compiles.
2. `cargo test -p ra2 --lib bridge_state::tests` — existing tests stay green.

---

## Task 2 — Walker triple-write role skip (4 walker bodies + 4 cascade leaves)

**File:** [src/sim/bridge_state/walker.rs](../../src/sim/bridge_state/walker.rs)

**Edit:** Each triple-write loop currently looks like:

```rust
for (slot, opt_pos) in Self::ns_triple(rx, ry).into_iter().enumerate() {
    if let Some(pos) = opt_pos {
        if let Some(c) = self.cell_mut(pos.0, pos.1) {
            c.overlay_byte = next;
            // ...
        }
    }
}
```

Replace each `if let Some(c) = self.cell_mut(...)` with the role-skip:

```rust
        if let Some(c) = self.cell_mut(pos.0, pos.1) {
            if matches!(c.role, crate::sim::bridge_state::BridgeCellRole::Bridgehead) {
                // Pass 4 contract: bridgeheads are never mutated by the
                // overlay-direct walker. A body cell adjacent to a bridgehead
                // along the body axis lands here via the triple write;
                // pre-registration this was silently a no-op (no cell).
                continue;
            }
            c.overlay_byte = next;
            // ...
        }
```

Eight sites need this fix, all in [walker.rs](../../src/sim/bridge_state/walker.rs):

| Function | Line of `cell_mut` write |
| --- | --- |
| `apply_bridge_destruction_ns_high` | [317](../../src/sim/bridge_state/walker.rs#L317) |
| `apply_bridge_destruction_ew_high` | [367](../../src/sim/bridge_state/walker.rs#L367) |
| `destroy_bridge_walker_ns_high` | [437](../../src/sim/bridge_state/walker.rs#L437) |
| `destroy_bridge_walker_ew_high` | [526](../../src/sim/bridge_state/walker.rs#L526) |
| `apply_bridge_destruction_ns_low` | [668](../../src/sim/bridge_state/walker.rs#L668) |
| `apply_bridge_destruction_ew_low` | [717](../../src/sim/bridge_state/walker.rs#L717) |
| `destroy_bridge_walker_ns_low` | [780](../../src/sim/bridge_state/walker.rs#L780) |
| `destroy_bridge_walker_ew_low` | [865](../../src/sim/bridge_state/walker.rs#L865) |

**Subtlety:** the destruction loops set `c.damage_state` inside the same
`if let Some(c)` block. The `continue` must live AFTER `cell_mut` returns Some
so the skip applies to *both* the overlay write and the damage-state write.
Use `continue` (skips this triple slot) rather than `return` (would abort the
walker).

**Verification:**

1. `cargo check -p ra2 --lib`.
2. `cargo test -p ra2 --lib bridge_state::walker` — Task 7/8 walker tests
   stay green (they don't seed bridgeheads in triples).
3. Add the regression test in Task 5 (`bridgehead_survives_adjacent_body_collapse`).

---

## Task 3 — Scope-exclude pass-4 bridgeheads from `path_matches_cell` HighSM/LowSM role gate

**File:** [src/sim/bridge_state/mod.rs](../../src/sim/bridge_state/mod.rs)

**Edit at lines 709–717:**

```rust
                if !matches!(
                    cell.role,
                    BridgeCellRole::Anchor
                        | BridgeCellRole::Body
                        | BridgeCellRole::Tail
                        | BridgeCellRole::Bridgehead
                ) {
                    return false;
                }
```

becomes

```rust
                if !matches!(
                    cell.role,
                    BridgeCellRole::Anchor
                        | BridgeCellRole::Body
                        | BridgeCellRole::Tail
                        | BridgeCellRole::Bridgehead
                ) {
                    return false;
                }
                // Pass-4 bridgeheads (created by `from_resolved_terrain`'s
                // bridgehead pass) have axis = None and would cause
                // `bridgehead_advance_state` to return NoChange — but only
                // after the per-path BridgeStrength RNG roll already burned a
                // draw. Reject them here so the dispatcher never rolls RNG
                // for a pass-4-targeted event (lockstep). Pass-3 bridgeheads
                // (axis = Some, registered from `bridge_layer.direction`) keep
                // their existing routing into Task 14's state machine.
                if matches!(cell.role, BridgeCellRole::Bridgehead) && cell.axis.is_none() {
                    return false;
                }
```

**Subtlety:** Existing Task 14 tests (`bridgehead_advance_*` in mod.rs around
lines 2466–2592) call `bridgehead_advance_state` **directly**, bypassing
`path_matches_cell`. They keep working. The dispatcher's bridgehead branch at
[bridge_orchestrator.rs:646](../../src/sim/world/bridge_orchestrator.rs#L646)
remains live for pass-3 bridgeheads (axis = Some) — no behavior change for
real maps that exercise the existing pass-3 path. Only pass-4 (newly created)
bridgeheads are rejected.

**Verification:**

1. `cargo check -p ra2 --lib`.
2. `cargo test -p ra2 --lib bridge_state::tests::path_matches_` — existing
   path_matches tests (lines 2665–2818) pass; none seed pass-4-shaped
   `Bridgehead` (role = Bridgehead AND axis = None).
3. Manual: confirm there is no test that seeds `role: BridgeCellRole::Bridgehead`
   with `axis: None` and then asserts `path_matches_cell(HighStateMachine, …) == true`.
   (Existing pass-3-shaped fixtures with `axis: Some(...)` continue to pass
   the gate — that's intentional.)

---

## Task 4 — Fix `test_layered_path_rebuild_blocks_destroyed_bridge_deck` fixture + destruction loop

**File:** [src/sim/pathfinding/core_tests.rs](../../src/sim/pathfinding/core_tests.rs)

**Problem:** The test at line 655 papers over the G7 bug TWO ways:

1. It sets `has_bridge_deck: true` on the two "bridgehead" cells (rx=1 and
   rx=3), making them register as body cells in `BridgeRuntimeState`.
   Realistic bridgehead semantics require `has_bridge_deck=false`.
2. The destruction loop at lines 719–723 forcibly sets `damage_state` to
   `Destroyed` on **all three** cells (rx=1, 2, 3), not just the body
   cell. Post-Task-1, that would mutate pass-4 bridgehead cells' damage
   state — violating the contract that pass-4 bridgeheads stay
   `Healthy { variant: 0 }` permanently. The destruction must be scoped
   to the body cell (rx=2) so the test actually exercises "bridgeheads
   survive body collapse".

**Edit at lines 666–672 and 683–689** (fixture):

```rust
            ResolvedTerrainCell {
                bridge_walkable: true,
                bridge_transition: true,
                bridge_deck_level: 4,
                has_bridge_deck: true,
                ..make_resolved_cell(1, 0)
            },
```

becomes (for both rx=1 and rx=3):

```rust
            ResolvedTerrainCell {
                bridge_walkable: true,
                bridge_transition: true,
                bridge_deck_level: 4,
                // Realistic bridgehead semantics: walkable, transition cell,
                // but NOT part of the deck — `has_bridge_deck=false`. Pass 4
                // of BridgeRuntimeState::from_resolved_terrain registers
                // these as Bridgehead-role cells so PathCell.bridge_walkable
                // survives rebuild.
                has_bridge_deck: false,
                ..make_resolved_cell(1, 0)  // and (3, 0) for the south cell
            },
```

**Edit at lines 719–723** (destruction loop):

```rust
    for (rx, ry) in [(1u16, 0u16), (2, 0), (3, 0)] {
        if let Some(c) = bridge_state.cell_mut(rx, ry) {
            c.damage_state = crate::sim::bridge_state::DamageState::Destroyed;
        }
    }
```

becomes

```rust
    // Destroy only the body cell (rx=2). The bridgeheads at rx=1 / rx=3 must
    // stay Healthy — pass 4 of `from_resolved_terrain` creates them with
    // `damage_state = Healthy { variant: 0 }` permanently, and the contract
    // for `BridgeCellRole::Bridgehead` is that no code mutates that field.
    if let Some(c) = bridge_state.cell_mut(2, 0) {
        c.damage_state = crate::sim::bridge_state::DamageState::Destroyed;
    }
```

**Verification:**

1. `cargo test -p ra2 --lib pathfinding::core_tests::test_layered_path_rebuild_blocks_destroyed_bridge_deck`
   — must still pass. The body cell at rx=2 is `Destroyed` →
   `state.is_bridge_walkable(2, 0) = false` → `PathCell.bridge_walkable = false`
   at rx=2 → A* cannot traverse the bridge layer through the middle cell →
   no Ground→Bridge→Ground path from (0,0) to (4,0). Bridgeheads at rx=1 / rx=3
   remain walkable; the test now actually demonstrates that bridgehead
   walkability is preserved while body collapse blocks the route.

---

## Task 5 — Unit tests in `bridge_state/mod.rs`

**File:** [src/sim/bridge_state/mod.rs](../../src/sim/bridge_state/mod.rs)

Add these tests inside `#[cfg(test)] mod tests` (after the existing
`make_bridge_terrain` helper). They cover the design doc's testing strategy
items #1, #2, #3 plus the walker-safety regression for Task 2.

Helper — bridge with explicit bridgehead cells flanking the deck:

```rust
    /// 5×1 grid: ground(0,0), bridgehead(1,0), body(2,0), bridgehead(3,0),
    /// ground(4,0). Bridgeheads carry the realistic resolved-terrain shape:
    /// `bridge_walkable=true`, `has_bridge_deck=false`, `transition=true`,
    /// `bridge_deck_level=4`. The body cell at (2,0) has has_bridge_deck=true.
    fn make_bridge_with_bridgeheads_terrain() -> ResolvedTerrainGrid {
        let mut cells = Vec::new();
        for rx in 0..5u16 {
            let is_body = rx == 2;
            let is_head = rx == 1 || rx == 3;
            cells.push(ResolvedTerrainCell {
                rx,
                ry: 0,
                source_tile_index: 0,
                source_sub_tile: 0,
                final_tile_index: 0,
                final_sub_tile: 0,
                level: 0,
                filled_clear: false,
                tileset_index: Some(0),
                land_type: 0,
                slope_type: 0,
                template_height: 0,
                render_offset_x: 0,
                render_offset_y: 0,
                terrain_class: TerrainClass::Clear,
                speed_costs: SpeedCostProfile::default(),
                is_water: false,
                is_cliff_like: false,
                is_cliff_redraw: false,
                variant: 0,
                is_rough: false,
                is_road: false,
                accepts_smudge: false,
                has_ramp: false,
                canonical_ramp: None,
                ground_walk_blocked: is_body,
                terrain_object_blocks: false,
                overlay_blocks: false,
                zone_type: 0,
                base_ground_walk_blocked: false,
                base_build_blocked: false,
                build_blocked: is_body,
                has_bridge_deck: is_body,
                bridge_walkable: is_body || is_head,
                bridge_transition: is_head,
                bridge_deck_level: if is_body || is_head { 4 } else { 0 },
                bridge_layer: None,
                radar_left: [0, 0, 0],
                radar_right: [0, 0, 0],
                has_damaged_data: false,
            });
        }
        ResolvedTerrainGrid::from_cells(5, 1, cells)
    }
```

Test #1 — registration:

```rust
    #[test]
    fn bridgeheads_registered_with_bridgehead_role() {
        let state =
            BridgeRuntimeState::from_resolved_terrain(&make_bridge_with_bridgeheads_terrain(), true, 300);
        for rx in [1u16, 3] {
            let cell = state.cell(rx, 0).expect("bridgehead cell must register");
            assert!(matches!(cell.role, BridgeCellRole::Bridgehead));
            assert!(cell.deck_present, "bridgeheads carry deck_present=true");
            assert!(matches!(cell.damage_state, DamageState::Healthy { variant: 0 }));
            assert!(cell.bridge_group_id.is_none());
            assert!(cell.anchor_span_id.is_none());
            assert!(cell.axis.is_none());
            assert_eq!(cell.deck_level, 4);
        }
    }
```

Test #2 — walkability:

```rust
    #[test]
    fn bridgehead_is_bridge_walkable_returns_true() {
        let state =
            BridgeRuntimeState::from_resolved_terrain(&make_bridge_with_bridgeheads_terrain(), true, 300);
        assert!(state.is_bridge_walkable(1, 0));
        assert!(state.is_bridge_walkable(3, 0));
    }
```

Test #3 — survives body collapse:

```rust
    #[test]
    fn bridgehead_survives_body_cell_collapse() {
        let mut state =
            BridgeRuntimeState::from_resolved_terrain(&make_bridge_with_bridgeheads_terrain(), true, 50);
        // Body cell at (2,0): force to Destroyed. (No real damage path here —
        // direct mutation matches the lower-level test pattern at line 1686.)
        if let Some(c) = state.cell_mut(2, 0) {
            c.damage_state = DamageState::Destroyed;
        }
        // Bridgeheads stay walkable.
        assert!(state.is_bridge_walkable(1, 0));
        assert!(state.is_bridge_walkable(3, 0));
        assert!(matches!(
            state.cell(1, 0).unwrap().damage_state,
            DamageState::Healthy { variant: 0 }
        ));
        assert!(matches!(
            state.cell(3, 0).unwrap().damage_state,
            DamageState::Healthy { variant: 0 }
        ));
        // Body cell is gone.
        assert!(!state.is_bridge_walkable(2, 0));
    }
```

Test #4 — walker triple-write must not corrupt adjacent bridgehead
(regression test for Task 2). The walker writes the (this, north, south)
triple. With a 1D NS bridge layout, `north` and `south` of a body cell can
hit bridgeheads.

```rust
    #[test]
    fn ns_walker_triple_skips_bridgehead_neighbors() {
        use crate::sim::bridge_state::DamageState;
        // 5×3 NS bridge: head(2,1), body(2,2), head(2,3) — the walker
        // triple-writes (this, north=2,1, south=2,3) which would corrupt the
        // bridgeheads if Task 2's role skip is missing.
        let mut state = BridgeRuntimeState::default();
        // Seed body at (2,2) with HIGH-NS pre-final overlay 0xD3.
        state.test_seed_cell(
            2, 2,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 4,
                bridge_group_id: Some(1),
                damage_state: DamageState::Healthy { variant: 0 },
                axis: Some(Axis::NS),
                role: BridgeCellRole::Body,
                anchor_span_id: Some(1),
                overlay_byte: 0xD3,
                damaged_variant: false,
            },
        );
        // Seed bridgeheads north + south.
        for ry in [1u16, 3] {
            state.test_seed_cell(
                2, ry,
                BridgeRuntimeCell {
                    deck_present: true,
                    destroyable: true,
                    deck_level: 4,
                    bridge_group_id: None,
                    damage_state: DamageState::Healthy { variant: 0 },
                    axis: None,
                    role: BridgeCellRole::Bridgehead,
                    anchor_span_id: None,
                    overlay_byte: 0,
                    damaged_variant: false,
                },
            );
        }
        let terrain = crate::map::resolved_terrain::ResolvedTerrainGrid::from_cells(3, 4, Vec::new());

        let _ = state.destroy_bridge_walker_ns_high(2, 2, &terrain);

        // Bridgeheads must be untouched.
        for ry in [1u16, 3] {
            let head = state.cell(2, ry).expect("bridgehead survives walker");
            assert_eq!(head.overlay_byte, 0, "bridgehead overlay_byte untouched");
            assert!(matches!(head.damage_state, DamageState::Healthy { variant: 0 }));
            assert!(matches!(head.role, BridgeCellRole::Bridgehead));
        }
    }
```

**Note:** `ResolvedTerrainGrid::from_cells(3, 4, Vec::new())` creates an
empty grid; the walker only consults its own `cells[]` (not terrain) for
the triple write, so the empty terrain is fine. If a future walker change
adds terrain lookups, this test will need a populated grid.

**Verification:**

1. `cargo test -p ra2 --lib bridge_state::tests::bridgeheads_registered_with_bridgehead_role`
2. `cargo test -p ra2 --lib bridge_state::tests::bridgehead_is_bridge_walkable_returns_true`
3. `cargo test -p ra2 --lib bridge_state::tests::bridgehead_survives_body_cell_collapse`
4. `cargo test -p ra2 --lib bridge_state::tests::ns_walker_triple_skips_bridgehead_neighbors`

---

## Task 6 — Integration test in `core_tests.rs`: cross-rebuild PathCell invariant

**File:** [src/sim/pathfinding/core_tests.rs](../../src/sim/pathfinding/core_tests.rs)

Add (after `test_layered_path_rebuild_blocks_destroyed_bridge_deck` near
line 743):

```rust
#[test]
fn test_pathcell_bridge_walkable_preserved_for_bridgeheads_across_rebuild() {
    // Reuses the realistic bridgehead fixture from Task 4. After the G7 fix,
    // PathCell.bridge_walkable for bridgeheads (rx=1, rx=3) must stay true
    // across every PathGrid rebuild driven by rebuild_dynamic_path_grid
    // (which calls from_resolved_terrain_with_bridges).
    let terrain = ResolvedTerrainGrid::from_cells(
        5,
        1,
        vec![
            ResolvedTerrainCell { level: 4, ..make_resolved_cell(0, 0) },
            ResolvedTerrainCell {
                bridge_walkable: true,
                bridge_transition: true,
                bridge_deck_level: 4,
                has_bridge_deck: false,
                ..make_resolved_cell(1, 0)
            },
            ResolvedTerrainCell {
                ground_walk_blocked: true,
                build_blocked: true,
                base_build_blocked: true,
                bridge_walkable: true,
                bridge_deck_level: 4,
                has_bridge_deck: true,
                is_water: true,
                ..make_resolved_cell(2, 0)
            },
            ResolvedTerrainCell {
                bridge_walkable: true,
                bridge_transition: true,
                bridge_deck_level: 4,
                has_bridge_deck: false,
                ..make_resolved_cell(3, 0)
            },
            ResolvedTerrainCell { level: 4, ..make_resolved_cell(4, 0) },
        ],
    );
    let bridge_state = BridgeRuntimeState::from_resolved_terrain(&terrain, true, 10);

    // Multiple rebuilds (each fires from a non-bridge event in production:
    // unit spawn, ownership change, destroyed structure).
    for _ in 0..3 {
        let grid = PathGrid::from_resolved_terrain_with_bridges(&terrain, Some(&bridge_state));
        let pc1 = grid.get(1, 0).expect("bridgehead cell exists");
        let pc3 = grid.get(3, 0).expect("bridgehead cell exists");
        assert!(pc1.bridge_walkable, "rx=1 bridgehead must remain bridge_walkable");
        assert!(pc3.bridge_walkable, "rx=3 bridgehead must remain bridge_walkable");
        assert!(pc1.transition, "rx=1 bridgehead must remain a transition cell");
        assert!(pc3.transition, "rx=3 bridgehead must remain a transition cell");
    }
}
```

**Verification:**

1. `cargo test -p ra2 --lib pathfinding::core_tests::test_pathcell_bridge_walkable_preserved_for_bridgeheads_across_rebuild`
2. Sanity: comment out Task 1's pass-4 code and confirm this test fails
   (bridgeheads not registered → `state.is_bridge_walkable(1,0) == false` →
   `PathCell.bridge_walkable = false`). Uncomment and re-run.

---

## Task 7 — Integration test in `world_tests.rs`: cross-trigger invariant + A* layered path

**File:** [src/sim/world/world_tests.rs](../../src/sim/world/world_tests.rs)

Two integration tests at the end of the file. These cover design doc testing
strategy items #6 and #7.

Test #6 — bridgehead walkability survives all three non-bridge rebuild
triggers (`destroyed_structure`, `ownership_changed`, `spawned_entities`):

```rust
#[test]
fn test_bridgehead_walkability_invariant_across_non_bridge_rebuild_triggers() {
    // Set up a sim with the realistic bridgehead fixture from Task 6.
    // Fire each rebuild trigger; assert PathCell.bridge_walkable for the
    // bridgehead cells is true at every step. Reuses the helper from Task 6
    // — extract `make_bridgehead_terrain()` into a shared fn if the same
    // shape is used twice. (See implementation note below.)
    //
    // The three triggers are simulated via rebuild_dynamic_path_grid called
    // directly; in production app_sim_tick.rs:549-557 fires this on each of
    // the three signal events. We assert the per-grid bridgehead state is
    // invariant.
    //
    // Implementation: build Simulation, attach the terrain + bridge_state,
    // call sim.rebuild_dynamic_path_grid() (or the lower-level path grid
    // rebuild), and inspect sim.path_grid().cell(1, 0).bridge_walkable.
    //
    // (Concrete code body deferred — depends on the exact rebuild API
    // available in Simulation. The test plan landed during task 7
    // execution should look at sim/world/mod.rs for the right entrypoint
    // before fixing the function body.)
}
```

**Implementation note for Task 7:** The exact rebuild API in `Simulation`
needs a quick look during task execution. Candidates:

- `Simulation::rebuild_dynamic_path_grid` — if it exists and is callable
  from test code.
- Direct `PathGrid::from_resolved_terrain_with_bridges(&terrain, Some(&bs))`
  followed by `sim.path_grid = new_grid`.
- A test-only helper that simulates the `app_sim_tick.rs` triggers.

Pick the closest-fit existing pattern in `world_tests.rs` (search for tests
that already exercise `rebuild_dynamic_path_grid` or PathGrid rebuilds in
the world layer) and follow it.

Test #7 — A* finds a layered path before and after an unrelated building
death:

```rust
#[test]
fn test_layered_astar_can_traverse_bridge_after_unrelated_building_death() {
    // Set up a sim with: a 5×1 high bridge (ground, bridgehead, body,
    // bridgehead, ground) at h=4 ground and deck_level=4, plus an
    // unrelated building at e.g. (10, 5) that doesn't touch the bridge.
    //
    // Step 1: verify A* finds a Ground→Bridge→Ground path from (0,0) to
    //         (4,0). Should succeed pre-fix (intact bridge) and post-fix.
    //         The "before" rebuild check is implicit — sim init builds
    //         the path grid once.
    //
    // Step 2: kill the unrelated building (fires the `destroyed_structure`
    //         rebuild trigger). Re-verify the same A* path still exists.
    //         PRE-G7 BUG: the rebuild flips bridgeheads' bridge_walkable
    //         from true → false, and A* cannot enter the bridge layer at
    //         the bridgehead → no path.
    //         POST-G7 FIX: bridgeheads keep bridge_walkable=true → A*
    //         still routes Ground→Bridge→Ground.
    //
    // Implementation note: see Task 7 implementation note. The destruction
    // path here is the standard "set health to 0 + advance_tick" flow that
    // existing world_tests.rs:* tests already exercise — pick the closest
    // pattern.
}
```

**Verification:**

1. `cargo test -p ra2 --lib world_tests::test_bridgehead_walkability_invariant_across_non_bridge_rebuild_triggers`
2. `cargo test -p ra2 --lib world_tests::test_layered_astar_can_traverse_bridge_after_unrelated_building_death`
3. Sanity: comment out Task 1 and confirm both tests fail with bridgehead
   walkability collapsing on the first rebuild. Uncomment and re-run.

---

## Final verification

Run end-to-end before declaring the PR done:

```
cargo fmt --all
cargo check -p ra2 --lib --tests
cargo test -p ra2 --lib bridge_state
cargo test -p ra2 --lib pathfinding::core_tests
cargo test -p ra2 --lib world_tests
cargo test -p ra2 --lib   # full suite — confirm no regressions outside the touched modules
```

Expected: all bridge / pathfinding / world tests pass. The `state_hash` bump
is silent — no test pins a hash value. PathCell.bridge_walkable for
bridgeheads is now stable across every rebuild trigger.

---

## Out-of-scope (do NOT bundle in this PR)

These are deferred follow-ups from the design doc §"Tech debt deferred". Both
require RE work that has not been done. Each gets its own PR after the RE
clarifies the gamemd baseline.

1. [src/sim/production_placement.rs:391](../../src/sim/production_placement.rs#L391)
   — still reads `ResolvedTerrainCell.bridge_walkable` directly. Migration
   pending RE on whether gamemd allows naval-yard placement under destroyed
   bridge gaps.

2. [src/sim/movement/movement_path.rs:23-24](../../src/sim/movement/movement_path.rs#L23)
   `is_under_bridge_blocked_cell` — still reads
   `ResolvedTerrainCell.is_elevated_bridge_cell()` directly. Migration
   pending `/re-investigate` on `TooBigToFitUnderBridge` naval-unit
   destroyed-bridge semantics in gamemd.

3. Re-wire Task 14's `bridgehead_advance_state` for pass-4 bridgeheads after
   deriving `axis` from an adjacent body cell. The dispatcher's bridgehead
   branch at
   [bridge_orchestrator.rs:646](../../src/sim/world/bridge_orchestrator.rs#L646)
   stays live for pass-3 bridgeheads after Task 3; only pass-4 cells are
   excluded (via the `axis.is_none()` clause). A future PR can:
   - Walk cardinal neighbors during pass 4 to find an adjacent
     `bridge_layer.direction` and derive `axis` via `bridge_direction_to_axis`.
   - Drop the `Bridgehead && axis.is_none()` exclusion from `path_matches_cell`
     once pass-4 cells reliably have a derived axis.
   - Add Task 14 integration tests against a real terrain that constructs
     pass-4 bridgeheads.

   Driver: Tiny-Detail Ledger items #2 + #5 (bridgeheads survive collapse;
   `BridgeHead` flag persists). Implementing the full bridgehead damage
   visuals while preserving walkability is its own design + plan cycle.
