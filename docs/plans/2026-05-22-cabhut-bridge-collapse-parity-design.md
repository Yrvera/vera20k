# CABHUT Bridge Collapse Parity Design

## Goal

Make CABHUT-triggered bridge collapse match `gamemd.exe` for seed selection, bounded walker presentation, per-cell `BlowUpBridge` fallout, debris RNG, and TWLT sound data.

## Architecture Context

Bridge collapse execution is currently owned by `src/sim/world/bridge_orchestrator.rs`. CABHUT C4 expiry enters through `src/sim/world/world_orders.rs`, but the hut-specific bridge work is delegated to `dispatch_bridge_collapse_from_hut`.

The simulation already keeps the relevant systems separated:

- `world_orders.rs` owns the C4 timer and decides when a BridgeRepairHut collapse dispatch happens.
- `bridge_orchestrator.rs` owns bridge collapse mutation, fallout, bridge debris effects, and adjacent-zone updates.
- `sim/bridge_state/*` owns deterministic bridge cell state and walker primitives.
- `WorldEffect` in `src/sim/components.rs` is the existing sim-local representation for temporary SHP effects.
- `SimSoundEvent` in `src/sim/world/mod.rs` is pure data drained by the app layer; app/audio code converts it into `GameSoundEvent`.

The design must preserve the project boundary that `sim/` does not depend on render, ui, audio, net, or sidebar. All gameplay and RNG logic stays deterministic and fixed-point/integer based.

## Impact Analysis

Expected implementation touch points:

- `src/sim/world/bridge_orchestrator.rs`
  - Replace direct first-hit seed use with `DestroyBridgeFromCell_*`-style canonicalization.
  - Add walker pre-destroy `BridgeExplosions` emission before each destroy retry loop.
  - Scope DropIn and debris to actual `BlowUpBridge` cells.
  - Fix debris RNG ranges, gates, ordering, and placement.
- `src/sim/components.rs`
  - Extend `WorldEffect` or add an equivalent pure-data effect event to carry optional selected anim sound metadata.
- `src/sim/world/mod.rs`
  - Add a pure `SimSoundEvent` variant only if sound cannot ride cleanly on effect metadata.
- `src/app_sim_tick.rs`
  - Convert the sim-side selected anim sound metadata into app-layer sound playback when the delayed TWLT effect starts.
- Focused tests under existing bridge/CABHUT test modules.

Risk areas:

- RNG draw count/order can drift if walker visuals or debris effects are emitted after mutation instead of at the gamemd point.
- Borrowing pressure in `dispatch_bridge_collapse_from_hut` may tempt a two-pass design that changes RNG ordering.
- Aggregate `destroyed_set` is broader than actual `BlowUpBridge` cells; using it for DropIn/debris is a known parity bug.
- Adding sound data to sim must not create a sim-to-audio dependency.
- Existing worktree is broadly dirty, so the implementation must stay scoped and avoid unrelated app/render/sidebar edits.

## Chosen Approach

Use a targeted parity repair in the existing bridge orchestrator.

The bridge orchestrator remains the owner of CABHUT collapse execution. The direct first-overlay seed selection is replaced with a canonicalizing helper that models `DestroyBridgeFromCell_Low/High`. The existing bounded walker is retained, but it receives a small sim-local presentation context so it can consume RNG and enqueue the three pre-destroy `BridgeExplosions` before each step's bridge mutation attempts.

Fallout is shifted toward a per-actual-`BlowUpBridge` cell helper so ground kills, deck DropIn, collapsed-cell notification, debris, and TWLT effect creation happen in the verified order. This avoids reconstructing presentation after the aggregate state outcome and keeps the player-visible behavior tied to the same cells gamemd uses.

Before implementation, run one narrow read-only Ghidra check for the exact jitter-to-subcell coordinate transform. The current reports verify RNG ranges and order, but not the final coordinate conversion precisely enough for pixel parity.

## Tiny-Detail Ledger

