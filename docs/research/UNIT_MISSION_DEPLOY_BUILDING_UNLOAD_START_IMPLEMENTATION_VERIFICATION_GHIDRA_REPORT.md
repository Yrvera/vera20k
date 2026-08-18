# Unit Mission Deploy Building Unload-Start Implementation Verification - Ghidra Report

**Address(es):** `UnitClass::Mission_Deploy_Building @ 0x0073D630`, `MissionClass::Mission_Dispatch @ 0x005B3060`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** current Rust `MissionQueued -> Pivoting -> start_unload_deploy -> Unloading` behavior versus active YR stock `HARV/CMIN -> GAREFN/NAREFN` mission `0x10` unload-start side effects after radio `0x15` has queued mission `0x10`.  
**Non-Scope:** accepted-cell coordinate proof, 0x15 sender-source race, complete cargo-credit arithmetic, full sound/auditory proof outside the decompiled unload-start block, and Rust edits.  
**Confidence:** High for active binary unload-start field writes, path/facing gates, timer return values, current Rust scan, and implementation deltas. Medium for sound-negative scope because the sibling sound/anim slot is expected to close full audio routing.  
**Active in YR:** Yes. Stock `[CMIN]` and `[HARV]` have `Harvester=yes`; stock `[GAREFN]` and `[NAREFN]` have `DockUnload=yes`/`Refinery=yes`; queued mission `0x10` dispatches through UnitClass vtable slot `+0x23C` to `0x0073D630`.

## 0. Working Notes

Target question: Does current Rust's `phase_mission_queued`, `phase_pivoting`, `start_unload_deploy`, and `phase_unloading` match active YR `UnitClass::Mission_Deploy_Building` unload-start side effects after radio `0x15` queues mission `0x10`?

Non-goals: Do not re-prove NW+3/NW+2, do not edit Rust, do not rediscover `0x15` receiver semantics unless a direct contradiction appears, and do not audit the full later deposit/exit lifecycle beyond the first unload-start latch.

Evidence needed to mark COMPLETE: Decompile plus assembly context for `Mission_Dispatch` mission `0x10` and `Mission_Deploy_Building` path/facing/init ranges; INI/default proof for stock activation; current Rust scan of the named functions; handoff deltas for path gate, facing gate, `+0x6D1`, `+0xF8`, `+0x100..+0x10C`, optional anim slot 7, and `+0xBC = 3`.

Stop conditions: Stop after proving the stock mission `0x10` path reaches `0x0073D630`, identifying every unload-start write/order in the active stock branch, comparing each to current Rust, and listing negative facts/do-not-do items.

## 1. Overview

Current Rust is much closer than the older `phase_linked` model: it has a `MissionQueued` phase where `0x15` no longer starts unload immediately, and `Pivoting` gates unload start on the same east-facing window used by gamemd. That is the right high-level split.

The remaining parity gaps are implementation-level. Active gamemd mission `0x10` first checks `PathType::Has_Valid_Steps`, then checks the facing RateTimer window. If facing is not ready it calls locomotor `+0x4C(0x4000)` and returns delay `5`, without setting unload-active fields. Only after path/facing pass does it write `+0xF8=0`, `+0x6D1=1`, initialize the timer cluster at `+0x100..+0x10C`, optionally set refinery anim slot `7`, and finally write `+0xBC=3`.

Rust currently starts unload by setting Rust bookkeeping/visual state (`link_on_pad`, `display_type_override`, forced east facing, `DockDeploy`, `unload_timer = interval - 10`) rather than modeling those exact fields and return cadence. That is closer than before but still a DRIFT under the parity bar.

## 2. Class Layout / Key Offsets

