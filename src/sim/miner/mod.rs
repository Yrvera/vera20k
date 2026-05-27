//! Ore miner types, configuration, and ECS component.
//!
//! Defines the Miner component (state machine), CargoBale (discrete resource
//! unit), MinerConfig (tunable defaults), and ResourceType. Attached to
//! harvester entities (CMIN = Chrono Miner, HARV = War Miner).
//!
//! ## Dependency rules
//! - Part of sim/ -- may depend on rules/ for data-driven miner detection.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

pub mod miner_dock;
mod miner_dock_sequence;
pub(crate) mod miner_system;

#[cfg(test)]
#[path = "miner_tests.rs"]
mod miner_tests;

pub(crate) use self::miner_dock_sequence::interrupt_refinery_docked_miners;
pub(crate) use self::miner_system::{extract_bale, search_local_ore};

use std::collections::BTreeMap;

use crate::rules::object_type::ObjectType;
use crate::rules::ruleset::GeneralRules;
use crate::sim::movement::facing_class::FacingClass;

/// Which kind of resource a map cell or cargo bale contains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ResourceType {
    Ore,
    Gem,
}

/// A resource node on the map — tracks type and remaining amount.
///
/// Replaces the old bare `u16` in `resource_nodes` so the sim knows whether
/// a cell contains ore or gems (affects bale value and palette).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResourceNode {
    pub resource_type: ResourceType,
    pub remaining: u16,
}

/// Which miner chassis this entity uses.
/// Determines movement behavior (drive vs chrono-teleport) and cargo capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MinerKind {
    /// Soviet War Miner (HARV): drives both ways, armed, large cargo.
    War,
    /// Allied Chrono Miner (CMIN): drives to ore, teleports back to refinery.
    Chrono,
    /// Yuri Slave Miner (SMIN): deploys into refinery (YAREFN), spawns slave infantry.
    /// Does not harvest directly — slaves harvest and deposit at the deployed building.
    Slave,
}

/// State machine for the miner harvest loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MinerState {
    /// Looking for the nearest ore/gem cell to harvest.
    SearchOre,
    /// Pathing toward the target ore cell.
    MoveToOre,
    /// Extracting bales from the current cell.
    Harvest,
    /// Heading back (or teleporting) to the assigned refinery.
    ReturnToRefinery,
    /// Waiting in the dock queue outside the refinery.
    Dock,
    /// Incrementally unloading cargo bales into credits.
    Unload,
    /// No ore found anywhere on the map; idle.
    WaitNoOre,
    /// Player issued a manual return order.
    ForcedReturn,
}

/// Sub-state machine for the refinery docking sequence.
///
/// Active when `MinerState::Dock` is the current top-level state. Mirrors
/// the stock refinery inbound radio sequence, then the unit deploy mission
/// that starts the unload FSM.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum RefineryDockPhase {
    /// Mission_Harvest state 2 sends HELLO(0x02). Accepted miners enter the
    /// refinery Contacts[] list; busy refineries can deny HELLO without
    /// evicting the current contact, but the following CAN_DOCK path can
    /// still defer with a receiver target instead of clearing the miner.
    #[default]
    Approach,
    /// Mission_Enter sends CAN_DOCK(0x0E). Building case 0x0E replies with
    /// the accepted cell (anchor + (3, 1)); only an already-there reply starts
    /// the contact-entered/facing-sync handoff. Each dispatch schedules the
    /// stock Enter retry delay.
    MissionEnter,
    /// Moving toward the accepted cell returned by CAN_DOCK. Arrival returns
    /// to Mission_Enter; accepted-cell arrival does not bypass the Enter
    /// retry timer.
    AwaitingAcceptedCell,
    /// Contact flag is set and ordinary radio 0x16 has synchronized the
    /// locomotor/facing rate timer. This is not radio 0x15 and has no unload,
    /// sound, pad-snap, or on-pad side effects.
    #[serde(alias = "Linked")]
    FaceSync,
    /// Building radio 0x15 has queued sender mission 0x10 with queued flag 0.
    /// No position snap, pad occupancy, cargo drain, deploy sound, or unload
    /// animation starts in this phase.
    MissionQueued,
    /// Unit mission 0x10 (`Mission_Deploy_Building`) runs the path/facing gate
    /// before starting the unload substate. The same facing timer initiated by
    /// radio 0x16 is sampled here; only once the gate accepts do unload-active
    /// effects start.
    Pivoting,
    /// Per-slot deposit pulse. Each timer crossing (HarvesterDumpRate × 900
    /// = 14.4 ticks) drains one StorageClass slot — all bales of one
    /// resource type at once — and emits a single BaleDepositEvent.
    /// Slot order matches gamemd: Ore (slot 0) first, Gems (slot 1) second.
    /// On the first empty-slot gate after the last drain: transition to
    /// Departing for the stock state-4 cleanup.
    Unloading,
    /// Legacy/pass-through phase for older save states. Stock unload now
    /// reaches Departing directly from the empty-slot dump gate.
    DepositCooldown,
    /// Rust's stock zero-link state-4 handoff. Normal stock refinery unload
    /// completion does not seed `Force_Track(0x47)`, play the conditional
    /// `ReleaseDockedHarvester` departure sound, or install a cached
    /// queue-cell destination. This phase clears the dock bookkeeping and
    /// returns to SearchOre/Harvest scheduling.
    Departing,
}