- CABHUT C4 branch is active in standard YR and the hut survives; the branch dispatches bridge collapse instead of normal building damage. Source: `BRIDGE_COLLAPSE_SYSTEM_MODEL_SYNTHESIS.md`, `CABHUT_C4_COLLAPSE_ENTRY_GHIDRA_REPORT.md`.
- Hut entry routes through `DestroyBridgeFromCell_Low/High`, not direct first overlay hit. Source: `BRIDGE_CABHUT_SEED_AND_PRESENTATION_FOCUSED_RECHECK.md`, `MapClass__DestroyBridgeFromCell_High @ 0x005749C0`, `MapClass__DestroyBridgeFromCell_Low @ 0x00574780`.
- High subranges `0xCD..=0xD5`, `0xDF..=0xE2`, `0xE7` route to physical EW high walker. Source: `BRIDGE_CABHUT_SEED_AND_PRESENTATION_FOCUSED_RECHECK.md`.
- High subranges `0xD6..=0xDE`, `0xE3..=0xE6`, `0xE8` route to physical NS high walker. Source: `BRIDGE_CABHUT_SEED_AND_PRESENTATION_FOCUSED_RECHECK.md`.
- Low subranges `0x4A..=0x52`, `0x5C..=0x5F`, `0x64` route to the low counterpart of the physical EW branch; `0x53..=0x5B`, `0x60..=0x63`, `0x65` route to the low counterpart of the physical NS branch. Source: `BRIDGE_CABHUT_SEED_AND_PRESENTATION_FOCUSED_RECHECK.md`.
- Canonicalization probes one and two cells behind the matched overlay: NS subrange probes `(x, y - 1)` and `(x, y - 2)`; EW subrange probes `(x - 1, y)` and `(x - 2, y)`. Source: `BRIDGE_CABHUT_SEED_AND_PRESENTATION_FOCUSED_RECHECK.md`.
- If the first back probe is outside the bridge band, gamemd calls the collapse walker at `matched + 1`; if one back probe is inside and the second is outside, it calls at `matched`; if both are inside, it calls at `matched - 1` or the helper-equivalent coordinate. Source: `BRIDGE_CABHUT_SEED_AND_PRESENTATION_FOCUSED_RECHECK.md`.
- CABHUT walker is bounded to exactly four axial iterations, with up to three `DestroyBridge_*` retries per iteration. Source: `BRIDGE_COLLAPSE_SYSTEM_MODEL_SYNTHESIS.md`, live Ghidra `CollapseBridge_NS_High @ 0x00575BA0`.
- The walker biases the start with `(back - fwd) / 2` using signed integer division toward zero, then steps toward the longer side. Source: current Rust comments backed by `BRIDGE_COLLAPSE_SYSTEM_MODEL_SYNTHESIS.md`.
- Before each walker step's destroy retry loop, gamemd spawns three perpendicular `BridgeExplosions` when the center cell is not the terminal destroyed cap for that walker. Source: `BRIDGE_CABHUT_SEED_AND_PRESENTATION_FOCUSED_RECHECK.md`.
- Each walker pre-destroy animation consumes X jitter `RandomRanged(0, 0x7FFFFFFE)`, Y jitter `RandomRanged(0, 0x7FFFFFFE)`, delay `RandomRanged(1,5)`, and bridge explosion slot `RandomRanged(0, BridgeExplosions.ActiveCount - 1)`. Source: `BRIDGE_CABHUT_SEED_AND_PRESENTATION_FOCUSED_RECHECK.md`.
- `BlowUpBridge` order per actual cell is ground occupant kill, deck occupant DropIn, collapsed-cell queue append, then debris block. Source: `BRIDGE_CABHUT_SEED_AND_PRESENTATION_FOCUSED_RECHECK.md`, `CellClass__BlowUpBridge @ 0x0047DD70`.
- The debris block is gated by `BridgeExplosions.ActiveCount > 0`. Metallic debris alone does not enable it. Source: `BRIDGE_CABHUT_SEED_AND_PRESENTATION_FOCUSED_RECHECK.md`.
- `BridgeVoxelMax` does not gate standard YR `BlowUpBridge` debris. Source: `BRIDGE_CABHUT_SEED_AND_PRESENTATION_FOCUSED_RECHECK.md`.
- Per `BlowUpBridge` debris cell, the outer 95 percent gate uses `RandomRanged(0, 0x7FFFFFFE)`. Source: `BRIDGE_CABHUT_SEED_AND_PRESENTATION_FOCUSED_RECHECK.md`.
- Per `BlowUpBridge` debris cell, jitter X and Y each use `RandomRanged(0, 0x7FFFFFFE)`. Source: `BRIDGE_CABHUT_SEED_AND_PRESENTATION_FOCUSED_RECHECK.md`.
- Per `BlowUpBridge` debris cell, metallic 50 percent gate uses `RandomRanged(0, 0x7FFFFFFE)`; if it passes, slot selection uses `RandomRanged(0, MetallicDebris.ActiveCount - 1)`. Source: `BRIDGE_CABHUT_SEED_AND_PRESENTATION_FOCUSED_RECHECK.md`.
- Per `BlowUpBridge` debris cell, one delayed `BridgeExplosions` animation is attempted with delay `RandomRanged(1,5)` and slot `RandomRanged(0, BridgeExplosions.ActiveCount - 1)`. Source: `BRIDGE_CABHUT_SEED_AND_PRESENTATION_FOCUSED_RECHECK.md`.
- TWLT sounds come from the selected animation's `StartSound`, falling back to `Report`, and should play when the delayed animation starts. Source: `BRIDGE_DEEP_SLOT5_AUDIO_RENDER_PRESENTATION_TRACE.md`, `ANIMATION_SOUNDS_GHIDRA_REPORT.md`.
- Exact jitter-to-subcell coordinate conversion is `UNKNOWN - needs narrow Ghidra check` before implementation.

