# CABHUT C4 Bridge Collapse Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Make C4 on `BridgeRepairHut=yes` buildings collapse the connected bridge like gamemd.exe, including overlay-first discovery, ramp/flag fallback, and C4 marker cleanup.

**Architecture:** This is a sim-only change. It keeps the C4 order/timer flow in `world_orders.rs`, replaces the hut-specific span-only bridge dispatch in `bridge_orchestrator.rs`, and reuses the existing overlay-direct destroy walkers in `bridge_state/walker.rs`. The new CABHUT path remains separate from normal combat bridge damage but converges at `StateOutcome` cascade aggregation.

**Design Doc:** `docs/plans/2026-05-18-cabhut-c4-bridge-collapse-design.md`

---

## Grounding Summary

`BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §18A is the primary behavior spec. It verifies that C4 on CABHUT is valid in standard YR, `Immune=yes` is not the C4 action gate, `BuildingClass::Update @ 0x0043FB20` branches on `BridgeRepairHut`, dispatches `DestroyBridge_*_MapInit`, then clears the C4 marker and attacker pointer.

Live Ghidra verification in this planning pass confirmed the same: `BuildingClass__Update @ 0x0043FB20` calls low/high hut destruction based on the 5x5 low evidence scan and clears `field_0x6DF` and `field_0x540` afterward. `MapClass__DestroyBridge_Low_MapInit @ 0x00574C20` and `MapClass__DestroyBridge_High_MapInit @ 0x00574000` scan the hut-centered 5x5 for first matching overlay, else search bridge/ramp flags. `DestroyBridgeFromCell_Low @ 0x00574780` and `DestroyBridgeFromCell_High @ 0x005749C0` canonicalize the start cell before collapse.

The repo already has the right low-level pattern: `BridgeRuntimeState::destroy_bridge_low` and `destroy_bridge_high` in `src/sim/bridge_state/walker.rs` drive overlay-direct bridge destruction and return `StateOutcome`. `src/sim/world/bridge_orchestrator.rs` already aggregates `StateOutcome::Collapsed` into occupant kills, deck drop-in, debris, adjacent refresh, trigger hook, and zone refresh.

INI grounding: `[CombatDamage] C4Delay=.03`, `[CombatDamage] C4Warhead=Super`, `[GHOST] C4=yes`, `[TANY] C4=yes`, and `[CABHUT] BridgeRepairHut=yes` are present in YR `rulesmd.ini`. `CanC4` defaults true for CABHUT; `InvisibleInGame` defaults false.

Still unknown: exact gamemd `CellClass+0x140` flag reconstruction in Rust terrain. This plan avoids a persisted flag clone and uses local bridge evidence derived from `ResolvedTerrainCell` and `BridgeRuntimeCell`. If `/review-plan` rejects that approximation, replace the fallback helper with an explicit bridge-flag model before implementation.

---

## Key Technical Decisions

- Reuse existing overlay-direct destroy walkers inside a CABHUT-specific four-step sweep runner instead of calling a single walker coordinate once. **Confidence:** high
  - **Source:** repo pattern `src/sim/bridge_state/walker.rs`; Ghidra `DestroyBridgeFromCell_*` funnels to `CollapseBridge_*_*`, which advances along the bridge axis for up to four cells.

- Replace the span-only hut scan with overlay-first low/high discovery. **Confidence:** high
  - **Source:** Ghidra `0x00574C20`, `0x00574000`; design doc Chosen Approach.

- Implement ramp/flag fallback as direction-preserving local bridge evidence from `ResolvedTerrainCell` and `BridgeRuntimeCell`, not as persisted gamemd `CellClass+0x140` bits. **Confidence:** medium
  - **Source:** design doc Architectural Decisions; current repo terrain model. Flag for `/review-plan`.

- Detect BridgeRepairHut before the Rust invulnerability check, dispatch the hut bridge path regardless of runtime Iron Curtain state, and clear `pending_c4_detonation` after the hut dispatch returns, even if no bridge changed. **Confidence:** high
  - **Source:** Ghidra `BuildingClass::Update @ 0x0043FB20`, report §18A.3.

- Keep normal combat bridge damage on the existing `apply_bridge_damage_events` path. **Confidence:** high
  - **Source:** repo pattern `bridge_orchestrator.rs`; design doc Impact Analysis.

---

## Open Questions

### Resolved During Planning

- Is `Immune=yes` blocking CABHUT C4? No. `InfantryClass::What_Action_OnObject @ 0x0051E3B0` permits C4 action when `C4=yes`, target building, `CanC4=true`, and `InvisibleInGame=false`; `Immune=yes` is not read in that action gate.
- Is the stale Rust pending C4 state intentional? No. Ghidra shows CABHUT dispatch clears the marker and attacker pointer after `DestroyBridge_*_MapInit`.
- Do we need a new bridge collapse subsystem? No. Existing `destroy_bridge_low/high` and cascade aggregation already match the needed architecture.

### Deferred to Implementation

- Exact fallback reachability from bridgehead/ramp evidence depends on how small test fixtures encode `ResolvedTerrainCell` bridgehead and deck facts. The implementation must prove the helper can follow a connected direction from bridgehead/ramp evidence to a real overlay/body entry with a focused unit test. Do not use arbitrary square/radius overlay search for fallback.
- Exact current ignored/dirty worktree state is not part of this plan. Implementation must avoid reverting unrelated user/session edits in touched files.

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/bridge_state/walker.rs` | Expose crate-visible overlay predicate helpers for low/high destroy overlays, wrapping existing ranges. |
| Modify | `src/sim/world/bridge_orchestrator.rs` | Replace span-only CABHUT dispatch with overlay-first/fallback dispatch and reuse cascade aggregation. |
| Modify | `src/sim/world/world_orders.rs` | Clear CABHUT `pending_c4_detonation` after hut dispatch; keep non-hut C4 behavior unchanged. |
| Modify | `src/sim/world/world_orders_bridge_repair_tests.rs` | Add focused C4-on-CABHUT tests for low, high, fallback, and marker cleanup. |

