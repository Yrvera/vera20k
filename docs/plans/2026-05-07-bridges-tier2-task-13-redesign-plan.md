# Bridges Tier 2 — Task 13 Redesign Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.
> Do not skip the parallel-session rule (CLAUDE.md): if `cargo build` or
> `cargo test` fails in a file you didn't modify, stop and report.

**Goal:** Implement the body-cell branch of `ProcessBridgeDamageStateMachine_High @ 0x576BA0` end-to-end (anchor `Healthy → Damaged → Destroyed` transitions + perpendicular-target state-byte writes via `apply_ramp_transition` + `BlowUpBridge` cascade via `set_bridge_direction`), with `overlay_byte` storage on `BridgeRuntimeCell` as a predecessor schema task. The perpendicular-target overlay-write branch (pavement-toggle / bridgehead `SetOverlayAndPropagate`) is deferred to a follow-up task because its trigger constants (`DAT_00abad30`, `DAT_00aa1028`, `DAT_00abc1e8`, `DAT_00aa0e38`, `DAT_00aa0e28`) are runtime-initialized from rules data and cannot be statically resolved from the binary image.

**Architecture:** Continues Phase C body-branch parity. Pure helpers (`apply_ramp_transition`, `pick_destruction_overlay`, `set_bridge_direction`) already on `dev` (`c9395be`, `2c8c315`, `16cf81c`); this plan adds the schema extension that lets cells carry mutable overlay state (Task 1), the typed conversion helpers between `DamageState` and binary state byte (Task 2), the perpendicular-walk wrapper that calls `apply_ramp_transition` on the partner (Task 3 — state-byte branch only), and the body-cell driver that ties them together (Task 4).

**Design Doc:** [docs/plans/2026-05-07-bridges-tier2-task-13-redesign-design.md](2026-05-07-bridges-tier2-task-13-redesign-design.md)

---

## Grounding Summary

