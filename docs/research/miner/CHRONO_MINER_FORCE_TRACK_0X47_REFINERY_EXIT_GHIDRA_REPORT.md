# Chrono Miner Force_Track 0x47 Refinery Exit — Ghidra Research Report

**Address(es):** `0x004595C0`, `0x004593A0`, `0x004B0C40`, `0x004B0F20`, `0x004B4780`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** DriveLocomotion/BuildingClass refinery-exit forced track `0x47` for chrono miner post-unload bib step and interrupt exits.  
**Non-Scope:** inbound docking track choice, full Mission_Harvest economy, full TeleportLocomotion warp sequence, runtime visual side-by-side capture.  
**Confidence:** High for binary constants, call paths, track table data, and normal/interrupt activity split; Medium for exact player-visible first-frame impression because that needs runtime capture.  
**Active in YR:** Conditional for nonzero reciprocal-link release / sell / destruction / temporal interrupt exits when a docked unit exists and its active locomotor type query returns Drive (`1`). Superseded for normal stock CMIN/HARV DockUnload by the 2026-05-21 zero-link state-4 findings.

> **Correction 2026-05-21 - stock-path reachability**
>
> `CHRONO_MINER_FORCE_TRACK_0X47_EXIT_NAVCOM_STEP_GHIDRA_REPORT.md` supersedes
> this report's "normal CMIN post-unload" reachability wording. The function body
> facts below remain useful for the conditional `ReleaseDockedHarvester` /
> `UndockUnit` paths, but stock zero-link `CMIN/HARV -> GAREFN/NAREFN`
> DockUnload completion does **not** call `ReleaseDockedHarvester`, does **not**
> issue `Force_Track(0x47)`, and does **not** install a new NavCom destination.
> The stock path exits through `UnitClass::Mission_Deploy_Building` state 4,
> clears the dock-active byte `+0x6D1`, may stop the locomotor if it is still
> moving, and returns through normal harvest/mission scheduling. Treat all
> `Force_Track(0x47)` findings in this document as conditional on a nonzero
> reciprocal `+0x2E4` release/interrupt context, not as standard stock refinery
> unload completion.

## 1. Overview

Superseding stock-path verdict: normal stock `CMIN/HARV -> GAREFN/NAREFN`
post-unload completion does not use `BuildingClass::UndockUnit` or
`BuildingClass::ReleaseDockedHarvester`; it exits through the zero-link
`UnitClass::Mission_Deploy_Building` state-4 path. `ReleaseDockedHarvester`
still calls the active locomotor's vtable slot `+0x70` with hardcoded track
index `0x47` and building-center lepton offsets `x - 0x80`, `y + 0x80` when a
conditional nonzero reciprocal-link release reaches it.

`0x47` is a DriveLocomotion TurnTrack index, not a direct unit facing write. The unit's body facing is updated later by `DriveLocomotionClass::Process_Drive_Track` from RawTrack point facing values, via `FacingClass__UpdateFacing`, while the forced track is processed.

## 2. Class Layout / Key Offsets

| Object | Offset | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| BuildingClass / unit | `+0x2E4` (`[0xB9]` in int* decompile) | mutual dock-link pointer | `0x004595C0`, `0x004593A0` clear/read it | Yes |
| UnitClass | `+0x674` (`[0x19D]`) | active `ILocomotion*` | both building exit funcs assert non-null before vtable calls | Yes |
| DriveLoco ILocomotion view | `+0x54` | TurnTrack index written by `Force_Track` | `0x004B0C40` write | Yes |
| DriveLoco ILocomotion view | `+0x58` | track point index / raw-track progress reset to `0` | `0x004B0C40` write | Yes |
| DriveLoco ILocomotion view | `+0x3C/+0x40/+0x44` | head-to coordinate triplet | `0x004B0C40` writes target coord | Yes |
| DriveLoco ILocomotion view | `+0x30/+0x34/+0x38` | destination coordinate triplet | `0x004B0C40` writes after accepted track | Yes |
| DriveLoco ILocomotion view | `+0x4C/+0x50` | double speed cap/current speed = `1.0` | `0x004B0C40` writes low `0`, high `0x3FF00000` | Yes |

