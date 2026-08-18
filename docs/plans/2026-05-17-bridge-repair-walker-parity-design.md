# Bridge Repair Walker Parity Design

## Goal

Replace Rust's current span-based bridge repair mutation with a `gamemd.exe`-shaped engineer repair dispatcher and high/low repair walkers, so repairing a bridge through a `BridgeRepairHut=yes` building changes the same visible bridge cells, routes random healthy-variant selection through a dedicated repair picker, refreshes the same pathing/radar surfaces, and preserves low-bridge tube identity.

## Architecture Context

### Current Rust Shape

Engineer repair is currently orchestrated in `src/sim/world/world_orders.rs`. `tick_bridge_repair_orders` finds engineers targeting `BridgeRepairHut=yes`, checks Chebyshev adjacency, emits `SimSoundEvent::BridgeRepaired`, builds a 5x5 scan around the engineer, and calls `BridgeRuntimeState::body_cell_repair_state`.

`body_cell_repair_state` lives in `src/sim/bridge_state/mod.rs`. It collects unique `anchor_span_id`s from the scan, iterates spans through a `BTreeSet`, repairs span slots in fixed slot order, and draws `rng.next_range_u32(4)` once per repaired main-deck runtime cell. This is deterministic Rust behavior, but it is not the binary repair walker shape.

`src/sim/bridge_state/walker.rs` already owns overlay-direct bridge destruction walkers for high and low bridges. It contains the closest local pattern for binary-shaped bridge walker dispatch: classify overlay byte, shift to a stable walker start, walk a 3-wide strip, mutate bridge runtime cells, and return a state outcome. Repair walker parity belongs beside this code, not in `world_orders.rs`.

Low-bridge support has moved since older docs were written. Current Rust has `ResolvedTerrainCell::tube_index`, `BridgeRecordKind::Low`, tube facts, and tube-aware pathfinding pieces. The design must not treat low bridge repair as blocked by a missing tube model. It must preserve tube identity and avoid reducing low repair to ordinary road repair.

### Source-Of-Truth Research

Relevant research reports:

- `docs/research/BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`
- `docs/research/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`
- `docs/research/LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`
- `docs/research/LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md`
- `docs/research/ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md`

Live Ghidra re-checks during planning confirmed the core premise:

- `InfantryClass__PerCellProcess @ 0x00519630` runs engineer bridge repair synchronously when the target building type has `BridgeRepairHut=yes`.
- The synchronous engineer repair path calls `ProcessBridgeDestruction_High` or `ProcessBridgeDestruction_Low`, which scan for bridge overlay bytes and dispatch to `MapClass__RepairBridge_High` or `MapClass__RepairBridge_Low`.
- `BuildingClass__Update @ 0x0043FB20` uses `+0x6DF` for delayed hut collapse / delay-kill paths, not engineer bridge repair.
- `MapClass__RepairBridge_High @ 0x0057F440` and `MapClass__RepairBridge_Low @ 0x0057F200` choose NS/EW repair walkers from overlay sub-ranges and neighbor checks.
- `MapClass__RepairBridgeWalker_NS_High`, `MapClass__RepairBridgeWalker_EW_High`, `MapClass__RepairBridgeWalker_NS_Low`, and `MapClass__RepairBridgeWalker_EW_Low` walk 3-cell-wide strips and mutate center plus two perpendicular cells.
- `FUN_00598030 @ 0x00598030` is the bounded random picker used by repair walkers for healthy bridge variant selection. It uses a retry loop around the binary RNG and must not be described as Rust's current one-modulo `next_range_u32(4)` behavior.

### Observable Surface

The player-visible surfaces are:

- repaired bridge body art and rail/deck footprint;
- whether damaged bridge pavement/variant visuals clear;
- whether destroyed spans become passable again;
- whether low-bridge tube-backed movement remains consistent;
- minimap/radar refresh for restored destroyed cells;
- route recalculation after a repaired bridge reconnects zones;
- repair sound and engineer consumption timing.

The sound and engineer-consumption path is already close enough to keep in `world_orders.rs`. The mutation of bridge cells is the disparity.

## Impact Analysis

| File | Change |
|---|---|
| `src/sim/bridge_state/walker.rs` | Add high/low repair dispatcher helpers and NS/EW repair walkers beside the existing destruction walkers. |
| `src/sim/bridge_state/mod.rs` | Add or adapt repair outcome/state helper types; keep `cells_in_5x5_scan`; retire `body_cell_repair_state` from production use after tests move. |
| `src/sim/world/world_orders.rs` | Replace the call to `body_cell_repair_state` with the new binary-shaped repair dispatcher; keep sound, adjacency, and engineer despawn orchestration here. |
| `src/sim/world/world_orders_bridge_repair_tests.rs` | Update integration tests to assert repair dispatcher output rather than span-slot repair behavior. |
| `src/sim/bridge_state/*tests*` | Add focused overlay transition, repair-variant helper order, 3-wide strip, and radar/zone outcome tests. Existing body-cell repair tests become stale or are renamed to test only any retained compatibility helper. |
| `src/sim/world/bridge_orchestrator.rs` | Read-only unless shared outcome plumbing needs alignment; hut collapse remains destruction/collapse, not repair. |
| `src/map/resolved_terrain.rs` and tube modules | Read-only for this feature; tests should prove repair does not clear `tube_index`. |

