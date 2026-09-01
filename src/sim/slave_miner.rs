//! Slave Miner system — deploy/undeploy, slave spawn, slave harvest AI, scan correction.
//!
//! The Slave Miner (SMIN) is Yuri's harvester. Unlike War/Chrono Miners it does NOT
//! harvest directly. Instead it deploys into a refinery building (YAREFN) and spawns
//! SLAV infantry who do the actual harvesting. Slaves pick up ore bales, walk them
//! back to the deployed master, and deposit credits directly (no dock queue needed).
//!
//! ## Key behaviors (from RA2/YR)
//! - **Deploy**: SMIN vehicle → YAREFN building + spawn `SlavesNumber` (5) SLAV infantry
//! - **Undeploy**: YAREFN building → SMIN vehicle, slaves recalled/killed
//! - **Slave harvest loop**: SearchOre → MoveToOre → Harvest → ReturnToMaster → Deposit
//! - **Slave regen**: Dead slaves respawn after `SlaveRegenRate` (500) frames
//! - **Scan correction**: Deployed YAREFN periodically checks if a closer ore patch exists
//!   (SlaveMinerKickFrameDelay=150 frames, SlaveMinerScanCorrection=3 cells improvement)
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/miner, sim/miner_system, rules/.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::rules::ruleset::RuleSet;
use crate::sim::components::{BuildingDown, Health};
use crate::sim::economy::apply_income_mult;
use crate::sim::house_state::{house_state_for_owner_mut, income_ppm_for_owner};
use crate::sim::intern::InternedId;
use crate::sim::miner::miner_system::{
    effective_purifier_count, is_cell_path_clear_for_scan, resource_cell_present,
    search_local_resource,
};
use crate::sim::miner::{CargoBale, MinerConfig};
use crate::sim::miner::{extract_bale, search_local_ore};
use crate::sim::pathfinding::PathGrid;
use crate::sim::production::credits_entry_for_owner;
use crate::sim::world::{PlacementEvidence, Simulation, UnitDeployOutcome};

/// Deployed state of a Slave Miner (SMIN vehicle ↔ YAREFN building).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SlaveMinerMode {
    /// SMIN vehicle form — moving toward ore field to deploy.
    Mobile,
    /// SMIN → YAREFN deploy animation in progress.
    Deploying,
    /// YAREFN building form — slaves are active.
    Deployed,
    /// YAREFN → SMIN undeploy animation in progress.
    Undeploying,
}

/// Slave harvest AI state machine — one per SLAV infantry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SlaveHarvestState {
    /// Looking for nearest ore cell within scan radius of master.
    SearchOre,
    /// Walking toward the target ore cell.
    MoveToOre,
    /// Extracting bales from the ore cell.
    Harvest,
    /// Walking back to the deployed master (YAREFN).
    ReturnToMaster,
    /// Depositing cargo at the master — credits awarded immediately.
    Deposit,
    /// No ore found — idle near master.
    Idle,
}

/// ECS component for SLAV infantry — attached to slave entities.
///
/// Each slave has its own mini harvest loop: find ore near master,
/// walk to it, harvest, walk back, deposit at master, repeat.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SlaveHarvester {
    /// Stable ID of the master entity (deployed YAREFN or mobile SMIN).
    pub master_id: u64,
    /// Current harvest AI state.
    pub state: SlaveHarvestState,
    /// Cargo bales currently carried by this slave.
    pub cargo: Vec<CargoBale>,
    /// Max bales this slave can carry (Storage=4 for SLAV).
    pub capacity: u16,
    /// Countdown timer for harvest action (HarvestRate=150 frames).
    pub harvest_timer: u32,
    /// The ore cell being targeted.
    pub target_cell: Option<(u16, u16)>,
}

impl SlaveHarvester {
    /// Create a new SlaveHarvester bound to a master entity.
    pub fn new(master_id: u64, capacity: u16) -> Self {
        Self {
            master_id,
            state: SlaveHarvestState::SearchOre,
            cargo: Vec::with_capacity(capacity as usize),
            capacity,
            harvest_timer: 0,
            target_cell: None,
        }
    }

    /// True when cargo is at capacity.
    pub fn is_full(&self) -> bool {
        self.cargo.len() as u16 >= self.capacity
    }

    /// Total credit value of all bales currently carried.
    pub fn cargo_value(&self) -> u32 {
        self.cargo.iter().map(|b| b.value as u32).sum()
    }
}

/// Snapshot of one slave entity for two-phase processing.
struct SlaveSnapshot {
    entity_id: u64,
    owner: InternedId,
    rx: u16,
    ry: u16,
    harvester: SlaveHarvester,
}

/// Slave-side combined scan filter. Mirrors `miner_system::build_scan_filter`'s
/// occupancy/path-grid check; zone reachability is skipped (slaves anchor to
/// the master refinery and the slave path planner handles per-step passability).
fn build_slave_scan_filter<'a>(
    sim: &'a Simulation,
    path_grid: Option<&'a PathGrid>,
    self_id: u64,
) -> Box<dyn Fn((u16, u16)) -> bool + 'a> {
    let occupancy = &sim.substrate.occupancy;
    Box::new(move |cell: (u16, u16)| {
        is_cell_path_clear_for_scan(occupancy, path_grid, cell, self_id)
    })
}

/// Tick all slave harvesters. Called once per sim tick from resource economy.
///
/// Uses the two-phase snapshot pattern: snapshot → process → write back.
pub(super) fn tick_slave_harvesters(
    sim: &mut Simulation,
    live_order: &[u64],
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
) {
    // Phase 1: Snapshot all slave harvesters.
    let mut snapshots: Vec<SlaveSnapshot> = Vec::new();
    for &id in live_order {
        let Some(entity) = sim.substrate.entities.get(id) else {
            continue;
        };
        let Some(ref sh) = entity.slave_harvester else {
            continue;
        };
        snapshots.push(SlaveSnapshot {
            entity_id: id,
            owner: entity.owner,
            rx: entity.position.rx,
            ry: entity.position.ry,
            harvester: sh.clone(),
        });
    }

    if snapshots.is_empty() {
        return;
    }

    // Phase 2: Process each slave.
    for snap in &mut snapshots {
        process_slave(sim, rules, config, path_grid, overlay_registry, snap);
    }

    // Phase 3: Write back.
    for snap in &snapshots {
        if let Some(entity) = sim.substrate.entities.get_mut(snap.entity_id) {
            entity.slave_harvester = Some(snap.harvester.clone());
        }
    }
}

/// Process one slave through its harvest state machine.
fn process_slave(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    snap: &mut SlaveSnapshot,
) {
    // Check master is still alive.
    if sim
        .substrate
        .entities
        .get(snap.harvester.master_id)
        .is_none()
    {
        // Master destroyed — slave becomes idle (in real RA2, freed slaves wander).
        snap.harvester.state = SlaveHarvestState::Idle;
        return;
    }

    match snap.harvester.state {
        SlaveHarvestState::SearchOre => {
            handle_slave_search(sim, rules, config, path_grid, overlay_registry, snap)
        }
        SlaveHarvestState::MoveToOre => handle_slave_move_to_ore(snap),
        SlaveHarvestState::Harvest => {
            handle_slave_harvest(sim, rules, config, overlay_registry, snap)
        }
        SlaveHarvestState::ReturnToMaster => handle_slave_return(sim, snap),
        SlaveHarvestState::Deposit => handle_slave_deposit(sim, rules, config, snap),
        SlaveHarvestState::Idle => {
            handle_slave_idle(sim, rules, config, path_grid, overlay_registry, snap)
        }
    }
}

/// Slave searches for ore within SlaveMinerSlaveScan of the master.
fn handle_slave_search(
    sim: &Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    snap: &mut SlaveSnapshot,
) {
    let scan_radius: u16 = rules.general.slave_miner_slave_scan.max(1) as u16;

    // Search from the master's position (slaves harvest around their deployed base).
    let master_pos = sim
        .substrate
        .entities
        .get(snap.harvester.master_id)
        .map(|e| (e.position.rx, e.position.ry))
        .unwrap_or((snap.rx, snap.ry));

    let scan_filter = build_slave_scan_filter(sim, path_grid, snap.entity_id);
    let filter_ref: Option<&dyn Fn((u16, u16)) -> bool> = Some(&*scan_filter);
    if let Some(cell) = search_local_resource(
        sim,
        rules,
        overlay_registry,
        master_pos,
        scan_radius,
        filter_ref,
        config,
    ) {
        snap.harvester.target_cell = Some(cell);
        snap.harvester.state = SlaveHarvestState::MoveToOre;
    } else {
        snap.harvester.state = SlaveHarvestState::Idle;
    }
}

/// Slave moving toward ore. Simplified: instant arrival if at target cell.
/// Real movement is driven by the locomotor system; here we check proximity.
fn handle_slave_move_to_ore(snap: &mut SlaveSnapshot) {
    let Some(target) = snap.harvester.target_cell else {
        snap.harvester.state = SlaveHarvestState::SearchOre;
        return;
    };

    // If the slave has arrived at (or is adjacent to) the target, start harvesting.
    let dx: i32 = snap.rx as i32 - target.0 as i32;
    let dy: i32 = snap.ry as i32 - target.1 as i32;
    if dx.abs() <= 1 && dy.abs() <= 1 {
        snap.harvester.state = SlaveHarvestState::Harvest;
        snap.harvester.harvest_timer = 0;
    }
    // Movement itself is handled by the locomotor/movement system.
}

/// Slave extracting bales from the ore cell.
fn handle_slave_harvest(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    snap: &mut SlaveSnapshot,
) {
    let Some(cell) = snap.harvester.target_cell else {
        snap.harvester.state = SlaveHarvestState::SearchOre;
        return;
    };

    // Check if ore still exists at target.
    let has_ore = resource_cell_present(sim, rules, overlay_registry, cell);

    if !has_ore {
        // Ore depleted — search for more.
        snap.harvester.target_cell = None;
        if snap.harvester.cargo.is_empty() {
            snap.harvester.state = SlaveHarvestState::SearchOre;
        } else {
            snap.harvester.state = SlaveHarvestState::ReturnToMaster;
        }
        return;
    }

    // Harvest timer countdown.
    if snap.harvester.harvest_timer > 0 {
        snap.harvester.harvest_timer -= 1;
        return;
    }

    // Extract one bale.
    if let Some(bale) = extract_bale(sim, rules, overlay_registry, cell, config) {
        snap.harvester.cargo.push(bale);
    }

    if snap.harvester.is_full() {
        snap.harvester.state = SlaveHarvestState::ReturnToMaster;
    } else {
        // Reset harvest timer (HarvestRate from rules, stored on the ObjectType).
        // Default 150 frames at RA2's 15fps logic = 10 seconds.
        // At 15Hz sim, 1 tick = 1 RA2 game frame, so use directly.
        snap.harvester.harvest_timer = SLAVE_HARVEST_RATE_TICKS;
    }
}

/// Default harvest rate for slaves (HarvestRate=150 in rulesmd.ini).
/// In a fully data-driven version this would come from the SLAV ObjectType's
/// `harvest_rate` field. For now, use the standard value.
const SLAVE_HARVEST_RATE_TICKS: u32 = 150;

/// Slave returning to master. Check proximity to master position.
fn handle_slave_return(sim: &Simulation, snap: &mut SlaveSnapshot) {
    let Some(master) = sim.substrate.entities.get(snap.harvester.master_id) else {
        snap.harvester.state = SlaveHarvestState::Idle;
        return;
    };
    let mx: u16 = master.position.rx;
    let my: u16 = master.position.ry;

    let dx: i32 = snap.rx as i32 - mx as i32;
    let dy: i32 = snap.ry as i32 - my as i32;

    // Adjacent or on master cell → start depositing.
    if dx.abs() <= 2 && dy.abs() <= 2 {
        snap.harvester.state = SlaveHarvestState::Deposit;
    }
    // Movement handled by locomotor system.
}

/// Slave depositing cargo at master — credits awarded immediately per bale.
fn handle_slave_deposit(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    snap: &mut SlaveSnapshot,
) {
    if snap.harvester.cargo.is_empty() {
        snap.harvester.state = SlaveHarvestState::SearchOre;
        return;
    }

    // Pop one bale per tick (slaves deposit faster than refinery unload). amount = 1 bale.
    let bale: CargoBale = snap.harvester.cargo.remove(0);
    let value: i32 = i32::from(bale.value);

    let owner_str = sim.interner.resolve(snap.owner).to_string();
    // P7: per-country IncomeMult (single truncation, 1.0/identity on stock). The base
    // credit + the HarvestedCredits stat (1 bale × 5) accrue first.
    let income_ppm = income_ppm_for_owner(&sim.houses, &sim.interner, rules, &owner_str);
    let base_credits = apply_income_mult(value, income_ppm);

    // Ore Purifier bonus stacks per real purifier owned by the slave's owner; non-human
    // houses also receive the AI virtual-purifier bonus. Single-truncation credit + stat
    // (the shared economy helpers), amount = 1 bale.
    let purifier_count = effective_purifier_count(sim, rules, &owner_str);
    let bonus_ppm = rules.general.purifier_bonus_ppm;
    let bonus_credits =
        crate::sim::economy::purifier_bonus_credits(value, purifier_count, bonus_ppm, income_ppm);

    {
        let credits: &mut i32 = credits_entry_for_owner(sim, &owner_str);
        *credits = credits.saturating_add(base_credits.saturating_add(bonus_credits));
    }
    // HarvestedCredits stat (statistics-only): base 1 bale × 5, plus the single-truncation
    // bonus term trunc(count × 0.25 × 1 × 5).
    if let Some(h) = house_state_for_owner_mut(&mut sim.houses, &owner_str, &sim.interner) {
        h.economy.add_harvested(1);
        h.economy
            .add_harvested_raw(crate::sim::economy::purifier_bonus_harvested(
                1,
                purifier_count,
                bonus_ppm,
            ));
    }

    // Keep depositing until empty. Unload tick interval for slaves is instant
    // (one bale per tick) since HarvesterDumpRate doesn't apply to slaves.
    // After empty, we'll transition to SearchOre next tick.
    let _ = config; // config available for future tuning
}

/// Slave idle — periodically re-scan for ore.
fn handle_slave_idle(
    sim: &Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    snap: &mut SlaveSnapshot,
) {
    // Try to find ore every few ticks (reuse search logic).
    let scan_radius: u16 = rules.general.slave_miner_slave_scan.max(1) as u16;
    let master_pos = sim
        .substrate
        .entities
        .get(snap.harvester.master_id)
        .map(|e| (e.position.rx, e.position.ry))
        .unwrap_or((snap.rx, snap.ry));

    let scan_filter = build_slave_scan_filter(sim, path_grid, snap.entity_id);
    let filter_ref: Option<&dyn Fn((u16, u16)) -> bool> = Some(&*scan_filter);
    if let Some(cell) = search_local_resource(
        sim,
        rules,
        overlay_registry,
        master_pos,
        scan_radius,
        filter_ref,
        config,
    ) {
        snap.harvester.target_cell = Some(cell);
        snap.harvester.state = SlaveHarvestState::MoveToOre;
    }
}

// ---------------------------------------------------------------------------
// Slave Miner deploy/undeploy
// ---------------------------------------------------------------------------

/// Configuration for the Slave Miner deploy system, derived from rules.
pub struct SlaveMinerConfig {
    /// Number of slaves to spawn on deploy (SlavesNumber=5).
    pub slaves_number: i32,
    /// Frames between slave respawns after death (SlaveRegenRate=500).
    pub slave_regen_rate: u32,
    /// Minimum frames between consecutive respawns (SlaveReloadRate=25).
    pub slave_reload_rate: u32,
    /// Frames between scan correction checks (SlaveMinerKickFrameDelay=150).
    pub kick_frame_delay: u32,
    /// Cell improvement threshold for scan correction (SlaveMinerScanCorrection=3).
    pub scan_correction: u16,
    /// Short scan radius for deployed slave miner (SlaveMinerShortScan=8).
    pub short_scan: u16,
    /// Slave scan radius — how far slaves search for ore (SlaveMinerSlaveScan=14).
    pub slave_scan: u16,
}