| Offset / field | Owner | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| Mission id `0x10` | MissionClass | queued by stock refinery `0x15`; dispatches through UnitClass vtable `+0x23C` | `Mission_Dispatch @ 0x005B3060`, assembly `0x005B3260..0x005B3264` | Yes |
| Unit vtable `+0x23C` | UnitClass | mission `0x10` handler, `UnitClass::Mission_Deploy_Building` | decompile `0x005B3060`, prior vtable evidence in `RADIO_0X15_START_UNLOAD_SIDE_EFFECTS_GHIDRA_REPORT.md` | Yes |
| UnitType `+0xE0E` | UnitTypeClass | `Harvester=yes` gate for HARV/CMIN unload path | `0x0073D678`, `0x0073E005`, INI `[CMIN]/[HARV] Harvester=yes` | Yes |
| UnitType `+0xE0F` | UnitTypeClass | `Weeder=yes`; TS/conditional path, not stock HARV/CMIN | `0x0073D686`, stock INI absence for HARV/CMIN | No for stock HARV/CMIN |
| Unit `+0x2E4` | UnitClass | reciprocal dock-link pointer/index tested at function entry; stock zero-link path uses `0` | entry branch in `0x0073D630`, prior reachability docs | Conditional; normal stock refinery unload uses zero-link |
| Unit `+0x388` | UnitClass | FacingClass/RateTimer sampled for east-window accept | `0x0073DF56..0x0073DF72` | Yes |
| Unit locomotor `+0x674` | UnitClass | called through vtable `+0x4C(0x4000)` when facing not ready | `0x0073DF7A..0x0073DFAD` | Yes |
| Unit `+0xF8` | UnitClass | integer dump accumulator; reset to `0` at unload start | `0x0073DFD0` | Yes |
| Unit `+0x100` | UnitClass | timer start frame; set to `g_CurrentFrameCounter` at unload start | `0x0073DFE0..0x0073DFF3` | Yes |
| Unit `+0x104` | UnitClass | timer secondary/Z storage copied from stack value | `0x0073DFF5..0x0073DFF9` | Yes |
| Unit `+0x108` | UnitClass | timer duration/step interval set to `1` | `0x0073DFFC` | Yes |
| Unit `+0x10C` | UnitClass | timer active/repeat flag set to `1` | `0x0073DFED` | Yes |
| Unit byte `+0x6D1` | UnitClass | unload-active latch; gates UnloadingClass draw | `0x0073DFDA`, render proof in `UNLOADINGCLASS_RENDER_ORIENTATION_GHIDRA_REPORT.md` | Yes |
| Unit `+0xBC` | MissionClass/UnitClass | mission substate; unload start writes `3` after anim-slot attempt | `0x0073E093` | Yes |
| Building anim slot `7` | BuildingClass | optional first unload-start refinery anim slot if adjacent refinery found | `0x0073E013..0x0073E08E` | Conditional; active code path, visual only if asset/slot exists |

## 3. Core Logic

### 3.1 Mission `0x10` dispatch target

`MissionClass::Mission_Dispatch @ 0x005B3060` dispatches the current mission id. For mission `0x10`, the decompile calls vtable slot `+0x23C`; assembly context confirms `0x005B3260 MOV EDX,[EDI]`, `0x005B3262 MOV ECX,EDI`, `0x005B3264 CALL dword ptr [EDX + 0x23C]`.

Active in YR: Yes. This is the live MissionClass dispatcher for UnitClass missions, and stock refinery `0x15` queues mission `0x10` with queued flag `0`.

### 3.2 Entry path and stock harvester activation

For stock `HARV/CMIN`, the relevant branch is the zero-link stock unload path, not the conditional reciprocal `+0x2E4` branch. `UnitClass::Mission_Deploy_Building @ 0x0073D630` checks `UnitTypeClass+0xE0E Harvester` and `+0xE0F Weeder`; stock HARV/CMIN take the harvester branch and not the Weeder branch.

Active in YR: Yes. `ini/rulesmd.ini` has `[CMIN] Harvester=yes`, `Storage=20`, `UnloadingClass=CMON`; `[HARV] Harvester=yes`, `Storage=40`, `UnloadingClass=HORV`; stock refineries have `[GAREFN]/[NAREFN] DockUnload=yes` and `Refinery=yes`.

### 3.3 Path gate before unload start

At `0x0073DEE0..0x0073DEE9`, the function calls `PathType::Has_Valid_Steps`. If false, it does cleanup: calls vtable `+0x484(0,1)`, clears `+0x6D1`, may call locomotor stop/mission queue helpers, and returns `1`. It does not initialize state `3`.

