# Chrono Miner Force_Track 0x47 Exit NavCom Step - Ghidra Research Report

**Address(es):** `0x0073D630`, `0x004595C0`, `0x004B0C40`, `0x004AFD40`, `0x004D94B0`, `0x00741970`, `0x004DF0D0`, `0x004AFE00`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Force_Track `0x47` refinery exit semantics, DriveLocomotion head/destination split, and NavCom/movement destination set-or-clear behavior immediately after stock `Mission_Deploy_Building` exit and conditional `ReleaseDockedHarvester` exit.
**Non-Scope:** Full refinery dock FSM, full `Find_Nearby_Passable_Cell` ranking, runtime pixel capture, all blocker/crush branches, and unrelated radio cases.
**Confidence:** High for static binary call ordering, fields, and active-YR reachability. Medium only for exact first rendered frame because that requires runtime capture.
**Active in YR:** Yes for stock `Mission_Deploy_Building` state-4 exit; Conditional for `ReleaseDockedHarvester` Force_Track exit, because it requires nonzero reciprocal dock link `unit+0x2E4/building+0x2E4`, which stock DockUnload refinery arrival does not create.

## 1. Overview

The older `CHRONO_MINER_FORCE_TRACK_0X47_REFINERY_EXIT_GHIDRA_REPORT.md` and `CHRONO_MINER_POST_UNLOAD_EXIT_ANCHOR_GHIDRA_REPORT.md` correctly describe what `BuildingClass::ReleaseDockedHarvester @ 0x004595C0` does when reached: it calls DriveLocomotion slot `+0x70` as `Force_Track(0x47, center.x-0x80, center.y+0x80, z)`, then installs a separate Foot/NavCom destination through unit vtable `+0x480`.

The reachability claim in those reports is stale for stock CMIN/HARV -> GAREFN/NAREFN unload. Fresh decompile of `UnitClass::Mission_Deploy_Building @ 0x0073D630` shows the stock zero-link DockUnload FSM drains cargo and exits through state 4 without calling `ReleaseDockedHarvester`, without calling `Force_Track`, and without installing a new exit destination. It clears dock-active byte `+0x6D1`, optionally stops the locomotor if it is still moving, and returns to Harvest/normal mission scheduling.

## 2. Key Offsets

| Offset | Owner | Meaning | Evidence | Active in YR |
|---:|---|---|---|---|
| `+0x2E4` | Unit/Building | reciprocal dock-link pointer used by `ReleaseDockedHarvester` | `0x0073D630` branch on `unit[0xB9]`; `0x004595C0` reads/clears | Conditional; stock DockUnload leaves it zero |
| `+0x5A4` | FootClass | NavCom destination target pointer | `FootClass__Set_Destination_Internal @ 0x004D94B0` writes `param_1[0x169]` | Yes |
| `+0x674` | FootClass | active locomotor pointer | `0x004595C0`, `0x004D94B0`, `0x0073D630` vtable calls | Yes |
| `+0x6D1` | UnitClass byte | dock/unload active latch initialized during unload startup and cleared on stock state-4 exit | `0x0073E0xx`, `0x0073E0F4`, `0x0073E214`/`0x0073E13A` | Yes |
| `+0x2F` | UnitClass mission substate | unload FSM state; stock exit branch is state 4 | `0x0073D630` switch | Yes |
| Drive `+0x3C/+0x40/+0x44` | DriveLocomotion | `head_to` coordinate written by `Force_Track` | `0x004B0C40` | Conditional for forced-track path |
| Drive `+0x30/+0x34/+0x38` | DriveLocomotion | locomotor destination coordinate | `0x004B0C40`, `0x004AFD40`, `0x004AFE00` | Yes when Drive active |
| Drive `+0x54/+0x58` | DriveLocomotion | forced track index and point index | `0x004B0C40` | Conditional for forced-track path |

## 3. Verified Binary Findings

### 3.1 Stock unload exit path does not Force_Track

