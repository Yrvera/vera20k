# Miner Short-Scan Continuation Before Return — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** When a harvester's current ore cell depletes with partial cargo, scan within `TiberiumShortScan` radius for more ore and continue harvesting if found; only return to refinery if the short scan fails.

**Architecture:** Lives entirely inside [src/sim/miner/miner_system.rs](src/sim/miner/miner_system.rs). The fix replaces the eager-return branch in `handle_harvest` (lines 433-441) with a short-scan-then-return cascade. The reachability filter currently inlined in `handle_search_ore` is extracted into a private helper and shared between the two call sites. No struct changes, no public API changes, no tick-order changes.

**Design Doc:** [docs/plans/2026-05-11-miner-short-scan-continuation-design.md](docs/plans/2026-05-11-miner-short-scan-continuation-design.md)

---

## Grounding Summary

- **What the docs say:** [HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md §2 State 1](docs/research/HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md) describes gamemd's per-tick on-cell behavior. On extraction failure (cell empty), if the miner is not full, `FootClass__Search_For_Tiberium_And_Move(TiberiumShortScan, 0)` runs. Hit → set destination, stay in state 1. Miss + no existing destination → state 2 (return).
- **Ghidra verification:** Decompiled `UnitClass::Mission_Harvest` at `0x0073E5E0`. State 1 path confirmed: `if (cVar1 == '\0' && param_1[0x169] == 0) state = 2; else state = 1`. Doc was right on State 1. Doc was *wrong* on State 0 (claimed chrono uses `TiberiumShortScan`; binary shows both normal and chrono use `TiberiumLongScan` via `+0x177C`). Doc fix is out-of-scope, listed under Open Follow-ups in design doc.
- **Repo pattern:** [handle_search_ore](src/sim/miner/miner_system.rs#L216-L311) is the canonical model for "build reachability filter → call `search_local_ore`". Its filter setup (lines 228-249) is what we extract; its `search_local_ore` call shape (line 252-261) is what we reuse in `handle_harvest`. `MinerState::MoveToOre` is the existing transition target for "go to a freshly chosen ore cell."
- **INI keys driving behavior:** `[General] TiberiumShortScan` (default 6) — already parsed into `GeneralRules::tiberium_short_scan` ([rules/ruleset.rs](src/rules/ruleset.rs)) and wired into `MinerConfig::local_continuation_radius` via `MinerConfig::from_general_rules` ([miner/mod.rs:188-207](src/sim/miner/mod.rs#L188-L207)). No new INI keys needed.
- **Git state:** `git log --oneline -10` on `src/sim/miner/miner_system.rs` shows the last 10 commits all predate this design and touch chrono sound, dock-phase consolidation, exit cell, and per-unit Storage. None touch `handle_harvest`'s cell-depletion path. Design premise is current.
- **Still unknown:** `search_local_ore`'s ranking (gems>ore→density→distance→tie-break) versus gamemd's diamond-spiral best-value-in-ring. Out of scope per design doc.

## Key Technical Decisions

- **Extract `build_reachable_filter` rather than inline-duplicate** — design Approach B. **Confidence:** high. **Source:** Design doc §"Chosen Approach"; repo pattern at [miner_system.rs:228-249](src/sim/miner/miner_system.rs#L228-L249).
- **Use `(snap.rx, snap.ry)` as scan center, not `last_harvest_cell`** — State 1 in gamemd centers on the unit's current position (where it just was). **Confidence:** high. **Source:** Ghidra `0x0073E5E0` case 1 (after `Harvest_Ore_Tick`); the scan call passes no explicit center because the function reads it from `param_1` itself, which is the unit instance.
- **Transition target is `MoveToOre`, not staying in `Harvest`** — gamemd "stays in state 1" by setting a destination; our equivalent is `MoveToOre`, which pathfinds and re-enters `Harvest` on arrival. Observable result identical. **Confidence:** high. **Source:** Existing usage at [handle_search_ore:259](src/sim/miner/miner_system.rs#L259).
- **Empty-cargo cell-depletion falls through to `SearchOre`, not `begin_return`** — user-accepted minor drift. gamemd would return-to-refinery with 0 cargo here. Observably equivalent (miner ends up looking for ore again). **Confidence:** high. **Source:** User decision in brainstorm.
- **Scan radius from `config.local_continuation_radius`** — already wired to `[General] TiberiumShortScan` via `from_general_rules`. **Confidence:** high. **Source:** [miner/mod.rs:198](src/sim/miner/mod.rs#L198).

## Open Questions

### Resolved During Planning

- **Does extracting the helper require lifetime annotations?** Yes — closures capture `&Simulation` and a `MovementZone`/`MovementLayer` by value. Same lifetime pattern as the existing inline code: `Box<dyn Fn((u16, u16)) -> bool + 'a>`.
- **Does the new short-scan branch need to clear `target_ore_cell` if scan misses?** No — when miss path hits `begin_return`, that function manages its own state; when miss path hits `state = SearchOre`, the SearchOre handler clears/sets `target_ore_cell` on its next tick. Same semantics as today's empty-cargo path.
- **Does `last_harvest_cell` need updating?** No — only `extract_bale` paths set it (line 413 today), and we're not changing that. The fix only affects what happens *after* a cell empties.

### Deferred to Implementation

- **Existing-destination guard equivalent.** gamemd's `param_1[0x169] == 0` check stays in state 1 if a destination is already set on scan miss. Our `Harvest` state implies the miner has arrived (no `movement_target`). Skip the guard for now; flag if it surfaces as a visible bug.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/miner/miner_system.rs` | Extract `build_reachable_filter` helper, refactor `handle_search_ore` to call it, modify `handle_harvest` cell-depletion branch to short-scan before returning |
| Modify | `src/sim/miner/miner_tests.rs` | Strengthen existing Test 9 assertion (currently permissive), add two new tests for short-scan-miss-with-cargo and short-scan-miss-empty-cargo |

No new files. No public API changes.

## Interface Changes

- **New private fn:** `build_reachable_filter<'a>(sim: &'a Simulation, snap: &MinerSnapshot) -> Option<Box<dyn Fn((u16, u16)) -> bool + 'a>>` — internal to `miner_system.rs`, not exported.
- **No public interface changes.** `search_local_ore`, `Miner`, `MinerState`, `MinerConfig`, `tick_miners`, all unchanged.

## Sim Checklist

- [x] All math uses `fixed`-point — no f32/f64 introduced (this fix uses only `u16` cell coords and `u32`/`u8` counters that already exist)
- [x] New state included in deterministic state hash — no new state added
- [x] No dependencies on render/ui/sidebar/audio/net — fix lives entirely in `sim/miner/`
- [x] Tick ordering impact: zero — same number of state transitions per tick, same handlers
- [x] BTreeMap iteration order: `search_local_ore` iterates `BTreeMap<(u16,u16), ResourceNode>` deterministically; unchanged

## Risk Areas

From the design doc impact analysis:

- **VFX cadence (voxel anim, harvest overlay):** Phase 4/4b in `tick_miners` gates on `state == Harvest`. When the new branch transitions to `MoveToOre`, VFX correctly clears. Verified by code inspection; covered by existing tests indirectly. No regression test needed.
- **Existing `local_continuation_after_cell_depletes` test (Test 9):** currently asserts a permissive OR-condition that passes under both old and new behavior. After this fix, that test should pass with the STRONGER assertion (nearby ore IS picked, miner does NOT return). Strengthening is a Task in this plan.
- **Refactor of `handle_search_ore`:** byte-for-byte filter behavior must be preserved. Verified by running all existing miner tests after the refactor (Task 3) before adding the short-scan logic.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 4 | Short-scan radius = `TiberiumShortScan` (default 6 cells) from `config.local_continuation_radius` | Player-observable: harvesters near ore fields fill up before going home, instead of yo-yoing to the refinery on every depleted cell. Fires on every cell-depletion in normal play (many times per minute). | New test `harvest_continues_to_nearby_ore_when_cell_depletes_partial_cargo` (Task 5) asserts `MoveToOre` transition with target == nearby cell. |
| Task 4 | Short-scan miss + cargo > 0 → return to refinery | Player-observable: when the ore patch is genuinely exhausted, harvester goes home (not stuck or wandering). | New test `harvest_returns_when_no_ore_within_short_scan` (Task 5) asserts `ReturnToRefinery` transition. |
| Task 4 | Short-scan miss + cargo == 0 → fall through to `SearchOre` (4-stage cascade) | Empty-cargo miner on a depleted cell shouldn't head home with 0 bales when ore exists farther out. | New test `empty_cargo_falls_back_to_full_search` (Task 5) asserts miner finds far ore. |
| Task 6 | Existing Test 9 strengthened to assert the nearby-ore path is actually taken | Today's test passes under buggy behavior. Strengthening it locks in the new behavior as a regression guard. | Update assertion from permissive OR to exact `target_ore_cell == Some((22, 20))`. |

---

## Tasks

### Task 1: Extract `build_reachable_filter` helper

**Why:** Eliminate the 20-line duplication that would otherwise result when `handle_harvest` needs the same filter. Pure refactor — no behavior change. Done first because both subsequent edits depend on it.

**Files:**
- Modify: `src/sim/miner/miner_system.rs` (new fn near `handle_search_ore`)

**Pattern:** Follows the same `Box<dyn Fn>` pattern already used at [miner_system.rs:241-249](src/sim/miner/miner_system.rs#L241-L249). No new pattern.

**Step 1: Add the helper fn**

Add this function in `src/sim/miner/miner_system.rs`, immediately before `handle_search_ore` (around line 215, after `process_miner` and before the `// -- State handlers --` block — anywhere is fine, but keeping it near the only two callers helps discoverability):

```rust
/// Build a zone-grid-based reachability filter for ore scans.
///
/// Returns `None` if any of (zone_grid, locomotor, effective zone cell)
/// is missing. In that case the caller falls back to an unfiltered scan
/// for this tick — the next tick will likely succeed once the harvester
/// moves to a passable cell.
///
/// Shared by `handle_search_ore` (State 0 fresh search) and
/// `handle_harvest` (State 1 cell-depletion continuation scan).
fn build_reachable_filter<'a>(
    sim: &'a Simulation,
    snap: &MinerSnapshot,
) -> Option<Box<dyn Fn((u16, u16)) -> bool + 'a>> {
    let entity = sim.entities.get(snap.entity_id);
    let mz = entity
        .and_then(|e| e.locomotor.as_ref())
        .map(|loc| loc.movement_zone)
        .unwrap_or(MovementZone::Normal);
    let layer = entity
        .map(|e| e.movement_layer_or_ground())
        .unwrap_or(MovementLayer::Ground);
    let harvester_anchor = sim
        .zone_grid
        .as_ref()
        .and_then(|zg| effective_zone_cell(zg, mz, snap.rx, snap.ry));

    match (sim.zone_grid.as_ref(), harvester_anchor) {
        (Some(zg), Some(anchor)) => Some(Box::new(move |ore_cell: (u16, u16)| {
            ore_reachable(zg, mz, layer, anchor, ore_cell)
        })),
        _ => None,
    }
}
```

**Step 2: Verify compile**

Run: `cargo check -p vera20k --lib`
Expected: compiles clean (the helper is unused for now — that's fine; the next task removes the inline duplicate).

**Step 3: Commit**

Commit message: `sim/miner: extract build_reachable_filter helper`

---

### Task 2: Refactor `handle_search_ore` to use the helper

**Why:** Replace the inline filter setup with the helper call. Pure refactor — same behavior. Must be done before Task 4 introduces a second caller, so that the second caller mirrors a known-good shape.

**Files:**
- Modify: `src/sim/miner/miner_system.rs` (handle_search_ore body)

**Pattern:** Removing duplication; no new pattern.

**Step 1: Replace the inline filter setup in `handle_search_ore`**

In `src/sim/miner/miner_system.rs`, find this block (currently lines 228-249):

```rust
    // Build a reachability filter from the zone grid + harvester locomotor.
    // If any of (zone_grid, locomotor, effective zone cell) is missing, fall
    // back to unfiltered search for this tick — the next tick will likely
    // succeed once the harvester moves to a passable cell.
    let entity = sim.entities.get(snap.entity_id);
    let mz = entity
        .and_then(|e| e.locomotor.as_ref())
        .map(|loc| loc.movement_zone)
        .unwrap_or(MovementZone::Normal);
    let layer = entity
        .map(|e| e.movement_layer_or_ground())
        .unwrap_or(MovementLayer::Ground);
    let harvester_anchor = sim
        .zone_grid
        .as_ref()
        .and_then(|zg| effective_zone_cell(zg, mz, snap.rx, snap.ry));

    type OreFilter<'a> = dyn Fn((u16, u16)) -> bool + 'a;
    let reachable_filter: Option<Box<OreFilter<'_>>> =
        match (sim.zone_grid.as_ref(), harvester_anchor) {
            (Some(zg), Some(anchor)) => Some(Box::new(move |ore_cell: (u16, u16)| {
                ore_reachable(zg, mz, layer, anchor, ore_cell)
            })),
            _ => None,
        };
    let filter_ref: Option<&dyn Fn((u16, u16)) -> bool> = reachable_filter.as_deref();
```

Replace it with:

```rust
    // Reachability filter — see build_reachable_filter for the fallback
    // semantics when zone_grid / locomotor / anchor is missing.
    let reachable_filter = build_reachable_filter(sim, snap);
    let filter_ref: Option<&dyn Fn((u16, u16)) -> bool> = reachable_filter.as_deref();
```

The rest of `handle_search_ore` (the four scan stages and their use of `filter_ref`) stays unchanged.

**Step 2: Verify compile**

Run: `cargo check -p vera20k --lib`
Expected: compiles clean. The `OreFilter` type alias at the old line 241 is removed by the replacement above; if any other code referenced it, the compiler will flag that. (None should — verified by [Grep above](#grounding-summary).)

**Step 3: Run miner tests to confirm no regression**

Run: `cargo test -p vera20k --lib sim::miner -- --nocapture`
Expected: all existing miner tests pass. If any fail, the filter behavior changed during extraction — investigate before proceeding.

**Step 4: Commit**

Commit message: `sim/miner: route handle_search_ore through build_reachable_filter`

---

### Task 3: Add the short-scan branch to `handle_harvest`

**Why:** The user-visible fix. Replaces the eager-return at lines 433-441 with a short-scan-then-return cascade matching gamemd State 1 cell-depletion behavior.

**Files:**
- Modify: `src/sim/miner/miner_system.rs` (handle_harvest tail)

**Pattern:** Mirrors `handle_search_ore`'s first stage (short-radius scan + transition to `MoveToOre` on hit). Uses the helper from Task 1.

**Step 1: Replace the cell-depletion tail of `handle_harvest`**

In `src/sim/miner/miner_system.rs`, find this block at the end of `handle_harvest` (currently lines 433-441):

```rust
    // Cell depleted (or was already empty). If we have some cargo, return.
    if !snap.miner.cargo.is_empty() {
        begin_return(sim, rules, config, path_grid, snap);
        return;
    }

    // No cargo — search for more ore (local continuation).
    snap.miner.state = MinerState::SearchOre;
```

Replace it with:

```rust
    // Cell depleted. Per gamemd State 1 (Mission_Harvest case 1, after
    // Harvest_Ore_Tick returns 0): before returning, do a short-radius
    // continuation scan from the unit's current position. If ore is
    // reachable within TiberiumShortScan, set it as the new target and
    // keep harvesting. Only if nothing is found nearby does the miner
    // return to refinery (with cargo) or fall back to the full
    // SearchOre cascade (empty cargo).
    let reachable_filter = build_reachable_filter(sim, snap);
    let filter_ref: Option<&dyn Fn((u16, u16)) -> bool> = reachable_filter.as_deref();
    if let Some(next_cell) = search_local_ore(
        &sim.production.resource_nodes,
        (snap.rx, snap.ry),
        config.local_continuation_radius,
        filter_ref,
    ) {
        snap.miner.target_ore_cell = Some(next_cell);
        snap.miner.state = MinerState::MoveToOre;
        return;
    }

    // No nearby ore. If we have cargo, head home. Otherwise fall back
    // to SearchOre's wider cascade (matches existing empty-cargo
    // behavior; gamemd would return-to-refinery here too but our 4-stage
    // SearchOre cascade is observably equivalent).
    if !snap.miner.cargo.is_empty() {
        begin_return(sim, rules, config, path_grid, snap);
        return;
    }
    snap.miner.state = MinerState::SearchOre;
```

**Step 2: Verify compile**

Run: `cargo check -p vera20k --lib`
Expected: compiles clean.

**Step 3: Commit**

Commit message: `sim/miner: short-scan continuation before refinery return`

---

### Task 4: Add new tests for the short-scan behavior

**Why:** Lock in the new behavior with assertions strong enough to catch a regression. Three cases: (a) nearby ore found, miner continues; (b) no nearby ore + partial cargo, miner returns; (c) no nearby ore + empty cargo, miner falls back to full search.

**Files:**
- Modify: `src/sim/miner/miner_tests.rs` (append two new tests after Test 9 at line 627)

**Pattern:** Mirrors existing Test 9 setup (`miner_rules`, `spawn_miner`, `spawn_refinery`, `place_ore`, `tick_miners_n`). Uses `War` miner kind (chrono and war share the same continuation behavior; war avoids the teleport delay that complicates assertions).

**Step 1: Add three tests appended after Test 9**

In `src/sim/miner/miner_tests.rs`, locate the end of Test 9 at line 627 (closing `}` of `local_continuation_after_cell_depletes`). After that closing brace, before Test 10's `// ====` separator, insert:

```rust
// ==========================================================================
// Test 9a: Cell depletes with PARTIAL cargo → miner continues to nearby ore
//          (the short-scan-before-return behavior, gamemd State 1)
// ==========================================================================
#[test]
fn harvest_continues_to_nearby_ore_when_cell_depletes_partial_cargo() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 20, 20);
    spawn_refinery(&mut sim, 2, 10, 10);
    // Cell at miner's position: depletes after 2 bales.
    place_ore(&mut sim, 20, 20, 2);
    // Nearby ore well within TiberiumShortScan (radius 6 cells).
    place_ore(&mut sim, 23, 20, 100);

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::Harvest;
        miner.target_ore_cell = Some((20, 20));
        miner.harvest_timer = 0;
    }

    // Tick enough to deplete (20,20) and trigger the continuation scan.
    tick_miners_n(&mut sim, &rules, 30);

    let miner = get_miner(&sim, miner_id);
    assert!(
        !miner.cargo.is_empty(),
        "Miner should have extracted bales before cell depleted"
    );
    assert_eq!(
        miner.target_ore_cell,
        Some((23, 20)),
        "After cell depleted, miner should pick the nearby ore via short scan"
    );
    assert!(
        matches!(miner.state, MinerState::MoveToOre | MinerState::Harvest),
        "Miner should move to / be harvesting the new ore cell, not return-to-refinery; \
         state was {:?}",
        miner.state,
    );
    assert!(
        !matches!(
            miner.state,
            MinerState::ReturnToRefinery | MinerState::Dock
        ),
        "Miner with ore nearby must NOT head to refinery on partial cargo"
    );
}

// ==========================================================================
// Test 9b: Cell depletes with PARTIAL cargo + no ore nearby → miner returns
// ==========================================================================
#[test]
fn harvest_returns_when_no_ore_within_short_scan() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 20, 20);
    spawn_refinery(&mut sim, 2, 10, 10);
    // Only the miner's cell has ore. Nothing within the short-scan radius
    // (default 6 cells). The further ore patch is well outside.
    place_ore(&mut sim, 20, 20, 2);
    place_ore(&mut sim, 50, 50, 100); // far outside local_continuation_radius

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::Harvest;
        miner.target_ore_cell = Some((20, 20));
        miner.harvest_timer = 0;
    }

    tick_miners_n(&mut sim, &rules, 30);

    let miner = get_miner(&sim, miner_id);
    assert!(
        !miner.cargo.is_empty(),
        "Miner should have extracted bales before depletion"
    );
    assert!(
        matches!(
            miner.state,
            MinerState::ReturnToRefinery | MinerState::Dock
        ),
        "With cargo but no nearby ore, miner must head to refinery; state was {:?}",
        miner.state,
    );
}

// ==========================================================================
// Test 9c: EMPTY-cargo cell depletion falls back to SearchOre 4-stage cascade
//          (regression guard: ensures the empty-cargo path keeps working)
// ==========================================================================
#[test]
fn empty_cargo_cell_depletion_falls_back_to_full_search() {
    let mut sim = Simulation::new();
    let rules = miner_rules();

    let miner_id = spawn_miner(&mut sim, 1, MinerKind::War, 20, 20);
    spawn_refinery(&mut sim, 2, 10, 10);
    // No ore on the miner's cell when Harvest state runs.
    // Nothing within short-scan radius (6 cells).
    // Ore exists within long-scan radius (default 48).
    place_ore(&mut sim, 40, 20, 100); // ~20 cells away, within long scan

    {
        let entity = sim.entities.get_mut(miner_id).expect("miner");
        let miner = entity.miner.as_mut().expect("miner component");
        miner.state = MinerState::Harvest;
        miner.target_ore_cell = Some((20, 20));
        miner.harvest_timer = 0;
        // Cargo intentionally empty — extract_bale will fail on first tick.
        assert!(miner.cargo.is_empty());
    }

    tick_miners_n(&mut sim, &rules, 5);

    let miner = get_miner(&sim, miner_id);
    assert!(
        miner.cargo.is_empty(),
        "No ore was on the cell, so no bales should have been extracted"
    );
    assert_eq!(
        miner.target_ore_cell,
        Some((40, 20)),
        "Empty-cargo cell depletion should fall through SearchOre and find the \
         long-scan ore at (40, 20)"
    );
    assert!(
        matches!(miner.state, MinerState::MoveToOre),
        "Miner should be heading to the new ore cell; state was {:?}",
        miner.state,
    );
}
```

**Step 2: Verify tests pass**

Run: `cargo test -p vera20k --lib sim::miner::miner_tests::harvest_continues_to_nearby_ore_when_cell_depletes_partial_cargo sim::miner::miner_tests::harvest_returns_when_no_ore_within_short_scan sim::miner::miner_tests::empty_cargo_cell_depletion_falls_back_to_full_search -- --nocapture`
Expected: all three pass.

If `harvest_continues_to_nearby_ore_when_cell_depletes_partial_cargo` fails — the short-scan-then-MoveToOre wiring from Task 3 is broken; re-verify the replacement in `handle_harvest`.

If `harvest_returns_when_no_ore_within_short_scan` fails — check that no stray ore was placed within `local_continuation_radius` (6 cells) of (20, 20).

If `empty_cargo_cell_depletion_falls_back_to_full_search` fails — verify the empty-cargo branch routes to `SearchOre` (line `snap.miner.state = MinerState::SearchOre;` at the end of the new block in Task 3).

**Step 3: Commit**

Commit message: `sim/miner: tests for short-scan continuation behavior`

---

### Task 5: Strengthen existing Test 9 assertion

**Why:** Test 9 (`local_continuation_after_cell_depletes`) currently asserts a permissive OR that passes under both the old (eager-return) and new (short-scan) behavior. Strengthening it locks in the new behavior as a regression guard.

**Files:**
- Modify: `src/sim/miner/miner_tests.rs:619-626`

**Pattern:** Tightening an existing assertion. No new pattern.

**Step 1: Replace the permissive assertion**

In `src/sim/miner/miner_tests.rs`, find the assertion block at the end of `local_continuation_after_cell_depletes` (currently lines 619-626):

```rust
    let miner = get_miner(&sim, miner_id);
    // After depleting (20,20), miner should have found (22,20) via local scan.
    // It should be in MoveToOre or Harvest at the new cell.
    let found_nearby = miner.target_ore_cell == Some((22, 20))
        || matches!(miner.state, MinerState::MoveToOre | MinerState::Harvest);
    assert!(
        found_nearby || !miner.cargo.is_empty(),
        "Miner should find nearby ore via local continuation or have started returning"
    );
```

Replace with:

```rust
    let miner = get_miner(&sim, miner_id);
    // After (20,20) depletes, the short-scan continuation must pick (22,20)
    // and the miner transitions to MoveToOre / Harvest (gamemd State 1
    // depletion path: stay harvesting, move to new cell within
    // TiberiumShortScan radius).
    assert_eq!(
        miner.target_ore_cell,
        Some((22, 20)),
        "Short-scan continuation should pick the nearby ore at (22, 20)"
    );
    assert!(
        matches!(miner.state, MinerState::MoveToOre | MinerState::Harvest),
        "Miner should be moving to / harvesting the new cell; state was {:?}",
        miner.state,
    );
```

**Step 2: Verify**

Run: `cargo test -p vera20k --lib sim::miner::miner_tests::local_continuation_after_cell_depletes -- --nocapture`
Expected: passes (the new behavior makes this assertion true).

**Step 3: Commit**

Commit message: `sim/miner: tighten Test 9 to assert nearby-ore continuation`

---

### Task 6: Final regression run

**Why:** Confirm the full miner test suite passes and the library compiles after all changes.

**Files:** None modified.

**Step 1: Run all miner tests**

Run: `cargo test -p vera20k --lib sim::miner -- --nocapture`
Expected: all tests pass, including Tests 1-12 from the existing suite and the three new ones added in Task 4.

**Step 2: Run a broader test sweep (smoke check for unintended regressions in production / app_context_order / app_cursor)**

Run: `cargo test -p vera20k --lib production app_cursor app_context_order -- --nocapture`
Expected: all pass. (These touch refinery / miner-adjacent code paths.)

**Step 3: Compile-check the whole crate**

Run: `cargo check -p vera20k --lib`
Expected: clean compile (existing warnings about other unrelated unused fns are acceptable).

**Step 4: No commit**

This task only verifies; nothing changed.

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-11-miner-short-scan-continuation-design.md](docs/plans/2026-05-11-miner-short-scan-continuation-design.md)
- **Ghidra report (primary):** [docs/research/HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md](docs/research/HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md) §2 State 1
- **Ghidra report (secondary):** [docs/research/HARVESTER_DOCK_UNLOAD.md](docs/research/HARVESTER_DOCK_UNLOAD.md) — refinery-dock side; not changed by this plan but provides context for `begin_return`
- **gamemd.exe addresses (kept here, NOT in Rust comments per CLAUDE.md):**
  - `0x0073E5E0` — `UnitClass::Mission_Harvest` — state machine including the State 1 cell-depletion path this plan mirrors
  - `0x004DCFE0` — `FootClass::Search_For_Tiberium_And_Move` — the gamemd helper our `search_local_ore + MoveToOre` pair mirrors
  - `RulesClass + 0x1778` — `TiberiumShortScan` (default 6 cells in INI, stored leptons)
- **INI keys:** `[General] TiberiumShortScan` (default 6) — parsed at [src/rules/ruleset.rs](src/rules/ruleset.rs), wired to `MinerConfig::local_continuation_radius` at [src/sim/miner/mod.rs:198](src/sim/miner/mod.rs#L198).
- **Related code:**
  - [src/sim/miner/miner_system.rs](src/sim/miner/miner_system.rs) — the file being changed
  - [src/sim/miner/miner_tests.rs](src/sim/miner/miner_tests.rs) — the test file extended
  - [src/sim/miner/mod.rs](src/sim/miner/mod.rs) — `MinerConfig`, `MinerState` definitions (read-only for this plan)
- **Prior commits** (last 10 on miner module per `git log`): all about chrono sound, dock-phase consolidation, exit cell, per-unit Storage — none touch the cell-depletion path. Design premise is current.
