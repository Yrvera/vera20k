# Chrono Miner Forced Return / Unload Command Movement-State Trace

**Scenario:** loaded Allied Chrono Miner (`CMIN`) receives a forced return/unload command to its own Allied refinery (`GAREFN`).

**Concrete coordinates traced:** `CMIN` at cell `(40,40)`, `GAREFN` anchor at `(10,10)`, standard YR 4x3 refinery. This matches the repo's existing forced-return chrono test setup and gives a concrete retail distance comparison.

**Scope:** command acceptance -> miner movement state -> teleport-vs-drive decision -> arrival coordinate -> dock/unload entry. Adjacent sound, post-unload departure, ore search, and render-only effects are out of scope.

**Sources used:** live read-only Ghidra decompile of `FootClass__ClickedAction_Object`, `UnitClass__What_Action_OnObject`, `UnitClass__Mission_Harvest`, `TechnoClass__Set_Destination`, `UnitClass__PerCellProcess`, `UnitClass__Receive_Radio`, `UnitClass__Mission_Deploy_Building`, `BuildingClass__ReleaseDockedHarvester`, `FootClass__Find_Docking_Bay`; retail `ini/rulesmd.ini`, `ini/artmd.ini`; Rust files under `src/sim` and `src/app_context_order.rs`.

> **Repo-status supersession 2026-05-25:** Findings that depend on current Rust
> using a hardcoded `CHRONO_INBOUND_WARP_THRESHOLD_CELLS = 2` are stale. Current
> Rust uses parsed `ChronoHarvTooFarDistance` for this close/far decision. Other
> command, dock-entry, and radio/pivot findings were not re-run here.

## Pipeline

Player right-click / forced unload on friendly refinery -> command carries refinery target -> miner return FSM -> retail Mission_Harvest return-state distance/radio branch vs VERA20k direct chrono-return helper -> arrival / dock contact -> dock link -> unload mission / dump loop.

## Stage Table

| Stage | Boundary Checked | VERA20k Output | gamemd.exe Output | Verdict |
|---|---|---|---|---|
| 1 | Scenario data | `CMIN` has miner component; `GAREFN` is accepted as explicit `target_refinery_id` | Retail INI has `CMIN Harvester=yes`, `Teleporter=yes`, `Storage=20`, `Dock=NAREFN,GAREFN`; `GAREFN DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`; active paths read these type flags | UNCHECKED |
| 2 | Object command preserves clicked refinery | `app_context_order.rs:128-137` emits `Command::MinerReturn { entity_id, target_refinery_id: Some(clicked_refinery) }`; `world_commands.rs:651-688` stores it as `reserved_refinery` | `FootClass__ClickedAction_Object` case `0x1A` calls vtable `+0x378` with the clicked object pointer still present | UNCHECKED |
| 3 | Command application timing | Command phase sets `forced_return=true`, `state=ForcedReturn`, clears movement target at `world_commands.rs:694-697`; miner FSM runs later in production phase | Exact mission queue timing for the `0x1A -> 0x0B` order path was not fully decoded | UNCHECKED |
| 4 | Teleport-vs-drive numeric threshold | `try_issue_chrono_return_teleport` uses hardcoded `CHRONO_INBOUND_WARP_THRESHOLD_CELLS = 2`, compares `(40,40)` to refinery center `(12,11)`: `28^2 + 29^2 = 1625 > 4`, so it issues teleport | `UnitClass__Mission_Harvest` state 2 compares distance to dock-offset/passable cell against `ChronoHarvTooFarDistance * 256 = 50 * 256 = 12800`; for dock offset `(14,11)`, distance is `sqrt(26^2+29^2)*256 ~= 9971 <= 12800`, so it takes the close/refinery radio path, not teleport | FAIL |
| 5 | Whether teleport is issued immediately | Next miner FSM tick calls `issue_teleport_command` directly from `handle_forced_return -> handle_return -> try_issue_chrono_return_teleport` | For this coordinate, no teleport command is issued; Mission_Harvest state 2 first performs dock lookup, movement/distance checks, then radio/enter path | FAIL |
| 6 | First arrival coordinate after command | VERA20k teleport target is `refinery_pad_for_sid`: fallback pad `(rx + w - 1, ry + h / 2) = (13,11)` | gamemd does not relocate immediately for this scenario; the unit remains in movement/enter flow until it physically reaches the dock/pad path | FAIL |
| 7 | Far-return teleport destination formula | If VERA20k teleports, it targets pad `(13,11)` | In the retail far branch, Mission_Harvest reads BuildingType dock offset `(+4,+1)` and calls `Find_Nearby_Passable_Cell` from `(14,11)` before `Set_Destination` | FAIL |
| 8 | Dock entry after arrival | After teleport clears, `handle_return` sees `(13,11)` as dock contact and transitions to `Dock/Approach`; then `phase_approach` can link immediately because pad is also `(13,11)` | `UnitClass__PerCellProcess` detects pad-cell arrival under Mission Enter, calls `FootClass__PerCellProcess(2)`, sends radio `0x15`, stops locomotor, and Mission_Unload / `Mission_Deploy_Building` handles dump entry | UNCHECKED |
| 9 | Unload-entry rotation / sync | `phase_linked` sets `display_type_override=CMON`, emits `DockDeploy`, sets `facing_target=0x40`, then `phase_pivoting` waits until body facing reaches East before `Unloading` | Live `UnitClass__Receive_Radio` case `0x16` writes locomotor rate `0x4000`; `Mission_Deploy_Building` waits on the rate/timer gate and enters dump state. I did not prove literal facing byte equality for this exact CMIN arrival | UNCHECKED |
| 10 | Unload mission activation | VERA20k enters `RefineryDockPhase::Unloading` only after the local linked/pivoting path | gamemd sets mission `0x10` unload through the dock radio/per-cell path and `Mission_Deploy_Building` runs the storage dump loop | UNCHECKED |