No new source file is required. If `bridge_orchestrator.rs` grows past the local style threshold during implementation, split only the hut dispatch helpers into a sibling module under `src/sim/world/bridge_orchestrator/` after checking the current module layout.

---

## Interface Changes

- Add crate-visible helper predicates on `BridgeRuntimeState`:
  - `pub(crate) fn is_low_destroy_overlay(overlay: u8) -> bool`
  - `pub(crate) fn is_high_destroy_overlay(overlay: u8) -> bool`
- Add crate-visible axis helpers on `BridgeRuntimeState`:
  - `pub(crate) fn low_destroy_overlay_axis(overlay: u8) -> Option<Axis>`
  - `pub(crate) fn high_destroy_overlay_axis(overlay: u8) -> Option<Axis>`
- Keep `dispatch_bridge_collapse_from_hut(sim, rules, hut_center) -> bool` signature unchanged.
- No command enum, rules schema, render API, audio API, or save schema changes.

Dependencies:

- `bridge_orchestrator.rs` depends on the helper predicates.
- Tests use the same predicates only through public behavior, not by asserting helper internals unless helper-specific tests are added in `walker.rs`.

---

## Sim Checklist

- [ ] All math uses integer cell coordinates; no f32/f64 in game logic.
- [ ] No new persistent sim state; deterministic state hash unchanged.
- [ ] No dependencies on render/ui/sidebar/audio/net.
- [ ] Tick ordering unchanged: C4 still detonates inside `tick_c4_plants`; only the hut branch behavior changes.
- [ ] BTreeMap iteration order remains relevant only in existing C4 attacker/building collection; new helper scan order is explicit arrays/vectors.

---

## Risk Areas

- Fallback evidence might collapse a bridge gamemd would not reach if direction continuity is not enforced.
- Fallback evidence might still fail to find valid bridgehead/ramp-only CABHUT layouts if it only follows span/deck cells.
- C4 pending cleanup must run for CABHUT but must not make Iron Curtain non-hut C4 stop retrying.
- Reusing direct walkers changes cases that previously no-oped; tests must confirm existing normal bridge damage behavior remains stable.
- Existing files are already modified in the worktree. Implementation must read current file contents before editing and preserve unrelated edits.

---

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 1 | Low/high overlay predicate ranges | Wrong range sends CABHUT to the wrong bridge family or no-ops | Unit tests for `0x4A`, `0x65`, `0xCD`, `0xE8`, and boundary misses |
| 2 | 5x5 scan order and low-first decision | Determines which nearby bridge segment collapses when several candidates exist | Unit test with both low/high evidence and expected low result |
| 3 | Ramp/bridgehead fallback | Fixes the reported visible bug where C4 plants but bridge stays intact | Unit test with spanless bridgehead/ramp evidence and direction-connected overlay; wrong-nearby-bridge regression |
| 4 | Destroy attempt loop cap and axis advance | Multi-stage overlay destruction must reach collapse without infinite loops and must sweep more than the original coordinate | Unit test where first attempt is `Absorbed` and later attempt collapses; axis-advance regression |
| 5 | CABHUT pending C4 cleanup | Prevents repeated detonation/no-op and allows later C4 orders like gamemd | Unit tests for pending cleared after changed and unchanged hut dispatch |
| 6 | Hut survives C4 | CABHUT should not lose HP from C4 branch | Existing and new C4-on-CABHUT tests assert HP unchanged |
| 7 | Existing cascade side effects | Ground kill, drop-in, debris, rim, trigger hook, and zones remain observable bridge-collapse behavior | Existing bridge collapse tests plus targeted C4 hut collapse test |

---

## Tasks

### Task 1: Expose Destroy-Overlay Predicate Helpers

**Why:** The hut dispatcher needs low/high overlay classification and bridge-axis classification without duplicating numeric ranges in `bridge_orchestrator.rs`.

**Files:**
- Modify: `src/sim/bridge_state/walker.rs`

**Pattern:** Follows existing associated helper predicates such as `is_ns_walker_overlay_low` and `is_ew_walker_overlay_high`.

**Step 1: Add crate-visible wrappers inside `impl BridgeRuntimeState`**

```rust
pub(crate) fn is_low_destroy_overlay(overlay: u8) -> bool {
    (0x4A..=0x65).contains(&overlay)
}

pub(crate) fn is_high_destroy_overlay(overlay: u8) -> bool {
    (0xCD..=0xE8).contains(&overlay)
}
```

**Step 2: Add crate-visible axis helpers**

Use the existing private subrange predicates so the orchestrator can reproduce the `CollapseBridge_*_*` axis sweep without retyping overlay tables:

```rust
pub(crate) fn low_destroy_overlay_axis(overlay: u8) -> Option<crate::sim::bridge_state::Axis> {
    if Self::is_ns_walker_overlay_low(overlay) {
        Some(crate::sim::bridge_state::Axis::NS)
    } else if Self::is_ew_walker_overlay_low(overlay) {
        Some(crate::sim::bridge_state::Axis::EW)
    } else {
        None
    }
}

pub(crate) fn high_destroy_overlay_axis(overlay: u8) -> Option<crate::sim::bridge_state::Axis> {
    if Self::is_ns_walker_overlay_high(overlay) {
        Some(crate::sim::bridge_state::Axis::NS)
    } else if Self::is_ew_walker_overlay_high(overlay) {
        Some(crate::sim::bridge_state::Axis::EW)
    } else {
        None
    }
}
```

