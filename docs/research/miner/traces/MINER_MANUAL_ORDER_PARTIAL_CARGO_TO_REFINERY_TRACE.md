# Manual Chrono Miner Order To Friendly Refinery With Partial Cargo Trace

**Scenario:** Player selects one standard YR Chrono Miner (`CMIN`) with partial cargo and right-clicks an owned Allied Ore Refinery (`GAREFN`).

**Scope:** Cursor/action acceptance, command routing, miner FSM transition, refinery reservation, and first dock/queue movement only. Adjacent miner return/deposit/exit behavior is not traced here.

**Report date:** 2026-05-20  
**Subagent slot:** 4  
**Output contract:** Detection only. No Rust, INI, or repo-doc edits.

## Sources

- Retail rules data: `ini/rulesmd.ini` (`[CMIN]`, `[GAREFN]`) and `ini/artmd.ini` (`[GAREFN]`).
- Rust code: `src/app_cursor.rs`, `src/app_context_order.rs`, `src/sim/command.rs`, `src/sim/world/world_commands.rs`, `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs`.
- Existing research: `DETERMINE_ACTION_DOWNSTREAM_GHIDRA_REPORT.md`, `HARVESTER_DOCK_UNLOAD.md`, `MISSION_HARVEST_GHIDRA_REPORT.md`, `REFINERY_RADIO_DOCKING_ACCEPTANCE_QUEUE_GHIDRA_REPORT.md`, `CHRONO_MINER_SYSTEM_OVERVIEW.md`, `SOUND_TRIGGERS_COMPLETE_GHIDRA_REPORT.md`, `units/allied/CMIN.md`.
- Live read-only Ghidra checks in this run:
  - `UnitClass__What_Action_OnObject` at `0x0073FD50`.
  - `TechnoClass__What_Action_OnObject` at `0x006FFEC0`.
  - `FootClass__ClickedAction_Object` at `0x004D74E0`.

No mutating Ghidra operation was used.

## Pipeline

```text
Hover refinery with CMIN selected
  -> cursor feedback / What_Action object path
  -> right-click object action dispatch
  -> command payload queued
  -> sim command execution
  -> miner ForcedReturn/return state
  -> refinery selection/reservation
  -> first queue/pad movement
```

## Stage Results

| Stage | Boundary Checked | Our Output | gamemd Evidence | Verdict |
|---:|---|---|---|---|
| 1 | Scenario data: `CMIN` is harvester, `GAREFN` is refinery/dock target | Rules files contain `CMIN Dock=NAREFN,GAREFN`, `Harvester=yes`, `Storage=20`; `GAREFN DockUnload=yes`, `Refinery=yes`, `FreeUnit=CMIN`, `Storage=200` | Same retail INI values feed gamemd, but I did not inspect loaded runtime type fields for this exact session | UNCHECKED |
| 2 | Hover acceptance over friendly refinery | `app_cursor.rs:340-346` returns `CursorFeedbackKind::Enter` when selected entity has `miner` and hovered friendly structure is a refinery | `DETERMINE_ACTION_DOWNSTREAM_GHIDRA_REPORT.md` maps action `0x1A` to dock/enter; live `UnitClass__What_Action_OnObject` routes human friendly building dock/enter checks through radio `0x0F` | UNCHECKED |
| 3 | Cursor numeric frame / cursor SHP id | `CursorFeedbackKind::Enter` maps to `CursorId::Enter` in `app_cursor.rs:523-525` | I did not compute the exact gamemd cursor frame index for action `0x1A` in this run | UNCHECKED |
| 4 | Right-click object action keeps clicked refinery as target | `app_context_order.rs:124-132` queues `Command::MinerReturn { entity_id }`; the clicked refinery id is discarded | Live `FootClass__ClickedAction_Object` case `0x1A` calls vtable `+0x378` with the original `param_3` target object still passed as the order target | FAIL |
| 5 | Command payload representation | `Command::MinerReturn` has only `entity_id` (`src/sim/command.rs:99-101`) | gamemd object-click action carries the clicked object into the unit order path; no evidence that it degrades to "any refinery" at click time | FAIL |
| 6 | Immediate order voice | `app_context_order.rs:721-725` emits `VoiceMove` for non-attack orders; `emit_order_voice` only supports `VoiceMove`, `VoiceAttack`, and special attack paths | `CMIN` has distinct `VoiceMove=ChronoMinerMove` and `VoiceEnter=ChronoMinerReturn`; existing CMIN docs say `VoiceEnter` is the unique refinery-return set. I did not re-decompile the exact SelectClass voice branch for object action `0x1A` | UNCHECKED |
| 7 | Sim command acceptance | `world_commands.rs:637-659` verifies owner/deployed/miner, then sets `forced_return=true`, `state=ForcedReturn`, and clears movement | gamemd object action dispatches a target-object order. Exact mission code/name after vtable `+0x378` was not fully decoded here | UNCHECKED |
| 8 | ForcedReturn refinery choice | `handle_forced_return` calls `find_nearest_refinery` when no reservation exists (`miner_system.rs:688-735`) | gamemd click path has the clicked refinery object available at order dispatch. Whether later mission logic may retarget under specific invalidation conditions was not fully traced | FAIL |
| 9 | Partial cargo eligibility | Our path does not require full cargo; a partial-cargo CMIN can enter `ForcedReturn` | I did not find a gamemd cargo-amount gate in the live object-click path, but did not complete a targeted storage check for partial cargo | UNCHECKED |
| 10 | Reservation handshake | Our reservation is acquired in `phase_approach` via `DockReservations::try_reserve` only after `MinerState::Dock` (`miner_dock_sequence.rs:404-426`) | Existing `REFINERY_RADIO_DOCKING_ACCEPTANCE_QUEUE_GHIDRA_REPORT.md` verifies active YR HELLO/CAN_DOCK radio stack, Contacts capacity, and CAN_DOCK queue-cell/pad command; exact tick equality was not computed | UNCHECKED |
| 11 | First queue movement if dock busy | Our busy dock path sets `dock_queued=true` and moves/continues toward `QueueingCell` (`miner_dock_sequence.rs:426-433`) | gamemd standard refinery has `NumberOfDocks=1`; HELLO accepts/rejects based on Contacts capacity and CAN_DOCK later issues move-to queue/dock commands. Exact same-tick movement order was not computed | UNCHECKED |
| 12 | First pad movement if dock free | Our free dock path immediately targets pad cell and transitions `Approach -> Linked` (`miner_dock_sequence.rs:416-424`) | gamemd CAN_DOCK accepted path sends MOVE_TO_CELL/ENTER_DOCK; exact pad/queue coordinate equality for this clicked refinery instance was not computed | UNCHECKED |

