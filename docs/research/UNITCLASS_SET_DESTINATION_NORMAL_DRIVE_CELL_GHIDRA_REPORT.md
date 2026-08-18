# UnitClass Set_Destination Normal Drive Cell - Ghidra Research Report

**Address(es):** `0x00741970`, `0x004D94B0`, `0x004AFD40`, `0x004DF0D0`, `0x0065AE30`, `0x0065AD30`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** active standard YR `UnitClass` vtable `+0x480` destination assignment for a normal Drive-locomotor vehicle receiving a non-null empty-cell destination, through the tail call into `FootClass::Set_Destination_Internal` and the Drive locomotor `Head_To_Coord` write.
**Non-Scope:** building/dock/refinery/transport/repair/reload destinations, chrono-miner piggyback swaps, hover-to-drive swaps, aircraft/garrison/infantry paths, full player-command UI dispatch, `NavQueue` producers, and arrival clearing after the cell is reached.
**Confidence:** High for the verified fall-through, guard predicates, field writes, and Rust deltas in this narrow slice; Medium for exact human-readable names of several owner virtual predicates because the decompile exposes vtable offsets, not stable semantic names.
**Active in YR:** Yes. `TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md` verifies `UnitClass` vtable `0x007F5C70 + 0x480 = 0x00741970`; standard YR Drive vehicles use Drive CLSID `{4A582741-9839-11d1-B709-00A024DDAFD1}` from `rulesmd.ini`.

## Working Notes

**Target question:** For a normal Drive-locomotor vehicle ordered to move to an empty cell, what does active standard YR do from `UnitClass::Set_Destination` vtable `+0x480` through `FootClass::Set_Destination_Internal`, and which branch predicates can silently stop, drop, or early-return the order before Rust Phase 1 NavCom lifecycle work?

**Non-goals:** Do not investigate building/dock/chrono/aircraft/garrison handling, radio handshake, `NavQueue` producer paths, full pathfinding internals, or Drive movement physics after `Head_To_Coord`.

**Evidence needed to mark COMPLETE:** Unit vtable identity evidence, decompile of `0x00741970`, decompile of `0x004D94B0`, decompile of Drive `Head_To_Coord`/destination setter at `0x004AFD40`, evidence for `Stop_Moving` clearing `NavCom`, and a focused Rust scan of current destination ownership surfaces.

**Stop conditions:** Stop once the normal non-null empty-cell path, all in-scope early returns, and Rust-facing deltas are documented. If the path enters dock/radio/chrono/hover/aircraft branches, record the gate and leave it out of scope.

## 1. Overview

For a normal Drive vehicle move-to-empty-cell order, `0x00741970` performs preprocessing and then falls through to `FootClass::Set_Destination_Internal @ 0x004D94B0`. In the normal case, the dock/radio, chrono, hover, repair/reload, and transport blocks are skipped, and the destination is committed only by `Set_Destination_Internal`.

`Set_Destination_Internal` first clears `NavCom_Aux`, then may silently drop non-null destinations behind deploy/guard/warp-style predicates, then writes `Foot+0x5A4 = target`, obtains the target coordinate through target vtable `+0x4C`, and calls the active locomotor vtable `+0x44`. For a Drive locomotor, that vtable method is `DriveLocomotionClass__Set_Destination @ 0x004AFD40`; it stores the coordinate in Drive destination fields and adds bridge Z if the target cell has the bridge flag.

## 2. Class Layout / Key Offsets

