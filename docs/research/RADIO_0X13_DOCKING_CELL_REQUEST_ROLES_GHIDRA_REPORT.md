# Radio 0x13 Docking Cell Request Roles - Ghidra Research Report

**Address(es):** `0x004D8FB0` (`FootClass::Receive_Radio`, case `0x13`), `0x0043C2D0` (`BuildingClass::Receive_Radio`, case `0x0E`), `0x00737430` (`UnitClass::Receive_Radio`, case `0x0E`), `0x004190B0` (`AircraftClass::Receive_Radio`), `0x00416D50` (`AircraftClass::Mission_Move_Carryall`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** radio message `0x13` sender/receiver roles across refinery, aircraft, and carryall contexts; when it behaves as a dock-cell admission subquery versus carryall `NEED_TO_MOVE`; payload/return semantics; active standard YR paths.
**Non-Scope:** full `0x12`, `0x0E`, airfield reload, passenger boarding, UnitRepair state machines, or carryall visual/drop physics beyond the necessary call chain.
**Confidence:** High for role map from existing live-Ghidra reports; no fresh Ghidra MCP endpoint was exposed in this session, so this report reconciles prior decompile+assembly-backed reports instead of adding new decompilation.
**Active in YR:** Yes for refinery/harvester dock admission; Conditional for aircraft/carryall paths.

## 0. Investigation Contract

**Target question:** What exactly does radio `0x13` mean in each sender/receiver context: refinery dock-cell request, aircraft receiver query, or carryall landing `NEED_TO_MOVE`?

**Non-goals:** Do not redo all `0x12` or `0x0E` logic; do not audit all aircraft radio cases; do not implement Rust; do not mutate Ghidra; do not edit in-repo docs.

**Evidence needed to mark COMPLETE:**

- Receiver-side decompile plus assembly for `FootClass::Receive_Radio(0x13)`.
- Sender-side decompile plus assembly/caller evidence for refinery/building `0x0E` using `0x13`.
- Sender-side decompile plus debug-string/caller evidence for carryall LAND using `0x13`.
- Aircraft receiver gate evidence for when `AircraftClass::Receive_Radio` delegates `0x13`.
- INI/default evidence for stock refinery and carryall activity.
- Current Rust surface scan and implementation handoff.

**Stop conditions:** Stop after the role map is reconciled and stale naming is identified. Defer full airfield reload, UnitRepair repair-produce, passenger, and campaign carryall activation questions.

## 1. Overview

Radio `0x13` is not a unique "dock cell" opcode. The receiver behavior is the same `NEED_TO_MOVE` query in FootClass-derived receivers: write the receiver's current NavCom/chrono destination pointer (`this+0x5A4`) into `*payload`, then return `NEGATORY(10)` only if that pointer is non-null and the locomotor is still moving; otherwise return `ROGER(1)`.

The "docking-cell request" meaning is a sender-side role layered on top of that query. Standard refinery `CAN_DOCK(0x0E)` uses `0x13` as the readiness probe before sending `MOVE_TO_CELL(0x12)` with the hardcoded queue cell. Carryall LAND uses the same `0x13` code as `RADIO_NEED_TO_MOVE` before limboing/adding cargo. In both roles, the payload is the receiver's current destination/status, not the assigned dock cell.

## 2. Class Layout / Key Offsets

| Class / offset | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `FootClass+0x5A4` | NavCom / chrono destination pointer written to `*payload` by `0x13` | `FootClass::Receive_Radio @ 0x004D90E8`; assembly `004d90ec`, `004d90f2`; `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md` | Yes |
| `FootClass+0x674` | `ILocomotion*`; null-asserted if `+0x5A4 != 0`, then vtable `+0x10` checks moving | `FootClass::Receive_Radio @ 0x004D90E8`; assembly `004d9102..004d9124` | Yes |
| `BuildingType+0x16B3` | `DockUnload=`; standard refinery path in BuildingClass `0x0E` | `RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md`; `rulesmd.ini:11726`, `12519` | Yes |
| `BuildingType+0x16CB` | `Helipad=`; alternate BuildingClass `0x0E` branch after `0x13` | `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`; `rulesmd.ini:11820`, `12342` | Yes for helipad buildings, not refinery |
| `AircraftType+0xDFC` | `Carryall=`; dispatch gate into carryall mission | `CARRYALL_RADIO_LIVENESS_AND_STOCK_ACTIVITY_GHIDRA_REPORT.md`; reader `0x0041CC20`; `rulesmd.ini:10822` | Conditional |
| `AircraftClass+0xBC` | Carryall mission sub-state; state 3 is LAND and sends `0x13` | `AircraftClass::Mission_Move_Carryall @ 0x00416D50` | Conditional |
| `Techno+0x294` | `AirstrikeClass*`; AircraftClass radio gate exception for scoped missions | `AIRCRAFTCLASS_0XA5_RADIO_GATE_WRITERS_GHIDRA_REPORT.md`; assembly `0x004190D3..0x004190E3` | Conditional |

## 3. Core Logic

### Receiver semantics: FootClass case `0x13`

`FootClass::Receive_Radio @ 0x004D8FB0` dispatches `0x13` to `0x004D90E8`.

The verified behavior is:

1. Write `this+0x5A4` into `*payload` before any guard.
2. If `this+0x5A4 == 0`, return `ROGER(1)`.
3. If `this+0x5A4 != 0`, assert `this+0x674` exists, call `ILocomotion::Is_Moving` via vtable `+0x10`.
4. Return `NEGATORY(10)` while moving; otherwise return `ROGER(1)`.

Assembly evidence from the FootClass full-switch report:

```text
004d90e8: MOV EDX, [ESP+0x34]   ; payload ptr
004d90ec: MOV ECX, [ESI+0x5a4]  ; this+0x5A4
004d90f2: MOV [EDX], ECX        ; *payload = this+0x5A4
004d9102: MOV EAX, [ESI+0x674]  ; ILocomotion*
004d911f: CALL [EAX+0x10]       ; Is_Moving()
004d912d: MOV EAX, 0xa          ; NEGATORY when moving
```

**Conclusion:** The receiver does not compute or return a dock cell. It reports its current destination/status and readiness to accept the next instruction.

### Standard refinery role: `0x13` as admission/readiness probe before dock-cell assignment

`BuildingClass::Receive_Radio(0x0E) @ 0x0043C2D0` is the active stock refinery admission path. For `DockUnload=yes` / `Weeder=yes`:

1. Building receives `CAN_DOCK(0x0E)` from the harvester.
2. Building sends `Transmit_Radio(0x13, harvester)`.
3. If the reply is `NEGATORY(10)` with the documented stack sentinel state, the building returns `ROGER(1)` as a silent "not now" rather than hard-failing.
4. On acceptance, the building sends `MOVE_TO_CELL(0x12)` with the queue cell.
5. Only after the receiver replies `CELL_ACCEPTED(0x14)` does the building send `0x18` and `0x16`.

The queue cell sent by `0x12` is hardcoded as building anchor `(+3,+1)` in standard refinery path. `QueueingCell=` is parsed but not read here. Evidence: `RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md` decompile of `0x0043C2D0`, plus `ObjectClass__Get_Cell_Packed @ 0x0041BEA0`.

**Role wording:** In the refinery sender context, `0x13` means "are you ready / what are you already moving toward?" The actual dock-cell request/assignment is `0x12`, not `0x13`.

### UnitClass sender role: `0x13` inside vehicle-side `0x0E`

`UnitClass::Receive_Radio(0x0E) @ 0x007377D8` also uses `0x13` in the admission subprotocol. The full-switch report says the vehicle-side case can send `Transmit_Radio(0x13, sender)`, then write `this` into `*payload`, then send `Transmit_Radio_Impl(0x12, payload, sender)`, aborting with `0x03` if the cell assignment fails.

This is why old reports describe "UnitClass `0x0E` sends `0x13` to refinery as dock cell request." The verified chain exists, but the precise role is readiness/payload setup before the `0x12` move-cell exchange. The `0x13` return is still `ROGER/NEGATORY`; the cell is carried by the following `0x12`.

### Carryall role: `0x13` as named `RADIO_NEED_TO_MOVE`

`AircraftClass::Mission_Move_Carryall @ 0x00416D50` sends `0x13` in sub-state `3` (LAND). The protocol report ties this to the carryall debug string at `0x00817C14` and assembly around `0x00417272`: LAND sends `vtable+0x274(0x13)` and logs `RADIO_NEED_TO_MOVE got RADIO_ROGER`.

On `ROGER`, the carryall limbos/detaches the cargo, adds it to cargo/passenger storage, then sends `BREAK(0x03)` and begins lifting. On rejection it resets to validation. Evidence: `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md` and `CARRYALL_RADIO_LIVENESS_AND_STOCK_ACTIVITY_GHIDRA_REPORT.md`.

**Active in YR:** Conditional. The binary path is live behind `AircraftType+0xDFC Carryall`, but standard stock YR skirmish exposes no normal build path because only `[HIND]` has `Carryall=yes`, and it has `TechLevel=-1`.

### Aircraft receiver role

`AircraftClass::Receive_Radio @ 0x004190B0` has an AircraftClass-level switch and can handle/delegate `0x13`. The full protocol report lists AircraftClass handled cases including `0x13`, and FootClass callers include AircraftClass cases `0x0E`, `0x12`, `0x13`, and `0x1F` delegating to FootClass. The important activity gate is separate: for missions `{4, 0x1A, 0x1B, 0x1E, 0x1F}`, AircraftClass returns `0` before normal radio handling when `Techno+0x294 == null`. Stock aircraft normally have no `AirstrikeClass*`, so that gate blocks those scoped missions by default.

**Role wording:** Aircraft receiving `0x13` uses the same FootClass `NEED_TO_MOVE` payload/return behavior when the AircraftClass gate allows it. This is not a refinery dock-cell role unless the sender's larger protocol uses the reply that way.

## 4. INI Keys

| Key | Stock value / owner | Binary reader / use | Effect on `0x13` role | Active in YR |
|---|---|---|---|---|
| `DockUnload=` | `GAREFN=yes` at `rulesmd.ini:11726`; `NAREFN=yes` at `12519` | BuildingType flag `+0x16B3`, used in BuildingClass `0x0E` | Enables stock refinery path: `0x13` probe, then `0x12` queue cell | Yes |
| `Refinery=` | `GAREFN=yes` at `11727`; `NAREFN=yes` at `12520` | refinery identity and related dock/search logic | Confirms standard ore refinery content | Yes |
| `NumberOfDocks=` | `1` at `11729`, `12521` | Building constructor contact capacity | Capacity for radio contacts, not `0x13` semantics | Yes |
| `Dock=` | HARV/CMIN variants list `NAREFN,GAREFN` | unit-side dock search/eligibility | Makes stock harvesters target refineries | Yes |
| `Helipad=` | stock helipads at `rulesmd.ini:11820`, `12342` | BuildingClass `0x0E` helipad branch | Uses `0x13` admission path but not refinery queue cell | Yes |
| `Carryall=` | `[HIND] Carryall=yes` at `rulesmd.ini:10822`; ctor default false | `AircraftTypeClass::ReadINI @ 0x0041CC20`, writes `AircraftType+0xDFC` | Enables carryall LAND sender of `0x13` | Conditional |
| `TechLevel=` | `[HIND] TechLevel=-1` at `rulesmd.ini:10819` | `TechnoTypeClass::ReadINI @ 0x00712170` | Makes stock carryall dormant in normal skirmish | Conditional / dormant |
| `AirstrikeTeam*` | stock `[BORIS]` only, not stock aircraft | `TechnoClass::Init_Managers @ 0x006F3F40` | Aircraft radio gate exception; not a `0x13` role flag | Conditional |

## 5. Integration Points

| Context | Sender | Receiver | `0x13` payload | `0x13` return | Next step | Active in standard YR |
|---|---|---|---|---|---|---|
| Stock refinery CAN_DOCK | Building/refinery during `0x0E` | Harvester (`FootClass` via UnitClass) | Receiver writes its `+0x5A4` into `*payload` | `1` if ready, `10` while moving to non-null destination | Building sends `0x12` with anchor `(+3,+1)` queue cell | Yes |
| Vehicle-side dock subprotocol | UnitClass case `0x0E` | Building/refinery or dock target | Shared scratch; later caller writes `this` before `0x12` | `1`, `10`, or later `0x12` reply codes | `0x12`, maybe `0x03` abort | Yes for active dock negotiation |
| Carryall LAND | Carryall aircraft | Ground cargo (`UnitClass`/FootClass) | Cargo writes current destination pointer | `1` permits limbo/add-cargo; `10` resets to validation | `CargoClass::AddPassenger`, `0x03` BREAK | Conditional; dormant in normal skirmish |
| Aircraft receiver | Building/aircraft depending sender | AircraftClass then FootClass | Aircraft/Foot writes `+0x5A4` if gate allows | `1`/`10`, or `0` if AircraftClass mission gate blocks earlier | Sender-specific | Conditional |
| UnitRepair/repair-produce | Building repair code can send `0x13` | Docked/target unit | Same receiver payload | Same receiver return | Repair path-specific checks | Active for UnitRepair, but full path out-of-scope |

## 6. Current Rust Implementation Status

Current Rust has no generic synchronous `RadioClass` receive switch. Relevant surfaces:

- `src/sim/miner/miner_dock.rs`: `RefineryDockContacts` stores `contacts`, `waiting_retry_queue`, `contact_entered`, and `on_pad`; `hello_or_wait` is a specialized refinery contact/admission model.
- `src/sim/miner/miner_dock_sequence.rs`: current miner sequence has direct phases and `contact_entered`/pad transitions instead of explicit `0x13 -> 0x12 -> 0x18 -> 0x16` synchronous returns.
- `src/sim/docking/aircraft_dock.rs`: `AirfieldDocks` manages pad reservations/queues, not radio opcode dispatch.
- `src/sim/passenger.rs`: passenger/cargo state is direct `PassengerRole`, not carryall radio choreography.
- `src/sim/game_entity.rs`: `radio_contacts: Vec<u64>` exists for contact-like passability, but it is not a full sparse reciprocal RadioClass implementation.
- Source scan found no `Carryall=` parser or carryall mission implementation.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FootClass::Receive_Radio` case `0x13` payload/return | verified | decompile `0x004D8FB0`; assembly `0x004D90E8..0x004D9136`; `FOOTCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` | none |
| BuildingClass standard refinery `0x0E -> 0x13 -> 0x12` chain | verified | decompile `0x0043C2D0`; `RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md`; `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` | none for role map |
| UnitClass `0x0E` sender of `0x13` | verified | decompile/disassembly `0x007377D8..0x00737A95`; `UNITCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` | exact "which side first" phrasing in older docs remains confusing |
| Carryall LAND sender of `0x13` | verified | decompile `0x00416D50`; assembly/debug string around `0x00417272`, string `0x00817C14`; caller `0x004166C0` | campaign trigger activation deferred |
| AircraftClass receiver/delegation/gate | verified enough for scope | decompile `0x004190B0`; assembly `0x004190D3..0x004190E3`; protocol report case list | full aircraft radio switch not separately reprinted here |
| Stock refinery INI activity | verified | `rulesmd.ini:11726..11729`, `12519..12521`; base rules corroborate | none |
| Stock carryall INI activity | verified | `rulesmd.ini:10819`, `10822`; reader `0x0041CC20` | packed campaign usage deferred |
| Current Rust surfaces | verified | Codegraph and `rg` source scan of miner/aircraft/passenger/radio-contact surfaces | no source edits by design |
| Fresh live Ghidra MCP in this session | deferred | tool discovery returned no Ghidra MCP tools | if needed, re-run with Ghidra MCP exposed to re-decompile cold |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - What is the canonical receiver behavior of `0x13`? -> FootClass writes `this+0x5A4` to `*payload`, then returns `1` unless that pointer is non-null and the locomotor is still moving, in which case it returns `10`.` (evidence: `0x004D90E8`, assembly `004d90ec..004d912d`)
- `[RESOLVED] OQ-02 - Does receiver `0x13` itself compute a dock cell? -> No; dock/queue cell is sent later by sender-side `0x12`.` (evidence: `0x004D90E8`; `0x0043C2D0` refinery chain)
- `[RESOLVED] OQ-03 - Why do prior docs call `0x13` a docking-cell request? -> Because refinery/UnitClass `0x0E` sender contexts use `0x13` as the readiness probe immediately before `0x12 MOVE_TO_CELL`; the label describes the sender protocol role, not the receiver opcode body.` (evidence: `0x0043C2D0`, `0x007377D8`)
- `[RESOLVED] OQ-04 - Is standard refinery use active in YR? -> Yes, stock `GAREFN/NAREFN` have `DockUnload=yes`, `Refinery=yes`, and `NumberOfDocks=1`.` (evidence: `rulesmd.ini:11726..11729`, `12519..12521`)
- `[RESOLVED] OQ-05 - Is carryall `0x13` the same opcode? -> Yes; carryall LAND sends `vtable+0x274(0x13)` and the debug string names it `RADIO_NEED_TO_MOVE`.` (evidence: `0x00416D50`, `0x00417272`, string `0x00817C14`)
- `[RESOLVED] OQ-06 - Is carryall `0x13` active in normal stock skirmish? -> Conditional/dormant: binary path is live, but the only stock `Carryall=yes` aircraft is `[HIND]` with `TechLevel=-1`.` (evidence: `0x0041CC20`, `rulesmd.ini:10819`, `10822`)
- `[RESOLVED] OQ-07 - Does AircraftClass add a separate dock-cell meaning for `0x13`? -> No scoped evidence; when AircraftClass permits/delegates `0x13`, the FootClass payload/return behavior applies. Aircraft-specific mission gate can return `0` before delegation.` (evidence: `0x004190B0`; `AIRCRAFTCLASS_0XA5_RADIO_GATE_WRITERS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-08 - Which Rust systems are affected? -> miner refinery dock sequence, future generic radio/contact model, aircraft dock/carryall surfaces, and passenger/cargo surfaces.` (evidence: source scan of `src/sim/miner`, `src/sim/docking`, `src/sim/passenger`, `src/sim/game_entity.rs`)
- `[DEFERRED] OQ-09 - Do stock packed campaign maps activate HIND carryall `0x13`?` (category: `bounded-cost-too-high`; reason: full campaign MIX trigger decode is outside this radio-role slice; next-step-if-pursued: decode campaign maps and scan aircraft placements/scripts for `HIND` carryall move orders)
- `[DEFERRED] OQ-10 - What exact UnitRepair/repair-produce role uses `0x13` at `0x0044C8AC`?` (category: `out-of-scope`; reason: target only needed to distinguish refinery/carryall/aircraft roles; next-step-if-pursued: dedicated UnitRepair radio sender report)
- `[DEFERRED] OQ-11 - Fresh cold re-decompile of all primary functions in this session.` (category: `requires-different-system-context`; reason: no Ghidra MCP tools were exposed to this subagent; next-step-if-pursued: rerun with Ghidra MCP and spot-check `0x004D8FB0`, `0x0043C2D0`, `0x00416D50`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `0x13` receiver behavior is `NEED_TO_MOVE`: write receiver `+0x5A4` to payload, return `10` only if non-null destination and locomotor still moving, else `1`. | `FootClass::Receive_Radio @ 0x004D90E8`; assembly `004d90ec..004d912d` | unchecked/missing as generic radio opcode | future `sim::radio`; `src/sim/miner/miner_dock_sequence.rs`; aircraft/carryall surfaces | Model `0x13` as readiness/status query, not as the cell assignment itself. | Proposed test: `radio_0x13_need_to_move_returns_negatory_only_for_moving_nonnull_destination` | Do not make `0x13` return a cell or mutate movement target directly. |
| Standard refinery admission uses `0x13` before `0x12`; the queue cell is assigned by `0x12` as anchor `(+3,+1)`, and `0x13` rejection is a soft wait path. | `BuildingClass::Receive_Radio @ 0x0043C2D0`; `RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md` | partially represented by direct miner dock phases and `RefineryDockContacts` | `src/sim/miner/miner_dock.rs`; `src/sim/miner/miner_dock_sequence.rs` | Preserve two separate decisions: readiness probe vs move-cell assignment. | Miner already moving to a chrono/dock destination receives refinery admission probe and is not advanced into unload until ready/accepted-cell path completes. | Do not label the hardcoded `(+3,+1)` queue cell as `0x13` payload; it belongs to `0x12`. |
| Carryall LAND sends the same `0x13` as named `RADIO_NEED_TO_MOVE`; stock skirmish normally leaves it dormant because `[HIND]` is unbuildable. | `AircraftClass::Mission_Move_Carryall @ 0x00416D50`; `0x00417272`; `rulesmd.ini:10819/10822` | missing carryall parser/mission | `src/rules/object_type.rs`; future carryall aircraft mission; `src/sim/passenger.rs`/cargo handling | Future carryall support should reuse `0x13` readiness semantics and only limbo/add cargo after `ROGER`. | Carryall targeting a busy ground unit loops/revalidates instead of immediately inserting cargo. | Do not implement carryall as `Passengers>0`; gate on `Carryall=yes`. |
| AircraftClass may block `0x13` before FootClass in scoped scripted missions when `Techno+0x294 == null`. | `AircraftClass::Receive_Radio @ 0x004190B0`; assembly `0x004190D3..0x004190E3` | missing aircraft radio receive switch/Airstrike manager | `src/sim/aircraft/mod.rs`; future radio receiver | If generic radio dispatch is added, apply AircraftClass mission gate before FootClass semantics. | Paradrop/scripted aircraft in gated missions ignores dock/move radio unless it has an AirstrikeClass-style manager. | Do not fake this with a paradrop-specific boolean; it is the shared `AirstrikeClass*` pointer. |

## 10. Negative Facts / Do Not Do

- Do not globally rename `0x13` to "dock-cell request." The binary-confirmed global name is `NEED_TO_MOVE`; "dock-cell request" is a sender-side refinery role.
- Do not put the refinery queue cell in the `0x13` payload. The cell is sent by the following `0x12 MOVE_TO_CELL`.
- Do not swap `0x11` and `0x13` back to older naming. Prior protocol work explicitly corrected `0x11` away from `NEED_TO_MOVE`; `0x13` is confirmed by the carryall debug string.
- Do not treat carryall `0x13` as active normal stock skirmish behavior; `[HIND]` has `TechLevel=-1`.
- Do not treat AircraftClass `0x13` as proof of stock aircraft dock-cell assignment; AircraftClass has its own radio gate and activity conditions.

## 11. Remaining Uncertainty

- No fresh Ghidra MCP tools were exposed to this subagent session, so this report relies on existing reports that already include live decompile, assembly, xref, and INI reader evidence. The role map is still high confidence, but a future audit can cold re-decompile the five primary functions if desired.
- Packed campaign/script usage of `[HIND]` carryall was not decoded here; this affects frequency only, not opcode semantics.
- UnitRepair/repair-produce sender use of `0x13` at `0x0044C8AC` is acknowledged but not expanded because it is outside the refinery/aircraft/carryall role map.

## 12. Stale Docs / Follow-up Docs

Suggested replacement wording for reports that say `0x13` is `REQUEST_DOCK_CELL`, `QUEUE_DOCK`, or "asks the refinery which cell should I go to":

> Radio `0x13` is `NEED_TO_MOVE`: the receiver writes its current NavCom/chrono destination pointer to the payload and returns `ROGER` unless it is still moving with a non-null destination. In refinery docking this is used as a readiness probe immediately before the sender transmits `0x12 MOVE_TO_CELL` with the actual queue/dock cell, so "dock-cell request" is a contextual role, not the opcode's receiver semantics.

Specific stale wording candidates:

- `UNITCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`: replace `0x13 = REQUEST_DOCK_CELL / QUEUE_DOCK` with the wording above, while preserving the verified `0x0E -> 0x13 -> 0x12` sequence.
- `FOOTCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`: replace section title `REQUEST_DOCK_CELL (Query: can you accept my dock request?)` with `NEED_TO_MOVE / readiness query`; keep the payload/return details unchanged.
- `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`: one line near the subclass table still says "foot-level 0x12 (MOVE_TO_CELL) and 0x13 (IS_UNIT_LINKED)"; replace `0x13 (IS_UNIT_LINKED)` with `0x13 (NEED_TO_MOVE)`.

## Sources

- Existing live-Ghidra reports: `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`, `FOOTCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`, `UNITCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`, `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`, `CARRYALL_RADIO_LIVENESS_AND_STOCK_ACTIVITY_GHIDRA_REPORT.md`, `RADIO_0X07_DOCKING_COMPLETE_SENDER_AND_CASE7_REACHABILITY_GHIDRA_REPORT.md`, `AIRCRAFTCLASS_0XA5_RADIO_GATE_WRITERS_GHIDRA_REPORT.md`, `miner/RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md`.
- Ghidra evidence cited from those reports: decompile/assembly for `0x004D8FB0`, `0x004D90E8`, `0x0043C2D0`, `0x007377D8`, `0x004190B0`, `0x00416D50`, `0x00417272`, `0x0041CC20`, `0x00712170`, string `0x00817C14`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust scanned: `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/docking/aircraft_dock.rs`, `src/sim/passenger.rs`, `src/sim/game_entity.rs`, `src/rules/object_type.rs`.

## Status

COMPLETE for the scoped role map using existing decompile-backed evidence; fresh same-session Ghidra re-decompile is explicitly deferred because no Ghidra MCP tools were exposed.
