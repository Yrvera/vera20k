# Bridges Tier 2 — Task 13 Redesign (Body-Cell State-Machine Driver)

## Goal

Replace the broken Task 13 plan section with a verified-against-binary design for the body-cell branch of `ProcessBridgeDamageStateMachine_High @ 0x576BA0`, including the perpendicular `UpdateRamp_*_High` side effects (state-byte writes + overlay-class-gated overlay writes) needed for player-visible parity.

## Architecture Context

`BridgeRuntimeState` (`src/sim/bridge_state.rs`) owns mutable bridge runtime state layered on top of `ResolvedTerrainGrid`. Cells are stored in a `Vec<Option<BridgeRuntimeCell>>` indexed by `ry * width + rx`; `anchor_spans: BTreeMap<u16, AnchorSpan>` records one span per anchor cell, built at map load by `walk_anchor_pattern` (Phase B mirror of `SetBridgeDirection_NESW @ 0x47E040`).

Existing entry point `BridgeRuntimeState::apply_damage(BridgeDamageEvent) -> Option<BridgeStateChange>` is the single-shot flag-bit damage model that Tier 2 replaces. Cells in spans tag themselves with `anchor_span_id`, `role` (`Anchor`/`Body`/`Bridgehead`/`Tail`), and `axis`.

Phase C pure helpers already on `dev` and verified against binary:
- `apply_ramp_transition(state_byte, axis, phase) -> Option<u8>` — mirrors the inner state-byte transitions of all 8 `UpdateRamp_*_High/_Low` helpers (commit `c9395be`).
- `pick_destruction_overlay(neighbor_check, axis, is_high_bridge) -> Option<u8>` — mirrors the 4 `ApplyBridgeDestruction_*` 16-entry tables (commit `2c8c315`).
- `set_bridge_direction(span, set) -> SetBridgeDirectionResult` — emits per-cell `BlowUpBridge` / `FlagOnly` actions per binary `0x47E040` (commit `16cf81c`).

The body-cell branch of `0x576BA0` (verified live this session):
1. Filter: damaged cell must have body flag `+0x140 & 0x100`.
2. If `+0x140 & 0x80 == 0` (not anchor), follow `+0x2C` partner pointer to anchor.
3. Switch on anchor's `+0x11E` state byte.
4. **Healthy (states 0..=5 NS, 9..=14 EW)**: anchor state ← 6 (NS) or 0xF (EW). Call `UpdateRamp_*_DamageA(anchor, perpA_dir)` and `UpdateRamp_*_DamageB(anchor, perpB_dir)`. Return 0 (absorbed).
5. **Damaged (state 6 NS, 15 EW)**: call `UpdateRamp_*_CollapseA + B`. Then `SetBridgeDirection_NESW(0, 0)` for NS or `(6, 0)` for EW. Anchor state ← 0; `IsoTileTypeIndex` ← -1. `UpdateAdjacentBridges_High` × 2. `InvalidateBridgeZones` → maybe `UpdateBridgeZonesHelper`. Return 1 (collapsed).
6. **Partial collapse (states 7/8 NS, 16/17 EW)**: single `UpdateRamp_*_Collapse{A or B}` call, then collapse-finalization. Return 1.
7. **Destroyed (state 0 with body flag)** or any other state: return 0 (no-op).

`UpdateRamp_*_High` (verified at `0x00572230` etc.) walks one cell from passed coord in given direction (perpendicular to bridge axis), then on the target cell does two independent things:
- **Anchor-flag-gated state byte transition** — only if `target.flags & 0x80` is set. Transitions per `apply_ramp_transition`.
- **IsoTileTypeIndex-class-gated overlay write** — independent of the anchor flag. Pavement-class targets get `ToggleBridgePavement`; bridgehead+0/+2 class targets get `SetOverlayAndPropagate(target, base + class_offset + BridgeSet, …)`.

## Impact Analysis

**Predecessor (Task 12.5):**
- `src/sim/bridge_state.rs` — add `pub overlay_byte: u8` to `BridgeRuntimeCell`. Populate at map-load from `resolved.bridge_layer.overlay_id` (already present on `ResolvedTerrainCell`).
- `src/sim/world/world_hash.rs` — extend cell hash to include `overlay_byte`.
- `src/sim/world/world_tests.rs` — extend snapshot round-trip assertion.
- Renderer `display_tile` (Phase D, out of scope here) — will read this once Tier 2 lands.

