# Mission handler absorption — recommended order

**Date:** 2026-07-27
**Context:** The authority flip landed (`ca58aa3f`): MissionCom is verb-owned, the
projection layer is deleted. What remains is caller-side work: absorb the legacy
per-system FSMs into real mission handlers dispatched at the host, and wire the
remaining verified native caller families onto the existing verbs. This doc is the
work order — not a status ledger; derive status from git.

The per-slice pattern (established, do not reinvent):
1. **Relocate** — the host's dispatch point calls the FSM's step function when
   `current` matches and the dispatch timer is due (template:
   `src/sim/miner/harvest_mission.rs::harvest_mission_step`, the L5 seam).
2. **Invert state** — `MissionCom.handler_state` becomes the cursor; the bespoke
   FSM field retires. Cadence comes from `MissionControl` rates written through
   `write_dispatch_epilogue` (the dispatch timer comes alive here).
3. **Handlers are plain functions** — `match` on mission id, native commit order,
   no traits/vtables. Callers keep using the live verbs; handlers use the
   verified host writes (`increment_ai_counter`, epilogue, leaf latches).

Every flip shifts the state hash → land one at a time, each with its own
documented re-baseline (coordinate via `docs/scans/PENDING_REBASELINES.md`).

## Track A — handler absorption (strictly ordered)

### A1. Harvest (miner)
- Why first: the routing seam already exists (`harvest_mission_step`), the
  miner-dock suite is the densest in the repo, and the departure verbs
  (Queue(10,0)+Commence at `0x0073E283`) already landed with the flip.
- Scope: dispatch Harvest from the host Unit arm; substate-authority flip
  (`miner.state` → `handler_state`) — the "shell S5" flip the seam doc names;
  Enter/Unload cadence from MissionControl (W1 groundwork already landed:
  `ddb4c1be`, `aa37a8b0`).
- Evidence anchors: `harvest_mission.rs` header (what L5 deferred),
  MISSION_ENTER_CROSSWALK_AND_GAPS_GHIDRA_REPORT.md.

### A2. Move + Guard (Units)
- Why second: the "Checkpoint A" cloned host-trace tests in
  `src/sim/world/techno_ai.rs` (`trace_cloned_ordinary_drive_host` + its matrix)
  already pin the exact binary-verified ordinary-Drive Move handler behavior —
  a dormant oracle the real handler must satisfy. Guard is the idle mission and
  the passive-acquire gate's main input (S4c shadow becomes live here).
- Scope: Move handler (timer-gated, epilogue writes ~[Move] Rate), Guard handler
  (idle selector + acquire hook), retire the movement phase's mission-side
  bookkeeping for Units.
- Needs RE first: exact Mission_Move/Mission_Guard bodies + epilogue write
  points (see the swarm targets below).

### A3. Enter / dock
- Scope: Enter handler for Units (transport/bunker/depot approach), unify with
  the radio HELLO/dock choreography; the readiness queued-Enter branch is
  already exact.
- Watch: dock admission is registry-authoritative with the RadioBus as shadow —
  the registry-retire slice ("Slice 8" in the dock code) intersects here.

### A4. Aircraft (last)
- Why last: largest FSM (`aircraft_mission`), most sub-states, and the leaf
  latch writers (action / transition-ready) must land with it or its Ready gate
  stays partially inert.
- Scope: per-mission aircraft handlers (Guard/Attack/Enter/ReturnToBase →
  missions 5/1/7 + RTB), latch writers at their verified positions, airstrike
  manager presence stays deferred.

## Track B — caller-family wiring (parallel, any time, no A-dependency)

Each wires onto existing verbs at positions already enumerated in the two
active-caller census reports (`MISSION_ASSIGN_OVERRIDE_...` /
`MISSION_QUEUE_COMMENCE_RESTORE_...`). Small, independent commits.

- B1. Creation/idle: spawn-time Queue(Guard,0)+Commence (`0x0065E460` family),
  Unlimbo R>C (`0x006F6E49`), multiplayer initial-unit Assign(5/0xB)
  (`0x005D7420`), production-exit Assign(2)/Queue family (`ExitObject`).
- B2. Obstruction Override: locomotor blockage → Override(Attack, blocker, 0)
  (8 sites, all families — Drive `0x004B3BE9` et al.). First Override activation;
  requires the concrete Target/NavCom effects provider to go live.
- B3. Damage-response Override: retaliation packet Override(1, source, 0) at
  ReceiveDamage (`0x00702B41`) — longer term this REPLACES the legacy
  retaliation path, coordinate with combat.
- B4. Restore families: Aircraft Enter_Idle_Mode Restore (`0x00417706`) + the
  other two; suspended Target/NavCom archives already exist.
- B5. Building ready-latch writers at the animation/mission transitions, making
  the Building Update consume points (already wired at the host) actually fire.

## Standing gaps that gate exactness (close opportunistically)

- Locomotor readiness producers: each family writes `mission_ready_state` at its
  verified points; removes the host's "not moving" degradation.
- Signed object height: real ObjectClass height dword owner.
- Second Unit-AI R>C point (`0x007366FD`) once the host bracket grows a post-combat
  position.
- Building-under anchor frame for multi-cell foundations (UNCHECKED).

## Verification gates

- Per slice: scoped suite + one full `--lib` at merge; hash re-baseline with
  documented reason; RNG stream pins must hold unless the slice provably moves
  draws.
- End-state parity gate (once A2+ lands): oracle cross-engine comparison of
  per-object CurrentMission vs MissionCom on a scripted scenario via the
  parity-digest plumbing — the only path to VERIFIED for the emergent state.

## Pre-A2 RE swarm targets (/re-swarm, when starting Track A2)

1. Mission_Move / Mission_Guard handler bodies + dispatch-timer epilogue writes.
2. Locomotor readiness producer write points (per family).
3. Aircraft/Building leaf latch writers.
4. Enter_Idle_Mode + creation mission assignment details.
