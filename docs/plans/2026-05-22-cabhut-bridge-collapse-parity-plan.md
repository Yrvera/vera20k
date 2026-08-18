# CABHUT Bridge Collapse Parity Implementation Plan

> Execute task-by-task. Do not start Rust edits until Task 1 confirms the remaining jitter-placement detail.

**Goal:** Implement the approved Approach A design for CABHUT-triggered bridge collapse parity: canonical seed selection, bounded walker pre-destroy effects, per-`BlowUpBridge` fallout, debris RNG/placement, and TWLT sound metadata.

**Design Doc:** [docs/plans/2026-05-22-cabhut-bridge-collapse-parity-design.md](2026-05-22-cabhut-bridge-collapse-parity-design.md)

**Scope:** CABHUT collapse execution only. Do not fold in C4 plant Iron Curtain timing, engineer repair cursor/radar, minimap terrain dirtying, or any unverified bridge follow-up items.

---

## Grounding Summary

- `gamemd.exe` behavior is the spec.
- CABHUT C4 expiry dispatches bridge collapse and leaves the hut itself alive. Source: `CABHUT_C4_COLLAPSE_ENTRY_GHIDRA_REPORT.md`, `BRIDGE_COLLAPSE_SYSTEM_MODEL_SYNTHESIS.md`.
- CABHUT bridge collapse routes through `DestroyBridgeFromCell_Low/High`, which canonicalizes the seed before the bounded walker. Source: `BRIDGE_CABHUT_SEED_AND_PRESENTATION_FOCUSED_RECHECK.md`.
- The bounded walker runs four axial steps max, and each step can spawn three perpendicular `BridgeExplosions` before the `DestroyBridge_*` retry loop. Source: live Ghidra spot-checks in the focused recheck and synthesis.
- `CellClass::BlowUpBridge` fallout is per actual `BlowUpBridge` cell, not per aggregate destroyed bridge cell. Source: `CellClass__BlowUpBridge @ 0x0047DD70` recheck.
- Standard YR `BlowUpBridge` debris is gated by `BridgeExplosions.ActiveCount > 0`, not `BridgeVoxelMax`, and uses normalized `RandomRanged(0, 0x7FFFFFFE)` draws for the 95 percent gate, jitter, and metallic 50 percent gate. Source: focused recheck.
- TWLT sounds come from the selected animation's `StartSound` or fallback `Report`, and play when the delayed animation starts. Source: `BRIDGE_DEEP_SLOT5_AUDIO_RENDER_PRESENTATION_TRACE.md`, `ANIMATION_SOUNDS_GHIDRA_REPORT.md`.

## Open Questions

### Must Resolve Before Rust Edits

- Exact conversion from normalized jitter draws to sub-cell/lepton placement for bridge explosion and metallic debris effects. The RNG ranges and ordering are verified, but pixel placement is not yet precise enough.
- Exact comparison predicates for the 95 percent debris gate and metallic 50 percent gate. The plan must use the binary's threshold/comparison, not a label-derived approximation.

### Deferred Out Of Scope

- C4 plant Iron Curtain timing mismatch.
- Engineer repair cursor identity, repair adjacency/hut-cell resolution, radar event type 14, and minimap dirtying.
- Dynamic minimap terrain update for bridge collapse.
- Generic `AnimTypeClass` sound system beyond the minimal TWLT hook needed here.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Verify | `docs/research/traces/BRIDGE_CABHUT_SEED_AND_PRESENTATION_FOCUSED_RECHECK.md` | Update with the jitter placement fact if Ghidra confirms it |
| Modify | `src/sim/world/bridge_orchestrator.rs` | Seed canonicalization, bounded walker presentation, per-cell fallout, debris RNG/placement |
| Modify | `src/rules/art_data.rs` and related art-data tests | Parse/expose `StartSound` / `Report` for bridge explosion anims if not already modeled |
| Modify | `src/app_init.rs` or `src/app_init_helpers.rs` | Intern and pass selected anim sound metadata into `Simulation` if needed |
| Modify | `src/sim/components.rs` | Add pure sim metadata for selected anim sound if using `WorldEffect` |
| Modify | `src/sim/world/mod.rs` | Add pure `SimSoundEvent` only if effect metadata cannot cleanly carry delayed anim sound |
| Modify | `src/app_sim_tick.rs` | Convert selected TWLT sound metadata to app-layer playback at delayed effect start |
| Modify | `src/audio/events.rs` | Add app-layer event variant only if existing `GameSoundEvent` variants cannot represent delayed anim sound |
| Modify | `src/sim/world/world_orders_bridge_repair_tests.rs` or `src/sim/world/world_tests.rs` | CABHUT collapse integration tests |
| Modify | `src/sim/world/bridge_orchestrator.rs` tests module | Canonicalization, RNG, debris, and fallout unit tests |

