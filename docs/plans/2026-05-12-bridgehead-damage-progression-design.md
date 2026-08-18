# Bridgehead Damage Progression — Design (G3)

## Goal

Make Rust's bridgehead direct-damage path produce the same observable output as `ProcessBridgeDamageStateMachine_High` @ 0x00576BA0: sparse, mostly absorbed, with at most one anchor-tile-class transition per bridgehead first-hit, never collapsing the bridge from sustained ramp fire alone.

## Architecture Context

**State model** ([src/sim/bridge_state/mod.rs:372](../../src/sim/bridge_state/mod.rs#L372) `BridgeRuntimeCell`):

- `damage_state: DamageState` — mirrors `CellClass+0x11E` (state byte 0-17). Drives renderer SHP frame selection in [src/app_instances/bridges.rs:65](../../src/app_instances/bridges.rs#L65) `compute_bridge_body_shp_frame`.
- `role: BridgeCellRole` — `Body / Anchor / Bridgehead / Tail`. Set at map-load by the anchor-span walker.
- `axis: Option<Axis>` — NS or EW.
- `anchor_span_id: Option<u16>` — links anchor/body cells to their `AnchorSpan` record; bridgeheads have `None`.
- `overlay_byte: u8` — mirrors `+0x44` (body overlay 0xCD..0xE8).
- `damaged_variant: bool` — pavement toggle (`+0x140 & 0x2000`).

**Bridge-class ID source** (gamemd's `+0x11A`): comes from TMP per-tile byte 40, decoded in [src/assets/tmp_decode.rs:58](../../src/assets/tmp_decode.rs#L58), stored as `ResolvedTerrainCell.template_height` ([src/map/resolved_terrain.rs:80](../../src/map/resolved_terrain.rs#L80)). No new field needed for G3 — the helper reads it via a closure.

**Existing helpers** in [src/sim/bridge_specs.rs](../../src/sim/bridge_specs.rs):

- `bridgehead_walk_to_anchor` (line 624) — takes `start + axis + direction + height-lookup`; walks ≤16 steps until `template_height == 4 (NS) / 2 (EW)`. **Today the direction is a caller-supplied parameter** and the helper has a stricter mid-walk parity check than gamemd.
- `bridgehead_blow_up_row` (line 682) — emits the 3-cell row for the most-damaged collapse. Already verified-match against the `+3` branch.
- `update_ramp_perpendicular` (line 533) — walks 1 perpendicular cell from anchor, applies `+0x11E` state-byte transition if target role is `Anchor`. Docstring explicitly defers the tile-class write branch.

**Driver** `bridgehead_advance_state` ([src/sim/bridge_state/mod.rs:1111](../../src/sim/bridge_state/mod.rs#L1111)):

Today walks (wrong direction), then writes `DamageState::Damaged` or `DamageState::Destroyed` to the bridgehead cell itself (wrong cell) over two hits (wrong outcome — gamemd never reaches collapse from this path).

**Dispatcher** [src/sim/world/bridge_orchestrator.rs:636-672](../../src/sim/world/bridge_orchestrator.rs#L636-L672):

Routes `DispatchPath::HighStateMachine` to `bridgehead_advance_state` vs `body_cell_advance_state` based on `cell.role`. IonCannon retries state-machine paths up to 4× on `NoChange`. Loop breaks on `Absorbed | Collapsed`.

**Renderer coupling**: [src/app_instances/bridges.rs:122](../../src/app_instances/bridges.rs#L122) reads `damage_state` for every bridge cell to pick a sprite. Writing `Damaged` to a bridgehead cell *is* observable today; this is why Gap 2 (wrong cell modified) is a real player-visible regression.

## Impact Analysis

**Touched files (G3 scope):**

- [src/sim/bridge_state/mod.rs](../../src/sim/bridge_state/mod.rs) — new enum, new field on `BridgeRuntimeCell`, rewrite `bridgehead_advance_state`, update `from_resolved_terrain` to initialize the new field.
- [src/sim/bridge_specs.rs](../../src/sim/bridge_specs.rs) — change `bridgehead_walk_to_anchor` signature (drop direction param; compute internally from start height), drop the mid-walk parity check; extend `update_ramp_perpendicular` with the tile-class write branch.
- [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs) — include the new field in the deterministic bridge-state hash.
- Tests: [src/sim/world/world_orders_bridge_repair_tests.rs](../../src/sim/world/world_orders_bridge_repair_tests.rs) and bridge_state unit tests — extend coverage for the new branches.

**Files explicitly NOT touched (out of scope, flagged as follow-up):**

- [src/app_instances/bridges.rs](../../src/app_instances/bridges.rs) — renderer doesn't read the new field yet. After this design lands, the renderer will need a separate brainstorm to map `BridgeheadAnchorClass` → anchor TMP tile (requires .TMP data inspection to find the BridgeSet-relative tile-class offsets per theater).
- `AnchorSpan` struct — left alone (per Approach A choice).

**Risk areas:**

- **Save-format compat.** Adding a field to a serde-derived struct breaks existing saves unless we default-fill. Need to check the project's save-version handling at write-plan time.
- **Determinism / state hash.** New field must be included in the cell hash; the function itself stays deterministic (no RNG draws on this path).
- **Renderer regression window.** Between this G3 landing and the follow-up renderer work, players will see *no* visible damage from direct ramp fire (currently they see the wrong damage). This is a strict parity improvement on the absorbed-h=5/7 cases and a temporary parity drift on h=8 cases. Acceptable per CLAUDE.md's "net parity improvement" reading; will need to be flagged in commit message and tracked as the follow-up.
- **Dispatcher retry budget.** Bridgehead absorbs now return `Absorbed` (not `NoChange`), so the dispatcher's 4-attempt IonCannon retry loop correctly breaks on first attempt. If a hit lands on an odd-h cell, the helper returns `NoChange` and the loop retries — wastes 3 RNG draws but does no observable damage. Need to confirm this RNG-draw cost is determinism-equivalent (it is, because the dispatcher loop is already there; not a new draw count).

## Chosen Approach

**Approach A** from the brainstorm: store the anchor's tile-class state in a new field on every `BridgeRuntimeCell` (cleaner renderer access than per-`AnchorSpan` storage); restructure the driver to write that field on the anchor cell; never mutate the bridgehead cell's own `damage_state` on the direct-damage path; push the walk-direction computation into the helper.

Rejected alternatives are listed under "Alternatives Considered" at the bottom.

## Tiny-Detail Ledger

Each item names its destination in the design under "Components" below. All ledger items are sourced from the verify-doc and RE-follow-up sessions ([AUDIT_LOG.md](../../../ra2-rust-game-docs/AUDIT_LOG.md) entry 2026-05-12 for `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`) plus today's literal-disassembly read of 0x00576BA0.

1. **Entry gate (which cells reach the bridgehead branch).** Cell's `(IsoTileTypeIndex - BridgeSet) + 1` in `[ABAD30..ABAD30+3]` ∪ `[AA1028..AA1028+3]` AND `+0x140 & 0x100 == 0`. `[GHIDRA 0x576BCB-0x576BFA]` — already handled by the dispatcher's role-based routing in [bridge_orchestrator.rs:639-645](../../src/sim/world/bridge_orchestrator.rs#L639-L645).
2. **Height-parity early-return (NS).** `if ((puVar9[0x11A] & 1) != 0) return 0;` — odd `template_height` (h=5, h=7) absorbs damage with zero state change. `[GHIDRA 0x005771c3]` — fires only on the START cell; handled in revised `bridgehead_walk_to_anchor`.
3. **Height upper-bound early-return (EW).** `if (4 < uVar6) return 0;` — rejects h=0xC (high-ramp peak) and any other h > 4. `[GHIDRA 0x00576c91]` — handled in revised `bridgehead_walk_to_anchor`.
4. **Walk direction (NS, verified literal).** `h < 4` (even) → SOUTH `(DAT_0089f698)`; `h == 4` → at anchor; `h > 4` (even) → NORTH `(DAT_0089f688)`. `[GHIDRA 0x005771d3]` — computed inside revised helper.
5. **Walk direction (EW, verified literal — corrects prior fidelity-check artifact).** `h < 2` → EAST `(DAT_0089f690)`; `h == 2` → at anchor; `h > 2` (and ≤ 4) → WEST `(DAT_0089f6a0)`. `[GHIDRA 0x00576ca3]` — computed inside revised helper. **Note:** [docs/fidelity-checks/bridgehead-damage-progression.md](../../docs/fidelity-checks/bridgehead-damage-progression.md) listed `h<2 → W` (swapped) — fix as part of this work.
6. **Walk termination.** Loop runs while `template_height != target` (4 NS / 2 EW). No mid-loop parity check, no upper-bound check after entry — only the start-cell gates. `[GHIDRA 0x00577237 / 0x00576d07]` — revised helper drops the mid-walk parity check.
7. **Mid-walk parity (was a Rust-only stricter check).** gamemd silently walks through odd `+0x11A` intermediates; current Rust returns `None`. `[GHIDRA 0x005771eb-0x00577237]` — revised helper removes the mid-walk check.
7b. **Sentinel cell at off-map reads.** `DAT_00ABDC50+0x11A = 0`; off-map walks loop with h=0. Rust's existing 16-iter cap stays as an internal safety net (not a parity item, just a defensive bound).
8. **First-hit write target = anchor's tile class.** `SetOverlayAndPropagate(anchor.coord, ABAD30+2+BridgeSet, -1, -1, 0)` — writes anchor's `+0x38`, not the hit cell's. Constant **`+2` for all input classes `+0/+1/+2`**. `[GHIDRA 0x00577701 (NS) / 0x0057769b (EW)]` — driver writes `anchor.bridgehead_anchor_class = Damaged`.
9. **No mutation of the hit bridgehead cell itself.** Hit cell's `+0x38`, `+0x44`, `+0x11E` are NOT modified by this branch. `[GHIDRA 0x576BA0 NS branch full trace]` — driver writes only to the anchor, never to the hit cell.
10. **Propagation (anchor's tile change spreads).** `SetOverlayAndPropagate` recurses on 8 neighbors where neighbor's `+0x38 == param_3 == -1`. No real cell has `+0x38 == -1`, so propagation is a no-op for the +2 write — only the anchor changes. `[GHIDRA 0x0056EB80]` — driver writes one cell, doesn't propagate.
11. **Two `UpdateRamp_*_*A/B` calls fired (with anchor's MapCoord).** NS: `DamageA(anchor, 2)` walks E; `DamageB(anchor, 6)` walks W. EW: `DamageA(anchor, 4)` walks S; `DamageB(anchor, 0)` walks N. `[GHIDRA 0x577713-0x577727 / 0x0057769d-0x005776c5]` — already handled by `perpendicular_direction()` in current `update_ramp_perpendicular`.
12. **UpdateRamp inner: state-byte bump on target with `+0x140 & 0x80` (= `role == Anchor` in Rust).** NS_DamageA: 0-3→4, 5→6. NS_DamageB: 0-3→5, 4→6. EW_DamageA: 9-12→0xE, 0xD→0xF. EW_DamageB: 9-12→0xD, 0xE→0xF. `[GHIDRA per HIGH §11.1]` — already in `apply_ramp_transition()`.
13. **UpdateRamp inner: tile-class write branch (NEW for G3).** NS_DamageA preserves: target's class `ABAD30 → ABAD30`, `ABAD30+2 → ABAD30+2`. NS_DamageB progresses: `ABAD30 → ABAD30+1`, `ABAD30+1 → ABAD30+2`. Pavement branch (target's class `== ABC1E8 / AA0E38`) deferred per item 17. `[GHIDRA 0x00572230 / 0x00572330]` — extended `update_ramp_perpendicular` writes target's `bridgehead_anchor_class`.
14. **Return value: `0` (absorbed).** State machine returns 0 for every damage outcome; the `+3` collapse path returns 1 but is unreachable from sustained direct fire (item 15). `[GHIDRA 0x00577727]` — driver returns `StateOutcome::Absorbed` on success.
15. **`+3` "most-damaged" branch never reached from sustained direct fire.** Triggered only when the hit cell's `+0x38` is already `ABAD30+3+BridgeSet` — comes from map-load or body-cell cascade, not from sustained bridgehead fire. `[GHIDRA 0x576CC1 + 0x576CB2 + fidelity-check]` — driver has no `Damaged → Destroyed` branch.
16. **`+3` collapse blow-up row (when reached from elsewhere).** Already-implemented `bridgehead_blow_up_row` is correct; unused from this driver but stays available for the body cascade. `[GHIDRA bridgehead_blow_up_row verified match]` — no change.
17. **Pavement variant (`+0x140 & 0x2000`).** Each UpdateRamp helper has a pavement branch keyed on `iVar2 == DAT_00abc1e8 / DAT_00aa0e38`. **Deferred** — not relevant to high bridges directly; tracked separately under low-bridge work.
18. **No `zones_dirty` on absorb.** `UpdateBridgeZonesHelper` / `InvalidateBridgeZones` only called on the `+3` collapse cascade. The damage path returns `Absorbed` without zone updates. `[GHIDRA 0x577864-0x577880, only inside collapse branch]` — driver returns `Absorbed` with `zones_dirty: false`.

## Design

### Components

**New enum** in [src/sim/bridge_state/mod.rs](../../src/sim/bridge_state/mod.rs):

```
BridgeheadAnchorClass {
    Variant0,    // ABAD30+0  -- map-load intact (default)
    Variant1,    // ABAD30+1  -- map-load intermediate variant; written by DamageB progressions
    Damaged,     // ABAD30+2  -- the runtime "damaged" anchor tile
    AboutToFall, // ABAD30+3  -- reached only via body-cell collapse cascade or map-load
}
```

Default `Variant0` at map load. Meaningful only when `role == Anchor` (other cells leave it at default and the renderer ignores it once the renderer follow-up lands).

Source of truth: ledger items 8 + 13. Maps directly to gamemd's `(IsoTileTypeIndex - BridgeSet) - ABAD30` (i.e., the 0..3 offset into the `DAT_00ABAD30` four-value range), with the EW mirror reaching the same enum via `AA1028` instead of `ABAD30` (renderer follow-up resolves the tile-class index per axis).

**New field** on `BridgeRuntimeCell`:

```
bridgehead_anchor_class: BridgeheadAnchorClass
```

Initialized to `Variant0` in `BridgeRuntimeState::from_resolved_terrain`. Save/load: serde-default to `Variant0` if missing in an older save.

**Revised `bridgehead_walk_to_anchor`** in [src/sim/bridge_specs.rs](../../src/sim/bridge_specs.rs):

Signature change: **drop the `direction` parameter**. New body:
1. Read start cell's height. If NS: `h & 1 != 0 → None`. If EW: `h > 4 → None`. (Ledger items 2, 3.)
2. If `h == target (4 NS / 2 EW)` → return `Some(start)`.
3. Compute walk direction from start height (ledger items 4, 5):
   - NS: `h < 4 → S`, `h > 4 → N`.
   - EW: `h < 2 → E`, `h > 2 → W`.
4. Loop up to 16 iterations (internal safety cap, ledger item 7b):
   - Step one cell in the computed direction.
   - Read new cell's height. If `h == target` → return `Some(current)`.
   - **No mid-walk parity check** (ledger items 6, 7).
   - **No mid-walk direction recomputation** — gamemd recomputes direction every iteration, but in practice the height only converges (8 → 6 → 4 or 0 → 2), so direction is monotonic. For parity safety, the design DOES recompute direction every iteration (matches binary exactly with no performance cost).
5. If 16-iter cap exhausted → `None` (defensive only).

**Revised `bridgehead_advance_state`** in [src/sim/bridge_state/mod.rs](../../src/sim/bridge_state/mod.rs):

Body (replaces lines 1111-1230):
1. Resolve hit cell. If `role != Bridgehead` or `axis == None` → `NoChange`.
2. Build height-lookup closure over `ResolvedTerrainGrid.template_height`.
3. Call revised `bridgehead_walk_to_anchor(hit_pos, axis, lookup, w, h)`. If `None` → `NoChange` (absorbed by the parity/upper-bound gate — ledger items 2, 3 — or walked off map).
4. Write `anchor.bridgehead_anchor_class = Damaged` on the resolved anchor cell. (Ledger item 8 — write is idempotent on repeat hits.)
5. Call `update_ramp_perpendicular(state, anchor, axis, Phase::DamageA, is_high)` and again with `Phase::DamageB`. These now do both the existing state-byte bump AND the new tile-class write (ledger items 11, 12, 13).
6. Return `StateOutcome::Absorbed { zones_dirty: false, … }`. (Ledger items 14, 18.) No `Collapsed` branch, no `set_bridge_direction` call, no `destroyed_cells`, no blow-up row.

**Removed**: the entire `DamageState::Damaged → Destroyed` branch (ledger item 15 — gamemd doesn't reach collapse from this path). `bridgehead_blow_up_row` becomes unused-from-this-driver; left in place for body-cascade use.

**Extended `update_ramp_perpendicular`** in [src/sim/bridge_specs.rs](../../src/sim/bridge_specs.rs):

Existing body (state-byte branch) unchanged. After the state-byte write, before returning, add the **tile-class write branch** (ledger item 13):

```
Target's existing class \  Phase
                        \  DamageA  DamageB         CollapseA  CollapseB
Variant0                 Variant0   Variant1        Damaged    Damaged
Variant1                 (no-op)    Damaged         Damaged    Damaged
Damaged                  Damaged    (no-op)         Damaged    Damaged
AboutToFall              (no-op)    (no-op)         (no-op)    (no-op)
```

Reads:
- DamageA preserves Variant0 / Damaged (gamemd `ABAD30 → ABAD30`, `ABAD30+2 → ABAD30+2`).
- DamageB progresses Variant0 → Variant1, Variant1 → Damaged (gamemd `ABAD30 → ABAD30+1`, `ABAD30+1 → ABAD30+2`).
- Collapse variants advance any non-Variant-0 / non-Damaged source to Damaged (mirrors the existing collapse helpers' tile-class writes seen during verify-doc).
- AboutToFall (`+3`) is reached only via the body cascade hitting an already-`+3` cell; preserved as-is — the recursive `+3 → +3` write is a no-op for our field.

The target cell may be **either** an `Anchor` role (existing case — state-byte bumps fire) **or** a `Bridgehead` role (new case — only the tile-class write fires, no state-byte bump because bridgeheads don't have `+0x140 & 0x80`). Decision logic:
- If `target.role == Anchor`: do state-byte bump (existing) + tile-class write.
- If `target.role == Bridgehead`: skip state-byte bump; do tile-class write only.
- Else: no-op.

Pavement branch (ledger item 17) explicitly excluded.

### Interfaces / Contracts

**Public surface changes:**
- `BridgeheadAnchorClass` enum: new public type.
- `BridgeRuntimeCell.bridgehead_anchor_class`: new public field.
- `bridgehead_walk_to_anchor`: removes the `direction: Direction` parameter. Existing callers update accordingly.

**Internal contract:**
- `BridgeheadAnchorClass` on cells with `role != Anchor` and `role != Bridgehead` is meaningless; renderer is documented to only read it when role is Anchor.
- `update_ramp_perpendicular` is documented to write `bridgehead_anchor_class` on Bridgehead-role targets and to mutate both fields on Anchor-role targets.

**Dispatcher contract** (no change):
- `bridgehead_advance_state` continues to return `StateOutcome::{Absorbed, NoChange}`. The dispatcher's IonCannon retry budget is unchanged; we simply return `NoChange` more often (when the parity/upper-bound gates fire), and `Absorbed` instead of `Collapsed` when work happens. Since both `Absorbed` and `Collapsed` break the retry loop, behavior is equivalent.

### Data Flow

```
Apply_area_damage (cells in radius)
  → bridge_orchestrator::tick_bridge_damage_events
    → path = HighStateMachine; cell.role == Bridgehead
      → bridgehead_advance_state(hit_pos, true, terrain)
        ├─ filter (role == Bridgehead)
        ├─ bridgehead_walk_to_anchor(hit_pos, axis, height_lookup, w, h)
        │   ├─ start-cell gate (NS: h&1; EW: h>4) → None on fail
        │   ├─ direction computed from h
        │   └─ loop until h == target → Some(anchor)
        ├─ anchor.bridgehead_anchor_class = Damaged
        ├─ update_ramp_perpendicular(anchor, axis, DamageA, …)
        │   └─ writes target's damage_state (anchor target) AND/OR bridgehead_anchor_class (bridgehead target)
        ├─ update_ramp_perpendicular(anchor, axis, DamageB, …)
        └─ return Absorbed
```

### Error Handling

No errors per se — the function returns `NoChange` (no work done) or `Absorbed` (work done). Off-map cells and odd-`+0x11A` cells return `NoChange`. Invariants checked via debug assertions only (no runtime panics on real inputs).

### Testing Strategy

**Unit tests** in [src/sim/bridge_state/mod.rs](../../src/sim/bridge_state/mod.rs) and [src/sim/bridge_specs.rs](../../src/sim/bridge_specs.rs):

1. **Parity gate (NS).** Cell with `template_height=5` (odd NS ramp). Call driver. Assert `NoChange`, no `bridgehead_anchor_class` change anywhere. Cover h=7 too.
2. **Parity gate (EW).** Cell with `template_height=0xC` (EW high-ramp peak). Assert `NoChange`. Cover h=5, h=6 too (anything > 4).
3. **Walk + anchor write (NS, h=8).** Cell with h=8. Map laid out so h=8 → walk N → h=4 anchor. Assert `Absorbed`; anchor's `bridgehead_anchor_class == Damaged`; bridgehead cell unmodified.
4. **Walk + anchor write (EW, h=0).** Cell with h=0. Map laid out so h=0 → walk E → h=2 anchor. Assert anchor written.
5. **Walk + anchor write (EW, h=4).** Cell with h=4. Walk W → h=2 anchor. Catches the EW direction correction (fidelity-check artifact had this swapped).
6. **Idempotency.** Hit bridgehead 5×. Anchor goes Variant0 → Damaged on hit 1; stays Damaged on hits 2-5. Bridgehead never changes.
7. **No collapse from sustained ramp fire.** Hit bridgehead 100×. Bridge never reports `Collapsed` from this driver.
8. **Mid-walk odd-cell tolerance.** Map laid out as: hit cell h=8 → step N → h=5 (odd) → step N → h=4 anchor. Assert anchor is reached (mid-walk parity check is dropped). Counter-test: current Rust returns None here.
9. **DamageB progression on neighbor bridgehead.** Hit a body cell that triggers `update_ramp_perpendicular` DamageB walking onto a bridgehead at Variant0. Assert bridgehead's `bridgehead_anchor_class` advances to Variant1. Repeat: Variant1 → Damaged. (Ledger item 13.)
10. **DamageA preserve.** Mirror: DamageA on a Variant0 bridgehead leaves it Variant0; on a Damaged bridgehead leaves it Damaged.
11. **Dispatcher integration.** Through the orchestrator, confirm a tank shell (non-IonCannon) on a ramp cell with h=5: passes Wall+SpecialFlags gates, BridgeStrength RNG passes, dispatcher routes to bridgehead_advance_state, driver returns `Absorbed`, no further dispatch attempts. (Ledger item 14.)
12. **State-hash determinism.** Two parallel runs with identical inputs produce identical bridge-state hashes after the new field's contribution lands in world_hash.

**Existing tests to update:**
- The current `bridgehead_advance_state` tests (if any) that expected `Damaged → Destroyed` need rewriting against the new contract. The legacy "collapse on hit 2" assertion is wrong per gamemd.

**Integration test** (existing pattern from G1's repair work, see [src/sim/world/world_orders_bridge_repair_tests.rs](../../src/sim/world/world_orders_bridge_repair_tests.rs)):

- Fire IonCannonWH at a high-bridge ramp cell on a small synthetic map; assert: bridge body cells unmodified, bridgehead unmodified, anchor's `bridgehead_anchor_class == Damaged`. No `BridgeCollapsed` sim event emitted.

## Architectural Decisions

**Patterns followed:**
- Per-cell state mirror of a gamemd field. `damage_state` mirrors `+0x11E`, `overlay_byte` mirrors `+0x44`, `bridgehead_anchor_class` mirrors the relevant subset of `+0x38` values. Same shape as adjacent fields.
- Helper accepts a closure for terrain data (`height_lookup`) rather than coupling directly to `ResolvedTerrainGrid` — matches the existing `bridgehead_walk_to_anchor` API.
- Driver returns `StateOutcome` and lets the dispatcher own the retry/cascade loop — same pattern as `body_cell_advance_state`.

**Patterns deviated from:**
- The walk-direction parameter on `bridgehead_walk_to_anchor` is being **removed**. Today's API made the caller responsible for computing direction; the binary computes it from start height inside the same function. The new API pushes that computation into the helper. The previous API was a mistake (it allowed Gap 4 to exist); fixing it brings the helper closer to the binary's contract.
- `update_ramp_perpendicular` now does TWO writes per call instead of one. Documented in its updated docstring. Acceptable because the binary's UpdateRamp helpers also do two writes (state byte + tile class).

**Tech debt introduced:**
- The renderer follow-up is mandatory for full parity but not in this design's scope. Tracked as: **"render anchor TMP tile based on `bridgehead_anchor_class` (HIGH bridges first, then LOW)."** Without it, the player sees less damage feedback than gamemd shows. Net parity is still an improvement (no more wrong collapses), but the loop isn't fully closed.

**Determinism:**
- New field is included in `world_hash`. Function is deterministic (no RNG draws). State hash diverges from the previous Rust build (incompatible save/replay across the transition), but is stable within the new build.

## Alternatives Considered

**Approach B (field on AnchorSpan, not on BridgeRuntimeCell).** State lives at the span level instead of per-cell. Pros: less memory waste; semantically correct (one tile-class per anchor span, not per cell). Cons: renderer must look up the span (`BTreeMap` lookup per anchor on every draw); doesn't match gamemd's flat per-cell field layout. Rejected because the renderer access pattern wants flat reads and the memory savings are trivial on our scale (~1 byte × bridge cells).

**Repurpose `damage_state` on the anchor.** When a bridgehead is hit, mutate the anchor's existing `damage_state` instead of adding a new field. Rejected because the body driver already manages the anchor's `damage_state` (state byte 6 = "next hit collapses"), and the bridgehead-hit path does NOT advance toward collapse. Two writers with conflicting semantics on the same field is a foot-gun.

**Renderer-derived (no new sim state).** Renderer queries adjacent cells to derive the anchor's appearance. Rejected because it (a) crosses the sim/render boundary in the wrong direction (sim is supposed to compute observable state, renderer reads it), (b) doesn't match gamemd's data flow (which writes a flat field), and (c) requires the renderer to encode bridge-graph adjacency knowledge.

**Mutate `ResolvedTerrainCell.final_tile_index` directly.** Most literal mirror of gamemd's `+0x38` write. Rejected because `ResolvedTerrainGrid` is currently treated as immutable input to sim; mutating it would require a much larger refactor (pathfinding caches, renderer caches, save/load) that's out of scope for G3.

## Open Items for /write-plan

1. **Save-format compat strategy.** Confirm the project's serde versioning approach. `#[serde(default)]` on the new field should suffice for backward compat with existing saves.
2. **Renderer follow-up scope.** Separate brainstorm: how does `BridgeheadAnchorClass` map to anchor TMP tile selection in [src/app_instances/bridges.rs](../../src/app_instances/bridges.rs)? Requires .TMP file inspection per theater (BridgeSet vs WoodBridgeSet etc.).
3. **Fidelity-check artifact correction.** [docs/fidelity-checks/bridgehead-damage-progression.md](../../docs/fidelity-checks/bridgehead-damage-progression.md) lists `h<2 → W` for EW; should be `h<2 → E`. Fix at write-plan time.