## Findings

### F1 - Forced Return Teleports When Retail Would Drive

**Stage:** 4-5

For the concrete setup `(40,40) -> GAREFN(10,10)`, VERA20k immediately starts a chrono teleport because it uses a hardcoded 2-cell threshold from refinery center. Retail YR compares against `ChronoHarvTooFarDistance=50` cells in leptons from the dock-offset/passable-cell target, so this miner is inside the drive-to-refinery band.

**Player-visible difference:** the miner vanishes and reappears at the refinery instead of driving back. This changes timing, vulnerability, path blocking, and visual continuity.

**Our code:** `src/sim/miner/miner_system.rs:36-40`, `src/sim/miner/miner_system.rs:855-875`.

**gamemd evidence:** `UnitClass__Mission_Harvest @ 0x0073E5E0`, state 2, reads `RulesClass+0xD7C`, multiplies by `0x100`, and branches to the close/refinery-radio path when distance is `<= threshold`.

### F2 - Teleport Is Issued Before The Retail Movement/Radio Checks

**Stage:** 5

VERA20k's forced-return state directly calls the chrono helper once the command has set `ForcedReturn`. Retail's active standard-YR harvester return path does dock lookup, existing destination/movement checks, distance comparison, and refinery radio before choosing the enter path; the teleport branch is only reached after those checks fail the close-distance condition.

**Player-visible difference:** the command resolves as an instant warp in VERA20k in cases where retail is still negotiating/refining the return movement state.

**Our code:** `src/sim/miner/miner_system.rs:662-696`, `src/sim/miner/miner_system.rs:580-650`.

**gamemd evidence:** `UnitClass__Mission_Harvest @ 0x0073E5E0`, state 2; `FootClass__Find_Docking_Bay @ 0x004DF040`; `UnitClass__Receive_Radio @ 0x00737430` cases `0x0E`, `0x15`, `0x16`.

### F3 - First Post-Command Position Differs

**Stage:** 6

In VERA20k, the first special-movement relocation places the miner on `(13,11)`. In retail for this concrete distance, there is no special relocation; the miner remains at/near its pre-command location until normal movement advances it.

**Player-visible difference:** the miner appears at the refinery too early and skips the visible return trip.

**Our code:** `src/sim/movement/teleport_movement.rs:183-210`; target from `src/sim/miner/miner_system.rs:862-875`.

**gamemd evidence:** same Mission_Harvest state 2 threshold branch above; computed `~9971 <= 12800` leptons, so no teleport state is entered.

### F4 - Far-Branch Destination Uses Pad Instead Of DockOffset Passable Cell

**Stage:** 7

This is adjacent to the chosen coordinate because retail did not take the far branch here, but it is directly in the same forced-return helper: if a far forced return does teleport, VERA20k targets pad `(13,11)`, while gamemd's far branch seeds `Find_Nearby_Passable_Cell` from BuildingType dock offset `(14,11)` for GAREFN.

**Player-visible difference:** far returns can materialize one cell too far inside/on the dock flow, skipping the retail queue/dock-offset arrival step.

**Our code:** `src/sim/miner/miner_system.rs:862-875`; `src/sim/miner/miner_dock_sequence.rs:100-119`.

**gamemd evidence:** `UnitClass__Mission_Harvest @ 0x0073E5E0`, far branch reads `BuildingTypeClass+0x1618/+0x161C`, calls `FootClass__Find_Nearby_Passable_Cell`, then `Set_Destination`.

## Adjacent Findings

- The older manual-refinery trace that said VERA20k loses the clicked refinery target is stale for current code: `Command::MinerReturn` now carries `target_refinery_id`.
- The unload-entry facing/rate handshake needs a separate focused trace. I verified the active YR radio and unload functions, but did not prove literal facing byte equality for CMIN in this run.
- Post-unload `ReleaseDockedHarvester` and Force_Track `0x47` were not part of this forced-return entry trace.

## Verdict Tally

PASS: 0 | FAIL: 4 | UNCHECKED: 6 | NOT-IMPLEMENTED: 0

## Status

COMPLETE
