//! Slice 6 — verb API + dispatch-adoption integration tests.
//!
//! Two jobs:
//!   1. `replay_hash_stable_through_slice6` — the behavior-preserving gate. A
//!      scripted skirmish drives every retasking command site (Move / Stop /
//!      Attack / ForceAttack / ForceAttackCell / AttackMove) and asserts the
//!      end-of-run `state_hash()` equals the committed baseline. At the slice's
//!      introduction, this exposed wrong `DockTeardown` subsets and dropped
//!      legacy-field clears. Later hash-schema changes require a separately
//!      proven composition-only re-baseline.
//!   2. The verb-write + retaliation-gate tripwires (added below the gate).

use super::*;
use crate::map::entities::{EntityCategory, MapEntity};
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::combat::AttackTarget;
use crate::sim::command::{Command, CommandEnvelope};
use crate::sim::components::OrderIntent;
use crate::sim::mission::{MissionId, MissionType};
use crate::sim::pathfinding::PathGrid;
use std::collections::BTreeMap;

fn slice6_rules() -> RuleSet {
    // Two attack-capable vehicles + an infantry; ranges short enough that no
    // auto-combat fires during the scripted window (commands drive everything,
    // keeping the RNG stream out of the picture).
    let ini: IniFile = IniFile::from_str(
        "[InfantryTypes]\n0=E1\n\n\
         [VehicleTypes]\n0=MTNK\n\n\
         [AircraftTypes]\n\n\
         [BuildingTypes]\n0=GACNST\n\n\
         [E1]\nLocomotor={4A582744-9839-11d1-B709-00A024DDAFD1}\nStrength=125\nArmor=flak\nSpeed=4\nPrimary=M60\n\n\
         [MTNK]\nLocomotor={4A582741-9839-11d1-B709-00A024DDAFD1}\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=105mm\n\n\
         [GACNST]\nStrength=1000\nArmor=wood\nFoundation=4x3\n\n\
         [M60]\nDamage=25\nROF=20\nRange=5\nWarhead=SA\n\n\
         [105mm]\nDamage=65\nROF=50\nRange=6\nWarhead=AP\n\n\
         [SA]\nVerses=100%,100%,100%,90%,70%,25%,100%,25%,25%,0%,0%\n\n\
         [AP]\nVerses=100%,100%,90%,75%,75%,75%,60%,30%,20%,0%,0%\n",
    );
    RuleSet::from_ini(&ini).expect("slice6 test rules should parse")
}

fn cmd_envelope(
    sim: &Simulation,
    owner: &str,
    execute_tick: u64,
    payload: Command,
) -> CommandEnvelope {
    let owner_id = sim
        .interner
        .get(owner)
        .unwrap_or_else(|| panic!("owner '{owner}' not interned"));
    CommandEnvelope::new(owner_id, execute_tick, payload)
}

fn unit(owner: &str, type_id: &str, cx: u16, cy: u16, cat: EntityCategory) -> MapEntity {
    MapEntity {
        owner: owner.to_string(),
        type_id: type_id.to_string(),
        health: 256,
        cell_x: cx,
        cell_y: cy,
        facing: 64,
        category: cat,
        sub_cell: 0,
        veterancy: 0,
        high: false,
        mission: None,
        recruitable_a: true,
        recruitable_b: true,
        structure_upgrades: [None, None, None],
    }
}

