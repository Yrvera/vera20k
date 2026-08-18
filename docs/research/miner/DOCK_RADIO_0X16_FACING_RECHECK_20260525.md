# Dock Radio 0x16 Facing Recheck

**Date:** 2026-05-25
**Investigation Mode:** coverage-map, downgraded from exhaustive-slice because no live Ghidra instance was available
**Claimed Scope:** reconcile existing Ghidra-backed reports and current Rust around stock YR harvester/refinery `UnitClass::Receive_Radio(0x16)`, `DriveLocomotionClass::Do_Turn(0x4000)`, and Rust's dock East-facing pivot.
**Non-Scope:** fresh live decompilation, runtime memory/watch trace, and implementation patches.
**Confidence:** High for `UnitClass::Receive_Radio(0x16)` control-flow shape from existing reports; Medium for exact field-level body-facing equivalence because reports disagree in wording and live Ghidra was unavailable.
**Active in YR:** Yes for stock `[CMIN]`/`[HARV] -> [GAREFN]`/`[NAREFN]` refinery docking.

## 1. Overview

This should be examined before a patch, but the actionable finding is not simply "do not face East." Existing Ghidra-backed reports agree that radio `0x16` does not directly write a body-facing byte, call `GetDockCoord`, move the unit, or start unload by itself on the first unsynchronized call. They also agree that the first ordinary path can call the active locomotor vtable slot `+0x4C` with argument `0x4000`.

The unresolved part is ownership/mechanism: whether Rust's explicit miner-FSM `FacingClass` pivot is byte-equivalent to gamemd's active-locomotor `Do_Turn(0x4000)` plus `RateTimer` gate. Current evidence says the player-visible East-facing outcome may be correct, but Rust's ownership and timing are still suspect.

## 2. Key Offsets / Fields

| Offset / value | Owner | Evidence summary | Recheck status |
|---|---|---|---|
| `0x00737430` case `0x16` | `UnitClass::Receive_Radio` | Calls base radio, checks `+0x6AF`, checks `RateTimer::Current(+0x388)`, calls locomotor `+0x4C(0x4000)` if not synchronized, otherwise may send `0x15`. | Verified by prior Ghidra reports. |
| `+0x388` | Unit / Foot facing timer | Existing reports call this primary-facing `RateTimer`; `0x16` reads it and `Do_Turn` updates it. | Verified by prior reports, exact struct naming still source-dependent. |
| `+0x674` | Unit locomotor pointer | `0x16` calls vtable `+0x4C` and `+0x10` through this pointer. | Verified by prior reports. |
| `+0x6AF` | Unit chrono/teleporting gate | If nonzero, the timer-set branch is skipped. During dock drive-in reports say stock CMIN/HARV have it clear. | Verified by prior reports; runtime value still worth tracing. |
| `+0x418` | Contact-entered/radio flag | Later `0x16 -> 0x15` path requires it. | Verified by prior reports. |
| `0x4000` | 16-bit direction / timer target | Numerically corresponds to East in 16-bit facing space, but the binary passes it to locomotor `Do_Turn`/`RateTimer`, not a direct body-facing setter in `UnitClass`. | Mechanism requires live field-level recheck. |

## 3. Reconciled Model

For stock refinery docking:

1. Building-side dock admission sends `0x12`, then `0x18`, then `0x16` when the unit is accepted/already at the accepted cell.
2. Unit radio `0x18` sets the contact-entered flag at `+0x418`.
3. Unit radio `0x16` first invokes base radio handling.
4. If `+0x6AF == 0` and `RateTimer::Current(+0x388) != 0x4000`, the active locomotor receives vtable `+0x4C(0x4000)` and `0x16` returns `1`.
5. If already synchronized, stopped, has a building destination, `+0x418 != 0`, and unit mission is `7`, `0x16` may transmit `0x15` to the building.
6. The active locomotor at stock dock arrival is reported as Drive for HARV and CMIN, so CMIN should use Drive piggyback at this point, not Teleport override ownership.

The important implementation inference: `0x16` is a locomotor-owned turn/timer request. It is not a miner-FSM body-facing assignment, even if the eventual target direction is East.

## 4. Current Rust Status

Current Rust in `src/sim/miner/miner_dock_sequence.rs` models the dock sync as:

