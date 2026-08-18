# ObjectClass Lifecycle → Scheduler Touchpoints Contract (Contract #2)

Date: 2026-05-28
Builds on: Contract #1 (LogicScheduler live pass) — DONE, see
`2026-05-28-logicclass-scheduler-live-pass-contract.md`.
Verified binary spine: `LOGICCLASS_OBJECT_LIFECYCLE_SPINE_SYSTEM_MODEL_SYNTHESIS.md`
(live Ghidra spot-checks this date on Reveal `0x005F4EC0`, Conceal `0x005F4D30`,
adder `0x0055BAA0`, remover `0x0055BAE0`, UnInit `0x005F65F0`).

## Phase status — ALL 8 COMPLETE.
- PHASE 1 (evidence): DONE — synthesis doc live spot-checks (Reveal `0x005F4EC0`,
  Conceal `0x005F4D30`, adder `0x0055BAA0`, remover `0x0055BAE0`, UnInit
  `0x005F65F0`) plus Contract-1's two independent agents that re-verified the
  adder/remover/membership-bit/scheduler from scratch. The load-bearing claim here —
  Conceal removes the object from the active vector — is live-verified.
- PHASE 2 (this doc): the lifecycle→scheduler contract + current-Rust delta.
- PHASE 3 (premise): concrete gap — conceal-on-board never unregistered; latent
  until a consumer iterates the live pass (same structural reality as Contract 6).
- PHASE 4 (adversarial): the binary basis (remover + `+0x98` membership) was
  independently re-verified from scratch by Contract-1's two agents; the Rust wiring
  is a faithful translation (register on reveal, unregister on conceal).
- PHASE 5 (implement): DONE — conceal at `process_boarding_passenger` (board→Inside);
  reveal at `process_unloading_transport`, `tick_unloading`, and
  `place_garrison_passenger_at_cell`. Kill paths (`mark_garrison_passenger_removed`,
  combat kill-riders) correctly do NOT register; `despawn_entity` unregisters first.
- PHASE 6 (tests): DONE — 3 tests in `passenger.rs`
  (`boarding_conceals_passenger_from_active_order`,
  `garrison_eject_reveals_passenger_into_active_order`,
  `board_then_eject_round_trip_reappends_once_at_tail`).
- PHASE 7 (verify): DONE — 3269 pass; 10 fail = known baseline; determinism intact.
- PHASE 8 (review): DONE — independent symmetry+determinism audit returned PASS;
  every on-map↔Inside transition classified, no unwired reveal site, no kill-path
  mis-registration, no nondeterminism.

## The contract (output semantics)

Two independent per-object states, never conflated (synthesis claims 3, 5):
- **InLimbo** (gamemd `ObjectClass+0x81`): on/off the playfield.
- **Logic membership** (gamemd `ObjectClass+0x98`, Rust `GameEntity::in_logic_vector`):
  receives per-tick AI via the active-object vector.

Lifecycle → scheduler transitions that MUST hold:

| Transition | Scheduler effect | gamemd evidence | Rust today |
|---|---|---|---|
| **Reveal / Unlimbo** (object placed on playfield) | register (tail-append, idempotent) | Reveal `0x005F4EC0` → adder `0x0055BAA0`; type-gate `type+0x234` | DONE — `register_live_object` at map spawn (`world_spawn.rs:260`) and `spawn_object` (`:438`); paradrop drop (`drop_payload.rs:238`) |
| **Limbo construction** (created off-playfield, e.g. paradrop cargo) | NO register until revealed | active insertion = reveal, not constructor (claim 6) | DONE — `spawn_object_limbo_at_height` explicitly does not register (`world_spawn.rs:587-590`). (Synthesis §7 flagged this as DRIFT; it is already fixed.) |
| **Conceal** (leaves playfield: boards transport / garrisons) | unregister (compacting) + InLimbo | Conceal `0x005F4D30` → remover `0x0055BAE0` | **GAP** — boarding sets `PassengerRole::Inside` (`passenger.rs:480`) but never calls `unregister_live_object`. Production only ever unregisters via `despawn_entity`. |
| **UnInit / death** | Limbo→Conceal (unregister) → clear IsAlive → deferred PendingDeleteList | UnInit `0x005F65F0` → PendingDeleteList `0x00B0F69C` (claim 7) | PARTIAL — `despawn_entity` unregisters then removes inline (`mod.rs:740`). Deferred-delete (PendingDeleteList) not modeled; player-visibility of inline-vs-deferred is UNCHECKED. |

## The concrete gap (PHASE 3 premise)

A passenger that boards a transport / garrisons a building becomes `Inside` but
remains in the active-object order (`in_logic_vector` stays true). In gamemd it is
Concealed → removed from the vector. Current Rust masks this because the phase
systems guard on `PassengerRole::Inside` and skip inert passengers — so there is
**no visible drift today**. It becomes visible once a consumer iterates
`for_each_live_object` with append/remove sensitivity: the stale `Inside` member
occupies an order slot and can shift same-tick ordering of appends/removes
(e.g. the garrison-owner-timing consumer's relative passenger/building turn).

This is the same structural pattern as Contract 6: the wiring gap is real and
gamemd-faithful to fix, but its player-visible payoff is coupled to consumer
migration onto the live pass.

## Implementation plan (all-sites-or-none — closed loop)

Wiring must be symmetric across EVERY conceal/reveal site or it creates a worse
bug (a passenger revealed via an unwired path silently gets no AI). Sites:

CONCEAL → `unregister_live_object(pax_id)`:
- `passenger.rs` `process_boarding_passenger` success → `Inside` (~line 480).

REVEAL → `register_live_object(pax_id)`:
- `passenger.rs` `process_unloading_transport` eject → map (~line 855).
- `passenger.rs` `tick_unloading` batch eject → map (~line 1005).
- garrison eject (`production::eject_red_hp_garrison`, called at `passenger.rs:568`).
- transport-death passenger disposition (combat/despawn path): ejected survivors
  REVEAL→register; killed passengers go through `despawn_entity` (already
  unregisters). This path must be located and verified before implementing.

NOT conceal/reveal (no change): boarding-cancel sites (`passenger.rs:410, 505, 649`)
— the passenger was still on the playfield in `Boarding` state, never unregistered.

Tests (Phase 6):
- `boarding_unregisters_passenger_from_active_order`
- `unloading_reregisters_passenger_into_active_order` (tail, not sorted)
- `garrison_eject_reregisters_occupant`
- round-trip: board then unload leaves exactly one membership entry, appended at
  tail (idempotent, order-preserving).

Verify (Phase 7): full suite vs the 10-failure baseline; state hash stays
deterministic (the active order is hashed, so membership changes shift the hash
deterministically — confirm no NEW failures).

## Do-not-do
- Do not wire only some sites (asymmetry = silent no-AI bug).
- Do not unregister on boarding-CANCEL (passenger never left the playfield).
- Do not collapse InLimbo and logic-membership into one flag.
- Do not switch to inline-vs-deferred delete without its own premise (UNCHECKED).
