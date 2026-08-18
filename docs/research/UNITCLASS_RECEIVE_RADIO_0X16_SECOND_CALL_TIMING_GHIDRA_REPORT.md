# UnitClass Receive_Radio 0x16 Second-Call Timing - Ghidra Research Report

**Address(es):** `UnitClass::Receive_Radio @ 0x00737430`, case `0x16 @ 0x007376AD`; `BuildingClass::Receive_Radio @ 0x0043C2D0`; `FootClass::Receive_Radio @ 0x004D8FB0`; `TechnoClass::Receive_Radio @ 0x006F4AB0`; `DriveLocomotionClass::Do_Turn @ 0x004B0EF0`; `RateTimer::Set @ 0x004C9220`; `RateTimer::Current @ 0x004C93D0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** repeat / already-synchronized `UnitClass::Receive_Radio(0x16)` behavior after the first call sets the `+0x388` timer, and whether this path can send `0x15` without physical `GetDockCoord` arrival.
**Non-Scope:** full tick-order winner between the `0x16` cascade and `UnitClass::PerCellProcess`, full `Mission_Enter`, full locomotor path completion, and non-refinery docking.
**Confidence:** High for the `0x16` branch, predicates, return values, and no-GetDockCoord negative facts; Medium for caller cadence because exact tick-order is covered by sibling slots.
**Active in YR:** Yes for stock `[CMIN]`/`[HARV]` into `[GAREFN]`/`[NAREFN]` through `DockUnload=yes`.

## 0. Scope Contract

**Target question:** Does a second or later `UnitClass::Receive_Radio(0x16)` occur after the first `0x16` only sets the `+0x388` timer through locomotor `+0x4C`, and can that later call send `0x15` before physical `GetDockCoord` arrival?

**Non-goals:** Do not redo the accepted `0x12` target, stock `GetDockCoord`, full `PerCellProcess`, or full `Mission_Enter` timing proofs except where needed to identify the only sender-side source of another `0x16`.

**Evidence needed to mark COMPLETE:** Decompile of `UnitClass::Receive_Radio(0x16)`, decompile of base `FootClass`/`TechnoClass` `0x16` side effects, decompile of the refinery sender that can issue another `0x16`, decompile of timer helpers, and current Rust handoff mapping.

**Stop conditions:** Stop once exact `0x16` predicates and returns are proven, second-call source is bounded to the stock refinery `0x0E` sender path, and global first-winner tick order is handed to sibling slots instead of inferred.

## 1. Overview

`UnitClass::Receive_Radio(0x16)` has two different behaviors depending on the unit's `+0x388` rate timer. The first non-chrono call normally sets the timer target to `0x4000` through locomotor vtable `+0x4C` and returns `1` immediately. A later call, or any call where `RateTimer::Current(+0x388)` already returns `0x4000`, skips that early return and may send `0x15` to the current destination building.

The `0x16` branch does not call `GetDockCoord`, does not compare cells, and does not move the unit. Therefore the `0x16` cascade can issue `0x15` without proving the unit is physically on the stock refinery `GetDockCoord` cell, provided a later/already-synchronized `0x16` is delivered and the unit is idle, has a building destination, and is still on mission `7`.

## 2. Class Layout / Key Offsets

| Offset / slot | Owner | Meaning in this slice | Active in YR | Evidence |
|---|---|---|---|---|
| `+0x388` | Unit/Foot | Primary facing `RateTimer`; `0x16` reads current value and `Do_Turn` sets target `0x4000` | Yes | `0x007376CE..0x007376D9`, `0x004B0EF0`, `0x004C9220`, `0x004C93D0` |
| `+0x6AF` | Unit | chrono/teleporting flag; nonzero skips the timer-set branch and goes straight to cascade checks | Conditional | `0x007376BF..0x007376C7` |
| `+0x674` / decompiler `+0x19D` dword index | Unit | `ILocomotion*`; vtable `+0x4C` sets turn timer, vtable `+0x10` checks `Is_Moving` | Yes | `0x007376E0..0x00737709`, `0x0073771B..0x0073773D` |
| `+0x418` / decompiler `param_1[0x106]` byte | Foot/Techno | has destination / dock-contact state gate before `0x15` cascade | Yes | `0x0073773F..0x0073774A`; Techno `0x18/0x19` at `0x006F4AB0` |
| vtable `+0x184` | Unit | current mission getter; must return `7` before `0x16` sends `0x15` | Yes | `0x0073775B..0x00737771` |
| vtable `+0x278` | Unit/Building | direct `Transmit_Radio(msg, target)`; `0x16` uses it to send `0x15` | Yes | `0x00737773..0x0073777A` |
| BuildingType `+0x16B3` | Building type | `DockUnload=yes`; enables stock refinery sender path | Yes for GAREFN/NAREFN | `0x0043CA1B..0x0043CA2F`, `ini/rulesmd.ini` |

## 3. Core Logic

### 3.1 First `0x16` call when timer is not already `0x4000`

Active in YR: Yes.

Verified flow from `UnitClass::Receive_Radio @ 0x00737430`, case entry `0x007376AD`:

1. Calls `FootClass::Receive_Radio(sender, 0x16, payload)` first. Evidence: decompile `0x00737430`, call before the `+0x6AF` read.
2. `FootClass::Receive_Radio @ 0x004D8FB0` has no direct `0x16` case, so it falls to `TechnoClass::Receive_Radio @ 0x006F4AB0`.
3. `TechnoClass::Receive_Radio` groups cases `7`, `9`, and `0x16`, sends `0x18` back to the sender through vtable `+0x278`, then calls `RadioClass::Receive_Radio`, and returns `1`.
4. `UnitClass` checks byte `+0x6AF`. If zero, it calls `RateTimer::Current` on `+0x388` and compares the returned low word to `0x4000`.
5. If the current timer value is not `0x4000`, it asserts the locomotor pointer if null, calls `ILocomotion+0x4C(this_loco, 0x4000)`, and returns `1` immediately.

Material consequence: the first ordinary `0x16` does not reach `Is_Moving`, `GetDestination`, destination class, mission, or `0x15` send checks when `RateTimer::Current(+0x388) != 0x4000`.

### 3.2 Later / already-synchronized `0x16` call

Active in YR: Yes when the stock refinery sender emits another `0x16` while the unit's `+0x388` current value is already `0x4000`, or when the chrono flag skips the timer branch.

Verified cascade predicates after the timer branch is skipped:

1. Locomotor pointer must be non-null or assert.
2. `ILocomotion+0x10 / Is_Moving()` must return false.
3. `FootClass::GetDestination(0)` is called; decompiler then gates on `(char)param_1[0x106] != 0`.
4. Destination pointer must be non-null.
5. Destination `WhatAmI()` via vtable `+0x2C` must return `6` (building).
6. The unit's own mission via vtable `+0x184` must return `7`.
7. Then the unit calls vtable `+0x278` with message `0x15` and the destination building pointer.
8. The return value from transmitting `0x15` is ignored.
9. The `0x16` receiver returns `1` regardless of whether it sent `0x15`.

Important correction: the `0x16` branch does **not** check the destination building's mission. Older summaries that say "building mission == 7" are stale. The decompile shows the mission check is on `param_1` (the receiving unit), not `piVar5` (the destination building).

### 3.3 Where another `0x16` can come from

Active in YR: Yes for stock refinery admission.

`UnitClass::Receive_Radio(0x16)` does not schedule, queue, recurse, or self-send a future `0x16`. A second or later `0x16` must be delivered by an external sender.

For stock refinery docking, the bounded sender is `BuildingClass::Receive_Radio(0x0E) @ 0x0043C2D0`:

1. Case `0x0E` calls `TechnoClass::Receive_Radio`.
2. It requires power and the normal admission/contact checks.
3. In the `DockUnload=yes` / `Weeder=yes` branch it sends `0x13`.
4. It computes a `CellClass*` payload at building cell `+(3,1)` and sends `0x12`.
5. It only continues if the `0x12` reply is exactly `0x14` ("already at accepted cell").
6. It sends `0x18` to the unit.
7. It sends `0x16` to the unit.

Therefore a later `0x16` can occur on any later stock `0x0E` admission pass that again reaches this sender path and again gets `0x14` from `0x12`. This report does not prove the global tick pass that produces that later `0x0E`; sibling slots cover `Mission_Enter`, object update order, and locomotor arrival timing.

### 3.4 Can `0x16` send `0x15` before physical `GetDockCoord` arrival?

Active in YR: Conditional. The `0x16` branch can do it if a later/already-synchronized `0x16` is delivered before physical `GetDockCoord` arrival.

Yes as a mechanism: `UnitClass::Receive_Radio(0x16)` contains no `GetDockCoord` call, no `vtable+0xA8` call on the building, no lepton-to-cell comparison, and no check for the stock refinery pad cell. Its only movement-related gate is `Is_Moving()==false`.

This is distinct from `UnitClass::PerCellProcess @ 0x00739EC0`, which separately compares current unit cell to destination building `GetDockCoord` and sends `0x15` only on equality. The two `0x15` sources have different gates:

| Source | Gate before `0x15` | Evidence |
|---|---|---|
| `UnitClass::Receive_Radio(0x16)` | timer already `0x4000` or chrono skip; not moving; has building destination; unit mission `7` | `0x007376AD..0x00737783` |
| `UnitClass::PerCellProcess` | current unit cell equals destination building `GetDockCoord` cell | `0x00739EC0` decompile; prior slot-4 report |

## 4. INI Keys

No INI key controls the `0x16` receiver branch directly. Standard YR activation comes from data that reaches the stock refinery admission path:

| Key | Stock YR value | Effect in this slice | Active in YR | Evidence |
|---|---|---|---|---|
| `DockUnload=yes` | `[GAREFN]`, `[NAREFN]` | sets BuildingType `+0x16B3`; enables the standard refinery sender path that sends `0x18` and `0x16` after `0x12 == 0x14` | Yes | `ini/rulesmd.ini:11726`, `12519`; `0x0043CA1B..0x0043CA2F` |
| `Refinery=yes` | `[GAREFN]`, `[NAREFN]` | stock refinery identity; relevant to other dock reports, not read by `0x16` receiver | Yes | `ini/rulesmd.ini:11727`, `12520` |
| `QueueingCell=4,1` | `[GAREFN]`, `[NAREFN]` art | wait/fallback reference point; not read by `0x16` receiver | Yes for queueing, No for this branch | `ini/artmd.ini:1716`, `1773`; no read in `0x007376AD..0x00737783` |

## 5. Integration Points

- `BuildingClass::Receive_Radio(0x0E)` is the stock sender that can emit first and later `0x16` calls after `0x12` returns `0x14`.
- `FootClass::Receive_Radio(0x16)` is only a base-chain hop; it has no direct `0x16` case.
- `TechnoClass::Receive_Radio(0x16)` sends `0x18` back to the sender before `UnitClass` performs timer/cascade logic.
- `DriveLocomotionClass::Do_Turn @ 0x004B0EF0` calls `RateTimer::Set` for linked unit `+0x388`; it is not movement.
- `RateTimer::Current @ 0x004C93D0` can return `0x4000` after elapsed frames catch the interpolation up to target, allowing the later `0x16` cascade to run.
- `BuildingClass::Receive_Radio(0x15)` for stock `DockUnload=yes` queues sender mission `0x10` and returns `1`; it is the receiver of the `0x16` cascade's `0x15`.

## 6. Current Rust Implementation Status

Current Rust uses a miner-specific state machine rather than a generic radio layer:

| Rust surface | Current behavior observed | Delta risk |
|---|---|---|
| `src/sim/miner/miner_dock_sequence.rs::phase_mission_enter` | moves to accepted cell, then marks contact entered and jumps to `Linked` once already there and not moving | Missing explicit first `0x16` timer-set-vs-later-cascade split |
| `src/sim/miner/miner_dock_sequence.rs::phase_awaiting_accepted_cell` | defers handoff until movement completes and another `MissionEnter` pass runs | Matches `0x12 == 1` vs `0x14` distinction, but not full `0x16` repeat predicate |
| `src/sim/miner/miner_dock_sequence.rs::phase_linked` | snapshots miner to pad/GetDockCoord cell, links pad, starts pivot/sound | Risk: begins the `0x15`-like handoff from accepted-cell admission without modeling whether a later `0x16` or `PerCellProcess` actually won |
| `refinery_can_dock_queue_cell` / `refinery_pad_cell` | split accepted NW+(3,1) from pad/GetDockCoord NW+(2,1) | Naming still needs to prevent accepted vs GetDockCoord confusion |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `UnitClass::Receive_Radio(0x16)` first-call timer branch | verified | decompile `0x00737430`, case `0x16`; prior assembly `0x007376AD..0x0073770F` | none |
| `UnitClass::Receive_Radio(0x16)` later cascade branch | verified | decompile `0x00737430`, case `0x16`; prior assembly `0x0073771B..0x00737783` | none |
| `FootClass::Receive_Radio` base side effect for `0x16` | verified | decompile `0x004D8FB0` falls through to Techno for no `0x16` case | none |
| `TechnoClass::Receive_Radio` `0x16` side effect | verified | decompile `0x006F4AB0`, cases `7/9/0x16` send `0x18`, return `1` | none |
| `DriveLocomotionClass::Do_Turn` / `RateTimer::Set` | verified | decompile `0x004B0EF0`, `0x004C9220` | exact visual facing duration depends on RateTimer parameters, outside this report |
| `BuildingClass::Receive_Radio(0x0E)` as later `0x16` sender | verified for sender mechanics | decompile `0x0043C2D0` | exact tick-order cadence deferred to sibling slots |
| Global winner: `0x16` cascade vs `PerCellProcess` | deferred | sibling swarm scope | requires tick/update order and locomotor completion proof |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Does the first ordinary 0x16 reach the 0x15 cascade? -> No, if +0x6AF is clear and RateTimer::Current(+0x388) != 0x4000, it calls locomotor +0x4C(0x4000) and returns 1 immediately.` (evidence: `0x007376BF..0x0073770F`)
- `[RESOLVED] OQ-2 - Does 0x16 self-schedule a second 0x16? -> No self-send, queue, mission write, or timer callback exists in the branch; another sender must deliver it.` (evidence: `0x007376AD..0x00737783`)
- `[RESOLVED] OQ-3 - What sender can deliver a later stock refinery 0x16? -> `BuildingClass::Receive_Radio(0x0E)` sends 0x16 after another pass gets `0x12 == 0x14` from accepted cell NW+(3,1).` (evidence: `0x0043C2D0`)
- `[RESOLVED] OQ-4 - What exact predicates gate the later 0x15 cascade? -> Timer branch skipped, `Is_Moving()==false`, destination flag set, destination non-null, destination WhatAmI==6, unit mission==7.` (evidence: `0x0073771B..0x0073777A`)
- `[RESOLVED] OQ-5 - Does 0x16 check destination building mission == 7? -> No. The mission check is on the receiver unit (`param_1`), not destination `piVar5`.` (evidence: decompile `0x00737430` case `0x16`)
- `[RESOLVED] OQ-6 - Does 0x16 call GetDockCoord or compare current cell to pad cell? -> No; no vtable +0xA8 call, coordinate conversion, or cell comparison appears in case 0x16.` (evidence: `0x007376AD..0x00737783`)
- `[RESOLVED] OQ-7 - What base side effect occurs before UnitClass timer logic? -> Foot falls through to Techno; Techno cases 7/9/0x16 send 0x18 to sender and return 1.` (evidence: `0x004D8FB0`, `0x006F4AB0`)
- `[DEFERRED] OQ-8 - In stock tick order, does a later 0x16 arrive before the PerCellProcess GetDockCoord gate fires?` (category: requires-different-system-context; reason: this slot is the UnitClass `0x16` receiver slice; requires sibling Mission_Enter/tick-order/locomotor reports; next-step-if-pursued: reconcile slots 1, 3, and 5)
- `[DEFERRED] OQ-9 - Exact RateTimer frame count from first Set(0x4000) to Current()==0x4000 for every Rot value?` (category: bounded-cost-too-high; reason: this report proves the branch predicate, not full visual turn timing; next-step-if-pursued: RateTimer/facing-duration report using unit type Rot)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| First ordinary `0x16` can only set `+0x388` target `0x4000` and return `1`, without sending `0x15`, when current timer is not already `0x4000` | `0x007376BF..0x0073770F`; `0x004B0EF0`; `0x004C9220` | Missing explicit timer-set-only handoff | `src/sim/miner/miner_dock_sequence.rs::phase_mission_enter`, `phase_linked`, `phase_pivoting` | Split accepted-cell entry from later/already-synced `0x16` cascade; do not start unload solely because the first `0x16` was received | Miner reaches accepted NW+(3,1) with facing timer not east; first dock sync starts facing target and does not queue unload in the same modeled step | `miner_dock_first_0x16_sets_facing_without_prepare` | Do not fold first and second `0x16` into one `Linked` state |
| Later/already-synced `0x16` may send `0x15` without `GetDockCoord` equality if unit is idle, has building destination, and unit mission is `7` | `0x0073771B..0x0073777A` | Rust currently snapshots to pad in `phase_linked`, masking which source won | same file plus tests | Model a separate `0x16` cascade path that does not require physical NW+(2,1) if global tick order proves the cascade wins | Idle mission-enter miner at accepted cell with facing current east and building destination sends the prepare/unload handoff without a pad-cell equality check | `miner_dock_synced_0x16_can_prepare_from_accepted_cell` | Do not require GetDockCoord before every `0x15` |
| A second/later `0x16` is not self-scheduled; it must come from another sender pass, stock path is Building `0x0E` after `0x12 == 0x14` | `0x007376AD..0x00737783`; `0x0043C2D0` | Current Rust's re-entry timing is approximate | `phase_awaiting_accepted_cell`, admission retry tests | Keep the already-at-accepted-cell retry pass distinct from movement assignment; only that retry may emit another sync | `0x12` reply `1` assigns movement and stops; later `0x12` reply `0x14` opens `0x18/0x16` | `miner_dock_second_0x16_requires_reaccepted_already_at_cell` | Do not create an automatic timer callback that fires `0x16` without sender admission |

