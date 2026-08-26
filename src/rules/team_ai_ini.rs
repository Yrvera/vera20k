//! Active-YR AIMD + map registry ingestion.
//!
//! `gamemd.exe` keeps these definitions outside `RulesClass`: fixed
//! `AIMD.INI` records are visited first and the scenario INI then re-reads
//! matching identities in place or appends new ones. This module preserves
//! that source order and raw payload. It does not select AI triggers, create
//! Teams, recruit members, or consume simulation RNG.

use std::collections::BTreeMap;

use crate::rules::ini_parser::{IniFile, IniSection};
use crate::rules::ini_value::{atoi_lenient, parse_read_double, parse_read_int_value};
use crate::util::native_x87::NativeF64Bits;

const SCRIPT_ACTION_CAPACITY: usize = 50;
const TASK_FORCE_CAPACITY: usize = 6;
const AI_TRIGGER_TOKEN_COUNT: usize = 18;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum TeamAiDefinitionSource {
    FixedAimd,
    Scenario,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptActionIni {
    pub action_id: i32,
    pub argument: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptTypeIni {
    pub id: String,
    pub actions: Vec<ScriptActionIni>,
    pub source: TeamAiDefinitionSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskForceEntryIni {
    pub count: i32,
    pub member_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskForceIni {
    pub id: String,
    pub group: i32,
    pub entries: Vec<TaskForceEntryIni>,
    pub source: TeamAiDefinitionSource,
}

/// One TeamType's authored fields after fixed/map current-field overlay.
///
/// Typed consumers resolve only fields backed by their own native evidence;
/// retaining the ordered raw payload prevents later AI-trigger/recruitment
/// work from having to reconstruct the loader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamTypeIni {
    pub id: String,
    pub fields: Vec<(String, String)>,
    pub source: TeamAiDefinitionSource,
}

impl TeamTypeIni {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find_map(|(candidate, value)| (candidate == key).then_some(value.as_str()))
    }

    pub fn read_int(&self, key: &str, default: i32) -> i32 {
        self.get(key)
            .and_then(parse_read_int_value)
            .unwrap_or(default)
    }

    pub fn read_bool(&self, key: &str, default: bool) -> bool {
        self.get(key).map_or(default, |raw| read_bool(raw, default))
    }

    fn overlay(&mut self, section: &IniSection, source: TeamAiDefinitionSource) {
        for key in section.keys() {
            let Some(value) = section.get(key) else {
                continue;
            };
            if let Some((_, current)) = self
                .fields
                .iter_mut()
                .find(|(candidate, _)| candidate == key)
            {
                *current = value.to_string();
            } else {
                self.fields.push((key.to_string(), value.to_string()));
            }
        }
        self.source = source;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiTriggerOwnerIni {
    All,
    Country(String),
}

/// One AITriggerType after the fixed/map replacement pass.
///
/// The typed fields are limited to storage proven in the active YR raw reader
/// at `0x0041F580`. `tokens` remains authoritative for token 12 and for the
/// still-untraced semantic names of tokens 11 through 14.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiTriggerTypeIni {
    pub id: String,
    pub tokens: [String; AI_TRIGGER_TOKEN_COUNT],
    pub display_name: String,
    pub primary_team_type: Option<String>,
    pub owner: AiTriggerOwnerIni,
    pub condition: i32,
    pub object_type: Option<String>,
    pub comparison_mask: [u8; 32],
    pub weights: [NativeF64Bits; 3],
    pub storage_flag_d0: bool,
    pub storage_i32_ac: i32,
    pub storage_flag_d1: bool,
    pub secondary_team_type: Option<String>,
    pub difficulty_enabled: [bool; 3],
    pub enabled: bool,
    pub source: TeamAiDefinitionSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TeamAiIniDiagnostic {
    MissingDefinitionSection {
        registry: &'static str,
        id: String,
        source: TeamAiDefinitionSource,
    },
    MalformedScriptAction {
        script_id: String,
        key: usize,
        value: String,
        source: TeamAiDefinitionSource,
    },
    MalformedTaskForceEntry {
        task_force_id: String,
        key: usize,
        value: String,
        source: TeamAiDefinitionSource,
    },
    MalformedAiTrigger {
        trigger_id: String,
        token_count: usize,
        source: TeamAiDefinitionSource,
    },
    MalformedAiTriggerComparison {
        trigger_id: String,
        value: String,
        source: TeamAiDefinitionSource,
    },
    UnknownAiTriggerEnable {
        trigger_id: String,
    },
}

impl TeamAiIniDiagnostic {
    fn source(&self) -> TeamAiDefinitionSource {
        match self {
            Self::MissingDefinitionSection { source, .. }
            | Self::MalformedScriptAction { source, .. }
            | Self::MalformedTaskForceEntry { source, .. }
            | Self::MalformedAiTrigger { source, .. }
            | Self::MalformedAiTriggerComparison { source, .. } => *source,
            Self::UnknownAiTriggerEnable { .. } => TeamAiDefinitionSource::Scenario,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TeamAiRegistryCounts {
    pub team_types: usize,
    pub scripts: usize,
    pub task_forces: usize,
    pub ai_triggers: usize,
}

/// Immutable definitions captured immediately after the fixed AIMD pass for
/// each registry. Map overlays may replace the live record, but cannot erase a
/// fixed-source resolution obligation that production loading must validate.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct FixedTeamAiDefinitions {
    team_types: Vec<TeamTypeIni>,
    scripts: Vec<ScriptTypeIni>,
    task_forces: Vec<TaskForceIni>,
    ai_triggers: Vec<AiTriggerTypeIni>,
}

/// Ordered unresolved AI definitions from fixed AIMD and the scenario INI.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TeamAiIniRegistry {
    pub team_types: Vec<TeamTypeIni>,
    pub scripts: Vec<ScriptTypeIni>,
    pub task_forces: Vec<TaskForceIni>,
    pub ai_triggers: Vec<AiTriggerTypeIni>,
    pub diagnostics: Vec<TeamAiIniDiagnostic>,
    pub fixed_counts: TeamAiRegistryCounts,
    fixed_definitions: FixedTeamAiDefinitions,
    team_type_read_transactions: Vec<TeamTypeIni>,
    fixed_team_type_read_count: usize,
    game_mode_nonzero: bool,
}

impl TeamAiIniRegistry {
    /// Reproduce the active-YR per-registry fixed/map pass sequence.
    ///
    /// Retail provenance: `ScenarioClass::Full_Init @ 0x00686B20`, exact
    /// calls `0x0068797A..0x006879E3`.
    pub fn from_sources(
        fixed_aimd: &IniFile,
        scenario: &IniFile,
        game_mode_nonzero: bool,
    ) -> Self {
        let mut registry = Self {
            game_mode_nonzero,
            ..Self::default()
        };

        registry.read_team_types(fixed_aimd, TeamAiDefinitionSource::FixedAimd);
        registry.fixed_counts.team_types = registry.team_types.len();
        registry.fixed_definitions.team_types = registry.team_types.clone();
        registry.fixed_team_type_read_count = registry.team_type_read_transactions.len();
        registry.read_team_types(scenario, TeamAiDefinitionSource::Scenario);

        registry.read_scripts(fixed_aimd, TeamAiDefinitionSource::FixedAimd);
        registry.fixed_counts.scripts = registry.scripts.len();
        registry.fixed_definitions.scripts = registry.scripts.clone();
        registry.read_scripts(scenario, TeamAiDefinitionSource::Scenario);

        registry.read_task_forces(fixed_aimd, TeamAiDefinitionSource::FixedAimd);
        registry.fixed_counts.task_forces = registry.task_forces.len();
        registry.fixed_definitions.task_forces = registry.task_forces.clone();
        registry.read_task_forces(scenario, TeamAiDefinitionSource::Scenario);

        registry.read_ai_triggers(fixed_aimd, TeamAiDefinitionSource::FixedAimd);
        registry.fixed_counts.ai_triggers = registry.ai_triggers.len();
        registry.fixed_definitions.ai_triggers = registry.ai_triggers.clone();
        registry.read_ai_triggers(scenario, TeamAiDefinitionSource::Scenario);
        registry.read_ai_trigger_enables(scenario, game_mode_nonzero);

        registry
    }

    /// Whether the required fixed AIMD source produced all four non-empty
    /// registries without a refused definition. Scenario diagnostics do not
    /// invalidate the fixed source because map additions are optional overlays.
    pub fn fixed_source_is_complete(&self) -> bool {
        self.fixed_counts.team_types > 0
            && self.fixed_counts.scripts > 0
            && self.fixed_counts.task_forces > 0
            && self.fixed_counts.ai_triggers > 0
            && !self
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.source() == TeamAiDefinitionSource::FixedAimd)
    }

    pub(crate) fn fixed_resolution_view(&self) -> Self {
        Self {
            team_types: self.fixed_definitions.team_types.clone(),
            scripts: self.fixed_definitions.scripts.clone(),
            task_forces: self.fixed_definitions.task_forces.clone(),
            ai_triggers: self.fixed_definitions.ai_triggers.clone(),
            diagnostics: Vec::new(),
            fixed_counts: self.fixed_counts,
            fixed_definitions: self.fixed_definitions.clone(),
            team_type_read_transactions: self.team_type_read_transactions
                [..self.fixed_team_type_read_count]
                .to_vec(),
            fixed_team_type_read_count: self.fixed_team_type_read_count,
            game_mode_nonzero: self.game_mode_nonzero,
        }
    }

    pub(crate) fn game_mode_nonzero(&self) -> bool {
        self.game_mode_nonzero
    }

    /// TeamTypes are read from fixed AIMD and then from the map before either
    /// referenced registry is populated. Keep every successful read in that
    /// source's encounter order and with the current-field state at that read:
    /// the merged identity vector cannot reconstruct a map pass that lists a
    /// new identity before an override of an existing one.
    pub(crate) fn team_type_read_sequence(&self) -> impl Iterator<Item = &TeamTypeIni> {
        self.team_type_read_transactions.iter()
    }

    fn read_team_types(&mut self, ini: &IniFile, source: TeamAiDefinitionSource) {
        let Some(list) = ini.section("TeamTypes") else {
            return;
        };
        let mut index = identity_index(self.team_types.iter().map(|entry| entry.id.as_str()));
        for id in list.get_values().into_iter().filter(|id| valid_identity(id)) {
            let Some(section) = ini.section(id) else {
                self.diagnostics
                    .push(TeamAiIniDiagnostic::MissingDefinitionSection {
                        registry: "TeamTypes",
                        id: id.to_string(),
                        source,
                    });
                continue;
            };
            let identity = canonical_identity(id);
            let position = if let Some(position) = index.get(&identity).copied() {
                self.team_types[position].overlay(section, source);
                position
            } else {
                let mut definition = TeamTypeIni {
                    id: id.to_string(),
                    fields: Vec::new(),
                    source,
                };
                definition.overlay(section, source);
                let position = self.team_types.len();
                index.insert(identity, position);
                self.team_types.push(definition);
                position
            };
            self.team_type_read_transactions
                .push(self.team_types[position].clone());
        }
    }

    fn read_scripts(&mut self, ini: &IniFile, source: TeamAiDefinitionSource) {
        let Some(list) = ini.section("ScriptTypes") else {
            return;
        };
        let mut index = identity_index(self.scripts.iter().map(|entry| entry.id.as_str()));
        for id in list.get_values().into_iter().filter(|id| valid_identity(id)) {
            let Some(section) = ini.section(id) else {
                self.diagnostics
                    .push(TeamAiIniDiagnostic::MissingDefinitionSection {
                        registry: "ScriptTypes",
                        id: id.to_string(),
                        source,
                    });
                continue;
            };
            let mut actions = Vec::new();
            for key in 0..SCRIPT_ACTION_CAPACITY {
                let Some(value) = section.get(&key.to_string()) else {
                    continue;
                };
                let Some((action, argument)) = value.split_once(',') else {
                    self.diagnostics
                        .push(TeamAiIniDiagnostic::MalformedScriptAction {
                            script_id: id.to_string(),
                            key,
                            value: value.to_string(),
                            source,
                        });
                    continue;
                };
                actions.push(ScriptActionIni {
                    action_id: atoi_lenient(action.trim()),
                    argument: atoi_lenient(argument.trim()),
                });
            }
            let definition = ScriptTypeIni {
                id: id.to_string(),
                actions,
                source,
            };
            upsert_ordered(&mut self.scripts, &mut index, definition, |entry| &entry.id);
        }
    }

    fn read_task_forces(&mut self, ini: &IniFile, source: TeamAiDefinitionSource) {
        let Some(list) = ini.section("TaskForces") else {
            return;
        };
        let mut index = identity_index(self.task_forces.iter().map(|entry| entry.id.as_str()));
        for id in list.get_values().into_iter().filter(|id| valid_identity(id)) {
            let Some(section) = ini.section(id) else {
                self.diagnostics
                    .push(TeamAiIniDiagnostic::MissingDefinitionSection {
                        registry: "TaskForces",
                        id: id.to_string(),
                        source,
                    });
                continue;
            };
            let mut entries = Vec::new();
            for key in 0..TASK_FORCE_CAPACITY {
                let Some(value) = section.get(&key.to_string()) else {
                    continue;
                };
                let Some((count, member_type)) = value.split_once(',') else {
                    self.diagnostics
                        .push(TeamAiIniDiagnostic::MalformedTaskForceEntry {
                            task_force_id: id.to_string(),
                            key,
                            value: value.to_string(),
                            source,
                        });
                    continue;
                };
                let member_type = member_type.trim();
                if member_type.is_empty() || !valid_identity(member_type) {
                    self.diagnostics
                        .push(TeamAiIniDiagnostic::MalformedTaskForceEntry {
                            task_force_id: id.to_string(),
                            key,
                            value: value.to_string(),
                            source,
                        });
                    continue;
                }
                entries.push(TaskForceEntryIni {
                    count: atoi_lenient(count.trim()),
                    member_type: member_type.to_string(),
                });
            }
            let definition = TaskForceIni {
                id: id.to_string(),
                group: section.read_int("Group", -1),
                entries,
                source,
            };
            upsert_ordered(
                &mut self.task_forces,
                &mut index,
                definition,
                |entry| &entry.id,
            );
        }
    }

    fn read_ai_triggers(&mut self, ini: &IniFile, source: TeamAiDefinitionSource) {
        let Some(section) = ini.section("AITriggerTypes") else {
            return;
        };
        let mut index = identity_index(self.ai_triggers.iter().map(|entry| entry.id.as_str()));
        for id in section.keys().filter(|id| valid_identity(id)) {
            let Some(value) = section.get(id) else {
                continue;
            };
            let tokens: Vec<String> = value
                .split(',')
                .map(|token| token.trim().to_string())
                .collect();
            let token_count = tokens.len();
            let Ok(tokens) = <Vec<String> as TryInto<[String; AI_TRIGGER_TOKEN_COUNT]>>::try_into(
                tokens,
            ) else {
                self.diagnostics
                    .push(TeamAiIniDiagnostic::MalformedAiTrigger {
                        trigger_id: id.to_string(),
                        token_count,
                        source,
                    });
                continue;
            };
            let Some(comparison_mask) = parse_ai_trigger_comparison(&tokens[6]) else {
                self.diagnostics
                    .push(TeamAiIniDiagnostic::MalformedAiTriggerComparison {
                        trigger_id: id.to_string(),
                        value: tokens[6].clone(),
                        source,
                    });
                continue;
            };
            let identity = canonical_identity(id);
            let enabled = index
                .get(&identity)
                .map_or(source == TeamAiDefinitionSource::FixedAimd, |position| {
                    self.ai_triggers[*position].enabled
                });
            let definition = AiTriggerTypeIni {
                id: id.to_string(),
                display_name: tokens[0].clone(),
                primary_team_type: optional_reference(&tokens[1]),
                owner: if tokens[2].eq_ignore_ascii_case("<all>") {
                    AiTriggerOwnerIni::All
                } else {
                    AiTriggerOwnerIni::Country(tokens[2].clone())
                },
                condition: atoi_lenient(&tokens[4]),
                object_type: optional_reference(&tokens[5]),
                comparison_mask,
                weights: [
                    NativeF64Bits::from_bits(parse_read_double(&tokens[7]).to_bits()),
                    NativeF64Bits::from_bits(parse_read_double(&tokens[8]).to_bits()),
                    NativeF64Bits::from_bits(parse_read_double(&tokens[9]).to_bits()),
                ],
                storage_flag_d0: read_bool(&tokens[10], false),
                storage_i32_ac: atoi_lenient(&tokens[12]),
                storage_flag_d1: read_bool(&tokens[13], false),
                secondary_team_type: optional_reference(&tokens[14]),
                difficulty_enabled: [
                    read_bool(&tokens[15], true),
                    read_bool(&tokens[16], true),
                    read_bool(&tokens[17], true),
                ],
                tokens,
                enabled,
                source,
            };
            if let Some(position) = index.get(&identity).copied() {
                self.ai_triggers[position] = definition;
            } else {
                index.insert(identity, self.ai_triggers.len());
                self.ai_triggers.push(definition);
            }
        }
    }

    fn read_ai_trigger_enables(&mut self, scenario: &IniFile, game_mode_nonzero: bool) {
        let Some(section) = scenario.section("AITriggerTypesEnable") else {
            return;
        };
        let index = identity_index(self.ai_triggers.iter().map(|entry| entry.id.as_str()));
        for id in section.keys() {
            let Some(value) = section.get(id) else {
                continue;
            };
            let Some(position) = index.get(&canonical_identity(id)).copied() else {
                self.diagnostics
                    .push(TeamAiIniDiagnostic::UnknownAiTriggerEnable {
                        trigger_id: id.to_string(),
                    });
                continue;
            };
            // `FUN_0041F2E0`: a false authored value disables only when
            // `g_GameMode == 0`; every listed key is enabled in skirmish/MP.
            self.ai_triggers[position].enabled = read_bool(value, false) || game_mode_nonzero;
        }
    }
}

fn valid_identity(id: &str) -> bool {
    let id = id.trim();
    !id.is_empty() && !id.starts_with('<')
}

fn optional_reference(raw: &str) -> Option<String> {
    valid_identity(raw).then(|| raw.trim().to_string())
}

fn parse_ai_trigger_comparison(raw: &str) -> Option<[u8; 32]> {
    let bytes = raw.trim().as_bytes();
    if bytes.len() != 64 {
        return None;
    }

    let mut decoded = [0_u8; 32];
    for (index, slot) in decoded.iter_mut().enumerate() {
        let high = hex_nibble(bytes[index * 2])?;
        let low = hex_nibble(bytes[index * 2 + 1])?;
        *slot = (high << 4) | low;
    }
    Some(decoded)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn canonical_identity(id: &str) -> String {
    id.trim().to_ascii_uppercase()
}

fn identity_index<'a>(ids: impl Iterator<Item = &'a str>) -> BTreeMap<String, usize> {
    ids.enumerate()
        .map(|(position, id)| (canonical_identity(id), position))
        .collect()
}

fn upsert_ordered<T>(
    entries: &mut Vec<T>,
    index: &mut BTreeMap<String, usize>,
    definition: T,
    id: impl Fn(&T) -> &str,
) {
    let identity = canonical_identity(id(&definition));
    if let Some(position) = index.get(&identity).copied() {
        entries[position] = definition;
    } else {
        index.insert(identity, entries.len());
        entries.push(definition);
    }
}

fn read_bool(raw: &str, default: bool) -> bool {
    match raw
        .trim()
        .as_bytes()
        .first()
        .map(|byte| byte.to_ascii_uppercase())
    {
        Some(b'1') | Some(b'T') | Some(b'Y') => true,
        Some(b'0') | Some(b'F') | Some(b'N') => false,
        _ => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eighteen_tokens(name: &str, primary: &str) -> String {
        let comparison = "00".repeat(32);
        [
            name,
            primary,
            "<all>",
            "2",
            "4",
            "<none>",
            comparison.as_str(),
            "40.0",
            "10.0",
            "40.0",
            "1",
            "0",
            "1",
            "0",
            "<none>",
            "1",
            "1",
            "1",
        ]
        .join(",")
    }

    fn complete_fixed_aimd() -> IniFile {
        IniFile::from_str(&format!(
            "[TeamTypes]\n0=TEAM\n[TEAM]\nScript=SCRIPT\nTaskForce=FORCE\n\
             [ScriptTypes]\n0=SCRIPT\n[SCRIPT]\n0=2,0\n\
             [TaskForces]\n0=FORCE\n[FORCE]\n0=1,E1\nGroup=-1\n\
             [AITriggerTypes]\nTRIGGER={}\n",
            eighteen_tokens("Trigger", "TEAM")
        ))
    }

    #[test]
    fn complete_fixed_source_requires_all_four_nonempty_clean_registries() {
        let loaded = TeamAiIniRegistry::from_sources(
            &complete_fixed_aimd(),
            &IniFile::from_str(""),
            false,
        );

        assert_eq!(
            loaded.fixed_counts,
            TeamAiRegistryCounts {
                team_types: 1,
                scripts: 1,
                task_forces: 1,
                ai_triggers: 1,
            }
        );
        assert!(loaded.fixed_source_is_complete());
        assert!(loaded.diagnostics.is_empty());
    }

    #[test]
    fn scenario_cannot_repair_refused_fixed_definitions() {
        let fixed = IniFile::from_str(&format!(
            "[TeamTypes]\n0=GOOD_TEAM\n1=MISSING_TEAM\n\
             [GOOD_TEAM]\nScript=GOOD_SCRIPT\nTaskForce=GOOD_FORCE\n\
             [ScriptTypes]\n0=GOOD_SCRIPT\n1=BAD_SCRIPT\n\
             [GOOD_SCRIPT]\n0=2,0\n[BAD_SCRIPT]\n0=missing-comma\n\
             [TaskForces]\n0=GOOD_FORCE\n1=BAD_FORCE\n\
             [GOOD_FORCE]\n0=1,E1\n[BAD_FORCE]\n0=missing-comma\n\
             [AITriggerTypes]\nGOOD={}\nBAD=a,b,c\n",
            eighteen_tokens("Good", "GOOD_TEAM")
        ));
        let scenario = IniFile::from_str(&format!(
            "[TeamTypes]\n0=MISSING_TEAM\n\
             [MISSING_TEAM]\nScript=BAD_SCRIPT\nTaskForce=BAD_FORCE\n\
             [ScriptTypes]\n0=BAD_SCRIPT\n[BAD_SCRIPT]\n0=2,0\n\
             [TaskForces]\n0=BAD_FORCE\n[BAD_FORCE]\n0=1,E2\n\
             [AITriggerTypes]\nBAD={}\n",
            eighteen_tokens("Repaired", "MISSING_TEAM")
        ));

        let loaded = TeamAiIniRegistry::from_sources(&fixed, &scenario, false);

        assert_eq!(loaded.team_types.len(), 2);
        assert_eq!(loaded.scripts.len(), 2);
        assert_eq!(loaded.task_forces.len(), 2);
        assert_eq!(loaded.ai_triggers.len(), 2);
        assert!(!loaded.fixed_source_is_complete());
        assert_eq!(
            loaded
                .diagnostics
                .iter()
                .filter(|diagnostic| {
                    diagnostic.source() == TeamAiDefinitionSource::FixedAimd
                })
                .count(),
            4
        );
    }

    #[test]
    fn refused_scenario_definition_does_not_invalidate_clean_fixed_source() {
        let scenario = IniFile::from_str("[ScriptTypes]\n0=MISSING_MAP_SCRIPT\n");
        let loaded = TeamAiIniRegistry::from_sources(&complete_fixed_aimd(), &scenario, false);

        assert!(loaded.fixed_source_is_complete());
        assert_eq!(
            loaded.diagnostics,
            vec![TeamAiIniDiagnostic::MissingDefinitionSection {
                registry: "ScriptTypes",
                id: "MISSING_MAP_SCRIPT".to_string(),
                source: TeamAiDefinitionSource::Scenario,
            }]
        );
    }

    #[test]
    fn fixed_then_map_reuses_identity_in_place_and_appends_new_ids() {
        let fixed = IniFile::from_str(&format!(
            "[TeamTypes]\n0=FIRST\n1=SECOND\n\
             [FIRST]\nPriority=5\nAutocreate=yes\nScript=S1\nTaskForce=T1\n\
             [SECOND]\nPriority=7\nScript=S1\nTaskForce=T1\n\
             [ScriptTypes]\n0=S1\n[S1]\n0=2,0\n2=6,3\n\
             [TaskForces]\n0=T1\n[T1]\n0=1,E1\nGroup=3\n\
             [AITriggerTypes]\nA={}\n",
            eighteen_tokens("Fixed", "FIRST")
        ));
        let map = IniFile::from_str(&format!(
            "[TeamTypes]\n0=THIRD\n1=first\n\
             [THIRD]\nPriority=9\nScript=S1\nTaskForce=T1\n\
             [first]\nPriority=20\n\
             [ScriptTypes]\n0=S1\n1=S2\n[S1]\n4=49,0\n[S2]\n0=2,0\n\
             [TaskForces]\n0=T1\n1=T2\n[T1]\n0=2,E1\n6=9,IGNORED\n[T2]\n0=1,E2\n\
             [AITriggerTypes]\nA={}\nB={}\n[AITriggerTypesEnable]\nA=no\nB=yes\n",
            eighteen_tokens("Map override", "FIRST"),
            eighteen_tokens("Map append", "THIRD")
        ));

        let loaded = TeamAiIniRegistry::from_sources(&fixed, &map, false);

        assert_eq!(
            loaded
                .team_types
                .iter()
                .map(|entry| entry.id.to_ascii_uppercase())
                .collect::<Vec<_>>(),
            ["FIRST", "SECOND", "THIRD"]
        );
        assert_eq!(
            loaded
                .team_type_read_sequence()
                .map(|entry| entry.id.to_ascii_uppercase())
                .collect::<Vec<_>>(),
            ["FIRST", "SECOND", "THIRD", "FIRST"],
            "the map read pass keeps source order even when merged identity order differs"
        );
        assert_eq!(
            loaded
                .team_type_read_sequence()
                .map(|entry| entry.read_int("Priority", 7))
                .collect::<Vec<_>>(),
            [5, 7, 9, 20],
            "each replay transaction retains the current-field state at that read"
        );
        assert_eq!(loaded.team_types[0].read_int("Priority", 7), 20);
        assert!(loaded.team_types[0].read_bool("Autocreate", false));
        assert_eq!(loaded.scripts[0].actions, vec![ScriptActionIni { action_id: 49, argument: 0 }]);
        assert_eq!(loaded.task_forces[0].entries.len(), 1);
        assert_eq!(loaded.task_forces[0].entries[0].count, 2);
        assert_eq!(loaded.ai_triggers.len(), 2);
        assert!(!loaded.ai_triggers[0].enabled);
        assert!(loaded.ai_triggers[1].enabled);
        let trigger = &loaded.ai_triggers[0];
        assert_eq!(trigger.display_name, "Map override");
        assert_eq!(trigger.primary_team_type.as_deref(), Some("FIRST"));
        assert_eq!(trigger.owner, AiTriggerOwnerIni::All);
        assert_eq!(trigger.tokens[3], "2");
        assert_eq!(trigger.condition, 4);
        assert_eq!(trigger.object_type, None);
        assert_eq!(trigger.comparison_mask, [0; 32]);
        assert_eq!(
            trigger.weights,
            [
                NativeF64Bits::from_bits(40.0_f64.to_bits()),
                NativeF64Bits::from_bits(10.0_f64.to_bits()),
                NativeF64Bits::from_bits(40.0_f64.to_bits()),
            ]
        );
        assert!(trigger.storage_flag_d0);
        assert_eq!(trigger.storage_i32_ac, 1);
        assert!(!trigger.storage_flag_d1);
        assert_eq!(trigger.secondary_team_type, None);
        assert_eq!(trigger.difficulty_enabled, [true, true, true]);
        assert!(loaded.diagnostics.is_empty());

        let skirmish = TeamAiIniRegistry::from_sources(&fixed, &map, true);
        assert!(
            skirmish.ai_triggers.iter().all(|trigger| trigger.enabled),
            "listed map enable keys force enabled when g_GameMode is nonzero"
        );
    }

    #[test]
    fn script_gaps_compact_and_task_forces_stop_after_six_keys() {
        let fixed = IniFile::from_str(
            "[ScriptTypes]\n0=S\n[S]\n0=2,0\n3=-1,9\n49=64,-7\n50=6,2\n\
             [TaskForces]\n0=T\n[T]\n0=-2,E0\n5=3,E5\n6=4,E6\nGroup=-1\n",
        );
        let loaded = TeamAiIniRegistry::from_sources(&fixed, &IniFile::from_str(""), false);

        assert_eq!(
            loaded.scripts[0].actions,
            vec![
                ScriptActionIni { action_id: 2, argument: 0 },
                ScriptActionIni { action_id: -1, argument: 9 },
                ScriptActionIni { action_id: 64, argument: -7 },
            ]
        );
        assert_eq!(
            loaded.task_forces[0].entries,
            vec![
                TaskForceEntryIni { count: -2, member_type: "E0".to_string() },
                TaskForceEntryIni { count: 3, member_type: "E5".to_string() },
            ]
        );
    }

    #[test]
    fn malformed_ai_trigger_is_refused_without_padding() {
        let fixed = IniFile::from_str("[AITriggerTypes]\nBAD=a,b,c\n");
        let loaded = TeamAiIniRegistry::from_sources(&fixed, &IniFile::from_str(""), false);

        assert!(loaded.ai_triggers.is_empty());
        assert_eq!(
            loaded.diagnostics,
            vec![TeamAiIniDiagnostic::MalformedAiTrigger {
                trigger_id: "BAD".to_string(),
                token_count: 3,
                source: TeamAiDefinitionSource::FixedAimd,
            }]
        );
    }

    #[test]
    fn malformed_ai_trigger_comparison_is_a_source_tagged_refusal() {
        let fixed = IniFile::from_str(&format!(
            "[AITriggerTypes]\nBAD={}\n",
            [
                "Bad", "<none>", "<all>", "0", "0", "<none>", "0g", "1", "1", "1", "1", "0", "1",
                "0", "<none>", "1", "1", "1",
            ]
            .join(",")
        ));
        let loaded = TeamAiIniRegistry::from_sources(&fixed, &IniFile::from_str(""), false);

        assert!(loaded.ai_triggers.is_empty());
        assert_eq!(
            loaded.diagnostics,
            vec![TeamAiIniDiagnostic::MalformedAiTriggerComparison {
                trigger_id: "BAD".to_string(),
                value: "0g".to_string(),
                source: TeamAiDefinitionSource::FixedAimd,
            }]
        );
        assert!(!loaded.fixed_source_is_complete());
    }

    #[test]
    fn sentinel_task_force_member_is_a_source_tagged_refusal() {
        let fixed = IniFile::from_str(
            "[TaskForces]\n0=BAD_FORCE\n[BAD_FORCE]\n0=1,<none>\n",
        );
        let loaded = TeamAiIniRegistry::from_sources(&fixed, &IniFile::from_str(""), false);

        assert!(loaded.task_forces[0].entries.is_empty());
        assert_eq!(
            loaded.diagnostics,
            vec![TeamAiIniDiagnostic::MalformedTaskForceEntry {
                task_force_id: "BAD_FORCE".to_string(),
                key: 0,
                value: "1,<none>".to_string(),
                source: TeamAiDefinitionSource::FixedAimd,
            }]
        );
        assert!(!loaded.fixed_source_is_complete());
    }

    #[test]
    #[ignore = "requires VERA20K_RETAIL_AIMD pointing at extracted retail aimd.ini"]
    fn retail_aimd_registry_oracle() {
        let path = std::env::var_os("VERA20K_RETAIL_AIMD")
            .expect("set VERA20K_RETAIL_AIMD to extracted retail aimd.ini");
        let bytes = std::fs::read(path).expect("read retail aimd.ini");
        let fixed = IniFile::from_bytes(&bytes).expect("parse retail aimd.ini");
        let loaded = TeamAiIniRegistry::from_sources(&fixed, &IniFile::from_str(""), false);

        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        assert_eq!(loaded.task_forces.len(), 132);
        assert_eq!(loaded.scripts.len(), 88);
        assert_eq!(loaded.team_types.len(), 163);
        assert_eq!(loaded.ai_triggers.len(), 165);
        assert!(loaded.ai_triggers.iter().all(|trigger| trigger.tokens.len() == 18));
        assert_eq!(
            loaded
                .ai_triggers
                .iter()
                .filter(|trigger| matches!(trigger.owner, AiTriggerOwnerIni::Country(_)))
                .count(),
            10
        );
        assert_eq!(
            loaded
                .ai_triggers
                .iter()
                .filter(|trigger| trigger.object_type.is_some())
                .count(),
            109
        );
        assert_eq!(
            loaded
                .ai_triggers
                .iter()
                .filter(|trigger| trigger.secondary_team_type.is_some())
                .count(),
            49
        );
        let anti_nuke = loaded
            .ai_triggers
            .iter()
            .find(|trigger| trigger.id == "0CAD0DCC-G")
            .expect("stock Allied Anti-Nuke trigger");
        assert_eq!(anti_nuke.display_name, "Allied Anti-Nuke 1");
        assert_eq!(anti_nuke.primary_team_type.as_deref(), Some("08DA125C-G"));
        assert_eq!(anti_nuke.owner, AiTriggerOwnerIni::All);
        assert_eq!(anti_nuke.tokens[3], "9");
        assert_eq!(anti_nuke.condition, 0);
        assert_eq!(anti_nuke.object_type.as_deref(), Some("NAMISL"));
        assert_eq!(&anti_nuke.comparison_mask[..5], &[1, 0, 0, 0, 3]);
        assert_eq!(
            anti_nuke.weights,
            [
                NativeF64Bits::from_bits(70.0_f64.to_bits()),
                NativeF64Bits::from_bits(10.0_f64.to_bits()),
                NativeF64Bits::from_bits(70.0_f64.to_bits()),
            ]
        );
        assert!(anti_nuke.storage_flag_d0);
        assert_eq!(anti_nuke.storage_i32_ac, 1);
        assert!(!anti_nuke.storage_flag_d1);
        assert_eq!(anti_nuke.secondary_team_type.as_deref(), Some("0CB246CC-G"));
        assert_eq!(anti_nuke.difficulty_enabled, [false, true, true]);
        assert_eq!(
            loaded
                .team_types
                .iter()
                .filter(|team| team.read_bool("Autocreate", false))
                .count(),
            163
        );
        assert_eq!(
            loaded
                .team_types
                .iter()
                .filter(|team| team.read_bool("IsBaseDefense", false))
                .count(),
            12
        );

        let mut priorities = BTreeMap::<i32, usize>::new();
        for team in &loaded.team_types {
            *priorities.entry(team.read_int("Priority", 7)).or_default() += 1;
        }
        assert_eq!(
            priorities,
            BTreeMap::from([
                (5, 89),
                (7, 46),
                (10, 4),
                (12, 1),
                (14, 2),
                (15, 1),
                (20, 2),
                (25, 6),
                (30, 4),
                (50, 8),
            ])
        );
    }
}
