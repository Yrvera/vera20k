# CMIN Force_Track 0x47 Exit First Visible Movement And Facing - Ghidra Research Report

**Address(es):** `0x0073D630`, `0x004595C0`, `0x004593A0`, `0x004B0C40`, `0x004B0F20`, `0x004B4780`, `0x0055A8F0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** standard YR Chrono Miner post-unload exit visuals/motion around `Force_Track(0x47)`: when it fires versus stock zero-link no-`Force_Track` exit, what `0x47` means, `Power_On`/speed restore side effects, and what Rust should require for the first visible post-dump movement.  
**Non-Scope:** drain timing, two-miner handoff, destroyed-refinery sequencing beyond using `UndockUnit` as contrast, full pathfinding/blocked-delay behavior, and runtime frame capture.  
**Confidence:** High for branch conditions, table data, call ordering, and static first-track-point behavior; Medium for exact first rendered pixel/frame because that requires runtime capture.  
**Active in YR:** Conditional. Standard stock `CMIN/HARV -> GAREFN/NAREFN` cargo-empty completion is active in YR but does **not** call `Force_Track(0x47)`. `Force_Track(0x47)` is active only for nonzero reciprocal-link release/interrupt contexts whose docked unit has active Drive locomotion.

## Working Notes

- **Target question:** Verify exact standard YR Chrono Miner post-unload exit `Force_Track(0x47)` conditions and first visible movement/facing behavior.
- **Non-goals:** Do not cover drain timing, two-miner handoff, destroyed-refinery behavior except as contrast, Rust edits, or broad locomotion/pathfinding research.
- **Evidence needed to mark COMPLETE:** decompile plus assembly/xref evidence for `Force_Track(0x47)` caller and no-call stock exit, track-table meaning of `0x47`, `Power_On`/speed restore side effects, Rust surface scan, and acceptance handoff.
- **Stop conditions:** every scoped `Force_Track`/stock-exit/first-visible-motion question is resolved or explicitly deferred, report written to the requested path, no Rust/doc files outside this path modified.

## 1. Overview

The player-visible correction is simple: a normal stock Chrono Miner that finishes unloading at a stock Allied/Soviet refinery does not get a special `Force_Track(0x47)` bib-step. It exits the zero-link `UnitClass::Mission_Deploy_Building` state-4 path, clears unload-active bookkeeping, returns to Harvest/Search scheduling, and does not seed a new exit destination or a direct facing/track prelude.

`Force_Track(0x47)` remains a real YR behavior, but only in conditional reciprocal-link contexts: `BuildingClass::ReleaseDockedHarvester` when `unit/building +0x2E4` is already nonzero, and `BuildingClass::UndockUnit` for sell/damage/temporal interrupt ejection. In those paths `0x47` is a DriveLocomotion TurnTrack index, not a body-facing value.

## 2. Class Layout / Key Offsets

| Field / item | Offset / address | Meaning in this slice | Active in YR |
|---|---:|---|---|
| Unit/Building `+0x2E4` | int index `[0xB9]` | reciprocal dock-link branch selector | Conditional; stock refinery completion keeps it zero |
| Unit `+0x674` | int index `[0x19D]` | active `ILocomotion*` used for `Power_On` and `Force_Track` | Yes |
| Unit byte `+0x6D1` | byte | unload-active / state-3 initialized byte cleared by stock state 4 | Yes |
| Drive loco `+0x54` | int | TurnTrack index written by `Force_Track` | Conditional |
| Drive loco `+0x58` | int | track point index/progress reset to `0` by `Force_Track` | Conditional |
| Drive loco `+0x3C/+0x40/+0x44` | coord | head-to coordinate written by `Force_Track` | Conditional |
| Drive loco `+0x30/+0x34/+0x38` | coord | destination coordinate written after accepted `Apply_Track_Delta` path | Conditional |
| Drive loco `+0x4C/+0x50` | double halves | residual reset and speed set to double `1.0` | Conditional |
| Drive TurnTrack table | `0x007E7B28` | 72 x 12-byte turn descriptors | Yes |
| Drive RawTrack table | `0x007E7A28` | 16 x 16-byte raw track descriptors | Yes |

## 3. Core Logic

### 3.1 Stock standard post-unload exit has no Force_Track

`UnitClass::Mission_Deploy_Building @ 0x0073D630` starts with:

```text
0x0073D63B  CMP [ESI+0x2E4], 0
0x0073D641  JZ  0x0073D6E6
...
0x0073D66D  CALL 0x004595C0   ; only nonzero +0x2E4 branch
```

The stock refinery path is the zero-link branch. Its state-4 branch starts at `0x0073E17F`, waits if the rediscovered refinery has `Refinery=yes` and `building+0x57C != 0`, then clears `unit+0x6D1`, sets mission `0x0A` (Harvest), optionally sends radio `3`, queues/advances the mission, and returns through the timer epilogue. The relevant assembly order is:

- `0x0073E1DF` checks `building+0x57C`; `0x0073E1EA` direct-returns while it is non-null.
- `0x0073E1F6` clears `byte [ESI+0x6D1]`.
- `0x0073E24F..0x0073E254` pushes `0`, pushes `0x0A`, and calls vtable `+0x1E8`.
- `0x0073E268..0x0073E279` calls `PathType__Has_Valid_Steps` and conditionally sends radio `3`.
- `0x0073E27F..0x0073E283` calls vtable `+0x1EC`.

There is no call to `ILocomotion+0x58`, no push of `0x47`, no vtable `+0x70` call, no speed-multiplier restore, and no new NavCom/destination in this stock state-4 block.

**Active in YR:** Yes. Evidence: `rulesmd.ini:[CMIN] Dock=NAREFN,GAREFN`, `Harvester=yes`, `Teleporter=yes`, `Locomotor` teleport; `[GAREFN]/[NAREFN] DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`; `STANDARD_REFINERY_0X2E4_WRITER_INVENTORY_GHIDRA_REPORT.md` verifies stock refinery docking does not write the reciprocal `+0x2E4` link.

### 3.2 Conditional ReleaseDockedHarvester path fires Force_Track

`BuildingClass::ReleaseDockedHarvester @ 0x004595C0` is called only at `0x0073D66D`, the nonzero `unit+0x2E4` branch. Inside it:

- reads `building+0x2E4`; null path only clears `building+0x718`, sets building mission `5`, and returns;
- calls the docked unit vtable `+0x2C` and proceeds only if it returns `1` (Drive locomotion);
- clears `unit+0x2E4` before locomotion commands;
- calls active locomotor vtable `+0x58` (`Power_On`);
- calls building vtable `+0x48` (`GetCoords`);
- applies `x -= 0x80`, `y += 0x80`;
- pushes literal `0x47` and calls active locomotor vtable `+0x70`;
- calls unit vtable `+0x544` with `(0, 0x3FF00000)` = double `1.0`;
- computes and installs a passable destination, sets mission `2`, clears building link, resets building mission, and sends radio `3`.

Assembly support: `0x004596DD` checks locomotion type `== 1`; `0x004596E6` clears `unit+0x2E4`; `0x00459709` calls vtable `+0x58`; `0x00459726` `SUB EBP,0x80`; `0x0045972C` `ADD EBX,0x80`; `0x00459751` pushes `0x47`; `0x00459760` calls vtable `+0x70`; `0x00459767` pushes `0x3FF00000`.

**Active in YR:** Conditional. The helper is live, but stock standard refinery completion normally does not reach it because stock DockUnload does not set the reciprocal `+0x2E4` link.

### 3.3 Interrupt UndockUnit path fires the same track without destination

`BuildingClass::UndockUnit @ 0x004593A0` has callers from `BuildingClass::Sell`, `BuildingClass::ReceiveDamage`, and `TemporalClass::Update`. It reads `building+0x2E4`, requires the docked unit's type query to return `1`, calls active locomotor `+0x58`, calls building `GetCoords`, issues `Force_Track(0x47, x-0x80, y+0x80, z)`, calls speed multiplier `1.0`, clears both `+0x2E4` links, and sends radio `3`.

Unlike `ReleaseDockedHarvester`, `UndockUnit` does not call `Find_Nearby_Passable_Cell`, does not call `Set_Destination`, and does not set unit mission `MOVE=2`.

**Active in YR:** Conditional. This is standard YR interrupt behavior only when a building has a nonzero docked-unit pointer and the unit active locomotor reports Drive.

### 3.4 Power_On is not Stop_Moving

`LocomotionClass::Power_On @ 0x0055A8F0` sets byte `this+0xC` to `1` and calls vtable `+0x60`. It does not clear destination/head-to fields and does not stop the locomotor. This matters because older wording called the slot "Stop"; in this slice it re-enables the locomotor before issuing the forced track.

**Active in YR:** Yes. DriveLocomotion vtable data includes `0x0055A8F0` at slot `+0x58`; both `ReleaseDockedHarvester` and `UndockUnit` call the active locomotor's `+0x58` before `+0x70`.

### 3.5 Force_Track semantics

`DriveLocomotionClass::Force_Track @ 0x004B0C40` writes:

- `this+0x54 = track_index`;
- `this+0x58 = 0`;
- head-to coordinate fields to the passed coordinate;
- on accepted non-null target, `Apply_Track_Delta(target, 1)`;
- destination coordinate fields to the same coordinate;
- `this+0x4C = 0`;
- `this+0x50 = 0x3FF00000` (double high half for `1.0`).

Assembly support: `0x004B0C53` writes `[EBP+0x54] = EAX`; `0x004B0C56` writes `[EBP+0x58] = 0`; `0x004B0D35..0x004B0D3A` calls `Apply_Track_Delta` with `1`; `0x004B0D4A..0x004B0D4F` writes destination coordinate; `0x004B0D52` clears residual; `0x004B0D59` writes `0x3FF00000`.

**Active in YR:** Conditional for this scenario, because the function is active DriveLocomotion behavior but only called by these exit paths when their conditions hold.

### 3.6 Track 0x47 means TurnTrack[71] -> RawTrack[15]

Read-only memory:

- `TurnTrack[71] @ 0x007E7E7C`: `0f 0f 00 00 c0 00 00 00 00 00 00 00`.
- Interpreted as normal raw track `15`, short raw track `15`, target facing `0xC0`, flags `0`.
- `RawTrack[15] @ 0x007E7B18`: pointer `0x007E7968`, chain `-1`, entry `0`, cell-cross `-1`.
- `Track15 @ 0x007E7968`: 16 points from `(128,-128,facing 0x80)` through `(16,-4,facing 0xBC)`.

`DriveLocomotionClass::Transform_Track_Coords @ 0x004B4780` applies track flags bits `1/2/4` to swap/negate coordinates and adjust facing. Because TurnTrack[71] flags are `0`, Track15 coordinates and facings are used untransformed.

`DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` reads the TurnTrack entry by `track_index * 12`, selects normal/short raw track, transforms the current raw point, moves the owner, then updates body facing using the track point's facing shifted into the 16-bit facing representation (`point_facing << 8` in the decompile around `0x004B1A77`).

**Active in YR:** Conditional for the miner exit; yes for DriveLocomotion table processing when `Force_Track(0x47)` is installed.

## 4. INI Keys

| INI key | Stock YR value | Effect in this slice | Active in YR |
|---|---|---|---|
| `rulesmd.ini:[CMIN] Dock` | `NAREFN,GAREFN` | standard refinery candidates | Yes |
| `rulesmd.ini:[CMIN] Harvester` | `yes` | enables harvester unload FSM | Yes |
| `rulesmd.ini:[CMIN] Speed` | `4` | unit full speed; conditional release restores multiplier `1.0` rather than changing this key | Yes |
| `rulesmd.ini:[CMIN] UnloadingClass` | `CMON` | unload display override, cleared by stock Rust handoff | Yes |
| `rulesmd.ini:[CMIN] Teleporter` | `yes` | chrono identity; not checked by stock state 4 or exit helper bodies | Yes |
| `rulesmd.ini:[CMIN] Locomotor` | teleport CLSID | active outside dock; conditional helper requires active Drive (`1`) at ejection time | Yes |
| `rulesmd.ini:[GAREFN] DockUnload/Refinery/NumberOfDocks` | `yes` / `yes` / `1` | live stock Allied refinery unload | Yes |
| `rulesmd.ini:[NAREFN] DockUnload/Refinery/NumberOfDocks` | `yes` / `yes` / `1` | live stock Soviet refinery unload | Yes |
| `artmd.ini:[GAREFN]/[NAREFN] Foundation` | `4x3` | coordinate context for `GetCoords` | Yes |
| `artmd.ini:[GAREFN]/[NAREFN] QueueingCell` | `4,1` | waiting/staging data; not the stock state-4 exit movement | Yes |

## 5. Integration Points

| Function / area | Role | Status |
|---|---|---|
| `UnitClass::Mission_Deploy_Building @ 0x0073D630` | stock unload FSM and nonzero-link branch | verified |
| `BuildingClass::ReleaseDockedHarvester @ 0x004595C0` | conditional reciprocal-link forced exit | verified |
| `BuildingClass::UndockUnit @ 0x004593A0` | sell/damage/temporal forced ejection | verified |
| `LocomotionClass::Power_On @ 0x0055A8F0` | re-enable locomotor before forced track | verified |
| `DriveLocomotionClass::Force_Track @ 0x004B0C40` | writes direct TurnTrack index and destination/speed state | verified |
| `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` | consumes track points and updates body facing from raw point facing | verified for this slice |
| `DriveLocomotionClass::Transform_Track_Coords @ 0x004B4780` | applies TurnTrack flags to coords/facing | verified |
| `STANDARD_REFINERY_0X2E4_WRITER_INVENTORY` | proves stock refinery path no reciprocal writer | referenced as prior verified inventory |

## 6. Current Rust Implementation Status

Rust already separates stock zero-link `Departing` from conditional forced ejection:

- `src/sim/miner/miner_dock_sequence.rs::phase_departing` releases dock/contact bookkeeping, clears display override, clears movement target, clears `drive_track` and `forced_drive_track`, clears `facing_target`, clears stale `exit_cell`, and returns to `SearchOre`.
- `src/sim/miner/miner_dock_sequence.rs::interrupt_refinery_docked_miners` seeds `begin_forced_turn_track(0x47, ...)` only for miners physically on the pad when a refinery is interrupted.
- `src/sim/movement/drive_track.rs` contains TurnTrack[71], RawTrack[15], Track15 points, and `begin_forced_turn_track`.
- Current tests include `stock_departing_does_not_start_force_track_0x47`, `stock_departing_hands_directly_to_search_without_exit_move`, `stock_departing_does_not_start_explicit_exit_move`, `sell_refinery_interrupts_docked_miner_with_force_track_0x47`, and `sell_refinery_cancels_contact_miner_without_force_track_0x47`.

Observed delta: stock path direction matches this report. Conditional forced-track support is structurally present; exact first rendered pixel/frame for the conditional curve still needs runtime capture or a focused Rust movement-step test if a deterministic approximation is accepted.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Required working notes | verified | report Working Notes section | none |
| Output path did not exist before writing | verified | `Test-Path` false | none |
| Stock state-4 no Force_Track path | verified | `0x0073D630`, assembly `0x0073D63B`, `0x0073E1F6`, `0x0073E24F`, `0x0073E275` | exact runtime first visible movement after Harvest scheduling |
| Nonzero reciprocal-link ReleaseDockedHarvester path | verified | xref `0x0073D66D`, decompile/assembly `0x004595C0` | frequency in non-stock maps/mods |
| Interrupt UndockUnit path | verified | xrefs from sell/damage/temporal; decompile `0x004593A0` | destroyed-refinery timing out of scope |
| `Power_On` semantics | verified | decompile `0x0055A8F0`, vtable xrefs | exact slot `+0x60` helper name not needed |
| `Force_Track` writes | verified | decompile/assembly `0x004B0C40` | crate-pickup early exit edge not relevant to stock no-call |
| TurnTrack[71] table bytes | verified | memory `0x007E7E7C` | none |
| RawTrack[15] metadata and points | verified | memory `0x007E7B18`, `0x007E7968` | exact rendered-frame capture |
| Track transform flags | verified | decompile `0x004B4780`, flags `0` | none |
| Facing update from track point | verified | decompile `0x004B0F20` | exact render interpolation timing |
| Rust stock surfaces | verified by scan | `miner_dock_sequence.rs`, `miner_tests.rs` | focused movement-step test for conditional curve |

## 8. Open Questions - Final State Of The Investigation Log

- `[RESOLVED] OQ-01 - What mode applies? -> exhaustive-slice for the bounded Force_Track 0x47 exit condition and first-track-point static behavior.` (evidence: scoped target and primary functions)
- `[RESOLVED] OQ-02 - Does the output report already exist? -> No.` (evidence: `Test-Path docs/research/miner/CMIN_FORCE_TRACK_0X47_EXIT_FIRST_VISIBLE_MOVEMENT_AND_FACING_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-03 - Does stock standard cargo-empty completion call Force_Track(0x47)? -> No; stock state 4 is on the zero-+0x2E4 branch and has no vtable +0x70 call or 0x47 push.` (evidence: `0x0073D630`, `0x0073E17F..0x0073E2BE`)
- `[RESOLVED] OQ-04 - When does ReleaseDockedHarvester call Force_Track? -> Only after nonzero reciprocal link path, docked unit exists, and unit type query returns Drive=1.` (evidence: `0x0073D66D`, `0x004596DD`, `0x00459751..0x00459760`)
- `[RESOLVED] OQ-05 - Does UndockUnit call Force_Track for interrupts? -> Yes, under nonzero building+0x2E4 and Drive=1; no normal stock caller.` (evidence: `0x004593A0` xrefs and decompile)
- `[RESOLVED] OQ-06 - Is Power_On a stop or movement clear? -> No; it sets powered byte and calls slot +0x60.` (evidence: `0x0055A8F0`)
- `[RESOLVED] OQ-07 - Does speed restore happen inside Force_Track and/or unit vtable call? -> Both conditional helpers call unit vtable +0x544 with double 1.0 after Force_Track; Force_Track itself also writes drive speed high half 0x3FF00000 on the accepted path.` (evidence: `0x00459767`, `0x004B0D52..0x004B0D59`)
- `[RESOLVED] OQ-08 - What is 0x47? -> TurnTrack index 71, not body facing.` (evidence: `0x00459751`, memory `0x007E7E7C`)
- `[RESOLVED] OQ-09 - Which raw track does it use? -> RawTrack[15] for both normal and short forms.` (evidence: TurnTrack[71] bytes `0f 0f ...`; RawTrack[15] at `0x007E7B18`)
- `[RESOLVED] OQ-10 - What are the first conditional track coordinates/facing? -> Track15 point 0 is `(128,-128,0x80)`, flags 0, so first processed point is untransformed.` (evidence: `0x007E7968`, `0x004B4780`)
- `[RESOLVED] OQ-11 - Does 0x47 write a facing target? -> No direct unit facing or facing-target write; facing changes later from track points in `Process_Drive_Track`.` (evidence: `0x004595C0`, `0x004593A0`, `0x004B0F20`)
- `[RESOLVED] OQ-12 - What should stock Rust first-visible post-dump behavior require? -> no forced track, no forced facing, no queue-cell exit movement; hand off to SearchOre/Harvest scheduling at the pad.` (evidence: stock state-4 branch and Rust scan)
- `[RESOLVED] OQ-13 - What should conditional Rust first-track behavior require? -> forced state stores TurnTrack 0x47 / RawTrack 15 and first processed point/facing comes from Track15, not 0x47.` (evidence: table reads and Rust `begin_forced_turn_track`)
- `[RESOLVED] OQ-14 - Are QueueingCell and stock post-dump exit the same? -> No; stock state 4 does not install a QueueingCell exit move.` (evidence: `0x0073E17F..0x0073E2BE`; art `QueueingCell` only context)
- `[DEFERRED] OQ-15 - Exact first rendered pixel/frame after conditional Force_Track install.` (category: `needs-runtime-debugger`; reason: static Ghidra proves command and track-point data but not render-frame sampling; next-step-if-pursued: retail runtime capture/watch position, facing, track index, point index across the first frame after interrupt/release)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Standard stock CMIN cargo-empty completion does not call `Force_Track(0x47)` | `0x0073D63B`, `0x0073D641`, state-4 `0x0073E17F..0x0073E2BE`; writer inventory | none observed | `src/sim/miner/miner_dock_sequence.rs::phase_departing` | stock handoff clears unload/contact state and returns to SearchOre/Harvest scheduling with no forced track or explicit exit move | `chrono_miner_stock_state4_first_visible_post_dump_has_no_force_track` | Do not reintroduce release-helper track/destination for normal stock completion |
| Conditional reciprocal-link/interrupt ejection calls `Power_On`, `Force_Track(0x47, center.x-0x80, center.y+0x80, z)`, then speed multiplier `1.0` | `0x004595C0`, `0x004593A0`, assembly `0x00459709`, `0x00459751`, `0x00459767`; xrefs | partially implemented for interrupt; reciprocal release not a stock path | `interrupt_refinery_docked_miners`, future reciprocal-link helper if implemented | only physically docked/on-pad nonzero-link contexts get forced track; track setup must precede any ordinary movement | `sell_refinery_on_pad_chrono_miner_begins_force_track_0x47_raw15` | Do not apply forced track to contact-only miners or stock zero-link Departing |
| `0x47` is TurnTrack[71] -> RawTrack[15], flags 0, first raw point `(128,-128,0x80)`, final target facing `0xC0` | memory `0x007E7E7C`, `0x007E7B18`, `0x007E7968`; `0x004B4780`, `0x004B0F20` | structural support present; exact movement-step test should be added if not already covered | `src/sim/movement/drive_track.rs`, movement tick surfaces | forced curve should update body facing from Track15 points, not set facing/target to `0x47` | `force_track_0x47_first_advance_uses_track15_point0_facing_0x80` | Do not treat literal `0x47` as a DirStruct/body-facing value |

### Concrete Rust Test-Name Proposals

- `chrono_miner_stock_state4_first_visible_post_dump_has_no_force_track`
- `chrono_miner_stock_state4_does_not_write_facing_target_0x47`
- `chrono_miner_stock_state4_keeps_pad_position_until_search_scheduling_moves`
- `sell_refinery_on_pad_chrono_miner_begins_force_track_0x47_raw15`
- `force_track_0x47_first_advance_uses_track15_point0_facing_0x80`

## 10. Negative Facts / Do Not Do

- Do not model normal stock `CMIN/HARV -> GAREFN/NAREFN` cargo-empty completion as `ReleaseDockedHarvester`.
- Do not call `BuildingClass::UndockUnit` for healthy stock post-unload exit.
- Do not seed `Force_Track(0x47)`, body facing `0x47`, `facing_target=0x47`, or a queue-cell exit move on stock state 4.
- Do not describe `Power_On` slot `+0x58` as `Stop_Moving`; it re-enables the locomotor and does not clear destination/head-to fields.
- Do not collapse `QueueingCell=4,1` into a stock post-dump exit destination; stock state 4 installs no such destination.
- Do not give contact-only waiting miners an interrupt forced track; the binary guard requires a docked unit pointer and active Drive locomotion.

## 11. Stale Docs / Follow-up Docs

- `docs/research/miner/CHRONO_MINER_FORCE_TRACK_0X47_REFINERY_EXIT_GHIDRA_REPORT.md`: replace "standard `[CMIN]` reaches the normal exit as a harvester unloading at `[GAREFN]`/`[NAREFN]`" with "standard stock `[CMIN] -> [GAREFN]/[NAREFN]` cargo-empty completion exits through zero-link `UnitClass::Mission_Deploy_Building` state 4 and does not call `Force_Track(0x47)`; `Force_Track(0x47)` is conditional on nonzero reciprocal-link release or interrupt ejection."
- `docs/research/miner/BUILDING_UNDOCKUNIT_0x4593A0_CHRONO_MINER_GHIDRA_REPORT.md`: replace "ILocomotion::Stop (vtable+0x58)" with "ILocomotion::Power_On (vtable+0x58), which sets the powered byte and calls slot `+0x60`; it is not Stop_Moving."
- `docs/research/miner/CHRONO_MINER_POST_UNLOAD_EXIT_ANCHOR_GHIDRA_REPORT.md`: replace "the forced-track prelude starts from the docked/pad position" in any stock-normal framing with "the forced-track prelude starts only in conditional reciprocal-link/interrupt contexts; stock zero-link state 4 has no forced-track prelude."

## Remaining Uncertainty

- Exact first rendered pixel/frame after a conditional `Force_Track(0x47)` install needs retail runtime capture; static evidence gives the first raw point and facing but not render sampling.
- Exact behavior if `DriveLocomotionClass::Force_Track` target is rejected by the crate/target validation side path was not pursued because it is not reached by the stock no-call path and is not expected for the refinery-center offset.

## Sources

- Ghidra decompiled/read-only: `0x0073D630`, `0x004595C0`, `0x004593A0`, `0x004B0C40`, `0x004B0F20`, `0x004B4780`, `0x0055A8F0`.
- Ghidra xrefs/read-only: `0x004595C0` from `0x0073D66D`; `0x004593A0` from `BuildingClass::Sell`, `BuildingClass::ReceiveDamage`, `TemporalClass::Update`; `0x004B0C40` vtable data at `0x007E7F20`; `0x0055A8F0` DriveLocomotion vtable slot data.
- Ghidra assembly/read-only: `0x0073D63B`, `0x0073D641`, `0x0073D66D`, `0x0073E1F6`, `0x0073E24F`, `0x0073E275`, `0x004596DD`, `0x00459709`, `0x00459726`, `0x0045972C`, `0x00459751`, `0x00459760`, `0x00459767`, `0x004B0C53`, `0x004B0C56`, `0x004B0D35`, `0x004B0D52`, `0x004B0D59`.
- Ghidra memory/read-only: `0x007E7E7C` TurnTrack[71], `0x007E7B18` RawTrack[15], `0x007E7968` Track15 points.
- Prior docs referenced: `STANDARD_REFINERY_0X2E4_WRITER_INVENTORY_GHIDRA_REPORT.md`, `STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_PATHTYPE_STATE4_GHIDRA_REPORT.md`, `HARV_POST_UNLOAD_EXIT_PATH_GHIDRA_REPORT.md`, `BUILDING_UNDOCKUNIT_0x4593A0_CHRONO_MINER_GHIDRA_REPORT.md`, `RELEASEDOCKEDHARVESTER_0x4595C0_GHIDRA_REPORT.md`, `TWO_MINER_ONE_REFINERY_ZERO_LINK_HANDOFF_TIMING_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/artmd.ini`.
- Rust scanned: `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_tests.rs`, `src/sim/movement/drive_track.rs`, `src/sim/components.rs`.

**Status:** PARTIAL for the full visual target because the runtime first rendered pixel/frame after a conditional `Force_Track(0x47)` install remains explicitly deferred. COMPLETE for static branch conditions, track meaning, and Rust acceptance boundaries.
