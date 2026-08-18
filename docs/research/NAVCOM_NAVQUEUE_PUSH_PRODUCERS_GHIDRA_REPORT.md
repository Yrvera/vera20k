# NavCom NavQueue Push Producers - Ghidra Research Report

**Address(es):** `0x004DB3C0`, `0x004D82B0`, `0x004D9290`, `0x004D9960`, `0x006E9050`, `0x006EB490`, `0x006EBAD0`, `0x004C6CB0`, `0x006DD8B0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** standard YR runtime writers/populators for `FootClass` NavQueue (`+0x588` vector, buffer `+0x58C`, active count `+0x598`) across TeamClass/AI movement, map/trigger waypoint actions, and player command routing.
**Non-Scope:** queue consumers beyond writer proof, full OnArrival tail hooks, full TechnoClass destination preprocessing, nonstandard editor/save corruption cases, and Rust implementation changes.
**Confidence:** High for "no standard runtime push producer found"; High for save-load population; Medium for historical source of nonzero saved counts.
**Active in YR:** Conditional. Runtime consumers are active; save-load population is active only when a save contains nonzero queue entries; no active standard runtime producer was found.

## 1. Overview

Target question: does any active standard YR path write/populate `FootClass` NavQueue at `+0x588/+0x598`?

Non-goals: do not reinvestigate queue pop/consumer behavior except to prove writer absence; do not decode all TeamClass script semantics; do not implement Rust.

Evidence needed to mark COMPLETE: direct binary coverage for all `+0x598` mutators plus decompile/caller evidence for TeamClass/AI, trigger waypoint, and player command paths.

Stop conditions: no unexamined direct `+0x58C/+0x598` writer remains in the bounded slice; all plausible command/script paths either route through `Set_Destination` or are explicitly deferred.

Result: no active standard YR runtime producer was found. The only positive population path is `FootClass::Load`, which reconstructs the queue from serialized save data. Normal player commands, TeamClass convoy/script movement, and trigger actions reissue or clear destinations through vtable `+0x480`; they do not append to NavQueue.

## 2. Class Layout / Key Offsets

| Offset | Type | Purpose | Evidence | Active in YR |
|---|---|---|---|---|
| `Foot+0x588` | `DynamicVectorClass<AbstractClass*>` header | NavQueue owner | constructor `0x004D31E0` initializes vector at `+0x588` | Yes, field exists on FootClass |
| `Foot+0x58C` | pointer buffer | queued destination entries | load/write/shift sites at `0x004DB3C0`, `0x004D82B0`, `0x004D9290`, `0x004D9960` | Conditional, only meaningful when count > 0 |
| `Foot+0x598` | int active count | queue count used by consumers | displacement scan and disassembly at `0x004D837C`, `0x004D92FB`, `0x004D9B97` | Conditional, usually zero in standard runtime |
| `Foot+0x59C` | int capacity | constructor/load set capacity 10 | decompile `0x004D31E0`, `0x004DB3C0` | Yes |
| `Foot+0x5A4` | `AbstractClass*` | current NavCom destination | settled prior work, used here only as guard/context | Yes |

## 3. Core Logic

The positive writer is save-load reconstruction:

1. `FootClass::Load @ 0x004DB3C0` resets the vector header, sets capacity to `10`, and sets `+0x598 = 0`.
2. It reads the serialized queue count, then for each serialized pointer it conditionally increments `+0x598` and stores the pointer into `*(+0x58C + index*4)`.
3. After load, it validates each loaded queue pointer through the pointer-fixup path (`FUN_006CF240`).

Active in YR: Conditional. This path is active during savegame load and can populate NavQueue if a save file contains nonzero entries. It does not prove any standard runtime producer.

Runtime mutators found in the FootClass queue slice are consumers or cleanup only:

| Function | Queue write | Meaning | Evidence | Active in YR |
|---|---|---|---|---|
| `FootClass::OnArrival @ 0x004D82B0` | decrements `+0x598`, shifts `+0x58C` left | FIFO pop after arrival | disassembly `0x004D837C..0x004D83CA` | Conditional, if count > 0 |
| `FootClass::Mission_Enter @ 0x004D9290` | decrements `+0x598`, shifts `+0x58C` left | consumes queued entry after accepted/preserved enter path when NavCom is null | disassembly `0x004D92ED..0x004D93CF` | Conditional |
| `FootClass::PointerExpired @ 0x004D9960` | decrements `+0x598`, shifts `+0x58C` left | removes expired pointer entries | disassembly `0x004D9B97..0x004D9BDD` | Conditional |

No runtime append/increment pattern comparable to `Load` was found outside save-load.

## 4. INI Keys

No INI key was found that directly enables or populates `FootClass` NavQueue. Map waypoints and TeamClass scripts can drive movement targets, but the verified binary paths route through team target fields and vtable `+0x480`, not the Foot NavQueue vector.

## 5. Integration Points

### TeamClass / AI movement sequencing

`TeamClass::Convoy_Script_Move_To_Cell @ 0x006EC7D0`, `TeamClass::Convoy_Script_Move @ 0x006ECCE0`, `TeamClass::Convoy_Script_Patrol @ 0x006ED090`, `TeamClass::Convoy_Script_Attack_Move @ 0x006EF700`, and `TeamClass::Convoy_Script_Random_Move @ 0x006EFA10` call `TeamClass::Set_Convoy_Target @ 0x006E9050` and then `Convoy_Move_*`.

`TeamClass::Set_Convoy_Target @ 0x006E9050` writes team-level target fields and calls each member vtable `+0x480` with null in reset cases. `TeamClass::Convoy_Move_With_Target @ 0x006EB490` and `TeamClass::Convoy_Move_Without_Target @ 0x006EBAD0` issue member vtable `+0x480(target, 1)` or `+0x480(0, 1)`. They do not reference `Foot+0x58C/+0x598`.

Active in YR: Conditional. Team scripts are active for AI/scripted teams in standard YR, but this queue-push mechanism is not active because these paths do not append to NavQueue.

### Map / trigger waypoint actions

`TriggerAction::Execute @ 0x006DD8B0` dispatches many map trigger actions and calls team/waypoint helpers such as `0x006E0AA0`, `0x006E0FE0`, `0x006E11C0`, `0x006E2050`, and related helpers. A displacement search for `+0x598` found no hit in the `0x006E****` trigger/team helper range. The visible `DynamicVectorClass__Add` in `TriggerAction::Execute` is for trigger/action bookkeeping, not `Foot+0x588`.

Active in YR: Yes for triggers generally; No for NavQueue push in this slice.

### Player command path

`EventClass::Execute @ 0x004C6CB0` contains active command routing that calls object vtable `+0x480` for movement/stop-like cases and writes the global network command queue. It does not reference `Foot+0x58C/+0x598` and does not append to `FootClass` NavQueue.

Active in YR: Yes for player/network command dispatch; No for NavQueue push in this slice.

## 6. Current Rust Implementation Status

Rust currently has an explicit `NavigationState` with `nav_com`, `suspended_nav_com`, and `nav_queue` in `src/sim/components.rs:298` (corrected 2026-05-28: was :288; Rust scan shows struct at line 298 — STALE_LINE_NUMBER). `GameEntity` owns it in `src/sim/game_entity.rs:184` (corrected 2026-05-28: was :118; Rust scan shows `pub navigation: NavigationState` at line 184 — STALE_LINE_NUMBER).

`src/sim/movement/movement_commands.rs` no longer appends to `navigation.nav_queue` on a queued move; the append code has been removed and only `nav_queue.clear()` calls remain (corrected 2026-05-28: was "392-407 appends to nav_queue"; Rust scan of movement_commands.rs finds only two `nav_queue.clear()` calls at lines 85 and 556, no push/append — STALE). `src/app_target_lines.rs:195-204` still prefers the last queue entry over `nav_com` for selected action-line endpoints (CONFIRMED).

The nav_queue append has already been removed from the Rust codebase. The action-line reader at `app_target_lines.rs:195-204` remains and correctly handles a non-empty queue (e.g. from save-load).

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Direct `+0x598` displacement scan | verified | `search_byte_patterns "98 05 00 00"`; Foot-related hits grouped under `0x004D31E0`, `0x004DB3C0`, `0x004DB690`, `0x004D82B0`, `0x004D9290`, `0x004D9960`, `0x004B0500`, `0x0069FC10`, action-line readers | none for bounded slice |
| `FootClass::Load` queue population | verified | decompile/disassembly `0x004DB3C0`; write/increment at `0x004DB3C0` body, validation at `0x004DB64F..0x004DB678` | source of nonzero saved count remains historical/adjacent |
| `FootClass::OnArrival` queue pop | verified | `0x004D837C..0x004D83CA` | consumer details beyond writer proof out-of-scope |
| `FootClass::Mission_Enter` queue pop | verified | `0x004D92ED..0x004D93CF` | consumer details beyond writer proof out-of-scope |
| `FootClass::PointerExpired` queue cleanup | verified | `0x004D9B97..0x004D9BDD` | none |
| TeamClass script movement | verified | decompile `0x006E9050`, `0x006EB490`, `0x006EBAD0`, script callers from `get_function_callers TeamClass__Set_Convoy_Target` | exact semantics of each script action are out-of-scope |
| Trigger waypoint/action path | verified-negative | decompile `0x006DD8B0`; no `+0x598` hits in `0x006E****` helper range from displacement scan | exact helper labels remain mostly unnamed |
| Player command path | verified-negative | decompile `0x004C6CB0`; no `+0x58C/+0x598` references, direct vtable `+0x480` calls | exact command case labels not all decoded |
| Current Rust queue append | corrected 2026-05-28 | nav_queue append removed; only `nav_queue.clear()` at movement_commands.rs:85,556; action-line reader at `src/app_target_lines.rs:195-204` confirmed (was: `movement_commands.rs:392-407` appends — STALE) | none — append already gone |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Are there direct runtime writers that increment `Foot+0x598` and store into `Foot+0x58C`? -> Only `FootClass::Load` shows the append/increment/store pattern; runtime Foot sites decrement/shift or read.` (evidence: `0x004DB3C0`, `0x004D82B0`, `0x004D9290`, `0x004D9960`)
- `[RESOLVED] OQ-02 - Do TeamClass movement scripts push queued destinations? -> No; they set team targets and issue member vtable `+0x480` destinations.` (evidence: `0x006E9050`, `0x006EB490`, `0x006EBAD0`, callers of `TeamClass__Set_Convoy_Target`)
- `[RESOLVED] OQ-03 - Do map trigger waypoint actions push `Foot+0x588`? -> No writer was found; trigger dispatch helpers do not hit `+0x598`, and visible vector add is trigger bookkeeping.` (evidence: `0x006DD8B0`, displacement scan)
- `[RESOLVED] OQ-04 - Do player commands append waypoints? -> No; command execution calls vtable `+0x480` and global command queue logic, not `Foot` NavQueue.` (evidence: `0x004C6CB0`)
- `[RESOLVED] OQ-05 - Is save/load a producer? -> Save writes count/entries; Load reconstructs count/entries and validates pointers. It is a persistence producer, not a gameplay command producer.` (evidence: `0x004DB3C0`, `0x004DB690`)
- `[RESOLVED] OQ-06 - Is this standard YR active or TS legacy? -> Consumers and save-load are active in standard YR binary; runtime push producer is not found on standard player/team/trigger paths.` (evidence: addresses above)
- `[DEFERRED] OQ-07 - What historical path could create a nonzero queue before save?` (category: `bounded-cost-too-high`; reason: no standard runtime producer found in this slice; could require TS-era legacy, editor-only, or obsolete command code archaeology; next-step-if-pursued: run runtime watchpoint on `Foot+0x598` while exercising campaign scripts and legacy waypoint commands)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Standard player/team/trigger movement does not push `Foot` NavQueue; it reissues `Set_Destination` through vtable `+0x480`. | `0x004C6CB0`, `0x006E9050`, `0x006EB490`, `0x006EBAD0`, `0x006DD8B0` | resolved: nav_queue append has been removed from Rust (corrected 2026-05-28: was "mismatch: Rust appends navigation.nav_queue on queued move at movement_commands.rs:392-407"; Rust scan shows only clear() calls remain — STALE) | `src/sim/world/world_commands.rs`; `src/app_context_order.rs` | Normal queued/shift move should replace or reissue destination according to command semantics, not append to `navigation.nav_queue`. | Select moving unit, issue Shift+Move to a second cell; `nav_queue` remains empty and `nav_com`/movement target reflect the binary-equivalent current destination. Proposed test: `shift_move_replaces_destination_without_navqueue_append` | Do not re-introduce a nav_queue append; binary evidence does not support it. |
| TeamClass AI script movement sequences targets through team fields and member `Set_Destination`, not `Foot+0x58C`. | `0x006EC7D0`, `0x006ECCE0`, `0x006ED090`, `0x006EF700`, `0x006EFA10`, `0x006E9050` | unchecked/mismatch risk if future AI ports reuse `nav_queue` for patrol scripts | future AI/team script movement surfaces | Implement AI waypoints as repeated destination issues from the team/script state owner, not as Foot NavQueue entries. | Scripted team move-to-cell followed by patrol step issues member destination and leaves each member `navigation.nav_queue` empty. Proposed test: `team_script_move_does_not_populate_foot_navqueue` | Do not model TeamClass script lists as per-foot NavQueue. |
| Save-load may reconstruct nonzero NavQueue entries, and consumers/action lines must tolerate them. | `0x004DB3C0`, `0x004DB690`, `0x004D82B0`, `0x004DC060`, `0x004DC340` | partial: Rust has `nav_queue` and action-line priority, but runtime producer should be removed/narrowed | `src/sim/components.rs`; `src/app_target_lines.rs`; future save import | Keep the data shape for deserialization/consumer compatibility, but do not create entries from standard runtime commands. | Synthetic loaded entity with `nav_com` plus two `nav_queue` entries uses last queue entry for action line and FIFO consumer code can pop without normal commands adding entries. Proposed test: `loaded_navqueue_action_line_uses_last_entry_without_runtime_append` | Do not delete `nav_queue` entirely; consumers and save-load evidence exist. |

