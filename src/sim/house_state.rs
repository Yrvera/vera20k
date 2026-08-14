//! Per-player game state — identity + economy.
//!
//! Split from the monolithic HouseClass into purpose-specific systems. This module
//! holds the lightweight core: identity, economy scalars, and defeat/victory flags.
//!
//! Stored in `Simulation.houses: BTreeMap<InternedId, HouseState>` keyed by
//! interned owner name for deterministic iteration (BTreeMap + InternedId give
//! sorted order natively; all peers intern in the same order).

use std::collections::BTreeMap;

use crate::sim::cell_rect::PlayfieldBounds;
use crate::sim::economy::Economy;
use crate::sim::intern::InternedId;
use crate::util::native_x87::{X87Chop53, sqrt_approx_f32};

/// Native per-house AI difficulty index stored by `HouseClass`.
///
/// The discriminants are part of the gameplay contract: stock
/// `AIVirtualPurifiers=4,2,0` is indexed directly in Hard/Normal/Easy order.
#[repr(i32)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum HouseDifficulty {
    Hard = 0,
    Normal = 1,
    Easy = 2,
}

impl Default for HouseDifficulty {
    fn default() -> Self {
        Self::Normal
    }
}

impl HouseDifficulty {
    /// Convert a native HouseClass difficulty value without accepting drifted
    /// or out-of-range values.
    pub const fn from_native(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Hard),
            1 => Some(Self::Normal),
            2 => Some(Self::Easy),
            _ => None,
        }
    }

    /// Exact index into native hardest-first difficulty-control tables.
    pub const fn table_index(self) -> usize {
        self as usize
    }
}

/// Accepted native HouseClass match result whose SavourDelay still owns the
/// scenario's deterministic frame lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum HouseOutcomeKind {
    Victory,
    Defeat,
}

/// Persistent HouseClass result transition.
///
/// The absolute target keeps the remaining SavourDelay frame count stable
/// across save/load. The wall-clock Vox drain that follows `exit_ready` belongs
/// to the app and is deliberately not represented here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct HouseOutcomeState {
    pub kind: HouseOutcomeKind,
    pub savour_until_tick: u64,
    pub exit_ready: bool,
}

