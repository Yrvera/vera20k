# FootClass Receive_Radio 0x12 Move Fields/NavCom - Ghidra Research Report

**Address(es):** `0x004D8FB0` primary; `0x005B2DA0`, `0x005B3040`, `0x005B3060`, `0x005B35E0`, `0x004D94B0`, `0x0043C2D0` supporting.
**Investigation Mode:** exhaustive-slice.
**Claimed Scope:** `FootClass::Receive_Radio` case `0x12` field writes at `FootClass+0xB4` and `+0xC8..+0xD0`, payload coordinate source, write order, and immediate NavCom/retry interaction for standard CMIN refinery docking.
**Non-Scope:** Full radio protocol, all senders of `0x12`, post-dump departure track `0x47`, full `TechnoClass::Set_Destination` preprocessing, and runtime watchpoints.
**Confidence:** High for binary-verified field identity, source, and ordering; Medium for broader player-visible retry consequences because this slot did not run a live debugger.
**Active in YR:** Yes for standard CMIN/refinery docking; case `0x12` is reached from stock `DockUnload=yes` refinery admission.

## 1. Overview

`FootClass::Receive_Radio` case `0x12` is the FootClass `MOVE_TO_CELL` receiver. In the standard refinery path, `BuildingClass::Receive_Radio` case `0x0E` sends it to the harvester with a `CellClass*` payload for the accepted dock approach cell.

The disputed `+0xB4` field is not a team id in this path. It is `MissionClass::QueuedMission`. The disputed `+0xCC` write is not an independent chrono-miner timestamp. Case `0x12` writes the middle dword of the MissionClass dispatch timer triplet after setting destination: `+0xC8 = g_CurrentFrameCounter`, `+0xCC = local target coord Y`, `+0xD0 = 0`.

## 2. Class Layout / Key Offsets

| Offset | Role in this slice | Evidence | Active in YR |
|---:|---|---|---|
| `+0xAC` | `CurrentMission` | `MissionClass::GetCurrentMission @ 0x005B3040` reads `param+0xAC` before falling back | Yes; used by mission dispatch and case `0x12` gates |
| `+0xB4` | `QueuedMission` | Constructor `0x005B2DA0` initializes `param_1[0x2D] = -1`; `Queue_Mission @ 0x005B35E0` writes `param_1[0x2D] = mission`; `GetCurrentMission` falls back to it | Yes; read by case `0x12` |
| `+0xB8` | queued-mission aux byte | `Queue_Mission @ 0x005B35E0` clears byte at `param_1+0xB8` when it writes `+0xB4` | Yes; adjacent mission queue state |
| `+0xC8` | dispatch timer start frame | `Mission_Dispatch @ 0x005B3060` compares `g_CurrentFrameCounter - param_1[0x32]` against duration | Yes; case `0x12` writes current frame |
| `+0xCC` | middle dword in dispatch timer storage; case `0x12` source is target coord Y local | case `0x12` assembly `0x004D91F6` reads `[ESP+0x20]`, then `0x004D920A` writes `[this+0xCC]` | Yes; written on standard path |
| `+0xD0` | dispatch timer duration | `Mission_Dispatch @ 0x005B3060` reads `param_1[0x34]`; case `0x12` writes zero at `0x004D920D` | Yes; zero means no dispatch wait |
| `+0x5A4` | NavCom destination pointer | `FootClass::Set_Destination_Internal @ 0x004D94B0` writes `param_1[0x169] = target` | Yes; set via case `0x12` vtable `+0x480` call |

## 3. Core Logic

Case `0x12` first checks `*payload`. If non-null, it calls the payload target vtable `+0x48` with a stack buffer at `ESP+0x1C`, reads target X/Y from that returned coord, converts X/Y leptons to cells using `(value + ((value >> 31) & 0xFF)) >> 8`, calls self vtable `+0x1B8` for current cell, and returns `0x14` if already on the target cell.