`UnitClass::Mission_Deploy_Building @ 0x0073D630` splits immediately on `unit+0x2E4`. If `unit+0x2E4 == 0`, it enters the standard unload FSM. That FSM uses adjacent-cell lookup (`g_refinery_unload_adjacent_lookup_dx/dy`) to find the refinery, drains storage in state 3, sets substate 4, waits on `BuildingClass+0x57C` if needed, clears `unit+0x6D1`, and returns to harvest/mission scheduling.

No call to `BuildingClass::ReleaseDockedHarvester`, no locomotor slot `+0x70`, and no unit vtable `+0x480` destination assignment occur in the stock state-4 exit branch.

**Active in YR: Yes.** Evidence: `0x0073D630` state-4 branch; stock `[CMIN] Harvester=yes`, `Dock=NAREFN,GAREFN`, `Teleporter=yes`, `Locomotor={4A582747...}` at `ini/rulesmd.ini:7351`, `7361`, `7364`, `7396`, `7398`; `[GAREFN] DockUnload=yes` at `ini/rulesmd.ini:11726` and `[NAREFN] DockUnload=yes` at `ini/rulesmd.ini:12519`.

### 3.2 ReleaseDockedHarvester is conditional reciprocal-link exit

When `unit+0x2E4 != 0`, `Mission_Deploy_Building` looks up the adjacent building and calls `BuildingClass::ReleaseDockedHarvester @ 0x004595C0` at `0x0073D66D`. That function reads `building+0x2E4`; if null, it only clears `building+0x718`, sets building mission `5`, and returns. If non-null and the docked unit reports locomotion type `1`, it clears `unit+0x2E4`, calls active locomotor slot `+0x58`, calls `Force_Track`, sets speed `1.0`, computes a passable cell from `building.Get_Cell_Packed()+(-1,+1)`, calls unit vtable `+0x480(dest,1)`, then sets mission `MOVE=2`.

**Active in YR: Conditional.** Evidence: `0x0073D630` else-branch to `0x004595C0`; stock DockUnload nonzero reciprocal link is refuted by `BUILDING_RECEIVE_RADIO_0X15_DOCKUNLOAD_HANDOFF_GHIDRA_REPORT.md` (no `+0x2E4` writer) and `CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING_GHIDRA_REPORT.md` (reciprocal writer is Bunker-gated `FUN_00458E50`, not stock refinery).

### 3.3 Force_Track has its own internal target/destination, separate from Foot NavCom

`DriveLocomotionClass::Force_Track @ 0x004B0C40` writes the supplied track index to Drive `+0x54`, resets point index `+0x58` to `0`, writes the supplied coordinate to `head_to` fields `+0x3C/+0x40/+0x44`, calls `Apply_Track_Delta(target,1)` on the accepted path, then writes the same coordinate to Drive destination fields `+0x30/+0x34/+0x38` and speed `1.0`.

This is not the Foot/NavCom destination. It is local DriveLocomotion state. The Foot/NavCom destination is `Foot+0x5A4`, written later by `FootClass::Set_Destination_Internal @ 0x004D94B0`.

**Active in YR: Conditional.** Evidence: `0x004B0C40`; reached by `0x004595C0` only under the reciprocal-link condition above, and by interrupt paths such as `UndockUnit`.

### 3.4 Set_Destination overwrites NavCom and calls active locomotor Head_To_Coord

The unit vtable `+0x480` resolves through `TechnoClass::Set_Destination @ 0x00741970` and ends in `FootClass::Set_Destination_Internal @ 0x004D94B0`. The internal setter clears `Foot+0x5A0`, writes `Foot+0x5A4 = destination`, gets the destination object's coordinate through vtable `+0x4C`, and calls active locomotor slot `+0x44` with that coordinate.

For DriveLocomotion, slot `+0x44` is `DriveLocomotionClass::Set_Destination @ 0x004AFD40`, which writes only Drive destination `+0x30/+0x34/+0x38`. It does not change Force_Track's track index `+0x54`, point index `+0x58`, or `head_to` fields `+0x3C/+0x40/+0x44`.

