//! Scenario-boundary resolution for ordered Team AI definitions.

use std::collections::BTreeMap;

use crate::rules::ruleset::RuleSet;
use crate::rules::team_ai_ini::TeamAiIniRegistry;
use crate::sim::intern::{InternedId, StringInterner};

use super::{
    TeamAiInstallDiagnostic, TeamAiTriggerDefinition, TeamScriptAction, TeamScriptDefinition,
    TeamScriptVm, TeamTaskForceDefinition, TeamTaskForceEntry, TeamTypeDefinition,
    TeamTypeIniMetadata,
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
                &trigger.tokens[1],
                &vm.team_types,
                interner,
                &mut diagnostics,
                trigger.source,
            );
            let secondary_team_type = resolve_trigger_team_type(
                &trigger.id,
                &trigger.tokens[14],
                &vm.team_types,
                interner,
                &mut diagnostics,
                trigger.source,
            );
            vm.ai_trigger_order.push(id);
            vm.ai_triggers.insert(
                id,
                TeamAiTriggerDefinition {
                    id,
                    tokens: trigger.tokens.clone(),
                    enabled: trigger.enabled,
                    primary_team_type,
                    secondary_team_type,
                    source: trigger.source,
                },
            );
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
    requested: &str,
    definitions: &BTreeMap<InternedId, TeamTypeDefinition>,
    interner: &StringInterner,
    diagnostics: &mut Vec<TeamAiInstallDiagnostic>,
    source: crate::rules::team_ai_ini::TeamAiDefinitionSource,
) -> Option<InternedId> {
    if requested.is_empty() || requested.starts_with('<') {
        return None;
    }
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