impl SlaveMinerConfig {
    /// Build from parsed GeneralRules.
    pub fn from_rules(rules: &RuleSet) -> Self {
        let g = &rules.general;
        Self {
            slaves_number: 5, // overridden per-object from ObjectType.slaves_number
            slave_regen_rate: g.slave_miner_kick_frame_delay.max(1), // actually SlaveRegenRate per-obj
            slave_reload_rate: 25,
            kick_frame_delay: g.slave_miner_kick_frame_delay.max(1),
            scan_correction: g.slave_miner_scan_correction.max(0) as u16,
            short_scan: g.slave_miner_short_scan.max(1) as u16,
            slave_scan: g.slave_miner_slave_scan.max(1) as u16,
        }
    }
}

/// Deploy a Slave Miner (SMIN) vehicle into its refinery form (YAREFN).
///
/// The newly constructed YAREFN first creates its own slave pool. Native
/// `PowerUp_Cleanup @ 0x006AF580` then destroys that temporary manager and
/// transfers the SMIN's existing pool, retaining every constructor RNG draw.
///
/// Returns the new YAREFN stable_id only when conversion commits. `None` means
/// either rejection or an accepted target-facing turn whose later Unit AI retry
/// still owns the commit.
pub fn deploy_slave_miner(sim: &mut Simulation, stable_id: u64, rules: &RuleSet) -> Option<u64> {
    let outcome: UnitDeployOutcome =
        sim.begin_unit_deploy_into_building_with_overlay_context(stable_id, rules, None);
    outcome.deployed_id()
}

/// Commit the already-placed and already-facing SMIN/YAREFN specialization of
/// the shared UnitClass forward transaction. The shared owner must prove the
/// target foundation and `DeployFacing` before entering here.
pub(crate) fn commit_aligned_slave_miner_deploy_with_overlay_context(
    sim: &mut Simulation,
    stable_id: u64,
    target_type: &str,
    rx: u16,
    ry: u16,
    rules: &RuleSet,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
) -> Option<u64> {
    // Read deploy data before mutating.
    let deploy_data = {
        let entity = sim.substrate.entities.get(stable_id)?;
        let type_str = sim.interner.resolve(entity.type_ref);
        let obj = rules.object_case_insensitive(type_str)?;
        let authored_target = obj.deploys_into.as_deref()?;
        if !authored_target.eq_ignore_ascii_case(target_type) {
            return None;
        }
        rules.object(target_type)?;
        let enslaves: String = obj.enslaves.clone()?;
        let slaves_number: i32 = obj.slaves_number.max(0);
        let owner_str = sim.interner.resolve(entity.owner).to_string();
        Some((
            owner_str,
            entity.position.z,
            entity.selected,
            entity.attached_trigger_tag,
            entity.health,
            enslaves,
            slaves_number,
        ))
    }?;

    let (owner, z, was_selected, attached_trigger_tag, source_health, slave_type, slaves_number) =
        deploy_data;

    // Construct and reveal the YAREFN before consuming any source state. A
    // rejected target Unlimbo leaves the SMIN, tag, and slave manager intact.
    let new_sid: u64 = sim.spawn_deploy_target_building_at_height_with_overlay_context(
        target_type,
        &owner,
        rx,
        ry,
        0,
        z,
        rules,
        overlay_registry,
    )?;

    #[cfg(test)]
    sim.trace_lifecycle_for_test(
        crate::sim::world::LifecycleTestEvent::ForwardDeployTargetConstructed {
            stable_id: new_sid,
            scenario_rng_state: sim.scenario_rng.state(),
        },
    );

    if let Some(target) = sim.substrate.entities.get_mut(new_sid) {
        target.health.current =
            Simulation::active_retail_conversion_health_current(source_health, target.health.max);
        target.initialize_building_damage_state_gate(rules.general.condition_yellow_x1000);
    }

    // Preserve any existing manager/bindings when this is a redeploy after a
    // retail-style YAREFN -> SMIN reverse conversion.
    let existing_slave_ids = sim.production.slave_bindings.remove(&stable_id);

    let slave_capacity: u16 = rules
        .object_case_insensitive(&slave_type)
        .map(|obj| obj.storage.max(1) as u16)
        .unwrap_or(4);

    let slave_ids = if let Some(existing_slave_ids) = existing_slave_ids {
        // The YAREFN constructor has already created and drawn for this pool.
        // Destroy it before installing the old manager, exactly as the native
        // brain-transplant helper does.
        let discarded = sim.discard_constructor_owned_slave_pool(new_sid);
        debug_assert!(discarded, "YAREFN constructor must own a fresh slave pool");

        let mut live_slave_ids = Vec::with_capacity(existing_slave_ids.len());
        for slave_sid in existing_slave_ids {
            if let Some(slave_entity) = sim.substrate.entities.get_mut(slave_sid) {
                slave_entity.slave_harvester = Some(SlaveHarvester::new(new_sid, slave_capacity));
                live_slave_ids.push(slave_sid);
            }
        }
        sim.production
            .slave_bindings
            .insert(new_sid, live_slave_ids.clone());
        live_slave_ids
    } else {
        // A legacy/source object without a represented manager cannot donate
        // one. Keep the manager the target constructor just created.
        sim.production
            .slave_bindings
            .get(&new_sid)
            .cloned()
            .unwrap_or_default()
    };

    // Preserve the existing Rust timing: once the building form exists, wake
    // its retained limbo slaves around it without reconstructing them.
    for (i, slave_sid) in slave_ids
        .into_iter()
        .enumerate()
        .take(slaves_number as usize)
    {
        let in_limbo = sim
            .substrate
            .entities
            .get(slave_sid)
            .is_some_and(|slave| slave.lifecycle.in_limbo);
        if !in_limbo {
            continue;
        }
        let offset_x = (i as i32 % 3) - 1;
        let offset_y = (i as i32 / 3) - 1;
        let sx = (rx as i32 + offset_x).clamp(0, u16::MAX as i32) as u16;
        let sy = (ry as i32 + offset_y).clamp(0, u16::MAX as i32) as u16;
        let _ = sim.unlimbo_held_production_object(
            slave_sid,
            sx,
            sy,
            0,
            z,
            PlacementEvidence::EvaluateMark,
            rules,
        );
    }

    // `PowerUp_Cleanup @ 0x006AF580` has now installed the donated manager.
    // UnitClass::Deploy selects the target, moves the AttachedTag reference,
    // and only then destroys the old Unit, so pointer-expiry observers see the
    // new masters and the complete player-visible handoff.
    if let Some(target) = sim.substrate.entities.get_mut(new_sid) {
        target.selected = was_selected;
    }
    sim.substrate
        .entities
        .get_mut(new_sid)
        .expect("successfully spawned YAREFN remains stored")
        .attached_trigger_tag = attached_trigger_tag;
    if attached_trigger_tag.is_some()
        && let Some(source) = sim.substrate.entities.get_mut(stable_id)
    {
        source.attached_trigger_tag = None;
    }
    sim.uninit_with_rules(stable_id, rules);

    Some(new_sid)
}

/// Complete a deferred Slave Miner refinery (YAREFN) reverse conversion.
///
/// The caller owns `BuildingDown` scheduling and consumes that component before
/// entering this transaction. The owned payload supplies every serialized and
/// v115-hashed spawn input; `source_health` is sampled at completion, matching
/// `BuildingClass::Sell @ 0x00449E66..0x00449E70` rather than command start.
///
/// 1. Construct a complete SMIN plus its fresh manager while YAREFN is active
/// 2. Limbo the YAREFN footprint and attempt SMIN Unlimbo exactly once
/// 3. On failure, refund/destroy YAREFN and retain the fresh SMIN pool in limbo
/// 4. On success only, replace that pool with the YAREFN manager and transfer tags
pub(crate) fn complete_slave_miner_undeploy_with_overlay_context(
    sim: &mut Simulation,
    stable_id: u64,
    building_down: BuildingDown,
    attached_trigger_tag: Option<InternedId>,
    source_type_id: InternedId,
    source_health: Health,
    rules: &RuleSet,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
) -> Option<u64> {
    let source = sim.substrate.entities.get(stable_id)?;
    if !source.lifecycle.object_alive || source.lifecycle.in_limbo {
        return None;
    }
    let target_type = sim.interner.resolve(building_down.spawn_type).to_string();
    let owner = sim.interner.resolve(building_down.spawn_owner).to_string();
    let rx = building_down.spawn_rx;
    let ry = building_down.spawn_ry;
    let z = building_down.spawn_z;
    let was_selected = building_down.was_selected;
    let deploy_facing = rules
        .object_case_insensitive(sim.interner.resolve(source_type_id))
        .map_or(0x80, |object| object.deploy_facing);

    // BuildingClass::Sell @ 0x00449C30 constructs the target Unit and all of
    // its constructor-owned children while the source Building is still live.
    let Some(new_sid) =
        sim.construct_object_limbo_at_height(&target_type, &owner, rx, ry, deploy_facing, z, rules)
    else {
        let refund = crate::sim::production::active_retail_reverse_refund_for_building(
            sim, rules, stable_id,
        )
        .unwrap_or(0);
        crate::sim::production::credit_reverse_failure_refund(sim, &owner, refund);
        if sim.production.slave_bindings.contains_key(&stable_id) {
            sim.production
                .reverse_failure_slave_manager_finalizers
                .insert(stable_id);
        }
        sim.uninit_with_rules(stable_id, rules);
        return None;
    };

    // Save the source-derived refund before detaching its footprint. A rejected
    // Unit Unlimbo is destructive: no source restoration, no transfer, and no
    // cleanup of the constructed target or its fresh SlaveManager pool.
    let refund =
        crate::sim::production::active_retail_reverse_refund_for_building(sim, rules, stable_id)
            .unwrap_or(0);
    let _ = sim.techno_limbo_with_rules(stable_id, rules);
    if sim
        .reveal_constructed_object_at_height_with_unit_context(
            new_sid,
            rx,
            ry,
            deploy_facing,
            z,
            PlacementEvidence::EvaluateMark,
            rules,
            overlay_registry,
            new_sid,
        )
        .is_none()
    {
        crate::sim::production::credit_reverse_failure_refund(sim, &owner, refund);
        if sim.production.slave_bindings.contains_key(&stable_id) {
            sim.production
                .reverse_failure_slave_manager_finalizers
                .insert(stable_id);
        }
        sim.uninit_with_rules(stable_id, rules);
        return None;
    }

    if let Some(target) = sim.substrate.entities.get_mut(new_sid) {
        target.health.current =
            Simulation::active_retail_conversion_health_current(source_health, target.health.max);
    }

    let slave_ids = sim.production.slave_bindings.remove(&stable_id);

    if let Some(slave_ids) = slave_ids {
        // The SMIN constructor's own fresh pool and its draws already exist;
        // PowerUp_Cleanup destroys that pool before transferring the YAREFN
        // manager and rewriting every surviving child's master pointer.
        let discarded = sim.discard_constructor_owned_slave_pool(new_sid);
        debug_assert!(discarded, "SMIN constructor must own a fresh slave pool");
        let mut live_slave_ids = Vec::with_capacity(slave_ids.len());
        for slave_id in slave_ids {
            if let Some(slave) = sim.substrate.entities.get_mut(slave_id) {
                if let Some(ref mut harvester) = slave.slave_harvester {
                    harvester.master_id = new_sid;
                }
                live_slave_ids.push(slave_id);
            }
        }
        sim.production
            .slave_bindings
            .insert(new_sid, live_slave_ids);
    }

    if let Some(target) = sim.substrate.entities.get_mut(new_sid) {
        target.attached_trigger_tag = attached_trigger_tag;
    }
    if attached_trigger_tag.is_some()
        && let Some(source) = sim.substrate.entities.get_mut(stable_id)
    {
        source.attached_trigger_tag = None;
    }
    if let Some(target) = sim.substrate.entities.get_mut(new_sid) {
        target.selected = was_selected;
    }
    sim.uninit_with_rules(stable_id, rules);

    Some(new_sid)
}

/// Physical-destructor fallback for a represented SlaveManager whose master
/// reached TechnoClass destruction without an earlier attacker/new-house
/// `MasterDestroyed` call.
///
/// Active-retail evidence:
/// - `TechnoClass__Destructor @ 0x006F4500` calls
///   `SlaveManagerClass__MasterDestroyed(manager, null, null) @ 0x006B0AE0`
///   before deleting the manager;
/// - MasterDestroyed walks controls in reverse, clears each SlaveOwner
///   back-reference first, UnInits limbo slaves, and liberates visible slaves
///   to the stock Civilian/Neutral house;
/// - the failed reverse target owns a different freshly constructed manager,
///   so finalizing the source must never touch that target-keyed pool.
///
/// The no-Civilian mod branch applies Rules.C4Warhead, and attacker-caused
/// master death supplies the attacker before this destructor fallback. Those
/// broader contexts remain outside this active-retail null/null boundary.
pub(crate) fn finalize_active_retail_slave_manager_null_attacker(
    sim: &mut Simulation,
    master_id: u64,
    rules: Option<&RuleSet>,
) {
    let Some(slave_ids) = sim.production.slave_bindings.get(&master_id).cloned() else {
        return;
    };
    let master_cell = sim
        .substrate
        .entities
        .get(master_id)
        .map(|master| (master.position.rx, master.position.ry));
    let mut any_slave_survived = false;

    // The native fallback resolves the Civilian country, then scans the live
    // House array in order. Stock skirmish's matching house is Neutral. Preserve
    // session House order where available; test/diagnostic worlds fall back to
    // the deterministic house map without inventing a missing house.
    let neutral_owner = {
        let is_neutral_house = |owner: InternedId| {
            sim.interner.resolve(owner).eq_ignore_ascii_case("Neutral")
                || sim
                    .houses
                    .get(&owner)
                    .and_then(|house| house.country)
                    .is_some_and(|country| {
                        sim.interner
                            .resolve(country)
                            .eq_ignore_ascii_case("Neutral")
                    })
        };
        sim.session
            .house_order
            .iter()
            .copied()
            .find(|&owner| sim.houses.contains_key(&owner) && is_neutral_house(owner))
            .or_else(|| {
                sim.houses
                    .keys()
                    .copied()
                    .find(|&owner| is_neutral_house(owner))
            })
    };

    for slave_id in slave_ids.iter().rev().copied() {
        let Some((object_alive, in_limbo)) = sim
            .substrate
            .entities
            .get(slave_id)
            .map(|slave| (slave.lifecycle.object_alive, slave.lifecycle.in_limbo))
        else {
            continue;
        };
        if !object_alive {
            continue;
        }

        // Native clears Techno+0x2DC before either the limbo-UnInit arm or the
        // visible-liberation arm. `SlaveHarvester` is the represented owner
        // back-reference as well as the slave AI admission gate.
        if let Some(slave) = sim.substrate.entities.get_mut(slave_id) {
            slave.slave_harvester = None;
        }

        if in_limbo {
            if let Some(rules) = rules {
                sim.uninit_with_rules(slave_id, rules);
            } else {
                sim.uninit(slave_id);
            }
            continue;
        }

        let Some(neutral_owner) = neutral_owner else {
            // Active stock always has Neutral. Until the C4Warhead branch is a
            // represented shared SlaveManager death context, remove this
            // otherwise-orphaned mod-world slave through ordinary UnInit.
            if let Some(rules) = rules {
                sim.uninit_with_rules(slave_id, rules);
            } else {
                sim.uninit(slave_id);
            }
            continue;
        };

        if let Some(rules) = rules {
            sim.change_owner_with_rules(slave_id, neutral_owner, rules);
        } else {
            sim.change_owner(slave_id, neutral_owner);
        }
        if let Some(slave) = sim.substrate.entities.get_mut(slave_id) {
            crate::sim::movement::clear_navigation_for_entity(slave);
            slave.attack_target = None;
            slave.passively_acquired_target = false;
            slave.order_intent = None;
        }
        let now = sim.session.binary_frame;
        let _ = sim.mission_assign_exact(
            slave_id,
            crate::sim::mission::MissionId::from_known(crate::sim::mission::MissionType::Guard),
            now,
        );
        any_slave_survived = true;
    }

    if any_slave_survived && let Some((rx, ry)) = master_cell {
        sim.sound_events
            .push(crate::sim::world::SimSoundEvent::SlaveWorkerLiberated { rx, ry });
    }

    // Equivalent to manager.owner=null followed by the manager's scalar
    // destructor. Children that were UnInit above remain queued and are
    // eligible later in this same live pending-delete drain.
    sim.production.slave_bindings.remove(&master_id);
}

// ---------------------------------------------------------------------------
// Slave regeneration
// ---------------------------------------------------------------------------