**Active in YR: Yes for all Foot destination assignments; Conditional for Drive slot effects when the active locomotor is Drive.** Evidence: `0x004D94B0`, `0x00741970`, `0x004AFD40`.

### 3.5 Stop paths clear different layers

`FootClass::Stop_Moving @ 0x004DF0D0` only clears `Foot+0x5A0` and `Foot+0x5A4`. `DriveLocomotionClass::Stop_Moving @ 0x004AFE00` clears Drive destination `+0x30/+0x34/+0x38` and clamps speed/residual, but does not clear the Drive `head_to` fields. In stock `Mission_Deploy_Building` state-4 exit, the code clears `+0x6D1`, tests active locomotor `Is_Moving` via slot `+0x10`, and if true calls unit vtable `+0x500` before continuing. Prior `UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md` labels that slot `ForceStop`.

**Active in YR: Yes.** Evidence: `0x004DF0D0`, `0x004AFE00`, `0x0073E223..0x0073E237`, `UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md` vtable table.

## 4. INI Keys

| Key | Stock value | Evidence | Effect in this slice | Active in YR |
|---|---|---|---|---|
| `[CMIN] Dock` | `NAREFN,GAREFN` | `ini/rulesmd.ini:7361` | allows standard refinery docking | Yes |
| `[CMIN] Harvester` | `yes` | `ini/rulesmd.ini:7364` | reaches harvester/refinery missions | Yes |
| `[CMIN] Teleporter` | `yes` | `ini/rulesmd.ini:7396` | primary locomotor is chrono, but dock logic can interact with Drive piggyback | Yes |
| `[CMIN] Locomotor` | teleport CLSID | `ini/rulesmd.ini:7398` | stock chrono miner identity | Yes |
| `[GAREFN] DockUnload` / `Refinery` | `yes` / `yes` | `ini/rulesmd.ini:11726`, `11727` | stock Allied refinery unload path | Yes |
| `[NAREFN] DockUnload` / `Refinery` | `yes` / `yes` | `ini/rulesmd.ini:12519`, `12520` | stock Soviet refinery unload path | Yes |
| `[GAREFN]/[NAREFN] Foundation` | `4x3` | `ini/artmd.ini:1766`, `1709` | confirms standard refinery geometry; not a Force_Track gate | Yes |

## 5. Reconciliation With Prior Reports

`CHRONO_MINER_FORCE_TRACK_0X47_REFINERY_EXIT_GHIDRA_REPORT.md` and `CHRONO_MINER_POST_UNLOAD_EXIT_ANCHOR_GHIDRA_REPORT.md` remain correct for the conditional `ReleaseDockedHarvester` body: track `0x47` is a Drive turn-track index, the forced-track target is center-derived, and the later passable-cell destination is a separate NavCom assignment.

They are wrong or overbroad where they claim that this is the normal stock CMIN post-unload exit. Fresh evidence and sibling reports show stock DockUnload does not set the reciprocal `+0x2E4` link required for `ReleaseDockedHarvester`; the standard zero-link path exits through `Mission_Deploy_Building` state 4.

**Active in YR: Yes for the corrected stock path; Conditional for the preserved `ReleaseDockedHarvester` path.** Evidence: `0x0073D630`, `0x004595C0`, `BUILDING_RECEIVE_RADIO_0X15_DOCKUNLOAD_HANDOFF_GHIDRA_REPORT.md`, `CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING_GHIDRA_REPORT.md`.

## 6. Integration Points

| Function / branch | Status | Evidence | Active in YR |
|---|---|---|---|
| `Mission_Deploy_Building` zero-link stock unload | verified | `0x0073D630` | Yes |
| `Mission_Deploy_Building` nonzero-link release branch | verified | call at `0x0073D66D` | Conditional |
| `ReleaseDockedHarvester` body | verified | `0x004595C0` | Conditional |
| `DriveLocomotion::Force_Track` local writes | verified | `0x004B0C40` | Conditional |
| `Set_Destination` NavCom handoff | verified | `0x00741970`, `0x004D94B0` | Yes |
| Drive destination setter | verified | `0x004AFD40` | Conditional |
| Foot and Drive stop split | verified | `0x004DF0D0`, `0x004AFE00` | Yes |