**Step 3: Replace internal direct range checks in direct-destroy entries**

Use `is_low_destroy_overlay` in `destroy_bridge_low` and low scan/fallback helpers. Use `is_high_destroy_overlay` in `destroy_bridge_high` and high scan/fallback helpers. Do not rewrite repair predicates such as `is_low_repair_overlay`.

**Step 4: Add unit tests in `walker.rs`**

```rust
#[test]
fn destroy_overlay_predicates_match_gamemd_ranges() {
    assert!(!BridgeRuntimeState::is_low_destroy_overlay(0x49));
    assert!(BridgeRuntimeState::is_low_destroy_overlay(0x4A));
    assert!(BridgeRuntimeState::is_low_destroy_overlay(0x65));
    assert!(!BridgeRuntimeState::is_low_destroy_overlay(0x66));

    assert!(!BridgeRuntimeState::is_high_destroy_overlay(0xCC));
    assert!(BridgeRuntimeState::is_high_destroy_overlay(0xCD));
    assert!(BridgeRuntimeState::is_high_destroy_overlay(0xE8));
    assert!(!BridgeRuntimeState::is_high_destroy_overlay(0xE9));
}
```

Add a second test for representative axis values: low `0x4A -> NS`, low `0x53 -> EW`, high `0xCD -> NS`, high `0xD6 -> EW`, plus one miss for each family.

**Step 5: Verify**

Run:

```powershell
cargo test destroy_overlay_predicates_match_gamemd_ranges --lib -- --nocapture
```

Expected: test passes.

**Step 6: Commit**

Commit message: `sim: expose bridge destroy overlay predicates`

---

### Task 2: Factor Hut Collapse Cascade Aggregation

**Why:** The new hut dispatcher will collect outcomes from overlay and fallback attempts, then run the same cascade currently embedded in `dispatch_bridge_collapse_from_hut`.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs`

**Pattern:** Follows current aggregation in `dispatch_bridge_collapse_from_hut` and `apply_bridge_damage_events`.

**Step 1: Add helper signature near `dispatch_bridge_collapse_from_hut`**

```rust
fn run_bridge_collapse_cascade_from_outcomes(
    sim: &mut Simulation,
    rules: &RuleSet,
    outcomes: &[StateOutcome],
) -> bool
```

**Step 2: Move existing aggregation body into the helper**

Move the current code that builds `destroyed_set`, `blow_up_cells`, `rim_cells`, and `any_zones_dirty`, then calls:

- `kill_ground_occupants_at`
- `drop_in_bridge_deck_entities`
- `spawn_bridge_debris`
- `update_adjacent_bridges`
- `notify_bridge_span_collapse`
- `refresh_bridge_zones_if_dirty`

Return `!destroyed_set.is_empty()`.

**Step 3: Update existing `dispatch_bridge_collapse_from_hut` temporarily**

After collecting outcomes in the current span-only logic, replace the inlined cascade with:

```rust
run_bridge_collapse_cascade_from_outcomes(sim, rules, &outcomes)
```

This task should be behavior-preserving.

**Step 4: Verify**

Run:

```powershell
cargo test c4_on_cabhut_collapses_bridge_and_hut_survives --lib -- --nocapture
```

Expected: existing seeded test still passes.

**Step 5: Commit**

Commit message: `sim: factor bridge collapse cascade aggregation`

---

### Task 3: Add Hut Bridge Evidence Helpers

**Why:** CABHUT fallback must discover bridge/ramp evidence without depending on `anchor_span_id`.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs`

**Pattern:** Local helper functions in the orchestrator that keep bridge-dispatch details private to `bridge_orchestrator.rs`, matching the placement of `update_adjacent_bridges` and `refresh_bridge_zones_if_dirty`.

**Step 1: Define local enum**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HutBridgeFamily {
    Low,
    High,
}
```

**Step 2: Add overlay read helper**

```rust
fn bridge_overlay_at(sim: &Simulation, rx: u16, ry: u16) -> Option<u8> {
    sim.bridge_state
        .as_ref()
        .and_then(|bs| bs.cell(rx, ry))
        .map(|cell| cell.overlay_byte)
}
```

**Step 3: Add low-evidence helper**

```rust
fn is_low_hut_scan_evidence(sim: &Simulation, rx: u16, ry: u16) -> bool {
    let low_overlay = bridge_overlay_at(sim, rx, ry)
        .map(crate::sim::bridge_state::BridgeRuntimeState::is_low_destroy_overlay)
        .unwrap_or(false);
    let low_tile = sim
        .resolved_terrain
        .as_ref()
        .and_then(|terrain| terrain.cell(rx, ry))
        .map(|cell| cell.is_wood_bridge_repair_tile)
        .unwrap_or(false);
    low_overlay || low_tile
}
```

If `is_wood_bridge_repair_tile` is too narrow for high/low selection in current terrain, use the existing bridge facts fields already read by `BridgeRuntimeState::from_resolved_terrain` to identify low bridge tile evidence. Do not introduce a hardcoded tile id table in this task.

**Step 4: Add generic bridge evidence helper**

```rust
fn has_hut_fallback_bridge_evidence(sim: &Simulation, rx: u16, ry: u16) -> bool {
    let runtime = sim.bridge_state.as_ref().and_then(|bs| bs.cell(rx, ry));
    let terrain = sim.resolved_terrain.as_ref().and_then(|grid| grid.cell(rx, ry));

    runtime.is_some_and(|cell| {
        cell.deck_present
            || cell.anchor_span_id.is_some()
            || matches!(cell.role, crate::sim::bridge_state::BridgeCellRole::Bridgehead)
    }) || terrain.is_some_and(|cell| {
        cell.bridge_walkable
            || cell.has_bridge_deck
            || cell.bridge_layer.is_some()
            || cell.bridge_facts.overlay_id.is_some()
    })
}
```

Adjust exact field access to current struct visibility. If a field is private or missing, use existing public accessors or add a narrow accessor in the owning module.

**Step 5: Add helper tests through behavior**

Add tests in `bridge_orchestrator.rs` test module if one exists; otherwise add them in `world_orders_bridge_repair_tests.rs` by exercising the public hut dispatch. Do not expose `HutBridgeFamily` outside the module just for tests.

**Step 6: Verify**

Run:

```powershell
cargo test hut --lib -- --nocapture
```

Expected: existing hut-related tests pass.

**Step 7: Commit**

Commit message: `sim: add hut bridge evidence helpers`

---

### Task 4: Implement Overlay-First Hut Entry Selection

**Why:** Gamemd first scans the hut-centered 5x5 for low/high overlay evidence before any fallback. This directly fixes the span-only no-op for maps where the scan sees an overlay but not a span-tagged cell.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs`

