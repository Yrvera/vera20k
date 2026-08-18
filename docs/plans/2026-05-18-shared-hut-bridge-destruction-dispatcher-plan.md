# Shared Hut Bridge Destruction Dispatcher Implementation Plan

> **For Codex:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Implement a shared gamemd-style bridge-hut destruction dispatcher so C4 on `CABHUT` collapses the connected bridge on real map topologies, including ramp/bridgehead fallback, while preserving existing direct-overlay C4 behavior.

**Architecture:** This is a sim-only change. It keeps C4 order and timer state in `world_orders.rs`, keeps bridge mutation and cascade side effects in `bridge_orchestrator.rs`, and reuses existing `BridgeRuntimeState` direct/state-machine mutators. No app, render, audio, UI, net, rules schema, or save schema changes are planned.

**Design Doc:** `docs/plans/2026-05-18-shared-hut-bridge-destruction-dispatcher-design.md`

---

## Grounding Summary

Primary research source is `docs/research/BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`, especially section 18A. It verifies that SEAL/Tanya C4 on CABHUT is valid in standard YR, CABHUT C4 skips normal building damage, calls `DestroyBridge_*_MapInit`, and clears the C4 marker afterward.

Live Ghidra verification in this session confirmed:

- `BuildingClass__Update @ 0x0043FB20` checks the C4 marker, waits the timer, branches on `BridgeRepairHut`, calls low/high bridge hut destruction, then clears marker and attacker pointer.
- `MapClass__DestroyBridge_High_MapInit @ 0x00574000` scans 5x5 for high overlay `0xCD..=0xE8`, otherwise searches bridge/ramp flags and calls `ApplyDamageToCell`.
- `ApplyDamageToCell @ 0x00587180` direct low range is `0x4A..=0x63`, direct high range is `0xCD..=0xE6`, and non-overlay cells can go through state-machine dispatch.

Repo pattern:

- `src/sim/world/bridge_orchestrator.rs` already owns bridge damage event orchestration and collapse cascade aggregation.
- `src/sim/bridge_state/walker.rs` already owns direct overlay walkers.
- `src/sim/bridge_state/mod.rs` already owns bridgehead/body state-machine mutation helpers.
- `src/sim/world/world_orders.rs` already owns C4 marker, timer, CABHUT branch, and pending-marker cleanup.

INI grounding:

- `ini/rulesmd.ini` has `[CombatDamage] C4Delay=.03`, parsed as 27 ticks.
- `ini/rulesmd.ini` has `[CABHUT] BridgeRepairHut=yes`.
- `CanC4` defaults true for CABHUT; listed `CanC4=no` overrides are unrelated structures.

Open implementation risk:

- Rust does not store exact gamemd `CellClass+0x140` bridge flags. The plan uses existing `ResolvedTerrainCell` and `BridgeRuntimeCell` evidence inside a local helper. If review rejects that approximation, add an exact bridge-flag modeling task before Task 3.

---

## Key Technical Decisions

- Add a no-RNG hut `ApplyDamageToCell` equivalent instead of calling `apply_bridge_damage_events`. **Confidence:** high
  - **Source:** Ghidra `ApplyDamageToCell @ 0x00587180`; normal combat event path uses `BridgeStrength` RNG but hut map-init damage does not.

- Keep `dispatch_bridge_collapse_from_hut` signature stable and route it through a shared internal helper. **Confidence:** high
  - **Source:** current `world_orders.rs` caller; minimizes blast radius.

- Use fixed direction-connected fallback cells, not arbitrary nearest-overlay search. **Confidence:** high
  - **Source:** Ghidra `DestroyBridge_High_MapInit @ 0x00574000` walks bridge/ramp evidence directionally after overlay-first scan fails.

- Clear CABHUT C4 pending state in `world_orders.rs` regardless of whether the dispatcher changed bridge state. **Confidence:** high
  - **Source:** Ghidra `BuildingClass__Update @ 0x0043FB20` clears after the dispatcher call.

- Preserve current direct-overlay happy path and tests. **Confidence:** high
  - **Source:** existing `cargo test c4_on_cabhut --lib -- --nocapture` passes and should remain a regression guard.

---

## Open Questions

### Resolved During Planning

