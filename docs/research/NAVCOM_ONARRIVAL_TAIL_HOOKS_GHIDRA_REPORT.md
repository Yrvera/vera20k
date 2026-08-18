# NavCom OnArrival Tail Hooks - Ghidra Research Report

**Address(es):** `0x004D82B0` (`FootClass::OnArrival`), `0x004D3710` (`TechnoClass::SetSpeedFraction`), Unit vtable `+0x174 -> 0x00743A50`, Infantry vtable `+0x174 -> 0x0051D0D0`, Aircraft vtable `+0x174 -> 0x0041A590`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `FootClass::OnArrival` tail/side-effect hooks after movement arrival: `+0x687` conditional `vtable+0x174(&DAT_008B3DA8,1,0)`, NavQueue pop ordering, and final `vtable+0x544(0,0)` speed clamp / idle tail.  
**Non-Scope:** full `Mission_Move`, full `DriveLocomotionClass` arrival, NavQueue producers, all `+0x687` producers, attack-move targeting internals, and full Unit/Infantry scatter algorithm.  
**Confidence:** High for hook identity/order and Rust-facing deltas; Medium for `+0x687` producer/liveness because producers were not in scope.  
**Active in YR:** Conditional. `OnArrival` is live for Unit, Infantry, and Aircraft `vtable+0x484` dispatch; individual tail branches depend on runtime queue/flag/mission state.

## 0. Working Notes Gate

- Target question: Resolve `FootClass::OnArrival @ 0x004D82B0` tail/side-effect hooks: vtable `+0x544(0,0)`, `+0x174(&DAT_008B3DA8,1,0)` gated by `+0x687`, and queue-pop/idle ordering needed for Rust handoff.
- Non-goals: Do not redo full `Mission_Move` or DriveLocomotion arrival; do not investigate producers outside this tail slice; do not edit Rust/INI/Ghidra.
- Evidence needed to mark COMPLETE: Decompile plus disassembly/caller/vtable evidence for `OnArrival`, `+0x174`, `+0x544`, and live `+0x484` caller entries.
- Stop conditions: All seeded OnArrival tail-hook questions resolved/deferred; exactly this report plus shared claims file updated.

## 1. Overview

`FootClass::OnArrival` is the common post-arrival helper reached by Unit, Infantry, and Aircraft `vtable+0x484` handlers. Its tail order is: one-per-AI-tick guard, Techno base arrival helper, optional deferred scatter hook, locomotor piggyback assert, NavQueue pop/reissue, attack/infantry close-cell branches, and finally `SetSpeedFraction(0.0)`.

The main correction is that the `+0x687` branch is not an EVA arrival-sound hook. On stock Unit and Infantry vtables, `+0x174` resolves to class scatter functions; the branch is a conditional deferred scatter call with a zero-coordinate argument.

## 2. Class Layout / Key Offsets

| Offset | Type | Meaning in this slice | Evidence | Active in YR |
|---|---:|---|---|---|
| `Foot+0x588` | ptr | NavQueue buffer (`param_1[0x163]`) | `0x004D82B0` decompile | Yes, if queue non-empty |
| `Foot+0x598` | int | NavQueue active count (`param_1[0x166]`) | `0x004D82B0` decompile | Yes, if count > 0 |
| `Foot+0x674` | ptr | active locomotor COM pointer (`param_1[0x19D]`) | `0x004D82B0`, `0x004DA530` decompile | Yes |
| `Foot+0x687` | byte | deferred `vtable+0x174` request flag in this slice | `0x004D82B0`, constructor `0x004D31E0`, save `0x00744640` | Conditional |
| `Foot+0x6B3` | byte | per-AI-tick OnArrival guard | `0x004D82B0`, `0x004DA530` | Yes |
| `Foot+0x578/+0x57C` | double | speed fraction written by `vtable+0x544` | `0x004D3710`; vtable reads | Yes |

## 3. Core Logic

`FootClass::OnArrival @ 0x004D82B0` has this ordered behavior:

1. If `Foot+0x6B3 != 0`, return `0` immediately. Otherwise set `+0x6B3 = 1`. `FootClass::AI @ 0x004DA530` clears `+0x6B3 = 0` near the start of each active AI tick after `TechnoClass::AI_Update`.
2. Call the Techno base arrival helper `0x00709A40` directly. That helper detaches temporal state if present, then runs virtual `+0x4D0` / `+0x430` related checks; it is not dispatched through the subclass vtable.
3. If `Foot+0x687 != 0`, clear it to `0`, then call current object's `vtable+0x174(&DAT_008B3DA8, 1, 0)`.
4. If a locomotor COM pointer exists, query the piggyback interface via `0x0045AF20` and assert on unexpected failure.
5. If `NavQueue.Count > 0`, call `vtable+0x480(first_queue_entry, 0)`, then decrement count and shift remaining entries left by one dword. Return `1`. This skips all later idle/attack/infantry-tail handling.
6. If no queued destination, optional attack/infantry close-cell branches may run. These are non-scope except for order: they are after queue-pop and before final speed clamp.
7. Call `vtable+0x544(0,0)` and return `0`.

Active in YR: Yes for the function and guard/order. Evidence: decompile `0x004D82B0`, disassembly range `0x004D82B0..0x004D84AF`, callers `0x00738970`, `0x0051CBA0`, `0x004176F0`, vtable entries below.

### `vtable+0x174` Identity

| Class vtable evidence | Slot target | Observed behavior | Active in YR |
|---|---|---|---|
| Unit vtable `0x007F5C70 + 0x174 = 0x007F5DE4` reads `0x00743A50` | `UnitClass::Scatter` | scatter candidate/path selection; no EVA/sound call in scoped entry | Yes for Unit arrivals when `+0x687` is set |
| Infantry vtable `0x007EB058 + 0x174 = 0x007EB1CC` reads `0x0051D0D0` | `InfantryClass::Scatter` | infantry scatter candidate/path selection; no EVA/sound call in scoped entry | Yes for Infantry arrivals when `+0x687` is set |
| Foot vtable `0x007E8C94 + 0x174 = 0x007E8E08` reads `0x005F43A0` | no-op base helper | returns immediately | Yes as base vtable fact; not the stock Unit/Infantry concrete path |
| Aircraft vtable `0x007E22A4 + 0x174 = 0x007E2418` reads `0x0041A590` | checks `MissionTimerEntry+9`; if set calls `vtable+0x484(0,1)` | Conditional for Aircraft arrivals |

`DAT_008B3DA8` read as 64 zero bytes in the static image; `OnArrival` uses it as the zero-coordinate argument for the deferred `+0x174` call. Active in YR: Conditional on `+0x687 != 0`; the branch is live code in a live helper, but full producer proof is deferred.

### `vtable+0x544` Identity

For Foot/Unit/Infantry/Aircraft vtables, slot `+0x544` points to `0x004D3710`. `TechnoClass::SetSpeedFraction` clamps the incoming double: `>= 1.0` writes exactly `1.0`, `<= 0.0` writes exactly `0.0`, otherwise writes the incoming double to `Foot+0x578/+0x57C`.

`OnArrival` passes `(0,0)`, so the final tail writes speed fraction `0.0`. Active in YR: Yes on empty-queue arrival fallthrough. Evidence: `read_memory` at vtable slots `0x007E91D8`, `0x007EB59C`, `0x007F61B4` all read `0x004D3710`; decompile `0x004D3710`.

## 4. INI Keys

No INI key is directly read by `OnArrival` for the scoped hooks. Scatter callees and mission-control flags do read type/rules state, but their full algorithms are outside this slice.

## 5. Integration Points

