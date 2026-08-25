//! Deterministic TeamClass/ScriptClass execution seam for YR 1.001.
//!
//! The data here is resolved at the scenario boundary rather than parsed from
//! INI directly. `TeamClass::AI` is intentionally represented as raw action
//! records: only native action bodies with closed evidence execute.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use crate::sim::command::CommandEnvelope;
use crate::sim::intern::InternedId;

/// One resolved ScriptType action record.
///
/// This matches the `(action, argument)` pair read by `ScriptClass` instead of
/// promoting descriptive opcode names into a complete, guessed enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamScriptAction {
    pub action_id: u8,
    pub argument: i32,
}

/// One resolved ScriptType program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamScriptDefinition {
    pub id: InternedId,
    pub actions: Vec<TeamScriptAction>,
}

/// One TaskForce requirement. `member_type` is a resolved TechnoType identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamTaskForceEntry {
    pub member_type: InternedId,
    pub count: u32,
}

/// The TeamType-attached TaskForce definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamTaskForceDefinition {
    pub id: InternedId,
    pub entries: Vec<TeamTaskForceEntry>,
}

/// The resolved ScriptType and TaskForce attachments carried by a TeamType.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamTypeDefinition {
    pub id: InternedId,
    pub script_id: InternedId,
    pub task_force_id: InternedId,
    /// Signed TeamType `Priority=` consumed by House base-defence suspension.
    pub priority: i32,
    /// TeamType `IsBaseDefense=` byte used by responder admission/assignment.
    pub is_base_defense: bool,
}

/// A candidate member passed to TeamType/TaskForce admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamScriptMember {
    pub entity_id: u64,
    pub member_type: InternedId,
}

/// Action-19 side effect emitted in TeamClass member-list order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamScriptEffect {
    PanicMember { entity_id: u64 },
}

/// Effects produced by one TeamClass update pass.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct TeamScriptTick {
    pub effects: Vec<TeamScriptEffect>,
}

/// A serializable refusal instead of an inferred ScriptType transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeamScriptRefusal {
    MissingScript { script_id: InternedId },
    MissingTeamType { team_type_id: InternedId },
    MissingTaskForce { task_force_id: InternedId },
    UnsupportedAction { action_id: u8 },
    OutOfRangeAction { action_id: u8 },
}

/// Persistent TeamClass execution state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamScriptState {
    id: u64,
    owner: InternedId,
    team_type_id: Option<InternedId>,
    task_force_id: Option<InternedId>,
    script_id: InternedId,
    cursor: i32,
    advance_pending: bool,
    delay_remaining_frames: u32,
    wait_condition_complete: bool,
    /// Three independent TeamClass bytes written by `FUN_006EC250`. Neutral
    /// displacement names avoid assigning broader semantics than the live
    /// suspension transaction proves.
    response_latch_7d: bool,
    response_latch_7e: bool,
    response_latch_83: bool,
    response_suspend_start_frame: i32,
    response_suspend_duration_frames: i32,
    members: Vec<u64>,
    member_type_counts: BTreeMap<InternedId, u32>,
    target: Option<u64>,
    succeeded: bool,
    completed: bool,
    refusal: Option<TeamScriptRefusal>,
}

impl TeamScriptState {
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn owner(&self) -> InternedId {
        self.owner
    }

    pub fn team_type_id(&self) -> Option<InternedId> {
        self.team_type_id
    }

    pub fn task_force_id(&self) -> Option<InternedId> {
        self.task_force_id
    }

    pub fn script_id(&self) -> InternedId {
        self.script_id
    }

    pub fn cursor(&self) -> i32 {
        self.cursor
    }

    pub fn advance_pending(&self) -> bool {
        self.advance_pending
    }

    pub fn wait_frames_remaining(&self) -> u32 {
        self.delay_remaining_frames
    }

    pub fn members(&self) -> &[u64] {
        &self.members
    }

    pub(crate) fn response_suspension_state(&self) -> (bool, bool, bool, i32, i32) {
        (
            self.response_latch_7d,
            self.response_latch_7e,
            self.response_latch_83,
            self.response_suspend_start_frame,
            self.response_suspend_duration_frames,
        )
    }

    pub fn member_type_counts(&self) -> &BTreeMap<InternedId, u32> {
        &self.member_type_counts
    }

