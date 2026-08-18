# Unload Visual Stale Frame After Refinery Loss - Ghidra Research Report

**Address(es):** `0x0073D630`, `0x0073CEC0`, `0x00737430`, `0x005B35E0`, `0x005B3570`, `0x004593A0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** standard YR `HARV`/`CMIN` unloading display behavior when the stock zero-link state-3 refinery lookup fails because the refinery was sold, destroyed, or otherwise removed before the next unload mission pass. This report only verifies whether the stale `HORV`/`CMON` visual can be statically cleared before presentation.  
**Non-Scope:** cargo drain and credit ownership beyond ordering proof, healthy unload cadence, two-miner handoff, linked nonzero `+0x2E4` dock exit visuals, exact rendered pixel/frame capture, and runtime scheduler instrumentation.  
**Confidence:** High for static write/order evidence and current Rust comparison; Medium for player-visible stale-frame count because a runtime capture is required.  
**Active in YR:** Yes. Stock `[HARV]` and `[CMIN]` have `Harvester=yes` and `UnloadingClass=HORV/CMON`; stock `[GAREFN]`/`[NAREFN]` are live refinery unload targets.

## 1. Overview

The state-3 null-refinery abort path does not clear `UnitClass+0x6D1`, the byte that makes `DrawExtras` temporarily render the miner with its `UnloadingClass`. The branch optionally sends radio `BREAK(3)`, queues Harvest `0x0A` with immediate flag `1`, then exits through the mission timer epilogue.

Static Ghidra evidence found no same-branch or same-call-chain clear before return. Verified clear paths are normal state 4 and radio `0x17`. Therefore Rust's immediate `display_type_override = None` on invalid-refinery abort is not statically proven to match stock; it should be treated as runtime-needing unless a capture proves no stale `HORV`/`CMON` frame is ever presented.

## 2. Key Offsets

| Offset / field | Owner | Meaning | Evidence |
|---|---|---|---|
| `+0x6D1` | `UnitClass` | unload-active display flag; consumed by `DrawExtras` | set in deploy-building path, clear at `0x0073E1F6` and `0x00737AC9` |
| `+0x6B8` | `UnitTypeClass` | `UnloadingClass` type pointer | `UnitClass::DrawExtras @ 0x0073CEC0` |
| `+0xE0E` | `UnitTypeClass` | `Harvester=yes` gate for draw swap | `0x0073CEC0`, INI |
| `+0xBC` | mission/unit | deploy-building substate; `3` unload loop, `4` release handoff | `0x0073D630` |
| `+0xB4` / `+0xB8` | mission | queued mission id / queued flag | `MissionClass::Queue_Mission @ 0x005B35E0` |
| `+0x2E4` | unit/building | reciprocal linked dock pointer; separate sell/destroy cleanup path | `BuildingClass::UndockUnit @ 0x004593A0` |

## 3. Core Logic

### 3.1 Null-refinery abort branch

In `UnitClass::Mission_Deploy_Building @ 0x0073D630`, state 3 recomputes the refinery cell and calls `Look_up_building_in_cell`.

Assembly context:

- `0x0073E306`: calls `Look_up_building_in_cell`.
- `0x0073E30D`: compares returned building pointer with null.
- `0x0073E30F`: non-null branches to the storage/credit path at `0x0073E355`.
- `0x0073E313`: null path checks `PathType::Has_Valid_Steps`.
- `0x0073E31E..0x0073E322`: if valid steps exist, sends radio `3`.
- `0x0073E32A..0x0073E330`: queues mission `0x0A` with immediate flag `1`.
- `0x0073E338+`: enters mission timer epilogue.

No `MOV byte ptr [ESI+0x6D1],0` appears in this branch. The branch exits before the storage/credit path and before the normal state-4 clear.

### 3.2 Verified clear paths

Normal state 4 clears the unloading display byte:

- `0x0073E1F6`: `MOV byte ptr [ESI + 0x6D1], 0`.
- This follows the state-4 wait guard that checks the adjacent refinery and slot-8 `ProductionAnim` pointer.

Radio `0x17` also clears the byte:

- `UnitClass::Receive_Radio @ 0x00737430`, case `0x17`.
- Gate: current type is harvester or weeder and `unit+0x6D1 != 0`.
- `0x00737AC9`: `MOV byte ptr [ESI + 0x6D1], 0`.
- Then it queues Harvest `0x0A` and may commence.

Radio `3` does not clear it:

- `UnitClass::Receive_Radio @ 0x00737430`, case `3`, only optionally queues Guard/Stop when current mission is `0x0C`, calls `FootClass::Receive_Radio`, and returns `1`.
- No `+0x6D1` write exists in the case-3 decompile.

### 3.3 Queue/commence do not clear the display flag

`MissionClass::Queue_Mission @ 0x005B35E0` writes the queued mission fields and may call `Commence`; it does not write `+0x6D1`.

`MissionClass::Commence @ 0x005B3570` moves `+0xB4` to current mission `+0xAC`, resets `+0xBC` to `0`, writes mission timer fields, and clears the queued flag. It does not write `+0x6D1`.

This matters because the null-refinery abort's immediate Harvest queue/commence path is not itself a draw-override clear.

### 3.4 Draw path keeps using `UnloadingClass` while `+0x6D1` remains set

`UnitClass::DrawExtras @ 0x0073CEC0` gates the temporary type swap on:

1. current UnitType `Harvester=yes` at `+0xE0E`;
2. `unit+0x6D1 != 0`;
3. current UnitType has non-null `UnloadingClass` pointer at `+0x6B8`.

When all pass, it writes the current type pointer to the `UnloadingClass` type before the body draw and restores afterward. There is no mission/substate guard in this draw gate. If render occurs after the null-refinery branch and before some later clear path, the miner can still draw as `HORV`/`CMON`.

## 4. Current Rust Implementation Status

Current Rust source scan, no edits:

- `src/sim/miner/miner_dock_sequence.rs:471` defines `abort_invalid_refinery`.
- `src/sim/miner/miner_dock_sequence.rs:478..484` immediately clears `entity.display_type_override`, facing, movement target, drive track, and forced drive track.
- `src/sim/miner/miner_tests.rs:4925` test `dying_refinery_aborts_unload_without_credit_or_stuck_visual` asserts `display_type_override == None`.
- `src/app_instances/units.rs:167..173` consumes `display_type_override` as the Rust equivalent of the gamemd `UnloadingClass` draw swap.

Static gamemd evidence does not prove this immediate visual clear for the zero-link null-refinery abort. It may still be visually correct if native scheduling clears or redraws before presentation, but that requires runtime capture.

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| State-3 null-refinery abort | verified | `0x0073E306..0x0073E338`; decompile `0x0073D630` | exact render-frame presentation |
| Radio `3` receiver path | verified | `UnitClass::Receive_Radio @ 0x00737430`, case `3` | none for `+0x6D1` |
| Radio `0x17` clear path | verified | `0x00737AC9` | caller reachability after this exact abort remains runtime/order-specific |
| Normal state-4 clear path | verified | `0x0073E1F6` | none for healthy release |
| `Queue_Mission` writes | verified | `0x005B35E0` | none for `+0x6D1` |
| `Commence` writes | verified | `0x005B3570` | none for `+0x6D1` |
| `DrawExtras` unload swap | verified | `0x0073CEC0` | render scheduler boundary |
| `UndockUnit` linked cleanup | touched-not-exhausted | `0x004593A0` | linked dock visual-frame details out of scope |
| Current Rust invalid-refinery visual clear | verified by source scan | `miner_dock_sequence.rs:478..484`, `miner_tests.rs:4925` | compare to runtime capture |

## 6. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is the target visual path active in standard YR? -> Yes; stock HARV/CMIN are harvesters with UnloadingClass HORV/CMON.` (evidence: `rulesmd.ini`; `0x0073CEC0`)
- `[RESOLVED] OQ-02 - Does the zero-link null-refinery abort clear +0x6D1? -> No static clear in `0x0073E306..0x0073E338`.` (evidence: `0x0073D630`)
- `[RESOLVED] OQ-03 - Does the optional radio 3 on abort clear +0x6D1? -> No; case 3 delegates to FootClass and returns.` (evidence: `0x00737430`)
- `[RESOLVED] OQ-04 - Does Queue_Mission clear +0x6D1? -> No.` (evidence: `0x005B35E0`)
- `[RESOLVED] OQ-05 - Does Commence clear +0x6D1? -> No.` (evidence: `0x005B3570`)
- `[RESOLVED] OQ-06 - What verified paths clear +0x6D1? -> normal state 4 and radio 0x17.` (evidence: `0x0073E1F6`, `0x00737AC9`)
- `[RESOLVED] OQ-07 - Does DrawExtras require the miner still be in substate 3? -> No; it gates on Harvester, +0x6D1, and UnloadingClass pointer.` (evidence: `0x0073CEC0`)
- `[RESOLVED] OQ-08 - Does linked UndockUnit clear +0x6D1? -> No static write in that helper.` (evidence: `0x004593A0`)
- `[RESOLVED] OQ-09 - What is current Rust behavior? -> invalid refinery abort clears display_type_override immediately.` (evidence: `src/sim/miner/miner_dock_sequence.rs:478..484`)
- `[DEFERRED] OQ-10 - Exact number of stale HORV/CMON rendered frames after zero-link null-refinery abort.` (category: `needs-runtime-debugger`; reason: static code proves no immediate clear, but cannot prove render boundary or later same-frame dispatch; next-step-if-pursued: runtime trace/capture `+0x6D1`, mission dispatch, and render after selling/killing refinery during state-3 unload)
- `[DEFERRED] OQ-11 - Exact same-frame ordering when combat death removes the refinery during the render/tick boundary.` (category: `needs-runtime-debugger`; reason: static caller order is not enough to prove presentation; next-step-if-pursued: non-breaking trace of refinery death, miner mission dispatch, and draw flag reads)

