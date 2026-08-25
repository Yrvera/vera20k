//! Active-YR AIMD + map registry ingestion.
//!
//! `gamemd.exe` keeps these definitions outside `RulesClass`: fixed
//! `AIMD.INI` records are visited first and the scenario INI then re-reads
//! matching identities in place or appends new ones. This module preserves
//! that source order and raw payload. It does not select AI triggers, create
//! Teams, recruit members, or consume simulation RNG.

use std::collections::BTreeMap;

use crate::rules::ini_parser::{IniFile, IniSection};
use crate::rules::ini_value::{atoi_lenient, parse_read_int_value};

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
pub struct AiTriggerTypeIni {
    pub id: String,
    pub tokens: [String; AI_TRIGGER_TOKEN_COUNT],
    pub enabled: bool,
    pub source: TeamAiDefinitionSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TeamAiIniDiagnostic {
    MissingDefinitionSection {
        registry: &'static str,
        id: String,
    },
    MalformedScriptAction {
        script_id: String,
        key: usize,
        value: String,
    },
    MalformedTaskForceEntry {
        task_force_id: String,
        key: usize,
        value: String,
    },
    MalformedAiTrigger {
        trigger_id: String,
        token_count: usize,
    },
    UnknownAiTriggerEnable {
        trigger_id: String,
    },
}

/// Ordered unresolved AI definitions from fixed AIMD and the scenario INI.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TeamAiIniRegistry {
    pub team_types: Vec<TeamTypeIni>,
    pub scripts: Vec<ScriptTypeIni>,
    pub task_forces: Vec<TaskForceIni>,
    pub ai_triggers: Vec<AiTriggerTypeIni>,
    pub diagnostics: Vec<TeamAiIniDiagnostic>,
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
        let mut registry = Self::default();

        registry.read_team_types(fixed_aimd, TeamAiDefinitionSource::FixedAimd);
        registry.read_team_types(scenario, TeamAiDefinitionSource::Scenario);

        registry.read_scripts(fixed_aimd, TeamAiDefinitionSource::FixedAimd);
        registry.read_scripts(scenario, TeamAiDefinitionSource::Scenario);

        registry.read_task_forces(fixed_aimd, TeamAiDefinitionSource::FixedAimd);
        registry.read_task_forces(scenario, TeamAiDefinitionSource::Scenario);

        registry.read_ai_triggers(fixed_aimd, TeamAiDefinitionSource::FixedAimd);
        registry.read_ai_triggers(scenario, TeamAiDefinitionSource::Scenario);
        registry.read_ai_trigger_enables(scenario, game_mode_nonzero);

        registry
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
                    });
                continue;
            };
            let identity = canonical_identity(id);
            if let Some(position) = index.get(&identity).copied() {
                self.team_types[position].overlay(section, source);
            } else {
                let mut definition = TeamTypeIni {
                    id: id.to_string(),
                    fields: Vec::new(),
                    source,
                };
                definition.overlay(section, source);
                index.insert(identity, self.team_types.len());
                self.team_types.push(definition);
            }
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
                        });
                    continue;
                };
                let member_type = member_type.trim();
                if member_type.is_empty() || !valid_identity(member_type) {
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
                    });
                continue;
            };
            let identity = canonical_identity(id);
            if let Some(position) = index.get(&identity).copied() {
                let enabled = self.ai_triggers[position].enabled;
                self.ai_triggers[position] = AiTriggerTypeIni {
                    id: id.to_string(),
                    tokens,
                    enabled,
                    source,
                };
            } else {
                let enabled = source == TeamAiDefinitionSource::FixedAimd;
                index.insert(identity, self.ai_triggers.len());
                self.ai_triggers.push(AiTriggerTypeIni {
                    id: id.to_string(),
                    tokens,
                    enabled,
                    source,
                });
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
        [
            name, primary, "<all>", "2", "4", "<none>", "00", "40.0", "10.0",
            "40.0", "1", "0", "1", "0", "<none>", "1", "1", "1",
        ]
        .join(",")
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
            "[TeamTypes]\n0=first\n1=THIRD\n\
             [first]\nPriority=20\n\
             [THIRD]\nPriority=9\nScript=S1\nTaskForce=T1\n\
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
        assert_eq!(loaded.team_types[0].read_int("Priority", 7), 20);
        assert!(loaded.team_types[0].read_bool("Autocreate", false));
        assert_eq!(loaded.scripts[0].actions, vec![ScriptActionIni { action_id: 49, argument: 0 }]);
        assert_eq!(loaded.task_forces[0].entries.len(), 1);
        assert_eq!(loaded.task_forces[0].entries[0].count, 2);
        assert_eq!(loaded.ai_triggers.len(), 2);
        assert!(!loaded.ai_triggers[0].enabled);
        assert!(loaded.ai_triggers[1].enabled);
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
            }]
        );
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