| Rust surface | Current behavior | Recheck verdict |
|---|---|---|
| `DOCK_FACING_EAST = 0x40` / `DOCK_FACING_EAST_DIR = 0x4000` | Treats the dock sync target as East. | Plausible player-visible target, but not sufficient proof of exact mechanism. |
| `sync_dock_facing` | Owns a `FacingClass`, advances it in the miner dock FSM, writes `entity.facing`, and sets `facing_target`. | Suspect mechanism drift: gamemd evidence points to active locomotor `Do_Turn` and unit `+0x388` RateTimer ownership. |
| `FaceSync` / `Pivoting` | Splits first sync from unload start and waits for the target-facing window. | Directionally useful, but source/timing should be tied to explicit `0x16` first-call vs later-call semantics. |
| `start_unload_deploy` | Forces final `entity.facing = DOCK_FACING_EAST`. | Potentially too direct unless proven to mirror the final `RateTimer` sample/update order. |

## 5. Evidence Reconciliation

| Source | Finding | How to use it |
|---|---|---|
| `DOCK_ARRIVAL_PIVOT_SEQUENCE_DOC_CONFLICT_AUDIT_GHIDRA_REPORT.md` | `0x16` has no `GetDockCoord`, no `Set_Destination`, no location write, no direct facing-field setter; first unsynced call only invokes locomotor `+0x4C(0x4000)` and returns. | Strong negative evidence against implementing `0x16` inside miner FSM as direct body-facing write. |
| `UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_TIMING_GHIDRA_REPORT.md` | Later/already-synced `0x16` can send `0x15` under idle/destination/contact/mission gates; first unsynced call does not. | Rust should preserve first-vs-later `0x16` behavior. |
| `CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING_GHIDRA_REPORT.md` | Locomotor slot `+0x4C` resolves to `DriveLocomotionClass::Do_Turn @ 0x004B0EF0`, which calls `RateTimer__Set(&param_2)`. | The turn should be owned by active Drive locomotor state. |
| `HARV_VS_CMIN_DUMP_FACING_COMPARISON_GHIDRA_REPORT.md` | Interprets `0x4000` as East and says both HARV and CMIN reach the same harvester-general path. | Supports keeping the East target as a hypothesis, but its wording "SET FACING" is too direct compared with later audits. |
| `DOCK_RADIO_0X16_FACING_CONFLICT_AUDIT_20260525.md` | Earlier local audit concludes the direct East pivot is suspect/unresolved, not definitely false. | Superseded by this recheck only in emphasis: the key issue is ownership and timer mechanism, not necessarily target direction. |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `UnitClass::Receive_Radio(0x16)` first-call branch | verified-from-existing-docs | prior Ghidra reports at `0x007376AD..0x0073770F` | live spot-check when Ghidra is available |
| `UnitClass::Receive_Radio(0x16)` later `0x15` branch | verified-from-existing-docs | prior Ghidra reports at `0x0073771B..0x00737783` | exact first-winner runtime trace |
| `DriveLocomotionClass::Do_Turn(0x4000)` ownership | touched-not-exhausted | prior report names `0x004B0EF0` and `RateTimer__Set` | field-level writes and owner facing consumers need live recheck |
| HARV vs CMIN common path | verified-from-existing-docs | no Teleporter gate in `0x16`; CMIN active Drive at dock arrival in prior reports | runtime active locomotor snapshot would strengthen proof |
| Current Rust `sync_dock_facing` | touched | local source scan | exact replacement design should wait for `Do_Turn` field proof |

## 7. Open Questions - Final State

