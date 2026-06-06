//! Production queue management: enqueue items, advance timers, spawn completed units.
//!
//! Core queue loop driven by `tick_production()`. Handles credit deduction,
//! timer advancement with dynamic rate scaling, and completed-item dispatch.

use std::collections::{BTreeMap, BTreeSet};

use crate::rules::ruleset::RuleSet;
use crate::sim::intern::InternedId;
use crate::sim::miner::{ResourceNode, ResourceType};
use crate::sim::world::Simulation;

use super::production_economy::tick_resource_economy;
use super::production_spawn::{
    find_helipad_for_aircraft, find_spawn_selection_for_owner_with_type,
    mark_war_factory_spawn_contact,
};
use super::production_tech::{
    build_option_for_owner, build_time_base_frames, effective_time_to_build_frames_for_type,
    estimated_real_time_ms, owner_matches_build_identity, production_category_for_object,
    should_use_relaxed_build_mode, supports_live_production,
};
use super::production_types::*;
use super::{CancelOutcome, PRODUCTION_STEPS};

/// Set rally point for an owner's production output.
pub fn set_rally_point_for_owner(sim: &mut Simulation, owner: &InternedId, rx: u16, ry: u16) {
    if let Some(house) = sim.houses.get_mut(owner) {
        house.rally_point = Some((rx, ry));
    }
}

/// Return current rally point for owner, if one has been set.
pub fn rally_point_for_owner(sim: &Simulation, owner: &str) -> Option<(u16, u16)> {
    sim.interner
        .get(owner)
        .and_then(|id| sim.houses.get(&id))
        .and_then(|h| h.rally_point)
}

pub fn credits_for_owner(sim: &Simulation, owner: &str) -> i32 {
    sim.interner
        .get(owner)
        .and_then(|id| sim.houses.get(&id))
        .map(|h| h.credits)
        .unwrap_or(STARTING_CREDITS)
}

pub fn power_balance_for_owner(sim: &Simulation, _rules: &RuleSet, owner: &str) -> (i32, i32) {
    // Read from cached PowerState (health-scaled output, full-rated drain).
    // Updated each tick by power_system::tick_power_states().
    let Some(owner_id) = sim.interner.get(owner) else {
        return (0, 0);
    };
    sim.power_states
        .get(&owner_id)
        .map(|state| (state.total_output, state.total_drain))
        .unwrap_or((0, 0))
}

/// Sum of |Power=| from TypeClass for ALL owned buildings (including under
/// construction). Used by the sidebar power bar fill curve.
pub fn theoretical_power_for_owner(sim: &Simulation, owner: &str) -> i32 {
    let Some(owner_id) = sim.interner.get(owner) else {
        return 0;
    };
    sim.power_states
        .get(&owner_id)
        .map(|state| state.theoretical_total_power)
        .unwrap_or(0)
}

pub(in crate::sim) fn credits_entry_for_owner<'a>(
    sim: &'a mut Simulation,
    owner: &str,
) -> &'a mut i32 {
    let key = sim.interner.intern(owner);
    // Ensure house entry exists (auto-create with defaults if missing).
    // is_human defaults to true: in real games app_init seeds every house
    // with its actual flag, so the only callers that hit this fallback are
    // tests / edge cases that never declared a player. Defaulting to human
    // keeps those paths from accidentally activating AI-only behavior
    // (e.g., AIVirtualPurifiers credit bonus in the deposit path).
    if !sim.houses.contains_key(&key) {
        sim.houses.insert(
            key,
            crate::sim::house_state::HouseState::new(key, 0, None, true, STARTING_CREDITS, 10),
        );
    }
    &mut sim.houses.get_mut(&key).unwrap().credits
}

pub(super) fn next_enqueue_order(sim: &mut Simulation) -> u64 {
    let order = sim.production.next_enqueue_order;
    sim.production.next_enqueue_order = sim.production.next_enqueue_order.saturating_add(1);
    order
}

