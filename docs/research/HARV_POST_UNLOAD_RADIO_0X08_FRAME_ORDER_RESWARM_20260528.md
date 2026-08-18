# HARV Post-Unload Radio 0x08 Frame Order - Reswarm Report

**Address(es):** `UnitClass::Mission_Deploy_Building @ 0x0073D630`, `UnitClass::PerCellProcess @ 0x00739EC0`, `TechnoClass::Receive_Radio @ 0x006F4AB0`, `RadioClass::Transmit_Radio_Impl @ 0x0065A970`, `RadioClass::Receive_Radio @ 0x0065A820`, `BuildingClass::Receive_Radio @ 0x0043C2D0`, `FootClass::Mission_Enter @ 0x004D9290`, `MissionClass::Mission_Dispatch @ 0x005B3060`, `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`.
**Investigation Mode:** exhaustive-slice downgraded to partial for exact same-frame runtime ordering.
**Claimed Scope:** static/read-only proof of stock `HARV`/`CMIN` post-unload cleanup ordering for state-4 `BREAK(0x03)`, later `PerCellProcess` `0x08 -> 0x19 -> 0x03`, `+0x418` clearing, contact release, and the conditions under which a waiting second miner can retry in the same live-vector frame.
**Non-Scope:** re-proving the whole unload FSM, `ReleaseDockedHarvester`, non-stock reciprocal `+0x2E4` paths, first rendered locomotor pixel after second-miner admission, slave miner behavior, and runtime debugger observation.
**Confidence:** High for static intra-function order and active stock path gates; Medium for same-frame second-miner condition; Low for the concrete retail frame without runtime trace.
**Active in YR:** Yes for stock `HARV`/`CMIN -> GAREFN/NAREFN`. Stock rules have harvester `Dock=NAREFN,GAREFN`, stock refineries have `DockUnload=yes`, `Refinery=yes`, and `NumberOfDocks=1`.

## 1. Overview

Stock healthy post-unload cleanup has two different cleanup mechanisms that must not be collapsed. The normal state-4 path in `UnitClass::Mission_Deploy_Building` clears `+0x6D1`, assigns Harvest mission `0x0A`, and, if the miner still has a radio contact and the vtable `+0x200` gate passes, sends `BREAK(0x03)` through `Transmit_Radio_ToFirst`.

The `UnitClass::PerCellProcess` cleanup branch at `0x0073A93D` sends radio `0x08` later only if `+0x418` is still set and its mission/cell gates pass. `TechnoClass::Receive_Radio(0x08)` then sends directed `0x19` followed by directed `0x03`. Therefore, when state 4 successfully sends direct `BREAK(0x03)`, it clears the contact and `+0x418` before a later post-unload per-cell `0x08` can be eligible. The exact same-frame admission of a waiting second miner remains runtime-order conditional: it can happen only if the waiter has not yet run in that live-object pass and its Mission Enter timer is due.

## 2. Class Layout / Key Offsets

| Field / message | Owner | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `Unit+0xBC` | Unit | `Mission_Deploy_Building` substate; state 4 is post-empty cleanup | `0x0073D630` decompile | Yes |
| `Unit+0x6D1` | Unit | unload-active/render latch; state 4 clears it before radio cleanup | asm `0x0073E1F6` | Yes |
| `Techno+0x418` | Unit/building | radio/contact-entered byte; set by `0x18`, cleared by `0x19` | asm `0x006F4B72`, `0x006F4BA6` | Yes / Conditional |
| `Radio Contacts[]` | Radio `+0xE4/+0xE8` | one-slot stock refinery contact array; inserted by `0x02`, removed by `0x03` | `0x0065A970`, `0x0065A820`; `NumberOfDocks=1` | Yes |
| `Building+0x57C` | Building | state-4 guard; if live, cleanup returns early | asm `0x0073E1D5..0x0073E1EA` | Conditional; stock usually clear |
| `0x03` | Radio BREAK | removes Contacts[]; Techno case `3` can first send `0x19` if both endpoints have `+0x418` | `0x0065A970`, `0x006F4AB0`, `0x0065A820` | Conditional |
| `0x08` | Techno cleanup | per-cell cleanup message; receiver sends `0x19` then `0x03` | asm `0x0073A939..0x0073A93D`, `0x006F4C34..0x006F4C41` | Conditional |
| `0x19` | Techno clear | clears `+0x418` before propagation | asm `0x006F4BA6..0x006F4BAD` | Conditional |
| Mission `7` | Foot | Mission Enter retry; waiter admission is its own mission dispatch, not refinery promotion | `0x004D9290`, `0x005B3060` | Yes |