**Task 13:**
- `src/sim/bridge_state.rs` — `DamageState::to_state_byte(self, axis) -> u8` + `from_state_byte(byte) -> Option<Self>`. New method `BridgeRuntimeState::body_cell_advance_state(&mut self, rx, ry, is_high_bridge, terrain) -> StateOutcome`.
- `src/sim/bridge_specs.rs` — add `update_ramp_perpendicular(state, terrain, anchor_pos, axis, phase, is_high_bridge)` wrapper. Reuses already-shipped `apply_ramp_transition` for state-byte transitions.

**Risk areas:**
- **Anchor partner lookup:** body driver follows `cell.anchor_span_id` → `state.anchor_span(id).anchor` when input cell is `Body` or `Tail`. Verify all body cells have a populated `anchor_span_id` after map-load (Phase B).
- **Multi-borrow:** `&mut self` (BridgeRuntimeState) + `&ResolvedTerrainGrid` is fine (different objects). Body driver reads anchor-span data into locals before mutating cells, to avoid `&self.anchor_spans` + `&mut self.cells` overlap.
- **Determinism:** body driver is purely state-machine; no RNG. Outer-gate RNG (BridgeStrength + IonCannon retry) lives at the orchestrator boundary (Phase F). Method invocations happen in a deterministic per-event loop.
- **Out-of-scope side effects:** `UpdateAdjacentBridges_High` (rim re-eval) and `UpdateBridgeZonesHelper` (zone graph rebuild) are emitted as flags in `StateOutcome::Collapsed { adjacent_bridges_dirty, zones_dirty }`. Orchestrator (Phase F) consumes them. Don't implement those primitives in this task.
- **Bridgehead branch interaction:** Task 13 implements only the body branch of `0x576BA0`. Bridgehead branch is Task 14. Outer entry `apply_area_damage` dispatches to body vs bridgehead; today's task assumes the dispatcher already correctly chose body.

## Chosen Approach

**Single full-parity body-cell driver, predecessor schema task, on-the-fly perpendicular lookup, conversion methods on `DamageState`.**

Settled by brainstorm Q1–Q4:
- **Q1 (scope):** Option B — full parity, single task. Smaller alternatives leave a player-visible perpendicular-overlay drift that fires every match with bridge combat.
- **Q2 (perpendicular partner):** Compute on the fly via `Direction::offset()`. Mirrors binary's `g_DirectionOffsets[dir & 7]` walk. No schema growth; no map-load partner-validation pass.
- **Q3 (DamageState ↔ u8):** Methods on `DamageState`. Reusable for snapshot/hash debug, future bridgehead driver, future overlay-derivation code. The collapse-final ambiguity (state 0 = `Healthy{0}` initial OR `Destroyed` post-collapse) is resolved at the call site by phase + prior-state context.
- **Q4 (overlay byte):** Option A1 — store `overlay_byte: u8` on `BridgeRuntimeCell` as a predecessor task (12.5), separate commit. Schema extension is independently testable (round-trip, hash, map-load population) and isolates schema risk from state-machine logic risk.

## Tiny-Detail Ledger

Each item must be preserved in implementation; cited source.

