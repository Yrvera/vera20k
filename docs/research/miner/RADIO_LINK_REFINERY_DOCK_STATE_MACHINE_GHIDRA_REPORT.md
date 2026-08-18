# Radio Link State Machine for Refinery Dock — Unified State Machine

**Investigation date:** 2026-05-20
**Confidence:** HIGH overall — every load-bearing claim cites a Ghidra MCP call from one of the 12 source reports
**Active in YR:** Yes — the entire system is live in every standard YR skirmish (per-case verdicts in §9)
**Source reports:** 12 reports written 2026-05-20 + 10+ prior-art docs corroborated (see §18)

---

> **Correction 2026-05-21 - stock refinery DockUnload / NavCom refinement**
>
> Later focused reports supersede several broad statements in this unified doc:
> `TECHNOCLASS_RECEIVE_RADIO_DOCK_CASES_NAVCOM_GHIDRA_REPORT.md`,
> `UNITCLASS_PERCELLPROCESS_DOCK_ARRIVAL_00739EC0_NAVCOM_GHIDRA_REPORT.md`,
> `CHRONO_MINER_FORCE_TRACK_0X47_EXIT_NAVCOM_STEP_GHIDRA_REPORT.md`,
> `STANDARD_REFINERY_0X2E4_WRITER_INVENTORY_GHIDRA_REPORT.md`, and
> `CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING_GHIDRA_REPORT.md`.
>
> Current verdict for stock `CMIN/HARV -> GAREFN/NAREFN`: radio `0x18` and `0x19`
> in `TechnoClass::Receive_Radio @ 0x006F4AB0` toggle byte `+0x418`, not
> reciprocal dock pointer `+0x2E4`. The stock DockUnload path does not establish a
> reciprocal `unit/building +0x2E4` link; it uses radio admission, `+0x418`,
> `+0x5A4`, `+0x6D1`, and the zero-`+0x2E4` `Mission_Deploy_Building` unload FSM.
> Radio `0x10` remains receiver-live on `BuildingClass` but has no meaningful
> standard CMIN refinery sender or TechnoClass handler. `ReleaseDockedHarvester`
> and `Force_Track(0x47)` are conditional reciprocal-link/interrupt-style release
> paths, not the normal stock zero-link DockUnload completion path.
>
> The 2026-05-21 nav/radio/state-machine re-swarm adds these refinements:
> stock return sends `HELLO(0x02)` before `Mission_Enter`; `Mission_Enter`
> sends `CAN_DOCK(0x0E)`; building case `0x0E` replies `0x13/0x12` and only
> later `0x18/0x16` after the accepted-cell check; pad arrival sends `0x15`;
> stock inbound refinery docking does **not** send `0x0C`; case `0x08` does
> not return `0x17 QUEUED` for stock refineries; incoming full HELLO returns
> `NEGATORY(10)` rather than evicting the refinery contact; and
> `ReleaseDockedHarvester` directly sends only `0x03 BREAK`. See
> `BUILDING_RECEIVE_RADIO_0X08_CLEARANCE_QUEUE_GHIDRA_REPORT.md`,
> `MISSION_ENTER_DOCK_ARRIVED_0X0C_GHIDRA_REPORT.md`,
> `MISSION_HARVEST_STATE2_CLOSE_RETURN_RADIO_TIMING_GHIDRA_REPORT.md`,
> `BUILDINGCLASS_FIELD_0X2E4_REFINERY_DOCK_GHIDRA_REPORT.md`, and
> `RELEASEDOCKEDHARVESTER_EXIT_RADIO_0X19_GHIDRA_REPORT.md`.
>
> **Correction 2026-05-22 - follow-up closure**
>
> Three follow-up reports should be treated as canonical for their slices:
> `STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_REACHABILITY_GHIDRA_REPORT.md`,
> `CHRONO_MINER_REFINERY_CONTACT_SATURATION_QUEUE_EVICTION_GHIDRA_REPORT.md`,
> and `miner/traces/CHRONO_MINER_FULL_CARGO_CLOSE_RETURN_MISSION_DISPATCH_TIMING_TRACE.md`.
> They confirm that stock unload completion is the zero-`+0x2E4` state-4 path,
> receiver-side full `HELLO(0x02)` returns `NEGATORY(10)` without evicting, sender-side
> HELLO eviction only evicts the sender's old `Contacts[0]`, and `QueueingCell=4,1`
> is fallback/staging data after non-acceptance or too-far return, not the accepted
> `0x0E` cell. The trace also finds current Rust mismatches in close-return threshold
> math, early `0x02 HELLO`, and collapsed mission/radio timing; do not use this
> document's older "current design is valid" paragraph as an implementation verdict.
>
> **Correction 2026-05-24 - dock admission timing reswarm**
>
> The current stock entry point is
> `miner/STOCK_REFINERY_DOCK_UNLOAD_STATE_MACHINE_CURRENT_SYSTEM_MODEL_SYNTHESIS.md`.
> Treat older wording below that says Mission Enter retries `0x0E` "per tick",
> that shows `0x15` as only a PerCellProcess/pad source, or that implies `0x16`
> always means unload handoff as stale. Verified current split: one
> `FootClass::Mission_Enter @ 0x004D9290` dispatch sends one `0x0E` and returns
> stock `14..16` frames; building sends `0x18/0x16` only after `0x12` returns
> `0x14`; first ordinary `0x16` may be sync-only; later/already-synced `0x16`
> can send `0x15` without `GetDockCoord` equality. Normal stock exit remains the
> zero-link `Mission_Deploy_Building` state-4 path.
>
> **Correction 2026-07-11 - live-binary cadence and accumulator ordering audit**
>
> `FootClass::Mission_Enter @ 0x004D9290` sends one `CAN_DOCK(0x0E)` per
> scheduled mission dispatch and returns the mission-timer entry plus
> `Random(0,2)`; it is not an every-tick resend loop. Radio `0x15` can be sent
> both by `UnitClass::PerCellProcess @ 0x00739EC0` and by
> `UnitClass::Receive_Radio @ 0x00737430` case `0x16` after its facing/ready
> gates. Most importantly, although `UnitClass::Unlimbo @ 0x00737BA0` can write
> `Random(0,29)` to `unit+0xF8`, accepted stock unload initialization later
> writes `unit+0xF8 = 0` at `0x0073DFD0`, then sets unload state 3 at
> `0x0073E093`; the Unlimbo seed does not jitter stock refinery unload cadence.
> (Corrected via `decompile_function 0x004D9290`, `decompile_function
> 0x00737430`, `disassemble_function 0x00739EC0`, `decompile_function
> 0x00737BA0`, and `disassemble_function 0x0073D630` -
> `OPERATOR_OR_ORDER_DRIFT` / `INFERENCE_HARDENED`.)

## 1. Goal & Scope

Four top-level questions from the investigation plan:

| # | Question | Status |
|---|----------|--------|
| Q1 | What radio messages does the harvester send to the refinery, and when? | ANSWERED — §4, §5, §10 |
| Q2 | What radio messages does the refinery send back to the harvester, and in what order? | ANSWERED — §4, §5, §10 |
| Q3 | Which message codes are dead (TS-legacy receiver with no live sender)? | ANSWERED — §4, §9; especially 0x10 |
| Q4 | What are the exact field writes (`+0x418` radio/contact byte, conditional `+0x2E4` link, anim slots, contact array) at each stage? | ANSWERED — §7, §11, §12 |

All four questions have been answered. No fundamental unknowns remain; several low-priority open questions are listed in §16.

---

## 2. System Overview

When a harvester (UnitClass) returns to a refinery (BuildingClass) with ore, the dock sequence proceeds in three broad phases: (a) approach, where each scheduled `FootClass::Mission_Enter` dispatch issues one `CAN_DOCK(0x0E)` and returns the next mission delay; (b) deposit, where `UnitClass::Mission_Deploy_Building` runs a 4-state FSM on the harvester side draining ore one storage slot per ~15-frame gate crossing; and (c) stock zero-link exit, where `Mission_Deploy_Building` state 4 clears `unit+0x6D1`, may send BREAK(0x03), and returns the unit to Harvest scheduling without `ReleaseDockedHarvester`, `UndockUnit`, or `Force_Track(0x47)`. Throughout, the radio protocol (RadioClass) acts as the synchronous RPC glue: the harvester is the primary initiator and the refinery is the primary responder, but the building side sends four sequential messages (0x13→0x12→0x18→0x16) as a reply burst inside a single `Receive_Radio` invocation. The actual ore drain and credit logic live entirely in `UnitClass::Mission_Deploy_Building` on the harvester side — the building's per-tick mission plays no role in the standard DockUnload refinery path. (Corrected 2026-07-11 via `decompile_function 0x004D9290`: one vtable+`0x278` send of `0x0E`, followed by returned mission-timer entry + `Random(0,2)` — `OPERATOR_OR_ORDER_DRIFT`.)

---

## 3. Class Hierarchy & Vtable Binding

### 3.1 Receive_Radio dispatch (vtable+0x194)

All entries verified by live `read_memory` on the binary in source reports #1 and #2.