## Design

### Components

`canonicalize_hut_destroy_seed`

- Input: `BridgeRuntimeState`, scan hit `(rx, ry)`, family, overlay byte.
- Output: canonical seed `(rx, ry)` plus physical span axis.
- Responsibility: model the live `DestroyBridgeFromCell_Low/High` seed adjustment without importing Ghidra naming confusion into the rest of the walker.

`HutCollapsePresentation`

- A small sim-local context passed into `run_hut_collapse_bounded`.
- Holds mutable access to RNG/effect queues without exposing render/audio types.
- Emits pre-destroy `BridgeExplosions` in the verified order.

`blow_up_bridge_cell_fallout`

- Runs the fallout for actual `CellAction::BlowUpBridge` cells only.
- Performs ground kill, deck DropIn, collapsed-cell append/notification, debris, and bridge explosion effect emission in verified order.
- Keeps aggregate `destroyed_set` only for state notifications and bridge span collapse bookkeeping.

`WorldEffect` sound metadata or `SimSoundEvent::AnimStarted`

- Pure data only.
- Carries the selected TWLT anim's start/report sound ID as an `InternedId`.
- The app layer converts it to `GameSoundEvent` at delayed effect start.

### Interfaces / Contracts

- `dispatch_bridge_collapse_from_hut` remains the public hut-collapse entry point.
- `run_hut_collapse_bounded` remains a bounded four-step walker; it must not become full-span.
- Seed canonicalization returns physical span axis, while bridge-state write primitives keep their existing family-specific meaning.
- Presentation helpers may enqueue `WorldEffect` and pure `SimSoundEvent`, but cannot reference render/audio/app modules.
- All RNG is consumed from `Simulation.rng` in the same order the binary consumes it.

### Data Flow

1. CABHUT C4 timer expiry calls `dispatch_bridge_collapse_from_hut`.
2. Hut-local 5x5 scan remains X-major.
3. First bridge overlay candidate is classified by family/subrange.
4. Canonicalization probes back one/two cells and returns the adjusted seed plus physical axis.
5. Bounded walker measures extents, chooses biased start and step direction, and runs up to four iterations.
6. Each walker iteration emits pre-destroy perpendicular `BridgeExplosions` before `DestroyBridge_*` retries.
7. Each collapsed outcome records actual `BlowUpBridge` cells separately from all destroyed bridge state cells.
8. Actual `BlowUpBridge` cells run fallout and debris in verified order.
9. Aggregate destroyed cells still feed adjacent bridge updates, bridge-span notifications, and zone refresh.
10. App/audio layer plays selected TWLT sound when the delayed effect starts.