| Integration | Evidence | Order / implication | Active in YR |
|---|---|---|---|
| Unit `vtable+0x484` | Unit vtable `0x007F60F4 -> 0x00738970`; function calls `OnArrival` first | Unit arrival helper wraps `OnArrival`, then EMP/mission/convoy/idle transition logic | Yes |
| Infantry `vtable+0x484` | Infantry vtable `0x007EB4DC -> 0x0051CBA0`; function calls `OnArrival` first | Infantry arrival helper wraps `OnArrival`, then class mission selection | Yes |
| Aircraft `vtable+0x484` | Aircraft vtable `0x007E2728 -> 0x004176F0`; function calls `OnArrival` after some aircraft-specific early gates | Aircraft arrival helper can clear/set destinations after `OnArrival` depending mission state | Yes |
| `FootClass::AI` tick guard reset | `0x004DA530` decompile clears `+0x6B3 = 0`; callers Unit/Infantry/Aircraft AI | `OnArrival` can run once per active AI tick, not once forever | Yes |
| Post-arrival NavQueue pop | `0x004D82B0` decompile | queue pop happens after `+0x687` deferred scatter and before final speed clamp | Yes, if queue count > 0 |
| Convoy queue helper | `0x004DA030`, called by Unit/Infantry wrappers after `OnArrival` | separate from `OnArrival` NavQueue; only runs after wrapper continues | Conditional |

## 6. Current Rust Implementation Status

Rust now has separate `NavigationState` fields (`nav_com`, `suspended_nav_com`, `nav_queue`) in `src/sim/components.rs` and `src/sim/game_entity.rs`, plus `set_destination_internal_cell/null` in `src/sim/movement/navcom.rs`. It still lacks a mission-level `OnArrival` hook matching the binary order.

Current drift points:

- `src/sim/movement/movement_tick.rs::finalize_finished_entities` clears `nav_queue` unconditionally after `set_destination_internal_null`, then clears `movement_target` and sets locomotor idle. Gamemd pops exactly one queued destination and reissues it through `Set_Destination(first,0)`; non-empty queue returns `1` and skips the speed-clamp tail.
- `src/sim/movement/navcom.rs::drive_stop_moving` clamps current speed fraction down to `0.3`, but `OnArrival`'s final `vtable+0x544(0,0)` writes speed fraction exactly `0.0` on empty-queue fallthrough.
- `src/sim/movement/scatter.rs::tick_idle_scatter` exists but is disabled in `src/sim/world/mod.rs`; no deferred `+0x687` arrival-scatter flag or `OnArrival`-ordered scatter hook is modeled.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FootClass::OnArrival @ 0x004D82B0` tail order | verified | decompile plus disassembly `0x004D82B0..0x004D84AF` | none for scoped order |
| `+0x6B3` guard set/reset | verified | `0x004D82B0`, `0x004DA530` | none |
| `+0x687` branch action | verified | `0x004D82B0`; vtable slot reads/decompiles for `+0x174` targets | producers deferred |
| Unit/Infantry `+0x174` identity | verified | reads `0x007F5DE4`, `0x007EB1CC`; decompile `0x00743A50`, `0x0051D0D0` | full scatter algorithm out-of-scope |
| Aircraft `+0x174` identity | verified | read `0x007E2418`; decompile `0x0041A590` | aircraft-specific implications out-of-scope |
| `+0x544(0,0)` speed write | verified | vtable slot reads; decompile `0x004D3710` | none |
| NavQueue pop/shift | verified | `0x004D82B0` | producers outside slot |
| Attack/infantry close-cell branches after queue | touched-not-exhausted | `0x004D82B0` | separate attack-move/infantry-cell investigation |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is `OnArrival` live in stock YR movement arrival? -> Yes, Unit/Infantry/Aircraft `vtable+0x484` entries point to functions that call `0x004D82B0`.` (evidence: vtable reads `0x007F60F4`, `0x007EB4DC`, `0x007E2728`)
- `[RESOLVED] OQ-02 - What does `+0x544(0,0)` do? -> It dispatches to `TechnoClass::SetSpeedFraction @ 0x004D3710`, which clamps/writes speed fraction; `(0,0)` writes exactly `0.0`.` (evidence: vtable reads and decompile `0x004D3710`)
- `[RESOLVED] OQ-03 - Is `+0x174(&DAT_008B3DA8,1,0)` an EVA/sound hook? -> No for stock Unit/Infantry; concrete slot targets are scatter functions, and scoped decompiles show scatter/path logic, not EVA playback.` (evidence: `0x00743A50`, `0x0051D0D0`)
- `[RESOLVED] OQ-04 - What is `DAT_008B3DA8` for this call? -> Static zero coordinate data passed by address to `+0x174`.` (evidence: `read_memory 0x008B3DA8`)
- `[RESOLVED] OQ-05 - Does NavQueue pop before or after the deferred scatter branch? -> After; `+0x687` branch precedes piggyback check and queue count check.` (evidence: `0x004D82B0`)
- `[RESOLVED] OQ-06 - Does non-empty NavQueue still run the final speed clamp? -> No; queue branch calls `Set_Destination(first,0)`, shifts, and returns `1` before `+0x544`.` (evidence: `0x004D82B0`)
- `[RESOLVED] OQ-07 - What clears the re-entry guard? -> `FootClass::AI @ 0x004DA530` clears `+0x6B3` to zero near active tick start.` (evidence: `0x004DA530`)
- `[DEFERRED] OQ-08 - Which live producers set `Foot+0x687`?` (category: out-of-scope; reason: this slot only resolves the OnArrival consumer/hook identity; next-step-if-pursued: writer sweep for `+0x687` and save/load restoration paths)
- `[DEFERRED] OQ-09 - Full attack-move arrival target engagement after empty queue.` (category: out-of-scope; reason: branch is after queue pop and before speed clamp but target helper bodies are separate combat/mission scope; next-step-if-pursued: focused attack-move arrival investigation)
- `[DEFERRED] OQ-10 - Full Unit/Infantry scatter destination mechanics from `+0x174`.` (category: out-of-scope; reason: enough resolved to identify hook; complete scatter has its own reports; next-step-if-pursued: verify scatter docs or run focused scatter audit)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Non-empty OnArrival NavQueue pops exactly one entry, reissues `Set_Destination(first,0)`, shifts remaining entries, returns `1`, and skips final speed clamp. | `0x004D82B0` decompile; live `+0x484` callers | mismatch: `finalize_finished_entities` clears the entire `nav_queue` | `src/sim/movement/movement_tick.rs`; `src/sim/movement/navcom.rs`; `NavigationState` | Add mission-level arrival handling that pops one queue entry and reattaches/reissues destination without clearing all queued waypoints or forcing idle. | Unit arrives at first waypoint with two queued waypoints; after arrival, `nav_com` is second waypoint, queue contains only third, movement continues, speed is not hard-stopped by the empty-tail clamp. Proposed test: `on_arrival_pops_one_navqueue_entry_and_skips_idle_speed_clamp` | Do not model queued waypoints by extending one `MovementTarget` path and clearing `nav_queue` on first path exhaustion. |
| Empty-queue OnArrival fallthrough calls `SetSpeedFraction(0.0)` after optional attack/infantry tail checks. | `0x004D82B0`; `0x004D3710`; vtable reads | mismatch: Drive null path clamps speed to `0.3`; generic finish sets locomotor idle directly | `src/sim/movement/navcom.rs`; `src/sim/movement/movement_tick.rs`; future mission scheduler | Empty-queue arrival should run a binary-shaped OnArrival tail and write speed fraction exactly zero for the owner where the binary does. | Normal Drive unit reaches final cell with empty queue; after mission arrival, speed fraction is exactly zero and NavQueue remains empty. Proposed test: `empty_navqueue_onarrival_sets_speed_fraction_zero` | Do not treat Drive `Stop_Moving`'s locomotor clear as a substitute for the `OnArrival` speed-fraction tail. |
| `Foot+0x687` is a deferred `vtable+0x174(&zero_coord,1,0)` hook; Unit/Infantry concrete targets are scatter functions, not EVA/sound. | `0x004D82B0`; vtable reads `0x007F5DE4`, `0x007EB1CC`; decompile `0x00743A50`, `0x0051D0D0`; `read_memory 0x008B3DA8` | missing: no `+0x687` arrival-scatter flag/hook; idle scatter is disabled and separate | `src/sim/movement/scatter.rs`; future Foot arrival/flag state | If/when `+0x687` producers are implemented, arrival must clear the flag then invoke class scatter before NavQueue pop. | Unit with deferred-arrival-scatter flag and queued destination reaches arrival; scatter hook is processed/cleared before queue pop ordering is observed. Proposed test: `deferred_arrival_scatter_runs_before_navqueue_pop` | Do not implement this as an EVA voice or generic arrival notification. |