| Class | Vtable base | +0x194 address | Function |
|-------|-------------|----------------|----------|
| RadioClass | `0x007F0508` | `0x007F069C` → `20 A8 65 00` | `RadioClass::Receive_Radio` @ 0x0065A820 |
| TechnoClass | `0x007F4960` | `0x007F4AF4` → `B0 4A 6F 00` | `TechnoClass::Receive_Radio` @ 0x006F4AB0 |
| FootClass | `0x007E8C94` | `0x007E8E28` → `B0 8F 4D 00` | `FootClass::Receive_Radio` @ 0x004D8FB0 |
| **InfantryClass** | `0x007EB058` | `0x007EB1EC` → `B0 8F 4D 00` | **Inherits FootClass** — no separate override |
| UnitClass | `0x007F5C70` | `0x007F5E04` → `30 74 73 00` | `UnitClass::Receive_Radio` @ 0x00737430 |
| BuildingClass | `0x007E3EBC` | `0x007E4050` → `D0 C2 43 00` | `BuildingClass::Receive_Radio` @ 0x0043C2D0 |
| AircraftClass | `0x007E22A4` | `0x007E2438` → `B0 90 41 00` | `AircraftClass::Receive_Radio` @ 0x004190B0 |

### 3.2 Transmit-side slots (+0x274–+0x280)