- **R1 — ra2-rust-game-docs:** Primary source `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` §3.1 (body-cell branch) + §11.1 (8 `UpdateRamp_*_High` helpers' state transitions, all addressed). Doc cites `0x576BA0` body branch, `0x00572230` etc. for `UpdateRamp_*` helpers.
- **R2 — Ghidra verifications (live this session):** `ProcessBridgeDamageStateMachine_High @ 0x576BA0` body branch decompiled; switch arms mapped to DamageState variants (see Tiny-Detail Ledger). `UpdateRamp_NS_DamageA_High @ 0x00572230` decompiled; confirmed two-branch structure (anchor-flag-gated state-byte write + IsoTileTypeIndex-class-gated overlay-write). State-transition primitive `apply_ramp_transition` already verified live in commit `c9395be`. **Five tile-class constants confirmed runtime-initialized (zero in static image), blocking the overlay-write branch.**
- **R3 — Repo patterns:** `BridgeRuntimeState::apply_damage` ([bridge_state.rs:424-462](../../src/sim/bridge_state.rs#L424-L462)) is the existing method-on-state mutation pattern — Task 4's `body_cell_advance_state` mirrors its shape (`&mut self`, returns outcome enum). `BridgeRuntimeCell` field-extension precedent: Phase B commit `a9d64bc`. World-hash extension precedent: `b5d6a5e`. Phase C pure helpers in `bridge_specs.rs` (`apply_ramp_transition`, `pick_destruction_overlay`, `set_bridge_direction`) — Task 3's `update_ramp_perpendicular` extends this pattern.
- **R4 — INI keys:** None new for this plan. `BridgeStrength`, `DestroyableBridges` already parsed; their consumers are out of scope (Phase F orchestrator).

**Unknowns (deferred to follow-up):** The five `DAT_*` tile-class constants the `UpdateRamp_*` overlay-write branch reads cannot be statically resolved. Resolution requires either runtime debugger observation of `gamemd.exe` after rules load, or a repo-side iso-tile classification pass against retail tile-set data. This is its own research task — listed under Open Questions.

## Key Technical Decisions

| Decision | Rationale | Confidence | Source |
|---|---|---|---|
| Method-on-`BridgeRuntimeState` for body driver, free function in `bridge_specs.rs` for the perpendicular wrapper | Mirrors existing `apply_damage` shape and Phase C pure-helpers location | high | repo pattern: [bridge_state.rs:424-462](../../src/sim/bridge_state.rs#L424-L462) and [bridge_specs.rs](../../src/sim/bridge_specs.rs) |
| Compute perpendicular partner via `Direction::offset()` at call time, no schema field | Mirrors binary's `g_DirectionOffsets[dir & 7]` walk; lookup is one arithmetic + Vec index | high | `[GHIDRA UpdateRamp_NS_DamageA_High @ 0x00572230]` |
| Conversion methods on `DamageState` (`to_state_byte(self, axis)`, `from_state_byte(byte)`) | One bijection point, reusable for snapshot/hash debug + future Task 14 bridgehead driver | high | doc HIGH §3.1 + `apply_ramp_transition` docstring (already shipped) |
| `overlay_byte: u8` stored on `BridgeRuntimeCell`, populated at map-load from `bridge_layer.overlay_id` | Mirrors binary `+0x44` storage 1:1; renderer reads this; alternative (compute at render time) couples render to sim state machine | high | binary CellClass+0x44 + design doc Q4 = A1 |
| Defer perpendicular overlay-write branch to follow-up task | The 5 tile-class constants gating the branch are runtime-initialized; can't ship correctly without observing them at runtime | high | `[GHIDRA read_memory 0x00abad30, 0x00aa1028, 0x00abc1e8, 0x00aa0e38, 0x00aa0e28]` — all zero in static image |
| `StateOutcome::Collapsed` carries `adjacent_bridges_dirty: Vec<(u16,u16)>` and `zones_dirty: bool` flags rather than calling rim/zone primitives directly | Those primitives (`UpdateAdjacentBridges_High`, `UpdateBridgeZonesHelper`) are Phase F orchestrator scope; body driver emits intent, orchestrator dispatches | high | design doc §Impact + tier-2 file map [Tier 2 plan §132](2026-05-07-bridges-tier2-damage-state-machine-plan.md) |
| Body driver does not replace `apply_damage` in this plan; both coexist | `apply_damage` is the single-shot flag-bit fallback; Phase F integration chooses dispatch. Don't hot-swap mid-Phase-C | medium | inferred from tier-2 plan structure — `apply_damage` removal is part of Phase F task list |

## Open Questions

### Resolved During Planning

- *"Where does the perpendicular partner cell live in our types?"* — Compute on the fly via `anchor + Direction::offset()`. No schema growth. (Brainstorm Q2.)
- *"How do we convert `DamageState` to/from binary state byte?"* — Methods on `DamageState`. Bijection per axis except for state 0 (Healthy{0} initial vs Destroyed post-collapse — caller resolves via context). (Brainstorm Q3.)
- *"Should overlay byte be stored or computed at render time?"* — Stored. (Brainstorm Q4.)
- *"Should Task 13 be split or monolithic?"* — Monolithic for the body branch. (Brainstorm Q1.)

### Deferred to Implementation

- *"Exact pavement and bridgehead+0/+2 tile-class constants for the perpendicular overlay-write branch."* — Blocked on runtime observation of `DAT_00abad30 / DAT_00aa1028 / DAT_00abc1e8 / DAT_00aa0e38 / DAT_00aa0e28` after `gamemd.exe` rules load. Requires either debugger-on-live-game or repo-side iso-tile classification research. Spawn a follow-up task ("Task 13.5: perpendicular overlay-write branch") once these are resolved.
- *"Visible parity gap from the deferred overlay-write branch."* — When an end-anchor of a bridge takes Healthy→Damaged or Damaged→Collapsed transitions, the perpendicular bridgehead/ramp tile's overlay does NOT update in this plan's output. Player sees stale ramp overlay until Task 13.5 lands. Acknowledged drift; tracked.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/sim/bridge_state.rs` | Add `pub overlay_byte: u8` field on `BridgeRuntimeCell` (Task 1). Populate at map-load (Task 1). Add `DamageState::to_state_byte` / `from_state_byte` methods (Task 2). Add `StateOutcome` enum + `BridgeRuntimeState::body_cell_advance_state` method (Task 4). Tests for each. |
| Modify | `src/sim/bridge_specs.rs` | Add `update_ramp_perpendicular` free function — state-byte branch only (Task 3). Tests. |
| Modify | `src/sim/world/world_hash.rs` | Extend `hash_bridge_state` to include `overlay_byte` (Task 1). |

## Interface Changes

**Public:**
- `BridgeRuntimeCell` gains `pub overlay_byte: u8`. All snapshot serialization (serde-derive) and world-hash already cover the struct's other public fields; adding one field doesn't break existing consumers but **does** invalidate any pre-existing serialized snapshots (acceptable per CLAUDE.md "no production save format yet").
- `DamageState::to_state_byte(self, axis: Axis) -> u8` — new method.
- `DamageState::from_state_byte(byte: u8) -> Option<Self>` — new method.
- `bridge_specs::update_ramp_perpendicular(...) -> RampOutcome` — new free function.
- `BridgeRuntimeState::body_cell_advance_state(...) -> StateOutcome` — new method, distinct entry point from `apply_damage` (which stays unchanged in this plan).
- `bridge_state::StateOutcome` enum — new public type.
- `bridge_specs::RampOutcome` struct — new public type.

**Internal:** none.

## Sim Checklist

- [x] All math uses `fixed`-point — no f32/f64. State byte is `u8`, overlay byte is `u8`, transitions are integer-pure.
- [x] New state included in deterministic state hash — Task 1 extends `hash_bridge_state` to cover `overlay_byte`.
- [x] No dependencies on render/ui/sidebar/audio/net — body driver takes `&ResolvedTerrainGrid` (rules/map module) and `&mut BridgeRuntimeState`.
- [x] Tick ordering impact noted — `body_cell_advance_state` is a deterministic state transition; called from Phase F orchestrator (not part of this plan). No new RNG draws in this plan.
- [x] BTreeMap iteration order considered — `anchor_spans` is `BTreeMap<u16, AnchorSpan>` (existing); body driver looks up by `anchor_span_id` directly, no iteration.

## Risk Areas

- **Anchor partner lookup robustness.** Body driver follows `cell.anchor_span_id` → `state.anchor_span(id).anchor` when input cell is `Body` or `Tail`. If a body cell has `anchor_span_id == None` (Phase B map-load gap, edge case), we return `StateOutcome::NoChange`. Tests cover this.
- **State-byte 0 ambiguity.** `Healthy{variant: 0}` and `Destroyed` both map to state byte 0. The conversion methods document this; body driver writes `Destroyed` directly (not via `from_state_byte(0)`) at the collapse-final write point, sidestepping the ambiguity.
- **Snapshot binary format breaks** for in-flight dev saves due to new `overlay_byte` field — acceptable per CLAUDE.md (no production save format).
- **Perpendicular overlay-write deferral.** Visible cosmetic drift at bridge ends until Task 13.5 lands. Documented; not a regression of currently-shipping behavior.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|---|---|---|---|
| Task 4 | Body driver follows `+0x80`/`+0x2C` partner indirection (in our model: `cell.role != Anchor` → `anchor_span_id` lookup) | Damaging a non-anchor body cell must transition the anchor's state, not the damaged cell. Off-by-this fails on every multi-cell bridge | Unit test: damage on `BridgeCellRole::Body` cell mutates anchor's `damage_state` |
| Task 4 | `Healthy → Damaged` writes anchor state to byte 6 (NS) / 0xF (EW); fires UpdateRamp DamageA + DamageB on perpendicular targets | Wrong anchor state byte = wrong renderer tile = visible | Unit test asserts `anchor.damage_state == Damaged` after first hit; perpendicular `update_ramp_perpendicular` returned `state_changed=true` for both A and B |
| Task 4 | `Damaged → Collapsed` writes anchor state to 0, fires UpdateRamp CollapseA + CollapseB, emits `set_bridge_direction(span, false)` | Cascading `BlowUpBridge` on cells 0/1/2/4 (along axis) is the visible "bridge falls down" effect | Unit test asserts `StateOutcome::Collapsed { set_bridge_direction.actions }` contains 4 `BlowUpBridge` entries |
| Task 4 | `PartialCollapseA` (state 7/17) fires CollapseA only; `PartialCollapseB` (state 8/16) fires CollapseB only | Partial-collapse states are reachable via bridgehead cascade (Task 14); wrong dispatch = wrong overlay tile + wrong cascade | Unit test for each partial state |
| Task 3 | Perpendicular target state byte transitions match `apply_ramp_transition` exactly when target has anchor flag | Off-by-one on target state byte = renderer mispicks tile on adjacent bridge cells | Unit test for each (axis, phase) × (target healthy / damaged / partial) combo |
| Task 3 | Perpendicular target with `role != Anchor` does not mutate (anchor-flag gate) | Mutating a non-anchor target's state byte writes garbage | Unit test: target = Body cell → no mutation |
| Task 1 | `overlay_byte` populated at map-load from `bridge_layer.overlay_id` for every BridgeRuntimeCell | Renderer's initial display (before any damage) needs correct overlay byte; pre-damage parity | Test: map fixture with known overlay ids → `state.cell(rx, ry).overlay_byte` matches input |

---

## Tasks

### Task 1: Add `overlay_byte` to `BridgeRuntimeCell`; populate at map-load; extend world hash

**Why:** Tier 2 body driver and the deferred Task 13.5 perpendicular-overlay branch both write to per-cell overlay byte. Renderer (Phase D) reads it for display tile selection. Adding the field as an isolated schema task lets snapshot/hash testing run against the schema before any state-machine logic depends on it.

**Files:** Modify `src/sim/bridge_state.rs`, `src/sim/world/world_hash.rs`.

**Pattern:** Mirrors Phase B commit `a9d64bc` (BridgeRuntimeCell field extension) + `b5d6a5e` (world_hash extension).

**Step 1: Add the field**

In `src/sim/bridge_state.rs`, locate `pub struct BridgeRuntimeCell` (currently around line 184-210; the `Hash` derive must remain). Add the new field at the end of the struct:

```rust
    /// Per-cell visible overlay byte (mirrors binary `CellClass+0x44`).
    /// Populated at map-load from `ResolvedTerrainCell.bridge_layer.overlay_id`;
    /// mutated at runtime by the body-cell state machine and (future) perpendicular
    /// overlay-write branch. Renderer queries this to pick the visible tile.
    pub overlay_byte: u8,
```

The struct currently has `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]` — keep all derives. The new `u8` is `Copy` so this stays valid.

**Step 2: Populate at map-load**

In `BridgeRuntimeState::from_resolved_terrain`, find Pass 1's BFS loop where `cells[idx] = Some(BridgeRuntimeCell { ... });` is constructed (currently around line 290-300). Add `overlay_byte` to the struct literal:

```rust
                cells[idx] = Some(BridgeRuntimeCell {
                    deck_present: true,
                    destroyable,
                    deck_level: resolved.bridge_deck_level,
                    bridge_group_id: Some(group_id),
                    damage_state: DamageState::Healthy { variant: 0 },
                    axis: bridge_layer_to_axis(resolved.bridge_layer.as_ref()),
                    role: BridgeCellRole::Body, // overwritten in pass 2
                    anchor_span_id: None,
                    bridgehead_step: 0,
                    overlay_byte: resolved
                        .bridge_layer
                        .as_ref()
                        .map(|bl| bl.overlay_id)
                        .unwrap_or(0),
                });
```

The `unwrap_or(0)` covers the (rare) case of a `has_bridge_deck` cell without a populated `bridge_layer` — defensive default; should not occur for cells reaching this branch.

**Step 3: Extend `hash_bridge_state` in world_hash.rs**

In `src/sim/world/world_hash.rs`, locate `fn hash_bridge_state` (currently lines 210-237). Add one line inside the per-cell `for ((rx, ry), cell) in entries { ... }` loop, immediately after `cell.bridgehead_step.hash(hasher);`:

```rust
            cell.overlay_byte.hash(hasher);
```

**Step 4: Add `cell_mut` accessor on `BridgeRuntimeState`**

Task 3's `update_ramp_perpendicular` and Task 4's `body_cell_advance_state` both need mutable cell access. `BridgeRuntimeState` currently only exposes a read-only `cell(...)` accessor. Add the mutable peer in `src/sim/bridge_state.rs`, in the `impl BridgeRuntimeState` block, immediately after the existing `pub fn cell(&self, ...)` method:

```rust
    /// Mutable cell access. Returns `None` if `(rx, ry)` is out of bounds or
    /// the cell is not a bridge runtime cell.
    pub fn cell_mut(&mut self, rx: u16, ry: u16) -> Option<&mut BridgeRuntimeCell> {
        index_of(self.width, self.height, rx, ry)
            .and_then(move |idx| self.cells.get_mut(idx))
            .and_then(|cell| cell.as_mut())
    }
```

**Step 5: Add `test_seed_cell` + `test_seed_anchor_span` test-only helpers**

Tests in this task (and Tasks 3 and 4) need to construct precise minimal bridge states without going through `from_resolved_terrain`. In the same `impl BridgeRuntimeState` block, after `cell_mut`, add:

```rust
    /// Test-only: insert a `BridgeRuntimeCell` at `(rx, ry)`, growing the
    /// internal `cells` Vec and `width`/`height` to fit if needed. Used by
    /// unit tests that need precise control over cell placement and state
    /// without going through `from_resolved_terrain`.
    #[cfg(test)]
    pub(crate) fn test_seed_cell(&mut self, rx: u16, ry: u16, cell: BridgeRuntimeCell) {
        let needed_w = (rx + 1).max(self.width);
        let needed_h = (ry + 1).max(self.height);
        if needed_w != self.width || needed_h != self.height {
            // Resize while preserving existing (rx, ry) → cell mappings.
            let mut new_cells = vec![None; needed_w as usize * needed_h as usize];
            for old_ry in 0..self.height {
                for old_rx in 0..self.width {
                    let old_idx = old_ry as usize * self.width as usize + old_rx as usize;
                    let new_idx = old_ry as usize * needed_w as usize + old_rx as usize;
                    new_cells[new_idx] = self.cells[old_idx];
                }
            }
            self.cells = new_cells;
            self.width = needed_w;
            self.height = needed_h;
        }
        let idx = ry as usize * self.width as usize + rx as usize;
        self.cells[idx] = Some(cell);
    }

    /// Test-only: insert an `AnchorSpan` directly into the registry.
    #[cfg(test)]
    pub(crate) fn test_seed_anchor_span(&mut self, span: AnchorSpan) {
        self.anchor_spans.insert(span.id, span);
    }
```

**Step 6: Add focused tests**

In `src/sim/bridge_state.rs` `mod tests` (currently ends around line 886), append at the end of the test module (before its closing `}`):

```rust
    #[test]
    fn overlay_byte_populated_at_map_load() {
        // make_bridge_terrain in this file creates a 5x1 strip; the constructor
        // populates overlay_byte from bridge_layer.overlay_id (or 0 if none).
        let state = BridgeRuntimeState::from_resolved_terrain(
            &make_bridge_terrain(),
            true,
            1500,
        );
        // Field is reachable on every populated bridge cell; type is u8.
        for (_, cell) in state.iter_cells() {
            let _byte: u8 = cell.overlay_byte;
        }
    }

    #[test]
    fn overlay_byte_round_trips_via_snapshot() {
        let state = BridgeRuntimeState::from_resolved_terrain(
            &make_bridge_terrain(), true, 1500,
        );
        let json = serde_json::to_string(&state).expect("serialize");
        let restored: BridgeRuntimeState =
            serde_json::from_str(&json).expect("deserialize");
        for ((rx, ry), cell) in state.iter_cells() {
            let r = restored.cell(rx, ry).expect("restored cell present");
            assert_eq!(cell.overlay_byte, r.overlay_byte, "overlay_byte at ({rx},{ry})");
        }
    }

    #[test]
    fn test_seed_cell_grows_grid_to_fit() {
        let mut state = BridgeRuntimeState::default();
        let cell = BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 0,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::NS),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(1),
            bridgehead_step: 0,
            overlay_byte: 0x18,
        };
        state.test_seed_cell(5, 5, cell);
        let read = state.cell(5, 5).expect("seeded cell present");
        assert_eq!(read.overlay_byte, 0x18);
        assert_eq!(read.role, BridgeCellRole::Anchor);
    }

    #[test]
    fn cell_mut_writes_visible_through_cell_read() {
        let mut state = BridgeRuntimeState::default();
        let cell = BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 0,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::NS),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(1),
            bridgehead_step: 0,
            overlay_byte: 0x18,
        };
        state.test_seed_cell(2, 2, cell);
        state.cell_mut(2, 2).unwrap().overlay_byte = 0xD2;
        assert_eq!(state.cell(2, 2).unwrap().overlay_byte, 0xD2);
    }