| # | Detail | Source | Implementation home |
|---|---|---|---|
| 1 | Damaged cell that's not anchor follows `+0x2C` partner ptr to anchor before any work | `[GHIDRA 0x576BA0]` | Body driver: `if cell.role != Anchor { anchor = state.anchor_span(cell.anchor_span_id?)?.anchor }` |
| 2 | NS axis state range 0..=8; EW axis state range 9..=17 | `[GHIDRA 0x576BA0 switch]` + doc HIGH §3.1 | `DamageState::to_state_byte(self, axis)` |
| 3 | Healthy → Damaged anchor write: NS state ← 6, EW state ← 0xF | `[GHIDRA 0x576BA0 case 0..5 / 9..14]` | Body driver: `anchor.damage_state = Damaged` |
| 4 | UpdateRamp dispatch directions: NS uses dirs 2 (E), 6 (W); EW uses dirs 4 (S), 0 (N). A-side called first, B-side second. | `[GHIDRA 0x576BA0]` | Body driver call sites |
| 5 | UpdateRamp inner state-byte gate: only mutates target if `target.flags & 0x80` (target is anchor) | `[GHIDRA UpdateRamp_NS_DamageA_High @ 0x00572230]` | UpdateRamp wrapper: `if target.role == Anchor { ... }` |
| 6 | UpdateRamp state-byte transitions per axis × phase | already shipped (Task 11, c9395be) | `apply_ramp_transition` (verified live) |
| 7 | UpdateRamp overlay-write branch: target IsoTileTypeIndex class — pavement → ToggleBridgePavement, bridgehead+0 → SetOverlayAndPropagate(+0+BridgeSet), bridgehead+2 → SetOverlayAndPropagate(+2+BridgeSet) | `[GHIDRA 0x00572230 second branch]` | **Deferred to Task 13.5** (follow-up). Blocked on runtime observation of `DAT_00abad30 / DAT_00aa1028 / DAT_00abc1e8 / DAT_00aa0e38 / DAT_00aa0e28` — all zero in static binary image, populated at game init from rules data. |
| 8 | Collapse-final write set: anchor state ← 0, IsoTileTypeIndex ← -1, role ← (still Anchor; downstream sees damage_state = Destroyed) | `[GHIDRA 0x576BA0 LAB_0057778a, LAB_005778cc]` | Body driver finalize block |
| 9 | SetBridgeDirection_NESW called with `(0, 0)` for NS collapse, `(6, 0)` for EW collapse | `[GHIDRA 0x576BA0 LAB_0057778a vs after-switch]` | Body driver invokes `set_bridge_direction(span, false)` (already shipped, Task 12) |
| 10 | UpdateAdjacentBridges_High called twice on collapse, with directional offsets from anchor | `[GHIDRA 0x576BA0 MapClass__UpdateAdjacentBridges_High calls]` | Body driver emits `adjacent_bridges_dirty: [(u16,u16); 2]` for orchestrator |
| 11 | InvalidateBridgeZones called; if returns true → UpdateBridgeZonesHelper | `[GHIDRA 0x576BA0]` | Body driver emits `zones_dirty: bool` flag |
| 12 | Return value: 0 = damage absorbed (Healthy → Damaged), 1 = collapse | `[GHIDRA 0x576BA0 return statements]` | `StateOutcome::Absorbed` vs `StateOutcome::Collapsed` |
| 13 | Damaged state on Destroyed cell: no-op | `[GHIDRA 0x576BA0 default case]` | `DamageState::Destroyed => StateOutcome::NoChange` |
| 14 | Body driver only fires when damaged cell has `flags & 0x100` (bridge body) | `[GHIDRA 0x576BA0 entry filter]` | Outer entry asserts `cell.role` ∈ {`Anchor`, `Body`, `Tail`}; bridgehead falls through to Task 14 |
| 15 | UpdateAdjacentBridges target: passed the ORIGINAL damaged cell coord, not the anchor | `[GHIDRA 0x576BA0]` | Body driver passes original `(rx, ry)`, not anchor pos, in `adjacent_bridges_dirty` |
| 16 | Overlay byte for healthy bridge tiles is the `bridge_layer.overlay_id` from terrain | `[bridge_layer.overlay_id]` | 12.5 map-load init |
| 17 | Partial collapse states 7/17 fire CollapseA only; 8/16 fire CollapseB only | `[GHIDRA 0x576BA0 case 7, 8, 16, 17]` | Body driver: separate match arms for `PartialCollapseA` / `PartialCollapseB` |
| 18 | DamageState ↔ state byte bijection per axis: Healthy{var} ↔ var (NS) / 9+var (EW); Damaged ↔ 6 / 15; PartialCollapseA ↔ 7 / 17; PartialCollapseB ↔ 8 / 16 | `[doc HIGH §3.1]` + `apply_ramp_transition` docstring | `DamageState::to_state_byte(self, axis)` table |

## Design

### Components

**1. Schema extension (Task 12.5):**
```rust
// src/sim/bridge_state.rs
pub struct BridgeRuntimeCell {
    // ... existing fields ...
    /// Per-cell overlay byte (CellClass+0x44 in binary). Mutated by
    /// UpdateRamp_*_High overlay-write branch and by ApplyBridgeDestruction_*.
    /// Renderer reads this to pick visible tile.
    pub overlay_byte: u8,
}
```

**2. State-byte conversion methods (Task 13):**
```rust
// src/sim/bridge_state.rs
impl DamageState {
    /// Encode to binary state byte (CellClass+0x11E).
    /// Note: `Destroyed` always maps to 0 — caller must distinguish from
    /// `Healthy{variant: 0}` via context (e.g., post-collapse vs initial).
    pub fn to_state_byte(self, axis: Axis) -> u8 { ... }

    /// Decode from binary state byte. Returns None for invalid bytes.
    /// Note: byte 0 maps to `Healthy{variant: 0}`; collapse-final detection
    /// is the caller's responsibility.
    pub fn from_state_byte(byte: u8) -> Option<Self> { ... }
}
```

