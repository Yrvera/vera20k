# Chrono Miner Locomotion Architecture System Model Synthesis

**Date:** 2026-05-25  
**Investigation Mode:** model-synthesis with conflict-map sections  
**Scope:** stock YR `[CMIN]` locomotion across ore approach, full-cargo return, far-return teleport, close refinery approach, dock radio/unload handoff, post-unload return to harvest, and current Rust architecture risks.  
**Non-Scope:** fresh live Ghidra decompilation, runtime debugger watch, implementation patches, exact render/audio pixel proof.  
**Model Safety:** Partially implementation-safe. The broad harvest/return skeleton is safe; dock `0x16` facing equivalence and full active-locomotor piggyback ownership remain investigation-blocked.  
**Spot-check Status:** Ghidra MCP had no running instance on 2026-05-25, so disputed slices were not live-spot-checked.

## 1. Current Model

Stock CMIN is a normal `UnitClass` harvester with `Harvester=yes`, `Teleporter=yes`, `Dock=NAREFN,GAREFN`, and `ROT=5`. Its primary locomotor is TeleportLocomotion. Drive movement is not a special miner state; it is obtained by active-locomotor swapping through the IPiggyback mechanism.

The high-level loop is:

1. `Mission_Harvest` state 0 searches ore.
2. Ore approach drives, not warps, in the currently best model. A Drive locomotor is active/piggybacked for normal ore pathing.
3. State 1 harvests bales until full.
4. State 2 finds a refinery and compares 3D object-coordinate lepton distance against `ChronoHarvTooFarDistance * 256`; stock threshold is `50 * 256`.
5. If close, state 2 sends HELLO/radio `0x02` to the refinery and advances to the later mission-enter/radio path only on accepted reply.
6. If far or refused, state 2 computes a fallback staging seed from refinery art `QueueingCell=4,1`, validates a nearby passable cell, and calls `Set_Destination` to that empty cell. Because the target cell is empty and the active locomotor is Teleport, the Teleporter decision skips Drive piggyback and the miner warps.
7. After warp relocation, the miner is at the staging/near-refinery cell. In gamemd the active locomotor lifecycle then allows the unit to drive the final dock approach.
8. Mission Enter sends `CAN_DOCK(0x0E)`; stock refinery acceptance sends accepted cell `building_anchor + (3,1)`, not `QueueingCell`.
9. After accepted-cell arrival, the building-side sequence sends `0x18` then `0x16`. Unit `0x16` first calls active locomotor `+0x4C(0x4000)` and returns; a later/already-synced `0x16` can send `0x15` to queue deploy/unload.
10. Unload runs through `Mission_Deploy_Building`; stock zero-link completion does not normally call `ReleaseDockedHarvester` or `Force_Track(0x47)`.
11. State returns to harvest scheduling; active locomotor restoration must follow IPiggyback `Is_Ok_To_End`/`End_Piggyback` conditions, not a miner-local assumption.

## 2. Claim Table

| Claim | Best Evidence | Status | Safe? |
|---|---|---|---|
| Stock CMIN return threshold is `ChronoHarvTooFarDistance=50` cells, 3D lepton distance, inclusive close branch | `CMIN_CLOSE_FAR_RETURN_SPLIT...`; `ini/rulesmd.ini:294` | confirmed | IMPLEMENTATION_SAFE for stock |
| Far/refused return uses `QueueingCell=4,1` as staging seed; accepted `CAN_DOCK` uses anchor `+(3,1)` | close/far split report; `BuildingClass::Receive_Radio 0x0E`; `ini/artmd.ini:1716,1773` | confirmed | IMPLEMENTATION_SAFE |
| Normal stock unload completion does not use `Force_Track(0x47)` | zero-link state-4 reports and conflict audits | confirmed | IMPLEMENTATION_SAFE |
| Stock CMIN effective `ROT=` remains `5`; `Harvester=yes` writes a separate `+0x398=10` field | `CMIN_RUNTIME_ROT_AFTER_PARSER_OVERRIDES...`; `ini/rulesmd.ini:7364,7378` | confirmed | IMPLEMENTATION_SAFE |
| CMIN ore approach drives rather than warps | newer teleport/system/ore-acquisition reports | confirmed enough for current implementation, but older traces conflict | DOC_PATCH_READY / verify if disputed |
| Active Drive/Teleport piggyback lifecycle is represented exactly by current Rust | system overview vs current `LocomotorState::override_state` | contradicted | NEEDS_REINVESTIGATE before broad patch |
| Dock radio `0x16` is directly an East body-facing pivot | current Rust/comments vs audit/conflict docs | disputed / unproven | NEEDS_REINVESTIGATE |
| Rust far-return fallback nearby-cell search is exact | fallback search report vs current `EXIT_SEARCH_MAX_RADIUS=16` and helper shape | contradicted | IMPLEMENTATION_DELTA |
| Rust unload lookup by `reserved_refinery` is exact | adjacent-cell lookup report vs current dock sequence | contradicted | IMPLEMENTATION_DELTA |