**Pattern:** Mirrors `repair_bridge_from_engineer_scan` in `src/sim/bridge_state/walker.rs`.

**Step 1: Add family selection helper**

```rust
fn choose_hut_bridge_family(sim: &Simulation, scan: &[(u16, u16)]) -> HutBridgeFamily {
    if scan
        .iter()
        .any(|&(rx, ry)| is_low_hut_scan_evidence(sim, rx, ry))
    {
        HutBridgeFamily::Low
    } else {
        HutBridgeFamily::High
    }
}
```

**Step 2: Add first overlay entry helper**

```rust
fn find_hut_overlay_entry(
    sim: &Simulation,
    scan: &[(u16, u16)],
    family: HutBridgeFamily,
) -> Option<(u16, u16)> {
    scan.iter().copied().find(|&(rx, ry)| {
        let Some(overlay) = bridge_overlay_at(sim, rx, ry) else {
            return false;
        };
        match family {
            HutBridgeFamily::Low => {
                crate::sim::bridge_state::BridgeRuntimeState::is_low_destroy_overlay(overlay)
            }
            HutBridgeFamily::High => {
                crate::sim::bridge_state::BridgeRuntimeState::is_high_destroy_overlay(overlay)
            }
        }
    })
}
```

**Step 3: Add temporary single-step destroy entry runner**

This helper only isolates overlay-first entry selection. Task 6 replaces it with the parity-critical axis-aware sweep runner before the full CABHUT path is considered complete.

```rust
fn run_hut_destroy_entry_once(
    bridge_state: &mut crate::sim::bridge_state::BridgeRuntimeState,
    terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid,
    family: HutBridgeFamily,
    rx: u16,
    ry: u16,
) -> StateOutcome {
    match family {
        HutBridgeFamily::Low => bridge_state.destroy_bridge_low(rx, ry, terrain),
        HutBridgeFamily::High => bridge_state.destroy_bridge_high(rx, ry, terrain),
    }
}
```

**Step 4: Replace the `anchor_span_id` filter in `dispatch_bridge_collapse_from_hut`**

Within the scoped mutable bridge-state borrow:

1. Compute `family`.
2. Try `find_hut_overlay_entry`.
3. If found, call `run_hut_destroy_entry_once`.
4. Push non-`NoChange` outcomes.

Do not implement fallback in this task; return existing cascade result for overlay cases and `false` when no overlay exists.

**Step 5: Add overlay-first low test**

Use an existing seeded C4/CABHUT fixture. Set a terminal-ready low overlay cell such as `0x50`, `0x51`, or `0x52` within the 5x5, without relying on `anchor_span_id` for entry selection. This isolates overlay-first entry selection; the healthy-overlay multi-step sweep is covered in Task 6. Assert:

- `dispatch_bridge_collapse_from_hut` returns true or `tick_c4_plants` reports `bridge_state_changed`.
- At least one low bridge cell reaches `DamageState::Destroyed`.
- CABHUT health is unchanged.

**Step 6: Add overlay-first high test**

Create a terminal-ready high overlay cell such as `0xD3..=0xD5` or `0xDC..=0xDE` within the 5x5. Assert that a high terminal cap such as `0xE7` or `0xE8` appears after dispatch, depending on fixture orientation.

**Step 7: Verify**

Run:

```powershell
cargo test c4_on_cabhut --lib -- --nocapture
cargo test bridge --lib -- --nocapture
```

Expected: new low/high overlay tests pass; existing bridge tests still pass.

**Step 8: Commit**

Commit message: `sim: route cabhut c4 through overlay bridge destroy`

---

### Task 5: Implement Direction-Preserving Fallback from Bridgehead/Ramp Evidence

**Why:** Full scope requires matching gamemd's no-overlay fallback well enough that a hut-local bridgehead/ramp can still find the connected bridge and collapse it. The fallback must not search an arbitrary square for any nearby overlay; it must preserve the direction/evidence chain that led from the hut to the bridge/ramp cell.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs`

**Pattern:** Local deterministic search helper; follows gamemd's 8-direction, 3-cell evidence search, then traces from that evidence along connected bridge/ramp/deck facts until a destroy overlay in the selected family is found.

**Step 1: Define fixed direction order and seed type**

Use the same 8-direction order already used by nearby bridge code where possible. If no shared constant exists, define this local constant:

```rust
const HUT_FALLBACK_DIRS: [(i16, i16); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HutFallbackSeed {
    pos: (u16, u16),
    /// Direction from hut center to evidence. `None` means the hut center
    /// itself had evidence, so all directions must be tested with continuity.
    dir: Option<(i16, i16)>,
}
```

**Step 2: Add bounded evidence scan that preserves direction**

```rust
fn find_hut_fallback_seed(sim: &Simulation, hut_center: (u16, u16)) -> Option<HutFallbackSeed> {
    if has_hut_fallback_bridge_evidence(sim, hut_center.0, hut_center.1) {
        return Some(HutFallbackSeed { pos: hut_center, dir: None });
    }
    for &(dx, dy) in &HUT_FALLBACK_DIRS {
        for distance in 1..=3i16 {
            let rx = hut_center.0 as i32 + dx as i32 * distance as i32;
            let ry = hut_center.1 as i32 + dy as i32 * distance as i32;
            if rx < 0 || ry < 0 {
                continue;
            }
            let pos = (rx as u16, ry as u16);
            if has_hut_fallback_bridge_evidence(sim, pos.0, pos.1) {
                return Some(HutFallbackSeed { pos, dir: Some((dx, dy)) });
            }
        }
    }
    None
}
```

**Step 3: Add connected trace from seed to destroy overlay**

Do not use radius, square, or row-major overlay search. From the fallback seed, walk only along the seed direction. If the seed was the hut center, test each fixed direction in `HUT_FALLBACK_DIRS` order. Each intermediate cell must have fallback bridge evidence or a matching destroy overlay; stop that direction as soon as continuity breaks.

```rust
fn find_destroy_overlay_along_connected_fallback(
    sim: &Simulation,
    seed: HutFallbackSeed,
    family: HutBridgeFamily,
) -> Option<(u16, u16)> {
    let single_dir;
    let directions: &[(i16, i16)] = if let Some(dir) = seed.dir {
        single_dir = [dir];
        &single_dir
    } else {
        &HUT_FALLBACK_DIRS
    };

    for &(dx, dy) in directions {
        let mut pos = seed.pos;
        for _step in 0..HUT_FALLBACK_TRACE_LIMIT {
            if let Some(overlay) = bridge_overlay_at(sim, pos.0, pos.1) {
                let matches = match family {
                    HutBridgeFamily::Low => {
                        crate::sim::bridge_state::BridgeRuntimeState::is_low_destroy_overlay(overlay)
                    }
                    HutBridgeFamily::High => {
                        crate::sim::bridge_state::BridgeRuntimeState::is_high_destroy_overlay(overlay)
                    }
                };
                if matches {
                    return Some(pos);
                }
            }

            if !has_hut_fallback_bridge_evidence(sim, pos.0, pos.1) {
                break;
            }

            let next_rx = pos.0 as i32 + dx as i32;
            let next_ry = pos.1 as i32 + dy as i32;
            if next_rx < 0 || next_ry < 0 {
                break;
            }
            pos = (next_rx as u16, next_ry as u16);
        }
    }
    None
}
```

Choose `HUT_FALLBACK_TRACE_LIMIT` as a small named constant high enough for real hut-to-ramp layouts in current tests. Do not make it map-wide unless the helper also proves bridge-layer/group continuity from `ResolvedTerrainCell` or `BridgeRuntimeCell` on every step.

**Step 4: Wire fallback into `dispatch_bridge_collapse_from_hut`**

If `find_hut_overlay_entry` returns `None`, call:

1. `find_hut_fallback_seed`.
2. `find_destroy_overlay_along_connected_fallback`.
3. `run_hut_destroy_entry`.

Push the returned outcomes as in overlay-first dispatch.

**Step 5: Add connected fallback test**

Construct a CABHUT fixture where the hut 5x5 contains a bridgehead/ramp runtime or terrain cell with no `anchor_span_id`, and a connected low or high overlay cell just outside the original 5x5 but reachable by the preserved evidence direction. Assert:

- `tick_c4_plants` or `dispatch_bridge_collapse_from_hut` reports bridge state changed.
- The bridge overlay reaches the expected destroyed cap.
- The test would fail against the old span-only implementation.

**Step 6: Add wrong-nearby-bridge regression test**

Construct a hut fallback seed with a connected bridge overlay in one direction and an unrelated matching-family overlay closer in row-major square order but not connected by the seed direction/evidence chain. Assert the connected bridge collapses and the unrelated overlay does not change.

**Step 7: Add no-false-positive test**

Construct a hut with no bridge evidence and a bridge overlay outside the fallback envelope. Assert dispatch returns false and no bridge overlay changes.

**Step 8: Verify**

Run:

```powershell
cargo test hut_fallback --lib -- --nocapture
cargo test c4_on_cabhut --lib -- --nocapture
```

Expected: fallback and no-false-positive tests pass.

**Step 9: Commit**

Commit message: `sim: find cabhut bridge collapse fallback entry`

---

### Task 6: Add Axis-Aware Four-Step Hut Destroy Runner

**Why:** Ghidra shows up to three destroy attempts per step and a collapse sweep up to four cells along the bridge axis. Existing `destroy_bridge_low/high` can return `Absorbed` for intermediate overlay transitions, so the hut entry must continue attempts on the current cell, then advance to the next bridge-axis cell instead of repeatedly hitting the original coordinate.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs`

**Pattern:** Bounded deterministic loop shaped like `CollapseBridge_EW/NS_Low/High`: classify overlay axis, count overlay-band length on both sides, choose the shorter-side sweep direction, midpoint-bias the start coordinate, attempt up to three destroys per cell, then advance one cell along the selected axis for at most four cells.

**Step 1: Add small coordinate helpers**

Keep these local to `bridge_orchestrator.rs`.

```rust
fn matching_destroy_overlay(family: HutBridgeFamily, overlay: u8) -> bool {
    match family {
        HutBridgeFamily::Low => {
            crate::sim::bridge_state::BridgeRuntimeState::is_low_destroy_overlay(overlay)
        }
        HutBridgeFamily::High => {
            crate::sim::bridge_state::BridgeRuntimeState::is_high_destroy_overlay(overlay)
        }
    }
}