/// Seed deterministic resource nodes from parsed map overlays.
///
/// Returns how many resource cells were added.
pub fn seed_resource_nodes_from_overlays(
    sim: &mut Simulation,
    overlays: &[crate::map::overlay::OverlayEntry],
    overlay_names: &BTreeMap<u8, String>,
) -> usize {
    let mut added = 0usize;
    let mut warned_ids: BTreeSet<u8> = BTreeSet::new();
    for entry in overlays {
        let Some(name) = overlay_names.get(&entry.overlay_id) else {
            if warned_ids.insert(entry.overlay_id) {
                log::warn!(
                    "Overlay ID {} not in overlay_names -- resource nodes with this ID skipped",
                    entry.overlay_id,
                );
            }
            continue;
        };
        let upper = name.to_ascii_uppercase();
        let is_ore = upper.starts_with("TIB");
        let is_gem = upper.starts_with("GEM");
        if !is_ore && !is_gem {
            continue;
        }
        let richness = u16::from(entry.frame.min(11)).saturating_add(1);
        let base = if is_gem { 180 } else { 120 };
        let stock = base * richness;
        let res_type = if is_gem {
            ResourceType::Gem
        } else {
            ResourceType::Ore
        };
        let key = (entry.rx, entry.ry);
        sim.production
            .resource_nodes
            .entry(key)
            .and_modify(|node| node.remaining = node.remaining.saturating_add(stock))
            .or_insert(ResourceNode {
                resource_type: res_type,
                remaining: stock,
            });
        added += 1;
    }
    added
}

/// Try to enqueue a default buildable unit for `owner`.
///
/// Returns the enqueued type ID on success.
pub fn enqueue_default_unit_for_owner(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: &str,
) -> Option<InternedId> {
    let type_id: InternedId = pick_default_buildable_unit(sim, rules, owner)?;
    let type_str = sim.interner.resolve(type_id).to_string();
    enqueue_by_type(sim, rules, owner, &type_str).then_some(type_id)
}

/// Enqueue a specific unit type.
pub fn enqueue_by_type(sim: &mut Simulation, rules: &RuleSet, owner: &str, type_id: &str) -> bool {
    let relaxed: bool = should_use_relaxed_build_mode(sim, rules, owner);
    let mode = if relaxed {
        BuildMode::PrototypeRelaxed
    } else {
        BuildMode::Strict
    };
    if let Some(opt) = build_option_for_owner(sim, rules, owner, type_id, mode) {
        if !opt.enabled {
            return false;
        }
    } else {
        return false;
    }
    let Some(obj) = rules.object(type_id) else {
        return false;
    };
    if !supports_live_production(obj) {
        return false;
    }
    let queue_category = production_category_for_object(obj);
    let owner_credits = credits_for_owner(sim, owner);
    if obj.cost <= 0 || owner_credits < obj.cost {
        return false;
    }
    let total_base_frames: u32 = build_time_base_frames(rules, obj);
    // The upfront debit is RETIRED at the authority flip: the per-step `advance_one_step`
    // (driven by `step_all` at the Phase-7 head) charges the cost down over the build
    // against the one wallet (`house.credits`). Enqueue only checks affordability (the
    // can-afford-to-START gate above) and appends the queue item.
    let owner_id = sim.interner.intern(owner);
    let type_interned = sim.interner.intern(type_id);
    let enqueue_order = next_enqueue_order(sim);
    let cost = obj.cost.max(0);
    // P5d: append directly to the registry queue-of-record (create-or-append). With no
    // active build the registry arms it inline (the retired reconcile SEED); otherwise it
    // joins the FIFO tail. No upfront debit (the per-step charge owns the cost).
    sim.production.factory_shadow.enqueue(
        owner_id,
        queue_category,
        type_interned,
        enqueue_order,
        total_base_frames,
        cost,
    );
    true
}

