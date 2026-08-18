# Bridgehead Damage Progression Implementation Plan (G3)

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Make Rust's bridgehead direct-damage path produce the same observable output as gamemd's `ProcessBridgeDamageStateMachine_High` @ 0x00576BA0: sparse, mostly absorbed, with at most one anchor tile-class transition per first-hit, never collapsing the bridge from sustained ramp fire alone.

**Architecture:** Add a new per-cell tile-class enum field (`bridgehead_anchor_class`) that mirrors the binary's `IsoTileTypeIndex` writes on the anchor cell. Refactor `bridgehead_walk_to_anchor` to compute direction internally per the binary's literal `JGE` branch, and drop the mid-walk parity check (Rust was stricter than gamemd). Extend `update_ramp_perpendicular` to also write the new field on Anchor and Bridgehead targets, mirroring the binary's asymmetric A/B tile-class progression. Rewrite `bridgehead_advance_state` to never mutate the bridgehead cell's own state and to remove the wrong `Damaged → Destroyed` collapse branch.

**Design Doc:** [docs/plans/2026-05-12-bridgehead-damage-progression-design.md](2026-05-12-bridgehead-damage-progression-design.md)

---

## Grounding Summary

**Design doc** lays out 18-item Tiny-Detail Ledger sourced from today's verify-doc audit (YELLOW status, [AUDIT_LOG.md](../../../ra2-rust-game-docs/AUDIT_LOG.md) entry 2026-05-12), today's [fidelity-check artifact](../fidelity-checks/bridgehead-damage-progression.md) (corrected), and the literal-disassembly RE follow-ups for items 5, 7.

**Binary anchors** (all decompiled today, confidence content+identity HIGH, binding MEDIUM):
- `ProcessBridgeDamageStateMachine_High` @ 0x00576BA0 — main driver; NS branch at 0x005771c3, EW branch at 0x00576c5f
- `SetOverlayAndPropagate` @ 0x0056EB80 — writes `IsoTileTypeIndex` (not OverlayTypeIndex; function name is misleading)
- `UpdateRamp_NS_DamageA_High` @ 0x00572230 — preserves tile class on target
- `UpdateRamp_NS_DamageB_High` @ 0x00572330 — progresses `ABAD30→ABAD30+1`, `ABAD30+1→ABAD30+2`
- `UpdateRamp_NS_CollapseA_High` @ 0x00572440 — collapse variant
- `UpdateRamp_NS_CollapseB_High` @ 0x005727E0 — collapse variant
- EW siblings @ 0x00572B80 / 0x00572C90 / 0x00572DA0 / 0x00573170

**Walk direction (verified literal):**
- NS @ 0x005771d3: `h < 4 → S (DAT_0089f698)`, `h == 4 → at anchor`, `h > 4 → N (DAT_0089f688)`
- EW @ 0x00576ca3: `h < 2 → E (DAT_0089f690)`, `h == 2 → at anchor`, `h > 2 → W (DAT_0089f6a0)`
- Mid-walk parity check: gamemd has NONE; current Rust check is stricter than the binary