/// Tick slave regeneration for all deployed Slave Miners.
///
/// When a slave dies (removed from entity store), spawn a replacement after
/// `SlaveRegenRate` ticks. `SlaveReloadRate` is the minimum gap between spawns.
pub(super) fn tick_slave_regen(
    sim: &mut Simulation,
    live_order: &[u64],
    rules: &RuleSet,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
) {
    // Preserve a dead-but-stored master only when reverse failure armed the
    // proved null/null destructor context. Every other dead/absent binding
    // retains the pre-M39 cleanup behavior until attacker-aware combat
    // MasterDestroyed is represented.
    let stale_master_ids = sim
        .production
        .slave_bindings
        .keys()
        .copied()
        .filter(|&master_id| {
            let master_is_alive = sim
                .substrate
                .entities
                .get(master_id)
                .is_some_and(|master| master.lifecycle.object_alive);
            !master_is_alive
                && !sim
                    .production
                    .reverse_failure_slave_manager_finalizers
                    .contains(&master_id)
        })
        .collect::<Vec<_>>();
    for master_id in stale_master_ids {
        sim.production.slave_bindings.remove(&master_id);
    }

    // Regeneration is master AI work and therefore follows authoritative
    // LogicVector order. An explicit empty vector performs no regeneration.
    let master_ids = live_order
        .iter()
        .copied()
        .filter(|master_id| sim.production.slave_bindings.contains_key(master_id))
        .collect::<Vec<_>>();

    for master_id in master_ids {
        let Some(master) = sim.substrate.entities.get(master_id) else {
            // Master died — clean up bindings.
            sim.production.slave_bindings.remove(&master_id);
            continue;
        };

        let master_type = sim.interner.resolve(master.type_ref).to_string();
        let owner = sim.interner.resolve(master.owner).to_string();
        let mrx: u16 = master.position.rx;
        let mry: u16 = master.position.ry;
        let mz: u8 = master.position.z;

        // Get expected slave count and slave type from rules.
        let (slave_type, max_slaves) = {
            let obj = match rules.object_case_insensitive(&master_type) {
                Some(o) => o,
                None => continue,
            };
            let st = match obj.enslaves.as_deref() {
                Some(s) => s.to_string(),
                None => continue,
            };
            (st, obj.slaves_number.max(0))
        };

        // Count living slaves.
        let slave_ids = match sim.production.slave_bindings.get(&master_id) {
            Some(ids) => ids.clone(),
            None => continue,
        };
        let alive_count: i32 = slave_ids
            .iter()
            .filter(|&&sid| sim.substrate.entities.get(sid).is_some())
            .count() as i32;

        // Remove dead slave IDs from bindings.
        let alive_ids: Vec<u64> = slave_ids
            .iter()
            .copied()
            .filter(|&sid| sim.substrate.entities.get(sid).is_some())
            .collect();

        if alive_count < max_slaves {
            // Spawn one replacement per tick (SlaveReloadRate could throttle this,
            // but for simplicity we spawn one per tick until full).
            let sx: u16 = mrx.saturating_add(1);
            let sy: u16 = mry.saturating_add(1);

            if let Some(slave_sid) = sim.spawn_object_at_height_with_overlay_context(
                &slave_type,
                &owner,
                sx,
                sy,
                0,
                mz,
                rules,
                overlay_registry,
            ) {
                let slave_capacity: u16 = rules
                    .object_case_insensitive(&slave_type)
                    .map(|obj| obj.storage.max(1) as u16)
                    .unwrap_or(4);

                if let Some(slave_entity) = sim.substrate.entities.get_mut(slave_sid) {
                    slave_entity.slave_harvester =
                        Some(SlaveHarvester::new(master_id, slave_capacity));
                }
                let mut updated_ids: Vec<u64> = alive_ids;
                updated_ids.push(slave_sid);
                sim.production.slave_bindings.insert(master_id, updated_ids);
            } else {
                sim.production.slave_bindings.insert(master_id, alive_ids);
            }
        } else if alive_ids.len() != slave_ids.len() {
            // Just clean up dead IDs.
            sim.production.slave_bindings.insert(master_id, alive_ids);
        }
    }
}

// ---------------------------------------------------------------------------
// Scan correction (Phase 7)
// ---------------------------------------------------------------------------

/// Check if a deployed Slave Miner should reposition to a closer ore patch.
///
/// Called periodically (every SlaveMinerKickFrameDelay ticks). If the nearest
/// ore from the master's position is `SlaveMinerScanCorrection` cells closer
/// than the current nearest ore to the slaves, trigger an undeploy + move.
///
/// Returns Some((rx, ry)) = cell to reposition to, None = stay put.
pub fn check_scan_correction(
    sim: &Simulation,
    rules: &RuleSet,
    path_grid: Option<&PathGrid>,
    master_id: u64,
) -> Option<(u16, u16)> {
    let master = sim.substrate.entities.get(master_id)?;
    let mrx: u16 = master.position.rx;
    let mry: u16 = master.position.ry;

    let short_scan: u16 = rules.general.slave_miner_short_scan.max(1) as u16;
    let correction: u16 = rules.general.slave_miner_scan_correction.max(0) as u16;
    let cfg = MinerConfig::from_rules(rules);

    let scan_filter = build_slave_scan_filter(sim, path_grid, master_id);
    let filter_ref: Option<&dyn Fn((u16, u16)) -> bool> = Some(&*scan_filter);

    // Find nearest ore from current position.
    let current_nearest = search_local_ore(
        &sim.production.resource_nodes,
        (mrx, mry),
        short_scan,
        filter_ref,
        cfg.ore_bale_value,
        cfg.gem_bale_value,
    )?;

    let current_dist: u16 = manhattan_distance(mrx, mry, current_nearest.0, current_nearest.1);

    // Search the broader area (SlaveMinerLongScan) for a better patch.
    let long_scan: u16 = rules.general.slave_miner_long_scan.max(1) as u16;
    let better_ore = search_local_ore(
        &sim.production.resource_nodes,
        (mrx, mry),
        long_scan,
        filter_ref,
        cfg.ore_bale_value,
        cfg.gem_bale_value,
    )?;

    let better_dist: u16 = manhattan_distance(mrx, mry, better_ore.0, better_ore.1);

    // If the improvement exceeds SlaveMinerScanCorrection, recommend repositioning.
    if current_dist > better_dist && (current_dist - better_dist) >= correction {
        Some(better_ore)
    } else {
        None
    }
}

