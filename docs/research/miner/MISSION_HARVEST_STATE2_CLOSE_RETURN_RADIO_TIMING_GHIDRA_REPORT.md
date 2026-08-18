# Mission_Harvest State 2 Close Return Radio Timing - Ghidra Research Report

**Address(es):** `0x0073E5E0` primary; supporting `0x0065A970`, `0x0065A820`, `0x004D9290`, `0x0043C2D0`, `0x004D8FB0`, `0x006F4AB0`, `0x00737430`, `0x00739EC0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** stock YR `CMIN`/`HARV` full-cargo close return from `UnitClass::Mission_Harvest` state 2 through the first refinery dock-admission radio burst and the handoff into Mission_Enter/dock arrival.  
**Non-Scope:** far-return `QueueingCell` fallback search internals, post-unload exit, queue eviction edge cases, exact live frame count between mission queue and mission dispatch.  
**Confidence:** High for static binary ordering; Medium for exact frame separation because this slot did not run a live debugger.  
**Active in YR:** Yes. Stock `[CMIN]` and `[HARV]` have `Harvester=yes` and `Dock=NAREFN,GAREFN`; stock `[GAREFN]`/`[NAREFN]` have `DockUnload=yes` and `Refinery=yes`.

## 1. Overview

`Mission_Harvest` state 2 does not send the full dock-admission burst. On a close return, it sends radio `0x02` (`HELLO`) to the chosen refinery and, only if that returns `ROGER(1)`, writes harvest substate `3`.

Substate `3` then queues mission `7` (`Mission_Enter`). The actual refinery admission burst is driven from Mission_Enter/Unit Set_Destination: `0x0E CAN_DOCK` to the building, then building-side `0x13`, `0x12`, `0x18`, and `0x16`. If `0x12` reports the unit is already on the accepted cell, the same synchronous `0x0E` call can send `0x18` and `0x16`; otherwise it only sets NavCom toward the accepted cell and waits for physical arrival.

## 2. Key Offsets / Slots

| Offset / slot | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| Unit `+0xBC` | harvest substate; `2` = return, `3` = enter-refinery handoff | `0x0073E5E0`, write at `0x0073EE68` | Yes |
| Unit `+0x5A4` | current destination/NavCom pointer | `0x004D9290`, `0x004D8FB0`, `0x00741970` | Yes |
| Unit/Techno `+0x418` | radio dock/contact flag set by `0x18`; read by Mission_Enter fallback and `0x16` cascade | `0x006F4AB0`, `0x004D9290`, `0x00737430` | Yes |
| Unit `+0x6AF` | chrono-timing gate checked by `0x16` receiver before `Do_Turn(0x4000)` | `0x00737430` | Yes |
| Unit type `+0xCD4` | `Teleporter=yes`; selects CMIN close threshold | `0x0073E5E0`; `ini/rulesmd.ini:7396` | Yes for CMIN |
| Rules `+0xD78` | `HarvesterTooFarDistance` | `0x0073E5E0`; `ini/rulesmd.ini:293` | Yes for HARV |
| Rules `+0xD7C` | `ChronoHarvTooFarDistance` | `0x0073E5E0`; `ini/rulesmd.ini:294` | Yes for CMIN |
| Unit vtable `+0x278` | directed/broadcast radio transmit | `0x0073EE59`, `0x004D92B9`, `0x0043C2D0` | Yes |
| Unit vtable `+0x274` | transmit to first contact | `0x004D92D4`, `0x00739EC0` | Yes |
| Unit vtable `+0x1E8` | queue/set mission | `0x0073EE93`, `0x0043C2D0` case `0x15` | Yes |
| Unit vtable `+0x480` | Set_Destination / NavCom update | `0x004D9290`, `0x004D8FB0`, `0x00741970` | Yes |

## 3. Core Timing

### 3.1 State 2 close-return decision

`UnitClass::Mission_Harvest @ 0x0073E5E0` state 2 first requires no current destination (`+0x5A4 == 0`) before selecting a refinery. If a dock object is found, it computes 3D lepton distance between the miner and refinery object coordinates.

For non-chrono harvesters, the close test is:

```text
distance <= Rules.HarvesterTooFarDistance * 0x100
```

For chrono miners, the close test is:

```text
distance <= Rules.ChronoHarvTooFarDistance * 0x100
```

The comparison is inclusive. Stock values are `5` cells for `HARV` and `50` cells for `CMIN`.

**Active in YR:** Yes. Evidence: `0x0073E5E0`, assembly around `0x0073EE40..0x0073EE4B`, INI lines `rulesmd.ini:293..294`.

### 3.2 State 2 sends only `HELLO(0x02)`

On close pass, state 2 calls:

```text
unit->Transmit_Radio(0x02, refinery)
```

The assembly sequence is `PUSH refinery`, `PUSH 0x2`, `CALL [unit.vtable+0x278]` at `0x0073EE54..0x0073EE59`. The return is compared with `1` at `0x0073EE5F`; only then does it write `+0xBC = 3` at `0x0073EE68`.

There is no `0x0E`, `0x12`, `0x18`, `0x16`, or `0x15` send in this state-2 close branch.

**Active in YR:** Yes. Standard full-cargo CMIN/HARV return reaches this branch when close enough and not already moving to a destination.

### 3.3 What radio `0x02` does

`RadioClass::Transmit_Radio @ 0x0065AAA0` forwards to `Transmit_Radio_Impl @ 0x0065A970` with a scratch parameter. For `0x02`, the transmitter:

1. returns `1` immediately if the target is already in `Contacts[]`;
2. otherwise finds a free contact slot;
3. if full, evicts slot 0 by sending `0x03`;
4. calls the target receiver with `0x02`;
5. writes the target pointer into the sender's `Contacts[]` only if the receiver returns `1`.

The receiver side eventually falls to `RadioClass::Receive_Radio @ 0x0065A820`, which accepts `0x02` only through ally/contact/free-slot checks and returns `1` or `10`.

**Active in YR:** Yes. This is the live RadioClass protocol used by stock harvesters and refineries.

### 3.4 State 3 queues Mission_Enter, not dock admission

The next harvest substate is state `3`, which executes:

```text
unit->SetMission(7, 0)
```

Evidence: `0x0073EE8D..0x0073EE93` pushes `0`, pushes `7`, calls vtable `+0x1E8`. The state returns `1`; it does not itself send `0x0E`.

**Active in YR:** Yes. Mission `7` is the Mission_Enter path used by the subsequent dock approach.

### 3.5 Mission_Enter sends `CAN_DOCK(0x0E)`

`FootClass::Mission_Enter @ 0x004D9290` checks the existing first radio contact/destination candidate. On the standard path, it sends:

```text
unit->Transmit_Radio(0x0E, target)
```

Evidence: `0x004D92B4..0x004D92B9` pushes target, pushes `0x0E`, calls vtable `+0x278`, then compares the reply to `1` at `0x004D92BF`.

If reply is neither `1` nor the unit's `+0x418` flag is set, Mission_Enter sends `0x03` to the first contact and calls vtable `+0x484` with `(0,1)` to clear/stop the attempted entry path. Evidence: `0x004D92C4..0x004D92E2`.

If reply is `1` or `+0x418 != 0`, Mission_Enter continues into NavCom/path handling. For CMIN with no current `+0x5A4` and queued destination entries, it may call vtable `+0x480`; if `+0x5A4` is already present, it proceeds to the default mission timer.

**Active in YR:** Yes. Stock CMIN/HARV Mission_Enter reaches this after state 3 queues mission `7`.

### 3.6 Building-side `0x0E` admission burst

`BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x0E` accepts stock DockUnload refineries by:

1. forwarding to `TechnoClass::Receive_Radio`;
2. checking power and service/bunker rejection gates;
3. ensuring the sender is or can become a contact;
4. sending `0x13 NEED_TO_MOVE` to the unit;
5. setting `*param_4 = this` temporarily;
6. computing the accepted cell as building anchor `+(3,1)`;
7. sending `0x12 MOVE_TO_CELL` with that `CellClass*`;
8. only if the `0x12` reply is `0x14 ALREADY_THERE`, sending `0x18 ENTER_DOCK`;
9. then sending `0x16 TIMING_SYNC`.

The accepted `+(3,1)` cell is hardcoded in case `0x0E`; it is not `QueueingCell=4,1` and not `DockingOffset%d`.

**Active in YR:** Yes for `[GAREFN]`/`[NAREFN]` because both have `DockUnload=yes` and `Refinery=yes`.

### 3.7 `0x12` usually sets NavCom unless already on the accepted cell

`FootClass::Receive_Radio @ 0x004D8FB0` case `0x12` compares the unit's current cell with the payload cell. If already there, it returns `0x14`.

If not already there, it calls vtable `+0x480` with the payload target and force flag `1`, then writes the mission dispatch timer triplet:

```text
+0xC8 = g_CurrentFrameCounter
+0xCC = target coord Y local
+0xD0 = 0
```

Because `+0xD0` is zero, the move update does not intentionally add a dispatch wait. The load-bearing effect is the NavCom destination and retry reset.

**Active in YR:** Yes. `0x12` is sent by stock refinery case `0x0E`.

### 3.8 `0x18` / `0x16` only happen after `0x12` says already there

The building sends `0x18` and `0x16` only after `0x12` returns `0x14`. Therefore the close-return timing has two possible visible shapes:

1. If the harvester is already on the accepted `+(3,1)` cell when `0x0E` runs, the same synchronous `0x0E` exchange proceeds through `0x18` then `0x16`.
2. If the harvester is not yet on that cell, `0x0E` returns after `0x12` and leaves the unit moving to the accepted cell; `0x18`/`0x16` occur on a later Mission_Enter/arrival attempt.

`TechnoClass::Receive_Radio @ 0x006F4AB0` case `0x18` sets byte `+0x418 = 1` before propagating `0x18`. `UnitClass::Receive_Radio @ 0x00737430` case `0x16` then calls `FootClass::Receive_Radio`, and if not chrono-gated and the facing timer is not already `0x4000`, calls the locomotor vtable `+0x4C` with `0x4000` and returns `1`.

**Active in YR:** Yes. The standard refinery burst sends `0x18` before `0x16`.

### 3.9 Physical pad arrival sends `0x15`

When the unit physically reaches the dock pad, `UnitClass::PerCellProcess @ 0x00739EC0` validates mission/destination/cell/locomotor gates, calls `FootClass::PerCellProcess(2)`, then sends `0x15` through vtable `+0x274`, then calls locomotor slot `+0x5C`.

`BuildingClass::Receive_Radio` case `0x15` checks `DockUnload=yes` and queues sender mission `0x10`. That `0x10` is a mission id, not a radio message.

**Active in YR:** Yes for stock CMIN/HARV dock arrival at GAREFN/NAREFN.

## 4. INI Keys

| INI path | Stock value | Effect | Active in YR |
|---|---:|---|---|
| `[General] HarvesterTooFarDistance` | `5` | Standard harvester close-return direct radio threshold, cells converted by `*0x100` | Yes |
| `[General] ChronoHarvTooFarDistance` | `50` | CMIN close-return direct radio threshold, cells converted by `*0x100` | Yes |
| `[CMIN] Dock` | `NAREFN,GAREFN` | Dock list searched by state 2 | Yes |
| `[CMIN] Harvester` | `yes` | Enables harvester mission path | Yes |
| `[CMIN] Teleporter` | `yes` | Selects chrono branch/threshold | Yes |
| `[HARV] Dock` | `NAREFN,GAREFN` | Standard harvester dock list | Yes |
| `[HARV] Harvester` | `yes` | Enables same state machine without chrono threshold | Yes |
| `[GAREFN]`, `[NAREFN] DockUnload` | `yes` | Building case `0x0E`/`0x15` refinery unload path | Yes |
| `[GAREFN]`, `[NAREFN] Refinery` | `yes` | Refinery identity for dock/unload behavior | Yes |
| `artmd.ini QueueingCell` | `4,1` | Far/wait fallback seed, not the accepted `0x0E` cell | Conditional |
| `artmd.ini [GAREFN] RemoveOccupy1` | `3,1` | Matches hardcoded accepted cell for GAREFN dock opening | Yes |

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `UnitClass::Mission_Harvest @ 0x0073E5E0` | close-return state 2 sends `0x02`, state 3 queues mission `7` | direct decompile and assembly | Yes |
| `RadioClass::Transmit_Radio_Impl @ 0x0065A970` | synchronous `0x02` contact establishment; generic dispatch for other messages | direct decompile | Yes |
| `RadioClass::Receive_Radio @ 0x0065A820` | receiver-side `0x02` ally/free-slot accept | direct decompile | Yes |
| `FootClass::Mission_Enter @ 0x004D9290` | per-mission driver sends `0x0E` | direct decompile and assembly | Yes |
| `BuildingClass::Receive_Radio @ 0x0043C2D0` | accepts `0x0E`, sends `0x13/0x12/0x18/0x16`; receives `0x15` and queues mission `0x10` | direct decompile | Yes |
| `FootClass::Receive_Radio @ 0x004D8FB0` | handles `0x12` already-there vs NavCom destination | prior direct decompile/report | Yes |
| `TechnoClass::Receive_Radio @ 0x006F4AB0` | `0x18` sets `+0x418`, `0x19` clears it | direct decompile | Yes |
| `UnitClass::Receive_Radio @ 0x00737430` | `0x16` performs `Do_Turn(0x4000)` / optional `0x15` cascade | direct decompile | Yes |
| `UnitClass::PerCellProcess @ 0x00739EC0` | physical pad arrival sends `0x15` then locomotor `+0x5C` | prior direct decompile/report | Yes |

## 6. Current Rust Implementation Status

No Rust was modified.

Rust has data for the same observed flow but models it as explicit miner/refinery state rather than raw RadioClass:

| Area | Current Rust reference | Status |
|---|---|---|
| Threshold parsing | `src/rules/ruleset.rs:958..959`; `src/sim/miner/mod.rs:229..230` | Present; stock rules supply `5` and `50` |
| Return handler | `src/sim/miner/miner_system.rs:587..665` | Close/far return and dock contact are modeled directly |
| Dock admission FSM | `src/sim/miner/miner_dock_sequence.rs:568..615` | Approach/accepted split approximates `HELLO`/admission |
| Accepted pad / pivot / unload | `src/sim/miner/miner_dock_sequence.rs:617..705` | Linked -> Pivoting -> Unloading captures the `0x18/0x16/0x15` result surface |
| Explicit dock phases | `src/sim/miner/mod.rs:86..127` | Present; comments already call out the verified `0x16` pivot |

Potential implementation implication for later work: do not collapse state-2 close return into immediate `0x0E`/pivot/unload. The binary has a distinct `0x02` contact step, then state `3`, then Mission_Enter `0x0E`, with `0x18/0x16` gated by `0x12 == 0x14`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Mission_Harvest` state 2 close threshold | verified | `0x0073E5E0`, `rulesmd.ini:293..294` | none |
| `Mission_Harvest` close radio send | verified | `0x0073EE54..0x0073EE68` | none |
| Radio `0x02` contact semantics | verified | `0x0065A970`, `0x0065A820` | none |
| State 3 mission queue | verified | `0x0073EE8D..0x0073EE93` | exact live frame of next dispatch needs runtime trace |
| Mission_Enter `0x0E` send | verified | `0x004D92B4..0x004D92BF` | none |
| Building case `0x0E` burst | verified | `0x0043C2D0` | queue eviction details assigned to slot 2 |
| `0x12` already-there gate | verified | `0x004D8FB0`; existing NAVCOM report | none for standard payload |
| `0x18` before `0x16` ordering | verified | `0x0043C2D0`, `0x006F4AB0`, `0x00737430` | none |
| Physical arrival `0x15` | verified | `0x00739EC0`; existing arrival report | exact runtime frame after drive arrival is live-debugger work |
| Far `QueueingCell` fallback | touched-not-exhausted | prior `MISSION_HARVEST_STATE2_TOOFAR...` | out of this close-return slot |
| Radio `0x10` | verified absent in this close path | `0x0043C2D0`, `0x00739EC0`, prior sender trace | no stock close-return sender found |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Does state 2 close return send `0x0E` directly? No. It sends `0x02` and writes substate `3` only on reply `1`. Evidence: `0x0073EE54..0x0073EE68`.