## Interface Changes

- Keep `dispatch_bridge_collapse_from_hut(sim, rules, hut_center) -> bool` as the external hut-collapse entry point.
- Internal helper signatures may change to pass a sim-local presentation context into the bounded walker.
- `WorldEffect` may gain optional sound metadata. If so, every existing `WorldEffect` construction site must set the new field explicitly.
- Preferred delayed-sound contract: `WorldEffect` carries optional selected anim sound metadata and a one-shot "sound already emitted" flag; `WorldEffect` ticking in `Simulation::tick` returns or queues a start-edge event when delay reaches zero. Sim then emits a pure `SimSoundEvent` for app/audio conversion. Do not rely on `app_sim_tick` polling `WorldEffect` internals after sim tick.

## Sim Checklist

- [ ] No render/ui/sidebar/audio/net imports in `sim/`.
- [ ] No `f32`/`f64` in gameplay or bridge execution logic.
- [ ] Any new persistent simulation state is included in world hashing, or explicitly marked `serde(skip)`/transient like existing effect queues.
- [ ] RNG draw order matches the verified ledger.
- [ ] BTreeSet/BTreeMap iteration remains deterministic.
- [ ] Existing dirty worktree changes outside the touched bridge/effect/audio-adapter files are left alone.

## Risk Areas

- **Seed off-by-one:** wrong probe axis or wrong +1/0/-1 adjustment shifts the collapsed footprint.
- **Axis naming trap:** Ghidra labels can mislead. Code should name physical span axis and overlay subrange clearly.
- **RNG order drift:** emitting walker effects after mutation, or grouping all debris after the aggregate collapse, changes RNG consumption and output.
- **Borrow checker pressure:** avoid solving borrow issues by delaying event generation past the verified point.
- **Sound timing:** TWLT sound should play when the delayed anim starts, not when the effect is enqueued.
- **Fallout scope:** `destroyed_set` is too broad for DropIn/debris. Use actual `BlowUpBridge` cells.
- **Fallout order:** scoping to actual `BlowUpBridge` cells is not enough. Each cell must execute ground kill, deck DropIn, collapsed-cell append/notification, then debris before moving to the next `BlowUpBridge` cell.

---

## Tasks

### Task 1: Verify jitter-to-subcell placement in Ghidra

**Why:** The implementation must not guess pixel placement for debris and TWLT animations.

**Files:** no Rust edits.

**Steps:**

1. Re-open the relevant functions:
   - `CellClass__BlowUpBridge @ 0x0047DD70`
   - high walker function already checked around `0x00575BA0`
   - the matching EW/high and low walker functions if the coordinate math is factored differently.
2. Trace the values returned by the two `RandomRanged(0, 0x7FFFFFFE)` jitter calls into the animation coordinate or cell-offset constructor.
3. Verify and record the exact comparison predicates for:
   - the outer 95 percent debris gate,
   - the metallic 50 percent gate,
   - whether the same predicate style is used in walker pre-destroy and `BlowUpBridge` debris paths.
4. Record the exact transform:
   - final X/Y sub-cell coordinate or lepton offset,
   - signed vs unsigned interpretation,
   - modulo/division/shift constants,
   - whether metallic debris and TWLT use the same transform,
   - whether walker pre-destroy effects and `BlowUpBridge` debris share the same transform.
5. Update `docs/research/traces/BRIDGE_CABHUT_SEED_AND_PRESENTATION_FOCUSED_RECHECK.md` immediately with the verified facts.
6. If the transform or comparison predicates cannot be verified, stop and report the blocker instead of implementing centered or guessed placement/probability.

**Acceptance:** Documented verified transform and gate predicates exist, or implementation is blocked with a clear reason.

### Task 2: Add canonical seed helper and tests

**Why:** Current Rust uses the first 5x5 overlay hit directly, but gamemd canonicalizes through `DestroyBridgeFromCell_*`.

**Files:**

- Modify: `src/sim/world/bridge_orchestrator.rs`

**Steps:**

1. Replace or wrap `find_destroy_overlay_seed` with a helper that:
   - finds the first X-major scan hit,
   - classifies low/high subrange,
   - determines physical span axis,
   - probes one and two cells behind along the verified axis,
   - returns the adjusted seed and physical axis.
2. Keep bridge write-family classification separate from physical span axis.
3. Add unit tests:
   - `cabhut_seed_canonicalization_shifts_edge_hit_forward`
   - `cabhut_seed_canonicalization_keeps_middle_hit`
   - `cabhut_seed_canonicalization_shifts_two_cells_in_backward`
   - `cabhut_seed_canonicalization_maps_high_subranges_to_physical_axes`
   - `cabhut_seed_canonicalization_maps_low_subranges_to_physical_axes`