    pub fn target(&self) -> Option<u64> {
        self.target
    }

    pub fn succeeded(&self) -> bool {
        self.succeeded
    }

    pub fn completed(&self) -> bool {
        self.completed
    }

    pub fn refusal(&self) -> Option<TeamScriptRefusal> {
        self.refusal
    }
}

/// Owns resolved TeamType/TaskForce/ScriptType definitions and live TeamClass cursors.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TeamScriptVm {
    scripts: BTreeMap<InternedId, TeamScriptDefinition>,
    task_forces: BTreeMap<InternedId, TeamTaskForceDefinition>,
    team_types: BTreeMap<InternedId, TeamTypeDefinition>,
    teams: BTreeMap<u64, TeamScriptState>,
    next_team_id: u64,
}

impl TeamScriptVm {
    pub fn register_script(&mut self, definition: TeamScriptDefinition) {
        self.scripts.insert(definition.id, definition);
    }

    pub fn register_task_force(&mut self, definition: TeamTaskForceDefinition) {
        self.task_forces.insert(definition.id, definition);
    }

    pub fn register_team_type(&mut self, definition: TeamTypeDefinition) {
        self.team_types.insert(definition.id, definition);
    }

    /// Install an already-admitted TeamClass member list.
    ///
    /// This remains useful for scenario seams that do not yet instantiate
    /// TaskForce-backed members. New callers should use `create_team_from_type`.
    pub fn create_team(
        &mut self,
        owner: InternedId,
        script_id: InternedId,
        members: Vec<u64>,
        target: Option<u64>,
    ) -> u64 {
        self.insert_team(
            owner,
            None,
            None,
            script_id,
            members,
            BTreeMap::new(),
            target,
        )
    }

    /// Resolve the TeamType attachments and admit matching candidates in
    /// TaskForce-entry order, preserving input order within each type.
    pub fn create_team_from_type(
        &mut self,
        owner: InternedId,
        team_type_id: InternedId,
        candidates: &[TeamScriptMember],
        target: Option<u64>,
    ) -> u64 {
        let Some(team_type) = self.team_types.get(&team_type_id).copied() else {
            return self.insert_refused_team(
                owner,
                team_type_id,
                TeamScriptRefusal::MissingTeamType { team_type_id },
                target,
            );
        };
        let Some(task_force) = self.task_forces.get(&team_type.task_force_id) else {
            return self.insert_refused_team(
                owner,
                team_type_id,
                TeamScriptRefusal::MissingTaskForce {
                    task_force_id: team_type.task_force_id,
                },
                target,
            );
        };
        if !self.scripts.contains_key(&team_type.script_id) {
            return self.insert_refused_team(
                owner,
                team_type_id,
                TeamScriptRefusal::MissingScript {
                    script_id: team_type.script_id,
                },
                target,
            );
        }

        let mut used = vec![false; candidates.len()];
        let mut members = Vec::new();
        let mut counts = BTreeMap::new();
        for entry in &task_force.entries {
            for _ in 0..entry.count {
                let Some((index, candidate)) =
                    candidates.iter().enumerate().find(|(index, candidate)| {
                        !used[*index] && candidate.member_type == entry.member_type
                    })
                else {
                    break;
                };
                used[index] = true;
                members.push(candidate.entity_id);
                *counts.entry(entry.member_type).or_insert(0) += 1;
            }
        }

        self.insert_team(
            owner,
            Some(team_type_id),
            Some(team_type.task_force_id),
            team_type.script_id,
            members,
            counts,
            target,
        )
    }

    pub fn team(&self, id: u64) -> Option<&TeamScriptState> {
        self.teams.get(&id)
    }

    /// Resolve one entity's TeamClass pointer analogue in stable Team creation
    /// order. A member can belong to at most one live TeamClass in native.
    pub(crate) fn team_for_member(&self, entity_id: u64) -> Option<(u64, bool)> {
        self.teams.values().find_map(|team| {
            team.members.contains(&entity_id).then(|| {
                let is_base_defense = team
                    .team_type_id
                    .and_then(|id| self.team_types.get(&id))
                    .is_some_and(|definition| definition.is_base_defense);
                (team.id, is_base_defense)
            })
        })
    }

