//! Minimal runtime evaluation for map-authored triggers.
//!
//! This is intentionally narrow: it executes a small, high-value subset of
//! RA2/YR trigger behavior that already fits the current engine structure.
//! Supported today:
//! - Event 47: elapsed scenario time
//! - Event 27/28 and 36/37: global/local variable set/clear
//! - Action 22: force trigger
//! - Action 28/29: set/clear global variable
//! - Action 40: change visible map area
//! - Action 53/54: enable/disable trigger
//! - Action 48/112: center camera at waypoint
//! - Action 137/138: set/clear a House's alternate base cell
//!
//! The goal is to turn parsed trigger data into real runtime behavior without
//! committing to a full mission-script system yet.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};

use crate::map::actions::{ActionEntry, ActionMap};
use crate::map::events::{EventCondition, EventMap};
use crate::map::trigger_graph::{LinkedTrigger, TriggerGraph};
use crate::map::trigger_program::TriggerProgram;
use crate::map::triggers::TriggerMap;
use crate::map::variable_names::LocalVariableMap;
use crate::sim::intern::InternedId;
use crate::sim::rng::SimRng;
use crate::sim::world::Simulation;

const ACTION_FORCE_TRIGGER: i32 = 22;
const ACTION_SET_GLOBAL: i32 = 28;
const ACTION_CLEAR_GLOBAL: i32 = 29;
const ACTION_CHANGE_VISIBLE_MAP_AREA: i32 = 40;
const ACTION_CENTER_CAMERA: i32 = 48;
const ACTION_ENABLE_TRIGGER: i32 = 53;
const ACTION_DISABLE_TRIGGER: i32 = 54;
const ACTION_SET_LOCAL: i32 = 56;
const ACTION_CLEAR_LOCAL: i32 = 57;
const ACTION_ANNOUNCE_WIN: i32 = 67;
const ACTION_ANNOUNCE_LOSE: i32 = 68;
const ACTION_END_SCENARIO: i32 = 69;
const ACTION_JUMP_CAMERA: i32 = 112;
const ACTION_SET_ALTERNATE_BASE: i32 = 137;
const ACTION_CLEAR_ALTERNATE_BASE: i32 = 138;

const EVENT_GLOBAL_IS_SET: i32 = 27;
const EVENT_GLOBAL_IS_CLEAR: i32 = 28;
const EVENT_LOCAL_IS_SET: i32 = 36;
const EVENT_LOCAL_IS_CLEAR: i32 = 37;
const EVENT_ELAPSED_SCENARIO_TIME: i32 = 47;
const EVENT_TECHTYPE_EXISTS: i32 = 60;
const EVENT_TECHTYPE_DOES_NOT_EXIST: i32 = 61;
const LOGICAL_FRAMES_PER_SECOND: i32 = crate::util::fixed_math::RA2_LOGIC_FRAMES_PER_SECOND as i32;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TriggerEffect {
    CenterCameraAtWaypoint { waypoint: u32, immediate: bool },
    MissionAnnouncement { text: String },
    MissionResult { title: String, detail: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum MissionAnnouncementKind {
    Victory,
    Defeat,
}

/// Source-ordered attachment stream consumed by fresh trigger runtime
/// construction. It is compiled from the raw `[CellTags]` section and the
/// successfully spawned object registry; neither source is reconstructed from
/// unordered compatibility maps.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TriggerAttachmentPlan {
    pub cell_tag_types: Vec<(u32, (u16, u16))>,
    pub object_tag_types: Vec<u32>,
}

impl TriggerAttachmentPlan {
    pub(crate) fn from_loaded_map(
        program: &TriggerProgram,
        map: &crate::map::map_file::MapFile,
        simulation: &Simulation,
    ) -> Self {
        let valid_cells: HashSet<(u16, u16)> =
            map.cells.iter().map(|cell| (cell.rx, cell.ry)).collect();
        let mut occupied_tag_cells = HashSet::new();
        let mut cell_tag_types = Vec::new();
        if let Some(section) = map.ini.section("CellTags") {
            for key in section.keys() {
                let Ok(packed) = key.trim().parse::<u32>() else {
                    continue;
                };
                let cell = ((packed % 1000) as u16, (packed / 1000) as u16);
                if !valid_cells.contains(&cell) || occupied_tag_cells.contains(&cell) {
                    continue;
                }
                let Some(tag_type_index) = section
                    .get(key)
                    .and_then(|id| program.tag_type_index(id.trim()))
                else {
                    continue;
                };
                // The Cell setter marks the slot occupied only after resolving
                // a valid TagType. A later numeric alias for a rejected row can
                // therefore still become the first successful attachment.
                occupied_tag_cells.insert(cell);
                cell_tag_types.push((index_u32(tag_type_index), cell));
            }
        }

        let object_tag_types = simulation
            .entities()
            .values()
            .filter_map(|entity| entity.attached_tag_id)
            .filter_map(|id| program.tag_type_index(simulation.interner.resolve(id)))
            .map(index_u32)
            .collect();

        Self {
            cell_tag_types,
            object_tag_types,
        }
    }
}

/// One native TagClass runtime. `trigger_instances` is evaluation/list order,
/// the reverse of construction order because each new TriggerClass is
/// push-fronted into the Tag.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TagRuntime {
    pub tag_type_index: u32,
    pub trigger_instances: Vec<u32>,
    pub attachment_count: i32,
    pub attached_cell: Option<(u16, u16)>,
    pub disabled_or_uninit: bool,
    pub busy: bool,
    pub registered: bool,
    pub pending_finalization: bool,
}