## 7. Current Rust Implementation Status

Not audited as an implementation slot. The only implementation-facing consequence is that Rust should not treat `Force_Track(0x47)` as proven for the stock CMIN/GAREFN/NAREFN unload-complete path. If it models `ReleaseDockedHarvester`, that path should be conditional on a nonzero reciprocal dock link or other verified caller context, not assumed for every stock refinery dump completion.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Stock zero-link `Mission_Deploy_Building` exit | verified | `0x0073D630` state 4 | none for this slice |
| Nonzero-link `ReleaseDockedHarvester` branch | verified | `0x0073D66D -> 0x004595C0` | exact non-refinery producers outside bunker/interrupt docs deferred |
| Force_Track target/destination writes | verified | `0x004B0C40` | no runtime capture |
| Foot/NavCom destination install | verified | `0x004D94B0`, `0x00741970` | none |
| Drive destination setter | verified | `0x004AFD40` | none |
| Stop/clear semantics | verified | `0x004DF0D0`, `0x004AFE00`, `0x0073E223..0x0073E237` | vtable `+0x500` exact implementation name not re-decompiled |
| Full path after next `Mission_Harvest` tick | touched-not-exhausted | `0x0073E5E0` decompile | broader harvest/ore search context |

## 9. Open Questions - Final State

[RESOLVED] OQ-1 - Is `Force_Track(0x47)` the normal stock CMIN post-unload exit? No. It is conditional on nonzero reciprocal `+0x2E4`; stock zero-link DockUnload exits through `Mission_Deploy_Building` state 4. Evidence: `0x0073D630`, sibling reports on `0x15` and dock-link writers.

[RESOLVED] OQ-2 - Does `Force_Track` set Foot/NavCom destination? No. It writes Drive-local head/destination fields; Foot/NavCom `+0x5A4` is written by `FootClass::Set_Destination_Internal`. Evidence: `0x004B0C40`, `0x004D94B0`.

[RESOLVED] OQ-3 - In the conditional release path, which command comes first: forced track or NavCom destination? Forced track first, then unit vtable `+0x480(dest,1)`, then mission `MOVE=2`. Evidence: `0x004595C0`.

[RESOLVED] OQ-4 - Does Drive `Set_Destination` clear the forced track index? No. `0x004AFD40` writes destination coord `+0x30/+0x34/+0x38`; it does not write `+0x54/+0x58`. Evidence: `0x004AFD40`.

[RESOLVED] OQ-5 - Does stock state-4 exit install a new passable destination? No. The branch clears `+0x6D1`, may stop if moving, and returns through mission scheduling; no `+0x480` call appears in that branch. Evidence: `0x0073E0F4..0x0073E289`.

[DEFERRED] OQ-6 - Exact first rendered frame if a conditional `ReleaseDockedHarvester` exit fires. Category: needs-runtime-debugger. Static Ghidra resolves command ordering and field writes, but not frame capture.

## Sources

- Ghidra decompiled/read-only: `0x0073D630`, `0x004595C0`, `0x004B0C40`, `0x004AFD40`, `0x004D94B0`, `0x00741970`, `0x004DF0D0`, `0x004AFE00`, `0x0073E5E0`.
- Ghidra xrefs/read-only: caller of `0x004595C0` is `UnitClass__Mission_Deploy_Building @ 0x0073D630`; callers of `0x004D94B0` include `TechnoClass__Set_Destination @ 0x00741970`.
- Existing reports reconciled: `miner/CHRONO_MINER_FORCE_TRACK_0X47_REFINERY_EXIT_GHIDRA_REPORT.md`, `miner/CHRONO_MINER_POST_UNLOAD_EXIT_ANCHOR_GHIDRA_REPORT.md`, `miner/BUILDING_RECEIVE_RADIO_0X15_DOCKUNLOAD_HANDOFF_GHIDRA_REPORT.md`, `miner/CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING_GHIDRA_REPORT.md`, `UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/artmd.ini`.
