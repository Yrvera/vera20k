# DriveLocomotion Head-To / Clear Navigation State - Ghidra Research Report

**Address(es):** `0x004AFCC0`, `0x004AFD40`, `0x004AFE00`, plus `0x004AFB80`, `0x004AFC20`, `0x004AFC90`, `0x004B0500`, `0x004B0F20`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** DriveLocomotion destination/head-to/null-coordinate/bridge-Z fields and the clear-navigation/stop state needed before Phase 1 NavCom lifecycle.
**Non-Scope:** full `Process_Movement`, exact speed-budget parity, full NavQueue semantics, building/dock special cases, chrono piggyback lifecycle, convoy TypeClass field naming.
**Confidence:** High for field writes and bridge-Z behavior; Medium for caller naming because several vtable slots use misleading historical names.
**Active in YR:** Yes.
**Status:** COMPLETE for the requested field/state slice; broader DriveLocomotion lifecycle remains out of scope.

## Target Question

Which concrete DriveLocomotion fields are written by the head-to/destination and clear-navigation paths, and which fields does Rust need before implementing Phase 1 NavCom ownership?

## Non-Goals

- Do not design or implement Rust.
- Do not claim full `DriveLocomotionClass::Process_Movement` parity.
- Do not decode all owner guard vtable calls in `0x004AFD40`.
- Do not decode the `TypeClass+0xC94` convoy gate name.

## Evidence Needed To Mark COMPLETE

- `0x004AFCC0` decompiled and distinguished from setter-style `0x004AFD40`.
- `0x004AFD40` decompiled with exact destination fields, guard behavior, null handling, and bridge-Z adjustment.
- `0x004AFE00` decompiled with exact clear-navigation writes.
- `0x004AFB80` / `0x004AFC20` checked only enough to know when clearing head-to/destination changes moving state.
- `0x004B0500` / `0x004B0F20` checked for process-side head-to clearing and arrival interactions.

All items above were satisfied by Ghidra read-only decompilation.

## Stop Conditions

Stop when the requested Drive fields and clear-navigation implications are proven from binary. Do not continue into full speed formulas, pathfinding result dispatch, or building/dock target handling unless needed to decide the field writes.

## 1. Overview

DriveLocomotion has two separate coordinate triplets: a public/ILocomotion destination and an active head-to/intermediate coordinate. The setter-style locomotor call used by `FootClass::Set_Destination_Internal` writes only the destination triplet; it does not install a track head-to. Clear-navigation for Drive is the Drive `Stop_Moving` slot, and it clears only the destination triplet, not the head-to triplet or active track fields.

The naming is dangerous: `0x004AFCC0` is a getter returning the current head-to coordinate, while `0x004AFD40` is historically documented as `Set_Destination` but is the concrete vtable target called by Foot's "Head_To_Coord" contract.

## 2. Class Layout / Key Offsets

Offsets below are absolute from the DriveLocomotion object base. ILocomotion-vtable methods receive `this = object_base + 4`, so their decompiled field offsets are 4 bytes lower.

| Absolute offset | Vtable-this offset | Type | Purpose | Evidence |
|---|---:|---|---|---|
| `+0x34/+0x38/+0x3C` | `+0x30/+0x34/+0x38` | `Coord3D` | destination triplet | constructor `0x004AF540`, getter `0x004AFC90`, setter `0x004AFD40`, stop `0x004AFE00` |
| `+0x40/+0x44/+0x48` | `+0x3C/+0x40/+0x44` | `Coord3D` | head-to/intermediate triplet | constructor `0x004AF540`, getter `0x004AFCC0`, moving checks `0x004AFB80`/`0x004AFC20`, track/process clears `0x004B0F20` |
| `+0x4C` | internal | `int` | residual/budget field for track stepping | constructor `0x004AF540`, process-track `0x004B0F20` |
| `+0x50` | `+0x4C` | `double` | current speed fraction | stop clamp `0x004AFE00`, force-track `0x004B0C40`, process-track `0x004B0F20` |
| `+0x58` | internal / `+0x54` in vtable-this funcs | `int` | active track index, `-1` means no active track | constructor `0x004AF540`, process-track `0x004B0F20`, force-track `0x004B0C40` |
| `+0x5C` | internal / `+0x58` in vtable-this funcs | `int` | track point index | constructor `0x004AF540`, process-track `0x004B0F20`, force-track `0x004B0C40` |
| `+0x63` | `+0x5F` | `byte` | head-to/track-valid flag; set when head-to is installed, cleared when head-to is nulled | constructor `0x004AF540`, force-track `0x004B0C40`, process-track `0x004B0F20` |

