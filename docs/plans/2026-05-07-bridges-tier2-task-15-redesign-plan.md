# Bridges Tier 2 — Task 15 Redesign Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.
> Do not skip the parallel-session rule (CLAUDE.md): if `cargo build` or
> `cargo test` fails in a file you didn't modify, stop and report.

**Goal:** Implement the bridgehead-cell branch of `ProcessBridgeDamageStateMachine_High @ 0x576BA0` end-to-end (4-slot bridgehead → 2-hit destroy + perpendicular `UpdateRamp_*` writes via `update_ramp_perpendicular` + 3-cell `BlowUpBridge` body-axis-aligned row). The visible overlay-byte progression (`SetOverlayAndPropagate(anchor, ABAD30+offset+BridgeSet)`) and 10-slot debris loop are deferred to a follow-up task (Task 15.5) — same blocker as body driver's deferred Task 13.5: runtime-init globals (`DAT_00abad30`, `DAT_00aa1028`, `DAT_00aa0e28`) are zero in the static binary image.

**Architecture:** Continues Phase C bridgehead-branch parity, paralleling the body-driver landing (5478e17 → 9711833 → 20b8fdc). Three small commits: (1) drop the dead `bridgehead_step` field and migrate bridgehead state representation to the unified `damage_state: DamageState`, (2) add a pure `bridgehead_blow_up_row` helper for the 3-cell BlowUp geometry, (3) add the `bridgehead_advance_state` driver method composing existing helpers (`bridgehead_walk_to_anchor`, `update_ramp_perpendicular`, `compute_adjacent_bridges_dirty`) plus the new helper.

**Design Doc:** [docs/plans/2026-05-07-bridges-tier2-task-15-redesign-design.md](2026-05-07-bridges-tier2-task-15-redesign-design.md)

---

## Grounding Summary

