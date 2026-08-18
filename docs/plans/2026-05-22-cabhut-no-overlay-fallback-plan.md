# CABHUT No-Overlay Fallback Implementation Plan

> **For Codex:** Execute this plan task-by-task. Each task is self-contained. Do not write Rust code until this plan is approved for implementation.

**Goal:** Replace the current traced CABHUT no-overlay fallback with a binary-shaped starter/anchor/ramp plan matching `MapClass__DestroyBridge_{High,Low}_OnHutDeath`.

**Architecture:** This is a deterministic `sim/world` change. `bridge_orchestrator.rs` owns hut-specific dispatch and keeps the existing `StateOutcome` cascade. `bridge_state` remains the mutation primitive layer. No render/audio/UI/sidebar/net dependencies are introduced.

**Design Doc:** `docs/plans/2026-05-22-cabhut-no-overlay-fallback-design.md`

---

## Grounding Summary

The primary spec is `docs/research/BRIDGE_HUT_FALLBACK_FLAGS_RAMP_ONLY_GHIDRA_REPORT.md`.

Binary facts verified for this plan:

- `MapClass__DestroyBridge_High_OnHutDeath @ 0x00574000` and low twin `0x00574C20` run overlay-first 5x5 scan before fallback.
- The no-overlay fallback accepts a starter only when `CellClass+0x140 & 0x500 != 0`.
- The hut cell is tested first. If not accepted, the binary searches direction indices `0..7`, distances `1`, `2`, `3`.
- The search stops at the first accepted cell.
- `0x80` and `0x800` are modifiers only; neither qualifies a starter alone.
- Pure `0x400` fallback has special anchor math: walk E when `0x800` clear or S when set; stop at the first non-`0x400`; anchor two cells opposite. Four continuation cells returns early.
- The ramp phase calls `ApplyDamageToCell` in two bounded groups of up to 3 attempts.

Current Rust mismatch:

- `find_hut_fallback_cells` returns a traced list.
- `append_hut_fallback_trace` walks contiguous broad bridge evidence up to `HUT_FALLBACK_TRACE_LIMIT`.
- `dispatch_bridge_collapse_from_hut` applies generic damage to every returned fallback cell until collapse.

The plan replaces that list-shaped fallback with a local `HutFallbackPlan` while leaving overlay-first dispatch and final collapse cascade unchanged.

---

## Key Technical Decisions

- Keep `dispatch_bridge_collapse_from_hut(sim, rules, hut_center) -> bool` unchanged. **Confidence:** high.
  - **Source:** current public orchestrator contract; design doc.

- Use `ResolvedTerrainCell.bridge_flags()` as the source for `CellClass+0x140` fallback starter flags. **Confidence:** medium-high.
  - **Source:** `ResolvedTerrainCell::bridge_flags()` returns `bridge_facts.raw_flags`; bridge facts are the current map-stage representation of the binary bridge flag word.

- Do not use broad runtime evidence as starter acceptance. **Confidence:** high.
  - **Source:** binary mask is exactly `0x500`; current broad evidence is the cause of over-tracing.

- Keep `apply_hut_damage_to_cell` as the mutation primitive. **Confidence:** high.
  - **Source:** current Rust already funnels bridgehead/body/tail damage through this helper; binary fallback calls `ApplyDamageToCell`.

- Replace `find_hut_fallback_cells`/`append_hut_fallback_trace` with plan-builder helpers and an execution result that carries non-collapse side effects. **Confidence:** high.
  - **Source:** binary fallback is not a cell list.

- Add private unit tests in `bridge_orchestrator.rs` for plan construction when world-order fixture setup would obscure the exact starter/anchor behavior. **Confidence:** high.
  - **Source:** existing integration tests are too broad to catch starter priority drift.

- Do not implement bridge collapse sound or event `0x1F` in this plan. **Confidence:** high.
  - **Source:** separate re-swarm slots; not part of CABHUT fallback mutation.

Low-confidence boundary:

- Current terrain facts must be checked during implementation for enough data to reproduce `IsBridgeRampTile` and `IsLowBridgeEndpointTile`. If they are insufficient, pause and run a narrow RE/data-model follow-up rather than approximating the ramp/endpoint probes.
- The implementation must prove every ramp/endpoint damage target selected from terrain flags has a corresponding `BridgeRuntimeCell` before calling `apply_hut_damage_to_cell`. If a selected target lacks runtime bridge state, stop and add a terrain-to-runtime mapping task rather than silently converting the case into `NoChange`.