[RESOLVED] OQ-2 - What does state 3 do? It queues mission `7` via vtable `+0x1E8`; it does not itself transmit dock admission messages. Evidence: `0x0073EE8D..0x0073EE93`.

[RESOLVED] OQ-3 - Where does `0x0E` first fire? In `FootClass::Mission_Enter @ 0x004D9290`, via vtable `+0x278` at `0x004D92B9`. Active in YR: Yes.

[RESOLVED] OQ-4 - What is the first building-side accepted cell? Building anchor `+(3,1)` from case `0x0E`, sent by `0x12`; it is not `QueueingCell=4,1`. Evidence: `0x0043C2D0`, `artmd.ini:1773`, `artmd.ini:1795`.

[RESOLVED] OQ-5 - Does `0x18`/`0x16` happen unconditionally after `0x0E`? No. It happens only if the `0x12` MOVE_TO_CELL call returns `0x14`. Evidence: `0x0043C2D0`.

[RESOLVED] OQ-6 - What is the radio order before the dock sequence when already at the accepted cell? `0x02` in Mission_Harvest state 2, state 3 queues mission `7`, Mission_Enter sends `0x0E`, building sends `0x13`, `0x12`, `0x18`, `0x16`; later physical arrival/per-cell sends `0x15` if not already cascaded. Evidence: functions above.

