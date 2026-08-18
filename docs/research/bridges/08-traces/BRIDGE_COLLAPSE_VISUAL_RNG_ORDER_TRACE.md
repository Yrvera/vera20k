# Bridge Collapse Visual RNG Order Trace

Scenario: CABHUT-triggered bridge collapse on a visible high bridge segment.

Scope: Trace gamemd BridgeExplosions and MetallicDebris visual spawn timing and RNG draw order for high `CollapseBridge_*` and `BlowUpBridge`, then compare to current Rust `spawn_bridge_debris` / `WorldEffect` timing. This trace does not implement fixes and does not expand into occupant, trigger, zone, or bridge-footprint behavior except where visual/RNG ordering depends on it.

## Verdict

Status: COMPLETE.

Tally: PASS: 1 | FAIL: 6 | UNCHECKED: 2 | NOT-IMPLEMENTED: 1

The current Rust collapse shape has the bounded hut walker, but visual/RNG order is still not gamemd-equivalent. gamemd has a two-layer visual path:

1. `CollapseBridge_EW_High` / `CollapseBridge_NS_High` spawns three `BridgeExplosions` animations before each per-step `DestroyBridge_High` call.
2. Later, each `CellClass::BlowUpBridge` call performs its own per-cell `MetallicDebris` and delayed `BridgeExplosions` block after ground kill, deck DropIn, and collapsed-cell queue insertion.

Rust currently has only one aggregated post-collapse `spawn_bridge_debris` pass over `destroyed_set`. It does not model the walker-spawned pre-destroy explosions, does not preserve gamemd's visual/RNG interleaving around each `DestroyBridge_High` call, and does not emit the selected anim's `Report=` sound when delayed bridge explosions start.

## Concrete Pipeline

Trigger: CABHUT death / C4 detonation finds a high bridge seed and dispatches to bounded high bridge collapse.

gamemd path:

`DestroyBridge_High_OnHutDeath @ 0x00574000` -> `DestroyBridgeFromCell_High @ 0x005749C0` -> `CollapseBridge_EW_High @ 0x00575870` or `CollapseBridge_NS_High @ 0x00575BA0` -> per axial iteration: 3 `BridgeExplosions` anim spawns -> `DestroyBridge_High` retry loop -> eventual `CellClass::BlowUpBridge @ 0x0047DD70` per affected bridge cell -> optional `MetallicDebris` plus delayed `BridgeExplosions` -> `AnimClass::Middle @ 0x00424CE0` plays anim `StartSound` / fallback `Report` when delay expires.

Rust path:

`dispatch_bridge_collapse_from_hut` -> `run_hut_collapse_bounded` -> collect `StateOutcome`s -> `apply_hut_bridge_execution` -> kill ground occupants at `blow_up_cells` -> DropIn deck entities for `destroyed_set` -> `spawn_bridge_debris(sim, rules, &destroyed_set)` -> push `WorldEffect`s.

## Stage Results

### Stage 1 - Active standard YR path

gamemd: `DestroyableBridges=yes` in stock `ini/rulesmd.ini`; `BridgeExplosions=TWLT026,TWLT036,TWLT050,TWLT070`; high `CollapseBridge_EW_High` and `CollapseBridge_NS_High` decompiles are active bridge-collapse walkers.

Rust: high hut collapse dispatch is implemented in `src/sim/world/bridge_orchestrator.rs`.

Verdict: PASS for path activity only.

### Stage 2 - Walker-spawned pre-destroy BridgeExplosions

gamemd: In both high walkers, each axial iteration checks the current center cell, then runs a three-cell perpendicular loop. For each of those three cells it computes jittered coordinates, selects a delay in `1..5`, selects a `BridgeExplosions` anim from the four-entry rules vector, and constructs the `AnimClass`. This happens before the `DestroyBridge_High` retry loop. With a full four-step high bridge segment and no destroyed-anchor skip, the walker schedules 12 pre-destroy `BridgeExplosions`.

Rust: `run_hut_collapse_bounded` calls `call_destroy_per_family` directly inside the four-step loop and has no visual spawn or RNG consumption before the destroy call. Visuals are deferred to `spawn_bridge_debris` after all outcomes have been aggregated.

Verdict: FAIL. Player-visible explosion timing/order differs, and lockstep RNG order diverges before the first bridge overlay mutation in each iteration.

Rust evidence: `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:703`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:708`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:299`.

gamemd evidence: fresh read-only Ghidra decompile of `0x00575870` and `0x00575BA0`; existing `BRIDGE_COLLAPSE_FALLOUT_ORDERING_GHIDRA_REPORT.md` section 3.3.

### Stage 3 - Walker RNG draw count and order

