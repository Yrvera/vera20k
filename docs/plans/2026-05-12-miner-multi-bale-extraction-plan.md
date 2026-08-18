# Miner Multi-Bale Extraction Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Drain an ore cell of all bales that fit in remaining cargo capacity in **one** harvest extraction call (matching gamemd's `Harvest_Ore_Tick` at 0x73D450), and delay the first refinery-dock bale by one `unload_tick_interval` to match gamemd's per-bale gate.

**Architecture:** Adds `extract_bales_max` as a sibling free function to `extract_bale` in [src/sim/miner/miner_system.rs](src/sim/miner/miner_system.rs); `handle_harvest` swaps to the new bulk helper while `extract_bale` stays unchanged for `slave_miner.rs:253`. The dock first-bale fix is a single-line change to `phase_linked`'s `unload_timer` init in [src/sim/miner/miner_dock_sequence.rs](src/sim/miner/miner_dock_sequence.rs). No INI changes, no data-structure changes, no cross-module surface change.

**Design Doc:** [docs/plans/2026-05-12-miner-multi-bale-extraction-design.md](docs/plans/2026-05-12-miner-multi-bale-extraction-design.md)

---

## Grounding Summary

**Docs (R1):** Verified by gap-scan + brainstorm:
- [MISSION_HARVEST_GHIDRA_REPORT.md](ra2-rust-game-docs/MISSION_HARVEST_GHIDRA_REPORT.md) — Mission_Harvest 5-state machine, case 1 step-counter wait
- [HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md](ra2-rust-game-docs/HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md) — Harvest_Ore_Tick logic
- [REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §9.1](ra2-rust-game-docs/REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md) — per-bale gate `HarvesterDumpRate × 900 ≤ field_0x3E`

**Ghidra (R2):** Re-decompiled live during gap-scan this session:
- `0x73D450 UnitClass::Harvest_Ore_Tick` — confirmed `Math__ftol(Storage - currentAmount) → CellClass::Reduce_Tiberium(amount)` extracts up to that many density levels in ONE call, followed by ONE `StorageClass::AddAmount(N, type)`. Step counter reset to 0; timer reloaded to HarvesterLoadRate.
- `0x73E5E0 UnitClass::Mission_Harvest` case 1 — confirmed 9-step wait gate, 18-frame cadence between calls.
- `0x480A80 CellClass::Reduce_Tiberium` — confirmed full-drain removes overlay and partial-drain decrements density.