## 7. Visual Composition Ledger

| Order | Function / address | Condition / flag proof | Asset/type | Anchor / rect | Active for target? | Role |
|---|---|---|---|---|---|---|
| 1 | `UnitClass::DrawExtras @ 0x0073CEC0` | `Type+0xE0E != 0`, `unit+0x6D1 != 0`, `Type+0x6B8 != 0` | `HORV` / `CMON` | normal unit draw anchor | yes while flag remains set | unloading body override |
| 2 | `UnitClass::DrawExtras @ 0x0073CEC0` | saved original type restored after draw | `HARV` / `CMIN` | same | always after draw | gameplay type restoration |

| Asset/type | Loaded | Drawn | Visible in target | Role | Evidence |
|---|---|---|---|---|---|
| `HORV` | yes | yes when HARV `+0x6D1` remains set | possible stale after null-refinery abort | unloading override | `UnloadingClass=HORV`; `0x0073CEC0` |
| `CMON` | yes | yes when CMIN `+0x6D1` remains set | possible stale after null-refinery abort | unloading override | `UnloadingClass=CMON`; `0x0073CEC0` |
| `HARV` / `CMIN` | yes | yes when no override | normal miner | base unit | INI; draw fallback |

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Zero-link state-3 null-refinery abort does not clear native `+0x6D1`; it optionally sends radio `3` and queues Harvest. | `0x0073E306..0x0073E338` | mismatch/uncertain: Rust clears `display_type_override` immediately | `src/sim/miner/miner_dock_sequence.rs::abort_invalid_refinery`; render consumer in `src/app_instances/units.rs` | Keep immediate clear only if runtime capture proves no native stale frame is presented; otherwise model delayed clear. | `cmin_refinery_loss_unloading_visual_runtime_frame_count` | Do not cite static Ghidra as proof that abort clears `HORV`/`CMON`. |
| Radio `3`, `Queue_Mission`, and `Commence` are not display-clear paths. | `0x00737430`, `0x005B35E0`, `0x005B3570` | current Rust collapse may hide a stale-frame possibility | miner dock abort and mission transition tests | Separate cargo/mission abort correctness from visual-clear timing. | `invalid_refinery_abort_preserves_cargo_but_does_not_claim_native_visual_clear` | Do not assume returning to Harvest implies normal unit art on the same presented frame. |
| Verified display clears remain normal state 4 and radio `0x17`. | `0x0073E1F6`, `0x00737AC9` | healthy state-4 clear modeled through `phase_departing`; invalid abort differs | `phase_departing`, `abort_invalid_refinery` | Make tests name which native clear path they represent. | `unloading_class_override_clears_on_state4_handoff_not_null_abort` | Do not conflate healthy empty-slot release with refinery-loss abort. |