/// One discrete cargo bale carried by a miner.
///
/// Each harvest tick pops one bale worth of resource from the map cell and
/// pushes it into the miner's cargo hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CargoBale {
    pub resource_type: ResourceType,
    pub value: u16,
}

/// Tunable configuration for the miner/refinery/resource system.
///
/// Ship with RA2-like defaults; override for balance mods.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MinerConfig {
    // -- Bale values --
    /// Credits per ore bale.
    pub ore_bale_value: u16,
    /// Credits per gem bale.
    pub gem_bale_value: u16,

    // -- Cargo capacities (in bales) --
    /// War Miner bale capacity (1000 / 25 = 40 bales for ore).
    pub war_miner_capacity: u16,
    /// Chrono Miner bale capacity (500 / 25 = 20 bales for ore).
    pub chrono_miner_capacity: u16,

    // -- Timing (in sim ticks at 15Hz = RA2 game frames) --
    /// Ticks between each harvest action (extract one bale).
    pub harvest_tick_interval: u8,
    /// Tenths-of-a-tick between each unload action (deposit one bale).
    /// Default 144 = 14.4 ticks/bale, matching gamemd's
    /// `HarvesterDumpRate(0.016) × 900 = 14.4`. The fractional precision
    /// is preserved by counting an `unload_timer` in tenths and
    /// decrementing by 10 per tick — bales deposit on average every 14.4
    /// ticks instead of an integer-truncated 14.
    pub unload_tick_interval: u16,

    // -- Search radii --
    /// Short scan radius: cells to scan around last harvest cell (TiberiumShortScan).
    pub local_continuation_radius: u16,
    /// Long scan radius: cells to search from current position when short scan fails
    /// (TiberiumLongScan). If this also fails, falls back to unbounded global search.
    pub long_scan_radius: u16,
    /// If the nearest ore is farther than this, the miner considers it "too far"
    /// and will try local continuation first. Standard miners (HarvesterTooFarDistance).
    pub too_far_threshold_standard: u16,
    /// Too-far threshold for Chrono Miners (much larger because they teleport back)
    /// (ChronoHarvTooFarDistance).
    pub too_far_threshold_chrono: u16,
    /// Ticks to wait before re-scanning in WaitNoOre state.
    pub rescan_cooldown_ticks: u8,
}

impl Default for MinerConfig {
    fn default() -> Self {
        Self {
            ore_bale_value: 25,
            gem_bale_value: 50,
            // War Miner: 40 bales * 25 = 1000 ore, 40 * 50 = 2000 gems
            war_miner_capacity: 40,
            // Chrono Miner: 20 bales * 25 = 500 ore, 20 * 50 = 1000 gems
            chrono_miner_capacity: 20,
            // HarvesterLoadRate=2 (frames per StepTimer step). One bale requires
            // 9 steps, so interval = 2 * 9 = 18 frames/bale at 15fps (~1.2s).
            harvest_tick_interval: 18,
            // HarvesterDumpRate=0.016 min/bale × 900 (60s × 15fps) = 14.4 frames/bale.
            // Stored in tenths so fractional ticks accumulate exactly (no
            // 0.4-tick-per-bale drift from u8 truncation). War Miner full ore:
            // 40 × 14.4 = 576 ticks ≈ 38.4s. Chrono Miner: 20 × 14.4 ≈ 19.2s.
            unload_tick_interval: 144,
            local_continuation_radius: 6,
            long_scan_radius: 48,
            too_far_threshold_standard: 5,
            too_far_threshold_chrono: 50,
            // TibSun legacy: 0x69 = 105 frames at 15fps logic rate (~7 seconds).
            // Prevents aggressive re-scanning when no ore exists on the map.
            rescan_cooldown_ticks: 105,
        }
    }
}

