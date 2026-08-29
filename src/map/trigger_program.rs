//! Immutable source-ordered trigger definitions.
//!
//! Runtime state lives in `sim::trigger_runtime`; this module preserves the
//! native definition/list identities that mutable Tag and Trigger instances
//! reference. In particular, no execution order is reconstructed from the
//! compatibility HashMaps exposed by the older parsers.

use std::collections::{HashMap, HashSet};

use crate::map::actions::{ActionEntry, ActionMap, MaterializedActionOperand};
use crate::map::events::{EventCondition, EventMap};
use crate::map::tags::TagMap;
use crate::map::triggers::{TriggerDifficulty, TriggerMap};
use crate::rules::ini_parser::IniFile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerProgram {
    /// TriggerType definitions in `[Triggers]` source order.
    pub trigger_types: Vec<TriggerTypeDefinition>,
    /// TagType definitions in `[Tags]` source order.
    pub tag_types: Vec<TagTypeDefinition>,
    trigger_type_index: HashMap<String, usize>,
    tag_type_index: HashMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerTypeDefinition {
    pub id: String,
    pub owner: Option<String>,
    pub linked_trigger_type_id: Option<String>,
    pub authored_enabled: bool,
    pub difficulty: TriggerDifficulty,
    /// Native TEvent linked-list order: reverse textual CSV-chunk order.
    pub events: Vec<EventCondition>,
    /// Native TAction linked-list order: textual CSV-chunk order.
    pub actions: Vec<TypedActionDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagTypeDefinition {
    pub id: String,
    pub repeat_mode: i32,
    pub name: String,
    pub trigger_type_head_id: Option<String>,
    /// Forward TriggerType chain. Runtime construction appends these globally
    /// in this order and push-fronts them into the Tag-local list.
    pub trigger_type_chain: Vec<usize>,
    /// Native `TagTypeClass::GetEventCategoryBitmask` projection across the
    /// complete TriggerType chain. Initial runtime materialization uses bits
    /// 4, 0x10, and 8 in three independent source-order postpasses.
    pub category_bits: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedActionDefinition {
    pub entry: ActionEntry,
    /// Native `TActionClass+0x90`, materialized once at read/compile time.
    pub materialized_operand: MaterializedActionOperand,
    /// Param3's exact TriggerType identity for Actions 22/53/54.
    pub trigger_type_target: Option<String>,
}

impl TriggerProgram {
    pub fn compile(
        ini: &IniFile,
        tags: &TagMap,
        triggers: &TriggerMap,
        events: &EventMap,
        actions: &ActionMap,
    ) -> Result<Self, String> {
        let trigger_order = source_order(ini, "Triggers", triggers);
        let tag_order = source_order(ini, "Tags", tags);

        let mut trigger_types = Vec::with_capacity(trigger_order.len());
        let mut trigger_type_index = HashMap::with_capacity(trigger_order.len());
        for id in trigger_order {
            let definition = triggers
                .get(&id)
                .ok_or_else(|| format!("ordered TriggerType {id} is missing"))?;
            let event_nodes = events.get(&id).map_or(&[][..], |row| row.conditions.as_slice());
            if let Some(row) = events.get(&id) {
                validate_event_row(row)?;
            }
            let mut runtime_events = event_nodes.to_vec();
            runtime_events.reverse();

            let action_nodes = if let Some(row) = actions.get(&id) {
                validate_action_row(row)?;
                row.entries
                    .iter()
                    .cloned()
                    .map(compile_action)
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                Vec::new()
            };

            let index = trigger_types.len();
            trigger_type_index.insert(id.clone(), index);
            trigger_types.push(TriggerTypeDefinition {
                id,
                owner: definition.owner.clone(),
                linked_trigger_type_id: definition.linked_trigger_id.clone(),
                authored_enabled: definition.enabled,
                difficulty: definition.difficulty.clone(),
                events: runtime_events,
                actions: action_nodes,
            });
        }

        validate_force_graph(&trigger_types)?;

        let mut tag_types = Vec::with_capacity(tag_order.len());
        let mut tag_type_index = HashMap::with_capacity(tag_order.len());
        for id in tag_order {
            let tag = tags
                .get(&id)
                .ok_or_else(|| format!("ordered TagType {id} is missing"))?;
            if !(0..=2).contains(&tag.repeat_mode) {
                return Err(format!(
                    "TagType {id} has invalid repeat mode {}",
                    tag.repeat_mode
                ));
            }
            let trigger_type_chain = compile_chain(
                tag.trigger_type_head_id.as_deref(),
                &trigger_types,
                &trigger_type_index,
                &id,
            )?;
            let category_bits = trigger_type_chain.iter().fold(0, |bits, &trigger_index| {
                bits | trigger_category_bits(&trigger_types[trigger_index])
            });
            let index = tag_types.len();
            tag_type_index.insert(id.clone(), index);
            tag_types.push(TagTypeDefinition {
                id,
                repeat_mode: tag.repeat_mode,
                name: tag.name.clone(),
                trigger_type_head_id: tag.trigger_type_head_id.clone(),
                trigger_type_chain,
                category_bits,
            });
        }

        Ok(Self {
            trigger_types,
            tag_types,
            trigger_type_index,
            tag_type_index,
        })
    }

    pub fn trigger_type_index(&self, id: &str) -> Option<usize> {
        self.trigger_type_index.get(&id.to_ascii_uppercase()).copied()
    }

    pub fn tag_type_index(&self, id: &str) -> Option<usize> {
        self.tag_type_index.get(&id.to_ascii_uppercase()).copied()
    }
}

impl Default for TriggerProgram {
    fn default() -> Self {
        let ini = IniFile::from_str("");
        Self::compile(
            &ini,
            &TagMap::new(),
            &TriggerMap::new(),
            &EventMap::new(),
            &ActionMap::new(),
        )
        .expect("empty trigger program is valid")
    }
}

/// `TriggerTypeClass::GetEventCategoryBitmask @ 0x007271E0` combines the
/// exhaustive Event switch at `0x0071F680` with the Action switch at
/// `0x006E3EE0`. The latter contributes only bit 2 and therefore cannot by
/// itself enter any of the 4/0x10/8 materialization registries.
fn trigger_category_bits(trigger: &TriggerTypeDefinition) -> u32 {
    let event_bits = trigger.events.iter().fold(0, |bits, event| {
        bits | match event.kind {
            0 | 1 | 4 | 8 | 0x18 | 0x19 | 0x1A | 0x1F | 0x35 | 0x36 | 0x3B => 0x01,
            _ => 0,
        } | match event.kind {
            0 | 1 | 2 | 4 | 6 | 7 | 8 | 0x1D | 0x21..=0x2C | 0x30 | 0x31 => 0x02,
            _ => 0,
        } | match event.kind {
            8 | 0x18 => 0x04,
            _ => 0,
        } | match event.kind {
            3 | 5 | 8..=0x16 | 0x1E | 0x20 | 0x34 | 0x37..=0x3A => 0x08,
            _ => 0,
        } | match event.kind {
            8 | 0x0D | 0x0E | 0x17 | 0x1B | 0x1C | 0x24 | 0x25 | 0x2D..=0x2F
            | 0x32 | 0x33 | 0x3C | 0x3D => 0x10,
            _ => 0,
        }
    });
    trigger.actions.iter().fold(event_bits, |bits, action| {
        bits | match action.entry.kind {
            0x0E | 0x20 | 0x3C | 0x3D | 0x3E | 0x5B | 0x6F => 0x02,
            _ => 0,
        }
    })
}

fn source_order<T>(ini: &IniFile, section_name: &str, entries: &HashMap<String, T>) -> Vec<String> {
    let mut result = Vec::with_capacity(entries.len());
    if let Some(section) = ini.section(section_name) {
        for key in section.keys() {
            let id = key.trim().to_ascii_uppercase();
            if entries.contains_key(&id) {
                result.push(id);
            }
        }
    }
    // Synthetic callers can supply typed maps without raw INI. Keep that
    // compatibility deterministic, while production always takes the branch
    // above and therefore preserves source order.
    if result.is_empty() && !entries.is_empty() {
        result.extend(entries.keys().cloned());
        result.sort();
    }
    result
}

fn validate_event_row(event: &crate::map::events::MapEvent) -> Result<(), String> {
    let declared = event
        .fields
        .first()
        .map(|value| crate::rules::ini_value::atoi_lenient(value))
        .unwrap_or(0);
    if declared < 0 || declared as usize != event.conditions.len() {
        return Err(format!(
            "Event row {} declared {declared} entries but materialized {}",
            event.id,
            event.conditions.len()
        ));
    }
    for condition in &event.conditions {
        match condition.kind {
            27 | 28 if !(0..=49).contains(&condition.scalar()) => {
                return Err(format!(
                    "Event row {} has out-of-range global index {}",
                    event.id,
                    condition.scalar()
                ));
            }
            36 | 37 if !(0..=99).contains(&condition.scalar()) => {
                return Err(format!(
                    "Event row {} has out-of-range local index {}",
                    event.id,
                    condition.scalar()
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_action_row(action: &crate::map::actions::MapAction) -> Result<(), String> {
    let declared = action
        .fields
        .first()
        .map(|value| crate::rules::ini_value::atoi_lenient(value))
        .unwrap_or(0);
    if declared < 0 || declared as usize != action.entries.len() {
        return Err(format!(
            "Action row {} declared {declared} entries but materialized {}",
            action.id,
            action.entries.len()
        ));
    }
    Ok(())
}

fn compile_action(entry: ActionEntry) -> Result<TypedActionDefinition, String> {
    let materialized_operand = entry.materialized_operand();
    if entry.kind == 108
        && matches!(
            &materialized_operand,
            MaterializedActionOperand::UnresolvedRegistry { .. }
        )
    {
        return Err("Action 108 requires an unresolved dialog/sound/theme registry".to_string());
    }
    if matches!(entry.kind, 48 | 112 | 108)
        && entry.waypoint_index.is_none_or(|waypoint| waypoint > 701)
    {
        return Err(format!(
            "Action {} has an invalid native waypoint index",
            entry.kind
        ));
    }
    if entry.kind == 48 {
        let selector = entry
            .params
            .get(1)
            .map_or(0, |value| crate::rules::ini_value::atoi_lenient(value));
        if !(0..=4).contains(&selector) {
            return Err(format!("Action 48 has invalid speed selector {selector}"));
        }
    }
    let trigger_type_target = matches!(entry.kind, 22 | 53 | 54)
        .then(|| entry.params.get(1).map(|value| value.trim()))
        .flatten()
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("<none>"))
        .map(str::to_ascii_uppercase);
    Ok(TypedActionDefinition {
        entry,
        materialized_operand,
        trigger_type_target,
    })
}

fn compile_chain(
    head: Option<&str>,
    trigger_types: &[TriggerTypeDefinition],
    index: &HashMap<String, usize>,
    tag_id: &str,
) -> Result<Vec<usize>, String> {
    let mut chain = Vec::new();
    let mut seen = HashSet::new();
    let mut current = head.map(str::to_ascii_uppercase);
    while let Some(id) = current {
        if !seen.insert(id.clone()) {
            return Err(format!("TagType {tag_id} has a cyclic TriggerType chain at {id}"));
        }
        let trigger_index = index
            .get(&id)
            .copied()
            .ok_or_else(|| format!("TagType {tag_id} references missing TriggerType {id}"))?;
        chain.push(trigger_index);
        current = trigger_types[trigger_index]
            .linked_trigger_type_id
            .clone();
    }
    Ok(chain)
}

fn validate_force_graph(trigger_types: &[TriggerTypeDefinition]) -> Result<(), String> {
    let mut edges: HashMap<&str, Vec<&str>> = HashMap::new();
    for trigger in trigger_types {
        for action in &trigger.actions {
            if action.entry.kind == 22
                && let Some(target) = action.trigger_type_target.as_deref()
            {
                edges.entry(&trigger.id).or_default().push(target);
            }
        }
    }
    fn visit<'a>(
        id: &'a str,
        edges: &HashMap<&'a str, Vec<&'a str>>,
        active: &mut HashSet<&'a str>,
        done: &mut HashSet<&'a str>,
    ) -> bool {
        if done.contains(id) {
            return false;
        }
        if !active.insert(id) {
            return true;
        }
        let cycle = edges
            .get(id)
            .is_some_and(|targets| targets.iter().any(|target| visit(target, edges, active, done)));
        active.remove(id);
        done.insert(id);
        cycle
    }
    let mut active = HashSet::new();
    let mut done = HashSet::new();
    for id in edges.keys().copied() {
        if visit(id, &edges, &mut active, &mut done) {
            return Err(format!("Action 22 Force graph contains a cycle at {id}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{actions, events, tags, triggers};

    #[test]
    fn source_order_chain_event_reverse_and_action_append_are_independent() {
        let ini = IniFile::from_str(
            "[Triggers]\nZ=Neutral,A,Zed,1,1,1,1,99\nA=Neutral,<none>,Alpha,1,1,1,1,2\n\
             [Tags]\nTAG_Z=2,Z tag,Z\nTAG_A=0,A tag,A\n\
             [Events]\nZ=2,47,0,1,27,0,3\nA=1,47,0,2\n\
             [Actions]\nZ=2,29,0,3,0,0,0,0,A,56,0,4,0,0,0,0,B\n",
        );
        let tags = tags::parse_tags(&ini);
        let triggers = triggers::parse_triggers(&ini);
        let events = events::parse_events(&ini);
        let actions = actions::parse_actions(&ini);
        let program = TriggerProgram::compile(&ini, &tags, &triggers, &events, &actions)
            .expect("typed ordered program");

        assert_eq!(
            program
                .trigger_types
                .iter()
                .map(|trigger| trigger.id.as_str())
                .collect::<Vec<_>>(),
            vec!["Z", "A"]
        );
        assert_eq!(
            program
                .tag_types
                .iter()
                .map(|tag| tag.id.as_str())
                .collect::<Vec<_>>(),
            vec!["TAG_Z", "TAG_A"]
        );
        assert_eq!(program.tag_types[0].repeat_mode, 2);
        assert_eq!(program.tag_types[0].trigger_type_chain, vec![0, 1]);
        assert_eq!(program.tag_types[0].category_bits, 0x10);
        assert_eq!(
            program.trigger_types[0]
                .events
                .iter()
                .map(|event| event.kind)
                .collect::<Vec<_>>(),
            vec![27, 47]
        );
        assert_eq!(
            program.trigger_types[0]
                .actions
                .iter()
                .map(|action| action.entry.kind)
                .collect::<Vec<_>>(),
            vec![29, 56]
        );
        assert!(program.trigger_types[0].authored_enabled);
    }

    #[test]
    fn native_category_classifier_aggregates_the_complete_tag_chain() {
        let ini = IniFile::from_str(
            "[Triggers]\nA=Neutral,B,A,1,1,1,1,0\nB=Neutral,<none>,B,1,1,1,1,0\n\
             [Tags]\nG=0,G,A\n\
             [Events]\nA=1,24,0,0\nB=2,13,0,0,3,0,0\n\
             [Actions]\nB=1,14,0,0,0,0,0,0,A\n",
        );
        let program = TriggerProgram::compile(
            &ini,
            &tags::parse_tags(&ini),
            &triggers::parse_triggers(&ini),
            &events::parse_events(&ini),
            &actions::parse_actions(&ini),
        )
        .expect("category program");
        assert_eq!(program.tag_types[0].category_bits, 0x1F);
    }

    #[test]
    fn typed_compile_rejects_invalid_runtime_domains() {
        for body in [
            "[Triggers]\nT=Neutral,<none>,T,1,1,1,1,0\n[Tags]\nG=0,G,T\n[Events]\nT=1,27,0,50\n",
            "[Triggers]\nT=Neutral,<none>,T,1,1,1,1,0\n[Tags]\nG=0,G,T\n[Actions]\nT=1,108,6,SOUND,0,0,0,0,A\n",
            "[Triggers]\nT=Neutral,<none>,T,1,1,1,1,0\n[Tags]\nG=0,G,T\n[Actions]\nT=1,48,0,5,0,0,0,0,A\n",
        ] {
            let ini = IniFile::from_str(body);
            assert!(TriggerProgram::compile(
                &ini,
                &tags::parse_tags(&ini),
                &triggers::parse_triggers(&ini),
                &events::parse_events(&ini),
                &actions::parse_actions(&ini),
            )
            .is_err());
        }
    }

    #[test]
    fn trigger_field_seven_never_owns_repeat_and_nonzero_flags_are_true() {
        let ini = IniFile::from_str(
            "[Triggers]\nT=Neutral,<none>,T,2,-3,0,9,2\n[Tags]\nG=1,G,T\n",
        );
        let tags = tags::parse_tags(&ini);
        let triggers = triggers::parse_triggers(&ini);
        assert_eq!(tags["G"].repeat_mode, 1);
        assert!(triggers["T"].enabled);
        assert_eq!(
            triggers["T"].difficulty,
            TriggerDifficulty {
                easy: true,
                medium: false,
                hard: true,
            }
        );
    }
}
