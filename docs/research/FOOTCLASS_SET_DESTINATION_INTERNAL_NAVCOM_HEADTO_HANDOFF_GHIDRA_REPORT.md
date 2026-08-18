# FootClass::Set_Destination_Internal NavCom / Head_To_Coord Handoff - Ghidra Research Report

**Address(es):** `0x004D94B0` primary; supporting `0x004AFD40`, `0x004AFE00`, `0x004DF0D0`, `0x004D8F40`, `0x004D8F80`, `0x00741970`, `0x0041AA80`, `0x0051AA40`  
**Investigation Mode:** exhaustive-slice  
**Target question:** What exact `FootClass::Set_Destination_Internal @ 0x004D94B0` state writes and locomotor calls must Rust own for normal ground/Drive `NavCom -> Head_To_Coord` lifecycle parity?  
**Non-goals:** Full `TechnoClass::Set_Destination` preprocessing, refinery/dock special cases, chrono piggyback lifecycle, aircraft/ship variants, full Drive `Process_Movement` speed/track timing.  
**Evidence needed to mark COMPLETE:** Primary decompile plus disassembly for `0x004D94B0`; vtable pointer proof; caller/xref proof of active YR entry; concrete Drive `Head_To_Coord` and clear-navigation target functions; current Rust surface scan; accepted uncertainty separated.  
**Stop conditions:** No Ghidra read-only access; function boundary missing; inability to verify offsets from assembly; broad expansion into docking/chrono/queue systems.  
**Confidence:** High for this bounded function and normal Drive handoff.  
**Active in YR:** Yes. FootClass vtable slot `+0x480` points directly to `0x004D94B0` (`read_memory 0x007E9114 4 -> b0 94 4d 00`), and Ghidra xrefs show direct calls from active movement/dock command paths plus data vtable usage.

## 1. Overview

`FootClass::Set_Destination_Internal` is the final `FootClass` destination writer. It clears `NavCom_Aux`, conditionally drops non-null destinations under three owner-state guards, writes `NavCom`, dispatches either locomotor `Head_To_Coord` for a non-null target or locomotor clear/stop for a null target, then resets path/blocked retry state.

For normal ground Drive units, this means Rust needs a `NavCom` owner separate from `MovementTarget`: the owner target pointer/cell remains the gameplay/UI destination, while the Drive locomotor owns the current `head_to` coordinate and track state.

## 2. Field / Slot Findings

| Offset / slot | Verified role in this slice | Evidence | Active in YR |
|---|---|---|---|
| Foot `+0x5A0` | `NavCom_Aux`; cleared before any guard | `0x004D94C7` writes zero | Yes |
| Foot `+0x5A4` | `NavCom`; written to target after guards | `0x004D9510` | Yes |
| Foot `+0x5A8` | Suspended NavCom; save/restore helper storage | `0x004D8F40`, `0x004D8F80` | Conditional |
| Foot `+0x6AD` | non-null destination guard; also null undeploy branch gate | `0x004D94BE..0x004D94D1`, `0x004D9518..0x004D9538` | Conditional |
| Foot `+0x82` | byte guard that silently drops non-null destination | `0x004D94D7..0x004D94E3` | Conditional |
| Foot `+0x2E4` | pointer/state guard that silently drops non-null destination | `0x004D94E9..0x004D94F3` | Conditional |
| Foot `+0x2AC` | chrono/deploy warp pointer; non-null target calls helper before writing NavCom | `0x004D94F9..0x004D9509` | Conditional |
| Foot `+0x2B0` | cleared with pointed object's `+0x2AC` on null destination when `+0x6AD` set | `0x004D9522..0x004D9538` | Conditional |
| Foot `+0x304` | active particle system/contact-like pointer; vtable `+0xF8` then clear before Head_To_Coord | `0x004D954B..0x004D955D` | Conditional |
| Foot `+0x6AC` | one-shot skip-Head_To_Coord byte; if set, clear byte and skip target-coordinate dispatch | `0x004D9607..0x004D9618` | Conditional |
| Foot `+0x6B7` | path-blocked/failed byte cleared on every non-dropped call | `0x004D96C2` | Yes |
| Foot `+0x668/+0x66C/+0x670` | blocked retry timer state reset to current frame, zero, `Rules+0x1768` | `0x004D96C9..0x004D96ED` | Yes |
| Foot `+0x640/+0x644/+0x648` | movement retry timer state reset to current frame, zero, zero; Walk branch may first set duration 1 | `0x004D95C7..0x004D9601`, `0x004D96F0..0x004D9707` | Yes |
| ILocomotion `+0x44` | `Head_To_Coord`/set-destination dispatch for non-null target | `0x004D9647..0x004D965D`; Drive vtable `0x007E7EB0+0x44 -> 0x004AFD40` | Yes |
| ILocomotion `+0x48` | clear navigation / stop-moving dispatch for null target, except attack carve-out | `0x004D96B0..0x004D96B9`; Drive vtable `0x007E7EB0+0x48 -> 0x004AFE00` | Yes |

