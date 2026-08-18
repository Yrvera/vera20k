# Building Receive Radio Dock Clearance / Handoff / Exit - Ghidra Research Report

**Address(es):** `0x0043C2D0`, `0x00739EC0`, `0x0073D630`, `0x004595C0`, `0x006F4AB0`, `0x0065A970`, `0x0065ACB0`, `0x0065A820`, `0x005B35E0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** `BuildingClass::Receive_Radio` dock/refinery messages `0x08` and `0x15`, plus stock refinery exit/clearance/handoff paths involving `0x19` or `BREAK(0x03)` for stock `GAREFN`/`NAREFN` and `HARV`/`CMIN`.
**Non-Scope:** full `BuildingClass::Receive_Radio`, full refinery admission `0x0E`, full ore credit math, full `Mission_Harvest`, full repair/bunker/factory radio FSMs, and non-stock refinery mods.
**Confidence:** High for static branch order, message effects, and stock YR reachability. Medium for exact post-state-4 live-frame timing of the later cleanup cascade because that needs a runtime trace.
**Active in YR:** Yes for stock `0x15` handoff and zero-link state-4 unload exit; No for stock refinery `0x08 -> 0x17 QUEUED`; Conditional for `0x19` because it requires live `+0x418` radio/contact flags or a cancel path.

## 0. Investigation Frame

**Target question:** How do `BuildingClass::Receive_Radio` messages `0x08` and `0x15`, plus `0x19`/`BREAK`, coordinate dock queue clearance, accepted-dock handoff, and release timing for stock `GAREFN`/`NAREFN` harvesters?

**Non-goals:** Do not re-decode the whole building switch, `0x0E` admission, ore drain math, or all refinery FSM states. Do not implement Rust.

**Evidence needed to mark COMPLETE:** fresh decompile of the receiver and exit functions; assembly context for the hot branches; caller evidence showing how `0x15` and exit are reached; INI proof that the stock objects take those branches; Rust surface scan sufficient for implementation handoff.

**Stop conditions:** stop once `0x08`, `0x15`, and `0x19`/`BREAK` stock paths are either resolved or explicitly deferred; downgrade to PARTIAL if any scoped branch remains unverified.

## 1. Overview

For stock refineries, `0x08` is not the dock queue-clearance handoff. `BuildingClass::Receive_Radio(0x08)` calls the TechnoClass cleanup path and returns `ROGER(1)` for `GAREFN`/`NAREFN`, because those buildings have `DockUnload=yes` and `Refinery=yes`, not `WeaponsFactory=yes`, `UnitRepair=yes`, or `Bunker=yes`.

The accepted dock handoff is `0x15`: the building receives the harvester's pad-arrival message and queues sender mission `0x10` with argument `0`. It does not create a reciprocal `unit/building +0x2E4` link. Ordinary stock unload completion exits through the zero-link `Mission_Deploy_Building` state-4 branch, clears `unit+0x6D1`, may send direct `BREAK(0x03)`, and releases radio/contact state separately from any `ReleaseDockedHarvester` forced-track path.

## 2. Key Offsets / Fields

| Owner | Offset / slot | Meaning in this slice | Active in YR |
|---|---:|---|---|
| BuildingType | `+0x16B3` | `DockUnload=yes`; selects stock refinery `0x15` handoff | Yes |
| BuildingType | `+0x16BB` | `Refinery=yes`; zero-link unload FSM refinery guard | Yes |
| BuildingType | `+0x16BD` | `WeaponsFactory=yes`; enables `0x08 -> 0x17` reply | Conditional; not GAREFN/NAREFN |
| BuildingType | `+0x16A9` | `UnitRepair=yes`; `0x08` near-distance and queue reply gate | Conditional; not GAREFN/NAREFN |
| BuildingType | `+0x16AB` | `Bunker=yes`; `0x08` queue reply and reciprocal-link writer gate | Conditional; not GAREFN/NAREFN |
| RadioClass | `+0xE4/+0xE8` | Contacts array pointer/capacity | Yes |
| Techno/Unit | `+0x418` | byte radio/contact-entered flag set by `0x18`, cleared by `0x19` | Yes / Conditional |
| Unit/Building | `+0x2E4` | reciprocal dock/bunker link; not stock refinery DockUnload state | Conditional |
| Unit | `+0x6D1` | stock unload-active latch cleared on zero-link state-4 exit | Yes |
| Unit | `+0xB4` | queued mission written by `Queue_Mission(0x10,0)` | Yes |
| Building | `+0x57C` | `Anims_0[8]` state-4 depart guard; usually empty for stock refineries | Yes |
| vtable | `+0x194` | concrete `Receive_Radio` dispatch | Yes |
| vtable | `+0x274` | `Transmit_Radio_ToFirst` | Yes |
| vtable | `+0x278` | directed radio transmit | Yes |
| vtable | `+0x1E8` | `MissionClass::Queue_Mission` | Yes |

## 3. Core Logic

### 3.1 `BuildingClass::Receive_Radio(0x08)`

Decompile evidence: `BuildingClass__Receive_Radio @ 0x0043C2D0`.

Behavior:

```text
if Type.UnitRepair or Type.Bunker:
    distance(sender.coord, building.coord)
    if distance < 0x180 leptons:
        return 1