**3. UpdateRamp wrapper (Task 13):**
```rust
// src/sim/bridge_specs.rs
pub fn update_ramp_perpendicular(
    state: &mut BridgeRuntimeState,
    terrain: &ResolvedTerrainGrid,
    anchor_pos: (u16, u16),
    axis: Axis,
    phase: Phase,           // DamageA / DamageB / CollapseA / CollapseB
    is_high_bridge: bool,
) -> RampOutcome { ... }

pub struct RampOutcome {
    pub state_changed: bool,
    pub overlay_written: Option<u8>,
}
```

Internally:
- Computes target = `anchor_pos + perpendicular_direction(axis, phase).offset()`.
- Looks up `state.cell(target)` and `terrain.cell(target)`.
- Branch 1: if `target.role == Anchor`, convert `target.damage_state.to_state_byte(axis)`, call `apply_ramp_transition(byte, axis, phase)`, write `target.damage_state = DamageState::from_state_byte(next)?`.
- Branch 2: classify `terrain.cell(target).bridge_layer.overlay_id` against pavement/bridgehead-class set. Write `target.overlay_byte` accordingly. (Pavement toggle and bridgehead+0/+2 are distinct sub-cases.)

**4. Body-cell driver (Task 13):**
```rust
// src/sim/bridge_state.rs
impl BridgeRuntimeState {
    pub fn body_cell_advance_state(
        &mut self,
        rx: u16,
        ry: u16,
        is_high_bridge: bool,
        terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome { ... }
}

#[derive(Debug, Clone)]
pub enum StateOutcome {
    Absorbed,
    Collapsed {
        destroyed_cells: Vec<(u16, u16)>,
        set_bridge_direction: SetBridgeDirectionResult,
        adjacent_bridges_dirty: Vec<(u16, u16)>,
        zones_dirty: bool,
    },
    NoChange,
}
```

### Interfaces / Contracts