### Risk Areas

- **Random variant determinism:** Current tests lock a span-slot random-variant order. The binary-shaped order walks overlay strips, so the lockstep contract must be replaced deliberately, not patched accidentally. Exact `FUN_00598030` retry-loop draw count is a separate RNG parity boundary.
- **Low bridge/tube coupling:** Low repair should update overlay/state/zones without deleting or fabricating tube records.
- **Damage-state mirror consistency:** `AnchorSpan.damage_state`, per-cell runtime state, overlay byte, damaged variant bit, and pathgrid rebuild signals must stay coherent after repair.
- **Old tests encoding stale behavior:** Tests for `body_cell_repair_state` span collection may assert behavior that `gamemd.exe` does not have.
- **Ramp branch complexity:** `ProcessBridgeDestruction_*` has overlay-found and no-overlay ramp/pavement branches. Overlay walker repair is the first parity target; ramp branches must be scoped explicitly, not accidentally faked by the span repair fallback.

## Chosen Approach

Implement a unified high/low overlay repair dispatcher in `BridgeRuntimeState`, called from `tick_bridge_repair_orders`.

The production repair entry should mirror the binary shape:

1. `world_orders.rs` keeps engineer eligibility, adjacency, sound event, and despawn.
2. `world_orders.rs` passes the engineer-centered 5x5 scan to a `BridgeRuntimeState` repair dispatcher.
3. The dispatcher scans the engineer-centered 5x5 for the binary low/high decision before choosing a family. It should preserve the observed outer decision shape from `InfantryClass__PerCellProcess`: low is selected when any scanned cell matches either the low bridge tile/ramp predicate (`CellClass+0x38` in `[DAT_00abad1c, DAT_00abad1c + 0x10)`) or the low overlay range `[0x4A..=0x65]`; otherwise high is selected. In current Rust, the low tile/ramp predicate must be represented explicitly with terrain data, not approximated as overlay-only.
4. The low/high process function scans the same 5x5 neighborhood for the first overlay byte in that family and calls the corresponding `repair_bridge_low` or `repair_bridge_high` direction dispatcher.
5. The direction dispatcher chooses NS or EW repair walker from the binary overlay sub-ranges and neighbor-start checks.
6. The repair walker rewinds to the start of the overlay band, walks forward one center cell per step, mutates center plus two perpendicular cells, calls the repair transition table for the center overlay family, clears damaged variant state, marks radar dirty only for destroyed-source cells, and sets zone dirty only when a main/destroyed repair reconnects bridge zones.
7. The dispatcher returns a `RepairOutcome` compatible with existing `TickResult.bridge_state_changed` handling.

Do not implement a high-only repair fix. Current code now has enough low-bridge infrastructure that a high-only repair walker would knowingly leave a common visible bridge class on stale span behavior.

Do not keep `body_cell_repair_state` as a fallback for production overlay repair. If an overlay repair branch is not implemented yet, report `NoChange` or add a separate explicitly named ramp-branch task. A silent span fallback would hide a parity gap.

## Tiny-Detail Ledger

The implementation plan must carry these details through unchanged:

- Engineer repair is synchronous in `InfantryClass__PerCellProcess`; it does not use `BuildingClass+0x6DF`.
- `+0x6DF` remains relevant to C4/Ivan/delay-kill hut collapse, handled separately by `BuildingClass__Update` equivalents.
- The engineer-centered scan is 5x5, offsets `-2..=+2`.
- Low-vs-high outer dispatch checks two predicates in that 5x5 scan: binary low tile/ramp index `[DAT_00abad1c, DAT_00abad1c + 0x10)` and low overlay bytes `0x4A..=0x65`. Current Rust should model the tile/ramp side with a named terrain predicate, for example a helper over `ResolvedTerrainCell` that treats explicit `BridgeDirection::Low` terrain as low and keeps no-overlay ramp handling visible rather than silently falling into high repair.
- `ProcessBridgeDestruction_Low/High` perform another 5x5 scan for an overlay byte in the selected family before entering `RepairBridge_Low/High`.
- Low NS dispatcher overlays: `[0x4A..=0x52]`, `[0x5C..=0x5F]`, `0x64`.
- Low EW dispatcher overlays: `[0x53..=0x5B]`, `[0x60..=0x63]`, `0x65`.
- High NS dispatcher overlays: `[0xCD..=0xD5]`, `[0xDF..=0xE2]`, `0xE7`.
- High EW dispatcher overlays: `[0xD6..=0xDE]`, `[0xE3..=0xE6]`, `0xE8`.
- NS/EW names describe the 3-cell perpendicular strip orientation in the decompiled reports; avoid assuming they name the visible bridge axis without checking the local walker functions.
- Repair walkers rewind backward through the bridge overlay band, step forward, and mutate the center cell plus two perpendicular neighbor cells.
- Healthy-variant repair uses `FUN_00598030()` then adds the family base. Rust must call a dedicated repair-variant helper, not `SimRng::next_range_u32(4)` directly; until `SimRng` matches the binary RNG and retry loop, this helper is a documented RNG parity boundary rather than a claim of exact gamemd draw count.
  - low NS damaged/destroyed main: `0x4A..=0x4D`;
  - low EW damaged/destroyed main: `0x53..=0x56`;
  - high NS damaged/destroyed main: `0xCD..=0xD0`;
  - high EW damaged/destroyed main: `0xD6..=0xD9`.