TechnoClass::Receive_Radio(0x08)

if not Type.WeaponsFactory and not Type.UnitRepair and not Type.Bunker:
    return 1

return 0x17
```

Stock `GAREFN`/`NAREFN` do not set any of the three final `0x17` flags. They therefore do not receive a stock refinery queued reply from this path.

**Evidence:** decompile `0x0043C2D0`; prior focused report `miner/BUILDING_RECEIVE_RADIO_0X08_CLEARANCE_QUEUE_GHIDRA_REPORT.md`; INI lines `rulesmd.ini:[GAREFN] DockUnload=yes/Refinery=yes` and `[NAREFN] DockUnload=yes/Refinery=yes`.

**Active in YR:** Conditional. The case is live, but its `0x17` queue reply is active for factory/repair/bunker-style buildings, not stock refineries.

### 3.2 TechnoClass cleanup under `0x08`

`TechnoClass::Receive_Radio(0x08) @ 0x006F4AB0` sends directed `0x19` to the sender and then directed `BREAK(0x03)` to the sender. For a stock refinery receiver, `BuildingClass` then returns `1`.

This means `0x08` can be a cleanup trigger for an existing radio/contact relationship, but it is not the stock busy-refinery queue admission result.

**Evidence:** decompile `0x006F4AB0`; `UNITCLASS_0X418_DOCK_FLAG_LIFECYCLE_AND_CONSUMERS_GHIDRA_REPORT.md` assembly context for `0x006F4C34..0x006F4C41`; fresh assembly context for `0x006F4C50..0x006F4C80` confirms `+0x418` gates and fallthrough to `RadioClass__Receive_Radio`.

**Active in YR:** Conditional. Live code; exact use after stock unload depends on remaining contact state and the UnitClass per-cell cleanup branch.

### 3.3 `BuildingClass::Receive_Radio(0x15)`

For stock `DockUnload=yes` refineries, case `0x15` only queues mission `0x10` on the sender and returns `1`.

Assembly context:

```asm
0043c788: MOV  CL, byte ptr [EAX + 0x16b3]
0043c78e: TEST CL,CL
0043c790: JZ   0x0043ce43
0043c796: MOV  ECX,dword ptr [ESP + 0x54]
0043c79a: PUSH 0x0
0043c79c: PUSH 0x10
0043c79e: MOV  EDX,dword ptr [ECX]
0043c7a0: CALL dword ptr [EDX + 0x1e8]
0043c7a9: MOV  EAX,0x1
```

`MissionClass__Queue_Mission @ 0x005B35E0` for `(0x10, 0)` writes queued mission `+0xB4` and byte `+0xB8 = 0` if the queued/current mission differs. Because the second argument is zero, it does not call the commence-now mission transition.

**Evidence:** decompile `0x0043C2D0`, assembly `0x0043C788..0x0043C7A0`, decompile `0x005B35E0`.

**Active in YR:** Yes. `[CMIN]` and `[HARV]` use `Dock=NAREFN,GAREFN` and `Harvester=yes`; `[GAREFN]` and `[NAREFN]` use `DockUnload=yes`.

### 3.4 Pad-arrival caller for `0x15`

`UnitClass::PerCellProcess @ 0x00739EC0` is the stock pad-arrival caller. The relevant branch calls:

```text
FootClass::PerCellProcess(2)
this.vtable+0x274(0x15)
locomotor.vtable+0x5C()
```

That transmit goes through `RadioClass__Transmit_Radio_ToFirst @ 0x0065ACB0`, then `RadioClass__Transmit_Radio_Impl @ 0x0065A970`, and dispatches the refinery's concrete receiver through target vtable `+0x194`.

**Evidence:** decompile `0x00739EC0`; decompile `0x0065ACB0` and `0x0065A970`; prior focused report `miner/BUILDING_RECEIVE_RADIO_0X15_DOCKUNLOAD_HANDOFF_GHIDRA_REPORT.md`.

**Active in YR:** Yes for stock HARV/CMIN refinery arrival.

### 3.5 Normal stock unload exit is zero-link state 4

`UnitClass::Mission_Deploy_Building @ 0x0073D630` splits at entry on `unit+0x2E4`. If the field is zero, it enters the ordinary unload FSM. Stock `0x15` handoff does not write `+0x2E4`, so stock refinery unload remains on this zero-link side.

State 4 checks the adjacent refinery and slot-8 guard. If `building+0x57C != 0`, it returns `1` and waits. If clear, it writes `unit+0x6D1 = 0`, queues/continues Harvest (`mission 10 / 0x0A`), and may send direct `BREAK(0x03)` through the unit's `vtable+0x274` only if `PathType__Has_Valid_Steps` succeeds.

Assembly context:

```asm
0073e1f0: MOV  EAX,dword ptr [ESI + 0x5a4]
0073e1f6: MOV  byte ptr [ESI + 0x6d1],0x0
0073e1fd: CMP  EAX,EBX
0073e207: CMP  EAX,-0x1
...
0073e26f: TEST AL,AL
0073e271: JZ   0x0073e27f
0073e275: PUSH 0x3
0073e279: CALL dword ptr [EDX + 0x274]
0073e283: CALL dword ptr [EAX + 0x1ec]
```

No `ReleaseDockedHarvester`, no `UndockUnit`, no `Force_Track(0x47)`, and no new `+0x480` exit destination appear in this stock state-4 branch.

**Evidence:** decompile `0x0073D630`; assembly `0x0073E1F0..0x0073E289`; reports `MISSION_DEPLOY_BUILDING_DOCKED_VS_UNDOCKED_BRANCH_GHIDRA_REPORT.md`, `CHRONO_MINER_FORCE_TRACK_0X47_EXIT_NAVCOM_STEP_GHIDRA_REPORT.md`, and `miner/STANDARD_REFINERY_0X2E4_WRITER_INVENTORY_GHIDRA_REPORT.md`.

**Active in YR:** Yes for stock `CMIN/HARV -> GAREFN/NAREFN`.

### 3.6 Conditional `ReleaseDockedHarvester` exit and `BREAK`

`BuildingClass::ReleaseDockedHarvester @ 0x004595C0` is a conditional reciprocal-link helper reached from `Mission_Deploy_Building` only when `unit+0x2E4 != 0` and a building is found. It is not the ordinary stock zero-link unload completion.

When the helper succeeds, it clears `unit+0x2E4`, commands locomotion, sets a passable destination, sets unit mission `MOVE=2`, clears `building+0x2E4` and `building+0x718`, queues building mission `5`, then sends direct `BREAK(0x03)` through `vtable+0x274`.

Assembly context:

```asm
004596e6: MOV dword ptr [ESI + 0x2e4],EBX
...
00459814: MOV dword ptr [EDI + 0x2e4],EBX
0045981a: MOV dword ptr [EDI + 0x718],EBX
00459820: CALL dword ptr [EDX + 0x1e8]
00459828: PUSH 0x3
0045982c: CALL dword ptr [EAX + 0x274]
00459839: RET
```

The helper does not directly send `0x19`.

**Evidence:** decompile `0x004595C0`; assembly `0x004596E6`, `0x00459814..0x0045982C`; report `miner/RELEASEDOCKEDHARVESTER_EXIT_RADIO_0X19_GHIDRA_REPORT.md`.

**Active in YR:** Conditional. Live helper, but standard stock DockUnload does not create the required reciprocal `+0x2E4` link.

### 3.7 `BREAK(0x03)` can indirectly trigger `0x19`

`TechnoClass::Receive_Radio(0x03) @ 0x006F4AB0` checks receiver `+0x418` and sender `+0x418`. If both are nonzero, it sends directed `0x19` before falling through to `RadioClass::Receive_Radio(0x03)`.

Assembly context:

```asm
006f4c50: MOV  AL,byte ptr [ESI + 0x418]
006f4c5a: TEST AL,AL
006f4c5c: JZ   0x006f4c80
006f4c5e: MOV  AL,byte ptr [EDI + 0x418]
006f4c64: TEST AL,AL
006f4c66: JZ   0x006f4c80
006f4c6b: PUSH 0x19
006f4c7a: CALL dword ptr [EDX + 0x278]
006f4c89: CALL 0x0065a820
```

`TechnoClass::Receive_Radio(0x19)` clears receiver byte `+0x418` before propagating:

```asm
006f4ba6: MOV  byte ptr [ESI + 0x418],0x0
006f4bad: CALL dword ptr [EDX + 0x278]
```

`RadioClass::Transmit_Radio_Impl(0x03)` first clears matching target slots in the sender's contact vector, then dispatches target `Receive_Radio(0x03)`.

**Evidence:** decompile `0x006F4AB0`, `0x0065A970`, `0x0065A820`; assembly `0x006F4BA6..0x006F4BAD`, `0x006F4C50..0x006F4C89`.

**Active in YR:** Conditional. Live for Techno-derived objects; requires both radio/contact flags to still be set.

## 4. INI Keys

| INI key | Stock value | Evidence | Effect | Active in YR |
|---|---|---|---|---|
| `[CMIN] Dock` | `NAREFN,GAREFN` | `ini/rulesmd.ini:7361` | Allows chrono miner to target stock refineries | Yes |
| `[CMIN] Harvester` | `yes` | `ini/rulesmd.ini:7364` | Enables stock harvester unload family | Yes |
| `[HARV] Dock` | `NAREFN,GAREFN` | `ini/rulesmd.ini:8225` | Allows war miner to target stock refineries | Yes |
| `[HARV] Harvester` | `yes` | `ini/rulesmd.ini:8228` | Enables stock harvester unload family | Yes |
| `[GAREFN] DockUnload` | `yes` | `ini/rulesmd.ini:11726` | Selects `0x15` sender mission handoff | Yes |
| `[GAREFN] Refinery` | `yes` | `ini/rulesmd.ini:11727` | Stock unload FSM refinery guard | Yes |
| `[NAREFN] DockUnload` | `yes` | `ini/rulesmd.ini:12519` | Selects `0x15` sender mission handoff | Yes |
| `[NAREFN] Refinery` | `yes` | `ini/rulesmd.ini:12520` | Stock unload FSM refinery guard | Yes |
| `QueueingCell=4,1` | art entry | `ini/artmd.ini:1716`, `1773` | Not read by `0x08` or `0x15`; not the accepted cell handoff | No for this slice |
| `DockingOffset0` | commented/asset dock data | `ini/artmd.ini:1725`, `1773` context | Not read by `0x08` or `0x15` stock handoff | No for this slice |

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `BuildingClass::Receive_Radio @ 0x0043C2D0` | Receiver for `0x08` and `0x15` | fresh decompile; assembly `0x0043C788..0x0043C7A0` | Yes / Conditional |
| `UnitClass::PerCellProcess @ 0x00739EC0` | pad arrival sender of `0x15` | fresh decompile | Yes |
| `RadioClass::Transmit_Radio_ToFirst @ 0x0065ACB0` | sends to `Contacts[0]` | fresh decompile; assembly `0x0065ACB0..` | Yes |
| `RadioClass::Transmit_Radio_Impl @ 0x0065A970` | generic dispatch and `BREAK` contact clear | fresh decompile; assembly `0x0065A970..0x0065AA54` | Yes |
| `MissionClass::Queue_Mission @ 0x005B35E0` | sender mission `0x10` handoff callee | fresh decompile | Yes |
| `UnitClass::Mission_Deploy_Building @ 0x0073D630` | stock unload FSM and state-4 exit | fresh decompile; assembly `0x0073E1F0..0x0073E289` | Yes |
| `BuildingClass::ReleaseDockedHarvester @ 0x004595C0` | conditional nonzero-link exit helper | fresh decompile; assembly `0x004596E6..0x0045982C` | Conditional |
| `TechnoClass::Receive_Radio @ 0x006F4AB0` | `+0x418`, `0x08`, `0x19`, and `BREAK` cascade | fresh decompile; assembly `0x006F4BA6..0x006F4C89` | Yes / Conditional |
| `RadioClass::Receive_Radio @ 0x0065A820` | incoming HELLO/BREAK receiver cleanup | fresh decompile | Yes |

## 6. Current Rust Implementation Status

Rust has a clean sim-side model in `src/sim/miner/miner_dock.rs` and `src/sim/miner/miner_dock_sequence.rs`:

- `RefineryDockContacts` tracks `contacts`, `waiting_retry_queue`, `contact_entered`, and `on_pad`.
- `phase_approach` and `phase_mission_enter` use `hello_or_wait`, not a `0x08 -> 0x17` refinery queue result.
- `phase_mission_enter` marks contact-entered before `Linked`, matching the `0x18`/`0x16` ordering requirement at a clean abstraction level.
- `phase_departing` releases on-pad/contact state and does not seed `ReleaseDockedHarvester`, `Force_Track(0x47)`, or a new exit destination.

Current tests already include `empty_unload_gate_releases_dock_on_next_stock_state4_handoff` and `queued_miner_takes_over_immediately_after_empty_gate_handoff`. The remaining risk is not the broad shape; it is the precise contact cleanup order when `BREAK` indirectly emits `0x19`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Target definitions and stop conditions | verified | section 0 | none |
| Building `0x08` stock refinery return | verified | `0x0043C2D0`; `rulesmd.ini` | none |
| Building `0x08` Techno cleanup side effect | verified | `0x006F4AB0`; `UNITCLASS_0X418...` | exact live post-exit frame deferred |
| Building `0x15` DockUnload handoff | verified | `0x0043C788..0x0043C7A0`; `0x005B35E0` | none |
| Pad-arrival `0x15` caller | verified | `0x00739EC0`; `0x0065ACB0`; `0x0065A970` | none |
| Stock zero-link state-4 exit | verified | `0x0073D630`; `0x0073E1F0..0x0073E289` | none for static ordering |
| `ReleaseDockedHarvester` direct radio behavior | verified | `0x004595C0`; `0x00459828..0x0045982C` | non-stock producers outside scope |
| `BREAK -> 0x19` cascade | verified | `0x006F4C50..0x006F4C89` | live frame timing deferred |
| `0x19` clear ordering | verified | `0x006F4BA6..0x006F4BAD` | none |
| Rust surface scan | verified | `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs`, tests | no code changes made |
| Full `0x0E` admission | deferred | prior docs only | out-of-scope |
| Full factory/repair/bunker `0x08` semantics | deferred | `0x0043C2D0` touched | out-of-scope |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-001 - Does stock `GAREFN`/`NAREFN` use `BuildingClass::Receive_Radio(0x08)` to return `0x17 QUEUED`? -> No. Stock refineries lack `WeaponsFactory`, `UnitRepair`, and `Bunker`, so case `0x08` returns `1` after Techno cleanup.` (evidence: `0x0043C2D0`; `ini/rulesmd.ini:11726-11727`, `12519-12520`)
- `[RESOLVED] OQ-002 - Does case `0x08` send `0x0B` or directly hand off the dock? -> No. It returns a code only, after possible Techno cleanup.` (evidence: `0x0043C2D0`)
- `[RESOLVED] OQ-003 - What does case `0x15` do for stock DockUnload? -> It calls sender `vtable+0x1E8(0x10,0)` and returns `1`.` (evidence: `0x0043C788..0x0043C7A0`)
- `[RESOLVED] OQ-004 - Does `0x15` write reciprocal `unit/building +0x2E4`? -> No; neither the branch nor immediate radio/mission callees write `+0x2E4`.` (evidence: `0x0043C788..0x0043C7A0`, `0x005B35E0`, `0x0065A970`, `0x0065ACB0`)
- `[RESOLVED] OQ-005 - Who sends stock pad-arrival `0x15`? -> `UnitClass::PerCellProcess` sends it through `vtable+0x274` after `FootClass::PerCellProcess(2)`.` (evidence: `0x00739EC0`)
- `[RESOLVED] OQ-006 - Is normal stock unload exit `ReleaseDockedHarvester`? -> No. Stock unload remains zero-link and exits through `Mission_Deploy_Building` state 4.` (evidence: `0x0073D630`, `0x0073E1F0..0x0073E289`)
- `[RESOLVED] OQ-007 - Does stock state-4 release immediately clear the dock contact? -> It clears `+0x6D1`, queues/continues Harvest, and may send direct `BREAK(0x03)` if `PathType__Has_Valid_Steps` succeeds; contact clearing is via radio protocol, not reciprocal `+0x2E4` teardown.` (evidence: `0x0073E1F6`, `0x0073E275..0x0073E279`, `0x0065A970`)
- `[RESOLVED] OQ-008 - Does `ReleaseDockedHarvester` directly send `0x19`? -> No. It directly sends only `BREAK(0x03)` after linked-unit teardown.` (evidence: `0x00459828..0x0045982C`)
- `[RESOLVED] OQ-009 - Can `BREAK(0x03)` produce `0x19`? -> Yes, conditionally: TechnoClass case `0x03` sends `0x19` if both receiver and sender `+0x418` are set.` (evidence: `0x006F4C50..0x006F4C7A`)
- `[RESOLVED] OQ-010 - What clears `+0x418`? -> TechnoClass case `0x19` writes `0` before propagating `0x19`.` (evidence: `0x006F4BA6..0x006F4BAD`)
- `[RESOLVED] OQ-011 - Are stock `QueueingCell` or `DockingOffset` needed for `0x08`/`0x15` handoff? -> No for this slice; `0x08`/`0x15` do not read those art fields.` (evidence: `0x0043C2D0`; `ini/artmd.ini:1716`, `1773`)
- `[DEFERRED] OQ-012 - Exact rendered/live frame when the post-unload `+0x418` cleanup cascade fires after stock state-4 exit.` (category: `needs-runtime-debugger`; reason: static code proves the branch and gates but not a live replay frame; next-step-if-pursued: trace CMIN unloading through first post-state-4 per-cell tick)
- `[DEFERRED] OQ-013 - Complete factory/repair/bunker use of `0x08 -> 0x17`.` (category: `out-of-scope`; reason: this slot is stock refinery handoff/exit only; next-step-if-pursued: run a separate factory/repair docking investigation)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Stock refineries do not use `0x08 -> 0x17` for queue admission; busy contacts reject/wait through HELLO/contact capacity and repeated admission | `0x0043C2D0`; `0x0065A820`; INI GAREFN/NAREFN | none observed for broad shape | `src/sim/miner/miner_dock.rs`; `phase_approach`, `phase_mission_enter` | Keep queueing as Rust retry order, not a stock `0x08` refinery reply | existing `occupied_can_dock_defers_without_clearing_waiting_miner_target`-style tests | Do not model stock refinery busy response as case `0x08` returning `0x17` |
| `0x15` handoff queues sender mission `0x10` only and does not establish `+0x2E4` | `0x0043C788..0x0043C7A0`; `0x005B35E0` | none observed for zero-link stock path | `src/sim/miner/miner_dock_sequence.rs` `phase_linked`/`phase_pivoting`/`phase_unloading` | Deposit should start from unit-side mission/dock phase, not reciprocal building link | proposed test: `stock_refinery_0x15_handoff_does_not_create_reciprocal_release_link` | Do not synthesize `ReleaseDockedHarvester` state at `0x15` |
| Stock empty cargo exits through state-4 next handoff tick, clears unload-active state, releases contact/pad, and returns to search/harvest scheduling | `0x0073E1F0..0x0073E289` | mostly covered by tests | `phase_unloading`, `phase_departing`, `RefineryDockContacts` | Keep release after empty-slot gate/state-4 handoff, not on last bale drain | existing `empty_unload_gate_releases_dock_on_next_stock_state4_handoff`; proposed stronger queue test above | Do not release dock on the last non-empty slot drain |
| `ReleaseDockedHarvester` is conditional nonzero-link cleanup; direct radio is only `BREAK(0x03)` | `0x004595C0`; `0x00459828..0x0045982C` | interrupt path exists; stock path correctly avoids it | `interrupt_refinery_docked_miners`, any future bunker/linked-dock model | Use force-track/linked cleanup only for verified nonzero-link/interrupt contexts | sell/destroy interrupt test, e.g. existing `sell_refinery_interrupts_docked_miner_with_force_track_0x47` | Do not use Force_Track(0x47) for normal stock refinery completion |
| `BREAK` can trigger `0x19` only through TechnoClass when both sides have `+0x418`; `0x19` clears `+0x418` before propagation | `0x006F4BA6..0x006F4BAD`, `0x006F4C50..0x006F4C89` | needs focused ordering test if not already explicit | `RefineryDockContacts::contact_entered`, `release_contact`, `cancel_miner` | Contact-entered cleanup should mirror conditional `0x19` clear, but not require a direct exit-function `0x19` send | proposed test: `break_clears_contact_entered_without_reciprocal_dock_link` | Do not implement exit helpers as directly sending `0x19` unconditionally |