- **R1 — ra2-rust-game-docs:** Primary source `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` §3.2 (bridgehead-cell branch), §11.1 (8 `UpdateRamp_*_High` helpers' state transitions), §11.4 (`CellClass::BlowUpBridge` complete behavior), §13.5 (correction: `+0x11A` is bridge-class ID, not generic height — relevant for height-predicate semantics on the walker). All addresses in §11.5 / §11.1.
- **R2 — Ghidra verifications (live this session):** `ProcessBridgeDamageStateMachine_High @ 0x576BA0` decompiled in full; bridgehead-branch case-arms enumerated; **two doc corrections caught and incorporated**:
  - **Step progression is 2-hit, not 4-step.** Steps 0/1/2 (any cosmetic healthy variant) all transition in one shot to step 3 (write overlay slot 2 raw → next iVar2 read = ABAD30+3 = collapse trigger). Step 3 → collapse. Same shape as body driver's `Healthy → Damaged → Destroyed`.
  - **3-cell BlowUp row is body-axis-aligned, NOT perpendicular** (§3.2 / §11.1 wording was sloppy). NS: column at `(anchor.X or anchor.X-1, anchor.Y±1)`. EW: row at `(anchor.X±1, anchor.Y or anchor.Y-1)`. Offset of which column/row chosen depends on the anchor's height-bit predicate (`h&1` for NS, `h<5` for EW) — same predicates as `bridgehead_walk_to_anchor`.
  - **Bridgehead branch does NOT call `SetBridgeDirection_NESW`** (verified: zero `SetBridgeDirection_NESW` references in the bridgehead branch decompile; doc HIGH §3.2 also lists none). The body span survives ramp destruction with state byte advanced one tier via the perpendicular `UpdateRamp_*_Collapse` call — multi-stage destruction mechanic.
- **R3 — Repo patterns:**
  - **Body driver shape** ([bridge_state.rs:616-754](../../src/sim/bridge_state.rs#L616-L754), commit 20b8fdc): method on `BridgeRuntimeState`, `&mut self`, returns `StateOutcome`. The bridgehead driver mirrors this exactly except for the anchor-resolution path (height-predicate walker instead of `anchor_span_id` lookup) and the cascade shape (3-cell row instead of `set_bridge_direction(span, false)`).
  - **Pure helper precedent** for `bridgehead_blow_up_row`: `bridgehead_walk_to_anchor` ([bridge_specs.rs:608-647](../../src/sim/bridge_specs.rs#L608-L647), commit 2474058), `compute_adjacent_bridges_dirty` ([bridge_state.rs:950-965](../../src/sim/bridge_state.rs#L950-L965)). All emit `(u16, u16)` cell coords with map-edge-clamping logic.
  - **`StateOutcome` reuse**: existing enum ([bridge_state.rs:238-265](../../src/sim/bridge_state.rs#L238-L265)) carries body driver's collapse shape. Bridgehead emits the same enum with `set_bridge_direction.actions = 3 BlowUpBridge entries` (the field's misnomer for bridgehead is documented in the driver docstring).
  - **Field-removal precedent**: no exact match in recent history, but field-additions in commits `a9d64bc` (extend cell), `42b16e1` (add `overlay_byte`), `b5d6a5e` (extend hash) show the touch-points: struct definition, `from_resolved_terrain` Pass 1+2+3, world_hash, test fixtures, plus serde derive (auto-handled).
- **R4 — INI keys:** None new for this plan. Bridge state-machine constants are runtime-initialized; no INI exposure. `BridgeStrength`, `DestroyableBridges` already parsed; their consumers are out of scope (Phase F orchestrator).

**Unknowns (deferred):**
- Visible overlay-byte progression on the anchor cell (`SetOverlayAndPropagate(anchor, ABAD30+2+BridgeSet)` for damage and `+3+BridgeSet` for collapse). Same blocker as body Task 13.5 — needs runtime observation of `DAT_00abad30 / DAT_00aa1028 / DAT_00aa0e28`. Spawn follow-up task once observed.
- 10-slot debris-anim spawn loop on collapse. Out of scope for this plan; orchestrator (Phase F) can attach later.
- Variant fidelity for bridgehead's initial `Healthy { variant: 0..=2 }` from initial overlay slot. Currently seeded as `variant: 0` regardless. Recovered in Task 15.5 once `ABAD30` is observed live.
- **Height-source field on `ResolvedTerrainCell`.** Per HIGH §13.5, binary's `+0x11A` is "bridge-class ID" (values 4=NS body, 5=NS ramp, 7=NS bridgehead variant, 8=NS high-ramp peak; 2=EW body, 0xC=EW high-ramp peak), NOT generic height. The shipped `bridgehead_walk_to_anchor` helper takes a closure consumer-decided. Existing helper tests use template-height-like values (8, 6, 4, 2, 0) which suggests `template_height` semantics. The driver in Task 3 passes `terrain.cell(pos).map(|c| c.template_height)` and flags this in its docstring as "approximation; §13.5 verification deferred to a future RE pass." If parity tests reveal a discrepancy, the closure source moves to a derived `bridge_class_id` field on `ResolvedTerrainCell`.

## Key Technical Decisions

| Decision | Rationale | Confidence | Source |
|---|---|---|---|
| Three commits (drop field → add helper → add driver), parallel to body driver shipping | Mirrors body driver cadence (5478e17 → 9711833 → 20b8fdc); each commit independently `cargo test` green; clean revert path | high | repo precedent: body driver split |
| Drop `bridgehead_step` entirely; reuse `damage_state: DamageState` for both body and bridgehead roles | Field is currently dead (never written, always 0). Unified queryable state — `is_bridge_walkable` works without two-field sync. Variant fidelity for bridgehead's 3 cosmetic variants gets recovered when `ABAD30` is observed live. | high | brainstorm Q2; verified `bridgehead_step` is dead in repo (5 init sites, 0 mutation sites) |
| `bridgehead_advance_state` is a method on `BridgeRuntimeState` (not free function) | Mirrors `body_cell_advance_state` shape — `&mut self`, returns `StateOutcome`, single orchestrator-facing entry point | high | repo pattern: [bridge_state.rs:616](../../src/sim/bridge_state.rs#L616) |
| `bridgehead_blow_up_row` is a pure free function in `bridge_specs.rs` | Phase C pure-helpers location; matches `bridgehead_walk_to_anchor` and `apply_ramp_transition` placement | high | repo pattern: [bridge_specs.rs:351,608](../../src/sim/bridge_specs.rs#L351) |
| 3-cell BlowUp row is BODY-AXIS-ALIGNED (not perpendicular). Per-axis × per-height-predicate geometry table in helper. | Live `[GHIDRA 0x576BA0]` step-3 branch, both NS and EW even/odd cases verified. §3.2/§11.1 wording was wrong; binary is the spec. | high | `[GHIDRA 0x576BA0]` decompile this session |
| Bridgehead branch does NOT compose `set_bridge_direction(anchor.span, false)` | Binary doesn't call it; body span must survive ramp destruction with state byte advanced one tier (multi-stage destruction). Adding it would over-collapse. | high | `[GHIDRA 0x576BA0]` (zero `SetBridgeDirection_NESW` calls) + HIGH §3.2 |
| `StateOutcome::Collapsed.set_bridge_direction` reused as the cascade-list carrier; bridgehead populates with 3 BlowUpBridge entries (no SBD call) | Saves a parallel cascade type; orchestrator iterates `result.actions` regardless of source. Field-name misnomer documented in docstring. | medium | design Q4 alternative; chosen for shipping simplicity |
| `update_ramp_perpendicular` already shipped; bridgehead reuses it as-is. | Identical anchor-perpendicular call pattern as body branch; no new state-byte primitive needed. | high | already shipped (commit 9711833) |
| Driver takes `&ResolvedTerrainGrid` parameter for height-predicate access | `bridgehead_walk_to_anchor` already takes a closure; same pattern. Avoids storing redundant terrain data on `BridgeRuntimeCell`. | high | repo pattern: [bridge_specs.rs:608](../../src/sim/bridge_specs.rs#L608) |
| Height-source field is `ResolvedTerrainCell.template_height` for now | Existing `bridgehead_walk_to_anchor` tests use template-height-like values; closure consumer-decides | medium | inferred from existing test fixtures; **flagged for /review-plan** + post-impl parity check |

## Open Questions

### Resolved During Planning

- *"How many state representations does bridgehead need?"* — One: unified `damage_state: DamageState`. Drop `bridgehead_step`. (Brainstorm Q2.)
- *"Does bridgehead branch call `SetBridgeDirection_NESW`?"* — No. Verified live `0x576BA0` decompile: zero calls. Binary leaves anchor-span flag bits untouched; body span survives. (R2 verification.)
- *"Is the BlowUp row geometry perpendicular or body-axis-aligned?"* — Body-axis-aligned. NS: column. EW: row. Offset by anchor height-bit. (R2 verification — doc §3.2 was wrong.)
- *"4-step or 2-hit progression?"* — 2-hit. Any healthy variant (slot 0/1/2) → step 3 → collapse. (R2 verification.)
- *"Should the driver be split, monolithic, or merged with body driver?"* — Monolithic, separate from body driver. (Brainstorm Q1.)

### Deferred to Implementation

- *"Anchor's `damage_state` after bridgehead collapse — is it set explicitly?"* — No: per binary, only the perpendicular UpdateRamp targets get state-byte writes. Driver must NOT write anchor.damage_state, only bridgehead.damage_state. Tests assert this explicitly.
- *"Visible parity gap from deferred overlay-write."* — When a bridgehead is hit Healthy→Damaged or Damaged→Collapsed, the bridgehead's own visible overlay byte does NOT update in this plan's output. Player sees stale healthy overlay until Task 15.5 lands. Acknowledged drift; tracked.
- *"Height-source field: `template_height` vs derived `bridge_class_id`."* — Plan ships with `template_height`. If the existing `bridgehead_walk_to_anchor` test fixtures (heights 8, 6, 4, 2, 0) match runtime template_height for high bridges, parity holds. If not, follow-up task adds a `bridge_class_id` field to `ResolvedTerrainCell` and re-points the driver's closure. Flagged for `/review-plan`.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/sim/bridge_state.rs` | **Task 1**: drop `pub bridgehead_step: u8` field on `BridgeRuntimeCell`; remove the doc-comment reference on `anchor_span_id`; remove field from 4 in-file init/test sites (`from_resolved_terrain` Pass 1 line 388, Pass 3 line 470, test fixtures lines 1241, 1262, 1366). **Task 3**: add `BridgeRuntimeState::bridgehead_advance_state` method + tests. |
| Modify | `src/sim/bridge_specs.rs` | **Task 1**: drop `bridgehead_step: 0` from 2 test fixtures (lines 1162, 1246). **Task 2**: add `bridgehead_blow_up_row` pure free function + tests. |
| Modify | `src/sim/world/world_hash.rs` | **Task 1**: drop `cell.bridgehead_step.hash(hasher);` (line 229) and `bridgehead_step: 0` from test fixture (line 563). |

## Interface Changes

**Public:**
- `BridgeRuntimeCell` loses `pub bridgehead_step: u8`. Field was unused outside the struct definition; no consumers break. Snapshot deserialization of pre-existing snapshots fails — acceptable per CLAUDE.md (no production save format yet) and matches previous Tier 2 schema deltas (commits a9d64bc, 42b16e1).
- `BridgeRuntimeState::bridgehead_advance_state(&mut self, rx: u16, ry: u16, is_high_bridge: bool, terrain: &ResolvedTerrainGrid) -> StateOutcome` — new method.
- `bridge_specs::bridgehead_blow_up_row(anchor_pos: (u16, u16), axis: Axis, anchor_height: u8, map_width: u16, map_height: u16) -> [Option<(u16, u16)>; 3]` — new free function.

**Internal:** `anchor_span_id` docstring updated to remove `bridgehead_step` reference (Task 1).

## Sim Checklist

- [x] All math uses `fixed`-point — no f32/f64. State byte is `u8`, height predicate is `u8`, transitions are integer-pure.
- [x] New state included in deterministic state hash — Task 1 *removes* a field from `hash_bridge_state`; the field was always 0 so the hash output is unchanged for any current state.
- [x] No dependencies on render/ui/sidebar/audio/net — driver takes `&ResolvedTerrainGrid` (map module) and `&mut BridgeRuntimeState`.
- [x] Tick ordering impact noted — `bridgehead_advance_state` is a deterministic state transition; called from Phase F orchestrator (not part of this plan). No new RNG draws.
- [x] BTreeMap iteration order considered — driver does not iterate `anchor_spans`; reads `cell.axis` directly and walks via `bridgehead_walk_to_anchor`. No iteration ordering concerns.

## Risk Areas

- **`bridgehead_step` field removal** breaks any in-flight dev snapshots. Acceptable per CLAUDE.md.
- **Anchor's `damage_state` left untouched on bridgehead collapse.** Easy to get wrong by mirroring body driver's `anchor.damage_state = Destroyed` write. Test asserts `anchor.damage_state == Healthy{0}` (or whatever it was prior) AFTER bridgehead collapse.
- **Perpendicular UpdateRamp writes state byte ≥ 4 (Healthy variant 4/5) on Healthy→Damaged path, NOT PartialCollapseA/B.** A/B cooperative damage advances the perpendicular's state byte 0..=3 → 4 (DamageA) or 5 (DamageB); pair converges to state byte 6 = Damaged after both A and B fire. Tests must assert the exact variant transition, not "PartialCollapseA/B".
- **3-cell BlowUp row geometry per height-bit:** four distinct cases (NS even/odd, EW low/high). All four have unit tests in Task 2.
- **Height-source field `template_height` is a working approximation.** If parity tests against an in-game scenario reveal incorrect anchor walks, follow-up RE task adds a derived `bridge_class_id` field to `ResolvedTerrainCell`. Flagged for /review-plan.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|---|---|---|---|
| Task 3 | Bridgehead `Healthy → Damaged` transitions to single Damaged tier in one hit (any cosmetic variant 0/1/2 jumps); fires UpdateRamp DamageA + DamageB on perpendicular targets | First damage hit on a bridgehead must always lead to "next hit collapses" — off-by-one would mean 3+ hits to destroy any bridgehead | Unit tests assert `bridgehead.damage_state == Damaged` after first hit (regardless of starting variant); `update_ramp_perpendicular` returned `state_changed=true` for both A and B perpendicular targets |
| Task 3 | Bridgehead `Damaged → Destroyed`: 3-cell BlowUp row + perpendicular UpdateRamp CollapseA + CollapseB + adjacent_bridges_dirty + zones_dirty | The collapse cascade is the entire visible "ramp falls down" effect. Wrong row geometry = wrong visible cells get debris/occupant kill | Unit test asserts `StateOutcome::Collapsed { set_bridge_direction.actions has 3 BlowUpBridge entries with coords matching the per-axis × per-height table in `bridgehead_blow_up_row`, adjacent_bridges_dirty has 2 perpendiculars of *bridgehead* coord, zones_dirty: true }` |
| Task 3 | Bridgehead branch does NOT modify anchor's `damage_state` directly | Setting anchor.damage_state = Destroyed would make `is_bridge_walkable(anchor)` false even though body span should survive — ramp-only destruction breaks | Unit test seeds anchor at `Healthy{0}`, runs bridgehead collapse, asserts `anchor.damage_state == Healthy{0}` after |
| Task 3 | Bridgehead branch does NOT compose `set_bridge_direction(anchor.span, false)` | Calling SBD would BlowUpBridge the entire anchor span (4-5 body cells) on top of the 3-cell ramp row — over-collapse, parity drift | Unit test asserts `set_bridge_direction.actions.len() == 3` (only the row, no anchor-span 4-5 entries) |
| Task 3 | `adjacent_bridges_dirty` uses BRIDGEHEAD's coord (not anchor's) | Wrong coord = wrong perpendicular cells get rim re-eval; visible cosmetic drift on the bridgehead-adjacent terrain | Unit test asserts `adjacent_bridges_dirty == compute_adjacent_bridges_dirty(rx, ry, axis)` where (rx, ry) is the bridgehead input, not anchor |
| Task 2 | 3-cell BlowUp row geometry per axis × height-bit (4 cases): NS even h&1==0 → column at anchor.X; NS odd → column at anchor.X-1; EW h<5 → row at anchor.Y; EW h>=5 → row at anchor.Y-1 | Wrong geometry = blowing up wrong cells (wrong occupants killed, wrong debris locations) | Unit test for each of 4 cases against fixture coords |
| Task 1 | `bridgehead_step` field removal is fully mechanical; no behavior change | Determinism within post-commit code preserved (hash output will differ from pre-commit by one byte per cell, but no consumer compares cross-version hashes) | Snapshot round-trip determinism: serialize state post-commit, deserialize, recompute world hash → identical hash within post-commit code |

---

## Tasks

### Task 1: Drop `bridgehead_step` field; migrate to unified `damage_state` representation

**Why:** Phase B added `bridgehead_step: u8` ([bridge_state.rs:292](../../src/sim/bridge_state.rs#L292), commit a9d64bc) anticipating the bridgehead driver. Brainstorm Q2 settled on a unified `damage_state` for both body and bridgehead, so `bridgehead_step` is now dead weight (5 init sites, 0 mutation sites). Remove it before adding the driver so commit 3 doesn't introduce two parallel state representations.

**Files:** Modify `src/sim/bridge_state.rs`, `src/sim/bridge_specs.rs`, `src/sim/world/world_hash.rs`.

**Pattern:** Mirrors recent Phase B/C field migrations — touch struct, all init sites, world_hash, test fixtures.

**Step 1: Drop the struct field**

In [src/sim/bridge_state.rs](../../src/sim/bridge_state.rs), locate `pub struct BridgeRuntimeCell` (currently line 268). Remove the `bridgehead_step` field (line 292) and its preceding doc comment (lines 290-291):

Delete:
```rust
    /// Bridgehead 4-step progression counter (0..=3). Only meaningful when
    /// `role == BridgeCellRole::Bridgehead`.
    pub bridgehead_step: u8,
```

**Step 2: Update neighbouring docstring**

In the same struct, the `anchor_span_id` field (currently line 286-288) has a docstring that references `bridgehead_step`:
```rust
    /// Stable ID of containing `AnchorSpan` (for body cells); `None` for
    /// bridgehead cells (which use `bridgehead_step` instead).
    pub anchor_span_id: Option<u16>,
```

Replace with:
```rust
    /// Stable ID of containing `AnchorSpan` (for body cells); `None` for
    /// bridgehead cells.
    pub anchor_span_id: Option<u16>,
```

**Step 3: Drop init at Pass 1 (BFS bridge-deck loop)**

In `BridgeRuntimeState::from_resolved_terrain`, locate Pass 1's `cells[idx] = Some(BridgeRuntimeCell { ... });` (currently around line 379). Remove the `bridgehead_step: 0,` line (line 388). The struct literal becomes:

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
                    overlay_byte: resolved
                        .bridge_layer
                        .as_ref()
                        .map(|bl| bl.overlay_id)
                        .unwrap_or(0),
                });
```

**Step 4: Drop write at Pass 3 (bridgehead classifier)**

In the same `from_resolved_terrain`, Pass 3 (currently line 451-473), the bridgehead-tagging block writes `c.bridgehead_step = 0;` (line 470). Remove that line:

Before:
```rust
            if let Some(c) = cells[idx].as_mut() {
                c.role = BridgeCellRole::Bridgehead;
                c.anchor_span_id = None;
                c.bridgehead_step = 0;
                c.axis = Some(bridge_direction_to_axis(bl.direction));
            }
```

After:
```rust
            if let Some(c) = cells[idx].as_mut() {
                c.role = BridgeCellRole::Bridgehead;
                c.anchor_span_id = None;
                c.axis = Some(bridge_direction_to_axis(bl.direction));
            }
```

**Step 5: Drop field from in-file test fixtures**

Three test fixtures in `mod tests` of [src/sim/bridge_state.rs](../../src/sim/bridge_state.rs) construct `BridgeRuntimeCell` literals with `bridgehead_step: 0,`:

- Line ~1241: in some test setup helper.
- Line ~1262: in another test fixture.
- Line ~1366: in another.

For each, remove the `bridgehead_step: 0,` line from the struct literal. Use search-and-replace to find them — the literal `bridgehead_step: 0,` is unique enough. The exact line numbers may shift slightly between sessions; locate by pattern.

**Step 6: Drop field from `bridge_specs.rs` test fixtures**

In [src/sim/bridge_specs.rs](../../src/sim/bridge_specs.rs), two test fixtures have `bridgehead_step: 0,` (lines 1162, 1246). Remove the field from both `BridgeRuntimeCell` literals.

**Step 7: Drop field from world_hash**

In [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs), `hash_bridge_state` (currently line 210-238) has the line `cell.bridgehead_step.hash(hasher);` (line 229). Remove that line. The hash loop becomes:

```rust
        for ((rx, ry), cell) in entries {
            rx.hash(hasher);
            ry.hash(hasher);
            cell.deck_present.hash(hasher);
            cell.damage_state.hash(hasher);
            cell.destroyable.hash(hasher);
            cell.deck_level.hash(hasher);
            cell.bridge_group_id.hash(hasher);
            cell.axis.hash(hasher);
            cell.role.hash(hasher);
            cell.anchor_span_id.hash(hasher);
            cell.overlay_byte.hash(hasher);
        }
```

Also, `world_hash.rs`'s test fixture (currently line 563) constructs `BridgeRuntimeCell` with `bridgehead_step: 0,`. Remove that line from the literal.

**Step 8: Verify**

Run:
```
cargo build --lib
cargo test --lib sim::bridge -- --nocapture
cargo test --lib sim::world::world_hash -- --nocapture
```

Expected: all green. Hash output will differ from pre-commit (one fewer byte hashed per cell — every `Hash::hash` call contributes to the input byte sequence regardless of value), but determinism within post-commit code is preserved: any state freshly constructed post-commit hashes deterministically, and lockstep peers all run post-commit code so they agree. No production save format depends on hash continuity (per CLAUDE.md).

**Step 9: Commit**

```
git add src/sim/bridge_state.rs src/sim/bridge_specs.rs src/sim/world/world_hash.rs
git commit -m "$(cat <<'EOF'
bridge_state: drop dead bridgehead_step field; migrate to unified damage_state

bridgehead_step was added in a9d64bc anticipating the bridgehead driver but
remained dead state (5 init sites, 0 mutation sites). Brainstorm Q2 settled
on damage_state: DamageState for both body and bridgehead roles — Healthy
variants for cosmetic slots, Damaged for ready-to-collapse, Destroyed
post-collapse. Removes the field, all init sites, world_hash entry, and
test fixtures. Unblocks Task 15 driver landing without two parallel state
representations.
EOF
)"
```

---

### Task 2: Add `bridgehead_blow_up_row` pure helper

**Why:** Bridgehead-collapse final step blows up 3 cells in a body-axis-aligned row (NS column or EW row), with offset depending on anchor's height-bit predicate. Verified live `[GHIDRA 0x576BA0]` step-3 branch — corrects the §3.2/§11.1 "perpendicular_row" wording. Pure helper testable in isolation; consumed by Task 3's driver.

**Files:** Modify `src/sim/bridge_specs.rs`.

**Pattern:** Mirrors `bridgehead_walk_to_anchor` ([bridge_specs.rs:608](../../src/sim/bridge_specs.rs#L608)) and `compute_adjacent_bridges_dirty` ([bridge_state.rs:950](../../src/sim/bridge_state.rs#L950)) — pure free function returning fixed-size result with `Option` for off-map cells.

**Step 1: Add helper**

In [src/sim/bridge_specs.rs](../../src/sim/bridge_specs.rs), append the helper function immediately after `bridgehead_walk_to_anchor` (which currently ends around line 647, before `#[cfg(test)] mod tests`):

```rust
/// Three cells receiving `BlowUpBridge` on bridgehead final-step collapse.
/// Geometry verified live `[GHIDRA 0x576BA0]` step-3 branch (this session).
///
/// Body-axis-aligned 3-cell row (NOT perpendicular — corrects §3.2/§11.1
/// "perpendicular_row" wording). Offset to which row/column is chosen
/// depends on `anchor_height`'s parity bit:
///
/// | Axis | `anchor_height` predicate | Row geometry                                              |
/// |------|---------------------------|-----------------------------------------------------------|
/// | NS   | `h & 1 == 0` (even)       | column at `anchor.X`,    Y in `{anchor.Y-1, anchor.Y, anchor.Y+1}` |
/// | NS   | `h & 1 != 0` (odd)        | column at `anchor.X-1`,  Y in `{anchor.Y-1, anchor.Y, anchor.Y+1}` |
/// | EW   | `h < 5`                   | row    at `anchor.Y`,    X in `{anchor.X-1, anchor.X, anchor.X+1}` |
/// | EW   | `h >= 5`                  | row    at `anchor.Y-1`,  X in `{anchor.X-1, anchor.X, anchor.X+1}` |
///
/// Off-map cells return `None` and are skipped by the caller.
///
/// `anchor_height` is `ResolvedTerrainCell.template_height` (or whatever
/// the consumer of `bridgehead_walk_to_anchor` uses for its closure).
pub fn bridgehead_blow_up_row(
    anchor_pos: (u16, u16),
    axis: Axis,
    anchor_height: u8,
    map_width: u16,
    map_height: u16,
) -> [Option<(u16, u16)>; 3] {
    let (anchor_x, anchor_y) = (anchor_pos.0 as i32, anchor_pos.1 as i32);
    let (col_x, row_y) = match axis {
        Axis::NS => {
            let x_offset = if anchor_height & 1 == 0 { 0 } else { -1 };
            (anchor_x + x_offset, anchor_y)
        }
        Axis::EW => {
            let y_offset = if anchor_height < 5 { 0 } else { -1 };
            (anchor_x, anchor_y + y_offset)
        }
    };
    let mut out: [Option<(u16, u16)>; 3] = [None; 3];
    for (i, delta) in [-1i32, 0, 1].iter().enumerate() {
        let (cx, cy) = match axis {
            Axis::NS => (col_x, row_y + delta),
            Axis::EW => (col_x + delta, row_y),
        };
        if cx >= 0 && cy >= 0 && (cx as u16) < map_width && (cy as u16) < map_height {
            out[i] = Some((cx as u16, cy as u16));
        }
    }
    out
}
```

**Step 2: Add tests**

In the `mod tests` block (currently ends around line 1321), append before the closing `}`:

```rust
    #[test]
    fn bridgehead_blow_up_row_ns_even_height() {
        // NS even (h & 1 == 0): column at anchor.X = 5
        let row = bridgehead_blow_up_row((5, 5), Axis::NS, 4, 10, 10);
        assert_eq!(row[0], Some((5, 4))); // anchor.Y - 1
        assert_eq!(row[1], Some((5, 5))); // anchor.Y
        assert_eq!(row[2], Some((5, 6))); // anchor.Y + 1
    }

    #[test]
    fn bridgehead_blow_up_row_ns_odd_height() {
        // NS odd (h & 1 != 0): column at anchor.X - 1 = 4
        let row = bridgehead_blow_up_row((5, 5), Axis::NS, 5, 10, 10);
        assert_eq!(row[0], Some((4, 4)));
        assert_eq!(row[1], Some((4, 5)));
        assert_eq!(row[2], Some((4, 6)));
    }

    #[test]
    fn bridgehead_blow_up_row_ew_low_height() {
        // EW h < 5: row at anchor.Y = 5
        let row = bridgehead_blow_up_row((5, 5), Axis::EW, 2, 10, 10);
        assert_eq!(row[0], Some((4, 5))); // anchor.X - 1
        assert_eq!(row[1], Some((5, 5))); // anchor.X
        assert_eq!(row[2], Some((6, 5))); // anchor.X + 1
    }

    #[test]
    fn bridgehead_blow_up_row_ew_high_height() {
        // EW h >= 5: row at anchor.Y - 1 = 4
        let row = bridgehead_blow_up_row((5, 5), Axis::EW, 5, 10, 10);
        assert_eq!(row[0], Some((4, 4)));
        assert_eq!(row[1], Some((5, 4)));
        assert_eq!(row[2], Some((6, 4)));
    }

    #[test]
    fn bridgehead_blow_up_row_clamps_off_map_cells() {
        // NS even at left edge: anchor.X = 0, anchor.Y = 0.
        // anchor.Y - 1 = -1 → None.
        let row = bridgehead_blow_up_row((0, 0), Axis::NS, 4, 10, 10);
        assert_eq!(row[0], None);          // (0, -1) off map
        assert_eq!(row[1], Some((0, 0)));
        assert_eq!(row[2], Some((0, 1)));
    }

    #[test]
    fn bridgehead_blow_up_row_clamps_negative_x_for_ns_odd() {
        // NS odd at X=0: column at -1 → all None.
        let row = bridgehead_blow_up_row((0, 5), Axis::NS, 5, 10, 10);
        assert_eq!(row[0], None);
        assert_eq!(row[1], None);
        assert_eq!(row[2], None);
    }

    #[test]
    fn bridgehead_blow_up_row_clamps_at_map_max() {
        // NS even at X = map_width - 1: column at map_width - 1, ok.
        // Y at top edge: y+1 = map_height, off map.
        let row = bridgehead_blow_up_row((9, 9), Axis::NS, 4, 10, 10);
        assert_eq!(row[0], Some((9, 8)));
        assert_eq!(row[1], Some((9, 9)));
        assert_eq!(row[2], None); // (9, 10) off map
    }
```

**Step 3: Verify**

Run:
```
cargo test --lib sim::bridge_specs::tests::bridgehead_blow_up -- --nocapture
```

Expected: 7 tests pass.

**Step 4: Commit**

```
git add src/sim/bridge_specs.rs
git commit -m "$(cat <<'EOF'
bridge_specs: add bridgehead_blow_up_row — body-axis 3-cell row geometry per axis x height-bit (verified live 0x576BA0)
EOF
)"
```

---

### Task 3: Add `bridgehead_advance_state` driver method

**Why:** Bridgehead state-machine driver. Counterpart to `body_cell_advance_state` (commit 20b8fdc). Composes `bridgehead_walk_to_anchor`, `update_ramp_perpendicular`, `bridgehead_blow_up_row`, `compute_adjacent_bridges_dirty`. Mirrors the bridgehead-cell branch of `0x576BA0` end-to-end on the state-machine side. Overlay-byte progression and 10-slot debris loop deferred to Task 15.5.

**Files:** Modify `src/sim/bridge_state.rs`.

**Pattern:** Mirrors `body_cell_advance_state` ([bridge_state.rs:616-754](../../src/sim/bridge_state.rs#L616-L754), commit 20b8fdc) — `&mut self` method on `BridgeRuntimeState`, returns `StateOutcome`, defensive `NoChange` on edge cases.

**Step 1: Add the driver method**

In [src/sim/bridge_state.rs](../../src/sim/bridge_state.rs), in the `impl BridgeRuntimeState` block, append the new method immediately after `body_cell_advance_state` (currently ends around line 754, just before `pub fn endpoint_records`):

```rust
    /// Bridgehead-cell state-machine driver. Mirrors the bridgehead branch of
    /// binary `ProcessBridgeDamageStateMachine_High @ 0x576BA0` (HIGH §3.2,
    /// verified live).
    ///
    /// Counterpart to `body_cell_advance_state`. Filters on
    /// `role == Bridgehead`, walks to the anchor body cell via
    /// `bridgehead_walk_to_anchor`'s height predicate (NS rejects `h&1`,
    /// walks until `h==4`; EW rejects `h>4`, walks until `h==2`), then
    /// transitions per the cell's `damage_state`. Fires perpendicular
    /// `UpdateRamp_*` writes via `update_ramp_perpendicular` exactly like
    /// the body driver. On collapse: emits a 3-cell `BlowUpBridge` row
    /// (body-axis-aligned via `bridgehead_blow_up_row`) plus
    /// `adjacent_bridges_dirty` flags for orchestrator-side rim re-eval.
    ///
    /// **Critical structural difference vs body branch:** does NOT compose
    /// `set_bridge_direction(anchor.span, false)`. Binary leaves the body
    /// span's flag bits untouched on bridgehead destruction; the body span
    /// survives with state byte advanced one tier via the perpendicular
    /// `UpdateRamp_*_Collapse` call. Subsequent damage on the body cells
    /// continues the collapse via `body_cell_advance_state`.
    ///
    /// Returns:
    /// - `StateOutcome::Absorbed` — bridgehead `Healthy → Damaged` (any of
    ///   the 3 cosmetic healthy variants jump-transitions to the single
    ///   Damaged tier in one hit; mirror of the binary writing overlay
    ///   slot 2 raw which encodes step 3 on next read).
    /// - `StateOutcome::Collapsed { destroyed_cells, set_bridge_direction
    ///   (3-entry BlowUpBridge row — note: field name is reused from body
    ///   driver; bridgehead does NOT call SetBridgeDirection_NESW),
    ///   adjacent_bridges_dirty (perpendiculars of the *bridgehead* coord),
    ///   zones_dirty: true }` — bridgehead `Damaged → Destroyed`.
    /// - `StateOutcome::NoChange` — non-bridgehead role, anchor walk
    ///   failed (off-map / odd-height intermediate / `cell.axis == None`),
    ///   already `Destroyed`, or `PartialCollapseA/B` (body-only states,
    ///   defensive).
    ///
    /// `is_high_bridge` is currently unused (state transitions identical
    /// for HIGH and LOW per HIGH §11.1) but kept for API symmetry with the
    /// future overlay-write branch (Task 15.5) and the body driver.
    pub fn bridgehead_advance_state(
        &mut self,
        rx: u16,
        ry: u16,
        is_high_bridge: bool,
        terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid,
    ) -> StateOutcome {
        let _ = is_high_bridge;
        // 1. Resolve input cell.
        let Some(input_cell) = self.cell(rx, ry).copied() else {
            return StateOutcome::NoChange;
        };

        // 2. Filter: must be a Bridgehead. Body/Anchor/Tail cells route to
        //    body driver.
        if !matches!(input_cell.role, BridgeCellRole::Bridgehead) {
            return StateOutcome::NoChange;
        }
        let Some(axis) = input_cell.axis else {
            return StateOutcome::NoChange;
        };

        // 3. Walk to anchor via height predicate.
        // Closure source: ResolvedTerrainCell.template_height. Per HIGH
        // §13.5 the binary field +0x11A is "bridge-class ID"; if the
        // template_height-based walk shows discrepancies in parity tests,
        // a derived bridge_class_id field on ResolvedTerrainCell will
        // replace this closure source.
        let map_w = self.width;
        let map_h = self.height;
        let height_lookup = |pos: (u16, u16)| -> Option<u8> {
            terrain.cell(pos.0, pos.1).map(|c| c.template_height)
        };
        // Walk direction: same as anchor_walk_direction(axis) used by
        // map-load anchor-pattern walker — NS = E (dir 2), EW = S (dir 4).
        let walk_dir = match axis {
            Axis::NS => Direction::E,
            Axis::EW => Direction::S,
        };
        let Some(anchor_pos) = crate::sim::bridge_specs::bridgehead_walk_to_anchor(
            (rx, ry), axis, walk_dir, height_lookup, map_w, map_h,
        ) else {
            return StateOutcome::NoChange;
        };

        // 4. Switch on bridgehead's damage_state.
        match input_cell.damage_state {
            DamageState::Healthy { .. } => {
                // Hit 1: any cosmetic healthy variant jumps to Damaged.
                // Fire DamageA + DamageB on anchor's perpendicular partners.
                let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
                    self, anchor_pos, axis, Phase::DamageA, is_high_bridge,
                );
                let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
                    self, anchor_pos, axis, Phase::DamageB, is_high_bridge,
                );
                if let Some(c) = self.cell_mut(rx, ry) {
                    c.damage_state = DamageState::Damaged;
                }
                StateOutcome::Absorbed
            }
            DamageState::Damaged => {
                // Hit 2: collapse.
                // Read anchor height for blow-up row predicate.
                let anchor_height =
                    terrain.cell(anchor_pos.0, anchor_pos.1)
                        .map(|c| c.template_height)
                        .unwrap_or(0);

                // Compute the 3-cell BlowUpBridge row.
                let row = crate::sim::bridge_specs::bridgehead_blow_up_row(
                    anchor_pos, axis, anchor_height, map_w, map_h,
                );

                // Fire perpendicular CollapseA + CollapseB.
                let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
                    self, anchor_pos, axis, Phase::CollapseA, is_high_bridge,
                );
                let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
                    self, anchor_pos, axis, Phase::CollapseB, is_high_bridge,
                );

                // Bridgehead's own state → Destroyed (binary writes overlay
                // slot 3 raw, our model maps to Destroyed). NOTE: anchor's
                // damage_state is NOT modified — body span survives with
                // state byte advanced via the perpendicular UpdateRamp call.
                if let Some(c) = self.cell_mut(rx, ry) {
                    c.damage_state = DamageState::Destroyed;
                }

                // Collect any perpendicular cells that hit collapse-final
                // (became Destroyed via update_ramp_perpendicular). Mirror
                // of body driver convention.
                let mut destroyed = vec![(rx, ry)];
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

                // Emit the 3-cell BlowUpBridge row as a SetBridgeDirectionResult.
                // Bridgehead branch does NOT call SetBridgeDirection_NESW;
                // we reuse the result type as a cascade carrier for the
                // orchestrator. Slot index is set to 0 for all 3 entries —
                // bridgehead's 3-cell row is not part of an AnchorSpan, so
                // the slot has no per-anchor meaning here.
                let actions: Vec<(
                    (u16, u16),
                    usize,
                    crate::sim::bridge_specs::CellAction,
                )> = row
                    .iter()
                    .filter_map(|c| {
                        c.map(|cell| {
                            (cell, 0usize, crate::sim::bridge_specs::CellAction::BlowUpBridge)
                        })
                    })
                    .collect();
                let sbd = crate::sim::bridge_specs::SetBridgeDirectionResult { actions };

                let adj = compute_adjacent_bridges_dirty(rx, ry, axis);
                StateOutcome::Collapsed {
                    destroyed_cells: destroyed,
                    set_bridge_direction: sbd,
                    adjacent_bridges_dirty: adj,
                    zones_dirty: true,
                }
            }
            DamageState::PartialCollapseA
            | DamageState::PartialCollapseB
            | DamageState::Destroyed => StateOutcome::NoChange,
        }
    }
```

**Step 2: Add tests**

At the bottom of `mod tests` in [src/sim/bridge_state.rs](../../src/sim/bridge_state.rs) (currently ends around line 1499), append before the closing `}`. The tests use the existing test-only helpers `test_seed_cell` and `test_seed_anchor_span`. The terrain fixtures are minimal `ResolvedTerrainGrid` instances with `template_height` set to drive the walk.

```rust
    /// Build a minimal ResolvedTerrainGrid for bridgehead driver tests.
    /// 5x5 grid; bridgehead at (2,2) with template_height 8 (NS peak),
    /// anchor at (4,2) with template_height 4 (NS body). Walk from (2,2)
    /// east through (3,2) [template_height=6] to (4,2) [template_height=4].
    fn make_bridgehead_terrain_ns() -> crate::map::resolved_terrain::ResolvedTerrainGrid {
        use crate::map::resolved_terrain::ResolvedTerrainCell;
        use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
        let mut cells = Vec::with_capacity(25);
        for ry in 0..5u16 {
            for rx in 0..5u16 {
                // Heights along Y=2: (0,2)=10, (1,2)=10, (2,2)=8 (bridgehead),
                // (3,2)=6, (4,2)=4 (anchor body).
                let template_height: u8 = if ry == 2 {
                    match rx {
                        0 | 1 => 10,
                        2 => 8,
                        3 => 6,
                        4 => 4,
                        _ => 0,
                    }
                } else {
                    0
                };
                cells.push(ResolvedTerrainCell {
                    rx, ry,
                    source_tile_index: 0, source_sub_tile: 0,
                    final_tile_index: 0, final_sub_tile: 0,
                    level: 0, filled_clear: true, tileset_index: None,
                    land_type: 0, slope_type: 0, template_height,
                    render_offset_x: 0, render_offset_y: 0,
                    terrain_class: TerrainClass::Clear,
                    speed_costs: SpeedCostProfile::default(),
                    is_water: false, is_cliff_like: false, is_rough: false,
                    is_road: false, accepts_smudge: true,
                    is_cliff_redraw: false, variant: 0,
                    has_ramp: false, canonical_ramp: None,
                    ground_walk_blocked: false, terrain_object_blocks: false,
                    overlay_blocks: false, zone_type: 0,
                    base_ground_walk_blocked: false, base_build_blocked: false,
                    build_blocked: false, has_bridge_deck: false,
                    bridge_walkable: false, bridge_transition: false,
                    bridge_deck_level: 0, bridge_layer: None,
                    radar_left: [0, 0, 0], radar_right: [0, 0, 0],
                });
            }
        }
        crate::map::resolved_terrain::ResolvedTerrainGrid::from_cells(5, 5, cells)
    }

    /// Seed a minimal bridge state with: bridgehead at (2,2) NS, anchor at
    /// (4,2) NS, perpendicular partner anchor at (4,1) NS (north of anchor —
    /// for DamageB / CollapseB walker direction). All cells start
    /// `Healthy{0}`.
    fn make_bridgehead_state_ns() -> BridgeRuntimeState {
        let mut state = BridgeRuntimeState::default();
        state.test_seed_cell(2, 2, BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 0,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::NS),
            role: BridgeCellRole::Bridgehead,
            anchor_span_id: None,
            overlay_byte: 0x18,
        });
        state.test_seed_cell(4, 2, BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 0,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::NS),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(1),
            overlay_byte: 0x20,
        });
        // East perpendicular partner of anchor (DamageA/CollapseA walks E):
        state.test_seed_cell(5, 2, BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 0,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::NS),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(1),
            overlay_byte: 0x21,
        });
        // West perpendicular partner of anchor (DamageB/CollapseB walks W):
        state.test_seed_cell(3, 2, BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 0,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::NS),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(1),
            overlay_byte: 0x22,
        });
        state
    }

    #[test]
    fn bridgehead_advance_healthy_to_damaged_ns() {
        let mut state = make_bridgehead_state_ns();
        let terrain = make_bridgehead_terrain_ns();
        let outcome = state.bridgehead_advance_state(2, 2, true, &terrain);
        assert_eq!(outcome, StateOutcome::Absorbed);
        // Bridgehead's damage_state advances from Healthy to Damaged in
        // one hit (any cosmetic variant collapses to Damaged).
        let bh = state.cell(2, 2).expect("bridgehead present");
        assert_eq!(bh.damage_state, DamageState::Damaged);
        // Anchor (4,2) is NOT mutated — only perpendicular partners.
        let anchor = state.cell(4, 2).expect("anchor present");
        assert_eq!(anchor.damage_state, DamageState::Healthy { variant: 0 });
        // East partner (5,2) — DamageA wrote state byte 4 → Healthy{4}.
        let east = state.cell(5, 2).expect("east partner present");
        assert_eq!(east.damage_state, DamageState::Healthy { variant: 4 });
        // West partner (3,2) — DamageB wrote state byte 5 → Healthy{5}.
        let west = state.cell(3, 2).expect("west partner present");
        assert_eq!(west.damage_state, DamageState::Healthy { variant: 5 });
    }

    #[test]
    fn bridgehead_advance_damaged_to_destroyed_ns() {
        let mut state = make_bridgehead_state_ns();
        // Pre-set bridgehead to Damaged.
        state.cell_mut(2, 2).unwrap().damage_state = DamageState::Damaged;
        let terrain = make_bridgehead_terrain_ns();
        let outcome = state.bridgehead_advance_state(2, 2, true, &terrain);
        match outcome {
            StateOutcome::Collapsed {
                destroyed_cells,
                set_bridge_direction,
                adjacent_bridges_dirty,
                zones_dirty,
            } => {
                // Bridgehead is destroyed; anchor is NOT.
                assert!(destroyed_cells.contains(&(2, 2)));
                assert_eq!(
                    state.cell(2, 2).unwrap().damage_state,
                    DamageState::Destroyed
                );
                assert_eq!(
                    state.cell(4, 2).unwrap().damage_state,
                    DamageState::Healthy { variant: 0 },
                    "anchor's damage_state must not be modified by bridgehead collapse"
                );
                // Perpendicular partners advance to PartialCollapseA / B.
                // CollapseA from anchor walks E to (5,2): state 0 → 7 = PartialCollapseA.
                // CollapseB from anchor walks W to (3,2): state 0 → 8 = PartialCollapseB.
                assert_eq!(
                    state.cell(5, 2).unwrap().damage_state,
                    DamageState::PartialCollapseA
                );
                assert_eq!(
                    state.cell(3, 2).unwrap().damage_state,
                    DamageState::PartialCollapseB
                );
                // 3-cell BlowUp row: anchor (4,2) template_height=4 (even),
                // NS axis → column at anchor.X=4, Y in {1, 2, 3}.
                assert_eq!(set_bridge_direction.actions.len(), 3);
                let blow_cells: Vec<(u16, u16)> = set_bridge_direction
                    .actions
                    .iter()
                    .map(|(c, _, _)| *c)
                    .collect();
                assert!(blow_cells.contains(&(4, 1)));
                assert!(blow_cells.contains(&(4, 2)));
                assert!(blow_cells.contains(&(4, 3)));
                // adjacent_bridges_dirty uses bridgehead's coord (2,2),
                // axis NS → perpendiculars E/W → (3,2) and (1,2).
                let adj_set: std::collections::BTreeSet<(u16, u16)> =
                    adjacent_bridges_dirty.iter().copied().collect();
                assert!(adj_set.contains(&(3, 2)));
                assert!(adj_set.contains(&(1, 2)));
                assert!(zones_dirty);
            }
            _ => panic!("expected Collapsed, got {:?}", outcome),
        }
    }

    #[test]
    fn bridgehead_advance_destroyed_no_change() {
        let mut state = make_bridgehead_state_ns();
        state.cell_mut(2, 2).unwrap().damage_state = DamageState::Destroyed;
        let terrain = make_bridgehead_terrain_ns();
        let outcome = state.bridgehead_advance_state(2, 2, true, &terrain);
        assert_eq!(outcome, StateOutcome::NoChange);
    }

    #[test]
    fn bridgehead_advance_non_bridgehead_role_no_change() {
        let mut state = make_bridgehead_state_ns();
        // Pre-set the cell to Body role.
        state.cell_mut(2, 2).unwrap().role = BridgeCellRole::Body;
        let terrain = make_bridgehead_terrain_ns();
        let outcome = state.bridgehead_advance_state(2, 2, true, &terrain);
        assert_eq!(outcome, StateOutcome::NoChange);
    }

    #[test]
    fn bridgehead_advance_anchor_walk_failure_no_change() {
        // Same state but terrain has no path to h=4 — all heights 10.
        let mut state = make_bridgehead_state_ns();
        let mut terrain = make_bridgehead_terrain_ns();
        for c in terrain.cells.iter_mut() {
            c.template_height = 10;
        }
        let outcome = state.bridgehead_advance_state(2, 2, true, &terrain);
        assert_eq!(outcome, StateOutcome::NoChange);
        // Bridgehead's damage_state is unchanged.
        assert_eq!(
            state.cell(2, 2).unwrap().damage_state,
            DamageState::Healthy { variant: 0 }
        );
    }

    #[test]
    fn bridgehead_advance_partial_collapse_states_no_change() {
        // PartialCollapseA/B are body-only states; bridgehead never reaches
        // them. Defensive NoChange.
        let mut state = make_bridgehead_state_ns();
        let terrain = make_bridgehead_terrain_ns();
        for partial in [DamageState::PartialCollapseA, DamageState::PartialCollapseB] {
            state.cell_mut(2, 2).unwrap().damage_state = partial;
            let outcome = state.bridgehead_advance_state(2, 2, true, &terrain);
            assert_eq!(outcome, StateOutcome::NoChange);
        }
    }

    #[test]
    fn bridgehead_advance_off_map_no_change() {
        let mut state = make_bridgehead_state_ns();
        let terrain = make_bridgehead_terrain_ns();
        let outcome = state.bridgehead_advance_state(99, 99, true, &terrain);
        assert_eq!(outcome, StateOutcome::NoChange);
    }