### Negative Facts / Do Not Do

- Do not implement shift-click waypoint chaining by appending to `Foot+0x58C/+0x598`; `EventClass::Execute @ 0x004C6CB0` issues vtable `+0x480` and has no queue-field reference. Active in YR: Yes.
- Do not implement TeamClass patrol/move scripts as per-foot NavQueue pushes; TeamClass scripts call `Set_Convoy_Target` and member `+0x480`, with no `+0x598` reference. Active in YR: Conditional AI/script path.
- Do not treat `TriggerAction::Execute`'s `DynamicVectorClass__Add` as Foot NavQueue evidence; the add occurs in trigger bookkeeping and the `0x006E****` trigger helper range has no `+0x598` hits. Active in YR: Yes for triggers, No for NavQueue push.
- Do not remove NavQueue storage/readers completely; `FootClass::Load`, `Save`, `OnArrival`, `Mission_Enter`, `PointerExpired`, and action-line readers prove the field exists and can matter if nonzero. Active in YR: Conditional.
- Do not infer a producer from consumers. `OnArrival`, `Mission_Enter`, and `PointerExpired` only decrement/shift existing entries. Active in YR: Conditional.

### Stale Docs / Follow-up Docs

- `docs/research/FOOTCLASS_ENTER_QUEUE_AND_NAVCOM_SYSTEM.md`: replace "This queue stores **waypoints/intermediate destinations**. When a unit arrives at a NavCom destination, OnArrival pops from this queue to set the next destination." with "The queue is serialized, deserialized, consumed, and cleaned, but this investigation found no standard YR runtime producer in player, TeamClass/AI, or trigger waypoint command paths. Treat nonzero entries as save/load or legacy/unknown state until a producer is separately verified."
- `docs/research/NAVCOM_LIFECYCLE_GHIDRA_REPORT.md`: replace section 6.4 hypothesis wording with "Follow-up `NAVCOM_NAVQUEUE_PUSH_PRODUCERS_GHIDRA_REPORT.md` verified no standard YR player command, TeamClass/AI movement, or trigger waypoint path pushes `Foot+0x588/+0x598`; the only positive population path found is `FootClass::Load @ 0x004DB3C0` reconstructing serialized entries."

