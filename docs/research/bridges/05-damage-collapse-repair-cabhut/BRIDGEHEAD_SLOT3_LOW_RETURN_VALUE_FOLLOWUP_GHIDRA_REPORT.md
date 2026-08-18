# Bridgehead Slot +3 Low Return-Value Follow-up - Ghidra Report

Date: 2026-05-23
Investigation mode: exhaustive-slice

## Scope

This report follows up `BRIDGEHEAD_DIRECT_DAMAGE_SLOT3_COLLAPSE_GHIDRA_REPORT.md` on one question: does the low bridgehead slot `+3` return value matter outside `ProcessBridgeDamageStateMachine_Low`?

In scope:

- Callers of `ApplyDamageToCell @ 0x00587180`
- Callers of `ProcessBridgeDamageStateMachine_Low @ 0x00571490`
- Callers of `ProcessBridgeDamageStateMachine_High @ 0x00576BA0`
- Retry/stop-targeting behavior controlled by the boolean return

Out of scope:

- Re-verifying the slot `+3` collapse side effects themselves
- Identifying every campaign trigger action label by user-facing INI name
- Runtime map frequency for low bridgehead slot `+3`

## Open Questions Log

Resolved:

- Who calls `ProcessBridgeDamageStateMachine_Low`? Only `ApplyDamageToCell @ 0x00587180`.
- Who calls `ProcessBridgeDamageStateMachine_High`? Only `ApplyDamageToCell @ 0x00587180`.
- Who consumes `ApplyDamageToCell`'s boolean? Five direct callers do.
- Does low slot `+3` returning `0` affect control flow? Yes. It controls retry loops and `TechnoClass__StopAllTargeting` calls.

Deferred:

- Exact user-facing trigger action names for `FUN_006e0490` and `FUN_006e2050`. They are called from `TriggerAction__Execute`, but the label-to-action decode was not needed to answer the return-value question.

## Binary Evidence

### 1. Direct callers

Evidence: Ghidra caller/xref queries. Confidence: high.

`ProcessBridgeDamageStateMachine_Low @ 0x00571490` has one direct caller:

- `ApplyDamageToCell @ 0x00587180`

`ProcessBridgeDamageStateMachine_High @ 0x00576BA0` has one direct caller:

- `ApplyDamageToCell @ 0x00587180`

`ApplyDamageToCell @ 0x00587180` has five direct callers:

- `Apply_area_damage @ 0x00489280`
- `FUN_006e0490 @ 0x006e0490`
- `FUN_006e2050 @ 0x006e2050`
- `MapClass__DestroyBridge_High_OnHutDeath @ 0x00574000`
- `MapClass__DestroyBridge_Low_OnHutDeath @ 0x00574c20`

### 2. Area damage caller uses the boolean for retries and stop-targeting

Evidence: `Apply_area_damage @ 0x00489280`. Confidence: high. Active in YR: yes, conditional on `DestroyableBridges`, `Wall=yes`, bridge path identity, and the BridgeStrength/IonCannon gate.

High state-machine block:

- Calls `ApplyDamageToCell`.
- If the return is false and the warhead is the IonCannon bridge-damage special identity (`Rules+0xFF0`), retries up to three more times.
- If any call returns true, calls `TechnoClass__StopAllTargeting`.
- Dirties a tactical screen rectangle whether or not the call returned true after the block was entered.

Low state-machine block:

- Has the same structure.
- Calls `ApplyDamageToCell`.
- If false and IonCannon special, retries up to three more times.
- If true, calls `TechnoClass__StopAllTargeting`.
- Dirties a low-bridge-sized tactical screen rectangle whether or not the return became true.

Therefore, low bridgehead slot `+3` returning false after collapse side effects is observable in control flow:

- non-Ion path: no retry, no `TechnoClass__StopAllTargeting` from this return
- IonCannon path: up to four total `ApplyDamageToCell` calls, each allowed to repeat side effects if the low branch keeps returning false

Relevant existing research aligns with this:

- `WEAPON_AOE_BRIDGE_DAMAGE_ENTRY_GHIDRA_REPORT.md` records `Rules+0xFF0` as `[CombatDamage] IonCannonWarhead=`, and states the state-machine paths get first attempt plus up to three retries when previous attempts return false.

### 3. Trigger-action callers retry on false

Evidence: `FUN_006e0490 @ 0x006e0490`, `FUN_006e2050 @ 0x006e2050`, and caller/xref queries showing both are called from `TriggerAction__Execute @ 0x006dd8b0`. Confidence: high for retry behavior, medium for user-facing action identity.

`FUN_006e0490`:

