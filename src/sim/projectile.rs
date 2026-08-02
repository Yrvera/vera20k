//! Authoritative in-flight projectile state.
//!
//! This module deliberately owns only delayed flight, target expiry, collision,
//! and detonation admission. Combat remains the authority for turning a returned
//! [`ProjectileDetonation`] into damage, warhead effects, and terrain changes.
//! Keeping that handoff narrow lets the world phase place this system at the
//! verified native frame rung without duplicating combat arithmetic here.

use std::collections::BTreeMap;

use crate::sim::intern::InternedId;

/// Lepton-space position for an in-flight projectile.
///
/// Cells contain 256 leptons. Keeping the flight state in signed integer
/// leptons avoids float math and makes the serialized state independent of
/// render coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ProjectileCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ProjectileCoord {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

/// The original target retained by a projectile after weapon fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ProjectileTarget {
    Entity(u64),
    Cell(ProjectileCoord),
}

/// What to do when an entity target no longer exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum TargetExpiryPolicy {
    /// Remove the projectile without a damage handoff.
    Expire,
    /// Detonate at the last target coordinate observed by the projectile.
    DetonateAtLastKnown,
}

/// Terrain checks admitted for one ordinary `BulletClass` flight.
///
/// The cases intentionally stay narrow: special trajectory kernels retain the
/// typed immediate path until their native coordinate contracts are ported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectileCollisionPolicy {
    /// `BulletTypeClass::Level`: detonate after entering a non-water cell.
    pub level_non_water: bool,
    /// `BulletTypeClass::SubjectToWalls`: detonate after entering a live wall.
    pub subject_to_walls: bool,
}

impl ProjectileCollisionPolicy {
    pub const NONE: Self = Self {
        level_non_water: false,
        subject_to_walls: false,
    };
}

/// Stable projectile payload transferred to combat only at detonation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectilePayload {
    /// Damage after firing-side modifiers, before target/warhead resolution.
    pub base_damage: i32,
    pub warhead: InternedId,
    /// Weapon identity retained for impact-only effects such as radiation.
    pub weapon: InternedId,
    /// Firing house retained when the source dies before impact.
    pub owner: InternedId,
}

/// Immutable admission data for an ordinary, non-vertical projectile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectileSpawn {
    pub source_id: u64,
    pub origin: ProjectileCoord,
    pub target: ProjectileTarget,
    /// Target coordinate captured when the weapon fires. Non-homing shots keep
    /// this destination; homing shots replace it from the live target table.
    pub initial_target_position: ProjectileCoord,
    pub payload: ProjectilePayload,
    /// Native weapon speed expressed by the caller in leptons per logical frame.
    pub speed_leptons_per_frame: u16,
    /// A projectile may not detonate until this many frames have elapsed.
    pub arm_frames: u16,
    /// Optional fuse duration; zero means the fuse detonates on this advance.
    pub fuse_frames: Option<u16>,
    /// Only homing projectiles update their destination from a live target.
    pub tracks_target: bool,
    pub target_expiry: TargetExpiryPolicy,
    pub collision: ProjectileCollisionPolicy,
}

/// Persistent state corresponding to one native `BulletClass` instance.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Projectile {
    pub id: u64,
    pub source_id: u64,
    pub position: ProjectileCoord,
    pub target: ProjectileTarget,
    pub last_target_position: ProjectileCoord,
    pub payload: ProjectilePayload,
    pub speed_leptons_per_frame: u16,
    pub arm_frames_remaining: u16,
    pub fuse_frames_remaining: Option<u16>,
    pub tracks_target: bool,
    pub target_expiry: TargetExpiryPolicy,
    pub collision: ProjectileCollisionPolicy,
}

/// Why a projectile reached its combat detonation handoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProjectileDetonationReason {
    ReachedTarget,
    Fuse,
    Collision,
    TargetExpired,
}

/// One deferred `BulletClass::Detonate` handoff for combat to apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectileDetonation {
    pub projectile_id: u64,
    pub source_id: u64,
    pub target: ProjectileTarget,
    pub impact: ProjectileCoord,
    pub payload: ProjectilePayload,
    pub reason: ProjectileDetonationReason,
}

/// Results from one stable-order projectile pass.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ProjectileAdvanceResult {
    pub detonations: Vec<ProjectileDetonation>,
    pub expired: Vec<u64>,
}

/// Serialized, stable-id ordered projectile collection.
///
/// `BTreeMap` makes creation-order IDs and processing order explicit. New
/// projectiles are only advanced by the next call, matching the usual
/// object-pass boundary instead of recursively advancing a newly fired shot.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectileStore {
    next_id: u64,
    projectiles: BTreeMap<u64, Projectile>,
}