## 10. Negative Facts / Do Not Do

- Do not use `BuildingClass::Receive_Radio(0x08)` as the stock refinery queue-accepted/queued handoff. It returns `1` for stock GAREFN/NAREFN after Techno cleanup, not `0x17`.
- Do not treat `0x15` as a dock-link writer. It queues sender mission `0x10` only.
- Do not create or require reciprocal `unit/building +0x2E4` for ordinary stock CMIN/HARV refinery unload.
- Do not run `ReleaseDockedHarvester` or `Force_Track(0x47)` on normal stock empty-cargo completion.
- Do not send direct `0x19` from stock state-4 exit or from `ReleaseDockedHarvester`; `0x19` is a TechnoClass/cancel-path consequence.
- Do not read `QueueingCell=4,1` or `DockingOffset0` for this `0x08`/`0x15` handoff slice.

## 11. Remaining Uncertainty

Static evidence does not prove the exact live frame when a lingering `+0x418` contact after stock state-4 exit is consumed by `UnitClass::PerCellProcess` and cascades through `0x08 -> 0x19/0x03`. The branch and gates are verified; runtime timing needs a debugger trace or deterministic replay instrumentation.

## 12. Stale Docs / Follow-up Wording

- `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` should replace "Case 0x08 Active in YR: YES - fires during any factory-exit or repair-dock approach" with: "Case 0x08 is active for factory/repair/bunker cleanup/queue contexts; stock GAREFN/NAREFN do not use its `0x17` queued reply and return `ROGER(1)` after Techno cleanup."
- The same report should replace "`0x180 leptons` = 3 cells distance threshold" with: "`0x180` is 384 leptons, i.e. 1.5 cells at 256 leptons/cell."
- Any doc that says normal stock CMIN/GAREFN/NAREFN exit uses `ReleaseDockedHarvester` or `Force_Track(0x47)` should say: "That helper is conditional on a nonzero reciprocal `+0x2E4` link; stock DockUnload exits via zero-link `Mission_Deploy_Building` state 4."
- Any doc that says `ReleaseDockedHarvester` sends `0x19` should say: "It directly sends only `BREAK(0x03)`; `0x19` can occur indirectly through TechnoClass `BREAK` handling if both `+0x418` flags are set."