impl MinerConfig {
    /// Create a MinerConfig from parsed `[General]` rules data.
    ///
    /// Replaces hardcoded defaults with data-driven values from rules.ini.
    /// Bale values and capacities stay at defaults (not exposed in [General]).
    pub fn from_general_rules(general: &GeneralRules) -> Self {
        // HarvesterLoadRate: frames per step. 9 steps per bale.
        let load_rate = general.harvester_load_rate.max(1);
        let harvest_interval = (load_rate * 9).min(255) as u8;
        // HarvesterDumpRate is a double in gamemd (default 0.016 min/bale).
        // We store frames × 10 so the 0.4-frame fraction at default rate is
        // preserved exactly: 0.016 × 9000 = 144 tenths = 14.4 ticks.
        let unload_interval = general.harvester_dump_tenths.max(1);

        Self {
            local_continuation_radius: general.tiberium_short_scan.max(1) as u16,
            long_scan_radius: general.tiberium_long_scan.max(1) as u16,
            too_far_threshold_standard: general.harvester_too_far_distance.max(1) as u16,
            too_far_threshold_chrono: general.chrono_harv_too_far_distance.max(1) as u16,
            harvest_tick_interval: harvest_interval,
            unload_tick_interval: unload_interval,
            ..Self::default()
        }
    }
}

/// ECS component: miner state machine and cargo hold.
///
/// Attached to harvester entities alongside Position, Owner, TypeRef, etc.
/// The miner_system tick reads and mutates this each frame.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Miner {
    pub kind: MinerKind,
    pub state: MinerState,
    /// StableEntityId of the "home" refinery (may change after unloading).
    pub home_refinery: Option<u64>,
    /// StableEntityId of the refinery this miner has reserved a dock slot at.
    pub reserved_refinery: Option<u64>,
    /// The ore/gem cell we are currently targeting.
    pub target_ore_cell: Option<(u16, u16)>,
    /// Discrete cargo bales currently carried.
    pub cargo: Vec<CargoBale>,
    /// Maximum number of bales this miner can carry.
    pub capacity_bales: u16,
    /// Countdown timer for the next harvest action.
    pub harvest_timer: u8,
    /// Countdown timer for the next unload action, in tenths-of-a-tick.
    /// Decremented by 10 each sim tick; when ≤ 0 a bale deposits and the
    /// timer is incremented by `unload_tick_interval` (preserving any
    /// negative leftover so average cadence matches the configured rate).
    pub unload_timer: i16,
    /// Whether the player issued a manual return order.
    pub forced_return: bool,
    /// Whether this miner is queued (but not yet occupying) a dock.
    pub dock_queued: bool,
    /// Cooldown ticks before re-scanning for ore in WaitNoOre state.
    pub rescan_cooldown: u8,
    /// Archive ("ghost cell") of a nearby still-productive ore patch,
    /// saved on the `Harvest` → `ReturnToRefinery` transition (when the
    /// miner becomes full). Survives the entire dock cycle so the next
    /// `SearchOre` returns directly to it; consumed and cleared at
    /// `SearchOre` entry.
    pub last_harvest_cell: Option<(u16, u16)>,
    /// Current phase of the refinery docking sequence.
    /// Only meaningful when `state == MinerState::Dock`.
    pub dock_phase: RefineryDockPhase,
    /// Active 16-bit FacingClass timer for the refinery dock pivot.
    ///
    /// gamemd does not rotate the docked miner by manual 8-bit facing steps.
    /// Radio 0x16 calls the locomotor's `Do_Turn(0x4000)`, which drives the
    /// unit body through its PrimaryFacing RateTimer until deploy accepts the
    /// target-facing window.
    #[serde(default)]
    pub dock_pivot_facing: Option<FacingClass>,
    /// Stock Mission_Enter retry timer start frame (`MissionClass +0xC8`).
    /// Used after CAN_DOCK dispatches; accepted-cell arrival does not bypass it.
    #[serde(default)]
    pub dock_enter_retry_start_frame: Option<u32>,
    /// Stock Mission_Enter retry duration (`MissionClass +0xD0`), in frames.
    #[serde(default)]
    pub dock_enter_retry_duration: u8,
    /// MissionClass +0xC8 for queued mission 0x10 (`Unload`).
    #[serde(default)]
    pub mission_deploy_start_frame: Option<u32>,
    /// MissionClass +0xD0 for queued mission 0x10 (`Unload`).
    #[serde(default)]
    pub mission_deploy_duration: u8,
    /// Unit+0x6D1 unload-active latch.
    #[serde(default)]
    pub unload_active: bool,
    /// Unit+0xF8 dump accumulator.
    #[serde(default)]
    pub unload_accumulator: i32,
    /// Unit+0xFC timer-fired marker.
    #[serde(default)]
    pub unload_timer_fired: bool,
    /// Unit+0x100 timer-cluster start frame.
    #[serde(default)]
    pub unload_cluster_start_frame: Option<u32>,
    /// Unit+0x104 opaque timer-cluster scratch.
    #[serde(default)]
    pub unload_cluster_scratch: i32,
    /// Unit+0x108 timer-cluster duration.
    #[serde(default)]
    pub unload_cluster_duration: u32,
    /// Unit+0x10C timer-cluster repeat interval / active flag.
    #[serde(default)]
    pub unload_cluster_repeat: u32,
    /// Unit+0x110 accumulator increment step. Constructor default is 1.
    #[serde(default = "default_unload_accumulator_step")]
    pub unload_accumulator_step: i32,
    /// Sim ticks remaining in legacy `DepositCooldown` save states.
    /// Stock unload completion now reaches Departing directly from the
    /// empty-slot dump gate, so new unloads should leave this at 0.
    pub deposit_cooldown_ticks: u16,
    /// Legacy/conditional exit cell cache. Stock zero-link refinery unload
    /// completion does not install a queue-cell destination; this remains
    /// serialized so old saves and conditional release experiments can be
    /// cleaned up deterministically.
    pub exit_cell: Option<(u16, u16)>,
}