/// Shared mutable state owned by one TriggerType definition. Two independent
/// Tag instances of this TriggerType observe the same per-event owner slots.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TriggerTypeRuntime {
    pub event_last_raising_owners: Vec<Option<InternedId>>,
}

/// Per-Tag TriggerClass state. The immutable Event/Action nodes remain in the
/// source-ordered TriggerProgram and are referenced by `trigger_type_index`.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TriggerInstance {
    pub trigger_type_index: u32,
    pub next: Option<u32>,
    pub raising_house: Option<InternedId>,
    pub pending_delete: bool,
    pub timer_start_frame: i32,
    /// Native +0x38 is uninitialized save residue with no semantic/CRC read.
    /// Rust always normalizes it to zero and excludes it from state hashes.
    pub opaque_timer_word: i32,
    pub timer_duration: i32,
    pub satisfied_mask: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TriggerRuntime {
    /// `BTreeSet` (not `HashSet`) so save files have a deterministic iteration
    /// order — required for replay/lockstep correctness.
    pub globals_set: BTreeSet<u32>,
    pub locals_set: BTreeSet<u32>,
    pub disabled_triggers: BTreeSet<String>,
    pub fired_one_shot_triggers: BTreeSet<String>,
    last_announcement: Option<MissionAnnouncementKind>,
    /// Tag master registry in first-materialization order.
    #[serde(default)]
    pub tags: Vec<TagRuntime>,
    /// TagType definition index -> first reusable Tag master entry.
    #[serde(default)]
    pub tag_by_type: Vec<Option<u32>>,
    /// TriggerClass global registry in actual constructor order.
    #[serde(default)]
    pub trigger_instances: Vec<TriggerInstance>,
    /// Shared TEvent owner memory, one entry per immutable TriggerType.
    #[serde(default)]
    pub trigger_types: Vec<TriggerTypeRuntime>,
    /// Successful CellClass tag slots, used for exact detach/pointer expiry.
    #[serde(default)]
    pub cell_tags: BTreeMap<(u16, u16), u32>,
    /// Three independent native category registries.
    #[serde(default)]
    pub destroyed_event_tags: Vec<u32>,
    #[serde(default)]
    pub polling_tags: Vec<u32>,
    #[serde(default)]
    pub proximity_event_tags: Vec<u32>,
}

