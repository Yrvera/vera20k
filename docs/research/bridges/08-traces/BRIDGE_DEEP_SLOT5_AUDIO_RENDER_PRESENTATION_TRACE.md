# Bridge Deep Slot 5 Audio/Render Presentation Trace

Date: 2026-05-22

Scope: concrete player-visible presentation for two related events only:

1. C4-on-CABHUT collapse succeeds and bridge cells are destroyed.
2. Engineer bridge repair succeeds.

Focus: TWLT/debris sounds, repair SFX/EVA, event order relative to mutation, destroyed-bridge water/TMP reveal, minimap/radar updates, and INI-derived sound IDs.

Non-goals: bridge damage gates, C4 eligibility scoring, engineer targeting/pathing, and unverified follow-up mechanics.

## Verdict Summary

PASS: 2 | FAIL: 4 | UNCHECKED: 3 | NOT-IMPLEMENTED: 3

Overall status: PARTIAL. The trace found concrete presentation mismatches, but exact first-frame render parity and exact audio mixer/sample-onset parity were not computed, so those stages remain UNCHECKED instead of PASS.

## Evidence Used

- `docs/research/BRIDGE_COLLAPSE_SOUND_SOURCE_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_RNG_CALL_ORDER_CLASSIFICATION_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_COLLAPSE_FALLOUT_ORDERING_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_RENDERING_GHIDRA_REPORT.md`
- `docs/research/RADAR_EVENT_CLASS_GHIDRA_REPORT.md`
- `docs/research/ANIMATION_SOUNDS_GHIDRA_REPORT.md`
- `docs/research/traces/BRIDGE_REPAIR_SOUND_EVA_ORDERING_TRACE.md`
- `ini/rulesmd.ini`
- `ini/artmd.ini`
- `ini/soundmd.ini`
- `ini/evamd.ini`
- Current Rust source under `src/`

## Active Standard YR Confirmation

The referenced gamemd paths are active in standard YR for this scenario:

- C4-on-CABHUT destruction reaches the BridgeRepairHut C4 expiry branch and dispatches bridge destruction from the hut-centered scan.
- Bridge destruction reaches active `CollapseBridge_*` / `CellClass::BlowUpBridge` paths, not dormant TS-only terrain logic.
- Standard YR has non-empty `[General] BridgeExplosions=TWLT026,TWLT036,TWLT050,TWLT070`.
- Standard YR has `[AudioVisual] RepairBridgeSound= BridgeRepaired`.
- Standard YR has `EVA_BridgeRepaired` in `evamd.ini`.
- Standard YR radar event type 14 is `BridgeRepaired`.

## Stage Results

| Stage | Boundary | gamemd behavior | Rust behavior | Verdict |
|---|---|---|---|---|
| 1 | Stock symbolic sound IDs | `BridgeExplosions=TWLT026,TWLT036,TWLT050,TWLT070`, `RepairBridgeSound=BridgeRepaired`, `EVA_BridgeRepaired` are standard YR data. | Rules/app paths read the same symbolic `BridgeExplosions` and `RepairBridgeSound`; repair app path resolves `EVA_BridgeRepaired`. | PASS |
| 2 | Collapse debris/explosion RNG gates | `BlowUpBridge` debris block is gated by `BridgeExplosions.ActiveCount`; 95 percent gate and jitter use normalized `RandomRanged(0,0x7FFFFFFE)` draws; metallic 50 percent gate also uses normalized draw. | `spawn_bridge_debris` returns only when both explosion and metallic lists are empty, uses `next_range_u32(20)`, two `next_range_u32(0xFFFF)` draws, and `next_range_u32(2)`. | FAIL |
| 3 | Collapse visual sub-cell jitter | gamemd uses the two normalized jitter draws to offset debris/explosion presentation within the cell. | Rust discards its jitter draws and places effects at `CELL_CENTER_LEPTON`. | FAIL |
| 4 | Collapse TWLT report/start sounds | TWLT anims play sound through `AnimTypeClass` `StartSound`, falling back to `Report`; standard YR TWLT reports resolve to `ExplosionShard`, `Explosion06`, `Explosion07`, and `Explosion09`. | `WorldEffect` carries only visual fields and no sound/report ID, so bridge-collapse TWLT sounds are not emitted. | NOT-IMPLEMENTED |
| 5 | C4 CollapseBridge walker presentation | C4 hut destruction uses the bridge walker path; before destroying cells it spawns up to three perpendicular `BridgeExplosions` anims per step, then mutates bridge cells. | Rust has aggregate `spawn_bridge_debris` presentation around the destroyed set, not the walker-specific pre-destroy three-cell animation pass. | NOT-IMPLEMENTED |
| 6 | Destroyed bridge water/TMP reveal | gamemd mutates bridge cell state and marks terrain/tactical redraw; destroyed deck presentation reveals underlying terrain/TMP according to the updated cell state. | Rust render code skips bridge body/rail/shadow when effective bridge render state is destroyed, but exact first visible frame and TMP/water equality were not computed against gamemd. | UNCHECKED |
| 7 | Collapse minimap/radar terrain update | gamemd calls bridge terrain dirty paths and sets the tactical redraw flag after bridge destruction. | Rust minimap terrain and overlay pixels are precomputed/static; no dynamic bridge dirty-cell update path was found for collapse. | FAIL |
| 8 | Repair SFX symbolic ID | gamemd uses `[AudioVisual] RepairBridgeSound=BridgeRepaired`. | Rust reads `BridgeRules.repair_sound` and emits `GameSoundEvent::BridgeRepaired { sound_id }` with that symbolic ID. | PASS |
| 9 | Repair SFX mixer details | gamemd plays `BridgeRepaired` at the hut/building location through the normal sound system; standard INI has `Sounds=urepair`, `Type=global`, `MinVol=55`, `Volume=55`. | Rust plays the resolved SFX with spatial volume from app sound code. Exact sound index, alias handling for `MinVol`, channel, attenuation, and sample-onset equality were not computed. | UNCHECKED |
| 10 | Repair EVA exact voice | gamemd repair branch plays `EVA_BridgeRepaired` for the local human side before the 5x5 repair mutation. | Rust resolves `EVA_BridgeRepaired` by local owner/faction and plays `eva_sound_id` when present. Exact local-human predicate and final faction sample equality were not recomputed in this trace. | UNCHECKED |
| 11 | Repair order: EVA/SFX vs mutation | gamemd order is radar/EVA, then `RepairBridgeSound`, then the 5x5 bridge repair scan/mutation. | Rust queues the sim event before mutation, but actual app playback happens after the sim tick; the app playback branch plays SFX before EVA. | FAIL |
| 12 | Repair radar/minimap event | gamemd creates radar event type 14 `BridgeRepaired` with hardcoded radar-event semantics, and repair updates bridge terrain/radar dirty state. | Rust radar event enum lacks type 14, repair code drops `outcome.radar_cells`, and minimap overlay/terrain pixels are static. | NOT-IMPLEMENTED |