**Repo pattern to mirror:** `body_cell_advance_state` at [src/sim/bridge_state/mod.rs:797](../../src/sim/bridge_state/mod.rs#L797) — closest structural analog (same `(rx, ry, is_high_bridge, terrain)` signature, same `StateOutcome` return, same `update_ramp_perpendicular` calls).

**Recent commits to be aware of** (landed AFTER the brainstorm session — important):
- `7d1f147` — threaded `&ResolvedTerrainGrid` through `body_cell_*` and `update_ramp_perpendicular`. The helper now takes `terrain` as a 6th parameter.
- `1c9b63e` — `update_ramp_perpendicular` now also fires `state.apply_damaged_variant_flood_fill(target, true, terrain)` after a successful state-byte write. Tile-class write extension must compose with this, not replace it.
- `6f150dc` — `body_cell_repair_state` clears damaged-variant on repair.

**INI keys:** `[CombatDamage] BridgeStrength=` (default 1500) already wired into `BridgeRuntimeState.bridge_strength` and consumed by the dispatcher's RNG gate at [src/sim/world/bridge_orchestrator.rs:621](../../src/sim/world/bridge_orchestrator.rs#L621). No new INI plumbing required.

**Save-format compat:** no `save_version` mechanism exists in this project. Serde derive on these structs is for in-memory snapshotting; adding a field with `#[serde(default)]` is enough.

**What's still unknown** (→ Deferred Open Questions):
- Renderer mapping `BridgeheadAnchorClass → anchor TMP tile` (requires .TMP inspection per theater)
- Whether real maps actually have multi-cell ramps where the mid-walk parity divergence would trigger

## Key Technical Decisions

- **Field shape: `BridgeheadAnchorClass` 4-variant enum on every `BridgeRuntimeCell`.** Approach A from the brainstorm. **Confidence:** high. **Source:** design doc Approach Choice; rejected alternatives documented therein.
- **Walk direction computed inside `bridgehead_walk_to_anchor`, not by caller.** Eliminates Gap 4 by making the helper's contract identical to the binary. **Confidence:** high. **Source:** Ghidra 0x005771d3 (NS) + 0x00576ca3 (EW), literal asm read today.
- **Drop mid-walk parity check.** Rust was stricter than gamemd. **Confidence:** high. **Source:** Ghidra 0x005771eb-0x00577237 — no parity check inside walk loop.
- **`update_ramp_perpendicular` accepts both `Anchor` and `Bridgehead` target roles.** Bridgehead targets get only the tile-class write (no state-byte bump because bridgeheads don't have `+0x140 & 0x80`). **Confidence:** high. **Source:** Ghidra `UpdateRamp_NS_DamageB_High` @ 0x00572330 — writes ABAD30+1 on neighbor regardless of role flag.
- **Remove `Damaged → Destroyed` branch from `bridgehead_advance_state`.** Sustained bridgehead-only fire never collapses a bridge in gamemd. **Confidence:** high. **Source:** ledger item 15, design doc fidelity-check.
- **`update_ramp_perpendicular` still fires `apply_damaged_variant_flood_fill` after tile-class writes.** Compose with the recent G4 work, don't break it. **Confidence:** high. **Source:** commit `1c9b63e` + read of current helper body.

## Open Questions

### Resolved During Planning

- **Save-format compat strategy** — no `save_version` exists in the project. `#[serde(default)]` on the new field suffices.
- **EW walk direction (ledger item 5)** — resolved by today's literal disassembly read: `h < 2 → E`, `h > 2 → W`. Fidelity-check artifact corrected.
- **Mid-walk parity behavior (ledger item 7)** — resolved by literal asm read: gamemd has NO mid-walk parity check.

### Deferred to Implementation

- **Real-world frequency of multi-cell ramps.** Map data dependent. The mid-walk parity fix lands regardless, but its observable impact is unknown until tested on real YR maps.
- **Renderer mapping `BridgeheadAnchorClass → TMP tile`.** Out of scope for G3 per design doc. Tracked as a separate brainstorm follow-up.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/bridge_state/mod.rs` (add enum, add field, rewrite driver, update tests) | New tile-class state on cells; restructured bridgehead state machine |
| Modify | `src/sim/bridge_specs.rs` (refactor walker, extend perpendicular helper) | Walk-direction computation, asymmetric tile-class writes |
| Modify | `src/sim/world/world_hash.rs` (one-line addition) | Include new field in deterministic state hash |

## Interface Changes

- **New public type** in `bridge_state::`: `enum BridgeheadAnchorClass { Variant0, Variant1, Damaged, AboutToFall }` (Serialize, Deserialize, Default = Variant0, PartialEq, Eq, Hash, Clone, Copy, Debug).
- **New public field** on `BridgeRuntimeCell`: `bridgehead_anchor_class: BridgeheadAnchorClass`. Decorated `#[serde(default)]` for backward serialization compat.
- **Signature change** on `bridge_specs::bridgehead_walk_to_anchor`: drops the `direction: Direction` parameter. New signature:
  ```
  pub fn bridgehead_walk_to_anchor(
      start: (u16, u16),
      axis: Axis,
      cell_height: impl Fn((u16, u16)) -> Option<u8>,
      map_width: u16,
      map_height: u16,
  ) -> Option<(u16, u16)>
  ```
- **Behavior change (no signature change)** on `bridge_specs::update_ramp_perpendicular`: target-role filter now accepts both `Anchor` and `Bridgehead` (previously only `Anchor`). Bridgehead targets receive only the tile-class write; Anchor targets receive both state-byte bump and tile-class write.
- **Behavior change (no signature change)** on `bridge_state::bridgehead_advance_state`: returns `Absorbed` (with anchor's tile class written) or `NoChange`. Never returns `Collapsed`. Never mutates the bridgehead cell's own `damage_state`.

**Callers affected:**
- `bridge_specs::bridgehead_walk_to_anchor` is currently called only from `bridge_state::bridgehead_advance_state` ([line 1142](../../src/sim/bridge_state/mod.rs#L1142)). Caller dropped the direction-passing as part of the rewrite in Task 9.
- `update_ramp_perpendicular` callers: `body_cell_advance_state` and the new `bridgehead_advance_state`. No signature change, no caller updates needed.

## Sim Checklist

- [x] All math uses `fixed`-point or integer types — no f32/f64 in any new code (only enum + u16 coords).
- [x] New state included in deterministic state hash — Task 3 adds `bridgehead_anchor_class.hash(hasher)` to `hash_bridge_state` at [src/sim/world/world_hash.rs:233](../../src/sim/world/world_hash.rs#L233).
- [x] No dependencies on render/ui/sidebar/audio/net — all changes are in `sim/`.
- [x] Tick ordering: bridgehead damage already runs inside `tick_bridge_damage_events` (Phase 5 of `World::advance_tick`); no changes to ordering.
- [x] BTreeMap iteration order: `BridgeRuntimeState.cells` is a `Vec<Option<...>>`, not a BTreeMap; iteration is index-ordered and deterministic. AnchorSpan registry is `BTreeMap<u16, AnchorSpan>` which iterates sorted. No determinism risk.
- [x] No RNG draws on this path. The function is purely deterministic.

## Risk Areas

From design doc Impact Analysis:

- **Renderer regression window.** Between G3 landing and the renderer follow-up, players will see *no* visible damage from direct ramp fire (currently they see wrong damage). Net parity improvement vs status quo. Commit message must flag this.
- **Existing test rewrites.** Seven existing `bridgehead_advance_state` tests at lines 2466, 2490, 2548, 2557, 2566, 2581, 2592 — two need rewriting (Healthy→Damaged, Damaged→Destroyed) because the contract changes. The others remain valid.
- **`apply_damaged_variant_flood_fill` composition.** New tile-class writes must NOT break the existing flood-fill behavior (G4 work). Confirm via test that flood-fill still fires after a successful state-byte write on an Anchor target, even when our new tile-class write also fires.

## Parity-Critical Items

Every task with player-observable parity stakes:

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 5 | Walk direction tie-breaker (NS: `h<4→S`, `h>4→N`; EW: `h<2→E`, `h>2→W`) | A wrong direction lands on a different anchor cell or runs off map — wrong cell shows damage | Unit tests cover all four boundaries (h=0/2/6/8 for NS, h=0/2/4 for EW); Ghidra cite 0x005771d3, 0x00576ca3 |
| Task 5 | Mid-walk parity tolerance | Multi-cell ramps with odd intermediates: Rust stricter than gamemd means damage absorbed where gamemd damages anchor | Unit test: h=8 → h=5 → h=4 walk path; assert anchor reached |
| Task 5 | Start-cell parity gate (NS: `h&1`, EW: `h>4`) | Preserves "ramps mostly absorb damage" parity | Unit tests for h=5 NS (absorb), h=0xC EW (absorb) |
| Task 7 | DamageB asymmetric progression (Variant0→Variant1, Variant1→Damaged) | Only mechanism showing `+1` intermediate visual on neighbor bridgeheads when body is damaged | Unit test fires DamageB on Variant0 bridgehead; asserts Variant1. Repeat: Variant1 → Damaged. Ghidra cite 0x00572330. |
| Task 7 | DamageA preserve | Asymmetric counterpart — DamageA must NOT advance the tile class | Unit test fires DamageA on Variant0; asserts still Variant0. On Damaged; asserts still Damaged. |
| Task 9 | Bridgehead's own state never mutated on direct hits | gamemd writes to anchor's `+0x38`, not the hit cell. Wrong cell visibly changes today. | Test: hit bridgehead 5×, assert `cell.damage_state` unchanged throughout |
| Task 9 | Anchor's `bridgehead_anchor_class = Damaged` on first hit (idempotent thereafter) | The single observable transition on this path | Test: hit bridgehead once, assert anchor field is Damaged; hit again, assert still Damaged |
| Task 9 | No collapse from sustained direct ramp fire | gamemd never collapses from this path | Test: hit ramp 100×, assert no `StateOutcome::Collapsed` ever returned |
| Task 11 | Determinism stability after the field is in the state hash | Replays/snapshots remain reproducible | Two parallel sim runs with identical inputs produce identical hashes |

---

## Tasks

### Task 1: Define `BridgeheadAnchorClass` enum

**Why:** Foundation type for the new anchor tile-class state. All later tasks depend on this existing.

**Files:**
- Modify: `src/sim/bridge_state/mod.rs` — add the enum after `DamageState` (line 53), before `BridgeCellRole` (line 132).

**Pattern:** Mirrors the shape of the existing `DamageState` enum at [src/sim/bridge_state/mod.rs:38](../../src/sim/bridge_state/mod.rs#L38) — derives match (`Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize`), plus `Default`.

**Step 1: Add the enum**

Insert after the closing `}` of the `impl DamageState` block (after line 130, before `BridgeCellRole`):

```rust
/// Per-anchor tile-class state, mirroring the binary's `IsoTileTypeIndex`
/// writes inside the BridgeSet bridgehead-class space.
///
/// gamemd's `ProcessBridgeDamageStateMachine_High` @ 0x00576BA0 writes one of
/// four offsets (`DAT_00ABAD30 + 0..3` NS / `DAT_00AA1028 + 0..3` EW) into
/// the anchor cell's `+0x38` IsoTileTypeIndex when damage lands on a
/// bridgehead-class cell. The four offsets render as distinct tile variants:
///
/// - `Variant0` — intact anchor (map-load default).
/// - `Variant1` — intact intermediate variant; reached only via
///   `UpdateRamp_*_DamageB`'s `+0 → +1` write on a neighbor bridgehead when
///   the body is damaged.
/// - `Damaged` — the runtime "damaged" anchor tile. Reached by every
///   bridgehead direct-hit (single-step transition; no progression beyond +2).
/// - `AboutToFall` — reached only via map-load (pre-damaged maps) or the
///   body-cell collapse cascade landing on an already-`+3` cell. Never
///   reached from sustained bridgehead-direct fire.
///
/// Meaningful only when `BridgeRuntimeCell.role` is `Anchor` (the binary
/// writes the anchor's tile class) or `Bridgehead` (DamageB neighbor
/// progression). Renderer ignores it on other roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default)]
pub enum BridgeheadAnchorClass {
    #[default]
    Variant0,
    Variant1,
    Damaged,
    AboutToFall,
}
```

**Step 2: Verify compile**

Run: `cargo check -p ra2_rust_game --lib 2>&1 | head -40`

Expected: clean compile (enum is unused but valid).

**Step 3: Commit**

`git add src/sim/bridge_state/mod.rs && git commit -m "sim/bridge_state: add BridgeheadAnchorClass enum"`

---

### Task 2: Add `bridgehead_anchor_class` field to `BridgeRuntimeCell`

**Why:** The new state needs a home on the cell struct, with serde defaulting for backward compat.

**Files:**
- Modify: `src/sim/bridge_state/mod.rs:372-404` — `BridgeRuntimeCell` struct.
- Modify: `src/sim/bridge_state/mod.rs:482-497` — `from_resolved_terrain` initialization site.

**Pattern:** Mirrors the existing `damaged_variant: bool` field added in the recent G4 work — same `#[serde(default)]` would apply if there were existing saves, but per Grounding Summary there's no save-version mechanism, so a plain field is enough. We still add `#[serde(default)]` defensively so in-memory snapshots from older builds can deserialize.

**Step 1: Add the field**

In the `BridgeRuntimeCell` struct definition at [src/sim/bridge_state/mod.rs:372](../../src/sim/bridge_state/mod.rs#L372), append after the `damaged_variant: bool` field (line 403):

```rust
    /// Anchor tile-class mirror of gamemd's `IsoTileTypeIndex` (+0x38) when
    /// the cell sits in the BridgeSet bridgehead-class space. Written by the
    /// bridgehead state machine on Anchor cells, and by
    /// `UpdateRamp_*_DamageB` on Bridgehead-role neighbors. Defaults to
    /// `Variant0` at map load. Renderer reads this on Anchor cells to pick
    /// the anchor's TMP tile variant (renderer follow-up; not wired in G3).
    #[serde(default)]
    pub bridgehead_anchor_class: BridgeheadAnchorClass,
```

**Step 2: Initialize at map load**

In `BridgeRuntimeState::from_resolved_terrain` at [src/sim/bridge_state/mod.rs:482](../../src/sim/bridge_state/mod.rs#L482), inside the `BridgeRuntimeCell { ... }` literal, append after the `damaged_variant: false,` line (line 496):

```rust
                    bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
```

There are likely 2-3 places in `from_resolved_terrain` that construct `BridgeRuntimeCell` literals (one per cell-discovery path: BFS body cells, anchor cells, bridgehead cells). Search the file for `BridgeRuntimeCell {` and add the field to **every** literal site to satisfy the struct's exhaustive-field requirement.

Run: `grep -n "BridgeRuntimeCell {" src/sim/bridge_state/mod.rs`

Add the new field to each match.

**Step 3: Update any test helpers that construct `BridgeRuntimeCell`**

Run: `grep -rn "BridgeRuntimeCell {" src/`

Test helpers in [src/sim/world/world_hash.rs:626](../../src/sim/world/world_hash.rs#L626) and elsewhere will need the new field too. Add `bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,` to each construction site.

**Step 4: Verify compile + tests still pass**

Run: `cargo check -p ra2_rust_game --lib && cargo test -p ra2_rust_game --lib bridge_state -- --nocapture 2>&1 | tail -30`

Expected: clean compile, existing tests still pass (no behavior change yet).

**Step 5: Commit**

`git add -A && git commit -m "sim/bridge_state: add bridgehead_anchor_class field"`

---

### Task 3: Include `bridgehead_anchor_class` in state hash

**Why:** Determinism — the new field must contribute to the bridge-state hash for replay/lockstep correctness.

**Files:**
- Modify: `src/sim/world/world_hash.rs:213-234` — `hash_bridge_state` function.

**Pattern:** Mirrors the existing `cell.damaged_variant.hash(hasher);` line at [src/sim/world/world_hash.rs:233](../../src/sim/world/world_hash.rs#L233).

**Step 1: Add the hash call**

In `hash_bridge_state` at [src/sim/world/world_hash.rs:233](../../src/sim/world/world_hash.rs#L233), after the line `cell.damaged_variant.hash(hasher);`, append:

```rust
            cell.bridgehead_anchor_class.hash(hasher);
```

**Step 2: Add a hash-divergence test**

Append a new test to the `bridge_overlay_hash_tests` module at [src/sim/world/world_hash.rs:615](../../src/sim/world/world_hash.rs#L615), mirroring the shape of `overlay_byte_difference_changes_state_hash` at line 643:

```rust
    #[test]
    fn bridgehead_anchor_class_difference_changes_state_hash() {
        use crate::sim::bridge_state::BridgeheadAnchorClass;
        let mut sim_a = SimulationState::default();
        let mut sim_b = SimulationState::default();

        let mut state_a = make_bridge_state_with_overlay(0x18);
        let mut state_b = make_bridge_state_with_overlay(0x18);
        // Mutate the same cell to a different anchor class.
        if let Some(cell) = state_a.cell_mut(2, 2) {
            cell.bridgehead_anchor_class = BridgeheadAnchorClass::Damaged;
        }
        sim_a.bridge_state = Some(state_a);
        sim_b.bridge_state = Some(state_b);

        assert_ne!(
            sim_a.compute_state_hash(),
            sim_b.compute_state_hash(),
            "bridgehead_anchor_class must contribute to state hash",
        );
    }
```

If the `make_bridge_state_with_overlay` helper doesn't place a cell at (2, 2), adjust the coordinate to whatever cell the helper does create.

**Step 3: Verify**

Run: `cargo test -p ra2_rust_game --lib bridgehead_anchor_class_difference_changes_state_hash -- --nocapture 2>&1 | tail -20`

Expected: PASS.

Also re-run all bridge tests to confirm no regression:
`cargo test -p ra2_rust_game --lib bridge -- --nocapture 2>&1 | tail -30`

Expected: all PASS.

**Step 4: Commit**

`git add -A && git commit -m "sim/world: include bridgehead_anchor_class in bridge state hash"`

---

### Task 4: Refactor `bridgehead_walk_to_anchor` — drop direction param, compute internally

**Why:** Gap 4 fix. The helper's `direction` parameter forces the caller to know the walk geometry; the binary computes it from the start-cell's `+0x11A` inside `ProcessBridgeDamageStateMachine_High`. Pushing the computation into the helper makes the helper's contract identical to the binary's literal asm.

**Files:**
- Modify: `src/sim/bridge_specs.rs:624-663` — function body.
- Modify: `src/sim/bridge_specs.rs:631-635` — function signature.

**Pattern:** Walk-direction-from-height computation matches Ghidra 0x005771d3 (NS) and 0x00576ca3 (EW). The 16-iter cap stays as an internal defensive bound (gamemd has no such cap; we keep it).

**Step 1: Rewrite the function**

Replace the entire `bridgehead_walk_to_anchor` function at [src/sim/bridge_specs.rs:631-663](../../src/sim/bridge_specs.rs#L631-L663) with:

```rust
/// Walk from a bridgehead cell to its anchor body cell. Returns the anchor
/// cell coord, or `None` if the start cell fails the per-axis parity / upper-
/// bound gate or the walk runs off the map.
///
/// Mirrors `ProcessBridgeDamageStateMachine_High` @ 0x00576BA0:
/// - **NS branch (start-cell gate):** reject `(h & 1) != 0` — odd heights
///   (h=5, h=7) absorb damage with no state change.
/// - **EW branch (start-cell gate):** reject `h > 4` — high-ramp peak (h=0xC)
///   and other oversized heights early-return.
/// - **Walk direction (NS):** `h < 4 → S`, `h == 4 → at anchor`, `h > 4 → N`.
///   Verified literal asm at 0x005771d3.
/// - **Walk direction (EW):** `h < 2 → E`, `h == 2 → at anchor`, `h > 2 → W`.
///   Verified literal asm at 0x00576ca3.
/// - **Mid-walk parity:** none. gamemd silently walks through odd-h
///   intermediates. (Previous Rust check was stricter than the binary and
///   caused damage-absorption on multi-cell ramps.)
///
/// Walk terminates when `height == target` (4 NS / 2 EW). The 16-iter cap is
/// an internal defensive bound — gamemd has no cap but bridges aren't placed
/// near map edges in practice.
///
/// `cell_height` should read `ResolvedTerrainCell.template_height` (the TMP
/// per-tile byte at offset 40, mirroring gamemd's `CellClass+0x11A`).
pub fn bridgehead_walk_to_anchor(
    start: (u16, u16),
    axis: Axis,
    cell_height: impl Fn((u16, u16)) -> Option<u8>,
    map_width: u16,
    map_height: u16,
) -> Option<(u16, u16)> {
    let target_height: u8 = match axis {
        Axis::NS => 4,
        Axis::EW => 2,
    };

    // Start-cell gate (parity check / upper-bound check).
    let start_h = cell_height(start)?;
    match axis {
        Axis::NS => {
            if start_h & 1 != 0 {
                return None;
            }
        }
        Axis::EW => {
            if start_h > 4 {
                return None;
            }
        }
    }
    if start_h == target_height {
        return Some(start);
    }

    let mut current = start;
    let mut h = start_h;
    for _ in 0..16 {
        // Compute walk direction from current h. Per binary, direction is
        // recomputed every iteration; height converges monotonically so the
        // direction never flips in practice but the recompute matches asm.
        let dir = match axis {
            Axis::NS => {
                if h < 4 {
                    Direction::S
                } else {
                    // h > 4 (h == 4 handled by the target-equality check below)
                    Direction::N
                }
            }
            Axis::EW => {
                if h < 2 {
                    Direction::E
                } else {
                    // h > 2 (h == 2 handled by the target-equality check below)
                    Direction::W
                }
            }
        };
        let (dx, dy) = dir.offset();
        let nx = current.0 as i32 + dx;
        let ny = current.1 as i32 + dy;
        if nx < 0 || ny < 0 || nx as u16 >= map_width || ny as u16 >= map_height {
            return None;
        }
        current = (nx as u16, ny as u16);
        h = cell_height(current)?;
        // No mid-walk parity check — gamemd walks through odd intermediates.
        if h == target_height {
            return Some(current);
        }
    }
    None
}
```

**Step 2: Verify compile (caller still needs updating; expect a compile error)**

Run: `cargo check -p ra2_rust_game --lib 2>&1 | tail -20`

Expected: compile error at [src/sim/bridge_state/mod.rs:1142](../../src/sim/bridge_state/mod.rs#L1142) — the existing call to `bridgehead_walk_to_anchor` passes the now-removed `walk_dir` parameter. This is intentional; the call site is rewritten in Task 9.

**Step 3: Do NOT commit yet**

The function is in a half-rewritten state — the caller breaks. The next two tasks land the test coverage and the caller rewrite atomically.

---

### Task 5: Unit tests for the refactored `bridgehead_walk_to_anchor`

**Why:** Tasks 1-4 only added types. Task 5 is the first task that actually verifies behavior matches the binary's literal asm. These tests are parity-critical (every walk-direction boundary is covered).

**Files:**
- Modify: `src/sim/bridge_specs.rs` — append to the existing `#[cfg(test)] mod tests` block. Find the closing `}` of `mod tests` (look for the last `#[test]` plus closing braces near end of file).

**Pattern:** Mirrors the existing `bridgehead_walk_to_anchor_*` tests if any exist (`grep -n "bridgehead_walk_to_anchor" src/sim/bridge_specs.rs`); otherwise mirror the existing `bridgehead_blow_up_row_*` tests in the same module.

**Step 1: Locate the tests module + delete old walker tests if any**

Run: `grep -n "fn bridgehead_walk_to_anchor\|mod tests {" src/sim/bridge_specs.rs`

If existing tests reference the dropped `direction` parameter (signature mismatch), delete them — they're replaced by the new tests below.

**Step 2: Add the new tests**

Append to the `tests` module:

```rust
    /// Helper: build a height-lookup from a small (X, Y) → height map.
    fn height_lookup_from(
        cells: &'static [((u16, u16), u8)],
    ) -> impl Fn((u16, u16)) -> Option<u8> + 'static {
        move |pos: (u16, u16)| {
            cells
                .iter()
                .find_map(|&(p, h)| if p == pos { Some(h) } else { None })
        }
    }

    #[test]
    fn bridgehead_walk_ns_odd_height_returns_none() {
        // h=5 (NS ramp) → start-cell parity gate fires.
        let lookup = height_lookup_from(&[((5, 5), 5)]);
        let out = bridgehead_walk_to_anchor((5, 5), Axis::NS, lookup, 32, 32);
        assert!(out.is_none(), "h=5 must return None (parity gate)");
    }

    #[test]
    fn bridgehead_walk_ns_h7_returns_none() {
        // h=7 (NS bridgehead variant) → also odd, also gated.
        let lookup = height_lookup_from(&[((5, 5), 7)]);
        let out = bridgehead_walk_to_anchor((5, 5), Axis::NS, lookup, 32, 32);
        assert!(out.is_none(), "h=7 must return None (parity gate)");
    }

    #[test]
    fn bridgehead_walk_ns_h8_walks_north() {
        // h=8 (high-ramp peak) → walks N (decreasing Y) → finds h=4 anchor.
        // Layout: (5, 5)=8, (5, 4)=4.
        let lookup = height_lookup_from(&[((5, 5), 8), ((5, 4), 4)]);
        let out = bridgehead_walk_to_anchor((5, 5), Axis::NS, lookup, 32, 32);
        assert_eq!(out, Some((5, 4)), "h=8 must walk N and find anchor");
    }

    #[test]
    fn bridgehead_walk_ns_h0_walks_south() {
        // h=0 (hypothetical, but tests the h<4 branch) → walks S (+Y).
        // Layout: (5, 5)=0, (5, 6)=4.
        let lookup = height_lookup_from(&[((5, 5), 0), ((5, 6), 4)]);
        let out = bridgehead_walk_to_anchor((5, 5), Axis::NS, lookup, 32, 32);
        assert_eq!(out, Some((5, 6)), "h=0 must walk S and find anchor");
    }

    #[test]
    fn bridgehead_walk_ns_h4_returns_start() {
        // h=4 is already at the anchor — return immediately.
        let lookup = height_lookup_from(&[((5, 5), 4)]);
        let out = bridgehead_walk_to_anchor((5, 5), Axis::NS, lookup, 32, 32);
        assert_eq!(out, Some((5, 5)), "h=4 must return start (already at anchor)");
    }

    #[test]
    fn bridgehead_walk_ns_walks_through_odd_intermediate() {
        // Parity divergence fix: gamemd walks through odd-h intermediates.
        // Layout: (5, 5)=8 → walks N → (5, 4)=5 (odd!) → walks N → (5, 3)=4 anchor.
        let lookup = height_lookup_from(&[((5, 5), 8), ((5, 4), 5), ((5, 3), 4)]);
        let out = bridgehead_walk_to_anchor((5, 5), Axis::NS, lookup, 32, 32);
        assert_eq!(
            out,
            Some((5, 3)),
            "walk must pass through odd-h intermediates and reach anchor",
        );
    }

    #[test]
    fn bridgehead_walk_ew_h_gt_4_returns_none() {
        // h=0xC (EW high-ramp peak) → upper-bound gate fires.
        let lookup = height_lookup_from(&[((5, 5), 0x0C)]);
        let out = bridgehead_walk_to_anchor((5, 5), Axis::EW, lookup, 32, 32);
        assert!(out.is_none(), "h=0xC EW must return None (upper-bound gate)");
    }

    #[test]
    fn bridgehead_walk_ew_h0_walks_east() {
        // h=0 → walks E (+X) → finds h=2 anchor.
        // Layout: (5, 5)=0, (6, 5)=2.
        let lookup = height_lookup_from(&[((5, 5), 0), ((6, 5), 2)]);
        let out = bridgehead_walk_to_anchor((5, 5), Axis::EW, lookup, 32, 32);
        assert_eq!(out, Some((6, 5)), "EW h=0 must walk E and find anchor");
    }

    #[test]
    fn bridgehead_walk_ew_h4_walks_west() {
        // h=4 (EW, > 2 but ≤ 4) → walks W (-X) → finds h=2 anchor.
        // Layout: (5, 5)=4, (4, 5)=2.
        let lookup = height_lookup_from(&[((5, 5), 4), ((4, 5), 2)]);
        let out = bridgehead_walk_to_anchor((5, 5), Axis::EW, lookup, 32, 32);
        assert_eq!(out, Some((4, 5)), "EW h=4 must walk W and find anchor");
    }

    #[test]
    fn bridgehead_walk_ew_h2_returns_start() {
        // h=2 is at the anchor — return immediately.
        let lookup = height_lookup_from(&[((5, 5), 2)]);
        let out = bridgehead_walk_to_anchor((5, 5), Axis::EW, lookup, 32, 32);
        assert_eq!(out, Some((5, 5)), "EW h=2 must return start");
    }

    #[test]
    fn bridgehead_walk_off_map_returns_none() {
        // Start at (0, 0) h=8, walk N → off map → None.
        let lookup = height_lookup_from(&[((0, 0), 8)]);
        let out = bridgehead_walk_to_anchor((0, 0), Axis::NS, lookup, 32, 32);
        assert!(out.is_none(), "off-map walk must return None");
    }
```

**Step 3: Verify (still expect callsite error)**

Run: `cargo test -p ra2_rust_game --lib bridgehead_walk -- --nocapture 2>&1 | tail -60`

Expected: the test module compiles (it doesn't use the old caller signature). But `cargo check --lib` still fails at the call site. That's intentional.

If tests don't compile because the closing brace of `mod tests` is in the wrong place, fix by relocating the new tests just above `mod tests`'s closing `}`.

**Step 4: Run only this test file's tests**

Run: `cargo test -p ra2_rust_game --lib --no-fail-fast bridge_specs::tests::bridgehead_walk 2>&1 | tail -40`

Expected: 11 tests PASS.

**Step 5: Do NOT commit yet** — caller still broken.

---

### Task 6: Extend `update_ramp_perpendicular` — accept Bridgehead targets + tile-class write

**Why:** Tile-class write is ledger item 13 — the only mechanism showing the `+1` intermediate variant on neighbor bridgeheads when the body is damaged. Composes with the existing state-byte branch (Anchor targets) and the existing `apply_damaged_variant_flood_fill` call (recent G4 work).

**Files:**
- Modify: `src/sim/bridge_specs.rs:533-616` — function body.

**Pattern:** Three branches inside the helper, each gated by target role:
- Anchor: state-byte bump (existing) + tile-class write (new) + flood-fill (existing).
- Bridgehead: tile-class write only (no state-byte bump — bridgeheads don't have `+0x140 & 0x80`).
- Other: no-op (existing).

The asymmetric A/B tile-class table is sourced from Ghidra 0x00572230 (DamageA preserves), 0x00572330 (DamageB progresses), 0x00572440 / 0x005727E0 (Collapse variants).

**Step 1: Rewrite the function body**

Replace the body of `update_ramp_perpendicular` ([src/sim/bridge_specs.rs:533-616](../../src/sim/bridge_specs.rs#L533-L616)) with:

```rust
pub fn update_ramp_perpendicular(
    state: &mut BridgeRuntimeState,
    anchor_pos: (u16, u16),
    axis: Axis,
    phase: Phase,
    _is_high_bridge: bool,
    terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid,
) -> RampOutcome {
    let dir = perpendicular_direction(axis, phase);
    let (dx, dy) = dir.offset();
    let target_x = anchor_pos.0 as i32 + dx;
    let target_y = anchor_pos.1 as i32 + dy;
    if target_x < 0 || target_y < 0 {
        return RampOutcome {
            state_changed: false,
        };
    }
    let target_pos = (target_x as u16, target_y as u16);

    let Some(target_cell) = state.cell(target_pos.0, target_pos.1).copied() else {
        return RampOutcome {
            state_changed: false,
        };
    };
    let target_role = target_cell.role;

    use crate::sim::bridge_state::{BridgeCellRole, BridgeheadAnchorClass};

    // Per-target-role branching mirrors the binary's UpdateRamp_*_High:
    // - Anchor (target.+0x140 & 0x80): state-byte bump + tile-class write.
    // - Bridgehead (no 0x80 flag): tile-class write only.
    // - Other: no-op.
    let mut state_byte_changed = false;
    let mut tile_class_changed = false;

    match target_role {
        BridgeCellRole::Anchor => {
            // Existing state-byte branch.
            let Some(target_axis) = target_cell.axis else {
                return RampOutcome {
                    state_changed: false,
                };
            };
            let current_byte = target_cell.damage_state.to_state_byte(target_axis);
            if let Some(next_byte) = apply_ramp_transition(current_byte, axis, phase) {
                let next_state = if next_byte == 0 {
                    DamageState::Destroyed
                } else {
                    match DamageState::from_state_byte(next_byte) {
                        Some(s) => s,
                        None => DamageState::Destroyed,
                    }
                };
                if let Some(cell_mut) = state.cell_mut(target_pos.0, target_pos.1) {
                    cell_mut.damage_state = next_state;
                    state_byte_changed = true;
                }
            }

            // New tile-class branch on Anchor targets (Anchor cells can also
            // carry a bridgehead_anchor_class — the binary writes both
            // anchor's +0x11E and anchor's +0x38 from the body-cell driver).
            let new_class = apply_anchor_class_transition(
                target_cell.bridgehead_anchor_class,
                phase,
            );
            if new_class != target_cell.bridgehead_anchor_class {
                if let Some(cell_mut) = state.cell_mut(target_pos.0, target_pos.1) {
                    cell_mut.bridgehead_anchor_class = new_class;
                    tile_class_changed = true;
                }
            }
        }
        BridgeCellRole::Bridgehead => {
            // Bridgehead targets: tile-class write only, no state-byte bump.
            // Mirrors UpdateRamp_NS_DamageB_High @ 0x00572330 writing
            // ABAD30 → ABAD30+1 on neighbor bridgeheads.
            let new_class = apply_anchor_class_transition(
                target_cell.bridgehead_anchor_class,
                phase,
            );
            if new_class != target_cell.bridgehead_anchor_class {
                if let Some(cell_mut) = state.cell_mut(target_pos.0, target_pos.1) {
                    cell_mut.bridgehead_anchor_class = new_class;
                    tile_class_changed = true;
                }
            }
        }
        _ => {
            return RampOutcome {
                state_changed: false,
            };
        }
    }

    // Existing flood-fill: fires on any successful state-byte write on an
    // Anchor target. Preserves G4 commit 1c9b63e behavior.
    if state_byte_changed {
        let _ = state.apply_damaged_variant_flood_fill(target_pos.0, target_pos.1, true, terrain);
    }

    RampOutcome {
        state_changed: state_byte_changed || tile_class_changed,
    }
}

/// Asymmetric tile-class transition table for `update_ramp_perpendicular`.
/// Mirrors the binary's UpdateRamp helpers (HIGH §11.1):
/// - DamageA (Ghidra 0x00572230 / 0x00572B80): preserves Variant0 and
///   Damaged; no-op on Variant1 and AboutToFall.
/// - DamageB (Ghidra 0x00572330 / 0x00572C90): progresses Variant0 → Variant1
///   and Variant1 → Damaged; no-op on Damaged and AboutToFall.
/// - CollapseA / CollapseB (Ghidra 0x00572440 / 0x005727E0 / 0x00572DA0 /
///   0x00573170): advance Variant0/Variant1/Damaged to Damaged; preserve
///   AboutToFall (the recursive +3 → +3 write in the binary is a no-op
///   semantically and we keep AboutToFall as the latched state).
fn apply_anchor_class_transition(
    current: crate::sim::bridge_state::BridgeheadAnchorClass,
    phase: Phase,
) -> crate::sim::bridge_state::BridgeheadAnchorClass {
    use crate::sim::bridge_state::BridgeheadAnchorClass as BC;
    match (current, phase) {
        // DamageA: preserve Variant0 and Damaged; no-op others.
        (BC::Variant0, Phase::DamageA) => BC::Variant0,
        (BC::Damaged, Phase::DamageA) => BC::Damaged,
        (BC::Variant1, Phase::DamageA) => BC::Variant1,
        (BC::AboutToFall, Phase::DamageA) => BC::AboutToFall,

        // DamageB: progress Variant0 → Variant1, Variant1 → Damaged.
        (BC::Variant0, Phase::DamageB) => BC::Variant1,
        (BC::Variant1, Phase::DamageB) => BC::Damaged,
        (BC::Damaged, Phase::DamageB) => BC::Damaged,
        (BC::AboutToFall, Phase::DamageB) => BC::AboutToFall,

        // CollapseA / CollapseB: advance to Damaged, preserve AboutToFall.
        (BC::Variant0, Phase::CollapseA | Phase::CollapseB) => BC::Damaged,
        (BC::Variant1, Phase::CollapseA | Phase::CollapseB) => BC::Damaged,
        (BC::Damaged, Phase::CollapseA | Phase::CollapseB) => BC::Damaged,
        (BC::AboutToFall, Phase::CollapseA | Phase::CollapseB) => BC::AboutToFall,
    }
}
```

**Step 2: Update the function's top-level docstring**

Replace the existing docstring (lines 524-532) with:

```rust
/// Walk one perpendicular cell from `anchor_pos` and fire the appropriate
/// `UpdateRamp_*_High` side effects based on target role.
///
/// Mirrors the per-cell side effects of binary `UpdateRamp_NS_DamageA_High @
/// 0x00572230` and peers (HIGH §11.1).
///
/// - **Anchor target** (target has `+0x140 & 0x80` in binary): state-byte
///   transition on `+0x11E` (existing `apply_ramp_transition` logic) +
///   tile-class transition on the new `bridgehead_anchor_class` field
///   (asymmetric A/B per `apply_anchor_class_transition`). Also fires
///   `apply_damaged_variant_flood_fill` after a successful state-byte write.
/// - **Bridgehead target** (no `+0x80` flag): tile-class transition only.
///   This is the mechanism by which the body-damage cascade shows the `+1`
///   intermediate variant on neighbor bridgeheads (verified DamageB on
///   `0x00572330`).
/// - **Other roles**: no-op.
///
/// `is_high_bridge` is unused (state transitions are identical for HIGH
/// and LOW per HIGH §11.1).
```

**Step 3: Verify compile (call site at bridgehead_advance_state still broken; ignore)**

Run: `cargo check -p ra2_rust_game --lib 2>&1 | tail -20`

Expected: compile error still at `bridgehead_advance_state` call site. The new helper code compiles standalone.

**Step 4: Do NOT commit yet.**

---

### Task 7: Unit tests for the tile-class write branch

**Why:** Lock down the asymmetric A/B transition table. Parity-critical because the `+1 → +2` DamageB progression is the only way intermediate variants appear in the game.

**Files:**
- Modify: `src/sim/bridge_specs.rs` — append to `#[cfg(test)] mod tests`.

**Pattern:** Mirrors the existing `update_ramp_perpendicular_*` tests in the same module (if any). Each test constructs a small `BridgeRuntimeState`, places an Anchor or Bridgehead target cell, fires the helper, and asserts the resulting state.

**Step 1: Find/borrow an existing helper for building `BridgeRuntimeState`**

Run: `grep -n "fn build_test_bridge\|fn make_bridge\|BridgeRuntimeState::default\|test_bridge_state" src/sim/bridge_specs.rs src/sim/bridge_state/mod.rs | head -10`

There should be a small builder used by existing tests. If not, the unit tests can use `BridgeRuntimeState::default()` and `state.set_cell(...)` (or whatever the API is — look for existing patterns in [src/sim/bridge_state/mod.rs:2466](../../src/sim/bridge_state/mod.rs#L2466) onward for the bridgehead_advance tests, which build small states inline).

**Step 2: Add the tests**

Append to the `tests` module:

```rust
    /// Build a minimal BridgeRuntimeState with one anchor at (2, 2) and a
    /// perpendicular neighbor at the given position with the given role.
    fn make_state_with_neighbor(
        neighbor_pos: (u16, u16),
        neighbor_role: crate::sim::bridge_state::BridgeCellRole,
        neighbor_class: crate::sim::bridge_state::BridgeheadAnchorClass,
        axis: Axis,
    ) -> (
        crate::sim::bridge_state::BridgeRuntimeState,
        crate::map::resolved_terrain::ResolvedTerrainGrid,
    ) {
        use crate::sim::bridge_state::{BridgeRuntimeCell, BridgeRuntimeState, DamageState};
        // Use the existing inline test-state builder pattern from
        // src/sim/bridge_state/mod.rs:2466+. Replicate enough of it here:
        let terrain = crate::map::resolved_terrain::ResolvedTerrainGrid::empty(32, 32);
        let mut state = BridgeRuntimeState::with_dims(32, 32);
        // Place anchor at (2, 2).
        state.set_cell(
            2,
            2,
            Some(BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 0,
                bridge_group_id: Some(1),
                damage_state: DamageState::Healthy { variant: 0 },
                axis: Some(axis),
                role: crate::sim::bridge_state::BridgeCellRole::Anchor,
                anchor_span_id: Some(1),
                overlay_byte: 0,
                damaged_variant: false,
                bridgehead_anchor_class: neighbor_class,
            }),
        );
        // Place neighbor at the requested perpendicular position.
        state.set_cell(
            neighbor_pos.0,
            neighbor_pos.1,
            Some(BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 0,
                bridge_group_id: Some(1),
                damage_state: DamageState::Healthy { variant: 0 },
                axis: Some(axis),
                role: neighbor_role,
                anchor_span_id: if matches!(neighbor_role, crate::sim::bridge_state::BridgeCellRole::Anchor) {
                    Some(1)
                } else {
                    None
                },
                overlay_byte: 0,
                damaged_variant: false,
                bridgehead_anchor_class: neighbor_class,
            }),
        );
        (state, terrain)
    }

    #[test]
    fn update_ramp_perpendicular_damageb_progresses_bridgehead_variant0_to_variant1() {
        use crate::sim::bridge_state::{BridgeCellRole, BridgeheadAnchorClass};
        // NS axis: DamageB walks W (-1, 0) from anchor (2, 2) → neighbor at (1, 2).
        let (mut state, terrain) = make_state_with_neighbor(
            (1, 2),
            BridgeCellRole::Bridgehead,
            BridgeheadAnchorClass::Variant0,
            Axis::NS,
        );
        let outcome = update_ramp_perpendicular(
            &mut state,
            (2, 2),
            Axis::NS,
            Phase::DamageB,
            true,
            &terrain,
        );
        assert!(outcome.state_changed);
        let neighbor = state.cell(1, 2).unwrap();
        assert_eq!(
            neighbor.bridgehead_anchor_class,
            BridgeheadAnchorClass::Variant1,
            "DamageB on Variant0 bridgehead must progress to Variant1",
        );
        // damage_state must NOT be modified on Bridgehead targets.
        assert!(matches!(
            neighbor.damage_state,
            crate::sim::bridge_state::DamageState::Healthy { .. }
        ));
    }

    #[test]
    fn update_ramp_perpendicular_damageb_progresses_variant1_to_damaged() {
        use crate::sim::bridge_state::{BridgeCellRole, BridgeheadAnchorClass};
        let (mut state, terrain) = make_state_with_neighbor(
            (1, 2),
            BridgeCellRole::Bridgehead,
            BridgeheadAnchorClass::Variant1,
            Axis::NS,
        );
        let outcome = update_ramp_perpendicular(
            &mut state,
            (2, 2),
            Axis::NS,
            Phase::DamageB,
            true,
            &terrain,
        );
        assert!(outcome.state_changed);
        let neighbor = state.cell(1, 2).unwrap();
        assert_eq!(neighbor.bridgehead_anchor_class, BridgeheadAnchorClass::Damaged);
    }

    #[test]
    fn update_ramp_perpendicular_damagea_preserves_bridgehead_variant0() {
        use crate::sim::bridge_state::{BridgeCellRole, BridgeheadAnchorClass};
        // NS axis: DamageA walks E (+1, 0) → neighbor at (3, 2).
        let (mut state, terrain) = make_state_with_neighbor(
            (3, 2),
            BridgeCellRole::Bridgehead,
            BridgeheadAnchorClass::Variant0,
            Axis::NS,
        );
        let outcome = update_ramp_perpendicular(
            &mut state,
            (2, 2),
            Axis::NS,
            Phase::DamageA,
            true,
            &terrain,
        );
        // DamageA preserves Variant0 → no change → state_changed=false.
        assert!(!outcome.state_changed);
        let neighbor = state.cell(3, 2).unwrap();
        assert_eq!(neighbor.bridgehead_anchor_class, BridgeheadAnchorClass::Variant0);
    }

    #[test]
    fn update_ramp_perpendicular_damagea_preserves_bridgehead_damaged() {
        use crate::sim::bridge_state::{BridgeCellRole, BridgeheadAnchorClass};
        let (mut state, terrain) = make_state_with_neighbor(
            (3, 2),
            BridgeCellRole::Bridgehead,
            BridgeheadAnchorClass::Damaged,
            Axis::NS,
        );
        let outcome = update_ramp_perpendicular(
            &mut state,
            (2, 2),
            Axis::NS,
            Phase::DamageA,
            true,
            &terrain,
        );
        assert!(!outcome.state_changed);
        let neighbor = state.cell(3, 2).unwrap();
        assert_eq!(neighbor.bridgehead_anchor_class, BridgeheadAnchorClass::Damaged);
    }

    #[test]
    fn update_ramp_perpendicular_other_role_is_noop() {
        use crate::sim::bridge_state::{BridgeCellRole, BridgeheadAnchorClass};
        // Body role is not Anchor and not Bridgehead — should be a no-op.
        let (mut state, terrain) = make_state_with_neighbor(
            (1, 2),
            BridgeCellRole::Body,
            BridgeheadAnchorClass::Variant0,
            Axis::NS,
        );
        let outcome = update_ramp_perpendicular(
            &mut state,
            (2, 2),
            Axis::NS,
            Phase::DamageB,
            true,
            &terrain,
        );
        assert!(!outcome.state_changed);
        let neighbor = state.cell(1, 2).unwrap();
        assert_eq!(neighbor.bridgehead_anchor_class, BridgeheadAnchorClass::Variant0);
    }
```

**Step 3: Verify**

Run: `cargo test -p ra2_rust_game --lib --no-fail-fast update_ramp_perpendicular -- --nocapture 2>&1 | tail -40`

Expected: 5 new tests PASS. Existing `update_ramp_perpendicular_*` tests (if any) should also pass — they were exercising the Anchor-only code path which is preserved.

If the test helper API differs from what's assumed (`with_dims`, `set_cell`), adjust to match the actual API by reading [src/sim/bridge_state/mod.rs:422-440](../../src/sim/bridge_state/mod.rs#L422) for `BridgeRuntimeState` constructor + accessors.

**Step 4: Do NOT commit yet** — caller still broken.

---

### Task 8: Rewrite `bridgehead_advance_state` body

**Why:** The single load-bearing fix for Gaps 1, 2, 3 from the fidelity check. After this task, `cargo check` is green again.

**Files:**
- Modify: `src/sim/bridge_state/mod.rs:1111-1230` — function body.

**Pattern:** Same skeleton as `body_cell_advance_state` ([src/sim/bridge_state/mod.rs:797](../../src/sim/bridge_state/mod.rs#L797)): resolve input cell → role/axis filters → walk → write anchor state → fire perpendiculars → return `Absorbed`. Remove the wrong `Damaged → Destroyed` branch entirely.

**Step 1: Rewrite the function body**

Replace the body of `bridgehead_advance_state` ([src/sim/bridge_state/mod.rs:1111-1230](../../src/sim/bridge_state/mod.rs#L1111)) with:

```rust
    pub fn bridgehead_advance_state(
        &mut self,
        rx: u16,
        ry: u16,
        is_high_bridge: bool,
        terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid,
    ) -> StateOutcome {
        // 1. Resolve input cell.
        let Some(input_cell) = self.cell(rx, ry).copied() else {
            return StateOutcome::NoChange;
        };

        // 2. Filter: must be Bridgehead role. Body/Anchor/Tail route through
        //    body_cell_advance_state.
        if !matches!(input_cell.role, BridgeCellRole::Bridgehead) {
            return StateOutcome::NoChange;
        }
        let Some(axis) = input_cell.axis else {
            return StateOutcome::NoChange;
        };

        // 3. Walk to anchor. The helper computes walk direction internally
        //    per Ghidra 0x005771d3 (NS) / 0x00576ca3 (EW). Start-cell parity
        //    / upper-bound gates fire first (NS: h&1; EW: h>4) and return
        //    None — that's how gamemd absorbs damage on most ramp cells
        //    without state change.
        let map_w = self.width;
        let map_h = self.height;
        let height_lookup = |pos: (u16, u16)| -> Option<u8> {
            terrain.cell(pos.0, pos.1).map(|c| c.template_height)
        };
        let Some(anchor_pos) = crate::sim::bridge_specs::bridgehead_walk_to_anchor(
            (rx, ry),
            axis,
            height_lookup,
            map_w,
            map_h,
        ) else {
            return StateOutcome::NoChange;
        };

        // 4. Write the anchor's bridgehead_anchor_class to Damaged. This is
        //    the single observable transition on the bridgehead-direct path.
        //    Mirrors gamemd's SetOverlayAndPropagate(anchor, ABAD30+2+BridgeSet,
        //    -1, -1, 0) at Ghidra 0x00577701. The write is idempotent on
        //    repeat hits (already Damaged → stays Damaged).
        if let Some(anchor_cell) = self.cell_mut(anchor_pos.0, anchor_pos.1) {
            anchor_cell.bridgehead_anchor_class = BridgeheadAnchorClass::Damaged;
        }

        // 5. Fire UpdateRamp_*_DamageA + DamageB on perpendicular neighbors
        //    of the anchor. These now also do the asymmetric tile-class
        //    progression on Bridgehead-role neighbors.
        let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
            self,
            anchor_pos,
            axis,
            Phase::DamageA,
            is_high_bridge,
            terrain,
        );
        let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
            self,
            anchor_pos,
            axis,
            Phase::DamageB,
            is_high_bridge,
            terrain,
        );

        // 6. Return Absorbed. gamemd's state machine returns 0 on every
        //    bridgehead-direct outcome except the +3 collapse branch, which
        //    is unreachable from sustained direct fire (ledger item 15).
        //    Note: bridgehead cell's own damage_state is NEVER modified on
        //    this path — gamemd writes to anchor.+0x38, not hit-cell.
        StateOutcome::Absorbed
    }
```

**Step 2: Update the function's top-level docstring**

Replace the existing docstring (lines 1071-1110) with:

```rust
    /// Bridgehead-cell state-machine driver. Mirrors the bridgehead branch
    /// of binary `ProcessBridgeDamageStateMachine_High @ 0x00576BA0`.
    ///
    /// Sparse-by-design: gamemd absorbs damage on most bridgehead cells via
    /// the per-axis start-cell gate (NS: `+0x11A & 1`; EW: `+0x11A > 4`).
    /// Only one cell type per axis reaches the anchor-write path (NS high-
    /// ramp peak h=8; EW h in {0, 1, 3, 4}).
    ///
    /// On a successful walk: writes the anchor cell's
    /// `bridgehead_anchor_class = Damaged` (mirroring gamemd's
    /// `SetOverlayAndPropagate(anchor, ABAD30+2+BridgeSet, …)` write to
    /// anchor's `+0x38` IsoTileTypeIndex) and fires `UpdateRamp_*_DamageA`
    /// + `_DamageB` on the anchor's perpendicular neighbors. The hit
    /// bridgehead cell's own `damage_state` is NEVER modified on this path.
    ///
    /// Returns:
    /// - `StateOutcome::Absorbed` on a successful walk + anchor write.
    /// - `StateOutcome::NoChange` on role mismatch, missing axis, parity-
    ///   gated start cell, or walk-off-map.
    /// - **Never** returns `Collapsed`. Sustained bridgehead direct fire
    ///   cannot collapse a bridge in gamemd; collapse requires the body-
    ///   cell cascade or BridgeRepairHut DelayKill death.
    ///
    /// `is_high_bridge` is currently unused (HIGH and LOW state transitions
    /// are identical per HIGH §11.1) but kept for API symmetry with the
    /// body driver.
```

**Step 3: Verify whole-crate compile is clean**

Run: `cargo check -p ra2_rust_game --lib 2>&1 | tail -20`

Expected: clean compile.

**Step 4: Run the existing bridge tests to see what breaks**

Run: `cargo test -p ra2_rust_game --lib bridgehead_advance -- --nocapture 2>&1 | tail -40`

Expected: tests `bridgehead_advance_healthy_to_damaged_ns` and `bridgehead_advance_damaged_to_destroyed_ns` FAIL because their assertions encode the wrong-cell-mutation contract. They'll be rewritten in Task 9.

Other tests (`bridgehead_advance_destroyed_no_change`, `bridgehead_advance_non_bridgehead_role_no_change`, `bridgehead_advance_anchor_walk_failure_no_change`, `bridgehead_advance_partial_collapse_states_no_change`, `bridgehead_advance_off_map_no_change`) should still PASS.

**Step 5: Do NOT commit yet** — failing tests in tree.

---

### Task 9: Rewrite existing `bridgehead_advance_state` tests + add new coverage

**Why:** Two existing tests encode the wrong contract and must be replaced. Adds the new tests for the corrected behavior. Idempotency, no-collapse, mid-walk tolerance.

**Files:**
- Modify: `src/sim/bridge_state/mod.rs:2466-2600` — existing test block.

**Pattern:** Mirrors the existing test layout (each test builds its own small `BridgeRuntimeState` + `ResolvedTerrainGrid` and calls the driver, asserts on state outcome + cell mutations).

**Step 1: Read the existing test bodies**

Run: `sed -n '2460,2600p' src/sim/bridge_state/mod.rs` (or use `Read` with offset=2460, limit=140).

Identify the helper(s) the existing tests use to construct a small bridge state with an anchor + bridgehead and an associated `ResolvedTerrainGrid` with `template_height` values. Mirror that helper for the new tests.

**Step 2: Delete `bridgehead_advance_damaged_to_destroyed_ns`**

The `Damaged → Destroyed` branch no longer exists. Delete the entire test (lines 2490 onward, up to the next `#[test]`).

**Step 3: Rewrite `bridgehead_advance_healthy_to_damaged_ns`**

Replace the existing test body to assert the **new** contract: the bridgehead's own `damage_state` does NOT change; the anchor's `bridgehead_anchor_class` becomes `Damaged`.

```rust
    #[test]
    fn bridgehead_advance_first_hit_writes_anchor_damaged() {
        let (mut state, terrain) = build_ns_bridge_with_h8_bridgehead_at_2_2_anchor_at_2_4();
        // Capture pre-hit state.
        let pre_hit_bridgehead = state.cell(2, 2).copied().unwrap();
        let pre_hit_anchor = state.cell(2, 4).copied().unwrap();
        assert!(matches!(
            pre_hit_anchor.bridgehead_anchor_class,
            BridgeheadAnchorClass::Variant0
        ));

        let outcome = state.bridgehead_advance_state(2, 2, true, &terrain);
        assert!(matches!(outcome, StateOutcome::Absorbed));

        // Bridgehead's own damage_state unchanged.
        let post_bridgehead = state.cell(2, 2).copied().unwrap();
        assert_eq!(post_bridgehead.damage_state, pre_hit_bridgehead.damage_state);

        // Anchor's bridgehead_anchor_class is Damaged.
        let post_anchor = state.cell(2, 4).copied().unwrap();
        assert_eq!(post_anchor.bridgehead_anchor_class, BridgeheadAnchorClass::Damaged);
    }
```

The helper `build_ns_bridge_with_h8_bridgehead_at_2_2_anchor_at_2_4` is new — write it next to the existing test helpers. It should construct a `ResolvedTerrainGrid` where (2, 2) has `template_height = 8` and (2, 3), (2, 4) have heights that lead the NS walk N to land on (2, 4) with `template_height = 4`. Sample skeleton:

```rust
    fn build_ns_bridge_with_h8_bridgehead_at_2_2_anchor_at_2_4(
    ) -> (BridgeRuntimeState, crate::map::resolved_terrain::ResolvedTerrainGrid) {
        let mut terrain = crate::map::resolved_terrain::ResolvedTerrainGrid::empty(32, 32);
        // Set heights along the walk path: (2,2)=8, (2,3)=6, (2,4)=4.
        // (Choose values that don't trigger the start-cell parity gate;
        // walk N from h=8 reads (2,3) then (2,2-2)=(2,2)=... wait — let's
        // use (2,3)=6 to confirm walk passes through. Both 8 and 6 are
        // even and > 4 so walk continues N.)
        terrain.set_cell_template_height(2, 2, 8);
        terrain.set_cell_template_height(2, 3, 6);
        terrain.set_cell_template_height(2, 4, 4);

        let mut state = BridgeRuntimeState::with_dims(32, 32);
        // Place bridgehead at (2, 2) and anchor at (2, 4).
        state.set_cell(2, 2, Some(BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 0,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::NS),
            role: BridgeCellRole::Bridgehead,
            anchor_span_id: None,
            overlay_byte: 0,
            damaged_variant: false,
            bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
        }));
        state.set_cell(2, 4, Some(BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 0,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::NS),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(1),
            overlay_byte: 0,
            damaged_variant: false,
            bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
        }));
        // Insert an AnchorSpan for span_id 1 with anchor at (2, 4).
        // (Mirror whatever existing tests do here — likely a state method
        // or direct BTreeMap insert.)
        // ...
        (state, terrain)
    }
```

If `ResolvedTerrainGrid::empty` doesn't exist with that signature, find the actual test-builder pattern in [src/sim/bridge_state/mod.rs:2466-2600](../../src/sim/bridge_state/mod.rs#L2466) — there will be precedent.

**Step 4: Add idempotency test**

```rust
    #[test]
    fn bridgehead_advance_repeat_hits_stay_damaged_no_collapse() {
        let (mut state, terrain) = build_ns_bridge_with_h8_bridgehead_at_2_2_anchor_at_2_4();
        // Hit 100 times.
        for _ in 0..100 {
            let outcome = state.bridgehead_advance_state(2, 2, true, &terrain);
            assert!(
                matches!(outcome, StateOutcome::Absorbed),
                "every hit must return Absorbed, never Collapsed",
            );
        }
        let post_anchor = state.cell(2, 4).copied().unwrap();
        assert_eq!(post_anchor.bridgehead_anchor_class, BridgeheadAnchorClass::Damaged);
        let post_bridgehead = state.cell(2, 2).copied().unwrap();
        assert!(matches!(post_bridgehead.damage_state, DamageState::Healthy { .. }));
    }
```

**Step 5: Add parity-gate tests**

```rust
    #[test]
    fn bridgehead_advance_odd_h_ns_absorbs_with_no_change() {
        // Bridgehead at h=5 (NS ramp): parity gate fires, NoChange returned.
        let mut terrain = crate::map::resolved_terrain::ResolvedTerrainGrid::empty(32, 32);
        terrain.set_cell_template_height(2, 2, 5);
        let mut state = BridgeRuntimeState::with_dims(32, 32);
        state.set_cell(2, 2, Some(BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 0,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::NS),
            role: BridgeCellRole::Bridgehead,
            anchor_span_id: None,
            overlay_byte: 0,
            damaged_variant: false,
            bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
        }));
        let outcome = state.bridgehead_advance_state(2, 2, true, &terrain);
        assert!(matches!(outcome, StateOutcome::NoChange));
        let post = state.cell(2, 2).copied().unwrap();
        assert!(matches!(post.damage_state, DamageState::Healthy { .. }));
    }

    #[test]
    fn bridgehead_advance_h_gt_4_ew_absorbs_with_no_change() {
        // Bridgehead at h=0xC (EW high-ramp peak): upper-bound gate fires.
        let mut terrain = crate::map::resolved_terrain::ResolvedTerrainGrid::empty(32, 32);
        terrain.set_cell_template_height(2, 2, 0x0C);
        let mut state = BridgeRuntimeState::with_dims(32, 32);
        state.set_cell(2, 2, Some(BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 0,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::EW),
            role: BridgeCellRole::Bridgehead,
            anchor_span_id: None,
            overlay_byte: 0,
            damaged_variant: false,
            bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
        }));
        let outcome = state.bridgehead_advance_state(2, 2, true, &terrain);
        assert!(matches!(outcome, StateOutcome::NoChange));
    }
```

**Step 6: Add mid-walk-tolerance test**

```rust
    #[test]
    fn bridgehead_advance_walks_through_odd_intermediate() {
        // Layout: (2,2)=8 bridgehead, (2,3)=5 (odd, no parity check inside walk),
        // (2,4)=4 anchor. Per Ghidra 0x576BA0 walk loop has no mid-walk parity.
        let mut terrain = crate::map::resolved_terrain::ResolvedTerrainGrid::empty(32, 32);
        terrain.set_cell_template_height(2, 2, 8);
        terrain.set_cell_template_height(2, 3, 5);
        terrain.set_cell_template_height(2, 4, 4);
        let mut state = BridgeRuntimeState::with_dims(32, 32);
        // Place bridgehead at (2, 2) + anchor at (2, 4); (2, 3) is unsubscribed
        // (no BridgeRuntimeCell entry) — the walk just reads template_height.
        state.set_cell(2, 2, /* same Bridgehead literal as above */ None_PLACEHOLDER);
        state.set_cell(2, 4, /* same Anchor literal as above */ None_PLACEHOLDER);
        // (Use the builder helper if you wrote one in Step 3.)

        let outcome = state.bridgehead_advance_state(2, 2, true, &terrain);
        assert!(matches!(outcome, StateOutcome::Absorbed));
        let post_anchor = state.cell(2, 4).copied().unwrap();
        assert_eq!(post_anchor.bridgehead_anchor_class, BridgeheadAnchorClass::Damaged);
    }
```

(Replace `None_PLACEHOLDER` with the actual cell literals matching the builder helper from Step 3.)

**Step 7: Verify**

Run: `cargo test -p ra2_rust_game --lib bridgehead_advance -- --nocapture 2>&1 | tail -40`

Expected: all bridgehead_advance tests PASS. Pre-existing tests that the design didn't touch (`bridgehead_advance_destroyed_no_change`, `bridgehead_advance_non_bridgehead_role_no_change`, `bridgehead_advance_anchor_walk_failure_no_change`, `bridgehead_advance_partial_collapse_states_no_change`, `bridgehead_advance_off_map_no_change`) should still pass — they test branches that survive the rewrite (role filter, walk-failure path, off-map).

**Step 8: Run the whole bridge test suite**

Run: `cargo test -p ra2_rust_game --lib bridge -- --nocapture 2>&1 | tail -50`

Expected: all bridge tests PASS (including the body_cell, update_ramp_perpendicular, and bridgehead_walk tests from Tasks 5, 7).

**Step 9: Commit (single commit covering Tasks 4-9)**

`git add -A && git commit -m "$(cat <<'EOF'
sim/bridge: rewrite bridgehead damage state machine for gamemd parity

Refactors bridgehead_walk_to_anchor to compute walk direction internally
(NS: h<4→S, h>4→N; EW: h<2→E, h>2→W per literal asm at 0x576BA0); drops
the mid-walk parity check that was stricter than gamemd. Extends
update_ramp_perpendicular to accept Bridgehead-role targets and write the
new bridgehead_anchor_class field per the asymmetric A/B tile-class
table. Rewrites bridgehead_advance_state to write Damaged on the anchor's
new field (mirroring gamemd's SetOverlayAndPropagate target) and removes
the wrong Damaged→Destroyed branch; sustained direct ramp fire no longer
collapses bridges.

Note: renderer doesn't read bridgehead_anchor_class yet — follow-up.
EOF
)"`

---

### Task 10: Integration test through the orchestrator (IonCannon on ramp)

**Why:** Confirms the end-to-end dispatch path (Apply_area_damage → bridge_orchestrator → bridgehead_advance_state) produces the right outcome. Tests at the boundary between dispatcher and driver.

**Files:**
- Modify: `src/sim/world/world_orders_bridge_repair_tests.rs` (existing integration-test file for bridge work) — or `src/sim/world/bridge_orchestrator.rs` `#[cfg(test)] mod tests` if that's the actual pattern.

**Pattern:** Mirrors the existing integration tests added in G1's repair work. Check the file for the existing test layout before writing.

**Step 1: Find the integration test pattern**

Run: `grep -n "fn.*test.*ramp\|#\\[test\\]\|world_orders\|bridge_orchestrator.*test" src/sim/world/world_orders_bridge_repair_tests.rs src/sim/world/bridge_orchestrator.rs | head -20`

Read the closest existing integration test as a pattern template.

**Step 2: Add the new test**

Append to the most fitting file (likely `world_orders_bridge_repair_tests.rs`):

```rust
    #[test]
    fn ramp_fire_does_not_collapse_high_bridge() {
        // Build a small map with an NS high bridge:
        //   anchor at (5, 5), bridgehead at (5, 3) with template_height=8.
        // Fire 10 IonCannon shots at (5, 3). Assert:
        //   - bridgehead's damage_state never changes
        //   - anchor's bridgehead_anchor_class becomes Damaged after first hit
        //   - no BridgeCollapsed event emitted on any hit
        // (Use the project's existing test-world builder if there is one;
        // otherwise hand-build SimulationState with the minimum surface.)
        // ...
    }
```

This test stub is a placeholder marker — fill in concrete setup from whatever helper the existing G1 repair tests use to build a test world with a bridge. Run:

`grep -n "fn build_test_world\|fn make_test_simulation\|fn setup_bridge_world" src/sim/world/`

to find it.

**Step 3: Verify**

Run: `cargo test -p ra2_rust_game --lib ramp_fire_does_not_collapse -- --nocapture 2>&1 | tail -20`

Expected: PASS.

**Step 4: Run the full sim test suite**

Run: `cargo test -p ra2_rust_game --lib 2>&1 | tail -20`

Expected: all tests PASS, no regressions.

**Step 5: Commit**

`git add -A && git commit -m "sim/world: integration test for ramp-fire-no-collapse parity"`

---

### Task 11: Final verification + parity-acceptance log

**Why:** Confirm nothing else broke and the parity story is documented for the next reader.

**Files:**
- Nothing modified — verification + documentation only.

**Step 1: Full test suite**

Run: `cargo test -p ra2_rust_game 2>&1 | tail -20`

Expected: all tests PASS across the workspace.

**Step 2: Clippy + fmt**

Run: `cargo clippy -p ra2_rust_game --lib --tests -- -D warnings 2>&1 | tail -40`

Expected: no warnings.

Run: `cargo fmt`

Expected: no diff to commit.

**Step 3: Update the AUDIT_LOG.md with the implementation entry**

The verify-doc skill already added the audit entry. After implementation, append a SHORT line to [AUDIT_LOG.md](../../../ra2-rust-game-docs/AUDIT_LOG.md) noting the G3 implementation, following the format of prior entries. Skip if not requested.

**Step 4: Optional — manual in-game verification**

If user requests: run the game, fire a tank at a high bridge ramp, observe that the bridge does NOT collapse (vs. current behavior where it collapses on second hit). Note in the commit description.

**Step 5: Final commit (if anything was tweaked)**

`git status`

If clean, no commit. If anything was adjusted in clippy or fmt, commit with `cargo fmt + clippy cleanup` message.

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-12-bridgehead-damage-progression-design.md](2026-05-12-bridgehead-damage-progression-design.md)
- **Fidelity check:** [docs/fidelity-checks/bridgehead-damage-progression.md](../fidelity-checks/bridgehead-damage-progression.md) (corrected today; EW direction + NS direction fixed)
- **Verify-doc audit:** [ra2-rust-game-docs/AUDIT_LOG.md](../../../ra2-rust-game-docs/AUDIT_LOG.md) entry 2026-05-12 for `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` — YELLOW status, 3 WRONG findings (one of them load-bearing for G3), 1 MISLEADING (the one the user's framing was based on)
- **Primary research doc:** [ra2-rust-game-docs/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md) §§ 3.1, 3.2, 11.1 (caveat: §3.2 prose "progressive damage" misleading per audit; pseudocode is accurate)
- **gamemd.exe addresses** (kept here, NOT in Rust code comments per CLAUDE.md):
  - `ProcessBridgeDamageStateMachine_High` @ 0x00576BA0 — main driver. NS branch entry 0x005771c3; EW branch entry 0x00576c5f.
  - `SetOverlayAndPropagate` @ 0x0056EB80 — writes IsoTileTypeIndex (`+0x38`). Misleading function name.
  - `UpdateRamp_NS_DamageA_High` @ 0x00572230 — tile-class preserves (Variant0 / Damaged)
  - `UpdateRamp_NS_DamageB_High` @ 0x00572330 — tile-class progresses (Variant0 → Variant1, Variant1 → Damaged)
  - `UpdateRamp_NS_CollapseA_High` @ 0x00572440 — collapse variant; advances to Damaged, preserves AboutToFall
  - `UpdateRamp_NS_CollapseB_High` @ 0x005727E0 — collapse variant
  - EW siblings @ 0x00572B80 / 0x00572C90 / 0x00572DA0 / 0x00573170
  - NS walk direction tie-breaker @ 0x005771d3 (`JGE` after `CMP EAX, 0x4`)
  - EW walk direction tie-breaker @ 0x00576ca3 (`JGE` after `CMP EAX, 0x2`)
  - NS walk loop body @ 0x005771eb-0x00577237 (no parity check inside)
  - EW walk loop body @ 0x00576cbb-0x00576d07 (no parity check inside)
  - Sentinel cell @ `DAT_00ABDC50` (h=0 at offset +0x11A)
- **Globals** (theater-load filled; zero in static image): `DAT_00AA0E28` = BridgeSet; `DAT_00ABAD30` = NS bridgehead class offset; `DAT_00AA1028` = EW bridgehead class offset; each carries 4 consecutive class values (+0/+1/+2/+3).
- **INI keys:** `[CombatDamage] BridgeStrength=1500` (rulesmd.ini) — already parsed into `BridgeRuntimeState.bridge_strength`; consumed by dispatcher's RNG gate. No new INI plumbing.
- **Related code patterns:**
  - `body_cell_advance_state` @ [src/sim/bridge_state/mod.rs:797](../../src/sim/bridge_state/mod.rs#L797) — closest structural analog for the driver.
  - `update_ramp_perpendicular` @ [src/sim/bridge_specs.rs:533](../../src/sim/bridge_specs.rs#L533) — base helper to extend.
  - `apply_damaged_variant_flood_fill` @ [src/sim/bridge_state/mod.rs:983](../../src/sim/bridge_state/mod.rs#L983) — must compose, not replace.
- **Recent prior commits** to be aware of: `7d1f147` (terrain plumbing through bridge helpers), `1c9b63e` (flood-fill in update_ramp_perpendicular), `6f150dc` (damaged-variant clear in repair).