impl TriggerRuntime {
    /// Fold mutable trigger state into the simulation's lockstep hash.
    ///
    /// The map definitions are static input, but these latches decide which
    /// later YR LogicClass trigger actions may run after a save or replay.
    pub(crate) fn hash_state(&self, hasher: &mut impl Hasher) {
        self.globals_set.len().hash(hasher);
        for value in &self.globals_set {
            value.hash(hasher);
        }

        self.locals_set.len().hash(hasher);
        for value in &self.locals_set {
            value.hash(hasher);
        }

        self.disabled_triggers.len().hash(hasher);
        for value in &self.disabled_triggers {
            value.hash(hasher);
        }

        self.fired_one_shot_triggers.len().hash(hasher);
        for value in &self.fired_one_shot_triggers {
            value.hash(hasher);
        }

        match self.last_announcement {
            None => 0u8.hash(hasher),
            Some(MissionAnnouncementKind::Victory) => 1u8.hash(hasher),
            Some(MissionAnnouncementKind::Defeat) => 2u8.hash(hasher),
        }

        self.tags.len().hash(hasher);
        for tag in &self.tags {
            tag.tag_type_index.hash(hasher);
            tag.trigger_instances.hash(hasher);
            tag.attachment_count.hash(hasher);
            tag.attached_cell.hash(hasher);
            tag.disabled_or_uninit.hash(hasher);
            tag.busy.hash(hasher);
            tag.registered.hash(hasher);
            tag.pending_finalization.hash(hasher);
        }
        self.tag_by_type.hash(hasher);
        self.trigger_instances.len().hash(hasher);
        for instance in &self.trigger_instances {
            instance.trigger_type_index.hash(hasher);
            instance.next.hash(hasher);
            instance.raising_house.hash(hasher);
            instance.pending_delete.hash(hasher);
            instance.timer_start_frame.hash(hasher);
            // Deliberately omit inert native +0x38 residue.
            instance.timer_duration.hash(hasher);
            instance.satisfied_mask.hash(hasher);
            instance.enabled.hash(hasher);
        }
        self.trigger_types.len().hash(hasher);
        for trigger_type in &self.trigger_types {
            trigger_type.event_last_raising_owners.hash(hasher);
        }
        self.cell_tags.hash(hasher);
        self.destroyed_event_tags.hash(hasher);
        self.polling_tags.hash(hasher);
        self.proximity_event_tags.hash(hasher);
    }

    /// Fresh-load native Tag/Trigger runtime materialization.
    ///
    /// gamemd-derived from `ScenarioClass__Full_Init @ 0x00686B20`, the five
    /// map attachment readers, `FUN_00684C30`'s 4/0x10/8 postpasses, and
    /// `TriggerClass__Constructor @ 0x00725FA0`.
    pub fn materialize_fresh(
        program: &TriggerProgram,
        local_variables: &LocalVariableMap,
        attachments: &TriggerAttachmentPlan,
        difficulty_raw: i32,
        binary_frame: u32,
        scenario_rng: &mut SimRng,
    ) -> Self {
        let mut runtime = Self {
            tag_by_type: vec![None; program.tag_types.len()],
            trigger_types: program
                .trigger_types
                .iter()
                .map(|definition| TriggerTypeRuntime {
                    event_last_raising_owners: vec![None; definition.events.len()],
                })
                .collect(),
            ..Self::default()
        };
        for local in local_variables.values() {
            if local.initially_set {
                runtime.locals_set.insert(local.index);
            }
        }

        // Successful Cell setters are first, in raw source order. Reusing a
        // Tag increments one shared signed count and overwrites Tag+0x30.
        for &(tag_type_index, cell) in &attachments.cell_tag_types {
            let tag_index = runtime.ensure_tag(
                program,
                tag_type_index,
                difficulty_raw,
                binary_frame,
                scenario_rng,
            );
            let tag = &mut runtime.tags[tag_index as usize];
            tag.attachment_count = tag.attachment_count.wrapping_add(1);
            tag.attached_cell = Some(cell);
            runtime.cell_tags.insert(cell, tag_index);
        }

        // Successfully spawned Units/Aircraft/Infantry/Structures already
        // occupy stable-id order in the exact section construction stream.
        for &tag_type_index in &attachments.object_tag_types {
            let tag_index = runtime.ensure_tag(
                program,
                tag_type_index,
                difficulty_raw,
                binary_frame,
                scenario_rng,
            );
            let tag = &mut runtime.tags[tag_index as usize];
            tag.attachment_count = tag.attachment_count.wrapping_add(1);
        }

        // These are three complete, independent `[Tags]` source-order walks.
        for category_mask in [0x04u32, 0x10, 0x08] {
            for (tag_type_index, definition) in program.tag_types.iter().enumerate() {
                if definition.category_bits & category_mask == 0 {
                    continue;
                }
                let tag_index = runtime.ensure_tag(
                    program,
                    index_u32(tag_type_index),
                    difficulty_raw,
                    binary_frame,
                    scenario_rng,
                );
                match category_mask {
                    0x04 => runtime.destroyed_event_tags.push(tag_index),
                    0x10 => runtime.polling_tags.push(tag_index),
                    0x08 => runtime.proximity_event_tags.push(tag_index),
                    _ => unreachable!(),
                }
            }
        }

        runtime
    }

