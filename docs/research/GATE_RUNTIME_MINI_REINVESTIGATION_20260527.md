# Gate Runtime Mini Reinvestigation - Close Delay And Live Obstruction

**Date:** 2026-05-27  
**Mode:** small re-investigate / bounded spot-check  
**Scope:** Gate mission/runtime details still needed before implementing the minimal gate state machine: `GateCloseDelay`, open/close transition duration source, live obstruction scan, and passability timing.  
**Non-scope:** full render frame composition, audio parity, campaign fixture frequency, and non-building overlay gates.  
**Active in YR:** Conditional. The path is live for `BuildingTypeClass Gate=yes` buildings such as stock `[GAGATE_A]`, when a path/obstacle check contacts the gate.

## Summary

The minimal Rust gate runtime should be implemented. The remaining uncertainty from the previous gate-writer report is now mostly closed:

- `GateCloseDelay=` is parsed into `BuildingTypeClass+0xE28` as a `double`.
- Building mission `0x18` reads `BuildingType+0xE28`, multiplies by `900.0`, truncates through `_ftol`, and uses that as the open-hold timer.
- Open and close transition animation durations both use `TechnoType/BuildingType+0x3C8/+0x3CC` (`DeployTime=`), not `GateCloseDelay`.
- The hold-state obstruction check is a live object-list scan over the building's coordinate list. It ignores only the gate building itself; any other object in the scanned cell object chain keeps the gate open.
- Passability remains stable-open only: mission `0x18` plus helper bytes `active=0, open_side=1`. Opening and closing are blocked.

## Fresh Binary Checks

### `GateCloseDelay=` parser

String search found `GateCloseDelay` at `0x0081A8DC`; xref `0x00460DD8` is inside `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`.

Relevant assembly context:

```text
00460dd0  MOV EAX,dword ptr [EBP + 0xe28]
00460dca  MOV EDX,dword ptr [EBP + 0xe2c]
00460dd6  PUSH EDX
00460dd7  PUSH EAX
00460dd8  PUSH 0x81a8dc        ; "GateCloseDelay"
00460ddd  PUSH EBX
00460de0  CALL 0x005283d0      ; ReadDouble
00460de5  FSTP double ptr [EBP + 0xe28]
```

Conclusion: `GateCloseDelay` is definitely `BuildingType+0xE28` / `+0xE2C`, defaulting from the previous value.

### Mission `0x18` timer usage

`BuildingClass` mission `0x18` at `0x0044E440` uses two different durations:

- Opening start in state 0 calls `0x004A51F0` with `BuildingType+0x3C8/+0x3CC`.
- Closing start in state 3 calls `0x004A5240` with `BuildingType+0x3C8/+0x3CC`.
- State 0 and state 2 seed/reseed the hold timer from `BuildingType+0xE28 * 900.0`.

Key assembly:

```text
0044e484  FLD double ptr [EAX + 0xe28]
0044e48a  FMUL double ptr [0x007e27f8] ; 900.0
0044e490  CALL 0x007c5f00              ; ftol/truncate
...
0044e51a  MOV EAX,dword ptr [EDX + 0x3cc]
0044e521  MOV ECX,dword ptr [EDX + 0x3c8]
0044e52a  CALL 0x004a51f0              ; StartOpening
...
0044e5da  MOV ECX,dword ptr [EAX + 0x3cc]
0044e5e0  MOV EDX,dword ptr [EAX + 0x3c8]
0044e5ee  CALL 0x004a5240              ; StartClosing
...
0044e69f  MOV ECX,dword ptr [ESI + 0x520]
0044e6a5  FLD double ptr [ECX + 0xe28]
0044e6ab  FMUL double ptr [0x007e27f8]
0044e6b1  CALL 0x007c5f00              ; reseed hold timer
```

For stock `[GAGATE_A] GateCloseDelay=.2`, the hold duration is `trunc(0.2 * 900) = 180` frames. Stock `DeployTime=.044` gives transition duration `trunc(0.044 * 900) = 39` frames.

