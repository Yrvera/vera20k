# Bridge Zone Record Kind Plan

## Goal

Fix the bridge-zone record model so Rust can represent the `gamemd.exe`
`BridgeRecord.bridge_kind` field and prevent low-bridge records from being used
by high-bridge-only lookup paths.

This is a simulation/pathfinding parity fix. It is not a renderer fix, not an
INI parsing change, and not a low-bridge TubeClass implementation.

## Grounding

Design source: no dedicated design doc exists for this exact gap. This plan is
grounded in the end-to-end bridge disparity scan plus verified Ghidra reports:

- `docs/gap-scans/2026-05-15-disparity-scan-bridges-end-to-end.md`
- `docs/plans/2026-05-15-bridge-parity-fix-priority-list.md`
- `docs/research/BRIDGE_LOW_AND_ZONE_RECORDS_GHIDRA_SUPPLEMENT.md`
- `docs/research/BRIDGE_SYSTEM.md`
- `docs/research/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`

Verified binary behavior to preserve:

- `MapClass__ComputeBridgeZones @ 0x0056D6E0` builds 16-byte bridge records:
  endpoint A, endpoint B, intact byte, padding, and `bridge_kind`.
- High bridge records write `bridge_kind = 0`.
- Low bridge records write `bridge_kind = 1`.
- Records are retained; destruction toggles intact/active state rather than
  removing the record.
- `MapClass__FindBridgeRecord @ 0x0056DA10` linearly scans records but skips
  any record where `bridge_kind != 0`, so it is high-only.
- `UpdateBridgeZonesHelper` / `AddBridgeZoneEdges` must be rechecked before
  changing adjacency behavior, because it may consume all intact records while
  `FindBridgeRecord` consumes high records only.

## Current Rust Gap

Current `src/sim/bridge_state/mod.rs` has:

```rust
pub struct BridgeEndpointRecord {
    pub endpoint_a: (u16, u16),
    pub endpoint_b: (u16, u16),
    pub group_id: u16,
    pub active: bool,
}
```

There is no field equivalent to `bridge_kind`. As a result,
`src/sim/pathfinding/zone_build.rs` currently feeds every active record into:

- `inject_bridge_adjacency`
- `build_bridge_redirect`

That means Rust cannot express "low bridge records exist, but high-bridge
lookup skips them."

## Non-Goals

- Do not implement true low-bridge `TubeClass` pathing in this patch.
- Do not change bridge rendering.
- Do not change bridge damage state machine semantics except where record
  active flags already mirror intact/destroyed state.
- Do not introduce floating point into simulation logic.
- Do not create broad pathfinding rewrites or refactors.

## Implementation Tasks

### 1. Reconfirm The Consumer Split

Before editing Rust, re-open or verify the relevant Ghidra/report evidence:

- `MapClass__ComputeBridgeZones @ 0x0056D6E0`
- `MapClass__FindBridgeRecord @ 0x0056DA10`
- `UpdateBridgeZonesHelper` and/or `AddBridgeZoneEdges`

Record the implementation decision in code comments:

- high-only consumers: any path mirroring `FindBridgeRecord` / high bridge deck
  lookup.
- all-intact consumers: only if verified for `UpdateBridgeZonesHelper` /
  `AddBridgeZoneEdges`.

Expected starting assumption:

- `build_bridge_redirect` should use high-only records.
- `inject_bridge_adjacency` should keep all active records only if the binary
  adjacency helper does not filter `bridge_kind`.

### 2. Add Bridge Record Kind To Runtime Records

In `src/sim/bridge_state/mod.rs`:

- Add a small enum, preserving deterministic derives:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BridgeRecordKind {
    High,
    Low,
}
```

- Add `pub bridge_kind: BridgeRecordKind` to `BridgeEndpointRecord`.
- Keep `endpoint_records()` returning all records for compatibility.
- Add a convenience method:

```rust
impl BridgeEndpointRecord {
    pub fn is_high(&self) -> bool {
        self.bridge_kind == BridgeRecordKind::High
    }
}
```

Use the name `bridge_kind` rather than a generic `kind` so it maps cleanly to
the Ghidra record field.

### 3. Classify Records During Endpoint Construction

Update `compute_bridge_endpoints` in `src/sim/bridge_state/mod.rs`.

For each bridge group:

- classify the group as low if its resolved bridge-layer facts indicate low
  bridge cells.
- otherwise classify it as high.

Use existing resolved terrain facts rather than hardcoding tile names. The
likely source is `ResolvedTerrainCell.bridge_layer.direction` / existing bridge
direction data already used by bridge runtime construction.

Important guardrail:

- If a fixture does not carry bridge-layer direction, preserve the existing
  high-bridge default unless the resolved terrain explicitly identifies a low
  bridge. Tests that use minimal synthetic bridge cells should not become low
  accidentally.

### 4. Make Zone Consumers Explicit About Record Use

In `src/sim/pathfinding/zone_build.rs`, add an explicit filter mechanism so a
reader can see which binary behavior each helper mirrors. A simple local enum is
enough:

```rust
pub(crate) enum BridgeRecordFilter {
    AllActive,
    HighActiveOnly,
}
```

Then apply it inside record loops:

- inactive records are always skipped.
- low records are skipped only for `HighActiveOnly`.

Update call sites in:

- `src/sim/pathfinding/zone_incremental.rs`
- `src/sim/pathfinding/zone_map.rs`, if needed by signature changes

Expected use:

- `build_bridge_redirect(..., HighActiveOnly)`
- `inject_bridge_adjacency(..., AllActive)` unless Task 1 proves the binary
  adjacency helper is high-only too.

Do not silently filter all zone behavior to high-only; that risks deleting valid
low-record zone edges if `UpdateBridgeZonesHelper` really consumes all intact
records.

### 5. Preserve Damage/Repair Active Flag Behavior

Update `refresh_endpoint_active_flags` in `src/sim/bridge_state/mod.rs` only as
needed for the new field.

Required behavior:

- active/intact still follows current group destroyed/repair state.
- `bridge_kind` never changes during damage or repair.
- records remain present after destruction.

### 6. Update Hashing And Serialization Expectations

`BridgeEndpointRecord` already derives `Hash`, but `WorldSim::hash_bridge_state`
currently hashes bridge cells and anchor spans, not endpoint records.

In `src/sim/world/world_hash.rs`:

- include endpoint records in deterministic order.
- hash endpoint A, endpoint B, group id, active flag, and `bridge_kind`.

This is a state-hash-impacting change, but only because the previous hash was
missing existing bridge record state. It should remain deterministic and ordered
by the existing endpoint record vector.

Update serialization round-trip tests so `bridge_kind` is compared/restored.

### 7. Add Focused Tests

Add tests in `src/sim/bridge_state/mod.rs`:

- `bridge_endpoint_records_mark_high_groups_high`
- `bridge_endpoint_records_mark_low_groups_low`
- `bridge_record_kind_survives_damage_active_refresh`
- extend the existing serialization round-trip coverage to assert
  `bridge_kind`.

Add tests in `src/sim/pathfinding/zone_build.rs` or the existing pathfinding
test module:

- `bridge_redirect_ignores_low_bridge_records`
- `bridge_adjacency_filter_all_active_includes_low_records`
- `bridge_adjacency_filter_high_active_only_skips_low_records`

If Task 1 proves adjacency must be high-only, rename the all-active test to
document the verified behavior instead of keeping a false expectation.

Add or update one hash test in `src/sim/world/world_hash.rs`:

- two otherwise identical bridge states with different `bridge_kind` hash
  differently.

### 8. Run Verification

Run the narrow tests first:

```powershell
cargo test bridge_state --lib -- --nocapture
cargo test zone_build --lib -- --nocapture
cargo test world_hash --lib -- --nocapture
```

Then run bridge/pathfinding broader coverage:

```powershell
cargo test bridge --lib -- --nocapture
cargo test zone --lib -- --nocapture
```

If unrelated dirty working-tree changes cause failures, do not revert them.
Report the unrelated failure and keep this patch scoped.

## Risk Notes

- The highest risk is filtering low records out of the wrong consumer. Keep
  `FindBridgeRecord`-style lookup high-only, but verify adjacency before
  changing it.
- Synthetic tests may not populate full resolved bridge facts. Default unknown
  fixtures to high unless the cell explicitly carries low-bridge data.
- This patch still will not make low bridges fully parity-complete. It only
  prevents the record model from erasing the high/low distinction needed by
  later low-bridge work.

## Done Criteria

- `BridgeEndpointRecord` carries `bridge_kind`.
- Low and high bridge records can coexist in runtime state.
- High-only lookup/redirect skips low records.
- Adjacency behavior is explicitly matched to the verified binary helper.
- Damage/repair flips active state without changing record kind.
- Serialization and deterministic world hash include the new record field.
- Targeted bridge and zone tests pass.