    fn ensure_tag(
        &mut self,
        program: &TriggerProgram,
        tag_type_index: u32,
        difficulty_raw: i32,
        binary_frame: u32,
        scenario_rng: &mut SimRng,
    ) -> u32 {
        if let Some(Some(tag_index)) = self.tag_by_type.get(tag_type_index as usize) {
            return *tag_index;
        }
        self.construct_tag(
            program,
            tag_type_index,
            difficulty_raw,
            binary_frame,
            scenario_rng,
            true,
        )
    }

    /// TeamType construction is the sole native no-reuse factory: every Team
    /// gets a distinct Tag/Trigger group even when an ordinary map attachment
    /// already materialized the same TagType.
    pub fn materialize_team_tag(
        &mut self,
        program: &TriggerProgram,
        tag_type_index: u32,
        difficulty_raw: i32,
        binary_frame: u32,
        scenario_rng: &mut SimRng,
    ) -> u32 {
        self.construct_tag(
            program,
            tag_type_index,
            difficulty_raw,
            binary_frame,
            scenario_rng,
            false,
        )
    }

    fn construct_tag(
        &mut self,
        program: &TriggerProgram,
        tag_type_index: u32,
        difficulty_raw: i32,
        binary_frame: u32,
        scenario_rng: &mut SimRng,
        install_reuse_lookup: bool,
    ) -> u32 {
        let definition = &program.tag_types[tag_type_index as usize];
        let tag_index = index_u32(self.tags.len());
        // Native appends the Tag master before constructing its chain.
        self.tags.push(TagRuntime {
            tag_type_index,
            registered: true,
            ..TagRuntime::default()
        });
        if install_reuse_lookup {
            self.tag_by_type[tag_type_index as usize] = Some(tag_index);
        }

        for &trigger_type_index in &definition.trigger_type_chain {
            let trigger_definition = &program.trigger_types[trigger_type_index];
            let instance_index = index_u32(self.trigger_instances.len());
            let previous_head = self.tags[tag_index as usize]
                .trigger_instances
                .first()
                .copied();
            let mut instance = TriggerInstance {
                trigger_type_index: index_u32(trigger_type_index),
                next: previous_head,
                enabled: true,
                opaque_timer_word: 0,
                ..TriggerInstance::default()
            };
            // Global append precedes timer reset and the final enable gate.
            self.trigger_instances.push(instance.clone());
            reset_trigger_timer(
                &mut instance,
                trigger_definition,
                binary_frame,
                scenario_rng,
            );
            instance.enabled = trigger_definition.authored_enabled
                && match difficulty_raw {
                    0 => trigger_definition.difficulty.easy,
                    1 => trigger_definition.difficulty.medium,
                    2 => trigger_definition.difficulty.hard,
                    _ => true,
                };
            self.trigger_instances[instance_index as usize] = instance;
            self.tags[tag_index as usize]
                .trigger_instances
                .insert(0, instance_index);
        }
        tag_index
    }