---

## Open Questions

### Resolved During Planning

- **Should the fix include collapse audio?** No. Audio is a separate effect/audio routing issue.
- **Should event `0x1F` be implemented now?** No. It is trigger-only and safe to stub for skirmish.
- **Should fallback remain list-shaped with better guards?** No. The binary picks one starter and one walk path.
- **Is this TS legacy?** No. The hut destruction functions are live in YR through `BridgeRepairHut` C4/demo-truck callers, though this branch is topology-conditional.

### Deferred

- **Stock packed-map incidence:** Not needed to implement parity. Requires MIX extraction if frequency ranking matters.
- **Exact public/editor label for event `0x1F`:** Out of scope.
- **Collapse sound routing:** Out of scope.

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/sim/world/bridge_orchestrator.rs` | Replace traced fallback with `HutFallbackPlan`; add starter/anchor/ramp helpers; preserve final cascade. |
| Modify | `src/sim/world/world_orders_bridge_repair_tests.rs` | Add focused C4-on-CABHUT fallback integration tests. |
| Optional modify | `src/map/bridge_facts.rs` or `src/map/resolved_terrain.rs` | Only if a narrow accessor is needed for existing raw flags/ramp facts. Do not add new behavior unless required. |
| Read only | `src/sim/bridge_state/mod.rs` and `src/sim/bridge_state/walker.rs` | Confirm mutation helper expectations and available test fixture APIs. |

No new source file is required. If `bridge_orchestrator.rs` becomes unwieldy, split only after the functional change is understood and tested.

---

## Interface Changes

No public API change is expected.

Internal additions in `bridge_orchestrator.rs`:

```rust
struct HutFallbackStarter {
    pos: (u16, u16),
    flags: u32,
}

enum HutFallbackPlan {
    NoAcceptedStarter,
    MissingAnchor,
    PureBridgeheadTooLong,
    RampWalk {
        starter: HutFallbackStarter,
        anchor: (u16, u16),
    },
}

struct HutFallbackExecution {
    outcomes: Vec<StateOutcome>,
    zones_dirty: bool,
    adjacent_dirty_anchor: Option<(u16, u16)>,
}
```

Internal helper signatures may include:

```rust
fn hut_fallback_flags(sim: &Simulation, pos: (u16, u16)) -> u32
fn find_hut_fallback_starter(sim: &Simulation, hut_center: (u16, u16)) -> Option<HutFallbackStarter>
fn resolve_hut_fallback_anchor(sim: &Simulation, starter: HutFallbackStarter) -> HutFallbackPlan
fn resolve_pure_bridgehead_anchor(sim: &Simulation, starter: HutFallbackStarter) -> HutFallbackPlan
fn run_hut_fallback_plan(
    bridge_state: &mut BridgeRuntimeState,
    terrain: &ResolvedTerrainGrid,
    family: HutBridgeFamily,
    plan: HutFallbackPlan,
) -> HutFallbackExecution
```

Keep `apply_hut_damage_to_cell` private and reuse it.

---

## Sim Checklist

- [ ] No `f32`/`f64` in sim logic.
- [ ] No new persistent sim state unless explicitly justified.
- [ ] No dependencies from `sim/` to render/audio/ui/sidebar/net.
- [ ] Iteration order is explicit and deterministic.
- [ ] `EntityStore` and world hash behavior remain unchanged unless bridge state actually mutates.
- [ ] Overlay-first path remains unchanged.
- [ ] Final cascade order remains unchanged: occupants, deck drops, debris, adjacent refresh, trigger hook, zones.
- [ ] Non-collapse fallback side effects flow through the same final refresh sequence before the production path is wired.

---

## Risk Areas

- Accidentally accepting `0x80` or `0x800` as starter evidence.
- Accidentally preserving broad `has_hut_fallback_bridge_evidence` semantics in the new starter path.
- Misreading pure `0x400` offset direction and stepping two more cells in the same direction.
- Losing the "four continuation cells returns early" behavior.
- Treating no-ramp, post-ramp bounds exit, and endpoint exit as the same side effect path.
- Existing world-order tests may still pass if the exact fallback starter is wrong; helper-level tests are required.

---

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|---|---|---|---|
| 1 | Exact raw flag source | Starter acceptance must match `CellClass+0x140 & 0x500` | Helper tests using seeded `bridge_facts.raw_flags` |
| 2 | Starter order | Chooses which bridge/hut side collapses | Test N distance-2 beats E distance-1 |
| 3 | Modifier rejection | Prevents false positives from `0x80`/`0x800` | Tests reject each alone |
| 4 | `0x100` anchor branches | Determines ramp walk origin | Plan-builder tests |
| 5 | Pure `0x400` anchor math | Easy source of one/two-cell drift | E->W and S->N tests |
| 6 | Four-continuation early return | Prevents over-collapse | Plan returns `PureBridgeheadTooLong` |
| 7 | Ramp/endpoint retry groups | Determines actual bridge state mutation | Tests for up to 3 attempts per group |
| 8 | Non-collapse side effects | No-ramp and post-ramp exits can rebuild zones / dirty adjacent bridges | Execution-result tests before wiring production |
| 9 | Overlay-first preserved | Existing working path must not regress | Existing terminal-overlay tests plus focused smoke test |
| 10 | Final cascade preserved | Player-visible collapse side effects remain | Existing CABHUT collapse tests |

---

## Tasks

### Task 1: Add Exact Raw-Flag Helpers

**Why:** The binary starter test is `flags & 0x500`, not generic bridge evidence.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs`