```

(The existing `bridge_runtime_state_snapshot_round_trip` test at line 858 already does cell-level `PartialEq` comparison, so the new field is implicitly covered there too — these tests add explicit assertions on the new field and exercise the new accessors.)

**Step 7: Hash-coverage test for `overlay_byte`**

In `src/sim/world/world_hash.rs`, append a new test module at the end of the file. This test uses `test_seed_cell` to construct two bridge states differing only in `overlay_byte` and asserts their hashes differ — directly verifying that Step 3's hash extension reaches `overlay_byte`.

```rust
#[cfg(test)]
mod bridge_overlay_hash_tests {
    use super::Simulation;
    use crate::sim::bridge_state::{
        Axis, BridgeCellRole, BridgeRuntimeCell, BridgeRuntimeState, DamageState,
    };

    fn make_bridge_state_with_overlay(byte: u8) -> BridgeRuntimeState {
        let mut state = BridgeRuntimeState::default();
        state.test_seed_cell(
            2, 2,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 0,
                bridge_group_id: Some(1),
                damage_state: DamageState::Healthy { variant: 0 },
                axis: Some(Axis::NS),
                role: BridgeCellRole::Anchor,
                anchor_span_id: None,
                bridgehead_step: 0,
                overlay_byte: byte,
            },
        );
        state
    }

    #[test]
    fn overlay_byte_difference_changes_state_hash() {
        let mut sim_a = Simulation::new();
        let mut sim_b = Simulation::new();
        sim_a.bridge_state = Some(make_bridge_state_with_overlay(0x18));
        sim_b.bridge_state = Some(make_bridge_state_with_overlay(0xD2));
        assert_ne!(
            sim_a.state_hash(),
            sim_b.state_hash(),
            "overlay_byte must contribute to state hash",
        );
    }

    #[test]
    fn identical_overlay_bytes_hash_equal() {
        let mut sim_a = Simulation::new();
        let mut sim_b = Simulation::new();
        sim_a.bridge_state = Some(make_bridge_state_with_overlay(0x18));
        sim_b.bridge_state = Some(make_bridge_state_with_overlay(0x18));
        assert_eq!(
            sim_a.state_hash(),
            sim_b.state_hash(),
            "identical bridge states must hash equal",
        );
    }
}
```

**Step 8: Verify**

```
cargo test --lib sim::bridge_state
cargo test --lib sim::world::world_hash
cargo build
```

Expected:
- All existing `sim::bridge_state` tests still pass; 4 new tests added, all pass.
- `sim::world::world_hash` tests still pass; 2 new `bridge_overlay_hash_tests` pass.
- `cargo build` exits 0.

**Parallel-session rule:** if `cargo build` or `cargo test` fails with errors in files you did NOT modify (`src/sim/bridge_state.rs`, `src/sim/world/world_hash.rs`), stop and report. Do not attempt to fix unrelated errors.

**Step 9: Commit**

```
git add src/sim/bridge_state.rs src/sim/world/world_hash.rs
git commit -m "bridge_state: add overlay_byte to BridgeRuntimeCell + cell_mut accessor + test-seed helpers; cover overlay_byte in world hash"
```

**Constraints:**
- `git add` with explicit paths only. Never `-A` / `.` / `-u`.
- Do NOT push. Local `dev` only.
- Do NOT amend. New commit only.
- Do NOT use `--no-verify`.
- Do NOT touch `docs/`.

---

### Task 2: Add `DamageState::to_state_byte` and `from_state_byte`

**Why:** Task 3's perpendicular wrapper needs to convert `target.damage_state: DamageState` to the binary state byte (for `apply_ramp_transition`) and back. The bijection lives once on the type so future Task 14 (bridgehead driver) and snapshot debug code reuse it.

**Files:** Modify `src/sim/bridge_state.rs`.

**Pattern:** New methods on existing public enum, inline tests.

**Step 1: Add the methods**

In `src/sim/bridge_state.rs`, locate the `DamageState` enum (currently at line 35-51). Immediately after the `pub enum DamageState { ... }` block, add:

```rust
impl DamageState {
    /// Encode to binary state byte (`CellClass+0x11E`).
    ///
    /// Per HIGH §3.1 / `apply_ramp_transition` docstring:
    /// - NS axis: Healthy{variant: 0..=5} → 0..=5; Damaged → 6;
    ///   PartialCollapseA → 7; PartialCollapseB → 8; Destroyed → 0.
    /// - EW axis: Healthy{variant: 0..=5} → 9..=14; Damaged → 0xF;
    ///   PartialCollapseA → 0x11; PartialCollapseB → 0x10; Destroyed → 0.
    ///
    /// **Note:** `Destroyed` always maps to byte 0, which is also the encoding
    /// for `Healthy{variant: 0}` initial state. Callers must use context
    /// (phase + prior state) to disambiguate after a `from_state_byte(0)` decode.
    /// `to_state_byte` is unambiguous (every variant has exactly one encoding).
    pub fn to_state_byte(self, axis: Axis) -> u8 {
        let ns_base: u8 = 0;
        let ew_base: u8 = 9;
        let base = match axis { Axis::NS => ns_base, Axis::EW => ew_base };
        match self {
            DamageState::Healthy { variant } => base + variant.min(5),
            DamageState::Damaged => match axis { Axis::NS => 6, Axis::EW => 0xF },
            DamageState::PartialCollapseA => match axis { Axis::NS => 7, Axis::EW => 0x11 },
            DamageState::PartialCollapseB => match axis { Axis::NS => 8, Axis::EW => 0x10 },
            DamageState::Destroyed => 0,
        }
    }