/// Manhattan distance between two cells.
fn manhattan_distance(ax: u16, ay: u16, bx: u16, by: u16) -> u16 {
    ax.abs_diff(bx) + ay.abs_diff(by)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use crate::sim::command::{Command, CommandEnvelope};
    use crate::sim::house_state::HouseState;
    use crate::sim::miner::ResourceType;
    use crate::sim::mission::{MissionId, MissionType};
    use crate::sim::rng::SimRng;
    use crate::sim::world::LifecycleTestEvent;

    fn complete_undeploy_for_test(
        sim: &mut Simulation,
        stable_id: u64,
        rules: &RuleSet,
    ) -> Option<u64> {
        if !sim.undeploy_building(stable_id, rules) {
            return None;
        }
        let (building_down, attached_trigger_tag, source_type_id, source_health) = {
            let source = sim.substrate.entities.get_mut(stable_id)?;
            (
                source.building_down.take()?,
                source.attached_trigger_tag,
                source.type_ref,
                source.health,
            )
        };
        complete_slave_miner_undeploy_with_overlay_context(
            sim,
            stable_id,
            building_down,
            attached_trigger_tag,
            source_type_id,
            source_health,
            rules,
            None,
        )
    }

    #[test]
    fn slave_harvester_capacity_and_value() {
        let mut sh = SlaveHarvester::new(100, 4);
        assert!(!sh.is_full());
        assert_eq!(sh.cargo_value(), 0);

        for _ in 0..4 {
            sh.cargo.push(CargoBale {
                resource_type: ResourceType::Ore,
                value: 25,
            });
        }
        assert!(sh.is_full());
        assert_eq!(sh.cargo_value(), 100);
    }

    #[test]
    fn slave_harvester_state_transitions() {
        let sh = SlaveHarvester::new(1, 4);
        assert_eq!(sh.state, SlaveHarvestState::SearchOre);
    }

    #[test]
    fn undeploy_transfers_slave_manager_to_new_smin() {
        let rules = make_test_rules();
        let seed = 0x51A7_E001;
        let mut sim = Simulation::with_seed(seed);
        let mut expected = SimRng::new(seed);
        let sm_bld = sim
            .spawn_object_at_height("SMIN", "YuriCountry", 10, 10, 0, 0, &rules)
            .expect("spawn SMIN");
        for _ in 0..6 {
            let _ = expected.next_u32();
        }
        assert_eq!(sim.scenario_rng.logical_state(), expected.logical_state());
        let yarefn = deploy_slave_miner(&mut sim, sm_bld, &rules).expect("deploy to YAREFN");
        for _ in 0..6 {
            let _ = expected.next_u32();
        }
        assert_eq!(sim.scenario_rng.logical_state(), expected.logical_state());
        let slave_ids = sim
            .production
            .slave_bindings
            .get(&yarefn)
            .cloned()
            .expect("YAREFN should own slave manager");
        let tag = sim.interner.intern("TAG_YAREFN_UNDEPLOY");
        sim.substrate
            .entities
            .get_mut(yarefn)
            .unwrap()
            .attached_trigger_tag = Some(tag);
        {
            let source = sim.substrate.entities.get_mut(yarefn).unwrap();
            source.health.current = 777;
            source.building_up = None;
        }
        assert_eq!(slave_ids, vec![2, 3, 4, 5, 6]);
        for discarded_id in 8..=12 {
            assert!(sim.substrate.entities.get(discarded_id).is_none());
        }

        let smin = complete_undeploy_for_test(&mut sim, yarefn, &rules).expect("undeploy to SMIN");
        for _ in 0..6 {
            let _ = expected.next_u32();
        }
        assert_eq!(sim.scenario_rng.logical_state(), expected.logical_state());
        assert_eq!(smin, 13);
        assert_eq!(
            sim.substrate.entities.get(smin).unwrap().health.current,
            776
        );
        for discarded_id in 14..=18 {
            assert!(sim.substrate.entities.get(discarded_id).is_none());
        }
        assert!(sim.production.slave_bindings.get(&yarefn).is_none());
        assert_eq!(sim.production.slave_bindings.get(&smin), Some(&slave_ids));
        assert_eq!(
            sim.substrate
                .entities
                .get(smin)
                .unwrap()
                .attached_trigger_tag,
            Some(tag)
        );
        assert_eq!(
            sim.substrate
                .entities
                .get(yarefn)
                .expect("deferred YAREFN source")
                .attached_trigger_tag,
            None
        );
        for slave_id in slave_ids {
            let slave = sim
                .substrate
                .entities
                .get(slave_id)
                .expect("slave remains live");
            assert_eq!(slave.slave_harvester.as_ref().unwrap().master_id, smin);
        }
    }

    #[test]
    fn deploy_transfers_attached_tag_to_new_yarefn_and_clears_source() {
        let rules = make_test_rules();
        let mut sim = Simulation::with_seed(0x51A7_E002);
        let smin = sim
            .spawn_object_at_height("SMIN", "YuriCountry", 10, 10, 0, 0, &rules)
            .expect("spawn tagged SMIN");
        let tag = sim.interner.intern("TAG_SMIN_DEPLOY");
        sim.substrate
            .entities
            .get_mut(smin)
            .expect("live SMIN")
            .attached_trigger_tag = Some(tag);
        sim.substrate
            .entities
            .get_mut(smin)
            .expect("live SMIN")
            .health
            .current = 777;

        let yarefn = deploy_slave_miner(&mut sim, smin, &rules).expect("deploy tagged SMIN");

        let target = sim.substrate.entities.get(yarefn).expect("new YAREFN");
        assert_eq!(target.health.current, 776);
        assert!(target.building_damage_state_active);
        assert_eq!(target.building_anim_reset_revision, 0);
        assert_eq!(target.attached_trigger_tag, Some(tag));
        assert_eq!(target.mission.current(), MissionId::NONE);
        assert_eq!(
            target.mission.queued(),
            MissionId::from_known(MissionType::Construction)
        );
        assert_eq!(
            target.mission.effective(),
            MissionId::from_known(MissionType::Construction)
        );
        assert_eq!(
            target
                .mission_leaf
                .as_building()
                .expect("YAREFN Building leaf")
                .ready_latch(),
            0
        );
        assert!(target.building_up.is_some());
        assert_eq!(
            sim.substrate
                .entities
                .get(smin)
                .expect("deferred-deletion SMIN remains resolvable")
                .attached_trigger_tag,
            None,
            "successful transfer clears the dying source reference"
        );
        assert_eq!(sim.interner.resolve(tag), "TAG_SMIN_DEPLOY");
    }

    #[test]
    fn misfaced_stock_smin_turns_then_ai_retry_transplants_manager_once() {
        let rules = make_test_rules();
        let seed = 0x51A7_E006;
        let mut sim = Simulation::with_seed(seed);
        let yuri = sim.interner.intern("YuriCountry");
        sim.houses
            .insert(yuri, HouseState::new(yuri, 2, Some(yuri), true, 0, 10));
        sim.session.house_order.push(yuri);
        let height_map = BTreeMap::new();
        let smin = sim
            .spawn_object_at_height("SMIN", "YuriCountry", 10, 10, 0x80, 0, &rules)
            .expect("spawn misfaced SMIN");
        let original_slave_ids = sim
            .production
            .slave_bindings
            .get(&smin)
            .cloned()
            .expect("SMIN constructor manager");
        assert_eq!(original_slave_ids.len(), 5);
        let idle_hold_frame = sim.session.binary_frame;
        for slave_id in &original_slave_ids {
            sim.substrate
                .entities
                .get_mut(*slave_id)
                .and_then(|entity| entity.infantry.as_mut())
                .expect("live slave infantry")
                .idle_action_timer
                .defer(idle_hold_frame, 100);
        }
        // This fixture proves the forward-deploy constructor's six draws. Keep
        // the five live SLAV idle-action feeders outside its 40-tick window so
        // their unrelated first-frame wait/action/facing draws do not pollute
        // that exact stream assertion.
        let tag = sim.interner.intern("TAG_SMIN_FACING_RETRY");
        sim.substrate
            .entities
            .get_mut(smin)
            .expect("live SMIN")
            .attached_trigger_tag = Some(tag);
        let rng_before_command = sim.scenario_rng.clone();

        assert!(sim.apply_command(
            "YuriCountry",
            &Command::DeployMcv { entity_id: smin },
            Some(&rules),
            None,
            &height_map,
        ));
        let source = sim.substrate.entities.get(smin).expect("turning SMIN");
        assert_eq!(source.facing, 0x80);
        assert_eq!(source.facing_target, Some(0));
        assert!(source.forward_deploy_retry);
        assert_eq!(source.attached_trigger_tag, Some(tag));
        assert_eq!(
            sim.production.slave_bindings.get(&smin),
            Some(&original_slave_ids)
        );
        assert_eq!(
            sim.scenario_rng.logical_state(),
            rng_before_command.logical_state(),
            "facing retry must not construct a target or draw RNG"
        );
        assert!(
            !sim.substrate
                .entities
                .values()
                .any(|entity| sim.interner.resolve(entity.type_ref) == "YAREFN")
        );

        let mut committed = None;
        for _ in 0..40 {
            let tick = sim.advance_tick(&[], Some(&rules), &height_map, None, None, 16);
            let yarefn = sim
                .substrate
                .entities
                .values()
                .find(|entity| sim.interner.resolve(entity.type_ref) == "YAREFN")
                .map(|entity| entity.stable_id);
            if let Some(yarefn) = yarefn {
                committed = Some((yarefn, tick));
                break;
            }
        }
        let (yarefn, commit_tick) = committed.expect("matching Unit AI retry commits YAREFN");
        assert!(commit_tick.spawned_entities);
        assert!(sim.substrate.entities.get(smin).is_none());
        assert_eq!(
            sim.substrate
                .entities
                .values()
                .filter(|entity| sim.interner.resolve(entity.type_ref) == "YAREFN")
                .count(),
            1
        );
        let target = sim
            .substrate
            .entities
            .get(yarefn)
            .expect("committed YAREFN");
        assert_eq!(target.attached_trigger_tag, Some(tag));
        assert!(target.building_up.is_some());
        assert_eq!(
            sim.production.slave_bindings.get(&yarefn),
            Some(&original_slave_ids),
            "fresh constructor pool must be discarded and source identities transplanted"
        );
        for slave_id in &original_slave_ids {
            assert_eq!(
                sim.substrate
                    .entities
                    .get(*slave_id)
                    .expect("transplanted slave remains live")
                    .slave_harvester
                    .as_ref()
                    .expect("slave manager link")
                    .master_id,
                yarefn
            );
        }
        let mut expected_rng = rng_before_command;
        for _ in 0..6 {
            let _ = expected_rng.next_u32();
        }
        assert_eq!(
            sim.scenario_rng.logical_state(),
            expected_rng.logical_state()
        );
    }

    #[test]
    fn forward_deploy_retry_near_attack_draws_before_constructor_and_never_fires() {
        let rules = make_test_rules();
        let seed = 0x51A7_E007;
        let mut sim = Simulation::with_seed(seed);
        let yuri = sim.interner.intern("YuriCountry");
        let americans = sim.interner.intern("Americans");
        sim.houses
            .insert(yuri, HouseState::new(yuri, 2, Some(yuri), true, 0, 10));
        sim.houses.insert(
            americans,
            HouseState::new(americans, 0, Some(americans), true, 0, 10),
        );
        sim.session.house_order.extend([yuri, americans]);
        let height_map = BTreeMap::new();
        let smin = sim
            .spawn_object_at_height("SMIN", "YuriCountry", 10, 10, 0x80, 0, &rules)
            .expect("spawn misfaced SMIN");
        let target = sim
            .spawn_object_at_height("TARGET", "Americans", 10, 14, 0, 0, &rules)
            .expect("spawn in-range target");
        let target_health = sim
            .substrate
            .entities
            .get(target)
            .expect("target")
            .health
            .current;
        assert!(sim.apply_command(
            "YuriCountry",
            &Command::DeployMcv { entity_id: smin },
            Some(&rules),
            None,
            &height_map,
        ));
        assert!(sim.apply_command(
            "YuriCountry",
            &Command::Attack {
                attacker_id: smin,
                target_id: target,
            },
            Some(&rules),
            None,
            &height_map,
        ));
        let staged = sim.substrate.entities.get(smin).expect("staged SMIN");
        assert!(staged.forward_deploy_retry);
        assert_eq!(staged.mission.queued().known(), Some(MissionType::Attack));

        // Isolate the source-local ordering from unrelated full-tick RNG
        // consumers. Ready sees the completed facing, promotes Attack, runs its
        // approach producer, then Mission_Attack draws RandomRanged(0,2).
        // PerCell(0) performs the deploy transaction immediately afterward.
        for other_id in sim
            .live_object_order_snapshot()
            .into_iter()
            .filter(|&id| id != smin)
            .collect::<Vec<_>>()
        {
            assert!(sim.unregister_live_object(other_id));
        }
        {
            let source = sim.substrate.entities.get_mut(smin).expect("staged SMIN");
            source.facing = 0;
            source.facing_target = None;
            source.body_facing = None;
        }
        sim.clear_lifecycle_test_events_for_test();
        let rng_before_attack_dispatch = sim.scenario_rng.clone();
        let commit_tick = sim.advance_tick(&[], Some(&rules), &height_map, None, None, 16);
        assert!(commit_tick.spawned_entities);
        assert!(
            sim.substrate.entities.get(smin).is_none(),
            "the full production tick drains the consumed SMIN after PerCell deploy"
        );

        let yarefn = sim
            .substrate
            .entities
            .values()
            .find(|entity| sim.interner.resolve(entity.type_ref) == "YAREFN")
            .map(|entity| entity.stable_id)
            .expect("near Attack production tick must deploy the source");
        let mut expected_after_constructor = rng_before_attack_dispatch;
        let _ = expected_after_constructor.next_range_u32_inclusive(0, 2);
        let expected_after_attack = expected_after_constructor.state();
        for _ in 0..6 {
            let _ = expected_after_constructor.next_u32();
        }
        let events = sim.lifecycle_test_events_for_test();
        let attack_boundary = events
            .iter()
            .enumerate()
            .find_map(|(index, event)| match event {
                LifecycleTestEvent::ForwardDeployAttackCadenceDrawn {
                    stable_id,
                    scenario_rng_state,
                } if *stable_id == smin => Some((index, *scenario_rng_state)),
                _ => None,
            })
            .expect("full tick must dispatch current Attack before PerCell");
        let constructor_boundary = events
            .iter()
            .enumerate()
            .find_map(|(index, event)| match event {
                LifecycleTestEvent::ForwardDeployTargetConstructed {
                    stable_id,
                    scenario_rng_state,
                } if *stable_id == yarefn => Some((index, *scenario_rng_state)),
                _ => None,
            })
            .expect("full tick must construct YAREFN after the Attack dispatch");
        assert!(attack_boundary.0 < constructor_boundary.0);
        assert_eq!(attack_boundary.1, expected_after_attack);
        assert_eq!(constructor_boundary.1, expected_after_constructor.state());
        assert!(sim.substrate.entities.get(yarefn).is_some());
        assert_eq!(
            sim.substrate
                .entities
                .get(target)
                .expect("target survives")
                .health
                .current,
            target_health,
            "PerCell deploy consumes SMIN before Unit fire"
        );
    }

    #[test]
    fn forward_deploy_retry_force_fire_variants_use_attack_owner_before_deploy() {
        let rules = make_test_rules();
        let mut sim = Simulation::with_seed(0x51A7_E011);
        let yuri = sim.interner.intern("YuriCountry");
        let americans = sim.interner.intern("Americans");
        sim.houses
            .insert(yuri, HouseState::new(yuri, 2, Some(yuri), true, 0, 10));
        sim.houses.insert(
            americans,
            HouseState::new(americans, 0, Some(americans), true, 0, 10),
        );
        sim.session.house_order.extend([yuri, americans]);
        let height_map = BTreeMap::new();
        let object_attacker = sim
            .spawn_object_at_height("SMIN", "YuriCountry", 10, 10, 0x80, 0, &rules)
            .expect("spawn force-object SMIN");
        let cell_attacker = sim
            .spawn_object_at_height("SMIN", "YuriCountry", 20, 20, 0x80, 0, &rules)
            .expect("spawn force-cell SMIN");
        let target = sim
            .spawn_object_at_height("TARGET", "Americans", 10, 14, 0, 0, &rules)
            .expect("spawn force-object target");

        for attacker in [object_attacker, cell_attacker] {
            assert!(sim.apply_command(
                "YuriCountry",
                &Command::DeployMcv {
                    entity_id: attacker,
                },
                Some(&rules),
                None,
                &height_map,
            ));
        }
        assert!(sim.apply_command(
            "YuriCountry",
            &Command::ForceAttack {
                attacker_id: object_attacker,
                target_id: target,
            },
            Some(&rules),
            None,
            &height_map,
        ));
        assert!(sim.apply_command(
            "YuriCountry",
            &Command::ForceAttackCell {
                attacker_id: cell_attacker,
                target_rx: 20,
                target_ry: 16,
            },
            Some(&rules),
            None,
            &height_map,
        ));

        let object_source = sim
            .substrate
            .entities
            .get(object_attacker)
            .expect("object force-fire source remains live");
        assert!(object_source.forward_deploy_retry);
        assert!(object_source.owns_forward_deploy_attack_retry());
        assert_eq!(
            object_source
                .attack_target
                .as_ref()
                .map(|attack| attack.target),
            Some(crate::sim::combat::TargetKind::Entity(target))
        );
        assert!(object_source.navigation.nav_com.is_none());
        assert!(object_source.movement_target.is_none());

        let cell_source = sim
            .substrate
            .entities
            .get(cell_attacker)
            .expect("cell force-fire source remains live");
        assert!(cell_source.forward_deploy_retry);
        assert!(cell_source.owns_forward_deploy_attack_retry());
        assert_eq!(
            cell_source
                .attack_target
                .as_ref()
                .map(|attack| attack.target),
            Some(crate::sim::combat::TargetKind::Cell(20, 16))
        );
        assert!(cell_source.navigation.nav_com.is_none());
        assert!(cell_source.movement_target.is_none());

        // Exercise the actual object scheduler, not only command-state bits.
        // Keep just the two source Units in LogicVector order. Both targets are
        // comfortably in range. Each mission-1 variant must dispatch cadence
        // before its owner-local PerCell retry consumes the source.
        for other_id in sim
            .live_object_order_snapshot()
            .into_iter()
            .filter(|id| ![object_attacker, cell_attacker].contains(id))
            .collect::<Vec<_>>()
        {
            assert!(sim.unregister_live_object(other_id));
        }
        sim.clear_lifecycle_test_events_for_test();
        let target_health = sim
            .substrate
            .entities
            .get(target)
            .expect("force-fire target")
            .health
            .current;
        let mut conversion_tick = None;
        for step in 1..=64 {
            let tick = sim.advance_tick(&[], Some(&rules), &height_map, None, None, 16);
            let yarefn_count = sim
                .substrate
                .entities
                .values()
                .filter(|entity| sim.interner.resolve(entity.type_ref) == "YAREFN")
                .count();
            if yarefn_count == 2 {
                assert!(tick.spawned_entities);
                conversion_tick = Some(step);
                break;
            }
        }
        assert!(
            conversion_tick.is_some(),
            "both force-fire sources deploy normally"
        );
        assert!(sim.substrate.entities.get(object_attacker).is_none());
        assert!(sim.substrate.entities.get(cell_attacker).is_none());
        assert_eq!(
            sim.substrate
                .entities
                .get(target)
                .expect("target survives both owner-local conversions")
                .health
                .current,
            target_health,
            "source liveness must suppress the object force-fire shot after conversion"
        );

        let events = sim.lifecycle_test_events_for_test();
        for attacker in [object_attacker, cell_attacker] {
            let cadence_index = events
                .iter()
                .position(|event| {
                    matches!(
                        event,
                        LifecycleTestEvent::ForwardDeployAttackCadenceDrawn { stable_id, .. }
                            if *stable_id == attacker
                    )
                })
                .expect("each force-fire variant must dispatch Mission_Attack cadence");
            let constructor_index = events
                .iter()
                .enumerate()
                .skip(cadence_index + 1)
                .find_map(|(index, event)| {
                    matches!(
                        event,
                        LifecycleTestEvent::ForwardDeployTargetConstructed { .. }
                    )
                    .then_some(index)
                })
                .expect("each force-fire cadence must precede a deploy constructor");
            assert!(cadence_index < constructor_index);
        }
        let deployed_cells = sim
            .substrate
            .entities
            .values()
            .filter(|entity| sim.interner.resolve(entity.type_ref) == "YAREFN")
            .map(|entity| (entity.position.rx, entity.position.ry))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(deployed_cells, [(10, 10), (20, 20)].into_iter().collect());
    }

    #[test]
    fn forward_deploy_retry_far_force_fire_variants_approach_before_deploy() {
        let rules = make_test_rules();
        for (cell_target, seed, case) in [
            (false, 0x51A7_E012, "object force-fire"),
            (true, 0x51A7_E013, "cell force-fire"),
        ] {
            let mut sim = Simulation::with_seed(seed);
            let yuri = sim.interner.intern("YuriCountry");
            let americans = sim.interner.intern("Americans");
            sim.houses
                .insert(yuri, HouseState::new(yuri, 2, Some(yuri), true, 0, 10));
            sim.houses.insert(
                americans,
                HouseState::new(americans, 0, Some(americans), true, 0, 10),
            );
            sim.session.house_order.extend([yuri, americans]);
            for house in sim.houses.values_mut() {
                house.multiplay_passive = true;
            }
            let heights = BTreeMap::new();
            let grid = PathGrid::new(40, 40);
            let source_cell = (20, 20);
            let attacker = sim
                .spawn_object_at_height(
                    "SMIN",
                    "YuriCountry",
                    source_cell.0,
                    source_cell.1,
                    0x80,
                    0,
                    &rules,
                )
                .expect("spawn far force-fire SMIN");
            assert!(sim.apply_command(
                "YuriCountry",
                &Command::DeployMcv {
                    entity_id: attacker
                },
                Some(&rules),
                Some(&grid),
                &heights,
            ));
            let (command, expected_target) = if cell_target {
                (
                    Command::ForceAttackCell {
                        attacker_id: attacker,
                        target_rx: 20,
                        target_ry: 12,
                    },
                    crate::sim::combat::TargetKind::Cell(20, 12),
                )
            } else {
                let target = sim
                    .spawn_object_at_height("TARGET", "Americans", 20, 12, 0, 0, &rules)
                    .expect("spawn far object target");
                sim.spawn_object_at_height("SPOTTER", "YuriCountry", 25, 12, 0, 0, &rules)
                    .expect("spawn allied sight provider");
                if let Some(target_entity) = sim.substrate.entities.get_mut(target) {
                    target_entity.health.current = u16::MAX;
                    target_entity.health.max = u16::MAX;
                }
                (
                    Command::ForceAttack {
                        attacker_id: attacker,
                        target_id: target,
                    },
                    crate::sim::combat::TargetKind::Entity(target),
                )
            };
            assert!(
                sim.apply_command("YuriCountry", &command, Some(&rules), Some(&grid), &heights,),
                "{case}"
            );
            let source = sim.substrate.entities.get(attacker).expect("staged source");
            assert!(source.owns_forward_deploy_attack_retry());
            assert_eq!(
                source.attack_target.as_ref().map(|attack| attack.target),
                Some(expected_target)
            );
            assert!(source.navigation.nav_com.is_none());
            assert!(source.movement_target.is_none());
            sim.clear_lifecycle_test_events_for_test();
            let mut approach_cell = None;
            let mut residual = None;
            for _ in 0..256 {
                let tick = sim.advance_tick(&[], Some(&rules), &heights, Some(&grid), None, 16);
                assert!(tick.frame_committed, "{case}");
                residual = sim.substrate.entities.get(attacker).map(|source| {
                    (
                        source.forward_deploy_retry,
                        source.mission.current().known(),
                        source.mission.queued().known(),
                        source.attack_target.as_ref().map(|attack| attack.target),
                        source.navigation.nav_com,
                        source
                            .movement_target
                            .as_ref()
                            .map(|target| target.final_goal),
                        (source.position.rx, source.position.ry),
                    )
                });
                if let Some(source) = sim.substrate.entities.get(attacker)
                    && source.owns_forward_deploy_attack_retry()
                    && source.mission.current().known() == Some(MissionType::Attack)
                    && source.navigation.nav_com.is_some()
                    && source.movement_target.is_some()
                {
                    assert_eq!(
                        source.attack_target.as_ref().map(|attack| attack.target),
                        Some(expected_target),
                        "{case}"
                    );
                    approach_cell = Some((source.position.rx, source.position.ry));
                    break;
                }
            }
            assert!(
                approach_cell.is_some(),
                "{case} must install a live far approach before deploy; residual={residual:?}"
            );
            assert!(sim.interner.get("YAREFN").is_none_or(|yarefn| {
                sim.substrate
                    .entities
                    .values()
                    .all(|entity| entity.type_ref != yarefn)
            }));

            for _ in 0..2048 {
                sim.advance_tick(&[], Some(&rules), &heights, Some(&grid), None, 16);
                if sim.substrate.entities.get(attacker).is_none() {
                    break;
                }
            }
            assert!(sim.substrate.entities.get(attacker).is_none(), "{case}");
            let deployed = sim
                .substrate
                .entities
                .values()
                .find(|entity| sim.interner.resolve(entity.type_ref) == "YAREFN")
                .expect("far force-fire must deploy YAREFN");
            assert!(
                deployed.position.ry < source_cell.1,
                "{case} must deploy from an approached cell, not the original cell: {:?}",
                (deployed.position.rx, deployed.position.ry)
            );
            assert!(
                sim.lifecycle_test_events_for_test().iter().any(|event| {
                    matches!(
                        event,
                        LifecycleTestEvent::ForwardDeployAttackCadenceDrawn { stable_id, .. }
                            if *stable_id == attacker
                    )
                }),
                "{case}"
            );
        }
    }

    #[test]
    fn forward_deploy_retry_attack_target_loss_draws_jitter_and_keeps_attack_until_deploy() {
        let rules = make_test_rules();
        let seed = 0x51A7_E008;
        let mut sim = Simulation::with_seed(seed);
        let yuri = sim.interner.intern("YuriCountry");
        let americans = sim.interner.intern("Americans");
        sim.houses
            .insert(yuri, HouseState::new(yuri, 2, Some(yuri), true, 0, 10));
        sim.houses.insert(
            americans,
            HouseState::new(americans, 0, Some(americans), true, 0, 10),
        );
        sim.session.house_order.extend([yuri, americans]);
        let height_map = BTreeMap::new();
        let smin = sim
            .spawn_object_at_height("SMIN", "YuriCountry", 10, 10, 0x80, 0, &rules)
            .expect("spawn misfaced SMIN");
        let target = sim
            .spawn_object_at_height("TARGET", "Americans", 10, 14, 0, 0, &rules)
            .expect("spawn target");

        assert!(sim.apply_command(
            "YuriCountry",
            &Command::DeployMcv { entity_id: smin },
            Some(&rules),
            None,
            &height_map,
        ));
        assert!(sim.apply_command(
            "YuriCountry",
            &Command::Attack {
                attacker_id: smin,
                target_id: target,
            },
            Some(&rules),
            None,
            &height_map,
        ));
        assert_eq!(
            sim.substrate
                .entities
                .get(smin)
                .expect("staged SMIN")
                .mission
                .queued()
                .known(),
            Some(MissionType::Attack)
        );

        sim.stop_all_targeting_on_detach(target);
        let detached = sim.substrate.entities.get(smin).expect("detached attacker");
        assert!(detached.attack_target.is_none());
        assert!(detached.forward_deploy_retry);
        assert_eq!(detached.mission.queued().known(), Some(MissionType::Attack));

        for other_id in sim
            .live_object_order_snapshot()
            .into_iter()
            .filter(|&id| id != smin)
            .collect::<Vec<_>>()
        {
            assert!(sim.unregister_live_object(other_id));
        }
        {
            let source = sim.substrate.entities.get_mut(smin).expect("staged SMIN");
            source.facing = 0;
            source.facing_target = None;
            source.body_facing = None;
        }

        sim.clear_lifecycle_test_events_for_test();
        let rng_before_dispatch = sim.scenario_rng.clone();
        let commit_tick = sim.advance_tick(&[], Some(&rules), &height_map, None, None, 16);
        assert!(commit_tick.spawned_entities);
        assert!(
            sim.substrate.entities.get(smin).is_none(),
            "the full production tick drains the targetless attacker after deploy"
        );

        let yarefn = sim
            .substrate
            .entities
            .values()
            .find(|entity| sim.interner.resolve(entity.type_ref) == "YAREFN")
            .map(|entity| entity.stable_id)
            .expect("targetless Attack production tick must deploy the source");
        let mut expected_after_constructor = rng_before_dispatch;
        let _ = expected_after_constructor.next_range_u32_inclusive(0, 2);
        let expected_after_attack = expected_after_constructor.state();
        for _ in 0..6 {
            let _ = expected_after_constructor.next_u32();
        }
        let events = sim.lifecycle_test_events_for_test();
        let attack_boundary = events
            .iter()
            .enumerate()
            .find_map(|(index, event)| match event {
                LifecycleTestEvent::ForwardDeployAttackCadenceDrawn {
                    stable_id,
                    scenario_rng_state,
                } if *stable_id == smin => Some((index, *scenario_rng_state)),
                _ => None,
            })
            .expect("targetless current Attack must still draw its cadence jitter");
        let constructor_boundary = events
            .iter()
            .enumerate()
            .find_map(|(index, event)| match event {
                LifecycleTestEvent::ForwardDeployTargetConstructed {
                    stable_id,
                    scenario_rng_state,
                } if *stable_id == yarefn => Some((index, *scenario_rng_state)),
                _ => None,
            })
            .expect("targetless Attack must reach the later deploy constructor");
        assert!(attack_boundary.0 < constructor_boundary.0);
        assert_eq!(attack_boundary.1, expected_after_attack);
        assert_eq!(constructor_boundary.1, expected_after_constructor.state());
    }

    #[test]
    fn forward_deploy_retry_same_frame_stop_attack_uses_immediate_then_staged_order() {
        let rules = make_test_rules();
        let mut sim = Simulation::with_seed(0x51A7_E009);
        let yuri = sim.interner.intern("YuriCountry");
        let americans = sim.interner.intern("Americans");
        sim.houses
            .insert(yuri, HouseState::new(yuri, 2, Some(yuri), true, 0, 10));
        sim.houses.insert(
            americans,
            HouseState::new(americans, 0, Some(americans), true, 0, 10),
        );
        sim.session.house_order.extend([yuri, americans]);
        for house in sim.houses.values_mut() {
            house.multiplay_passive = true;
        }
        let height_map = BTreeMap::new();
        let grid = PathGrid::new(40, 40);
        let smin = sim
            .spawn_object_at_height("SMIN", "YuriCountry", 10, 20, 0x80, 0, &rules)
            .expect("spawn misfaced SMIN");
        let target = sim
            .spawn_object_at_height("TARGET", "Americans", 10, 8, 0, 0, &rules)
            .expect("spawn far target");
        let _spotter = sim
            .spawn_object_at_height("SPOTTER", "YuriCountry", 15, 8, 0, 0, &rules)
            .expect("spawn allied vision provider");

        assert!(sim.apply_command(
            "YuriCountry",
            &Command::DeployMcv { entity_id: smin },
            Some(&rules),
            Some(&grid),
            &height_map,
        ));
        sim.substrate
            .entities
            .get_mut(smin)
            .expect("turning SMIN")
            .radio_contacts
            .insert(target);
        sim.substrate
            .entities
            .get_mut(target)
            .expect("radio peer")
            .radio_contacts
            .insert(smin);

        let execute_tick = sim.session.tick + 1;
        // Deliberately submit Attack first. Native drains opcode-6 Stop from
        // the primary ring before the staged opcode-4 MegaMission FIFO.
        let tick = sim.advance_tick(
            &[
                CommandEnvelope::new(
                    yuri,
                    execute_tick,
                    Command::Attack {
                        attacker_id: smin,
                        target_id: target,
                    },
                ),
                CommandEnvelope::new(yuri, execute_tick, Command::Stop { entity_id: smin }),
            ],
            Some(&rules),
            &height_map,
            Some(&grid),
            None,
            16,
        );
        assert!(tick.frame_committed);
        let staged = sim.substrate.entities.get(smin).expect("staged attacker");
        assert!(staged.forward_deploy_retry);
        assert_eq!(staged.mission.current().known(), Some(MissionType::Unload));
        assert_eq!(staged.mission.queued().known(), Some(MissionType::Attack));
        assert_eq!(
            staged.attack_target.as_ref().map(|attack| attack.target),
            Some(crate::sim::combat::TargetKind::Entity(target))
        );
        assert!(staged.navigation.nav_com.is_none());
        assert!(staged.movement_target.is_none());
        assert!(staged.radio_contacts.is_empty());
        assert!(
            sim.substrate
                .entities
                .get(target)
                .expect("radio peer survives")
                .radio_contacts
                .is_empty()
        );

        sim.clear_lifecycle_test_events_for_test();
        let mut approach_started = false;
        for _ in 0..80 {
            let tick = sim.advance_tick(&[], Some(&rules), &height_map, Some(&grid), None, 16);
            assert!(tick.frame_committed);
            approach_started = sim.substrate.entities.get(smin).is_some_and(|entity| {
                entity.forward_deploy_retry
                    && entity.mission.current().known() == Some(MissionType::Attack)
                    && entity.mission.queued() == MissionId::NONE
                    && entity.attack_target.as_ref().map(|attack| attack.target)
                        == Some(crate::sim::combat::TargetKind::Entity(target))
                    && entity.navigation.nav_com.is_some()
                    && entity.movement_target.is_some()
            });
            if approach_started {
                break;
            }
        }
        assert!(
            approach_started,
            "the staged Attack must survive Stop and select the far approach"
        );
        assert!(sim.lifecycle_test_events_for_test().iter().any(|event| {
            matches!(
                event,
                LifecycleTestEvent::ForwardDeployAttackCadenceDrawn { stable_id, .. }
                    if *stable_id == smin
            )
        }));
        assert!(sim.interner.get("YAREFN").is_none_or(|yarefn| {
            sim.substrate
                .entities
                .values()
                .all(|entity| entity.type_ref != yarefn)
        }));
    }

    #[test]
    fn forward_deploy_retry_far_attack_revalidates_at_promotion_before_first_contact() {
        let rules = make_test_rules();
        let mut sim = Simulation::with_seed(0x51A7_E00A);
        let yuri = sim.interner.intern("YuriCountry");
        let americans = sim.interner.intern("Americans");
        sim.houses
            .insert(yuri, HouseState::new(yuri, 2, Some(yuri), true, 0, 10));
        sim.houses.insert(
            americans,
            HouseState::new(americans, 0, Some(americans), true, 0, 10),
        );
        sim.session.house_order.extend([yuri, americans]);
        let heights = BTreeMap::new();
        let grid = PathGrid::new(40, 40);
        let smin = sim
            .spawn_object_at_height("SMIN", "YuriCountry", 10, 20, 0x80, 0, &rules)
            .expect("spawn misfaced SMIN");
        let target = sim
            .spawn_object_at_height("TARGET", "Americans", 10, 8, 0, 0, &rules)
            .expect("spawn far target");
        sim.spawn_object_at_height("SPOTTER", "YuriCountry", 15, 8, 0, 0, &rules)
            .expect("spawn allied vision provider");
        assert!(sim.apply_command(
            "YuriCountry",
            &Command::DeployMcv { entity_id: smin },
            Some(&rules),
            Some(&grid),
            &heights,
        ));
        assert!(sim.apply_command(
            "YuriCountry",
            &Command::Attack {
                attacker_id: smin,
                target_id: target,
            },
            Some(&rules),
            Some(&grid),
            &heights,
        ));

        // Isolate the scheduler seam exactly as the near-Attack ordering
        // acceptance does: complete FacingClass between actor visits while
        // leaving the real queued Attack and retry ownership untouched. The
        // next production visit must promote Attack, choose the far approach,
        // draw cadence, then run rotation-finished PerCell(0).
        {
            let source = sim.substrate.entities.get_mut(smin).expect("staged SMIN");
            source.facing = 0;
            source.facing_target = None;
            source.body_facing = None;
        }
        let staged = sim
            .substrate
            .entities
            .get(smin)
            .expect("promotion boundary");
        assert!(staged.forward_deploy_retry);
        assert_eq!(staged.mission.queued().known(), Some(MissionType::Attack));

        // YAREFN is 2x2 at the SMIN cell. Rooting another YAREFN one cell east
        // leaves the northbound approach lane open but overlaps the candidate
        // deploy footprint's eastern column. It appears after the final facing
        // visit and therefore must be caught by the first Attack-owned
        // PerCell(0), before any committed cell contact.
        let source_cell = sim
            .substrate
            .entities
            .get(smin)
            .map(|entity| (entity.position.rx, entity.position.ry))
            .expect("source at promotion boundary");
        let blocker = sim
            .spawn_object_at_height(
                "YAREFN",
                "Americans",
                source_cell.0 + 1,
                source_cell.1,
                0,
                0,
                &rules,
            )
            .expect("spawn promotion-window footprint blocker");
        let yarefn_type = sim.interner.get("YAREFN").expect("YAREFN interned");
        let sound_count = sim.sound_events.len();
        sim.clear_lifecycle_test_events_for_test();

        let tick = sim.advance_tick(&[], Some(&rules), &heights, Some(&grid), None, 16);
        assert!(
            !tick.spawned_entities,
            "blocked PerCell(0) cannot construct"
        );
        let source = sim
            .substrate
            .entities
            .get(smin)
            .expect("placement rejection retains SMIN source");
        assert_eq!(
            (source.position.rx, source.position.ry),
            source_cell,
            "promotion-boundary rejection must precede the first committed cell contact"
        );
        assert_eq!(source.mission.current().known(), Some(MissionType::Guard));
        assert_eq!(source.mission.queued(), MissionId::NONE);
        assert!(!source.forward_deploy_retry);
        assert!(source.navigation.nav_com.is_some());
        assert!(source.movement_target.is_some());
        assert!(source.drive_track.is_some());
        assert!(sim.lifecycle_test_events_for_test().iter().any(|event| {
            matches!(
                event,
                LifecycleTestEvent::ForwardDeployAttackCadenceDrawn { stable_id, .. }
                    if *stable_id == smin
            )
        }));
        assert!(sim.substrate.entities.get(blocker).is_some());
        assert_eq!(
            sim.substrate
                .entities
                .values()
                .filter(|entity| entity.type_ref == yarefn_type)
                .count(),
            1,
            "only the pre-existing blocker YAREFN may remain"
        );
        assert_eq!(sim.sound_events.len(), sound_count + 1);
        assert!(sim.sound_events.iter().any(
            |event| matches!(event, crate::sim::world::SimSoundEvent::CannotDeployHere { owner } if *owner == yuri)
        ));
    }

    #[test]
    fn forward_deploy_retry_far_attack_rejects_at_intermediate_per_cell_contact() {
        let rules = make_test_rules();
        let mut sim = Simulation::with_seed(0x51A7_E009);
        let yuri = sim.interner.intern("YuriCountry");
        let americans = sim.interner.intern("Americans");
        sim.houses
            .insert(yuri, HouseState::new(yuri, 2, Some(yuri), true, 0, 10));
        sim.houses.insert(
            americans,
            HouseState::new(americans, 0, Some(americans), true, 0, 10),
        );
        sim.session.house_order.extend([yuri, americans]);
        let heights = BTreeMap::new();
        let grid = PathGrid::new(40, 40);
        let smin = sim
            .spawn_object_at_height("SMIN", "YuriCountry", 10, 20, 0x80, 0, &rules)
            .expect("spawn misfaced SMIN");
        let target = sim
            .spawn_object_at_height("TARGET", "Americans", 10, 8, 0, 0, &rules)
            .expect("spawn far target");
        sim.spawn_object_at_height("SPOTTER", "YuriCountry", 15, 8, 0, 0, &rules)
            .expect("spawn allied vision provider");
        assert!(sim.apply_command(
            "YuriCountry",
            &Command::DeployMcv { entity_id: smin },
            Some(&rules),
            Some(&grid),
            &heights,
        ));
        assert!(sim.apply_command(
            "YuriCountry",
            &Command::Attack {
                attacker_id: smin,
                target_id: target,
            },
            Some(&rules),
            Some(&grid),
            &heights,
        ));

        let mut approach_cell = None;
        for _ in 0..80 {
            sim.advance_tick(&[], Some(&rules), &heights, Some(&grid), None, 16);
            approach_cell = sim.substrate.entities.get(smin).and_then(|entity| {
                (entity.forward_deploy_retry
                    && entity.mission.current().known() == Some(MissionType::Attack)
                    && entity.movement_target.is_some()
                    && entity.drive_track.is_some())
                .then_some((entity.position.rx, entity.position.ry))
            });
            if approach_cell.is_some() {
                break;
            }
        }
        let (start_rx, start_ry) = approach_cell.expect("far Attack starts a live Drive track");
        assert!(
            start_ry >= 2,
            "fixture needs a northbound adjacent blocker cell"
        );

        // YAREFN is 2x2. A blocker rooted one cell east and two north does not
        // overlap the source's current footprint, but it overlaps the footprint
        // rooted at the first northbound committed cell at exactly one cell.
        let blocker = sim
            .spawn_object_at_height(
                "YAREFN",
                "Americans",
                start_rx + 1,
                start_ry - 2,
                0,
                0,
                &rules,
            )
            .expect("spawn adjacent PerCell footprint blocker");
        let yarefn_type = sim.interner.get("YAREFN").expect("YAREFN interned");
        let sound_count = sim.sound_events.len();
        let mut rejected_contact = None;
        for step in 1..=512 {
            let before = sim
                .substrate
                .entities
                .get(smin)
                .map(|entity| {
                    (
                        (entity.position.rx, entity.position.ry),
                        entity.movement_target.is_some()
                            || crate::sim::movement::drive_locomotor_is_moving(entity),
                    )
                })
                .expect("source remains before the blocking contact");
            let tick = sim.advance_tick(&[], Some(&rules), &heights, Some(&grid), None, 16);
            assert!(
                !tick.spawned_entities,
                "blocked PerCell retry cannot construct"
            );
            let source = sim
                .substrate
                .entities
                .get(smin)
                .expect("placement rejection retains SMIN source");
            if !source.forward_deploy_retry {
                rejected_contact = Some((step, before, (source.position.rx, source.position.ry)));
                assert!(
                    before.1,
                    "the rejecting callback must begin with a live approach"
                );
                assert_ne!(
                    before.0,
                    (source.position.rx, source.position.ry),
                    "rejection must occur on the committed cell-contact tick"
                );
                assert_eq!(source.mission.effective().known(), Some(MissionType::Guard));
                assert!(source.movement_target.is_some());
                break;
            }
        }
        assert!(
            rejected_contact.is_some(),
            "the first blocked intermediate PerCell contact must terminate the retry"
        );
        assert!(sim.substrate.entities.get(blocker).is_some());
        assert_eq!(
            sim.substrate
                .entities
                .values()
                .filter(|entity| entity.type_ref == yarefn_type)
                .count(),
            1,
            "only the pre-existing blocker YAREFN may remain"
        );
        assert_eq!(sim.sound_events.len(), sound_count + 1);
        assert!(sim.sound_events.iter().any(
            |event| matches!(event, crate::sim::world::SimSoundEvent::CannotDeployHere { owner } if *owner == yuri)
        ));
    }

    #[test]
    fn forward_deploy_retry_stop_during_far_attack_finishes_head_then_deploys() {
        let rules = make_test_rules();
        let mut sim = Simulation::with_seed(0x51A7_E008);
        let yuri = sim.interner.intern("YuriCountry");
        let americans = sim.interner.intern("Americans");
        sim.houses
            .insert(yuri, HouseState::new(yuri, 2, Some(yuri), true, 0, 10));
        sim.houses.insert(
            americans,
            HouseState::new(americans, 0, Some(americans), true, 0, 10),
        );
        sim.session.house_order.extend([yuri, americans]);
        let height_map = BTreeMap::new();
        let grid = PathGrid::new(40, 40);
        let smin = sim
            .spawn_object_at_height("SMIN", "YuriCountry", 10, 20, 0x80, 0, &rules)
            .expect("spawn misfaced SMIN");
        let target = sim
            .spawn_object_at_height("TARGET", "Americans", 10, 8, 0, 0, &rules)
            .expect("spawn far target");
        let _spotter = sim
            .spawn_object_at_height("SPOTTER", "YuriCountry", 15, 8, 0, 0, &rules)
            .expect("spawn allied vision provider");

        assert!(sim.apply_command(
            "YuriCountry",
            &Command::DeployMcv { entity_id: smin },
            Some(&rules),
            Some(&grid),
            &height_map,
        ));
        assert!(sim.apply_command(
            "YuriCountry",
            &Command::Attack {
                attacker_id: smin,
                target_id: target,
            },
            Some(&rules),
            Some(&grid),
            &height_map,
        ));

        let mut approach_started = false;
        for _ in 0..80 {
            let _ = sim.advance_tick(&[], Some(&rules), &height_map, Some(&grid), None, 16);
            approach_started = sim.substrate.entities.get(smin).is_some_and(|entity| {
                entity.forward_deploy_retry
                    && entity.mission.current().known() == Some(MissionType::Attack)
                    && entity.attack_target.is_some()
                    && entity.navigation.nav_com.is_some()
                    && entity.movement_target.is_some()
                    && entity.drive_track.is_some()
            });
            if approach_started {
                break;
            }
        }
        assert!(
            approach_started,
            "far Attack must install its approach before RNG"
        );
        {
            let source = sim
                .substrate
                .entities
                .get_mut(smin)
                .expect("approaching SMIN");
            source.radio_contacts.insert(target);
            let drive = source.drive_locomotion.as_mut().expect("Drive runtime");
            drive.target_speed_fraction = crate::util::fixed_math::SIM_ONE;
            drive.current_speed_fraction = crate::util::fixed_math::SIM_HALF;
        }
        sim.substrate
            .entities
            .get_mut(target)
            .expect("radio receiver")
            .radio_contacts
            .insert(smin);
        let mission_before = sim.substrate.entities.get(smin).expect("source").mission;

        assert!(sim.apply_command(
            "YuriCountry",
            &Command::Stop { entity_id: smin },
            Some(&rules),
            Some(&grid),
            &height_map,
        ));
        let stopped = sim.substrate.entities.get(smin).expect("stopped SMIN");
        assert!(stopped.forward_deploy_retry);
        assert_eq!(stopped.mission.current(), mission_before.current());
        assert_eq!(stopped.mission.queued(), mission_before.queued());
        assert!(stopped.attack_target.is_none());
        assert!(stopped.navigation.nav_com.is_none());
        assert!(stopped.movement_target.is_some());
        assert!(stopped.radio_contacts.is_empty());
        assert!(
            sim.substrate
                .entities
                .get(target)
                .expect("radio receiver")
                .radio_contacts
                .is_empty()
        );
        assert_eq!(
            stopped
                .drive_locomotion
                .as_ref()
                .expect("Drive runtime")
                .target_speed_fraction,
            crate::util::fixed_math::SimFixed::lit("0.3")
        );

        // Stage the retained current Attack as due now. Stop preserves the
        // committed head and its Mission-dispatch state; making that state
        // explicitly due proves the next production visit still executes the
        // native Attack cadence before the locomotor retires the head.
        let now = sim.session.binary_frame;
        sim.substrate
            .entities
            .get_mut(smin)
            .expect("stopped SMIN")
            .mission
            .write_dispatch_epilogue(now as i32, 0);

        // Anything traced while establishing the far approach predates Stop
        // and cannot prove the retained committed head keeps current Attack's
        // native cadence. Observe only post-Stop production visits.
        sim.clear_lifecycle_test_events_for_test();
        let mut cadence_while_head_live = false;
        let mut committed = false;
        // Speed=3 is seven leptons per baseline tick: ten cells alone require
        // at least 366 ticks before acceleration and Drive-curve overhead.
        for _ in 0..768 {
            let translating_before = sim.substrate.entities.get(smin).is_some_and(|entity| {
                entity.movement_target.is_some()
                    || crate::sim::movement::drive_locomotor_is_moving(entity)
            });
            let event_count_before = sim.lifecycle_test_events_for_test().len();
            let mut expected_after_cadence = sim.scenario_rng.clone();
            let _ = expected_after_cadence.next_range_u32_inclusive(0, 2);
            let tick = sim.advance_tick(&[], Some(&rules), &height_map, Some(&grid), None, 16);
            if let Some(scenario_rng_state) = sim.lifecycle_test_events_for_test()
                [event_count_before..]
                .iter()
                .find_map(|event| match event {
                    LifecycleTestEvent::ForwardDeployAttackCadenceDrawn {
                        stable_id,
                        scenario_rng_state,
                    } if *stable_id == smin => Some(*scenario_rng_state),
                    _ => None,
                })
            {
                assert!(
                    translating_before,
                    "post-Stop Attack cadence must run before the retained head retires"
                );
                assert_eq!(
                    scenario_rng_state,
                    expected_after_cadence.state(),
                    "the source is first in LogicVector order, so its due cadence is exactly one RandomRanged(0,2) draw"
                );
                cadence_while_head_live = true;
            }
            let yarefn_count = sim
                .substrate
                .entities
                .values()
                .filter(|entity| sim.interner.resolve(entity.type_ref) == "YAREFN")
                .count();
            if yarefn_count == 1 {
                assert!(translating_before, "conversion belongs to head retirement");
                assert!(tick.spawned_entities);
                let source = sim
                    .substrate
                    .entities
                    .get(smin)
                    .expect("late-tick deploy awaits the next delete drain");
                assert!(!source.forward_deploy_retry);
                assert!(!source.lifecycle.object_alive);
                assert!(source.dying);
                assert!(source.lifecycle.in_limbo);
                assert!(sim.substrate.pending_delete.contains(&smin));
                committed = true;
                break;
            }
            assert!(
                sim.substrate
                    .entities
                    .get(smin)
                    .expect("source remains")
                    .forward_deploy_retry
            );
        }
        assert!(
            cadence_while_head_live,
            "a due current-Attack dispatch must consume cadence RNG while Stop's committed head remains live"
        );
        assert!(committed, "retained Stop head must end in same-tick deploy");
        sim.flush_pending_delete();
        assert!(sim.substrate.entities.get(smin).is_none());
    }

    #[test]
    fn forward_deploy_retry_move_then_attack_variants_clear_stale_move_navcom() {
        let rules = make_test_rules();
        // Cover both Rust eager shapes. Facing south makes the eastbound Move
        // wait in TurnFirst with no curve; facing east allocates a curve and
        // reserves its head before any locomotor process. Native has committed
        // neither shape when the following same-frame mission-1 event nulls
        // destination. Exercise all three accepted event producers because the
        // active binary converges ordinary object Attack, Ctrl-object Attack,
        // and Ctrl-cell Attack onto that same envelope.
        for (attack_variant, order_case) in [
            (0_u8, "object Attack"),
            (1_u8, "object ForceAttack"),
            (2_u8, "cell ForceAttack"),
        ] {
            for (initial_facing, expects_eager_curve, movement_case) in
                [(0x80, false, "TurnFirst"), (0x40, true, "eager curve")]
            {
                let case = format!("{order_case} / {movement_case}");
                let mut sim = Simulation::new();
                let yuri = sim.interner.intern("YuriCountry");
                let americans = sim.interner.intern("Americans");
                sim.houses
                    .insert(yuri, HouseState::new(yuri, 2, Some(yuri), true, 0, 10));
                sim.houses.insert(
                    americans,
                    HouseState::new(americans, 0, Some(americans), true, 0, 10),
                );
                sim.session.house_order.extend([yuri, americans]);
                let heights = BTreeMap::new();
                let grid = PathGrid::new(40, 40);
                let source_cell = (10, 20);
                let smin = sim
                    .spawn_object_at_height(
                        "SMIN",
                        "YuriCountry",
                        source_cell.0,
                        source_cell.1,
                        initial_facing,
                        0,
                        &rules,
                    )
                    .expect("spawn SMIN");
                let target = sim
                    .spawn_object_at_height("TARGET", "Americans", 10, 16, 0, 0, &rules)
                    .expect("spawn target");
                assert!(sim.apply_command(
                    "YuriCountry",
                    &Command::DeployMcv { entity_id: smin },
                    Some(&rules),
                    Some(&grid),
                    &heights,
                ));
                assert!(sim.apply_command(
                    "YuriCountry",
                    &Command::Move {
                        entity_id: smin,
                        target_rx: 20,
                        target_ry: 20,
                        queue: false,
                        group_id: None,
                    },
                    Some(&rules),
                    Some(&grid),
                    &heights,
                ));
                let eager_head = {
                    let moving = sim.substrate.entities.get(smin).expect("moving SMIN");
                    assert!(moving.navigation.nav_com.is_some(), "{case}");
                    let drive = moving.drive_locomotion.as_ref().expect("Drive runtime");
                    assert!(
                        !drive.track_valid,
                        "{case} has not reached Process_Movement"
                    );
                    assert!(drive.head_to.is_some(), "{case} staged a Move head");
                    assert_eq!(moving.drive_track.is_some(), expects_eager_curve, "{case}");
                    assert_eq!(
                        drive.occupation_head_to.is_some(),
                        expects_eager_curve,
                        "{case} reservation shape"
                    );
                    drive.occupation_head_to
                };
                if let Some(head) = eager_head {
                    assert_ne!((head.rx, head.ry), source_cell);
                    assert_ne!(
                        sim.substrate
                            .cell_occupation
                            .vehicle_bits(head.rx, head.ry, head.layer)
                            & crate::sim::occupancy::VEHICLE_OCCUPATION_BIT,
                        0,
                        "eager curve head is reserved before command replacement"
                    );
                }

                assert!(sim.apply_command(
                    "YuriCountry",
                    &match attack_variant {
                        0 => Command::Attack {
                            attacker_id: smin,
                            target_id: target,
                        },
                        1 => Command::ForceAttack {
                            attacker_id: smin,
                            target_id: target,
                        },
                        2 => Command::ForceAttackCell {
                            attacker_id: smin,
                            target_rx: 10,
                            target_ry: 16,
                        },
                        _ => unreachable!(),
                    },
                    Some(&rules),
                    Some(&grid),
                    &heights,
                ));
                {
                    let attacker = sim.substrate.entities.get(smin).expect("attacker");
                    assert!(attacker.navigation.nav_com.is_none(), "{case}");
                    assert!(attacker.movement_target.is_none(), "{case}");
                    assert!(attacker.drive_track.is_none(), "{case}");
                    assert_eq!(
                        attacker.mission.queued().known(),
                        Some(MissionType::Attack),
                        "{case}"
                    );
                    let expected_target = if attack_variant == 2 {
                        crate::sim::combat::TargetKind::Cell(10, 16)
                    } else {
                        crate::sim::combat::TargetKind::Entity(target)
                    };
                    assert_eq!(
                        attacker.attack_target.as_ref().map(|attack| attack.target),
                        Some(expected_target),
                        "{case}"
                    );
                    assert!(attacker.forward_deploy_retry, "{case}");
                    assert_eq!(
                        attacker.facing_target,
                        Some(0),
                        "{case} must restore YAREFN's deploy-facing owner"
                    );
                    let drive = attacker.drive_locomotion.as_ref().expect("Drive runtime");
                    assert!(drive.destination.is_none(), "{case}");
                    assert!(drive.head_to.is_none(), "{case}");
                    assert!(drive.path.directions.is_empty(), "{case}");
                    assert!(drive.occupation_head_to.is_none(), "{case}");
                    assert!(drive.occupation_handoff.is_none(), "{case}");
                    assert_eq!(
                        drive.target_speed_fraction,
                        crate::util::fixed_math::SIM_ZERO,
                        "{case}"
                    );
                    assert_eq!(
                        drive.current_speed_fraction,
                        crate::util::fixed_math::SIM_ZERO,
                        "{case}"
                    );
                    assert_eq!(drive.owner_current_speed, 0, "{case}");
                }
                if let Some(head) = eager_head {
                    assert_eq!(
                        sim.substrate
                            .cell_occupation
                            .vehicle_bits(head.rx, head.ry, head.layer)
                            & crate::sim::occupancy::VEHICLE_OCCUPATION_BIT,
                        0,
                        "cancelled eager curve cannot strand its head reservation"
                    );
                }

                // Complete the independent deploy facing between actor visits, as
                // in the near-Attack ordering acceptance. The next real production
                // tick must promote Attack and convert at the original cell; a
                // retained Move head would translate or wedge this transaction.
                for other_id in sim
                    .live_object_order_snapshot()
                    .into_iter()
                    .filter(|&id| id != smin)
                    .collect::<Vec<_>>()
                {
                    assert!(sim.unregister_live_object(other_id));
                }
                {
                    let source = sim.substrate.entities.get_mut(smin).expect("staged SMIN");
                    source.facing = 0;
                    source.facing_target = None;
                    source.body_facing = None;
                }
                sim.clear_lifecycle_test_events_for_test();
                let tick = sim.advance_tick(&[], Some(&rules), &heights, Some(&grid), None, 16);
                assert!(tick.spawned_entities, "{case}");
                assert!(sim.substrate.entities.get(smin).is_none(), "{case}");
                let deployed = sim
                    .substrate
                    .entities
                    .values()
                    .find(|entity| sim.interner.resolve(entity.type_ref) == "YAREFN")
                    .expect("same-frame replacement must deploy YAREFN");
                assert_eq!(
                    (deployed.position.rx, deployed.position.ry),
                    source_cell,
                    "{case} must not carry the conversion toward stale Move"
                );
                assert!(sim.lifecycle_test_events_for_test().iter().any(|event| {
                    matches!(
                        event,
                        LifecycleTestEvent::ForwardDeployAttackCadenceDrawn { stable_id, .. }
                            if *stable_id == smin
                    )
                }));
            }
        }
    }

    #[test]
    fn forward_deploy_retry_move_then_attack_variants_discard_unprocessed_ship_head() {
        let rules = make_test_rules();
        for (attack_variant, case) in [
            (0_u8, "object Attack"),
            (1_u8, "object ForceAttack"),
            (2_u8, "cell ForceAttack"),
        ] {
            let mut sim = Simulation::new();
            let yuri = sim.interner.intern("YuriCountry");
            let americans = sim.interner.intern("Americans");
            sim.houses
                .insert(yuri, HouseState::new(yuri, 2, Some(yuri), true, 0, 10));
            sim.houses.insert(
                americans,
                HouseState::new(americans, 0, Some(americans), true, 0, 10),
            );
            sim.session.house_order.extend([yuri, americans]);
            let heights = BTreeMap::new();
            let grid = PathGrid::new(40, 40);
            let smin = sim
                .spawn_object_at_height("SMIN", "YuriCountry", 10, 20, 0x40, 0, &rules)
                .expect("spawn Ship-shaped SMIN");
            let target = sim
                .spawn_object_at_height("TARGET", "Americans", 10, 16, 0, 0, &rules)
                .expect("spawn target");
            assert!(sim.apply_command(
                "YuriCountry",
                &Command::DeployMcv { entity_id: smin },
                Some(&rules),
                Some(&grid),
                &heights,
            ));
            {
                let source = sim.substrate.entities.get_mut(smin).expect("live SMIN");
                let locomotor = source.locomotor.as_mut().expect("locomotor");
                locomotor.kind = crate::rules::locomotor_type::LocomotorKind::Ship;
                locomotor.slot = crate::sim::movement::locomotion::LocomotorSlot::from_kind(
                    crate::rules::locomotor_type::LocomotorKind::Ship,
                );
                source.drive_locomotion = None;
                source.ship_locomotion = Some(Default::default());
            }
            assert!(sim.apply_command(
                "YuriCountry",
                &Command::Move {
                    entity_id: smin,
                    target_rx: 20,
                    target_ry: 20,
                    queue: false,
                    group_id: None,
                },
                Some(&rules),
                Some(&grid),
                &heights,
            ));
            {
                let staged = sim.substrate.entities.get(smin).expect("staged Ship curve");
                assert!(staged.drive_track.is_some(), "{case}");
                let ship = staged.ship_locomotion.as_ref().expect("Ship runtime");
                assert!(ship.head_to.is_some(), "{case}");
                assert!(
                    !ship.track_valid,
                    "{case}: command admission is not Ship Process_Movement"
                );
            }

            let command = match attack_variant {
                0 => Command::Attack {
                    attacker_id: smin,
                    target_id: target,
                },
                1 => Command::ForceAttack {
                    attacker_id: smin,
                    target_id: target,
                },
                2 => Command::ForceAttackCell {
                    attacker_id: smin,
                    target_rx: 10,
                    target_ry: 16,
                },
                _ => unreachable!(),
            };
            assert!(sim.apply_command(
                "YuriCountry",
                &command,
                Some(&rules),
                Some(&grid),
                &heights,
            ));
            let cleared = sim.substrate.entities.get(smin).expect("retasked Ship");
            assert!(cleared.navigation.nav_com.is_none(), "{case}");
            assert!(cleared.movement_target.is_none(), "{case}");
            assert!(cleared.drive_track.is_none(), "{case}");
            assert!(cleared.forward_deploy_retry, "{case}");
            assert_eq!(
                cleared.mission.queued().known(),
                Some(MissionType::Attack),
                "{case}"
            );
            let expected_target = if attack_variant == 2 {
                crate::sim::combat::TargetKind::Cell(10, 16)
            } else {
                crate::sim::combat::TargetKind::Entity(target)
            };
            assert_eq!(
                cleared.attack_target.as_ref().map(|attack| attack.target),
                Some(expected_target),
                "{case}"
            );
            let ship = cleared.ship_locomotion.as_ref().expect("Ship runtime");
            assert!(ship.destination.is_none(), "{case}");
            assert!(ship.head_to.is_none(), "{case}");
            assert!(!ship.track_valid, "{case}");
            assert!(ship.path.directions.is_empty(), "{case}");
            assert_eq!(
                ship.target_speed_fraction,
                crate::util::fixed_math::SIM_ZERO
            );
            assert_eq!(
                ship.current_speed_fraction,
                crate::util::fixed_math::SIM_ZERO
            );
            assert_eq!(ship.owner_current_speed, 0);
        }
    }

    #[test]
    fn forward_deploy_retry_processed_bridge_drive_head_survives_stop_and_attack() {
        let rules = make_test_rules();
        for (use_attack, case) in [(false, "Stop"), (true, "Attack")] {
            let mut sim = Simulation::new();
            let yuri = sim.interner.intern("YuriCountry");
            let americans = sim.interner.intern("Americans");
            sim.houses
                .insert(yuri, HouseState::new(yuri, 2, Some(yuri), true, 0, 10));
            sim.houses.insert(
                americans,
                HouseState::new(americans, 0, Some(americans), true, 0, 10),
            );
            sim.session.house_order.extend([yuri, americans]);
            let heights = BTreeMap::new();
            let grid = PathGrid::new(40, 40);
            let smin = sim
                .spawn_object_at_height("SMIN", "YuriCountry", 10, 20, 0x40, 4, &rules)
                .expect("spawn bridge SMIN");
            let target = sim
                .spawn_object_at_height("TARGET", "Americans", 10, 8, 0, 0, &rules)
                .expect("spawn Attack target");
            assert!(sim.apply_command(
                "YuriCountry",
                &Command::DeployMcv { entity_id: smin },
                Some(&rules),
                Some(&grid),
                &heights,
            ));
            assert!(sim.apply_command(
                "YuriCountry",
                &Command::Move {
                    entity_id: smin,
                    target_rx: 20,
                    target_ry: 20,
                    queue: false,
                    group_id: None,
                },
                Some(&rules),
                Some(&grid),
                &heights,
            ));
            let (old_head, track_identity) = {
                let source = sim.substrate.entities.get_mut(smin).expect("moving SMIN");
                source.on_bridge = true;
                source.bridge_occupancy =
                    Some(crate::sim::components::BridgeOccupancy { deck_level: 4 });
                let movement = source.movement_target.as_mut().expect("Move path");
                movement
                    .path_layers
                    .fill(crate::sim::movement::locomotor::MovementLayer::Bridge);
                let track = source.drive_track.as_ref().expect("Drive curve");
                let track_identity = (track.raw_track_index, track.point_index);
                let drive = source.drive_locomotion.as_mut().expect("Drive runtime");
                let reference = drive.path.reference_cell.expect("accepted Drive head");
                let old_head = (
                    u16::try_from(reference.0).expect("head x"),
                    u16::try_from(reference.1).expect("head y"),
                );
                drive.track_valid = true;
                drive.occupation_head_to = None;
                drive.occupation_handoff = None;
                (old_head, track_identity)
            };

            let command = if use_attack {
                Command::Attack {
                    attacker_id: smin,
                    target_id: target,
                }
            } else {
                Command::Stop { entity_id: smin }
            };
            assert!(sim.apply_command(
                "YuriCountry",
                &command,
                Some(&rules),
                Some(&grid),
                &heights,
            ));
            let retained = sim
                .substrate
                .entities
                .get(smin)
                .expect("retained bridge head");
            assert!(retained.navigation.nav_com.is_none(), "{case}");
            let movement = retained
                .movement_target
                .as_ref()
                .expect("processed Bridge head remains movement authority");
            assert_eq!(movement.final_goal, Some(old_head), "{case}");
            assert_eq!(
                movement.path_layers.last(),
                Some(&crate::sim::movement::locomotor::MovementLayer::Bridge),
                "{case}"
            );
            let track = retained.drive_track.as_ref().expect("live Bridge curve");
            assert_eq!(
                (track.raw_track_index, track.point_index),
                track_identity,
                "{case}"
            );
            let drive = retained.drive_locomotion.as_ref().expect("Drive runtime");
            assert!(drive.track_valid, "{case}");
            assert!(drive.occupation_head_to.is_none(), "{case}");
            assert!(
                crate::sim::movement::drive_locomotor_is_moving(retained),
                "{case}: retained Bridge curve must not freeze behind the movement-target gate"
            );
        }
    }

    #[test]
    fn forward_deploy_retry_committed_head_far_retarget_installs_new_attack_destination() {
        let rules = make_test_rules();
        for (cell_retarget, case) in [(false, "object Attack"), (true, "cell ForceAttack")] {
            let mut sim = Simulation::new();
            let yuri = sim.interner.intern("YuriCountry");
            let americans = sim.interner.intern("Americans");
            sim.houses
                .insert(yuri, HouseState::new(yuri, 2, Some(yuri), true, 0, 10));
            sim.houses.insert(
                americans,
                HouseState::new(americans, 0, Some(americans), true, 0, 10),
            );
            sim.session.house_order.extend([yuri, americans]);
            for house in sim.houses.values_mut() {
                house.multiplay_passive = true;
            }
            let heights = BTreeMap::new();
            let grid = PathGrid::new(50, 50);
            let smin = sim
                .spawn_object_at_height("SMIN", "YuriCountry", 10, 20, 0x80, 0, &rules)
                .expect("spawn retarget SMIN");
            let first_target = sim
                .spawn_object_at_height("TARGET", "Americans", 10, 5, 0, 0, &rules)
                .expect("spawn first far target");
            let second_target = sim
                .spawn_object_at_height("TARGET", "Americans", 35, 20, 0, 0, &rules)
                .expect("spawn second far target");
            let _spotter_a = sim
                .spawn_object_at_height("SPOTTER", "YuriCountry", 15, 5, 0, 0, &rules)
                .expect("spawn north spotter");
            let _spotter_b = sim
                .spawn_object_at_height("SPOTTER", "YuriCountry", 30, 20, 0, 0, &rules)
                .expect("spawn east spotter");
            assert!(sim.apply_command(
                "YuriCountry",
                &Command::DeployMcv { entity_id: smin },
                Some(&rules),
                Some(&grid),
                &heights,
            ));
            assert!(sim.apply_command(
                "YuriCountry",
                &Command::Attack {
                    attacker_id: smin,
                    target_id: first_target,
                },
                Some(&rules),
                Some(&grid),
                &heights,
            ));

            let mut committed = false;
            for _ in 0..96 {
                let _ = sim.advance_tick(&[], Some(&rules), &heights, Some(&grid), None, 16);
                committed = sim.substrate.entities.get(smin).is_some_and(|source| {
                    source.forward_deploy_retry
                        && source.mission.current().known() == Some(MissionType::Attack)
                        && source.drive_track.is_some()
                        && source
                            .drive_locomotion
                            .as_ref()
                            .is_some_and(|drive| drive.track_valid)
                });
                if committed {
                    break;
                }
            }
            assert!(
                committed,
                "{case}: first far approach must own a processed head"
            );
            let old_head = {
                let source = sim.substrate.entities.get(smin).expect("approaching SMIN");
                let reference = source
                    .drive_locomotion
                    .as_ref()
                    .expect("Drive runtime")
                    .path
                    .reference_cell
                    .expect("processed Drive head");
                (
                    u16::try_from(reference.0).expect("head x"),
                    u16::try_from(reference.1).expect("head y"),
                )
            };
            let retarget = if cell_retarget {
                Command::ForceAttackCell {
                    attacker_id: smin,
                    target_rx: 35,
                    target_ry: 20,
                }
            } else {
                Command::Attack {
                    attacker_id: smin,
                    target_id: second_target,
                }
            };
            assert!(sim.apply_command(
                "YuriCountry",
                &retarget,
                Some(&rules),
                Some(&grid),
                &heights,
            ));
            {
                let source = sim
                    .substrate
                    .entities
                    .get_mut(smin)
                    .expect("retargeted SMIN");
                assert!(source.navigation.nav_com.is_none(), "{case}");
                assert_eq!(
                    source
                        .movement_target
                        .as_ref()
                        .and_then(|movement| movement.final_goal),
                    Some(old_head),
                    "{case}: null destination retains only the processed physical head"
                );
                source
                    .mission
                    .write_dispatch_epilogue(sim.session.binary_frame as i32, 0);
            }
            let _ = sim.advance_tick(&[], Some(&rules), &heights, Some(&grid), None, 0);
            let source = sim
                .substrate
                .entities
                .get(smin)
                .expect("retarget survives zero-dt owner visit");
            assert!(source.navigation.nav_com.is_some(), "{case}");
            let movement = source
                .movement_target
                .as_ref()
                .expect("new far approach is installed");
            assert_eq!(
                movement.path.first().copied(),
                Some(old_head),
                "{case}: the new route starts at the physical head still in flight"
            );
            assert_ne!(
                movement.final_goal,
                Some(old_head),
                "{case}: the old physical head cannot remain destination authority"
            );
        }
    }

    #[test]
    fn forward_deploy_retry_move_after_current_far_attack_keeps_latch_until_per_cell() {
        let rules = make_test_rules();
        let mut sim = Simulation::new();
        let yuri = sim.interner.intern("YuriCountry");
        let americans = sim.interner.intern("Americans");
        sim.houses
            .insert(yuri, HouseState::new(yuri, 2, Some(yuri), true, 0, 10));
        sim.houses.insert(
            americans,
            HouseState::new(americans, 0, Some(americans), true, 0, 10),
        );
        sim.session.house_order.extend([yuri, americans]);
        // This is a command-interruption fixture, not a match-outcome fixture.
        // Passive houses keep defeat from freezing the native frame clock while
        // the long Drive retask runs.
        for house in sim.houses.values_mut() {
            house.multiplay_passive = true;
        }
        let heights = BTreeMap::new();
        let grid = PathGrid::new(40, 40);
        let smin = sim
            .spawn_object_at_height("SMIN", "YuriCountry", 10, 20, 0x80, 0, &rules)
            .expect("spawn SMIN");
        let target = sim
            .spawn_object_at_height("TARGET", "Americans", 10, 8, 0, 0, &rules)
            .expect("spawn far target");
        sim.spawn_object_at_height("SPOTTER", "YuriCountry", 15, 8, 0, 0, &rules)
            .expect("spawn allied sight provider");
        assert!(sim.apply_command(
            "YuriCountry",
            &Command::DeployMcv { entity_id: smin },
            Some(&rules),
            Some(&grid),
            &heights,
        ));
        assert!(sim.apply_command(
            "YuriCountry",
            &Command::Attack {
                attacker_id: smin,
                target_id: target,
            },
            Some(&rules),
            Some(&grid),
            &heights,
        ));

        let mut current_far_attack = false;
        for _ in 0..80 {
            let tick = sim.advance_tick(&[], Some(&rules), &heights, Some(&grid), None, 16);
            assert!(
                tick.frame_committed,
                "the acceptance clock must remain live"
            );
            current_far_attack = sim.substrate.entities.get(smin).is_some_and(|entity| {
                entity.forward_deploy_retry
                    && entity.mission.current().known() == Some(MissionType::Attack)
                    && entity.navigation.nav_com.is_some()
                    && entity.movement_target.is_some()
                    && entity.drive_track.is_some()
            });
            if current_far_attack {
                break;
            }
        }
        assert!(
            current_far_attack,
            "far Attack must become current and translate"
        );

        assert!(sim.apply_command(
            "YuriCountry",
            &Command::Move {
                entity_id: smin,
                target_rx: 20,
                target_ry: 20,
                queue: false,
                group_id: None,
            },
            Some(&rules),
            Some(&grid),
            &heights,
        ));
        let retasked = sim.substrate.entities.get(smin).expect("retasked SMIN");
        assert_eq!(
            retasked.mission.current().known(),
            Some(MissionType::Attack)
        );
        assert_eq!(retasked.mission.queued().known(), Some(MissionType::Move));
        assert!(retasked.navigation.nav_com.is_some());
        assert!(retasked.forward_deploy_retry);

        let mut committed = false;
        // Stock SMIN Speed=3 needs well over 160 simulation ticks to cover the
        // ten-cell retask after finishing Attack's already-committed head.
        for _ in 0..768 {
            let tick = sim.advance_tick(&[], Some(&rules), &heights, Some(&grid), None, 16);
            assert!(
                tick.frame_committed,
                "the acceptance clock must remain live"
            );
            if tick.spawned_entities {
                committed = sim
                    .substrate
                    .entities
                    .values()
                    .any(|entity| sim.interner.resolve(entity.type_ref) == "YAREFN");
                break;
            }
            assert!(
                sim.substrate
                    .entities
                    .get(smin)
                    .expect("retasked source remains")
                    .forward_deploy_retry,
                "Move after current Attack must not take the Unload-only NavCom clear"
            );
        }
        let residual = sim.substrate.entities.get(smin).map(|entity| {
            (
                (entity.position.rx, entity.position.ry),
                entity.mission.current().known(),
                entity.mission.queued().known(),
                entity.navigation.nav_com,
                entity
                    .movement_target
                    .as_ref()
                    .map(|target| (target.next_index, target.path.len(), target.final_goal)),
                entity.drive_track.as_ref().map(|track| track.point_index),
                entity.forward_deploy_retry,
            )
        });
        assert!(
            committed,
            "retasked movement must retain retry through PerCell; residual={residual:?}"
        );
    }

    #[test]
    fn forward_deploy_retry_far_attack_snapshot_replays_hash_rng_and_commit() {
        use crate::sim::snapshot::GameSnapshot;

        let rules = make_test_rules();
        let mut live = Simulation::new();
        let yuri = live.interner.intern("YuriCountry");
        let americans = live.interner.intern("Americans");
        live.houses
            .insert(yuri, HouseState::new(yuri, 2, Some(yuri), true, 0, 10));
        live.houses.insert(
            americans,
            HouseState::new(americans, 0, Some(americans), true, 0, 10),
        );
        live.session.house_order.extend([yuri, americans]);
        for house in live.houses.values_mut() {
            house.multiplay_passive = true;
        }
        let heights = BTreeMap::new();
        let grid = PathGrid::new(40, 40);
        let smin = live
            .spawn_object_at_height("SMIN", "YuriCountry", 10, 20, 0x80, 0, &rules)
            .expect("spawn SMIN");
        let target = live
            .spawn_object_at_height("TARGET", "Americans", 10, 12, 0, 0, &rules)
            .expect("spawn far target");
        live.spawn_object_at_height("SPOTTER", "YuriCountry", 15, 12, 0, 0, &rules)
            .expect("spawn allied sight provider");
        assert!(live.apply_command(
            "YuriCountry",
            &Command::DeployMcv { entity_id: smin },
            Some(&rules),
            Some(&grid),
            &heights,
        ));
        assert!(live.apply_command(
            "YuriCountry",
            &Command::Attack {
                attacker_id: smin,
                target_id: target,
            },
            Some(&rules),
            Some(&grid),
            &heights,
        ));

        let mut current_far_attack = false;
        for _ in 0..80 {
            let tick = live.advance_tick(&[], Some(&rules), &heights, Some(&grid), None, 16);
            assert!(
                tick.frame_committed,
                "the snapshot boundary clock must remain live"
            );
            current_far_attack = live.substrate.entities.get(smin).is_some_and(|entity| {
                entity.forward_deploy_retry
                    && entity.mission.current().known() == Some(MissionType::Attack)
                    && entity.attack_target.is_some()
                    && entity.navigation.nav_com.is_some()
                    && entity.movement_target.is_some()
                    && entity.drive_track.is_some()
            });
            if current_far_attack {
                break;
            }
        }
        let boundary_residual = live.substrate.entities.get(smin).map(|entity| {
            (
                entity.forward_deploy_retry,
                entity.mission.current().known(),
                entity.mission.queued().known(),
                entity.attack_target.is_some(),
                entity.navigation.nav_com,
                entity
                    .movement_target
                    .as_ref()
                    .map(|target| (target.next_index, target.path.len(), target.final_goal)),
                entity.drive_track.as_ref().map(|track| track.point_index),
            )
        });
        assert!(
            current_far_attack,
            "snapshot boundary needs a live far approach; residual={boundary_residual:?}"
        );

        live.scenario_rng = SimRng::new(0);
        let saved_hash = live.state_hash();
        let bytes = GameSnapshot::save(&live, 0, 0, "forward-deploy-far-attack", 0);
        let mut restored = GameSnapshot::load(&bytes)
            .expect("current far-Attack deploy snapshot")
            .sim;
        restored
            .restore_after_snapshot_load()
            .expect("far-Attack deploy snapshot restores structurally");
        assert_eq!(restored.state_hash(), saved_hash);
        let restored_source = restored
            .substrate
            .entities
            .get(smin)
            .expect("restored source");
        assert!(restored_source.forward_deploy_retry);
        assert_eq!(
            restored_source.mission.current().known(),
            Some(MissionType::Attack)
        );
        assert!(restored_source.attack_target.is_some());
        assert!(restored_source.navigation.nav_com.is_some());
        let restored_approach = restored_source
            .movement_target
            .as_ref()
            .expect("restored far-Attack approach");
        assert!(
            restored_approach.accel_factor > crate::util::fixed_math::SIM_ZERO,
            "Mission_Attack pursuit must carry the SMIN type's acceleration ramp"
        );
        assert!(restored_source.drive_track.is_some());

        let mut commit_step = None;
        for step in 1..=1024 {
            let live_tick = live.advance_tick(&[], Some(&rules), &heights, Some(&grid), None, 16);
            let restored_tick =
                restored.advance_tick(&[], Some(&rules), &heights, Some(&grid), None, 16);
            assert!(live_tick.frame_committed && restored_tick.frame_committed);
            assert_eq!(restored_tick.spawned_entities, live_tick.spawned_entities);
            assert_eq!(restored.state_hash(), live.state_hash());
            assert_eq!(
                restored.scenario_rng.logical_state(),
                live.scenario_rng.logical_state()
            );
            let live_yarefn = live
                .substrate
                .entities
                .values()
                .filter(|entity| live.interner.resolve(entity.type_ref) == "YAREFN")
                .count();
            let restored_yarefn = restored
                .substrate
                .entities
                .values()
                .filter(|entity| restored.interner.resolve(entity.type_ref) == "YAREFN")
                .count();
            assert_eq!(restored_yarefn, live_yarefn);
            if live_yarefn == 1 {
                assert!(live_tick.spawned_entities);
                commit_step = Some(step);
                break;
            }
        }
        let residual = live.substrate.entities.get(smin).map(|entity| {
            (
                (entity.position.rx, entity.position.ry),
                (entity.position.sub_x, entity.position.sub_y),
                entity.mission.current().known(),
                entity.mission.queued().known(),
                entity.navigation.nav_com,
                entity
                    .movement_target
                    .as_ref()
                    .map(|target| (target.next_index, target.path.len(), target.final_goal)),
                entity.drive_track.as_ref().map(|track| track.point_index),
                entity
                    .drive_locomotion
                    .as_ref()
                    .map(|drive| (drive.destination, drive.head_to, drive.track_index)),
                entity.attack_target.is_some(),
                entity.forward_deploy_retry,
            )
        });
        assert!(
            commit_step.is_some(),
            "both branches must deploy after approach; residual={residual:?}"
        );
        assert_eq!(restored.state_hash(), live.state_hash());
    }

    #[test]
    fn failed_undeploy_refunds_source_and_retains_fresh_smin_manager_pool() {
        let rules = make_test_rules();
        let seed = 0x51A7_E004;
        let mut sim = Simulation::with_seed(seed);
        sim.session.game_mode_nonzero = true;
        let yuri = sim.interner.intern("YuriCountry");
        let neutral = sim.interner.intern("Neutral");
        sim.houses
            .insert(yuri, HouseState::new(yuri, 2, Some(yuri), true, 0, 10));
        sim.houses.insert(
            neutral,
            HouseState::new(neutral, 3, Some(neutral), false, 0, 10),
        );
        sim.session.house_order.extend([yuri, neutral]);
        let yarefn = sim
            .spawn_object_at_height("YAREFN", "YuriCountry", 10, 10, 0, 0, &rules)
            .expect("spawn tagged YAREFN before restricting the playfield");
        let tag = sim.interner.intern("TAG_YAREFN_UNDEPLOY");
        sim.substrate
            .entities
            .get_mut(yarefn)
            .expect("live YAREFN")
            .attached_trigger_tag = Some(tag);
        sim.substrate
            .entities
            .get_mut(yarefn)
            .expect("live YAREFN")
            .health
            .current = 1;
        let slave_ids = sim
            .production
            .slave_bindings
            .get(&yarefn)
            .cloned()
            .expect("YAREFN constructor owns its slave manager");
        assert_eq!(slave_ids, vec![2, 3, 4, 5, 6]);
        assert!(
            sim.unlimbo_held_production_object(
                slave_ids[0],
                14,
                10,
                0,
                0,
                PlacementEvidence::EvaluateMark,
                &rules,
            )
            .is_some(),
            "one visible old slave exercises native liberation at source destruction"
        );
        sim.substrate.entities.get_mut(yarefn).unwrap().selected = true;
        sim.playfield_bounds = Some(crate::map::playfield::PlayfieldBounds {
            base: 0,
            off_fc: 0,
            off_100: 0,
            off_104: 0,
            off_108: 0,
        });

        assert_eq!(complete_undeploy_for_test(&mut sim, yarefn, &rules), None);
        let source = sim
            .substrate
            .entities
            .get(yarefn)
            .expect("failed undeploy source remains until the physical drain");
        assert!(!source.lifecycle.object_alive);
        assert!(source.lifecycle.in_limbo);
        assert!(source.dying);
        assert_eq!(source.attached_trigger_tag, Some(tag));
        assert!(sim.substrate.pending_delete.contains(&yarefn));

        let smin = sim
            .substrate
            .entities
            .values()
            .find(|entity| sim.interner.resolve(entity.type_ref) == "SMIN")
            .map(|entity| entity.stable_id)
            .expect("failed Unit Unlimbo retains the constructed SMIN");
        assert_eq!(smin, 7);
        let target = sim.substrate.entities.get(smin).unwrap();
        assert!(target.lifecycle.object_alive);
        assert!(target.lifecycle.in_limbo);
        assert!(!target.lifecycle.cell_marked);
        assert!(!target.dying);
        assert_eq!(target.health.current, target.health.max);
        assert_eq!(target.attached_trigger_tag, None);
        assert!(!target.selected);
        let fresh_slave_ids = sim
            .production
            .slave_bindings
            .get(&smin)
            .cloned()
            .expect("failed target retains its constructor-owned manager pool");
        assert_eq!(fresh_slave_ids, vec![8, 9, 10, 11, 12]);
        assert_eq!(
            sim.production.slave_bindings.get(&yarefn),
            Some(&slave_ids),
            "ObjectClass::UnInit does not run the source manager destructor"
        );
        assert!(
            sim.production
                .reverse_failure_slave_manager_finalizers
                .contains(&yarefn),
            "only reverse failure arms the deferred null-attacker destructor"
        );
        assert_eq!(sim.houses[&yuri].credits, 1750);
        assert_eq!(sim.houses[&yuri].owned_building_count, 0);
        assert_eq!(sim.houses[&yuri].owned_unit_count, 11);
        let mut expected_rng = SimRng::new(seed);
        for _ in 0..12 {
            let _ = expected_rng.next_u32();
        }
        assert_eq!(
            sim.scenario_rng.logical_state(),
            expected_rng.logical_state(),
            "source and failed target constructors each consume parent plus five child draws"
        );

        let rng_after_failure = sim.scenario_rng.logical_state();
        assert_eq!(complete_undeploy_for_test(&mut sim, yarefn, &rules), None);
        assert_eq!(sim.scenario_rng.logical_state(), rng_after_failure);
        assert_eq!(sim.houses[&yuri].credits, 1750);
        assert_eq!(
            sim.substrate
                .entities
                .values()
                .filter(|entity| sim.interner.resolve(entity.type_ref) == "SMIN")
                .count(),
            1,
            "dead-but-stored source cannot repeat construction or refund"
        );

        sim.process_pending_delete_with_rules(Some(&rules));
        assert!(sim.substrate.entities.get(yarefn).is_none());
        assert!(!sim.production.slave_bindings.contains_key(&yarefn));
        assert!(
            !sim.production
                .reverse_failure_slave_manager_finalizers
                .contains(&yarefn)
        );
        let liberated = sim
            .substrate
            .entities
            .get(slave_ids[0])
            .expect("visible old slave survives as a liberated Civilian");
        assert_eq!(liberated.owner, neutral);
        assert!(liberated.slave_harvester.is_none());
        assert!(liberated.is_active());
        assert_eq!(
            liberated.mission.current().known(),
            Some(MissionType::Guard)
        );
        for &old_limbo_slave in &slave_ids[1..] {
            assert!(
                sim.substrate.entities.get(old_limbo_slave).is_none(),
                "old limbo slaves UnInit during source manager destruction"
            );
        }

        let ghost = sim
            .substrate
            .entities
            .get(smin)
            .expect("source finalization must not delete the failed target ghost");
        assert!(ghost.lifecycle.object_alive && ghost.lifecycle.in_limbo);
        assert_eq!(
            sim.production.slave_bindings.get(&smin),
            Some(&fresh_slave_ids)
        );
        for fresh_slave_id in fresh_slave_ids {
            let fresh = sim
                .substrate
                .entities
                .get(fresh_slave_id)
                .expect("target constructor-owned slave persists");
            assert!(fresh.lifecycle.object_alive && fresh.lifecycle.in_limbo);
            assert_eq!(fresh.slave_harvester.as_ref().unwrap().master_id, smin);
        }
        assert_eq!(sim.houses[&yuri].owned_unit_count, 6);
        assert_eq!(sim.houses[&neutral].owned_unit_count, 1);
        assert!(sim.sound_events.iter().any(|event| matches!(
            event,
            crate::sim::world::SimSoundEvent::SlaveWorkerLiberated { rx: 10, ry: 10 }
        )));
        assert!(sim.substrate.pending_delete.is_empty());
    }

    #[test]
    fn failed_deploy_keeps_attached_tag_on_source() {
        let rules = make_test_rules();
        let mut sim = Simulation::with_seed(0x51A7_E003);
        let smin = sim
            .spawn_object_at_height("SMIN", "YuriCountry", 10, 10, 0, 0, &rules)
            .expect("spawn tagged SMIN before restricting the playfield");
        let tag = sim.interner.intern("TAG_SMIN_DEPLOY");
        sim.substrate
            .entities
            .get_mut(smin)
            .expect("live SMIN")
            .attached_trigger_tag = Some(tag);
        sim.playfield_bounds = Some(crate::map::playfield::PlayfieldBounds {
            base: 0,
            off_fc: 0,
            off_100: 0,
            off_104: 0,
            off_108: 0,
        });

        assert_eq!(deploy_slave_miner(&mut sim, smin, &rules), None);
        assert!(sim.substrate.entities.get(smin).unwrap().is_active());
        assert!(sim.production.slave_bindings.contains_key(&smin));
        assert_eq!(
            sim.substrate
                .entities
                .get(smin)
                .expect("failed deploy source remains resolvable")
                .attached_trigger_tag,
            Some(tag),
            "target construction failure must not detach the source tag"
        );
    }

    #[test]
    fn manhattan_distance_basic() {
        assert_eq!(manhattan_distance(10, 10, 13, 14), 7);
        assert_eq!(manhattan_distance(5, 5, 5, 5), 0);
        assert_eq!(manhattan_distance(0, 0, 100, 50), 150);
    }

    #[test]
    fn scan_correction_returns_none_without_entities() {
        // With no entities, check_scan_correction returns None (master not found).
        let sim = Simulation::new();
        let rules = make_test_rules();
        assert!(check_scan_correction(&sim, &rules, None, 999).is_none());
    }

    /// Minimal rules for slave miner tests.
    fn make_test_rules() -> RuleSet {
        use crate::rules::ini_parser::IniFile;
        let ini_str: &str = "\
[InfantryTypes]\n1=SLAV\n2=TARGET\n3=SPOTTER\n\
[VehicleTypes]\n1=SMIN\n\
[BuildingTypes]\n1=YAREFN\n\
[SLAV]\nStrength=125\nSpeed=3\nSlaved=yes\nStorage=4\nHarvestRate=150\n\
[TARGET]\nStrength=125\nSpeed=3\n\
[SPOTTER]\nStrength=100\nSight=5\n\
[SMIN]\nStrength=2000\nSpeed=3\nROT=5\nLocomotor={4A582741-9839-11D1-B709-00A024DDAFD1}\nPrimary=20mmRapid\nTurret=yes\nOpportunityFire=yes\nEnslaves=SLAV\nSlavesNumber=5\nDeploysInto=YAREFN\nResourceGatherer=yes\nResourceDestination=yes\n\
[YAREFN]\nStrength=2000\nCost=1750\nSoylent=1750\nDeployFacing=0\nEnslaves=SLAV\nSlavesNumber=5\nUndeploysInto=SMIN\nFoundation=2x2\n\
[20mmRapid]\nDamage=30\nROF=20\nRange=5.5\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,90%,70%,25%,100%,25%,25%,0%,0%\nCellSpread=0\n\
[Attack]\nRate=.016\n\
[General]\nRefundPercent=50%\nSlaveMinerShortScan=8\nSlaveMinerSlaveScan=14\nSlaveMinerLongScan=48\nSlaveMinerScanCorrection=3\nSlaveMinerKickFrameDelay=150\n\
[AudioVisual]\nSlavesFreeSound=SlaveWorkerLiberated\n\
";
        let ini: IniFile = IniFile::from_str(ini_str);
        RuleSet::from_ini(&ini).expect("test rules should parse")
    }
}
