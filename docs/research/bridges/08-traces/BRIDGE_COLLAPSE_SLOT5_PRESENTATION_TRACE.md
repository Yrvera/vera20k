# Bridge Collapse Slot 5 Presentation Trace

Scenario: weapon bridge collapse succeeds on a standard YR bridge cell.

Scope: Trace only the presentation consequences after the weapon path has already passed the bridge-damage gate and invoked the bridge-collapse/destruction path. This covers `CellClass::BlowUpBridge` debris probability, RNG draw ranges/order, `[General] BridgeExplosions=` and `MetallicDebris=` selection, `Report=`/`StartSound` sound emission, render-state transition, and whether `BridgeVoxelMax=` participates in standard YR. This trace does not cover C4/CABHUT collapse, bridge repair, occupant fallout except where it orders presentation work, or the weapon damage gate itself.

## Verdict

Status: COMPLETE.

Tally: PASS: 1 | FAIL: 5 | UNCHECKED: 2 | NOT-IMPLEMENTED: 1

End-to-end parity is not complete for bridge-collapse presentation. The bridge can collapse and render-state mutation is wired into Rust's post-tick bridge renderer, but the visual/audio presentation after `BlowUpBridge` is not numerically equivalent to `gamemd.exe`.

The high-impact failures are:

1. Rust uses small-range RNG calls for debris probability/jitter/metallic gating where `gamemd.exe` uses `RandomRanged(0, 0x7FFFFFFE)`.
2. Rust places debris/explosion effects at exact cell center; `gamemd.exe` consumes two normalized jitter draws and uses them for visible sub-cell offsets.
3. Rust gates metallic debris on `BridgeVoxelMax`; standard YR `BlowUpBridge` does not read `BridgeVoxelMax`.
4. Rust has no delayed animation `Report=`/`StartSound` sound emission for selected bridge explosion anims.
5. Rust's current debris tests mirror Rust's own wrong small-range calls, so they are not binary-parity proof.

## Pipeline

gamemd path after a successful weapon collapse:

`Apply_area_damage @ 0x00489280` -> bridge destruction dispatch -> per affected bridge cell `CellClass::BlowUpBridge @ 0x0047DD70` -> ground/deck fallout -> collapsed-cell queue -> optional debris/explosion animation block -> selected `BridgeExplosions` anim starts after delay -> `AnimClass::Middle @ 0x00424CE0` plays the anim's resolved `StartSound`/fallback `Report`.

Rust path after a successful weapon collapse:

`Simulation::tick` applies collected bridge damage events -> `bridge_orchestrator::apply_bridge_damage_events` -> `run_dispatch_loop` -> aggregate destroyed cells -> `drop_in_bridge_deck_entities` -> `spawn_bridge_debris(sim, rules, &destroyed_set)` -> push visual-only `WorldEffect`s -> app bridge builders read post-tick `BridgeRuntimeState::effective_render_state`.

## Stage Results

### Stage 1 - Standard YR presentation data

gamemd: Standard YR has `BridgeExplosions=TWLT026,TWLT036,TWLT050,TWLT070`, `MetallicDebris=DBRIS1LG..DBRS10SM` with 20 entries, and `BridgeVoxelMax=3` in `rulesmd.ini`. The `BlowUpBridge` and animation-sound reports both mark the path active in standard YR.

Rust: `BridgeRules::from_ini` parses `[General] BridgeExplosions=` into `bridge_rules.explosions`; `GeneralRules::from_ini` parses `[General] MetallicDebris=`, and init interns both lists into `Simulation.bridge_explosions` and `Simulation.metallic_debris`.

Verdict: PASS for retail data identity and active-path status.