**Steps:**
1. Add named constants near the hut fallback constants:
   - `HUT_FALLBACK_STARTER_MASK: u32 = 0x500`
   - `HUT_FLAG_ANCHOR_SELF: u32 = 0x80`
   - `HUT_FLAG_STRUCTURAL: u32 = 0x100`
   - `HUT_FLAG_PURE_BRIDGEHEAD: u32 = 0x400`
   - `HUT_FLAG_DIRECTION_FLIP: u32 = 0x800`
2. Add `hut_fallback_flags(sim, pos) -> u32` that reads `sim.resolved_terrain.cell(pos).bridge_flags()`.
3. Do not fall back to `BridgeRuntimeCell` role/deck/span data in this helper.
4. Add `is_hut_fallback_starter_flags(flags) -> bool` returning `flags & 0x500 != 0`.
5. Retain the old broad evidence helper temporarily only if still needed by existing code before later tasks remove it.

**Tests:**
- Helper accepts `0x100`, `0x400`, and `0x500`.
- Helper rejects `0x80`, `0x800`, `0x880`, and `0`.

**Verify:**
Run:

```powershell
cargo test hut_fallback_starter --lib -- --nocapture
```

Expected: helper tests pass.

### Task 2: Replace Traced Cell Search With Single Starter Search

**Why:** The binary picks one starter and stops.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs`

**Steps:**
1. Add `HutFallbackStarter`.
2. Implement `find_hut_fallback_starter(sim, hut_center) -> Option<HutFallbackStarter>`.
3. First test hut center.
4. If hut center is not accepted, iterate `HUT_FALLBACK_DIRS` in order.
5. For each direction, test distance `1`, `2`, then `3`.
6. Stop and return immediately at the first accepted starter.
7. Remove or stop using `append_hut_fallback_trace`.
8. Keep `find_hut_fallback_cells` only as a temporary wrapper if needed during this task; it must be deleted by Task 8.

**Tests:**
- Hut cell starter beats all searched cells.
- N distance-2 beats E distance-1 because direction index 0 beats index 2.
- N distance-1 beats N distance-2.
- No accepted flags returns `None`.

**Verify:**
Run:

```powershell
cargo test hut_fallback_starter --lib -- --nocapture
```

Expected: starter order tests pass.

### Task 3: Implement Anchor Resolution For `0x100` Starters

**Why:** The ramp walk starts from different anchors depending on `0x80`.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs`

**Steps:**
1. Add `resolve_hut_fallback_anchor(sim, starter) -> HutFallbackPlan`.
2. If `starter.flags & 0x100 != 0` and `starter.flags & 0x80 != 0`, anchor is `starter.pos`.
3. If `starter.flags & 0x100 != 0` and `0x80` is clear, anchor should be the Rust equivalent of binary `cell+0x2C` coordinate.
4. Preferred Rust source for the `cell+0x2C` anchor:
   - terrain `bridge_facts.anchor.map(|relation| relation.anchor)`;
   - if absent, runtime `BridgeRuntimeCell.anchor_span_id` -> `BridgeRuntimeState::anchor_span(span_id).anchor`.
5. If neither source exists, return `HutFallbackPlan::MissingAnchor` rather than guessing.
6. Pure `0x400` starters should be delegated to Task 4.