### Stale Docs / Follow-up Docs

Replace wording like:

> `0x16` sends `0x15` when destination building mission == 7.

with:

> `0x16` sends `0x15` when the receiving unit is not moving, has a non-null building destination, and the receiving unit's current mission is `7`; the `0x16` branch does not read the destination building's mission.

Replace wording like:

> `0x16` waits for alignment before returning to the building.

with:

> `0x16` always returns `1`; the first unsynchronized call may set the facing timer and return before the `0x15` cascade, while later/already-synchronized calls may send `0x15` if their idle/destination/mission gates pass.

## 10. Negative Facts / Do Not Do

- Do not model `0x16` as a movement or physical snap to stock `GetDockCoord`; it has no `GetDockCoord`, `Set_Destination`, `MOVE_TO_CELL`, or location write.
- Do not require destination building mission `7` for the `0x16` cascade; the verified mission check is the unit's own mission.
- Do not treat `0x16` return `1` as proof that `0x15` was sent. Both timer-set-only and no-cascade paths also return `1`.
- Do not add an autonomous Rust timer callback that sends `0x15`; gamemd's later `0x15` from this path requires another `0x16` receiver invocation.
- Do not collapse accepted NW+(3,1) and stock GetDockCoord NW+(2,1); `0x16` can be independent of the latter, while `PerCellProcess` is not.

## Sources

- Ghidra `decompile_function 0x00737430` - `UnitClass::Receive_Radio`, direct case `0x16`.
- Ghidra `decompile_function 0x004D8FB0` - `FootClass::Receive_Radio`, no direct `0x16` case.
- Ghidra `decompile_function 0x006F4AB0` - `TechnoClass::Receive_Radio`, cases `7/9/0x16`.
- Ghidra `decompile_function 0x0043C2D0` - `BuildingClass::Receive_Radio`, stock refinery sender.
- Ghidra `decompile_function 0x004B0EF0` - `DriveLocomotionClass::Do_Turn`.
- Ghidra `decompile_function 0x004C9220` - `RateTimer::Set`.
- Ghidra `decompile_function 0x004C93D0` - `RateTimer::Current`.
- `docs/research/RADIO_0x16_RECEIVER_UNITCLASS_CASE_16_GHIDRA_REPORT.md`
- `docs/research/REFINERY_DOCK_0X16_BRIDGE_VERIFICATION_GHIDRA_REPORT.md`
- `docs/research/RADIO_0X12_MOVE_TO_CELL_PAYLOAD_AND_TIMESTAMPS_GHIDRA_REPORT.md`
- `src/sim/miner/miner_dock_sequence.rs`
