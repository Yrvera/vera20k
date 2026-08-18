# Dock Radio 0x16 Facing Conflict Audit

**Date:** 2026-05-25  
**Investigation Mode:** focused doc-conflict audit  
**Claimed Scope:** current evidence for stock YR harvester/refinery unit radio `0x16`, locomotor vtable slot `+0x4C(0x4000)`, and whether current Rust's explicit East body-facing pivot is proven correct.  
**Non-Scope:** fresh live Ghidra decompilation, runtime memory watch, implementation patches, and full refinery unload FSM audit.  
**Confidence:** High for the direct `UnitClass::Receive_Radio(0x16)` control-flow shape already captured in prior Ghidra reports; Medium/Blocked for the exact body-facing equivalence of `DriveLocomotionClass::Do_Turn(0x4000)` because existing reports disagree and no Ghidra instance was available on 2026-05-25.  
**Active in YR:** Yes for stock `[CMIN]` and `[HARV]` refinery docking.

## 1. Bottom Line

The earlier wording "gamemd does not set body facing there" was too strong.

What is verified: `UnitClass::Receive_Radio(0x16)` does not directly assign a unit body-facing field, call `GetDockCoord`, call `Set_Destination`, or write unit position. On its first ordinary pass it checks unit/locomotor state and calls the active locomotor vtable slot `+0x4C` with argument `0x4000`, then returns. On a later/already-synced pass it can send radio `0x15` to the destination building under stopped/contact/mission gates.

What is not settled by the current local evidence: whether Rust should model that locomotor `+0x4C(0x4000)` call as an explicit immediate `entity.facing = East` update, as a locomotor-owned `RateTimer`/turn-target state that later affects body facing, or as some narrower wait gate. Existing research docs conflict on that point.

Therefore the safe current verdict is: current Rust's direct East-facing pivot is **suspect DRIFT / unresolved mechanism**, not a proven binary-equivalent implementation. Do not patch it solely from the earlier negative claim; first resolve `DriveLocomotionClass::Do_Turn @ 0x004B0EF0` and its consumers.

## 2. Evidence Inventory

| Evidence | What it says | Status for this audit |
|---|---|---|
| `DOCK_ARRIVAL_PIVOT_SEQUENCE_DOC_CONFLICT_AUDIT_GHIDRA_REPORT.md` | `0x16` calls base, reads `+0x6AF`, calls `RateTimer::Current(+0x388)`, conditionally calls locomotor `+0x4C(0x4000)`, and otherwise may send `0x15`. It classifies "`0x16` sets facing East / `0x4000` is a facing" as DRIFT because no body-facing setter appears in the handler. | Strong evidence for no direct facing write in the `0x16` handler. Not enough alone to prove the locomotor call cannot update facing indirectly. |
| `HARV_VS_CMIN_DUMP_FACING_COMPARISON_GHIDRA_REPORT.md` | Labels active locomotor slot `+0x4C` as `ILocomotion::Head_To`; says `0x4000` maps to 8-bit East (`0x40`); says both HARV and CMIN pivot East with no chrono-specific gate. | Conflicts with the doc-conflict audit. Needs fresh verification of the slot implementation and field writes. |
| `CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING_GHIDRA_REPORT.md` | Says slot `+0x4C` resolves to `DriveLocomotionClass::Do_Turn @ 0x004B0EF0`, which calls `RateTimer__Set(&param_2)`. Describes `0x4000` as a facing/timing `RateTimer` target, not speed scalar or link-field write. | Bridges the conflict but remains ambiguous: `RateTimer` may be the exact facing-turn mechanism, but current Rust still bypasses the locomotor-owned state. |
| `DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md` | Says `0x4000` is a `RateTimer` value, not a facing angle, and the handler has no facing operation. Also notes unload checks against a timer-derived value. | Supports "do not write body facing directly from `0x16`"; conflicts with the HARV/CMIN dump-facing report's East-facing interpretation. |
| Current Rust `src/sim/miner/miner_dock_sequence.rs` | Defines `DOCK_FACING_EAST = 0x40`, `DOCK_FACING_EAST_DIR = 0x4000`, uses `FacingClass`, `FaceSync`, `Pivoting`, `dock_pivot_accepts`, and writes `entity.facing = DOCK_FACING_EAST` when the local pivot gate completes. | Mechanism is explicitly an East-facing body pivot, not a raw radio/locomotor `Do_Turn`/`RateTimer` handoff. Exact equivalence is unproven. |