/// Build a production list across supported sidebar categories for an owner.
///
/// In RA2, only items the player has unlocked via the tech tree are shown.
/// Items with missing prerequisites, wrong faction, or no factory are hidden
/// entirely — only items with insufficient credits are shown greyed out.
pub fn build_options_for_owner(sim: &Simulation, rules: &RuleSet, owner: &str) -> Vec<BuildOption> {
    let strict: Vec<BuildOption> =
        super::production_tech::build_options_for_owner_mode(sim, rules, owner, BuildMode::Strict);

    // Diagnostic: log reason breakdown when nothing is buildable.
    let enabled_count = strict.iter().filter(|o| o.enabled).count();
    if enabled_count == 0 && sim.tick % 90 == 0 {
        let mut reason_counts: BTreeMap<&str, usize> = BTreeMap::new();
        for opt in &strict {
            let key = match &opt.reason {
                Some(BuildDisabledReason::UnbuildableTechLevel) => "UnbuildableTechLevel",
                Some(BuildDisabledReason::WrongOwner) => "WrongOwner",
                Some(BuildDisabledReason::WrongHouse) => "WrongHouse",
                Some(BuildDisabledReason::ForbiddenHouse) => "ForbiddenHouse",
                Some(BuildDisabledReason::RequiresStolenTech) => "RequiresStolenTech",
                Some(BuildDisabledReason::MissingPrerequisite(_)) => "MissingPrerequisite",
                Some(BuildDisabledReason::NoFactory) => "NoFactory",
                Some(BuildDisabledReason::AtBuildLimit) => "AtBuildLimit",
                Some(BuildDisabledReason::InsufficientCredits) => "InsufficientCredits",
                Some(BuildDisabledReason::PlacementModeUnavailable) => "PlacementModeUnavailable",
                None => "Enabled",
            };
            *reason_counts.entry(key).or_default() += 1;
        }
        log::warn!(
            "[BUILD-DIAG] owner='{}' tick={} total_items={} reasons={:?}",
            owner,
            sim.tick,
            strict.len(),
            reason_counts
        );
        // Log owned structures and their factory status.
        for e in sim.substrate.entities.values() {
            if sim.interner.resolve(e.owner).eq_ignore_ascii_case(owner)
                && e.category == crate::map::entities::EntityCategory::Structure
            {
                let ts = sim.interner.resolve(e.type_ref);
                log::warn!(
                    "[BUILD-DIAG]   structure '{}' building_up={} factory_type={:?}",
                    ts,
                    e.building_up.is_some(),
                    rules.factory_type(ts)
                );
            }
        }
        // Log a few sample failures to show the exact reason per item.
        for opt in strict.iter().filter(|o| !o.enabled).take(5) {
            let type_str = sim.interner.resolve(opt.type_id);
            log::warn!(
                "[BUILD-DIAG]   sample: '{}' reason={:?}",
                type_str,
                opt.reason
            );
        }
    }

    let visible: Vec<BuildOption> = strict
        .into_iter()
        .filter(|opt| {
            opt.enabled
                || matches!(
                    opt.reason,
                    Some(BuildDisabledReason::InsufficientCredits)
                        | Some(BuildDisabledReason::AtBuildLimit)
                )
        })
        .collect();
    let visible = dedupe_visible_build_options(visible, sim, rules, owner, &sim.interner);
    if !visible.is_empty() || !super::production_tech::prototype_fallback_enabled() {
        return visible;
    }
    dedupe_visible_build_options(
        super::production_tech::build_options_for_owner_mode(
            sim,
            rules,
            owner,
            BuildMode::PrototypeRelaxed,
        ),
        sim,
        rules,
        owner,
        &sim.interner,
    )
}

fn dedupe_visible_build_options(
    options: Vec<BuildOption>,
    sim: &Simulation,
    rules: &RuleSet,
    owner: &str,
    interner: &crate::sim::intern::StringInterner,
) -> Vec<BuildOption> {
    let mut deduped: Vec<BuildOption> = Vec::new();
    let mut seen: BTreeMap<(ProductionCategory, String), usize> = BTreeMap::new();

    for option in options {
        let Some(key) = build_option_sidebar_key(rules, &option, interner) else {
            deduped.push(option);
            continue;
        };

        let seen_key = (option.queue_category, key);
        if let Some(existing_idx) = seen.get(&seen_key).copied() {
            let existing = &deduped[existing_idx];
            if prefers_sidebar_variant(sim, rules, owner, &option, existing, interner) {
                deduped[existing_idx] = option;
            }
            continue;
        }

        seen.insert(seen_key, deduped.len());
        deduped.push(option);
    }

    deduped
}

fn build_option_sidebar_key(
    rules: &RuleSet,
    option: &BuildOption,
    interner: &crate::sim::intern::StringInterner,
) -> Option<String> {
    let type_str = interner.resolve(option.type_id);
    let obj = rules.object(type_str)?;
    let image_key = if obj.image.trim().is_empty() {
        obj.id.to_ascii_uppercase()
    } else {
        obj.image.to_ascii_uppercase()
    };
    Some(format!("{}:{image_key}", option.object_category as u8))
}