## 3. Core Logic

### 3.1 State-4 cleanup order

On the stock zero-link state-4 path, `UnitClass::Mission_Deploy_Building @ 0x0073D630` performs the following order:

1. Re-finds the adjacent refinery and checks `BuildingType+0x16BB` plus `building+0x57C`.
2. If `building+0x57C != 0`, returns immediately at `0x0073E1EA -> 0x0073E5B1`. It does not clear `+0x6D1`, does not send `0x03`, and does not free contacts that frame.
3. Clears `unit+0x6D1 = 0` at `0x0073E1F6`.
4. In the normal branch, calls vtable `+0x1E8` with mission `0x0A` and queued flag `0` at `0x0073E24F..0x0073E254`.
5. Calls vtable `+0x200`; if false, skips radio and mission-queue advance at `0x0073E25E..0x0073E266`.
6. Calls `PathType__Has_Valid_Steps @ 0x0065AE30`; in this radio context the helper is a contact-present scan.
7. If a contact exists, pushes `0x03` and calls vtable `+0x274` at `0x0073E275..0x0073E279`.
8. Calls vtable `+0x1EC` at `0x0073E27F..0x0073E283`, then returns through the mission timer epilogue.

Evidence is stronger than decompiler prose: assembly context shows the exact sequence `+0x57C` guard -> `+0x6D1` clear -> `SetMission(0x0A)` -> `+0x200` -> contact scan -> `PUSH 0x3` -> `+0x274` -> `+0x1EC`.

### 3.2 Direct `BREAK(0x03)` effect

State-4 `0x03` is synchronous. `RadioClass::Transmit_Radio_Impl @ 0x0065A970` first scans sender `Contacts[]` and nulls every slot equal to the target. Assembly at `0x0065A9A8..0x0065A9B8` shows the sender-side slot write to zero before the call to the target receive path at `0x0065A9C9`.

On the receiver side, `BuildingClass::Receive_Radio(3)` calls `GrandOpening`, then delegates to `TechnoClass::Receive_Radio(3)`, then returns `1`. Techno case `3` checks both endpoints' `+0x418`; if both are nonzero, it sends `0x19` before falling to base `RadioClass::Receive_Radio(3)`. Base radio then finds the sender in receiver `Contacts[]`, calls `ObjectClass::Receive_Radio`, nulls that receiver slot, and returns `1`.

Ordering detail: state-4 direct `BREAK(0x03)` clears the miner's contact slot before the refinery receives BREAK. The `+0x418` clear cascade, if both endpoint bytes are set, happens before the refinery base radio slot is nulled.

### 3.3 Per-cell `0x08` cleanup is later and conditional

`UnitClass::PerCellProcess @ 0x00739EC0` has a later cleanup branch gated by `param_1->field_0x418 != 0`, mission/cell conditions, and not being in mission `0x10`. The load-bearing call site is:

```text
0073A939: PUSH 0x8
0073A93B: MOV ECX,EBP
0073A93D: CALL dword ptr [EDX + 0x274]
0073A943: CMP EAX,0x17
```

For a stock refinery, a received `0x08` does not return `0x17`; `BuildingClass::Receive_Radio(0x08)` delegates to Techno cleanup and returns `1` because stock refineries lack `WeaponsFactory`, `UnitRepair`, and `Bunker`.