gamemd: For each walker-spawned `BridgeExplosions` anim, the high walkers consume two jitter `RandomRanged(0,0x7ffffffe)` calls, then `RandomRanged(1,5)` for delay, then `RandomRanged(0, BridgeExplosionsCount-1)` for the anim slot. With 3 perpendicular cells x 4 axial iterations, that is 48 random draws before the corresponding per-step destruction work, assuming allocation succeeds and the center cell is not the destroyed-anchor sentinel.

Rust: current hut walker consumes zero visual RNG draws before `call_destroy_per_family`.

Verdict: FAIL. The exact gamemd draw count is 4 per walker anim; Rust count is 0 at this point.

Rust evidence: `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:703`.

gamemd evidence: fresh read-only Ghidra decompile of `0x00575870` and `0x00575BA0`.

### Stage 4 - BlowUpBridge visual block scope and interleaving

gamemd: `CellClass::BlowUpBridge` runs as a per-cell primitive. Its order is ground-list kill, deck-list DropIn, collapsed-cell queue insertion, then optional `MetallicDebris` and one delayed `BridgeExplosions` for that same cell. This is interleaved with the bridge destruction routines that call `BlowUpBridge`.

Rust: `apply_hut_bridge_execution` aggregates all `destroyed_set` cells across all outcomes, performs all DropIns, then calls `spawn_bridge_debris` once for the whole set. The debris pass is not tied to each `BlowUpBridge` call and uses `destroyed_set`, not only `blow_up_cells`.

Verdict: FAIL. Visual timing and RNG order are grouped after all collapse outcomes instead of occurring per `BlowUpBridge` cell.

Rust evidence: `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:258`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:270`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:296`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:299`.

gamemd evidence: fresh read-only Ghidra decompile of `0x0047DD70`; existing `BRIDGE_COLLAPSE_FALLOUT_ORDERING_GHIDRA_REPORT.md` sections 3.1 and 3.5.

### Stage 5 - BlowUpBridge outer gate and jitter math

gamemd: `BlowUpBridge` gates the whole debris/explosion block with `BridgeExplosions.ActiveCount > 0` and a `RandomRanged(0,0x7ffffffe)` double-threshold comparison against `0.95`. If it passes, it consumes two more `RandomRanged(0,0x7ffffffe)` draws and applies visible X/Y jitter of roughly `[-25,+25)` leptons before metallic and bridge explosion spawns.

Rust: `spawn_bridge_debris` approximates the outer gate with `next_range_u32(20) == 0`, consumes two `next_range_u32(0xFFFF)` jitter draws, discards the jitter values, and places both effects at exact cell center.

Verdict: FAIL. Probability may be close, but literal RNG range, threshold behavior, and visible sub-cell positions are not equal.

Rust evidence: `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:1062`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:1068`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:1091`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:1113`.

gamemd evidence: fresh read-only Ghidra decompile of `0x0047DD70`; existing `BRIDGE_COLLAPSE_SOUND_SOURCE_GHIDRA_REPORT.md` sections 3.1 to 3.3.

### Stage 6 - MetallicDebris spawn

gamemd: After the outer gate and jitter, `BlowUpBridge` runs a 50 percent gate; if it passes and `MetallicDebris` is non-empty and allocation succeeds, it selects a metallic debris anim and constructs it with delay 0 at the jittered coordinate.

Rust: `spawn_bridge_debris` implements a 50 percent gate, checks `voxel_max > 0` and non-empty metallic debris, selects a slot, and creates a zero-delay `WorldEffect`, but at cell center and in the aggregated post-collapse pass.

Verdict: FAIL because position and ordering are not equal, despite the broad shape being present.

Rust evidence: `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:1078`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:1083`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:1087`.

gamemd evidence: fresh read-only Ghidra decompile of `0x0047DD70`.

### Stage 7 - Delayed BridgeExplosions visual from BlowUpBridge

gamemd: If the outer gate passed, `BlowUpBridge` attempts one delayed `BridgeExplosions` anim with delay `RandomRanged(1,5)` inclusive and a slot from the four-entry standard YR pool.

Rust: `spawn_bridge_debris` creates one delayed bridge explosion `WorldEffect` with `next_range_u32_inclusive(1,5)` and `delay_ms = delay_frames * 67`.

Verdict: UNCHECKED. The high-level range matches, but this trace did not prove Rust's milliseconds-to-frame scheduling is literally equal to gamemd's frame countdown, and the effect is still ordered after aggregation rather than per `BlowUpBridge`.

Rust evidence: `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:1103`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:1121`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/components.rs:592`.