| Offset / slot | Owner | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| vtable `+0x480` | UnitClass | destination entry slot resolves to `0x00741970` | vtable read in `TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md`; live decompile `0x00741970` | Yes |
| `Foot+0x5A0` / `param_1[0x168]` | FootClass | `NavCom_Aux`, cleared before any non-null guard checks | `0x004D94B0` decompile | Yes |
| `Foot+0x5A4` / `param_1[0x169]` | FootClass | `NavCom`, current destination object pointer | `0x004D94B0`, `0x004DF0D0` decompile | Yes |
| `Foot+0x674` / `param_1[0x19D]` | FootClass | active `ILocomotion*` used for `Head_To_Coord` / `Clear_Navigation` | `0x004D94B0` decompile | Yes |
| `Foot+0x6AD` | FootClass | non-null destination silent-drop guard; also participates in null deploy-abort cleanup | `0x004D94B0` decompile | Yes |
| `Foot+0x82` | FootClass | non-null destination silent-drop guard | `0x004D94B0` decompile | Yes |
| `Foot+0x2E4` / `param_1[0xB9]` | FootClass | non-null destination silent-drop guard | `0x004D94B0` decompile | Yes |
| `Foot+0x2AC` / `param_1[0xAB]` | FootClass | non-null destination invokes chrono-warp deploy helper before `NavCom` write | `0x004D94B0` decompile | Conditional |
| `Drive+0x30..0x38` | Drive locomotor interface view | destination coordinate written by `Head_To_Coord` vtable `+0x44` implementation | `0x004AFD40` decompile | Yes |
| target vtable `+0x4C` | destination object | returns the coordinate used for locomotor `Head_To_Coord` | `0x004D94B0` decompile | Yes |

## 3. Core Logic

### 3.1 UnitClass `Set_Destination` normal empty-cell path

`0x00741970` is reached through the UnitClass vtable slot `+0x480`. The live decompile still labels it `TechnoClass__Set_Destination`, but the UnitClass vtable proof in `TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md` identifies the active Unit override.

For the target slice, assume:

- `param_2 != NULL`;
- destination is an empty CellClass destination, not building/dock/occupied-cell/garrison/transport;
- owner is a normal Drive unit, not aircraft, not chrono/teleporter, not hover special, not repair/reload/transport carrier special;
- current mission is not `Mission_Enter(7)` and no active dock/radio path is involved.

Under those conditions, the normal path through `0x00741970` is:

1. Skip the chrono self-redirect block because the target is non-null and the normal unit type flags are not chrono-only.
2. Compare `param_2` against current `Foot+0x5A4`; if same destination and byte `+0x1F8` is clear, return before any reset.
3. Clear byte `+0x1F8` after the same-destination check.
4. Apply the movement-capability stop guard; if it fires, call `FootClass::Stop_Moving` and return.
5. Skip cancel-dock/occupied-cell/dock radio logic for the empty-cell non-enter path.
6. Skip the Mission_Enter dock block because `What_Am_I()` / current mission are not `7`.
7. Skip chrono/teleporter and hover locomotor swap blocks for a normal Drive locomotor.
8. Skip UnitRepair/UnitReload and transport-carrier loops because the filtered building candidate is null for the empty-cell destination.
9. Run the locomotor suspend scan, then tail-call `FootClass::Set_Destination_Internal(param_2, param_3)` at the end of the function.

**Evidence:** `0x00741970` decompile; early same-destination gate appears before the `+0x1F8` clear; final tail call appears at the end of `0x00743140..0x00743186` after locomotor suspend scan. **Active in YR:** Yes for UnitClass destinations; branch subsets are conditional.

### 3.2 Early stop and early return predicates in `0x00741970`

These are in-scope because they can silently prevent Phase 1 NavCom ownership from changing:

| Predicate | Effect | Evidence | Active in YR |
|---|---|---|---|
| `param_2 == Foot+0x5A4 && +0x1F8 == 0` | returns immediately; no `NavCom_Aux` clear, no retry timer reset, no `Head_To_Coord` | `0x00741970` decompile near same-destination gate | Yes |
| Instance movement-state guard: `this+0x6E0 == 0` and (`this+0x6E1 != 0` or `this+0x6E2 != 0`) | calls `FootClass::Stop_Moving` then returns | `TECHNOCLASS_SET_DESTINATION_PREPROCESSING_FLAGS_GHIDRA_REPORT.md`, asm `0x00741A96..0x00741AB1` | Conditional |
| Instance movement-state guard: `this+0x6E0 != 0` and owner field `this+0x2B0` is null | calls `FootClass::Stop_Moving` then returns | `TECHNOCLASS_SET_DESTINATION_PREPROCESSING_FLAGS_GHIDRA_REPORT.md`, asm `0x00741A96..0x00741ACB` | Conditional |