## Sources

- Fresh Ghidra decompile: `BuildingClass::Receive_Radio @ 0x0043C2D0`
- Fresh Ghidra decompile: `UnitClass::PerCellProcess @ 0x00739EC0`
- Fresh Ghidra decompile: `UnitClass::Mission_Deploy_Building @ 0x0073D630`
- Fresh Ghidra decompile: `BuildingClass::ReleaseDockedHarvester @ 0x004595C0`
- Fresh Ghidra decompile: `TechnoClass::Receive_Radio @ 0x006F4AB0`
- Fresh Ghidra decompile: `RadioClass::Transmit_Radio_Impl @ 0x0065A970`
- Fresh Ghidra decompile: `RadioClass::Transmit_Radio_ToFirst @ 0x0065ACB0`
- Fresh Ghidra decompile: `RadioClass::Receive_Radio @ 0x0065A820`
- Fresh Ghidra decompile: `MissionClass::Queue_Mission @ 0x005B35E0`
- Fresh Ghidra assembly contexts: `0x0043C788..0x0043C7A0`, `0x0073E1F0..0x0073E289`, `0x004596E6..0x0045982C`, `0x006F4BA6..0x006F4BAD`, `0x006F4C50..0x006F4C89`, `0x0065A970..0x0065AA54`, `0x0065ACB0`
- Existing docs: `miner/BUILDING_RECEIVE_RADIO_0X08_CLEARANCE_QUEUE_GHIDRA_REPORT.md`
- Existing docs: `miner/BUILDING_RECEIVE_RADIO_0X15_DOCKUNLOAD_HANDOFF_GHIDRA_REPORT.md`
- Existing docs: `miner/RELEASEDOCKEDHARVESTER_EXIT_RADIO_0X19_GHIDRA_REPORT.md`
- Existing docs: `MISSION_DEPLOY_BUILDING_DOCKED_VS_UNDOCKED_BRANCH_GHIDRA_REPORT.md`
- Existing docs: `UNITCLASS_0X418_DOCK_FLAG_LIFECYCLE_AND_CONSUMERS_GHIDRA_REPORT.md`
- Existing docs: `CHRONO_MINER_FORCE_TRACK_0X47_EXIT_NAVCOM_STEP_GHIDRA_REPORT.md`
- Existing docs: `miner/STANDARD_REFINERY_0X2E4_WRITER_INVENTORY_GHIDRA_REPORT.md`
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`
- Rust scan: `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_tests.rs`