### Error Handling

Bridge collapse remains best-effort deterministic simulation behavior:

- Missing bridge state or resolved terrain returns `false` as today.
- Missing `BridgeExplosions` means no walker/debris explosion effects and no TWLT sound.
- Missing `MetallicDebris` only suppresses metallic debris; it must not suppress TWLT debris block when `BridgeExplosions` exists.
- Missing effect frame count keeps the existing fallback frame count behavior unless a later art-system design replaces it.
- Missing selected anim sound simply produces a silent visual effect.

### Testing Strategy

Focused tests should include:

- `cabhut_seed_canonicalization_shifts_edge_hit_forward`
- `cabhut_seed_canonicalization_keeps_middle_hit`
- `cabhut_seed_canonicalization_shifts_two_cells_in_backward`
- `cabhut_seed_canonicalization_maps_high_subranges_to_physical_axes`
- `cabhut_seed_canonicalization_maps_low_subranges_to_physical_axes`
- `cabhut_bounded_collapse_preserves_far_cells_on_long_bridge`
- `cabhut_walker_pre_destroy_effects_emit_before_mutation_debris`
- `cabhut_walker_pre_destroy_effects_consume_verified_rng_order`
- `blow_up_bridge_fallout_scopes_dropin_and_debris_to_blowup_cells`
- `bridge_debris_ignores_bridge_voxel_max_for_standard_blowupbridge`
- `bridge_debris_gated_by_bridge_explosions_not_metallic_debris`
- `bridge_debris_consumes_normalized_rng_draws`
- `bridge_twlt_sound_metadata_uses_selected_anim_start_or_report`

Run the smallest relevant cargo test filters for bridge orchestrator, bridge repair/CABHUT, and debris RNG tests after implementation.

## Architectural Decisions

- Keep the fix inside existing bridge orchestration instead of introducing a new bridge-collapse subsystem. The current module already owns the mutation and fallout path, so this minimizes boundary churn.
- Use pure sim data for visual/sound scheduling. This preserves the `sim/` dependency rule while allowing app/audio to play TWLT sounds later.
- Prefer explicit canonicalization helpers over broad lookup tables with unclear naming. The Ghidra labels around NS/EW are known to be misleading, so helper names should describe physical axis and overlay subrange clearly.
- Do not use `destroyed_set` as a proxy for `BlowUpBridge` cells. This is the central current mismatch for DropIn/debris scope.
- Do not implement broader C4 plant Iron Curtain timing, engineer repair cursor/radar, or minimap terrain dirty fixes in this design. They are separate player-visible parity gaps with different entry points.

## Alternatives Considered

### Return presentation events after walker mutation

This is easier to borrow-check, but it risks changing RNG order and event timing. The binary emits walker pre-destroy effects before the destroy retry loop, so this design was rejected.

### Build a generic `AnimTypeClass` event system first

This would eventually be useful for broader animation sound parity, but it is larger than the confirmed CABHUT collapse fix and would touch app/audio/render surfaces while the worktree is already broadly dirty. The bridge fix only needs a minimal pure-data sound hook.

### Keep aggregate `destroyed_set` debris and only fix RNG

This would improve randomness but leave visible DropIn/debris effects on the wrong cells. It was rejected because it preserves a known parity bug.

## Required Pre-Code Verification

Before implementing the jittered effect placement, do one narrow read-only Ghidra spot-check of the coordinate math that converts the normalized jitter draws into sub-cell/lepton offsets for `BridgeExplosions` and `MetallicDebris`.

The current evidence verifies active paths, RNG ranges, RNG order, sound source, and gating. It does not yet pin the final coordinate transform precisely enough for pixel parity.
