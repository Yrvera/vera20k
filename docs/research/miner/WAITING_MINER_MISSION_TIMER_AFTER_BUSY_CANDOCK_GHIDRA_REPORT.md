# Waiting Miner Mission Timer After Busy CAN_DOCK - Ghidra Research Report

**Address(es):** `0x004D9290`, `0x005B3060`, `0x005B3A00`, `0x005B3700`, `0x005B3760`, `0x0065C7E0`, `0x00744270`, `0x0043C2D0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Standard YR `HARV/CMIN -> GAREFN/NAREFN` waiting/refused refinery enter timing: what delay `FootClass::Mission_Enter` returns after a `CAN_DOCK(0x0E)` attempt, how `MissionClass::Mission_Dispatch` stores/checks that delay, the exact `[Enter]` mission timer value, random jitter bounds, and whether this controls normal two-miner refinery contention.  
**Non-Scope:** first rendered movement/pixel frame, full refinery receiver switch outside the `0x0E` refusal/acceptance implications, non-refinery enter targets, and runtime replay-specific object-vector order.  
**Confidence:** High for timer formula, field writes, `[Enter]` base value, and inclusive random bounds; Medium for concrete natural-replay same-frame outcomes because they still depend on live object order and current timer state.  
**Active in YR:** Yes. Mission `7` is the active Mission Enter handler for standard unit/refinery enter, and stock `[CMIN]`/`[HARV]` target stock one-dock `[GAREFN]`/`[NAREFN]`.

## 1. Overview

A waiting miner does not retry `CAN_DOCK` every tick by default. `MissionClass::Mission_Dispatch` calls Mission Enter only when the mission timer is eligible. For mission `7` (`[Enter]`) the stock timer is `ftol(0.016 * 900.0) + RandomRanged(0,2)`, which is a base `14` frames plus inclusive jitter `0..2`, so the normal Enter retry cadence is `14`, `15`, or `16` frames after the previous dispatched Mission Enter pass.

Same-frame takeover after another miner releases is therefore possible only when the waiter is processed later in the live-object pass and its Mission Enter timer is already due on that frame. A release does not reset or bypass the waiter's timer.

## 2. Class Layout / Key Offsets

| Offset / data | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| MissionClass `+0xAC` | current mission id; mission `7` dispatches Enter | `0x005B3060`, `0x005B3A00` | Yes |
| MissionClass `+0xB4` | queued mission id | `0x005B35E0`, `0x005B3570` | Yes |
| MissionClass `+0xB8` | queued/commence byte cleared on queue/commence | `0x005B35E0`, `0x005B3570` | Yes |
| MissionClass `+0xC8` | mission timer start frame | `0x005B3060` | Yes |
| MissionClass `+0xCC` | timer middle/scratch dword written from stack by dispatch | `0x005B3060` | Yes, not used by the gate in this slice |
| MissionClass `+0xD0` | mission timer duration returned by handler | `0x005B3060` | Yes |
| Unit byte `+0x418` | already-entered preserve flag for non-ROGER `CAN_DOCK` | `0x004D92BF..0x004D92CC` | Yes |
| `DAT_00A8E3A8` | 32 mission-control entries, 32 bytes each | `0x005B3A00`, `0x005B3700`, `0x00679C95` | Yes |
| Mission entry `+0x10` | `Rate` double | `0x005B3760`, `0x004D946C` assembly bytes | Yes |
| `DAT_007E27F8` | `900.0` multiplier | `0x004D9475` assembly bytes; same constant used by other timing reports | Yes |

## 3. Core Logic

### 3.1 Mission dispatch gate

`MissionClass::Mission_Dispatch @ 0x005B3060` runs `ObjectClass::AI`, then checks the mission timer before calling the current mission handler:

1. Read `start = this+0xC8` and `duration = this+0xD0`.
2. If `start != -1`, compute `elapsed = g_CurrentFrameCounter - start`.
3. If `elapsed >= duration`, dispatch the mission.
4. If `elapsed < duration`, return without calling the mission.
5. If `start == -1`, dispatch when `duration == 0`; otherwise return.
6. For mission id `7`, call vtable slot `+0x240` (`FootClass::Mission_Enter @ 0x004D9290`).
7. After the handler returns, write `+0xC8 = g_CurrentFrameCounter`, write `+0xCC` from the local stack slot, and write `+0xD0 = handler_return`.

The inclusive/exclusive boundary is therefore: the next dispatch is eligible when `elapsed >= duration`, not when `elapsed > duration`.

### 3.2 Mission Enter epilogue and exact Enter timer

`FootClass::Mission_Enter @ 0x004D9290` always reaches the standard mission-timer epilogue after the scoped `CAN_DOCK` branch. The relevant assembly bytes at `0x004D946C` are:

```text
CALL 0x005B3A00                 ; EAX = &MissionControl[current mission]
FLD  qword ptr [EAX+0x10]       ; Rate
FMUL qword ptr [0x007E27F8]     ; 900.0
CALL Math__ftol
MOV  ESI,EAX
...
PUSH 2
PUSH 0
CALL Random__RandomRanged
ADD  EAX,ESI
RET
```

`MissionClass::GetMissionTimerEntry @ 0x005B3A00` computes `DAT_00A8E3A8 + current_mission * 0x20`. The table constructor at `0x005B3700` initializes every entry with default `Rate = 0.016` and default `AARate = 0.016`. `RulesClass::ReadTypeData @ 0x00679C95` iterates 32 entries, writes the entry index, and calls `MissionClass::Read_INI`. `MissionClass::Read_INI @ 0x005B3760` reads the mission section name from `g_MissionNameTable`, then reads `Rate` into entry `+0x10` and `AARate` into `+0x18`, copying `Rate` to `AARate` when `AARate` is zero.

Stock `rulesmd.ini:[Enter]` has `Rate=.016` and no `AARate`, so mission `7` keeps `Rate=0.016`. `0.016 * 900.0 = 14.4`; both truncation and round-to-nearest produce `14` here. `Random__RandomRanged(0,2) @ 0x0065C7E0` is inclusive because it computes `max-min`, samples a masked value until `value <= max-min`, then returns `min + value`. The returned Mission Enter delay is therefore `14`, `15`, or `16` frames.

### 3.3 Busy/refused CAN_DOCK branch

`FootClass::Mission_Enter` sends `CAN_DOCK(0x0E)` to the selected target. If the reply is `1`, the enter path proceeds. If the reply is not `1` but unit byte `+0x418` is nonzero, it also preserves the enter path. If the reply is not `1` and `+0x418 == 0`, it sends radio `BREAK(3)` and calls the unit mission queue/assignment slot with `(0,1)` before falling through to the same timer epilogue.

That means a genuine refused `CAN_DOCK` is not a tight retry loop. It either remains under the current Mission Enter timer until a later commence boundary, or it queues/commences mission `0` depending on the unit readiness path. For normal stock two-miner refinery contention, the takeover rule does not rely on a building-side FIFO or an every-tick busy-CAN_DOCK loop; B can claim only on B's own eligible Mission Enter dispatch after the contact slot is free.

### 3.4 Standard YR two-miner activity

The path is active for stock YR:

- `[CMIN]` and `[HARV]` have `Harvester=yes` and `Dock=NAREFN,GAREFN`.
- `[GAREFN]` and `[NAREFN]` have `DockUnload=yes`, `Refinery=yes`, and `NumberOfDocks=1`.
- The earlier same-frame-order report verifies A's release only clears contacts; it does not call B.
- Therefore B's retry is governed by its own mission dispatch eligibility: live-object order plus `+0xC8/+0xD0`.

In concrete terms: if B's previous Mission Enter dispatch stored `+0xD0 = 14..16` at frame `F0`, B cannot send the next `CAN_DOCK` until `g_CurrentFrameCounter - F0 >= +0xD0`. If A frees the refinery on frame `F` and B is processed later on frame `F`, B can claim in that same frame only if this timer condition is already true.

## 4. INI Keys

| Key | Stock YR value / location | Effect | Active in YR |
|---|---|---|---|
| `[Enter] Rate` | `.016`, `ini/rulesmd.ini:30510` | Mission Enter base timer; `0.016 * 900 = 14.4 -> 14` | Yes |
| `[Enter] AARate` | absent / zero, copied from `Rate` | No different AA cadence for this mission | Yes |
| `[CMIN] Dock` / `Harvester` | `NAREFN,GAREFN` / `yes`, `ini/rulesmd.ini:7361`, `7364` | Makes CMIN use standard refinery return/enter path | Yes |
| `[HARV] Dock` / `Harvester` | `NAREFN,GAREFN` / `yes`, `ini/rulesmd.ini:8225`, `8228` | Makes War Miner use standard refinery return/enter path | Yes |
| `[GAREFN] DockUnload` / `Refinery` / `NumberOfDocks` | `yes` / `yes` / `1` | Standard one-contact refinery contention target | Yes |
| `[NAREFN] DockUnload` / `Refinery` / `NumberOfDocks` | `yes` / `yes` / `1` | Standard one-contact refinery contention target | Yes |

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `FootClass::Mission_Enter @ 0x004D9290` | Sends `CAN_DOCK`, handles accept/preserve/refuse, returns timer+jitter | decompile and assembly bytes | Yes |
| `MissionClass::Mission_Dispatch @ 0x005B3060` | Enforces `+0xC8/+0xD0` timer gate and stores handler return | decompile | Yes |
| `MissionClass::GetMissionTimerEntry @ 0x005B3A00` | Indexes mission-control entry by current mission id | decompile and bytes | Yes |
| Mission entry constructor `0x005B3700` | Sets default `Rate=AARate=0.016`, flags, id `-1` | raw code bytes / decompile boundary | Yes |
| `MissionClass::Read_INI @ 0x005B3760` | Reads `[Enter] Rate` / `AARate` | decompile | Yes |
| `Random__RandomRanged @ 0x0065C7E0` | Inclusive integer random range | decompile | Yes |
| `UnitClass::ShouldIdle @ 0x00744270` | Readiness gate used when mission queue is asked to commence immediately | decompile | Yes |
| `BuildingClass::Receive_Radio @ 0x0043C2D0` | Receiver-side `CAN_DOCK` accept/refuse owner | prior and current decompile | Yes |

## 6. Current Rust Implementation Status

Rust has order-dependent refinery tests, but it does not model the native MissionClass timer/jitter around `MissionEnter` as a first-class timer:

- `src/sim/miner/miner_system.rs` processes miner snapshots in deterministic stable-id order. This can represent the binary's order-dependent same-tick rule, but stable id is not gamemd's live-object vector identity.
- `src/sim/miner/miner_dock_sequence.rs::phase_mission_enter` can retry/admit immediately when the phase runs; there is no explicit `14..16` frame Mission Enter retry delay.
- `src/sim/miner/miner_tests.rs` already covers waiter-after-releaser and waiter-before-releaser order, plus QueueingCell vs accepted-cell split. Those tests should remain order-dependent, but a future parity pass should add Mission Enter timer cadence tests if exact takeover frame timing is implemented.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Mission dispatch `+0xC8/+0xD0` gate | verified | `0x005B3060` | none |
| Mission id `7` dispatch to `FootClass::Mission_Enter` | verified | `0x005B3060`, vtable xrefs from prior report | none |
| Mission Enter timer epilogue | verified | `0x004D946C..0x004D9497` bytes, decompile | none |
| Mission-control entry stride and rate offset | verified | `0x005B3A00`, `0x005B3760` | none |
| `[Enter] Rate=.016` stock value | verified | `ini/rulesmd.ini:30507..30510` | none |
| Inclusive `RandomRanged(0,2)` | verified | `0x0065C7E0` | none |
| Refused `CAN_DOCK` branch | verified | `0x004D92BF..0x004D92E8` | concrete replay frequency of this exact branch remains scenario-dependent |
| Natural two-miner same-frame outcome | touched-not-exhausted | static rule from this report plus same-frame-order report | runtime replay can still log exact object order/timer value |
| Current Rust order tests | touched | source scan | run focused tests in implementation pass |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-001 - Does B retry every tick after a prior Mission Enter pass? -> No. Mission dispatch returns before calling Mission Enter while `elapsed < +0xD0`.` (evidence: `0x005B3060`)
- `[RESOLVED] OQ-002 - What does Mission Enter return for stock `[Enter]`? -> `ftol(0.016 * 900.0) + RandomRanged(0,2)` = `14..16` frames when current mission remains Enter.` (evidence: `0x004D946C`, `0x005B3760`, `rulesmd.ini:[Enter]`)
- `[RESOLVED] OQ-003 - Is `RandomRanged(0,2)` inclusive? -> Yes; the helper samples `0..(max-min)` and returns `min + sample`.` (evidence: `0x0065C7E0`)
- `[RESOLVED] OQ-004 - What writes the next timer? -> `Mission_Dispatch` writes `+0xC8=current frame` and `+0xD0=handler return` after the mission handler returns.` (evidence: `0x005B3060`)
- `[RESOLVED] OQ-005 - Is same-frame B admission guaranteed after A releases? -> No. B must be processed later and have an expired Mission Enter timer.` (evidence: `0x005B3060`; parent report `MISSION_ENTER_CANDOCK_RETRY_SAME_FRAME_ORDER_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-006 - Does a non-ROGER `CAN_DOCK` preserve the path unconditionally? -> No. It preserves only if `+0x418 != 0`; otherwise it sends `BREAK(3)` and queues/commences mission `0` through the mission slot before the timer epilogue.` (evidence: `0x004D92BF..0x004D92E8`, `0x005B35E0`, `0x00744270`)
- `[RESOLVED] OQ-007 - Is this active for standard YR two-miner refinery contention? -> Yes for the Mission Enter timer gate; concrete same-frame outcome remains object-order/timer dependent.` (evidence: INI stock harvester/refinery keys; `0x005B3060`; parent same-frame report)
- `[DEFERRED] OQ-008 - What exact natural replay frame does a particular B retry on?` (category: `needs-runtime-debugger`; reason: requires sampling B's live `+0xC8/+0xD0`, RNG draw, and object index in a concrete replay; next-step-if-pursued: log B at `0x005B3060` and `0x004D9290` around A release)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Mission Enter retry is timer-gated: stock Enter returns `14..16` frames, not every tick. | `0x004D946C`, `0x005B3060`, `0x005B3760`, `rulesmd.ini:[Enter]` | missing/abstracted | `src/sim/miner/miner_dock_sequence.rs::phase_mission_enter`; possible miner timer state | If exact takeover timing is implemented, store a MissionEnter retry timer using base 14 plus deterministic RNG `0..2`, and only run `CAN_DOCK` when due. | `mission_enter_busy_retry_waits_14_to_16_frames_before_next_can_dock` | Do not poll busy/refused `CAN_DOCK` every sim tick. |
| Same-frame takeover depends on B's timer already being due before B's object update. | `0x005B3060`; parent same-frame-order report | partially represented by stable-id order tests; timer not represented | `src/sim/miner/miner_system.rs::tick_miners`; `miner_tests.rs` | Keep order-dependent tests, and add timer-dependent variants if native timer parity is targeted. | B later than A but timer not due: no same-tick claim; B later than A and timer due: same-tick claim allowed. | Do not hardcode same-frame admission just because A released earlier in the tick. |
| Non-ROGER `CAN_DOCK` without entered flag is a break/mission-queue path, not a silent retry preserve. | `0x004D92BF..0x004D92E8`, `0x005B35E0`, `0x00744270` | Rust currently treats waiting/admission through explicit dock queue and phase state | `src/sim/miner/miner_dock.rs`; `src/sim/miner/miner_dock_sequence.rs` | Separate live-refinery refusal from contact-free retry; preserve only when already-entered equivalent is set. | `mission_enter_can_dock_refused_without_entered_flag_breaks_contact_and_does_not_retry_next_tick` | Do not preserve a refused `CAN_DOCK` path unless modeling the `+0x418` already-entered exception. |

## 10. Negative Facts / Do Not Do

- Do not model B's waiting `CAN_DOCK` retry as every tick. The dispatch gate returns early while `elapsed < +0xD0`.
- Do not drop the random jitter. Stock `[Enter]` uses base `14` plus inclusive `0..2`.
- Do not claim same-frame takeover is guaranteed. The waiter must be processed later and be timer-eligible.
- Do not claim same-frame takeover is impossible. If the waiter is later and timer-eligible, its own Mission Enter can run after A releases.
- Do not treat `QueueingCell=4,1` as the accepted `CAN_DOCK` cell; that remains the separate receiver-side `NW+(3,1)` path from sibling reports.

## 11. Remaining Uncertainty

- A concrete replay's B retry frame still needs runtime logging if we care about the exact visible takeover frame: sample B's live object index, `+0xC8`, `+0xD0`, current mission, queued mission, and the RNG draw around A release.
- The exact frequency of the true non-ROGER `CAN_DOCK` branch in ordinary stock two-miner contention is scenario-dependent. The branch behavior and timer handling are verified; whether a given setup reaches it before contact frees needs runtime or a complete setup trace.

## 12. Stale Docs / Follow-up Docs

- `UNIT_MISSION_ENTER_REFINERY_RETRY_QUEUE_LOOP_GHIDRA_REPORT.md`: replace “runtime value of mission timer table entry deferred” with “stock `[Enter] Rate=.016`; `FootClass::Mission_Enter` returns `ftol(.016 * 900.0) + RandomRanged(0,2)` = `14..16` frames while current mission remains Enter.”
- `MISSION_ENTER_CANDOCK_RETRY_SAME_FRAME_ORDER_GHIDRA_REPORT.md`: replace “whatever the mission timer entry produces plus random `0..2` jitter” with “for stock `[Enter]`, the retry delay is `14..16` frames; same-frame admission requires this timer already due.”
- `TWO_CMIN_TAKEOVER_FRAME_ORDER_RETRY_GHIDRA_REPORT.md`: if it says the timer value is runtime-only, narrow that to “the stock base value is known (`14..16`); the replay-specific current `+0xC8/+0xD0` and RNG draw remain runtime-only.”

## Sources

- Ghidra read-only decompile: `FootClass::Mission_Enter @ 0x004D9290`.
- Ghidra read-only decompile: `MissionClass::Mission_Dispatch @ 0x005B3060`.
- Ghidra read-only decompile: `MissionClass::GetMissionTimerEntry @ 0x005B3A00`.
- Ghidra read-only decompile: `MissionClass::Read_INI @ 0x005B3760`.
- Ghidra read-only decompile: `Random__RandomRanged @ 0x0065C7E0`.
- Ghidra read-only decompile: `MissionClass::Queue_Mission @ 0x005B35E0`, `MissionClass::Commence @ 0x005B3570`, `UnitClass::ShouldIdle @ 0x00744270`.
- Raw `gamemd.exe` bytes inspected at `0x004D946C`, `0x005B3700`, `0x005B3A00`.
- INI checked: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`, `rules.ini`, `artmd.ini`, `art.ini`.
- Existing reports referenced: `C:/Users/enok/Documents/ra2-rust-game-docs/miner/MISSION_ENTER_CANDOCK_RETRY_SAME_FRAME_ORDER_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/UNIT_MISSION_ENTER_REFINERY_RETRY_QUEUE_LOOP_GHIDRA_REPORT.md`.

## Status

COMPLETE for the static mission timer and stock `[Enter]` retry cadence. Runtime logging remains needed only for a concrete replay's exact object order, stored timer state, and visible first movement frame.