Active in YR: Yes. This gate is in the stock harvester branch and runs before the facing gate and before any unload latch write.

Rust delta: `phase_pivoting` only calls `sync_dock_facing` and then `start_unload_deploy`; no exact PathType validity cleanup gate is modeled in `src/sim/miner/miner_dock_sequence.rs:837..852`.

### 3.4 Facing gate and return cadence

At `0x0073DF56..0x0073DF72`, gamemd reads the unit RateTimer current value and accepts only when `((current >> 7) + 1) & 0x1FE == 0x80`. If not accepted, and byte `+0x6AF` is clear, it calls the locomotor vtable `+0x4C(0x4000)` at `0x0073DF7A..0x0073DFAD`, then returns hard delay `5` at `0x0073DFB3..0x0073DFBC`.

Active in YR: Yes. This is the stock HARV/CMIN unload-start gate and is the same east-facing window used by previous 0x16 reports.

Rust delta: `sync_dock_facing` uses the same window shape and target `0x4000`, which is directionally correct. However `phase_pivoting` is evaluated every sim tick while not accepted (`src/sim/miner/miner_dock_sequence.rs:844..851`), whereas gamemd returns mission delay `5` after each not-ready mission `0x10` pass. Rust is too eager unless another scheduler layer enforces that five-frame delay.

### 3.5 Unload-start write order

When facing is accepted and `+0x6D1` is not already set, gamemd writes in this order:

1. `0x0073DFD0`: `Unit+0xF8 = 0`.
2. `0x0073DFDA`: byte `Unit+0x6D1 = 1`.
3. `0x0073DFE0`: read `g_CurrentFrameCounter`.
4. `0x0073DFED`: `Unit+0x10C = 1`.
5. `0x0073DFF3`: `Unit+0x100 = current frame`.
6. `0x0073DFF5..0x0073DFF9`: `Unit+0x104 = stack value`.
7. `0x0073DFFC`: `Unit+0x108 = 1`.
8. `0x0073E005..0x0073E00D`: if UnitType `Harvester=yes`, continue to adjacent-refinery anim lookup; otherwise skip to state write.
9. `0x0073E013..0x0073E08E`: look one adjacent cell from unit current cell, find building, compare building health to `Rules+0x1700`, then call `BuildingClass::SetAnimSlotImage(7, damaged, 0)` if found.
10. `0x0073E093`: `Unit+0xBC = 3`.
11. `0x0073E09D -> 0x0073E289`: return through mission timer epilogue (`MissionTimerEntry * 900`, `ftol`, plus `RandomRanged(0,2)`).

Active in YR: Yes. All writes are in the stock harvester branch after path/facing accept.

Rust delta: `start_unload_deploy` at `src/sim/miner/miner_dock_sequence.rs:805..830` does not model this field cluster. It writes Rust-side dock occupancy, display override, hard facing, sound event, and `unload_timer`, then enters `Unloading`.

### 3.6 Later cargo gate is not part of unload-start side effects

After state `3`, the later cargo gate compares the integer accumulator `+0xF8` against `Rules+0x1528 HarvesterDumpRate * 900.0` at `0x0073E35B..0x0073E374`. The first actual storage drain happens only when the threshold is crossed.

Active in YR: Yes. Stock default is effectively `0.016 * 900 = 14.4` frames.

Rust delta: `phase_unloading` drains when `unload_timer <= 0` and seeds `unload_timer = interval - 10` on unload start. This likely approximates the first 15-frame wait at default settings, but it is not the gamemd mechanism because Rust does not use the `+0xF8/+0x100..+0x10C` style per-frame accumulator.

## 4. INI Keys