    /// Native `FUN_006EC250`: visit TeamClass instances in creation order,
    /// suspend those owned by `owner` whose signed TeamType priority is below
    /// `suspend_priority`, remove every member in member-list order, set the
    /// three response bytes, and arm the signed start/duration timer.
    pub(crate) fn suspend_teams_for_base_defense(
        &mut self,
        owner: InternedId,
        suspend_priority: i32,
        current_frame: i32,
        duration_frames: i32,
    ) -> Vec<u64> {
        let team_types = &self.team_types;
        let mut removed = Vec::new();
        for team in self.teams.values_mut() {
            let Some(definition) = team
                .team_type_id
                .and_then(|team_type_id| team_types.get(&team_type_id))
            else {
                continue;
            };
            if team.owner != owner || definition.priority >= suspend_priority {
                continue;
            }

            removed.extend(team.members.drain(..));
            team.member_type_counts.clear();
            team.response_latch_7d = true;
            team.response_latch_7e = true;
            team.response_latch_83 = true;
            team.response_suspend_start_frame = current_frame;
            team.response_suspend_duration_frames = duration_frames;
        }
        removed
    }

    pub fn set_delay(&mut self, id: u64, remaining_frames: u32) -> bool {
        let Some(team) = self.teams.get_mut(&id) else {
            return false;
        };
        team.delay_remaining_frames = remaining_frames;
        true
    }

    pub fn set_wait_condition_complete(&mut self, id: u64, complete: bool) -> bool {
        let Some(team) = self.teams.get_mut(&id) else {
            return false;
        };
        team.wait_condition_complete = complete;
        true
    }

    /// Execute one `TeamClass::AI` pass for every live team.
    ///
    /// `TeamClass::AI` stores completion in `+0x80`; the next update performs
    /// the shared `ScriptClass::HasNextMission` cursor advance.
    pub fn tick<F>(&mut self, execute_tick: u64, owner_is_active: F) -> Vec<CommandEnvelope>
    where
        F: FnMut(InternedId) -> bool,
    {
        self.tick_effects(execute_tick, owner_is_active);
        Vec::new()
    }

    /// Execute one pass and return native-backed non-command effects.
    ///
    /// The current command rung has no panic command; callers that own member
    /// mission application can consume these ordered effects directly.
    pub fn tick_effects<F>(&mut self, _execute_tick: u64, mut owner_is_active: F) -> TeamScriptTick
    where
        F: FnMut(InternedId) -> bool,
    {
        let mut result = TeamScriptTick::default();

        for team in self.teams.values_mut() {
            if team.completed || team.refusal.is_some() || !owner_is_active(team.owner) {
                continue;
            }

            if team.advance_pending {
                team.advance_pending = false;
                team.cursor = team.cursor.wrapping_add(1);
                if !script_action_at(self.scripts.get(&team.script_id), team.cursor).is_some() {
                    team.completed = true;
                }
                continue;
            }

            if team.delay_remaining_frames > 0 {
                team.delay_remaining_frames -= 1;
                if team.delay_remaining_frames > 0 {
                    continue;
                }
            }

            let Some(script) = self.scripts.get(&team.script_id) else {
                team.refusal = Some(TeamScriptRefusal::MissingScript {
                    script_id: team.script_id,
                });
                log::warn!(
                    "TeamClass::AI stopped team {}: missing ScriptType {}",
                    team.id,
                    team.script_id
                );
                continue;
            };
            let Some(action) = script_action_at(Some(script), team.cursor) else {
                team.completed = true;
                continue;
            };

            match action.action_id {
                // TeamClass::AI default path for case 2: neither complete nor advance.
                2 => {}
                // TeamClass::AI cases 5 and 43 wait until their gate completes.
                5 | 43 => {
                    if team.wait_condition_complete {
                        team.wait_condition_complete = false;
                        team.advance_pending = true;
                    }
                }
                // TeamClass::AI case 6 calls SetMission(argument - 2), then advances next update.
                6 => {
                    team.cursor = action.argument.wrapping_sub(2);
                    team.advance_pending = true;
                }
                // TeamClass::AI case 19 invokes Foot's panic-family virtual in member-list order.
                19 => {
                    result.effects.extend(
                        team.members
                            .iter()
                            .copied()
                            .map(|entity_id| TeamScriptEffect::PanicMember { entity_id }),
                    );
                    team.advance_pending = true;
                }
                // TeamClass::AI case 24 takes the shared advance path.
                24 => team.advance_pending = true,
                // TeamClass::AI case 49 records success but its +0x84 byte is not CRC-covered.
                49 => {
                    team.succeeded = true;
                    team.advance_pending = true;
                }
                0..=64 => {
                    team.refusal = Some(TeamScriptRefusal::UnsupportedAction {
                        action_id: action.action_id,
                    });
                    log::warn!(
                        "TeamClass::AI stopped team {}: ScriptType action {} is not implemented",
                        team.id,
                        action.action_id
                    );
                }
                action_id => {
                    team.refusal = Some(TeamScriptRefusal::OutOfRangeAction { action_id });
                    log::warn!(
                        "TeamClass::AI stopped team {}: ScriptType action {} is outside 0..=64",
                        team.id,
                        action_id
                    );
                }
            }
        }

        result
    }