/// The pre-slice baseline. Captured from `dev` BEFORE the Slice-6 edits (run the
/// gate once, read the failure's `left:` value, paste it here). Slice 6 is
/// behavior-preserving, so this constant MUST NOT change for a Slice-6 *behavior*
/// reason. It DOES shift when a later slice adds a new field to the state hash:
/// the scripted scenario has no bunkers, so the value moved only because the
/// tank-bunker lifecycle state (`bunker_link`, `bunker_runtime`) now joins the
/// hash for every entity at its default — a hash-composition change, not a
/// behavior drift. Re-baselined for Slice 7b, then Slice 8 (MissionCom folded
/// into state_hash — every entity now contributes its default mission bytes;
/// composition change, not a behavior drift). Re-baselined for S3 idle→Guard:
/// idle machine-less Units hash mission Guard(5) instead of the legacy None
/// placeholder (hashed-representation fidelity fix; the retask behavior under
/// test is unchanged). Re-baselined for SC-2 (session identity — seed, map
/// name, theater, bounds, MP start table — folded into the hash; composition
/// change, not a behavior drift). Re-measured at the S3 × SC-2 merge (both
/// deltas combined; value from the merged tree's green run). Re-baselined for S4b
/// (the hashed `damage_particle_live_until` `+0x308`-equivalent field — every
/// entity now folds an extra 0; composition change, NOT a behavior drift, proven
/// by the baseline holding unchanged with the fold line disabled). Re-baselined
/// for startup authority: fresh MapGen now uses the verified native Seed(0)
/// logical state instead of the former synthetic all-zero object. `state_hash`
/// already folds MapGen and this fixture does not consume it, so this is the
/// expected corrected initial-state delta, not Slice-6 retask drift. Two no-edit
/// focused probes reproduced this exact value.
/// Re-baselined for lockstep hash completeness: body-facing presence, damage-fire
/// state/animation IDs, locomotor hover/altitude state, and the empty `AnimStore`
/// marker now join the hash. A current-tree legacy-schema probe reproduced the
/// prior `10575654478637980762` value exactly, proving this shift is composition
/// only rather than retask behavior drift.
/// Re-baselined for snapshot/hash schema v28: independent Object lifecycle axes,
/// deterministic lifecycle bookkeeping, and the ordered pending-delete queue
/// now join the lockstep hash. The current-tree legacy-schema probe below still
/// reproduces the prior value exactly; this is a Rust regression ratchet, not
/// gamemd parity evidence.
// Re-baselined for the Phase-0 native-tail and persistence contracts.
// EventClass now dispatches after the object/global walk, so each scripted
// retask first affects object AI on the following frame, as Main_Tick does.
// The common hash also drops diagnostic `total_sim_ms`, hashes only the
// retail-persisted Scenario RNG, and includes the newly persisted deterministic
// fields. This is an intentional behavior-bearing retail correction, so both
// provenance probes move with the current hash.
// Re-baselined 2026-08-02, same provenance as the global harness constants:
// the mover is 190490ba "match retail cell occupation lifecycle", found by
// bisecting dev..HEAD against this very probe (it is this test's FIRST assert,
// and this fixture has no ore, no resource_nodes, no overlay grid and no
// RNG-consuming path, which is what makes it a clean isolator).
//
// The composition-only hypothesis was tested and REFUTED: with the merge-base
// (6f78bac7) world_hash.rs swapped in and all branch behaviour kept, this probe
// read 0xFEEA0679D9429547 — neither the old baseline nor the branch value. So
// hashed state content changed, not just which fields are folded.
// MERGE 2026-08-03: both branches re-baselined these independently (dev:
// passive acquire + spawner; foundations: Move cadence + hashed runtime
// state). Neither side's values describe the merged tree; re-derived below
// from the merged tree's own output in the same merge commit.
// Native Move mission cadence now advances MissionCom and Scenario RNG.
// Re-baselined after hashing the newly persisted YR runtime-contract state.
/// Re-baselined 2026-08-04 for the GSI-07.02 constructed-`Rate` default
/// (0 -> 14 frames when a mission section or its `Rate=` key is absent).
/// `slice6_rules()` declares no mission sections, so every mission in this
/// fixture left the zero sentinel. Provenance note in
/// global_parity_harness_tests.rs at FINAL_STREAM_STATES.
// Re-baselined 2026-08-05 for the Drive cell-admission slice, with the schema
// and behaviour halves SEPARATED BY MEASUREMENT rather than argued.
// `DriveLocomotionRuntime` gained `occupation_handoff`, and the whole struct is
// hashed by its derived `Hash`, so an `Option` discriminant enters the fold for
// every vehicle even while the field is `None`.
//
// Experiment, no `world_hash.rs` change: keep the field, neutralise every
// behaviour writer that landed with it (the fresh-selection admission gate, the
// chained-curve refusal on the two temporary codes, the handoff mark at both
// install sites, the forced-track pre-clear), re-run. Three points, all
// observed in this tree:
//
//   committed (pre-change) : pre-v28 661EF70FF1847F63  pre-v29 599A096466E0970A  current 5BC6E9E7EEA3E80D
//   schema only            : pre-v28 2A9B661C382F5DC9  pre-v29 AD6A5FDD5EA76570  current EC8407239DC80358
//   schema + behaviour     : pre-v28 728141E6E7EE9CBA  pre-v29 5EA7E13419353AB0  current 22D855782E72D55E
//
// So unlike the global harness — whose entire shift is schema — this 16-tick
// scripted fixture carries BOTH: the field's discriminant moves it once, and
// the admission gate moves it again because its Move/AttackMove script does
// reach the selection lane. Rust regression ratchet, not gamemd evidence.
// Re-measured 2026-08-11 after the integrated GSI-04 authority changes. All
// three probes moved together, so this is state-content/behavior drift rather
// than a live-schema-only fold; values are from the same current-tree run.
// Re-baselined 2026-08-11 after the v69 merge folded the serialized projectile
// collision, Wave, Cell infantry-owner, fog/sensor, and cloak-owner substrates
// into every hash schema. This fixture does not exercise those producers; the
// shift is the intentional explicit-zero/empty composition change.
// Re-baselined 2026-08-13 after GSI-05.01 replaced the ProjectileStore,
// WaveStore, and ProductionState local next-ID sources with the shared global
// `ObjectSubstrate::next_stable_object_id`. The shared source remains hashed;
// removing the three obsolete local counter folds shifts all compositions.
// Re-baselined 2026-08-14 for the v77 GSI-13.06 SHP body-cadence state.
// `body_frame_counter` now contributes one persisted dword per entity, while
// the derived Drive/Ship runtime hashes include the persisted signed owner-
// speed carrier used by the native moving predicate. These folds sit outside
// the v28/v29 gates, so both named legacy probes intentionally move with the
// live composition; filtering them out would create an undocumented hybrid
// schema rather than reproduce either probe's stated contract.
// Re-baselined 2026-08-18: a mid-curve re-order now keeps the in-flight Drive
// curve and anchors the new path at its committed head cell (`TechnoClass::
// Set_Destination` @ 0x00741970 never touches the track cursor; see
// movement_commands.rs). This fixture retasks a moving MTNK at ticks 1/3/5/7,
// so its trajectory legitimately changes — previously each retask restarted
// the curve at the cell lead-in, teleporting the body backward. Behavior-
// bearing, NOT composition-only: position/movement state is hashed in every
// schema, so all three probes move together. Rust regression ratchet; the
// kept-curve contract is exercised by
// `movement_tests::test_reissue_mid_curve_keeps_track_and_anchors_path_at_head`.
// Re-baselined 2026-08-19 with the GSI-08.12 veterancy accumulator: the raw
// float bits and the rank cache are now hashed per object, a composition-only
// shift. No RNG draw is added — `VeterancyClass::Add @ 0x0074FF50` consumes
// none — and record/replay stayed equal at every tick.
// Re-baselined for TechnoClass::TechnoClass @ 0x006F2B90: all three authored
// Technos now consume the raw Scenario word stored at 0x006F3254. That
// behavior-bearing Scenario shift reaches every schema probe, and record/replay
// equality remains exact.
// Re-baselined 2026-08-30 for GSI-04.03 Drive slope payload ownership. The
// common locomotor fold reaches both historical probes; the scripted retasks
// and record/replay equality remain exact, so this is composition-only.
// Re-baselined 2026-09-02 for the native tiberium queue store (OQ-38, bridge transaction 3
// slice D): every class now carries the native entry array, float min-heap, capacity, and
// `native_rect`, rebuilds walk `CellIterator` order, and spread admission applies the
// `FirstObject` occupier gate. This is behavior-bearing on every fixture with ore, so the
// historical probes move as well; the RNG stream tuple and tick-for-tick record/replay
// equality remain exact.
const SLICE6_PRE_LIFECYCLE_V28_HASH: u64 = 0xDE9E_7A6C_EC29_29D1;
const SLICE6_PRE_MISSION_V29_HASH: u64 = 0xED16_9102_5536_B67E;
// Snapshot/hash schema v29 adds lossless Mission dwords, readiness leaves,
// suspended Target/falling state, and raw locomotor-ready inputs. The two
// schema probes below must prove the shift is composition-only before updating
// this live regression value.
// Re-baselined (all three constants) for the Mission authority flip:
// commands queue through the exact authority and the host promotes; the
// legacy per-tick projection is deleted, so every hashed mission value —
// including the reduced subset the legacy pre-v29 composition folds —
// changes together. Behavior-bearing Rust ratchet, not gamemd evidence.
//
// Re-baselined 2026-07-29 for the locomotion S2 readiness producers, twice:
// first for Drive/Ship/Teleport/Jumpjet, then again when Walk and Hover joined
// them so all six live families now write `mission_ready_state`. The ceremony this
// file requires — the two schema probes proving the shift is composition-only —
// is satisfied: BOTH probes above are unchanged and green, so only the
// current-schema hash moved, i.e. the delta is the hashed field going
// `None → Some` and not a behaviour divergence. Independently confirmed by
// neutralising the behaviour path (making the readiness gate ignore the
// produced state while the producers still ran): the hash was identical to
// this new value, so the deferral change contributes nothing here.
//
// Re-baselined 2026-07-30 when the readiness inputs stopped being stored on the
// locomotor and became derived at the Mission gate instead. gamemd's readiness
// virtual performs a fresh locomotor call at every one of its ~two dozen call
// sites, with no cached per-frame flag anywhere on that path, so a per-tick
// cache answered nearly all of them with stale state; verified from the Infantry
// and Unit readiness overrides and the queue-then-commence caller.
//
// Composition-only here, and this file's ceremony proves it: BOTH schema probes
// above are unchanged and green. The pre-v29 probe still hashes every position,
// facing and movement field, so if the derivation had changed *when* any unit
// commenced, that probe would have moved too. It did not — only the
// current-schema hash did, and its delta is exactly the removed readiness bytes.
// (Behaviour-neutral in THIS fixture is not behaviour-neutral in general: the
// paths the change exists for — dock, unlink, unload and deploy handoffs that
// stop a unit and queue-and-commence in the same tick — are not covered here.)
// Rust regression ratchet, not gamemd evidence.
//
// Re-baselined 2026-07-30 for S3b: the installed LocomotorSlot joins the hash.
// **Composition-only, proved by neutralisation** — the ceremony this file
// normally uses cannot decide it, because the locomotor block is hashed
// unconditionally, so BOTH schema probes move with the live value. Instead the
// new hash line was commented out and the whole suite re-run: all three
// constants returned to their previous committed values exactly, so the
// primary_kind -> slot retype changed no behaviour and no other hashed state,
// and the entire delta is the one new byte. The absolute per-stream RNG pins
// and the dense-scenario position fingerprint were unchanged throughout.
// Re-baselined 2026-07-30 for S5: the locomotor `powered` flag joins the hash.
// Composition-only, proved by neutralisation (the probe ceremony cannot decide
// it — the locomotor block is hashed unconditionally, so both probes move with
// the live value). With the new hash line commented out, all three constants
// returned to their S3b values exactly, which also proves the three power edges
// wired in this slice (deploy-begin off, undeploy-complete on, destination-
// accepted on) changed no other hashed state in these fixtures. The absolute
// per-stream RNG pins held throughout.
// MERGE 2026-08-03: both branches re-baselined these independently (dev:
// passive acquire + spawner; foundations: Move cadence + hashed runtime
// state). Neither side's values describe the merged tree; re-derived below
// from the merged tree's own output in the same merge commit.
//
// Re-baselined 2026-08-02 for passive/opportunity target acquisition.
// **Composition-only, but NOT for the reason this file's usual ceremony would
// suggest.** Both schema probes above run with the v29 block excluded, and every
// field this slice adds or changes — `passive_scan_timer`,
// `last_target_scan_frame`, `passively_acquired_target` — lives inside that
// block. So the probes are structurally incapable of moving for this change and
// prove nothing about it either way. Do not read their staying green as
// evidence.
//
// What actually isolates it is the fixture: it runs 16 ticks, short of the
// object's 45-frame initial scan delay, so no scan fires, no draw is consumed
// and no target is installed. The entire delta is therefore the three v29-block
// fields moving off their old values — `passive_scan_timer` armed at the
// construction frame instead of left unarmed, plus the two new fields folded at
// their defaults. Rust regression ratchet, not gamemd evidence.
/// Re-baselined 2026-08-05 for the Drive cell-admission slice; the measured
/// schema/behaviour split for all three of this fixture's constants is written
/// out at `SLICE6_PRE_LIFECYCLE_V28_HASH`.
// Current-schema value from the same 2026-08-11 measurement above.
// Re-baselined 2026-08-13 for the same global-ID hash-composition change above.
// Re-baselined 2026-08-14 for the same v77 authoritative-state composition
// described at the two legacy probes above.
// Re-baselined 2026-08-14 for v44 entity-animation hash authority. Both legacy
// probes remain exact, isolating this to the current-schema composition change.
// Re-baselined 2026-08-15 for `14e096ff`: authoritative animation timing now
// comes from the RuleSet in headless/replay frames, so this fixture's E1 advances
// the already-hashed v44 entity-animation cursor. Both legacy probes remain
// exact, isolating the change from the retask behavior under test. This is a
// Rust regression ratchet; the native sequence-timing sources are documented
// by that commit, while non-stock READY/GUARD precedence remains UNCHECKED.
// Re-baselined 2026-08-18 for the kept in-flight Drive curve on mid-curve
// re-orders; see the dated comment at the two legacy probes above.
// Re-baselined 2026-08-19 with the GSI-08.12 veterancy accumulator: the raw
// float bits and the rank cache are now hashed per object, a composition-only
// shift. No RNG draw is added — `VeterancyClass::Add @ 0x0074FF50` consumes
// none — and record/replay stayed equal at every tick.
// Re-baselined 2026-08-20 for the v87 TechnoClass+0x3D5 membership byte.
// The pre-v28 and pre-v29 probes above remained byte-identical, isolating this
// to the intentional current-schema hash composition change.
// Re-baselined 2026-08-20 for v88's deposited-sensor presence discriminator.
// None of this fixture's types has SensorsSight; both historical probes stayed
// byte-identical and the same-run hash is stable, proving a composition-only
// current-schema move.
// Re-baselined 2026-08-24 for snapshot/hash schema v90: current hashing now
// folds the exact serialized real `CellClass+0x140 & 0x1180` value authority
// once behind its schema tag. The pre-v28 and pre-v29 probes above remain
// byte-identical, and record/replay remains exact, proving composition-only.
// Re-baselined 2026-08-26 for snapshot/hash schemas v92-v102: Phase 3 adds
// empty/default base-reservation, House strategy/base-defense, TeamScript VM,
// AIMD/TeamType/TaskForce, and typed AITrigger authority to the current hash.
// Both historical probes remain byte-identical; this 16-tick fixture creates
// no Teams or AI registries, so the shift is current-schema composition only.
// Re-baselined 2026-08-26 for snapshot/hash schema v103: the empty
// `active_wave_links` map and empty persisted destroyable-cliff mutation map
// now join the current hash. Both historical probes remain byte-identical;
// this fixture creates no Wave or cliff mutation, so the measured shift is
// current-schema composition only.
// Techno constructor RNG also persists in the current-schema Techno fields;
// the two legacy probes above isolate the shared behavior-bearing portion.
// Re-baselined 2026-08-30 for the same slope payload move; both historical
// probes shift consistently and the replay remains deterministic.
// Re-baselined 2026-08-30 for v107's Spark shared-dummy tag plus level/slope
// folds. Both historical probes remain byte-identical, so only current-schema
// composition moved.
// Re-baselined 2026-08-30 for v110's unconditional ordered BasePlan authority.
// The dedicated pre-v110 probe reproduces the prior baseline exactly, isolating
// the measured shift to current-schema composition.
// Re-baselined 2026-09-01 for v114's unconditional raw 256-slot crate authority.
// The dedicated pre-v114 probe reproduces the prior current baseline exactly;
// this fixture's behavior and record/replay equality remain unchanged.
// Re-baselined 2026-09-01 for v115's retained wall-neighbor count authority mode and
// shared-dummy overlay identity/state folds. The dedicated pre-v115 probe reproduces the
// prior current baseline exactly; this fixture builds a legacy `None`-count grid, so only
// current-schema composition moved.
const SLICE6_PRE_BASE_PLAN_V110_HASH: u64 = 0xCEBC_0EC3_08AA_C9B7;
const SLICE6_PRE_CRATE_AUTHORITY_V114_HASH: u64 = 0xF6D7_06A1_4979_2F14;
const SLICE6_PRE_WALL_RUNTIME_V115_HASH: u64 = 0x4193_B427_916C_B830;
const SLICE6_BASELINE_HASH: u64 = 0x00EE_8FFD_59DA_F4E6;