impl ProjectileStore {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            projectiles: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.projectiles.len()
    }

    pub(crate) fn next_id(&self) -> u64 {
        self.next_id
    }

    pub fn is_empty(&self) -> bool {
        self.projectiles.is_empty()
    }

    pub fn get(&self, id: u64) -> Option<&Projectile> {
        self.projectiles.get(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&u64, &Projectile)> {
        self.projectiles.iter()
    }

    /// Admit one ordinary projectile. Vertical, airburst, cluster, and other
    /// special trajectories remain outside this bounded foundation.
    pub fn spawn(&mut self, spawn: ProjectileSpawn) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.projectiles.insert(
            id,
            Projectile {
                id,
                source_id: spawn.source_id,
                position: spawn.origin,
                target: spawn.target,
                last_target_position: spawn.initial_target_position,
                payload: spawn.payload,
                speed_leptons_per_frame: spawn.speed_leptons_per_frame,
                arm_frames_remaining: spawn.arm_frames,
                fuse_frames_remaining: spawn.fuse_frames,
                tracks_target: spawn.tracks_target,
                target_expiry: spawn.target_expiry,
                collision: spawn.collision,
            },
        );
        id
    }

    /// Advance every currently admitted projectile in ascending stable id.
    ///
    /// `target_positions` must contain live entity targets in lepton space.
    /// `collides_at` is a world-owned terrain/wall admission predicate for the
    /// candidate next coordinate; object collision remains a later port.
    pub fn advance(
        &mut self,
        target_positions: &BTreeMap<u64, ProjectileCoord>,
        mut collides_at: impl FnMut(&Projectile, ProjectileCoord) -> bool,
    ) -> ProjectileAdvanceResult {
        let mut result = ProjectileAdvanceResult::default();
        let ids: Vec<u64> = self.projectiles.keys().copied().collect();

        for id in ids {
            let Some(projectile) = self.projectiles.get_mut(&id) else {
                continue;
            };

            let target_position = match projectile.target {
                ProjectileTarget::Cell(position) => position,
                ProjectileTarget::Entity(target_id) => match target_positions.get(&target_id) {
                    Some(&position) => {
                        if projectile.tracks_target {
                            projectile.last_target_position = position;
                        }
                        projectile.last_target_position
                    }
                    None => match projectile.target_expiry {
                        TargetExpiryPolicy::Expire => {
                            result.expired.push(id);
                            continue;
                        }
                        TargetExpiryPolicy::DetonateAtLastKnown => {
                            if projectile.arm_frames_remaining == 0 {
                                result.detonations.push(detonation(
                                    projectile,
                                    projectile.last_target_position,
                                    ProjectileDetonationReason::TargetExpired,
                                ));
                            } else {
                                result.expired.push(id);
                            }
                            continue;
                        }
                    },
                },
            };

            if let Some(fuse) = projectile.fuse_frames_remaining.as_mut() {
                if *fuse == 0 {
                    result.detonations.push(detonation(
                        projectile,
                        projectile.position,
                        ProjectileDetonationReason::Fuse,
                    ));
                    continue;
                }
                *fuse -= 1;
            }

            let candidate = step_toward(
                projectile.position,
                target_position,
                i32::from(projectile.speed_leptons_per_frame),
            );
            if collides_at(projectile, candidate) {
                if projectile.arm_frames_remaining == 0 {
                    result.detonations.push(detonation(
                        projectile,
                        candidate,
                        ProjectileDetonationReason::Collision,
                    ));
                } else {
                    result.expired.push(id);
                }
                continue;
            }

            projectile.position = candidate;
            if projectile.arm_frames_remaining > 0 {
                projectile.arm_frames_remaining -= 1;
            }
            if candidate == target_position && projectile.arm_frames_remaining == 0 {
                result.detonations.push(detonation(
                    projectile,
                    candidate,
                    ProjectileDetonationReason::ReachedTarget,
                ));
            }
        }

        for id in result.expired.iter().chain(
            result
                .detonations
                .iter()
                .map(|detonation| &detonation.projectile_id),
        ) {
            self.projectiles.remove(id);
        }
        result
    }
}

// YR BulletClass::AI linkage: this is the bounded ordinary-flight rung; exact trajectory kernels remain separate.
fn step_toward(from: ProjectileCoord, target: ProjectileCoord, speed: i32) -> ProjectileCoord {
    if speed <= 0 || from == target {
        return from;
    }
    let dx = target.x - from.x;
    let dy = target.y - from.y;
    let dz = target.z - from.z;
    let max_delta = dx.abs().max(dy.abs()).max(dz.abs());
    if max_delta <= speed {
        return target;
    }
    ProjectileCoord::new(
        from.x + ((i64::from(dx) * i64::from(speed)) / i64::from(max_delta)) as i32,
        from.y + ((i64::from(dy) * i64::from(speed)) / i64::from(max_delta)) as i32,
        from.z + ((i64::from(dz) * i64::from(speed)) / i64::from(max_delta)) as i32,
    )
}