**Tests:**
- `0x100|0x80` returns anchor equal to starter.
- `0x100` without `0x80` uses `bridge_facts.anchor.anchor` when present.
- `0x100` without `0x80` returns `MissingAnchor` when no anchor relation exists.
- `0x80` without `0x100` was already rejected by Task 1/2.

**Verify:**
Run:

```powershell
cargo test hut_fallback_anchor --lib -- --nocapture
```

Expected: `0x100` anchor tests pass.

### Task 4: Implement Pure `0x400` Anchor Resolution

**Why:** This was the most error-prone stale-doc correction.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs`

**Steps:**
1. Add a helper for map-coordinate stepping by `HUT_FALLBACK_DIRS` index.
2. For pure `0x400`, compute:
   - clear `0x800`: scan direction index `2` (E), final offset direction index `6` (W);
   - set `0x800`: scan direction index `4` (S), final offset direction index `0` (N).
3. Start from `starter.pos`.
4. Step one cell at a time while the next cell has `flags & 0x400 != 0`.
5. Count only stepped continuation cells that still have `0x400`.
6. If count becomes 4, return `HutFallbackPlan::PureBridgeheadTooLong`.
7. At the first non-`0x400` break cell, anchor is break cell plus two steps in the opposite direction.
8. If any coordinate step underflows/overflows, return `HutFallbackPlan::MissingAnchor`.

**Tests:**
- Clear `0x800`, zero continuation cells: break cell is E of starter; anchor is two W from break.
- Clear `0x800`, one continuation cell: anchor shifts accordingly.
- Set `0x800` mirrors the behavior S then two N.
- Four continuation cells returns `PureBridgeheadTooLong`.
- Do not offset two more cells in the scan direction.

**Verify:**
Run:

```powershell
cargo test hut_fallback_pure_bridgehead --lib -- --nocapture
```

Expected: pure `0x400` tests pass.

### Task 5: Add Ramp And Endpoint Predicate Helpers

**Why:** The executor needs binary-shaped `IsBridgeRampTile` and `IsLowBridgeEndpointTile` checks without broad bridge evidence.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs`
- Optional modify: `src/map/resolved_terrain.rs` only for accessors over existing data.

**Steps:**
1. Add `is_hut_fallback_ramp_cell(terrain, pos) -> bool`.
2. Use existing terrain facts only if they map to verified bridge-ramp semantics:
   - allowed source: `ResolvedTerrainCell.bridge_facts.ramp_tile`;
   - allowed source: a new narrow helper over existing bridge tile metadata with a comment citing `MapClass__IsBridgeRampTile @ 0x005746C0`;
   - do not use general `ResolvedTerrainCell.has_ramp` by itself, because ordinary terrain slopes are not the binary bridge-ramp predicate.
3. Add `hut_fallback_endpoint_relative_tile(terrain, family, pos) -> Option<i32>` or equivalent.
4. This must return enough information to test the binary `relative_tile != -2` gate.
5. If existing data cannot distinguish endpoint relative tile `-2`, stop and create a narrow RE/data-model follow-up before approximating.
6. Keep these helpers private to the orchestrator unless they are clearly reusable.
7. Add `has_hut_fallback_runtime_damage_target(bridge_state, pos) -> bool` or equivalent and require it before any planned ramp/endpoint call to `apply_hut_damage_to_cell`.
8. If a terrain-selected ramp/endpoint damage target lacks a `BridgeRuntimeCell`, stop and add a terrain-to-runtime mapping task; do not let `apply_hut_damage_to_cell` silently convert the case into `NoChange`.

**Tests:**
- A seeded `bridge_facts.ramp_tile` is accepted as ramp.
- A general `has_ramp` terrain slope without bridge ramp facts is not accepted as ramp.
- Endpoint helper can distinguish the `-2` skip case from a non-`-2` endpoint case if existing fixtures can represent it.
- A planned damage target without runtime bridge state is rejected before mutation.

**Verify:**
Run:

```powershell
cargo test hut_fallback_ramp --lib -- --nocapture
```

Expected: ramp predicate tests pass, or implementation stops with a documented data-model blocker.

### Task 6: Implement The Fallback Plan Executor