### Negative Facts / Do Not Do

- Do not call the `+0x687` branch an EVA arrival sound. Evidence: Unit/Infantry vtable `+0x174` resolves to scatter functions (`0x00743A50`, `0x0051D0D0`), not audio/EVA playback.
- Do not clear `nav_queue` wholesale on arrival. Evidence: `0x004D82B0` decrements the count by one and shifts remaining dwords.
- Do not run final `+0x544(0,0)` after a successful queue pop. Evidence: queue branch returns `1` before the final tail.
- Do not make `+0x6B3` a persistent "already arrived forever" latch. Evidence: `FootClass::AI @ 0x004DA530` clears it each active tick.
- Do not use the current `0.3` Drive stop clamp as OnArrival parity. Evidence: `0x004D3710` with `(0,0)` writes exact double `0.0`.

### Stale Docs / Follow-up Docs

- `docs/research/TECHNO_VTABLE_0x484_DRIVE_PROCESS_ARRIVAL_GHIDRA_REPORT.md`: replace "`+0x687` is an arrived audio flag / play EVA arrival sound" with "`+0x687` is a deferred vtable `+0x174` hook in `FootClass::OnArrival`; for stock Unit and Infantry concrete vtables this is Scatter called with `&DAT_008B3DA8, 1, 0`. Producer/liveness beyond the OnArrival consumer remains a separate question."
- `docs/research/FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md`: replace "Clear-and-replay flag ... likely animation / movement-finish callback" with "Clear-and-dispatch deferred `+0x174` hook; Unit/Infantry resolve to Scatter, Aircraft resolves to an aircraft mission-timer helper. It is not proven to be animation/audio."
- `docs/research/DRIVELOCOMOTION_ARRIVAL_QUEUE_NULL_DESTINATION_GHIDRA_REPORT.md`: replace "arrival audio/callback byte `+0x687`" with "deferred `+0x174` hook byte `+0x687`; concrete Unit/Infantry behavior is Scatter."

## Sources

- Ghidra decompile/read-only: `0x004D82B0`, `0x004D31E0`, `0x004DA530`, `0x004D3710`, `0x00709A40`, `0x004DA030`, `0x00738970`, `0x0051CBA0`, `0x004176F0`, `0x00743A50`, `0x0051D0D0`, `0x0041A590`.
- Ghidra read-only memory/vtable: `0x007F5DE4`, `0x007EB1CC`, `0x007E2418`, `0x007E8E08`, `0x007E91D8`, `0x007EB59C`, `0x007F61B4`, `0x007F60F4`, `0x007EB4DC`, `0x007E2728`, `0x008B3DA8`.
- Existing docs referenced: `TECHNO_VTABLE_0x484_DRIVE_PROCESS_ARRIVAL_GHIDRA_REPORT.md`, `FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md`, `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md`, `SCATTER_ALL_CALLERS_GHIDRA_REPORT.md`.
- Rust scan: `src/sim/components.rs`, `src/sim/game_entity.rs`, `src/sim/movement/navcom.rs`, `src/sim/movement/movement_tick.rs`, `src/sim/movement/movement_commands.rs`, `src/sim/movement/scatter.rs`, `src/sim/world/mod.rs`.
