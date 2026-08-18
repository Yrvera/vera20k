# Bridge Repair Walker Parity Implementation Plan

> **For Codex:** Execute this plan task-by-task. Each task is self-contained. Do not write Rust code until this plan is approved for implementation.

**Goal:** Replace Rust's current span-based bridge repair mutation with a `gamemd.exe`-shaped engineer repair dispatcher and high/low repair walkers.

**Architecture:** This is a deterministic `sim/bridge_state` and `sim/world` change. `world_orders.rs` remains orchestration for engineer eligibility, sound, and despawn; `bridge_state/walker.rs` owns the binary-shaped overlay repair walkers; `map/` and `render/` stay out of the repair mutation. The implementation must preserve sim boundaries and low-bridge tube identity.

**Design Doc:** `docs/plans/2026-05-17-bridge-repair-walker-parity-design.md`

---

## Grounding Summary

- `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` identifies `InfantryClass::PerCellProcess @ 0x00519630` as the engineer repair entry and `MapClass::RepairBridge_Low/High @ 0x0057F200/0x0057F440` as repair direction dispatchers.
- The same report records the four repair walkers, their overlay ranges, transition tables, radar dirty gating, zone update gating, and `FUN_00598030` random variant picker. The outer low/high decision is low when the engineer 5x5 scan finds either a low bridge tile/ramp index (`CellClass+0x38` in `[DAT_00abad1c, DAT_00abad1c + 0x10)`) or a low overlay byte (`0x4A..=0x65`).
- `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` later corrects stale `+0x6DF` claims: engineer bridge repair is synchronous and does not use the building latch.
- Live Ghidra re-check confirmed `InfantryClass__PerCellProcess` dispatches repair directly and `BuildingClass__Update` uses `+0x6DF` for delayed hut collapse/delay-kill behavior, not engineer repair.
- Live Ghidra re-check confirmed `MapClass__RepairBridge_High`, `MapClass__RepairBridge_Low`, and high/low NS/EW walkers still match the report shape.
- `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md` says low bridge movement identity is `tube_index + LandType==10`, and low repair updates overlay/state/zones without assuming tube records are deleted.
- `LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md` says repair walkers clear the separate damaged-variant bit.
- Live Ghidra and `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` confirm `FUN_00598030` uses the binary RNG with a retry loop. Current Rust `SimRng::next_range_u32(4)` is one modulo draw, so exact gamemd RNG draw-count parity is not available until the sim RNG path is replaced or adapted.
- Current Rust repair entry is `World::tick_bridge_repair_orders` in `src/sim/world/world_orders.rs`.
- Current Rust mutation is `BridgeRuntimeState::body_cell_repair_state` in `src/sim/bridge_state/mod.rs`; it repairs by anchor span collection and span slot order, which is not the binary walker order.
- Current `src/sim/bridge_state/walker.rs` already hosts binary-shaped high/low destruction walkers and overlay classifiers; repair walkers should mirror that local pattern.
- INI grounding: `rulesmd.ini` has `[AudioVisual] RepairBridgeSound=BridgeRepaired`, `[CombatDamage] DestroyableBridges=yes`, `[CombatDamage] BridgeStrength=1500`, and `[CABHUT] BridgeRepairHut=yes`. Repair walker overlay IDs come from bridge overlay type families, not from new INI keys.
- Unknown after grounding: exact listener/campaign callback mapping after repair and full ramp/no-overlay branch parity. These are isolated follow-ups, not reasons to keep the current span repair fallback.

## Key Technical Decisions

- Add repair walkers beside existing destruction walkers in `src/sim/bridge_state/walker.rs`. **Confidence:** high.
  - **Source:** repo pattern in `walker.rs`; Ghidra `MapClass__RepairBridgeWalker_*`; design doc.
- Replace production use of `body_cell_repair_state` with a binary-shaped scan/dispatch/walker path. **Confidence:** high.
  - **Source:** live Ghidra `InfantryClass__PerCellProcess`, `ProcessBridgeDestruction_Low/High`, current Rust `body_cell_repair_state`.
- Preserve `RepairOutcome` as the world-facing side-effect contract unless tests prove it cannot express the binary behavior. **Confidence:** high.
  - **Source:** current `RepairOutcome` fields map cleanly to zones, radar cells, and mutation count.
- Implement both high and low overlay repair walkers in the same feature. **Confidence:** high.
  - **Source:** current Rust has low bridge/tube plumbing; Ghidra confirms low repair walkers are live.