## 3. Core Logic

Verified order for `0x004D94B0`:

1. Clear `Foot+0x5A0 = 0` before checking whether the incoming target will be accepted.
2. If target is non-null, silently return before writing `NavCom` when any of these is true: byte `+0x6AD != 0`, byte `+0x82 != 0`, or dword `+0x2E4 != 0`.
3. If `+0x2AC != 0` and target is non-null, call `BuildingClass__DeployUnit_ChronoWarp(1)` before writing `NavCom`.
4. Write `Foot+0x5A4 = target`.
5. If target is null and `+0x6AD != 0` and `+0x2B0 != 0`, clear `(*+0x2B0)+0x2AC`, clear `+0x2B0`, and set byte `+0x6AE = 1`.
6. If `NavCom` is null, usually call active locomotor vtable `+0x48`; but skip that call for `What_Am_I()==2` with current or queued mission `1` and non-null `+0x2B4` ArchiveTarget.
7. If `NavCom` is non-null, release `+0x304` if present, query active locomotor for `IPiggyback`, run a Walk-CLSID retry-timer special case, then either:
   - if byte `+0x6AC == 0`, call `NavCom->vtable+0x4C(out, this)` for coordinates and dispatch active locomotor vtable `+0x44` with those coordinates;
   - if byte `+0x6AC != 0`, clear byte `+0x6AC = 0` and skip this `Head_To_Coord` dispatch.
8. Release the temporary piggyback interface if acquired.
9. Reset `+0x6B7`, blocked retry timer `+0x668/+0x66C/+0x670`, and movement retry timer `+0x640/+0x644/+0x648`.

The function returns with `RET 0x8`. The first stack argument is the target pointer. The second stack argument is passed by callers as a force/suspend convention, but this function body does not branch on it in the verified disassembly.

## 4. Drive-Specific Dispatch

For normal Drive locomotors, the `Head_To_Coord` dispatch target is `DriveLocomotionClass::Set_Destination @ 0x004AFD40`, proven by Drive vtable data at `0x007E7EB0+0x44`.

`0x004AFD40` writes Drive internal destination coords at `Drive+0x30/+0x34/+0x38` after checking four owner predicates through the owner vtable: `+0x37C`, `+0x380`, `+0x1D4`, `+0x1D8`. If the target is not `NullCoord`, it resolves the target cell and adds `g_BridgeZOffset_Drive` to `Drive+0x38` when `Cell+0x140 & 0x100` is set.

For null destination, the `+0x48` dispatch target is `DriveLocomotionClass::Stop_Moving @ 0x004AFE00`. It may propagate stop to train followers when the current head-to is non-null and train-owner predicates pass, clamps `Drive+0x4C` to at most the global value at `0x007E6240`, and writes `Drive+0x30/+0x34/+0x38 = NullCoord`. It does not clear Foot `NavCom`; Foot `NavCom` was already cleared by `0x004D9510`.

