# Carryall Radio Liveness And Stock Activity -- Ghidra Research Report

**Address(es):** `0x004166C0` (`AircraftClass::Mission_Move`), `0x00416D50` (`AircraftClass::Mission_Move_Carryall`), `0x00416AF0` (`AircraftClass::Carryall_Pickup`), `0x00737430` (`UnitClass::Receive_Radio`), `0x0041CC20` (`AircraftTypeClass::ReadINI`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Carryall radio protocol liveness for `UnitClass::Receive_Radio` cases `0x0F` and `0x24`, `AircraftClass::Mission_Move_Carryall`, and stock YR `Carryall=` availability.
**Non-Scope:** Full aircraft movement parity, jumpjet physics, generic passenger boarding, IFV/open-topped weapon swaps, all campaign map trigger decoding, and Rust implementation.
**Confidence:** High for binary path and rules parsing; Medium for campaign-only exclusion because packed campaign maps were not fully decoded in this slice.
**Active in YR:** Conditional. The binary path is live when an aircraft type has `Carryall=yes` and is executing `Mission_Move` with a valid target; standard stock YR skirmish does not naturally expose it because the only stock `Carryall=yes` type is `[HIND]` with `TechLevel=-1`.

## 0. Investigation Frame

**Target question:** Are the carryall radio cases `0x0F`/`0x24` and `AircraftClass::Mission_Move_Carryall` active in standard YR, campaign-only, dormant, or mod-only, and what should Rust prioritize?

**Non-goals:** Do not implement carryalls; do not broaden into all aircraft missions; do not relabel radio opcodes beyond the carryall slice; do not treat TS/YRpp names as ground truth.

**Evidence needed to mark COMPLETE:**

- Decompile `AircraftClass::Mission_Move` and confirm the gate into `Mission_Move_Carryall`.
- Decompile `Mission_Move_Carryall` and confirm which radio messages it transmits.
- Decompile `UnitClass::Receive_Radio` and confirm receiver behavior for `0x0F` and `0x24`.
- Decompile `AircraftTypeClass::ReadINI` and confirm `Carryall=` reader/default offset.
- Check stock `rules.ini`/`rulesmd.ini` for `Carryall=yes` owners and tech availability.
- Scan current Rust surfaces for carryall parsing/logic.

**Stop conditions:**

- Stop after the carryall pickup radio path and stock data verdict; do not follow every transport/passenger branch.
- Stop if a candidate campaign reference requires full MIX-map trigger decoding; record that as remaining uncertainty rather than expanding scope.
- Stop before any Rust edits.

## 1. Overview

Carryall pickup is real live binary code, not a dead switch arm. `AircraftClass::Mission_Move` checks `AircraftTypeClass+0xDFC` and dispatches to `AircraftClass::Mission_Move_Carryall` when that byte is non-zero. That carryall mission synchronously drives the radio conversation with the ground unit: `HELLO` (`0x02`), `WANT_RIDE` (`0x24`), `DOCKING_COMPLETE` (`0x07`), then later `NEED_TO_MOVE` (`0x13`) and final `BREAK` (`0x03`).

The stock-content verdict is different from the binary-liveness verdict. `Carryall=yes` is parsed and active, but stock YR rules assign it only to `[HIND]`, and `[HIND]` is `TechLevel=-1`. Therefore carryall pickup is conditional/mod-or-script enabled, dormant in normal standard skirmish, and not a near-term parity priority compared with active refinery/passenger radio behavior.

## 2. Class Layout / Key Offsets

| Class / field | Offset | Type | Verified meaning | Evidence |
|---|---:|---|---|---|
| `AircraftTypeClass.Carryall` | `+0xDFC` | bool | Parsed from `Carryall=`; defaults false in ctor; gates carryall mission dispatch. | `AircraftTypeClass__Constructor @ 0x0041C8B0`, `ReadINI @ 0x0041CC20`, `Mission_Move @ 0x004166C0` |
| `AircraftClass.SubState` | int index `0x2F`, byte `+0xBC` | int | Carryall mission sub-state: 0 validate, 1 approach, 2 fly, 3 land. | `0x00416D50` switch on `param_1[0x2f]` |
| `AircraftClass.NavTarget` | int index `0x169`, byte `+0x5A4` | pointer | Carryall target/cargo candidate. | `0x00416D50` repeated checks |
| `TechnoClass/FootClass radio contacts` | vtable slots `+0x274`, `+0x278` | virtual calls | To-first transmit and targeted transmit used for pickup handshake. | `0x00416D50` decompile plus protocol report |
| `UnitClass.Type` | int index `0x1B1`, byte `+0x6C4` | type pointer | Receiver type fields for passenger capacity/size checks in case `0x0F`. | `0x00737430` case `0x0F` |
| `UnitClass.Passenger/cargo marker` | byte/int field around `+0x684` / int index `0x1A1` | sentinel | Case `0x24` returns `1` if `-1`, else `10`. | `0x00737430` case `0x24` |

## 3. Core Logic

### 3.1 Mission dispatch gate

`AircraftClass::Mission_Move @ 0x004166C0` starts with:

1. Read `*(char *)(this->Type + 0xDFC)`.
2. If non-zero, immediately call `AircraftClass::Mission_Move_Carryall @ 0x00416D50`.
3. Otherwise run the ordinary aircraft move state machine.

This is the core liveness fact: `Carryall=` is not parsed-only. It is a runtime gate on an active mission function.

### 3.2 Carryall radio sequence

`AircraftClass::Mission_Move_Carryall @ 0x00416D50` uses `SubState` at int index `0x2F`.

| Sub-state | Verified behavior | Radio traffic | Evidence |
|---:|---|---|---|
| `0` validate | If the target exists, maps to a live object, is not already carried, and ally checks pass, the carryall establishes contact and asks for pickup permission. | `Transmit_Radio_ToFirst(0x03)` if existing destination mismatch, `Transmit_Radio(0x02, target)`, `Transmit_Radio_ToFirst(0x24)`, then `Transmit_Radio_ToFirst(0x07)` on success. | `0x00416D50` decompile; caller `AircraftClass__Mission_Move @ 0x004166C0` |
| `1` approach | Reads target coordinates and sets the carryall locomotor destination; no radio. | none | `0x00416D50` case 1 |
| `2` fly | Verifies target still exists/still matches; transitions toward landing when locomotor stops. | none | `0x00416D50` case 2 |
| `3` land | If landing over the destination building/cargo context and `0x13` returns `ROGER`, limbos/adds cargo, then breaks the link and resumes lift. | `Transmit_Radio_ToFirst(0x13)`, `Transmit_Radio_ToFirst(0x03)` | `0x00417272` region inside `0x00416D50`; `AircraftClass__Carryall_Pickup @ 0x00416AF0` |

`AircraftClass::Carryall_Pickup @ 0x00416AF0` contains the cleanup order that matters for liveness: when the cargo's destination is the carryall, it sends `0x19` (`LEAVE_DOCK`) before `0x03` (`BREAK`). This preserves the docked-in byte cleanup before the radio contact slot is severed.

### 3.3 UnitClass receiver cases

`UnitClass::Receive_Radio @ 0x00737430` handles both carryall-related receiver cases directly.

`case 0x0F` is a passenger/transport accept query. It requires non-zero receiver capacity field, non-null sender, allied sender, not cloaked, no building in current cell, not mind-controlled, sender-side open-topped/passenger blocker checks, capacity/size checks, then returns `1`, `10`, or `0`. This case is broader than carryall: it is also used by ordinary passenger/transport entry.

`case 0x24` is carryall-specific target-side `WANT_RIDE`. It rejects cloaked units with `0`, rejects tunnel/no-enter and mission `0x10` with `10`, and otherwise returns `1` when the passenger/carry marker is free or `10` when occupied. No side effects are made in the receiver.

## 4. INI Keys

| Key | Stock owner(s) | Stock value | Reader / offset | Runtime effect |
|---|---|---:|---|---|
| `Carryall=` | `[HIND]` only in `rules.ini` and `rulesmd.ini` | `yes` | `AircraftTypeClass__ReadINI @ 0x0041CC20`, writes `AircraftType+0xDFC`; ctor default false at `0x0041C8B0` | Enables `AircraftClass::Mission_Move_Carryall` dispatch from `Mission_Move`. |
| `TechLevel=` | `[HIND]` | `-1` | `TechnoTypeClass__ReadINI @ 0x00712170`; documented field `TechnoType+0x634` | Makes HIND unavailable through normal skirmish production. |
| `Passengers=` | `[HIND]` | `10` | inherited type reader; Rust parses generic passenger capacity | Cargo capacity once a carryall exists. |
| `PipScale=` | `[HIND]` | `Passengers` | inherited type reader | UI/cargo display if exposed. |
| `Landable=` | `[HIND]` | `yes` | `AircraftTypeClass__ReadINI @ 0x0041CC20`, writes `+0xE0A` | Allows landing behavior; not the carryall gate. |

Stock YR extracted file evidence:

- `ini/rulesmd.ini:10810` starts `[HIND]`.
- `ini/rulesmd.ini:10819` has `TechLevel=-1`.
- `ini/rulesmd.ini:10822` has `Carryall=yes`.
- `ini/rulesmd.ini:10824` has `PipScale=Passengers`.
- `ini/rulesmd.ini:10825` has `Passengers=10`.
- `ini/rules.ini:7850..7865` has the same base RA2 HIND carryall block.
- `rg` over extracted repo INIs found no other `Carryall=yes` owner.
- ASCII scan of top-level loose maps and the major stock MIX files found carryall/HIND rule and language-table references, but no loose map object placement evidence for a stock skirmish path. This is a support check, not a full campaign-map decode.

## 5. Integration Points

| Integration | Status | Evidence |
|---|---|---|
| Mission entry | Verified | `AircraftClass::Mission_Move @ 0x004166C0` calls `0x00416D50` when `Type+0xDFC != 0`; `get_function_callers(0x00416D50)` returned only `AircraftClass__Mission_Move @ 004166c0`. |
| Radio sender | Verified | `0x00416D50` sends `0x02`, `0x24`, `0x07`, `0x13`, and `0x03`; `0x00416AF0` sends `0x19` then `0x03` in pickup cleanup. |
| Unit receiver | Verified | `0x00737430` has explicit cases `0x0F` and `0x24`; `get_function_xrefs(0x00737430)` shows vtable data xref `0x007F5E04`. |
| INI reader | Verified | `0x0041CC20` reads `"Carryall"` via `CCINIClass__ReadBool` and writes `+0xDFC`; `get_function_xrefs(0x0041CC20)` shows vtable data xref `0x007E28CC`. |
| Tick phase | Touched-not-exhausted | Carryall radio runs inside aircraft mission tick via `Mission_Move`; exact global tick order already lives in broader radio/mission docs. |

## 6. Current Rust Implementation Status

Codegraph and filesystem scans found no `carryall` symbol and no `Carryall=` parser field. Current nearby Rust surfaces are:

- `src/rules/object_type.rs`: parses generic `Passengers=`, `SizeLimit=`, `Size=`, `Gunner=`, `OpenTopped=`, and aircraft-related fields, but not `Carryall=`.
- `src/sim/aircraft/mod.rs`: has `AircraftMission::Move`, `Attack`, `Guard`, `ReturnToBase`, `Docking`, `DockedIdle`, and paradrop states, but no carryall pickup sub-state.
- `src/sim/passenger.rs`: implements ordinary passenger/transport boarding and unloading through `PassengerRole`, not carryall lift/limbo/drop.
- `src/sim/game_entity.rs`: has `radio_contacts: Vec<u64>` helpers, but no full RadioClass synchronous protocol and no carryall-specific contact choreography.

Current Rust delta: carryall parser and carryall mission behavior are missing, but this is lower priority for standard YR skirmish parity than active refinery, passenger, and radio-break cleanup paths.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `AircraftClass::Mission_Move` carryall gate | verified | Decompile `0x004166C0`; caller edge to `0x00416D50` | none |
| `AircraftClass::Mission_Move_Carryall` radio sequence | verified | Decompile `0x00416D50`; caller `AircraftClass__Mission_Move`; callees include `AircraftClass__Carryall_Pickup`, `FootClass__GetDestination`, `CargoClass__AddPassenger` | none for scoped liveness |
| `AircraftClass::Carryall_Pickup` cleanup | verified | Decompile `0x00416AF0`; call graph from `0x00416D50` | none for scoped liveness |
| `UnitClass::Receive_Radio` case `0x0F` | verified | Decompile `0x00737430` | broader non-carryall passenger semantics out-of-scope |
| `UnitClass::Receive_Radio` case `0x24` | verified | Decompile `0x00737430` | none |
| `AircraftTypeClass::ReadINI Carryall=` | verified | Decompile `0x0041CC20`; ctor `0x0041C8B0` default false | none |
| Stock `rulesmd.ini` availability | verified | `ini/rulesmd.ini:10810..10825`; unique `Carryall=yes` grep | none |
| Campaign-only activity | touched-not-exhausted | Loose map/major MIX ASCII scan; prior `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md` says no campaign or skirmish AI script invokes carryall mission | Full packed campaign map trigger decode not performed |
| Current Rust carryall support | verified | Codegraph search for `carryall` found no results; `rg` over Rust surfaces | none |

## 8. Open Questions -- Final State

- `[RESOLVED] OQ-1 -- Is carryall pickup binary code live or dead? -> Live behind `AircraftType+0xDFC`, called from `AircraftClass::Mission_Move` when the type has `Carryall=yes`.` (evidence: `0x004166C0`, `0x00416D50`)
- `[RESOLVED] OQ-2 -- Which radio messages does the carryall sender transmit? -> Validate path sends `0x02`, `0x24`, `0x07`; land path sends `0x13`; cleanup sends `0x03`, with pickup cleanup also sending `0x19` before `0x03`.` (evidence: `0x00416D50`, `0x00416AF0`)
- `[RESOLVED] OQ-3 -- Does UnitClass handle `0x0F` and `0x24` directly? -> Yes; both are explicit switch cases in `UnitClass::Receive_Radio`.` (evidence: `0x00737430`)
- `[RESOLVED] OQ-4 -- Is `0x24` generic docking or carryall-specific in this slice? -> `0x24` is the target-side carryall `WANT_RIDE` query in the carryall sender path; no standard refinery sender was found in the prior radio docs.` (evidence: `0x00416D50`, `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-5 -- Is `0x0F` carryall-only? -> No; the UnitClass `0x0F` accept query is also part of ordinary transport/passenger admission.` (evidence: `0x00737430`, `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-6 -- Where is `Carryall=` parsed? -> `AircraftTypeClass::ReadINI @ 0x0041CC20` reads `"Carryall"` and writes `AircraftType+0xDFC`.` (evidence: `0x0041CC20`)
- `[RESOLVED] OQ-7 -- What is the default `Carryall` value? -> False; constructor zeroes `+0xDFC` before INI override.` (evidence: `0x0041C8B0`, `AIRCRAFTTYPECLASS_COMPLETE_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-8 -- Which stock YR type sets `Carryall=yes`? -> Only `[HIND]` in extracted `rulesmd.ini` and base `rules.ini`.` (evidence: `ini/rulesmd.ini:10810..10825`, `ini/rules.ini:7850..7865`)
- `[RESOLVED] OQ-9 -- Is stock `[HIND]` normally buildable in skirmish? -> No; `[HIND]` has `TechLevel=-1`.` (evidence: `ini/rulesmd.ini:10819`, `TechnoTypeClass__ReadINI @ 0x00712170`)
- `[RESOLVED] OQ-10 -- Is this active in standard YR skirmish? -> No natural stock path found; binary path is live but content is unavailable through normal production.` (evidence: `0x004166C0`, `ini/rulesmd.ini:10819/10822`)
- `[RESOLVED] OQ-11 -- Is this mod-only? -> Conditional: mods or scripted maps can activate it by making/placing a `Carryall=yes` aircraft and issuing move-to-target behavior; the binary needs no patch.` (evidence: `0x004166C0`, `0x0041CC20`)
- `[DEFERRED] OQ-12 -- Is there any packed campaign map trigger that places HIND and exercises carryall pickup?` (category: `bounded-cost-too-high`; reason: Full MIX campaign map trigger extraction was outside this bounded radio-slice; next-step-if-pursued: extract/decode all `mapsmd03.mix` campaign maps and grep `[Aircraft]`, `[Triggers]`, `[ScriptTypes]`, and reinforcement actions for `HIND`/carryall mission targets.)
- `[RESOLVED] OQ-13 -- Does current Rust parse or implement carryalls? -> No `carryall` symbol or parser field found; aircraft and passenger systems exist but do not model this mission.` (evidence: Codegraph `carryall` search; `src/rules/object_type.rs`, `src/sim/aircraft/mod.rs`, `src/sim/passenger.rs`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `Carryall=yes` is parsed into `AircraftType+0xDFC` and gates `AircraftClass::Mission_Move_Carryall` from ordinary `Mission_Move`. | `0x0041CC20`, `0x0041C8B0`, `0x004166C0` | missing | `src/rules/object_type.rs`, `src/sim/aircraft/mod.rs` | Future carryall support should start with a parsed aircraft-type flag and a separate carryall move sub-state only when that flag is true. | Proposed test: `carryall_yes_hind_uses_carryall_move_state_when_ordered_to_lift_vehicle` | Do not make all passenger aircraft carryalls; `Passengers=` alone is not the gate. |
| Carryall pickup uses synchronous radio returns, not direct cargo insertion: `HELLO` -> `WANT_RIDE(0x24)` -> `DOCKING_COMPLETE(0x07)` -> later `NEED_TO_MOVE(0x13)` -> `BREAK`. | `0x00416D50`, `0x00737430`, `0x00416AF0` | missing | `src/sim/game_entity.rs` radio contact helpers, `src/sim/passenger.rs`, future carryall mission module | Future implementation should preserve visible failure modes: target can reject, be busy, be cloaked, be in tunnel/no-enter, or become invalid before pickup. | Deterministic test with a carryall target that is already in mission `0x10` should reject `WANT_RIDE` and leave both entities uncarried. | Do not shortcut to `PassengerRole::Inside` at order issue time; gamemd waits for the landing-state handshake. |
| Stock standard YR skirmish does not need carryall pickup for immediate parity because only `[HIND]` has `Carryall=yes` and it is `TechLevel=-1`. | `ini/rulesmd.ini:10819`, `ini/rulesmd.ini:10822`; parser `0x0041CC20` | missing but acceptable near-term | Production/rules priority planning | Prioritize active stock systems first: refinery radio, generic passenger boarding, limbo/BREAK cleanup, and aircraft ammo/airfield docking. | Proposed scheduling check: `stock_rulesmd_only_hind_has_carryall_and_is_unbuildable` | Do not label UnitClass case `0x0F` as dead; it remains relevant to active transport boarding even though carryall is dormant. |

### Stale Docs / Follow-up Docs

Replace this wording in `UNITCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`:

- Current: `Active in YR: YES -- fires for carryall pickup protocol. Carryalls are YR-active (Soviet carryall, YR-era).`
- Replacement: `Active in YR: Conditional -- the binary carryall pickup path is live, but stock standard YR skirmish does not naturally expose it because only [HIND] has Carryall=yes and [HIND] has TechLevel=-1. Mods or scripted maps can activate it.`

For `UnitClass::Receive_Radio` case `0x24`, prefer:

- `Active in YR: Conditional / dormant in standard skirmish -- receiver logic is live and called by AircraftClass::Mission_Move_Carryall, but stock rules make the sole Carryall=yes aircraft unbuildable.`

No correction is needed to `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md` section 5.2; this investigation corroborates its stock-activity verdict.

## 10. Negative Facts / Do Not Do

- Do not treat `Carryall=yes` as dead code. It is parsed and consumed by `AircraftClass::Mission_Move`.
- Do not treat carryall pickup as active normal stock skirmish behavior. The only stock carrier is `[HIND]` and it is `TechLevel=-1`.
- Do not implement carryall support by checking `Passengers>0`; HIND has both, but the binary dispatch gate is `AircraftType+0xDFC Carryall`.
- Do not relabel `UnitClass::Receive_Radio` case `0x0F` as carryall-only; it is also a generic passenger/transport accept query.
- Do not prioritize carryall ahead of active standard YR radio/passenger/refinery behavior unless a target mod/campaign scenario explicitly requires it.

## 11. Remaining Uncertainty

- Full packed campaign map/script extraction was not performed in this slice. The current verdict is strong for standard skirmish and stock rules availability; a campaign-specific proof would require decoding `mapsmd03.mix` contents and trigger/script actions.
- Exact visual drop/fall behavior after carryall death mid-flight belongs to the broader limbo/BREAK/passenger-ejection system, not this liveness slice.

## Sources

- Live Ghidra decompilation: `0x004166C0`, `0x00416D50`, `0x00416AF0`, `0x004190B0`, `0x00737430`, `0x0041C8B0`, `0x0041CC20`, `0x006723D0`, `0x00712170`.
- Ghidra call evidence: `get_function_callers(0x00416D50)` -> `AircraftClass__Mission_Move @ 004166c0`; `get_function_xrefs(0x00737430)` -> vtable data `0x007F5E04`; `get_function_xrefs(0x0041CC20)` -> vtable data `0x007E28CC`.
- Prior docs: `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`, `UNITCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`, `AIRCRAFTTYPECLASS_COMPLETE_GHIDRA_REPORT.md`, `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`.
- INI files checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust surfaces scanned: `src/rules/object_type.rs`, `src/sim/aircraft/mod.rs`, `src/sim/passenger.rs`, `src/sim/game_entity.rs`, `src/sim/world/world_commands.rs`.