## 3. Reconciled Current Model

For stock refinery docking, the best current model is:

1. The refinery building-side dock path admits the harvester and can send the accepted-cell radio sequence.
2. Unit radio `0x18` sets the contact-entered/arrival byte used by the later `0x16` path.
3. Unit radio `0x16` first calls base radio handling.
4. If the relevant unit byte `+0x6AF` is zero and `RateTimer::Current(+0x388) != 0x4000`, `0x16` calls active locomotor slot `+0x4C(0x4000)` and returns `1`.
5. In stock dock approach, the active locomotor for both HARV and CMIN is reported as Drive.
6. The Drive implementation of slot `+0x4C` is reported as `DriveLocomotionClass::Do_Turn @ 0x004B0EF0`, which calls `RateTimer__Set(&param_2)`.
7. On a later/already-synced `0x16`, if the unit is stopped, has a building destination, has the contact-entered byte, and is in mission 7, it sends radio `0x15` to the destination building.

The unresolved part is step 6's field-level meaning: whether `RateTimer__Set(0x4000)` is literally the body-facing target path, a locomotor turn timer that later drives facing, or just the wait condition consumed by unload/deploy logic.

## 4. Current Rust Risk

Rust currently collapses the verified radio/locomotor sequence into local dock phases:

- `FaceSync`
- `Pivoting`
- `dock_pivot_facing`
- `sync_dock_facing`
- `dock_pivot_accepts`
- final `entity.facing = DOCK_FACING_EAST`

That may still match the broad visual in simple cases if `DriveLocomotionClass::Do_Turn(0x4000)` ultimately turns the body East, but parity requires the same mechanism: same fields, same timing, same reads/writes, same wait condition, and same mission/radio ordering. Current Rust has not proven that.

## 5. Required Proof To Settle It

When Ghidra is available, verify these slices directly:

1. Decompile and disassemble `UnitClass::Receive_Radio @ 0x00737430` case `0x16` again to confirm the exact caller-side branch and fields.
2. Decompile `DriveLocomotionClass::Do_Turn @ 0x004B0EF0`.
3. Identify every field written by `Do_Turn(0x4000)`, especially owner body-facing storage, `PrimaryFacing`, and `RateTimer` fields.
4. Decompile `RateTimer__Set` and `RateTimer__Current` enough to know whether `0x4000` is an angle target, timer target, or both.
5. Recheck `UnitClass::Mission_Deploy_Building @ 0x0073D630` for the exact timer/facing gate before unload.
6. Runtime-watch one stock CMIN and one stock HARV dock: active locomotor class, unit facing byte, `PrimaryFacing`/`+0x388`, `+0x6AF`, `+0x418`, mission, and radio sequence before first `0x16`, after first `0x16`, after later `0x15`, and on first unload tick.

## 6. Updated Handoff

Do not treat "`0x16` does not directly set body facing" as proof that the visual East turn is false. Treat it as proof that Rust's current direct body-facing assignment is not yet mechanism-proven.

Before implementation, resolve the `Do_Turn`/`RateTimer` conflict. If a temporary design is needed, model a locomotor-owned `Do_Turn(0x4000)` state and the later `0x16 -> 0x15` gate, rather than directly writing `entity.facing` from the unit radio handler.

## Sources

- `docs/research/miner/DOCK_ARRIVAL_PIVOT_SEQUENCE_DOC_CONFLICT_AUDIT_GHIDRA_REPORT.md`
- `docs/research/miner/HARV_VS_CMIN_DUMP_FACING_COMPARISON_GHIDRA_REPORT.md`
- `docs/research/miner/CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING_GHIDRA_REPORT.md`
- `docs/research/miner/DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md`
- `src/sim/miner/miner_dock_sequence.rs`