- **Does the C4 sound prove plant claim?** Yes. Rust emits `SimSoundEvent::C4Planted` immediately after setting `pending_c4_detonation`.
- **Is the remaining bug before or after the timer?** After the timer. User confirms the C4 sound; trace shows the CABHUT bridge dispatcher can no-op.
- **Should this be C4-only?** No. User selected the shared dispatcher scope.

### Resolved Fixture Choice

- **Bridgehead/ramp fallback fixture:** Use a deterministic eastward fixture. Put the CABHUT center at `(9, 10)`. Ensure every cell in the hut-centered 5x5 scan (`x=7..=11`, `y=8..=12`) has no direct destroy overlay. Put the first fallback evidence at `(12, 10)` as a ramp/bridgehead-style evidence cell, then put the first collapsible bridge state-machine cell at `(13, 10)` on the same eastward trace. The test must assert that `(12, 10)` by itself does not collapse the bridge and that the dispatcher continues to `(13, 10)`.
- **Do existing dirty edits already implement part of the plan?** Before each task, read the current file and work with existing edits. Do not revert unrelated changes.

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/world/bridge_orchestrator.rs` | Add shared hut dispatcher, no-RNG cell damage helper, direction-connected fallback, and cascade reuse. |
| Modify | `src/sim/world/world_orders.rs` | Keep CABHUT C4 caller and marker cleanup correct; no behavior change for normal C4 retry. |
| Modify | `src/sim/world/world_orders_bridge_repair_tests.rs` | Add direct high, direct low, fallback, no-bridge, and cleanup regression tests. |
| Possibly modify | `src/sim/bridge_state/mod.rs` | Only if existing state-machine helpers are not visible enough from `bridge_orchestrator.rs`. |
| Possibly modify | `src/sim/bridge_state/walker.rs` | Only if existing direct overlay predicates or axis helpers are not visible enough. |

---

## Interface Changes

- Keep `dispatch_bridge_collapse_from_hut(sim, rules, hut_center) -> bool` unchanged.
- Add only private or `pub(crate)` helpers in `bridge_orchestrator.rs`.
- Do not add persistent state.
- Do not change command enum, rules parsing, app event APIs, or render APIs.

---

## Sim Checklist

- [ ] All new logic uses integer cell coordinates only.
- [ ] No f32/f64 in sim logic.
- [ ] No new persistent state; state hash unchanged.
- [ ] No dependencies from `sim/` to render/ui/sidebar/audio/net.
- [ ] Tick order unchanged: C4 detonation remains in `tick_c4_plants`.
- [ ] Deterministic scan and fallback order is explicit and tested.

---

## Risk Areas

- Fallback may be too broad and collapse unrelated bridges.
- Fallback may be too narrow and still miss real CABHUT bridgehead/ramp layouts.
- Direct overlay and fallback ranges differ; mixing `DestroyBridge_*_MapInit` scan ranges with `ApplyDamageToCell` ranges incorrectly can cause terminal destroyed overlays to behave wrong.
- Normal combat bridge damage must not inherit no-RNG hut behavior.
- Non-hut Iron Curtain C4 retry must keep pending state.

---

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 1 | Shared dispatcher preserves CABHUT C4 timing | Player hears plant sound, then bridge changes after C4 delay | Existing C4 timer tests plus CABHUT tests |
| 2 | Overlay-first scan ranges | Wrong inclusive bounds choose wrong bridge or no-op | Direct low/high overlay-first CABHUT tests with `0x4A`, `0x65`, `0xCD`, `0xE8`; fallback `ApplyDamageToCell` negative tests for `0x64`, `0x65`, `0xE7`, `0xE8` |
| 3 | No-RNG fallback cell damage | Combat RNG in this path would make hut C4 intermittent, unlike gamemd map-init damage | Test fallback collapse is deterministic |
| 4 | Bridgehead/ramp fallback | This is the reported visible failure | Test no direct hut overlay but fallback state-machine mutates bridge |
| 5 | C4 pending marker cleanup | Prevents repeated no-op and allows future C4 order | No-bridge and bridge-changed tests assert pending cleared |
| 6 | Normal C4 and Iron Curtain retry | CABHUT fix must not regress normal C4 behavior | `cargo test c4 --lib -- --nocapture` |

---

## Tasks

### Task 1: Introduce Shared Hut Dispatcher Skeleton

**Why:** Establish the single entry that C4 and future hut-destruction callers use before changing fallback behavior.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs`

