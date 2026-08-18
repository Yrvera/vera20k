# Building Receive Radio 0x15 DockUnload Handoff - Ghidra Research Report

**Address(es):** `0x0043C2D0` (`BuildingClass__Receive_Radio`), case `0x15` hot block at `0x0043C788..0x0043C7A0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `BuildingClass::Receive_Radio` case `0x15` for stock `DockUnload=yes` refineries (`GAREFN`/`NAREFN`) and its immediately-called radio/mission slots needed to determine whether the handoff writes or references unit/building `+0x2E4`.  
**Non-Scope:** Full `Mission_Deploy_Building`, full refinery dock admission (`0x0E`), normal/interrupt exit teardown, Bunker/UnitRepair/Hospital/Armory behavior beyond distinguishing their `0x15` branches from stock refineries.  
**Confidence:** High. The primary switch case, radio transmit helper, mission getter, and mission queue callee were decompiled fresh.  
**Active in YR:** Yes for stock CMIN/HARV docking with GAREFN/NAREFN.

## 1. Overview

For standard YR ore refineries, `BuildingClass::Receive_Radio` case `0x15` does one thing: it queues mission `0x10` on the sender unit with the second argument `0`. It does not write `building+0x2E4`, does not write `unit+0x2E4`, and does not call any callee that can write `+0x2E4` on this exact `DockUnload=yes` path.

The nearby branches for UnitRepair/UnitReload/Hospital/Armory/Bunker do set `building+0x6DD` and queue building missions, but those branches are gated by different BuildingType flags and are not the stock `GAREFN`/`NAREFN` path.

## 2. Key Offsets And Slots

| Field / slot | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| BuildingType `+0x16B3` | `DockUnload=yes`; selects the stock refinery `0x15` branch | `BuildingClass__Receive_Radio @ 0x0043C2D0`, assembly `0x0043C788` reads `[Type+0x16B3]`; `rulesmd.ini:[GAREFN]/[NAREFN] DockUnload=yes` | Yes |
| Building vtable `+0x184` | `MissionClass__GetCurrentMission`; first guard rejects mission `0x13` | `0x0043C2D0` decompile; `MissionClass__GetCurrentMission @ 0x005B3040` | Yes |
| Sender vtable `+0x1E8` | `MissionClass__Queue_Mission` | case `0x15` assembly `0x0043C79A PUSH 0`, `0x0043C79C PUSH 0x10`, `0x0043C7A0 CALL [EDX+0x1E8]`; callee `0x005B35E0` | Yes |
| Radio vtable `+0x274` | `RadioClass__Transmit_Radio_ToFirst`; sender-side path used by pad arrival to send `0x15` | `UnitClass__PerCellProcess @ 0x00739EC0` calls `param_1->vtable+0x274` with `0x15`; `RadioClass__Transmit_Radio_ToFirst @ 0x0065ACB0` forwards through `+0x27C` | Yes |
| Radio vtable `+0x27C` / receiver `+0x194` | `Transmit_Radio_Impl` dispatches target `Receive_Radio` | `RadioClass__Transmit_Radio_Impl @ 0x0065A970` calls target vtable `+0x194` for non-HELLO/non-BREAK messages | Yes |
| Building/unit `+0x2E4` | Mutual docked pointer in other refinery docs; not read or written by this case/callee path | Negative evidence from decompiles of `0x0043C2D0`, `0x005B3040`, `0x005B35E0`, `0x0065ACB0`, `0x0065A970` | No for this slice |

## 3. Core Logic

### 3.1 Case `0x15` branch order

Fresh decompile of `BuildingClass__Receive_Radio @ 0x0043C2D0` shows this case order:

```text
case 0x15:
  if self.GetCurrentMission() == 0x13:
      return 10

  type = self.Type

  if type.UnitAbsorb(+0x16AE): return 1
  if type.InfAbsorb(+0x16AF): return 1

  if type.UnitRepair(+0x16A9)
     or type.UnitReload(+0x16AA)
     or type.Hospital(+0x16C1)
     or type.Armory(+0x16C2):
      self+0x6DD = 1
      self.Queue_Mission(0x14, 0)
      sender.Queue_Mission(0, 0)
      return 1

  if type.Bunker(+0x16AB):
      self+0x6DD = 1
      self.Queue_Mission(0x14, 0)
      return 1

  if type.DockUnload(+0x16B3):
      sender.Queue_Mission(0x10, 0)
      return 1

  return TechnoClass__Receive_Radio(sender, 0x15, payload)
