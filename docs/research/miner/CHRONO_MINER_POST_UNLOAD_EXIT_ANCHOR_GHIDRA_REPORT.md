# Chrono Miner Post-Unload Exit Anchor - Ghidra Research Report

**Address(es):** `0x004595C0`, `0x004593A0`, `0x00447AC0`, `0x0041BEA0`, `0x0056DC20`, `0x004B0C40`, `0x0073E5E0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** post-unload refinery exit anchor coordinates for conditional CMIN reciprocal-link release and interrupt release: `ReleaseDockedHarvester`, `UndockUnit`, `Force_Track(0x47, building.x-0x80, building.y+0x80, z)`, passable-cell destination in that conditional release path, and how those coordinates relate to refinery origin, center, pad, queue cell, and first commanded exit motion. Superseded for normal stock zero-link DockUnload by 2026-05-21 reports.  
**Non-Scope:** inbound return/dock selection, accepted refinery radio anchor, dock arrival link timing, full `Find_Nearby_Passable_Cell` global behavior, runtime video capture.  
**Confidence:** High for binary formulas, call ordering, and active-YR reachability; Medium for exact first rendered frame because that requires runtime capture.  
**Active in YR:** Conditional for nonzero reciprocal-link release and interrupt release when a docked unit exists and its active locomotor type query returns Drive (`1`). Superseded for normal stock zero-link CMIN/HARV refinery completion.

> **Correction 2026-05-21 - stock-path reachability**
>
> `CHRONO_MINER_FORCE_TRACK_0X47_EXIT_NAVCOM_STEP_GHIDRA_REPORT.md` supersedes
> the "normal CMIN refinery release" reachability claim in this report. The
> coordinate formulas below are still correct for `ReleaseDockedHarvester` and
> `UndockUnit` when those functions are reached. They are **not** the normal
> stock zero-link DockUnload completion for `CMIN/HARV -> GAREFN/NAREFN`.
> Stock DockUnload does not establish the reciprocal `+0x2E4` link required for
> `ReleaseDockedHarvester`; it exits through `Mission_Deploy_Building` state 4
> without `Force_Track(0x47)`, without the west-origin passable destination, and
> without a new NavCom destination. Read this document as a conditional
> reciprocal-link/interrupt release coordinate reference, not as stock
> CMIN-refinery post-dump movement.

## 1. Overview

Superseding stock-path verdict: normal stock `CMIN/HARV -> GAREFN/NAREFN`
post-unload completion uses the zero-link `UnitClass::Mission_Deploy_Building`
state-4 path, not `BuildingClass::ReleaseDockedHarvester`. The formulas below
describe `ReleaseDockedHarvester` when a nonzero reciprocal-link context reaches
it; they are not the standard stock refinery DockUnload exit.

Interrupt release uses `BuildingClass::UndockUnit` at `0x004593A0`. It issues the same `Force_Track(0x47, center.x-0x80, center.y+0x80, z)` and speed reset, but it does not compute the normal passable destination and does not call unit `SetMission(MOVE)`.

## 2. Key Offsets and Coordinate Terms

| Item | Offset / formula | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| Building location | `+0x9C/+0xA0/+0xA4` | NW/origin lepton coordinate used by `Get_Cell_Packed` and `GetCoords` | `0x0041BEA0`, `0x00447AC0` | Yes |
| Building type | `+0x520` | type pointer used by `GetCoords` to read foundation width/height helpers | `0x00447AC0` | Yes |
| Dock link | `+0x2E4` | building-side docked unit pointer and unit-side docked building pointer | `0x004595C0`, `0x004593A0` | Yes |
| Active locomotor | unit `+0x674` | ILocomotion pointer used for `+0x58` and `+0x70` calls | `0x004595C0`, `0x004593A0` | Yes |
| Building center | `Location + width*0x80-0x80`, `Location + height*0x80-0x80` | lepton center from `BuildingClass::GetCoords` | `0x00447AC0` | Yes |
| Forced-track target | `center.x-0x80`, `center.y+0x80`, `center.z` | hardcoded target for track index `0x47` | `0x00459726`, `0x0045972C`, `0x00459401`, `0x00459407` | Yes |
| Normal passable anchor | `Get_Cell_Packed().x-1`, `.y+1` | normal release search origin before `Find_Nearby_Passable_Cell` | `0x0045977E` to `0x0045978D` | Yes |
| Art queue cell | `QueueingCell=4,1` | designer queue/waiting cell, not the normal release passable anchor | `ini/artmd.ini:1716`, `ini/artmd.ini:1773` | Yes |

For a 4x3 GAREFN/NAREFN placed at NW cell `(rx,ry)`:

| Coordinate | Formula | Example at `(10,10)` | Evidence |
|---|---:|---:|---|
| Origin / NW cell | `(rx,ry)` | `(10,10)` | `ObjectClass::Get_Cell_Packed` at `0x0041BEA0`; art foundation `4x3` |
| Center leptons | `(rx*256 + 384, ry*256 + 256)` | `(2944,2816)` | `0x00447AC0`: width/height scaled by `0x80`, minus `0x80` |
| Force-track target leptons | `(center.x-128, center.y+128)` | `(2816,2944)` = cell-centered `(11,11)` by integer cell conversion | `0x004595C0`, `0x004593A0` |
| Normal passable anchor | `(rx-1, ry+1)` | `(9,11)` | `0x0045978B` DEC x, `0x0045978D` INC y |
| Art queue cell | `(rx+4, ry+1)` | `(14,11)` | `QueueingCell=4,1` in artmd |
| Default refinery pad in Rust | `(rx+3, ry+1)` | `(13,11)` | current Rust fallback; binary pad path is outside this slot |

## 3. Core Logic

### Normal exit: `ReleaseDockedHarvester` (`0x004595C0`)

Verified sequence relevant to this slice:

1. Reads `building+0x2E4`; null path only clears `building+0x718`, sets building mission `5`, and returns.
2. Calls the docked unit vtable `+0x2C`; only proceeds when the active locomotion type reports `1` (Drive).
3. Clears the unit-side dock link at `unit+0x2E4` before locomotion commands.
4. Calls active locomotor vtable `+0x58`.
5. Calls building vtable `+0x48` -> `BuildingClass::GetCoords` (`0x00447AC0`).
6. Applies hardcoded `SUB x,0x80`, `ADD y,0x80`, then calls active locomotor vtable `+0x70` with pushed track index `0x47`.
7. Calls unit vtable `+0x544` with `(0,0x3FF00000)`, i.e. double `1.0`.
8. Calls building vtable `+0x1B8` -> `ObjectClass::Get_Cell_Packed` (`0x0041BEA0`), then decrements packed x and increments packed y.
9. Calls `FootClass::Find_Nearby_Passable_Cell` (`0x0056DC20`) from that `(origin.x-1, origin.y+1)` anchor.
10. Converts the returned packed cell to a `CellClass*`, calls unit vtable `+0x480` with `(dest,1)`, then unit vtable `+0x1E8` with `(2,0)`.
11. Clears building-side dock link and `+0x718`, sets building mission `5`, and sends radio/clear command `3`.

**Active in YR: Yes.** Evidence: sole xref is `UnitClass::Mission_Deploy_Building @ 0x0073D66D`; `[CMIN]` has `Harvester=yes`, `Dock=NAREFN,GAREFN`, `Teleporter=yes`, and teleport locomotor in `ini/rulesmd.ini:7351-7398`; `[GAREFN]` and `[NAREFN]` are live refineries with `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1` in `ini/rulesmd.ini:11722-11736` and `12515-12530`.

### Interrupt exit: `UndockUnit` (`0x004593A0`)

Verified sequence relevant to this slice:

1. Reads `building+0x2E4`; null returns immediately.
2. Requires docked unit vtable `+0x2C == 1`.
3. Calls active locomotor vtable `+0x58`.
4. Calls building vtable `+0x48`, applies the same `x-0x80`, `y+0x80`, and calls active locomotor vtable `+0x70` with track index `0x47`.
5. Sets speed multiplier to double `1.0`.
6. Clears both dock-link fields and sends command `3`.
7. Does not call `Get_Cell_Packed`, `Find_Nearby_Passable_Cell`, unit `Set_Destination`, or unit `SetMission(MOVE)`.

**Active in YR: Conditional.** Evidence: direct callers are `BuildingClass::Sell @ 0x0044AAB0`, `BuildingClass::ReceiveDamage @ 0x004424EA`, and `TemporalClass::Update @ 0x0071AA15`. These systems are live, but this exit occurs only when the building has a docked unit and the unit reports Drive locomotion type `1`.

### `GetCoords` and `Get_Cell_Packed`

`BuildingClass::GetCoords` (`0x00447AC0`) uses the building type foundation width and height. It computes:

```text
center.x = Location.x + width  * 0x80 - 0x80
center.y = Location.y + height * 0x80 - 0x80
center.z = Location.z
```

`ObjectClass::Get_Cell_Packed` (`0x0041BEA0`) converts `Location.x/y` to packed cell shorts using signed divide-by-256 semantics. It does not add foundation width/height. Therefore the normal passable anchor is tied to the building origin/NW cell, while the forced-track target is tied to the foundation center.

**Active in YR: Yes.** Evidence: BuildingClass vtable entries read as `0x00447AC0` at `0x007E3F04` and `0x0041BEA0` at `0x007E4074`; both are called from `0x004595C0`.

### `Force_Track` semantics

DriveLocomotion vtable slot `+0x70` resolves to `DriveLocomotionClass::Force_Track` at `0x004B0C40` (`0x007E7F20` contains `40 0C 4B 00`). It writes the track index at locomotor `+0x54`, resets point index `+0x58` to `0`, writes the head-to coordinate at `+0x3C/+0x40/+0x44`, and on the accepted path writes the same coordinate to destination fields `+0x30/+0x34/+0x38`, resets residual `+0x4C`, and sets speed high word `+0x50 = 0x3FF00000`.

**Active in YR: Yes.** Evidence: both exit functions call active locomotor vtable `+0x70` after requiring type `1`; DriveLocomotion vtable slot `+0x70` points to `0x004B0C40`.

## 4. INI Keys

| Key | Value | Source | Effect in this slice | Active in YR |
|---|---|---|---|---|
| `[CMIN] Dock` | `NAREFN,GAREFN` | `ini/rulesmd.ini:7361` | allows CMIN to use standard refineries | Yes |
| `[CMIN] Harvester` | `yes` | `ini/rulesmd.ini:7364` | reaches harvester/refinery unload path | Yes |
| `[CMIN] Teleporter` | `yes` | `ini/rulesmd.ini:7396` | CMIN uses teleport locomotor outside dock; dock exit path still requires Drive active | Yes |
| `[CMIN] Locomotor` | teleport CLSID | `ini/rulesmd.ini:7398` | chrono miner identity; no special branch in exit functions | Yes |
| `[GAREFN] DockUnload` / `Refinery` / `NumberOfDocks` | `yes` / `yes` / `1` | `ini/rulesmd.ini:11726-11729` | live Allied refinery unload path | Yes |
| `[NAREFN] DockUnload` / `Refinery` / `NumberOfDocks` | `yes` / `yes` / `1` | `ini/rulesmd.ini:12519-12521` | live Soviet refinery unload path | Yes |
| `[GAREFN] Foundation` | `4x3` | `ini/artmd.ini:1766` | width/height used by `GetCoords` | Yes |
| `[NAREFN] Foundation` | `4x3` | `ini/artmd.ini:1709` | width/height used by `GetCoords` | Yes |
| `[GAREFN]/[NAREFN] QueueingCell` | `4,1` | `ini/artmd.ini:1773`, `1716` | queue/waiting cell; not read by `ReleaseDockedHarvester` normal passable anchor | Yes |

## 5. Integration Points and First Exit Motion

Normal release is reached through `UnitClass::Mission_Deploy_Building -> ReleaseDockedHarvester`. Static call order shows the first outbound movement command in the exit function is the forced track to the center-derived coordinate, before the normal passable destination is searched. The later `Set_Destination(dest,1)` and `SetMission(MOVE=2)` install an ordinary movement target after the forced-track setup.

`UnitClass::Mission_Harvest` (`0x0073E5E0`) case 0 can overwrite or clear destinations on the next harvest-state pass. In the CMIN/TeleportLoco branch, it compares the active locomotor CLSID to the teleport CLSID and, if the dock-link field at `+0x5A4` is non-null, calls unit vtable `+0x480` with null destination at `0x0073E83E`; it then calls ore-search movement (`0x004DCF E0` path) at `0x0073E864`. This supports the prior-doc conclusion that the west-of-foundation normal-passable destination is not the durable player-visible goal after the miner resumes harvesting.

The coordinate relationship is therefore:

- The forced-track prelude starts from the docked/pad position and is aimed at a point derived from the refinery center, not the art queue cell.
- The normal passable destination is searched from one cell west of the NW origin, not from `QueueingCell=4,1`.
- The art queue cell `(rx+4,ry+1)` is still the visually relevant cell beside the pad for a standard 4x3 refinery because it is outside the east edge of the footprint and adjacent to the pad; the binary path evidence for that visible impression is indirect/static here, not a frame capture.
- Interrupt exits only get the forced-track prelude and immediate dock-link clear; no normal passable destination is installed by `UndockUnit`.

**Active in YR: Yes for normal release, Conditional for interrupt release.** Evidence: call xrefs above and standard INI content.

## 6. Current Rust Implementation Status

Rust has an explicit post-unload `Departing` phase and caches the exit cell once on first entry, matching the binary's one-shot destination computation pattern. Current code in `src/sim/miner/miner_dock_sequence.rs`:

- `refinery_queue_cell` uses art `QueueingCell` when available, falling back to `(rx+width, ry+height/2)`.
- `refinery_exit_cell` deliberately uses the queue cell / nearby passable fallback for observable exit movement, not gamemd's internal `(origin.x-1, origin.y+1)` anchor.
- `phase_departing` starts a `REFINERY_EXIT_FORCE_TRACK = 0x47` prelude, then issues ordinary movement to the cached exit cell after the forced track finishes.

This is a deliberate observable-behavior approximation: the code does not preserve the internal west-origin passable destination from `ReleaseDockedHarvester`, because prior static evidence shows the durable visible target is retargeted by harvest logic.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Normal `ReleaseDockedHarvester` coordinate sequence | verified | `0x004595C0` decompile/disassembly, especially `0x00459726-0x0045978D` | none for static slice |
| Interrupt `UndockUnit` coordinate sequence | verified | `0x004593A0` decompile/disassembly, especially `0x00459401-0x0045942C` | none for static slice |
| Building center formula | verified | `0x00447AC0`, vtable data `0x007E3F04 -> 0x00447AC0` | none |
| Building origin packed-cell formula | verified | `0x0041BEA0`, vtable data `0x007E4074 -> 0x0041BEA0` | none |
| Drive `Force_Track` slot binding and writes | verified | `0x004B0C40`, vtable data `0x007E7F20 -> 0x004B0C40` | none |
| `Find_Nearby_Passable_Cell` selection internals | touched-not-exhausted | `0x0056DC20`, call from `0x004597E3` | full global passability ranking is outside this slot |
| Mission_Harvest destination overwrite/retarget | touched-not-exhausted | `0x0073E5E0`, calls at `0x0073E83E` and `0x0073E864` | exact first rendered frame requires runtime capture |
| TS legacy gating | verified for this slice | standard YR INI + live xrefs from normal/interrupt callers | none for exit functions |

## 8. Open Questions - Final State

[RESOLVED / SUPERSEDED 2026-05-21] OQ1 - Does normal CMIN post-unload use `UndockUnit`? Answer: no. Newer branch evidence further refines this: normal stock zero-link DockUnload is also not `ReleaseDockedHarvester`; it is `Mission_Deploy_Building` state 4. `ReleaseDockedHarvester` remains conditional on a nonzero reciprocal `+0x2E4` link.

[RESOLVED] OQ2 - Are `x-0x80,y+0x80` relative to origin, center, pad, or queue cell? Answer: relative to `BuildingClass::GetCoords` center, then offset by hardcoded half-cell deltas. Evidence: `0x00447AC0`, `0x00459726/0x0045972C`, `0x00459401/0x00459407`.

[RESOLVED] OQ3 - Does normal release use art `QueueingCell=4,1` for its passable-cell anchor? Answer: no. It calls `Get_Cell_Packed` and uses `(origin.x-1, origin.y+1)`. Evidence: `0x0045977E-0x0045978D`; art queue exists at `ini/artmd.ini:1716,1773` but is not read here.

[RESOLVED] OQ4 - Does interrupt release install a passable-cell fallback destination? Answer: no. It lacks `Get_Cell_Packed`, `Find_Nearby_Passable_Cell`, `Set_Destination`, and unit `SetMission(MOVE)` calls. Evidence: full `0x004593A0` disassembly.

[RESOLVED] OQ5 - Is the exit path active for stock YR CMIN? Answer: normal path yes, interrupt path conditional. Evidence: standard CMIN/refinery INI keys plus call xrefs.

[DEFERRED] OQ6 - Which exact pixel/facing appears on the first rendered frame after release? Reason: static Ghidra proves command ordering and coordinate writes, but exact frame visibility needs retail runtime capture. Category: needs-runtime-debugger.

## Sources

- Ghidra decompiled/disassembled: `0x004595C0`, `0x004593A0`, `0x00447AC0`, `0x0041BEA0`, `0x004B0C40`, `0x0056DC20`, `0x0073E5E0`.
- Ghidra xrefs: `0x004595C0`, `0x004593A0`, `0x0056DC20`.
- Ghidra memory: `0x007E3F04`, `0x007E4074`, `0x007E7F20`.
- INI checked: `ini/rulesmd.ini`, `ini/artmd.ini`, with base `ini/rules.ini` / `ini/art.ini` checked for corresponding fallback refinery entries.
- Prior docs checked: `CHRONO_MINER_FORCE_TRACK_0X47_REFINERY_EXIT_GHIDRA_REPORT.md`, `BUILDING_UNDOCKUNIT_0x4593A0_CHRONO_MINER_GHIDRA_REPORT.md`, `RELEASEDOCKEDHARVESTER_0x4595C0_GHIDRA_REPORT.md`, `REFINERY_DOCK_EXIT_CHAIN_VERIFIED_GHIDRA_REPORT.md`.

**Status: COMPLETE**