/// Per-player game state.
///
/// Created once per player at game start, lives for the duration of the match.
/// Heavy subsystems (power, fog, production queues, AI) remain in their own
/// containers — HouseState holds the lightweight scalars.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct HouseState {
    /// Owner name as interned ID (resolve via interner for display).
    pub name: InternedId,
    /// Stable rules-owned side index. Stock YR uses 0=Allied, 1=Soviet,
    /// 2=Yuri, 3=Civilian, and 4=Mutant.
    pub side_index: u8,
    /// Country interned ID from map INI `Country=` key (e.g., "Americans", "Russians").
    pub country: Option<InternedId>,
    /// Native HouseClass `IsHuman` byte.
    pub is_human: bool,
    /// Native HouseClass `PlayerControl` byte. It differs from `IsHuman` in
    /// scenario modes and participates independently in EventClass admission.
    #[serde(default)]
    pub player_control: bool,
    /// Per-house native difficulty. Human houses retain Normal unless a map or
    /// game-mode initializer explicitly assigns another native value.
    #[serde(default)]
    pub difficulty: HouseDifficulty,
    /// `MultiplayPassive=` from this house's country/house-type rules.
    ///
    /// gamemd keeps this on the house type and reads it back out of the house
    /// during defeat evaluation: a passive house is never tested for defeat and
    /// never counted in the "everyone still alive is allied" game-over scan.
    /// Stock `Neutral` (Civilian) and `Special` (JP) both set it, and they exist
    /// in every skirmish, so without it the alive set can never shrink to one.
    ///
    /// Stamped once at house creation, while a `RuleSet` is still in hand, and
    /// then read straight off the house — `check_defeat` takes
    /// `rules: Option<&RuleSet>` and never has to resolve the country itself. A
    /// house built with no rules available is stamped `false`, the INI default
    /// for `MultiplayPassive=`; there is no runtime fallback, because gamemd has
    /// none.
    ///
    /// Persisted state, and versioned as such: it is an authoritative input to
    /// the win/loss outcome, so a save that dropped it would reload as an
    /// ordinary house and lose the match forever. It is nonetheless left out of
    /// the state hash and the retail multiplayer checksum, which fold the
    /// mutable `HouseClass` bytes — gamemd keeps this one on the house type.
    #[serde(default)]
    pub multiplay_passive: bool,
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
    /// Accepted win/loss transition plus the deterministic SavourDelay target.
    /// App-owned wall waits and audio teardown are intentionally excluded.
    #[serde(default)]
    pub outcome_state: Option<HouseOutcomeState>,
    /// HouseClass map-clear byte folded by the retail multiplayer checksum.
    ///
    /// Defeat/reveal paths set this independently of the win/loss flags, and
    /// shroud restoration can clear it again.
    #[serde(default)]
    pub map_is_clear: bool,
    /// Aggregate active SpySat state for this house.
    ///
    /// This is the edge-trigger authority for whole-map reveal/restoration.
    /// Individual uplinks do not own the transition. The first marked,
    /// non-limbo, non-selling SpySat in house building order decides it:
    /// warp-out clears the latch; otherwise that provider sets it.
    #[serde(default)]
    pub spy_sat_active: bool,
    /// Running count of owned buildings. Updated on spawn/despawn.
    pub owned_building_count: u32,
    /// Running count of owned non-building units. Updated on spawn/despawn.
    pub owned_unit_count: u32,
    /// Initial base location (MCV deploy point or first ConYard).
    pub base_center: Option<(u16, u16)>,
    /// Max tech level for this player. From game options at match start.
    pub tech_level: i32,
    /// Live HouseClass CurrentIQ (+0x24C), used by AI behavior thresholds.
    ///
    /// Named scenario houses read their own `IQ=`. Generated skirmish computer
    /// houses are stamped from `[IQ] MaxIQLevels`; generated human and special
    /// houses retain the native constructor value zero.
    pub current_iq: i32,
    /// Native `AngerStruct` scores keyed by the other house's stable identity.
    ///
    /// gamemd stores an O(N^2) vector in global HouseClass creation order. The
    /// Rust house registry already owns that exact order in
    /// `ScenarioSession::house_order`, so only touched scores live here; enemy
    /// selection still scans session order, never this map's key order.
    #[serde(default)]
    pub grudge_scores: BTreeMap<InternedId, i32>,
    /// House selected by `HouseClass::UpdateAngerNodes`, or native `-1` as None.
    #[serde(default)]
    pub enemy_house: Option<InternedId>,
    /// Edge of the playfield where this house spawns paradrop carriers.
    /// Encoding: 0=N, 1=E, 2=S, 3=W. Launch setup seeds it from the assigned
    /// start anchor; committed structures refresh it through lifecycle authority.
    pub waypoint_edge: u8,
    /// End-of-match score-screen statistics (Kills / Losses / Built columns).
    ///
    /// gamemd keeps the same three quantities on the house: per-house
    /// `UnitsKilled`/`BuildingsKilled` tables that the score screen sums, a
    /// `UnitsLost`/`BuildingsLost` pair, and four "quantity built" counters that
    /// its `Record_Last_Built` step increments once per finished factory item.
    /// Only the totals are player-visible, so the Rust model keeps totals.
    ///
    /// Deliberately NOT serialized and NOT folded into the state hash: they feed
    /// one post-match screen and never a sim decision, and adding them to either
    /// would move a shared schema this slice is not allowed to touch. The
    /// consequence is that a save/load resets them (recorded DRIFT).
    #[serde(skip)]
    pub stats: MatchStatistics,
    /// Per-house wallet/storage/statistics (the authority flip). The wallet stays
    /// the authoritative `HouseState.credits`; `economy.credits` is a per-sweep shim
    /// loaded from / stored to it and is NOT hashed. The statistics
    /// (`spent_credits`/`harvested_credits`/`purifier_count`) ARE serialized + hashed
    /// as of the flip.
    pub economy: Economy,
}

impl HouseState {
    /// Active offline EventClass house-scan eligibility.
    pub const fn event_dispatch_eligible(&self) -> bool {
        self.is_human || self.player_control
    }

