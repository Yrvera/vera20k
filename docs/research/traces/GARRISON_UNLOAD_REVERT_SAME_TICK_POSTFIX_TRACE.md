# Garrison Unload/Revert Same-Tick Postfix Trace

Date: 2026-05-27

Scenario: a captured civilian `CanBeOccupied` building has exactly one occupant and receives `OrderIntent::Unloading`. This traces only the tick where that last occupant leaves, checking same-tick unload before civilian-garrison reconciliation, `StructureAbandoned` owner, owner revert, and `ownership_changed`.

Status: COMPLETE for the scoped sim lifecycle. One player-visible app-layer audio gap remains.

## Pipeline

`OrderIntent::Unloading` on captured UC building -> Rust live-object-order passenger turn reaches the building -> unload one occupant -> cargo count becomes zero -> same building turn reconciles civilian garrison owner -> emit `StructureAbandoned { owner: pre_revert_owner }` -> write civilian owner -> return `ownership_changed=true` -> app maps event to `GameSoundEvent::StructureAbandoned` but audio drain currently ignores it.

## Active-YR Confirmation

- Standard YR has active `CanBeOccupied=yes` civilian buildings; `rulesmd.ini:19302` is `[CAGAS01]`, with `CanBeOccupied=yes` and `MaxNumberOccupants=10` at `rulesmd.ini:19322..19323`.
- `BuildingClass::Update @ 0x0043FB20` calls `CheckAutoSellOrCivilian @ 0x00458200` for `CanBeOccupied` buildings. Active in YR per `CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md:48`.
- Empty revert is active in YR: `count == 0 && owner != civilian_house` emits abandoned cues before `ChangeOwner(civilian_house, 0)`. Evidence: `CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md:62..64` and resolved OQ-07 at `:128`.
- The global scheduler is a live forward object-vector pass, active from standard `Main_Tick`. Evidence: `CIVILIAN_GARRISON_OWNER_TIMING_GLOBAL_ORDER_GHIDRA_REPORT.md:40..55`.

## Stage Verdicts

| Stage | gamemd result | Rust post-fix result | Verdict |
|---|---|---|---|
| 1. Building-turn ordering | In the building update, normal garrison ejection/count mutation occurs before the later `CheckAutoSellOrCivilian` reconciliation; for one occupant, count becomes `1 -> 0` before revert logic reads it. | `tick_boarding_and_garrison_reconciliation_in_order` processes `OrderIntent::Unloading` for a `CanBeOccupied` building at `passenger.rs:298..300`, then calls reconciliation at `passenger.rs:302`. `PassengerCargo::unload_first` removes index `0`; with one passenger, `len` becomes `1 -> 0` (`passenger.rs:99..107`). | PASS |
| 2. Same-turn empty revert trigger | `CheckAutoSellOrCivilian` sees `count == 0 && owner != civilian_house` during that same building update. | After unload, the same function call invokes `reconcile_civilian_garrison_owner_for_building`; the empty/non-civilian branch is at `passenger.rs:483..491`. | PASS |
| 3. `StructureAbandoned` event owner | Native abandoned cue branch runs before `ChangeOwner`; owner is the pre-revert captured owner. | Rust pushes `SimSoundEvent::StructureAbandoned { owner: current_owner }` before writing `building.owner = civilian_owner` (`passenger.rs:483..490`). For input owner `Americans`, event owner is `Americans`. | PASS |
| 4. Owner revert value | Native resolves the Civilian-side `HouseClass*` by side lookup and house-array scan, then calls `ChangeOwner(civilian_house, 0)`. | Rust `resolved_civilian_garrison_owner` returns `Neutral` if present, otherwise `Special`, otherwise interns `Neutral` (`passenger.rs:422..437`). Stock INI has `[Neutral] Side=Civilian` (`rulesmd.ini:3345..3353`), but literal native house-pointer equality was not runtime-computed. | UNCHECKED |
| 5. `ownership_changed` propagation | Native owner change is complete at the end of that building update, so post-update systems can see the changed owner. | Reconciliation returns `current_owner != civilian_owner`; `tick_passenger_system` returns the OR of the ordered reconciliation result (`passenger.rs:266..270`), and `TickResult.ownership_changed` receives it at `world/mod.rs:1573` and `world/mod.rs:1715`. For `Americans -> Neutral`, result is `true`. | PASS |
| 6. Next-tick duplicate event | Native empty-revert branch no longer fires after the owner is already the Civilian house. | Rust only emits when `cargo_empty && !is_civilian_garrison_owner(current_owner)` (`passenger.rs:483`); after revert to `Neutral`, the next pass does not emit another `StructureAbandoned`. | PASS |
| 7. Local EVA/event gate owner | Native abandoned EVA/audio is gated on the pre-revert human owner before `ChangeOwner`. | App mapping uses the sim event owner and local-owner comparison before selecting `EVA_StructureAbandoned` (`app_sim_tick.rs:475..492`). Owner gate matches for a local captured owner. | PASS |
| 8. Audible StructureAbandoned playback | Native abandoned cue is player-audible when the pre-revert owner is human. | Rust converts to `GameSoundEvent::StructureAbandoned`, but `drain_sound_events` explicitly does nothing for `StructureAbandoned` (`app_building_anim.rs:607..610`). The player will not hear the abandoned EVA even though the sim event owner is correct. | NOT-IMPLEMENTED |
| 9. Native radar/positional abandoned cue set | Native empty-revert branch creates sound/radar/EVA cues before owner change. | Rust has only `SimSoundEvent::StructureAbandoned { owner }`; no radar event or positional abandoned cue was found in the scoped source search. Exact cue-set parity is not implemented. | NOT-IMPLEMENTED |

Verdict tally: PASS: 6 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 2

## Failures / Not Implemented

1. Stage 8 - Audible `StructureAbandoned` playback is missing. Player-visible difference: the player gets no abandoned EVA despite the sim producing the correctly owned event. Rust: `src/app_building_anim.rs:607..610`. gamemd evidence: `CheckAutoSellOrCivilian @ 0x00458200` pre-revert abandoned cue branch, active YR per `CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md:62..64`.
2. Stage 9 - Full native abandoned cue set is incomplete. Player-visible difference: native radar/positional cue behavior is not represented by Rust's single sim sound event. Rust: `src/sim/passenger.rs:485..487`, no matching radar push found. gamemd evidence: same active empty-revert branch cited above.

## Adjacent Findings

- This trace does not re-check exact native normal-unload ejection placement, exit-cell scan, LIFO/FIFO occupant order, or scatter. Those belong to SellBuilding/ejection traces.
- This trace does not validate full `HouseClass` pointer identity for the Civilian-side house. Rust's stock `Neutral` result is plausible from INI, but the native side/house-array scan remains a separate exactness gap.
- This trace does not validate UI command eligibility for captured civilian garrison unload; the scenario starts with `OrderIntent::Unloading` already present.

## Sources

- Research docs: `docs/research/CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md`, `docs/research/CIVILIAN_GARRISON_OWNER_TIMING_GLOBAL_ORDER_GHIDRA_REPORT.md`.
- INI: `ini/rulesmd.ini`.
- Rust source: `src/sim/passenger.rs`, `src/sim/world/mod.rs`, `src/app_sim_tick.rs`, `src/app_building_anim.rs`.
- No Ghidra mutation was performed. No Cargo tests were run for this read-only trace.