## Failures

### F1 — Clicked Refinery Target Is Lost

**Stage:** 4-5  
**Player-visible difference:** If the player right-clicks a specific owned refinery while another valid friendly refinery is nearer to the Chrono Miner, VERA20k can send the miner to the nearer refinery instead of the clicked one. This makes a direct player order feel ignored in multi-refinery bases.

**Our code:** `src/app_context_order.rs:124-132` creates `Command::MinerReturn { entity_id }`; `src/sim/command.rs:99-101` has no target refinery field.  
**gamemd evidence:** Live read-only decompile of `FootClass__ClickedAction_Object` (`0x004D74E0`) shows case `0x1A` retaining the object-click `param_3` and passing it to the unit order vtable call. `DETERMINE_ACTION_DOWNSTREAM_GHIDRA_REPORT.md` identifies action `0x1A` as the dock/enter action. This path is active in standard YR object-click command routing.

### F2 — ForcedReturn Re-chooses Nearest Refinery Instead Of Using The Ordered Object

**Stage:** 8  
**Player-visible difference:** The first dock/queue movement can go toward a different refinery than the one the player clicked. This is common once the player has expanded or captured a second refinery.

**Our code:** `src/sim/miner/miner_system.rs:688-735` calls `find_nearest_refinery` for `ForcedReturn`; `src/sim/miner/miner_system.rs:993-1039` scores by squared distance to the refinery queue cell.  
**gamemd evidence:** The object-click dispatch has an explicit target object pointer for action `0x1A`. Existing radio/refinery docs confirm the refinery-side dock protocol is active in YR, not dormant TS legacy.

## Unchecked Risks

- Exact cursor frame parity for action `0x1A` was not computed. Our semantic cursor is `Enter`, but no numeric cursor-frame equality was established.
- Exact immediate voice response for this object action was not re-decompiled. Our current UI emits `VoiceMove`; CMIN has a distinct `VoiceEnter=ChronoMinerReturn`, so this is suspicious but remains UNCHECKED in this trace.
- Exact tick count from click to HELLO/CAN_DOCK and from CAN_DOCK to first movement was not computed on both engines.
- Exact queue/pad cell equality was not computed for a concrete refinery anchor coordinate because the scenario did not provide map coordinates.

## Adjacent Findings

- Existing traces already cover Chrono Miner return teleport visuals, dock pivot, unloading, and exit behavior. Those are intentionally not re-traced here.
- The target-loss bug affects other direct object orders if their command payloads similarly collapse an explicit clicked object into a generic intent; this trace only checked `CMIN -> friendly refinery`.

## Verdict Tally

PASS: 0 | FAIL: 3 | UNCHECKED: 9 | NOT-IMPLEMENTED: 0

## Status

COMPLETE