- Does several area-damage calls.
- If a coordinate predicate passes, calls `ApplyDamageToCell`.
- Sets a counter to `3`.
- While the return is false and the counter is still positive, calls `ApplyDamageToCell` again.
- Dirties a `0x100` by `0x100` tactical rectangle after the retry loop.
- Returns `1`.

`FUN_006e2050`:

- Applies damage to matching technos first.
- If the target cell differs from a global sentinel coordinate, calls `ApplyDamageToCell`.
- Sets a counter to `3`.
- While the return is false and the counter is still positive, calls `ApplyDamageToCell` again.
- Dirties a `0x100` by `0x100` tactical rectangle after the retry loop.
- Returns whether any techno was damaged earlier in the routine.

Both functions are reached from `TriggerAction__Execute`, so the false return on low slot `+3` can cause repeated bridge damage calls in trigger-driven bridge damage actions.

### 4. Hut-death fallback walkers retry on false

Evidence: `MapClass__DestroyBridge_High_OnHutDeath @ 0x00574000`, `MapClass__DestroyBridge_Low_OnHutDeath @ 0x00574c20`. Confidence: high. Active in YR: yes, bridge repair hut death/detonation path.

Both hut-death functions have the same fallback walker shape:

- Locate a ramp/endpoint candidate.
- Call `ApplyDamageToCell`.
- If the return is false, retry until the loop counter reaches `3`.
- If the return is true, break the retry loop early.
- Continue to adjacent bridge update, tactical dirty flag, and bridge zone update tail.

This means low bridgehead slot `+3` returning false can make hut-death fallback call `ApplyDamageToCell` up to three times on the same target even though the first low slot `+3` call already ran collapse side effects.

## Interpretation

The low slot `+3` return-value nuance is load-bearing. It does not decide whether the low bridgehead visually/pathingly collapses; the collapse side effects have already happened inside `ProcessBridgeDamageStateMachine_Low`. It does decide whether callers treat that call as successful for retry and targeting-cleanup purposes.

High slot `+3`:

- side effects collapse
- returns true
- callers stop retrying
- area damage calls `TechnoClass__StopAllTargeting`

Low slot `+3`:

- side effects collapse
- returns false
- callers may retry
- area damage does not call `TechnoClass__StopAllTargeting` from that return

## Current Rust Implication

Current Rust has a single `StateOutcome::Collapsed` path that drives collapse cascade, destroyed-cell aggregation, `BlowUpBridge` fallout, adjacent refresh, trigger notification, and zone refresh.

If low slot `+3` is implemented as a normal `StateOutcome::Collapsed` with no extra distinction, Rust will likely produce the correct visible collapse once, but it will not model the binary's retry/boolean behavior:

- IonCannon/trigger/hut paths may stop after one low slot `+3` collapse where gamemd retries.
- Any Rust equivalent of `StopAllTargeting` would incorrectly run if it keys off collapse for low bridgehead slot `+3`.

If low slot `+3` is implemented as `Absorbed`, Rust will preserve the false-return retry behavior but lose collapse cascade side effects unless a second channel carries those side effects.

Therefore the clean Rust model needs to separate two concepts:

1. side effects that occurred, including bridge collapse fallout
2. the `ApplyDamageToCell` boolean-success value used by caller retry/cleanup logic

## Implementation Handoff

Recommended design direction:

- Do not overload `StateOutcome::Collapsed` to mean both "collapse side effects happened" and "gamemd returned true".
- Add or refactor toward an outcome shape that can carry collapse side effects while also carrying the binary success boolean.
- High slot `+3` should be collapse side effects plus success `true`.
- Low slot `+3` should be collapse side effects plus success `false`.
- Existing callers that currently break on `Collapsed` vs `Absorbed` need to break/retry based on the success boolean, while the cascade aggregator still processes collapse side effects.

Acceptance scenarios:

- High bridgehead slot `+3`, non-Ion area damage: one call, collapse side effects, stop-targeting path eligible.
- Low bridgehead slot `+3`, non-Ion area damage: one call, collapse side effects, no stop-targeting from the return.
- High bridgehead slot `+3`, IonCannon area damage: one call, collapse side effects, no retry.
- Low bridgehead slot `+3`, IonCannon area damage: repeated calls up to the caller retry bound unless a later call returns true through another path.
- Hut-death fallback on low bridgehead slot `+3`: repeated `ApplyDamageToCell` calls up to the hut retry bound despite collapse side effects.
- Trigger-action bridge damage on low bridgehead slot `+3`: repeated calls up to the trigger helper retry bound despite collapse side effects.

## Bottom Line

Yes, something can need the low boolean difference. The return value controls retry and targeting-cleanup behavior in all direct `ApplyDamageToCell` caller families checked here. The implementation should carry collapse side effects and binary-success return separately instead of treating `StateOutcome::Collapsed` as equivalent to "ApplyDamageToCell returned true."