### Live obstruction scan

`FUN_0044E3A0` is called only by the gate mission state-2 hold logic. It:

1. Gets the building origin through vtable `+0x1B8`.
2. Gets a coordinate list through vtable `+0x108(0)`.
3. Iterates until sentinel `(0x7FFF, 0x7FFF)`.
4. For each coordinate, calls `MapClass__Get_CellClass`.
5. Reads `CellClass+0xE4`, the live object chain for that cell.
6. Walks the chain through object `+0x30`.
7. Sets return true if it finds any object pointer other than the gate building itself.

Key assembly:

```text
0044e3b2  CALL dword ptr [EAX + 0x1b8] ; origin
0044e3c4  CALL dword ptr [EDX + 0x108] ; coord list
...
0044e40d  CALL 0x005657a0              ; MapClass__Get_CellClass
0044e412  MOV EAX,dword ptr [EAX + 0xe4]
0044e41c  CMP EDI,EAX                  ; same building?
0044e420  MOV EAX,dword ptr [EAX + 0x30]
0044e429  MOV byte ptr [ESP + 0x13],0x1 ; obstruction found
```

Conclusion: gate close holding must be based on live object-list scanning, not static PathGrid, and not a precomputed occupancy shortcut.

## Rust Handoff

Implement the minimal gate runtime with these requirements:

| Behavior | Required Rust effect |
|---|---|
| `Gate=yes` building flag | Already parsed in current work; keep it as building-type data, separate from overlay gates. |
| `GateCloseDelay` | Parse/store as a double-like rules value or deterministic fixed representation; convert to ticks with `trunc(value * 900.0)`. |
| `DeployTime` for gate transition | Parse/store `DeployTime`; use `trunc(value * 900.0)` for opening and closing transitions. |
| Mission `0x18` local state | Add the minimal states: setup/opening wait/open hold/begin close/closing wait/post-close. |
| Helper state | ClosedStable, Opening, OpenStable, Closing; passability only for mission `0x18` and OpenStable. |
| Allied contact | Closed/closing allied gate contact assigns/starts mission `0x18` but the current entry check still blocks. Do not instant-pass. |
| Hold obstruction | During open hold, scan live objects on the gate coordinate list and ignore only the gate itself. Any other object reseeds the hold timer. |

Suggested focused tests:

- `gate_close_delay_dot2_truncates_to_180_ticks`
- `gate_deploy_time_dot044_truncates_to_39_ticks`
- `allied_closed_gate_request_starts_opening_but_same_check_blocks`
- `gate_open_hold_reseeds_while_live_object_occupies_gate_cells`
- `stable_open_gate_passable_but_opening_and_closing_block`

## Open Questions

- The exact semantic name of the vtable `+0x484(0,1)` call after stable close remains out of scope. It likely handles a post-close building state/dirty update and is not needed for the first passability/runtime fix.
- Campaign/skirmish frequency was not runtime-captured. Static liveness plus stock INI presence is enough for implementation priority.

## Sources

- Fresh Ghidra read-only checks: `0x0044E440`, `0x0044E3A0`, `0x00452540`, `0x004A51F0`, `0x004A5240`, `0x004A5360`, `0x0045FE50`, string `0x0081A8DC`.
- Existing verified docs: `docs/research/GATE_WRITER_STATE_MACHINE_GHIDRA_REPORT.md`, `docs/research/GATE_MECHANIC_BUILDING_GATE_PASSABILITY_GHIDRA_REPORT.md`, `docs/research/TRANSPORT_DOOR_TIMING_RADIO_0X11_DEPLOY_TRACKER_GHIDRA_REPORT.md`.
- Stock data: `ini/rulesmd.ini` `[GAGATE_A] Gate=yes`, `DeployTime=.044`, `GateCloseDelay=.2`.