Null coordinate is exactly `(0,0,0)`, initialized by `0x004AF4E0`.

## 3. Core Logic

### `0x004AFD40` - setter-style destination / Foot "Head_To_Coord" target

Verified behavior:

- Calls four owner guard vtable methods in order: owner vtable `+0x37C`, `+0x380`, `+0x1D4`, `+0x1D8`.
- If any guard returns nonzero, it returns without writing destination fields.
- If all guards return zero, writes destination X/Y/Z to Drive absolute `+0x34/+0x38/+0x3C`.
- If the input coordinate is not `(0,0,0)`, calls `CellClass::Get_Cell_At`.
- If `CellClass+0x140 & 0x100` is set, adds `g_BridgeZOffset_Drive` to destination Z.
- It does not write head-to `+0x40/+0x44/+0x48`.
- It does not write track index, point index, residual, or `+0x63`.

Bridge-Z proof:

- `0x004AF4A0` initializes `g_BridgeZOffset_Drive = ftol(g_DriveHeightStep * 4)`.
- `0x004AFD40` adds that value only after the non-null coordinate test and bridge-cell flag test.
- Null destination `(0,0,0)` does not call `CellClass::Get_Cell_At` and does not receive bridge-Z adjustment.

### `0x004AFCC0` - head-to getter

Verified behavior:

- If head-to `+0x40/+0x44/+0x48` is null, returns owner position from `owner+0x9C/+0xA0/+0xA4`.
- Otherwise returns head-to `+0x40/+0x44/+0x48`.
- It writes nothing.

This is not the setter used by `FootClass::Set_Destination_Internal`; it is a readback slot.

### `0x004AFC90` - destination getter

Returns destination `+0x34/+0x38/+0x3C` directly. It does not fall back to owner position.

### `0x004AFE00` - Drive clear-navigation / Stop_Moving

Verified behavior:

- Checks head-to only as a convoy-propagation gate.
- If head-to is non-null, owner type `+0xC94` is nonzero, owner `+0x6D0` is zero, and owner `+0x6C8` follower exists, it walks the follower chain and calls each follower locomotor vtable `+0x48`.
- Sets Drive current speed to `min(current_speed, 0.3)`. In decompiled vtable-this offsets this is `this+0x4C`; absolute object offset is `+0x50`.
- Clears destination `+0x34/+0x38/+0x3C` to null.
- It does not clear head-to `+0x40/+0x44/+0x48`.
- It does not clear track index `+0x58`, point index `+0x5C`, residual `+0x4C`, or valid flag `+0x63`.
- It does not write owner `NavCom`; owner `FootClass::Stop_Moving` at `0x004DF0D0` is the separate function that clears `Foot+0x5A0` and `Foot+0x5A4`.

### Moving-state dependency

`0x004AFB80` (`ILocomotion_Is_Moving`) decides:

- destination non-null -> moving.
- destination null and head-to null -> not moving.
- destination null and head-to XY equals owner XY -> not moving; Z is ignored in this equality.
- destination null and head-to XY differs from owner XY -> moving.

`0x004AFC20` (`Is_Moving_Now`) decides:

- CDTimer remaining -> moving now.
- Otherwise requires active locomotion slot result nonzero, non-null head-to, and owner speed query vtable `+0x538` greater than zero.

Implementation implication: clearing destination alone is not enough to guarantee not-moving if head-to still differs in XY. Clearing head-to alone is not enough if destination remains non-null.

### Process-side head-to clearing

`0x004B0500` (`DriveLocomotionClass::Process`) and `0x004B0F20` (`Process_Drive_Track`) are the places that actually clear the head-to triplet during normal movement/arrival paths. In the end-of-track zero-delta branch, `0x004B0F20` clears head-to, clears `+0x63`, sets track index `+0x58` to `-1`, and point index `+0x5C` to `0`. In accepted-arrival checks it can also clear destination and then head-to when the owner is at the NavCom cell with compatible Z.

## 4. INI Keys

No INI key directly controls `0x004AFCC0`, `0x004AFD40`, or the null coordinate. The convoy-propagation gate reads owner type byte `+0xC94`, but this slice did not prove its INI key name. A text search found no literal `IsTrain=` key in `ini/rules.ini` or `ini/rulesmd.ini`; prior docs that name this field as `IsTrain` should be treated as outside this report's proof.