## 3. Core Logic

### Conditional reciprocal-link release (`0x004595C0`)

Verified sequence:

1. Clears building anim slots `0xA` and `0xB`, plays `RulesClass+0x244` (`BunkerWallsDownSound`) if configured, and creates slots `0xC`/`0xD` before locomotion.
2. Reads `building+0x2E4`; if null, clears `building+0x718`, sets building mission `5`, and returns.
3. Calls docked unit vtable `+0x2C`; only proceeds if return is `1` (DriveLocomotion active).
4. Clears the unit-side dock link (`unit+0x2E4`) before locomotion commands.
5. Calls active locomotor vtable `+0x58`, then building vtable `+0x48` to get building coords.
6. Calls active locomotor vtable `+0x70` as `Force_Track/Head_To(track=0x47, x-0x80, y+0x80, z)`.
7. Calls unit vtable `+0x544` with `(0, 0x3FF00000)`, i.e. double `1.0`.
8. Computes a passable-cell destination from `building.GetCell + (-1,+1)`, calls unit vtable `+0x480(dest,1)`, then unit `SetMission(2)`.
9. Clears building-side dock link and `+0x718`, sets building mission `5`, and sends `RadioCommand(3)`.

**Active in YR:** Yes. Evidence: sole xref to `0x004595C0` is `UnitClass__Mission_Deploy_Building` at `0x0073D66D`; `[CMIN]` has `Harvester=yes`, `Dock=NAREFN,GAREFN`, `UnloadingClass=CMON`, and teleport locomotor in `ini/rulesmd.ini:7351-7398`; `[GAREFN]` has `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`, `FreeUnit=CMIN` at `ini/rulesmd.ini:11722-11736`.

### Interrupt/sell/destruction exit (`0x004593A0`)

`BuildingClass::UndockUnit` reads `building+0x2E4`, requires unit vtable `+0x2C == 1`, calls active locomotor `+0x58`, then calls active locomotor `+0x70` with the same `0x47`, `x-0x80`, `y+0x80`, `z` arguments and sets speed `1.0`. It clears both dock links and sends `RadioCommand(3)`, but it does not compute a passable-cell destination and does not call `SetMission(MOVE)`.

**Active in YR:** Conditional. Evidence: xrefs to `0x004593A0` are `BuildingClass__Sell` (`0x0044AAB0`), `TemporalClass__Update` (`0x0071AA15`), and `BuildingClass__ReceiveDamage` (`0x004424EA`). These are live YR systems, but only fire this path if a docked unit pointer exists and the unit active locomotor reports type `1`.

### Drive forced-track semantics (`0x004B0C40`)

`DriveLocomotionClass::Force_Track` writes the supplied track index, resets progress to `0`, installs the head-to coordinate, marks `is_on_track=1`, validates the target cell, calls `Apply_Track_Delta(target,1)`, sets destination equal to the target, and sets speed to double `1.0`.

**Active in YR:** Yes. Evidence: `0x004595C0` and `0x004593A0` indirect-call ILocomotion slot `+0x70`, and DriveLocomotion vtable slot `+0x70` resolves to `0x004B0C40` (`0x007E7F20` data xref).

### Track 0x47 / 71 shape

Binary table reads:

| Data | Address | Value | Meaning | Active in YR |
|---|---:|---|---|---|
| TurnTrack[71] | `0x007E7E7C` | `0f 0f 00 00 c0 00 00 00 00 00 00 00` | normal raw track `15`, short raw track `15`, target facing `0xC0`, flags `0` | Yes |
| RawTrack[15] | `0x007E7B18` | pointer `0x007E7968`, chain `-1`, entry `0`, cell-cross `-1` | special undock curve, no chain/cell-cross metadata | Yes |
| Track15 points | `0x007E7968` | 16 points of `{x,y,facing}` | starts `(128,-128,0x80)`, ends `(16,-4,0xBC)` | Yes |