impl Miner {
    /// Create a new miner in SearchOre state with the given kind and config.
    ///
    /// `obj_storage` is the per-unit `Storage=` value from rules.ini (in bales).
    /// When > 0 it overrides the kind-based default in `config`, matching gamemd
    /// reading TechnoTypeClass+0x800 directly as the harvester's max capacity.
    /// Pass 0 to use the kind default.
    pub fn new(kind: MinerKind, config: &MinerConfig, obj_storage: u16) -> Self {
        let capacity_bales = match kind {
            MinerKind::War => {
                if obj_storage > 0 {
                    obj_storage
                } else {
                    config.war_miner_capacity
                }
            }
            MinerKind::Chrono => {
                if obj_storage > 0 {
                    obj_storage
                } else {
                    config.chrono_miner_capacity
                }
            }
            // Slave Miners don't carry cargo — their slave infantry harvest instead.
            MinerKind::Slave => 0,
        };
        Self {
            kind,
            state: MinerState::SearchOre,
            home_refinery: None,
            reserved_refinery: None,
            target_ore_cell: None,
            cargo: Vec::with_capacity(capacity_bales as usize),
            capacity_bales,
            harvest_timer: 0,
            unload_timer: 0,
            forced_return: false,
            dock_queued: false,
            rescan_cooldown: 0,
            last_harvest_cell: None,
            dock_phase: RefineryDockPhase::default(),
            dock_pivot_facing: None,
            dock_enter_retry_start_frame: None,
            dock_enter_retry_duration: 0,
            mission_deploy_start_frame: None,
            mission_deploy_duration: 0,
            unload_active: false,
            unload_accumulator: 0,
            unload_timer_fired: false,
            unload_cluster_start_frame: None,
            unload_cluster_scratch: 0,
            unload_cluster_duration: 0,
            unload_cluster_repeat: 0,
            unload_accumulator_step: default_unload_accumulator_step(),
            deposit_cooldown_ticks: 0,
            exit_cell: None,
        }
    }

    /// True when cargo is at capacity.
    pub fn is_full(&self) -> bool {
        self.cargo.len() as u16 >= self.capacity_bales
    }

    /// How many of the 5 UI pips should be filled.
    /// Each pip = 20% of capacity, rounded down.
    pub fn cargo_pips(&self) -> u8 {
        if self.capacity_bales == 0 {
            return 0;
        }
        let ratio = (self.cargo.len() as u32 * 5) / self.capacity_bales as u32;
        (ratio as u8).min(5)
    }