    pub(crate) fn hash_state(&self, hasher: &mut impl Hasher) {
        // TeamClass::ComputeCRC observes live Team/Script state, not the VM's
        // source registry or allocator. Action-49's +0x84 success flag is not
        // included by the captured YR 1.001 CRC sequence.
        self.teams.len().hash(hasher);
        for (id, team) in &self.teams {
            id.hash(hasher);
            team.team_type_id.hash(hasher);
            team.task_force_id.hash(hasher);
            team.script_id.hash(hasher);
            team.owner.hash(hasher);
            team.cursor.hash(hasher);
            team.advance_pending.hash(hasher);
            team.delay_remaining_frames.hash(hasher);
            team.response_latch_7d.hash(hasher);
            team.response_latch_7e.hash(hasher);
            team.response_latch_83.hash(hasher);
            team.response_suspend_start_frame.hash(hasher);
            team.response_suspend_duration_frames.hash(hasher);
            team.members.hash(hasher);
            team.target.hash(hasher);
            team.member_type_counts.hash(hasher);
        }
    }

    fn insert_refused_team(
        &mut self,
        owner: InternedId,
        team_type_id: InternedId,
        refusal: TeamScriptRefusal,
        target: Option<u64>,
    ) -> u64 {
        let script_id = match refusal {
            TeamScriptRefusal::MissingScript { script_id } => script_id,
            _ => InternedId::from_index(0),
        };
        let id = self.insert_team(
            owner,
            Some(team_type_id),
            None,
            script_id,
            Vec::new(),
            BTreeMap::new(),
            target,
        );
        self.teams.get_mut(&id).expect("inserted team").refusal = Some(refusal);
        id
    }

    fn insert_team(
        &mut self,
        owner: InternedId,
        team_type_id: Option<InternedId>,
        task_force_id: Option<InternedId>,
        script_id: InternedId,
        members: Vec<u64>,
        member_type_counts: BTreeMap<InternedId, u32>,
        target: Option<u64>,
    ) -> u64 {
        let id = self.next_team_id;
        self.next_team_id = self.next_team_id.wrapping_add(1);
        self.teams.insert(
            id,
            TeamScriptState {
                id,
                owner,
                team_type_id,
                task_force_id,
                script_id,
                cursor: 0,
                advance_pending: false,
                delay_remaining_frames: 0,
                wait_condition_complete: false,
                response_latch_7d: false,
                response_latch_7e: false,
                response_latch_83: false,
                response_suspend_start_frame: 0,
                response_suspend_duration_frames: 0,
                members,
                member_type_counts,
                target,
                succeeded: false,
                completed: false,
                refusal: None,
            },
        );
        id
    }
}

