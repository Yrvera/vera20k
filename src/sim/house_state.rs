//! Per-player game state — identity + economy.
//!
//! Split from the monolithic HouseClass into purpose-specific systems. This module
//! holds the lightweight core: identity, economy scalars, and defeat/victory flags.
//!
//! Stored in `Simulation.houses: BTreeMap<InternedId, HouseState>` keyed by
//! interned owner name for deterministic iteration (BTreeMap + InternedId give
//! sorted order natively; all peers intern in the same order).

use crate::sim::economy::Economy;
use crate::sim::intern::InternedId;

/// Per-player game state.
///
/// Created once per player at game start, lives for the duration of the match.
/// Heavy subsystems (power, fog, production queues, AI) remain in their own
/// containers — HouseState holds the lightweight scalars.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct HouseState {
    /// Owner name as interned ID (resolve via interner for display).
    pub name: InternedId,
    /// Side index: 0=Allied, 1=Soviet, 2=Yuri. From HouseDefinition.side.
    pub side_index: u8,
    /// Country interned ID from map INI `Country=` key (e.g., "Americans", "Russians").
    pub country: Option<InternedId>,
    /// True if this house is human-controlled.
    pub is_human: bool,
    /// Current credit balance.
    pub credits: i32,
    /// Rally point for newly produced units (isometric cell coords).
    pub rally_point: Option<(u16, u16)>,
    /// Whether this player has been eliminated.
    pub is_defeated: bool,
    /// Victory flag.
    pub has_won: bool,
    /// Defeat flag. Note: Flag_To_Lose clears HasWon first.
    pub has_lost: bool,
    /// Running count of owned buildings. Updated on spawn/despawn.
    pub owned_building_count: u32,
    /// Running count of owned non-building units. Updated on spawn/despawn.
    pub owned_unit_count: u32,
    /// Initial base location (MCV deploy point or first ConYard).
    pub base_center: Option<(u16, u16)>,
    /// Max tech level for this player. From game options at match start.
    pub tech_level: i32,
    /// Edge of the playfield where this house spawns paradrop carriers.
    /// Encoding: 0=N, 1=E, 2=S, 3=W. Computed at game start from base_center
    /// via the closest-edge-of-bounds algorithm.
    pub waypoint_edge: u8,
    /// Per-house wallet/storage/statistics shadow (P1). Mirrors the authoritative
    /// `credits` each tick; non-serialized and non-hashed until the authority flip.
    /// `Economy` carries no serde derive in P1+P2, so this `#[serde(skip)]` field
    /// cannot change the bincode layout or the lockstep state hash.
    #[serde(skip)]
    pub economy: Economy,
}

impl HouseState {
    pub fn new(
        name: InternedId,
        side_index: u8,
        country: Option<InternedId>,
        is_human: bool,
        credits: i32,
        tech_level: i32,
    ) -> Self {
        Self {
            name,
            side_index,
            country,
            is_human,
            credits,
            rally_point: None,
            is_defeated: false,
            has_won: false,
            has_lost: false,
            owned_building_count: 0,
            owned_unit_count: 0,
            base_center: None,
            tech_level,
            waypoint_edge: 0,
            economy: Economy::default(),
        }
    }
}

/// Look up a HouseState by interned owner ID (O(1) BTreeMap lookup).
pub fn house_state_for_owner_id<'a>(
    houses: &'a std::collections::BTreeMap<InternedId, HouseState>,
    owner_id: InternedId,
) -> Option<&'a HouseState> {
    houses.get(&owner_id)
}

/// Mutable version of `house_state_for_owner_id`.
pub fn house_state_for_owner_id_mut<'a>(
    houses: &'a mut std::collections::BTreeMap<InternedId, HouseState>,
    owner_id: InternedId,
) -> Option<&'a mut HouseState> {
    houses.get_mut(&owner_id)
}

/// Look up a HouseState by owner name string (case-insensitive).
/// Requires the interner to convert the name to an InternedId first.
/// Returns None if the name is not interned or no house matches.
pub fn house_state_for_owner<'a>(
    houses: &'a std::collections::BTreeMap<InternedId, HouseState>,
    owner: &str,
    interner: &crate::sim::intern::StringInterner,
) -> Option<&'a HouseState> {
    let id = interner.get(owner)?;
    houses.get(&id)
}

/// Mutable version of `house_state_for_owner`.
pub fn house_state_for_owner_mut<'a>(
    houses: &'a mut std::collections::BTreeMap<InternedId, HouseState>,
    owner: &str,
    interner: &crate::sim::intern::StringInterner,
) -> Option<&'a mut HouseState> {
    let id = interner.get(owner)?;
    houses.get_mut(&id)
}

/// Map side name string to numeric index.
/// "Allies"/"GDI" → 0, "Soviet"/"Nod" → 1, "ThirdSide"/"YuriCountry" → 2.
pub fn side_index_from_name(side: Option<&str>) -> u8 {
    match side.map(|s| s.to_ascii_lowercase()).as_deref() {
        Some("allied" | "allies" | "gdi") => 0,
        Some("soviet" | "nod" | "russia") => 1,
        Some("thirdside" | "yuricountry" | "yuri") => 2,
        _ => 0, // default to Allied
    }
}

/// Compute the closest map edge to a given anchor cell.
///
/// Picks the minimum-distance edge from 4 reference points: top-edge midpoint,
/// bottom-right corner, south extension midpoint, and left-edge midpoint.
/// Encoding: 0=N, 1=E, 2=S, 3=W.
pub fn closest_edge_for(anchor: (u16, u16), map_width: u32, map_height: u32) -> u8 {
    let (ax, ay) = (anchor.0 as i64, anchor.1 as i64);
    let w = map_width as i64;
    let h = map_height as i64;

    let refs: [(i64, i64); 4] = [
        (w / 2, 1),     // 0: north — top edge midpoint
        (w, h),         // 1: east  — bottom-right corner-ish
        (w / 2, h * 2), // 2: south — south extension midpoint
        (0, h),         // 3: west  — left edge midpoint
    ];
    let mut best_edge = 0u8;
    let mut best_dsq = i64::MAX;
    for (i, &(rx, ry)) in refs.iter().enumerate() {
        let dx = ax - rx;
        let dy = ay - ry;
        let dsq = dx * dx + dy * dy;
        if dsq < best_dsq {
            best_dsq = dsq;
            best_edge = i as u8;
        }
    }
    best_edge
}

#[cfg(test)]
mod waypoint_edge_tests {
    use super::*;

    #[test]
    fn test_closest_edge_top_center_picks_north() {
        let edge = closest_edge_for((50, 5), 100, 100);
        assert_eq!(edge, 0);
    }

    #[test]
    fn test_closest_edge_left_middle_picks_west() {
        let edge = closest_edge_for((2, 50), 100, 100);
        assert_eq!(edge, 3);
    }

    #[test]
    fn test_closest_edge_bottom_right_picks_east() {
        let edge = closest_edge_for((95, 95), 100, 100);
        assert_eq!(edge, 1);
    }
}