```

**Active in YR:** Yes for the `DockUnload` branch. `rulesmd.ini` sets `DockUnload=yes` on `[GAREFN]` and `[NAREFN]`, and `[CMIN]`/`[HARV]` use `Dock=NAREFN,GAREFN` plus `Harvester=yes`.

### 3.2 Exact stock DockUnload instruction sequence

The stock refinery branch is the small assembly block at `0x0043C788..0x0043C7A0`:

```asm
0043C788  MOV  CL, byte ptr [EAX + 0x16B3]   ; BuildingType.DockUnload
0043C78E  TEST CL, CL
0043C790  JZ   0x0043CE43                    ; fall through if not DockUnload
0043C796  MOV  ECX, dword ptr [ESP + 0x54]   ; sender object
0043C79A  PUSH 0x0
0043C79C  PUSH 0x10
0043C79E  MOV  EDX, dword ptr [ECX]
0043C7A0  CALL dword ptr [EDX + 0x1E8]       ; sender.Queue_Mission(0x10, 0)
```

There is no memory access to offset `0x2E4` in this block. The only selected receiver-side flag read is `BuildingType+0x16B3`; the only side effect is the sender virtual call.

**Active in YR:** Yes. Stock refineries take this branch.

### 3.3 Immediate mission callees do not write `+0x2E4`

`MissionClass__GetCurrentMission @ 0x005B3040`:

```text
current = *(this + 0xAC)
if current == -1:
    current = *(this + 0xB4)
return current
```

This guard only reads mission fields `+0xAC` and `+0xB4`.

`MissionClass__Queue_Mission @ 0x005B35E0` for the exact call `Queue_Mission(0x10, 0)`:

```text
current = this[0x2B]          ; +0xAC
if current is not protected:
    if mission differs from current/queued:
        this[0x2D] = mission  ; +0xB4 queued mission
        byte(this+0xB8) = 0
    if commence_now != 0:
        maybe call vtable+0x200 and vtable+0x1EC
```

For the DockUnload call the second argument is `0`, so the `commence_now` block does not run. The only possible writes are `+0xB4` and `+0xB8`, not `+0x2E4`.

**Active in YR:** Yes. These are inherited MissionClass slots used by BuildingClass and UnitClass.

### 3.4 Immediate radio dispatch does not add a hidden `+0x2E4` write

The pad-arrival sender path in `UnitClass__PerCellProcess @ 0x00739EC0` calls:

```text
FootClass__PerCellProcess(2)
this.vtable+0x274(0x15)
locomotor.vtable+0x5C()
```

`RadioClass__Transmit_Radio_ToFirst @ 0x0065ACB0` checks the first contact slot and forwards to `+0x27C`:

```text
if Contacts[0] != 0:
    return this.vtable+0x27C(message, &g_RadioScratchBuffer, Contacts[0])
return 0
```

For message `0x15`, `RadioClass__Transmit_Radio_Impl @ 0x0065A970` takes the generic "any other msg" path and calls the target receiver through vtable `+0x194`. That dispatch is how the building reaches `BuildingClass__Receive_Radio @ 0x0043C2D0`. The radio helper does not write `+0x2E4`; its special writes are contact-array updates for `HELLO(0x02)` and `BREAK(0x03)`, neither of which applies to message `0x15`.

**Active in YR:** Yes. This is the standard harvester pad-arrival send path.

## 4. INI Keys

| INI data | Values | Effect for this slice | Active in YR |
|---|---|---|---|
| `rulesmd.ini:[GAREFN]` | `DockUnload=yes`, `Refinery=yes`, `FreeUnit=CMIN`, `Storage=200` | Selects case `0x15` DockUnload branch | Yes |
| `rulesmd.ini:[NAREFN]` | `DockUnload=yes`, `Refinery=yes`, `FreeUnit=HARV`, `Storage=200` | Same branch as GAREFN | Yes |
| `rulesmd.ini:[CMIN]` | `Dock=NAREFN,GAREFN`, `Harvester=yes` | CMIN can send this `0x15` to stock refineries | Yes |
| `rulesmd.ini:[HARV]` | `Dock=NAREFN,GAREFN`, `Harvester=yes` | HARV can send this `0x15` to stock refineries | Yes |

No stock GAREFN/NAREFN line in the checked local block sets UnitRepair, UnitReload, Bunker, Hospital, Armory, UnitAbsorb, or InfAbsorb, so those case `0x15` branches are not the standard refinery branch.

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `UnitClass__PerCellProcess @ 0x00739EC0` | Sends `0x15` from unit to current radio contact at pad arrival | decompile shows `vtable+0x274(0x15)` after `FootClass__PerCellProcess(2)` | Yes |
| `RadioClass__Transmit_Radio_ToFirst @ 0x0065ACB0` | Forwards `0x15` to first contact through `+0x27C` | fresh decompile | Yes |
| `RadioClass__Transmit_Radio_Impl @ 0x0065A970` | Generic non-HELLO/non-BREAK dispatch to target `Receive_Radio` slot `+0x194` | fresh decompile | Yes |
| `BuildingClass__Receive_Radio @ 0x0043C2D0` | Receiver case `0x15`; queues sender mission `0x10` for `DockUnload=yes` | fresh decompile and assembly `0x0043C788..0x0043C7A0` | Yes |
| `MissionClass__GetCurrentMission @ 0x005B3040` | Reads mission state for the case's `0x13` reject guard | fresh decompile | Yes |
| `MissionClass__Queue_Mission @ 0x005B35E0` | Queues sender mission `0x10` with `commence_now=0` | fresh decompile | Yes |

## 6. Answer To The Target Question

`BuildingClass::Receive_Radio` case `0x15` for stock `DockUnload=yes` refineries only queues mission `0x10` on the sender unit and returns `ROGER(1)`.

It does not establish the mutual unit/building `+0x2E4` link directly. The immediate callees needed for this branch also do not establish it:

- `MissionClass__GetCurrentMission @ 0x005B3040` reads `+0xAC` and maybe `+0xB4`.
- `MissionClass__Queue_Mission @ 0x005B35E0`, with `(0x10, 0)`, writes at most `+0xB4` and `+0xB8`.
- `RadioClass__Transmit_Radio_ToFirst @ 0x0065ACB0` and `Transmit_Radio_Impl @ 0x0065A970` route the message through contacts and `Receive_Radio`; for message `0x15` they do not touch `+0x2E4`.

Verified binary finding: if a stock refinery `+0x2E4` writer exists, it is not in `BuildingClass::Receive_Radio` case `0x15` and not in the immediate radio/mission callees used by that DockUnload branch.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingClass__Receive_Radio @ 0x0043C2D0` case `0x15` | verified | decompile; assembly `0x0043C788..0x0043C7A0` | none for requested slice |
| Stock `DockUnload=yes` branch | verified | `[Type+0x16B3]` read and sender `+0x1E8(0x10,0)` call | none |
| UnitRepair/Reload/Hospital/Armory/Bunker branches | touched-not-exhausted | case `0x15` decompile | full non-refinery behavior out-of-scope |
| `MissionClass__GetCurrentMission @ 0x005B3040` | verified | fresh decompile | none |
| `MissionClass__Queue_Mission @ 0x005B35E0` for `(0x10,0)` | verified | fresh decompile | none |
| `RadioClass__Transmit_Radio_ToFirst @ 0x0065ACB0` | verified | fresh decompile | none |
| `RadioClass__Transmit_Radio_Impl @ 0x0065A970` message `0x15` path | verified | fresh decompile | none |
| Full `Mission_Deploy_Building` writer search | not-touched | outside slot target | covered by sibling swarm slots |
| Full stock refinery `+0x2E4` writer inventory | not-touched | outside slot target | covered by sibling swarm slot 1 |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Does `BuildingClass::Receive_Radio` case `0x15` write `building+0x2E4` on the stock DockUnload branch? No. The branch reads `Type+0x16B3`, calls sender `vtable+0x1E8(0x10,0)`, and returns 1. Evidence: `0x0043C788..0x0043C7A0`.