fn prefers_sidebar_variant(
    sim: &Simulation,
    rules: &RuleSet,
    owner: &str,
    candidate: &BuildOption,
    existing: &BuildOption,
    interner: &crate::sim::intern::StringInterner,
) -> bool {
    sidebar_variant_rank(sim, rules, owner, candidate, interner)
        > sidebar_variant_rank(sim, rules, owner, existing, interner)
}

fn sidebar_variant_rank(
    sim: &Simulation,
    rules: &RuleSet,
    owner: &str,
    option: &BuildOption,
    interner: &crate::sim::intern::StringInterner,
) -> (u8, u16, u8) {
    let type_str = interner.resolve(option.type_id);
    let Some(obj) = rules.object(type_str) else {
        return (0, 0, 0);
    };

    let required_house_match = obj
        .required_houses
        .iter()
        .any(|house| owner_matches_build_identity(sim, owner, house));
    let owner_specificity = u16::MAX.saturating_sub(obj.owner.len() as u16);
    let enabled = option.enabled as u8;

    (required_house_match as u8, owner_specificity, enabled)
}

/// True if this owner has at least one strictly buildable production option.
///
/// This ignores prototype-relaxed fallback and is useful for picking a likely
/// local player house in UI code.
pub fn has_strict_build_option_for_owner(sim: &Simulation, rules: &RuleSet, owner: &str) -> bool {
    super::production_tech::build_options_for_owner_mode(sim, rules, owner, BuildMode::Strict)
        .iter()
        .any(|o| o.enabled)
}

/// Advance production timers and spawn completed items.
pub fn tick_production(
    sim: &mut Simulation,
    rules: &RuleSet,
    height_map: &BTreeMap<(u16, u16), u8>,
    path_grid: Option<&crate::sim::pathfinding::PathGrid>,
    tick_ms: u32,
) -> bool {
    tick_production_with_overlay_registry(sim, rules, height_map, path_grid, None, tick_ms)
}