    /// Total credit value of all bales currently in the hold.
    pub fn cargo_value(&self) -> u32 {
        self.cargo.iter().map(|b| b.value as u32).sum()
    }
}

fn default_unload_accumulator_step() -> i32 {
    1
}

/// Determine the miner chassis from parsed rules data.
///
/// Detection priority:
/// 1. `Enslaves=` present → Slave Miner (SMIN). Does NOT have `Harvester=yes`.
/// 2. `Harvester=yes` + `Teleporter=yes` → Chrono Miner (CMIN).
/// 3. `Harvester=yes` → War Miner (HARV).
pub fn miner_kind_for_object(object: &ObjectType) -> Option<MinerKind> {
    // Slave Miner detected via Enslaves= (SMIN does NOT have Harvester=yes).
    if object.enslaves.is_some() {
        return Some(MinerKind::Slave);
    }

    if !object.harvester {
        return None;
    }

    if object.teleporter {
        Some(MinerKind::Chrono)
    } else {
        Some(MinerKind::War)
    }
}

/// Reduce ore/gem density on a cell by `amount` density levels.
///
/// Returns the number of density levels actually removed. If the cell is
/// fully depleted, removes the resource node entirely.
///
/// Mirrors `CellClass::Reduce_Tiberium` (0x00480a80) in gamemd.exe.
/// Called by the combat system after warhead detonation.
pub(crate) fn reduce_tiberium(
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    cell: (u16, u16),
    amount: u16,
) -> u16 {
    if amount == 0 {
        return 0;
    }
    // Read type and density before deciding partial vs full removal.
    let (base, density_levels) = match resource_nodes.get(&cell) {
        Some(node) => {
            let base: u16 = match node.resource_type {
                ResourceType::Ore => 120,
                ResourceType::Gem => 180,
            };
            (base, node.remaining / base)
        }
        None => return 0,
    };
    if density_levels == 0 {
        return 0;
    }

    if amount < density_levels {
        // Partial reduction: reduce remaining by amount × base.
        resource_nodes.get_mut(&cell).unwrap().remaining -= amount * base;
        amount
    } else {
        // Full removal: destroy the resource node entirely.
        resource_nodes.remove(&cell);
        density_levels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_war_miner_ore_payout() {
        let cfg = MinerConfig::default();
        // War Miner full ore: capacity * ore_bale_value = 40 * 25 = 1000
        assert_eq!(
            cfg.war_miner_capacity as u32 * cfg.ore_bale_value as u32,
            1000
        );
    }

    #[test]
    fn default_config_war_miner_gem_payout() {
        let cfg = MinerConfig::default();
        // War Miner full gems: 40 * 50 = 2000
        assert_eq!(
            cfg.war_miner_capacity as u32 * cfg.gem_bale_value as u32,
            2000
        );
    }

    #[test]
    fn default_config_chrono_miner_ore_payout() {
        let cfg = MinerConfig::default();
        // Chrono Miner full ore: 20 * 25 = 500
        assert_eq!(
            cfg.chrono_miner_capacity as u32 * cfg.ore_bale_value as u32,
            500
        );
    }

    #[test]
    fn default_config_chrono_miner_gem_payout() {
        let cfg = MinerConfig::default();
        // Chrono Miner full gems: 20 * 50 = 1000
        assert_eq!(
            cfg.chrono_miner_capacity as u32 * cfg.gem_bale_value as u32,
            1000
        );
    }

    #[test]
    fn cargo_pips_shows_five_steps() {
        let cfg = MinerConfig::default();
        let mut miner = Miner::new(MinerKind::War, &cfg, 0);
        assert_eq!(miner.cargo_pips(), 0);
        // Fill 20% (8 of 40 bales)
        for _ in 0..8 {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Ore,
                value: 25,
            });
        }
        assert_eq!(miner.cargo_pips(), 1);
        // Fill 40%
        for _ in 0..8 {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Ore,
                value: 25,
            });
        }
        assert_eq!(miner.cargo_pips(), 2);
        // Fill 100%
        while !miner.is_full() {
            miner.cargo.push(CargoBale {
                resource_type: ResourceType::Ore,
                value: 25,
            });
        }
        assert_eq!(miner.cargo_pips(), 5);
    }

    #[test]
    fn miner_kind_detection_is_data_driven() {
        let mut war = ObjectType::from_ini_section(
            "MODHARV",
            &crate::rules::ini_parser::IniFile::from_str("[MODHARV]\nHarvester=yes\n")
                .section("MODHARV")
                .expect("section"),
            crate::rules::object_type::ObjectCategory::Vehicle,
        );
        assert_eq!(miner_kind_for_object(&war), Some(MinerKind::War));

        war.teleporter = true;
        assert_eq!(miner_kind_for_object(&war), Some(MinerKind::Chrono));

        let non_harvester = ObjectType::from_ini_section(
            "E1",
            &crate::rules::ini_parser::IniFile::from_str("[E1]\n")
                .section("E1")
                .expect("section"),
            crate::rules::object_type::ObjectCategory::Infantry,
        );
        assert_eq!(miner_kind_for_object(&non_harvester), None);
    }

    #[test]
    fn from_general_rules_overrides_scan_radii() {
        let mut general = GeneralRules::default();
        general.tiberium_short_scan = 10;
        general.tiberium_long_scan = 60;
        general.harvester_too_far_distance = 8;
        general.chrono_harv_too_far_distance = 40;

        let cfg = MinerConfig::from_general_rules(&general);
        assert_eq!(cfg.local_continuation_radius, 10);
        assert_eq!(cfg.long_scan_radius, 60);
        assert_eq!(cfg.too_far_threshold_standard, 8);
        assert_eq!(cfg.too_far_threshold_chrono, 40);
        // Bale values stay at defaults.
        assert_eq!(cfg.ore_bale_value, 25);
        assert_eq!(cfg.gem_bale_value, 50);
    }

    #[test]
    fn reduce_tiberium_partial_ore() {
        let mut nodes = BTreeMap::new();
        // 6 density levels of ore: remaining = 6 * 120 = 720.
        nodes.insert(
            (5, 5),
            ResourceNode {
                resource_type: ResourceType::Ore,
                remaining: 720,
            },
        );
        let removed = reduce_tiberium(&mut nodes, (5, 5), 2);
        assert_eq!(removed, 2);
        assert_eq!(nodes.get(&(5, 5)).unwrap().remaining, 720 - 2 * 120);
    }

    #[test]
    fn reduce_tiberium_full_removal_ore() {
        let mut nodes = BTreeMap::new();
        // 3 density levels: remaining = 360.
        nodes.insert(
            (5, 5),
            ResourceNode {
                resource_type: ResourceType::Ore,
                remaining: 360,
            },
        );
        let removed = reduce_tiberium(&mut nodes, (5, 5), 12);
        assert_eq!(removed, 3, "should return old density_levels");
        assert!(nodes.get(&(5, 5)).is_none(), "node should be removed");
    }

    #[test]
    fn reduce_tiberium_exact_density_is_full_removal() {
        let mut nodes = BTreeMap::new();
        // 5 density levels: remaining = 600.
        nodes.insert(
            (5, 5),
            ResourceNode {
                resource_type: ResourceType::Ore,
                remaining: 600,
            },
        );
        // amount(5) >= density_levels(5) → full removal (amount < density is false).
        let removed = reduce_tiberium(&mut nodes, (5, 5), 5);
        assert_eq!(removed, 5);
        assert!(nodes.get(&(5, 5)).is_none(), "exact match = full removal");
    }

    #[test]
    fn reduce_tiberium_empty_cell() {
        let mut nodes: BTreeMap<(u16, u16), ResourceNode> = BTreeMap::new();
        let removed = reduce_tiberium(&mut nodes, (5, 5), 10);
        assert_eq!(removed, 0);
    }

    #[test]
    fn reduce_tiberium_zero_amount() {
        let mut nodes = BTreeMap::new();
        nodes.insert(
            (5, 5),
            ResourceNode {
                resource_type: ResourceType::Ore,
                remaining: 720,
            },
        );
        let removed = reduce_tiberium(&mut nodes, (5, 5), 0);
        assert_eq!(removed, 0);
        assert_eq!(nodes.get(&(5, 5)).unwrap().remaining, 720, "unchanged");
    }

    #[test]
    fn reduce_tiberium_gem_base_rate() {
        let mut nodes = BTreeMap::new();
        // 4 density levels of gems: remaining = 4 * 180 = 720.
        nodes.insert(
            (5, 5),
            ResourceNode {
                resource_type: ResourceType::Gem,
                remaining: 720,
            },
        );
        let removed = reduce_tiberium(&mut nodes, (5, 5), 2);
        assert_eq!(removed, 2);
        assert_eq!(nodes.get(&(5, 5)).unwrap().remaining, 720 - 2 * 180);
    }
}