    /// Accept a victory and arm its deterministic grace interval.
    ///
    /// gamemd provenance: HouseClass::Flag_To_Win @ `0x004FC9E0` accepts only
    /// while Win/Draw/Lose are clear, sets HasWon, announces victory, and sets
    /// the house timer to `ftol(SavourDelay * 900)`.
    pub(crate) fn flag_to_win(&mut self, current_tick: u64, savour_frames: u64) -> bool {
        if self.has_won || self.has_lost {
            return false;
        }
        self.has_won = true;
        self.outcome_state = Some(HouseOutcomeState {
            kind: HouseOutcomeKind::Victory,
            savour_until_tick: current_tick.saturating_add(savour_frames),
            exit_ready: false,
        });
        true
    }

    /// Accept a defeat, replacing an earlier pending victory when present.
    ///
    /// gamemd provenance: HouseClass::Flag_To_Lose @ `0x004FCBD0` clears
    /// HasWon unconditionally, then (unless already Draw/Lost) sets HasLost,
    /// announces defeat, and re-arms the full SavourDelay interval.
    pub(crate) fn flag_to_lose(&mut self, current_tick: u64, savour_frames: u64) -> bool {
        self.has_won = false;
        if self.has_lost {
            return false;
        }
        self.has_lost = true;
        self.outcome_state = Some(HouseOutcomeState {
            kind: HouseOutcomeKind::Defeat,
            savour_until_tick: current_tick.saturating_add(savour_frames),
            exit_ready: false,
        });
        true
    }

    /// Advance the HouseClass result timer at the late house-update boundary.
    ///
    /// gamemd provenance: HouseClass::Update @ `0x004F8440` keeps the scenario
    /// running until this timer expires, then enters the bounded Vox wait.
    pub(crate) fn advance_outcome_savour(&mut self, current_tick: u64) -> bool {
        let Some(outcome) = self.outcome_state.as_mut() else {
            return false;
        };
        if current_tick >= outcome.savour_until_tick {
            outcome.exit_ready = true;
        }
        outcome.exit_ready
    }

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
            player_control: is_human,
            difficulty: HouseDifficulty::Normal,
            multiplay_passive: false,
            credits,
            rally_point: None,
            is_defeated: false,
            has_won: false,
            has_lost: false,
            outcome_state: None,
            map_is_clear: false,
            spy_sat_active: false,
            owned_building_count: 0,
            owned_unit_count: 0,
            base_center: None,
            tech_level,
            current_iq: 0,
            grudge_scores: BTreeMap::new(),
            enemy_house: None,
            waypoint_edge: 0,
            stats: MatchStatistics::default(),
            economy: Economy::default(),
        }
    }
}

/// Post-match statistics accumulated for the end-of-match score screen.
///
/// gamemd sums per-victim-house kill tables into one number for the Kills
/// column, adds its two loss counters for the Losses column, and sums its four
/// per-category built counters for the Built column. Totals are all the screen
/// ever reads, so these are kept as totals.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MatchStatistics {
    /// Non-building enemy objects this house destroyed.
    pub units_killed: u32,
    /// Enemy buildings this house destroyed.
    pub buildings_killed: u32,
    /// Non-building objects of this house that were destroyed.
    pub units_lost: u32,
    /// Buildings of this house that were destroyed.
    pub buildings_lost: u32,
    /// Objects this house finished producing.
    pub built: u32,
    /// Score earned by destroying other houses' objects: the sum of each
    /// victim's point value at the moment it died.
    ///
    /// gamemd keeps ONE score accumulator per house with two large feeders — the
    /// ore-deposit statistic and this kill-points stream — and the score screen
    /// shows their sum. The ore half is the existing hashed
    /// `Economy::harvested_credits`; this is the kill half, split out only so the
    /// hashed accumulator is not disturbed. Always read the two together through
    /// [`MatchStatistics::score`].
    pub score_points: i32,
}

impl MatchStatistics {
    /// Score-screen Kills column: units + buildings destroyed.
    pub const fn kills(&self) -> u32 {
        self.units_killed + self.buildings_killed
    }