fn destroy_overlay_axis(family: HutBridgeFamily, overlay: u8) -> Option<crate::sim::bridge_state::Axis> {
    match family {
        HutBridgeFamily::Low => {
            crate::sim::bridge_state::BridgeRuntimeState::low_destroy_overlay_axis(overlay)
        }
        HutBridgeFamily::High => {
            crate::sim::bridge_state::BridgeRuntimeState::high_destroy_overlay_axis(overlay)
        }
    }
}

fn step_axis(pos: (u16, u16), axis: crate::sim::bridge_state::Axis, dir: i16) -> Option<(u16, u16)> {
    let (rx, ry) = pos;
    match axis {
        crate::sim::bridge_state::Axis::EW => {
            let next = rx as i32 + dir as i32;
            (next >= 0).then_some((next as u16, ry))
        }
        crate::sim::bridge_state::Axis::NS => {
            let next = ry as i32 + dir as i32;
            (next >= 0).then_some((rx, next as u16))
        }
    }
}
```

**Step 2: Add overlay-band counting and midpoint start**

This mirrors the binary's "scan both directions, choose shorter side, midpoint-bias start" behavior. Count only cells whose current overlay remains in the selected low/high destroy band.

```rust
fn count_destroy_band(
    bridge_state: &crate::sim::bridge_state::BridgeRuntimeState,
    family: HutBridgeFamily,
    axis: crate::sim::bridge_state::Axis,
    start: (u16, u16),
    dir: i16,
) -> usize {
    let mut count = 0;
    let mut cursor = start;
    while let Some(next) = step_axis(cursor, axis, dir) {
        let Some(overlay) = bridge_state.cell(next.0, next.1).map(|c| c.overlay_byte) else {
            break;
        };
        if !matching_destroy_overlay(family, overlay) {
            break;
        }
        count += 1;
        cursor = next;
    }
    count
}