    /// Decode from binary state byte. Returns `None` for bytes outside the
    /// defined ranges (NS: 0..=8; EW: 9..=0x11).
    ///
    /// **State 0 ambiguity:** byte 0 always decodes to `Healthy{variant: 0}`.
    /// Post-collapse `Destroyed` cells also have byte 0 in the binary, but the
    /// caller (body driver) writes `Destroyed` directly without round-tripping
    /// through `from_state_byte`. Test fixtures and snapshot consistency checks
    /// should not rely on this method to recover `Destroyed`.
    pub fn from_state_byte(byte: u8) -> Option<Self> {
        match byte {
            0..=5 => Some(DamageState::Healthy { variant: byte }),
            6 => Some(DamageState::Damaged),
            7 => Some(DamageState::PartialCollapseA),
            8 => Some(DamageState::PartialCollapseB),
            9..=14 => Some(DamageState::Healthy { variant: byte - 9 }),
            0xF => Some(DamageState::Damaged),
            0x10 => Some(DamageState::PartialCollapseB),
            0x11 => Some(DamageState::PartialCollapseA),
            _ => None,
        }
    }
}
```

**Step 2: Add tests**

In the same file's `mod tests` (where Task 1's new tests now live), append:

```rust
    #[test]
    fn damage_state_to_byte_ns_axis() {
        assert_eq!(DamageState::Healthy { variant: 0 }.to_state_byte(Axis::NS), 0);
        assert_eq!(DamageState::Healthy { variant: 3 }.to_state_byte(Axis::NS), 3);
        assert_eq!(DamageState::Healthy { variant: 5 }.to_state_byte(Axis::NS), 5);
        assert_eq!(DamageState::Damaged.to_state_byte(Axis::NS), 6);
        assert_eq!(DamageState::PartialCollapseA.to_state_byte(Axis::NS), 7);
        assert_eq!(DamageState::PartialCollapseB.to_state_byte(Axis::NS), 8);
        assert_eq!(DamageState::Destroyed.to_state_byte(Axis::NS), 0);
    }

    #[test]
    fn damage_state_to_byte_ew_axis() {
        assert_eq!(DamageState::Healthy { variant: 0 }.to_state_byte(Axis::EW), 9);
        assert_eq!(DamageState::Healthy { variant: 5 }.to_state_byte(Axis::EW), 14);
        assert_eq!(DamageState::Damaged.to_state_byte(Axis::EW), 0xF);
        assert_eq!(DamageState::PartialCollapseA.to_state_byte(Axis::EW), 0x11);
        assert_eq!(DamageState::PartialCollapseB.to_state_byte(Axis::EW), 0x10);
        assert_eq!(DamageState::Destroyed.to_state_byte(Axis::EW), 0);
    }

    #[test]
    fn damage_state_to_byte_clamps_healthy_variant() {
        // Variant > 5 is invalid input; should clamp to 5 (max defined healthy).
        assert_eq!(DamageState::Healthy { variant: 7 }.to_state_byte(Axis::NS), 5);
        assert_eq!(DamageState::Healthy { variant: 10 }.to_state_byte(Axis::EW), 14);
    }

    #[test]
    fn damage_state_from_byte_ns_range() {
        assert_eq!(DamageState::from_state_byte(0), Some(DamageState::Healthy { variant: 0 }));
        assert_eq!(DamageState::from_state_byte(3), Some(DamageState::Healthy { variant: 3 }));
        assert_eq!(DamageState::from_state_byte(5), Some(DamageState::Healthy { variant: 5 }));
        assert_eq!(DamageState::from_state_byte(6), Some(DamageState::Damaged));
        assert_eq!(DamageState::from_state_byte(7), Some(DamageState::PartialCollapseA));
        assert_eq!(DamageState::from_state_byte(8), Some(DamageState::PartialCollapseB));
    }

    #[test]
    fn damage_state_from_byte_ew_range() {
        assert_eq!(DamageState::from_state_byte(9), Some(DamageState::Healthy { variant: 0 }));
        assert_eq!(DamageState::from_state_byte(14), Some(DamageState::Healthy { variant: 5 }));
        assert_eq!(DamageState::from_state_byte(0xF), Some(DamageState::Damaged));
        assert_eq!(DamageState::from_state_byte(0x10), Some(DamageState::PartialCollapseB));
        assert_eq!(DamageState::from_state_byte(0x11), Some(DamageState::PartialCollapseA));
    }

    #[test]
    fn damage_state_from_byte_out_of_range_returns_none() {
        assert_eq!(DamageState::from_state_byte(0x12), None);
        assert_eq!(DamageState::from_state_byte(0xFF), None);
    }

    #[test]
    fn damage_state_round_trip_for_each_variant_per_axis() {
        // For every (axis × variant) pair where Destroyed is excluded (it's the
        // ambiguous post-collapse state).
        for axis in [Axis::NS, Axis::EW] {
            for state in [
                DamageState::Healthy { variant: 0 },
                DamageState::Healthy { variant: 5 },
                DamageState::Damaged,
                DamageState::PartialCollapseA,
                DamageState::PartialCollapseB,
            ] {
                let byte = state.to_state_byte(axis);
                let decoded = DamageState::from_state_byte(byte)
                    .expect("decode succeeds for byte produced by encode");
                assert_eq!(decoded, state, "round-trip {state:?} via {axis:?}");
            }
        }
    }
```

**Step 3: Verify**

```
cargo test --lib sim::bridge_state
cargo build
```

Expected:
- 7 new `damage_state_*` tests pass; no regressions.
- `cargo build` exits 0.

Parallel-session rule applies.

**Step 4: Commit**

```
git add src/sim/bridge_state.rs
git commit -m "bridge_state: add DamageState::to_state_byte / from_state_byte conversion methods"
```

---

### Task 3: Add `update_ramp_perpendicular` wrapper — state-byte branch only

**Why:** Task 4's body driver calls this wrapper to mirror binary `UpdateRamp_*_High` (verified `0x00572230` etc.). This task implements the state-byte transition side: walk one perpendicular cell from anchor, gate on target's anchor flag (in our model: `target.role == Anchor`), apply `apply_ramp_transition` to the target's state byte, write back via `from_state_byte`. The overlay-write branch is **deferred** per Open Questions — the gating constants are runtime-initialized.

**Files:** Modify `src/sim/bridge_specs.rs` only.

**Pattern:** New free function alongside `apply_ramp_transition`, `pick_destruction_overlay`, `set_bridge_direction`. Pure-helper module style continues. Uses `cell_mut` and `test_seed_cell` already added on `BridgeRuntimeState` in Task 1 (Steps 4 + 5).

**Step 1: Extend top-of-file `use` and add the impl**

In `src/sim/bridge_specs.rs`, the top-of-file `use` from Task 12 reads:

```rust
use crate::sim::bridge_state::{AnchorSpan, Axis, Direction, Phase};
```

Extend it to include `BridgeRuntimeState` and `DamageState`:

```rust
use crate::sim::bridge_state::{AnchorSpan, Axis, BridgeRuntimeState, DamageState, Direction, Phase};
```

(`BridgeCellRole` may also be needed depending on how the role check is expressed below; add it to the import list if your implementation uses it directly.)

Then, immediately after the `set_bridge_direction` function (added in Task 12) and before the `#[cfg(test)] mod tests {` line, add:

```rust
/// Outcome of one perpendicular `UpdateRamp_*_High`-style call. Mirrors the
/// inner side effects of binary `UpdateRamp_NS_DamageA_High @ 0x00572230` and
/// peers (HIGH §11.1).
///
/// Currently models only the **anchor-flag-gated state-byte transition**.
/// The pavement/bridgehead-overlay-write branch fires off-screen
/// (`SetOverlayAndPropagate` / `ToggleBridgePavement`) and is deferred until
/// the runtime-initialized tile-class constants are observed live —
/// see plan §"Deferred to Implementation".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RampOutcome {
    /// True if the target cell's `damage_state` was mutated (target was an
    /// anchor and the `apply_ramp_transition` returned `Some`).
    pub state_changed: bool,
}

/// Compute the perpendicular-walk direction for a body-driver UpdateRamp call.
/// A-side and B-side perpendiculars per `[GHIDRA 0x576BA0]` body branch:
/// NS axis: A → E (dir 2), B → W (dir 6).
/// EW axis: A → S (dir 4), B → N (dir 0).
fn perpendicular_direction(axis: Axis, phase: Phase) -> Direction {
    let is_a_side = matches!(phase, Phase::DamageA | Phase::CollapseA);
    match (axis, is_a_side) {
        (Axis::NS, true) => Direction::E,
        (Axis::NS, false) => Direction::W,
        (Axis::EW, true) => Direction::S,
        (Axis::EW, false) => Direction::N,
    }
}

/// Walk one perpendicular cell from `anchor_pos` and apply the `UpdateRamp_*`
/// state-byte transition if the target is an anchor cell.
///
/// **State-byte branch only** — overlay-write branch deferred (see plan).
/// Mirrors the anchor-flag-gated `+0x11E` write of binary `UpdateRamp_*_High`.
///
/// `is_high_bridge` is currently unused (state transitions are identical for
/// HIGH and LOW per HIGH §11.1) but kept for API symmetry with the deferred
/// overlay-write branch and future Task 14 (bridgehead driver).
pub fn update_ramp_perpendicular(
    state: &mut BridgeRuntimeState,
    anchor_pos: (u16, u16),
    axis: Axis,
    phase: Phase,
    _is_high_bridge: bool,
) -> RampOutcome {
    let dir = perpendicular_direction(axis, phase);
    let (dx, dy) = dir.offset();
    let target_x = anchor_pos.0 as i32 + dx;
    let target_y = anchor_pos.1 as i32 + dy;
    if target_x < 0 || target_y < 0 {
        return RampOutcome { state_changed: false };
    }
    let target_pos = (target_x as u16, target_y as u16);

    // Snapshot target read (avoids borrow conflict with subsequent mut access).
    let Some(target_cell) = state.cell(target_pos.0, target_pos.1).copied() else {
        return RampOutcome { state_changed: false };
    };
    // Anchor-flag gate. In binary: `target.flags & 0x80`. In our model:
    // role == Anchor.
    if !matches!(target_cell.role, crate::sim::bridge_state::BridgeCellRole::Anchor) {
        return RampOutcome { state_changed: false };
    }
    let Some(target_axis) = target_cell.axis else {
        return RampOutcome { state_changed: false };
    };

    let current_byte = target_cell.damage_state.to_state_byte(target_axis);
    let Some(next_byte) = apply_ramp_transition(current_byte, axis, phase) else {
        return RampOutcome { state_changed: false };
    };

    // Decode next byte. Note: byte 0 here is ambiguous (Healthy{0} vs Destroyed).
    // Per `apply_ramp_transition` docstring, next_byte == 0 only fires for the
    // collapse-final case (state 7/8/0x10/0x11 + matching CollapseA/B phase).
    // The body driver detects this via prev_state.is_partial_collapse &&
    // phase.is_collapse and writes Destroyed directly to the *anchor* — but
    // here the *perpendicular target* is the one being mutated, and per the
    // binary's UpdateRamp helper, when the perpendicular target hits its
    // recurse-to-0 branch it ALSO sets `state = 0; IsoTileTypeIndex = -1`,
    // which in our model is Destroyed. So decode 0 → Destroyed for this path.
    let next_state = if next_byte == 0 {
        DamageState::Destroyed
    } else {
        match DamageState::from_state_byte(next_byte) {
            Some(s) => s,
            None => return RampOutcome { state_changed: false },
        }
    };

    // Mut access to write the new state.
    if let Some(cell_mut) = state.cell_mut(target_pos.0, target_pos.1) {
        cell_mut.damage_state = next_state;
        RampOutcome { state_changed: true }
    } else {
        RampOutcome { state_changed: false }
    }
}
```

**Step 2: Add tests**

In `src/sim/bridge_specs.rs`'s `mod tests` (extend the existing local `use` at the top of `mod tests` to include the new types):

```rust
    use crate::sim::bridge_state::{
        AnchorSpan, Axis, BridgeCellRole, BridgeRuntimeCell, BridgeRuntimeState,
        DamageState, Direction, Phase,
    };
```

Append these tests at the end of `mod tests`:

```rust
    /// Build a minimal BridgeRuntimeState for update_ramp tests:
    /// - anchor at (5, 5) with axis NS, role Anchor, damage_state Healthy{variant: 0}
    /// - perpendicular partners at (4, 5) [W] and (6, 5) [E], also Anchor + axis NS,
    ///   damage_state Healthy{variant: 0}, so state byte = 0
    /// Uses `test_seed_cell` introduced in Task 1 Step 5.
    fn make_perpendicular_test_state() -> BridgeRuntimeState {
        let mut state = BridgeRuntimeState::default();
        let template = BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 0,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::NS),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(1),
            bridgehead_step: 0,
            overlay_byte: 0x18, // HIGH bridge anchor overlay
        };
        for (rx, ry) in [(4u16, 5u16), (5, 5), (6, 5)] {
            state.test_seed_cell(rx, ry, template);
        }
        state
    }

    #[test]
    fn update_ramp_perpendicular_ns_damage_a_anchor_target_transitions_to_4() {
        let mut state = make_perpendicular_test_state();
        // Anchor at (5,5) calling NS DamageA → walks E → target (6,5).
        // Target state byte 0 (Healthy{variant:0}) → apply_ramp_transition NS DamageA → 4.
        let outcome = update_ramp_perpendicular(
            &mut state, (5, 5), Axis::NS, Phase::DamageA, true,
        );
        assert!(outcome.state_changed);
        let target = state.cell(6, 5).expect("E target");
        // State byte 4 = Healthy{variant: 4} per from_state_byte.
        assert_eq!(target.damage_state, DamageState::Healthy { variant: 4 });
    }

    #[test]
    fn update_ramp_perpendicular_ns_damage_b_anchor_target_walks_west() {
        let mut state = make_perpendicular_test_state();
        let outcome = update_ramp_perpendicular(
            &mut state, (5, 5), Axis::NS, Phase::DamageB, true,
        );
        assert!(outcome.state_changed);
        let target = state.cell(4, 5).expect("W target");
        // NS DamageB on state 0 → 5 = Healthy{variant: 5}.
        assert_eq!(target.damage_state, DamageState::Healthy { variant: 5 });
    }

    #[test]
    fn update_ramp_perpendicular_non_anchor_target_no_change() {
        let mut state = make_perpendicular_test_state();
        // Patch (6,5) to Body role (not Anchor).
        state.cell_mut(6, 5).unwrap().role = BridgeCellRole::Body;
        let outcome = update_ramp_perpendicular(
            &mut state, (5, 5), Axis::NS, Phase::DamageA, true,
        );
        assert!(!outcome.state_changed);
        // Target unchanged.
        assert_eq!(
            state.cell(6, 5).unwrap().damage_state,
            DamageState::Healthy { variant: 0 }
        );
    }

    #[test]
    fn update_ramp_perpendicular_target_off_map_no_change() {
        let mut state = make_perpendicular_test_state();
        // Anchor at (0, 0) calling NS DamageB → walks W → target x = -1 → out of bounds.
        let outcome = update_ramp_perpendicular(
            &mut state, (0, 0), Axis::NS, Phase::DamageB, true,
        );
        assert!(!outcome.state_changed);
    }

    #[test]
    fn update_ramp_perpendicular_collapse_final_target_to_destroyed() {
        let mut state = make_perpendicular_test_state();
        // Set target (6,5) to PartialCollapseB. NS CollapseA on state 8 → 0 (collapse-final).
        state.cell_mut(6, 5).unwrap().damage_state = DamageState::PartialCollapseB;
        let outcome = update_ramp_perpendicular(
            &mut state, (5, 5), Axis::NS, Phase::CollapseA, true,
        );
        assert!(outcome.state_changed);
        let target = state.cell(6, 5).expect("E target");
        assert_eq!(target.damage_state, DamageState::Destroyed);
    }

    #[test]
    fn update_ramp_perpendicular_ew_collapse_walks_south() {
        let mut state = make_perpendicular_test_state();
        // Reconfigure for EW axis test: place anchors at (5,4) and (5,6) too.
        for &(rx, ry) in &[(5, 4), (5, 6)] {
            // These positions need to exist as cells in the state. If
            // make_perpendicular_test_state's helper allows configurable
            // positions, use it. Otherwise this test may need a separate fixture.
            if let Some(c) = state.cell_mut(rx, ry) {
                c.role = BridgeCellRole::Anchor;
                c.axis = Some(Axis::EW);
                c.damage_state = DamageState::Healthy { variant: 0 };
            }
        }
        // Patch (5,5) to EW axis too (the source anchor in this test).
        state.cell_mut(5, 5).unwrap().axis = Some(Axis::EW);
        // EW CollapseA → walks S → target (5, 6). State byte 9 (Healthy{0} EW).
        // apply_ramp_transition EW CollapseA on 9 → 0x11 = PartialCollapseA.
        if state.cell(5, 6).is_some() {
            let outcome = update_ramp_perpendicular(
                &mut state, (5, 5), Axis::EW, Phase::CollapseA, true,
            );
            assert!(outcome.state_changed);
            let target = state.cell(5, 6).expect("S target");
            assert_eq!(target.damage_state, DamageState::PartialCollapseA);
        }
    }
```

