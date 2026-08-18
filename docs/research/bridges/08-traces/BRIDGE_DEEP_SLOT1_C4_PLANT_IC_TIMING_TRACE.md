# Bridge Deep Slot 1 - C4 Plant IC Timing Trace

## Scope

Scenario traced: a SEAL/Tanya player issues C4 on a CABHUT while the hut is not Iron-Curtained. After the command is underway, but before the infantry reaches the hut footprint and attempts to arm C4, the target CABHUT becomes Iron-Curtained/invulnerable.

This report traces only that timing gate. It does not trace the hut-centered bridge walker, repair, debris/audio, or normal non-hut C4 behavior.

## Verdict

Rust is not on gamemd parity for this scenario.

The player-visible mismatch is that gamemd re-checks the target building's Iron Curtain state at the actual C4 attach moment and refuses to arm the marker when the hut has become invulnerable. Rust checks invulnerability at hover/order/command validation, but once `c4_plant` is set on the infantry it does not re-check invulnerability at the plant-claim point. As a result, Rust can arm C4 on a CABHUT that became invulnerable during the walk-up, emit the plant animation/sound, and later collapse the bridge. gamemd does not attach the marker, so no C4Delay timer and no later bridge collapse are produced from that order.

Verdict tally: PASS: 1 | FAIL: 2 | UNCHECKED: 4 | NOT-IMPLEMENTED: 0

## Pipeline

1. INI/data: `[GHOST] C4=yes`, `[TANY] C4=yes`, `[CABHUT] Immune=yes`, `LegalTarget=yes`, `BridgeRepairHut=yes`; `CanC4` defaults true for buildings. PASS for the scenario prerequisites.
2. Cursor/action at command issue: gamemd returns C4 action `0x10` for uncurtained CABHUT; Rust returns `CursorFeedbackKind::Demolish` when the same target is not invulnerable. UNCHECKED for literal numeric equality because Rust does not expose the gamemd action code at this boundary.
3. Command acceptance and movement: gamemd assigns Mission_Sabotage (`0x11`) and walks to the hut; Rust accepts `Command::PlantC4` and sets `C4PlantState` if the target is still vulnerable when the command validates. UNCHECKED for exact tick equality because the app command-envelope execution tick was not dynamically sampled.
4. Mid-walk invulnerability: gamemd `IsIronCurtainActive()` and Rust `is_invulnerable()` both use active-while-elapsed-is-less-than-duration timer semantics. UNCHECKED for this concrete run because no live seed/frame sample was executed.
5. Arrival/plant claim: gamemd checks `target_building.IsIronCurtainActive() == 0` before setting the C4 marker; Rust sets `pending_c4_detonation` without re-checking invulnerability. FAIL.
6. SEAL/Tanya animation and sound at the failed claim: Rust switches to Attack sequence and emits `C4Planted` after setting the marker. gamemd's exact failed-IC branch animation/order outcome was not fully computed beyond the fact that the attach block is skipped. UNCHECKED.
7. Later C4Delay/collapse: gamemd has no marker, so no C4Delay expiry and no bridge collapse from this order. Rust has a marker, so after 27 ticks it reaches the CABHUT branch and dispatches bridge collapse; the CABHUT branch bypasses the detonation-time IC damage gate. FAIL.

## Evidence

### gamemd active standard YR evidence

- `docs/research/C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION.md:189` verifies `InfantryClass::What_Action_OnObject` returns action `0x10` for SEAL/Tanya on CABHUT through the C4 block.
- `docs/research/C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION.md:234` verifies the Mission_Sabotage on-arrival branch.
- `docs/research/C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION.md:245` through `:257` show the on-arrival pre-attach gates: target mission not `0x13`, `target_building.vtable[0x160]() == 0`, then `field_0x6df = 1`.
- `docs/research/C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION.md:264` through `:274` identifies vtable `0x160` as `IsIronCurtainActive()` and explicitly states the C4 attach branch consults Iron Curtain, not `Immune`.
- `docs/research/CABHUT_C4_COLLAPSE_ENTRY_GHIDRA_REPORT.md:39` through `:43` independently verifies the same marker write path is active in standard YR and that the marker is written only if the target is not Iron Curtain active.
- `docs/research/CABHUT_C4_COLLAPSE_ENTRY_GHIDRA_REPORT.md:18` and `:27` through `:30` verify the C4 timer fields and marker are live in standard YR.
- `docs/research/IRONCURTAIN_FORCESHIELD_GHIDRA_REPORT.md:102` through `:132` verify the active timer-based `IsIronCurtainActive()` semantics.

### Rust evidence

- `src/app_cursor.rs:250` through `:260` checks invulnerability for the C4 cursor.
- `src/app_context_order.rs:320` through `:338` checks invulnerability while resolving the C4 order target.
- `src/sim/world/world_commands.rs:970` through `:989` checks invulnerability when validating `Command::PlantC4`.
- `src/sim/world/world_commands.rs:1003` through `:1011` then sets `e.c4_plant = Some(C4PlantState { target_building_id })`.
- `src/sim/world/world_orders.rs:406` through `:454` walks the active `c4_plant` intent to the target footprint but does not re-check target invulnerability.
- `src/sim/world/world_orders.rs:456` through `:462` unconditionally sets `pending_c4_detonation` once the attacker cell is in the target footprint.
- `src/sim/world/world_orders.rs:464` through `:480` switches the attacker animation to Attack and emits `C4Planted`.
- `src/sim/world/world_orders.rs:512` through `:542` applies C4 after `rules.c4_delay_ticks`; retail/default Rust parsing gives 27 ticks for `C4Delay=.03`.
- `src/sim/world/world_orders.rs:720` through `:753` handles CABHUT before the normal target invulnerability check, dispatching bridge collapse and consuming the marker.
- `src/sim/world/world_orders.rs:755` through `:766` shows the detonation-time invulnerability gate exists only after the CABHUT early return.
- `src/sim/world/world_orders_bridge_repair_tests.rs:767` through `:805` currently codifies the post-marker CABHUT behavior: an invulnerable CABHUT with a pending marker still dispatches bridge collapse. That is correct only after a marker exists; it does not cover the missing plant-time re-check.

## Player-Visible Failures

1. Stage 5 FAIL: Rust arms C4 on a CABHUT that became Iron-Curtained before arrival; gamemd skips the marker write. Rust: `src/sim/world/world_orders.rs:456`; gamemd evidence: `docs/research/C4_ON_BRIDGE_REPAIR_HUT_GATE_INVESTIGATION.md:245`.
2. Stage 7 FAIL: Rust later collapses the bridge after `C4Delay=27` ticks from that wrongly armed marker; gamemd has no marker, so no timer expiry and no bridge collapse from this order. Rust: `src/sim/world/world_orders.rs:536`; gamemd evidence: `docs/research/CABHUT_C4_COLLAPSE_ENTRY_GHIDRA_REPORT.md:39`.

## Adjacent Findings

- If the CABHUT is already Iron-Curtained at issue time, Rust rejects the cursor/order through app and command validation. That case is outside this scenario and was not traced here.
- If a CABHUT already has a pending C4 marker, gamemd and Rust both prevent a second marker from being armed; that case is outside this scenario.
- The detonation-time CABHUT branch intentionally bypasses normal building damage and Iron Curtain checks once a marker already exists. That does not justify arming the marker when the hut is Iron-Curtained at arrival.
- Exact failed-IC SEAL mission/animation fallout in gamemd needs a separate narrow trace if visual idling/retasking after the rejected attach is important.

## Status

COMPLETE