`FootClass::Stop_Moving @ 0x004DF0D0` only zeros `Foot+0x5A0` and `Foot+0x5A4` in the decompiled body inspected here. It does not itself call the locomotor `Clear_Navigation`; that happens in `Set_Destination_Internal`'s null-target branch.

### 3.3 `FootClass::Set_Destination_Internal` commit order

`0x004D94B0` is the only in-scope function that writes `NavCom = target` for a successful non-null destination. The order is load-bearing:

1. Always write `Foot+0x5A0 = 0`.
2. If `target != NULL`, return before writing `NavCom` when any of these are true: `Foot+0x6AD != 0`, `Foot+0x82 != 0`, or `Foot+0x2E4 != 0`.
3. If `Foot+0x2AC != 0` and `target != NULL`, call `BuildingClass__DeployUnit_ChronoWarp(1)`.
4. Write `Foot+0x5A4 = target`.
5. For non-null `NavCom`, release `Foot+0x304` if present, perform piggyback/query checks, and apply the WalkLocomotion retry special only if the active locomotor CLSID is Walk.
6. If `Foot+0x6AC == 0`, call `target->vtable+0x4C(this)` to get the destination coordinate, then call active locomotor `+0x44(coord)`.
7. If `Foot+0x6AC != 0`, clear `Foot+0x6AC` and skip `Head_To_Coord`.
8. Always reset path/retry state at the common tail: `Foot+0x6B7 = 0`, `+0x668 = CurrentFrame`, `+0x66C = EBX/decompiler-local`, `+0x670 = RulesClass+0x1768`, `+0x640 = CurrentFrame`, `+0x644 = EBX/decompiler-local`, `+0x648 = 0`.

**Evidence:** `0x004D94B0` decompile; disassembly dry-run confirmed executable range `0x004D94B0..0x004D9700`. **Active in YR:** Yes.

### 3.4 Drive locomotor `Head_To_Coord` implementation

`FootClass::Set_Destination_Internal` calls active locomotor vtable `+0x44`. For Drive, this resolves to `DriveLocomotionClass__Set_Destination @ 0x004AFD40` in the helper report and live decompile.

The function:

1. Calls four owner virtual predicates at offsets `+0x37C`, `+0x380`, `+0x1D4`, and `+0x1D8`.
2. If any predicate returns nonzero, returns without writing the Drive destination fields.
3. Otherwise writes `Drive+0x30 = x`, `Drive+0x34 = y`, `Drive+0x38 = z`.
4. If the coordinate is not the Drive null coordinate triplet, gets the destination cell and, when `Cell+0x140 & 0x100` is set, adds `g_BridgeZOffset_Drive` to `Drive+0x38`.

**Evidence:** `0x004AFD40` decompile; disassembly dry-run confirmed executable range `0x004AFD40..0x004AFDA0`. **Active in YR:** Yes for Drive-locomotor units.

### 3.5 Path validity helper is not NavCom

`PathType__Has_Valid_Steps @ 0x0065AE30` returns true when the path array count at `+0xE8` is positive and at least one entry in the path array pointer at `+0xE4` is nonzero. It does not read `Foot+0x5A4` and does not inspect Drive `+0x30..0x38` destination fields.

**Evidence:** `0x0065AE30` decompile. **Active in YR:** Yes.

`FootClass__GetDestination @ 0x0065AD30` reads from the path/radio vector pointer at `param_1 + 0xE4`, indexed by the requested slot; it is not the same as `NavCom` itself. Existing docs use "GetDestination" for radio/path contacts; Phase 1 Rust should not conflate this helper with `Foot+0x5A4`.

