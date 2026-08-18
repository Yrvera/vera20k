# BayoPig High Bridge Grizzly Bridge Crossing Path Oracle Trace

Date: 2026-05-26

## Verdict

PASS: 0 | FAIL: 0 | UNCHECKED: all required field groups

This is an oracle/evidence artifact only. No bridge movement, bridge
pathfinding, terrain classification, or runtime bridge behavior fix is included.

## Scenario State

Candidate fixture: `BayoPig.mmx`, `TEMPERATE`, `[MTNK]`, `Americans`.

Concrete start cell, target cell, route window, and bridge overlay anchors are
still UNCHECKED because the local retail fixture capture did not complete during
this pass. The manifest is saved at
`docs/research/traces/bridge_oracle/bayopig_high_bridge_grizzly_manifest.json`
and intentionally leaves missing values as `null` or empty arrays instead of
inferring them from Rust.

## Evidence Artifacts

- Schema: `tools/bridge_oracle/schema.md`
- Comparator: `src/bin/bridge-oracle-compare.rs`
- gamemd placeholder trace:
  `docs/research/traces/bridge_oracle/bayopig_high_bridge_grizzly_gamemd_trace.json`
- Rust placeholder trace:
  `docs/research/traces/bridge_oracle/bayopig_high_bridge_grizzly_rust_trace.json`

## Missing Required Fields

- gamemd activation proof: unit pointer/id, `[MTNK]` type identity, house,
  issued order id/tick, pathfinder/search id, call-site category.
- gamemd map-load bridge facts for the selected route/window.
- gamemd A* candidate rows, including rejected candidates.
- gamemd `Can_Enter_Cell` / `CheckBridgeTraversal` return rows.
- gamemd runtime movement tick rows.
- Rust trace rows for the same concrete route/window.

## Scoped Result

The comparator and Rust diagnostic extraction points exist, but this scenario
remains UNCHECKED until a concrete high-bridge route and live gamemd rows are
captured. No PASS or FAIL bridge parity claim is made from this artifact.