- Preserve the binary outer low/high decision as a two-predicate check: low tile/ramp terrain or low overlay. **Confidence:** high for the binary predicate, medium for the first Rust terrain predicate.
  - **Source:** `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`; current Rust fields `ResolvedTerrainCell::bridge_layer`, `BridgeDirection::Low`, and `BridgeRuntimeCell::overlay_byte`.
- Route random healthy-variant selection through a dedicated `repair_variant_offset` helper instead of calling `next_range_u32(4)` inline. **Confidence:** high.
  - **Source:** `FUN_00598030 @ 0x00598030` retry loop; current Rust `SimRng` is not binary RNG-compatible yet.
- For fixed side-state repairs, write `DamageState::Healthy { variant: 0 }` and let the repaired overlay byte carry side A/B visual identity. **Confidence:** high.
  - **Source:** current `DamageState` enum has no bridgehead-A/B state variants; binary transition table writes fixed overlay bytes.
- Do not silently fall back to span repair for no-overlay/ramp branches. **Confidence:** high.
  - **Source:** parity risk from hiding a known branch behind non-binary behavior.
- Keep low bridge `tube_index` intact during repair. **Confidence:** medium-high.
  - **Source:** `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md` primary function decompilation found no direct tube-index deletion in low repair; confidence there is medium for exhaustive writes.

Low-confidence decisions to verify during `/review-plan`:

- Whether first implementation should include the no-overlay ramp/pavement repair branch or return `NoChange` plus a diagnostic. The plan below chooses `NoChange` for no-overlay scans and adds a follow-up diagnostic task because the overlay walker parity gap is already concrete and separable.
- Whether `BridgeDirection::Low` over `ResolvedTerrainCell::bridge_layer` is sufficient for every Rust representation of the binary low tile/ramp predicate. The plan makes this a named helper and tests it separately so the predicate can be tightened without changing the repair walker API.

## Open Questions

### Resolved During Planning

- **Does engineer bridge repair use `BuildingClass+0x6DF`?** No. Live Ghidra and `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md:2289` confirm synchronous repair.
- **Is low bridge repair blocked by missing Rust tube support?** No. Current Rust has `tube_index`, explicit tube facts, low bridge records, and tube-aware pathfinding pieces. Repair still must preserve tube identity.
- **Should `world_orders.rs` own repair walker logic?** No. It should call into `BridgeRuntimeState`; the binary-shaped walker code belongs in `bridge_state/walker.rs`.

### Deferred to Implementation