| Key / field | Status | Evidence | Effect in this slice |
|---|---|---|---|
| `CellClass+0x140 bit 0x100` | verified binary field | `0x004AFD40`, `0x004B0F20` | bridge-cell flag; gates bridge-Z destination add |
| `TypeClass+0xC94` | verified binary field, name unverified | `0x004AFE00` | convoy stop propagation gate |

## 5. Integration Points

| Integration point | Behavior | Evidence |
|---|---|---|
| `FootClass::Set_Destination_Internal` `0x004D94B0` | writes `Foot+0x5A4 = target`; for non-null target, calls target vtable `+0x4C` to get coord, then active locomotor vtable `+0x44` with that coord | Ghidra `0x004D94B0` |
| `FootClass::Set_Destination_Internal` null branch | when null and no attack carve-out, calls active locomotor vtable `+0x48`, then reasserts `NavCom = target` | Ghidra `0x004D94B0` |
| Drive vtable `+0x44` | data xref to `0x004AFD40`; concrete Drive destination setter | xref `0x007E7EF4` |
| Drive vtable `+0x48` | data xref to `0x004AFE00`; concrete Drive clear-navigation/stop | xref `0x007E7EF8` |
| Drive process `0x004B0500` | arrival with empty queue calls owner vtable `+0x480` as `Set_Destination(NULL,1)`; non-empty queue calls `FootClass::Stop_Moving` then owner vtable `+0x484` | Ghidra `0x004B0500` |
| Drive process `0x004B0500` | when no track/path and NavCom exists, gets fresh target coord via NavCom vtable `+0x4C` and calls Drive vtable `+0x44` if needed | Ghidra `0x004B0500` |

## 6. Current Rust Implementation Status

Rust currently has `MovementTarget` as the main destination/path owner and `DriveTrackState` as a track curve adapter.