| INI key | Stock value | Effect in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `[GAREFN] DockUnload` | `yes` | stock refinery receiver path queues mission `0x10` after `0x15` | `ini/rulesmd.ini:11726`, `RADIO_0X15_START_UNLOAD_SIDE_EFFECTS_GHIDRA_REPORT.md` | Yes |
| `[NAREFN] DockUnload` | `yes` | same | `ini/rulesmd.ini:12519` | Yes |
| `[GAREFN]/[NAREFN] Refinery` | `yes` | later state-3/state-4 refinery animation/exit branches | `ini/rulesmd.ini:11727`, `12520`; `0x0073E4DC`, `0x0073E1D5` | Yes |
| `[CMIN] Harvester` | `yes` | selects UnitType `+0xE0E` harvester branch | `ini/rulesmd.ini:7364`; `0x0073D678`/`0x0073E005` | Yes |
| `[HARV] Harvester` | `yes` | same | `ini/rulesmd.ini:8228`; `0x0073D678`/`0x0073E005` | Yes |
| `[CMIN] UnloadingClass` | `CMON` | gamemd draw path swaps TypeClass while `+0x6D1` is set | `ini/rulesmd.ini:7384`, `UNLOADINGCLASS_RENDER_ORIENTATION_GHIDRA_REPORT.md` | Yes |
| `[HARV] UnloadingClass` | `HORV` | same | `ini/rulesmd.ini:8246`, `UNLOADINGCLASS_RENDER_ORIENTATION_GHIDRA_REPORT.md` | Yes |
| `[General] HarvesterDumpRate` | default `0.016` if not overridden | later state-3 gate threshold `rate * 900.0` | `0x0073E361..0x0073E374`, prior `UNIT_0x3E_BALE_CADENCE_TIMER_GHIDRA_REPORT.md` | Yes |

## 5. Integration Points

| Function / point | Role | Evidence | Active in YR |
|---|---|---|---|
| `BuildingClass::Receive_Radio(0x15)` | queues sender mission `0x10`, queued flag `0`; no unload-start side effects | `RADIO_0X15_START_UNLOAD_SIDE_EFFECTS_GHIDRA_REPORT.md` | Yes |
| `MissionClass::Mission_Dispatch @ 0x005B3060` | dispatches mission `0x10` via vtable `+0x23C`, stores returned delay | decompile and assembly `0x005B3260..0x005B3264` | Yes |
| `UnitClass::Mission_Deploy_Building @ 0x0073D630` | owns path/facing gates and first unload latch/state writes | decompile and assembly contexts listed above | Yes |
| `TechnoClass::AI_Update` timer cluster | increments `+0xF8` via timer cluster after unload start | prior `UNIT_0x3E_BALE_CADENCE_TIMER_GHIDRA_REPORT.md` | Yes |
| `UnitClass::DrawExtras` | swaps to UnloadingClass while `+0x6D1` is set | `UNLOADINGCLASS_RENDER_ORIENTATION_GHIDRA_REPORT.md` | Yes |

## 6. Current Rust Implementation Status