## 5. Integration Points

Verified xrefs to `0x004D94B0`:

- `0x007E9114 [DATA]`: FootClass vtable slot `+0x480`; active generic Foot-derived Set_Destination entry.
- `TechnoClass__Set_Destination @ 0x00741970`: direct calls at `0x00742D0B` and `0x00743161` after preprocessing.
- `UnitClass__EnterBuildingOrDock @ 0x0041AA80`: direct calls at `0x0041AAA6`, `0x0041AD06`, and `0x0041ADB4`.
- `FUN_0051AA40`: direct call at `0x0051B1D2`; target/caller not fully scoped here.

Support helpers:

- `FootClass__Stop_Moving @ 0x004DF0D0` only clears `+0x5A0` and `+0x5A4`.
- `FootClass__Set_NavCom_With_Suspend @ 0x004D8F40` copies `+0x5A4 -> +0x5A8`, calls target/archive helper `0x007013A0`, then calls vtable `+0x480(new_target, 1)`.
- `FUN_004D8F80` calls helper `0x007013E0`; if true, restores `+0x5A8` through vtable `+0x480(saved_navcom, 1)`.

## 6. Current Rust Implementation Status

Rust currently has no explicit `NavCom`/`NavCom_Aux`/`SuspendedNavCom` owner on `GameEntity`. Movement destination ownership is represented primarily by `GameEntity.movement_target: Option<MovementTarget>`, with `MovementTarget.final_goal` storing a cell goal.

Relevant current surfaces:

- `src/sim/game_entity.rs`: `GameEntity` has `movement_target`, `drive_track`, `locomotor`, and `radio_contacts`, but no explicit NavCom equivalent.
- `src/sim/components.rs`: `MovementTarget` includes path, final goal, current speed, and retry/block timer surrogates.
- `src/sim/movement/movement_commands.rs`: `issue_move_command` builds `MovementTarget`, sets `final_goal`, and now starts initial DriveTrack for Drive units; it does not model `Foot+0x5A0/+0x5A4` or the guarded `Head_To_Coord` call.
- `src/sim/movement/movement_tick.rs`: `finalize_finished_entities` clears `movement_target` and `drive_track`, snaps subcell, and idles locomotor. This is not equivalent to `Set_Destination_Internal(NULL, 1)` plus Drive `Stop_Moving`, because Rust has no retained/cleared `NavCom` owner to route through.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FootClass::Set_Destination_Internal @ 0x004D94B0` field writes | verified | decompile + disassembly `0x004D94B0..0x004D9713` | none for this slice |
| Foot vtable `+0x480` active entry | verified | `read_memory 0x007E9114 -> b0944d00`; xref data | none |
| Non-null silent-drop guards | verified | `0x004D94BE..0x004D94F3` | semantic names for `+0x82/+0x2E4` remain out-of-scope |
| Null target clear-navigation branch | verified | `0x004D9672..0x004D96BC` | exact mission enum name for mission `1` accepted from existing docs, not re-derived here |
| Non-null target `Head_To_Coord` branch | verified | `0x004D9607..0x004D966D` | exact coordinate provider implementations per target type out-of-scope |
| Drive `Head_To_Coord` concrete target | verified | Drive vtable `0x007E7EB0+0x44 -> 0x004AFD40`; decompile/disasm `0x004AFD40` | none for normal Drive |
| Drive clear-navigation concrete target | verified | Drive vtable `0x007E7EB0+0x48 -> 0x004AFE00`; decompile/disasm `0x004AFE00` | train propagation not exhausted |
| `NavCom_Aux` non-null writers | touched-not-exhausted | primary function and helpers clear/save/restore around `+0x5A0/+0x5A8` | global search for non-null `+0x5A0` writers not completed |
| NavQueue/queued waypoint behavior | deferred | out-of-scope | needs slot-5 / action-line queue investigation |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does the function write NavCom before or after guard predicates? -> after non-null guards; `+0x5A0` is cleared before guards.` (evidence: `0x004D94C7`, `0x004D9510`)
- `[RESOLVED] OQ-02 - Which guard offset was the previously disputed secondary guard? -> byte `+0x82`, not `+0x208`.` (evidence: `0x004D94D7..0x004D94E3`)
- `[RESOLVED] OQ-03 - Is `+0x6B7` cleared or `+0x6DC`? -> byte `+0x6B7`.` (evidence: `0x004D96C2`)
- `[RESOLVED] OQ-04 - Does null destination always clear the locomotor? -> no; Unit/mission-1/ArchiveTarget carve-out skips vtable `+0x48`.` (evidence: `0x004D9672..0x004D96BC`)
- `[RESOLVED] OQ-05 - What concrete Drive function receives non-null `Head_To_Coord`? -> `0x004AFD40`, not the `0x004AFCC0` getter-like slot.` (evidence: Drive vtable `0x007E7EB0+0x44`)
- `[RESOLVED] OQ-06 - What concrete Drive function receives null clear-navigation? -> `0x004AFE00`.` (evidence: Drive vtable `0x007E7EB0+0x48`)
- `[RESOLVED] OQ-07 - Does the second stack argument affect this function's branch behavior? -> no branch or load-bearing read found in the verified body; callers still pass and clean up two args.` (evidence: disassembly `RET 0x8`)
- `[DEFERRED] OQ-08 - What are global non-null writers/readers of `NavCom_Aux +0x5A0`?` (category: out-of-scope; reason: this slot is bounded to `0x004D94B0`; next-step-if-pursued: global xref/dataflow audit for `+0x5A0`)
- `[DEFERRED] OQ-09 - How should queued waypoints alter NavCom/action-line endpoint?` (category: requires-different-system-context; reason: belongs to NavQueue/action-line slot; next-step-if-pursued: verify `+0x588..+0x598` queue owner and `DrawActionLines` priority)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `Set_Destination_Internal` owns `NavCom`: clear `NavCom_Aux`, apply non-null guards, write `NavCom`, dispatch locomotor `Head_To_Coord`, then reset retry timers | `0x004D94B0..0x004D9713` | Missing explicit NavCom owner; `MovementTarget` currently owns destination | `src/sim/game_entity.rs`, `src/sim/components.rs`, `src/sim/movement/movement_commands.rs` | Add gamemd-shaped destination owner for normal Drive move-to-cell: `NavCom`/cell target separate from path segment, and route first Drive destination through Head_To_Coord adapter | AMCV move-to-cell sets NavCom before path/track stepping and retains destination for action-line/arrival lifecycle | Do not make `MovementTarget.final_goal` the only source of truth for gamemd destination |
| Null destination clears `NavCom` first, then calls locomotor vtable `+0x48` except the Unit attack carve-out | `0x004D9510`, `0x004D9672..0x004D96BC`; Drive stop `0x004AFE00` | `finalize_finished_entities` directly clears `movement_target`/`drive_track` and idles locomotor | `src/sim/movement/movement_tick.rs`, future NavCom lifecycle helper | Arrival/stop paths should call a Rust equivalent of `Set_Destination_Internal(NULL, 1)` so Foot destination and Drive head-to clear in gamemd order | Empty-queue AMCV arrival clears NavCom and Drive destination through one lifecycle path, not ad hoc finalizer teardown | Do not equate "movement finished" with blind `movement_target=None` before modeling NavCom clear semantics |
| New accepted destination resets path-blocked byte and retry timers every non-dropped call | `0x004D96C2..0x004D9707`; `rulesmd.ini [General] BlockagePathDelay=60`, `PathDelay=.01` | Retry state exists inside `MovementTarget`; no reset occurs if there is no target or if future NavCom exists separately | `src/sim/components.rs`, `src/sim/movement/movement_commands.rs`, `src/sim/movement/movement_blocked.rs` | Store or mirror Foot-level retry/block fields so new destination resets happen even before/without a path segment | Issuing a fresh move after blockage clears path-blocked state and restarts blockage/path retry cadence | Do not leave retry timers solely inside path segment state if NavCom can outlive/restart segments |