    pub fn from_map(triggers: &TriggerMap, local_variables: &LocalVariableMap) -> Self {
        let mut runtime = TriggerRuntime::default();
        for trigger in triggers.values() {
            if !trigger.enabled || !trigger.difficulty.medium {
                runtime.disabled_triggers.insert(trigger.id.clone());
            }
        }
        for local in local_variables.values() {
            if local.initially_set {
                runtime.locals_set.insert(local.index);
            }
        }
        runtime
    }

    /// Evaluate and apply trigger actions against one authoritative gameplay frame.
    pub fn advance_at_frame(
        &mut self,
        current_frame: u32,
        graph: &TriggerGraph,
        triggers: &TriggerMap,
        events: &EventMap,
        actions: &ActionMap,
        simulation: Option<&mut Simulation>,
        rules: Option<&crate::rules::ruleset::RuleSet>,
    ) -> Vec<TriggerEffect> {
        self.advance_at_frame_with_waypoints(
            current_frame,
            graph,
            triggers,
            events,
            actions,
            simulation,
            rules,
            &HashMap::new(),
        )
    }

    /// Evaluate with the complete immutable scenario waypoint table.
    pub(crate) fn advance_at_frame_with_waypoints(
        &mut self,
        current_frame: u32,
        graph: &TriggerGraph,
        triggers: &TriggerMap,
        events: &EventMap,
        actions: &ActionMap,
        mut simulation: Option<&mut Simulation>,
        rules: Option<&crate::rules::ruleset::RuleSet>,
        waypoints: &HashMap<u32, crate::map::waypoints::Waypoint>,
    ) -> Vec<TriggerEffect> {
        let linked_by_id: BTreeMap<&str, &LinkedTrigger> = graph
            .triggers
            .iter()
            .map(|linked| (linked.trigger_id.as_str(), linked))
            .collect();

        let mut queue: VecDeque<String> = graph
            .triggers
            .iter()
            .filter(|linked| {
                self.is_trigger_ready(
                    linked,
                    triggers,
                    events,
                    current_frame,
                    simulation.as_deref(),
                )
            })
            .map(|linked| linked.trigger_id.clone())
            .collect();
        let mut queued: BTreeSet<String> = queue.iter().cloned().collect();
        let mut effects: Vec<TriggerEffect> = Vec::new();

        while let Some(trigger_id) = queue.pop_front() {
            queued.remove(&trigger_id);
            let Some(trigger) = triggers.get(&trigger_id) else {
                continue;
            };
            let Some(linked) = linked_by_id.get(trigger_id.as_str()).copied() else {
                continue;
            };
            if !self.is_trigger_ready(
                linked,
                triggers,
                events,
                current_frame,
                simulation.as_deref(),
            ) {
                continue;
            }

            if let Some(action) = actions.get(&trigger_id) {
                for entry in &action.entries {
                    self.apply_action(
                        entry,
                        &mut effects,
                        &mut queue,
                        &mut queued,
                        triggers,
                        trigger,
                        simulation.as_deref_mut(),
                        rules,
                        waypoints,
                    );
                }
            }

            if let Some(linked_trigger_id) = &trigger.linked_trigger_id {
                if triggers.contains_key(linked_trigger_id) {
                    enqueue_trigger(&mut queue, &mut queued, linked_trigger_id.clone());
                }
            }

            if !trigger.repeating {
                self.fired_one_shot_triggers.insert(trigger_id);
            }
        }

        effects
    }