**Why:** This is the actual binary-shaped replacement for tracing and generic per-cell iteration.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs`

**Steps:**
1. Add `HutFallbackExecution` before implementing the executor:
   - `outcomes: Vec<StateOutcome>`;
   - `zones_dirty: bool`;
   - `adjacent_dirty_anchor: Option<(u16, u16)>`.
2. Add `apply_hut_damage_retries`:
   - call `apply_hut_damage_to_cell` up to `MAX_HUT_ATTEMPTS_PER_STEP`;
   - collect `Absorbed` and `Collapsed` outcomes;
   - stop early on `NoChange` or first `Collapsed`.
3. Add `run_hut_fallback_plan`.
4. For `NoAcceptedStarter`, `MissingAnchor`, or `PureBridgeheadTooLong`, return `HutFallbackExecution::default()` with no side effects.
5. Compute forward direction from starter flags:
   - `0x800` set -> direction index `6` (W);
   - otherwise direction index `0` (N).
6. Walk from anchor in forward direction while in map bounds.
7. On each cell, require the equivalent of binary `MapClass+0x13C[cell_index] != 0` if such a terrain/cell-presence gate exists. If no direct equivalent exists, use `terrain.cell(pos).is_some()` and document this in code.
8. Stop at first ramp cell.
9. If no ramp is found before bounds exit, return execution with `zones_dirty = true` and no adjacent dirty anchor. This must be implemented before production wiring.
10. On ramp found:
   - reverse direction by `(forward - 4) & 7`;
   - apply first retry group on ramp cell.
11. Walk in reversed direction until endpoint or bounds exit.
12. If bounds exit after ramp, return execution with `zones_dirty = true` and `adjacent_dirty_anchor = Some(anchor)` plus any collected outcomes.
13. If endpoint found and relative tile is not `-2`, reverse again and apply second retry group one cell beyond endpoint in the original forward direction.
14. Endpoint exit returns execution with `zones_dirty = true` and `adjacent_dirty_anchor = Some(anchor)` plus collected outcomes.

**Tests:**
- First ramp retry calls damage at most 3 times.
- Stops retry on first collapse.
- Endpoint `-2` case skips second retry group.
- Non-`-2` endpoint applies second retry one cell beyond endpoint.
- No-ramp produces no damage outcomes but sets `zones_dirty = true`.
- Post-ramp bounds/endpoint exits set `zones_dirty = true` and `adjacent_dirty_anchor = Some(anchor)`.

**Verify:**
Run:

```powershell
cargo test hut_fallback_plan --lib -- --nocapture
```

Expected: executor tests pass.

### Task 7: Feed Fallback Execution Into The Existing Cascade

**Why:** The executor now carries side effects that can occur without collapsed cells, so the final refresh path needs to accept them before production fallback is wired.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs`

**Steps:**
1. Extend `apply_hut_bridge_outcomes` or add a sibling helper that accepts:
   - normal `StateOutcome`s;
   - fallback `zones_dirty`;
   - fallback `adjacent_dirty_anchor`.
2. Preserve the existing cascade order.
3. If `adjacent_dirty_anchor` is present, feed that coordinate into the existing `update_adjacent_bridges` input set.
4. If only `zones_dirty` is true and no cells collapsed, call `refresh_bridge_zones_if_dirty(sim, true)`.
5. Return value:
   - return true when cells collapsed;
   - return true when zone/adjacent side effects require pathgrid refresh observable to callers;
   - return false for no accepted starter, missing anchor, and pure-bridgehead-too-long no-op.
6. Add tests for no-ramp and post-ramp execution result side effects before changing production dispatch.

**Verify:**
Run:

```powershell
cargo test hut_fallback_side_effect --lib -- --nocapture
```

Expected: side-effect tests pass before production wiring.

### Task 8: Wire The Plan Into `dispatch_bridge_collapse_from_hut`

**Why:** Replace the current traced-list production path.

**Files:**
- Modify: `src/sim/world/bridge_orchestrator.rs`

**Steps:**
1. Keep overlay-first seed logic unchanged:
   - `find_destroy_overlay_seed(&scan, family)`
   - `run_hut_collapse_bounded`
2. Remove fallback overlay seed lookup from traced fallback cells:
   - no `find_destroy_overlay_seed(bs, &fallback_cells_lazy, family)`.
3. In the no-overlay branch:
   - build `find_hut_fallback_starter`;
   - resolve `HutFallbackPlan`;
   - execute `run_hut_fallback_plan`.
4. Feed the returned `HutFallbackExecution` into the side-effect-aware cascade helper from Task 7.
5. Preserve the current post-seed behavior only if it matches binary:
   - after fallback damage produces a collapse, it is acceptable to feed outcomes into the existing final cascade;
   - do not run `run_hut_collapse_bounded` from arbitrary traced cells.