Proposed test names:

- `test_drive_set_destination_internal_writes_navcom_before_head_to_coord`
- `test_drive_arrival_null_destination_clears_navcom_through_lifecycle`
- `test_new_navcom_destination_resets_blocked_retry_fields`
- `test_null_destination_attack_carveout_preserves_drive_head_to`

## 10. Negative Facts / Do Not Do

- Do not route normal Foot-derived `vtable+0x480` through the 500-line `TechnoClass::Set_Destination @ 0x00741970` by default. Foot vtable `+0x480` is `0x004D94B0` directly (`read_memory 0x007E9114 -> b0944d00`).
- Do not label the second guard as `+0x208`; the verified byte read is `Foot+0x82` (`0x004D94D7..0x004D94E3`).
- Do not clear `+0x6DC` for the path-blocked flag in this function; the verified write is byte `Foot+0x6B7 = 0` (`0x004D96C2`).
- Do not treat Drive `0x004AFCC0` as the destination-setting `Head_To_Coord` implementation used by `Set_Destination_Internal`; the vtable `+0x44` target is `0x004AFD40`. `0x004AFCC0` is vtable `+0x18` and returns a current/fallback head coordinate.
- Do not make Drive `Stop_Moving @ 0x004AFE00` clear Foot `NavCom`; Foot `NavCom` is cleared by `0x004D94B0`, and Drive stop only resets Drive internal destination coords.

## Stale Docs / Follow-up Docs

- `docs/research/DRIVE_LOCOMOTION_CLASS.md`: if any section still calls `0x004AFCC0` "Head_To_Coord" in the sense of the vtable `+0x44` destination setter, replace with: "`0x004AFCC0` is Drive vtable `+0x18` coordinate getter/fallback; Drive vtable `+0x44` destination setter called by `FootClass::Set_Destination_Internal` is `0x004AFD40`."
- `docs/research/NAVCOM_LIFECYCLE_GHIDRA_REPORT.md`: current corrected wording for `+0x82`, `+0x6B7`, and Foot vtable `+0x480` is corroborated by this slot. No replacement needed for those corrected sections.

## Sources

- Ghidra read-only: `get_function_by_address 004D94B0`; `decompile_function`/`disassemble_function` for `004D94B0`, `004AFD40`, `004AFE00`, `004DF0D0`, `004D8F40`, `004D8F80`, `00741970`, `0041AA80`, `0051AA40`.
- Ghidra read-only: `get_function_xrefs 004D94B0`; `read_memory 007E9114 4`; `read_memory 007E7EB0 96`.
- Local docs referenced: `docs/research/NAVCOM_LIFECYCLE_GHIDRA_REPORT.md`, `docs/research/TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md`, `docs/research/DRIVE_LOCOMOTION_CLASS.md`, `docs/research/DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md`, `docs/research/DRIVELOCOMOTOR_ACCEPTED_CELL_ARRIVAL_VISIBILITY_GHIDRA_REPORT.md`, `docs/research/UNIT_MISSION_ENTER_REFINERY_RETRY_QUEUE_LOOP_GHIDRA_REPORT.md`.
- Rust scan: `src/sim/game_entity.rs`, `src/sim/components.rs`, `src/sim/movement/movement_commands.rs`, `src/sim/movement/movement_tick.rs`.
- INI defaults: `ini/rulesmd.ini`, `ini/rules.ini` `[General] PathDelay=.01`, `BlockagePathDelay=60`.