#[test]
fn replay_hash_stable_through_slice6() {
    let rules = slice6_rules();
    let heights: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);
    let mut sim = Simulation::new();
    // id 1: Americans MTNK (the unit we retask). id 2: enemy MTNK (Soviet, hostile
    // by default — no alliance entry). id 3: Americans E1 (second attacker).
    sim.spawn_from_map(
        &[
            unit("Americans", "MTNK", 3, 3, EntityCategory::Unit),
            unit("Soviet", "MTNK", 25, 3, EntityCategory::Unit),
            unit("Americans", "E1", 5, 5, EntityCategory::Infantry),
        ],
        Some(&rules),
        &heights,
    );

    // (execute_tick, command) — apply_due_commands fires each when self.session.tick+1 == tick.
    let script: &[(u64, Command)] = &[
        (
            1,
            Command::Move {
                entity_id: 1,
                target_rx: 10,
                target_ry: 10,
                queue: false,
                group_id: None,
            },
        ),
        (
            3,
            Command::AttackMove {
                entity_id: 1,
                target_rx: 15,
                target_ry: 3,
                queue: false,
            },
        ),
        (
            5,
            Command::ForceAttackCell {
                attacker_id: 1,
                target_rx: 18,
                target_ry: 3,
            },
        ),
        (
            7,
            Command::ForceAttack {
                attacker_id: 1,
                target_id: 2,
            },
        ),
        (
            9,
            Command::Attack {
                attacker_id: 3,
                target_id: 2,
            },
        ),
        (11, Command::Stop { entity_id: 1 }),
    ];

    for tick in 0..16u64 {
        let due: Vec<CommandEnvelope> = script
            .iter()
            .filter(|(t, _)| *t == tick + 1)
            .map(|(t, c)| cmd_envelope(&sim, "Americans", *t, c.clone()))
            .collect();
        let _ = sim.advance_tick(&due, Some(&rules), &heights, Some(&grid), None, 67);
    }

    let pre_lifecycle_hash = sim.state_hash_before_lifecycle_v28_and_mission_v29();
    let pre_mission_hash = sim.state_hash_without_mission_v29();
    let pre_base_plan_hash = sim.state_hash_without_base_plan_v110();
    let pre_crate_authority_hash = sim.state_hash_without_crate_authority_v114();
    let pre_wall_runtime_hash = sim.state_hash_without_wall_runtime_v115();
    let hash = sim.state_hash();
    println!(
        "[slice6] hashes=pre-v28:{pre_lifecycle_hash:016X},pre-v29:{pre_mission_hash:016X},pre-v110:{pre_base_plan_hash:016X},pre-v114:{pre_crate_authority_hash:016X},pre-v115:{pre_wall_runtime_hash:016X},current:{hash:016X}"
    );
    assert_eq!(
        pre_lifecycle_hash, SLICE6_PRE_LIFECYCLE_V28_HASH,
        "pre-v28/pre-v29 schema probe must reproduce the historical baseline"
    );
    assert_eq!(
        pre_mission_hash, SLICE6_PRE_MISSION_V29_HASH,
        "v29 provenance probe must reproduce the prior live v28 baseline; otherwise this is behavior drift"
    );
    assert_eq!(
        pre_base_plan_hash, SLICE6_PRE_BASE_PLAN_V110_HASH,
        "the dedicated pre-v110 probe must reproduce the prior Slice 6 baseline"
    );
    assert_eq!(
        pre_crate_authority_hash, SLICE6_PRE_CRATE_AUTHORITY_V114_HASH,
        "the dedicated pre-v114 probe must reproduce the prior Slice 6 current baseline"
    );
    assert_eq!(
        pre_wall_runtime_hash, SLICE6_PRE_WALL_RUNTIME_V115_HASH,
        "the dedicated pre-v115 probe must reproduce the prior Slice 6 current baseline"
    );
    assert_eq!(
        hash, SLICE6_BASELINE_HASH,
        "Slice 6 scripted-retask state hash drifted. Treat this as behavior drift \
         unless a documented legacy-schema or equivalent provenance check proves \
         that an intentional hash-composition change is solely responsible"
    );
}

