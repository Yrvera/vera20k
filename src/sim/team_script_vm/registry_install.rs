//! Scenario-boundary resolution for ordered Team AI definitions.

use std::collections::BTreeMap;

use crate::rules::ruleset::RuleSet;
use crate::rules::team_ai_ini::{AiTriggerOwnerIni, TeamAiDefinitionSource, TeamAiIniRegistry};
use crate::sim::intern::{InternedId, StringInterner};

use super::{
    TeamAiInstallDiagnostic, TeamAiTriggerDefinition, TeamAiTriggerOwner, TeamScriptAction,
    TeamScriptDefinition, TeamScriptVm, TeamTaskForceDefinition, TeamTaskForceEntry,
    TeamTypeDefinition, TeamTypeIniMetadata,
};

impl TeamScriptVm {
    /// Resolve one fixed-AIMD + scenario registry load into deterministic sim
    /// identities. This installs definitions only: no Team, House timer, RNG,
    /// entity, or command state changes here.
    pub(crate) fn from_ini_registry(
        registry: &TeamAiIniRegistry,
        interner: &mut StringInterner,
        rules: &RuleSet,
    ) -> (Self, Vec<TeamAiInstallDiagnostic>) {
        // Validate the immutable fixed pass independently. Final records may
        // have been replaced or partially overlaid by the map, which must not
        // erase a required fixed-AIMD resolution refusal.
        let fixed_registry = registry.fixed_resolution_view();
        let mut fixed_interner = interner.clone();
        let (_, mut diagnostics) =
            Self::resolve_ini_registry(&fixed_registry, &mut fixed_interner, rules);
        let (vm, final_diagnostics) = Self::resolve_ini_registry(registry, interner, rules);
        for diagnostic in final_diagnostics {
            if !diagnostics
                .iter()
                .any(|fixed| same_install_issue(fixed, &diagnostic))
            {
                diagnostics.push(diagnostic);
            }
        }
        (vm, diagnostics)
    }

    fn resolve_ini_registry(
        registry: &TeamAiIniRegistry,
        interner: &mut StringInterner,
        rules: &RuleSet,
    ) -> (Self, Vec<TeamAiInstallDiagnostic>) {
        let mut vm = Self::default();
        let mut diagnostics = Vec::new();

        // Native registry passes establish identities before runtime selection.
        // Preserve their source order independently from keyed lookup order.
        for id in registry
            .team_types
            .iter()
            .map(|entry| entry.id.as_str())
            .chain(registry.scripts.iter().map(|entry| entry.id.as_str()))
            .chain(registry.task_forces.iter().map(|entry| entry.id.as_str()))
            .chain(registry.ai_triggers.iter().map(|entry| entry.id.as_str()))
        {
            interner.intern(id);
        }

        for script in &registry.scripts {
            let id = interner.intern(&script.id);
            vm.register_script(TeamScriptDefinition {
                id,
                actions: script
                    .actions
                    .iter()
                    .map(|action| TeamScriptAction {
                        action_id: action.action_id,
                        argument: action.argument,
                    })
                    .collect(),
                source: script.source,
            });
        }

        for task_force in &registry.task_forces {
            let id = interner.intern(&task_force.id);
            let entries = task_force
                .entries
                .iter()
                .filter_map(|entry| {
                    if rules.object(&entry.member_type).is_none() {
                        diagnostics.push(TeamAiInstallDiagnostic::UnknownTaskForceMember {
                            task_force_id: task_force.id.clone(),
                            member_type: entry.member_type.clone(),
                            source: task_force.source,
                        });
                        return None;
                    }
                    Some(TeamTaskForceEntry {
                        member_type: interner.intern(&entry.member_type),
                        count: entry.count,
                    })
                })
                .collect();
            vm.register_task_force(TeamTaskForceDefinition {
                id,
                group: task_force.group,
                entries,
                source: task_force.source,
            });
        }

        for team_type in &registry.team_types {
            let id = interner.intern(&team_type.id);
            let script_name = team_type.get("Script").unwrap_or("<none>");
            let (script_id, script_was_exact) = definition_id(
                script_name,
                &vm.scripts,
                &vm.script_order,
                interner,
            );
            if !script_was_exact {
                diagnostics.push(TeamAiInstallDiagnostic::MissingTeamTypeScript {
                    team_type_id: team_type.id.clone(),
                    script_id: script_name.to_string(),
                    source: team_type.source,
                });
            }
            let script_id = script_id.unwrap_or(InternedId::from_index(0));
            let task_force_name = team_type.get("TaskForce").unwrap_or("<none>");
            let (task_force_id, task_force_was_exact) = definition_id(
                task_force_name,
                &vm.task_forces,
                &vm.task_force_order,
                interner,
            );
            if !task_force_was_exact {
                diagnostics.push(TeamAiInstallDiagnostic::MissingTeamTypeTaskForce {
                    team_type_id: team_type.id.clone(),
                    task_force_id: task_force_name.to_string(),
                    source: team_type.source,
                });
            }
            let task_force_id = task_force_id.unwrap_or(InternedId::from_index(0));

            vm.register_team_type(TeamTypeDefinition {
                id,
                script_id,
                task_force_id,
                priority: team_type.read_int("Priority", 7),
                is_base_defense: team_type.read_bool("IsBaseDefense", false),
            });
            vm.team_type_ini.insert(
                id,
                TeamTypeIniMetadata {
                    max_teams: team_type.read_int("Max", -1),
                    autocreate: team_type.read_bool("Autocreate", false),
                    are_team_members_recruitable: team_type
                        .read_bool("AreTeamMembersRecruitable", true),
                    raw_fields: team_type.fields.clone(),
                    source: team_type.source,
                },
            );
        }

        for trigger in &registry.ai_triggers {
            let id = interner.intern(&trigger.id);
            let primary_team_type = resolve_trigger_team_type(
                &trigger.id,
                trigger.primary_team_type.as_deref(),
                &vm.team_types,
                interner,
                &mut diagnostics,
                trigger.source,
            );
            let secondary_team_type = resolve_trigger_team_type(
                &trigger.id,
                trigger.secondary_team_type.as_deref(),
                &vm.team_types,
                interner,
                &mut diagnostics,
                trigger.source,
            );
            let owner = resolve_trigger_owner(trigger, rules, &mut diagnostics);
            let object_type = resolve_trigger_object(trigger, rules, interner, &mut diagnostics);
            let primary_total = trigger_task_force_total(&vm, primary_team_type);
            let secondary_total = trigger_task_force_total(&vm, secondary_team_type);
            let threshold = [primary_total, secondary_total]
                .into_iter()
                .flatten()
                .fold(trigger.authored_threshold, i32::max);
            vm.register_ai_trigger(TeamAiTriggerDefinition {
                id,
                tokens: trigger.tokens.clone(),
                display_name: trigger.display_name.clone(),
                enabled: trigger.enabled,
                primary_team_type,
                owner,
                authored_threshold: trigger.authored_threshold,
                threshold,
                condition: trigger.condition,
                object_type,
                comparison_mask: trigger.comparison_mask,
                weights: trigger.weights,
                storage_flag_d0: trigger.storage_flag_d0,
                storage_i32_ac: trigger.storage_i32_ac,
                storage_flag_d1: trigger.storage_flag_d1,
                secondary_team_type,
                difficulty_enabled: trigger.difficulty_enabled,
                source: trigger.source,
            });
        }

        (vm, diagnostics)
    }