If not already there, it reads effective mission through vtable `+0x184`. When effective mission is `5` and `QueuedMission == -1`, it queues mission `2` through vtable `+0x1E8`. If `QueuedMission == 7` and vtable `+0x200` returns true, it calls vtable `+0x1EC`.

Then it calls vtable `+0x480` with `(*payload, 1)`. For FootClass-derived units in this path, this reaches `FootClass::Set_Destination_Internal @ 0x004D94B0`, which writes NavCom (`+0x5A4`) and resets path retry fields. Only after that destination call returns does case `0x12` write the MissionClass timer triplet in this order:

1. `+0xC8 = g_CurrentFrameCounter` (`0x004D91F1`, `0x004D9203`).
2. `+0xCC = [ESP+0x20]` (`0x004D91F6`, `0x004D920A`), the second dword of the target coord buffer for the standard non-null payload path.
3. `+0xD0 = 0` (`0x004D920D`).

`MissionClass::Mission_Dispatch @ 0x005B3060` uses `+0xC8` and `+0xD0` to decide whether to wait. Since case `0x12` writes `+0xD0 = 0`, the next mission dispatch is not delayed by this timer. The checked dispatch path writes `+0xCC` with an uninitialized local after mission handlers return, but does not use `+0xCC` as the wait duration.

## 4. INI Keys

| Key | Stock YR location | Effect on this slice | Active in YR |
|---|---|---|---|
| `[CMIN] Dock=NAREFN,GAREFN` | `ini/rulesmd.ini:7361` | Makes Chrono Miner eligible to dock with Allied/Soviet refineries | Yes |
| `[CMIN] Harvester=yes` | `ini/rulesmd.ini:7364` | Puts CMIN on harvester return/dock behavior | Yes |
| `[CMIN] Teleporter=yes` | `ini/rulesmd.ini:7396` | Enables chrono locomotor behavior; does not alter case `0x12` writes | Yes |
| `[CMIN] Locomotor={4A582747-9839-11d1-B709-00A024DDAFD1}` | `ini/rulesmd.ini:7398` | Teleport locomotor for CMIN; NavCom destination is still set through case `0x12` | Yes |
| `[General] BlockagePathDelay=60` | `ini/rulesmd.ini:3107` | Value copied into `+0x670` by `Set_Destination_Internal` retry reset | Yes |
| `[General] ChronoHarvTooFarDistance=50` | `ini/rulesmd.ini:294` | Upstream return decision; not read by case `0x12` | Yes |

## 5. Integration Points

`BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x0E` is the standard active refinery sender. It writes a `CellClass*` payload for the accepted cell, sends `0x12`, and treats return `0x14` as already-there before sending later dock messages.

`FootClass::Set_Destination_Internal @ 0x004D94B0` clears NavCom aux `+0x5A0`, writes NavCom `+0x5A4`, calls the locomotor `Head_To_Coord` path for non-null targets, clears path-failed flag `+0x6B7`, writes block retry frame fields `+0x668/+0x66C`, writes `+0x670 = RulesClass+0x1768`, and resets walker retry fields `+0x640/+0x644/+0x648`. Therefore, case `0x12` refreshes movement retry state before zeroing the mission-dispatch delay.

For CMIN docking, the player-visible consequence is immediate accepted-cell navigation with retry timers restarted. There is no binary evidence in this slice that `+0xCC` itself drives chrono miner dock timing; the load-bearing timer value for dispatch waiting is `+0xD0`, which case `0x12` clears to zero.

## 6. Reconciliation With Prior Reports

`FOOTCLASS_RADIO_MOVE_FIELDS_0XB4_0XCC_GHIDRA_REPORT` is confirmed: `+0xB4` is queued mission, case `0x12` writes `+0xC8..+0xD0`, and `+0xCC` comes from the local target coord buffer on the standard non-null path.

`FOOTCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT` is partially superseded on naming: its raw switch/case set and addresses are confirmed, but its older “team/sub-mission” wording for `+0xB4` is too weak. Ghidra evidence identifies it as `MissionClass::QueuedMission`.