**Implementation note:** `make_perpendicular_test_state` uses the `test_seed_cell` helper added on `BridgeRuntimeState` in Task 1 Step 5. The helper seeds (4,5)/(5,5)/(6,5) in one loop with the same template; insert order doesn't matter since all three are identical. The `make_body_driver_test_state` in Task 4 uses the same helper plus `test_seed_anchor_span`.

**Step 3: Verify**

```
cargo test --lib sim::bridge_specs
cargo build
```

Expected:
- 6 new `update_ramp_perpendicular_*` tests pass; no regressions.
- `cargo build` exits 0.

Parallel-session rule applies.

**Step 4: Commit**

```
git add src/sim/bridge_specs.rs
git commit -m "bridge_specs: add update_ramp_perpendicular wrapper (state-byte branch only; overlay-write deferred)"
```

---

### Task 4: Add `body_cell_advance_state` method + `StateOutcome` enum

**Why:** Final body-cell driver — ties together Tasks 1, 2, 3 plus already-shipped Phase C helpers (`apply_ramp_transition`, `set_bridge_direction`). This is the user-facing entry point that mirrors the body-cell branch of binary `0x576BA0`. Phase F orchestrator (future) calls this on every body-bridge damage event.

**Files:** Modify `src/sim/bridge_state.rs`.

**Pattern:** Method on `BridgeRuntimeState`, mirrors `apply_damage` shape but with richer return type. Composes `update_ramp_perpendicular` + `set_bridge_direction` rather than duplicating their logic.

**Step 1: Add `StateOutcome` enum**

In `src/sim/bridge_state.rs`, add the new public enum near the existing `BridgeStateChange` declaration (around line 179-182). Place it right after `BridgeStateChange`:

```rust
/// Outcome of one `body_cell_advance_state` invocation. Mirrors the return
/// codes of binary `ProcessBridgeDamageStateMachine_High @ 0x576BA0` body
/// branch (0 = absorbed, 1 = collapse), with structured fallout for the
/// orchestrator to dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateOutcome {
    /// Damage absorbed — anchor advanced from `Healthy` to `Damaged`. Bridge
    /// still passable. Renderer should redraw.
    Absorbed,
    /// Anchor collapsed — `damage_state` became `Destroyed`. Cascade actions
    /// for orchestrator follow.
    Collapsed {
        /// Cells whose `damage_state` was set to `Destroyed` in this call
        /// (typically just the anchor; perpendicular targets that hit
        /// collapse-final via `update_ramp_perpendicular` also appear here).
        destroyed_cells: Vec<(u16, u16)>,
        /// `BlowUpBridge` cascade actions emitted by `set_bridge_direction`.
        /// Orchestrator dispatches these (kill ground occupants, Limbo
        /// bridge-deck, spawn debris).
        set_bridge_direction:
            crate::sim::bridge_specs::SetBridgeDirectionResult,
        /// Cells where `UpdateAdjacentBridges_High` should run for rim
        /// re-evaluation. Orchestrator (Phase F Task 27) runs the actual
        /// rim helper.
        adjacent_bridges_dirty: Vec<(u16, u16)>,
        /// Whether the zone graph needs rebuild (`InvalidateBridgeZones` →
        /// `UpdateBridgeZonesHelper`). Orchestrator dispatches.
        zones_dirty: bool,
    },
    /// Cell is not a body-bridge cell, anchor span lookup failed, or anchor
    /// is already `Destroyed`. No-op.
    NoChange,
}
```

**Step 2: Add `body_cell_advance_state` method**

In the same file, in the existing `impl BridgeRuntimeState` block (after `apply_damage` ends around line 462), add:

```rust
    /// Body-cell state-machine driver. Mirrors the body branch of binary
    /// `ProcessBridgeDamageStateMachine_High @ 0x576BA0` (HIGH §3.1).
    ///
    /// Receives damage on a body-bridge cell at `(rx, ry)`. Resolves anchor
    /// (follows `anchor_span_id` if input cell is `Body` or `Tail`), reads
    /// anchor's current `damage_state`, transitions per binary switch arms,
    /// fires perpendicular `UpdateRamp_*` writes via `update_ramp_perpendicular`,
    /// and on collapse emits `set_bridge_direction(span, false)` for the
    /// `BlowUpBridge` cascade.
    ///
    /// Returns `StateOutcome::Absorbed` for `Healthy → Damaged`,
    /// `StateOutcome::Collapsed { ... }` for `Damaged → Destroyed` and
    /// partial-collapse → `Destroyed`, and `StateOutcome::NoChange` for
    /// already-destroyed / non-body / unresolvable-anchor inputs.
    ///
    /// `is_high_bridge` is currently unused (state transitions identical for
    /// HIGH and LOW per HIGH §11.1) but kept for API symmetry with the
    /// future overlay-write branch.
    pub fn body_cell_advance_state(
        &mut self,
        rx: u16,
        ry: u16,
        is_high_bridge: bool,
    ) -> StateOutcome {
        // 1. Resolve input cell.
        let Some(input_cell) = self.cell(rx, ry).copied() else {
            return StateOutcome::NoChange;
        };

        // 2. Filter: must be body-bridge (Anchor / Body / Tail). Bridgehead
        //    cells route to Task 14's bridgehead driver (not part of this plan).
        if !matches!(
            input_cell.role,
            BridgeCellRole::Anchor | BridgeCellRole::Body | BridgeCellRole::Tail
        ) {
            return StateOutcome::NoChange;
        }

        // 3. Resolve anchor.
        let anchor_pos = if matches!(input_cell.role, BridgeCellRole::Anchor) {
            (rx, ry)
        } else {
            // Non-anchor body cell: follow anchor_span_id to span.anchor.
            let Some(span_id) = input_cell.anchor_span_id else {
                return StateOutcome::NoChange;
            };
            let Some(span) = self.anchor_span(span_id) else {
                return StateOutcome::NoChange;
            };
            span.anchor
        };

        let Some(anchor_cell) = self.cell(anchor_pos.0, anchor_pos.1).copied() else {
            return StateOutcome::NoChange;
        };
        let Some(axis) = anchor_cell.axis else {
            return StateOutcome::NoChange;
        };
        let span_id = match anchor_cell.anchor_span_id {
            Some(id) => id,
            None => return StateOutcome::NoChange,
        };
        let span_clone = match self.anchor_span(span_id) {
            Some(s) => s.clone(),
            None => return StateOutcome::NoChange,
        };

        // 4. Switch on anchor's damage_state.
        match anchor_cell.damage_state {
            DamageState::Healthy { .. } => {
                // Anchor advances to Damaged.
                if let Some(c) = self.cell_mut(anchor_pos.0, anchor_pos.1) {
                    c.damage_state = DamageState::Damaged;
                }
                // Fire UpdateRamp_*A and _*B on perpendicular targets.
                let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
                    self, anchor_pos, axis, Phase::DamageA, is_high_bridge,
                );
                let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
                    self, anchor_pos, axis, Phase::DamageB, is_high_bridge,
                );
                StateOutcome::Absorbed
            }
            DamageState::Damaged => {
                // Full collapse — fire CollapseA + CollapseB perpendicular,
                // anchor → Destroyed, set_bridge_direction cascade.
                let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
                    self, anchor_pos, axis, Phase::CollapseA, is_high_bridge,
                );
                let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
                    self, anchor_pos, axis, Phase::CollapseB, is_high_bridge,
                );
                let mut destroyed = vec![anchor_pos];
                if let Some(c) = self.cell_mut(anchor_pos.0, anchor_pos.1) {
                    c.damage_state = DamageState::Destroyed;
                }
                // Collect any perpendicular cells that hit collapse-final
                // (became Destroyed via update_ramp_perpendicular).
                for &perp_dir in &[Direction::E, Direction::W, Direction::N, Direction::S] {
                    let (dx, dy) = perp_dir.offset();
                    let nx = anchor_pos.0 as i32 + dx;
                    let ny = anchor_pos.1 as i32 + dy;
                    if nx < 0 || ny < 0 { continue; }
                    let pos = (nx as u16, ny as u16);
                    if let Some(c) = self.cell(pos.0, pos.1) {
                        if matches!(c.damage_state, DamageState::Destroyed)
                            && !destroyed.contains(&pos)
                        {
                            destroyed.push(pos);
                        }
                    }
                }
                let sbd = crate::sim::bridge_specs::set_bridge_direction(&span_clone, false);
                let adj = compute_adjacent_bridges_dirty(rx, ry, axis);
                StateOutcome::Collapsed {
                    destroyed_cells: destroyed,
                    set_bridge_direction: sbd,
                    adjacent_bridges_dirty: adj,
                    zones_dirty: true,
                }
            }
            DamageState::PartialCollapseA => {
                // Single CollapseA call, then collapse-finalize.
                let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
                    self, anchor_pos, axis, Phase::CollapseA, is_high_bridge,
                );
                if let Some(c) = self.cell_mut(anchor_pos.0, anchor_pos.1) {
                    c.damage_state = DamageState::Destroyed;
                }
                let sbd = crate::sim::bridge_specs::set_bridge_direction(&span_clone, false);
                let adj = compute_adjacent_bridges_dirty(rx, ry, axis);
                StateOutcome::Collapsed {
                    destroyed_cells: vec![anchor_pos],
                    set_bridge_direction: sbd,
                    adjacent_bridges_dirty: adj,
                    zones_dirty: true,
                }
            }
            DamageState::PartialCollapseB => {
                let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
                    self, anchor_pos, axis, Phase::CollapseB, is_high_bridge,
                );
                if let Some(c) = self.cell_mut(anchor_pos.0, anchor_pos.1) {
                    c.damage_state = DamageState::Destroyed;
                }
                let sbd = crate::sim::bridge_specs::set_bridge_direction(&span_clone, false);
                let adj = compute_adjacent_bridges_dirty(rx, ry, axis);
                StateOutcome::Collapsed {
                    destroyed_cells: vec![anchor_pos],
                    set_bridge_direction: sbd,
                    adjacent_bridges_dirty: adj,
                    zones_dirty: true,
                }
            }
            DamageState::Destroyed => StateOutcome::NoChange,
        }
    }
```

**Step 3: Add the helper for adjacent-bridges-dirty cells**

In the same file, add a free function near the bottom of the module (before the `#[cfg(test)] mod tests` block):

```rust
/// Compute the two perpendicular cells where `UpdateAdjacentBridges_High`
/// should fire after a body-cell collapse. Per binary `0x576BA0`, the call
/// passes the ORIGINAL damaged cell coord (not the anchor); the offsets are
/// directional.
fn compute_adjacent_bridges_dirty(rx: u16, ry: u16, axis: Axis) -> Vec<(u16, u16)> {
    let mut out = Vec::with_capacity(2);
    let perpendiculars: [Direction; 2] = match axis {
        Axis::NS => [Direction::E, Direction::W],
        Axis::EW => [Direction::S, Direction::N],
    };
    for d in perpendiculars {
        let (dx, dy) = d.offset();
        let nx = rx as i32 + dx;
        let ny = ry as i32 + dy;
        if nx >= 0 && ny >= 0 {
            out.push((nx as u16, ny as u16));
        }
    }
    out
}
```

**Step 4: Add tests**

In `src/sim/bridge_state.rs` `mod tests`, append at the end (before its closing `}`):