**Repo pattern (R3):** Mirrors existing helper-function pattern in [miner_system.rs](src/sim/miner/miner_system.rs): `extract_bale` ([line 643](src/sim/miner/miner_system.rs#L643)), `search_local_ore` ([line 982](src/sim/miner/miner_system.rs#L982)), `player_has_purifier` ([line 1069](src/sim/miner/miner_system.rs#L1069)) — pub(crate) free functions exported via [mod.rs:19](src/sim/miner/mod.rs#L19). New `extract_bales_max` joins this pattern.

**INI (R4):** No INI changes. Drivers:
- `HarvesterLoadRate=2` (rules+0x1520) → `harvest_tick_interval = load_rate × 9 = 18` (already plumbed via [MinerConfig::from_general_rules](src/sim/miner/mod.rs#L188))
- `HarvesterDumpRate=0.016` × 900 = 14.4 → `unload_tick_interval = 144` tenths (already plumbed)
- `Storage=40` (HARV) / `Storage=20` (CMIN) → `capacity_bales` via per-unit `obj_storage` parameter

**Unknowns after grounding:** None. All constants and behavior verified.

**Precondition:** The uncommitted changes in working tree to `miner_dock_sequence.rs` and `miner_tests.rs` (spiral `refinery_exit_cell` rework) are the baseline this plan targets. Tasks here assume those edits remain in place. If the user reverts them, Task 4-5 line numbers need re-mapping.

## Key Technical Decisions

- **New helper `extract_bales_max` rather than modifying `extract_bale`** — keeps slave miner semantics untouched. **Confidence: high.** Source: brainstorm Approach B; grep confirmed second caller at `src/sim/slave_miner.rs:253`.
- **`harvest_tick_interval` stays at 18 (`load_rate × 9`)** — gamemd waits 9 step-counter increments × HarvesterLoadRate frames between successive Harvest_Ore_Tick calls. **Confidence: high.** Source: Ghidra 0x73E5E0 case 1, re-decompiled this session.
- **Bulk extraction is atomic: one `node.remaining` decrement, one overlay update, one `Vec::extend`** — mirrors gamemd's single `Reduce_Tiberium` + single `AddAmount` pattern. **Confidence: high.** Source: Ghidra 0x73D450.
- **`unload_timer` initialised to `unload_tick_interval` (not 0) on Linked→Unloading** — first dock bale waits 14.4 frames per gamemd's per-bale gate. **Confidence: high.** Source: REFINERY_DOCK_ANIM_SLOTS §9.1 (doc verified) + slot-7 init evidence.
- **`Vec<CargoBale>` cargo model retained** — internal-only divergence from gamemd's `StorageClass` float array; observable output (bale count, value, type) is preserved. **Confidence: high** (per CLAUDE.md "internals are not the spec — outputs are").
- **`phase_linked` signature gains `&MinerConfig` parameter** — minimal plumbing, matches the rest of the phase handlers' patterns (`phase_unloading` already takes `config`). **Confidence: high.** Source: existing [`miner_dock_sequence.rs:269-282`](src/sim/miner/miner_dock_sequence.rs#L269-L282) dispatch table.

## Open Questions

### Resolved During Planning

- **Does multi-bale extraction break slave miners?** No — slaves call `extract_bale` (single-bale), which is unchanged. Verified by grep.
- **Does the cargo `Vec` order matter for determinism?** No — bales pushed in one burst are homogeneous (same type, same value); BTreeMap iteration order unaffected.
- **Should the first extraction also pre-empt the 18-frame wait?** No — gamemd waits 18 frames before the first call (step counter starts at 0, must reach 9). Our `handle_harvest` already mirrors this via `harvest_timer = config.harvest_tick_interval` set on `MoveToOre → Harvest` transition.

### Deferred to Implementation

- **Exact test counts that need adjustment in `miner_tests.rs`.** Task 2 lists the known ones; running the test suite will surface any others that encode the buggy cadence.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/miner/miner_system.rs` | Add `extract_bales_max` helper; switch `handle_harvest` to use it |
| Modify | `src/sim/miner/miner_dock_sequence.rs` | Init `unload_timer` to `unload_tick_interval`; thread `&MinerConfig` to `phase_linked` |
| Modify | `src/sim/miner/miner_tests.rs` | Update tests that assert buggy per-bale cadence; add new tests |
| Modify | `src/sim/miner/mod.rs` (only if exports need update — likely none) | Re-export `extract_bales_max` if used externally; currently internal only |

No new files. No INI/asset changes. No cross-module API surface change.

## Interface Changes

- **New `pub(crate) fn extract_bales_max(...)` in `miner_system.rs`** — free function, exported via `pub(crate) use self::miner_system::extract_bales_max;` in `mod.rs` only if a future caller (e.g., slave miner if it ever needs multi-bale) might use it. **Initial scope: keep it as `pub(crate)` in the module file, no re-export from `mod.rs` until needed.**
- **`phase_linked` signature** — gains `config: &MinerConfig` parameter. Single caller: `handle_dock_sequence` dispatch in same file. No external surface impact.

## Sim Checklist

- [x] All math uses `fixed`-point — none required here (u16 bale counts, i16 unload timer; no fractional sim math)
- [x] New state included in deterministic state hash — `Miner.cargo` already part of entity serialization; behavior change only
- [x] No dependencies on render/ui/sidebar/audio/net — confirmed; new helper only touches `sim::production::resource_nodes` and `sim::overlay_grid`
- [x] Tick ordering impact noted — none; `tick_miners` position in `advance_tick` unchanged
- [x] BTreeMap iteration order considered — `extract_bales_max` does ONE `get_mut` on `resource_nodes`; no iteration

## Risk Areas

- **Tests encoding the buggy per-bale cadence:** several integration tests in `miner_tests.rs` assert "after N ticks, 1 bale extracted." These will fail after Task 2. Task 2 specifically updates them before Task 3 adds new ones.
- **Visual feedback duration:** `harvest_overlay` (oregath.shp) and `voxel_animation` play during `state == Harvest`. After fix, Harvest lasts ~18-36 ticks per cell (was ~198). This is the parity fix, not a regression — but it changes observable cadence for players testing builds against pre-fix screenshots/videos.
- **Replay determinism:** existing recorded replays' cargo trajectories will diverge. Any replay-determinism golden tests need regenerating (expected for parity fixes).
- **`phase_linked` signature change:** rippled through one call site (`handle_dock_sequence` dispatch). Caught at compile time; no silent risk.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|------|------|----------------|--------------|
| Task 1 | Per-call extraction = `min(empty_capacity, cell_density_levels)` bales | Wrong count = wrong cell-drain rate = wrong economy tempo (visible every cell visit) | Unit tests assert the count math against gamemd Ghidra 0x73D450 |
| Task 1 | Bales pushed in one burst, same type | Mid-call type mixing would corrupt later refinery-dump per-type credit calc | Unit test `extract_max_gem_cell` confirms type homogeneity |
| Task 1 | Full-drain clears overlay; partial-drain updates frame | Overlay frame is the player-visible ore density on the map | Unit tests `extract_max_full_drain_ore` + `extract_max_partial_capacity` |
| Task 2 | Cadence between calls stays at 18 ticks | gamemd timer reload at HarvesterLoadRate; faster cadence = faster-than-original harvest | Integration test `harvester_drains_full_cell_in_one_extraction_tick` asserts 18-tick wait holds |
| Task 5 | First dock bale waits `unload_tick_interval` (~14.4 ticks), not 0 | gamemd's per-bale gate starts at 0 and gates on `14.4 ≤ counter` — first bale delayed | New test `dock_first_bale_waits_one_unload_interval` |
| Task 5 | Subsequent bales cadence unchanged at 14.4 ticks (144 tenths) | Whole-dock-cycle duration must match gamemd | Existing dock tests should continue to pass without modification |
| Task 6 | End-to-end harvest→dock→deposit cycle timing | The compound effect — players notice "harvester feels right" or "drags" | Manual side-by-side comparison vs gamemd on a known map |

---

## Tasks

### Task 1: Add `extract_bales_max` helper with unit tests

**Why:** Foundation — the new bulk extraction primitive. Independent of all other tasks (`extract_bale` stays unchanged, no caller switching yet). Implementing + testing in isolation lets us verify the math in a vacuum.

**Files:**
- Modify: [src/sim/miner/miner_system.rs](src/sim/miner/miner_system.rs) — add new free function after the existing `extract_bale` (around line 678)
- Modify: [src/sim/miner/miner_tests.rs](src/sim/miner/miner_tests.rs) — add unit tests

**Pattern:** Free pub(crate) function alongside `extract_bale` ([miner_system.rs:643](src/sim/miner/miner_system.rs#L643)) — same module, same call style, same mutation pattern via `&mut Simulation`.

**Step 1: Add the new function**

Append after the existing `extract_bale` function (the one ending around line 678):

```rust
/// Drain as many bales from `cell` as fit within `empty_capacity_bales`.
///
/// Mirrors gamemd's `UnitClass::Harvest_Ore_Tick`:
///   amount    = ftol(Storage - current_load)        // bales requested
///   extracted = CellClass::Reduce_Tiberium(amount)  // clamped to density
///   StorageClass::AddAmount(extracted, type)        // one storage update
///
/// Updates `resource_nodes[cell].remaining` and the overlay grid in one
/// atomic mutation pass. Returns an empty Vec when the cell is missing,
/// has `remaining == 0`, or `empty_capacity_bales == 0`.
pub(crate) fn extract_bales_max(
    sim: &mut Simulation,
    cell: (u16, u16),
    config: &MinerConfig,
    empty_capacity_bales: u16,
) -> Vec<CargoBale> {
    if empty_capacity_bales == 0 {
        return Vec::new();
    }
    let Some(node) = sim.production.resource_nodes.get(&cell) else {
        return Vec::new();
    };
    if node.remaining == 0 {
        return Vec::new();
    }
    let (value, base): (u16, u16) = match node.resource_type {
        ResourceType::Ore => (config.ore_bale_value, 120),
        ResourceType::Gem => (config.gem_bale_value, 180),
    };
    let resource_type = node.resource_type;
    let density_levels = node.remaining / base;
    if density_levels == 0 {
        return Vec::new();
    }
    let n: u16 = empty_capacity_bales.min(density_levels);
    if n == 0 {
        return Vec::new();
    }

    let bales: Vec<CargoBale> = (0..n)
        .map(|_| CargoBale {
            resource_type,
            value,
        })
        .collect();

    let remaining_after: u16 = node.remaining - n * base;
    if remaining_after == 0 {
        sim.production.resource_nodes.remove(&cell);
        if let Some(grid) = &mut sim.overlay_grid {
            grid.clear_overlay(cell.0, cell.1);
        }
    } else {
        sim.production.resource_nodes.get_mut(&cell).unwrap().remaining = remaining_after;
        if let Some(grid) = &mut sim.overlay_grid {
            let frame = (remaining_after / base).saturating_sub(1).min(11) as u8;
            grid.set_overlay_data(cell.0, cell.1, frame);
        }
    }

    bales
}
```

**Step 2: Add unit tests**

Append to `miner_tests.rs` (before the closing brace of the test module if there is one, otherwise at end of file). All seven test cases must be written:

```rust
#[test]
fn extract_max_empty_cell() {
    let mut sim = Simulation::new_for_test();
    let config = MinerConfig::default();
    let bales = crate::sim::miner::miner_system::extract_bales_max(
        &mut sim, (5, 5), &config, 40,
    );
    assert!(bales.is_empty(), "no node → no bales");
}

#[test]
fn extract_max_full_drain_ore() {
    let mut sim = Simulation::new_for_test();
    let config = MinerConfig::default();
    // 11 density levels of ore at base 120: remaining = 11 * 120 = 1320
    sim.production.resource_nodes.insert(
        (5, 5),
        crate::sim::miner::ResourceNode {
            resource_type: crate::sim::miner::ResourceType::Ore,
            remaining: 1320,
        },
    );
    let bales = crate::sim::miner::miner_system::extract_bales_max(
        &mut sim, (5, 5), &config, 40,
    );
    assert_eq!(bales.len(), 11, "full drain extracts 11 bales");
    assert!(
        bales.iter().all(|b| b.resource_type == crate::sim::miner::ResourceType::Ore
                          && b.value == config.ore_bale_value),
        "all bales are ore-type with configured value"
    );
    assert!(
        sim.production.resource_nodes.get(&(5, 5)).is_none(),
        "node removed after full drain"
    );
}

#[test]
fn extract_max_partial_capacity() {
    let mut sim = Simulation::new_for_test();
    let config = MinerConfig::default();
    // 11 density levels of ore: remaining = 1320
    sim.production.resource_nodes.insert(
        (5, 5),
        crate::sim::miner::ResourceNode {
            resource_type: crate::sim::miner::ResourceType::Ore,
            remaining: 1320,
        },
    );
    let bales = crate::sim::miner::miner_system::extract_bales_max(
        &mut sim, (5, 5), &config, 3,
    );
    assert_eq!(bales.len(), 3, "capacity-limited to 3 bales");
    let after = sim.production.resource_nodes.get(&(5, 5)).expect("still present");
    assert_eq!(after.remaining, 1320 - 3 * 120, "remaining decremented by 3 levels");
}

#[test]
fn extract_max_partial_density_exact_match() {
    let mut sim = Simulation::new_for_test();
    let config = MinerConfig::default();
    // 5 density levels: remaining = 600
    sim.production.resource_nodes.insert(
        (5, 5),
        crate::sim::miner::ResourceNode {
            resource_type: crate::sim::miner::ResourceType::Ore,
            remaining: 600,
        },
    );
    let bales = crate::sim::miner::miner_system::extract_bales_max(
        &mut sim, (5, 5), &config, 40,
    );
    assert_eq!(bales.len(), 5, "extracts all 5 available density levels");
    assert!(
        sim.production.resource_nodes.get(&(5, 5)).is_none(),
        "exact match drains the cell"
    );
}

#[test]
fn extract_max_gem_cell() {
    let mut sim = Simulation::new_for_test();
    let config = MinerConfig::default();
    // 4 density levels of gems at base 180: remaining = 720
    sim.production.resource_nodes.insert(
        (5, 5),
        crate::sim::miner::ResourceNode {
            resource_type: crate::sim::miner::ResourceType::Gem,
            remaining: 720,
        },
    );
    let bales = crate::sim::miner::miner_system::extract_bales_max(
        &mut sim, (5, 5), &config, 40,
    );
    assert_eq!(bales.len(), 4, "gem cell yields 4 bales");
    assert!(
        bales.iter().all(|b| b.resource_type == crate::sim::miner::ResourceType::Gem
                          && b.value == config.gem_bale_value),
        "all bales are gem-type with configured value"
    );
}

#[test]
fn extract_max_zero_capacity() {
    let mut sim = Simulation::new_for_test();
    let config = MinerConfig::default();
    sim.production.resource_nodes.insert(
        (5, 5),
        crate::sim::miner::ResourceNode {
            resource_type: crate::sim::miner::ResourceType::Ore,
            remaining: 1320,
        },
    );
    let bales = crate::sim::miner::miner_system::extract_bales_max(
        &mut sim, (5, 5), &config, 0,
    );
    assert!(bales.is_empty(), "zero capacity → no bales");
    let after = sim.production.resource_nodes.get(&(5, 5)).expect("untouched");
    assert_eq!(after.remaining, 1320, "node remaining untouched");
}

#[test]
fn extract_max_node_remaining_zero() {
    let mut sim = Simulation::new_for_test();
    let config = MinerConfig::default();
    // Edge case: node present but somehow with remaining == 0.
    sim.production.resource_nodes.insert(
        (5, 5),
        crate::sim::miner::ResourceNode {
            resource_type: crate::sim::miner::ResourceType::Ore,
            remaining: 0,
        },
    );
    let bales = crate::sim::miner::miner_system::extract_bales_max(
        &mut sim, (5, 5), &config, 40,
    );
    assert!(bales.is_empty(), "remaining==0 → no bales");
}
```

**Step 3: Verify**
Run: `cargo test --lib sim::miner -- extract_max`
Expected: All 7 tests PASS. No existing tests touched.

**Step 4: Commit**
```
sim/miner: add extract_bales_max bulk-drain helper

Per gamemd Harvest_Ore_Tick (0x73D450): one call drains the cell of
min(empty_capacity, cell_density_levels) bales, single overlay update,
single node decrement. Mirrors gamemd's atomic Reduce_Tiberium +
AddAmount pattern. extract_bale (single-bale) untouched, still used by
slave miner.
```

---

### Task 2: Switch `handle_harvest` to use `extract_bales_max`; update failing tests

**Why:** Wires the new helper into the live harvest tick. Before adding new tests in Task 3, we need the existing test suite green — which means updating the tests that encode the "1 bale per 18 ticks" bug.

**Files:**
- Modify: [src/sim/miner/miner_system.rs:406-483](src/sim/miner/miner_system.rs#L406-L483) — `handle_harvest`
- Modify: [src/sim/miner/miner_tests.rs](src/sim/miner/miner_tests.rs) — failing assertions

**Pattern:** Same as existing `handle_harvest`. Replace the per-bale extraction with bulk extraction; the surrounding state machine and timer logic stay identical.

**Step 1: Replace the `handle_harvest` body**

Find the existing function at [miner_system.rs:406](src/sim/miner/miner_system.rs#L406). Replace its body (everything inside the braces) with:

```rust
fn handle_harvest(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
) {
    // Timer countdown.
    if snap.miner.harvest_timer > 0 {
        snap.miner.harvest_timer -= 1;
        return;
    }

    let cell = (snap.rx, snap.ry);
    let empty: u16 = snap
        .miner
        .capacity_bales
        .saturating_sub(snap.miner.cargo.len() as u16);

    let bales = extract_bales_max(sim, cell, config, empty);

    if !bales.is_empty() {
        snap.miner.cargo.extend(bales);
        snap.miner.last_harvest_cell = Some(cell);

        if snap.miner.is_full() {
            begin_return(sim, rules, config, path_grid, snap);
            return;
        }
        // Cell now empty (multi-bale drained it) but miner not full.
        // Reset timer and let next tick fall through to short-scan.
        // Matches gamemd: post-success step counter reset → next call
        // returns 0 → triggers TiberiumShortScan.
        snap.miner.harvest_timer = config.harvest_tick_interval;
        return;
    }

    // No bales extracted (cell empty). Mirrors gamemd Mission_Harvest
    // case 1 when Harvest_Ore_Tick returns 0:
    //   1. Full Harvester → state 2 (return), no scan.
    //   2. Otherwise run a TiberiumShortScan continuation scan from the
    //      current cell. Hit → keep harvesting via MoveToOre.
    //   3. Miss → state 2 (return), regardless of cargo.
    if snap.miner.is_full() {
        begin_return(sim, rules, config, path_grid, snap);
        return;
    }

    let continuation_target = {
        let reachable_filter = build_reachable_filter(sim, snap);
        let filter_ref: Option<&dyn Fn((u16, u16)) -> bool> = reachable_filter.as_deref();
        search_local_ore(
            &sim.production.resource_nodes,
            (snap.rx, snap.ry),
            config.local_continuation_radius,
            filter_ref,
        )
    };
    if let Some(next_cell) = continuation_target {
        snap.miner.target_ore_cell = Some(next_cell);
        snap.miner.state = MinerState::MoveToOre;
        return;
    }

    begin_return(sim, rules, config, path_grid, snap);
}
```

**Step 2: Run the test suite to identify breakage**

Run: `cargo test --lib sim::miner`

Expected: Some tests fail. Specifically, any test asserting "after N harvest ticks, cargo has 1 bale" or similar single-bale-per-tick patterns. Common candidates by name:
- Tests with "harvest" + a tick count assertion
- Tests that fill cargo over multiple harvest cycles

**Step 3: Update each failing test**

For each failing test, change the assertion from "1 bale per cycle" to "N bales per cycle" where N = `min(empty_capacity_at_cycle_start, cell_density_levels_at_cycle_start)`.

Concretely:
- If a test seeds a density-1 cell and expects 1 bale after 18 ticks → still 1 bale (capacity not the limit), unchanged
- If a test seeds a density-11 cell with an empty 40-capacity miner and expects "1 bale after 18 ticks, 2 bales after 36 ticks" → change to "11 bales after 18 ticks, miner moves to next cell"
- If a test seeds a density-3 cell and asserts intermediate-tick state → expect 3 bales after first 18-tick cycle, cell removed

**Do not** update tests that just assert state transitions (Harvest → SearchOre, etc.) — those flows are unchanged.

**Do not** update tests that test `extract_bale` directly (the single-bale function is still used by slave miner; its tests are still valid).

**Step 4: Verify**
Run: `cargo test --lib sim::miner`
Expected: All tests PASS.

**Step 5: Commit**
```
sim/miner: handle_harvest drains cell in one extraction call

Switches from per-bale extract_bale to bulk extract_bales_max. Mirrors
gamemd's Harvest_Ore_Tick pattern: one 18-frame wait, then drain
min(empty_capacity, cell_density) bales in one shot. Cell-empty branch
unchanged (TiberiumShortScan continuation). Updates existing tests that
encoded the per-bale-per-tick bug.
```

---

### Task 3: Add integration tests for multi-bale extraction behavior

**Why:** Lock in the new contract — failure here in the future means parity regression.

**Files:**
- Modify: [src/sim/miner/miner_tests.rs](src/sim/miner/miner_tests.rs) — new test cases

**Pattern:** Existing integration tests in `miner_tests.rs` use `tick_miners_n` + `spawn_miner` helpers (already present in that file). New tests follow the same pattern.

**Step 1: Add three integration tests**

Append the following tests (using the existing `spawn_miner` / `tick_miners_n` / `seed_ore_at` test helpers — confirm exact names by reading the top of `miner_tests.rs` first):

```rust
#[test]
fn harvester_drains_full_cell_in_one_extraction_tick() {
    let mut sim = Simulation::new_for_test();
    let rules = test_rules();
    let config = MinerConfig::default();

    // 11-density ore cell at (20, 20).
    sim.production.resource_nodes.insert(
        (20, 20),
        crate::sim::miner::ResourceNode {
            resource_type: crate::sim::miner::ResourceType::Ore,
            remaining: 11 * 120,
        },
    );

    // War Miner (capacity 40) starts on the ore cell, already in Harvest state.
    let miner_id = spawn_miner_at(&mut sim, 1, MinerKind::War, 20, 20);
    set_miner_state(&mut sim, miner_id, MinerState::Harvest);
    set_harvest_timer(&mut sim, miner_id, config.harvest_tick_interval);

    // Tick exactly harvest_tick_interval times. On the LAST tick the
    // extraction fires; the cell drains completely in one call.
    tick_miners_n(&mut sim, &rules, config.harvest_tick_interval as u32);

    let miner = miner_component(&sim, miner_id);
    assert_eq!(miner.cargo.len(), 11, "full cell drained in one tick");
    assert!(
        sim.production.resource_nodes.get(&(20, 20)).is_none(),
        "cell removed after full drain"
    );
}

#[test]
fn harvester_caps_extraction_at_remaining_capacity() {
    let mut sim = Simulation::new_for_test();
    let rules = test_rules();
    let config = MinerConfig::default();

    sim.production.resource_nodes.insert(
        (20, 20),
        crate::sim::miner::ResourceNode {
            resource_type: crate::sim::miner::ResourceType::Ore,
            remaining: 11 * 120,
        },
    );

    // War Miner with 38 of 40 bales already loaded.
    let miner_id = spawn_miner_at(&mut sim, 1, MinerKind::War, 20, 20);
    preload_cargo(&mut sim, miner_id, 38);
    set_miner_state(&mut sim, miner_id, MinerState::Harvest);
    set_harvest_timer(&mut sim, miner_id, config.harvest_tick_interval);

    tick_miners_n(&mut sim, &rules, config.harvest_tick_interval as u32);

    let miner = miner_component(&sim, miner_id);
    assert_eq!(miner.cargo.len(), 40, "capped at capacity");

    // 2 bales extracted from an 11-density cell → 9 levels remain.
    let after = sim
        .production
        .resource_nodes
        .get(&(20, 20))
        .expect("cell still has ore");
    assert_eq!(after.remaining, 9 * 120, "cell drops to density 9");
}

#[test]
fn harvester_continues_to_short_scan_when_partial_then_empty() {
    let mut sim = Simulation::new_for_test();
    let rules = test_rules();
    let config = MinerConfig::default();

    // Density-5 cell at (20, 20). Another density-5 cell at (21, 20).
    sim.production.resource_nodes.insert(
        (20, 20),
        crate::sim::miner::ResourceNode {
            resource_type: crate::sim::miner::ResourceType::Ore,
            remaining: 5 * 120,
        },
    );
    sim.production.resource_nodes.insert(
        (21, 20),
        crate::sim::miner::ResourceNode {
            resource_type: crate::sim::miner::ResourceType::Ore,
            remaining: 5 * 120,
        },
    );

    let miner_id = spawn_miner_at(&mut sim, 1, MinerKind::War, 20, 20);
    set_miner_state(&mut sim, miner_id, MinerState::Harvest);
    set_harvest_timer(&mut sim, miner_id, config.harvest_tick_interval);

    // First cycle: drain (20,20), 5 bales.
    tick_miners_n(&mut sim, &rules, config.harvest_tick_interval as u32);
    {
        let miner = miner_component(&sim, miner_id);
        assert_eq!(miner.cargo.len(), 5);
        assert_eq!(miner.state, MinerState::Harvest, "stays in Harvest, timer reset");
    }

    // Second cycle: cell now empty → short scan → MoveToOre to (21,20).
    tick_miners_n(&mut sim, &rules, config.harvest_tick_interval as u32);
    {
        let miner = miner_component(&sim, miner_id);
        assert_eq!(
            miner.state,
            MinerState::MoveToOre,
            "transitions to MoveToOre after empty-cell scan finds neighbour"
        );
        assert_eq!(miner.target_ore_cell, Some((21, 20)));
    }
}
```

**Note:** the helper names (`spawn_miner_at`, `set_miner_state`, `set_harvest_timer`, `preload_cargo`, `miner_component`, `test_rules`) above are illustrative — Task 3's first action MUST be to read the top of `miner_tests.rs` and substitute the actual helper names used in that file. If a needed helper doesn't exist, the test author adds a one-line wrapper in `miner_tests.rs` (still in the test module) rather than inventing a new module.

**Step 2: Verify**
Run: `cargo test --lib sim::miner -- harvester_drains harvester_caps harvester_continues`
Expected: All 3 new tests PASS.

**Step 3: Commit**
```
sim/miner: integration tests for bulk-drain harvest cadence

Locks in parity contract: full-cell drain in one extraction call (#1),
capacity-capped extraction, and partial-then-empty short-scan
continuation. Reference: docs/plans/2026-05-12-miner-multi-bale-
extraction-design.md ledger items 1, 5, 6.
```

---

### Task 4: Thread `&MinerConfig` to `phase_linked`

**Why:** Prerequisite for Task 5. `phase_linked` is where the dock first-bale `unload_timer` gets initialised, and it currently doesn't receive `config`. Adding the parameter cleanly first keeps Task 5 a single-line fix.

**Files:**
- Modify: [src/sim/miner/miner_dock_sequence.rs:241-287](src/sim/miner/miner_dock_sequence.rs#L241-L287) — `handle_dock_sequence` dispatch
- Modify: [src/sim/miner/miner_dock_sequence.rs:325-357](src/sim/miner/miner_dock_sequence.rs#L325-L357) — `phase_linked` signature

**Pattern:** Mirrors `phase_unloading` which already takes `config: &MinerConfig` ([miner_dock_sequence.rs:359](src/sim/miner/miner_dock_sequence.rs#L359)).

**Step 1: Update `phase_linked` signature**

Change [line 325-330](src/sim/miner/miner_dock_sequence.rs#L325-L330) from:

```rust
fn phase_linked(
    sim: &mut Simulation,
    rules: &RuleSet,
    snap: &mut MinerSnapshot,
    pad: (u16, u16),
    ref_sid: u64,
) {
```

to:

```rust
fn phase_linked(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    snap: &mut MinerSnapshot,
    pad: (u16, u16),
    ref_sid: u64,
) {
```

**Step 2: Update the dispatch call**

At [miner_dock_sequence.rs:273-275](src/sim/miner/miner_dock_sequence.rs#L273-L275) (inside `handle_dock_sequence`), change:

```rust
RefineryDockPhase::Linked => {
    phase_linked(sim, rules, snap, pad, ref_sid);
}
```

to:

```rust
RefineryDockPhase::Linked => {
    phase_linked(sim, rules, config, snap, pad, ref_sid);
}
```

**Step 3: Verify compilation**
Run: `cargo check --lib`
Expected: Clean compile (no warnings about unused `config` — Task 5 will use it).

If `clippy` flags `config` as unused, add `#[expect(unused_variables, reason = "wired in Task 5")]` as a temporary annotation that Task 5 removes. Prefer this over `_config` rename since Task 5 needs the real name.

**Step 4: Commit**
```
sim/miner: plumb &MinerConfig through phase_linked

Prep for the dock first-bale timing fix. Mirrors phase_unloading's
existing config parameter. No behavior change yet.
```

---

### Task 5: Initialise `unload_timer` to `unload_tick_interval` + add dock first-bale test

**Why:** The 1-line parity fix for gap-scan finding #2, plus the test that locks it in.

**Files:**
- Modify: [src/sim/miner/miner_dock_sequence.rs:355](src/sim/miner/miner_dock_sequence.rs#L355) — `unload_timer` init value
- Modify: [src/sim/miner/miner_tests.rs](src/sim/miner/miner_tests.rs) — new test

**Pattern:** Drop-in constant replacement at one site. Test mirrors existing dock-timing tests.

**Step 1: Change `unload_timer` init**

At [miner_dock_sequence.rs:352-356](src/sim/miner/miner_dock_sequence.rs#L352-L356), replace:

```rust
    // Initialize unload_timer to 0 — first bale fires after one full
    // unload_tick_interval, matching gamemd's per-bale gate.
    snap.miner.unload_timer = 0;
```

with:

```rust
    // Initialize unload_timer to one full interval — first bale fires
    // after ~14.4 frames (= unload_tick_interval/10), matching gamemd's
    // per-bale gate at HarvesterDumpRate × 900 ≤ field_0x3E with the
    // counter initialised to 0 on dock-link (see
    // ra2-rust-game-docs/REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §9.1).
    snap.miner.unload_timer = config.unload_tick_interval as i16;
```

Also remove the temporary `#[expect(unused_variables, ...)]` if Task 4 added one.

**Step 2: Add the test**

Append to `miner_tests.rs` (same conventions as Task 3 re: helper names — read the file first):

```rust
#[test]
fn dock_first_bale_waits_one_unload_interval() {
    let mut sim = Simulation::new_for_test();
    let rules = test_rules();
    let config = MinerConfig::default();

    // Standard 4×3 refinery at (10, 10) with 1 reserved dock slot.
    spawn_refinery(&mut sim, 100, 10, 10);

    // War Miner pre-loaded with 5 bales, positioned at the pad cell,
    // already in Unloading phase (skip the Approach / Linked dance for
    // this targeted timing test).
    let miner_id = spawn_miner_at(&mut sim, 1, MinerKind::War, 13, 11);
    preload_cargo(&mut sim, miner_id, 5);
    enter_unloading_phase(&mut sim, miner_id, 100); // helper that sets state=Dock,
                                                     // dock_phase=Unloading,
                                                     // reserves dock,
                                                     // and initialises unload_timer
                                                     // exactly the way phase_linked does.

    let initial_cargo = miner_component(&sim, miner_id).cargo.len();

    // unload_tick_interval = 144 tenths → first bale fires on tick 15 (ceil(14.4)).
    // Ticks 1..14: cargo unchanged.
    let pre_drop_ticks = (config.unload_tick_interval / 10) as u32 - 1; // 14
    tick_miners_n(&mut sim, &rules, pre_drop_ticks);
    assert_eq!(
        miner_component(&sim, miner_id).cargo.len(),
        initial_cargo,
        "no bale should drop in the first {} ticks", pre_drop_ticks
    );

    // Tick 15: first bale deposits.
    tick_miners_n(&mut sim, &rules, 1);
    assert_eq!(
        miner_component(&sim, miner_id).cargo.len(),
        initial_cargo - 1,
        "first bale deposits on the 15th tick after Unloading entry"
    );
}
```

If `enter_unloading_phase` doesn't exist as a test helper, the test author adds it as a small wrapper in `miner_tests.rs` that reproduces exactly what `phase_linked` does (reserve dock, set `state=Dock` + `dock_phase=Unloading`, set `unload_timer = config.unload_tick_interval as i16`). The wrapper must NOT bypass `phase_linked`'s init logic — the test exists to verify that init.

**Alternative if the wrapper is awkward:** drive the test through the full FSM (spawn refinery, spawn miner at queue cell, tick through Approach → Linked → Unloading) and start asserting only after the Linked → Unloading transition is observed. Reuse an existing dock-FSM test as the template.

**Step 3: Verify**
Run: `cargo test --lib sim::miner -- dock_first_bale`
Expected: PASS.

Also re-run the full miner test suite to confirm existing dock tests still pass: `cargo test --lib sim::miner`.

**Step 4: Commit**
```
sim/miner: first dock bale waits one unload_tick_interval

Per gamemd per-bale gate (HarvesterDumpRate × 900 ≤ field_0x3E,
counter initialised to 0 on dock-link): first bale waits ~14.4 frames
after Linked → Unloading instead of firing on the same tick. New test
locks in the 15-tick first-bale delay.

Closes gap-scan miner-deep finding #2.
```

---

### Task 6: Full regression sweep + parity verification

**Why:** Confirm no surprise breakage outside the miner module, and verify the observable behavior actually matches gamemd in a live skirmish.

**Files:** None modified.

**Step 1: Run the full test suite**
Run: `cargo test --lib`
Expected: All tests PASS. If any failures outside `sim::miner`, investigate — they're likely unrelated, but the timing change could surface latent assumptions.

**Step 2: Run clippy**
Run: `cargo clippy --lib -- -D warnings`
Expected: No warnings. The new helper and tests should be clippy-clean.

**Step 3: Run fmt**
Run: `cargo fmt --check`
If failures: `cargo fmt`, then re-verify with `--check`.

**Step 4: Manual parity verification against gamemd**

This is the parity gate per CLAUDE.md — code passing tests is not the same as code matching gamemd.

Perform a side-by-side observation on a fresh skirmish map with at least one ore field:

1. Launch this engine, load a small map, drop a refinery + harvester.
2. Note the time the harvester arrives at the first ore cell.
3. Note the time the harvester leaves that cell with cargo.
4. Time-to-drain should be ~18 ticks (~1.2 seconds at 15fps) per cell visit — NOT ~198 ticks.
5. Visually confirm the ore overlay frame drops in one step (full drain) or one step from initial density (partial drain), not 11 incremental steps.
6. At dock arrival, time-to-first-credit-deposit should be ~15 ticks (~1 second), not immediate.

If the harvester drains a cell in ~1 second and the first credit appears ~1 second after docking, parity is achieved.

If observations don't match, the implementation has a bug — STOP and diagnose; do not commit/declare done.

**Step 5: (No commit needed unless fmt produced changes; in that case)**
```
style: cargo fmt across miner harvest changes
```

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-12-miner-multi-bale-extraction-design.md](docs/plans/2026-05-12-miner-multi-bale-extraction-design.md)
- **Gap-scan source:** [docs/gap-scans/2026-05-12-gap-scan-miner-deep.md](docs/gap-scans/2026-05-12-gap-scan-miner-deep.md) findings #1, #2
- **Ghidra reports:**
  - [MISSION_HARVEST_GHIDRA_REPORT.md](ra2-rust-game-docs/MISSION_HARVEST_GHIDRA_REPORT.md)
  - [HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md](ra2-rust-game-docs/HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md)
  - [HARVESTER_DOCK_UNLOAD.md](ra2-rust-game-docs/HARVESTER_DOCK_UNLOAD.md)
  - [REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §9.1](ra2-rust-game-docs/REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md)
- **gamemd.exe addresses** (kept here, not in Rust comments):
  - `UnitClass::Harvest_Ore_Tick` @ 0x73D450 — multi-bale extraction pattern
  - `UnitClass::Mission_Harvest` @ 0x73E5E0 — case 1 step-counter wait
  - `CellClass::Reduce_Tiberium` @ 0x480A80 — overlay update on full/partial drain
  - `BuildingClass::ReleaseDockedHarvester` @ 0x4595C0 — release helper (referenced for context)
- **INI keys (already plumbed, no changes):**
  - [General] `HarvesterLoadRate=2` ([ini/rulesmd.ini:312](ini/rulesmd.ini#L312) area)
  - [General] `HarvesterDumpRate=0.016` (default)
  - [HARV] `Storage=40` ([ini/rulesmd.ini:8236](ini/rulesmd.ini#L8236))
  - [CMIN] `Storage=20` ([ini/rulesmd.ini:7374](ini/rulesmd.ini#L7374))
- **Related code:**
  - [src/sim/miner/miner_system.rs:643](src/sim/miner/miner_system.rs#L643) — existing `extract_bale` (unchanged)
  - [src/sim/slave_miner.rs:253](src/sim/slave_miner.rs#L253) — second caller of `extract_bale` (unchanged)
  - [src/sim/miner/miner_dock_sequence.rs:355](src/sim/miner/miner_dock_sequence.rs#L355) — `unload_timer` init site (changed in Task 5)
- **Precondition (uncommitted state at plan time):** working-tree diff in `miner_dock_sequence.rs` (spiral `refinery_exit_cell` rework) and `miner_tests.rs` (test updates for that rework) — committed or kept-in-tree is fine; tasks above target the working-tree state. If reverted, line numbers in Tasks 4-5 need re-mapping.