#[test]
fn slice6_move_command_retasks_via_mission_substrate_and_clears_state() {
    // A Move command must route through the compatibility boundary: the mission
    // substrate's `current` becomes Move (checked BEFORE any tick-tail shadow
    // refresh) AND the legacy conflicting fields are cleared.
    let rules = slice6_rules();
    let heights: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let grid = PathGrid::new(64, 64);
    let mut sim = Simulation::new();
    sim.spawn_from_map(
        &[unit("Americans", "MTNK", 3, 3, EntityCategory::Unit)],
        Some(&rules),
        &heights,
    );
    // Seed a conflicting prior order the Move must tear down.
    {
        let e = sim.substrate.entities.get_mut(1).expect("unit");
        e.attack_target = Some(AttackTarget::new(2));
        e.order_intent = Some(OrderIntent::Guard {
            anchor_rx: 3,
            anchor_ry: 3,
        });
    }

    let issued = sim.apply_command(
        "Americans",
        &Command::Move {
            entity_id: 1,
            target_rx: 10,
            target_ry: 10,
            queue: false,
            group_id: None,
        },
        Some(&rules),
        Some(&grid),
        &heights,
    );
    assert!(issued, "move command should issue");

    let e = sim.substrate.entities.get(1).expect("unit");
    assert_eq!(
        e.mission.queued(),
        MissionId::from_known(MissionType::Move),
        "the command queued Move through the exact authority (host promotes later)"
    );
    assert!(
        e.attack_target.is_none(),
        "Move tore down the attack target"
    );
    assert!(e.order_intent.is_none(), "Move tore down the order intent");
}