| Rust surface | Current state | Delta |
|---|---|---|
| `src/sim/components.rs` `MovementTarget` | owns `path`, `next_index`, speed, final goal, and blocked timers | no separate NavCom target pointer or Drive destination/head-to triplets |
| `src/sim/movement/locomotor.rs` `LocomotorState` | stores active kind, phase, speed fractions, bridge layer, terrain type, etc. | no Drive destination, head-to, track-valid flag, or Drive current-speed double-equivalent |
| `src/sim/game_entity.rs` | stores `movement_target`, `locomotor`, `drive_track`, `forced_drive_track` | Drive state is split across generic target and track state; no owner-style NavCom slot |
| `src/sim/movement/drive_track.rs` `DriveTrackState` | has raw track, point index, residual, head offsets, target facing | models track stepping but not Drive destination/head-to ownership |
| `src/sim/movement/movement_commands.rs` | move command attaches `MovementTarget` and now starts initial DriveTrack for Drive locomotors | still uses `MovementTarget` as destination owner; no `Head_To_Coord` destination write adapter |
| `src/sim/movement/movement_tick.rs` `finalize_finished_entities` | clears `movement_target` and `drive_track` on finish | does not model `Set_Destination(NULL,1)` -> Drive `Stop_Moving` -> later process-side head-to clear |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x004AFCC0` head-to getter | verified | Ghidra decompile | none for this slice |
| `0x004AFD40` destination setter / Foot head-to target | verified | Ghidra decompile, xref `0x007E7EF4` | owner guard function identities not decoded |
| Bridge-Z destination adjustment | verified | `0x004AFD40`, `0x004AF4A0` | exact map bridge record logic outside setter not covered |
| Null coordinate value | verified | `0x004AF4E0`, constructor `0x004AF540` | none |
| `0x004AFE00` Drive Stop_Moving / clear-navigation | verified | Ghidra decompile, xref `0x007E7EF8` | convoy type field name deferred |
| `0x004AFB80` moving predicate | verified for destination/head-to dependency | Ghidra decompile | full caller audit out of scope |
| `0x004AFC20` moving-now predicate | touched-not-exhausted | Ghidra decompile | CDTimer owner identity and speed query slot naming out of scope |
| `0x004B0500` process arrival/re-aim integration | touched-not-exhausted | Ghidra decompile | full tick order and all branches out of scope |
| `0x004B0F20` process-track head-to clearing | touched-not-exhausted | Ghidra decompile | full track stepping and collision dispatch out of scope |
| Rust destination ownership | verified enough for handoff | `components.rs`, `locomotor.rs`, `game_entity.rs`, `movement_commands.rs`, `movement_tick.rs`, `drive_track.rs` | implementation design not covered |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ1 - Is 0x004AFCC0 the setter called by FootClass Head_To_Coord? -> No. It is a getter returning head-to or owner position.` (evidence: `0x004AFCC0`, `0x004D94B0`)
- `[RESOLVED] OQ2 - Which function writes Drive destination for normal Set_Destination_Internal? -> Drive vtable +0x44 resolves to 0x004AFD40 and writes destination only.` (evidence: `0x004AFD40`, xref `0x007E7EF4`)
- `[RESOLVED] OQ3 - Does 0x004AFD40 write head-to? -> No, it only writes destination + bridge-adjusted Z.` (evidence: `0x004AFD40`)
- `[RESOLVED] OQ4 - What is Drive NullCoord? -> `(0,0,0)`.` (evidence: `0x004AF4E0`, `0x004AF540`)
- `[RESOLVED] OQ5 - Does bridge-Z apply to null destination? -> No; null skips the cell lookup and bridge flag branch.` (evidence: `0x004AFD40`)
- `[RESOLVED] OQ6 - What bridge-Z value is added? -> `ftol(g_DriveHeightStep * 4)`.` (evidence: `0x004AF4A0`, `0x004AFD40`)
- `[RESOLVED] OQ7 - What does Drive Clear_Navigation/Stop_Moving clear? -> destination triplet only, after speed clamp and optional convoy propagation.` (evidence: `0x004AFE00`)
- `[RESOLVED] OQ8 - Does Drive Stop_Moving clear head-to? -> No.` (evidence: `0x004AFE00`)
- `[RESOLVED] OQ9 - Does Drive Stop_Moving clear track index or point index? -> No.` (evidence: `0x004AFE00`; contrast `0x004B0F20`)
- `[RESOLVED] OQ10 - When can destination null still be moving? -> when head-to XY differs from owner XY.` (evidence: `0x004AFB80`)
- `[RESOLVED] OQ11 - Does moving predicate compare head-to Z against owner Z? -> No, the equality early-out compares X/Y only.` (evidence: `0x004AFB80`)
- `[RESOLVED] OQ12 - Where is owner NavCom cleared? -> `FootClass::Stop_Moving` clears `Foot+0x5A0/+0x5A4`; Drive Stop_Moving does not.` (evidence: `0x004DF0D0`, `0x004AFE00`)
- `[RESOLVED] OQ13 - Who calls Drive clear-navigation on null target? -> `FootClass::Set_Destination_Internal` null branch calls active locomotor vtable +0x48.` (evidence: `0x004D94B0`)
- `[RESOLVED] OQ14 - Where does normal process re-aim destination from NavCom? -> `0x004B0500` calls NavCom vtable +0x4C then Drive vtable +0x44.` (evidence: `0x004B0500`)
- `[RESOLVED] OQ15 - Which Rust state currently owns this? -> `MovementTarget` owns destination/path; `DriveTrackState` owns track curve; no Drive destination/head-to fields exist.` (evidence: `src/sim/components.rs`, `src/sim/movement/drive_track.rs`, `src/sim/movement/locomotor.rs`, `src/sim/game_entity.rs`)
- `[DEFERRED] OQ16 - What is the exact INI name for TypeClass+0xC94?` (category: `requires-different-system-context`; reason: not needed to identify Drive fields; next-step-if-pursued: TypeClass field audit around `+0xC94` readers and INI loader)
- `[DEFERRED] OQ17 - What are the four owner guard vtable calls in 0x004AFD40?` (category: `requires-different-system-context`; reason: Phase 1 state fields do not require guard-name decode; next-step-if-pursued: caller/owner vtable audit for slots `+0x37C/+0x380/+0x1D4/+0x1D8`)
- `[DEFERRED] OQ18 - What exact CDTimer does 0x004AFC20 test?` (category: `bounded-cost-too-high`; reason: only moving-state dependency was needed here; next-step-if-pursued: timer field audit of DriveLocomotion base and process calls)
- `[DEFERRED] OQ19 - Full `0x004B0F20` process-track branch coverage.` (category: `out-of-scope`; reason: speed/collision parity is a separate DriveTrack investigation; next-step-if-pursued: exhaustive-slice on `Process_Drive_Track` tick budget and arrival branches)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Drive has separate destination and head-to triplets. | `0x004AF540`, `0x004AFC90`, `0x004AFCC0` | missing | `src/sim/movement/locomotor.rs`, `src/sim/game_entity.rs` | add state capable of representing destination and head-to independently for Drive | Drive unit can have destination null while head-to remains non-null and still report moving if XY differs | do not collapse both into `MovementTarget.final_goal` |
| Foot non-null Set_Destination writes NavCom then calls locomotor vtable +0x44; Drive concrete writes destination only. | `0x004D94B0`, `0x004AFD40` | mismatch/partial | `src/sim/movement/movement_commands.rs` | move command should model owner NavCom plus Drive destination setter before path/track stepping | normal Drive move-to-cell creates NavCom target and Drive destination even before a track head-to exists | do not treat first `DriveTrackState` creation as the same as `Head_To_Coord` |
| Bridge target cells add `g_BridgeZOffset_Drive` to destination Z. | `0x004AFD40`, `0x004AF4A0` | unchecked | movement command / bridge adapter surfaces | destination Z must be bridge-adjusted when target cell has `CellClass+0x140 & 0x100` | move to bridge-deck cell stores destination Z four height levels above ground | do not derive this from current `on_bridge`; setter assumes bridge at destination |
| Clear-navigation Drive slot clears destination only and clamps speed to max 0.3. | `0x004AFE00` | missing | future Drive state clear path; `movement_tick.rs` arrival clearing | null-target path should clear Drive destination but leave head-to/track state to process-side logic | issuing stop/null destination while head-to differs leaves state consistent with `Is_Moving` until process clears or arrives | do not clear head-to just because `Set_Destination(NULL)` was called |
| Owner NavCom clear is `FootClass::Stop_Moving`, not Drive Stop_Moving. | `0x004DF0D0`, `0x004D94B0`, `0x004AFE00` | missing | owner entity destination state to be added | separate owner NavCom clear from locomotor state clear | empty-queue arrival routes through owner `Set_Destination(NULL,1)` and clears owner NavCom | do not store NavCom solely inside Drive locomotor state |
| Moving predicate first checks destination, then head-to; head-to arrival ignores Z. | `0x004AFB80` | missing | movement completion / Drive state predicates | implement Drive moving predicate from destination/head-to state, not only `movement_target.is_some()` | destination null + head-to same XY as owner reports not moving even if Z differs | do not use full XYZ equality for this predicate |
| Process-track clears head-to, `+0x63`, track index, and point index on end-of-track/arrival branches. | `0x004B0F20` | partial | `src/sim/movement/movement_step.rs`, `src/sim/movement/drive_track.rs` | DriveTrack completion must clear head-to/valid flag and track fields separately from destination clear | finishing a track leg can clear head-to while destination/NavCom state persists until arrival logic | do not make `DriveTrackState=None` imply destination is null |