If the contacted object receives `0x08`, `TechnoClass::Receive_Radio(0x08)` sends `0x19` and then `0x03` through directed vtable `+0x278`:

```text
006F4C30: PUSH 0x19
006F4C34: CALL dword ptr [EDX + 0x278]
006F4C3D: PUSH 0x3
006F4C41: CALL dword ptr [EAX + 0x278]
```

This means `0x08` is a catch-up/cleanup bridge into `0x19` and `0x03`. It is not the normal queue admission path, and it is not the first cleanup mechanism if state-4 `BREAK(0x03)` already succeeded.

### 3.4 Relationship between state 4 and later `PerCellProcess`

Static order closes the main relationship:

- State 4 runs in mission dispatch, not in `PerCellProcess`.
- In the same unit AI tick, mission dispatch runs before locomotor processing and before a movement-caused per-cell callback.
- If state 4 sends direct `BREAK(0x03)`, that synchronous call removes Contacts[] and clears `+0x418` through the `0x19` cascade before a later post-unload `PerCellProcess` cleanup branch could be eligible.
- Therefore Rust should not unconditionally run both state-4 `BREAK(0x03)` and a later `0x08` cleanup for the same successfully-cleared contact. The `0x08` path is for lingering `+0x418`/contact states that survive into a later per-cell pass.

The static slice also proves one important negative: if state 4 is blocked by `building+0x57C`, neither `+0x6D1` nor contact state is cleared on that frame. A waiting miner should not be admitted from cargo-empty detection alone while that guard is live.

### 3.5 Waiting second miner same-frame condition

There is no automatic refinery-side promotion callback in state 4. A waiting second miner must run its own `FootClass::Mission_Enter @ 0x004D9290` and send `CAN_DOCK(0x0E)`.

`MissionClass::Mission_Dispatch @ 0x005B3060` is timer-gated: mission `7` calls vtable `+0x240`, stores `g_CurrentFrameCounter` into `+0xC8`, and stores the returned delay into `+0xD0`. `FootClass::Mission_Enter` sends `0x0E` at `0x004D92B2..0x004D92B9`; if the return is not `1`, it preserves the path only when `+0x418 != 0`, otherwise it sends `BREAK(0x03)` at `0x004D92CE..0x004D92D4` and clears movement.

`LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` iterates the live object vector and calls each object's vtable `+0x5C` in order. Therefore a second miner can retry/admit on the same frame as the first miner's state-4 release only when all of these are true:

1. first miner state 4 has passed `building+0x57C` and sent direct `BREAK(0x03)` or otherwise freed the refinery contact before the second miner runs;
2. the second miner has not yet had its live-vector AI call for that frame;
3. the second miner's Mission Enter timer is due in `MissionClass::Mission_Dispatch`;
4. the second miner's own `CAN_DOCK(0x0E)` path observes the freed contact slot.

If the waiter already ran earlier in the live-vector pass, or its mission timer is not due, static evidence says the earliest retry is its next eligible mission-dispatch frame. The exact retail frame for a concrete two-miner setup requires runtime tracing of object order and timer fields.

## 4. INI Keys

| Key | Stock YR value | Effect in this slice | Evidence |
|---|---|---|---|
| `[CMIN] Dock` | `NAREFN,GAREFN` | stock Allied miner targets stock refineries | `ini/rulesmd.ini:7361` |
| `[CMIN] Harvester` | `yes` | uses harvester/refinery mission paths | `ini/rulesmd.ini:7364` |
| `[HARV] Dock` | `NAREFN,GAREFN` | stock Soviet miner targets stock refineries | `ini/rulesmd.ini:8225` |
| `[HARV] Harvester` | `yes` | uses harvester/refinery mission paths | `ini/rulesmd.ini:8228` |
| `[GAREFN] DockUnload` / `Refinery` / `NumberOfDocks` | `yes` / `yes` / `1` | stock one-contact DockUnload receiver | `ini/rulesmd.ini:11726..11729` |
| `[NAREFN] DockUnload` / `Refinery` / `NumberOfDocks` | `yes` / `yes` / `1` | stock one-contact DockUnload receiver | `ini/rulesmd.ini:12519..12521` |
| `QueueingCell` | `4,1` for stock refineries | waiting/staging data; not the state-4 cleanup trigger and not `0x08` admission | `artmd.ini`, prior accepted-cell reports |