#[test]
fn slice6_retaliation_still_suppressed_for_guarding_unit() {
    // The retaliation gate keeps the LITERAL `order_intent.is_some()` predicate
    // (a guarding unit must NOT retaliate). Tripwire: if the gate were swapped to
    // an `is_busy`-only check, the guard would retaliate and the first assert
    // would fail. The positive control (cleared intent) proves the suppression is
    // caused by `order_intent`, not by an absent valid weapon matchup.
    let rules = slice6_rules();
    let heights: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let mut sim = Simulation::new();
    sim.spawn_from_map(
        &[
            unit("Americans", "MTNK", 3, 3, EntityCategory::Unit),
            unit("Soviet", "MTNK", 4, 3, EntityCategory::Unit),
        ],
        Some(&rules),
        &heights,
    );
    {
        let guard = sim.substrate.entities.get_mut(1).expect("guard");
        guard.last_attacker_id = Some(2);
        guard.order_intent = Some(OrderIntent::Guard {
            anchor_rx: 3,
            anchor_ry: 3,
        });
        assert!(guard.attack_target.is_none());
    }

    crate::sim::combat::tick_retaliation(&mut sim.substrate.entities, &rules, &sim.interner, &[1]);
    assert!(
        sim.substrate
            .entities
            .get(1)
            .unwrap()
            .attack_target
            .is_none(),
        "a guarding unit (order_intent = Guard) must NOT retaliate — the literal \
         order_intent gate suppresses it"
    );
    // last_attacker is preserved (the gate `continue`s before the clear).
    assert_eq!(
        sim.substrate.entities.get(1).unwrap().last_attacker_id,
        Some(2)
    );

    // Positive control: drop the order intent and the SAME unit now retaliates,
    // confirming the suppression above was due to `order_intent`, not the matchup.
    sim.substrate.entities.get_mut(1).unwrap().order_intent = None;
    crate::sim::combat::tick_retaliation(&mut sim.substrate.entities, &rules, &sim.interner, &[1]);
    assert!(
        sim.substrate
            .entities
            .get(1)
            .unwrap()
            .attack_target
            .is_some(),
        "with no order intent the unit retaliates (gate no longer suppresses)"
    );
}