fn midpoint_biased_start(
    pos: (u16, u16),
    axis: crate::sim::bridge_state::Axis,
    backward_count: usize,
    forward_count: usize,
) -> Option<(u16, u16)> {
    let delta = (backward_count as i16 - forward_count as i16) / 2;
    let dir = if delta >= 0 { -1 } else { 1 };
    let mut cursor = pos;
    for _ in 0..delta.unsigned_abs() {
        cursor = step_axis(cursor, axis, dir)?;
    }
    Some(cursor)
}
```

Use Rust integer division's truncation toward zero. That matches the C integer division behavior in the verified decompile closely enough for this observable start bias.

**Step 3: Replace `run_hut_destroy_entry_once` with axis-aware capped runner**

```rust
fn run_hut_destroy_entry(
    bridge_state: &mut crate::sim::bridge_state::BridgeRuntimeState,
    terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid,
    family: HutBridgeFamily,
    rx: u16,
    ry: u16,
) -> Vec<StateOutcome> {
    const MAX_SWEEP_STEPS: usize = 4;
    const MAX_ATTEMPTS_PER_STEP: usize = 3;

    let Some(entry_overlay) = bridge_state.cell(rx, ry).map(|c| c.overlay_byte) else {
        return Vec::new();
    };
    let Some(axis) = destroy_overlay_axis(family, entry_overlay) else {
        return Vec::new();
    };

    let backward_count = count_destroy_band(bridge_state, family, axis, (rx, ry), -1);
    let forward_count = count_destroy_band(bridge_state, family, axis, (rx, ry), 1);
    let sweep_dir = if forward_count < backward_count { -1 } else { 1 };
    let Some(mut current) = midpoint_biased_start((rx, ry), axis, backward_count, forward_count) else {
        return Vec::new();
    };

    let mut outcomes = Vec::new();
    for _step in 0..MAX_SWEEP_STEPS {
        let Some(current_overlay) = bridge_state.cell(current.0, current.1).map(|c| c.overlay_byte) else {
            break;
        };
        if !matching_destroy_overlay(family, current_overlay) {
            break;
        }

        for _attempt in 0..MAX_ATTEMPTS_PER_STEP {
            let outcome = match family {
                HutBridgeFamily::Low => bridge_state.destroy_bridge_low(current.0, current.1, terrain),
                HutBridgeFamily::High => bridge_state.destroy_bridge_high(current.0, current.1, terrain),
            };
            match outcome {
                StateOutcome::NoChange => {}
                StateOutcome::Absorbed => {
                    outcomes.push(StateOutcome::Absorbed);
                }
                other @ StateOutcome::Collapsed { .. } => {
                    outcomes.push(other);
                    break;
                }
            }
        }

        let Some(next) = step_axis(current, axis, sweep_dir) else {
            break;
        };
        current = next;
    }
    outcomes
}
```

If `StateOutcome::Absorbed` has fields or variants beyond this shape, match the actual enum exactly.

This intentionally does not return after the first `Collapsed` outcome. Gamemd advances to the next axis cell and can collapse up to four strips from the chosen start.

**Step 4: Update callers**

Append the returned vector into the hut dispatcher outcomes:

```rust
outcomes.extend(run_hut_destroy_entry(bs, terrain, family, entry.0, entry.1));
```

Use this same runner from both overlay-first and fallback entries.

**Step 5: Add capped-loop test**

Use an overlay fixture that requires at least one intermediate `Absorbed` transition before final collapse. Assert:

- Dispatch returns true.
- Destroyed cap appears.
- The loop does not exceed the cap. If needed, assert indirectly by final state and lack of repeated extra changes.

**Step 6: Add axis-advance regression test**

Use a low or high bridge fixture where the first entry coordinate can collapse but gamemd's four-step sweep should also advance to at least one adjacent body-axis coordinate. Assert:

- At least two axis-adjacent bridge strips change after the hut dispatch.
- Repeating `destroy_bridge_low/high` on the original coordinate alone would not satisfy the assertion.

**Step 7: Verify**

Run:

```powershell
cargo test c4_on_cabhut --lib -- --nocapture
cargo test bridge --lib -- --nocapture
```

Expected: tests pass.

**Step 8: Commit**

Commit message: `sim: cap cabhut bridge destroy attempts`

---

### Task 7: Dispatch CABHUT Before Invulnerability Check and Clear Pending C4 Marker

**Why:** Gamemd's timer-expiry path branches on `BridgeRepairHut` and runs the bridge destruction dispatcher without rechecking runtime Iron Curtain state, then clears `BuildingClass+0x6DF` and the attacker pointer. Rust currently checks invulnerability before detecting BridgeRepairHut and leaves `pending_c4_detonation` on the surviving hut.

**Files:**
- Modify: `src/sim/world/world_orders.rs`

**Pattern:** State cleanup belongs where `pending_c4_detonation` is owned, not in bridge orchestration.

**Step 1: Move BridgeRepairHut detection before invulnerability**

In `apply_c4_damage_to_building`, compute `target_bridge_hut` before the `is_invulnerable` check. If `target_bridge_hut` is true, run the hut bridge dispatch branch before the normal invulnerability early return.

Keep the normal non-hut invulnerability behavior exactly as-is: non-hut C4 damage still returns default and leaves pending state when the target is currently invulnerable.

**Step 2: Extend `C4DamageOutcome` if needed**

If `apply_c4_damage_to_building` cannot currently tell the caller that the target was a hut, add:

```rust
pub(crate) struct C4DamageOutcome {
    pub killed_building: bool,
    pub bridge_state_changed: bool,
    pub consumed_pending_marker: bool,
}
```

If the struct is private, keep visibility unchanged. Default should set `consumed_pending_marker=false`.

**Step 3: Return marker-consumed for BridgeRepairHut branch**

In the hut branch of `apply_c4_damage_to_building`, return:

```rust
C4DamageOutcome {
    killed_building: false,
    bridge_state_changed,
    consumed_pending_marker: true,
}
```

**Step 4: Clear pending after applying C4 damage**

In `tick_c4_plants`, after `apply_c4_damage_to_building` returns:

```rust
if outcome.consumed_pending_marker {
    if let Some(building) = self.entities.get_mut(building_id) {
        building.pending_c4_detonation = None;
    }
}
```

Keep non-hut Iron Curtain behavior unchanged: if damage is nullified and `consumed_pending_marker=false`, pending remains and retries next tick as before.

**Step 5: Add cleanup test**

Add a test where CABHUT C4 timer expires and dispatch returns false because no bridge evidence exists. Assert:

- CABHUT remains alive.
- `pending_c4_detonation` is `None`.
- A subsequent C4 plant can claim the hut again.

**Step 6: Add CABHUT invulnerability-ordering test**

Add a test where a CABHUT has active runtime invulnerability when the C4 timer expires. Assert:

- The hut bridge dispatch still runs.
- `pending_c4_detonation` is cleared.
- CABHUT HP remains unchanged.

This covers the verified gamemd `BuildingClass::Update` behavior and prevents Rust's pre-existing invulnerability early return from skipping CABHUT bridge collapse.

**Step 7: Add non-hut regression test**

Use or add a non-hut C4 test. Assert a normal C4 target still dies and pending disappears via despawn path. If the target is Iron Curtained, assert pending remains.

**Step 8: Verify**

Run:

```powershell
cargo test c4 --lib -- --nocapture
```

Expected: CABHUT cleanup and non-hut regressions pass.

**Step 9: Commit**

Commit message: `sim: clear cabhut c4 marker after bridge dispatch`

---

### Task 8: Add Focused End-to-End CABHUT C4 Tests

**Why:** The bug is player-visible only through the full C4 timer path: order accepted, timer expires, hut survives, bridge changes, and pending marker clears.

**Files:**
- Modify: `src/sim/world/world_orders_bridge_repair_tests.rs`

**Pattern:** Follows existing C4-on-CABHUT and engineer bridge repair fixtures in the same file.

**Step 1: Add helper fixture for low overlay C4**

Build on existing `bridge_repair_test_rules()` and map/bridge-state helpers. The helper must create:

- one C4 infantry owned by the player;
- one enemy CABHUT with `BridgeRepairHut=yes`;
- a low overlay bridge cell in the hut 5x5 with enough neighboring overlay cells for the direct walker to collapse;
- resolved terrain and bridge runtime state.

**Step 2: Add helper fixture for high overlay C4**

Same as Step 1, but use high overlay bytes and a high bridge runtime cell.

**Step 3: Add helper fixture for spanless fallback**

Create a bridgehead/ramp evidence cell near the hut with `anchor_span_id=None`, plus a nearby real overlay entry in the chosen family. The test must assert this scenario failed under the old condition by including no span-tagged cell in the hut-centered 5x5.

**Step 4: Add tests**

Implement these test functions with explicit assertions:

```rust
#[test]
fn c4_on_cabhut_low_overlay_collapses_bridge_and_clears_pending() {
    // Arrange: low bridge overlay fixture inside hut 5x5.
    // Act: advance to C4 detonation through tick_c4_plants.
    // Assert: bridge_state_changed, hut HP unchanged, pending_c4_detonation is None.
}