/// Advance production timers and spawn completed items with optional native
/// tiberium context for harvester-side reduction/reseed.
pub fn tick_production_with_overlay_registry(
    sim: &mut Simulation,
    rules: &RuleSet,
    height_map: &BTreeMap<(u16, u16), u8>,
    path_grid: Option<&crate::sim::pathfinding::PathGrid>,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    tick_ms: u32,
) -> bool {
    if tick_ms == 0 {
        return false;
    }
    let miner_config = crate::sim::miner::MinerConfig::from_general_rules(&rules.general);
    tick_resource_economy(sim, rules, &miner_config, path_grid, overlay_registry);
    // P5d: the registry is the queue-of-record + completion authority. Collect the
    // (owner, category) keys whose active build has completed (progress == 54, object held,
    // not paused), in deterministic temporal (insertion_seq) order — the SAME order
    // step_all charged in and the hash folds. The registry advances (StartNextQueued) on a
    // successful delivery (C7), not on completion alone.
    let completed_keys = sim.production.factory_shadow.completed_keys();
    if completed_keys.is_empty() {
        return false;
    }

    let mut spawned_any = false;
    for (owner_id, queue_category) in completed_keys {
        let owner_str = sim.interner.resolve(owner_id).to_string();
        // The completed-held active object's type.
        let Some(done_type) = sim
            .production
            .factory_shadow
            .view(owner_id, queue_category)
            .and_then(|v| v.object.map(|o| o.type_id))
        else {
            continue;
        };
        let done_type_str = sim.interner.resolve(done_type).to_string();
        let produced_category = rules.object(&done_type_str).map(|o| o.category);
        if produced_category == Some(crate::rules::object_type::ObjectCategory::Building) {
            sim.production
                .ready_by_owner
                .entry(owner_id)
                .or_default()
                .push_back(done_type);
            sim.sound_events
                .push(crate::sim::world::SimSoundEvent::BuildingComplete { owner: owner_id });
            advance_after_delivery(sim, rules, owner_id, queue_category);
            continue;
        }
        let is_vehicle =
            produced_category == Some(crate::rules::object_type::ObjectCategory::Vehicle);
        // Aircraft use helipad spawn path; other units use exit cell path.
        let is_aircraft =
            produced_category == Some(crate::rules::object_type::ObjectCategory::Aircraft);
        let spawn_cell: Option<(u16, u16)>;
        let spawn_producer_id: Option<u64>;
        let helipad_airfield: Option<u64>;

        if is_aircraft {
            if let Some((af_id, rx, ry)) = find_helipad_for_aircraft(sim, rules, &owner_str) {
                spawn_cell = Some((rx, ry));
                spawn_producer_id = Some(af_id);
                helipad_airfield = Some(af_id);
            } else {
                // No free helipad — refund.
                if let Some(obj) = rules.object(&done_type_str) {
                    *credits_entry_for_owner(sim, &owner_str) += obj.cost.max(0);
                }
                advance_after_delivery(sim, rules, owner_id, queue_category);
                continue;
            }
        } else {
            let is_naval: bool = rules.object(&done_type_str).map_or(false, |o| o.naval);
            let spawn_selection = produced_category.and_then(|cat| {
                find_spawn_selection_for_owner_with_type(
                    sim,
                    rules,
                    &owner_str,
                    Some(&done_type_str),
                    cat,
                    path_grid,
                    is_naval,
                )
            });
            spawn_cell = spawn_selection.map(|selection| selection.cell);
            spawn_producer_id = spawn_selection.map(|selection| selection.producer_id);
            helipad_airfield = None;
            if spawn_cell.is_none() {
                if is_vehicle {
                    continue;
                }
                if let Some(obj) = rules.object(&done_type_str) {
                    *credits_entry_for_owner(sim, &owner_str) += obj.cost.max(0);
                }
                advance_after_delivery(sim, rules, owner_id, queue_category);
                continue;
            }
        }
        let (rx, ry) = spawn_cell.unwrap();

        let spawned = sim.spawn_object(&done_type_str, &owner_str, rx, ry, 64, rules, height_map);
        if let Some(stable_id) = spawned {
            if let Some(producer_id) = spawn_producer_id {
                mark_war_factory_spawn_contact(sim, rules, producer_id, stable_id);
            }
            // Aircraft spawned on helipad: reserve dock slot then set
            // DockedIdle carrying the assigned pad index.
            if let Some(af_id) = helipad_airfield {
                let max_slots = sim
                    .substrate.entities
                    .get(af_id)
                    .and_then(|af| {
                        let af_type = sim.interner.resolve(af.type_ref);
                        let af_obj = rules.object(af_type)?;
                        Some(af_obj.number_of_docks.max(1))
                    })
                    .unwrap_or(1);
                let assigned_pad = sim
                    .production
                    .airfield_docks
                    .try_reserve(af_id, stable_id, max_slots)
                    .unwrap_or(0); // Fresh spawn on a single-pad helipad always wins pad 0.
                if let Some(entity) = sim.substrate.entities.get_mut(stable_id) {
                    entity.aircraft_mission =
                        Some(crate::sim::aircraft::AircraftMission::DockedIdle {
                            airfield_id: af_id,
                            pad_index: assigned_pad,
                        });
                }
            }
            sim.sound_events
                .push(crate::sim::world::SimSoundEvent::UnitComplete { owner: owner_id });
            // Auto-move newly produced unit to rally point (if set).
            // Skip for aircraft docked on helipad — they wait for orders.
            if helipad_airfield.is_none() {
                if let (Some(grid), Some((tx, ty))) =
                    (path_grid, rally_point_for_owner(sim, &owner_str))
                {
                    let obj = rules.object(&done_type_str);
                    let loco_mult = sim
                        .substrate.entities
                        .get(stable_id)
                        .and_then(|e| e.locomotor.as_ref())
                        .map(|l| l.speed_multiplier)
                        .unwrap_or(crate::util::fixed_math::SIM_ONE);
                    let speed = obj
                        .map(|o| crate::util::fixed_math::ra2_speed_to_leptons_per_second(o.speed))
                        .unwrap_or(crate::util::fixed_math::ra2_speed_to_leptons_per_second(4));
                    let speed =
                        (speed * loco_mult).max(crate::util::fixed_math::SimFixed::lit("25"));
                    let speed_type = sim
                        .substrate.entities
                        .get(stable_id)
                        .and_then(|e| e.locomotor.as_ref())
                        .map(|l| l.speed_type);
                    let cost_grid = speed_type.and_then(|st| sim.terrain_costs.get(&st));
                    let _ = crate::sim::movement::issue_move_command_with_layered(
                        &mut sim.substrate.entities,
                        grid,
                        stable_id,
                        (tx, ty),
                        speed,
                        false,
                        cost_grid,
                        None,
                        sim.resolved_terrain.as_ref(),
                        sim.zone_grid.as_ref(),
                        None,
                        false, // mover_is_crusher
                    );
                }
            }
            spawned_any = true;
            advance_after_delivery(sim, rules, owner_id, queue_category);
        } else {
            if is_vehicle {
                continue;
            }
            if let Some(obj) = rules.object(&done_type_str) {
                *credits_entry_for_owner(sim, &owner_str) += obj.cost.max(0);
            }
            advance_after_delivery(sim, rules, owner_id, queue_category);
        }
    }

    // P5d: drop any factory left idle (delivered + empty queue) — replaces the
    // `queues_by_owner.retain` prune.
    sim.production.factory_shadow.prune_all_idle();
    spawned_any
}