    fn is_trigger_ready(
        &self,
        linked: &LinkedTrigger,
        triggers: &TriggerMap,
        events: &EventMap,
        current_frame: u32,
        simulation: Option<&Simulation>,
    ) -> bool {
        if self.disabled_triggers.contains(&linked.trigger_id) {
            return false;
        }

        let Some(trigger) = triggers.get(&linked.trigger_id) else {
            return false;
        };
        if !trigger.repeating && self.fired_one_shot_triggers.contains(&linked.trigger_id) {
            return false;
        }

        let Some(event_id) = &linked.event_id else {
            return false;
        };
        let Some(event) = events.get(event_id) else {
            return false;
        };
        !event.conditions.is_empty()
            && event
                .conditions
                .iter()
                .all(|condition| self.evaluate_event(condition, current_frame, simulation))
    }

    fn evaluate_event(
        &self,
        condition: &EventCondition,
        current_frame: u32,
        simulation: Option<&Simulation>,
    ) -> bool {
        match condition.kind {
            EVENT_ELAPSED_SCENARIO_TIME => {
                parse_i32_param(&condition.params, 0).is_some_and(|seconds| {
                    seconds <= (current_frame as i32) / LOGICAL_FRAMES_PER_SECOND
                })
            }
            EVENT_GLOBAL_IS_SET => parse_u32_param(&condition.params, 0)
                .is_some_and(|index| self.globals_set.contains(&index)),
            EVENT_GLOBAL_IS_CLEAR => parse_u32_param(&condition.params, 0)
                .is_some_and(|index| !self.globals_set.contains(&index)),
            EVENT_LOCAL_IS_SET => parse_u32_param(&condition.params, 0)
                .is_some_and(|index| self.locals_set.contains(&index)),
            EVENT_LOCAL_IS_CLEAR => parse_u32_param(&condition.params, 0)
                .is_some_and(|index| !self.locals_set.contains(&index)),
            EVENT_TECHTYPE_EXISTS => {
                let Some(sim) = simulation else { return false };
                let min_count = parse_u32_param(&condition.params, 0).unwrap_or(1);
                let Some(type_id) = condition.params.get(1).map(|value| value.trim()) else {
                    return false;
                };
                if type_id.is_empty() {
                    return false;
                }
                count_techtype(sim, type_id) >= min_count as usize
            }
            EVENT_TECHTYPE_DOES_NOT_EXIST => {
                let Some(sim) = simulation else { return false };
                let Some(type_id) = condition.params.get(1).map(|value| value.trim()) else {
                    return false;
                };
                if type_id.is_empty() {
                    return false;
                }
                count_techtype(sim, type_id) == 0
            }
            _ => false,
        }
    }