## Top Player-Visible Failures

1. Collapse TWLT sounds are missing. Standard YR bridge collapse produces TWLT report/start sounds (`ExplosionShard`, `Explosion06`, `Explosion07`, `Explosion09`) tied to the spawned animation; Rust `WorldEffect` is visual-only. Rust: `src/sim/world/bridge_orchestrator.rs:1125`, `src/sim/components.rs:570`. Evidence: `BRIDGE_COLLAPSE_SOUND_SOURCE_GHIDRA_REPORT.md`, `ANIMATION_SOUNDS_GHIDRA_REPORT.md`, `artmd.ini`.

2. C4 collapse presentation does not use the gamemd walker animation order. gamemd spawns bridge explosion anims on perpendicular cells before mutating bridge cells; Rust uses an aggregate destroyed-set effect path. Rust: `src/sim/world/bridge_orchestrator.rs:123`, `src/sim/world/bridge_orchestrator.rs:315`. Evidence: `BRIDGE_COLLAPSE_FALLOUT_ORDERING_GHIDRA_REPORT.md`, `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`.

3. Collapse debris/explosion randomization and placement differ. gamemd uses normalized random draws, active-count gating, and jittered coordinates; Rust uses different ranges and centered placement. Rust: `src/sim/world/bridge_orchestrator.rs:1078`, `src/sim/world/bridge_orchestrator.rs:1084`, `src/sim/world/bridge_orchestrator.rs:1095`, `src/sim/world/bridge_orchestrator.rs:1125`. Evidence: `BRIDGE_RNG_CALL_ORDER_CLASSIFICATION_GHIDRA_REPORT.md`.

4. Repair audio order differs at the app-visible boundary. gamemd repair branch orders radar/EVA, then repair SFX, then bridge mutation; Rust app playback occurs after sim mutation and plays SFX before EVA. Rust: `src/sim/world/world_orders.rs:332`, `src/sim/world/world_orders.rs:340`, `src/app_building_anim.rs:607`. Evidence: `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`.

5. Bridge repair/collapse radar and minimap updates are missing or static. gamemd marks bridge terrain/radar dirty and has radar event type 14 `BridgeRepaired`; Rust drops repair radar cells and minimap terrain/overlay pixels are precomputed. Rust: `src/sim/world/world_orders.rs:354`, `src/render/minimap.rs:75`, `src/render/minimap.rs:207`. Evidence: `BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md`, `RADAR_EVENT_CLASS_GHIDRA_REPORT.md`.

## Adjacent Findings

- The older repair sound/EVA trace contained a stale Rust finding that app playback ignored `eva_sound_id`. Current Rust does play the EVA voice event when resolved.
- `soundmd.ini` uses `MinVol=55` for `BridgeRepaired`; Rust sound parsing was observed to parse `MinVolume`. This trace did not verify whether gamemd accepts `MinVol` as an alias or whether both sides effectively fall back to the same value, so this remains UNCHECKED.
- Engineer repair action gating and C4 action gating were intentionally not re-scored here.

## Implementation Handoff

Do not merge collapse, repair, and radar fixes into one data source. The verified gamemd split is:

- Collapse presentation sound comes from the spawned bridge explosion animation's `StartSound`/`Report` sound ID.
- Repair SFX comes from `[AudioVisual] RepairBridgeSound`.
- Repair EVA comes from `EVA_BridgeRepaired`.
- Repair radar presentation uses radar event type 14 and separate dirty terrain/radar update behavior.

For parity work, the first high-value fix is to carry anim report/start sound IDs into bridge collapse `WorldEffect` or an equivalent sim-visible presentation event, then play those sounds when the delayed TWLT animation starts, not when the bridge cell is first selected for collapse.