- **Exact trigger/listener callback path after repair:** Current bridge repair world flow has no obvious listener/event channel matching the binary callback loop. Add a follow-up disparity scan or implementation plan once bridge triggers/listeners are represented.
- **Ramp/no-overlay branch parity:** If no bridge overlay byte is found in the selected 5x5 scan, return `RepairOutcome::default()` and log this as a targeted diagnostic gap. Do not use span repair as a fallback.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/sim/bridge_state/walker.rs` | Add high/low repair direction dispatchers and NS/EW repair walkers beside destruction walkers. |
| Modify | `src/sim/bridge_state/mod.rs` | Keep shared `RepairOutcome`, helper constants, and `cells_in_5x5_scan`; remove production reliance on `body_cell_repair_state`. |
| Modify | `src/sim/world/world_orders.rs` | Call new repair dispatcher from engineer repair order flow. |
| Modify | `src/sim/world/world_orders_bridge_repair_tests.rs` | Update integration tests for walker-shaped repair behavior. |
| Modify | `src/sim/bridge_state/mod.rs` test module or split test file | Add focused repair dispatcher/walker tests. |
| Read only | `src/sim/world/bridge_orchestrator.rs` | Confirm C4 hut collapse path remains separate. |
| Read only | `src/map/resolved_terrain.rs` | Confirm low repair tests can assert `tube_index` preservation. |
| Read only | `src/rules/ruleset.rs` and `src/rules/object_type.rs` | Confirm existing INI parsing for repair sound and bridge repair hut remains enough. |

## Interface Changes

- Add a production repair entry on `BridgeRuntimeState`:

```rust
pub fn repair_bridge_from_engineer_scan(
    &mut self,
    scan_cells: &[(u16, u16)],
    rng: &mut crate::sim::rng::SimRng,
    terrain: &ResolvedTerrainGrid,
) -> RepairOutcome
```

- Add private helpers in `walker.rs`:

```rust
fn repair_bridge_low_from_scan(...);
fn repair_bridge_high_from_scan(...);
fn repair_bridge_low(...);
fn repair_bridge_high(...);
fn repair_bridge_walker_ns_low(...);
fn repair_bridge_walker_ew_low(...);
fn repair_bridge_walker_ns_high(...);
fn repair_bridge_walker_ew_high(...);
fn repair_overlay_transition(...);
fn repair_variant_offset(...);
fn record_repair_mutation(...);
```

- Do not change `World::tick_bridge_repair_orders` visibility or `SimSoundEvent::BridgeRepaired`.
- Do not add render/UI/audio dependencies to `sim/`.
- Do not add new deterministic world state. Existing bridge runtime state mutations remain hash-covered through current bridge state hashing.

## Sim Checklist

- [ ] All math uses integer/fixed-point logic; no `f32` or `f64` in sim repair code.
- [ ] No new deterministic state outside existing bridge runtime state.
- [ ] No dependency from `sim/` to `render/`, `ui/`, `sidebar/`, `audio/`, or `net/`.
- [ ] Tick ordering remains: bridge repair orders run before capture orders in `Simulation::advance_tick`.
- [ ] `EntityStore` iteration order remains the existing deterministic `keys_sorted()` engineer snapshot.
- [ ] Random healthy-variant selection order is explicitly covered by tests; exact binary retry-loop draw count is documented as a current RNG parity boundary.

## Risk Areas

- Repair walker random variant order will intentionally differ from current span-slot order. Exact gamemd retry-loop draw count is still non-parity while Rust uses `SimRng` instead of the binary RNG.
- Low repair must not erase `tube_index` or make low bridge cells ordinary road.
- Overlay byte, runtime `DamageState`, `damaged_variant`, and `AnchorSpan.damage_state` mirrors can drift if only one representation is updated.
- Existing tests around `body_cell_repair_state` may encode old behavior. Update or retire those assertions only when a new walker test replaces the parity coverage.
- Returning `NoChange` for no-overlay/ramp scans is honest but still leaves a known branch unimplemented. Add an explicit diagnostic test so it is visible.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|---|---|---|---|
| 2 | Outer low/high scan decision | Repairing near low bridge cells must use low walker behavior instead of high/span behavior. | Unit tests with mixed high/low overlay scan and low-terrain/no-overlay scan. |
| 3 | Direction dispatcher overlay ranges | Wrong NS/EW choice repairs the wrong 3-wide strip and changes visible bridge art. | Overlay family tests for all high/low NS/EW bands. |
| 4 | Repair transition table | Damaged/destroyed bridge art must restore to the same visible healthy/damaged variants as gamemd. | Table-driven unit tests for high and low overlay transitions. |
| 5 | Random healthy-variant selection | Lockstep and visible random healthy variant selection depend on walker order. Exact gamemd retry-loop draw count is a known boundary until the sim RNG is binary-compatible. | Seeded tests assert one call to the repair-variant helper per random-healthy strip and selected family bases. |
| 6 | Radar and zone dirty gating | Destroyed spans should refresh minimap and pathing; side-only repairs should not over-refresh zones. | `RepairOutcome` assertions. |
| 7 | Low bridge tube preservation | Low bridge repair must not break tube-backed movement identity. | Test `tube_index` unchanged after low repair. |
| 9 | Engineer integration | Player hears repair sound, engineer is consumed, bridge reconnects this tick. | Existing integration tests updated to new dispatcher. |

---

## Tasks

### Task 1: Add Repair Overlay Constants And Classification Helpers

**Why:** Establish the binary overlay families before implementing mutation.

**Files:**
- Modify: `src/sim/bridge_state/walker.rs`

**Pattern:** Existing `is_ns_walker_overlay_high`, `is_ew_walker_overlay_high`, `is_ns_walker_overlay_low`, and `is_ew_walker_overlay_low`.

**Steps:**
1. Add private constants for inclusive overlay bands:
   - low body band: `0x4A..=0x65`;
   - high body band: `0xCD..=0xE8`;
   - low NS damaged/destroyed repair sources: `0x4E..=0x52`, `0x64`;
   - low EW damaged/destroyed repair sources: `0x57..=0x5B`, `0x65`;
   - high NS damaged/destroyed repair sources: `0xD1..=0xD5`, `0xE7`;
   - high EW damaged/destroyed repair sources: `0xDA..=0xDE`, `0xE8`.
2. Add helpers:

```rust
fn is_low_repair_overlay(overlay: u8) -> bool
fn is_high_repair_overlay(overlay: u8) -> bool
fn is_low_repair_outer_candidate(
    overlay: Option<u8>,
    terrain_cell: Option<&ResolvedTerrainCell>,
) -> bool
fn is_low_ns_repair_dispatch_overlay(overlay: u8) -> bool
fn is_low_ew_repair_dispatch_overlay(overlay: u8) -> bool
fn is_high_ns_repair_dispatch_overlay(overlay: u8) -> bool
fn is_high_ew_repair_dispatch_overlay(overlay: u8) -> bool
```

3. Implement `is_low_repair_overlay` and `is_high_repair_overlay` with the exact overlay ranges from the Tiny-Detail Ledger.
4. Implement `is_low_repair_outer_candidate` as the Rust representation of the binary outer low decision:
   - return true when `overlay.is_some_and(is_low_repair_overlay)`;
   - otherwise return true when `terrain_cell` has `bridge_layer.direction == BridgeDirection::Low`;
   - otherwise return false.
5. Do not use generic `bridge_facts.ramp_tile` alone as the low predicate. Current `BridgeRampTileTable` detection is bridge-set/ramp metadata and is not, by itself, the low-tile discriminator from `DAT_00abad1c`.
6. Implement NS/EW dispatch helpers with the exact ranges from the Tiny-Detail Ledger.
7. Add unit tests in the existing `walker.rs` test module or adjacent bridge-state tests:
   - each helper accepts the listed boundaries;
   - each helper rejects one byte below and one byte above the range;
   - low and high helpers do not overlap.
   - `is_low_repair_outer_candidate(None, low_bridge_terrain_cell)` returns true;
   - `is_low_repair_outer_candidate(None, high_bridge_or_plain_terrain_cell)` returns false.

**Verify:**
Run: `cargo test repair_overlay -- --nocapture`

Expected: helper tests pass.

### Task 2: Add The Engineer Scan Dispatcher Without Wiring It To World Yet

**Why:** Build the binary-shaped entry point behind tests before changing gameplay flow.

**Files:**
- Modify: `src/sim/bridge_state/walker.rs`
- Modify: `src/sim/bridge_state/mod.rs` only if `RepairOutcome` import/export placement needs adjustment.

**Pattern:** Existing `destroy_bridge_high` and `destroy_bridge_low` entries in `walker.rs`, plus `cells_in_5x5_scan`.

**Steps:**
1. Add `pub fn repair_bridge_from_engineer_scan(...) -> RepairOutcome` on `BridgeRuntimeState`.
2. The function must inspect `scan_cells` in their existing order and set `has_low_candidate = true` if any cell satisfies `is_low_repair_outer_candidate(runtime_overlay, terrain.cell(rx, ry))`.
3. If `has_low_candidate` is true, call `repair_bridge_low_from_scan`; otherwise call `repair_bridge_high_from_scan`.
4. Add private `repair_bridge_low_from_scan`:
   - iterate `scan_cells` in order;
   - first cell whose overlay byte is in `0x4A..=0x65` calls `repair_bridge_low(rx, ry, rng, terrain)`;
   - return default `RepairOutcome` if none found.
5. Add private `repair_bridge_high_from_scan`:
   - iterate `scan_cells` in order;
   - first cell whose overlay byte is in `0xCD..=0xE8` calls `repair_bridge_high(rx, ry, rng, terrain)`;
   - return default `RepairOutcome` if none found.
6. Do not call `body_cell_repair_state`.
7. Add tests:
   - scan with a low overlay and a high overlay dispatches low;
   - scan with no low overlay but with a `BridgeDirection::Low` terrain cell dispatches the low process branch and returns default outcome when no low overlay is found by the inner low overlay scan;
   - scan with only high overlay dispatches high;
   - scan with no selected-family overlay returns default outcome;
   - scan order chooses the first matching overlay in the selected family.

**Verify:**
Run: `cargo test repair_bridge_from_engineer_scan -- --nocapture`

Expected: dispatcher tests pass and no production world behavior changes yet.

### Task 3: Implement High And Low Direction Dispatchers

**Why:** Match `MapClass__RepairBridge_High/Low` before writing the strip walkers.

**Files:**
- Modify: `src/sim/bridge_state/walker.rs`

**Pattern:** Existing `destroy_bridge_high`, `destroy_bridge_low`, and `find_walker_start_*` helpers.

**Steps:**
1. Add `fn repair_bridge_high(&mut self, rx, ry, rng, terrain) -> RepairOutcome`.
2. Add `fn repair_bridge_low(&mut self, rx, ry, rng, terrain) -> RepairOutcome`.
3. For high:
   - if overlay is high NS dispatch family, use the same neighbor-start rule shape as `MapClass__RepairBridge_High` and call `repair_bridge_walker_ns_high`;
   - if overlay is high EW dispatch family, use the same x-axis neighbor-start rule shape and call `repair_bridge_walker_ew_high`;
   - otherwise return default outcome.
4. For low:
   - same as high but with low overlay band and low NS/EW dispatch families.
5. Reuse or split existing `find_walker_start_high_ns`, `find_walker_start_high_ew`, `find_walker_start_low_ns`, `find_walker_start_low_ew` only if their start-shift behavior matches repair dispatch. If destruction start shifting differs, add repair-specific helpers named `find_repair_walker_start_*`.
6. Add tests for start selection:
   - input at first cell in a 3-cell-wide band starts one cell forward when previous body-axis neighbor is off-band;
   - input in the middle of a band shifts to the stable walker start;
   - high and low use their own overlay ranges.

**Verify:**
Run: `cargo test repair_walker_start -- --nocapture`

Expected: high/low NS/EW dispatchers choose the expected walker start coordinates.

### Task 4: Add The Repair Overlay Transition Table

**Why:** Keep visible bridge art parity isolated from traversal and scan logic.

**Files:**
- Modify: `src/sim/bridge_state/walker.rs`

**Pattern:** Existing bridge destruction overlay tables in `src/sim/bridge_specs.rs`, but keep this repair table near repair walkers unless it grows large enough to justify moving to `bridge_specs.rs`.

**Steps:**
1. Add an internal enum for transition outcome:

```rust
enum RepairTransition {
    NoChange,
    Fixed(u8),
    RandomHealthy { base: u8 },
}
```

2. Add `fn repair_transition(overlay: u8, family: RepairFamily) -> RepairTransition`.
3. Add `RepairFamily` variants `LowNs`, `LowEw`, `HighNs`, `HighEw`.
4. Encode transitions:
   - `LowNs`: `0x4E..=0x52 | 0x64 => RandomHealthy { base: 0x4A }`; `0x5C | 0x5D => Fixed(0x5C)`; `0x5E | 0x5F => Fixed(0x5E)`.
   - `LowEw`: `0x57..=0x5B | 0x65 => RandomHealthy { base: 0x53 }`; `0x60 | 0x61 => Fixed(0x60)`; `0x62 | 0x63 => Fixed(0x62)`.
   - `HighNs`: `0xD1..=0xD5 | 0xE7 => RandomHealthy { base: 0xCD }`; `0xDF | 0xE0 => Fixed(0xDF)`; `0xE1 | 0xE2 => Fixed(0xE1)`.
   - `HighEw`: `0xDA..=0xDE | 0xE8 => RandomHealthy { base: 0xD6 }`; `0xE3 | 0xE4 => Fixed(0xE3)`; `0xE5 | 0xE6 => Fixed(0xE5)`.
5. Add table-driven unit tests for every listed source overlay.
6. Add tests that healthy overlays in the band return `NoChange`.

**Verify:**
Run: `cargo test repair_transition -- --nocapture`

Expected: every high/low transition maps to the verified output class.

### Task 5: Implement Center-Plus-Perpendicular Mutation Helper

**Why:** All four walkers need the same side-effect bookkeeping.

**Files:**
- Modify: `src/sim/bridge_state/walker.rs`

**Pattern:** Existing destruction walker mutation helpers and `body_cell_repair_state` damaged variant clearing.

**Steps:**
1. Add a helper:

```rust
fn apply_repair_to_strip_cell(
    &mut self,
    center: (u16, u16),
    perpendicular_a: (u16, u16),
    perpendicular_b: (u16, u16),
    family: RepairFamily,
    rng: &mut SimRng,
    terrain: &ResolvedTerrainGrid,
    outcome: &mut RepairOutcome,
)
```

2. Read the center cell's prior overlay byte.
3. Use `repair_transition` on the center overlay.
4. For `NoChange`, return without mutating center or perpendicular cells.
5. For `Fixed(new_overlay)`, first compare `new_overlay` to the center cell's prior overlay byte:
   - if `new_overlay == prior_center_overlay`, return without mutating center or perpendicular cells, without clearing damaged variant state, without adding repaired cells, and without touching RNG;
   - if `new_overlay != prior_center_overlay`, write `new_overlay` to center and both perpendicular cells.
6. Add a private helper for random healthy repair variants:

```rust
fn repair_variant_offset(rng: &mut SimRng) -> u8 {
    rng.next_range_u32(4) as u8
}
```

   This helper is intentionally isolated because binary `FUN_00598030` uses a retry loop around gamemd's RNG. Current Rust cannot claim exact gamemd draw-count parity while `SimRng` is xorshift plus modulo; tests should assert repair walker call order through this helper, not claim binary retry-loop equivalence.
7. For `RandomHealthy { base }`, call `repair_variant_offset(rng)` and write `base + draw` to center and both perpendicular cells.
8. Update the runtime `DamageState` for each mutated runtime bridge cell to the matching repaired state:
   - random healthy source becomes `DamageState::Healthy { variant: draw }`;
   - fixed repaired side-state becomes `DamageState::Healthy { variant: 0 }`; Rust has no bridgehead-A/B damage-state variants, so fixed side visual identity remains in the repaired overlay byte (`0x5C`, `0x5E`, `0x60`, `0x62`, `0xDF`, `0xE1`, `0xE3`, or `0xE5`).
9. Clear damaged-variant state for center and perpendicular cells by calling the existing damaged variant clearing helper or adding a private wrapper around `apply_damaged_variant_flood_fill(..., false, terrain)`.
10. Increment `outcome.repaired_cells` once per mutated runtime cell that actually changed state.
11. If the center prior overlay is a destroyed overlay (`0x64`, `0x65`, `0xE7`, `0xE8`), push center plus both perpendicular coordinates into `outcome.radar_cells`.
12. If the transition is `RandomHealthy`, set `outcome.zones_dirty = true`.
13. Re-sync any `AnchorSpan.damage_state` mirror for affected anchor spans after mutation.

**Verify:**
Run: `cargo test apply_repair_to_strip -- --nocapture`

Expected:
- one call to `repair_variant_offset` for each random-healthy strip repair under the current Rust RNG boundary;
- no `repair_variant_offset` call and no `SimRng` state change for fixed side-state repair;
- fixed side-state repair writes `DamageState::Healthy { variant: 0 }`;
- already-repaired fixed center overlays such as `0x5C`, `0x60`, `0xDF`, and `0xE3` skip the whole 3-cell strip rather than rewriting perpendicular cells;
- three coordinates mutate when all three exist;
- damaged variant state clears on all mutated cells;
- destroyed-source strip reports three radar dirty cells.

### Task 6: Implement The Four Repair Walkers

**Why:** This is the core parity replacement for span-slot repair.

**Files:**
- Modify: `src/sim/bridge_state/walker.rs`

**Pattern:** Existing `destroy_bridge_walker_ns_high`, `destroy_bridge_walker_ew_high`, `destroy_bridge_walker_ns_low`, and `destroy_bridge_walker_ew_low`.

**Steps:**
1. Add `repair_bridge_walker_ns_high`.
2. Add `repair_bridge_walker_ew_high`.
3. Add `repair_bridge_walker_ns_low`.
4. Add `repair_bridge_walker_ew_low`.
5. Each walker must rewind through its family overlay band before walking forward:
   - NS walkers decrement `x` until off-band, then increment to the first in-band `x`; each step walks `x += 1` at fixed `y`.
   - EW walkers decrement `y` until off-band, then increment to the first in-band `y`; each step walks `y += 1` at fixed `x`.
6. For NS walkers, the strip is `(x, y - 1)`, `(x, y)`, `(x, y + 1)`.
7. For EW walkers, the strip is `(x - 1, y)`, `(x, y)`, `(x + 1, y)`.
8. Use saturating/off-map guards equivalent to existing walker helper behavior; off-map perpendicular cells should be ignored rather than wrapping.
9. Stop when the next center overlay is outside that bridge family band.
10. Call `apply_repair_to_strip_cell` once per center cell.
11. Return the accumulated `RepairOutcome`.
12. Add tests:
   - high NS repairs a multi-cell strip and stops at first off-band center;
   - high EW repairs the transposed strip;
   - low NS and low EW do the same;
   - healthy center overlays in-band are skipped without RNG;
   - repaired strip order produces deterministic seeded repair-variant outputs.

**Verify:**
Run: `cargo test repair_bridge_walker -- --nocapture`

Expected: all four walkers mutate exactly the expected strip cells in walker order.

### Task 7: Add Low Bridge Tube Preservation Tests

**Why:** Low repair must not regress the tube-backed low bridge model.

**Files:**
- Modify: bridge-state repair tests in `src/sim/bridge_state/mod.rs` or the same test module used for Task 6.
- Read: `src/map/resolved_terrain.rs`

**Pattern:** Existing tests that set `ResolvedTerrainCell::tube_index = Some(TubeId(...))`.

**Steps:**
1. Build a low bridge repair fixture with a `ResolvedTerrainGrid` cell whose `tube_index` is `Some(TubeId(7))`.
2. Seed matching `BridgeRuntimeCell` overlay bytes in the low repair family.
3. Run `repair_bridge_from_engineer_scan`.
4. Assert the `ResolvedTerrainGrid` cell still reports `tube_index == Some(TubeId(7))`.
5. Assert `RepairOutcome.zones_dirty == true` only when a random-healthy main/destroyed low repair occurred.
6. Assert no code path writes a new tube id during repair.

**Verify:**
Run: `cargo test low_bridge_repair_preserves_tube -- --nocapture`

Expected: low repair mutates bridge state and preserves tube identity.

### Task 8: Wire World Engineer Repair To The New Dispatcher

**Why:** Replace the production disparity after the walker path is covered by unit tests.

**Files:**
- Modify: `src/sim/world/world_orders.rs`

**Pattern:** Existing `tick_bridge_repair_orders` flow.

**Steps:**
1. Keep candidate collection unchanged.
2. Keep `BridgeRepairHut=yes` target gating unchanged.
3. Keep Chebyshev adjacency unchanged.
4. Keep `SimSoundEvent::BridgeRepaired` emission at the building cell unchanged.
5. Replace:

```rust
bs.body_cell_repair_state(&scan, &mut self.rng, terrain)
```

with:

```rust
bs.repair_bridge_from_engineer_scan(&scan, &mut self.rng, terrain)
```

6. Keep `any_repair` calculation based on `outcome.zones_dirty || outcome.repaired_cells > 0`.
7. Keep radar cells as reserved outcome until a render/minimap dirty channel exists.
8. Keep engineer despawn unchanged.
9. Do not change capture order or C4 hut collapse flow.

**Verify:**
Run: `cargo test engineer_enters_cabhut_repairs_bridge -- --nocapture`

Expected: integration test still passes after expected assertion updates in Task 9.

### Task 9: Update Bridge Repair Integration Tests

**Why:** Existing integration tests should protect the player-visible world flow while no longer asserting span-repair details.

**Files:**
- Modify: `src/sim/world/world_orders_bridge_repair_tests.rs`

**Pattern:** Existing tests in that file.

**Steps:**
1. Update `engineer_enters_cabhut_repairs_bridge` to seed overlay bytes that the new repair dispatcher can find.
2. Keep assertions:
   - `TickResult.bridge_state_changed` true when bridge state mutates;
   - engineer despawned;
   - `BridgeRepaired` sound event emitted;
   - repaired bridge runtime cells have repaired state.
3. Update repaired state assertions to match the verified repair table and seeded RNG, not generic `Healthy`.
4. Keep `intact_bridge_emits_sound_no_mutation` behavior if no repairable overlay source exists.
5. Keep two-engineer same-tick behavior; if the first repair consumes all repairable overlays, the second engineer may emit sound and produce no mutation. Assert the exact current intended sound/mutation behavior explicitly.
6. Keep C4-on-CABHUT collapse tests unchanged except for imports or helper fixture updates.
7. Add a regression test that proves `tick_bridge_repair_orders` no longer calls `body_cell_repair_state` by constructing a scan where span repair would mutate a span but no overlay repair source exists; expected `bridge_state_changed == false`.

**Verify:**
Run: `cargo test world_orders_bridge_repair -- --nocapture`

Expected: repair integration tests pass with walker-shaped assertions; C4 hut collapse remains green.

### Task 10: Retire Or Re-scope Span Repair Tests

**Why:** Avoid keeping tests that encode non-binary repair behavior as if it were parity.

**Files:**
- Modify: `src/sim/bridge_state/mod.rs` test module.

**Pattern:** Existing tests near `body_cell_repair_state`.

**Steps:**
1. Review each `body_cell_repair_state` test.
2. For tests that only assert `cells_in_5x5_scan`, keep them unchanged.
3. For tests that assert span-slot repair order, old span-slot RNG assertions, or broad anchor-span repair, replace them with repair walker tests from Tasks 4-7.
4. If `body_cell_repair_state` remains as a private helper for tests or future tools, rename tests to make clear it is not the production engineer repair path.
5. If no production or test helper needs `body_cell_repair_state`, remove the method and its stale tests in the implementation pass.

**Verify:**
Run:
- `cargo test body_cell_repair -- --nocapture`
- `cargo test repair_bridge -- --nocapture`

Expected: no stale span-repair test remains as production parity coverage.

### Task 11: Add No-Overlay/Ramp Diagnostic Coverage

**Why:** Make the intentionally deferred ramp branch visible instead of hidden behind span repair.

**Files:**
- Modify: bridge-state repair tests in `src/sim/bridge_state/mod.rs` or `walker.rs`.
- Optionally add a short doc note under `docs/disparity-scans/` only if implementation reveals a new concrete output disparity.

**Pattern:** Existing tests for no-change outcomes.

**Steps:**
1. Build a repair scan with a bridge hut-adjacent engineer but no low/high repair overlay bytes in the 5x5 scan.
2. Include runtime bridge spans that the old span repair path would have repaired if called.
3. Assert `repair_bridge_from_engineer_scan` returns default `RepairOutcome`.
4. Assert no runtime bridge cell changed.
5. Add a test name that explicitly mentions the no-overlay ramp branch is deferred, for example `repair_scan_without_overlay_does_not_use_span_fallback`.
6. Do not emit a sound assertion here; this is a bridge-state test, not world order flow.

**Verify:**
Run: `cargo test span_fallback -- --nocapture`

Expected: no-overlay scans produce no mutation and do not hide ramp parity behind span repair.

### Task 12: Focused Verification

**Why:** This touches deterministic sim state and movement-relevant bridge zones.

**Files:** No edits.

**Steps:**
1. Run repair-focused tests:
   - `cargo test repair_bridge -- --nocapture`
   - `cargo test bridge_repair -- --nocapture`
   - `cargo test world_orders_bridge_repair -- --nocapture`
2. Run bridge state/path tests:
   - `cargo test bridge_state -- --nocapture`
   - `cargo test bridge_traversal -- --nocapture`
   - `cargo test tube -- --nocapture`
3. Run broader checks:
   - `cargo fmt --check`
   - `cargo check`
4. If unrelated dirty-tree changes break broad checks, record the exact command and first unrelated error. Do not fix unrelated files.

**Expected:** Focused repair and bridge tests pass; formatting passes for touched files; broad check either passes or has a clearly documented unrelated failure.

### Task 13: Implementation Handoff Notes

**Why:** Preserve the verified grounding and any remaining parity boundaries for follow-up work.

**Files:**
- Modify: implementation handoff or final response only.
- Modify docs only if implementation discovers a concrete new disparity.

**Steps:**
1. Summarize changed files and the new production repair path.
2. List tests run and results.
3. Call out deferred ramp/no-overlay branch parity.
4. Call out deferred listener/campaign callback parity.
5. Note that low bridge tube identity was tested and preserved.
6. Do not commit unless the user explicitly asks.

**Verify:** Final handoff includes all six points above.

## Sources & References

- **Design doc:** `docs/plans/2026-05-17-bridge-repair-walker-parity-design.md`
- **Disparity scan:** `docs/gap-scans/2026-05-17-disparity-scan-bridge-repair-rebuild.md`
- **Ghidra reports:**
  - `docs/research/BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`
  - `docs/research/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`
  - `docs/research/LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`
  - `docs/research/LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md`
  - `docs/research/ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md`
- **Live Ghidra functions checked during planning:**
  - `InfantryClass__PerCellProcess @ 0x00519630`
  - `BuildingClass__Update @ 0x0043FB20`
  - `ProcessBridgeDestruction_Low @ 0x00570050`
  - `ProcessBridgeDestruction_High @ 0x00573540`
  - `MapClass__RepairBridge_Low @ 0x0057F200`
  - `MapClass__RepairBridge_High @ 0x0057F440`
  - `MapClass__RepairBridgeWalker_NS_Low @ 0x0057F6A0`
  - `MapClass__RepairBridgeWalker_EW_Low`
  - `MapClass__RepairBridgeWalker_NS_High`
  - `MapClass__RepairBridgeWalker_EW_High`
  - `FUN_00598030 @ 0x00598030`
- **INI keys:**
  - `ini/rulesmd.ini` `[AudioVisual] RepairBridgeSound=BridgeRepaired`
  - `ini/rulesmd.ini` `[CombatDamage] DestroyableBridges=yes`
  - `ini/rulesmd.ini` `[CombatDamage] BridgeStrength=1500`
  - `ini/rulesmd.ini` `[CABHUT] BridgeRepairHut=yes`
  - `ini/rulesmd.ini` `[OverlayTypes]` bridge overlay families `BRIDGE1`, `BRIDGE2`, `LOBRDGxx`, `LOBRDGEx`, `LOBRDGBx`
- **Related code:**
  - `src/sim/world/world_orders.rs`
  - `src/sim/bridge_state/mod.rs`
  - `src/sim/bridge_state/walker.rs`
  - `src/sim/world/world_orders_bridge_repair_tests.rs`
  - `src/sim/world/bridge_orchestrator.rs`
  - `src/map/resolved_terrain.rs`
  - `src/rules/ruleset.rs`
  - `src/rules/object_type.rs`
- **Recent related commits touching this system:**
  - `927c3a7 Tighten high bridge traversal gates`
  - `055a0a9 Integrate explicit tubes into pathfinding`
  - `135ce8c Fix bridge occupancy object-list layers`
  - `92c0c10 Add low bridge TubeClass map facts`
  - `774ee7f Fix bridge pathfinding layer parity`