- `body_cell_advance_state` is the single Task 13 entry point. Caller (Phase F orchestrator's `apply_area_damage`) gates damage and dispatches; on body-cell hit it invokes this method once per damage event.
- Returns `StateOutcome::NoChange` if cell is not body-bridge, has no `anchor_span_id`, or anchor is `Destroyed`. No errors — invariants are enforced by Phase B map-load.
- `Collapsed.destroyed_cells` lists every cell whose `damage_state` was set to `Destroyed` in this call (typically the anchor; orchestrator uses this for ground-occupant kill in later tasks).
- `Collapsed.adjacent_bridges_dirty` and `zones_dirty` are pure flags — orchestrator triggers `UpdateAdjacentBridges_High` and zone-grid rebuild in dedicated post-pass tasks (Tasks 27, 28).

### Data Flow

```
apply_area_damage (Phase F)
  ├─ outer gate (BridgeStrength RNG, IonCannon retry — Phase F scope)
  ├─ classify cell (body vs bridgehead)
  └─ body branch:
       body_cell_advance_state(rx, ry, is_high, terrain)
         ├─ resolve anchor (cell.role == Anchor or follow anchor_span_id)
         ├─ read anchor.damage_state
         ├─ match damage_state:
         │   ├─ Healthy → anchor.damage_state = Damaged;
         │   │            update_ramp_perpendicular(A, phase=DamageA);
         │   │            update_ramp_perpendicular(B, phase=DamageB);
         │   │            return Absorbed
         │   ├─ Damaged → update_ramp_perpendicular(A, phase=CollapseA);
         │   │            update_ramp_perpendicular(B, phase=CollapseB);
         │   │            anchor.damage_state = Destroyed;
         │   │            sbd = set_bridge_direction(span, set=false);
         │   │            return Collapsed { … }
         │   ├─ PartialCollapseA → update_ramp_perpendicular(A, phase=CollapseA);
         │   │                     finalize as Collapsed
         │   ├─ PartialCollapseB → update_ramp_perpendicular(B, phase=CollapseB);
         │   │                     finalize as Collapsed
         │   └─ Destroyed → return NoChange
         └─ on Collapsed: emit adjacent_bridges_dirty (perpendicular offsets ±N/S or ±E/W from anchor), zones_dirty = true
```

### Error Handling

No `Result` types. Invalid cell positions, missing anchor spans, or out-of-range state bytes produce `StateOutcome::NoChange`. Phase B map-load establishes invariants (every body cell has an anchor span; every anchor span has a valid anchor). Anything outside those invariants is a sim bug, not a runtime error — assertions belong in `debug_assert!` for dev builds.

### Testing Strategy

**Task 12.5 tests:**
- Round-trip snapshot serialization preserves `overlay_byte`.
- World hash differs when `overlay_byte` differs.
- Map-load populates `overlay_byte` correctly from `bridge_layer.overlay_id` for a fixture grid covering body, bridgehead, and pavement classes.

**Task 13 tests** (in `src/sim/bridge_state.rs` `mod tests`):
- `to_state_byte` / `from_state_byte` bijection for every variant × axis × valid byte.
- Body driver: each switch arm of `0x576BA0` covered. NS healthy → Damaged absorbed. NS damaged → Collapsed with `set_bridge_direction` actions. NS partial collapse A/B → Collapsed with single ramp call. EW counterparts. Destroyed cell no-op. Non-anchor body cell follows anchor_span_id correctly.
- `update_ramp_perpendicular`: target with `role == Anchor` and `state byte 0..=3` → state advances to 4 on DamageA. Target without anchor flag → no state byte change. Target with bridgehead+0 IsoTileType → overlay written. Target with non-pavement non-bridgehead class → no overlay write.
- Determinism: same `(state, sequence of damage events)` → same final state hash across runs.

## Architectural Decisions

**Patterns followed:**
- Method on `BridgeRuntimeState` (matches existing `apply_damage` shape).
- Pure helpers in `bridge_specs.rs`, mutating method in `bridge_state.rs` (matches Phase B/C precedent).
- `set_bridge_direction(span, false)` is composed in, not duplicated.
- `apply_ramp_transition` is composed in via the new `update_ramp_perpendicular` wrapper, not duplicated.
- Conversion via methods on the type, not free functions or wrapping types.
- Side effects emitted as flags / lists for the orchestrator to consume; no upstream coupling to `UpdateAdjacentBridges` or zone refresh primitives that don't exist yet.

**Patterns deviated from:** none.

**Tech debt introduced:** none. The `overlay_byte` field is permanent runtime state that maps directly to binary `+0x44`, not a workaround.

**Determinism:** preserved. No RNG in Task 13 scope; switch-arm logic is total over `(DamageState × Axis)`; perpendicular lookup is deterministic from `anchor + Direction::offset()`.

## Alternatives Considered

- **Compute overlay byte from `(damage_state, axis, role, neighbor_state)` at render time, no storage** (Q4 Option B). Rejected: reproduces `CheckBridgeNeighbors_*` logic at render-time, couples renderer to bridge state-machine internals (sim ⊥ render boundary violation per CLAUDE.md), doesn't cleanly handle pavement-toggle or sibling-propagation writes.
- **Defer overlay storage; ship Task 13 with state-byte writes only** (Q4 Option C). Rejected: visible parity gap on every bridge damage event near a bridge end (every collapse and most damage hits in normal play). Violates 99% parity bar.
- **Add `partner_a/b: Option<(u16,u16)>` to `BridgeRuntimeCell` for perpendicular partner pre-computation** (Q2 Option 2). Rejected: schema bloat (~0.5 MB on max grid), solves a problem we don't have (perpendicular lookup is one arithmetic + Vec index, called from one driver, deterministic from `anchor + Direction::offset()`).
- **Typed wrapper `apply_ramp_transition_typed(DamageState, Axis, Phase) -> Option<DamageState>`** (Q3 Option B). Rejected as primary mechanism — kept available as a future thin wrapper if call sites get noisy. Conversion methods on the type are needed regardless (snapshot/hash, future bridgehead driver).
- **Split Task 13 into 13a (anchor state + cascade) + 13b (perpendicular UpdateRamp side effects)** (Q1 Option C). Rejected: the binary's natural unit is the whole body branch; splitting creates an artificial seam where 13a leaves anchor state in a half-transitioned shape that never matches the binary on its own.
- **Inline state-byte ↔ DamageState conversion at each call site** (Q3 Option C). Rejected: same conversion will appear in Task 14 (bridgehead driver), snapshot consistency checks, and overlay propagation; one place beats N.