**Pattern:** Follows existing private helper style in `bridge_orchestrator.rs`.

**Step 1: Add a private helper under `dispatch_bridge_collapse_from_hut`**

Use the same signature shape but keep it private:

```rust
fn dispatch_hut_bridge_destruction(
    sim: &mut Simulation,
    rules: &RuleSet,
    hut_center: (u16, u16),
) -> bool {
    dispatch_bridge_collapse_from_hut_impl(sim, rules, hut_center)
}
```

If the file already has a clean internal implementation body, name the helper `dispatch_bridge_collapse_from_hut_impl` and move the current body into it.

**Step 2: Keep the public crate-local wrapper stable**

```rust
pub(crate) fn dispatch_bridge_collapse_from_hut(
    sim: &mut Simulation,
    rules: &RuleSet,
    hut_center: (u16, u16),
) -> bool {
    dispatch_hut_bridge_destruction(sim, rules, hut_center)
}
```

**Step 3: Verify no behavior change**

Run:

```powershell
cargo test c4_on_cabhut --lib -- --nocapture
```

Expected: existing CABHUT tests still pass.

---

### Task 2: Factor Cascade Aggregation Into a Reusable Helper

**Why:** Overlay-first and fallback paths should both produce `StateOutcome`s and share one cascade side-effect path.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs`

**Pattern:** Extracts the existing code after `outcomes.is_empty()` in `dispatch_bridge_collapse_from_hut`.

**Step 1: Add helper signature**

```rust
fn apply_hut_bridge_outcomes(
    sim: &mut Simulation,
    rules: &RuleSet,
    outcomes: &[StateOutcome],
) -> bool
```

**Step 2: Move existing aggregation into helper**

Move the code that:

- builds `destroyed_set`;
- builds `blow_up_cells`;
- builds `rim_cells`;
- tracks `any_zones_dirty`;
- resolves C4 `InfDeath`;
- calls `kill_ground_occupants_at`;
- calls `drop_in_bridge_deck_entities`;
- calls `spawn_bridge_debris`;
- calls `update_adjacent_bridges`;
- calls `notify_bridge_span_collapse`;
- calls `refresh_bridge_zones_if_dirty`;
- returns `!destroyed_set.is_empty()`.

Do not change the order of those side effects.

**Step 3: Call helper from current dispatcher**

Replace the moved block with:

```rust
apply_hut_bridge_outcomes(sim, rules, &outcomes)
```

**Step 4: Verify**

Run:

```powershell
cargo test c4_on_cabhut --lib -- --nocapture
```

Expected: existing CABHUT tests still pass.

---

### Task 3: Add Hut Damage Entry Types

**Why:** Direct overlay entries and fallback cells have different dispatch rules in gamemd.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs`

**Pattern:** Same local enum style as existing `HutBridgeFamily` and `HutFallbackSeed`.

**Step 1: Add enum near existing hut helper types**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HutDamageEntry {
    DirectOverlay {
        rx: u16,
        ry: u16,
        family: HutBridgeFamily,
    },
    FallbackCell {
        rx: u16,
        ry: u16,
    },
}
```

**Step 2: Add small constructor helpers only if they improve call sites**

Acceptable helpers:

```rust
fn direct_hut_entry(rx: u16, ry: u16, family: HutBridgeFamily) -> HutDamageEntry {
    HutDamageEntry::DirectOverlay { rx, ry, family }
}

fn fallback_hut_entry(rx: u16, ry: u16) -> HutDamageEntry {
    HutDamageEntry::FallbackCell { rx, ry }
}
```

Do not add persistent sim state.

**Step 3: Verify compile**

Run:

```powershell
cargo test c4_on_cabhut --lib -- --nocapture
```

Expected: compile succeeds and existing tests pass.

---

### Task 4: Implement No-RNG Hut ApplyDamageToCell Helper

**Why:** Gamemd hut destruction fallback calls `ApplyDamageToCell`, which dispatches bridge damage without the combat `BridgeStrength` RNG gate.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs`