- Low NS damaged-side pairs repair to `0x5C` or `0x5E`; low EW pairs repair to `0x60` or `0x62`.
- High NS damaged-side pairs repair to `0xDF` or `0xE1`; high EW pairs repair to `0xE3` or `0xE5`.
- Fixed side-state repairs write `DamageState::Healthy { variant: 0 }` in Rust for the mutated runtime cells. Rust has no bridgehead-A/B `DamageState` variant; the repaired overlay byte (`0x5C`, `0x5E`, `0x60`, `0x62`, `0xDF`, `0xE1`, `0xE3`, or `0xE5`) carries the fixed side visual identity.
- Destroyed-source overlays mark radar terrain dirty for the three repaired perpendicular cells.
- `UpdateBridgeZonesHelper` is gated by a repaired main/destroyed-source path (`bVar1` in the decompilation), not by every overlay mutation.
- Repair clears the separate damaged-variant bit through the repair walker family; Rust should clear the equivalent damaged variant state for all repaired center/perpendicular cells.
- Low repair must not clear `ResolvedTerrainCell::tube_index`; low bridge tube identity is separate from overlay/state/zone activation.
- Listener/campaign callbacks after repair exist in the binary. They are not required for the first walker parity implementation unless Rust already has the corresponding trigger/listener channel; they should be listed as a follow-up rather than folded into bridge-state mutation.

## Interface Sketch

The names are provisional but the shape is fixed:

```rust
impl BridgeRuntimeState {
    pub fn repair_bridge_from_engineer_scan(
        &mut self,
        scan_cells: &[(u16, u16)],
        rng: &mut SimRng,
        terrain: &ResolvedTerrainGrid,
    ) -> RepairOutcome;

    fn repair_bridge_low_from_scan(...);
    fn repair_bridge_high_from_scan(...);
    fn repair_bridge_low(...);
    fn repair_bridge_high(...);
    fn repair_bridge_walker_ns_low(...);
    fn repair_bridge_walker_ew_low(...);
    fn repair_bridge_walker_ns_high(...);
    fn repair_bridge_walker_ew_high(...);
}
```

`RepairOutcome` should remain the world-facing contract if it can express the binary-shaped side effects:

- `zones_dirty`;
- `radar_cells`;
- `repaired_cells`.

If tests need to prove repair-variant helper calls or repaired overlay coordinates, expose that only through `#[cfg(test)]` helpers or by inspecting state after the call. Do not add debug-only fields to deterministic production state.

## Testing Strategy

Add focused unit tests before replacing the production call:

- direction dispatch chooses NS/EW for each high and low overlay family;
- repair walkers mutate exactly center plus two perpendicular cells per step;
- high and low transition tables map each damaged/destroyed family to the expected healthy or less-damaged overlay;
- random healthy-variant selection changes from span-slot order to walker order, while exact binary retry-loop draw count remains a named parity boundary until the sim RNG matches `FUN_00598030`;
- destroyed-source cells produce radar dirty entries for the three perpendicular cells;
- low repair preserves `tube_index`;
- engineer integration still emits `BridgeRepaired`, consumes the engineer, and sets `TickResult.bridge_state_changed` only when a repair mutation reconnects or changes bridge state.

Add one compatibility test that proves `body_cell_repair_state` is no longer used by `tick_bridge_repair_orders`. That protects against reintroducing the span fallback later.

## Deferred Open Questions

- Exact listener/campaign callback mapping after repair is verified in the binary but not obviously represented in current Rust. Keep it as a follow-up unless the implementation plan finds an existing trigger event target.
- Ramp/no-overlay branches in `ProcessBridgeDestruction_Low/High` are real. The first implementation plan should include a bounded task to return `NoChange` for no-overlay repair scans and a separate diagnostic/follow-up for ramp branch parity, unless the implementer chooses to add the ramp repair branch in the same feature with additional tests.
- Dirty-screen rectangle and tactical invalidation are render-side concerns in `gamemd.exe`. Rust should keep sim-side outcome data only; any renderer invalidation belongs in a later render bridge-refresh task.
