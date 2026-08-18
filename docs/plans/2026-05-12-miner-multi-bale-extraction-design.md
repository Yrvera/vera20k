# Miner Multi-Bale Extraction (+ Dock First-Bale Timing) — Design

**Date:** 2026-05-12
**Source:** [Gap-scan miner --deep](../gap-scans/2026-05-12-gap-scan-miner-deep.md) findings #1 and #2.

## Goal

Drain an ore cell of all bales that fit in remaining cargo capacity in **one** harvest extraction call (matching gamemd's `UnitClass::Harvest_Ore_Tick` at 0x73D450), and delay the first refinery-dock bale by one `unload_tick_interval` to match gamemd's per-bale gate.

## Architecture Context

The miner harvest pipeline lives in [src/sim/miner/](src/sim/miner/), organised into:

- [mod.rs](src/sim/miner/mod.rs) — data types: `Miner`, `MinerState`, `RefineryDockPhase`, `CargoBale`, `MinerConfig`. Also a separate [`reduce_tiberium`](src/sim/miner/mod.rs#L342) used by combat warhead damage (unrelated to harvest).
- [miner_system.rs](src/sim/miner/miner_system.rs) — `tick_miners` two-phase snapshot/apply driver and per-state handlers (`handle_search_ore`, `handle_move_to_ore`, `handle_harvest`, `handle_return`, …). Exports `extract_bale`, `search_local_ore`, `player_has_purifier`.
- [miner_dock_sequence.rs](src/sim/miner/miner_dock_sequence.rs) — `RefineryDockPhase` FSM (`Approach → Linked → Unloading → Departing`).
- [miner_dock.rs](src/sim/miner/miner_dock.rs) — single-slot dock reservation queue (`DockReservations`).

Adjacent caller: [src/sim/slave_miner.rs:253](src/sim/slave_miner.rs#L253) calls `extract_bale` once per slave walk cycle. The Slave Miner uses a different harvest semantics (infantry walk, scoop one density level, return) and must NOT be changed by this design.

`Miner.cargo: Vec<CargoBale>` stores discrete bales; one bale = one density level extracted from a cell; bale `value` is configured per `ResourceType` (25 for ore, 50 for gem). `Miner.capacity_bales` is per-unit (`Storage=40` for HARV, `Storage=20` for CMIN, read from rules.ini).

Resource nodes live in [Simulation.production.resource_nodes: BTreeMap<(u16,u16), ResourceNode>](src/sim/miner/mod.rs#L38), where `node.remaining` is in "credits-equivalent" units (base=120 for ore, base=180 for gem; density level = `remaining / base`, max 11 per RA2). The visible overlay frame is driven by `OverlayGrid.set_overlay_data(cell, frame)` where frame = density-1.

### gamemd parity (binary truth)

`UnitClass::Harvest_Ore_Tick` (0x73D450, re-decompiled live this session):

```c
iVar1 = *(int *)(param_1[0x1b1] + 0x800);          // Storage (bales)
fVar5 = (float10)StorageClass__GetTotalAmount();    // current load
uVar3 = Math__ftol();                                // floor(Storage - current)
uVar4 = CellClass__Reduce_Tiberium(uVar3);          // clamped to density
if (0 < (int)uVar4) {
    StorageClass__AddAmount((float)(int)uVar4, uVar2);  // ONE add
    param_1[0x3e] = 0;                                  // reset step counter
    param_1[0x43] = HarvesterLoadRate;                  // reload timer
    return 1;
}
```

`Mission_Harvest` case 1 wait: `if (param_1[0x3e] < 9) return 1;` — 9 step-counter ticks at HarvesterLoadRate=2 frames each = 18 frames between successive Harvest_Ore_Tick calls. Each call drains as much as fits.

`Mission_Deploy_Building` state 3 per-bale gate (refinery dump): `HarvesterDumpRate × 900.0 ≤ field_0x3E`; counter initialised to 0 on state-1-entry. First bale waits 15 frames (`ceil(14.4)`) after dock-link.

## Impact Analysis

**Touched:**
- `src/sim/miner/miner_system.rs` — add `extract_bales_max` (~35 lines); adjust `handle_harvest` call site (~10 lines diff).
- `src/sim/miner/miner_dock_sequence.rs:355` — change `unload_timer = 0` to `unload_timer = config.unload_tick_interval as i16`.
- `src/sim/miner/miner_tests.rs` — update tests that encode the per-bale-per-18-ticks bug. Add new tests for `extract_bales_max` and dock first-bale timing.

**Untouched:**
- `Miner`, `CargoBale`, `MinerConfig` data structures.
- `extract_bale` signature and behavior (still consumed by `slave_miner.rs`).
- `reduce_tiberium` (combat path).
- Dock FSM phases, `DockReservations`, refinery dump credit flow.
- rules.ini / artmd.ini — no INI changes.

**Determinism:**
- No new random or float-compare in either change.
- Cargo Vec push order: bales pushed in a deterministic burst (same type, same value, count = `min(empty_cap, density_levels)`).
- State hash recomputed end-of-tick — internal mid-tick state never observed by hash.
- Replay-compat: existing replays' cargo-count timing will diverge (the bug is fixed). This is expected — same as any parity fix.

**Risk:**
- Tests asserting "18 ticks per bale" will fail (they encode the bug). Easy to update.
- `harvest_overlay` (oregath.shp) and `voxel_animation` play during `state == Harvest`. After the fix, Harvest state lasts ~18-36 ticks per cell instead of ~198 ticks; visuals play for the correct shorter duration. Parity, not regression.

## Chosen Approach

**Approach B — new `extract_bales_max` helper alongside existing `extract_bale`.**

- One new free function in `miner_system.rs` that performs the full bulk extraction (compute N, decrement `node.remaining` once, push N bales, update overlay once or clear it).
- `handle_harvest` swaps its `extract_bale(...)` call for `extract_bales_max(...)` with `empty_capacity = capacity_bales - cargo.len()`.
- `extract_bale` keeps its current single-bale behavior for `slave_miner.rs`.

Why over A (inline loop): atomic single-update model matches gamemd's `Reduce_Tiberium` + `AddAmount` pattern, avoids N BTreeMap lookups and N overlay writes per cell.

Why over C (modify `extract_bale` signature): would force `slave_miner.rs` to pass `max_count=1` at every call to preserve its semantics, leaking harvester intent into the slave call site.

## Tiny-Detail Ledger

These details are the constraint set for `/write-plan` and implementation. Each cites its source; the design above explains where each lives.

### Harvest extraction (finding #1)

| # | Detail | Source | Where in design |
|---|---|---|---|
| 1 | Per-call bale count = `min(empty_capacity_bales, cell_density_levels)` | `[GHIDRA 0x73D450]` `Math__ftol(Storage - currentAmount)` → `Reduce_Tiberium` clamp | `extract_bales_max`: `n = empty_capacity.min(density_levels)` |
| 2 | All N bales added in one storage update | `[GHIDRA 0x73D450]` single `AddAmount(N, type)` | `extract_bales_max`: one `Vec::collect()` then `cargo.extend(bales)` |
| 3 | Cadence between calls = 18 frames (9 × HarvesterLoadRate=2) | `[GHIDRA 0x73E5E0]` case 1 wait + post-success timer reload | `handle_harvest` keeps `harvest_tick_interval = 18` |
| 4 | Stay in Harvest after success, wait 18 frames again, next call returns 0 | same as #3 | `handle_harvest` returns to wait branch when cell still has resources (post-fix this won't fire — drain is atomic — but the branch is preserved defensively) |
| 5 | Empty-cell branch → TiberiumShortScan (radius=6) continuation | `[GHIDRA 0x73E5E0]` case 1 cVar1==0 branch | `handle_harvest` existing `search_local_ore` with `local_continuation_radius` |
| 6 | Full-storage branch → state 2 (return) | same as #5 | `handle_harvest` existing `is_full() → begin_return` |
| 7 | Full drain → remove resource node + clear overlay | `[GHIDRA 0x480A80]` `Reduce_Tiberium` clears `OverlayTypeIndex` | `extract_bales_max`: `if remaining_after == 0 → resource_nodes.remove + clear_overlay` |
| 8 | Partial drain → overlay frame updates to new density | same as #7 | `extract_bales_max`: `frame = (remaining_after/base - 1).min(11)` |
| 9 | Cargo type homogeneity per call (one cell = one type) | structural | `extract_bales_max` reads `resource_type` once before loop |

### Dock first-bale timing (finding #2)

| # | Detail | Source | Where in design |
|---|---|---|---|
| 10 | First dock bale waits `ceil(14.4) = 15` frames after dock-link | `[doc: REFINERY_DOCK_ANIM_SLOTS §9.1]` gate `14.4 ≤ field_0x3E`, counter init = 0 | `phase_linked` sets `unload_timer = unload_tick_interval` (= 144 tenths) |
| 11 | Per-bale interval after first = 14.4 frames | same as #10 | `phase_unloading` existing `saturating_add(unload_tick_interval)` |
| 12 | Tenths-of-tick precision for the 14.4 fraction | `[ini: HarvesterDumpRate=0.016]` × 900 | `unload_timer` already stored in tenths; decrement by 10/tick |

### Determinism

| # | Detail | Source | Where in design |
|---|---|---|---|
| 13 | No random or float-compare in either path | structural | both functions are pure-by-input apart from `&mut sim` mutation |
| 14 | Cargo Vec push order deterministic (same type, same value, count derived) | structural | bales built via `(0..n).map(...).collect()` |

## Design

### Components

**New `extract_bales_max`** in [src/sim/miner/miner_system.rs](src/sim/miner/miner_system.rs), placed adjacent to existing `extract_bale`:

```rust
/// Drain as many bales as fit within `empty_capacity_bales` from `cell`.
///
/// Mirrors gamemd's `UnitClass::Harvest_Ore_Tick` (0x73D450):
///   amount    = ftol(Storage - current_load)        // bales requested
///   extracted = CellClass::Reduce_Tiberium(amount)  // clamped to density
///   StorageClass::AddAmount(extracted, type)        // one storage update
///
/// Returns the bales extracted (length 0..=min(empty_capacity, cell_density)).
/// Updates `resource_nodes[cell].remaining` and the overlay grid in one
/// atomic mutation pass. Returns an empty Vec when the cell is missing,
/// has remaining==0, or `empty_capacity==0`.
pub(crate) fn extract_bales_max(
    sim: &mut Simulation,
    cell: (u16, u16),
    config: &MinerConfig,
    empty_capacity: u16,
) -> Vec<CargoBale>
```

**Modified `handle_harvest`** in the same file:

The new flow:

```rust
fn handle_harvest(...) {
    if snap.miner.harvest_timer > 0 {
        snap.miner.harvest_timer -= 1;
        return;
    }

    let cell = (snap.rx, snap.ry);
    let empty = snap.miner.capacity_bales.saturating_sub(snap.miner.cargo.len() as u16);
    let bales = extract_bales_max(sim, cell, config, empty);

    if !bales.is_empty() {
        snap.miner.cargo.extend(bales);
        snap.miner.last_harvest_cell = Some(cell);

        if snap.miner.is_full() {
            begin_return(sim, rules, config, path_grid, snap);
            return;
        }
        // Cell was drained but miner not full. Reset timer and let
        // next tick's harvest attempt fall through to short-scan
        // (cell now empty). This matches gamemd's pattern: post-success
        // step counter reset → next call returns 0 → triggers scan.
        snap.miner.harvest_timer = config.harvest_tick_interval;
        return;
    }

    // No bales extracted (cell empty). Run TiberiumShortScan continuation.
    if snap.miner.is_full() {
        begin_return(sim, rules, config, path_grid, snap);
        return;
    }

    let continuation_target = { /* unchanged search_local_ore call */ };
    if let Some(next_cell) = continuation_target {
        snap.miner.target_ore_cell = Some(next_cell);
        snap.miner.state = MinerState::MoveToOre;
        return;
    }

    begin_return(sim, rules, config, path_grid, snap);
}
```

**Modified `phase_linked`** in [src/sim/miner/miner_dock_sequence.rs](src/sim/miner/miner_dock_sequence.rs):

```rust
// Init unload_timer to one full interval — first bale fires after
// ~14.4 frames, matching gamemd's per-bale gate (HarvesterDumpRate ×
// 900 ≤ field_0x3E; counter starts at 0 on dock-link).
snap.miner.unload_timer = config.unload_tick_interval as i16;
```

(The `phase_linked` signature needs `config: &MinerConfig` threaded through. It currently doesn't take it — small plumbing change in `handle_dock_sequence` dispatch.)

### Interfaces / Contracts

- `extract_bales_max`: returns `Vec<CargoBale>`. Empty Vec is the "no-op" signal (caller treats as cell-empty). Bales in the Vec are homogeneous in type.
- `handle_harvest`'s post-extraction branching is preserved (full-storage → return; cell-empty → scan-or-return).
- `phase_linked` now takes a `&MinerConfig` parameter; dispatch in `handle_dock_sequence` passes it through.

### Data Flow

```
tick_miners:
  for each MinerSnapshot:
    handle_harvest:
      harvest_timer > 0?  yes → tick & return                  ← unchanged
                          no  → continue

      empty = capacity - cargo.len
      bales = extract_bales_max(sim, cell, config, empty)
        ├ read node = resource_nodes.get(&cell)
        ├ if missing or remaining == 0: return vec![]
        ├ base = 120 (Ore) | 180 (Gem)
        ├ density_levels = node.remaining / base
        ├ n = empty.min(density_levels)
        ├ if n == 0: return vec![]
        ├ build N bales of node.resource_type
        ├ remaining_after = node.remaining - n*base
        ├ if remaining_after == 0:
        │    resource_nodes.remove(&cell)
        │    overlay_grid.clear_overlay(cell)
        ├ else:
        │    node.remaining = remaining_after
        │    overlay_grid.set_overlay_data(cell, frame)
        └ return bales

      if bales non-empty:
        cargo.extend(bales)
        last_harvest_cell = Some(cell)
        if is_full: begin_return; return
        harvest_timer = harvest_tick_interval
        return

      if is_full: begin_return; return
      next = search_local_ore(...)
      if next: target_ore_cell = next; state = MoveToOre; return
      begin_return; return
```

### Error Handling

No new error paths. Both `extract_bales_max` and the `unload_timer` init use only saturating arithmetic on `u16`/`i16`. No unwraps on data we don't own.

### Testing Strategy

**Unit tests** for `extract_bales_max` (in `miner_tests.rs`):

| Test | Setup | Assertion |
|---|---|---|
| `extract_max_empty_cell` | no node at cell | returns `vec![]`, overlay unchanged |
| `extract_max_full_drain_ore` | ore density=11, empty_capacity=40 | returns 11 ore-type bales, node removed, overlay cleared |
| `extract_max_partial_capacity` | ore density=11, empty_capacity=3 | returns 3 bales, node.remaining = 8×120, overlay frame=7 |
| `extract_max_partial_density` | ore density=5, empty_capacity=40 | returns 5 bales, node removed (exact match → full drain) |
| `extract_max_gem_cell` | gem density=4, empty_capacity=40 | returns 4 gem-type bales (value=50 each), node removed |
| `extract_max_zero_capacity` | ore density=11, empty_capacity=0 | returns `vec![]`, node untouched |
| `extract_max_zero_remaining` | node present but remaining=0 | returns `vec![]`, node optionally removed (covered by existing cleanup) |

**Integration tests** in `miner_tests.rs`:

| Test | Assertion |
|---|---|
| `harvester_drains_full_cell_in_one_extraction_tick` | After `harvest_tick_interval` ticks on a density-11 cell, cargo contains 11 bales (was 1 in bug) |
| `harvester_caps_extraction_at_remaining_capacity` | Miner with 38/40 bales on density-11 cell → cargo lands at 40, cell drops to density-9 |
| `harvester_re_scans_on_drained_cell_when_partial` | Miner with 11/40 after draining cell → next tick triggers MoveToOre via TiberiumShortScan |

Rename existing tests that encode the buggy cadence (e.g., anything asserting "one bale per N ticks") to reflect new "one extraction call per N ticks" semantics.

**Dock first-bale timing test** in `miner_tests.rs`:

| Test | Assertion |
|---|---|
| `dock_first_bale_waits_one_unload_interval` | At Linked→Unloading tick T, cargo length unchanged through T..T+14, drops by 1 on T+15 |
| `dock_subsequent_bales_fire_every_14_or_15_ticks` | Verifies the existing fractional-tick cadence (10-tenth decrement) is unchanged for bales 2..N |

**Replay determinism:** existing replay-determinism harness will catch any nondeterminism introduced by the change. The replay-recorded reference cargo trajectories need regenerating.

## Architectural Decisions

**Patterns followed:**
- Free-function helper alongside `extract_bale`, `search_local_ore`, `player_has_purifier` — established miner-module pattern.
- Two-phase snapshot/apply in `tick_miners` preserved; `extract_bales_max` mutates `sim` directly (same as `extract_bale` does today).
- Cargo as `Vec<CargoBale>` preserved — gamemd's internal `StorageClass` is a float array but observable output is bale count and type, which `Vec<CargoBale>` represents exactly.
- INI-driven constants only — no new hardcoded magic numbers.

**Patterns deviated:** None.

**Tech debt:**
- `extract_bale` (single-bale, for slave miner) and `extract_bales_max` (multi-bale, for War/Chrono) share base-value / overlay-update logic. A private `compute_extraction_amounts` helper could share the math, but the two callers have meaningfully different control flow and value derivation (slaves walk one cell per scoop; harvesters drain in one shot). Duplication is clearer than abstraction here. Acceptable.
- `phase_linked` signature gains a `&MinerConfig` param. Minor plumbing; matches the rest of the phase handlers' style.

## Alternatives Considered

- **Approach A (inline loop):** Functionally identical but performs N `BTreeMap` lookups and N `overlay_grid.set_overlay_data` calls per cell. Rejected for hot-path waste and weaker mapping to gamemd's atomic `Reduce_Tiberium` + `AddAmount` semantics.
- **Approach C (modify `extract_bale` signature):** Would force `slave_miner.rs:253` to pass `max_count=1` at every call site, leaking harvester intent into slave-system code. Rejected.
- **Aggregate cargo model (refactor `cargo: Vec<CargoBale>` to `(count: u16, type: ResourceType)`):** Considered but rejected — pure internal refactor with no observable-output benefit. Per CLAUDE.md the spec is output, not internals; the existing `Vec` model already produces the right credits and pip counts.