fn script_action_at(
    script: Option<&TeamScriptDefinition>,
    cursor: i32,
) -> Option<TeamScriptAction> {
    let index = usize::try_from(cursor).ok()?;
    script?.actions.get(index).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;

    fn action(action_id: u8, argument: i32) -> TeamScriptAction {
        TeamScriptAction {
            action_id,
            argument,
        }
    }

    fn state_hash(vm: &TeamScriptVm) -> u64 {
        let mut hasher = DefaultHasher::new();
        vm.hash_state(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn case_six_sets_argument_minus_two_then_advances_next_update() {
        let owner = InternedId::from_index(1);
        let script = InternedId::from_index(2);
        let mut vm = TeamScriptVm::default();
        vm.register_script(TeamScriptDefinition {
            id: script,
            actions: vec![action(6, 2), action(2, 0)],
        });
        let team = vm.create_team(owner, script, vec![], None);

        vm.tick_effects(1, |_| true);
        assert_eq!(vm.team(team).expect("team").cursor(), 0);
        assert!(vm.team(team).expect("team").advance_pending());

        vm.tick_effects(2, |_| true);
        assert_eq!(vm.team(team).expect("team").cursor(), 1);
        assert!(!vm.team(team).expect("team").advance_pending());
    }

    #[test]
    fn wait_actions_gate_then_use_the_common_advance() {
        let owner = InternedId::from_index(1);
        let script = InternedId::from_index(2);
        let mut vm = TeamScriptVm::default();
        vm.register_script(TeamScriptDefinition {
            id: script,
            actions: vec![action(5, 2), action(43, 0)],
        });
        let team = vm.create_team(owner, script, vec![], None);

        vm.tick_effects(10, |_| true);
        assert_eq!(vm.team(team).expect("team").cursor(), 0);
        assert!(!vm.team(team).expect("team").advance_pending());
        assert!(vm.set_wait_condition_complete(team, true));
        vm.tick_effects(11, |_| true);
        assert!(vm.team(team).expect("team").advance_pending());
        vm.tick_effects(12, |_| true);
        assert_eq!(vm.team(team).expect("team").cursor(), 1);
    }

    #[test]
    fn action_nineteen_preserves_member_order_and_defers_advance() {
        let owner = InternedId::from_index(1);
        let script = InternedId::from_index(2);
        let mut vm = TeamScriptVm::default();
        vm.register_script(TeamScriptDefinition {
            id: script,
            actions: vec![action(19, 0), action(2, 0)],
        });
        let team = vm.create_team(owner, script, vec![9, 3, 7], None);

        let tick = vm.tick_effects(1, |_| true);
        assert_eq!(
            tick.effects,
            vec![
                TeamScriptEffect::PanicMember { entity_id: 9 },
                TeamScriptEffect::PanicMember { entity_id: 3 },
                TeamScriptEffect::PanicMember { entity_id: 7 },
            ]
        );
        assert!(vm.team(team).expect("team").advance_pending());
        vm.tick_effects(2, |_| true);
        assert_eq!(vm.team(team).expect("team").cursor(), 1);
    }

    #[test]
    fn success_flag_is_persisted_but_not_crc_projected() {
        let owner = InternedId::from_index(1);
        let script = InternedId::from_index(2);
        let mut vm = TeamScriptVm::default();
        vm.register_script(TeamScriptDefinition {
            id: script,
            actions: vec![action(49, 0)],
        });
        let team = vm.create_team(owner, script, vec![], None);
        vm.tick_effects(1, |_| true);

        let mut without_success = vm.clone();
        without_success
            .teams
            .get_mut(&team)
            .expect("team")
            .succeeded = false;
        assert_eq!(state_hash(&vm), state_hash(&without_success));
        assert!(vm.team(team).expect("team").succeeded());
    }

    #[test]
    fn unsupported_and_out_of_range_actions_never_advance() {
        let owner = InternedId::from_index(1);
        let supported = InternedId::from_index(2);
        let out_of_range = InternedId::from_index(3);
        let mut vm = TeamScriptVm::default();
        vm.register_script(TeamScriptDefinition {
            id: supported,
            actions: vec![action(0, 0)],
        });
        vm.register_script(TeamScriptDefinition {
            id: out_of_range,
            actions: vec![action(65, 0)],
        });
        let first = vm.create_team(owner, supported, vec![], None);
        let second = vm.create_team(owner, out_of_range, vec![], None);

        vm.tick_effects(1, |_| true);
        assert_eq!(
            vm.team(first).expect("team").refusal(),
            Some(TeamScriptRefusal::UnsupportedAction { action_id: 0 })
        );
        assert_eq!(
            vm.team(second).expect("team").refusal(),
            Some(TeamScriptRefusal::OutOfRangeAction { action_id: 65 })
        );
    }

    #[test]
    fn task_force_admission_uses_entry_then_candidate_order() {
        let owner = InternedId::from_index(1);
        let script = InternedId::from_index(2);
        let task_force = InternedId::from_index(3);
        let team_type = InternedId::from_index(4);
        let tank = InternedId::from_index(5);
        let infantry = InternedId::from_index(6);
        let mut vm = TeamScriptVm::default();
        vm.register_script(TeamScriptDefinition {
            id: script,
            actions: vec![action(2, 0)],
        });
        vm.register_task_force(TeamTaskForceDefinition {
            id: task_force,
            entries: vec![
                TeamTaskForceEntry {
                    member_type: infantry,
                    count: 2,
                },
                TeamTaskForceEntry {
                    member_type: tank,
                    count: 1,
                },
            ],
        });
        vm.register_team_type(TeamTypeDefinition {
            id: team_type,
            script_id: script,
            task_force_id: task_force,
            priority: 0,
            is_base_defense: false,
        });
        let team = vm.create_team_from_type(
            owner,
            team_type,
            &[
                TeamScriptMember {
                    entity_id: 10,
                    member_type: tank,
                },
                TeamScriptMember {
                    entity_id: 20,
                    member_type: infantry,
                },
                TeamScriptMember {
                    entity_id: 30,
                    member_type: infantry,
                },
            ],
            None,
        );

        let state = vm.team(team).expect("team");
        assert_eq!(state.members(), &[20, 30, 10]);
        assert_eq!(state.member_type_counts().get(&infantry), Some(&2));
        assert_eq!(state.member_type_counts().get(&tank), Some(&1));
    }

    #[test]
    fn gsi_04_05_base_defense_suspension_uses_signed_priority_and_ordered_removal() {
        fn install(
            vm: &mut TeamScriptVm,
            owner: InternedId,
            ordinal: u32,
            priority: i32,
            is_base_defense: bool,
            members: &[u64],
        ) -> u64 {
            let script = InternedId::from_index(100);
            let member_type = InternedId::from_index(101);
            let task_force = InternedId::from_index(110 + ordinal);
            let team_type = InternedId::from_index(120 + ordinal);
            if !vm.scripts.contains_key(&script) {
                vm.register_script(TeamScriptDefinition {
                    id: script,
                    actions: vec![action(2, 0)],
                });
            }
            vm.register_task_force(TeamTaskForceDefinition {
                id: task_force,
                entries: vec![TeamTaskForceEntry {
                    member_type,
                    count: members.len() as u32,
                }],
            });
            vm.register_team_type(TeamTypeDefinition {
                id: team_type,
                script_id: script,
                task_force_id: task_force,
                priority,
                is_base_defense,
            });
            let candidates = members
                .iter()
                .copied()
                .map(|entity_id| TeamScriptMember {
                    entity_id,
                    member_type,
                })
                .collect::<Vec<_>>();
            vm.create_team_from_type(owner, team_type, &candidates, None)
        }

        let owner = InternedId::from_index(1);
        let other_owner = InternedId::from_index(2);
        let mut vm = TeamScriptVm::default();
        let low = install(&mut vm, owner, 0, 0, false, &[10, 20]);
        let high_base_defense = install(&mut vm, owner, 1, 1, true, &[30]);
        let other = install(&mut vm, other_owner, 2, -7, false, &[40]);
        assert_eq!(vm.team_for_member(30), Some((high_base_defense, true)));

        let before = state_hash(&vm);
        let removed = vm.suspend_teams_for_base_defense(owner, 1, -9, 1800);
        assert_eq!(removed, vec![10, 20]);
        assert!(vm.team(low).unwrap().members().is_empty());
        assert!(vm.team(low).unwrap().member_type_counts().is_empty());
        assert_eq!(
            vm.team(low).unwrap().response_suspension_state(),
            (true, true, true, -9, 1800)
        );
        assert_eq!(vm.team(high_base_defense).unwrap().members(), &[30]);
        assert_eq!(vm.team(other).unwrap().members(), &[40]);
        assert_eq!(vm.team_for_member(10), None);
        assert_ne!(before, state_hash(&vm));

        let restored: TeamScriptVm =
            serde_json::from_str(&serde_json::to_string(&vm).unwrap()).unwrap();
        assert_eq!(
            restored.team(low).unwrap().response_suspension_state(),
            (true, true, true, -9, 1800)
        );
        assert_eq!(state_hash(&restored), state_hash(&vm));
    }
}