#[test]
fn c4_on_cabhut_high_overlay_uses_high_destroy_path() {
    // Arrange: high bridge overlay fixture inside hut 5x5.
    // Act: advance to C4 detonation through tick_c4_plants.
    // Assert: a high terminal cap overlay is written and low terminal caps are absent.
}

#[test]
fn c4_on_cabhut_spanless_bridgehead_fallback_collapses_bridge() {
    // Arrange: hut 5x5 contains only spanless bridgehead/ramp evidence;
    // the nearest destroy overlay is reachable through fallback.
    // Act: advance to C4 detonation through tick_c4_plants.
    // Assert: bridge_state_changed and the nearby overlay reaches a destroyed cap.
}

#[test]
fn c4_on_cabhut_without_bridge_evidence_clears_pending_without_damage() {
    // Arrange: CABHUT with pending C4 and no bridge evidence in fallback envelope.
    // Act: advance to C4 detonation through tick_c4_plants.
    // Assert: bridge_state_changed is false, hut HP unchanged, pending_c4_detonation is None.
}
```

Each test should advance `sim.tick` or call `tick_c4_plants` so the same production path runs.

**Step 5: Verify**

Run:

```powershell
cargo test c4_on_cabhut --lib -- --nocapture
```

Expected: all C4-on-CABHUT tests pass.

**Step 6: Commit**

Commit message: `test: cover cabhut c4 bridge collapse cases`

---

### Task 9: Run Regression Suite for Bridge and C4 Paths

**Why:** The new hut dispatcher touches shared bridge collapse walkers and C4 timer behavior; regressions must be caught before handoff.

**Files:**
- No source edits unless a test failure reveals a bug in the just-implemented tasks.

**Pattern:** Existing project verification with targeted Rust tests.

**Step 1: Run targeted tests**

```powershell
cargo test c4_on_cabhut --lib -- --nocapture
cargo test bridge_repair --lib -- --nocapture
cargo test bridge --lib -- --nocapture
```

Expected: all pass.

**Step 2: Run broader C4 and world-order tests**

```powershell
cargo test c4 --lib -- --nocapture
cargo test world_orders --lib -- --nocapture
```

Expected: all pass.

**Step 3: If unrelated failures appear**

Do not fix unrelated dirty-worktree failures unless they are caused by this change. Record the failing test names and error summaries in the implementation handoff.

**Step 4: Commit**

Commit message: `test: verify cabhut c4 bridge collapse regressions`

---

### Task 10: Run Fidelity Follow-Up

**Why:** The original symptom was a parity failure: SEAL C4 plants on CABHUT but bridge does not blow. The final verification must be stated in parity terms.

**Files:**
- Modify: `docs/fidelity-checks/c4-on-bridge-repair-hut.md`

**Pattern:** Follows existing fidelity check artifact format: input scope, binary citations, trace table, findings, verification commands.

**Step 1: Update Trace 4**

Change the Rust output for the bridge-hut scan boundary from mismatch to fixed for the covered overlay and fallback cases. Keep the existing documented adjacency plant drift as out of scope.

**Step 2: Add implementation verification section**

Record:

- tests run;
- which scenario covers overlay-first low;
- which scenario covers overlay-first high;
- which scenario covers spanless bridgehead/ramp fallback;
- which scenario covers stale pending cleanup.

**Step 3: Run final targeted test**

```powershell
cargo test c4_on_cabhut --lib -- --nocapture
```

Expected: pass.

**Step 4: Commit**

Commit message: `docs: update cabhut c4 fidelity check`

---

## Sources & References

- **Design doc:** `docs/plans/2026-05-18-cabhut-c4-bridge-collapse-design.md`
- **Ghidra report:** `docs/research/BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`
  - §18A.1 C4 action gate
  - §18A.3 CABHUT timer branch and marker cleanup
  - §18A.4 overlay-first/flag-second hut entries
  - §18A.5 `DestroyBridgeFromCell_*`
  - §18A.6 collapse sweep
  - §18A.7 3-cell walker mutation
- **Fidelity check:** `docs/fidelity-checks/c4-on-bridge-repair-hut.md`
- **Bridge collapse trace:** `docs/traces/2026-05-08-trace-bridge-damage-collapse.md`
- **Live Ghidra addresses spot-checked in planning:**
  - `0x0043FB20` - `BuildingClass::Update`
  - `0x00574C20` - `MapClass::DestroyBridge_Low_MapInit`
  - `0x00574000` - `MapClass::DestroyBridge_High_MapInit`
  - `0x00574780` - `MapClass::DestroyBridgeFromCell_Low`
  - `0x005749C0` - `MapClass::DestroyBridgeFromCell_High`
- **INI keys:**
  - `ini/rulesmd.ini` `[CombatDamage] C4Delay=.03`
  - `ini/rulesmd.ini` `[CombatDamage] C4Warhead=Super`
  - `ini/rulesmd.ini` `[GHOST] C4=yes`
  - `ini/rulesmd.ini` `[TANY] C4=yes`
  - `ini/rulesmd.ini` `[CABHUT] BridgeRepairHut=yes`
- **Related code:**
  - `src/sim/world/world_orders.rs`
  - `src/sim/world/bridge_orchestrator.rs`
  - `src/sim/bridge_state/walker.rs`
  - `src/sim/bridge_state/mod.rs`
  - `src/sim/world/world_orders_bridge_repair_tests.rs`
- **Recent relevant commit:** `71b20b2 sim: repair bridges and layer splash damage`
