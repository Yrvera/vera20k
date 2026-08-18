# Native Tick-Spine Contract (Option B — native-ordered hybrid)

Date: 2026-05-29
Target: rebuild `Simulation::advance_tick` (src/sim/world/mod.rs) around gamemd's
spine — early globals -> single live object-AI pass (`for_each_live_object`) ->
factory/house tail -> late frame commit — keeping output-equivalent phases batched
in their native-order slot, and moving into the live pass ONLY phases with observable
same-tick cross-object coupling. Built on Contract 1 (`for_each_live_object`) and
Contract 2 (lifecycle membership).

Source: 4-agent read-only classification swarm (movement / combat / production-economy /
globals-vision-SW-passenger), 2026-05-29.

## The coupling verdict — the whole map

A phase is COUPLED (must run per-object in the live pass) iff within ONE tick object
A's processing reads/writes state object B's same-tick processing depends on, so
A-before-B != B-before-A is observable. Otherwise BATCHED-OK (pure function of
frame-start committed state) and just needs the right native-order slot.

### COUPLED -> live object-AI pass (per-object, deterministic order)
| Subsystem | The coupling |
|---|---|
| **Ground movement** | Shared `OccupancyGrid` mutate-as-you-go: A claiming/crushing/blocking cell C this tick changes B's same-tick entry/scatter/crush. |
| **Miners** | (1) Dock reservation race (A docks, B waits); (2) ore-density depletion (A empties a node, B's harvest amount/choice changes). |
| **Building docks / Aircraft docks** | Slot/pad reservation race — first-processed wins the slot, loser queues. Same pattern as miners (the dock-reservation pattern appears 3×). |
| **Passengers / garrison** | Same-frame ownership transfer: passenger boarding before vs after the building's reconcile turn changes whether ownership transfers this frame (proven by existing same-frame/next-frame tests). Already iterates the active order; Contract 2 wired conceal/reveal. |
| **Combat: damage->death commit, INSTANT-HIT weapons only** | A's instant-hit weapon kills B before B's AI runs -> B never fires this frame. Rust snapshots attackers up front so B fires anyway. **Converges with the C6 projectile model** (projectile weapons are frame-deferred, so they already match batched). |

### BATCHED-OK -> placed in native-order slot (no behavior change from batching)
- **Early-global (before object pass):** vision/fog refresh, power, global superweapon
  effects (LightningStorm strike + EMP pulse — must apply before objects take their turn).
- **Within/around the object region (pure frame-start functions):** air / teleport /
  tunnel / rocket / homing / droppod / parachute movement, body rocking, aircraft
  missions, turret rotation, combat target-acquisition + fire-decision + garrison
  auto-acquire, deploy / fear / prone, attack pursuit, retaliation (reads last_attacker,
  inter-phase-ordered after combat).
- **Own native-order phases (already deterministic, pre-combat):** capture orders, C4
  plants, bridge repair (the immediate-successor-skip is already modeled).
- **Post-combat / pre-retaliation:** particles (order-sensitive: damage must precede
  retaliation; keep the slot).
- **Global-slot:** ore growth/spread, terrain spawners (per-cell, RNG-consuming;
  sequential, no per-object interleave).
- **Factory/house tail:** production queue, repairs, per-house superweapon charge/ready
  (split out of the global SW effects). Shared per-house credit pool — preserve the
  miners->production->repairs->docks deduction order (observable when near-broke).
- **Late-slot:** building anims + undeploy spawn, defeat detection, radar-event aging,
  world-effect anims, then the late frame commit (binary_frame + tick + state_hash).

## The honest reframe (important)

**The current batched design already produces near-correct output.** Because every
batched phase iterates in deterministic `keys_sorted()` order with mutate-as-you-go,
and the inter-phase order is already carefully sequenced (pursuit before combat,
particles before retaliation, capture before combat, smudge before ore-growth), the
intra-subsystem coupling is ALREADY reproduced. The genuine residual same-tick DRIFTs
are narrow:
- **Instant-hit death-before-fire** (combat) — and this is the same issue as C6
  (authoritative projectile damage timing).
- Cross-subsystem same-tick "A moves then acts affecting B" — mostly already handled by
  the existing inter-phase ordering (e.g. pursuit-before-combat).

So the spine rebuild is primarily a **STRUCTURAL / future-enabling** investment, NOT a
pile of visible bug fixes:
1. It makes the tick a single explicit ordered spine instead of 20 ad-hoc phases
   (maintainability; prevents future drift; "two 5%-off systems compound" protection).
2. It makes same-tick semantics *reachable* so the coupled fixes (C6 projectiles +
   instant-hit) can be done cleanly.
3. It does NOT, by itself, change much that a player sees — because the batched design
   is already mostly output-correct.

This is the refined answer to "do we lack a foundational spine?": **structurally yes;
behaviorally, the deterministic batched phases already approximate it.** The value of
building the real spine is structure + enabling C6, not bug-fixing.

## Build sequence

- **Step 3a — pure structural skeleton (hash-identical, zero behavior change).**
  Reorganize `advance_tick` into explicit regions — `early_globals()` /
  `live_object_pass()` / `factory_house_tail()` / `late_commit()` — keeping the exact
  current call order initially. Pure refactor; state hash byte-identical; suite green.
  This is the shippable increment that makes the spine the explicit structure.
- **Step 3b — adopt native ordering where output-equivalent.** Move vision/power/global-SW
  to early-globals; split per-house SW charge into the house tail; place ore-growth/
  spawners in the global slot. Each move proven output-equivalent (hash may change
  deterministically; no NEW test failures). Skip any move that isn't provably equivalent.
- **Step 4 — collapse the COUPLED subsystems into a real per-object live pass.** Only the
  6 coupled subsystems. The instant-hit/combat-commit part converges with the C6
  projectile vertical — do them together.

## Determinism invariants (every step)
- RNG draw order preserved (ore-growth, scatter, combat, smudge all consume the shared
  stream; reordering RNG-consuming phases shifts all downstream outcomes).
- Entities/bullets in `world_hash` deterministically (BTreeMap id order).
- `binary_frame` committed late (pre-increment visible during the tick).
- No float in sim; fixed-point only.
- Suite stays at the 10-failure baseline; any NEW failure or hash/determinism change
  beyond a proven-equivalent reorder is a blocker.

## Do-not-do
- Do not invert all 20 phases into a literal per-object loop at once (max desync risk;
  most phases are batched-OK anyway).
- Do not move an RNG-consuming phase without accounting for the stream shift.
- Do not parallelize implementation — shared advance_tick/RNG/hash = desync.
- Do not move the truly-coupled combat-commit before the C6 projectile model is decided
  (they're the same problem).