No INI key directly maps to `Techno+0x418`; it is runtime radio state.

## 5. Integration Points

| Function | Role | Verified details |
|---|---|---|
| `UnitClass::Mission_Deploy_Building @ 0x0073D630` | state-4 cleanup owner | `+0x57C` wait, `+0x6D1` clear, Harvest assignment, conditional `BREAK(3)`, mission queue |
| `UnitClass::PerCellProcess @ 0x00739EC0` | later per-cell cleanup owner | can send `0x08` if `+0x418` persists and mission/cell gates pass |
| `TechnoClass::Receive_Radio @ 0x006F4AB0` | `+0x418` and `0x08` bridge | `0x18` sets, `0x19` clears, `0x08` sends `0x19` then `0x03`, `0x03` can send `0x19` first |
| `RadioClass::Transmit_Radio_Impl @ 0x0065A970` | sender-side radio mutation | `0x03` removes sender Contacts[] before forwarding |
| `RadioClass::Receive_Radio @ 0x0065A820` | receiver-side radio mutation | `0x03` removes receiver Contacts[] after Object side effects |
| `BuildingClass::Receive_Radio @ 0x0043C2D0` | refinery receiver | stock `0x08` is cleanup/ROGER, not queue `0x17`; stock `0x0E` admits only through contact retry |
| `FootClass::Mission_Enter @ 0x004D9290` | waiter retry owner | sends `0x0E`; no FIFO promotion; preserves non-ROGER only with `+0x418` |
| `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` | same-frame boundary | live-vector order determines whether waiter can run after releaser in the same frame |

## 6. Current Rust Implementation Status

Rust surfaces scanned only:

- `src/sim/miner/miner_dock.rs` models `contacts`, `waiting_retry_queue`, `contact_entered`, and `on_pad`.
- `release_contact` clears `contact_entered` and the contact slot but intentionally does not clear `on_pad`.
- `src/sim/miner/miner_dock_sequence.rs::phase_departing` releases `on_pad`, then releases contact, clears unload/display state, and hands back to search.
- Current tests cover pieces such as no direct waiter promotion on `release_contact`, queued miner entry after contact and pad release, stock no `Force_Track(0x47)`, and empty-slot gate releasing on the next state-4 handoff.

Rust drift risks:

- Rust has a deterministic `waiting_retry_queue`; gamemd static evidence proves retry through the waiter's own Mission Enter, not a refinery-owned FIFO promotion.
- Rust currently gates waiter entry on both contact and `on_pad`; static gamemd evidence proves contact release before verified physical movement, but does not prove an equivalent persistent pad-occupancy gate.
- Rust `phase_departing` clears contact and pad in one Rust phase. Binary state 4 clears contact via radio before any proven move-off-pad command, but exact same-frame waiter behavior still needs runtime ordering before changing Rust.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Stock state-4 `+0x57C` wait before cleanup | verified | asm `0x0073E1D5..0x0073E1EA` | none |
| State-4 `+0x6D1` clear before radio | verified | asm `0x0073E1F6`, `0x0073E275` | none |
| State-4 Harvest mission assignment before radio | verified | asm `0x0073E24F..0x0073E254`, `0x0073E275` | none |
| State-4 direct `BREAK(0x03)` | verified | asm `0x0073E26A..0x0073E279` | vtable `+0x200` semantic remains a branch gate |
| Sender contact removal on `0x03` | verified | decompile plus asm `0x0065A9A8..0x0065A9B8` | none |
| Receiver contact removal on `0x03` | verified | `0x0065A820` decompile | none |
| `+0x418` clear via `0x19` during `0x03`/`0x08` cleanup | verified | `0x006F4AB0`; asm `0x006F4BA6`, `0x006F4C34` | none |
| PerCellProcess `0x08` call site | verified | `0x00739EC0` decompile; asm `0x0073A939..0x0073A943` | exact replay frequency |
| Stock refinery `0x08` non-queue result | verified | `0x0043C2D0`; stock INI flags | none |
| Same-frame waiter admission condition | touched-not-exhausted | `0x0055AFB0`, `0x005B3060`, `0x004D9290` | runtime object order/timer observation |
| First rendered waiter movement | deferred | static code only | runtime coordinate trace |
| Current Rust contact model | touched-not-exhausted | `src/sim/miner/*.rs` scan | no tests run in this research slot |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does stock state 4 send `0x08`? -> No. State 4 conditionally sends `BREAK(0x03)`, not `0x08`.` (evidence: `0x0073E275..0x0073E279`)
- `[RESOLVED] OQ-02 - What runs first, state-4 radio cleanup or later per-cell cleanup? -> For the same unit AI tick, mission dispatch/state 4 runs before locomotor/per-cell processing.` (evidence: `UNITCLASS_PERCELLPROCESS_CALLER_TICK_ORDER_GHIDRA_REPORT.md`; `0x005B3060`; `0x00739EC0`)
- `[RESOLVED] OQ-03 - Does successful state-4 `0x03` make later `0x08` unnecessary for the same contact? -> Yes. `0x03` removes Contacts[] and can clear `+0x418` via `0x19` before later per-cell eligibility.` (evidence: `0x0065A970`, `0x006F4AB0`, `0x0065A820`)
- `[RESOLVED] OQ-04 - When can `0x08` still matter? -> When `+0x418`/contact state survives into `PerCellProcess` after mission/cell gates pass; then `0x08` bridges to `0x19` and `0x03`.` (evidence: `0x0073A93D`, `0x006F4C34..0x006F4C41`)
- `[RESOLVED] OQ-05 - Does state 4 clear contacts if `building+0x57C` is live? -> No. It returns before `+0x6D1` clear and before radio cleanup.` (evidence: `0x0073E1D5..0x0073E1EA`)
- `[RESOLVED] OQ-06 - Does stock refinery `0x08` admit a waiting miner? -> No. Stock refinery `0x08` returns ROGER after cleanup; queue `0x17` is factory/repair/bunker-gated.` (evidence: `0x0043C2D0`; `rulesmd.ini`)
- `[RESOLVED] OQ-07 - Does the first miner directly promote the second miner on release? -> No. The second miner must retry through its own Mission Enter dispatch.` (evidence: `0x0073D630`; `0x004D9290`)
- `[RESOLVED] OQ-08 - Can second-miner admission be same-frame? -> Conditional: only if the waiter is later in the live-object pass and its Mission Enter timer is due after the first miner releases contact.` (evidence: `0x0055AFB0`, `0x005B3060`, `0x004D9290`)
- `[RESOLVED] OQ-09 - Does Mission Enter own a persistent FIFO? -> No evidence in `0x004D9290`; it reads contact slot 0/fallback target and sends `0x0E`.` (evidence: `0x004D92B2..0x004D92D4`; prior retry report)
- `[DEFERRED] OQ-10 - Concrete stock replay frame for second-miner retry/admission` (category: `needs-runtime-debugger`; reason: static code gives the condition, not concrete live-vector order and timer state; next-step-if-pursued: runtime trace below)
- `[DEFERRED] OQ-11 - First rendered locomotor displacement after admitted retry` (category: `needs-runtime-debugger`; reason: static `Set_Destination` command does not prove first pixel/cell delta; next-step-if-pursued: log coordinates before/after locomotor Process)
- `[DEFERRED] OQ-12 - Full vtable `+0x200` identity in state 4` (category: `requires-different-system-context`; reason: branch order is verified, but method identity was not drained here; next-step-if-pursued: resolve UnitClass vtable `+0x200` and stock edge cases)

### Runtime Trace Plan

Trace a stock map with two full `CMIN` or `HARV` units targeting one stock `GAREFN`/`NAREFN`.

Watch per frame:

- First miner: object vector index, current mission, `+0xBC`, `+0x6D1`, `+0x418`, `Contacts[0]`, `+0xC8/+0xD0`, current cell, movement destination.
- Refinery: `Contacts[0]`, `+0x418`, `+0x57C`, `NumberOfDocks`-sized contact capacity.
- Second miner: object vector index, current mission, `+0x418`, `Contacts[0]`, `+0xC8/+0xD0`, destination, current cell.
- Radio events: first miner state-4 `0x03`, any later per-cell `0x08`, `0x19` propagation, second miner `0x0E`, `0x12`, `0x18`, `0x16`, `0x15`.
- Outcome labels: same-frame admission only if second miner's `Mission_Enter` runs after first miner's state-4 release in the same `g_CurrentFrameCounter` and sends successful `0x0E`.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| State 4 sends direct `BREAK(0x03)` after `+0x6D1` clear and Harvest assignment; it does not send `0x08`. | `0x0073E1F6`, `0x0073E24F..0x0073E279` | broad match, but cleanup abstraction hides radio ordering | `src/sim/miner/miner_dock_sequence.rs::phase_departing` | Preserve order: unload display/latch clear, Harvest/search handoff, then contact cleanup equivalent. | First miner finishes stock unload; no post-state4 `0x08` event is required when direct break clears contact. Proposed test: `war_miner_state4_break_preempts_percell_0x08_cleanup` | Do not model both `0x03` and later `0x08` as mandatory duplicate cleanups for the same cleared contact. |
| `0x08` is a later per-cell cleanup bridge to `0x19` then `0x03`, not stock refinery queue admission. | `0x0073A93D`, `0x006F4C34..0x006F4C41`, `0x0043C2D0` | Rust has contact release but no generic radio `0x08` surface | `src/sim/miner/miner_dock.rs`, future radio abstraction | If lingering contact-entered state survives, cleanup should clear `+0x418`-equivalent before/with contact release. | Lingering `contact_entered` plus contact cleanup clears entered and contact without admitting a waiter. Proposed test: `percell_radio_0x08_clears_entered_then_breaks_contact` | Do not implement `0x08 -> 0x17` for stock refineries. |
| State-4 `building+0x57C` guard blocks all cleanup before contact release. | `0x0073E1D5..0x0073E1EA` | modded/anim slot wait unchecked | `src/sim/miner/miner_dock_sequence.rs`, building anim slot state | Delay contact and waiter release while the equivalent slot-8 anim pointer is live. | Refinery with live slot-8 anim keeps first miner's contact until anim clears. Proposed test: `war_miner_state4_waits_slot8_before_contact_release` | Do not free the dock at cargo-empty detection or before the state-4 wait guard passes. |
| A waiting second miner is admitted only by its own Mission Enter retry after contact release; same-frame is object-order/timer conditional. | `0x0055AFB0`, `0x005B3060`, `0x004D9290` | Rust deterministic wait queue may over-specify binary FIFO | `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_tests.rs` | Treat Rust queue as retry scheduling, not refinery-side promotion; order by sim update eligibility. | Two miners with first lower vs higher stable/update order should expose whether same-tick admission differs after runtime trace. Proposed test: `two_miner_retry_after_release_is_object_order_and_timer_gated` | Do not hardcode universal same-tick or release+1 admission. |
| Direct `BREAK(0x03)` clears sender contact first, can clear `+0x418`, then clears receiver contact. | `0x0065A9A8..0x0065A9C9`, `0x006F4B8D..0x006F4BAD`, `0x0065A820` | Rust `release_contact` clears contact and entered together; order not byte-modeled | `src/sim/miner/miner_dock.rs::release_contact`, future generic radio | If byte-level radio state is modeled, preserve sender-clear-before-receiver-clear ordering. | Contact hash/trace shows sender slot is null before receiver slot during break. Proposed test: `radio_break_sender_contact_clears_before_receiver_contact` | Do not clear receiver first if implementing exact radio state bytes. |

## Negative Facts / Do Not Do

- Do not say state 4 sends `0x08`; it sends `0x03` directly when the contact-present branch fires. Evidence: `0x0073E275..0x0073E279`.
- Do not treat `0x08` as stock refinery queue admission. Evidence: `0x0043C2D0`; stock `GAREFN/NAREFN` lack `WeaponsFactory`, `UnitRepair`, and `Bunker`.
- Do not run `0x08 -> 0x19 -> 0x03` after a successful state-4 `0x03` already cleared `+0x418` and Contacts[] for the same pair. Evidence: `0x0065A970`, `0x006F4AB0`.
- Do not release a waiting miner while `building+0x57C` keeps state 4 in its early return. Evidence: `0x0073E1DF..0x0073E1EA`.
- Do not implement a refinery-owned instant promotion callback; the waiter retries through Mission Enter. Evidence: `0x004D9290`, `0x005B3060`.
- Do not treat Rust `waiting_retry_queue` as proven gamemd storage. It is a deterministic abstraction over Mission Enter retry unless a future trace proves otherwise.
- Do not claim a fixed same-frame or next-frame second-miner outcome from static evidence alone.

## Remaining Uncertainty

- Exact concrete same-frame vs next-frame waiter admission for a retail replay requires runtime object-vector order and Mission Enter timer observation.
- First rendered displacement after the waiter's accepted-cell `0x12` command requires coordinate logging across locomotor Process.
- UnitClass vtable `+0x200` in state 4 remains ordered but not semantically named in this slot.

## Sources

- Ghidra read-only decompile: `0x0073D630`, `0x00739EC0`, `0x006F4AB0`, `0x0065A970`, `0x0065A820`, `0x0043C2D0`, `0x004D9290`, `0x005B3060`, `0x0055AFB0`.
- Ghidra assembly contexts: `0x0073E1D5`, `0x0073E1F6`, `0x0073E24F`, `0x0073E26A`, `0x0073E275`, `0x0073E27F`, `0x0073A936`, `0x0073A93D`, `0x006F4B72`, `0x006F4BA6`, `0x006F4C34`, `0x006F4C41`, `0x0065A9A8`, `0x0065A9C9`, `0x004D92B2`, `0x004D92C4`, `0x004D92CE`.
- Prior docs: `docs/research/miner/HARV_POST_UNLOAD_EXIT_PATH_GHIDRA_REPORT.md`, `docs/research/miner/STOCK_REFINERY_RADIO_0X08_GLOBAL_SENDERS_GHIDRA_REPORT.md`, `docs/research/miner/TWO_MINER_ONE_REFINERY_ZERO_LINK_HANDOFF_TIMING_GHIDRA_REPORT.md`, `docs/research/UNITCLASS_0X418_DOCK_FLAG_LIFECYCLE_AND_CONSUMERS_GHIDRA_REPORT.md`, `docs/research/BUILDING_RADIO_0X18_CONTACT_LIFECYCLE_RESWARM_20260528.md`, `docs/research/UNITCLASS_PERCELLPROCESS_CALLER_TICK_ORDER_GHIDRA_REPORT.md`, `docs/research/UNIT_MISSION_ENTER_REFINERY_RETRY_QUEUE_LOOP_GHIDRA_REPORT.md`, `docs/research/miner/TWO_CMIN_ONE_REFINERY_TAKEOVER_TIMING_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust scanned: `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_tests.rs`.

**Status:** PARTIAL for exact same-frame runtime outcome; COMPLETE for static ordering of state-4 `0x03`, later per-cell `0x08`, `0x19`, contact release, and retry ownership.