4. Run:
   - `cargo test cabhut_seed_canonicalization`

**Acceptance:** Canonicalization tests pass and `dispatch_bridge_collapse_from_hut` uses the canonical seed.

### Task 3: Preserve bounded collapse footprint with long-span regression

**Why:** This prevents accidentally reintroducing old full-span behavior while fixing the seed.

**Files:**

- Modify: `src/sim/world/world_orders_bridge_repair_tests.rs` or the bridge orchestrator test module.

**Steps:**

1. Add or update a long-bridge CABHUT fixture.
2. Trigger one CABHUT collapse.
3. Assert cells near the canonical bounded footprint are destroyed/damaged as expected.
4. Assert far-end cells on the same long bridge remain intact.
5. Run:
   - `cargo test cabhut_bounded_collapse_preserves_far_cells_on_long_bridge`

**Acceptance:** One CABHUT event preserves cells outside the four-step bounded footprint.

### Task 4: Add a sim-local bridge animation spawn helper

**Why:** Walker pre-destroy effects and `BlowUpBridge` debris need the same verified RNG/placement/sound metadata behavior.

**Files:**

- Modify: `src/sim/world/bridge_orchestrator.rs`
- Modify: `src/sim/components.rs` if effect sound metadata is carried on `WorldEffect`

**Steps:**

1. Add a helper that selects a SHP ID from a provided interned list using inclusive `RandomRanged(0, count - 1)` semantics.
2. Apply the verified jitter transform from Task 1.
3. Set delay in the same units currently used by `WorldEffect.delay_ms`, preserving the verified `RandomRanged(1,5)` frame delay.
4. Attach optional selected anim sound metadata, using `StartSound` or fallback `Report` if available in current art data. If current rules/art data does not expose these fields, add the smallest parser/data-model extension needed and test it.
5. If art-data plumbing is needed, add explicit tests proving `TWLT026`, `TWLT036`, `TWLT050`, and `TWLT070` expose their selected `StartSound` or fallback `Report` IDs from loaded art data.
6. Keep the helper pure sim-side: no render/audio imports.

**Acceptance:** The helper can spawn bridge explosion or metallic debris effects with deterministic placement, delay, and optional sound metadata sourced from art data.

### Task 5: Emit walker pre-destroy `BridgeExplosions`

**Why:** gamemd shows three perpendicular TWLT effects before bridge mutation at each walker step.

**Files:**

- Modify: `src/sim/world/bridge_orchestrator.rs`

**Steps:**

1. Pass a small presentation context into `run_hut_collapse_bounded`, or otherwise allow it to enqueue effects at the verified point.
2. Before each per-step `DestroyBridge_*` retry loop, check the same terminal destroyed-cap condition verified in the focused recheck.
3. If `BridgeExplosions` is non-empty and the center cell is eligible, spawn three perpendicular bridge explosions.
4. Consume RNG in the verified per-animation order:
   - jitter X,
   - jitter Y,
   - delay `1..=5`,
   - explosion slot.
5. Add tests:
   - `cabhut_walker_pre_destroy_effects_emit_before_mutation_debris`
   - `cabhut_walker_pre_destroy_effects_consume_verified_rng_order`
6. Run:
   - `cargo test cabhut_walker_pre_destroy_effects`

**Acceptance:** Pre-destroy effects exist, are ordered before mutation/debris, and consume RNG in the verified order.

### Task 6: Refactor hut fallout to actual `BlowUpBridge` cells

**Why:** Rust currently uses aggregate `destroyed_set` for DropIn/debris; gamemd uses actual `BlowUpBridge` cells.

**Files:**

- Modify: `src/sim/world/bridge_orchestrator.rs`

**Steps:**

1. Preserve `destroyed_set` for bridge state notifications and span collapse bookkeeping.
2. Build a separate deterministic ordered collection of actual `BlowUpBridge` cells from `set_bridge_direction.actions`.
3. Add a `blow_up_bridge_cell_fallout` helper and call it once per actual `BlowUpBridge` cell.
4. Inside that helper, preserve the verified per-cell order:
   - force-kill ground occupants with C4Warhead semantics,
   - DropIn bridge-deck occupants,
   - append/notify the collapsed cell,
   - run the debris block for that same cell.
5. Do not batch "all kills, then all DropIns, then all debris" across the whole collapse.
6. Add tests:
   - `blow_up_bridge_fallout_scopes_dropin_and_debris_to_blowup_cells`
   - `blow_up_bridge_fallout_preserves_per_cell_order`
7. Run:
   - `cargo test blow_up_bridge_fallout_scopes`

**Acceptance:** DropIn/debris no longer run on non-`BlowUpBridge` cells in `destroyed_set`, and the per-cell fallout order matches the verified `CellClass::BlowUpBridge` order.

### Task 7: Fix `BlowUpBridge` debris RNG and gates