    /// Score-screen Losses column: units + buildings lost.
    pub const fn losses(&self) -> u32 {
        self.units_lost + self.buildings_lost
    }

    /// Score-screen Score column: the house's single native score accumulator,
    /// reassembled from its harvest and kill feeders.
    pub const fn score(&self, harvested_credits: i32) -> i32 {
        harvested_credits.saturating_add(self.score_points)
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

/// Resolve an owner's ore-income `IncomeMult` (parts-per-million; 1_000_000 = 1.0×) by
/// routing through its country: `HouseState.country` (InternedId) -> country name ->
/// `RuleSet::country_income_ppm`. An owner with no house, no country, or an unknown
/// country resolves to the neutral 1.0 (no income change) — so stock YR (all countries
/// 1.0, the key commented out) is the identity.
pub fn income_ppm_for_owner(
    houses: &std::collections::BTreeMap<InternedId, HouseState>,
    interner: &crate::sim::intern::StringInterner,
    rules: &crate::rules::ruleset::RuleSet,
    owner: &str,
) -> i64 {
    house_state_for_owner(houses, owner, interner)
        .and_then(|h| h.country)
        .map(|c| rules.country_income_ppm(interner.resolve(c)))
        .unwrap_or(crate::sim::economy::INCOME_PPM_SCALE)
}

/// Map side name string to numeric index.
/// "Allies"/"GDI" → 0, "Soviet"/"Nod" → 1, "ThirdSide"/"YuriCountry" → 2.
pub fn side_index_from_name(side: Option<&str>) -> u8 {
    side_index_alias(side).unwrap_or(0)
}

fn side_index_alias(side: Option<&str>) -> Option<u8> {
    match side.map(|s| s.to_ascii_lowercase()).as_deref() {
        Some("allied" | "allies" | "gdi") => Some(0),
        Some("soviet" | "nod" | "russia") => Some(1),
        Some("thirdside" | "yuricountry" | "yuri") => Some(2),
        _ => None,
    }
}

/// Resolve the side identity used to construct a house.
///
/// Rules-owned country membership is authoritative. An explicit side name is
/// the next-best source for incomplete scenario data, followed by the legacy
/// stock aliases and finally the caller's bounded fallback.
pub fn resolve_house_side_index(
    rules: &crate::rules::ruleset::RuleSet,
    country: Option<&str>,
    side: Option<&str>,
    fallback: u8,
) -> u8 {
    country
        .and_then(|country| rules.country_side_index(country))
        .or_else(|| side.and_then(|side| rules.side_index(side)))
        .map(|index| index.0)
        .or_else(|| side_index_alias(side))
        .unwrap_or(fallback)
}

/// The house type an absent `Country=` binds to.
///
/// gamemd's `[Houses]` reader asks the INI for `Country=` with a default of -1
/// and maps a -1 result to 0, so a house section with no `Country=` key binds to
/// the first `[Countries]` entry — stock `Americans`, which is not
/// MultiplayPassive. It does NOT fall back to the house's own section name.
const ABSENT_COUNTRY_IDX: crate::rules::ruleset::CountryIdx = crate::rules::ruleset::CountryIdx(0);

/// Resolve a house's `MultiplayPassive` fact from its country/house-type rules.
///
/// A house with no `Country=` resolves through [`ABSENT_COUNTRY_IDX`], matching
/// the native reader. Missing rules, an empty `[Countries]` registry, or an
/// unknown country name resolve to `false` — the INI default for
/// `MultiplayPassive=`.
pub fn resolve_multiplay_passive(
    rules: Option<&crate::rules::ruleset::RuleSet>,
    country: Option<&str>,
) -> bool {
    let Some(rules) = rules else {
        return false;
    };
    let key = match country {
        Some(country) => Some(country),
        None => rules.country_name(ABSENT_COUNTRY_IDX),
    };
    key.is_some_and(|key| rules.country_multiplay_passive(key))
}

/// Resolve a house type's `WallOwner=` permission. Missing data keeps the native
/// constructor default of `true`; an absent `Country=` binds to registry entry zero.
pub fn resolve_wall_owner(
    rules: Option<&crate::rules::ruleset::RuleSet>,
    country: Option<&str>,
) -> bool {
    let Some(rules) = rules else {
        return true;
    };
    let key = match country {
        Some(country) => Some(country),
        None => rules.country_name(ABSENT_COUNTRY_IDX),
    };
    key.map_or(true, |key| rules.country_wall_owner(key))
}

/// Convert a LocalSize-relative coordinate into the native cell-grid frame.
fn local_to_cell(local: (i32, i32), bounds: PlayfieldBounds) -> (i32, i32) {
    let q = local.0.wrapping_add(bounds.off_fc);
    let r = local.1.wrapping_add(bounds.off_100);
    (
        r.wrapping_add(1).wrapping_shr(1).wrapping_add(q),
        bounds.base.wrapping_add(r >> 1).wrapping_sub(q),
    )
}

/// Cell-space distance through the native Sqrt_Approx/Math::ftol pipeline.
fn native_edge_distance(anchor: (u16, u16), reference: (i32, i32)) -> i32 {
    let dx = X87Chop53::load_i32(i32::from(anchor.0 as i16).wrapping_sub(reference.0));
    let dy = X87Chop53::load_i32(i32::from(anchor.1 as i16).wrapping_sub(reference.1));
    let squared = X87Chop53::add(X87Chop53::mul(dx, dx), X87Chop53::mul(dy, dy));
    let root_bits =
        sqrt_approx_f32(squared).expect("playfield edge distance stays in finite f32 range");
    let root =
        X87Chop53::load_f32(root_bits).expect("Sqrt_Approx always returns a finite normal or zero");
    X87Chop53::ftol_i64(root).expect("playfield edge distance fits a signed integer") as i32
}

/// HouseClass-style playfield edge selection for a committed anchor cell.
///
/// The four asymmetric reference points live in the map's LocalSize frame and
/// must be skewed into cell-grid coordinates before comparison. Strictly-better
/// replacement preserves the native N/E/S/W tie order.
pub(crate) fn determine_waypoint_edge(anchor: (u16, u16), bounds: PlayfieldBounds) -> u8 {
    let references = [
        (bounds.off_104 / 2, 1),
        (bounds.off_104, bounds.off_108),
        (bounds.off_104 / 2, bounds.off_108.wrapping_mul(2)),
        (0, bounds.off_108),
    ];
    let mut best_edge = 0u8;
    let mut best_distance = i32::MAX;
    for (edge, local_reference) in references.into_iter().enumerate() {
        let reference = local_to_cell(local_reference, bounds);
        let distance = native_edge_distance(anchor, reference);
        if distance < best_distance {
            best_distance = distance;
            best_edge = edge as u8;
        }
    }
    best_edge
}

#[cfg(test)]
mod outcome_tests {
    use super::{HouseOutcomeKind, HouseState};

    #[test]
    fn gsi_01_04_savour_gates_exact_frame_and_defeat_restarts_pending_victory() {
        let mut house = HouseState::new(Default::default(), 0, None, true, 0, 10);
        assert!(house.flag_to_win(100, 90));
        assert!(!house.advance_outcome_savour(189));
        assert!(house.advance_outcome_savour(190));

        let mut replaced = HouseState::new(Default::default(), 0, None, true, 0, 10);
        assert!(replaced.flag_to_win(100, 90));
        assert!(replaced.flag_to_lose(150, 90));
        assert!(!replaced.has_won);
        assert!(replaced.has_lost);
        assert_eq!(
            replaced.outcome_state.expect("defeat outcome").kind,
            HouseOutcomeKind::Defeat
        );
        assert_eq!(
            replaced
                .outcome_state
                .expect("restarted defeat outcome")
                .savour_until_tick,
            240
        );
        assert!(!replaced.flag_to_win(160, 90));
        assert!(!replaced.flag_to_lose(170, 90));
        assert_eq!(
            replaced
                .outcome_state
                .expect("unchanged defeat outcome")
                .savour_until_tick,
            240
        );
    }
}

#[cfg(test)]
mod difficulty_tests {
    use super::{HouseDifficulty, HouseState};

    #[test]
    fn native_difficulty_values_are_hardest_first() {
        assert_eq!(HouseDifficulty::Hard as i32, 0);
        assert_eq!(HouseDifficulty::Normal as i32, 1);
        assert_eq!(HouseDifficulty::Easy as i32, 2);
        assert_eq!(HouseDifficulty::from_native(0), Some(HouseDifficulty::Hard));
        assert_eq!(
            HouseDifficulty::from_native(1),
            Some(HouseDifficulty::Normal)
        );
        assert_eq!(HouseDifficulty::from_native(2), Some(HouseDifficulty::Easy));
        assert_eq!(HouseDifficulty::from_native(-1), None);
        assert_eq!(HouseDifficulty::from_native(3), None);
    }

    #[test]
    fn new_house_defaults_to_normal_difficulty() {
        let house = HouseState::new(Default::default(), 0, None, false, 0, 10);
        assert_eq!(house.difficulty, HouseDifficulty::Normal);
        assert_eq!(house.current_iq, 0);
    }
}

#[cfg(test)]
mod multiplay_passive_tests {
    use super::resolve_multiplay_passive;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::ruleset::RuleSet;

    fn rules_with_country_order(first: &str, second: &str) -> RuleSet {
        let ini = IniFile::from_str(&format!(
            "[Countries]\n0={first}\n1={second}\n\
             [Americans]\nSide=Allies\n\
             [Neutral]\nSide=Civilian\nMultiplayPassive=true\n"
        ));
        RuleSet::from_ini(&ini).expect("country registry parses")
    }

    #[test]
    fn named_country_resolves_its_own_multiplay_passive() {
        let rules = rules_with_country_order("Americans", "Neutral");
        assert!(resolve_multiplay_passive(Some(&rules), Some("Neutral")));
        assert!(!resolve_multiplay_passive(Some(&rules), Some("Americans")));
        // Case-insensitive, like every other country lookup.
        assert!(resolve_multiplay_passive(Some(&rules), Some("neutral")));
    }

    #[test]
    fn absent_country_binds_to_the_first_countries_entry() {
        // The native `[Houses]` reader defaults a missing `Country=` to -1 and
        // maps -1 to 0, so the house takes the FIRST `[Countries]` entry. It
        // does not fall back to the house's own section name — with `Neutral`
        // sitting at entry 1, a section-name fallback would answer `true` here.
        let americans_first = rules_with_country_order("Americans", "Neutral");
        assert!(!resolve_multiplay_passive(Some(&americans_first), None));

        // Flip the registry order and the same absent key now follows entry 0.
        let neutral_first = rules_with_country_order("Neutral", "Americans");
        assert!(resolve_multiplay_passive(Some(&neutral_first), None));
    }

    #[test]
    fn missing_rules_or_unknown_country_is_not_passive() {
        let rules = rules_with_country_order("Americans", "Neutral");
        assert!(!resolve_multiplay_passive(None, Some("Neutral")));
        assert!(!resolve_multiplay_passive(None, None));
        assert!(!resolve_multiplay_passive(
            Some(&rules),
            Some("Nonexistent")
        ));
    }
}

#[cfg(test)]
mod waypoint_edge_tests {
    use super::*;

    fn square_bounds() -> PlayfieldBounds {
        PlayfieldBounds {
            base: 100,
            off_fc: 0,
            off_100: 0,
            off_104: 100,
            off_108: 100,
        }
    }

    #[test]
    fn transformed_reference_points_select_their_corresponding_edges() {
        let bounds = square_bounds();
        assert_eq!(determine_waypoint_edge((51, 50), bounds), 0);
        assert_eq!(determine_waypoint_edge((150, 50), bounds), 1);
        assert_eq!(determine_waypoint_edge((150, 150), bounds), 2);
        assert_eq!(determine_waypoint_edge((50, 150), bounds), 3);
    }

    #[test]
    fn gsi_04_16_dustbowl_local_size_skew_selects_south() {
        let bounds = PlayfieldBounds {
            base: 70,
            off_fc: 2,
            off_100: 8,
            off_104: 65,
            off_108: 62,
        };
        assert_eq!(local_to_cell((32, 124), bounds), (100, 102));
        assert_eq!(determine_waypoint_edge((69, 115), bounds), 2);
    }
}