**Pattern:** Mirrors the outcome-producing branch in `apply_bridge_damage_events`, but without event data, warhead fields, impact Z gate, or RNG.

**Step 1: Add helper signature**

```rust
fn apply_hut_damage_to_cell(
    bridge_state: &mut BridgeRuntimeState,
    terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid,
    rx: u16,
    ry: u16,
) -> StateOutcome
```

**Step 2: Implement direct overlay dispatch first**

Use gamemd `ApplyDamageToCell` direct ranges:

```rust
let Some(cell) = bridge_state.cell(rx, ry).copied() else {
    return StateOutcome::NoChange;
};
let overlay = cell.overlay_byte;
if (0x4A..=0x63).contains(&overlay) {
    return bridge_state.destroy_bridge_low(rx, ry, terrain);
}
if (0xCD..=0xE6).contains(&overlay) {
    return bridge_state.destroy_bridge_high(rx, ry, terrain);
}
```

**Step 3: Implement state-machine fallback**

Use runtime role and terrain evidence:

- If the cell role is `BridgeCellRole::Bridgehead`, call `bridgehead_advance_state` only as an absorbed ramp/anchor-side update. This helper never collapses the bridge. The caller must continue along the chosen fallback direction to a body/anchor/direct cell that can collapse.
- Otherwise, if the runtime cell has `bridge_group_id`, `anchor_span_id`, `deck_present`, or role `Anchor`, call `body_cell_advance_state`.
- Determine high vs low with a helper:

```rust
fn hut_cell_is_low_bridge(
    bridge_state: &BridgeRuntimeState,
    terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid,
    rx: u16,
    ry: u16,
) -> bool
```

The helper returns true when:

- runtime overlay is low destroy overlay;
- terrain cell has `is_wood_bridge_repair_tile`;
- terrain bridge facts identify a low/wood bridge if such field is available;
- terrain cell bridge layer overlay is low destroy overlay.

Otherwise it returns false, which selects high.

Then:

```rust
let is_high = !hut_cell_is_low_bridge(bridge_state, terrain, rx, ry);
match cell.role {
    BridgeCellRole::Bridgehead => bridge_state.bridgehead_advance_state(rx, ry, is_high, terrain),
    BridgeCellRole::Anchor | BridgeCellRole::Body => {
        bridge_state.body_cell_advance_state(rx, ry, is_high, terrain)
    }
    _ if cell.deck_present || cell.bridge_group_id.is_some() || cell.anchor_span_id.is_some() => {
        bridge_state.body_cell_advance_state(rx, ry, is_high, terrain)
    }
    _ => StateOutcome::NoChange,
}
```

If `BridgeCellRole` variants differ in current code, use the actual variants and keep the same role intent.

Do not treat `StateOutcome::Absorbed` from a bridgehead cell as success for CABHUT bridge collapse. It may be a real state-machine write, but it is not the player-visible bridge collapse. Fallback dispatch must keep walking until it reaches a body/anchor/direct cell that yields `StateOutcome::Collapsed` or the bounded trace is exhausted.

**Step 4: Add focused unit tests**

Add tests in `bridge_orchestrator.rs` test module if it exists; otherwise use `world_orders_bridge_repair_tests.rs` integration fixture.

Required assertions:

- direct low `0x4A` dispatch returns non-`NoChange` on a seeded low bridge cell;
- direct high `0xCD` dispatch returns non-`NoChange` on a seeded high bridge cell;
- terminal low `0x64` and `0x65` do not go through direct `ApplyDamageToCell` range;
- terminal high `0xE7` and `0xE8` do not go through direct `ApplyDamageToCell` range;
- a pure `BridgeCellRole::Bridgehead` fallback cell returns `Absorbed` or `NoChange` but never `Collapsed`, and the hut dispatcher continues to the next direction-connected candidate.

**Step 5: Verify**

Run:

```powershell
cargo test hut_damage_to_cell --lib -- --nocapture
```

Expected: new tests pass.

---

### Task 5: Replace Fallback Overlay Requirement With Direction-Connected Fallback Cells