gamemd evidence: fresh read-only Ghidra decompile of `0x0047DD70` and `0x00421EA0`.

### Stage 8 - Anim Report sound timing

gamemd: The selected `BridgeExplosions` anim plays its `StartSound` or fallback `Report` in `AnimClass::Middle` when the animation starts. For standard YR `TWLT026/TWLT036/TWLT050/TWLT070`, the sounds are `ExplosionShard`, `Explosion06`, `Explosion07`, and `Explosion09`. Because bridge explosion delay is `1..5`, the sound is delayed with the visual, not emitted at collapse time.

Rust: `WorldEffect` has no sound field, and `SimSoundEvent` has no bridge-collapse anim-start event. Existing bridge-specific audio is `BridgeRepaired`, which is repair-only.

Verdict: NOT-IMPLEMENTED.

Rust evidence: `C:/Users/enok/Documents/ra2-rust-game/src/sim/components.rs:563`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/mod.rs:96`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/mod.rs:181`.

gamemd evidence: fresh read-only Ghidra decompile of `0x00421EA0` and `0x00424CE0`; `BRIDGE_COLLAPSE_SOUND_SOURCE_GHIDRA_REPORT.md` sections 3.4 and 4.

### Stage 9 - Render start frame equality

gamemd: `AnimClass::Constructor` initializes animation state and calls `AnimClass::Middle` immediately only if delay is zero. Delayed bridge explosions begin after their delay countdown reaches zero.

Rust: `WorldEffect::tick` returns early while `delay_ms > 0`; when delay reaches zero, frame remains 0 until subsequent elapsed time reaches `rate_ms`.

Verdict: UNCHECKED. This trace did not compute a literal frame-by-frame equality table between gamemd anim countdown and Rust `delay_ms` / `elapsed_ms` behavior.

Rust evidence: `C:/Users/enok/Documents/ra2-rust-game/src/sim/components.rs:592`.

gamemd evidence: fresh read-only Ghidra decompile of `0x00421EA0`.

## Player-Visible Failures

1. Missing pre-destroy walker explosions: a visible high bridge collapse should show up to 12 walker-spawned TWLT explosions before/during the four destroy steps; Rust shows none until the aggregated post-collapse debris pass.
2. Wrong explosion interleaving: gamemd alternates walker visual RNG and `DestroyBridge_High` per axial step; Rust mutates/collects first and spawns visuals later.
3. Centered debris/explosion placement: gamemd jitters bridge visuals by roughly `[-25,+25)` leptons; Rust pins them to cell center.
4. Missing collapse explosion audio: gamemd plays the selected TWLT anim `Report=` sound when delayed visuals start; Rust bridge `WorldEffect`s are visual-only.
5. Aggregated debris scope: gamemd's `BlowUpBridge` visual block is per `BlowUpBridge` cell; Rust runs the block over `destroyed_set`, so cells and RNG order can differ from the binary.

## Adjacent Findings

- Occupant kill and DropIn order are covered by `BRIDGE_COLLAPSE_FALLOUT_ORDERING_GHIDRA_REPORT.md`; this trace did not re-score them except where they order the `BlowUpBridge` visual block.
- Trigger event `0x1F`, zone rebuild, and full redraw are adjacent to collapse fallout but not part of this visual/RNG scenario.
- Exact allocation-failure behavior is not modeled. The equality claims above assume normal successful `AnimClass` allocation, which is the standard gameplay path.

## Implementation Handoff

Required visual/RNG parity direction:

1. Add a separate walker visual path inside hut bounded collapse: for each of the four axial iterations, before `call_destroy_per_family`, spawn three `BridgeExplosions` at the perpendicular cells and consume RNG in gamemd order.
2. Keep that walker visual path separate from `BlowUpBridge` debris. It has no `MetallicDebris` and no outer 95 percent gate.
3. Move or split `spawn_bridge_debris` so `BlowUpBridge` metallic/deferred-explosion RNG is consumed per actual `BlowUpBridge` cell in binary order, not once over an aggregated `destroyed_set`.
4. Replace approximate/discarded jitter with stored sub-cell offsets on `WorldEffect`.
5. Add animation-start sound routing from selected anim `StartSound` / fallback `Report`; do not use `BridgeRepaired`.

## Sources

- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_COLLAPSE_FALLOUT_ORDERING_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_COLLAPSE_SOUND_SOURCE_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_COLLAPSE_CHAIN_MECHANISM_GHIDRA_REPORT.md`
- Fresh read-only Ghidra decompile: `gamemd.exe` `0x00575870`, `0x00575BA0`, `0x0047DD70`, `0x00421EA0`, `0x00424CE0`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/components.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/mod.rs`
- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`
- `C:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini`