```rust
    fn make_body_driver_test_state() -> BridgeRuntimeState {
        // Uses test_seed_cell + test_seed_anchor_span from Task 1 Step 5.
        // Layout for the body-driver tests:
        //   (5,5)  → anchor cell, axis NS, anchor_span_id=1
        //   (4,5), (6,5) → perpendicular anchor partners (axis NS, separate
        //                  span_id) — UpdateRamp_*A walks E, _*B walks W from
        //                  (5,5), so these are the wrappers' targets.
        //   (5,4)  → non-anchor body cell, anchor_span_id=1 — exercises the
        //                  "follow to anchor" path in the driver.
        // Other slots (7,5), (8,5) are referenced by the AnchorSpan but not
        // seeded — body driver doesn't read them, only the partner indirection
        // and the perpendicular cells.
        let mut state = BridgeRuntimeState::default();

        let healthy_template = BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 0,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::NS),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(1),
            bridgehead_step: 0,
            overlay_byte: 0x18,
        };

        // Anchor at (5,5).
        state.test_seed_cell(5, 5, healthy_template);

        // Perpendicular anchor partners. They are anchors of their own spans
        // (binary's `+0x80` flag is set), so use anchor_span_id=2.
        let perp = BridgeRuntimeCell {
            anchor_span_id: Some(2),
            ..healthy_template
        };
        state.test_seed_cell(4, 5, perp);
        state.test_seed_cell(6, 5, perp);

        // Non-anchor body cell with anchor_span_id=1 — used by
        // body_driver_non_anchor_body_cell_follows_to_anchor.
        state.test_seed_cell(
            5, 4,
            BridgeRuntimeCell {
                role: BridgeCellRole::Body,
                ..healthy_template
            },
        );

        // AnchorSpan registry entry. The driver looks up by anchor_span_id
        // and reads `span.anchor` to resolve. Slot positions beyond (5,5),
        // (4,5), (6,5) aren't seeded as cells because the driver doesn't
        // touch them in the body-cell branch.
        state.test_seed_anchor_span(AnchorSpan {
            id: 1,
            anchor: (5, 5),
            cells: [
                Some((5, 5)), Some((6, 5)), Some((7, 5)),
                Some((8, 5)), Some((4, 5)), None,
            ],
            axis: Axis::NS,
            direction: Direction::E,
            damage_state: DamageState::Healthy { variant: 0 },
            bridge_group_id: 1,
        });

        state
    }

    #[test]
    fn body_driver_anchor_healthy_advances_to_damaged_returns_absorbed() {
        let mut state = make_body_driver_test_state();
        let outcome = state.body_cell_advance_state(5, 5, true);
        assert!(matches!(outcome, StateOutcome::Absorbed));
        assert_eq!(state.cell(5, 5).unwrap().damage_state, DamageState::Damaged);
    }

    #[test]
    fn body_driver_non_anchor_body_cell_follows_to_anchor() {
        let mut state = make_body_driver_test_state();
        // Damage on a body cell, not the anchor.
        let outcome = state.body_cell_advance_state(5, 4, true);
        assert!(matches!(outcome, StateOutcome::Absorbed));
        // Anchor's damage_state advanced, not the input body cell's.
        assert_eq!(state.cell(5, 5).unwrap().damage_state, DamageState::Damaged);
        assert_eq!(state.cell(5, 4).unwrap().damage_state, DamageState::Healthy { variant: 0 });
    }

    #[test]
    fn body_driver_damaged_anchor_collapses_and_emits_set_bridge_direction() {
        let mut state = make_body_driver_test_state();
        state.cell_mut(5, 5).unwrap().damage_state = DamageState::Damaged;
        let outcome = state.body_cell_advance_state(5, 5, true);
        match outcome {
            StateOutcome::Collapsed {
                destroyed_cells,
                set_bridge_direction,
                adjacent_bridges_dirty,
                zones_dirty,
            } => {
                assert!(destroyed_cells.contains(&(5, 5)));
                // 4 BlowUpBridge actions per Task 12 invariant.
                let blow_ups = set_bridge_direction.actions.iter()
                    .filter(|(_, _, a)| matches!(a,
                        crate::sim::bridge_specs::CellAction::BlowUpBridge))
                    .count();
                assert_eq!(blow_ups, 4);
                // 2 perpendicular cells flagged dirty (E and W of (5,5)).
                assert_eq!(adjacent_bridges_dirty.len(), 2);
                assert!(zones_dirty);
            }
            other => panic!("expected Collapsed, got {other:?}"),
        }
        assert_eq!(state.cell(5, 5).unwrap().damage_state, DamageState::Destroyed);
    }

    #[test]
    fn body_driver_partial_collapse_a_collapses_with_single_ramp_call() {
        let mut state = make_body_driver_test_state();
        state.cell_mut(5, 5).unwrap().damage_state = DamageState::PartialCollapseA;
        let outcome = state.body_cell_advance_state(5, 5, true);
        assert!(matches!(outcome, StateOutcome::Collapsed { .. }));
        assert_eq!(state.cell(5, 5).unwrap().damage_state, DamageState::Destroyed);
    }

    #[test]
    fn body_driver_partial_collapse_b_collapses_with_single_ramp_call() {
        let mut state = make_body_driver_test_state();
        state.cell_mut(5, 5).unwrap().damage_state = DamageState::PartialCollapseB;
        let outcome = state.body_cell_advance_state(5, 5, true);
        assert!(matches!(outcome, StateOutcome::Collapsed { .. }));
        assert_eq!(state.cell(5, 5).unwrap().damage_state, DamageState::Destroyed);
    }

    #[test]
    fn body_driver_destroyed_anchor_returns_no_change() {
        let mut state = make_body_driver_test_state();
        state.cell_mut(5, 5).unwrap().damage_state = DamageState::Destroyed;
        let outcome = state.body_cell_advance_state(5, 5, true);
        assert!(matches!(outcome, StateOutcome::NoChange));
    }

    #[test]
    fn body_driver_bridgehead_cell_returns_no_change() {
        let mut state = make_body_driver_test_state();
        state.cell_mut(5, 5).unwrap().role = BridgeCellRole::Bridgehead;
        let outcome = state.body_cell_advance_state(5, 5, true);
        assert!(matches!(outcome, StateOutcome::NoChange));
    }

    #[test]
    fn body_driver_out_of_bounds_returns_no_change() {
        let mut state = make_body_driver_test_state();
        let outcome = state.body_cell_advance_state(99, 99, true);
        assert!(matches!(outcome, StateOutcome::NoChange));
    }
```

**Implementation note:** `make_body_driver_test_state` reuses `test_seed_cell` and `test_seed_anchor_span` from Task 1 Step 5. Layout is minimal: only the cells the body driver actually reads ((5,5) anchor, (4,5)/(6,5) perpendicular partners, (5,4) follow-to-anchor body cell) are seeded. Slots in the AnchorSpan that aren't reached by the body driver ((7,5), (8,5)) are intentionally left unseeded. If a future test needs them, seed inline in that test.

**Step 5: Verify**

```
cargo test --lib sim::bridge_state
cargo test --lib sim::bridge_specs
cargo build
```

Expected:
- 8 new `body_driver_*` tests pass; no regressions in Tasks 1, 2, 3 tests or Phase B/C tests.
- `cargo build` exits 0.

Parallel-session rule applies.

**Step 6: Commit**

```
git add src/sim/bridge_state.rs
git commit -m "bridge_state: add body_cell_advance_state body-cell driver + StateOutcome (matches binary 0x576BA0 body branch; overlay-write branch deferred)"
```

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-07-bridges-tier2-task-13-redesign-design.md](2026-05-07-bridges-tier2-task-13-redesign-design.md)
- **Brainstorm transcript** (this session) — Q1 (scope=B) → Q2 (perpendicular=compute) → Q3 (conversion=methods) → Q4 (overlay storage=A1) → B′ deferral on overlay-write branch
- **Predecessor plan:** [docs/plans/2026-05-07-bridges-tier2-damage-state-machine-plan.md](2026-05-07-bridges-tier2-damage-state-machine-plan.md) (Tier 2 megadoc; this redesign plan replaces its Task 13 section)
- **Ghidra reports:**
  - `ra2-rust-game-docs/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` §3.1 (body branch), §11.1 (UpdateRamp transitions, addressed)
- **gamemd.exe addresses (verified live this session):**
  - `0x00576BA0` `ProcessBridgeDamageStateMachine_High` (body branch decompiled)
  - `0x00572230` `UpdateRamp_NS_DamageA_High` (state-byte gate + overlay branch confirmed)
  - `0x00572440` `UpdateRamp_NS_CollapseA_High` (verified for collapse-final transition)
  - `0x00572DA0` `UpdateRamp_EW_CollapseA_High`, `0x00573170` `UpdateRamp_EW_CollapseB_High` (verified)
  - `0x0057E7A0` `ApplyBridgeDestruction_NS_High`, `0x0057ED00` `_EW_High`, `0x0057DD50` `_NS_Low`, `0x0057E2A0` `_EW_Low` (used by Task 11.5 `pick_destruction_overlay`; not directly called from Task 13's body driver but cited for context)
  - `0x47E040` `SetBridgeDirection_NESW` (used by Task 12 `set_bridge_direction`; called via `set_bridge_direction(span, false)` in Task 4's collapse paths)
- **Runtime-initialized constants (cannot be statically resolved):**
  - `DAT_00abad30`, `DAT_00aa1028`, `DAT_00abc1e8`, `DAT_00aa0e38`, `DAT_00aa0e28` — all zero in static image; observation requires live debugger session. **Blocks the perpendicular overlay-write branch — Task 13.5 follow-up.**
- **Repo patterns:**
  - [src/sim/bridge_state.rs:184-210](../../src/sim/bridge_state.rs#L184) `BridgeRuntimeCell` struct (Phase B `a9d64bc`, `e5cd73d`)
  - [src/sim/bridge_state.rs:424-462](../../src/sim/bridge_state.rs#L424) `BridgeRuntimeState::apply_damage` method (existing single-shot mutation pattern)
  - [src/sim/bridge_specs.rs](../../src/sim/bridge_specs.rs) Phase C pure helpers (`apply_ramp_transition`, `pick_destruction_overlay`, `set_bridge_direction`)
  - [src/sim/world/world_hash.rs:210-237](../../src/sim/world/world_hash.rs#L210) `hash_bridge_state` (Phase B `b5d6a5e`)
- **INI keys:** none new in this plan.
- **Prior commits:**
  - `e5cd73d` Phase B Axis/DamageState/BridgeCellRole/Direction/Phase + AnchorSpan
  - `a9d64bc` Phase B BridgeRuntimeCell extension
  - `6a20959` Phase B anchor walker
  - `b5d6a5e` Phase B world_hash extension
  - `d8f6bd0` Phase B snapshot round-trip test
  - `c9395be` Phase C Task 11 `apply_ramp_transition`
  - `2c8c315` Phase C Task 11.5 `pick_destruction_overlay`
  - `16cf81c` Phase C Task 12 `set_bridge_direction`