**Why:** Current fallback still requires finding a destroy overlay before any bridge mutation. Gamemd fallback can call `ApplyDamageToCell` from bridgehead/ramp evidence.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs`

**Pattern:** Extends existing `find_hut_fallback_seed` and `has_hut_fallback_bridge_evidence`.

**Step 1: Add fallback cell finder**

```rust
fn find_hut_fallback_cells(
    sim: &Simulation,
    hut_center: (u16, u16),
) -> Vec<(u16, u16)>
```

Use deterministic order:

1. hut center if it has bridge evidence;
2. for each direction in existing `HUT_FALLBACK_DIRS`;
3. for distance `1..=3`;
4. once evidence is found, follow only that direction for up to `HUT_FALLBACK_TRACE_LIMIT`;
5. push cells while `has_hut_fallback_bridge_evidence` is true.

Limit the returned vector to the first useful direction. Do not collect all directions into one broad search.

**Step 2: Keep existing overlay-first direct path**

The dispatcher should still do:

```rust
let entry = find_hut_overlay_entry(...);
```

first. Only call fallback cell finder if overlay entry is `None`.

**Step 3: Execute fallback cells with `apply_hut_damage_to_cell`**

For fallback cells, call `apply_hut_damage_to_cell` in order. Collect each non-`NoChange` outcome. Stop after first `Collapsed` outcome. If outcomes are `Absorbed` or intermediate damage, continue up to the existing bounded attempt/step constants.

Important: bridgehead cells are evidence and may receive the same anchor/ramp-side state writes as direct bridgehead damage, but `bridgehead_advance_state` is documented and tested to never collapse a span. A bridgehead-only fallback path is still a no-op for the user's visible CABHUT case. The fallback finder must therefore include the direction-connected body/anchor/direct cells behind the bridgehead/ramp evidence, not stop at the first bridgehead cell.

Do not call `destroy_bridge_low/high` directly from fallback unless the fallback cell has direct overlay in `ApplyDamageToCell` range.

**Step 4: Preserve no-evidence behavior**

If fallback returns empty or every cell returns `NoChange`, the dispatcher returns `false`. `world_orders.rs` will still clear the CABHUT pending marker because the hut dispatch was consumed.

**Step 5: Verify existing tests**

Run:

```powershell
cargo test c4_on_cabhut --lib -- --nocapture
```

Expected: existing direct-overlay tests pass.

---

### Task 6: Add Bridgehead/Ramp-Only CABHUT C4 Regression Test

**Why:** This test proves the reported symptom is fixed: C4 plants and timer expires even though direct overlay lookup from hut scan is insufficient.

**Files:**
- Modify: `src/sim/world/world_orders_bridge_repair_tests.rs`

**Pattern:** Use existing `build_sim`, `spawn_cabhut`, `spawn_seal`, `step`, and bridge seeding helpers.

**Step 1: Add fixture helper**

Create a helper that seeds this exact shape:

- CABHUT center at `(9, 10)`.
- No direct destroy overlay in the hut-centered 5x5 scan (`x=7..=11`, `y=8..=12`).
- First fallback evidence at `(12, 10)`, east of the hut, with bridgehead/ramp evidence only:
  - runtime role `BridgeCellRole::Bridgehead`;
  - axis `Some(Axis::EW)` or the axis required by the existing fixture helper;
  - no `bridge_group_id`;
  - no direct destroy overlay.
- First collapsible fallback cell at `(13, 10)`, same eastward trace:
  - runtime role `BridgeCellRole::Body` or `BridgeCellRole::Anchor`;
  - `bridge_group_id=Some(1)`;
  - `deck_present=true`;
  - state seeded so one hut dispatch can produce `StateOutcome::Collapsed` through `body_cell_advance_state`;
  - no direct destroy overlay.
- Additional connected span cells only if the existing body-cell helper requires them for cascade consistency.

Name it:

```rust
fn seed_hut_fallback_bridgehead_layout(sim: &mut Simulation)
```

The helper must set enough `ResolvedTerrainCell` and `BridgeRuntimeCell` data for `has_hut_fallback_bridge_evidence` to find `(12, 10)` first, then continue east to `(13, 10)`. The test must fail against the current overlay-required fallback and must also fail if the new fallback stops after the bridgehead `Absorbed` outcome.

**Step 2: Add test**

```rust
#[test]
fn c4_on_cabhut_bridgehead_fallback_collapses_bridge() {
    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let seal = spawn_seal(&mut sim, 9, 10);
    let hut_hp = sim.entities.get(cabhut).unwrap().health.current;
    seed_hut_fallback_bridgehead_layout(&mut sim);

    sim.entities.get_mut(cabhut).unwrap().pending_c4_detonation =
        Some(PendingC4Detonation {
            plant_start_tick: sim.tick,
            attacker_id: seal,
        });

    let mut bridge_state_changed_seen = false;
    for _ in 0..(rules.c4_delay_ticks as u64 + 1) {
        let result = step(&mut sim, &rules, &heights);
        bridge_state_changed_seen |= result.bridge_state_changed;
    }

    let hut = sim.entities.get(cabhut).unwrap();
    assert_eq!(hut.health.current, hut_hp);
    assert!(!hut.dying);
    assert!(hut.pending_c4_detonation.is_none());
    assert!(bridge_state_changed_seen);
}
```

Use the coordinates above unless existing helper boundaries require a larger map with the same relative layout. Do not place a direct overlay in the hut 5x5; the test must fail against the current overlay-required fallback. Assert after the tick loop that the `(13, 10)` body/anchor cell reached `DamageState::Destroyed`, not merely that the `(12, 10)` bridgehead absorbed damage.

**Step 3: Verify failure before fix if practical**

If the implementation is being done in strict TDD order, run the test before Task 5 and confirm it fails. If Task 5 is already implemented, confirm it passes.

**Step 4: Verify**

Run:

```powershell
cargo test c4_on_cabhut_bridgehead_fallback_collapses_bridge --lib -- --nocapture
```

Expected after Task 5: pass.

---

### Task 7: Add Wrong-Nearby-Bridge Regression Test

**Why:** Prevent the fallback from becoming an arbitrary nearest-overlay search that collapses an unrelated bridge.

**Files:**
- Modify: `src/sim/world/world_orders_bridge_repair_tests.rs`

**Pattern:** Similar fixture style to Task 6.

**Step 1: Add fixture**

Seed:

- one bridge/ramp evidence direction from hut that does not connect to a destroyable bridge;
- one direct bridge overlay nearby but off the evidence direction.

**Step 2: Add test**

The test should set pending C4 on CABHUT, advance through `C4Delay`, and assert:

- hut HP unchanged;
- pending marker cleared;
- `bridge_state_changed_seen == false`;
- the unrelated bridge cell damage state remains unchanged.

**Step 3: Verify**

Run:

```powershell
cargo test c4_on_cabhut_fallback_does_not_collapse_unrelated_bridge --lib -- --nocapture
```

Expected: pass.

---

### Task 8: Add Direct Low/Terminal Overlay CABHUT Tests

**Why:** Existing happy path coverage is high-bridge-shaped; low bridge selection and terminal overlay scan bounds must stay gamemd-compatible.

**Files:**
- Modify: `src/sim/world/world_orders_bridge_repair_tests.rs`

**Pattern:** Reuse existing seeded bridge helper or add a low-overlay equivalent.

**Step 1: Seed low bridge body cells**

Create helper:

```rust
fn seed_low_bridge_with_state(sim: &mut Simulation, state: DamageState)
```

Use low overlays in the `0x4A..=0x65` family and mark terrain evidence as low/wood where the existing model supports it.

**Step 2: Add low direct-overlay test**

Name:

```rust
#[test]
fn c4_on_cabhut_low_overlay_collapses_low_bridge()
```

Assertions:

- CABHUT survives;
- pending marker clears;
- `bridge_state_changed_seen` is true;
- at least one seeded low bridge cell reaches `DamageState::Destroyed`.

**Step 3: Add terminal overlay-first tests**

Add focused CABHUT overlay-first tests proving map-init scan bounds accept terminal overlays:

```rust
#[test]
fn c4_on_cabhut_low_terminal_overlay_0x65_uses_overlay_first_scan()

