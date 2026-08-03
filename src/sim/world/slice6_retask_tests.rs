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
const SLICE6_PRE_LIFECYCLE_V28_HASH: u64 = 0xC2FA_0728_1D1C_CE92;
const SLICE6_PRE_MISSION_V29_HASH: u64 = 0xDC26_BF96_164D_39C2;
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
const SLICE6_BASELINE_HASH: u64 = 0x2DAB_F65B_DAA0_9F67;

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
    let hash = sim.state_hash();
    println!(
        "[slice6] hashes=pre-v28:{pre_lifecycle_hash:016X},pre-v29:{pre_mission_hash:016X},current:{hash:016X}"
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