// YR BulletClass::Detonate linkage: only this handoff permits combat damage/effects.
fn detonation(
    projectile: &Projectile,
    impact: ProjectileCoord,
    reason: ProjectileDetonationReason,
) -> ProjectileDetonation {
    ProjectileDetonation {
        projectile_id: projectile.id,
        source_id: projectile.source_id,
        target: projectile.target,
        impact,
        payload: projectile.payload,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn(target: ProjectileTarget) -> ProjectileSpawn {
        ProjectileSpawn {
            source_id: 7,
            origin: ProjectileCoord::new(0, 0, 0),
            target,
            initial_target_position: match target {
                ProjectileTarget::Entity(_) => ProjectileCoord::new(128, 0, 0),
                ProjectileTarget::Cell(position) => position,
            },
            payload: ProjectilePayload {
                base_damage: 40,
                warhead: InternedId::from_index(3),
                weapon: InternedId::from_index(4),
                owner: InternedId::from_index(5),
            },
            speed_leptons_per_frame: 64,
            arm_frames: 0,
            fuse_frames: None,
            tracks_target: true,
            target_expiry: TargetExpiryPolicy::DetonateAtLastKnown,
            collision: ProjectileCollisionPolicy::NONE,
        }
    }

    #[test]
    fn advance_preserves_stable_creation_order_and_delays_new_projectiles() {
        let mut store = ProjectileStore::new();
        let first = store.spawn(spawn(ProjectileTarget::Cell(ProjectileCoord::new(
            64, 0, 0,
        ))));
        let second = store.spawn(spawn(ProjectileTarget::Cell(ProjectileCoord::new(
            128, 0, 0,
        ))));

        let result = store.advance(&BTreeMap::new(), |_, _| false);

        assert_eq!(
            result
                .detonations
                .iter()
                .map(|detonation| detonation.projectile_id)
                .collect::<Vec<_>>(),
            vec![first]
        );
        assert!(store.get(first).is_none());
        assert_eq!(store.get(second).unwrap().position.x, 64);
    }

    #[test]
    fn homing_projectile_uses_current_target_position() {
        let mut store = ProjectileStore::new();
        let id = store.spawn(spawn(ProjectileTarget::Entity(42)));
        let targets = BTreeMap::from([(42, ProjectileCoord::new(128, 128, 0))]);

        store.advance(&targets, |_, _| false);

        assert_eq!(
            store.get(id).unwrap().position,
            ProjectileCoord::new(64, 64, 0)
        );
    }

    #[test]
    fn target_expiry_detonates_at_last_known_position() {
        let mut store = ProjectileStore::new();
        let id = store.spawn(spawn(ProjectileTarget::Entity(42)));
        let targets = BTreeMap::from([(42, ProjectileCoord::new(128, 0, 0))]);
        store.advance(&targets, |_, _| false);

        let result = store.advance(&BTreeMap::new(), |_, _| false);

        assert_eq!(result.detonations.len(), 1);
        assert_eq!(result.detonations[0].projectile_id, id);
        assert_eq!(
            result.detonations[0].impact,
            ProjectileCoord::new(128, 0, 0)
        );
        assert_eq!(
            result.detonations[0].reason,
            ProjectileDetonationReason::TargetExpired
        );
    }

    #[test]
    fn fuse_and_collision_are_deferred_detonations() {
        let mut store = ProjectileStore::new();
        let mut fused = spawn(ProjectileTarget::Cell(ProjectileCoord::new(256, 0, 0)));
        fused.fuse_frames = Some(0);
        let fuse_id = store.spawn(fused);
        let collision_id = store.spawn(spawn(ProjectileTarget::Cell(ProjectileCoord::new(
            256, 0, 0,
        ))));

        let result = store.advance(&BTreeMap::new(), |_, coord| coord.x == 64);

        assert_eq!(result.detonations.len(), 2);
        assert_eq!(result.detonations[0].projectile_id, fuse_id);
        assert_eq!(
            result.detonations[0].reason,
            ProjectileDetonationReason::Fuse
        );
        assert_eq!(result.detonations[1].projectile_id, collision_id);
        assert_eq!(
            result.detonations[1].reason,
            ProjectileDetonationReason::Collision
        );
    }

    #[test]
    fn store_round_trips_through_snapshot_serialization() {
        let mut store = ProjectileStore::new();
        store.spawn(spawn(ProjectileTarget::Cell(ProjectileCoord::new(
            256, 64, 0,
        ))));

        let bytes = bincode::serialize(&store).unwrap();
        let restored: ProjectileStore = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored, store);
    }

    #[test]
    fn simulation_hash_and_save_preserve_pending_projectile() {
        let empty = crate::sim::world::Simulation::new();
        let mut sim = crate::sim::world::Simulation::new();
        sim.projectiles
            .spawn(spawn(ProjectileTarget::Cell(ProjectileCoord::new(
                256, 64, 0,
            ))));
        let expected_hash = sim.state_hash();

        assert_ne!(empty.state_hash(), expected_hash);
        let bytes = crate::sim::snapshot::GameSnapshot::save(&sim, 0, 0, "projectile", 0);
        let restored = crate::sim::snapshot::GameSnapshot::load(&bytes)
            .expect("projectile snapshot should load")
            .sim;

        assert_eq!(restored.state_hash(), expected_hash);
        assert_eq!(restored.projectiles.len(), 1);
    }
}