Evidence:
- gamemd/docs: `BRIDGE_RNG_CALL_ORDER_CLASSIFICATION_GHIDRA_REPORT.md:37`, `BRIDGE_COLLAPSE_SOUND_SOURCE_GHIDRA_REPORT.md:8`, `:102..115`
- INI: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:528`, `:529`; `C:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini:15656..15683`
- Rust: `C:/Users/enok/Documents/ra2-rust-game/src/rules/ruleset.rs:758`, `:1091`; `C:/Users/enok/Documents/ra2-rust-game/src/app_init_helpers.rs:375`

### Stage 2 - Debris block gate

gamemd: `BlowUpBridge` gates the entire visual/audio-producing debris block on `BridgeExplosions.ActiveCount > 0`, then runs the 95 percent probability gate. Standard YR count is 4. `MetallicDebris` alone does not enable this block.

Rust: `spawn_bridge_debris` returns only when both `explosion_count == 0 && metallic_count == 0`; with empty `BridgeExplosions` and non-empty `MetallicDebris`, Rust would still consume probability/jitter/metallic RNG and may spawn metallic debris.

Verdict: FAIL. The gate source is not numerically equal for modded data, and the conditional call count can diverge.

Evidence:
- gamemd/docs: `BRIDGE_RNG_CALL_ORDER_CLASSIFICATION_GHIDRA_REPORT.md:72`, `:81`, `:137`
- Rust: `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:1052`, `:1056`

### Stage 3 - Outer probability and jitter RNG

gamemd: After the active-count gate, `BlowUpBridge` calls `RandomRanged(0, 0x7FFFFFFE)` for the 95 percent outer gate. If it passes, it calls the same `RandomRanged(0, 0x7FFFFFFE)` range twice more for X/Y jitter.

Rust: `spawn_bridge_debris` uses `next_range_u32(20) == 0` for the outer gate and `next_range_u32(0xFFFF)` twice for jitter.

Verdict: FAIL. Literal range/order equality fails: gamemd uses three normalized `0..=0x7FFFFFFE` draws; Rust uses one `0..20` exclusive draw and two `0..0xFFFF` exclusive draws.

Evidence:
- gamemd/docs: `BRIDGE_RNG_CALL_ORDER_CLASSIFICATION_GHIDRA_REPORT.md:73`, `:74`, `:82`, `:83`, `:138`, `:139`
- Rust: `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:1062`, `:1068`, `:1069`

### Stage 4 - Effect position from jitter

gamemd: The two normalized jitter draws are applied to the centered cell coordinate before optional metallic debris and delayed bridge explosion creation.

Rust: The two jitter draws are discarded; both metallic debris and bridge explosion `WorldEffect`s use `CELL_CENTER_LEPTON` for `sub_x` and `sub_y`.

Verdict: FAIL. The player sees centered effects in Rust where gamemd can offset them within the cell.

Evidence:
- gamemd/docs: `BRIDGE_COLLAPSE_SOUND_SOURCE_GHIDRA_REPORT.md:42..49`, `BRIDGE_RNG_CALL_ORDER_CLASSIFICATION_GHIDRA_REPORT.md:74`
- Rust: `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:1068`, `:1091`, `:1092`, `:1129`, `:1130`

### Stage 5 - MetallicDebris gate and `BridgeVoxelMax`

gamemd: `BlowUpBridge` runs a normalized 50 percent gate using `RandomRanged(0, 0x7FFFFFFE)`, then selects from `MetallicDebris.ActiveCount` if the branch continues. Standard YR `BlowUpBridge` does not read `BridgeVoxelMax`; the prior report classifies `BridgeVoxelMax` as parsed but not live for standard YR debris behavior.

Rust: `spawn_bridge_debris` uses `next_range_u32(2) == 0` for the 50 percent gate and then requires `voxel_max > 0` before selecting a metallic debris slot.

Verdict: FAIL. The 50 percent RNG range is wrong, and `BridgeVoxelMax` incorrectly participates in Rust's standard-YR debris path.

Evidence:
- gamemd/docs: `BRIDGE_RNG_CALL_ORDER_CLASSIFICATION_GHIDRA_REPORT.md:75`, `:76`, `:84`, `:94`, `:128`, `:140`, `:150`
- Rust: `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:1054`, `:1079`, `:1083`, `:1084`; stale parity test at `:1598`

### Stage 6 - BridgeExplosions delay and slot

gamemd: If the outer gate passed, `BlowUpBridge` always attempts one `BridgeExplosions` anim for the cell. It draws `RandomRanged(1, 5)` inclusive for delay, then `RandomRanged(0, BridgeExplosions.ActiveCount - 1)` for the anim slot.

Rust: If `explosion_count > 0`, Rust draws `next_range_u32_inclusive(1, 5)`, then `next_range_u32(explosion_count)`, and stores `delay_ms = delay_frames * 67`.

Verdict: UNCHECKED. The isolated range shape matches, but this trace did not compute a shared-seed selected TWLT result after correcting earlier RNG draws, nor prove Rust's `67 ms` delay equals gamemd's frame countdown at the observable animation-start boundary.

Evidence:
- gamemd/docs: `BRIDGE_RNG_CALL_ORDER_CLASSIFICATION_GHIDRA_REPORT.md:77`, `:85`, `:86`, `:107`
- Rust: `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs:1121`, `:1122`, `:1123`, `:1137`; tick behavior at `C:/Users/enok/Documents/ra2-rust-game/src/sim/components.rs:592`

### Stage 7 - TWLT report sound

gamemd: The selected `BridgeExplosions` anim plays its resolved `StartSound` or fallback `Report` when the animation starts. Standard YR maps `TWLT026/TWLT036/TWLT050/TWLT070` to `ExplosionShard/Explosion06/Explosion07/Explosion09`. Because the bridge explosion delay is `1..5`, the sound is delayed with the visual.

Rust: `WorldEffect` stores no sound id, and `SimSoundEvent` has no bridge-collapse anim-start/report event. The only bridge-specific sound event is `BridgeRepaired`, which is repair-only.

Verdict: NOT-IMPLEMENTED.

Evidence:
- gamemd/docs: `BRIDGE_COLLAPSE_SOUND_SOURCE_GHIDRA_REPORT.md:19`, `:21`, `:88..96`, `:108..115`, `:170..172`
- Rust: `C:/Users/enok/Documents/ra2-rust-game/src/sim/components.rs:563`, `:585`; `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/mod.rs:96`, `:175..181`

### Stage 8 - Render-state transition timing

gamemd: The bridge cell mutation is visible through the live bridge overlay/body render path on the next tactical draw; high bridge body rendering is event-driven by cell state, with per-frame render reading `cell+0x44`, `cell+0x11E`, and bridge flags.

Rust: Bridge damage is applied during `Simulation::tick`, then bridge body/shadow/railing builders read post-tick `BridgeRuntimeState::effective_render_state`. If a cell's effective render state is `None`, body/railing rendering skips it.

Verdict: UNCHECKED. The Rust read surface is plausibly aligned with post-tick state, but this trace did not compute a frame-by-frame equality table for "collapse tick T -> first visible destroyed frame" against gamemd.

Evidence:
- gamemd/docs: `BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md:83`, `:87`, `:155..162`; `BRIDGE_RENDERING_GHIDRA_REPORT.md:15..18`
- Rust: `C:/Users/enok/Documents/ra2-rust-game/src/sim/bridge_state/mod.rs:919`; `C:/Users/enok/Documents/ra2-rust-game/src/app_instances/bridges.rs:120`, `:236`, `:320`

## Adjacent Findings

- C4/CABHUT bridge collapse has separate walker-spawned presentation work before destruction calls. That was intentionally not traced here because this slot's concrete scenario is weapon bridge collapse succeeds.
- Occupant kill, deck DropIn, collapsed-cell queueing, zone rebuild, radar/full redraw, and trigger event `0x1F` are adjacent to presentation ordering but not scored here except as the pre-debris ordering point in `BlowUpBridge`.
- Allocation failure behavior remains untested in Rust and is not needed for normal standard-YR parity assertions in this slot.

## Implementation Handoff

1. Replace debris outer/jitter/metallic probability calls with `RandomRanged(0, 0x7FFFFFFE)`-equivalent inclusive draws in the exact gamemd order.
2. Gate the whole `BlowUpBridge` debris block on `BridgeExplosions` count, not on "either list is non-empty".
3. Remove `BridgeVoxelMax` from standard YR `BlowUpBridge` debris gating.
4. Preserve and apply the two jitter draws to `WorldEffect.sub_x/sub_y` instead of discarding them.
5. Add delayed animation-start sound routing from the selected anim's resolved `StartSound`/fallback `Report`; do not use `BridgeRepaired` or a single hardcoded collapse sound.
6. Add a post-fix frame trace/test for `delay 1..5` to prove Rust's `WorldEffect` countdown starts visuals and sounds on the same frame as gamemd.

## Sources

- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_RNG_CALL_ORDER_CLASSIFICATION_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_COLLAPSE_SOUND_SOURCE_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_COLLAPSE_FALLOUT_ORDERING_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_RENDERING_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`
- `C:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini`
- `C:/Users/enok/Documents/ra2-rust-game/ini/soundmd.ini`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/bridge_orchestrator.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/components.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/mod.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/bridge_state/mod.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app_instances/bridges.rs`