`NAVCOM_LIFECYCLE_GHIDRA_REPORT` is confirmed for the immediate interaction: `Set_Destination_Internal` owns NavCom and retry reset. This slot narrows the ordering for case `0x12`: NavCom/retry update occurs before the `+0xC8/+0xCC/+0xD0` triplet write.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FootClass::Receive_Radio` case `0x12` | verified | decompile `0x004D8FB0`; assembly context `0x004D9139..0x004D9210` | null-payload source remains non-standard edge |
| Payload coordinate source | verified | target vtable `+0x48` call at `0x004D914B`; local buffer `ESP+0x1C`; `[ESP+0x20]` read at `0x004D91F6` | none for standard non-null path |
| `+0xB4` identity | verified | `0x005B2DA0`, `0x005B3040`, `0x005B35E0` | none |
| Dispatch timer use of `+0xC8/+0xD0` | verified | `Mission_Dispatch @ 0x005B3060` | broader readers of `+0xCC` not scanned outside dispatch |
| NavCom/retry interaction | verified | `FootClass::Set_Destination_Internal @ 0x004D94B0` | full locomotor Process behavior belongs to NAVCOM report |
| Standard refinery activation | verified | `BuildingClass::Receive_Radio @ 0x0043C2D0`; `[CMIN]` INI lines | none |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - What is `FootClass+0xB4` in this path? It is `MissionClass::QueuedMission`, initialized to `-1`, read as fallback mission, and written by `Queue_Mission`. Evidence: `0x005B2DA0`, `0x005B3040`, `0x005B35E0`. Active in YR: Yes.

[RESOLVED] OQ-2 - What is the case `0x12` payload coordinate source for `+0xCC`? In the standard non-null payload path, target vtable `+0x48` fills the local coord buffer at `ESP+0x1C`; `+0xCC` receives the second dword (`ESP+0x20`), target Y. Evidence: `0x004D9146..0x004D914E`, `0x004D91F6`, `0x004D920A`. Active in YR: Yes.

[RESOLVED] OQ-3 - What is the exact write order? `Set_Destination(*payload,1)` first, then `+0xC8`, then `+0xCC`, then `+0xD0`. Evidence: `0x004D91E1..0x004D920D`. Active in YR: Yes.

[RESOLVED] OQ-4 - How does this interact with movement retry? `Set_Destination_Internal` sets NavCom and resets path/block retry fields before case `0x12` clears mission dispatch duration. Evidence: `0x004D94B0`, `0x004D91EB..0x004D920D`. Active in YR: Yes.

[DEFERRED] OQ-5 - What value lands in `+0xCC` for a null `*payload` sender? Category: out-of-scope. Standard refinery `0x0E` sends a non-null `CellClass*` payload before `0x12`; null sender inventory requires a broader radio sweep.

## Sources

- Ghidra read-only decompile: `FootClass::Receive_Radio @ 0x004D8FB0`.
- Ghidra read-only assembly context: `0x004D9139`, `0x004D914B`, `0x004D91E1`, `0x004D91F1`, `0x004D920A`.
- Ghidra read-only decompile: `MissionClass::Constructor @ 0x005B2DA0`; `MissionClass::GetCurrentMission @ 0x005B3040`; `MissionClass::Mission_Dispatch @ 0x005B3060`; `MissionClass::Queue_Mission @ 0x005B35E0`.
- Ghidra read-only decompile: `FootClass::Set_Destination_Internal @ 0x004D94B0`; `BuildingClass::Receive_Radio @ 0x0043C2D0`.
- INI: `ini/rulesmd.ini:294`, `ini/rulesmd.ini:3107`, `ini/rulesmd.ini:7361`, `ini/rulesmd.ini:7364`, `ini/rulesmd.ini:7396`, `ini/rulesmd.ini:7398`.
- Prior docs: `FOOTCLASS_RADIO_MOVE_FIELDS_0XB4_0XCC_GHIDRA_REPORT.md`, `FOOTCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`, `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md`.