Track 15 therefore begins with a south-facing subcell offset `(128,-128)`, bends through facings `0x84,0x88,...`, and ends near west (`0xBC`, with TurnTrack target `0xC0`). There is no transform flip (`flags=0`) and no raw-track cell-cross marker. This is a curved sub-cell departure shape, not a one-tick facing snap.

**Active in YR:** Yes. Evidence: both normal and interrupt building-exit paths force TurnTrack index `0x47` for DriveLocomotion, and standard `[CMIN]` reaches the normal exit as a harvester unloading at `[GAREFN]`/`[NAREFN]`.

### Facing update during the forced track

`Process_Drive_Track` (`0x004B0F20`) reads the TurnTrack entry from `g_DriveTrackIndex_Table + track_index * 12`, chooses normal vs short raw track, reads raw point `{x,y,facing}`, transforms it through `DriveLocomotionClass::Transform_Track_Coords` (`0x004B4780`), moves the owner, and calls `FacingClass__UpdateFacing` with the track point's facing shifted into 16-bit facing space (`point_facing << 8` in the decompile). `Transform_Track_Coords` only changes facing when TurnTrack flags bits `1/2/4` are set; for TurnTrack[71], flags are `0`, so the raw Track15 facing sequence is used unchanged.

**Active in YR:** Yes. Evidence: decompile of `0x004B0F20` and binary TurnTrack[71] flags at `0x007E7E84 == 0`.

## 4. INI Keys

| Key | Value | Source | Effect | Active in YR |
|---|---|---|---|---|
| `[CMIN] Dock` | `NAREFN,GAREFN` | `ini/rulesmd.ini:7361` | permits docking at standard refineries | Yes |
| `[CMIN] Harvester` | `yes` | `ini/rulesmd.ini:7364` | uses harvester/refinery mission path | Yes |
| `[CMIN] UnloadingClass` | `CMON` | `ini/rulesmd.ini:7384` | visual no-back model during unload | Yes |
| `[CMIN] Locomotor` | TeleportLocomotion CLSID | `ini/rulesmd.ini:7398` | primary chrono locomotor; Drive can be active via piggyback during dock | Yes |
| `[GAREFN] DockUnload` | `yes` | `ini/rulesmd.ini:11726` | refinery unload cycle | Yes |
| `[GAREFN] Refinery` | `yes` | `ini/rulesmd.ini:11727` | refinery classification | Yes |
| `[GAREFN] FreeUnit` | `CMIN` | `ini/rulesmd.ini:11736` | Allied refinery grants Chrono Miner | Yes |
| `[AudioVisual] BunkerWallsDownSound` | `TankBunkerDown` | `ini/rulesmd.ini:720` | conditional nonzero-link release sound before `Force_Track`; not normal stock zero-link exit | Yes |

## 5. Integration Points

| Function / area | Status | Evidence | Active in YR |
|---|---|---|---|
| `UnitClass::Mission_Deploy_Building` nonzero reciprocal-link release branch | verified | xref `0x0073D66D -> 0x004595C0` | Conditional; not stock zero-link DockUnload completion |
| `BuildingClass::ReleaseDockedHarvester` | verified | decompile `0x004595C0` | Yes |
| `BuildingClass::UndockUnit` | verified | decompile `0x004593A0`, xrefs from sell/damage/temporal | Conditional |
| `DriveLocomotionClass::Force_Track` | verified | decompile `0x004B0C40`, vtable xref `0x007E7F20` | Yes |
| `DriveLocomotionClass::Process_Drive_Track` | verified for track read/facing update | decompile `0x004B0F20` | Yes |
| `Transform_Track_Coords` flags | verified | decompile `0x004B4780`, TurnTrack[71] flags `0` | Yes |

## 6. Current Rust Implementation Status