- `[RESOLVED] OQ1 - Should this be examined before implementation? -> Yes. Existing evidence leaves a real ownership/mechanism gap, and a direct patch based only on "no direct body-facing write" would be unsafe.` (evidence: conflicting prior reports listed above)
- `[RESOLVED] OQ2 - Is "gamemd definitely does not face East" proven? -> No. The binary passes `0x4000`, numerically East, to active locomotor `Do_Turn`; the direct unit handler does not write facing, but the locomotor may own the turn.` (evidence: `HARV_VS_CMIN_DUMP_FACING_COMPARISON_GHIDRA_REPORT.md`, `CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ3 - Is current Rust's miner-FSM-owned East pivot proven equivalent? -> No. It bypasses active-locomotor `Do_Turn` and `+0x388` RateTimer ownership.` (evidence: local scan of `miner_dock_sequence.rs`)
- `[RESOLVED] OQ4 - Does the new locomotor ownership bridge help this later? -> Yes. CMIN can now have active Drive during dock movement, which is the right owner for a future `Do_Turn(0x4000)` bridge.` (evidence: current Rust refactor surface)
- `[DEFERRED] OQ5 - Exactly which fields does `DriveLocomotionClass::Do_Turn @ 0x004B0EF0` write for `0x4000`?` (category: needs-runtime-debugger; reason: no live Ghidra instance available; next-step-if-pursued: decompile `0x004B0EF0`, `RateTimer__Set`, and the `+0x388` consumers)
- `[DEFERRED] OQ6 - What is the exact unit facing byte before first `0x16`, after first `0x16`, when `0x15` is sent, and on first unload frame for stock HARV and CMIN?` (category: needs-runtime-debugger; reason: requires runtime watch/capture; next-step-if-pursued: trace one HARV and one CMIN dock cycle)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| First unsynced `0x16` routes to active locomotor `+0x4C(0x4000)` and returns before later `0x15` cascade. | Existing Ghidra docs for `0x00737430` | Rust approximates with miner-owned `sync_dock_facing`. | `src/sim/miner/miner_dock_sequence.rs`; likely future movement/locomotor turn API | Represent `0x16` as a locomotor-owned turn/timer request, not direct miner FSM body-facing write. | First `0x16` starts turn/timer sync and does not start unload in the same modeled event. | Do not equate `0x16` return `1` with `0x15` sent. |
| `0x4000` may be the East target but is passed through `DriveLocomotionClass::Do_Turn`/RateTimer. | `CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING_GHIDRA_REPORT.md`; `HARV_VS_CMIN_DUMP_FACING_COMPARISON_GHIDRA_REPORT.md` | Rust writes `entity.facing = 0x40` directly at unload start. | `sync_dock_facing`, `start_unload_deploy`, future locomotor facing state | Keep East as a hypothesis/target only after proving the RateTimer consumer; move ownership toward active Drive locomotor. | HARV and CMIN both converge through identical Drive-owned dock sync path. | Do not special-case CMIN. |
| Later/already-synced `0x16` can send `0x15` under idle/destination/contact/mission gates. | `UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_TIMING_GHIDRA_REPORT.md` | Rust phase names approximate this but do not model radio source explicitly. | `FaceSync`, `MissionQueued`, `Pivoting` | Keep first `0x16` and later `0x16 -> 0x15` as distinct events. | Already-synced idle miner with building destination and contact flag can queue unload without requiring `GetDockCoord` equality. | Do not force every unload handoff through pad-cell equality. |

## 9. Recommended Next Step

Do one targeted binary/runtime pass before changing Rust:

1. Decompile `DriveLocomotionClass::Do_Turn @ 0x004B0EF0`.
2. Decompile `RateTimer__Set @ 0x004C9220` and `RateTimer::Current @ 0x004C93D0`.
3. Find all consumers of unit `+0x388` during `Mission_Deploy_Building`.
4. Runtime-watch HARV and CMIN during a dock cycle: active locomotor class, `+0x388`, `+0x6AF`, `+0x418`, facing byte, mission, and radio sequence.

If that confirms `Do_Turn(0x4000)` is the body-facing mechanism, the patch should move Rust's current miner-owned pivot into a locomotor-facing bridge instead of deleting the East turn.

## Sources

- `docs/research/miner/DOCK_ARRIVAL_PIVOT_SEQUENCE_DOC_CONFLICT_AUDIT_GHIDRA_REPORT.md`
- `docs/research/UNITCLASS_RECEIVE_RADIO_0X16_SECOND_CALL_TIMING_GHIDRA_REPORT.md`
- `docs/research/UNITCLASS_PERCELLPROCESS_GETDOCKCOORD_VS_0X16_RECONCILIATION_GHIDRA_REPORT.md`
- `docs/research/miner/CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING_GHIDRA_REPORT.md`
- `docs/research/miner/HARV_VS_CMIN_DUMP_FACING_COMPARISON_GHIDRA_REPORT.md`
- `docs/research/miner/DOCK_RADIO_0X16_FACING_CONFLICT_AUDIT_20260525.md`
- `src/sim/miner/miner_dock_sequence.rs`