[RESOLVED] OQ-7 - Is radio `0x10` part of this path? No. `0x10` appears as the mission id queued by building case `0x15`, not as a stock close-return radio message. Evidence: `0x0043C2D0` case `0x15`, `0x00739EC0` arrival report.

[DEFERRED] OQ-8 - Exact live frame separation between state 3 mission queue and first Mission_Enter dispatch. Category: needs-runtime-debugger. Static code proves the ordering, but not the exact frame in a live mission scheduler tick.

[DEFERRED] OQ-9 - Exact queue-eviction behavior when all refinery contact slots are occupied during the state-2 `0x02` or building `0x0E` contact check. Category: covered-by-other-slot. Slot 2 owns `BuildingClass::Receive_Radio 0x08/queue/eviction`.

## Sources

- Ghidra read-only decompile: `UnitClass::Mission_Harvest @ 0x0073E5E0`.
- Ghidra read-only assembly context: `0x0073EE54..0x0073EE68`, `0x0073EE8D..0x0073EE93`.
- Ghidra read-only decompile: `RadioClass::Transmit_Radio @ 0x0065AAA0`, `RadioClass::Transmit_Radio_Impl @ 0x0065A970`, `RadioClass::Receive_Radio @ 0x0065A820`.
- Ghidra read-only decompile and assembly context: `FootClass::Mission_Enter @ 0x004D9290`, especially `0x004D92B4..0x004D92E2`.
- Ghidra read-only decompile: `BuildingClass::Receive_Radio @ 0x0043C2D0`.
- Ghidra read-only decompile: `TechnoClass::Receive_Radio @ 0x006F4AB0`, `UnitClass::Receive_Radio @ 0x00737430`.
- Prior reports referenced: `CHRONO_MINER_MISSION_HARVEST_STATE2_RETURN_BRANCH_COORDS_GHIDRA_REPORT.md`, `MISSION_HARVEST_STATE2_TOOFAR_PATHFIND_BRANCH_GHIDRA_REPORT.md`, `FOOTCLASS_RECEIVE_RADIO_0X12_MOVE_FIELDS_NAVCOM_GHIDRA_REPORT.md`, `UNITCLASS_PERCELLPROCESS_DOCK_ARRIVAL_00739EC0_NAVCOM_GHIDRA_REPORT.md`, `UNITCLASS_0X418_DOCK_FLAG_LIFECYCLE_AND_CONSUMERS_GHIDRA_REPORT.md`.
- INI evidence: `ini/rulesmd.ini:293..294`, `ini/rulesmd.ini:7361`, `ini/rulesmd.ini:7364`, `ini/rulesmd.ini:7396`, `ini/rulesmd.ini:8225`, `ini/rulesmd.ini:8228`, `ini/rulesmd.ini:11726..11727`, `ini/rulesmd.ini:12519..12520`, `ini/artmd.ini:1773`, `ini/artmd.ini:1795`.