No subclass overrides any transmit slot. All four are RadioClass-only. Verified by `read_memory` on RadioClass, BuildingClass, and UnitClass vtables (#2).

| Slot | Address (RadioClass vtable) | Function | Address |
|------|-----------------------------|----------|---------|
| +0x274 | `0x007F077C` → `B0 AC 65 00` | `RadioClass::Transmit_Radio_ToFirst` | 0x0065ACB0 |
| +0x278 | `0x007F0780` → `A0 AA 65 00` | `RadioClass::Transmit_Radio` | 0x0065AAA0 |
| +0x27C | `0x007F0784` → `70 A9 65 00` | `RadioClass::Transmit_Radio_Impl` | 0x0065A970 |
| +0x280 | `0x007F0788` → `E0 AC 65 00` | `RadioClass::Broadcast_Radio_ToAll` | 0x0065ACE0 |

### 3.3 vtable+0x480 — Set_Destination per class

| Class | Slot+0x480 addr | Function |
|-------|-----------------|----------|
| TechnoClass | `0x007F4DE0` → 0x00709A30 | Stub `{ return; }` — no-op |
| FootClass | `0x007E9114` → **0x004D94B0** | `FootClass::Set_Destination_Internal` — writes `+0x5A4` with chrono/loco guards; calls `ILocomotion::Head_To_Coord`. Verified via `read_memory 0x007E9114` = `b0 94 4d 00` (2026-05-24 audit). Prior `0x004D94A0` was off-by-one slot — that is a different `+0x5A0` setter, not the vtable+0x480 slot. |
| **UnitClass** | `0x007F60F0` → **0x00741970** | **Real harvester dock initiator** |
| AircraftClass | `0x007E2724` → 0x0041AA80 | Aircraft-specific dock handler (mislabeled in Ghidra as `UnitClass::EnterBuildingOrDock`) |

(#6, #7; `read_memory` on each slot address)

---

## 4. Radio Message Catalog (in refinery-dock context)

The refinery dock chain uses **9 live message codes** (0x02, 0x03, 0x0E, 0x12, 0x13, 0x15, 0x16, 0x18, and conditional 0x19). All others are dead-send, aircraft-only, or out-of-scope for standard DockUnload buildings. `0x18` is sent inside the case `0x0E` reply burst after `0x12` returns `0x14`; its TechnoClass propagation sets both endpoints' `+0x418`. A later state-4 BREAK can therefore trigger the conditional TechnoClass `0x19` cascade, even though state 4 does not directly send `0x19`. (Corrected 2026-07-11 via `decompile_function 0x0043C2D0`, `decompile_function 0x006F4AB0`, `decompile_function 0x0065A970`, and `disassemble_function 0x0073D630` at `0x0073E268..0x0073E279` — `INFERENCE_HARDENED`.)

| Code | Name | Sender(s) in dock chain | Receiver handler | Active in YR refinery chain | Notes |
|------|------|------------------------|------------------|-----------------------------|-------|
| 0x02 | HELLO | `Mission_Harvest` state 2 (harvester→refinery) | `RadioClass::Receive_Radio` HELLO case — adds sender to Contacts[] | YES | Ally check + idempotent; Contacts[] slot allocated here |
| 0x03 | BREAK/OVER_AND_OUT | `Mission_Deploy_Building` state-4 stock exit; `TechnoClass::Set_Destination` cancel-dock path; conditional `ReleaseDockedHarvester` / `UndockUnit` reciprocal-link cleanup | `BuildingClass` case 0x03 → GrandOpening + TechnoClass → RadioClass → null Contacts[] | YES | TechnoClass BREAK also conditionally sends 0x19 to harvester first |
| 0x07 | DOCKING_COMPLETE | **Carryall pickup path only in verified direct sender inventory** (`AircraftClass::Mission_Move_Carryall`) | `UnitClass` case 0x07 delegates to Foot/Techno then clears destination/path/mission | YES (conditional carryall); NO for refinery | Not sent by standard DockUnload or `BuildingClass::MissionRepairAndProduce`; see `RADIO_0X07_DOCKING_COMPLETE_SENDER_AND_CASE7_REACHABILITY_GHIDRA_REPORT.md` |
| 0x0E | CAN_DOCK | `UnitClass::Set_Destination` @ 0x00741DDA (one-shot initial); `FootClass::Mission_Enter` @ 0x004D92B9 (one send per scheduled dispatch) | `BuildingClass` case 0x0E — full reply chain (0x13→0x12→0x18→0x16) | YES | Primary dock admission message; corrected 2026-07-11 via `decompile_function 0x004D9290` — `OPERATOR_OR_ORDER_DRIFT` |
| 0x0F | CAN_ENTER | Aircraft→building (helipad/repair); unit→transport | `BuildingClass` case 0x0F — multi-gate passenger/garrison path | YES (shared with garrison/helipad); NO for harvester→refinery | Not used in DockUnload harvester path |
| 0x10 | RESERVE_DOCK | **No sender found in binary** (exhaustive scan of 10 functions) | `BuildingClass` case 0x10 — returns ROGER for Refinery=yes / UnitRepair / Weeder | NO — receiver is live but unreachable | TS-legacy dead-send; see §9 |
| 0x12 | MOVE_TO_CELL | Refinery→harvester inside case 0x0E reply burst | `FootClass` case 0x12 — sets destination, writes 3-word timestamp at +0xC8 | YES | Returns 0x14 (ALREADY_THERE) if already at target cell |
| 0x13 | NEED_TO_MOVE | Refinery→harvester inside case 0x0E reply burst (probe before 0x12) | `FootClass` case 0x13 — checks chrono state; writes chrono dest into `*payload` | YES | Also used as gate: if chrono still moving, returns NEGATORY |
| 0x15 | TIMING_SYNC_BACK / DOCK_NOW | `UnitClass::PerCellProcess` (pad/contact paths) and `UnitClass::Receive_Radio` case 0x16 (later/already-facing ready path), harvester→refinery | `BuildingClass` case 0x15 — for DockUnload: `sender.Queue_Mission(Enter=0x10, 0)` | YES | Triggers harvester `Mission_Enter` → deposit starts; corrected 2026-07-11 via `disassemble_function 0x00739EC0` (`0x0073A503`, `0x0073A5C4`) and `decompile_function 0x00737430` case 0x16 — `INFERENCE_HARDENED` |
| 0x16 | TIMING_SYNC | Refinery→harvester inside case 0x0E reply burst (after 0x18) | `UnitClass` case 0x16 → faces harvester (RateTimer 0x4000 or Set_Facing); cascade sends 0x15 back | YES | ILocomotion vtable+0x4C dispatch differs: Drive→Do_Turn, Walk→Set_Facing |
| 0x17 | QUEUED | Sent by `BuildingClass` case 0x08 (factory/repair queues) | `UnitClass` has no explicit handler — falls through to FootClass | YES-shared (factory queues); not in refinery chain | Harvester treats non-ROGER as OVER_AND_OUT in `Set_Destination` |
| 0x18 | ENTER_DOCK | Refinery→harvester inside case 0x0E reply burst | `TechnoClass` case 0x18: sets byte `this+0x418 = 1`, propagates 0x18 | YES | Runtime radio/contact state; not reciprocal `+0x2E4` |
| 0x19 | LEAVE_DOCK | (a) `TechnoClass::Receive_Radio` case 0x03 BREAK (conditional); (b) `UnitClass::Set_Destination` cancel-dock path @ 0x00741B97 | `TechnoClass` case 0x19: clears byte `this+0x418 = 0`, propagates 0x19 | YES (conditional) | Stock state 4, `ReleaseDockedHarvester`, and `UndockUnit` do not directly send 0x19; their BREAK can indirectly trigger TechnoClass's conditional cascade. Corrected 2026-07-11 via `decompile_function 0x006F4AB0` and `disassemble_function 0x0073D630` — `INFERENCE_HARDENED`. |
| 0x22 | IS_REPAIRING | Building sends to query unit health (in case 0x0E filter) | `ObjectClass` case 0x22 — health/MaxHP ratio vs ConditionYellow | YES (as transmit in 0x0E filter); no building-side receive handler | |
| 0x23 | IS_OCCUPIED | Building sends to query occupancy (case 0x0F) | `FootClass` case 0x23 — checks if unit is inside a building cell | YES (as transmit in 0x0F); no building-side receive handler | |
| 0x24 | DOCK_QUERY | Building→unit (generic "are you dockable?") | `UnitClass` case 0x24 — checks cloaked, cell flags, mission state, has-destination | YES | Returns 0xA if unit busy; 1 or 10 based on `+0x684` |

AircraftClass-only (not in harvester-refinery path): 0x08, 0x0B (aircraft/factory), 0x0D, 0x1D, 0x1F, 0x21.
TS-vestigial/unhandled: 0x2C, 0x32 (fall through entire chain to ObjectClass default, return 0).

---

## 5. The Refinery Dock Cycle

### 5.1 Mermaid sequence diagram (approach → link → deposit → exit)

```mermaid
sequenceDiagram
    participant H as Harvester (UnitClass)
    participant R as Refinery (BuildingClass)
    participant RC as RadioClass state

    H->>R: 0x02 HELLO (Mission_Harvest state 2)
    R-->>RC: Contacts[i] = H
    R-->>H: ROGER (1)
    note over H: SetMission(Mission_Enter=7)

    loop scheduled Mission_Enter dispatches until dock accepted
        H->>R: 0x0E CAN_DOCK (Set_Destination @ 0x741DDA / Mission_Enter)
        R->>H: 0x13 NEED_TO_MOVE (probe — inside case 0x0E reply)
        H-->>R: ROGER (FootClass case 0x13)
        R->>H: 0x12 MOVE_TO_CELL, payload=accepted cell (NW+3, NW+1)
        H-->>R: 0x14 ALREADY_THERE or ROGER (FootClass case 0x12)
        alt 0x12 returned ALREADY_THERE
            R->>H: 0x18 ENTER_DOCK (TechnoClass sets +0x418=1)
            R->>H: 0x16 TIMING_SYNC (H faces east, RateTimer=0x4000)
        else still moving
            note over H,R: no 0x18/0x16 until accepted cell reached
        end
    end

    H->>R: 0x15 TIMING_SYNC_BACK (PerCellProcess or ready case-0x16 cascade)
    R-->>H: Queue_Mission(Enter=0x10)
    note over H: Enters Mission_Deploy_Building FSM

    loop per dump attempt (~15 accumulator units)
        note over H: particle emitter → SetAnimSlot(10)
        note over H: remove entire first nonempty resource slot, if any
        note over H: HouseClass::Add_Tiberium_Credits
    end

    note over H,R: Next threshold after last drain sees no nonempty slot → state 4
    H->>R: 0x03 BREAK (Mission_Deploy_Building state 4 exit)
    R->>H: conditional 0x19 LEAVE_DOCK cascade when both +0x418 bytes are set
    R-->>RC: Contacts[i] = NULL
    note over H,R: state 4 directly sends BREAK only; TechnoClass may propagate 0x19
```

### 5.2 ASCII state machines

**Harvester side (mission + substate):**

```
Mission_Harvest state 2
  HELLO(0x02) → ROGER
    SetMission(Mission_Enter=7)
      → scheduled Mission_Enter dispatch loop
          Set_Destination → Transmit CAN_DOCK(0x0E)
            NOT ROGER → OVER_AND_OUT + destination clear; later dispatch only if still scheduled
            ROGER → Contacts established, drive to accepted cell
      → PerCellProcess (cell-level) or ready UnitClass case 0x16
          Transmit TIMING_SYNC_BACK(0x15) to refinery
            Building replies Queue_Mission(Enter=0x10)
              → Mission_Deploy_Building (substate machine)
                  substate 0: not used in harvester path
                  substate 3 init: SetAnimSlot(7), reset accumulator
                  substate 3 loop: [per threshold attempt] particle+slot10; drain one nonempty resource slot if present
                  final empty attempt: FindFirstNonEmptySlot=-1 → slot8+clear slot10+state4
                  substate 4: depart guard (building+0x57C==0)
                    SetMission(Harvest=10), optional BREAK(0x03)
                    → ReleaseDockedHarvester only on conditional nonzero +0x2E4 branch
```

**Refinery side (Contacts[] + anim slots):**

```
Idle: Contacts[0..N-1] = NULL, Anims normal idle

CAN_DOCK received (case 0x0E):
  Power check, ally check, capacity check
  → NEED_TO_MOVE probe (0x13) → MOVE_TO_CELL (0x12) → ENTER_DOCK (0x18) → TIMING_SYNC (0x16)
  Contacts[free_slot] = harvester

TIMING_SYNC_BACK received (case 0x15):
  Queue_Mission(Enter=0x10) on harvester

Deposit in progress (harvester drives Mission_Deploy_Building):
  Anim slot 7 fires once (PreProductionAnim, if defined)
  Anim slot 10 is attempted per threshold crossing, including the final empty check (SpecialAnim)
  Anim slot 8 fires when the later final-empty attempt observes no nonempty resource slot (ProductionAnim)

BREAK received (case 0x03):
  GrandOpening() → reset idle anim
  TechnoClass BREAK → Contacts[slot] = NULL
```

### 5.3 Stage-by-stage table

| Stage | Harvester state | Refinery state | Radio messages | Field writes |
|-------|----------------|----------------|----------------|--------------|
| 1. Player orders harvester to refinery | Mission_Harvest state 3 → SetMission(Enter=7) | Idle | — | — |
| 2. Mission_Harvest state 2 sends HELLO | En route | Idle | HELLO(0x02) H→R | R.Contacts[i]=H |
| 3. Mission_Enter scheduled dispatch | Mission_Enter(7) | Idle | One CAN_DOCK(0x0E) H→R per dispatch; function returns mission timer + Random(0,2) | —; corrected 2026-07-11 via `decompile_function 0x004D9290` — `OPERATOR_OR_ORDER_DRIFT` |
| 4. Refinery accepts CAN_DOCK | Driving to accepted cell | Contacts active | 0x13 NEED_TO_MOVE -> 0x12 MOVE_TO_CELL; 0x18 ENTER_DOCK -> 0x16 TIMING_SYNC only after 0x12 returns ALREADY_THERE | H.+0x418=1 after 0x18; H.RateTimer=0x4000 after 0x16; H.destination=accepted cell |
| 5. Harvester handoff becomes ready | PerCellProcess pad/contact path or ready case-0x16 cascade | Contacts active | TIMING_SYNC_BACK(0x15) H→R | R triggers **H.Queue_Mission(Enter=0x10)** via `param_2->vtable+0x1E8` — mission queued on the harvester (sender), not the refinery. Corrected 2026-07-11 via `disassemble_function 0x00739EC0`, `decompile_function 0x00737430`, and `decompile_function 0x0043C2D0` — `INFERENCE_HARDENED`. |
| 6. Building queues harvester Enter | Enters Mission_Deploy_Building(0x10) | No refinery mission transition from DockUnload case 0x15 | none | H.substate=3 init; SetAnimSlot(7). Corrected 2026-07-11 via `decompile_function 0x0043C2D0`: vtable+0x1E8 receiver is `param_2` (sender/harvester) — `PARAM1_TYPE_MISREAD`. |
| 7. Per-bale drain loop | Mission_Deploy_Building substate 3 | Mission queued | none (radio silent) | particle; SetAnimSlot(10); StorageClass drain; credits |
| 8. Final empty gate after all slots drained | Mission_Deploy_Building substate 3 until the next threshold crossing, then substate 4 | | none | The drain path resets `+0xF8`; only a later threshold attempt where `StorageClass::FindFirstNonEmptySlot @ 0x006C9820` returns -1 calls slot 8, clears slot 10, and writes state 4. Corrected 2026-07-11 via `decompile_function 0x006C9820` and `disassemble_function 0x0073D630` at `0x0073E355..0x0073E539` — `OPERATOR_OR_ORDER_DRIFT`. |
| 9. Normal stock zero-link exit | Return/continue Harvest scheduling | Idle/Guard as applicable | BREAK(0x03) may be sent if contact remains; that BREAK conditionally cascades 0x19 when both endpoints still have +0x418 set | Contacts and +0x418 state clear; no normal stock reciprocal `unit/building +0x2E4` teardown. Corrected 2026-07-11 via `disassemble_function 0x0073D630`, `decompile_function 0x0065A970`, and `decompile_function 0x006F4AB0` — `INFERENCE_HARDENED`. |
| 9b. Conditional reciprocal-link release | Nonzero `unit+0x2E4` branch | SetMission(Guard=5) | `ReleaseDockedHarvester` directly sends BREAK(0x03) only | Clears reciprocal `+0x2E4`; any 0x19 is indirect TechnoClass BREAK cascade if both sides have +0x418 |
| 10. Interrupt exit (sell/destroy) | — | — | BREAK(0x03) via UndockUnit | same zeroing + Force_Track(0x47) |

---

## 6. Function Inventory & Coverage

Functions decompiled in this investigation, in tick-pipeline order:

| Address | Name | Role in dock chain | Source |
|---------|------|--------------------|--------|
| 0x0065A820 | `RadioClass::Receive_Radio` | Base HELLO/BREAK contact management | #1 |
| 0x0065A970 | `RadioClass::Transmit_Radio_Impl` | Core vtable+0x194 dispatch, HELLO eviction | #1 |
| 0x0065AAA0 | `RadioClass::Transmit_Radio` | Public wrapper (+g_RadioScratchBuffer) | #1 |
| 0x0065ACB0 | `RadioClass::Transmit_Radio_ToFirst` | Contacts[0]-only transmit | #1 |
| 0x0065AD90 | `RadioClass::FindDockSlot` | Lookup: is target in Contacts[]? Returns index or -1 | #2 |
| 0x0065ADF0 | `RadioClass::FindFreeContactSlot` | Capacity gate: free slot or target already there? | #2 |
| 0x005F5320 | `ObjectClass::Receive_Radio` | Terminal fallback: handles only 0x0D and 0x22 | #2 |
| 0x0043C2D0 | `BuildingClass::Receive_Radio` | Refinery-side 9-case receiver | #3 |
| 0x00737430 | `UnitClass::Receive_Radio` | Harvester-side 8-case receiver | #4 |
| 0x004D8FB0 | `FootClass::Receive_Radio` | Shared 6-case receiver (Unit+Infantry+Aircraft) | #5 |
| 0x0041AA80 | `AircraftClass::Set_Destination` | Aircraft dock initiator (Ghidra mislabeled as `UnitClass::EnterBuildingOrDock`) | #6 |
| 0x00741970 | `UnitClass::Set_Destination` (vtable+0x480) | Real harvester CAN_DOCK sender @ 0x741DDA | #7 |
| 0x004D9290 | `FootClass::Mission_Enter` | Scheduled approach dispatch; sends one 0x0E and returns mission timer + Random(0,2) | Corrected 2026-07-11 via `decompile_function 0x004D9290` — `OPERATOR_OR_ORDER_DRIFT` |
| 0x004595C0 | `BuildingClass::ReleaseDockedHarvester` | Conditional nonzero-`+0x2E4` release helper; directly sends BREAK(0x03), not 0x19 | 2026-05-21 re-swarm |
| 0x004593A0 | `BuildingClass::UndockUnit` | Interrupt exit: sell/destroy/temporal wipe | #8 |
| 0x0073D630 | `UnitClass::Mission_Deploy_Building` | Harvester-side deposit FSM (states 0/1/3/4) | #9 |
| 0x0044EFB0 | `BuildingClass::GetDockCellForObject` | Production exit oracle (NOT harvester pad) | #10 |
| 0x00451890 | `BuildingClass::CreateAnimForSlot` | Allocate/replace AnimClass in slot array | #10 |
| 0x00451750 | `BuildingClass::SetAnimSlotImage` | Select art variant → call CreateAnimForSlot | #10 |
| 0x00451E40 | `BuildingClass::ClearAnimSlot` | Destroy AnimClass in slot; arg -2 = clear all 21 | #10 |
| 0x004DFCB0 | `FootClass::Find_Nearest_Dock` | Nearest refinery search (Mission_Harvest state 2) | #11 |
| 0x00447B20 | `BuildingClass::GetDockCoord` | Physical dock coordinate for approach | #11 |
| 0x0073E5E0 | `UnitClass::Mission_Harvest` | Outer harvest FSM; state 3 → SetMission(Enter=7) | #11 |
| 0x00740EF0 | `UnitClass::Mission_Unload` | Weeder/carryall unload path; NOT standard refinery | #11 |
| 0x006AF6C0 | `SlaveManagerClass::AI_Update` | Slave Miner slave logic — NO connection to refinery | #11 |

**Functions NOT investigated** (out of scope or deferred):

- `UnitClass::PerCellProcess` @ 0x00739EC0 — original swarm deferred it; the 2026-07-11 audit decompiled/disassembled it and verified two `0x15` send sites at `0x0073A503` (ToFirst) and `0x0073A5C4` (targeted). (`disassemble_function 0x00739EC0` — `INFERENCE_HARDENED` correction.)
- `TechnoClass::Receive_Radio` @ 0x006F4AB0 — original swarm deferred it; the 2026-07-11 audit decompiled it and verified `0x18`/`0x19` set/clear propagation plus the conditional BREAK→`0x19` cascade. (`decompile_function 0x006F4AB0` — `INFERENCE_HARDENED` correction.)
- `BuildingClass::MissionRepairAndProduce` — UnitRepair path only; confirmed not used in DockUnload chain (#9 §Q2)
- `UnitClass::Mission_Unload` @ 0x00740EF0 — Weeder/carryall narrow path; decompiled but not the standard ore path (#11 §2.6)

---

## 7. Class Layout — Critical Field Offsets

All offsets verified from source docs as cited.

| Class | Offset | Field | Type | Source |
|-------|--------|-------|------|--------|
| RadioClass | +0xD4 | `RadioHistory[0]` (most recent msg code) | int | #1 |
| RadioClass | +0xD8 | `RadioHistory[1]` | int | #1 |
| RadioClass | +0xDC | `RadioHistory[2]` (oldest) | int | #1 |
| RadioClass | +0xE0 | `Contacts.vtable` | DynamicVectorClass vtable* | #2 |
| RadioClass | +0xE4 | `Contacts.data` | TechnoClass** array | #1, #2 |
| RadioClass | +0xE8 | `Contacts.Capacity` (iteration bound) | int | #1, #2 |
| RadioClass | +0xEC | `Contacts.CanGrow` | byte (1) | #2 |
| RadioClass | +0xED | `Contacts.Initialized` | byte (1) | #2 |
| FootClass | +0x388 | `PrimaryFacing RateTimer` **YELLOW (Unverified, 2026-05-24 audit):** storage at +0x388 inferred from source #4 case 0x16 (UnitClass calls `ILocomotion::vtable+0x4C(locomotor, 0x4000)` on `FootClass+0x674`) + `Do_Turn @ 0x004B0EF0` chain; `decompile_function 0x004B0EF0` shows `RateTimer__Set` called on a parameter-stack reference, not directly on a `+0x388` field — the timer's actual storage (FootClass+0x388 vs inside DriveLocomotionClass) remains unconfirmed. Informational; not load-bearing for the dock FSM. | RateTimer16 | #4 (case 0x16) |
| FootClass | +0x418 | **STALE LABEL**: radio/contact dock-state byte set by `0x18` and cleared by `0x19`, not NavCom destination | bool | 2026-05-21/22 follow-ups |
| FootClass | +0x5A4 | chrono destination pointer | ptr/int | #5 (cases 0x13, 0x17, 0x1C) |
| FootClass | +0x674 | `ILocomotion*` COM pointer | ILocomotion* | #5 |
| FootClass | +0x6AF | chrono-teleporting flag | byte | #5 |
| FootClass | +0x598 | dock-queue count (`param_1[0x166]`) | int | #8 §2 |
| FootClass | +0x58C | dock-queue array pointer | ptr | #8 §2 |
| FootClass | +0xB4 | team/sub-mission state field | int | #5 (cases 0x11, 0x12) |
| FootClass | +0xC8..+0xD0 | 12-byte frame timestamp (case 0x12 write) | 3×int | #5 |
| TechnoClass | +0x418 | radio/contact dock-state byte set by `0x18`, cleared by `0x19` | byte | 2026-05-21 re-swarm |
| Unit/BuildingClass | +0x2E4 | conditional reciprocal link field, not stock refinery DockUnload state | ptr/field | 2026-05-21 re-swarm |
| UnitClass | +0x350 | unload destination coordinate | coord | #4 (case 0x15) |
| UnitClass | +0x6D1 | "first-entry-done" flag (deposit FSM) | byte | #9 |
| UnitClass | +0xBC | deposit FSM substate (0/1/3/4) | int | #9 |
| UnitClass | +0xF8 | bale-rate accumulator | int | #9 |
| BuildingClass | +0x55C | `Anims_0[0]` base (21 slots × 4 bytes) | AnimClass*[21] | #10 |
| BuildingClass | +0x578 | `Anims_0[7]` (PreProductionAnim) | AnimClass* | #10 |
| BuildingClass | +0x57C | `Anims_0[8]` (ProductionAnim) — also used as state-4 guard | AnimClass* | #9, #10 |
| BuildingClass | +0x584 | `Anims_0[10]` (SpecialAnim) | AnimClass* | #9, #10 |
| BuildingClass | +0x6DD | dock-anim trigger flag | byte | #3 (case 0x15) |
| BuildingClass | +0x718 | dock-state/unloading-in-progress flag | int | #8 |
| BuildingClass | +0x81 | lockout flag (case 0x10 gate) | byte | #3 |
| BuildingClass | +0x118 | current passenger/occupant count | int | #3 |
| UnitTypeClass | +0x5E0 | `DockRange` | int | #4 |
| UnitTypeClass | +0xE0E | `Harvester=yes` flag | bool | #9 |
| UnitTypeClass | +0xE0F | `Weeder=yes` flag | bool | #4 (case 0x17), #9 |
| BuildingTypeClass | +0x16B3 | `DockUnload=yes` | bool | #3, #7 |
| BuildingTypeClass | +0x16BB | `Refinery=yes` (corrected 2026-05-20) | bool | #11, #12 |
| BuildingTypeClass | +0x1780 | `NumberOfDocks=` | int | #2 |
| BuildingTypeClass | +0x1788 | `DockingOffset%d` array | coord array | prior docs |

---

## 8. INI Keys & TypeClass Flags

| INI key | Section | Default | Offset | Effect in dock chain |
|---------|---------|---------|--------|---------------------|
| `DockUnload=` | [BuildingType] | no | +0x16B3 | Identifies standard refinery path (case 0x0E selects queue cell, case 0x15 sends harvester to Mission_Enter) |
| `Refinery=` | [BuildingType] | no | +0x16BB | Case 0x10 ROGER gate; state-4 anim slot 8 gate; GetDockCoord path |
| `UnitRepair=` | [BuildingType] | no | +0x16A9 | Service Depot path; shared with refinery for some radio cases |
| `Weeder=` | [BuildingType] | no | +0x16BC | Weeder refinery path (cases 0x0E, 0x10, 0x15, 0x17) |
| `NumberOfDocks=` | [BuildingType] | 1 | +0x1780 | Sets Contacts[] array capacity via `Set_Contact_Count` at ctor time |
| `QueueingCell=X/Y` | [BuildingType] | (n/a) | +0x1618/+0x161C | **NOT read** in `Receive_Radio` case 0x0E — queue cell is hardcoded `(NW+3, NW+1)` |
| `DockingOffset%d=` | [Art] | — | +0x1788 array | Used by `GetDockCoord` for helipad/UnitRepair-style dock-coordinate consumers. It does **not** initialize `DAT_0089F6A0`; that global is the hardcoded direction-table west offset `(-1,0)` from `Foundation_direction_table_init @ 0x0049F2F0`. |
| `HarvesterDumpRate=` | [General] | 0.016 | Rules+0x1528 | Threshold: 0.016 × 900 = 14.4 accumulator units per dump attempt, including the later final-empty attempt (corrected 2026-07-11 via `disassemble_function 0x0073D630` — `OPERATOR_OR_ORDER_DRIFT`) |
| `Harvester=` | [UnitType] | no | +0xE0E | Guards anim slot 7 call at deposit entry; gates harvester-path split |
| `Weeder=` | [UnitType] | no | +0xE0F | Weeder harvester path in case 0x17 and `Mission_Deploy_Building` |
| `DockRange=` | [UnitType] | — | +0x5E0 | Distance gate in `UnitClass::Receive_Radio` cases 0x0E, 0x0F, 0x15 |

---

## 9. Per-Case Verdicts (Active in YR)

| Code | Name | Refinery dock chain? | Notes |
|------|------|---------------------|-------|
| 0x02 | HELLO | YES | Every dock cycle; ally check + Contacts[] allocation |
| 0x03 | BREAK/OVER_AND_OUT | YES | Exit + abort paths; Contacts[] slot cleared |
| 0x07 | DOCKING_COMPLETE | NO (refinery) / YES (conditional carryall) | Standard DockUnload chain never sends 0x07; `BuildingClass::MissionRepairAndProduce` also does not send it. Verified direct sender: `AircraftClass::Mission_Move_Carryall @ 0x00416D50`. |
| 0x08 | REQUEST_DOCKING_CLEARANCE | NO (refinery) / YES (factory/repair) | `BuildingClass` case 0x08 is live but not in harvester→refinery sequence |
| 0x0B | DOCK_APPROACH | NO (harvester→refinery) | Building self-queues Mission_Unload(0x14); role in refinery unclear — needs caller trace |
| 0x0C | DOCK_ARRIVED | NO (stock inbound refinery) / YES-shared receiver | Stock HARV/CMIN inbound path does not send 0x0C; `Mission_Enter` sends 0x0E and pad arrival sends 0x15 |
| 0x0E | CAN_DOCK | YES | Primary dock admission; one send per scheduled Mission_Enter dispatch until accepted (corrected 2026-07-11 via `decompile_function 0x004D9290` — `OPERATOR_OR_ORDER_DRIFT`) |
| 0x0F | CAN_ENTER | NO (refinery) / YES (garrison/helipad/grinder) | Not in harvester→DockUnload path |
| 0x10 | RESERVE_DOCK | NO — zero senders found | Receiver is live (returns ROGER for Refinery=yes), but no sender exists in any of the 10 candidate functions exhaustively scanned (#12) |
| 0x11 | DRIVE_TO | YES-shared | FootClass case 0x11 gates on mission==7 or +0xB4==7 |
| 0x12 | MOVE_TO_CELL | YES | Refinery→harvester inside case 0x0E burst; sets harvester destination |
| 0x13 | NEED_TO_MOVE | YES | Probe before 0x12; chrono-gate; writes chrono dest into payload |
| 0x15 | TIMING_SYNC_BACK / DOCK_NOW | YES | Harvester→refinery from PerCellProcess pad/contact paths or the ready case-0x16 cascade; triggers deposit start (corrected 2026-07-11 via `disassemble_function 0x00739EC0` and `decompile_function 0x00737430` — `INFERENCE_HARDENED`) |
| 0x16 | TIMING_SYNC | YES | Refinery→harvester; sets facing; cascade sends 0x15 back |
| 0x17 | QUEUED | YES-shared (factory) / NO (refinery) | Sent by case 0x08 for factory/repair; refinery path returns ROGER not QUEUED |
| 0x18 | ENTER_DOCK | YES | Sets TechnoClass `+0x418` radio/contact byte; propagates |
| 0x19 | LEAVE_DOCK | YES (conditional) | No exit function directly sends it, but normal state-4 BREAK can trigger TechnoClass's indirect `0x19` cascade when receiver and sender `+0x418` are both set; `Set_Destination` also sends it on cancel-dock. Corrected 2026-07-11 via `disassemble_function 0x0073D630` and `decompile_function 0x006F4AB0` — `INFERENCE_HARDENED`. |
| 0x21 | — | NO (UnitClass) / YES (AircraftClass) | AircraftClass case 0x21 = ammo-full check for helipad reload (#11 §2.3) |
| 0x22 | IS_REPAIRING | YES (as transmit in 0x0E filter) | Building sends to query unit health; ObjectClass handles receive |
| 0x23 | IS_OCCUPIED | YES (as transmit in 0x0F) | Building sends to query occupancy; FootClass handles receive |
| 0x24 | DOCK_QUERY | YES | UnitClass case 0x24 — generic unit availability query |
| 0x2C, 0x32 | (TS stub IDs) | NO | Fall through entire chain; return 0 from ObjectClass default (#11 §3) |

---

## 10. Sender Inventory — Where Each Live Message Originates

| Message | Sender(s) | Trigger | Source |
|---------|-----------|---------|--------|
| 0x02 HELLO | `UnitClass::Mission_Harvest` state 2 (vtable+0x278 @ 0x0073EE55) | Harvester full, returning to refinery | #11 |
| 0x03 BREAK | `UnitClass::Mission_Deploy_Building` state-4 @ 0x0073E277; `UnitClass::Set_Destination` cancel-dock path; conditional `BuildingClass::ReleaseDockedHarvester` @ 0x0045982C | Stock zero-link exit; abort; reciprocal-link release | #8, #9, #7, 2026-05-21 re-swarm |
| 0x0E CAN_DOCK | `UnitClass::Set_Destination` @ 0x741DDA (vtable+0x278, initial); `FootClass::Mission_Enter` @ 0x004D92B9 (vtable+0x278, once per scheduled dispatch) | One-shot on destination set; later scheduled Mission_Enter dispatches each send once | Corrected 2026-07-11 via `disassemble_function 0x00741970` and `decompile_function 0x004D9290` — `OPERATOR_OR_ORDER_DRIFT` |
| 0x12 MOVE_TO_CELL | `BuildingClass::Receive_Radio` case 0x0E body (reply burst to harvester) | Inside case 0x0E, after 0x13 ROGER | #3 |
| 0x13 NEED_TO_MOVE | `BuildingClass::Receive_Radio` case 0x0E body (before 0x12) | Probe sent first inside case 0x0E | #3 |
| 0x15 TIMING_SYNC_BACK | `UnitClass::PerCellProcess` @ `0x0073A503` (vtable+0x274 ToFirst) and `0x0073A5C4` (vtable+0x278 targeted); `UnitClass::Receive_Radio` case 0x16 @ `0x00737776` (vtable+0x278 targeted) | Pad/contact processing, or a later/already-facing ready 0x16 handoff | Corrected 2026-07-11 via `disassemble_function 0x00739EC0` and `decompile_function 0x00737430` — `INFERENCE_HARDENED` |
| 0x16 TIMING_SYNC | `BuildingClass::Receive_Radio` case 0x0E body (after 0x18) | Inside case 0x0E reply burst | #3 |
| 0x18 ENTER_DOCK | `BuildingClass::Receive_Radio` case 0x0E body (after 0x12 ROGER) | Inside case 0x0E reply burst | #3 |
| 0x19 LEAVE_DOCK | `TechnoClass::Receive_Radio` case 0x03 (conditional: receiver +0x418 && sender +0x418); `UnitClass::Set_Destination` @ 0x741B97 (cancel-dock) | On BREAK if both radio/contact bytes are set; on abort if approaching | 2026-05-21 re-swarm |
| 0x22 IS_REPAIRING | `BuildingClass::Receive_Radio` case 0x0E (queries unit repair state) | As filter in CAN_DOCK for UnitRepair buildings | #3 |
| 0x23 IS_OCCUPIED | `BuildingClass::Receive_Radio` case 0x0F (queries occupancy) | As filter in CAN_ENTER | #3 |

---

## 11. Dock Teardown — Radio/Contact State Lifecycle

The radio protocol does not own a reciprocal `+0x2E4` teardown for stock refinery unload.

- **Stock radio/contact state:** `TechnoClass::Receive_Radio` case 0x18 writes byte `this+0x418 = 1`; case 0x19 clears byte `this+0x418 = 0`. These are radio/contact-state toggles, not reciprocal dock pointers.
- **Stock DockUnload exit:** `UnitClass::Mission_Deploy_Building` state 4 clears `unit+0x6D1`, may transmit BREAK(0x03) if a radio contact is still present, and returns/continues Harvest scheduling. It does not clear a reciprocal `unit/building +0x2E4` link because stock GAREFN/NAREFN DockUnload does not establish one.
- **Conditional reciprocal-link cleanup:** `BuildingClass::ReleaseDockedHarvester` and `BuildingClass::UndockUnit` still directly clear `unit/building +0x2E4` and `building+0x718` when reached from a nonzero-link context, but that is not the normal stock zero-link DockUnload completion.

**Key implication for the Rust port:** Model stock refinery unload as a zero-link unit-side FSM with `+0x418` radio/contact state and `+0x6D1` unload-active state. Do not synthesize a reciprocal `+0x2E4` link or a `ReleaseDockedHarvester`/`Force_Track(0x47)` exit for normal stock CMIN/HARV deliveries.

---

## 12. Anim Slot Lifecycle (Refinery Dock-Anim Choreography)

All slot calls via `BuildingClass::SetAnimSlotImage` @ 0x00451750 (art-name selector) → `BuildingClass::CreateAnimForSlot` @ 0x00451890 (allocator). 21 slots total (indices 0..20). Slot N lives at `building + 0x55C + N*4`.

| Stage | Slot # | INI Key | Operation | Caller | Condition |
|-------|--------|---------|-----------|--------|-----------|
| Dock arrival (one-time) | 7 | `PreProductionAnim` | `SetAnimSlotImage(7, low_health, 0)` | `Mission_Deploy_Building` state-3 init @ 0x0073E08E | `Harvester=yes` flag; fires only on first tick (`+0x6D1==0`); no-op if art name empty |
| Per-bale pulse | 10 | `SpecialAnim` | `SetAnimSlotImage(10, low_health, 0)` | `Mission_Deploy_Building` state-3 loop @ 0x0073E3BA | `building+0x584 == 0` (slot-10 not currently playing); fires AFTER particle emitter |
| Per-bale particle | n/a | — | `vtable+0x468` (particle emitter) | `Mission_Deploy_Building` state-3 loop @ 0x0073E37E | Fires unconditionally on every gate-crossing, before slot-10 call |
| Completion / cargo empty | 8 | `ProductionAnim` | `SetAnimSlotImage(8, low_health, 0)` | `Mission_Deploy_Building` @ 0x0073E517 | `FindFirstNonEmptySlot` returns -1 (all slots drained); gated on `Type[0x16BB]` (Refinery=yes) |
| Completion clear | 10 | — | `ClearAnimSlot(building, 0xA)` | `Mission_Deploy_Building` @ 0x0073E534 | Same tick as state→4 transition |
| Interrupt clear | 10 | — | `ClearAnimSlot(building, 0xA)` | `Mission_Deploy_Building` early-exit @ 0x0073E5AC | Mission override mid-unload |
| Reciprocal-link release clear | 0xA, 0xB | — | `ClearAnimSlot(A)`, `ClearAnimSlot(B)` | `ReleaseDockedHarvester` | Conditional non-stock-zero-link release |
| Reciprocal-link release create | 0xC, 0xD | `SpecialAnimThree/Four` | `CreateAnimForSlot(C)`, `CreateAnimForSlot(D)` | `ReleaseDockedHarvester` | Conditional non-stock-zero-link release animation |

**Notes:**
- `ClearAnimSlot` arg -2 clears all 21 slots. Slot 0xA == decimal 10. Slot arg is a decimal index.
- `CreateAnimForSlot` preamble: if `building.IsDamaged != low_health` at call time, ALL 21 occupied slots are mass-swapped before creating the requested slot. One slot call can trigger up to 20 additional recreations.
- `building+0x57C = Anims_0[8]` (ProductionAnim pointer) is also used as a state-4 depart guard in `Mission_Deploy_Building`: if `building+0x57C != 0`, function returns 1 (wait) before departing.

---

## 13. Tiny Details Worth Recording for Parity

1. **BREAK in Receive_Radio exits on first match; BREAK in Transmit_Radio_Impl scans all slots.** Two different behaviors for the same code at different call points. (#1 §8 item 1)

2. **Transmit_Radio_Impl null-target default uses Contacts[0] only, not first-non-null.** If Contacts[0] is null and Contacts[1] is occupied, returns 0 silently. (#1 §4)

3. **HELLO eviction path calls vtable+0x278 (Transmit_Radio), not vtable+0x27C (Transmit_Radio_Impl).** A subclass that overrides vtable+0x278 would be called during eviction. (#1 §4)

4. **g_RadioScratchBuffer at 0x00A8EC30** — shared static global written by any Receive_Radio callee via `*payload = x`. Safe in single-threaded gamemd; unsafe in any concurrent reimplementation. (#1 §5 item 4)

5. **Queue cell `(NW.X+3, NW.Y+1)` is hardcoded** inline in `BuildingClass::Receive_Radio` case 0x0E. `QueueingCell=` INI at +0x1618/+0x161C is stored but never read here. (#3 §6)

6. **Each successful dump attempt removes the entire amount in the first nonempty resource slot in one call** — `StorageClass::RemoveAmount(GetAmount(slot), slot)`. After the last such removal resets `+0xF8`, completion waits for the next threshold attempt to observe `FindFirstNonEmptySlot == -1`; that final empty attempt still runs the particle/slot-10 preamble. (Corrected 2026-07-11 via `decompile_function 0x006C9820` and `disassemble_function 0x0073D630` at `0x0073E355..0x0073E539` — `OPERATOR_OR_ORDER_DRIFT`.)

7. **`UnitClass::Unlimbo` can seed `+0xF8` with `Random(0,29)`, but stock accepted unload overwrites it with zero.** `decompile_function 0x00737BA0` verifies the conditional random write. In `Mission_Deploy_Building`, `0x0073DFD0` writes `unit+0xF8 = 0`, `0x0073DFDA` sets `+0x6D1`, and only then `0x0073E093` sets state 3. Therefore the Unlimbo seed does **not** desynchronize stock refinery unload cadence. (Corrected 2026-07-11 via `decompile_function 0x00737BA0` and `disassemble_function 0x0073D630` — `OPERATOR_OR_ORDER_DRIFT`.)

8. **ILocomotion vtable+0x4C dispatches differently per locomotor:** DriveLocomotionClass → `Do_Turn @ 0x004B0EF0`, whose decompile calls `RateTimer__Set(&param_2)`; WalkLocomotionClass → `Set_Facing @ 0x0075AE00`. The former's exact timer storage is still UNVERIFIED: this live decompile does not prove FootClass+0x388. (Corrected 2026-07-11 via `decompile_function 0x004B0EF0` — `INFERENCE_HARDENED`; deliberately remains YELLOW/non-load-bearing.)

9. **ClearAnimSlot arg for SpecialAnim is `0xA` (decimal 10)**, NOT 0xB. Confirmed `PUSH 0xA` at both call sites (0x0073E530 and 0x0073E5A8). The CALLs to `ClearAnimSlot` follow at 0x0073E534 and 0x0073E5AC respectively — verified via `read_memory 0x0073E5AC` = `e8 8f 38 d1` (CALL opcode, not PUSH; 2026-05-24 audit). (#9 §3)

10. **CreateAnimForSlot damage-state preamble** can replace ALL 21 slots when health crosses ConditionYellow. A single slot call may trigger 20 additional anim recreations. (#10 §3)

11. **FootClass case 0x12 writes 3-word timestamp** at `this+0xC8`, `+0xCC`, `+0xD0` = `[g_CurrentFrameCounter, iStack_10, 0]`. Verified via disassembly at 0x004D91fc–0x004D920d. (#5 §4)

12. **`PathType::Has_Valid_Steps @ 0x0065AE30` is misnamed** — the body actually walks `this->Contacts[]` (+0xE4, +0xE8) and returns 1 if any slot is non-null. Functionally = HasRadioContact, not HasPathSteps. (#6 §stage-4a note)

13. **`RadioHistory` is written but base RadioClass never reads it back** — the 3-slot push-down dedup log is write-only at the base class level. Subclasses may read it for dedup. (#1 §2)

14. **`Transmit_Radio` return type is `void` in Ghidra** but EAX passes through from Transmit_Radio_Impl. Callers via vtable+0x278 that examine EAX get the Impl's return code. (#1 §5)

15. **Alive guard on HELLO:** `*(int*)(this+0x6C) != 0` gates the entire HELLO branch in RadioClass::Receive_Radio. Dead/uninitialized objects reject HELLO. Field at +0x6C is in ObjectClass range; identity deferred. (#1 §3 step-C)

16. **Type[0x16BB] = Refinery=yes** — corrected 2026-05-20 from "unknown flag, likely TS-legacy" (the Phase 1 BUILDINGCLASS report). String "Refinery" at 0x0081AA5C; xref to `BuildingTypeClass_ReadINI_Water`; byte load at +0x16BB confirmed via `read_memory 0x00460A40`. (#11 §5.1)

17. **Case 0x10 RESERVE_DOCK is a dead-send:** 10 candidate functions scanned by disassembly; zero instances of `PUSH 0x10` + Transmit_Radio call found. The `PUSH 0x10` in `BuildingClass::ExitObject_Main` is `Queue_Mission(0x10)`, not a radio message. (#12)

18. **TechnoClass BREAK conditionally sends 0x19:** only fires when its contact-state condition and `sender+0x418 != 0` hold. `+0x418` is the radio/contact byte set by 0x18 and cleared by 0x19, not a generic "has destination" flag. (#5 §Q1; `UNITCLASS_0X418_DOCK_FLAG_LIFECYCLE_AND_CONSUMERS_GHIDRA_REPORT.md`)

19. **`DAT_0089F6A0`** is the hardcoded west-neighbor cell offset `(-1,0)` from the global 8-neighbor direction table initialized at `Foundation_direction_table_init @ 0x0049F2F0`. It is distinct from the hardcoded admission target `(NW+3, NW+1)`, `QueueingCell`, and `DockingOffset%d`; it is not read from artmd.ini. (`DAT_0089F6A0_RUNTIME_SOURCE_AND_VALUE_GHIDRA_REPORT.md`)

20. **Building+0x57C** (slot-8 ProductionAnim pointer) doubles as a depart-prep guard in `Mission_Deploy_Building` state 4 — if non-null, returns 1 (wait another tick). It is the live `Anims_0[8]` / `ProductionAnim` pointer, not a locomotor readiness field. (#9 §3 state-4; `BUILDINGCLASS_0X57C_DOCK_DEPART_GUARD_NAVCOM_GHIDRA_REPORT.md`)

21. **InfantryClass has NO Receive_Radio override** — inherits `FootClass::Receive_Radio @ 0x004D8FB0` directly. Verified by `read_memory 0x007EB1EC` → `B0 8F 4D 00`. (#2 §2.2)

22. **`FUN_0045AF20`** at 0x0045AF20 is a COM `QueryInterface` wrapper for `IID_IPiggyback`, called on each scheduled `FootClass::Mission_Enter` dispatch to check if the piggyback locomotor is ready to release. It is not a thunk to EnterBuildingOrDock. (Cadence wording corrected 2026-07-11 via `decompile_function 0x004D9290` — `OPERATOR_OR_ORDER_DRIFT`.)

23. **`SlaveManagerClass::AI_Update` @ 0x006AF6C0** (Ghidra-labeled `DOCKMANAGER_STATE_MACHINE_FUN_006AF6C0`) has zero connection to refinery docking — slave-miner only. (#11 §2.7)

24. **`Mission_Deploy_Building` credit write order:** base credits first, then purifier bonus. Two separate `HouseClass::Add_Tiberium_Credits` calls per drain event. Credits go to refinery owner (via `GetOwner()` on building), not to the current controller of the harvester — relevant for mind-control during unload. (#9 §3d, §8)

---

## 14. Corrections to Prior Docs

| Doc filename | Section | Stale claim | Corrected claim | Evidence |
|---|---|---|---|---|
| `HARVESTER_DOCK_UNLOAD_SEQUENCE.md` | §8.3 | "Radio 0x07 DOCKING_COMPLETE fires after the last ore bale" | 0x07 is never sent in the standard refinery chain; `Mission_Deploy_Building` has zero `PUSH 0x7` calls | #9 §Q2; #8 §5.4 |
| `HARVESTER_DOCK_UNLOAD.md` | §4a | "`BuildingClass::MissionRepairAndProduce` handles the refinery dump" | Refinery drain is entirely in `UnitClass::Mission_Deploy_Building` on the unit side. `MissionRepairAndProduce` is for UnitRepair/Bunker/Hospital only | #9 §Q2 |
| `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` | §case 0x10, §6 table | "Type[0x16BB] is unknown flag, not in stock YR rules (likely TS-legacy); case 0x10 returns NEGATORY for standard DockUnload refineries" | Type[0x16BB] = `Refinery=yes` INI key; case 0x10 returns ROGER for GAREFN/NAREFN | #11 §5.1 + §3 flag table; `read_memory 0x00460A40`; string "Refinery" at 0x0081AA5C |
| `UNITCLASS_ENTERBUILDINGORDOCK_GHIDRA_REPORT.md` (investigation plan label) | Title + scope | "This function is the harvester-side sender of 0x0E/0x16 traffic" | The function at 0x0041AA80 is `AircraftClass::Set_Destination` (vtable+0x480 of AircraftClass, not UnitClass). Ghidra label `UnitClass__EnterBuildingOrDock` is wrong | #6 §CRITICAL IDENTITY FINDING; `read_memory 0x007E2724` → `80 AA 41 00` |
| `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md` | PathType::Has_Valid_Steps description | "Checks if path has valid steps" | The body at 0x0065AE30 walks `this->Contacts[]` and returns 1 if any slot is non-null — functionally = HasRadioContact | #6 §stage-4a, #7 §tiny details |
| `DOCKMANAGER_STATE_MACHINE_FUN_006AF6C0_GHIDRA_REPORT.md` | Title | "DOCKMANAGER" in title | Content is correct: 0x006AF6C0 is `SlaveManagerClass::AI_Update`. Title is misleading. | #11 §2.7, §4 |

---

## 15. Deprecation Recommendations

1. **`DOCKMANAGER_STATE_MACHINE_FUN_006AF6C0_GHIDRA_REPORT.md`** — Rename to `SLAVEMANAGERCLASS_AI_UPDATE_0x6AF6C0_GHIDRA_REPORT.md`. Content is correct; only the title misleads searchers. No rewrite needed. (#11 §4)

2. **`HARVESTER_DOCK_UNLOAD.md` §4a** — Mark the building-side drain claim as CORRECTED with a reference to `MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md`. The section's core narrative is wrong for standard DockUnload buildings.

3. **`HARVESTER_DOCK_UNLOAD_SEQUENCE.md` §8.3** — Mark the 0x07 claim as WRONG with reference to #9. Remainder of doc is likely still valid.

---

## 16. Open Questions Carried Forward

> **2026-05-22 status note:** Several rows below are now resolved by focused
> follow-up reports but are left in-place for historical continuity. Resolved:
> OQ-5 (`+0xCC` is target coord Y local in `0x12`), OQ-6 (`+0xB4` is
> `MissionClass::QueuedMission`), OQ-7 for the stock path (`Force_Track(0x47)`
> is not normal stock zero-link refinery exit), OQ-8 (`PerCellProcess` sends
> radio `0x15`; `0x10` is the queued mission id), and OQ-10 (TechnoClass has
> no meaningful `0x10` handler for standard CMIN refinery docking).

| # | Question | Category | How to resolve |
|---|----------|----------|----------------|
| OQ-1 | `building+0x57C` exact semantic — "anim in progress" or "loco not ready"? Guards state-4 depart | Field offset | `decompile_function 0x0044B000` (UpdateAnimation) or trace all writes to +0x57C |
| OQ-2 | `TypeClass+0xDFC` in `UnitClass::Set_Destination` LEAVE_DOCK gate — which INI key? | INI | Search ReadINI for MOV/MOVZX at offset 0xDFC; ~10 min |
| OQ-3 | `+0x6C` alive-guard field on RadioClass HELLO — ObjectClass/MissionClass field; exact semantic | Struct | Trace ObjectClass layout; not critical for dock parity |
| OQ-4 | Sound cue branch in BuildingClass case 0x0E when 0x16 reply ≠ 1 — `DAT_0089c848` plays; specific cue identity | Audio | `read_memory 0x0089c848`; cross-reference audio table |
| OQ-5 | `iStack_10` in FootClass case 0x12 write to `this+0xCC` — origin (high-word of 64-bit counter?) | Field | Trace stack frame at call site |
| OQ-6 | `this+0xB4` role (value 7 vs -1 in FootClass cases 0x11/0x12) — TeamID, SubMission, other? | Field | Struct layout trace |
| OQ-7 | Force_Track 0x47 sub-cell bib step at refinery exit — track 15 in drive_track.rs exists but `head_offset` semantics need Ghidra verification before wiring (existing MEMORY item: `project_force_track_bib_step.md`) | Implementation | Verify track-15 head_offset via Ghidra; ~1 session |
| OQ-8 | **RESOLVED 2026-07-11:** `UnitClass::PerCellProcess @ 0x00739EC0` has two `0x15` sends (`0x0073A503` ToFirst and `0x0073A5C4` targeted); no radio-`0x10` send is part of those handoffs. Building case `0x15` instead queues mission `0x10` on the sender/harvester. | Protocol | `disassemble_function 0x00739EC0`; `decompile_function 0x0043C2D0` — `INFERENCE_HARDENED` |
| OQ-9 | `RulesClass+0x850` field identity (used in `UnitClass::Mission_Unload` as first transmit arg) | Struct | RulesClass struct layout doc |
| OQ-10 | Does `TechnoClass::Receive_Radio @ 0x006F4AB0` handle case 0x10? (not confirmed) | Protocol | `decompile_function 0x006F4AB0` |
| OQ-11 | Mission-ID label ambiguity: cases 0x13 (Mission_Unload_Refinery?) vs 0x14 (Mission_Unload?) vs 0x10 (Mission_Enter/Unload?) — canonical gamemd Mission enum was never read directly | Correctness | Read Mission name-table at binary mission-name array |

---

## 17. Implications for the Rust Port

**2026-05-22 status:** The older implementation verdict for the direct FSM is
superseded. A direct FSM can still be the right Rust architecture, but it must
match the verified radio/NavCom outputs and timing boundaries from the follow-up
reports instead of assuming the current behavior already matches stock.

**2026-05-22 implementation verdict update:** The close-return timing trace
found current observable mismatches:
threshold math, missing early `HELLO(0x02)`, and collapsed radio/mission timing.
Future Rust work should reproduce the verified radio/NavCom outputs and timing
boundaries from the 2026-05-21/22 follow-up reports, not rely on the older
"current design is valid" conclusion.

**If the project ever adds a RadioLink primitive,** the four seams to model are:

1. **Contacts[] allocator** — BuildingClass constructor calls `Set_Contact_Count(NumberOfDocks)` to size the array; all others default to 1 slot.
2. **Eviction protocol** — in sender-side outgoing `Transmit_Radio_Impl` HELLO
   path, when the sender's own slots are full, slot 0 is evicted by calling
   `Transmit_Radio(BREAK=3, Contacts[0])` (vtable+0x278, not +0x27C). Incoming
   full receiver-side refinery HELLO returns `NEGATORY(10)` and does not evict
   the refinery's current contact.
3. **Shared g_RadioScratchBuffer @ 0x00A8EC30** — both `Transmit_Radio` and `Transmit_Radio_ToFirst` use the same global as payload pointer. Must be per-call or single-threaded.
4. **vtable+0x194 dispatch** — each concrete class has its own receiver; InfantryClass inherits FootClass directly.

**Things the Rust port must get right for parity (not currently obvious from code alone):**

- Treat `Mission_Enter` as a scheduled dispatch that sends one `0x0E` and returns
  the next delay, not as an every-tick resend loop. Model both live `0x15`
  sources: PerCellProcess and the later/already-facing case-`0x16` cascade.
  (`decompile_function 0x004D9290`, `disassemble_function 0x00739EC0`, and
  `decompile_function 0x00737430`; corrected 2026-07-11 -
  `OPERATOR_OR_ORDER_DRIFT` / `INFERENCE_HARDENED`.)
- The stock dock-state byte is `+0x418`, set by `0x18` and cleared by `0x19`.
  Reciprocal `unit/building +0x2E4` is conditional cleanup/link state, not normal
  stock refinery DockUnload. `ReleaseDockedHarvester`/`UndockUnit` directly clear
  `+0x2E4` only when reached from a nonzero-link context. `ReleaseDockedHarvester`
  directly sends `BREAK(0x03)` only; any `0x19` is an indirect conditional
  TechnoClass BREAK cascade.
- Anim slot 7 (PreProductionAnim) fires **once on arrival** (when `+0x6D1==0`), not per bale.
- The **particle emitter fires BEFORE** `SetAnimSlotImage(10)` on every threshold crossing, including the final empty attempt.
- A successful dump attempt removes the entire first nonempty resource slot. After the last removal resets `+0xF8`, do not complete immediately: wait for the next threshold crossing, whose empty-slot result drives slot 8, slot-10 clear, and state 4. (`decompile_function 0x006C9820`; `disassemble_function 0x0073D630` at `0x0073E355..0x0073E539`; corrected 2026-07-11 — `OPERATOR_OR_ORDER_DRIFT`.)
- The gate threshold is **HarvesterDumpRate × 900** (default ≈ 14.4 accumulator units). Accepted stock unload initialization resets `unit+0xF8` to zero before state 3, so Unlimbo's conditional `Random(0,29)` seed does not jitter refinery unload cadence. (Corrected 2026-07-11 via `decompile_function 0x00737BA0` and `disassemble_function 0x0073D630` at `0x0073DFD0..0x0073E093` — `OPERATOR_OR_ORDER_DRIFT`.)
- Queue cell `(NW+3, NW+1)` is **hardcoded** in the binary — the INI key `QueueingCell=` is stored but never read in case 0x0E.
- Credits go to the **refinery owner** (via `GetOwner()` on the building), not to the harvester's controller. Mind-controlled harvesters pay the refinery owner.

---

## 18. Sources

### 12 new reports (2026-05-20)

| # | Filename | Summary |
|---|----------|---------|
| 1 | `RADIOCLASS_CORE_PRIMITIVES_VERIFIED_GHIDRA_REPORT.md` | Full decompile of Receive_Radio, Transmit_Radio_Impl, Transmit_Radio, Transmit_Radio_ToFirst; RadioHistory layout; vtable+0x194 binding for Building+Unit |
| 2 | `RADIO_VTABLE_BINDING_AND_SLOT_HELPERS_GHIDRA_REPORT.md` | Live read_memory vtable+0x194 for 7 classes; FindDockSlot; FindFreeContactSlot; ObjectClass::Receive_Radio decode |
| 3 | `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` | Full 9-case decode of BuildingClass::Receive_Radio (0x0043C2D0); PATCHED 2026-05-20 for Type[0x16BB]=Refinery= and case 0x10 verdict |
| 4 | `UNITCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` | Full 8-case decode of UnitClass::Receive_Radio (0x00737430); jump table verified; ILocomotion vtable+0x4C locomotor resolution |
| 5 | `FOOTCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` | Full 6-case decode of FootClass::Receive_Radio (0x004D8FB0); 0x19 sender trace; 0x07 sender trace |
| 6 | `UNITCLASS_ENTERBUILDINGORDOCK_GHIDRA_REPORT.md` | Identity correction: 0x0041AA80 is AircraftClass::Set_Destination, not UnitClass harvester helper |
| 7 | `TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md` | UnitClass vtable+0x480 @ 0x00741970 — the real harvester CAN_DOCK sender; radio traffic inventory |
| 8 | `REFINERY_DOCK_EXIT_CHAIN_VERIFIED_GHIDRA_REPORT.md` | FootClass::Mission_Enter + ReleaseDockedHarvester + UndockUnit re-decompile; definitive 0x19/0x07 absence proofs |
| 9 | `MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md` | UnitClass::Mission_Deploy_Building full FSM; per-bale drain; anim slot ordering; timing constants |
| 10 | `REFINERY_DOCK_CELL_AND_ANIM_HELPERS_GHIDRA_REPORT.md` | GetDockCellForObject; CreateAnimForSlot; SetAnimSlotImage; ClearAnimSlot; 21-slot table |
| 11 | `RADIO_REFINERY_DOCK_TS_LEGACY_AND_CONTEXT_GHIDRA_REPORT.md` | TS-legacy sweep of 7 functions; Type[0x16BB]=Refinery= correction; SlaveManager negative finding; AircraftClass::Receive_Radio |
| 12 | `RADIO_0x10_RESERVE_DOCK_SENDER_TRACE_GHIDRA_REPORT.md` | Exhaustive scan of 10 candidate functions; zero senders for 0x10 found |

### Prior-art docs corroborated by this swarm

| Filename | Role |
|----------|------|
| `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md` | Vtable slot table; RadioHistory layout (corroborated) |
| `RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md` | Full filter chain for case 0x0E (corroborated) |
| `RADIO_0x16_SENDER_BUILDINGCLASS_CASE_0x0E_GHIDRA_REPORT.md` | 0x16 emission; case 0x15 sequence (corroborated) |
| `RADIO_0x16_RECEIVER_UNITCLASS_CASE_16_GHIDRA_REPORT.md` | UnitClass case 0x16 (corroborated with locomotor caveat) |
| `RELEASEDOCKEDHARVESTER_0x4595C0_GHIDRA_REPORT.md` | ReleaseDockedHarvester body (fully verified #8) |
| `BUILDING_UNDOCKUNIT_0x4593A0_CHRONO_MINER_GHIDRA_REPORT.md` | UndockUnit body (fully verified #8) |
| `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md` + `_VERIFICATION_NOTES.md` | Approach choreography (corroborated) |
| `REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md` | Anim slot semantics (corroborated with slot-10 gate extension) |
| `BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md` | Building docking overview (corroborated) |
| `NUMBEROFDOCKS_VS_DOCKOFFSET_RECONCILE_GHIDRA_REPORT.md` | NumberOfDocks vs DockingOffset distinction (corroborated; GetDockCellForObject confirmed to not read +0x1788) |
