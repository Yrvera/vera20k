# Parity Gap Scan — Ranked Shortlist (2026-05-29)

Multi-modal gamemd→Rust parity-gap scan. 5 independent survey angles (docs, rust-code,
INI, Ghidra-binary, deferred/plans) → 51 raw findings → reconciled to 14 candidates →
each adversarially verified against **current** `src/` → ranked by severity
(player-visibility × frequency). 32 agents, 742 tool calls.

`agree` = how many independent survey angles flagged it (cross-modality confidence).
Most items are `agree=1` — flagged by a single angle; the scan skewed combat/sim-heavy
(see "Coverage gaps" at bottom).

## Ranked

| # | Severity | agree | System | Pipeline | Effort |
|---|----------|-------|--------|----------|--------|
| 1 | CRITICAL | 3 | Warhead special-effects (MindControl, EMP, Temporal, Radiation, Parasite, IvanBomb, Magnetron, Culling, Poison, Airstrike, TransactMoney) | brainstorm→impl | Large / multi-session |
| 2 | HIGH | 1 | Authoritative projectile flight damage model (travel time, mid-flight death, impact-time damage + retaliation) | re-investigate | Large, high blast radius |
| 3 | HIGH | 2 | DamageParticleSystems — spark stream + grey smoke on damaged vehicles/buildings | brainstorm→impl | Medium |
| 4 | HIGH | 3 | Veterancy promotion (XP-on-kill + stat multipliers + elite weapon swap + abilities) | brainstorm→impl | Medium-large |
| 5 | HIGH | 3 | Self-healing HP regen (SelfHealing= + SELF_HEAL ability) | brainstorm→impl | Small-to-medium |
| 6 | HIGH | 2 | Cloaking/stealth state machine + Spy/Mirage disguise | brainstorm→impl | Large |
| 7 | HIGH | 1 | Weapon AmbientDamage / RadLevel area radiation aura (RadSiteClass) | decode-system | Medium-to-large |
| 8 | HIGH | 1 | Gattling weapon stage escalation | brainstorm→impl | Medium (~2-3 sessions) |
| 9 | HIGH | 1 | Sound channel-management policy (priority eviction, Limit=, MinVolume, VoiceDie) | brainstorm→impl | Medium |
| 10 | HIGH | 2 | Spawner units (Carrier Hornets, Dreadnought/Boomer missiles, Destroyer ASW, V3 rocket) | brainstorm→impl | Large |
| 11 | CRITICAL | 1 | Superweapon launch handlers (Nuke, Chronosphere, ChronoWarp, Psychic Dominator, Spy Plane) | decode-system | Large |
| 12 | HIGH | 1 | Superweapon & lightning-storm audio (EVA warning + per-bolt thunder) | brainstorm→impl | Medium |
| 13 | HIGH | 1 | Spy infiltration effects (OnSpyInfiltrate 7-branch dispatch) | brainstorm→impl | Medium-to-large |
| 14 | MEDIUM | 1 | Tactical auto-scroll acceleration curve / coast levels / 16ms ramp | brainstorm→impl | Medium (1 session) |
| 15 | MEDIUM | 1 | Open-topped / tank-bunker transport fire damage, ROF, range modifiers | brainstorm→impl | Small-to-medium |
| 16 | MEDIUM | 2 | Weapon flags: Suicide, DrainWeapon, TurboBoost, FireWhileMoving / OpportunityFire | re-investigate | Medium per flag |
| 17 | MEDIUM | 1 | Projectile trajectory: Inaccurate scatter + Gravity arc + BallisticScatter | re-investigate | Medium |
| 18 | MEDIUM | 1 | Airburst secondary-weapon + Shrapnel + Cluster split detonation | brainstorm→impl | Large (needs projectile layer) |
| 19 | MEDIUM | 1 | Building damage-fire threshold selector (ConditionRed via CanBeOccupied) | direct-fix | Small (~1-2h) |
| 20 | MEDIUM | 1 | LogicClass scheduler / native tick spine (active-object order, factory/house tail) | brainstorm→impl | Large, high blast radius |
| 21 | LOW | 1 | Projectile Trailer smoke-trail visual | direct-fix | Small |
| 22 | LOW | 1 | Pathfinding zone reachability for naval/water movers | brainstorm→impl | Small-to-medium |

## Top recommendation

**#1 Warhead special-effects** — the only CRITICAL gap that fires every match on the
faction-defining weapons (Yuri mind control, Tesla/EMP, Chrono Legionnaire, Terror Drone,
Crazy Ivan, Desolator), all currently resolving to plain damage. Research is done across
6+ verified Ghidra reports. Shares the warhead `Detonate` path with #2 (projectiles),
#18 (airburst/cluster) and parts of #11 — scoping the Detonate dispatch once avoids
re-touching the combat path repeatedly.

## Highest-confidence (agree=3)
#1 Warhead effects, #4 Veterancy, #5 Self-healing — independently surfaced by three angles.

## Quick wins (small effort, research done)
#5 Self-healing (agree=3, every match), #19 Building damage-fire threshold (~1-2h, direct-fix),
#8 Gattling (RE cost ~0).

## Dependency cluster — the combat/projectile substrate
#1, #2, #11, #17, #18, #21 all touch the warhead Detonate path and/or a projectile
(BulletClass) flight layer that does not yet exist. #18 and #21 are explicitly blocked on
the #2 flight layer. Decide the substrate (#2) early if pursuing this cluster.

## Coverage gaps (this scan's blind spots — follow-up needed)
The rank pass was combat/sim-heavy and most items are single-angle. Under-covered areas a
follow-up `/gap-scan` should sweep, scoped per area: (1) **UI/HUD/sidebar** — tooltip
delays, cameo pulse, sidebar flash, selection-bracket/health-bar/pip draw order, placement
ghost, minimap, cursor-near-boundary; (2) **economy/production** — refinery dump cadence,
build-time/low-power slowdown, sell refund %, MCV deploy/undeploy timing; (3)
**map/terrain/bridge** — ore growth rate exactness, bridge destruction/repair edges,
tiberium-tree spawners, passability; (4) **audio breadth** — EVA cue selection/overlap,
music transitions, per-unit voice rotation; (5) **net/lockstep** — RNG draw-order parity
across the unimplemented effects, replay determinism; (6) **animation/facing** — turret
lag, rotation accel, facing interpolation, build-up/down timing; (7) **whole-unit
behaviors** a per-flag scan misses — Chrono Legionnaire/Iron Curtain interactions,
MCV-pack, paradrop/parabomb, IFV mode-swap.