## Stale Docs / Follow-up Docs

- `DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md` says `0x004AFE00` convoy logic is active for units with `Accelerates=true`. Replacement wording: "Convoy propagation in Drive `Stop_Moving` is gated by owner type byte `+0xC94`, owner `+0x6D0 == 0`, non-null head-to, and a non-null owner `+0x6C8` follower chain. This report did not verify the INI key name for `+0xC94`."
- Any doc that implies Drive `Stop_Moving` clears head-to should be corrected to: "Drive `Stop_Moving` clears destination only; process/track code clears head-to."
- Any doc that says `0x004AFCC0` is the setter called by `FootClass::Set_Destination_Internal` should be corrected to: "`0x004AFCC0` is the head-to getter; Drive vtable `+0x44` points at `0x004AFD40`, the setter-style destination writer."

## Sources

- Ghidra decompiled: `0x004AF4A0`, `0x004AF4E0`, `0x004AF500`, `0x004AF540`, `0x004AFB80`, `0x004AFC20`, `0x004AFC90`, `0x004AFCC0`, `0x004AFD40`, `0x004AFE00`, `0x004B0500`, `0x004B0C40`, `0x004B0F20`, `0x004D94B0`, `0x004DF0D0`.
- Ghidra xrefs: `0x004AFD40` from `0x007E7EF4`; `0x004AFE00` from `0x007E7EF8`; `0x004D94B0` callers/xrefs.
- Prior docs used as navigation only: `docs/research/DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md`, `docs/research/NAVCOM_LIFECYCLE_GHIDRA_REPORT.md`, `docs/research/DRIVELOCOMOTOR_ACCEPTED_CELL_ARRIVAL_VISIBILITY_GHIDRA_REPORT.md`, `docs/research/DRIVE_LOCOMOTION_CLASS.md`.
- Rust scanned: `src/sim/components.rs`, `src/sim/movement/locomotor.rs`, `src/sim/game_entity.rs`, `src/sim/movement/drive_track.rs`, `src/sim/movement/movement_commands.rs`, `src/sim/movement/movement_tick.rs`, `src/sim/movement/movement_step.rs`.