```

**Step 3: Verify**

Run:
```
cargo test --lib sim::bridge_state::tests::bridgehead_advance -- --nocapture
```

Expected: 7 tests pass.

Then run full bridge regression:
```
cargo test --lib sim::bridge -- --nocapture
```

Expected: all green. Body driver tests still pass (no shared state).

**Step 4: Commit**

```
git add src/sim/bridge_state.rs
git commit -m "$(cat <<'EOF'
bridge_state: add bridgehead_advance_state driver — 2-hit destroy + 3-cell BlowUp row + perpendicular UpdateRamp (matches binary 0x576BA0 bridgehead branch; overlay-write deferred to Task 15.5)
EOF
)"
```

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-07-bridges-tier2-task-15-redesign-design.md](2026-05-07-bridges-tier2-task-15-redesign-design.md)
- **Ghidra reports:** [ra2-rust-game-docs/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md](../../ra2-rust-game-docs/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md) §3.2 (bridgehead-cell branch), §11.1 (8 UpdateRamp helpers' state transitions), §11.4 (BlowUpBridge complete behavior), §13.5 (`+0x11A` is bridge-class ID correction).
- **gamemd.exe addresses:** `ProcessBridgeDamageStateMachine_High @ 0x00576BA0`, `UpdateRamp_NS_DamageA_High @ 0x00572230`, `UpdateRamp_NS_DamageB_High @ 0x00572330`, `UpdateRamp_NS_CollapseA_High @ 0x00572440`, `UpdateRamp_NS_CollapseB_High @ 0x005727E0`, `UpdateRamp_EW_DamageA_High @ 0x00572B80`, `UpdateRamp_EW_DamageB_High @ 0x00572C90`, `UpdateRamp_EW_CollapseA_High @ 0x00572DA0`, `UpdateRamp_EW_CollapseB_High @ 0x00573170`, `CellClass::BlowUpBridge @ 0x0047DD70`. Runtime-init globals: `DAT_00abad30` (NS bridgehead overlay class base), `DAT_00aa1028` (EW bridgehead overlay class base), `DAT_00aa0e28` (BridgeSet base).
- **INI keys:** none new for this plan. `BridgeStrength`, `DestroyableBridges` already parsed (Tier 1).
- **Related code:** [src/sim/bridge_state.rs](../../src/sim/bridge_state.rs), [src/sim/bridge_specs.rs](../../src/sim/bridge_specs.rs), [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs), [src/map/resolved_terrain.rs](../../src/map/resolved_terrain.rs).
- **Prior commits:** `e5cd73d` (enums), `a9d64bc` (BridgeRuntimeCell extension — adds the now-being-removed `bridgehead_step`), `6a20959` (anchor walker), `b5d6a5e` (world_hash extension), `d8f6bd0` (snapshot round-trip), `c9395be` (`apply_ramp_transition`), `2c8c315` (`pick_destruction_overlay`), `16cf81c` (`set_bridge_direction`), `42b16e1` (`overlay_byte` field), `5478e17` (`DamageState ↔ state_byte`), `9711833` (`update_ramp_perpendicular`), `20b8fdc` (`body_cell_advance_state`), `6f0428b` (PartialEq/Eq derives), `2474058` (`bridgehead_walk_to_anchor`).