**Why:** Current Rust uses different RNG ranges, centers placement, and gates metallic debris through `BridgeVoxelMax`.

**Files:**

- Modify: `src/sim/world/bridge_orchestrator.rs`

**Steps:**

1. Change the debris block outer gate to require `BridgeExplosions.ActiveCount > 0`.
2. Remove `BridgeVoxelMax` from standard YR `BlowUpBridge` debris gating.
3. Use the exact Task 1 verified comparison predicate for the outer 95 percent gate. Do not implement this from the percentage label alone.
4. Use normalized `RandomRanged(0, 0x7FFFFFFE)` semantics for:
   - jitter X,
   - jitter Y,
   - metallic 50 percent gate.
5. Use the exact Task 1 verified comparison predicate for the metallic 50 percent gate.
6. Spawn metallic debris only if metallic gate passes and `MetallicDebris.ActiveCount > 0`.
7. Always attempt one delayed `BridgeExplosions` animation after the metallic branch when the outer block passes.
8. Add tests:
   - `bridge_debris_ignores_bridge_voxel_max_for_standard_blowupbridge`
   - `bridge_debris_gated_by_bridge_explosions_not_metallic_debris`
   - `bridge_debris_consumes_normalized_rng_draws`
   - `bridge_debris_uses_verified_gate_predicates`
9. Run:
   - `cargo test bridge_debris`

**Acceptance:** Debris tests prove gates and RNG order match the verified ledger.

### Task 8: Route TWLT selected anim sound metadata

**Why:** Standard YR bridge collapse produces TWLT sounds from the selected animation, not from a hardcoded bridge sound.

**Files:**

- Modify: `src/sim/components.rs`
- Modify: `src/sim/world/mod.rs` only if needed
- Modify: `src/app_sim_tick.rs`
- Modify: `src/audio/events.rs` only if existing app event variants are insufficient

**Steps:**

1. Use the preferred contract unless implementation proves it unworkable: `WorldEffect` carries optional selected anim sound metadata plus a one-shot "sound emitted" flag.
2. Change `WorldEffect::tick` or the surrounding `Simulation::tick` world-effect loop so crossing from delayed to active returns/queues a start-edge sound event exactly once.
3. Sim emits a pure `SimSoundEvent` carrying the selected sound ID and effect cell when that start edge happens.
4. `app_sim_tick` converts that `SimSoundEvent` into a positional `GameSoundEvent`.
5. Do not make `app_sim_tick` infer the start edge by inspecting `world_effects` after sim ticking; the start edge lives inside sim because sim owns `WorldEffect::tick`.
6. Add tests:
   - `bridge_twlt_sound_metadata_uses_selected_anim_start_or_report`
   - `bridge_twlt_sound_fires_once_when_delay_elapses`
   - `bridge_twlt_sound_does_not_fire_when_effect_is_enqueued`
7. Run:
   - `cargo test bridge_twlt_sound_metadata`

**Acceptance:** A selected TWLT effect has the correct symbolic sound ID, does not play before its delay elapses, and fires exactly once on the sim-owned start edge.

### Task 9: Run focused bridge/CABHUT tests

**Why:** Catch interaction regressions without running the whole suite first.

**Commands:**

```powershell
cargo test cabhut_seed_canonicalization
cargo test cabhut_bounded_collapse_preserves_far_cells_on_long_bridge
cargo test cabhut_walker_pre_destroy_effects
cargo test blow_up_bridge_fallout_scopes
cargo test bridge_debris
cargo test bridge_twlt_sound_metadata
cargo test world_orders_bridge_repair
```

**Acceptance:** All focused tests pass.

### Task 10: Format and run the smallest broad check

**Why:** Ensure the patch is clean without masking unrelated dirty worktree changes.

**Commands:**

```powershell
cargo fmt
cargo test bridge
git diff --check -- src/sim/world/bridge_orchestrator.rs src/sim/components.rs src/sim/world/mod.rs src/app_sim_tick.rs src/audio/events.rs src/sim/world/world_orders_bridge_repair_tests.rs src/sim/world/world_tests.rs
```

If `cargo test bridge` is too broad or fails on unrelated dirty worktree changes, report the exact failing test and rerun the smallest relevant filters from Task 9.

**Acceptance:** Formatting complete, focused tests pass, and whitespace check has no new non-CRLF issues.

## Do Not Do

- Do not start another swarm for this implementation.
- Do not implement full-span CABHUT collapse.
- Do not use `[CombatDamage] DestroyableBridges` as a collapse execution gate.
- Do not gate standard YR `BlowUpBridge` debris with `BridgeVoxelMax`.
- Do not guess the jitter placement transform.
- Do not import render/audio/ui/sidebar/net into `sim/`.
- Do not edit unrelated dirty worktree files.