| Rust surface | Current behavior | Parity status |
|---|---|---|
| `src/sim/miner/mod.rs:103..111` | `FaceSync` and `MissionQueued` docs correctly say `0x16`/`0x15` have no unload, sound, pad-snap, or pad-occupancy side effects | Much closer; matches settled `0x15` split |
| `src/sim/miner/miner_dock_sequence.rs:741..769 phase_face_sync` | marks contact-entered, samples/syncs facing, waits for Enter retry due before `MissionQueued` | Broadly aligned with parent model; not the focus of this report |
| `src/sim/miner/miner_dock_sequence.rs:772..774 phase_mission_queued` | immediately advances to `Pivoting` on next FSM tick | Partial. It represents queued mission `0x10`, but does not model MissionClass current/queued mission fields or the exact dispatch-timer boundary |
| `src/sim/miner/miner_dock_sequence.rs:837..852 phase_pivoting` | samples facing every sim tick; on accept calls `start_unload_deploy` | Partial/DRIFT. Window shape is correct, but gamemd not-ready mission `0x10` returns delay `5`; Rust has no five-frame retry delay here and no PathType cleanup gate |
| `src/sim/miner/miner_dock_sequence.rs:805..830 start_unload_deploy` | calls `link_on_pad`, sets `display_type_override`, forces facing east, emits `DockDeploy`, seeds `unload_timer = interval - 10`, phase `Unloading` | DRIFT. These are not the exact gamemd field writes; `link_on_pad` has no proven stock field equivalent; force-facing and sound are not observed unload-start writes |
| `src/sim/miner/miner_dock_sequence.rs:854..935 phase_unloading` | waits by `unload_timer`, drains one whole resource slot, credits refinery owner, applies purifier bonus, emits bale event | Largely aligned with later state-3 slot-drain concept, but mechanism still differs because gamemd uses `+0xF8` accumulator and `+0x100..+0x10C` timer cluster |
| `src/sim/miner/miner_dock.rs:22..36`, `92..94` | `on_pad` is described as physical pad occupancy and set by `link_on_pad` | DRIFT/RISK. Normal stock zero-link path has no verified reciprocal pad/link field at unload start; using this as physical occupancy may overconstrain waiting miners |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Mission `0x10` dispatch through UnitClass vtable `+0x23C` | verified | `0x005B3060`, `0x005B3260..0x005B3264` | none |
| Stock HARV/CMIN branch activation | verified | `0x0073D678`, `0x0073E005`, INI HARV/CMIN `Harvester=yes` | none |
| PathType valid-steps gate before unload start | verified | `0x0073DEE0..0x0073DF55` | exact helper internals out of scope |
| Facing RateTimer accept window | verified | `0x0073DF56..0x0073DF72` | none |
| Not-ready facing side effect and return delay | verified | `0x0073DF7A..0x0073DFBC` | none |
| First unload-start writes `+0xF8`, `+0x6D1`, `+0x100..+0x10C`, `+0xBC=3` | verified | `0x0073DFD0..0x0073E093` | exact source of stack value written to `+0x104` not needed for Rust handoff yet |
| Optional anim slot 7 | verified | `0x0073E013..0x0073E08E` | full asset/timing visual proof belongs to slot 3 |
| Direct sound call inside unload-start init | verified negative for this range | no sound call in `0x0073DFD0..0x0073E09D` | full audio path deferred to sound/anim slot |
| Later state-3 cargo gate | touched-not-exhausted | `0x0073E35B..0x0073E374`, prior timer report | full credit arithmetic already covered elsewhere |
| Rust named surfaces | verified scan | source line references in section 6 | implementation patch separate |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does mission 0x10 dispatch to UnitClass::Mission_Deploy_Building? -> Yes, Mission_Dispatch calls vtable +0x23C for mission 0x10, and UnitClass +0x23C resolves to 0x0073D630 in prior vtable evidence.` (evidence: `0x005B3060`, `0x005B3260..0x005B3264`, `RADIO_0X15_START_UNLOAD_SIDE_EFFECTS_GHIDRA_REPORT.md`; Active in YR: Yes)
- `[RESOLVED] OQ-02 - Is this active for stock HARV/CMIN? -> Yes, stock HARV/CMIN set Harvester=yes and stock GAREFN/NAREFN set DockUnload=yes.` (evidence: `ini/rulesmd.ini:7364`, `8228`, `11726`, `12519`; Active in YR: Yes)
- `[RESOLVED] OQ-03 - Does mission 0x10 check path validity before unload init? -> Yes, PathType::Has_Valid_Steps is called at 0x0073DEE2 and false goes to cleanup/return 1 before unload latch writes.` (evidence: `0x0073DEE0..0x0073DF55`; Active in YR: Yes)
- `[RESOLVED] OQ-04 - What is the facing accept condition? -> `((RateTimerCurrent >> 7) + 1) & 0x1FE == 0x80`.` (evidence: `0x0073DF56..0x0073DF72`; Active in YR: Yes)
- `[RESOLVED] OQ-05 - What happens when facing is not ready? -> If +0x6AF is clear, locomotor +0x4C(0x4000) is called and the mission returns delay 5 without setting +0x6D1 or +0xBC=3.` (evidence: `0x0073DF7A..0x0073DFBC`; Active in YR: Yes)
- `[RESOLVED] OQ-06 - What starts unload-active state? -> The first accepted mission 0x10 pass writes +0xF8=0, +0x6D1=1, initializes +0x100..+0x10C, optionally slot 7, then +0xBC=3.` (evidence: `0x0073DFD0..0x0073E093`; Active in YR: Yes)
- `[RESOLVED] OQ-07 - Does unload start force the unit facing to East? -> No explicit facing write was observed in the unload-start init block; the branch requires the live facing timer already be in the accepted east window.` (evidence: `0x0073DF56..0x0073E09D`; Active in YR: Yes)
- `[RESOLVED] OQ-08 - Does unload start set a stock reciprocal pad/link field? -> No write to +0x2E4 is present in the stock zero-link unload-start block; normal stock refinery unload remains zero-link.` (evidence: `0x0073DFD0..0x0073E09D`, `STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_REACHABILITY_GHIDRA_REPORT.md`; Active in YR: Yes)
- `[RESOLVED] OQ-09 - Does unload start directly play a sound in this function range? -> No direct sound/Voc call appears in the verified init range; the verified side effect is optional building anim slot 7.` (evidence: `0x0073DFD0..0x0073E09D`; Active in YR: Yes for negative in this range)
- `[RESOLVED] OQ-10 - When does first cargo drain happen? -> Later state 3, gated by +0xF8 >= HarvesterDumpRate * 900.0, not in the unload-start write block.` (evidence: `0x0073E35B..0x0073E374`; Active in YR: Yes)
- `[RESOLVED] OQ-11 - Does current Rust keep 0x15 side effects separated? -> Yes at the named phase level: `MissionQueued` does not itself snap, sound, or drain; unload-active effects are delayed until `Pivoting` accepts.` (evidence: `src/sim/miner/mod.rs:103..116`, `src/sim/miner/miner_dock_sequence.rs:772..830`; Active in YR: Rust comparison)
- `[DEFERRED] OQ-12 - Exact full audio routing for DockDeploy or equivalent?` (category: requires-different-system-context; reason: sibling slot covers sound/anim timing; this report only proves no direct sound call in the unload-start init block; next-step-if-pursued: reconcile with `REFINERY_DOCK_DEPLOY_SOUND_ANIM_TIMING_IMPLEMENTATION_VERIFICATION`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Mission `0x10` dispatch is the first place actual unload-start side effects run | `0x005B3260..0x005B3264`, `0x0073D630` | Rust has `MissionQueued`, then next FSM tick `Pivoting`; no exact MissionClass queued/current dispatch timer | `phase_mission_queued`, miner mission/timer modeling | Preserve a queued-mission boundary and dispatch only on the modeled mission tick, not inside the radio receiver | After modeled `0x15`, same step shows no unload-active visual, no cargo drain, no pad occupancy; later mission dispatch can start gates | Do not collapse `0x15` and mission `0x10` dispatch into one side-effect step |
| Mission `0x10` first checks PathType valid steps and may cleanup/return before unload init | `0x0073DEE0..0x0073DF55` | Missing; `phase_pivoting` goes straight to facing sync | `phase_pivoting` / future mission deploy helper | Add a path/contact-valid gate equivalent before setting unload-active state | Fixture with invalid/lost path at mission `0x10` start must not set `UnloadingClass`, `unload_timer`, or `on_pad` | Do not start unload just because facing is accepted if the path gate would fail |
| Not-facing-ready mission `0x10` calls locomotor turn and returns delay `5` | `0x0073DF56..0x0073DFBC` | Rust checks facing every tick while Pivoting | `phase_pivoting`, miner dock timing | Gate repeated pivot checks by the mission return delay, or otherwise prove scheduler equivalence | Slow-turn miner requires at least 5 frames between not-ready mission `0x10` checks and does not start unload mid-delay | Do not poll the deploy facing gate every frame |
| Accepted unload start writes `+0xF8=0`, `+0x6D1=1`, `+0x100=current frame`, `+0x108=1`, `+0x10C=1`, then `+0xBC=3` | `0x0073DFD0..0x0073E093` | Rust uses `display_type_override`, `unload_timer`, and phase enum instead of explicit equivalent fields | `start_unload_deploy`, `Miner` state, render-facing unit state | Model an unload-active latch equivalent to `+0x6D1`, the bale accumulator/timer cluster, and substate `3` ordering | On the first accepted deploy pass, render swap becomes active before first cargo drain, accumulator is zero, and first drain waits for the dump-rate threshold | Do not use only `display_type_override` as the simulation source of truth if parity needs byte-state equivalence |
| Optional refinery anim slot 7 is set during unload-start init after the field cluster and before `+0xBC=3` | `0x0073E013..0x0073E08E` | Rust emits `DockDeploy` sound but does not explicitly model anim slot 7 | `start_unload_deploy`, building animation system | Open/trigger the building-side slot 7 effect at this point if the target slot/asset exists | First unload-start frame on refinery with slot-7 anim schedules the same building animation before deposit loop | Do not substitute an audio event for the building anim-slot side effect |
| No stock `+0x2E4` reciprocal link or position snap is written in unload-start init | `0x0073DFD0..0x0073E09D`, stock reachability reports | Rust calls `link_on_pad` and documents it as physical pad occupancy | `start_unload_deploy`, `RefineryDockContacts::on_pad` | Either remove physical meaning for stock zero-link unload or prove it is only internal queue bookkeeping with byte-identical external effects | Waiter behavior should be blocked by Contacts/contact-entered exactly as gamemd, not by an invented physical pad link | Do not treat Rust `on_pad` as a stock gamemd `+0x2E4` equivalent |
| Unload start does not explicitly force the facing byte; it requires the RateTimer already be in the east window | `0x0073DF56..0x0073E09D` | Rust sets `entity.facing = DOCK_FACING_EAST` in `start_unload_deploy` | `start_unload_deploy` | Preserve the live facing/timer result; avoid snapping if the accepted window can include neighboring values | Capture the facing value immediately before and after unload-start; it should not jump except by normal RateTimer/locomotor update | Do not hide pivot drift by snapping facing at unload start |
| First cargo drain is later state 3 threshold, not the unload-start frame | `0x0073E35B..0x0073E374` | Rust's `interval - 10` likely gives close default cadence but not exact timer mechanism | `phase_unloading`, `MinerConfig::unload_tick_interval`, future accumulator fields | Use an accumulator/timer equivalent if exact byte/timing parity is required | First slot drain occurs when integer accumulator crosses `HarvesterDumpRate * 900.0`, not because a local countdown was pre-decremented | Do not drain or emit a bale event on the first unload-start tick |

## 10. Negative Facts / Do Not Do

- Do not move unload-active effects back into radio `0x15`; that remains only mission queueing.
- Do not treat `MissionQueued` itself as deploy/unload start.
- Do not poll the mission `0x10` facing gate every frame unless a separate scheduler layer proves the same five-frame return cadence.
- Do not set physical pad/link occupancy as a claimed stock gamemd field; no `+0x2E4` write occurs in the verified stock zero-link unload-start block.
- Do not snap the miner's position or facing at unload start. The binary checks the current facing timer window and then writes unload state.
- Do not substitute `DockDeploy` sound for the verified unload-start side effect. The verified unload-start visual action is optional building anim slot `7`; full sound timing belongs to the sibling sound/anim verification.
- Do not replace the `+0xF8/+0x100..+0x10C` timer cluster with a local countdown without marking it as mechanism drift.

## Sources

- Ghidra decompiled read-only: `MissionClass::Mission_Dispatch @ 0x005B3060`.
- Ghidra assembly context: `0x005B3260..0x005B3264` mission `0x10` vtable `+0x23C` dispatch.
- Ghidra decompiled read-only: `UnitClass::Mission_Deploy_Building @ 0x0073D630`.
- Ghidra assembly context: `0x0073DEE0..0x0073E09D`, especially `0x0073DF56..0x0073DFBC`, `0x0073DFD0..0x0073E093`.
- Ghidra assembly context: `0x0073E289..0x0073E2BE` mission timer epilogue.
- Ghidra assembly context: `0x0073E35B..0x0073E374` later dump-rate gate.
- Prior docs referenced: `RADIO_0X15_START_UNLOAD_SIDE_EFFECTS_GHIDRA_REPORT.md`, `miner/MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md`, `UNIT_0x3E_BALE_CADENCE_TIMER_GHIDRA_REPORT.md`, `UNLOADINGCLASS_RENDER_ORIENTATION_GHIDRA_REPORT.md`, `STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_REACHABILITY_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scanned: `src/sim/miner/mod.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_dock.rs`.