/// C7 StartNextQueued after a successful delivery (or a completed-but-undeliverable refund):
/// clear the delivered active object and promote the next queued entry into the active slot,
/// cost-seeded from `rules`. Runs in `tick_production` (Phase 7, AFTER `step_all`), so the
/// promoted build's cadence (`step_delay = 0`) starts on the NEXT tick's sweep — never the
/// same tick it is promoted.
fn advance_after_delivery(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner_id: InternedId,
    category: ProductionCategory,
) {
    let next_cost = sim
        .production
        .factory_shadow
        .peek_next_queued(owner_id, category)
        .map(|t| sim.object_type(t, rules).map_or(0, |o| o.cost.max(0)))
        .unwrap_or(0);
    sim.production
        .factory_shadow
        .clear_active_and_advance(owner_id, category, next_cost, 0);
}

/// Build a queue snapshot for one owner, including progress metadata for UI.
///
/// P5d: projects the player-visible build queue from the registry (the queue-of-record),
/// byte-identically to the retired `queues_by_owner` view. Per factory: the active build
/// (head) then its FIFO tail. Sorted by `(category, stamp)` where stamp is the head's
/// `insertion_seq` or a tail entry's `enqueue_order` — algebraically the same order as the
/// retired `(queue_category, enqueue_order)` sort (D1: insertion_seq == active enqueue_order).
///
/// `state` is DERIVED (the `BuildQueueState` field is retired): head -> Paused if `manual`,
/// else Done if complete-held (`progress >= PRODUCTION_STEPS`, the blocked-exit case that
/// persists across ticks), else Building; tail -> Queued. `remaining` is DERIVED from
/// `progress` (the retired B2 mirror: `active_total_base_frames * (54 - progress) / 54`,
/// multiply-then-divide); a queued tail item has not started, so remaining == its total.
pub fn queue_view_for_owner(sim: &Simulation, rules: &RuleSet, owner: &str) -> Vec<QueueItemView> {
    let Some(owner_id) = sim.interner.get(owner) else {
        return Vec::new();
    };
    // (category, stamp, type_id, state, remaining_base_frames, total_base_frames)
    let mut items: Vec<(ProductionCategory, u64, InternedId, BuildQueueState, u32, u32)> =
        Vec::new();
    for f in sim.production.factory_shadow.iter_insertion_ordered() {
        if f.owner != owner_id {
            continue;
        }
        if let Some(obj) = f.object.as_ref() {
            let state = if f.manual {
                BuildQueueState::Paused
            } else if f.progress >= PRODUCTION_STEPS {
                BuildQueueState::Done
            } else {
                BuildQueueState::Building
            };
            let steps_left = PRODUCTION_STEPS.saturating_sub(f.progress.min(PRODUCTION_STEPS));
            let remaining = ((u64::from(f.active_total_base_frames) * u64::from(steps_left))
                / u64::from(PRODUCTION_STEPS)) as u32;
            items.push((
                f.category,
                f.insertion_seq,
                obj.type_id,
                state,
                remaining,
                f.active_total_base_frames,
            ));
        }
        for e in &f.queue {
            items.push((
                f.category,
                e.enqueue_order,
                e.type_id,
                BuildQueueState::Queued,
                e.total_base_frames,
                e.total_base_frames,
            ));
        }
    }
    items.sort_by_key(|&(category, stamp, ..)| (category, stamp));
    items
        .into_iter()
        .map(
            |(queue_category, _stamp, type_id, state, remaining_base, total_base)| {
                let type_str = sim.interner.resolve(type_id);
                let (display_name, remaining_frames, total_frames) = rules
                    .object(type_str)
                    .map(|obj| {
                        (
                            obj.name.clone().unwrap_or_else(|| type_str.to_string()),
                            effective_time_to_build_frames_for_type(
                                sim,
                                rules,
                                owner,
                                type_str,
                                remaining_base,
                            ),
                            effective_time_to_build_frames_for_type(
                                sim,
                                rules,
                                owner,
                                type_str,
                                total_base.max(1),
                            ),
                        )
                    })
                    .unwrap_or_else(|| (type_str.to_string(), remaining_base, total_base.max(1)));
                QueueItemView {
                    type_id,
                    display_name,
                    queue_category,
                    state,
                    remaining_ms: estimated_real_time_ms(remaining_frames, PRODUCTION_RATE_SCALE),
                    total_ms: estimated_real_time_ms(total_frames, PRODUCTION_RATE_SCALE),
                }
            },
        )
        .collect()
}