**Evidence:** `0x0065AD30` decompile. **Active in YR:** Yes.

## 4. INI Keys

No NavCom-specific INI key exists in this slice.

| Key | Section | Stock value / role | Evidence | Active in YR |
|---|---|---|---|---|
| `BlockagePathDelay` | `[General]` | `60`; copied into `Foot+0x670` by every successful `Set_Destination_Internal` tail | `rulesmd.ini:3107`, `0x004D94B0`, `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md` | Yes |
| `PathDelay` | `[General]` | `.01`; walker-specific retry cadence, not normal Drive-specific in this slice | `rulesmd.ini:3106`, `0x004D94B0`, `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md` | Yes |
| `Locomotor` | unit sections | Drive CLSID `{4A582741-9839-11d1-B709-00A024DDAFD1}` selects Drive locomotor for many normal vehicles | `rulesmd.ini` Locomotor entries, `DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md` | Yes |
| `Teleporter` | unit sections | Gates chrono/teleporter branches intentionally out of scope | `rulesmd.ini`, `0x00741970` Type flags per prior docs | Conditional |

## 5. Integration Points

| Function / point | Role | Evidence | Active in YR |
|---|---|---|---|
| UnitClass vtable `+0x480 -> 0x00741970` | public destination preprocessing for UnitClass objects | `TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md` vtable read; live decompile | Yes |
| `0x00741970` | preprocesses target, may early-return/stop, then delegates | live decompile | Yes |
| `0x004D94B0` | writes `NavCom` and calls locomotor `Head_To_Coord` | live decompile | Yes |
| `0x004AFD40` | Drive locomotor destination/head-to write | live decompile | Yes |
| `0x004DF0D0` | clears `NavCom_Aux` and `NavCom` | live decompile | Yes |
| `0x0065AE30` | path array valid-step query used by special branches | live decompile | Yes |
| `0x0065AD30` | path/radio destination vector accessor, not `NavCom` | live decompile | Yes |

## 6. Current Rust Implementation Status

Current Rust surfaces scanned:

| Surface | Current behavior | Delta / risk |
|---|---|---|
| `src/sim/components.rs` `NavigationState` + `MovementTarget` | `NavigationState` now owns `nav_com_aux`, `nav_com`, `suspended_nav_com`, and `nav_queue`; `MovementTarget` still owns active path/speed/final goal execution | Directionally closer than the older MovementTarget-only scan, but parity still needs the exact owner NavCom vs locomotor destination split and guarded Set_Destination ordering. |
| `src/sim/movement/movement_commands.rs:41` `can_accept_destination` | blocks dying/building-up/down/drop-pod falling/unloading | Partial analogue to binary early drop/stop guards, but not the same offset predicates; does not distinguish "silent drop before NavCom write" from false return semantics. |
| `src/sim/movement/movement_commands.rs:255` `issue_move_command_with_layered` | computes/attaches `MovementTarget` directly, and after the recent fix can start `DriveTrack` for normal Drive vehicles | This approximates `Set_Destination + Head_To_Coord` as one Rust operation; there is no explicit `navcom` owner that can survive/clear independently. |
| `src/sim/movement/movement_commands.rs:506` DriveTrack start | starts a DriveTrack for first leg and clears `facing_target` on success | Directionally matches routing into Drive tracks, but it is not a separate Drive locomotor object receiving a coordinate from `NavCom`. |
| `src/sim/movement/movement_tick.rs:530` | movement tick processes only entities with `movement_target.is_some()` | If Phase 1 adds `navcom`, active movement and destination-line ownership need separate predicates. |
| `src/sim/movement/movement_tick.rs:1156` `finalize_finished_entities` | clears `movement_target` and `drive_track` together on path finish | Gamemd can clear Drive movement state separately from `NavCom`, and empty-queue arrival clears `NavCom` through `Set_Destination(NULL,1)` rather than by path exhaustion alone. |
| `src/app_target_lines.rs` | selected move line can read `navigation.nav_queue.last().or(nav_com)` | Action-line ownership now has a NavCom-shaped source, but normal runtime queue appends must be narrowed because the newer producer audit found no standard YR command/team/trigger NavQueue push. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| UnitClass vtable `+0x480` identity | verified-via-prior-vtable-doc | `TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md`; live decompile `0x00741970` | direct memory-read tool was not available in this session |
| Normal empty-cell fall-through in `0x00741970` | verified | live decompile `0x00741970` | exact player UI command caller out of scope |
| Same-destination early return | verified | live decompile `0x00741970` | semantic name for `+0x1F8` remains doc-dependent |
| Movement-capability stop guard | corrected by follow-up | `TECHNOCLASS_SET_DESTINATION_PREPROCESSING_FLAGS_GHIDRA_REPORT.md`; `0x004DF0D0` | semantic names for instance bytes `+0x6E0/+0x6E1/+0x6E2` remain unresolved; the old Type `+0xD28/+0xD29/+0xD2A` interpretation was wrong |
| Dock/radio/occupied-cell branches | touched-not-exhausted | live decompile `0x00741970` | intentionally out of scope |
| Chrono/hover/piggyback branches | touched-not-exhausted | live decompile `0x00741970` | intentionally out of scope |
| `FootClass::Set_Destination_Internal` commit order | verified | live decompile `0x004D94B0` | decompiler-local EBX names remain unresolved but reset writes are verified |
| Drive `Head_To_Coord` write | verified | live decompile `0x004AFD40` | owner virtual predicate names unresolved |
| `PathType::Has_Valid_Steps` separation from NavCom | verified | live decompile `0x0065AE30` | no need for this slice |
| Current Rust destination surfaces | verified | focused `rg` + file reads listed in section 6 | implementation separate |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is `0x00741970` the active UnitClass destination slot for this target? -> Yes by prior vtable proof; the live decompile body is the preprocessing path used by UnitClass vtable +0x480.` (evidence: `TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md`; `0x00741970`)
- `[RESOLVED] OQ-02 - Does normal empty-cell movement enter dock/radio preprocessing? -> No, not when the destination is an empty cell and the unit is not in Mission_Enter/dock state; those branches are gated by mission/RTTI/path/contact predicates.` (evidence: `0x00741970`)
- `[RESOLVED] OQ-03 - What writes `NavCom` for the successful non-null destination? -> `FootClass::Set_Destination_Internal` writes `Foot+0x5A4 = target` after its silent-drop guards.` (evidence: `0x004D94B0`)
- `[RESOLVED] OQ-04 - Can a non-null destination be silently dropped before `NavCom` is written? -> Yes: `Foot+0x6AD`, `Foot+0x82`, or `Foot+0x2E4` returns before the `Foot+0x5A4` write.` (evidence: `0x004D94B0`)
- `[RESOLVED] OQ-05 - Does `Set_Destination_Internal` call the locomotor after writing `NavCom`? -> Yes; if `Foot+0x6AC == 0`, target vtable `+0x4C` supplies the coord and locomotor vtable `+0x44` receives it.` (evidence: `0x004D94B0`)
- `[RESOLVED] OQ-06 - What does Drive do with the `Head_To_Coord` coordinate? -> It writes Drive destination fields at `+0x30/+0x34/+0x38`, unless one of four owner virtual predicates returns nonzero; bridge cells add `g_BridgeZOffset_Drive` to Z.` (evidence: `0x004AFD40`)
- `[RESOLVED] OQ-07 - Does `Stop_Moving` clear NavCom? -> Yes, `0x004DF0D0` zeros `Foot+0x5A0` and `Foot+0x5A4` only in the inspected body.` (evidence: `0x004DF0D0`)
- `[RESOLVED] OQ-08 - Is `PathType::Has_Valid_Steps` equivalent to active Drive destination? -> No, it scans the `PathType` path array and does not inspect `NavCom` or Drive destination fields.` (evidence: `0x0065AE30`)
- `[UPDATED 2026-05-27] OQ-09 - Does current Rust have a separate `NavCom` owner? -> Partly yes: `NavigationState` now has `nav_com`, `suspended_nav_com`, and `nav_queue`; remaining work is exact Set_Destination guard order, queue producer narrowing, and arrival/PointerExpired lifecycle parity.` (evidence: `src/sim/components.rs`, newer NavCom reports)
- `[DEFERRED] OQ-10 - Exact player command handler and argument construction for the empty-cell `CellClass*`.` (category: out-of-scope; reason: this slot starts at the verified UnitClass vtable path; next-step-if-pursued: trace command/UI target selection to the virtual call.)
- `[DEFERRED] OQ-11 - Human-readable names for owner virtual predicates `+0x37C/+0x380/+0x1D4/+0x1D8` in Drive `Head_To_Coord`.` (category: bounded-cost-too-high; reason: not needed to prove that they can suppress the Drive coordinate write; next-step-if-pursued: run a vtable predicate slot audit.)
- `[DEFERRED] OQ-12 - Full arrival clear lifecycle after reaching the destination cell.` (category: requires-different-system-context; reason: owned by Drive arrival / `Set_Destination(NULL,1)` slots, not initial destination assignment; next-step-if-pursued: use slot 4 Drive arrival findings.)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Successful normal UnitClass empty-cell destination writes `NavCom` in `Set_Destination_Internal`, then separately tells the active Drive locomotor the target coordinate. | `0x00741970`, `0x004D94B0`, `0x004AFD40` | Missing: Rust has `MovementTarget` as combined destination/path owner | `src/sim/components.rs`; `src/sim/movement/movement_commands.rs`; `src/sim/movement/movement_tick.rs` | Add an explicit normal-drive destination owner, e.g. a `navcom`/`nav_destination` cell target, and let movement/DriveTrack be the locomotor-side execution beneath it. | Issue move to empty cell; `navcom` stores the requested/effective cell while `movement_target`/`drive_track` own active stepping; action line can read `navcom`. | Do not make `movement_target.is_some()` the only proof that a unit has a destination. |
| Non-null destinations can be swallowed before `NavCom` is written by `Foot+0x6AD`, `Foot+0x82`, or `Foot+0x2E4`; same-destination can return before any reset. | `0x00741970`, `0x004D94B0` | Partial: `can_accept_destination` blocks some states, but predicates and side effects do not match commit order | `src/sim/movement/movement_commands.rs:41`; deploy/drop/chrono state surfaces | Preserve silent-drop/early-return semantics separately from pathfinding failure: no new `navcom`, no new path, and no retry reset when the binary would return before the write. | A deployed/warped/falling-equivalent unit receives a move order; prior destination/path state remains exactly as the verified predicate requires. | Do not queue or retry orders that gamemd silently drops before `NavCom` write. |
| Drive `Head_To_Coord` can reject the coordinate after `NavCom` is written via four owner virtual predicates; otherwise it writes Drive destination fields and bridge-adjusted Z. | `0x004AFD40` | Missing: Rust path attachment and DriveTrack start are tied to pathfinding success, not a Drive destination-write contract | `src/sim/movement/movement_commands.rs:506`; `src/sim/movement/movement_tick.rs`; future Drive locomotor state | Model Drive destination/head-to state separately enough that `NavCom` can exist even if Drive destination write is suppressed or later clears. | Move command to bridge/normal cell writes nav destination; Drive head-to destination reflects bridge Z adjustment and can be independently cleared/blocked in later locomotor processing. | Do not equate `NavCom` write success with DriveTrack or vector movement already being active. |

Proposed test names:

- `test_normal_drive_move_sets_navcom_before_drive_head_to`
- `test_set_destination_internal_silent_drop_does_not_replace_navcom`
- `test_selected_move_line_uses_navcom_after_movement_target_clears`

## 10. Negative Facts / Do Not Do

- Do not implement Phase 1 as just another field inside `MovementTarget`; gamemd owns `NavCom` at `Foot+0x5A4` separately from Drive destination fields and path-array validity. Evidence: `0x004D94B0`, `0x004AFD40`, `0x0065AE30`.
- Do not treat `FootClass__GetDestination @ 0x0065AD30` as `NavCom`; it reads a vector at `+0xE4`, while `NavCom` is `+0x5A4`. Evidence: `0x0065AD30`, `0x004D94B0`.
- Do not clear or replace `NavCom` when `0x00741970` hits the same-destination early return; the return happens before the `+0x1F8` clear and before `Set_Destination_Internal`. Evidence: `0x00741970`.
- Do not let deploy/warp/drop predicates recompute path and retry later for free; `0x004D94B0` returns before writing `NavCom`. Evidence: `0x004D94B0`.
- Do not assume a successful `NavCom` write means Drive fields were written; Drive `Head_To_Coord` has four owner-predicate early returns. Evidence: `0x004AFD40`.

## 11. Remaining Uncertainty

- Exact names/default semantics for instance bytes `this+0x6E0/+0x6E1/+0x6E2`, `Foot+0x82`, and Drive owner virtual predicates `+0x37C/+0x380/+0x1D4/+0x1D8` are unresolved. The old `Type+0xD28/+0xD29/+0xD2A` interpretation was corrected by `TECHNOCLASS_SET_DESTINATION_PREPROCESSING_FLAGS_GHIDRA_REPORT.md`; those TypeClass fields are `Crusher`, `OmniCrusher`, and `OmniCrushResistant`, not this stop guard.
- This report does not trace the UI/player command construction of the `CellClass*`; it starts at the verified UnitClass destination virtual.
- The exact arrival teardown through `Set_Destination(NULL,1)` is intentionally left to the Drive arrival/navqueue slots.

## 12. Stale Docs / Follow-up Docs

`docs/research/NAVCOM_LIFECYCLE_GHIDRA_REPORT.md` contains stale/ambiguous wording in its overview saying `0x00741970` is "NOT invoked via FootClass vtable +0x480" and may imply all Foot-derived units bypass it. Replacement wording:

> `FootClass`'s own base vtable slot can point directly at `FootClass::Set_Destination_Internal`, but `UnitClass` overrides vtable `+0x480` with the preprocessing entry at `0x00741970`. For UnitClass vehicles, including normal Drive-locomotor vehicles, destination assignment through the UnitClass vtable reaches `0x00741970` before tail-calling `FootClass::Set_Destination_Internal`.

No patch was applied in this slot.

## Sources

- Ghidra decompile:
  - `0x00741970` - UnitClass vtable `+0x480` destination preprocessing, Ghidra label `TechnoClass__Set_Destination`
  - `0x004D94B0` - `FootClass::Set_Destination_Internal`
  - `0x004AFD40` - `DriveLocomotionClass__Set_Destination` / Drive `Head_To_Coord` coordinate writer
  - `0x004DF0D0` - `FootClass::Stop_Moving`
  - `0x0065AE30` - `PathType::Has_Valid_Steps`
  - `0x0065AD30` - `FootClass__GetDestination`
- Ghidra disassembly dry-runs:
  - `0x00741970..0x00741B00`
  - `0x00743140..0x00743186`
  - `0x004D94B0..0x004D9700`
  - `0x004AFD40..0x004AFDA0`
- Prior docs:
  - `docs/research/TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md`
  - `docs/research/NAVCOM_LIFECYCLE_GHIDRA_REPORT.md`
  - `docs/research/DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md`
  - `docs/research/DRIVELOCOMOTOR_ACCEPTED_CELL_ARRIVAL_VISIBILITY_GHIDRA_REPORT.md`
- Rust scanned:
  - `src/sim/components.rs`
  - `src/sim/movement/movement_commands.rs`
  - `src/sim/movement/movement_tick.rs`
  - `src/app_target_lines.rs`
- INI checked:
  - `ini/rulesmd.ini`
  - `ini/rules.ini`