## 9. Negative Facts / Do Not Do

- Do not say the zero-link null-refinery branch itself clears `HORV`/`CMON`.
- Do not treat radio `3` as equivalent to radio `0x17`; only `0x17` was verified to clear `+0x6D1`.
- Do not treat `Queue_Mission(Harvest, immediate=1)` or `Commence` as visual-clear operations.
- Do not infer the exact number of stale rendered frames from static decompilation alone.
- Do not re-investigate cargo/credits from this report; the only cargo fact used here is that the null path exits before the drain/credit branch.

## 10. Remaining Uncertainty

- Exact stale `HORV`/`CMON` rendered frame count after a zero-link null-refinery abort is runtime-only.
- Exact order between refinery death/sell removal, miner mission dispatch, and the next draw flag read needs a runtime trace or capture.
- Linked nonzero `+0x2E4` `UndockUnit` visual-frame behavior remains out of scope.

## 11. Stale Docs / Follow-Up Wording

- Replace "state-3 missing refinery clears the unload visual" with: "Static binary evidence shows the zero-link state-3 null-refinery branch does not clear `unit+0x6D1`; normal state 4 and radio `0x17` are verified clear paths. Exact stale `HORV`/`CMON` presentation after refinery loss requires runtime capture."
- Replace "current Rust no-stuck visual test proves native parity" with: "Current Rust clears `display_type_override` immediately on invalid-refinery abort; this is a practical behavior but remains native-parity-uncertain until runtime capture proves stock presents no stale frame."

## Sources

- Ghidra read-only decompiled: `UnitClass::Mission_Deploy_Building @ 0x0073D630`, `UnitClass::DrawExtras @ 0x0073CEC0`, `UnitClass::Receive_Radio @ 0x00737430`, `MissionClass::Queue_Mission @ 0x005B35E0`, `MissionClass::Commence @ 0x005B3570`, `BuildingClass::UndockUnit @ 0x004593A0`.
- Ghidra read-only assembly context: `0x0073E306`, `0x0073E31E`, `0x0073E330`, `0x0073E1F6`, `0x00737AC9`, `0x0073CEC0`.
- Prior reports checked: `miner/REFINERY_SOLD_DESTROYED_MID_UNLOAD_RUNTIME_EFFECTS_GHIDRA_REPORT.md`, `miner/EMPTY_SLOT_UNLOAD_GATE_TO_STATE4_RELEASE_TIMING_GHIDRA_REPORT.md`.
- Rust source scanned: `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_tests.rs`, `src/app_instances/units.rs`.

**Status:** COMPLETE