    fn apply_action(
        &mut self,
        action: &ActionEntry,
        effects: &mut Vec<TriggerEffect>,
        queue: &mut VecDeque<String>,
        queued: &mut BTreeSet<String>,
        triggers: &TriggerMap,
        trigger: &crate::map::triggers::MapTrigger,
        simulation: Option<&mut Simulation>,
        rules: Option<&crate::rules::ruleset::RuleSet>,
        waypoints: &HashMap<u32, crate::map::waypoints::Waypoint>,
    ) {
        match action.kind {
            ACTION_FORCE_TRIGGER => {
                if let Some(target) = parse_trigger_id_param(&action.params, 0) {
                    enqueue_trigger(queue, queued, target);
                }
            }
            ACTION_SET_GLOBAL => {
                if let Some(index) = parse_u32_param(&action.params, 0) {
                    self.globals_set.insert(index);
                }
            }
            ACTION_CLEAR_GLOBAL => {
                if let Some(index) = parse_u32_param(&action.params, 0) {
                    self.globals_set.remove(&index);
                }
            }
            ACTION_CHANGE_VISIBLE_MAP_AREA => {
                // `TActionClass::Read @ 0x006DD5B0` parses ActionID,
                // ParamType, Param3, then stores the four writer dwords at
                // +0x34..+0x40. ActionEntry.params is chunk[1..], so these are
                // exactly indices 2..5; params[0]/[1] must never leak in.
                if let Some(sim) = simulation
                    && let Some(raw_local_size) = parse_visible_map_area(&action.params)
                {
                    let _ = sim.change_visible_map_area(raw_local_size, rules);
                }
            }
            ACTION_ENABLE_TRIGGER => {
                if let Some(target) = parse_trigger_id_param(&action.params, 0) {
                    self.disabled_triggers.remove(&target);
                    if triggers.contains_key(&target) {
                        enqueue_trigger(queue, queued, target);
                    }
                }
            }
            ACTION_DISABLE_TRIGGER => {
                if let Some(target) = parse_trigger_id_param(&action.params, 0) {
                    self.disabled_triggers.insert(target);
                }
            }
            ACTION_SET_LOCAL => {
                if let Some(index) = parse_u32_param(&action.params, 0) {
                    self.locals_set.insert(index);
                }
            }
            ACTION_CLEAR_LOCAL => {
                if let Some(index) = parse_u32_param(&action.params, 0) {
                    self.locals_set.remove(&index);
                }
            }
            ACTION_CENTER_CAMERA => {
                if let Some(waypoint) = action.waypoint_index {
                    effects.push(TriggerEffect::CenterCameraAtWaypoint {
                        waypoint,
                        immediate: false,
                    });
                }
            }
            ACTION_JUMP_CAMERA => {
                if let Some(waypoint) = action.waypoint_index {
                    effects.push(TriggerEffect::CenterCameraAtWaypoint {
                        waypoint,
                        immediate: true,
                    });
                }
            }
            ACTION_SET_ALTERNATE_BASE => {
                let Some(sim) = simulation else { return };
                let Some(house_id) = resolve_trigger_house(sim, trigger.owner.as_deref(), rules)
                else {
                    return;
                };
                let Some(waypoint_index) = action.waypoint_index else {
                    return;
                };
                let Some(waypoint) = waypoints.get(&waypoint_index) else {
                    return;
                };
                if (waypoint.rx, waypoint.ry) == (0, 0) {
                    return;
                }
                // gamemd-derived: `TriggerAction__Execute @ 0x006DD8B0`
                // case 137 calls `FUN_006E44E0`, which rejects the packed-zero
                // waypoint and writes only `HouseClass+0x5494` through
                // `FUN_0050DFE0`.
                if let Some(house) = sim.houses.get_mut(&house_id) {
                    house.alternate_base_center = (waypoint.rx, waypoint.ry);
                }
            }
            ACTION_CLEAR_ALTERNATE_BASE => {
                let Some(sim) = simulation else { return };
                let Some(house_id) = resolve_trigger_house(sim, trigger.owner.as_deref(), rules)
                else {
                    return;
                };
                // gamemd-derived: `TriggerAction__Execute @ 0x006DD8B0`
                // case 138 calls `FUN_006E4540`, which writes packed zero only
                // to `HouseClass+0x5494` through `FUN_0050DFF0`.
                if let Some(house) = sim.houses.get_mut(&house_id) {
                    house.alternate_base_center = (0, 0);
                }
            }
            ACTION_ANNOUNCE_WIN => {
                self.last_announcement = Some(MissionAnnouncementKind::Victory);
                effects.push(TriggerEffect::MissionAnnouncement {
                    text: "Mission Accomplished".to_string(),
                });
            }
            ACTION_ANNOUNCE_LOSE => {
                self.last_announcement = Some(MissionAnnouncementKind::Defeat);
                effects.push(TriggerEffect::MissionAnnouncement {
                    text: "Mission Failed".to_string(),
                });
            }
            ACTION_END_SCENARIO => {
                let (title, detail) = match self.last_announcement {
                    Some(MissionAnnouncementKind::Victory) => (
                        "Mission Accomplished".to_string(),
                        "The scenario ended after a victory announcement.".to_string(),
                    ),
                    Some(MissionAnnouncementKind::Defeat) => (
                        "Mission Failed".to_string(),
                        "The scenario ended after a defeat announcement.".to_string(),
                    ),
                    None => (
                        "Scenario Ended".to_string(),
                        "A map trigger ended the scenario.".to_string(),
                    ),
                };
                effects.push(TriggerEffect::MissionResult { title, detail });
            }
            _ => {}
        }
    }
}