    pub(crate) fn registry_counts(&self) -> (usize, usize, usize, usize) {
        (
            self.task_forces.len(),
            self.scripts.len(),
            self.team_types.len(),
            self.ai_triggers.len(),
        )
    }

    pub(crate) fn team_type_order(&self) -> &[InternedId] {
        &self.team_type_order
    }

    pub(crate) fn script_order(&self) -> &[InternedId] {
        &self.script_order
    }

    pub(crate) fn script(&self, id: InternedId) -> Option<&TeamScriptDefinition> {
        self.scripts.get(&id)
    }

    pub(crate) fn task_force_order(&self) -> &[InternedId] {
        &self.task_force_order
    }

    pub(crate) fn task_force(&self, id: InternedId) -> Option<&TeamTaskForceDefinition> {
        self.task_forces.get(&id)
    }

    pub(crate) fn ai_trigger_order(&self) -> &[InternedId] {
        &self.ai_trigger_order
    }

    pub(crate) fn ai_trigger(&self, id: InternedId) -> Option<&TeamAiTriggerDefinition> {
        self.ai_triggers.get(&id)
    }

    pub(crate) fn team_type_ini(&self, id: InternedId) -> Option<&TeamTypeIniMetadata> {
        self.team_type_ini.get(&id)
    }
}

fn definition_id<T>(
    requested: &str,
    definitions: &BTreeMap<InternedId, T>,
    order: &[InternedId],
    interner: &StringInterner,
) -> (Option<InternedId>, bool) {
    let exact = interner
        .get(requested)
        .filter(|id| definitions.contains_key(id));
    (exact.or_else(|| order.first().copied()), exact.is_some())
}