## 3. Implementation-Safe Facts

- Keep `ChronoHarvTooFarDistance` data-driven and use strict `>` for the far branch; exact-threshold stock CMIN is close, not far.
- Keep `QueueingCell` and accepted `CAN_DOCK` cell separate. `QueueingCell=4,1` is fallback/wait staging; accepted stock refinery cell is anchor `+(3,1)`.
- Keep normal stock unload completion out of `Force_Track(0x47)`.
- Treat `ROT=5` as the stock CMIN/HARV runtime facing/DriveLocomotion `ROT=` source. Do not keep the Rust `Harvester=yes => turret_rot=10` override if `turret_rot` represents the parsed ROT/facing field.

## 4. Architectural Errors In Current Rust

### A. Locomotor Ownership Is Inverted

Rust uses `LocomotorState::override_state` as a temporary override where Teleport is applied over a base locomotor and then restored. The binary model is the opposite for CMIN: Teleport is the primary locomotor, and Drive is piggybacked/activated when a destination requires ground movement. This matters because `Set_Destination`, `Head_To_Coord`, `Is_Ok_To_End`, and restore timing all depend on which locomotor is active.

Current surfaces: `src/sim/movement/locomotor.rs`, `src/sim/movement/teleport_movement.rs`, `src/sim/miner/miner_system.rs`.

### B. Teleport Is Issued As A Miner Helper, Not As The Result Of `Set_Destination`

`try_issue_chrono_far_return_teleport` directly calls `issue_teleport_command`. In gamemd, state 2 computes a target and calls `TechnoClass::Set_Destination`; the Teleporter block decides whether the active locomotor remains Teleport or swaps to Drive based on destination-cell contents. Rust can match the visible far warp in common cases but bypasses the mechanism that also governs player orders, building-cell clicks, restore state, and later movement ownership.

Current surfaces: `src/sim/miner/miner_system.rs:971..1017`, `src/sim/movement/teleport_movement.rs`.

### C. Dock `0x16` Is Modeled In Miner State Instead Of Locomotor State

Rust has `FaceSync`, `Pivoting`, `dock_pivot_facing`, `sync_dock_facing`, and explicit `entity.facing = DOCK_FACING_EAST`. The confirmed handler does not directly assign facing; it calls active locomotor `+0x4C(0x4000)` / `RateTimer` and later sends `0x15` under gates. Existing docs conflict on whether `Do_Turn(0x4000)` is body-facing-equivalent East, but either way Rust is bypassing the active locomotor field ownership.

Current surfaces: `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/mod.rs`.

### D. Parser Data Model Merges Two Binary Fields

Rust `ObjectType::turret_rot` currently gets overwritten to `10` for `Harvester=yes`. Verified binary evidence says parsed `ROT=5` remains the facing/DriveLocomotion consumer field; the harvester override writes a different UnitType field. This is an architectural data-model error because every downstream movement/facing consumer is now reading the wrong source.

Current surface: `src/rules/object_type.rs`.

### E. Refinery Docking Stores Too Much In Miner-Local Identity

Rust threads `reserved_refinery` through dock/unload. Gamemd uses radio/contact state and, in stock zero-link unload state 3/4, rediscovers the refinery from the miner's current cell plus `(-1,0)`. A stable `reserved_refinery` shortcut will diverge when contact identity and adjacent building identity differ, or when the building is removed/replaced mid-unload.

Current surfaces: `src/sim/miner/mod.rs`, `src/sim/miner/miner_dock_sequence.rs`.

### F. Nearby-Cell Search Is A Simplified Helper

Rust uses radius 16 and helper variants that do not fully encode the verified `Find_Nearby_Passable_Cell` caller contract: effective cap normally 32, 24-candidate cap, direct/indirect preference, and frame modulo. This is both a behavior mismatch and an abstraction risk because the helper looks generic but is not parameterized enough for exact gamemd callers.

Current surface: `src/sim/miner/miner_dock_sequence.rs`.

### G. Teleport Side Effects Are Not Owned By The Locomotor State Machine