fn reset_trigger_timer(
    instance: &mut TriggerInstance,
    definition: &crate::map::trigger_program::TriggerTypeDefinition,
    binary_frame: u32,
    scenario_rng: &mut SimRng,
) {
    for (event_index, event) in definition.events.iter().enumerate() {
        let duration = match event.kind {
            13 => Some(event.scalar().wrapping_mul(15)),
            51 => {
                let scalar = event.scalar();
                let draw = scenario_rng.next_range_i32_inclusive(0, scalar);
                Some(scalar.wrapping_div(2).wrapping_add(draw).wrapping_mul(15))
            }
            _ => None,
        };
        if let Some(duration) = duration {
            instance.timer_start_frame = binary_frame as i32;
            instance.timer_duration = duration;
            instance.satisfied_mask &= !(1u32 << (event_index & 31));
        }
    }
}

fn index_u32(index: usize) -> u32 {
    u32::try_from(index).expect("active retail trigger registry index fits u32")
}

fn enqueue_trigger(
    queue: &mut VecDeque<String>,
    queued: &mut BTreeSet<String>,
    trigger_id: String,
) {
    if queued.insert(trigger_id.clone()) {
        queue.push_back(trigger_id);
    }
}

fn parse_u32_param(fields: &[String], index: usize) -> Option<u32> {
    fields.get(index)?.trim().parse::<u32>().ok()
}

/// Resolve a trigger's canonical HouseType owner to the first registered House.
///
/// gamemd-derived: `TriggerTypeClass::Read` canonicalizes the owner through
/// `HouseTypeClass__FindIndexOfName @ 0x005117D0` (alias before ID in source
/// order, with `<none>` selecting index zero). `TriggerClass::Spring @
/// 0x007265C0` passes that type index to `HouseClass__Find_By_Country_Index @
/// 0x00502D30`, whose global House-array scan returns the first matching
/// registration.
fn resolve_trigger_house(
    sim: &Simulation,
    trigger_owner: Option<&str>,
    rules: Option<&crate::rules::ruleset::RuleSet>,
) -> Option<crate::sim::intern::InternedId> {
    let rules = rules?;
    let trigger_house_type = rules.trigger_house_type_index(trigger_owner?)?;
    sim.session.house_order.iter().copied().find(|house_id| {
        sim.houses
            .get(house_id)
            .and_then(|house| house.country)
            .and_then(|country| rules.country_index(sim.interner.resolve(country)))
            == Some(trigger_house_type)
    })
}

fn parse_i32_param(fields: &[String], index: usize) -> Option<i32> {
    fields.get(index)?.trim().parse::<i32>().ok()
}

fn parse_visible_map_area(fields: &[String]) -> Option<[i32; 4]> {
    Some([
        crate::rules::ini_value::atoi_lenient(fields.get(2)?.trim()),
        crate::rules::ini_value::atoi_lenient(fields.get(3)?.trim()),
        crate::rules::ini_value::atoi_lenient(fields.get(4)?.trim()),
        crate::rules::ini_value::atoi_lenient(fields.get(5)?.trim()),
    ])
}

fn parse_trigger_id_param(fields: &[String], index: usize) -> Option<String> {
    let id = fields.get(index)?.trim();
    (!id.is_empty()).then(|| id.to_ascii_uppercase())
}

fn count_techtype(sim: &Simulation, type_id: &str) -> usize {
    sim.substrate
        .entities
        .values()
        .filter(|e| {
            sim.interner
                .resolve(e.type_ref)
                .eq_ignore_ascii_case(type_id)
        })
        .count()
}

#[cfg(test)]
#[path = "trigger_runtime_tests.rs"]
mod tests;