Rust has extracted Drive track data, including TurnTrack[71], RawTrack[15], and Track15 point data in `src/sim/movement/drive_track.rs`. Current stock `Departing` follows the superseding zero-link model: no normal `ReleaseDockedHarvester`, no normal `Force_Track(0x47)`, and no explicit exit destination. Conditional interruption support exists through forced-turn-track helpers, but reciprocal-link release parity remains a separate conditional branch. Do not infer a stock post-unload `Departing` delta from this report.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Conditional reciprocal-link CMIN release path | verified | `0x0073D66D` nonzero-`+0x2E4` branch xref, `0x004595C0`, INI lines | not the stock zero-link DockUnload completion |
| Interrupt sell/destruction/temporal exit | verified | `0x004593A0` xrefs from `0x0044AAB0`, `0x004424EA`, `0x0071AA15` | exact temporal target preconditions beyond docked BuildingClass are out-of-scope |
| Force_Track slot behavior | verified | `0x004B0C40` | none for this slice |
| Track 71 table entry and point shape | verified | memory `0x007E7E7C`, `0x007E7B18`, `0x007E7968` | runtime frame capture of exact pixel impression |
| Facing update from track point | verified | `0x004B0F20`, `0x004B4780` | none for this slice |
| Full obstruction behavior while exiting | touched-not-exhausted | `0x004B0F20` has large Can_Enter/crush/scatter branches | belongs to slot 3 / blocked-delay target |

## 8. Open Questions - Final State

[RESOLVED] OQ1 - Is `0x47` a facing byte or Drive TurnTrack index? Answer: TurnTrack index 71; no direct `unit->Facing = 0x47` write in either building exit function. Evidence: `0x004595C0`, `0x004593A0`, `0x004B0C40`, TurnTrack memory `0x007E7E7C`.

[RESOLVED] OQ2 - Are `-0x80,+0x80` INI-driven? Answer: no, hardcoded lepton offsets applied to `building.GetCoords()`. Evidence: decompile `0x004595C0`, `0x004593A0`.

[RESOLVED / SUPERSEDED 2026-05-21] OQ3 - Is the normal CMIN exit path `UndockUnit`? Answer: no. Newer branch evidence further refines this: normal stock zero-link DockUnload is also not `ReleaseDockedHarvester`; it is `Mission_Deploy_Building` state 4. `ReleaseDockedHarvester` remains conditional on a nonzero reciprocal `+0x2E4` link.

[RESOLVED] OQ4 - Does Track 71 have a cell-cross marker? Answer: no; RawTrack[15] cell-cross field is `-1`. Evidence: memory `0x007E7B18`.

[RESOLVED] OQ5 - Does Track 71 transform or mirror raw facing? Answer: no; TurnTrack[71] flags are `0`, so Track15 raw facing values are used unchanged. Evidence: memory `0x007E7E7C`, `0x004B4780`.

[DEFERRED] OQ6 - Exact first visible pixel/facing frame in retail capture. Reason: requires runtime side-by-side frame capture, not static Ghidra. Category: needs-runtime-debugger.

## Sources

- Ghidra decompiled: `0x004595C0`, `0x004593A0`, `0x004B0C40`, `0x004B0F20`, `0x004B4780`.
- Ghidra xrefs: `0x004595C0`, `0x004593A0`, `0x004B0C40`.
- Ghidra memory: `0x007E7E7C` (TurnTrack[71]), `0x007E7B18` (RawTrack[15]), `0x007E7968` (Track15 points).
- INI checked: `ini/rulesmd.ini`, `ini/artmd.ini`.
- Prior docs checked: `RELEASEDOCKEDHARVESTER_0x4595C0_GHIDRA_REPORT.md`, `BUILDING_UNDOCKUNIT_0x4593A0_CHRONO_MINER_GHIDRA_REPORT.md`, `DRIVE_LOCOMOTION_CLASS.md`, `DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md`, `CHRONO_MINER_SYSTEM_OVERVIEW.md`, `units/allied/CMIN.md`.