fn resolve_trigger_team_type(
    trigger_id: &str,
    requested: Option<&str>,
    definitions: &BTreeMap<InternedId, TeamTypeDefinition>,
    interner: &StringInterner,
    diagnostics: &mut Vec<TeamAiInstallDiagnostic>,
    source: TeamAiDefinitionSource,
) -> Option<InternedId> {
    let requested = requested?;
    let resolved = interner
        .get(requested)
        .filter(|id| definitions.contains_key(id));
    if resolved.is_none() {
        diagnostics.push(TeamAiInstallDiagnostic::MissingAiTriggerTeamType {
            trigger_id: trigger_id.to_string(),
            team_type_id: requested.to_string(),
            source,
        });
    }
    resolved
}

fn resolve_trigger_owner(
    trigger: &crate::rules::team_ai_ini::AiTriggerTypeIni,
    rules: &RuleSet,
    diagnostics: &mut Vec<TeamAiInstallDiagnostic>,
) -> Option<TeamAiTriggerOwner> {
    match &trigger.owner {
        AiTriggerOwnerIni::All => Some(TeamAiTriggerOwner::All),
        AiTriggerOwnerIni::Country(owner) => rules
            .country_index(owner)
            .map(TeamAiTriggerOwner::Country)
            .or_else(|| {
                diagnostics.push(TeamAiInstallDiagnostic::UnknownAiTriggerOwner {
                    trigger_id: trigger.id.clone(),
                    owner: owner.clone(),
                    source: trigger.source,
                });
                None
            }),
    }
}

fn resolve_trigger_object(
    trigger: &crate::rules::team_ai_ini::AiTriggerTypeIni,
    rules: &RuleSet,
    interner: &mut StringInterner,
    diagnostics: &mut Vec<TeamAiInstallDiagnostic>,
) -> Option<InternedId> {
    let object_type = trigger.object_type.as_deref()?;
    if rules.object(object_type).is_some() {
        return Some(interner.intern(object_type));
    }
    diagnostics.push(TeamAiInstallDiagnostic::UnknownAiTriggerObject {
        trigger_id: trigger.id.clone(),
        object_type: object_type.to_string(),
        source: trigger.source,
    });
    None
}

fn trigger_task_force_total(vm: &TeamScriptVm, team_type_id: Option<InternedId>) -> Option<i32> {
    team_type_id
        .and_then(|id| vm.team_types.get(&id))
        .and_then(|team_type| vm.task_forces.get(&team_type.task_force_id))
        .map(|task_force| {
            task_force
                .entries
                .iter()
                .fold(0_i32, |total, entry| total.wrapping_add(entry.count))
        })
}

fn same_install_issue(
    left: &TeamAiInstallDiagnostic,
    right: &TeamAiInstallDiagnostic,
) -> bool {
    match (left, right) {
        (
            TeamAiInstallDiagnostic::UnknownTaskForceMember {
                task_force_id: left_task_force,
                member_type: left_member,
                ..
            },
            TeamAiInstallDiagnostic::UnknownTaskForceMember {
                task_force_id: right_task_force,
                member_type: right_member,
                ..
            },
        ) => left_task_force == right_task_force && left_member == right_member,
        (
            TeamAiInstallDiagnostic::MissingTeamTypeScript {
                team_type_id: left_team,
                script_id: left_script,
                ..
            },
            TeamAiInstallDiagnostic::MissingTeamTypeScript {
                team_type_id: right_team,
                script_id: right_script,
                ..
            },
        ) => left_team == right_team && left_script == right_script,
        (
            TeamAiInstallDiagnostic::MissingTeamTypeTaskForce {
                team_type_id: left_team,
                task_force_id: left_task_force,
                ..
            },
            TeamAiInstallDiagnostic::MissingTeamTypeTaskForce {
                team_type_id: right_team,
                task_force_id: right_task_force,
                ..
            },
        ) => left_team == right_team && left_task_force == right_task_force,
        (
            TeamAiInstallDiagnostic::MissingAiTriggerTeamType {
                trigger_id: left_trigger,
                team_type_id: left_team,
                ..
            },
            TeamAiInstallDiagnostic::MissingAiTriggerTeamType {
                trigger_id: right_trigger,
                team_type_id: right_team,
                ..
            },
        ) => left_trigger == right_trigger && left_team == right_team,
        (
            TeamAiInstallDiagnostic::UnknownAiTriggerOwner {
                trigger_id: left_trigger,
                owner: left_owner,
                ..
            },
            TeamAiInstallDiagnostic::UnknownAiTriggerOwner {
                trigger_id: right_trigger,
                owner: right_owner,
                ..
            },
        ) => left_trigger == right_trigger && left_owner == right_owner,
        (
            TeamAiInstallDiagnostic::UnknownAiTriggerObject {
                trigger_id: left_trigger,
                object_type: left_object,
                ..
            },
            TeamAiInstallDiagnostic::UnknownAiTriggerObject {
                trigger_id: right_trigger,
                object_type: right_object,
                ..
            },
        ) => left_trigger == right_trigger && left_object == right_object,
        _ => false,
    }
}
