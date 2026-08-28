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

use crate::map::actions::{ActionEntry, ActionMap, MaterializedActionOperand};
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
const ACTION_CREATE_CRATE: i32 = 108;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeActionResult {
    False,
    True,
}

impl NativeActionResult {
    fn or(self, other: Self) -> Self {
        if self == Self::True || other == Self::True {
            Self::True
        } else {
            Self::False
        }
    }
}

/// Exact synchronous delivery payload used by TagClass-style callers. Object
/// and cell identities are stable handles so repeat cleanup can re-fetch and
/// detach only references that still point at the same live Tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeTriggerEvent {
    pub event_id: i32,
    pub object_id: Option<u64>,
    pub cell: Option<(u16, u16)>,
    pub raising_owner: Option<InternedId>,
    pub data: i32,
    pub editor_mode: bool,
}

impl NativeTriggerEvent {
    fn polling(event_id: i32, data: i32) -> Self {
        Self {
            event_id,
            object_id: None,
            cell: None,
            raising_owner: None,
            data,
            editor_mode: false,
        }
    }
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
    /// Stable object identity paired with its resolved TagType. Native stores
    /// the resulting Tag pointer on the object; retaining the stable id here
    /// lets the typed runtime reproduce that pointer without reconstructing it
    /// from the definition-only `attached_tag_id` later.
    pub object_tag_types: Vec<(u64, u32)>,
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
            .filter_map(|entity| {
                let id = entity.attached_tag_id?;
                let tag_type_index = program.tag_type_index(simulation.interner.resolve(id))?;
                Some((entity.stable_id, index_u32(tag_type_index)))
            })
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
    /// Ordinary ensure/reuse Tags participate in TagType lookup. Runtime Team
    /// construction explicitly creates no-reuse Tags and must never become the
    /// replacement lookup owner when an ordinary Tag expires.
    #[serde(default)]
    pub reusable: bool,
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
    /// ObjectClass stable id -> live Tag master entry. This is the typed owner
    /// for native AttachedTag pointer identity after fresh materialization.
    #[serde(default)]
    pub object_tags: BTreeMap<u64, u32>,
    /// Native variable-change delivery bytes represented as exact typed index
    /// sets. They remain asserted for the complete source-ordered poll walk so
    /// later Tags observe synchronous Action writes in the same pass.
    #[serde(default)]
    pub dirty_globals: BTreeSet<u32>,
    #[serde(default)]
    pub dirty_locals: BTreeSet<u32>,
    /// Logical destruction is synchronous; physical compaction occurs at the
    /// late main-tick finalizer in this recorded queue order.
    #[serde(default)]
    pub pending_tag_finalization: Vec<u32>,
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
            tag.reusable.hash(hasher);
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
        self.object_tags.hash(hasher);
        self.dirty_globals.hash(hasher);
        self.dirty_locals.hash(hasher);
        self.pending_tag_finalization.hash(hasher);
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
        for &(stable_id, tag_type_index) in &attachments.object_tag_types {
            let tag_index = runtime.ensure_tag(
                program,
                tag_type_index,
                difficulty_raw,
                binary_frame,
                scenario_rng,
            );
            let tag = &mut runtime.tags[tag_index as usize];
            tag.attachment_count = tag.attachment_count.wrapping_add(1);
            runtime.object_tags.insert(stable_id, tag_index);
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
            reusable: install_reuse_lookup,
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

/// One moved-runtime transaction for the native ordered owner. Recursive
/// Force/event paths reuse this same value, so no callback can observe the
/// placeholder `Simulation.trigger_runtime` while it is temporarily taken.
struct TriggerTransaction<'state, 'data> {
    runtime: &'state mut TriggerRuntime,
    program: &'data TriggerProgram,
    simulation: &'state mut Simulation,
    rules: Option<&'data crate::rules::ruleset::RuleSet>,
    overlay_registry: Option<&'data crate::rules::overlay_types::OverlayTypeRegistry>,
    waypoints: &'data HashMap<u32, crate::map::waypoints::Waypoint>,
    effects: Vec<TriggerEffect>,
}

impl TriggerRuntime {
    /// Native leading Logic trigger rung. Global Tag iteration deliberately
    /// increments after a synchronous stable erase, preserving retail's
    /// ordinary `[A,B,C] -> A retires -> C runs` cursor behavior.
    pub(crate) fn advance_native_poll(
        &mut self,
        program: &TriggerProgram,
        simulation: &mut Simulation,
        rules: Option<&crate::rules::ruleset::RuleSet>,
        overlay_registry: Option<&crate::rules::overlay_types::OverlayTypeRegistry>,
        waypoints: &HashMap<u32, crate::map::waypoints::Waypoint>,
    ) -> Vec<TriggerEffect> {
        let mut transaction = TriggerTransaction {
            runtime: self,
            program,
            simulation,
            rules,
            overlay_registry,
            waypoints,
            effects: Vec::new(),
        };
        transaction.poll();
        transaction.effects
    }

    /// Execute one exact synchronous object/cell/capture/crate delivery against
    /// an already-resolved live Tag index.
    pub(crate) fn dispatch_native_event(
        &mut self,
        program: &TriggerProgram,
        simulation: &mut Simulation,
        rules: Option<&crate::rules::ruleset::RuleSet>,
        overlay_registry: Option<&crate::rules::overlay_types::OverlayTypeRegistry>,
        waypoints: &HashMap<u32, crate::map::waypoints::Waypoint>,
        tag_index: u32,
        event: NativeTriggerEvent,
    ) -> (bool, Vec<TriggerEffect>) {
        let mut transaction = TriggerTransaction {
            runtime: self,
            program,
            simulation,
            rules,
            overlay_registry,
            waypoints,
            effects: Vec::new(),
        };
        let fired = transaction.deliver_tag(tag_index, event);
        (fired, transaction.effects)
    }

    /// Late physical Tag/Trigger release. Logical unregister and pointer expiry
    /// happened synchronously at delivery; this stable compaction rewrites every
    /// surviving registry handle in one deterministic pass.
    pub(crate) fn finalize_pending_tags(&mut self) {
        if self.pending_tag_finalization.is_empty() {
            return;
        }
        let removed_tags: BTreeSet<u32> = self.pending_tag_finalization.iter().copied().collect();
        let mut removed_instances = BTreeSet::new();
        for &tag_index in &removed_tags {
            if let Some(tag) = self.tags.get(tag_index as usize) {
                removed_instances.extend(tag.trigger_instances.iter().copied());
            }
        }

        let mut instance_remap = vec![None; self.trigger_instances.len()];
        let mut compact_instances = Vec::with_capacity(
            self.trigger_instances.len().saturating_sub(removed_instances.len()),
        );
        for (old_index, instance) in self.trigger_instances.drain(..).enumerate() {
            if removed_instances.contains(&index_u32(old_index)) {
                continue;
            }
            let new_index = index_u32(compact_instances.len());
            instance_remap[old_index] = Some(new_index);
            compact_instances.push(instance);
        }
        for instance in &mut compact_instances {
            instance.next = instance
                .next
                .and_then(|old| instance_remap.get(old as usize).copied().flatten());
        }
        self.trigger_instances = compact_instances;

        let mut tag_remap = vec![None; self.tags.len()];
        let mut compact_tags = Vec::with_capacity(self.tags.len().saturating_sub(removed_tags.len()));
        for (old_index, mut tag) in self.tags.drain(..).enumerate() {
            if removed_tags.contains(&index_u32(old_index)) {
                continue;
            }
            tag.trigger_instances = tag
                .trigger_instances
                .into_iter()
                .filter_map(|old| instance_remap.get(old as usize).copied().flatten())
                .collect();
            let new_index = index_u32(compact_tags.len());
            tag_remap[old_index] = Some(new_index);
            compact_tags.push(tag);
        }
        self.tags = compact_tags;

        let remap_tag = |old: u32| tag_remap.get(old as usize).copied().flatten();
        for slot in &mut self.tag_by_type {
            *slot = slot.and_then(remap_tag);
        }
        self.cell_tags.retain(|_, tag| {
            if let Some(new) = remap_tag(*tag) {
                *tag = new;
                true
            } else {
                false
            }
        });
        self.object_tags.retain(|_, tag| {
            if let Some(new) = remap_tag(*tag) {
                *tag = new;
                true
            } else {
                false
            }
        });
        for registry in [
            &mut self.destroyed_event_tags,
            &mut self.polling_tags,
            &mut self.proximity_event_tags,
        ] {
            *registry = registry
                .drain(..)
                .filter_map(remap_tag)
                .collect::<Vec<_>>();
        }
        self.pending_tag_finalization.clear();
    }
}

impl TriggerTransaction<'_, '_> {
    fn poll(&mut self) {
        let pickup_any_latch = self.simulation.crate_authority.pickup_any_latch;
        let mut index = 0usize;
        while index < self.runtime.polling_tags.len() {
            let tag_index = self.runtime.polling_tags[index];
            let mut fired = pickup_any_latch
                && self.deliver_tag(tag_index, NativeTriggerEvent::polling(50, 0));

            if !fired {
                let globals = self.runtime.dirty_globals.iter().copied().collect::<Vec<_>>();
                for variable in globals {
                    let event_id = if self.runtime.globals_set.contains(&variable) {
                        EVENT_GLOBAL_IS_SET
                    } else {
                        EVENT_GLOBAL_IS_CLEAR
                    };
                    if self.deliver_tag(
                        tag_index,
                        NativeTriggerEvent::polling(event_id, variable as i32),
                    ) {
                        fired = true;
                        break;
                    }
                }
            }
            if !fired {
                let locals = self.runtime.dirty_locals.iter().copied().collect::<Vec<_>>();
                for variable in locals {
                    let event_id = if self.runtime.locals_set.contains(&variable) {
                        EVENT_LOCAL_IS_SET
                    } else {
                        EVENT_LOCAL_IS_CLEAR
                    };
                    if self.deliver_tag(
                        tag_index,
                        NativeTriggerEvent::polling(event_id, variable as i32),
                    ) {
                        fired = true;
                        break;
                    }
                }
            }
            // The unconditional Event-13 attempt is the ordinary active-retail
            // entry that also lets Event 8 and state/timer conditions evaluate.
            if !fired {
                fired = self.deliver_tag(tag_index, NativeTriggerEvent::polling(13, 0));
            }
            if !fired {
                fired = self.deliver_tag(tag_index, NativeTriggerEvent::polling(51, 0));
            }
            if !fired {
                let _ = self.deliver_tag(tag_index, NativeTriggerEvent::polling(14, 0));
            }
            // Native increments unconditionally even if delivery erased the
            // current stable-list member.
            index += 1;
        }
        self.simulation.crate_authority.pickup_any_latch = false;
        self.runtime.dirty_globals.clear();
        self.runtime.dirty_locals.clear();
    }

    fn deliver_tag(&mut self, tag_index: u32, event: NativeTriggerEvent) -> bool {
        let Some(tag) = self.runtime.tags.get(tag_index as usize) else {
            return false;
        };
        if event.editor_mode
            || tag.busy
            || tag.disabled_or_uninit
            || !tag.registered
            || self
                .program
                .tag_types
                .get(tag.tag_type_index as usize)
                .is_none()
        {
            return false;
        }
        let tag_type_index = tag.tag_type_index;
        let repeat_mode = self.program.tag_types[tag_type_index as usize].repeat_mode;
        let instances = tag.trigger_instances.clone();
        let attachment_count = tag.attachment_count;
        self.runtime.tags[tag_index as usize].busy = true;

        let mut sprung = false;
        let mut satisfied_any = false;
        for instance_index in instances {
            if !self.evaluate_instance(instance_index, repeat_mode, event) {
                continue;
            }
            satisfied_any = true;
            if repeat_mode == 1 && attachment_count != 1 {
                continue;
            }
            let _ = self.spring_instance(instance_index);
            sprung = true;
            if repeat_mode != 2 {
                if let Some(instance) = self.runtime.trigger_instances.get_mut(instance_index as usize)
                {
                    instance.pending_delete = true;
                }
            }
        }
        if let Some(tag) = self.runtime.tags.get_mut(tag_index as usize) {
            tag.busy = false;
        }

        if !satisfied_any || repeat_mode == 2 {
            return sprung;
        }
        if repeat_mode == 1 && attachment_count != 1 {
            self.detach_supplied_sources(tag_index, event);
            return false;
        }

        self.detach_supplied_sources(tag_index, event);
        self.expire_tag(tag_index);
        sprung
    }

    fn evaluate_instance(
        &mut self,
        instance_index: u32,
        repeat_mode: i32,
        raised: NativeTriggerEvent,
    ) -> bool {
        let Some(instance) = self.runtime.trigger_instances.get(instance_index as usize) else {
            return false;
        };
        if !instance.enabled || instance.pending_delete {
            return false;
        }
        if repeat_mode == 2 {
            return true;
        }
        let trigger_type_index = instance.trigger_type_index as usize;
        let Some(definition) = self.program.trigger_types.get(trigger_type_index) else {
            return false;
        };
        let events = definition.events.clone();
        if events.is_empty() {
            return false;
        }
        let mut all_true = true;
        let mut persistence = repeat_mode == 2;
        let mut last_owner = None;
        for (event_index, event) in events.iter().enumerate() {
            let bit = 1u32 << (event_index & 31);
            let prelatched = self.runtime.trigger_instances[instance_index as usize]
                .satisfied_mask
                & bit
                != 0;
            let event_true = if prelatched {
                true
            } else {
                self.evaluate_event_value(instance_index, event_index, event, raised)
            };
            if event.kind == 1 && event_true {
                persistence = true;
            }
            if event_true && event_sets_persistence(event.kind) {
                persistence = true;
            }
            if event_true && persistence && event_is_latch_eligible(event.kind) {
                self.runtime.trigger_instances[instance_index as usize].satisfied_mask |= bit;
            }
            if event_true
                && let Some(owner) = self.runtime.trigger_types[trigger_type_index]
                    .event_last_raising_owners[event_index]
            {
                last_owner = Some(owner);
            }
            all_true &= event_true;
        }
        if let Some(owner) = last_owner {
            self.runtime.trigger_instances[instance_index as usize].raising_house = Some(owner);
        }
        if all_true && persistence {
            let instance = &mut self.runtime.trigger_instances[instance_index as usize];
            reset_trigger_timer(
                instance,
                definition,
                self.simulation.session.binary_frame,
                &mut self.simulation.scenario_rng,
            );
        }
        all_true
    }

    fn evaluate_event_value(
        &mut self,
        instance_index: u32,
        event_index: usize,
        event: &EventCondition,
        raised: NativeTriggerEvent,
    ) -> bool {
        let trigger_type_index = self.runtime.trigger_instances[instance_index as usize]
            .trigger_type_index as usize;
        match event.kind {
            1 => {
                let owner_matches = event.scalar() == -1
                    || raised.raising_owner.is_some_and(|owner| {
                        let Some(rules) = self.rules else {
                            return false;
                        };
                        self.simulation
                            .houses
                            .get(&owner)
                            .and_then(|house| house.country)
                            .and_then(|country| {
                                rules.country_index(self.simulation.interner.resolve(country))
                            })
                            .is_some_and(|country| i32::from(country.0) == event.scalar())
                    });
                let accepted = raised.event_id == 1
                    && raised.object_id.is_some()
                    && owner_matches;
                if accepted && let Some(owner) = raised.raising_owner {
                    self.runtime.trigger_types[trigger_type_index]
                        .event_last_raising_owners[event_index] = Some(owner);
                }
                accepted
            }
            8 => true,
            13 | 51 => {
                let instance = &self.runtime.trigger_instances[instance_index as usize];
                (self.simulation.session.binary_frame as i32)
                    .wrapping_sub(instance.timer_start_frame)
                    >= instance.timer_duration
            }
            14 => false,
            EVENT_GLOBAL_IS_SET => {
                let index = event.scalar();
                (0..=49).contains(&index) && self.runtime.globals_set.contains(&(index as u32))
            }
            EVENT_GLOBAL_IS_CLEAR => {
                let index = event.scalar();
                (0..=49).contains(&index) && !self.runtime.globals_set.contains(&(index as u32))
            }
            EVENT_LOCAL_IS_SET => {
                let index = event.scalar();
                (0..=99).contains(&index) && self.runtime.locals_set.contains(&(index as u32))
            }
            EVENT_LOCAL_IS_CLEAR => {
                let index = event.scalar();
                (0..=99).contains(&index) && !self.runtime.locals_set.contains(&(index as u32))
            }
            EVENT_ELAPSED_SCENARIO_TIME => {
                event.scalar()
                    <= (self.simulation.session.binary_frame as i32)
                        / LOGICAL_FRAMES_PER_SECOND
            }
            EVENT_TECHTYPE_EXISTS => event.type_name().is_some_and(|type_id| {
                count_techtype_exact(self.simulation, self.rules, type_id)
                    .is_some_and(|count| count >= event.scalar())
            }),
            EVENT_TECHTYPE_DOES_NOT_EXIST => event.type_name().is_some_and(|type_id| {
                count_techtype_exact(self.simulation, self.rules, type_id) == Some(0)
            }),
            49 | 50 => raised.event_id == event.kind,
            _ => false,
        }
    }

    fn spring_instance(&mut self, instance_index: u32) -> NativeActionResult {
        let Some(instance) = self.runtime.trigger_instances.get(instance_index as usize) else {
            return NativeActionResult::False;
        };
        if !instance.enabled || instance.pending_delete {
            return NativeActionResult::False;
        }
        let trigger_type_index = instance.trigger_type_index as usize;
        let actions = self.program.trigger_types[trigger_type_index].actions.clone();
        let mut result = NativeActionResult::False;
        for action in &actions {
            result = result.or(self.execute_action(trigger_type_index, action));
        }
        result
    }

    fn execute_action(
        &mut self,
        trigger_type_index: usize,
        action: &crate::map::trigger_program::TypedActionDefinition,
    ) -> NativeActionResult {
        let scalar = match &action.materialized_operand {
            MaterializedActionOperand::Value(value) => Some(*value),
            MaterializedActionOperand::UnresolvedRegistry { .. } => None,
        };
        match action.entry.kind {
            ACTION_FORCE_TRIGGER => {
                let Some(target) = action
                    .trigger_type_target
                    .as_deref()
                    .and_then(|id| self.program.trigger_type_index(id))
                else {
                    return NativeActionResult::False;
                };
                if self.runtime.trigger_instances.is_empty() {
                    return NativeActionResult::False;
                }
                let matches = self
                    .runtime
                    .trigger_instances
                    .iter()
                    .enumerate()
                    .filter_map(|(index, instance)| {
                        (instance.trigger_type_index as usize == target).then(|| index_u32(index))
                    })
                    .collect::<Vec<_>>();
                for instance_index in matches {
                    let _ = self.spring_instance(instance_index);
                }
                NativeActionResult::True
            }
            ACTION_SET_GLOBAL | ACTION_CLEAR_GLOBAL => {
                let Some(index) = scalar.filter(|value| (0..=49).contains(value)) else {
                    return NativeActionResult::False;
                };
                self.write_variable(true, index as u32, action.entry.kind == ACTION_SET_GLOBAL);
                NativeActionResult::True
            }
            ACTION_SET_LOCAL | ACTION_CLEAR_LOCAL => {
                let Some(index) = scalar.filter(|value| (0..=99).contains(value)) else {
                    return NativeActionResult::False;
                };
                self.write_variable(false, index as u32, action.entry.kind == ACTION_SET_LOCAL);
                NativeActionResult::True
            }
            ACTION_CHANGE_VISIBLE_MAP_AREA => {
                if let Some(raw_local_size) = parse_visible_map_area(&action.entry.params) {
                    let _ = self
                        .simulation
                        .change_visible_map_area(raw_local_size, self.rules);
                }
                NativeActionResult::True
            }
            ACTION_ENABLE_TRIGGER => {
                if let Some(target) = action
                    .trigger_type_target
                    .as_deref()
                    .and_then(|id| self.program.trigger_type_index(id))
                {
                    let difficulty = self.simulation.session.trigger_difficulty_raw;
                    let matches = self
                        .runtime
                        .trigger_instances
                        .iter()
                        .enumerate()
                        .filter_map(|(index, instance)| {
                            (instance.trigger_type_index as usize == target)
                                .then(|| index_u32(index))
                        })
                        .collect::<Vec<_>>();
                    for instance_index in matches {
                        let definition = &self.program.trigger_types[target];
                        let admitted = match difficulty {
                            0 => definition.difficulty.easy,
                            1 => definition.difficulty.medium,
                            2 => definition.difficulty.hard,
                            _ => true,
                        };
                        if !admitted {
                            continue;
                        }
                        let instance =
                            &mut self.runtime.trigger_instances[instance_index as usize];
                        instance.enabled = true;
                        reset_trigger_timer(
                            instance,
                            definition,
                            self.simulation.session.binary_frame,
                            &mut self.simulation.scenario_rng,
                        );
                    }
                }
                NativeActionResult::True
            }
            ACTION_DISABLE_TRIGGER => {
                if let Some(target) = action
                    .trigger_type_target
                    .as_deref()
                    .and_then(|id| self.program.trigger_type_index(id))
                {
                    for instance in &mut self.runtime.trigger_instances {
                        if instance.trigger_type_index as usize == target {
                            instance.enabled = false;
                        }
                    }
                }
                NativeActionResult::True
            }
            ACTION_CENTER_CAMERA | ACTION_JUMP_CAMERA => {
                if let Some(waypoint) = action.entry.waypoint_index {
                    self.effects.push(TriggerEffect::CenterCameraAtWaypoint {
                        waypoint,
                        immediate: action.entry.kind == ACTION_JUMP_CAMERA,
                    });
                }
                NativeActionResult::True
            }
            ACTION_CREATE_CRATE => {
                let Some(data) = scalar else {
                    return NativeActionResult::False;
                };
                let Some(waypoint_index) = action.entry.waypoint_index else {
                    return NativeActionResult::False;
                };
                let Some(waypoint) = self.waypoints.get(&waypoint_index) else {
                    return NativeActionResult::False;
                };
                let (Some(rules), Some(overlays)) = (self.rules, self.overlay_registry) else {
                    return NativeActionResult::False;
                };
                let path_grid = self.simulation.path_grid_snapshot();
                if self.simulation.place_specific_crate(
                    rules,
                    overlays,
                    path_grid.as_deref(),
                    (waypoint.rx, waypoint.ry),
                    data,
                ) {
                    NativeActionResult::True
                } else {
                    NativeActionResult::False
                }
            }
            ACTION_SET_ALTERNATE_BASE | ACTION_CLEAR_ALTERNATE_BASE => {
                let owner = self.program.trigger_types[trigger_type_index].owner.as_deref();
                let Some(house_id) = resolve_trigger_house(self.simulation, owner, self.rules) else {
                    return NativeActionResult::False;
                };
                let cell = if action.entry.kind == ACTION_CLEAR_ALTERNATE_BASE {
                    Some((0, 0))
                } else {
                    action
                        .entry
                        .waypoint_index
                        .and_then(|index| self.waypoints.get(&index))
                        .map(|waypoint| (waypoint.rx, waypoint.ry))
                        .filter(|cell| *cell != (0, 0))
                };
                if let (Some(cell), Some(house)) = (cell, self.simulation.houses.get_mut(&house_id))
                {
                    house.alternate_base_center = cell;
                    NativeActionResult::True
                } else {
                    NativeActionResult::False
                }
            }
            ACTION_ANNOUNCE_WIN => {
                self.effects.push(TriggerEffect::MissionAnnouncement {
                    text: "Mission Accomplished".to_string(),
                });
                NativeActionResult::True
            }
            ACTION_ANNOUNCE_LOSE => {
                self.effects.push(TriggerEffect::MissionAnnouncement {
                    text: "Mission Failed".to_string(),
                });
                NativeActionResult::True
            }
            ACTION_END_SCENARIO => NativeActionResult::True,
            _ => NativeActionResult::False,
        }
    }

    fn write_variable(&mut self, global: bool, index: u32, set: bool) {
        let changed = if global {
            if set {
                self.runtime.globals_set.insert(index)
            } else {
                self.runtime.globals_set.remove(&index)
            }
        } else if set {
            self.runtime.locals_set.insert(index)
        } else {
            self.runtime.locals_set.remove(&index)
        };
        if !changed {
            return;
        }
        if global {
            self.runtime.dirty_globals.insert(index);
        } else {
            self.runtime.dirty_locals.insert(index);
        }
        let timer_instances = self
            .runtime
            .trigger_instances
            .iter()
            .enumerate()
            .filter_map(|(instance_index, instance)| {
                self.program.trigger_types[instance.trigger_type_index as usize]
                    .events
                    .iter()
                    .any(|event| {
                        if global {
                            matches!(event.kind, EVENT_GLOBAL_IS_SET | EVENT_GLOBAL_IS_CLEAR)
                                && event.scalar() == index as i32
                        } else {
                            matches!(event.kind, EVENT_LOCAL_IS_SET | EVENT_LOCAL_IS_CLEAR)
                                && event.scalar() == index as i32
                        }
                    })
                    .then(|| index_u32(instance_index))
            })
            .collect::<Vec<_>>();
        for instance_index in timer_instances {
            let instance = &mut self.runtime.trigger_instances[instance_index as usize];
            let definition = &self.program.trigger_types[instance.trigger_type_index as usize];
            reset_trigger_timer(
                instance,
                definition,
                self.simulation.session.binary_frame,
                &mut self.simulation.scenario_rng,
            );
        }
    }

    fn detach_supplied_sources(&mut self, tag_index: u32, event: NativeTriggerEvent) {
        if let Some(object_id) = event.object_id
            && self.runtime.object_tags.get(&object_id) == Some(&tag_index)
        {
            self.runtime.object_tags.remove(&object_id);
            self.runtime.tags[tag_index as usize].attachment_count = self.runtime.tags
                [tag_index as usize]
                .attachment_count
                .wrapping_sub(1);
        }
        if let Some(cell) = event.cell
            && self.runtime.cell_tags.get(&cell) == Some(&tag_index)
        {
            self.runtime.cell_tags.remove(&cell);
            let tag = &mut self.runtime.tags[tag_index as usize];
            tag.attachment_count = tag.attachment_count.wrapping_sub(1);
            if tag.attached_cell == Some(cell) {
                tag.attached_cell = None;
            }
        }
    }

    fn expire_tag(&mut self, tag_index: u32) {
        let tag_type_index = self.runtime.tags[tag_index as usize].tag_type_index;
        self.runtime.object_tags.retain(|_, tag| *tag != tag_index);
        self.runtime.cell_tags.retain(|_, tag| *tag != tag_index);
        self.runtime.destroyed_event_tags.retain(|tag| *tag != tag_index);
        self.runtime.polling_tags.retain(|tag| *tag != tag_index);
        self.runtime.proximity_event_tags.retain(|tag| *tag != tag_index);
        let tag = &mut self.runtime.tags[tag_index as usize];
        tag.attachment_count = 0;
        tag.attached_cell = None;
        tag.registered = false;
        tag.pending_finalization = true;
        if !self.runtime.pending_tag_finalization.contains(&tag_index) {
            self.runtime.pending_tag_finalization.push(tag_index);
        }
        if self.runtime.tag_by_type.get(tag_type_index as usize) == Some(&Some(tag_index)) {
            self.runtime.tag_by_type[tag_type_index as usize] = self
                .runtime
                .tags
                .iter()
                .enumerate()
                .find_map(|(index, tag)| {
                    (tag.registered && tag.reusable && tag.tag_type_index == tag_type_index)
                        .then(|| index_u32(index))
                });
        }
    }
}

fn event_is_latch_eligible(kind: i32) -> bool {
    !matches!(kind, 1 | 8 | 27 | 28 | 36 | 37 | 47 | 60 | 61)
}

fn event_sets_persistence(kind: i32) -> bool {
    matches!(kind, 13 | 14 | 51)
}

fn count_techtype_exact(
    sim: &Simulation,
    rules: Option<&crate::rules::ruleset::RuleSet>,
    type_id: &str,
) -> Option<i32> {
    if let Some(rules) = rules {
        let handle = rules.type_handle(type_id)?;
        if rules.object_by_handle(handle).id != type_id {
            return None;
        }
    } else if !sim
        .substrate
        .entities
        .values()
        .any(|entity| sim.interner.resolve(entity.type_ref) == type_id)
    {
        return None;
    }
    Some(sim.substrate
        .entities
        .keys_sorted()
        .into_iter()
        .rev()
        .filter_map(|stable_id| sim.substrate.entities.get(stable_id))
        .filter(|entity| sim.interner.resolve(entity.type_ref) == type_id)
        .count() as i32)
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