6. Delete `append_hut_fallback_trace`.
7. Delete `HUT_FALLBACK_TRACE_LIMIT`.
8. Delete or repurpose `has_hut_fallback_bridge_evidence` so it cannot be used as starter acceptance.

**Tests:**
- Existing overlay-first tests still pass.
- Old fallback bridgehead test either updates to exact binary behavior or is replaced by focused tests from Tasks 2-6.

**Verify:**
Run:

```powershell
cargo test c4_on_cabhut --lib -- --nocapture
```

Expected: CABHUT tests pass or reveal fixture updates needed for exact raw flags.

### Task 9: Add End-To-End Integration Tests

**Why:** The player-visible path is C4/demo-truck destroying a bridge repair hut, not direct helper calls.

**Files:**
- Modify: `src/sim/world/world_orders_bridge_repair_tests.rs`

**Steps:**
1. Add or update fixture helpers to seed `ResolvedTerrainCell.bridge_facts.raw_flags`.
2. Add `c4_on_cabhut_fallback_uses_first_flag_starter_not_trace`.
3. Add `c4_on_cabhut_fallback_rejects_0x80_only_starter`.
4. Add `c4_on_cabhut_fallback_rejects_0x800_only_starter`.
5. Add `c4_on_cabhut_pure_bridgehead_four_continuations_noops`.
6. Add one positive integration test that reaches actual bridge mutation through the new fallback plan.
7. Keep assertions that hut HP stays unchanged and pending C4 marker clears.
8. Keep existing `c4_on_cabhut_low_terminal_overlay_0x65_uses_overlay_first_scan` and high twin green.

**Verify:**
Run:

```powershell
cargo test c4_on_cabhut --lib -- --nocapture
```

Expected: all CABHUT integration tests pass.

### Task 10: Focused Regression Verification

**Why:** This touches bridge collapse and C4 world-order behavior.

**Files:** No edits unless failures are caused by this change.

**Steps:**
1. Run:

```powershell
cargo test c4_on_cabhut --lib -- --nocapture
cargo test bridge_orchestrator --lib -- --nocapture
cargo test bridge_repair --lib -- --nocapture
```

2. Run broader checks:

```powershell
cargo fmt --check
cargo check
```

3. If unrelated failures appear, record exact command and first unrelated error. Do not fix unrelated dirty-worktree failures.

**Expected:** Focused CABHUT and bridge tests pass; formatting passes for touched files; broad check either passes or unrelated failure is documented.

### Task 11: Handoff Notes

**Why:** Keep the remaining bridge-collapse work clear after this fix lands.

**Files:**
- Final implementation response only unless implementation discovers a stale doc that must be patched.

**Steps:**
1. Summarize changed files.
2. List tests run.
3. State that collapse audio remains separate.
4. State that event `0x1F` remains intentionally stubbed for skirmish.
5. State whether ramp/endpoint predicate fidelity was fully expressible with current terrain facts or required a follow-up.
6. Do not commit unless the user explicitly asks.

---

## Sources & References

- **Design doc:** `docs/plans/2026-05-22-cabhut-no-overlay-fallback-design.md`
- **Priority verification:** `docs/research/BRIDGE_NEXT_FIX_PRIORITY_VERIFICATION_GHIDRA_REPORT.md`
- **Primary Ghidra report:** `docs/research/BRIDGE_HUT_FALLBACK_FLAGS_RAMP_ONLY_GHIDRA_REPORT.md`
- **Related report:** `docs/research/BRIDGE_COLLAPSE_SOUND_SOURCE_GHIDRA_REPORT.md`
- **Related report:** `docs/research/BRIDGE_DESTROYED_TRIGGER_EVENT_0X1F_GHIDRA_REPORT.md`
- **Ghidra addresses:**
  - `0x00574000` - `MapClass__DestroyBridge_High_OnHutDeath`
  - `0x00574C20` - `MapClass__DestroyBridge_Low_OnHutDeath`
  - `0x005746C0` - `MapClass__IsBridgeRampTile`
  - `0x00574600` - `MapClass__IsLowBridgeEndpointTile`
  - `0x00587180` - `ApplyDamageToCell`
- **Related code:**
  - `src/sim/world/bridge_orchestrator.rs`
  - `src/sim/world/world_orders_bridge_repair_tests.rs`
  - `src/map/bridge_facts.rs`
  - `src/map/resolved_terrain.rs`
  - `src/sim/bridge_state/mod.rs`
  - `src/sim/bridge_state/walker.rs`