#[test]
fn c4_on_cabhut_high_terminal_overlay_0xE8_uses_overlay_first_scan()
```

Each test should seed the terminal overlay inside the hut-centered 5x5 and assert the dispatcher enters the overlay-first path. If a terminal overlay is already destroyed and does not create new `DamageState::Destroyed` cells, assert a stable observable appropriate to the existing direct walker, but do not allow the dispatcher to fall back to unrelated bridge evidence.

**Step 4: Verify**

Run:

```powershell
cargo test c4_on_cabhut_low_overlay_collapses_low_bridge --lib -- --nocapture
```

Expected: pass.

---

### Task 9: Confirm CABHUT Pending Cleanup and Normal C4 Retry

**Why:** CABHUT cleanup and normal C4 retry use adjacent branches in `tick_c4_plants`; this task prevents regression.

**Files:**
- Modify only if tests reveal a cleanup gap:
  - `src/sim/world/world_orders.rs`
  - `src/sim/world/world_orders_c4_tests.rs`
  - `src/sim/world/world_orders_bridge_repair_tests.rs`

**Pattern:** Existing `c4_on_cabhut_without_bridge_clears_pending_marker` and Iron Curtain C4 tests.

**Step 1: Read current cleanup branch**

Confirm `outcome.consumed_pending_marker` clears:

- target building `pending_c4_detonation`;
- attacker `c4_plant` if it points at the building.

**Step 2: Add or update test for second C4 after CABHUT dispatch**

Name:

```rust
#[test]
fn c4_on_cabhut_can_be_marked_again_after_dispatch_cleanup()
```

Flow:

1. Seed CABHUT pending C4 and no bridge evidence.
2. Advance through delay.
3. Assert pending cleared.
4. Set a new attacker `c4_plant` targeting the same CABHUT and place the attacker on the hut cell.
5. Step once.
6. Assert a new `pending_c4_detonation` exists.

**Step 3: Run normal C4 and Iron Curtain tests**

Run:

```powershell
cargo test c4 --lib -- --nocapture
```

Expected:

- normal C4 happy path still passes;
- non-hut Iron Curtain C4 retry still passes;
- CABHUT cleanup tests pass.

---

### Task 10: Final Targeted Verification

**Why:** The change affects C4, CABHUT bridge collapse, and bridge mutation; final verification must cover all three.

**Files:**
- No edits unless tests expose defects.

**Step 1: Run targeted CABHUT tests**

```powershell
cargo test c4_on_cabhut --lib -- --nocapture
```

Expected: all CABHUT C4 tests pass.

**Step 2: Run all C4 tests**

```powershell
cargo test c4 --lib -- --nocapture
```

Expected: all C4 tests pass.

**Step 3: Run bridge repair/collapse related tests if time allows**

```powershell
cargo test bridge_repair --lib -- --nocapture
cargo test bridge_state --lib -- --nocapture
```

Expected: pass or only unrelated pre-existing warnings.

**Step 4: Manual runtime check**

Run the game, issue SEAL C4 on CABHUT, and observe:

- `SealPlaceBomb` plays at plant time;
- after about 27 ticks, the bridge visually collapses;
- CABHUT remains alive;
- bridge pathing updates after collapse.

---

## Sources & References

- **Design doc:** `docs/plans/2026-05-18-shared-hut-bridge-destruction-dispatcher-design.md`
- **Existing superseded design:** `docs/plans/2026-05-18-cabhut-c4-bridge-collapse-design.md`
- **Existing superseded plan:** `docs/plans/2026-05-18-cabhut-c4-bridge-collapse-plan.md`
- **Primary report:** `docs/research/BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`
- **CABHUT report:** `docs/research/TECH_CABHUT_GHIDRA_REPORT.md`
- **Fidelity check:** `docs/fidelity-checks/c4-on-bridge-repair-hut.md`
- **Ghidra addresses:** `0x0043FB20` (`BuildingClass__Update`), `0x00574000` (`MapClass__DestroyBridge_High_MapInit`), `0x00574C20` (`MapClass__DestroyBridge_Low_MapInit`), `0x00587180` (`ApplyDamageToCell`)
- **INI keys:** `ini/rulesmd.ini` `[CombatDamage] C4Delay=.03`, `[CABHUT] BridgeRepairHut=yes`
- **Related code:** `src/sim/world/bridge_orchestrator.rs`, `src/sim/world/world_orders.rs`, `src/sim/bridge_state/mod.rs`, `src/sim/bridge_state/walker.rs`, `src/sim/world/world_orders_bridge_repair_tests.rs`