[RESOLVED] OQ-2 - Does the sender mission call write `unit+0x2E4` through `MissionClass__Queue_Mission`? No for this call. `0x005B35E0` writes queued mission fields `+0xB4` and `+0xB8`; `commence_now=0` skips additional calls. Evidence: decompile `0x005B35E0`.

[RESOLVED] OQ-3 - Does the initial mission guard read or write `+0x2E4`? No. `MissionClass__GetCurrentMission @ 0x005B3040` reads `+0xAC`, and if that is `-1`, reads `+0xB4`. Evidence: decompile `0x005B3040`.

[RESOLVED] OQ-4 - Does the immediate radio transmit helper write `+0x2E4` for message `0x15`? No. `0x15` uses the generic target-receive path in `Transmit_Radio_Impl`; contact-array mutations are special to `0x02` and `0x03`. Evidence: decompile `0x0065ACB0`, `0x0065A970`.

[RESOLVED] OQ-5 - Is the branch active for stock YR CMIN/HARV and GAREFN/NAREFN? Yes. `[CMIN]` and `[HARV]` set `Dock=NAREFN,GAREFN` and `Harvester=yes`; `[GAREFN]` and `[NAREFN]` set `DockUnload=yes`. Evidence: `ini/rulesmd.ini`.

## Sources

- Ghidra `decompile_function 0043C2D0` - primary `BuildingClass__Receive_Radio` switch.
- Ghidra `get_assembly_context` at `0043C788`, `0043C796`, `0043C7A0` - exact stock DockUnload branch.
- Ghidra `decompile_function 005B3040` - `MissionClass__GetCurrentMission`.
- Ghidra `decompile_function 005B35E0` - `MissionClass__Queue_Mission`.
- Ghidra `decompile_function 0065ACB0` - `RadioClass__Transmit_Radio_ToFirst`.
- Ghidra `decompile_function 0065A970` - `RadioClass__Transmit_Radio_Impl`.
- Ghidra `decompile_function 00739EC0` - `UnitClass__PerCellProcess` pad-arrival sender.
- `docs/research/BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`
- `docs/research/UNITCLASS_PERCELLPROCESS_CHRONO_MINER_DOCK_ARRIVAL_00739EC0_GHIDRA_REPORT.md`
- `docs/research/RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md`
- `ini/rulesmd.ini` local stock YR data for CMIN/HARV/GAREFN/NAREFN.