pub fn ready_buildings_for_owner(
    sim: &Simulation,
    rules: &RuleSet,
    owner: &str,
) -> Vec<ReadyBuildingView> {
    let owner_id = sim.interner.get(owner);
    let ready = owner_id.and_then(|id| sim.production.ready_by_owner.get(&id));
    ready
        .map(|ready| {
            ready
                .iter()
                .filter_map(|&type_id| {
                    let type_str = sim.interner.resolve(type_id);
                    let obj = rules.object(type_str)?;
                    Some(ReadyBuildingView {
                        type_id,
                        display_name: obj.name.clone().unwrap_or_else(|| type_str.to_string()),
                        queue_category: production_category_for_object(obj),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Cancel the most recently queued item for this owner.
///
/// Post-flip refund rule: a queued (tail) item was never charged, so removing it
/// refunds NOTHING; only the active build (a single-item queue, where the most-recent
/// item IS the front) is abandoned with the C8 PARTIAL refund (`original_balance -
/// balance`) routed through the registry against the one wallet (`house.credits`).
pub fn cancel_last_for_owner(sim: &mut Simulation, _rules: &RuleSet, owner: &str) -> bool {
    let owner_id = sim.interner.intern(owner);
    // P5d: the registry owns the queue-of-record. `cancel_last` finds the global-max stamp
    // across the owner's factories (tail-back, else the active build) and removes it — a
    // tail item uncharged (QueuedRemoved), the active build with the C8 PARTIAL refund. The
    // abandon arm only fires for an empty tail, so no StartNextQueued advance is needed.
    let mut registry = std::mem::take(&mut sim.production.factory_shadow);
    let outcome = if let Some(house) = sim.houses.get_mut(&owner_id) {
        let mut wallet = std::mem::take(&mut house.economy);
        wallet.credits = house.credits;
        let outcome = registry.cancel_last(owner_id, &mut wallet);
        house.credits = wallet.credits;
        house.economy = wallet;
        outcome
    } else {
        let mut throwaway = crate::sim::economy::Economy::default();
        registry.cancel_last(owner_id, &mut throwaway)
    };
    registry.prune_all_idle();
    sim.production.factory_shadow = registry;
    matches!(
        outcome,
        CancelOutcome::QueuedRemoved | CancelOutcome::AbandonedActive { .. }
    )
}

/// Route a cancel of `type_id` for (owner, category) through the registry `cancel_one`
/// (the single precedence source: queued-tail FIRST, else active-abandon), charging the
/// C8 partial refund (or none, for a queued copy) against the ONE wallet
/// (`house.credits`) via a per-sweep `Economy` shim. The caller mirrors the resulting
/// queue change into `queues_by_owner`.
fn registry_cancel_active(
    sim: &mut Simulation,
    owner_id: InternedId,
    category: ProductionCategory,
    type_id: InternedId,
) -> CancelOutcome {
    let mut registry = std::mem::take(&mut sim.production.factory_shadow);
    let outcome = if let Some(house) = sim.houses.get_mut(&owner_id) {
        let mut wallet = std::mem::take(&mut house.economy);
        wallet.credits = house.credits; // load the authoritative balance into the shim
        let outcome = registry.cancel_one(owner_id, category, type_id, &mut wallet);
        house.credits = wallet.credits; // store the (possibly refunded) balance back
        house.economy = wallet;
        outcome
    } else {
        // No house to refund into; the cancel still resolves the registry deterministically.
        let mut throwaway = crate::sim::economy::Economy::default();
        registry.cancel_one(owner_id, category, type_id, &mut throwaway)
    };
    sim.production.factory_shadow = registry;
    outcome
}

/// Cancel one queued/active production of `type_id` for this owner (right-click cameo).
///
/// Routed through the registry `cancel_one` (the single precedence source): a QUEUED
/// tail copy is removed FIRST (FIRST front-to-back match, NO refund — a queued item was
/// never charged), else the ACTIVE build is abandoned with the C8 PARTIAL refund
/// (`original_balance - balance`) into the one wallet (`house.credits`). This replaces
/// the legacy `.rev()` last-match + full-cost refund (a DRIFT under the per-step charge).
/// When neither matches (or the build is complete-but-held), falls back to the
/// completed-building ready queue.
pub fn cancel_by_type_for_owner(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: &str,
    type_id: &str,
) -> bool {
    let owner_id = sim.interner.intern(owner);
    let type_interned = sim.interner.intern(type_id);
    // The registry is keyed by ProductionCategory; resolve it from the type (the same
    // routing `enqueue_by_type` used, so the item is in this category's queue).
    let category = match rules.object(type_id) {
        Some(obj) => production_category_for_object(obj),
        None => return cancel_ready_by_type_for_owner(sim, rules, owner, type_id),
    };

    match registry_cancel_active(sim, owner_id, category, type_interned) {
        CancelOutcome::QueuedRemoved => {
            // A queued (tail) copy was removed in the registry; the active build keeps
            // running. Sweep any now-idle factory (none here, but keep it uniform).
            sim.production.factory_shadow.prune_all_idle();
            true
        }
        CancelOutcome::AbandonedActive { .. } => {
            // C7: the active build was abandoned (object cleared, tail intact). Promote the
            // next queued entry into the active slot, cost-seeded. This runs in the COMMAND
            // phase (before this tick's `step_all`), so step_delay = 1 keeps the promoted
            // build's first charge on the next tick (the pre-P5d reconcile-at-tail schedule).
            if let Some(next_type) =
                sim.production.factory_shadow.peek_next_queued(owner_id, category)
            {
                let cost = sim
                    .object_type(next_type, rules)
                    .map(|o| o.cost.max(0))
                    .unwrap_or(0);
                sim.production
                    .factory_shadow
                    .clear_active_and_advance(owner_id, category, cost, 1);
            }
            sim.production.factory_shadow.prune_all_idle();
            true
        }
        CancelOutcome::NoMatch => {
            // Not an active/queued build (or a complete-but-held one) -> the ready queue.
            cancel_ready_by_type_for_owner(sim, rules, owner, type_id)
        }
    }
}

/// Cancel a completed building from the ready_by_owner queue (awaiting placement).
/// Used as fallback when `cancel_by_type_for_owner` finds nothing in the build queue.
fn cancel_ready_by_type_for_owner(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: &str,
    type_id: &str,
) -> bool {
    let owner_id = sim.interner.intern(owner);
    let type_interned = sim.interner.intern(type_id);
    let Some(ready_queue) = sim.production.ready_by_owner.get_mut(&owner_id) else {
        return false;
    };
    // Remove last instance of this type (consistent with queue cancel using .rev()).
    let ready_idx = ready_queue
        .iter()
        .enumerate()
        .rev()
        .find(|(_, tid)| **tid == type_interned)
        .map(|(i, _)| i);
    let Some(idx) = ready_idx else {
        return false;
    };
    ready_queue.remove(idx);
    if ready_queue.is_empty() {
        sim.production.ready_by_owner.remove(&owner_id);
    }
    // Refund full cost.
    if let Some(obj) = rules.object(type_id) {
        *credits_entry_for_owner(sim, owner) += obj.cost.max(0);
    }
    true
}

fn pick_default_buildable_unit(
    sim: &Simulation,
    rules: &RuleSet,
    owner: &str,
) -> Option<InternedId> {
    let mode = if should_use_relaxed_build_mode(sim, rules, owner) {
        BuildMode::PrototypeRelaxed
    } else {
        BuildMode::Strict
    };
    super::production_tech::build_options_for_owner_mode(sim, rules, owner, mode)
        .into_iter()
        .find(|opt| {
            opt.enabled
                && matches!(
                    opt.queue_category,
                    ProductionCategory::Infantry
                        | ProductionCategory::Vehicle
                        | ProductionCategory::Aircraft
                )
        })
        .map(|opt| opt.type_id)
}
