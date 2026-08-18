# BayOPigs High Bridge Grizzly Bridge Crossing Path Oracle Trace

Date: 2026-05-26

## Verdict

PASS: 0 | FAIL: 0 | UNCHECKED: 5

No bridge behavior fix is included. This pass created the oracle/comparator
surfaces and Rust-side opt-in diagnostics, but the selected scenario remains
UNCHECKED because no concrete gamemd capture exists yet and the BayOPigs route
coordinates/bridge anchors still need a map overlay probe.

## Scenario Status

- Candidate map: `BayOPigs.mmx` from the local retail directory.
- Candidate unit: `[MTNK]`.
- Candidate theater: `TEMPERATE` until map parse confirms otherwise.
- Manifest: `docs/research/traces/bridge_oracle/BAYOPIGS_HIGH_BRIDGE_GRIZZLY_MANIFEST.json`.
- Route status: `UNCHECKED`; start cell, target cell, route window, and bridge
  overlay anchors are intentionally `null`/empty until a real probe selects them.

## Oracle Surfaces Added

- Versioned JSON schema: `tools/bridge_oracle/schema.md`.
- Comparator binary: `bridge-oracle-compare`.
- Comparator sample traces: `tools/bridge_oracle/samples/`.
- Rust map-load cell facts dump:
  `ResolvedTerrainGrid::bridge_oracle_cell_facts`.
- Rust A* candidate-step tracing:
  `AStarOptions::trace_sink`, `trace_search_id`, and `trace_window`.
- Rust cell-entry split-layer diagnostic:
  `check_terrain_with_layers_oracle`.
- Rust runtime bridge transition diagnostic:
  `resolve_cell_transition_bridge_state_oracle`.

## Missing Required Evidence

- Concrete route start/target/window and high-bridge overlay anchors.
- gamemd map-load bridge facts for the selected cells.
- gamemd activation proof for the selected live `[MTNK]` move order.
- gamemd A* / `UnitClass::Can_Enter_Cell` candidate rows.
- gamemd runtime movement tick rows.

Because those fields are missing, the final oracle verdict for this scenario is
UNCHECKED, not PASS.

## Verification

- `cargo test -q --bin bridge-oracle-compare`: PASS.
- `cargo test -q resolved_terrain`: PASS.
- `cargo test -q cell_entry`: PASS.
- `cargo test -q pathfinding::core`: PASS.
- `cargo test -q movement_bridge`: PASS.
- `cargo test -q terrain_cost`: BLOCKED by unrelated test-build error in
  `src/sim/miner/miner_dock_sequence.rs` (`SimSoundEvent` import missing under
  test build).
- `cargo check -q`: PASS.