Rust teleport relocation handles occupancy and a debug lifecycle. Binary reports include stop-targeting, attached anim detach, chrono sounds/anims, mission reset/guard-area behavior, bridge flags, and being-warped draw flags. Some effects may be implemented elsewhere, but the architecture does not clearly make TeleportLocomotion the owner of the full phase-0 side-effect bundle.

Current surface: `src/sim/movement/teleport_movement.rs`.

## 5. Cross-Doc Conflicts

- Dock `0x16`: one report says `+0x4C(0x4000)` is effectively East-facing `Head_To`; another says treating `0x4000` as facing is drift and only a RateTimer sync is proven. This requires fresh Ghidra on `DriveLocomotionClass::Do_Turn @ 0x004B0EF0`.
- Ore approach: older traces claim CMIN warps to ore; newer system/teleport/ore-acquisition evidence supports Drive. Current Rust drives to ore. Re-investigate only if this remains contested.
- Older ForceTrack exit docs are superseded for normal stock zero-link refinery unload; keep their `ReleaseDockedHarvester` details only for conditional reciprocal-link paths.

## 6. Recommended Fix Architecture

Do not fix chrono miner locomotion by adding more one-off miner phases. The cleaner parity path is:

1. Correct data source first: split parsed `ROT=` from the separate harvester/weeder `+0x398` field.
2. Add an explicit active-locomotor/piggyback model: primary Teleport, active Drive when piggybacked, and exact `Is_Ok_To_End` restore gates.
3. Route CMIN movement orders through a gamemd-like `Set_Destination` decision instead of direct miner-owned teleport/drive helpers.
4. Move dock `Do_Turn(0x4000)` semantics into locomotor/facing state once `DriveLocomotionClass::Do_Turn` is freshly resolved.
5. Rework dock/unload to depend on radio/contact/cell rediscovery where gamemd does, not only saved `reserved_refinery`.
6. Replace the nearby-cell helper with a parameterized binary-shaped implementation usable by each verified caller.

## 7. Needs Re-Investigation

- `/re-investigate DriveLocomotionClass Do_Turn 0x004B0EF0 RateTimer 0x4000 dock radio 0x16`
- `/re-investigate chrono miner normal Drive Teleport piggyback restore lifecycle`
- `/re-investigate chrono miner Set_Destination teleport-vs-drive caller surfaces player move ore return dock`

## 8. Do-Not-Implement Notes

- Do not implement CMIN as "Drive base locomotor plus temporary Teleport override" if pursuing exact parity.
- Do not direct-call teleport from miner state as the final architecture; make it the result of active TeleportLocomotion receiving `Head_To_Coord`.
- Do not preserve `Harvester=yes => ROT=10` as a facing/locomotor fact.
- Do not treat `QueueingCell` as the accepted dock cell.
- Do not treat `0x16` as proof of direct body-facing assignment until `Do_Turn` field writes are verified.

## 9. Source Ledger

- `docs/research/miner/CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`
- `docs/research/miner/CHRONO_MINER_SYSTEM_OVERVIEW.md`
- `docs/research/miner/CMIN_CLOSE_FAR_RETURN_SPLIT_CHRONOHARVTOOFARDISTANCE_GHIDRA_REPORT.md`
- `docs/research/miner/CMIN_RUNTIME_ROT_AFTER_PARSER_OVERRIDES_GHIDRA_REPORT.md`
- `docs/research/miner/DOCK_RADIO_0X16_FACING_CONFLICT_AUDIT_20260525.md`
- `docs/research/miner/DOCK_ARRIVAL_PIVOT_SEQUENCE_DOC_CONFLICT_AUDIT_GHIDRA_REPORT.md`
- `docs/research/miner/CHRONO_MINER_LOCOMOTION_DISCREPANCY_MAP_20260525.md`
- `docs/research/miner/CHRONO_MINER_LOCOMOTION_DOC_REPO_DISAGREEMENTS_SYSTEM_MODEL_SYNTHESIS.md`
- `docs/research/miner/FIND_NEARBY_PASSABLE_CELL_FALLBACK_SEARCH_GHIDRA_REPORT.md`
- `docs/research/miner/DAT_0089F6A0_REFINERY_LOOKUP_OFFSET_SOURCE_GHIDRA_REPORT.md`
- `ini/rulesmd.ini`, `ini/artmd.ini`
- Current Rust surfaces: `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/mod.rs`, `src/sim/movement/locomotor.rs`, `src/sim/movement/teleport_movement.rs`, `src/rules/object_type.rs`