## Sources

- Ghidra read-only decompile/disassembly: `FootClass::Constructor @ 0x004D31E0`; `FootClass::Load @ 0x004DB3C0`; `FootClass::Save @ 0x004DB690`; `FootClass::OnArrival @ 0x004D82B0`; `FootClass::Mission_Enter @ 0x004D9290`; `FootClass::PointerExpired @ 0x004D9960`; `DriveLocomotionClass::Process @ 0x004B0500`; `ShipLocomotionClass::Process @ 0x0069FC10`; `TechnoClass::DrawActionLines @ 0x004DC060`; `TechnoClass::DrawRadarActionLines @ 0x004DC340`.
- Ghidra read-only decompile/callers: `TeamClass::Set_Convoy_Target @ 0x006E9050`; `TeamClass::Convoy_Move_With_Target @ 0x006EB490`; `TeamClass::Convoy_Move_Without_Target @ 0x006EBAD0`; `TeamClass::Convoy_Script_Move_To_Cell @ 0x006EC7D0`; `TeamClass::Convoy_Script_Move @ 0x006ECCE0`; `TeamClass::Convoy_Script_Patrol @ 0x006ED090`; `TeamClass::Convoy_Script_Attack_Move @ 0x006EF700`; `TeamClass::Convoy_Script_Random_Move @ 0x006EFA10`.
- Ghidra read-only decompile: `EventClass::Execute @ 0x004C6CB0`; `TriggerAction::Execute @ 0x006DD8B0`; `WaypointPathClass` constructors `0x00763730`, `0x00763810`.
- Existing docs referenced: `docs/research/NAVCOM_LIFECYCLE_GHIDRA_REPORT.md`; `docs/research/FOOTCLASS_ENTER_QUEUE_AND_NAVCOM_SYSTEM.md`; `docs/research/NAVCOM_NAVQUEUE_ACTION_LINE_ENDPOINT_VISIBILITY_GHIDRA_REPORT.md`; `docs/research/UNIT_MISSION_ENTER_REFINERY_RETRY_QUEUE_LOOP_GHIDRA_REPORT.md`.
- Rust scan: `src/sim/components.rs`; `src/sim/game_entity.rs`; `src/sim/movement/navcom.rs`; `src/sim/movement/movement_commands.rs`; `src/app_target_lines.rs`; `src/app_context_order.rs`; `src/sim/world/world_commands.rs`.
