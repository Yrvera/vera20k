# Substrate Parity Ledger — 2026-05-29

**Scope:** Core sim substrate (entity lifecycle, advance_tick ordering ladder, RNG streams/seed,
deterministic state-hash coverage, command dispatch/order-intent, global timing & logic-vector)
vs gamemd.exe. Detect-only — no code was changed. Burden of proof per CLAUDE.md: default verdict
on any difference is DRIFT/MISSING unless equivalence is proven; 1-tick/1-cell/1-sample all count.

---

## Ranked confirmed holes

29 confirmed findings. By severity: **HIGH ×12**, **MEDIUM ×11**, **LOW ×6**.

### HIGH

| id | dimension | kind | rust_location | player-visible symptom | gamemd vs rust | evidence |
|---|---|---|---|---|---|---|
| `rng-single-stream-1` | RNG routing | DRIFT | world/mod.rs:288-289 | Any match with scatter/HouseClass/TIBTRE rolls desyncs from gamemd from tick 1 | gamemd: two streams (g_MainRng + Scen->Random) identical at init, diverge by routing; rust: one `SimRng` | decompile 0x0052FC20 (two Random__Seed + 0xfd copies) |
| `rng-state-not-copied-from-seeded-both-4` | RNG routing | DRIFT | world/mod.rs:459 | Scatter/sub-cell/TIBTRE differ on first second-stream draw | gamemd: both streams seeded identical incl. index_b=0x67; rust: only one stream exists | decompile 0x0052FC20, 0x0065C6D0 |
| `rng-mp-seed-handshake-missing-3` | RNG routing | MISSING | world/mod.rs:75,459 | MP clients diverge frame 1 (no shared seed) | gamemd: shared u32 lobby seed broadcast in options packet; rust: hardcoded u64 constant | decompile 0x0052FC20 (entropy-skip gate); net/mod.rs stub |
| `rng-tibtre-house-smudge-stream-5` | RNG routing | DRIFT | terrain_spawn.rs / smudge_*.rs | TIBTRE spread, smudge angle, scorch/crater, house roll all draw wrong stream | gamemd: these route to Scen->Random; rust: single self.rng | disasm 0x0071C730 (Scen+0x218), 0x004F88FA |
| `combat-immediate-remove-skips-logic-unregister-1` | entity lifecycle | DRIFT | combat/mod.rs:1000-1008 | Structure/voxel death leaves dangling id in hashed logic order → lockstep desync | gamemd: UnInit compacts object out of per-tick logic vector; rust: combat `entities.remove` skips unregister | decompile 0x005f3b80 (array#5 removal), 0x0055afb0 |
| `dying-shp-despawn-driven-by-app-not-sim-1` | entity lifecycle | DRIFT | app_sim_tick.rs:292-307; combat/mod.rs:985-997 | Headless sim never frees dying infantry; GUI free 1+ anim ticks late | gamemd: UnInit + pending-delete drain same logic tick; rust: despawn driven by app layer | decompile 0x0055DE9F, 0x00725c70 |
| `hash-defaulthasher-nondeterminism-risk` (note) | hash coverage | DRIFT | world_hash.rs:34 | (LOW sev — listed under LOW) | — | — |
| `target-rank-distance-vs-score-1` (note) | command dispatch | DRIFT | (MEDIUM — see MEDIUM) | — | — | — |
| `scan-whole-world-no-ring-earlyreturn-3` | command dispatch | DRIFT | combat_targeting.rs:179-283 | Crowded scenes pick a target gamemd would never reach (no ring early-return) | gamemd: ring scan + quarter/half-radius early return + cell fallback; rust: scans whole store, nearest-first | decompile 0x006F8DF0 (ring early-return) |
| `no-targeting-cadence-throttle-4` | command dispatch | MISSING | world_orders.rs:45-76 | Guard/attack-move units retarget every tick ("twitchy"), wrong RNG draw count | gamemd: NormalTargetingDelay=27 / GuardAreaTargetingDelay=36 cadence + per-dispatch RNG; rust: every tick, no throttle | decompile 0x004D5070, 0x004D6AA0; rulesmd.ini:304-305 |
| `timing-tick-not-frame-1` | global timing | DRIFT | fixed_math.rs:51; world/mod.rs:1397 | Per-tick subsystems run ~3× faster than native frame logic | gamemd: 1 frame = 1 logic step (15Hz); rust: SIM_TICK_HZ=45, ~3 advance_tick per binary_frame | decompile 0x0055D360 (single late increment) |
| `logic-vector-not-driving-tick-3` | global timing | MISSING | logic_vector.rs; world/mod.rs:665 | Same-tick spawned/removed objects ordered differently → lockstep/replay drift | gamemd: single live-count-reload object vector (order 21); rust: per-subsystem BTreeMap phases | decompile 0x0055AFB0 (count reload loop) |
| `logic-vector-subsystem-order-4` | global timing | DRIFT | world/mod.rs:1428-1965 | Ore/EMP/lightning/team ordering vs object AI differs; RNG cursor shifts | gamemd: tiberium→bombs→teams→lasers→lightning→EMP→object→factory→house; rust: combat/SW early, ore/teams late | decompile 0x0055AFB0 (order table) |

### MEDIUM

| id | dimension | kind | rust_location | player-visible symptom | gamemd vs rust | evidence |
|---|---|---|---|---|---|---|
| `ladder-ore-placement-1` | ladder | DRIFT | world/mod.rs:1884-1934 | Ore density read-vs-mutate flips by one tick; bail counts/spread shift | gamemd: growth/spread orders 9-10 (top of tail); rust: Phase 7 after combat/production | decompile 0x0055AFB0 |
| `ladder-rng-cursor-ore-1` | ladder | DRIFT | world/mod.rs:1896-1934 | Ore/scatter/smudge RNG draws at wrong stream position → divergent placement | gamemd: ore draws (Scen->Random) before object-vector scatter; rust: scatter Phase 1 before ore Phase 7 | disasm 0x00722f00 (Scen+0x218) |
| `ladder-houseai-placement-1` | ladder | DRIFT | world/mod.rs:1312-1350 | Defeated-this-frame house can still emit AI commands; SW/factory order differs | gamemd: house loop order 28 (defeat inside HouseClass::Update before its AI tail); rust: AI before defeat | decompile 0x004F8440, 0x0055AFB0 |
| `ladder-object-vector-split-1` | ladder | DRIFT | world/mod.rs:1428-1670 | All units move then all fire (vs A moves+fires before B); first-shot/first-move flips | gamemd: single per-object interleaved AI pass; rust: split movement/combat phases | decompile 0x0055AFB0 |
| `target-rank-distance-vs-score-1` | command dispatch | DRIFT | combat_targeting.rs:277-285 | Guard/attack-move picks nearest, not highest-threat (tank vs engineer) | gamemd: strict-greater threat score (SpecialThreatValue, health, range); rust: nearest-first tuple | decompile 0x006F8DF0, 0x0070CD10 |
| `attackmove-no-anti-churn-mission-distinction-5` | command dispatch | DRIFT | world_commands.rs:389-482 | Fresh units use Guard not Area_Guard radius/cadence; AttackMove overwritable | gamemd: AttackMove=mission 28 w/ anti-churn, Area_Guard default for new units; rust: bool flags | decompile 0x005B2FD0, 0x00443C60, 0x740810 |
| `acquire-friendship-and-evaluate-gates-6` | command dispatch | MISSING | combat_targeting.rs:190-229 | Auto-acquires cloaked enemies / across bridge layers gamemd ignores | gamemd: cloak/sensor + OnBridge + Is_Ally gates; rust: fog friendship + cell-visible only | decompile 0x006F7CA0 |
| `command-order-per-owner-submission-7` | command dispatch | DRIFT | world/mod.rs:1262-1294 | Same-tick multi-house commands resolve in arbitrary intern order, not house registration | gamemd: outer loop house-array order; rust: sort by InternedId | decompile 0x0064c380 |
| `logic-vector-passenger-snapshot-5` | global timing | DRIFT | passenger.rs:355,367-383 | Mid-pass board/unload reconciled one tick late | gamemd: live-count-reload sees same-pass membership; rust: frozen snapshot | decompile 0x0055AFB0 |
| `hash-veterancy-missing` | hash coverage | MISSING | world_hash.rs:375-599 | Veterancy desync (weapon tier/damage) hashes identically — silent | OUR determinism tool; veterancy is unhashed authoritative state | game_entity.rs:152; combat_weapon.rs:102,111 |
| `hash-dying-missing` | hash coverage | MISSING | world_hash.rs:375-599 | dying-vs-alive (targeting/path gate) hashes identically | dying gates owned-count decrement independent of health | game_entity.rs:313; world/mod.rs:730-733 |
| `hash-order-intent-missing` | hash coverage | MISSING | world_hash.rs:375-599 | Guard-vs-attack-move / unloading-vs-idle divergence undetected | order_intent drives auto-acquire + unload loop, unhashed | game_entity.rs:234; passenger.rs:784 |
| `hash-movement-state-machines-missing` | hash coverage | MISSING | world_hash.rs:375-599 | Chrono/paradrop/missile/dock phase desync undetected | 8 state machines (teleport/tunnel/rocket/droppod/parachute/dock/aircraft) unhashed | game_entity.rs:236-283 |
| `hash-garrison-fire-index-missing` | hash coverage | MISSING | world_hash.rs:544-552 | Garrison fires from different occupant slot per client | garrison_fire_index round-robin unhashed | combat/mod.rs:2153 |
| `hash-building-up-down-missing` | hash coverage | MISSING | world_hash.rs:375-599 | Building/MCV completes on different tick per client | building_up/down timers gate entity creation, unhashed | world/mod.rs:1180-1204 |
| `hash-last-attacker-missing` | hash coverage | MISSING | world_hash.rs:375-599 | Idle unit retaliates at different attacker per client | last_attacker_id persisted+deferred, unhashed | game_entity.rs:198; decompile 0x701900 |
| `hash-slave-harvester-missing` | hash coverage | MISSING | world_hash.rs:375-599 | Slave Miner slaves harvest/return out of sync; credit diverges | slave_harvester AI state machine unhashed | game_entity.rs:232; slave_miner.rs:129-169 |

### LOW

| id | dimension | kind | rust_location | player-visible symptom | gamemd vs rust | evidence |
|---|---|---|---|---|---|---|
| `ladder-factory-placement-1` | ladder | DRIFT | world/mod.rs:1873-1880 | Production-vs-ore/death tick sequencing differs | gamemd: factories order 27; rust: Phase 7 before ore/AI | decompile 0x0055AFB0 |
| `ladder-superweapon-placement-1` | ladder | DRIFT | world/mod.rs:1606-1611 | Lightning bolt RNG/damage interleave vs object AI differs | gamemd: LightningStorm order 18; rust: Phase 4.5 pre-combat | decompile 0x0055AFB0 |
| `ladder-bridge-shroud-cadence-1` | ladder | DRIFT | world/mod.rs advance_tick | Stale shroud edge tile up to ~5s after gap-gen reshroud | gamemd: RecalcBridgeShroudFlags every 120 frames (order 6); rust: per-render-frame on demand | decompile 0x0055AFB0, 0x00578100 |
| `ladder-bombclass-missing-1` | ladder | DRIFT | world/mod.rs:1641 | C4-killed building's freed cells get ore one tick early | gamemd: BombClass::UpdateAll order 11 (after ore spread); rust: tick_c4_plants in Phase 5 | decompile 0x0055AFB0 |
| `rng-next-range-zero-noDraw-4` | RNG | DRIFT | rng.rs:121-126 | Latent: next_range_u32(0) skips a draw a literal RandomRanged(0,-1) would consume | gamemd: RandomRanged(0,-1) draws once; rust: early `return 0` | decompile 0x0065C7E0 |
| `rng-scatter-stream-and-range-2` | RNG | DRIFT | scatter.rs:123 | Idle scatter dir uses range 8 absolute vs gamemd RandomRanged(0,4) octant-jitter (currently dead code) | gamemd: ±2 jitter around facing octant from Scen->Random; rust: uniform 0..7 | disasm 0x0051D2AC |
| `timing-totalsimms-truncation-2` | global timing | DRIFT | app_types.rs:27; world/mod.rs:1397 | binary_frame cadence wobbles 1 tick (3-3-3-4) every ~46 ticks; bounded -1 frame offset | gamemd: pure integer frame increment; rust: 22ms truncation in ms→frame derivation | decompile 0x0055D360 |
| `hash-mind-controlled-missing` | hash coverage | MISSING | world_hash.rs:375-599 | Mind-controlled-vs-free divergence undetected (no writer yet) | mind_controlled gates garrison dock, unhashed | game_entity.rs:264; passenger.rs:272 |
| `hash-sub-cell-missing` | hash coverage | MISSING | world_hash.rs:375-599 | In-transit sub-cell slot divergence detected late | sub_cell slot (occupancy) unhashed; only sub_x/sub_y hashed | game_entity.rs:285 |
| `hash-defaulthasher-nondeterminism-risk` | hash coverage | DRIFT | world_hash.rs:34 | Cross-build/platform: identical state could hash differently → false desync | hash fn must be stable cross-client; std DefaultHasher (SipHash) is not guaranteed stable | world_hash.rs:34 |

---

## Fix direction — HIGH findings

Each describes the observable contract to satisfy. No literal C++ port; clean Rust that reproduces the output.

**`rng-single-stream-1` / `rng-state-not-copied-from-seeded-both-4` / `rng-tibtre-house-smudge-stream-5`** (two-stream root):
Simulation must hold **two** independent `SimRng` instances, both seeded from the same seed with
identical full state (including the index_b=0x67 start) at init, then advanced independently. Route
draws by gamemd's classification: combat/damage/sound-variant/particles/ore-growth-batch/laser-jitter/
bridge-collapse/center-smudge → the "main" stream; infantry & unit scatter direction, infantry sub-cell
rotation, survivor smudge roll+angle, anim scorch/crater 50/50, HouseClass cell-state roll, and TIBTRE
probability → the "scenario" stream. The two must produce the exact value sequence gamemd produces for
each consumer, which is only possible once a second untouched-by-combat stream exists.

**`rng-mp-seed-handshake-missing-3`:**
When the net layer lands, MP must establish one shared u32 seed before sim init (host generates and
broadcasts in the game-options packet; client decodes it), and seed BOTH RNG instances from it with no
entropy. Until then, SP/replay determinism via a constant is acceptable, but this is a hard MP-parity
blocker to track. Observable contract: all clients start byte-identical and stay in lockstep from frame 1.

**`combat-immediate-remove-skips-logic-unregister-1`:**
Combat's immediate removal of structures/voxel vehicles must unregister the killed id from the live
logic-order vector as part of teardown (same as `despawn_entity`), so the hashed active order never
retains a freed id. Observable contract: the logic-order length + id sequence after a combat death
matches what a save/load rebuild or a peer taking a different removal path produces.

**`dying-shp-despawn-driven-by-app-not-sim-1`:**
Death-animation advance and physical removal of dying SHP/infantry must happen inside the sim tick (the
lingering death sprite is a separate animation object, not the unit kept alive), so a headless sim frees
dying entities on the same logic frame as the GUI client. Observable contract: entity count, owned-count
timing, and logic order are identical between a non-rendering host and rendering clients; the unit is
gone within one logic frame of death, not coupled to render cadence.

**`scan-whole-world-no-ring-earlyreturn-3` / `target-rank-distance-vs-score-1`** (acquisition core):
Target acquisition must (a) rank candidates by an integer threat score (weapon effectiveness vs armor,
target SpecialThreatValue, health/strength, weapon-range, distance-beyond-range penalty,
EnemyHouseThreatBonus, threat-avoidance), keeping the best on strict greater-than with scan order as the
tie-break — distance is only a score term, never the primary key; and (b) walk cells in rings outward
with the quarter/half-radius early return and a cell-threat fallback, so farther candidates are not
examined once a nearer-ring candidate exists. Requires parsing the threat-score INI terms not currently
read. Observable contract: same target chosen as gamemd in mixed-threat and crowded engagements.

**`no-targeting-cadence-throttle-4`:**
Order-intent re-acquisition must be gated by a per-unit mission timer using NormalTargetingDelay (27) /
GuardAreaTargetingDelay (36) frames, with the per-dispatch RNG draw gamemd consumes — not run every tick.
Observable contract: idle units lock a newly-arrived enemy on the ~27/36-frame cadence and consume the
matching RNG draws.

**`timing-tick-not-frame-1`:**
The master logic clock must be one logic step per native frame (15Hz). Either run advance_tick at 15Hz,
or ensure every per-step decision (RNG draws, scatter/retaliation/production/ore passes, modulo gates)
fires once per native frame rather than ~3×. Observable contract: unit speeds, fire cadence, animation
and timer cadence match gamemd; per-frame RNG draw count matches.

**`logic-vector-not-driving-tick-3` / `logic-vector-subsystem-order-4`** (per-object pass + subsystem order):
The main per-object AI (movement+target+fire+turret for one object before the next) must run as a single
ordered pass over the live logic vector with the live count reloaded after each object (same-pass append
visibility, compacting-removal skip), positioned at native order 21. The surrounding subsystems must run
in native order: tiberium growth→spread (9-10), bombs (11), teams, disk-laser/laser/lightning/RadSite/EMP,
THEN the object pass, then tactical, factories (27), houses (28). Observable contract: same-tick
interleave (first shot/first move, mid-pass spawns) and RNG cursor sequence match gamemd; ore mutates
before object AI/factories read it.

---

## Needs further research

None. All findings in this batch are CONFIRMED (DRIFT or MISSING) and adversarially verified; the input
`needsResearch` list was empty.

Open sub-questions noted inline but not blocking a verdict:
- Building center-smudge discard-roll stream remains YELLOW in source docs (not assembly-traced for the
  discard portion); survivor/TIBTRE/HouseClass pillars of `rng-tibtre-house-smudge-stream-5` are confirmed.
- `ladder-bridge-shroud-cadence-1`: exact player-visible state RecalcBridgeShroudFlags writes in YR is
  shroud-edge tiles; sizing the stale-edge window impact around gap-generators would refine severity.

---

## Method & caveats

- **Docs-first, live Ghidra spot-checks.** Each finding's gamemd claim was re-verified live this session
  via `decompile_function` / `disassemble_function` / `get_function_callers` / `read_memory` rather than
  trusting doc citations; several doc inaccuracies were corrected during verification (e.g. the Conceal/
  FUN_0055BAE0 attribution in the combat-unregister finding, the g_MainRng-vs-Scen->Random misattribution
  in the ore-RNG finding, the misleading "House_AI_Tick" Ghidra label which is actually a debug HUD).
- **Default-DRIFT burden of proof.** No finding was downgraded without algebraic proof, bit-identical
  boundary test, or exhaustive caller verification. 1-tick / 1-cell / 1-sample differences are kept.
- **Severity = visibility × frequency**, used only to rank, never to drop a finding. Frequency clauses
  are in each row's source finding.
- **TS-legacy excluded:** all confirmed paths are active YR systems; the bridge-shroud recalc and the
  0x1000 fog-of-war SpecialFlag branch were checked and are not what these findings flag.
- **Currently-dormant but kept** (frequency note only, verdict unchanged): `rng-scatter-stream-and-range-2`
  (caller commented out), `hash-mind-controlled-missing` (no writer yet), `rng-mp-seed-handshake-missing-3`
  and `hash-defaulthasher-nondeterminism-risk` (fire only once networked cross-build MP ships).
