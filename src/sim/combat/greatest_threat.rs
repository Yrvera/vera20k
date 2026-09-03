//! `TechnoClass::Greatest_Threat @ 0x006F8DF0` — how an object picks what to
//! shoot when nobody told it to.
//!
//! This is the passive-acquisition scanner: the expanding-ring cell walk, the
//! one-candidate-per-cell rule, and the weighted score that ranks what the walk
//! finds. It is reached from `TechnoClass::AI_Update @ 0x006F9E50` through the
//! passive driver at `0x00709820` (vtable `+0x39C`) and the per-class `+0x3C4`
//! wrappers (`UnitClass @ 0x00743190`, `BuildingClass @ 0x00445F00`,
//! `FootClass @ 0x004D9920`).
//!
//! ## Shape of the native scan
//!
//! 1. Radius. `TechnoClass::Threat_Range @ 0x00707E60` (vtable `+0x31C`) turns
//!    the mission's mask into a lepton radius — that part lives in
//!    [`super::threat_range`]. A zero radius means "no cutoff of the scan's
//!    own", and the *walk* is then bounded by
//!    `wider weapon range + 1 + AirRangeBonus` cells (`0x006F90C8..0x006F9148`).
//! 2. Airborne pre-pass. An attacker with an AA projectile sweeps the airborne
//!    bucket grid first (`0x006F9169`), evaluating **every** aircraft in reach.
//!    Aircraft are therefore scored before any ground cell and win ties.
//! 3. Ring walk. `for r in 0..radius`: a row pass over `(cx-r..=cx+r, cy-r)`
//!    then `(…, cy+r)`, then a column pass over `(cx-r, cy+1-r..cy+r-1)` then
//!    `(cx+r, …)`. At `r == 0` the centre cell is visited twice — that is the
//!    literal loop, and it is harmless because the second visit re-picks the
//!    same candidate at the same score.
//! 4. Each cell contributes **at most one** candidate:
//!    `TechnoClass::Scan_Cell_For_Target @ 0x006F8960` walks a single object
//!    list — the bridge-deck list `CellClass+0xE8` when it is non-empty, else
//!    the ground list `CellClass+0xE4` — and stops at the first hostile entry.
//!    If that one entry is then rejected by the candidate gate, the cell yields
//!    nothing; the objects behind it are never looked at.
//! 5. The best is kept only on a **strictly greater** score (init −1), so ties
//!    go to whatever the walk reached first.
//! 6. After each ring, `if (best && (r == radius/4 || r == radius/2)) return`
//!    (`0x006F94F1`). A close-enough target ends the scan early — this is why a
//!    defence shoots the attacker at its feet rather than the juicier one three
//!    cells further out.
//!
//! ## The score
//!
//! `TechnoClass::Calculate_Threat_Score @ 0x0070CD10` is a weighted sum around
//! a `100000.0` base, truncated to an integer by the caller. Five coefficients
//! select between two sets on the scorer house's `HouseClass+0x1FB` byte
//! (`0x0070CD4E`): clear takes the `[General] Dumb*Coefficient` set, set takes
//! the scorer TYPE's own `MyEffectivenessCoefficient=` family. The byte is set
//! by `HouseClass::Constructor @ 0x004F644E` for any house built from a
//! `HouseTypeClass`, which is every house `ScenarioClass::Create_Houses @
//! 0x00687F10` makes, and nothing ever clears it — so the per-type set is live
//! from the first frame of the match. See [`HOUSE_SELECTS_OWN_COEFFICIENTS`].
//!
//! ## Shroud
//!
//! There is no shroud gate here, and the gate VERA used to apply
//! (`FogState::is_cell_visible` on the candidate's cell) has been removed.
//! `Evaluate_Candidate` was read end to end: its only visibility arms are the
//! cloak/sensor test at `0x006F7DA9` and the "discovered by the local player"
//! test at `0x006F81B8`, and the latter is behind `g_GameMode == 0` — campaign
//! only, since a skirmish runs mode 5. Retail therefore lets a skirmish unit
//! acquire an enemy standing in unexplored ground, and refuses the SHOT
//! instead (`GetFireError` returns the shrouded code, which the passive driver
//! at `0x00709820` turns into a target drop on the next cadence).
//!
//! RESIDUAL — VERA has the acquire half of that and not the drop half.
//! - Trigger: an enemy inside weapon range but in a cell this house has not
//!   explored. Common for the long-range artillery, whose `Sight=` is far
//!   *below* its weapon range: `[V3]` sees 7 and shoots 18, `[DRED]` sees 7 and
//!   shoots 25, and `HowitzerGun` reaches 12. Those types scan well past their
//!   own sight on every cadence.
//! - Player effect: small even so. Native drops the target on
//!   `GetFireError == 6` and then re-runs this same scan **inside the same
//!   call**, so it re-picks the same shrouded candidate; the end state is a
//!   unit holding a target it cannot fire on in both engines. What VERA loses
//!   is the one-cadence flicker, not the choice.
//! - Frequency: every artillery scan whose radius crosses unexplored ground.
//! - Downstream risk: closing it belongs with the passive driver's stale-target
//!   check, which needs VERA's fire-error query to report the shrouded case.
//!
//! ## Other residuals
//!
//! - The garrisoned-building auto-acquire scan in `combat/mod.rs` still uses
//!   the retired nearest-first key, and still carries the invented
//!   `FogState::is_cell_visible` gate (`combat/mod.rs:5388`) that this scan no
//!   longer has — so "the fog gate is gone" is true of passive acquisition and
//!   not of the garrison path. It is a separate caller with its own
//!   `OccupyWeapon` selection ladder; folding it into this walk is follow-up
//!   work. Trigger: an occupied civilian building choosing among several
//!   enemies, or one standing in unexplored ground. Frequency: garrison maps
//!   only.
//! - The `DistributedFire=` spread-fire assignment (`FUN_00709550`) and the
//!   AI-only ore-cell fallback (`TechnoClass::Cell_Threat_Fallback @
//!   0x006F8C10`, which returns 0 for every human-controlled house) are not
//!   represented. Neither is reachable for a human house today.
//! - The class-bit mask native derives from the attacker's projectile flags
//!   (`0x00772A90`, AA → `4`, AG → `0xB8`) is not modelled as a mask. It
//!   resolves to "an AG weapon may take ground classes, an AA weapon may take
//!   aircraft", which `select_weapon_for_target` already enforces per
//!   candidate, so the outcome matches without the bit word.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/, map/ and sim/ only.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::BTreeMap;

use super::combat_targeting::AttackerSnapshot;
use super::combat_weapon::{
    attacker_facts, attacker_facts_from_snapshot, is_ally_by_object, is_armed,
    select_weapon_for_target, techno_target_facts,
};
use super::threat_range::{ScanRange, max_weapon_range, scan_range};
use super::{armor_index, is_within_range_leptons, lepton_distance_sq_raw};
use crate::map::entities::EntityCategory;
use crate::map::houses::HouseAllianceMap;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::object_type::ObjectType;
use crate::rules::ruleset::RuleSet;
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::GameEntity;
use crate::sim::intern::StringInterner;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::occupancy::{CellListInsertion, OccupancyGrid, cell_list_layer_for_entity};
use crate::sim::vision::FogState;
use crate::util::fixed_math::SimFixed;
use crate::util::native_x87::{NativeF64Bits, X87Chop53, X87Value, sqrt_approx_f32};

/// `Sqrt_Approx` operand base: leptons per cell.
const LEPTONS_PER_CELL: i32 = 256;

/// `TechnoClass::Calculate_Threat_Score`'s additive base, `DAT_007F4E90`.
const THREAT_SCORE_BASE: f64 = 100_000.0;

/// `TechnoClass::Evaluate_Candidate @ 0x006F7D1F`'s Verses floor, `DAT_007F4E38`
/// — a `float` `0.02` widened to double, so the comparison is against
/// `0.019999999552965164` and an authored `Verses=2%` (exactly `0.02` as a
/// double) is *above* it and passes.
const VERSES_FLOOR: f64 = 0.02f32 as f64;

/// The five weights of `TechnoClass::Calculate_Threat_Score @ 0x0070CD10`, in
/// the order the native body loads them.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ThreatCoefficients {
    /// `A` — the scorer's own weapon effectiveness against the candidate.
    pub my_effectiveness: f64,
    /// `B` — the candidate's weapon effectiveness against the scorer, negated
    /// when the candidate is already aiming at the scorer.
    pub target_effectiveness: f64,
    /// `C` — the candidate type's `SpecialThreatValue=`.
    pub target_special_threat: f64,
    /// `D` — the candidate's live health ratio.
    pub target_strength: f64,
    /// `E` — whole cells beyond the scorer's weapon range.
    pub target_distance: f64,
}

impl ThreatCoefficients {
    /// The `HouseClass+0x1FB` branch at `0x0070CD4E`, which reads the byte on
    /// the SCORER's owner (`0x0070CD48: EAX = [EDI+0x21C]`, EDI = `this` = the
    /// attacker) and takes the scorer TYPE's own coefficients when it is set.
    ///
    /// Pass [`HOUSE_SELECTS_OWN_COEFFICIENTS`] for it; the clear branch is kept
    /// because it is the native alternative, not because VERA can reach it.
    pub(crate) fn resolve(
        rules: &RuleSet,
        scorer_type: &ObjectType,
        scorer_house_byte_set: bool,
    ) -> Self {
        let general = &rules.general;
        if scorer_house_byte_set {
            Self {
                my_effectiveness: scorer_type
                    .my_effectiveness_coefficient
                    .unwrap_or(general.my_effectiveness_coefficient_default),
                target_effectiveness: scorer_type
                    .target_effectiveness_coefficient
                    .unwrap_or(general.target_effectiveness_coefficient_default),
                target_special_threat: scorer_type
                    .target_special_threat_coefficient
                    .unwrap_or(general.target_special_threat_coefficient_default),
                target_strength: scorer_type
                    .target_strength_coefficient
                    .unwrap_or(general.target_strength_coefficient_default),
                target_distance: scorer_type
                    .target_distance_coefficient
                    .unwrap_or(general.target_distance_coefficient_default),
            }
        } else {
            Self {
                my_effectiveness: general.dumb_my_effectiveness_coefficient,
                target_effectiveness: general.dumb_target_effectiveness_coefficient,
                target_special_threat: general.dumb_target_special_threat_coefficient,
                target_strength: general.dumb_target_strength_coefficient,
                target_distance: general.dumb_target_distance_coefficient,
            }
        }
    }
}

/// `HouseClass+0x1FB` for every house VERA can create: **set**.
///
/// The byte has three writers in the whole image and none of them clears it
/// after construction:
/// - `HouseClass::Constructor @ 0x004F54A0` zeroes it at `0x004F5740`
///   (`MOV [EBP+0x1FB], BL` with `EBX == 0`) and then sets it at `0x004F644E`
///   (`MOV byte [EBP+0x1FB], 1`) behind `0x004F6448 CMP ESI,EBX / JZ` — ESI is
///   the constructor's `HouseTypeClass*` argument, the same pointer stored to
///   `HouseClass+0x34` and used for the country name copy a few lines later.
/// - `HouseClass::Mark_Has_Buildings @ 0x00509130` (`*(byte*)(this+0x1FB) = 1`
///   and nothing else), called only from `BuildingClass::Limbo @ 0x00445DEF`
///   and `BuildingClass::Unlimbo @ 0x00440A17`.
///
/// All four `HouseClass` constructions in `ScenarioClass::Create_Houses @
/// 0x00687F10` (`0x00687FC3`, `0x006881A0`, `0x006882FE`, `0x00688351`) push a
/// `HouseTypeClass*` taken from the type array at `0x00A83C9C` — by country
/// index for the player houses, by name lookup (`0x005117D0`) for the fixed
/// special houses. So the byte is 1 from the frame a house is created, the
/// Limbo/Unlimbo marker only re-sets an already-set byte, and the scorer TYPE's
/// own `MyEffectivenessCoefficient=` family is the live set for the whole
/// match — including the opening minute before the MCV deploys.
///
/// This replaces an earlier "does the house own a live building right now?"
/// derivation, which took the `[General] Dumb*Coefficient` branch at match
/// start and again after a house lost its last structure. Native takes neither.
pub(crate) const HOUSE_SELECTS_OWN_COEFFICIENTS: bool = true;

fn load_threat_double(value: f64) -> Option<X87Value> {
    X87Chop53::load_f64(NativeF64Bits::from_bits(value.to_bits())).ok()
}

fn threat_coord(entity: &GameEntity, terrain: Option<&ResolvedTerrainGrid>) -> (i32, i32, i32) {
    let x = i32::from(entity.position.rx)
        .wrapping_mul(LEPTONS_PER_CELL)
        .wrapping_add(entity.position.sub_x.to_num::<i32>());
    let y = i32::from(entity.position.ry)
        .wrapping_mul(LEPTONS_PER_CELL)
        .wrapping_add(entity.position.sub_y.to_num::<i32>());
    let z = terrain
        .and_then(|terrain| super::in_range::effective_z_leptons(entity, terrain))
        .and_then(|z| i32::try_from(z).ok())
        .unwrap_or_else(|| {
            i32::from(entity.position.z)
                .wrapping_mul(crate::util::lepton::LEPTONS_PER_LEVEL as i32)
                .wrapping_add(
                    entity
                        .locomotor
                        .as_ref()
                        .map(|locomotor| locomotor.altitude.to_num::<i32>())
                        .unwrap_or(0),
                )
        });
    (x, y, z)
}

/// `TechnoClass::Calculate_Threat_Score @ 0x0070CD10`, on the `&NullCoord`
/// branch every acquisition caller takes (`0x0070D023`) — the distance term is
/// therefore whole CELLS, `ftol(Sqrt_Approx(3-D lepton delta)) >> 8`, not
/// leptons.
///
/// Term order and the per-term 64-bit store are the native ones: every
/// intermediate is written back to the `[ESP+0x10]` double between terms, and
/// only the final `beyond*E + acc + 100000` is evaluated on the x87 stack.
///
/// Not modelled: the `Rules.EnemyHouseThreatBonus` term at `0x0070CF13`, which
/// is added when the scorer house's `+0x5600` "current enemy" index names the
/// candidate's owner. `+0x5600` is written only by `HouseClass::UpdateAngerNodes
/// @ 0x00504842`, an AI-strategy routine VERA does not run, so the term is
/// unreachable until an AI house ships. Recorded, not approximated.
#[allow(clippy::too_many_arguments)]
pub(crate) fn calculate_threat_score(
    entities: &EntityStore,
    scorer_id: u64,
    candidate_id: u64,
    rules: &RuleSet,
    interner: &StringInterner,
    terrain: Option<&ResolvedTerrainGrid>,
    alliances: Option<&HouseAllianceMap>,
    coefficients: ThreatCoefficients,
) -> Option<X87Value> {
    let scorer = entities.get(scorer_id)?;
    let candidate = entities.get(candidate_id)?;
    let scorer_type = rules.object(interner.resolve(scorer.type_ref))?;
    let candidate_type = rules.object(interner.resolve(candidate.type_ref))?;
    let coeff_a = load_threat_double(coefficients.my_effectiveness)?;
    let coeff_b = load_threat_double(coefficients.target_effectiveness)?;
    let coeff_c = load_threat_double(coefficients.target_special_threat)?;
    let coeff_d = load_threat_double(coefficients.target_strength)?;
    let coeff_e = load_threat_double(coefficients.target_distance)?;
    let mut score = X87Chop53::load_i32(0);

    // B: the candidate's selected weapon against the scorer. A candidate
    // already targeting the scorer contributes the negated term (`FCHS` at
    // `0x0070CEB9`).
    let scorer_as_target = techno_target_facts(
        scorer,
        scorer_type,
        terrain,
        is_ally_by_object(alliances, interner, candidate.owner, scorer.owner),
    );
    if let Some(selected) = select_weapon_for_target(
        rules,
        candidate_type,
        &attacker_facts(candidate, candidate_type),
        &scorer_as_target,
    ) {
        let verses =
            load_threat_double(selected.warhead.verses_f64[armor_index(&scorer_type.armor)])?;
        let mut term = X87Chop53::mul(coeff_b, verses);
        if candidate
            .attack_target
            .as_ref()
            .is_some_and(|target| target.target == super::TargetKind::Entity(scorer.stable_id))
        {
            term = X87Chop53::neg(term);
        }
        score = X87Chop53::add(score, term);
    }

    // C: candidate type SpecialThreatValue (`TechnoTypeClass+0x2C0`).
    score = X87Chop53::add(
        score,
        X87Chop53::mul(
            coeff_c,
            load_threat_double(candidate_type.special_threat_value)?,
        ),
    );

    // A: the scorer's selected weapon against the candidate. Retain the
    // selected weapon for the native range term below.
    let candidate_as_target = techno_target_facts(
        candidate,
        candidate_type,
        terrain,
        is_ally_by_object(alliances, interner, scorer.owner, candidate.owner),
    );
    let selected_scorer_weapon = select_weapon_for_target(
        rules,
        scorer_type,
        &attacker_facts(scorer, scorer_type),
        &candidate_as_target,
    );
    if let Some(selected) = selected_scorer_weapon.as_ref() {
        let verses =
            load_threat_double(selected.warhead.verses_f64[armor_index(&candidate_type.armor)])?;
        score = X87Chop53::add(score, X87Chop53::mul(coeff_a, verses));
    }

    // D: live candidate health ratio (`ObjectClass::GetHealthRatio @ 0x005F5C60`).
    let health_ratio = if candidate.health.max == 0 {
        X87Chop53::load_i32(0)
    } else {
        X87Chop53::div(
            X87Chop53::load_i32(i32::from(candidate.health.current)),
            X87Chop53::load_i32(i32::from(candidate.health.max)),
        )
        .ok()?
    };
    score = X87Chop53::add(score, X87Chop53::mul(coeff_d, health_ratio));

    // E: whole cells beyond the scorer's selected weapon range. With no weapon
    // selected native falls back to the scorer type's `GuardRange`
    // (`TechnoTypeClass+0x5B8`, `0x0070CF81`) — NOT `Sight`.
    let scorer_coord = threat_coord(scorer, terrain);
    let candidate_coord = threat_coord(candidate, terrain);
    let dx = X87Chop53::load_i32(scorer_coord.0.wrapping_sub(candidate_coord.0));
    let dy = X87Chop53::load_i32(scorer_coord.1.wrapping_sub(candidate_coord.1));
    let dz = X87Chop53::load_i32(scorer_coord.2.wrapping_sub(candidate_coord.2));
    let distance_sq = X87Chop53::add(
        X87Chop53::add(X87Chop53::mul(dx, dx), X87Chop53::mul(dy, dy)),
        X87Chop53::mul(dz, dz),
    );
    let distance_root = X87Chop53::load_f32(sqrt_approx_f32(distance_sq).ok()?).ok()?;
    let distance_leptons = i32::try_from(X87Chop53::ftol_i64(distance_root).ok()?).ok()?;
    let distance_cells = crate::util::direction_tables::lepton_to_cell(distance_leptons);
    let range_cells = selected_scorer_weapon.as_ref().map_or_else(
        || {
            scorer_type
                .guard_range
                .map_or(0, |range| range.to_num::<i32>())
        },
        |selected| selected.weapon.range.to_num::<i32>(),
    );
    let beyond_range = distance_cells.wrapping_sub(range_cells).max(0);
    score = X87Chop53::add(
        X87Chop53::mul(X87Chop53::load_i32(beyond_range), coeff_e),
        score,
    );
    Some(X87Chop53::add(
        score,
        load_threat_double(THREAT_SCORE_BASE)?,
    ))
}

/// `TechnoClass::Evaluate_Candidate`'s scoring tail: the single truncating
/// `ftol` at `0x006F86A9`, then the zero/negative handling at `0x006F8930`.
///
/// A score of exactly zero is a REJECTION, not a low-ranked accept; anything
/// negative is clamped up to 1 and accepted.
///
/// Not modelled here: the `VHPScan=` adjustments (`0x006F86B4`, gate `G3` at
/// `0x006F7D07`), which halve or double the score from the candidate's
/// `EstimatedHealth` bookkeeping (`TechnoClass+0x70`, debited by the passive
/// driver at `0x00709820`). VERA carries no `EstimatedHealth` field.
/// - Trigger: an attacker whose TYPE authors `VHPScan=`. Stock `rulesmd.ini`
///   has exactly one — `[NASAM] VHPScan=Strong` (the Patriot missile site).
/// - Player effect: a SAM site would skip candidates other SAM sites have
///   already committed lethal damage to, instead of piling on.
/// - Frequency: only among several SAM sites firing at one aircraft.
/// - Downstream risk: the field is written by the acquisition driver and read
///   by every other object's scan, so it is shared targeting state; it belongs
///   with the driver port, not here.
fn finish_score(score: X87Value) -> Option<i32> {
    let truncated = i32::try_from(X87Chop53::ftol_i64(score).ok()?).ok()?;
    if truncated == 0 {
        return None;
    }
    Some(truncated.max(1))
}

/// The cells of one Chebyshev ring, in `Greatest_Threat`'s literal loop order:
/// the north row west-to-east interleaved with the south row, then the west
/// column north-to-south interleaved with the east column.
///
/// `r == 0` yields the centre twice, exactly as the native `do/while` does.
fn ring_cells(cx: i32, cy: i32, r: i32) -> Vec<(i32, i32)> {
    let mut cells = Vec::with_capacity((8 * r.max(1) + 2) as usize);
    // Row pass: `for (dx = -r; dx <= r; dx++)`.
    for dx in -r..=r {
        cells.push((cx + dx, cy - r));
        cells.push((cx + dx, cy + r));
    }
    // Column pass: `for (dy = 1 - r; dy < r; dy++)` — skipped entirely at r = 0.
    let mut dy = 1 - r;
    while dy < r {
        cells.push((cx - r, cy + dy));
        cells.push((cx + r, cy + dy));
        dy += 1;
    }
    cells
}

/// The ring index at which the native walk returns early once it holds a
/// candidate (`0x006F94D0..0x006F94F1`).
fn is_early_return_ring(ring: i32, radius: i32) -> bool {
    ring == radius / 4 || ring == radius / 2
}

/// Walk bound in cells, `iStack_34` in `Greatest_Threat`.
///
/// A non-zero `Threat_Range` result is itself the bound (`range >> 8`). A zero
/// one — plain Guard on a type with no `GuardRange=` — bounds the walk at
/// `wider weapon range + 1 + AirRangeBonus` cells instead, which is a SEARCH
/// bound, deliberately wider than the acceptance the candidate gate applies.
fn scan_radius_cells(rules: &RuleSet, obj: &ObjectType, veterancy: u16, range: ScanRange) -> i32 {
    match range {
        ScanRange::Hard(cells) => cells.to_num::<i32>(),
        ScanRange::CanFireAt => {
            let weapon_cells = max_weapon_range(rules, obj, veterancy).to_num::<i32>();
            let air_bonus_cells = obj.air_range_bonus.map_or(0, |bonus| bonus.to_num::<i32>());
            weapon_cells + 1 + air_bonus_cells
        }
    }
}

/// One cell's selected object list plus the airborne objects over it.
///
/// The cell lists are built through [`OccupancyGrid`] itself, so the in-cell
/// order is the one the maintained index already models: non-buildings
/// prepended, buildings appended, in `occupancy_enter_order`. That is native's
/// `CellClass+0xE4`/`+0xE8` insertion contract.
///
/// It is rebuilt per scan rather than read from `ObjectSubstrate::occupancy`
/// because that grid's membership is the `Mark` transaction
/// (`GameEntity::lifecycle.cell_marked`), which is a different question from
/// "which objects may this scan see" — the membership this site has always used
/// is live/represented/not-in-a-transport. Changing membership is a separate
/// mechanism from changing the walk order, so the order is reused and the
/// membership is not. Using the maintained grid directly (and dropping the
/// rebuild) is the follow-up once acquisition and `Mark` agree.
///
/// RESIDUAL — cost at charter scale. Native reaches a candidate through
/// `CellClass+0xE4`, a list the map maintains, so one scan touches only the
/// cells its rings walk. [`ScanIndex::build`] instead makes one pass over
/// **every** live entity to find the ones inside the scan's bounding box, then
/// sorts that box subset and allocates a fresh [`OccupancyGrid`] for it. So per
/// scan the honest shape is `O(N)` over all entities plus `O(K log K)` and one
/// allocation over the `K` inside the box — the same `O(N)` order as the
/// `EntityStore::values()` walk this replaced, with an added sort and
/// allocation on the small `K`, and no longer doubled now that the coefficient
/// set is a constant instead of a second full pass.
/// - Trigger: every passive acquisition, i.e. every object on Guard/Move/
///   Harvest reaching its scan cadence.
/// - Player effect: none — this is frame time, not a behavioural difference.
/// - Frequency: at the charter's 20,000 objects and the stock scan cadence,
///   roughly 700 scans per frame, each walking all 20,000 entities — about
///   1.4e7 entity visits per frame, against native's near-zero.
/// - Downstream risk: the fix is a per-tick shared index, and it cannot simply
///   be cached across scans within a tick because objects die, spawn and move
///   between scans in the same tick; it needs the membership question settled
///   with `Mark` first, which is why it is deferred rather than patched here.
struct ScanIndex {
    cells: OccupancyGrid,
    /// Objects with no cell-list layer at all — airborne aircraft and anything
    /// else off the ground lists. Native keeps these in the 20x20 airborne
    /// bucket grid that the pre-pass at `0x006F9169` sweeps, never in
    /// `CellClass+0xE4`.
    airborne: BTreeMap<(u16, u16), Vec<u64>>,
}

impl ScanIndex {
    fn build(entities: &EntityStore, min: (i32, i32), max: (i32, i32)) -> Self {
        let mut ordered: Vec<&GameEntity> = entities
            .values()
            .filter(|entity| {
                // Native cell-list membership: a dead, limboed or carried
                // object has been unmarked from its cell and is not walked.
                entity.health.current > 0
                    && !entity.dying
                    && !entity.lifecycle.in_limbo
                    && !entity.passenger_role.is_inside_transport()
            })
            .filter(|entity| {
                let x = i32::from(entity.position.rx);
                let y = i32::from(entity.position.ry);
                // Buildings are indexed on every foundation cell, so admit them
                // by their own origin plus the widest stock foundation.
                let slack = if entity.category == EntityCategory::Structure {
                    STRUCTURE_FOOTPRINT_SLACK_CELLS
                } else {
                    0
                };
                x + slack >= min.0 && x - slack <= max.0 && y + slack >= min.1 && y - slack <= max.1
            })
            .collect();
        ordered.sort_by_key(|entity| (entity.occupancy_enter_order, entity.stable_id));

        let mut cells = OccupancyGrid::new();
        let mut airborne: BTreeMap<(u16, u16), Vec<u64>> = BTreeMap::new();
        for entity in ordered {
            let sid = entity.stable_id;
            let Some(layer) = cell_list_layer_for_entity(entity) else {
                airborne
                    .entry((entity.position.rx, entity.position.ry))
                    .or_default()
                    .push(sid);
                continue;
            };
            let sub = if entity.category == EntityCategory::Infantry {
                entity.sub_cell
            } else {
                None
            };
            let insertion = CellListInsertion::from_category(entity.category);
            for (rx, ry) in crate::sim::occupancy::entity_occupancy_cells(entity) {
                cells.add(rx, ry, sid, layer, sub, insertion);
            }
        }
        Self { cells, airborne }
    }

    /// `Scan_Cell_For_Target @ 0x006F89A6`: the bridge-deck list wins whenever
    /// it exists; only one list is ever walked.
    fn cell_list(
        &self,
        rx: u16,
        ry: u16,
    ) -> Option<(&crate::sim::occupancy::CellOccupancy, MovementLayer)> {
        let occupancy = self.cells.get(rx, ry)?;
        let layer = if occupancy.is_empty_on(MovementLayer::Bridge) {
            MovementLayer::Ground
        } else {
            MovementLayer::Bridge
        };
        Some((occupancy, layer))
    }
}

/// Widest stock building foundation, used only to decide which entities are
/// worth indexing for a bounded scan.
const STRUCTURE_FOOTPRINT_SLACK_CELLS: i32 = 8;

/// Everything one scan needs that does not change between candidates.
struct ScanContext<'a> {
    entities: &'a EntityStore,
    rules: &'a RuleSet,
    interner: &'a StringInterner,
    attacker: &'a AttackerSnapshot,
    attacker_obj: &'a ObjectType,
    fog: Option<&'a FogState>,
    terrain: Option<&'a ResolvedTerrainGrid>,
    require_playfield_membership: bool,
    range: ScanRange,
    coefficients: ThreatCoefficients,
}

impl ScanContext<'_> {
    fn alliances(&self) -> Option<&HouseAllianceMap> {
        self.fog.map(|fog| &fog.alliances)
    }

    /// `HouseClass::Is_Ally_ByObject` — allied houses and the owner itself.
    fn is_ally(&self, candidate: &GameEntity) -> bool {
        if candidate.owner == self.attacker.owner {
            return true;
        }
        self.fog.is_some_and(|fog| {
            fog.is_friendly(
                self.interner.resolve(self.attacker.owner),
                self.interner.resolve(candidate.owner),
            )
        })
    }
}

/// `TechnoClass::Greatest_Threat @ 0x006F8DF0` — the scan a passive object runs
/// to choose a target. Returns the winning candidate's stable id.
///
/// `scan_range_override` replaces the mission-derived radius with a hard cutoff;
/// it exists for garrisoned buildings, whose reach is foundation-derived.
#[allow(clippy::too_many_arguments)]
pub(crate) fn greatest_threat(
    entities: &EntityStore,
    rules: &RuleSet,
    interner: &StringInterner,
    attacker: &AttackerSnapshot,
    attacker_obj: &ObjectType,
    fog: Option<&FogState>,
    scan_range_override: Option<SimFixed>,
    terrain: Option<&ResolvedTerrainGrid>,
    require_playfield_membership: bool,
) -> Option<u64> {
    let range = match scan_range_override {
        Some(cells) => ScanRange::Hard(cells),
        None => scan_range(
            rules,
            attacker_obj,
            attacker.veterancy,
            attacker.scan_mission,
        ),
    };
    let radius = scan_radius_cells(rules, attacker_obj, attacker.veterancy, range);
    if radius <= 0 {
        // `for (r = 0; r < radius; r++)` never executes.
        return None;
    }

    let ctx = ScanContext {
        entities,
        rules,
        interner,
        attacker,
        attacker_obj,
        fog,
        terrain,
        require_playfield_membership,
        range,
        coefficients: ThreatCoefficients::resolve(
            rules,
            attacker_obj,
            HOUSE_SELECTS_OWN_COEFFICIENTS,
        ),
    };

    let cx = i32::from(attacker.pos_rx);
    let cy = i32::from(attacker.pos_ry);
    let index = ScanIndex::build(
        entities,
        (cx - radius, cy - radius),
        (cx + radius, cy + radius),
    );

    let mut best: Option<u64> = None;
    // `local_50 = -1` — the keep test is strictly greater, so a score of 0 (the
    // lowest an accepted candidate can carry, and itself a rejection) can never
    // displace nothing.
    let mut best_score: i32 = -1;

    // Airborne pre-pass. Native gates it on the attacker carrying an AA
    // projectile (`flags & 4`); here the weapon-selection ladder refuses a
    // ground-only weapon against a flying candidate anyway, so the gate is
    // implicit and the outcome is the same. Every aircraft in reach is scored —
    // there is no one-per-cell rule on this pass — and the ring walk that
    // follows can only displace an aircraft with a strictly better score.
    //
    // DRIFT — sweep order. Native iterates a 20x20 bucket grid
    // (`FUN_00412B40`/`FUN_004137A0`); this sweeps the same ring order the
    // ground walk uses. It selects the same aircraft except when two tie on
    // score, where the winner can differ. Frequency: two identical aircraft at
    // the same range and health, on the same tick.
    for ring in 0..radius {
        for (x, y) in ring_cells(cx, cy, ring) {
            let (Ok(rx), Ok(ry)) = (u16::try_from(x), u16::try_from(y)) else {
                continue;
            };
            let Some(occupants) = index.airborne.get(&(rx, ry)) else {
                continue;
            };
            for &candidate_id in occupants {
                if candidate_id == attacker.stable_id {
                    continue;
                }
                let Some(candidate) = entities.get(candidate_id) else {
                    continue;
                };
                if ctx.is_ally(candidate) {
                    continue;
                }
                let Some(score) = evaluate_candidate(&ctx, candidate) else {
                    continue;
                };
                if score > best_score {
                    best_score = score;
                    best = Some(candidate_id);
                }
            }
        }
    }

    for ring in 0..radius {
        for (x, y) in ring_cells(cx, cy, ring) {
            let (Ok(rx), Ok(ry)) = (u16::try_from(x), u16::try_from(y)) else {
                // `Cell_in_bounds_check @ 0x00568300`.
                continue;
            };
            let Some(candidate_id) = scan_cell_for_target(&ctx, &index, rx, ry) else {
                continue;
            };
            let Some(candidate) = entities.get(candidate_id) else {
                continue;
            };
            let Some(score) = evaluate_candidate(&ctx, candidate) else {
                continue;
            };
            if score > best_score {
                best_score = score;
                best = Some(candidate_id);
            }
        }
        if best.is_some() && is_early_return_ring(ring, radius) {
            return best;
        }
    }
    best
}

/// `TechnoClass::Scan_Cell_For_Target @ 0x006F8960`, narrowed to the armed
/// attacker every reachable caller has.
///
/// The walk stops at the first hostile Techno in the selected list; allied
/// entries are stepped over but do not end the walk, and if the list runs out
/// with only allies the cell contributes nothing (the post-check at
/// `0x006F8909` rejects the ally the walk was left holding). The result is that
/// a cell holding three enemy infantry offers only the list head — if the gate
/// then rejects that one, the other two are never seen.
///
/// Not modelled: the unarmed-attacker arm (`0x006F89EE`, an unarmed object
/// picks a WOUNDED ALLY, which is how a medic-like type finds its patient) and
/// the two infantry specials at `0x006F8A96`. VERA's acquisition entry requires
/// an armed attacker upstream, and neither infantry flag (`InfantryType+0x6D8`,
/// `+0xEC3`) is parsed.
fn scan_cell_for_target(ctx: &ScanContext<'_>, index: &ScanIndex, rx: u16, ry: u16) -> Option<u64> {
    let (occupancy, layer) = index.cell_list(rx, ry)?;
    for occupant in occupancy.iter_layer(layer) {
        if occupant.entity_id == ctx.attacker.stable_id {
            continue;
        }
        let Some(candidate) = ctx.entities.get(occupant.entity_id) else {
            continue;
        };
        if !ctx.is_ally(candidate) {
            return Some(occupant.entity_id);
        }
    }
    None
}

/// `TechnoClass::Evaluate_Candidate @ 0x006F7CA0` — the gate ladder, in native
/// order, followed by the score.
///
/// Returns the candidate's integer threat score, or `None` for a rejection.
/// Gate labels below are the ones used in the mapping of this function; the
/// AI-only flag branches (`G22`, `G23`, `G26`, `P2`–`P7`) are not represented
/// because no flag bit that reaches them is ever set on the passive path, where
/// the flag word is `1 | {AA 0x4, AG 0xB8}`.
fn evaluate_candidate(ctx: &ScanContext<'_>, candidate: &GameEntity) -> Option<i32> {
    // G1/G2/G4 — select the weapon this attacker would use against this
    // candidate, then refuse the candidate when that weapon's Verses against
    // its armor is at or below the `0.02f` floor. `select_weapon_for_target`
    // is the `SelectWeaponAgainst @ 0x006F3330` ladder and already carries the
    // 0% fallback, so a `None` here is native's `FIRE_ILLEGAL`.
    let candidate_obj = ctx.rules.object(ctx.interner.resolve(candidate.type_ref))?;
    let scanner_facts = ctx
        .entities
        .get(ctx.attacker.stable_id)
        .map(|entity| attacker_facts(entity, ctx.attacker_obj))
        .unwrap_or_else(|| attacker_facts_from_snapshot(ctx.attacker, ctx.attacker_obj));
    let candidate_facts = techno_target_facts(
        candidate,
        candidate_obj,
        ctx.terrain,
        is_ally_by_object(
            ctx.alliances(),
            ctx.interner,
            ctx.attacker.owner,
            candidate.owner,
        ),
    );
    let selected = select_weapon_for_target(
        ctx.rules,
        ctx.attacker_obj,
        &scanner_facts,
        &candidate_facts,
    )?;
    if selected.warhead.verses_f64[armor_index(&candidate_obj.armor)] <= VERSES_FLOOR {
        return None;
    }

    // G6 — `InLimbo`/`Health == 0`. Already excluded from the cell index, kept
    // here because the airborne pre-pass and the fixture paths reach this
    // function with candidates the index did not filter.
    if candidate.health.current == 0 || candidate.dying || candidate.lifecycle.in_limbo {
        return None;
    }

    // G7 — the cloak arm at `0x006F7DA9`. A fully cloaked candidate is illegal
    // unless the ATTACKER's house holds a sensor on the candidate's own cell,
    // or the two share an owner. Alliance does not exempt.
    if crate::sim::cloak_disguise::cloak_rejects_candidate(
        candidate
            .cloak
            .as_ref()
            .is_some_and(|cloak| cloak.is_fully_cloaked()),
        ctx.fog.is_some_and(|fog_state| {
            fog_state.has_sensor_for_house(
                ctx.attacker.owner,
                candidate.position.rx,
                candidate.position.ry,
            )
        }),
        candidate.owner == ctx.attacker.owner,
    ) {
        return None;
    }

    // G8 — `TechnoClass+0x3D5`, the represented-in-the-playfield byte.
    if ctx.require_playfield_membership && !candidate.in_playfield {
        return None;
    }

    // G10/G11 — the ally arm and the self test. The cell walk already stopped
    // on the first hostile, but the airborne pre-pass and the retarget callers
    // arrive here directly.
    if candidate.stable_id == ctx.attacker.stable_id || ctx.is_ally(candidate) {
        return None;
    }
    if candidate.passenger_role.is_inside_transport() {
        return None;
    }

    // G13 — distance. A non-zero `Threat_Range` is a hard cutoff; a zero one
    // defers to the attacker's own can-fire-at query, which is the range of the
    // weapon picked against this very candidate.
    let dist_sq = lepton_distance_sq_raw(
        ctx.attacker.pos_rx,
        ctx.attacker.pos_ry,
        ctx.attacker.sub_x,
        ctx.attacker.sub_y,
        candidate.position.rx,
        candidate.position.ry,
        candidate.position.sub_x,
        candidate.position.sub_y,
    );
    let in_range = match ctx.range {
        ScanRange::Hard(cells) => is_within_range_leptons(dist_sq, cells),
        ScanRange::CanFireAt => match (ctx.terrain, ctx.entities.get(ctx.attacker.stable_id)) {
            (Some(terrain), Some(attacker_entity)) => {
                let source_z = super::in_range::effective_z_leptons(attacker_entity, terrain)?;
                let src = (
                    i64::from(ctx.attacker.pos_rx) * i64::from(LEPTONS_PER_CELL)
                        + ctx.attacker.sub_x.to_num::<i64>(),
                    i64::from(ctx.attacker.pos_ry) * i64::from(LEPTONS_PER_CELL)
                        + ctx.attacker.sub_y.to_num::<i64>(),
                    source_z,
                );
                super::in_range::compute_in_range(
                    attacker_entity,
                    src,
                    &super::TargetKind::Entity(candidate.stable_id),
                    selected.weapon,
                    ctx.rules,
                    ctx.interner,
                    ctx.entities,
                    terrain,
                )
            }
            _ => is_within_range_leptons(dist_sq, selected.weapon.range),
        },
    };
    if !in_range {
        return None;
    }

    // G19 — `Insignificant=` at `0x006F8451`. Native only reaches the reject
    // for a non-Building candidate, or a Building owned by a `MultiplayPassive`
    // house; a player-owned Insignificant Building (every wall segment) falls
    // through here and is refused by G24 instead. VERA has no per-house
    // `MultiplayPassive` view at this site, so the Building arm is left out and
    // only the non-Building one is applied — which is the arm that matters:
    // stock authors `Insignificant=yes` on 26 civilian vehicles (cars, buses,
    // taxis) and 22 civilian infantry types (civilians, the cow). Without it a
    // Grizzly on Guard opens fire on passing traffic.
    if candidate_obj.insignificant
        && candidate.category != EntityCategory::Structure
        && !candidate.mind_controlled
    {
        return None;
    }

    // G21 — the disguise arm at `0x006F84B1..0x006F854B`, in its native slot.
    //
    // A `DetectDisguise=` attacker TYPE skips the whole arm — the dogs',
    // Yuri's and the Psi Corps Trooper's role against Spies and Mirage Tanks.
    //
    // RESIDUAL — the blink window and the AI detection roll. Native, having
    // rejected a `DetectDisguise`-less attacker, gives it a second chance while
    // the candidate's `+0x1EC`/`+0x1F4` timer is running AND the attacking
    // house is computer-controlled, at the cost of one `RandomRanged(0, 99)`
    // compared against `[General] DisabledDisguiseDetectionPercent=15,5,2`. The
    // mapping pass found the only writers of both timer fields to be
    // `TechnoClass::Constructor @ 0x006F2CDB` (start = creation frame,
    // duration = 0), which makes `elapsed >= duration` true forever and the
    // draw unreachable for every attacker in YR, human or AI. VERA stores no
    // blink timer and passes 0, which is the same reject.
    // - Trigger: any attacker without `DetectDisguise=` evaluating a disguised
    //   Spy or Mirage Tank. - Player effect: none. - Frequency: n/a.
    // - Downstream risk: if a computed-pointer writer of `+0x1F4` does exist,
    //   wiring it costs one scenario draw per evaluated disguised candidate and
    //   shifts RNG order for every later consumer in the tick.
    const BLINK_TIMER_NOT_MODELLED: i32 = 0;
    let attacker_owner_str = ctx.interner.resolve(ctx.attacker.owner);
    let candidate_disguised_to_attacker = candidate.disguise.as_ref().is_some_and(|disguise| {
        crate::sim::cloak_disguise::is_disguised_to(
            disguise.disguised,
            false,
            ctx.fog.is_some_and(|fog_state| {
                fog_state.detects_disguise_for_house(
                    ctx.attacker.owner,
                    candidate.position.rx,
                    candidate.position.ry,
                )
            }),
            disguise.disguised_as_house.is_some_and(|fake| {
                fake == ctx.attacker.owner
                    || ctx.fog.is_some_and(|fog_state| {
                        fog_state.is_friendly(attacker_owner_str, ctx.interner.resolve(fake))
                    })
            }),
            disguise.disguised_as_house.is_some(),
        )
    });
    if !matches!(
        crate::sim::cloak_disguise::disguise_rejects_candidate(
            candidate_disguised_to_attacker,
            ctx.attacker_obj.detect_disguise,
            BLINK_TIMER_NOT_MODELLED,
            true,
        ),
        crate::sim::cloak_disguise::DisguiseGateOutcome::Accept
    ) {
        return None;
    }

    // G24 — the human-attacker building gate at `0x006F85AB..0x006F8601`, the
    // one and only consumer of `ThreatPosed=` inside acquisition.
    //
    // For a human-controlled attacker that is not an AI team member, an enemy
    // BUILDING is a legal passive target only if it is 1x1-with-undeploy, or it
    // is armed AND its live `ThreatPosed` is non-zero. Both halves are needed:
    // the native reject at `0x006F85DB`/`0x006F85E0`/`0x006F85EE` fires on
    // **(no current weapon) OR (`ThreatPosed` == 0)**.
    //
    //   006f85d3  CALL [candidate vtable+0x3f4]   ; GetCurrentWeapon 0x0070E1A0
    //   006f85d9  TEST EAX,EAX      JZ 006f85f0   ; no weapon      -> reject
    //   006f85dd  CMP  [EAX],0x0    JZ 006f85f0   ; null WeaponType-> reject
    //   006f85e6  CALL [candidate vtable+0x2c0]   ; Get_ThreatPosed 0x00708B40
    //   006f85ee  TEST EAX,EAX      JNZ 006f8604  ; non-zero       -> pass
    //
    // Dropping the weapon half would make seven stock buildings that author a
    // non-zero `ThreatPosed=` with no weapon key of any kind auto-targets that
    // gamemd refuses: `GACSPH` Chronosphere, `GAWEAT` Weather Control, `NAIRON`
    // Iron Curtain, `YAGNTC` Genetic Mutator, `YAPPET`, `GADUMY` (all 1) and
    // `AMMOCRAT` (10). Dropping the `ThreatPosed` half would put every wall,
    // power plant, refinery and war factory back on the auto-target list, along
    // with the SAM Site, Flak Cannon and Grand Cannon, which are armed but
    // author `ThreatPosed=0` or omit the key. Pillbox, Sentry Gun, Tesla Coil,
    // Prism Tower, Gattling Cannon and Psychic Tower are armed and author 30-40,
    // so they stay auto-acquired.
    //
    // The weapon test is `combat_weapon::is_armed`, which models
    // `GetCurrentWeapon @ 0x0070E1A0` — the turret slot when `TurretCount > 0`,
    // else slot 0, plus `BuildingClass::GetWeapon @ 0x004526F0`'s occupied-
    // building arm. Reading `Primary=` directly would disarm `[YAGGUN]`
    // (`WeaponCount=6`, no `Primary=`) and every garrisoned civilian building.
    // `0x0070E1A0` is not overridden: `get_xrefs_to` shows it in all six Techno
    // vtable `+0x3F4` slots.
    //
    // Every VERA house is human-controlled, and `IsControlledByHuman @
    // 0x0050B730` is true for any human in a multiplayer game, so this arm is
    // always taken. The AI-team bypass (`TechnoClass+0x14 & 4` and a non-null
    // `FootClass+0x5D4` Team) has no VERA counterpart and is unreachable until
    // AI teams ship.
    //
    // RESIDUAL — the reject is unconditional here, where native falls through to
    // `0x006F860C` when the attacker's `vtable+0x330` byte (stored at
    // `0x006F7CBA`) is set. That byte also lets an ALLIED candidate past the
    // ally gate at `0x006F7F43`, and `0x006F860C` only accepts a damaged allied
    // Building — i.e. it is the medic/repair scan, which VERA does not run from
    // this site at all. Trigger: an attacker whose weapon heals or repairs.
    // Player effect: none today, since VERA never asks this scan for a repair
    // target. Frequency: n/a. Downstream risk: wiring service units in must
    // carry the whole `0x006F860C` arm, not just this fall-through.
    if candidate.category == EntityCategory::Structure
        && !is_one_by_one_undeployable(candidate_obj)
        && (!is_armed(candidate, candidate_obj)
            || live_threat_posed(ctx.rules, candidate, candidate_obj) == 0)
    {
        return None;
    }

    // RESIDUAL — G27, the bridge-layer gate at `0x006F8672`: when the ATTACKER's
    // cell and the CANDIDATE's cell both carry a bridge (`CellClass+0x140 &
    // 0x100`) and the two objects are on opposite sides of the deck, the
    // candidate is refused. VERA has `GameEntity::on_bridge` but no per-cell
    // "this cell carries a bridge" read at this site, so applying the deck test
    // alone would also reject the ordinary ground-vs-elevated pair the native
    // gate lets through.
    // - Trigger: an armed object on or under a bridge with an enemy on the
    //   other layer, inside its scan radius.
    // - Player effect: a unit under a bridge auto-acquires one crossing it (and
    //   vice versa) where retail would not, so it holds a target it usually
    //   cannot hit.
    // - Frequency: bridge maps only, and only while something is crossing.
    // - Downstream risk: closing it needs the bridge bit threaded from
    //   `ResolvedTerrainGrid` into the scan, which is terrain plumbing rather
    //   than targeting.

    // G28/P9 — score, truncate, and treat an exactly-zero score as a rejection.
    let score = calculate_threat_score(
        ctx.entities,
        ctx.attacker.stable_id,
        candidate.stable_id,
        ctx.rules,
        ctx.interner,
        ctx.terrain,
        ctx.alliances(),
        ctx.coefficients,
    )?;
    finish_score(score)
}

/// `BuildingTypeClass::Is1x1WithUndeploy @ 0x00465D40` (reached through the
/// candidate's vtable `+0x80`): a one-cell building that carries an
/// `UndeploysInto=` vehicle. Such a building is a legal passive target for a
/// human attacker regardless of its `ThreatPosed`, because it is really a
/// parked unit.
fn is_one_by_one_undeployable(obj: &ObjectType) -> bool {
    let (width, height) = crate::rules::foundation::foundation_dimensions(&obj.foundation);
    width == 1 && height == 1 && obj.undeploys_into.is_some()
}

/// `TechnoClass::Get_ThreatPosed @ 0x00708B40` (vtable `+0x2C0`).
///
/// A garrisoned building's threat is `occupants * [General] ThreatPerOccupant`
/// (`RulesClass+0x0DF4`) instead of its own type value; everything else reports
/// `TechnoTypeClass+0x670` directly. The mind-control substitution at
/// `TechnoClass+0x2E4` is not represented — VERA reads the candidate's own type
/// either way.
fn live_threat_posed(rules: &RuleSet, candidate: &GameEntity, obj: &ObjectType) -> i32 {
    if candidate.category == EntityCategory::Structure {
        let occupants = candidate
            .passenger_role
            .cargo()
            .map_or(0, |cargo| cargo.count());
        if occupants > 0 {
            return occupants as i32 * rules.general.threat_per_occupant;
        }
    }
    obj.threat_posed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;
    use crate::sim::intern::test_interner;

    /// A skirmish-shaped fixture: one gun tank, the two civilian object classes
    /// that stock authors `Insignificant=yes` on, an unarmed enemy structure, an
    /// armed enemy defence with a real `ThreatPosed=`, and two enemies that
    /// differ only in `SpecialThreatValue=` so the score ordering is observable
    /// on its own.
    ///
    /// The `[General]` coefficient defaults are the stock `rulesmd.ini` values.
    fn scan_rules() -> RuleSet {
        RuleSet::from_ini(&IniFile::from_str(
            "[General]\n\
             MyEffectivenessCoefficientDefault=200\n\
             TargetEffectivenessCoefficientDefault=-200\n\
             TargetSpecialThreatCoefficientDefault=200\n\
             TargetStrengthCoefficientDefault=-200\n\
             TargetDistanceCoefficientDefault=-10\n\
             ThreatPerOccupant=10\n\
             [VehicleTypes]\n0=GRIZZLY\n1=SCOUT\n2=PRIZE\n3=CAR\n\
             [InfantryTypes]\n0=ENGINEER\n\
             [BuildingTypes]\n0=WALL\n1=PILLBOX\n2=POWER\n3=CHRONO\n4=GATTLING\n\
             [WeaponTypes]\n0=105mm\n1=Vulcan\n\
             [GRIZZLY]\nStrength=300\nArmor=heavy\nPrimary=105mm\n\
             [SCOUT]\nStrength=300\nArmor=heavy\n\
             [PRIZE]\nStrength=300\nArmor=heavy\nSpecialThreatValue=10\n\
             [CAR]\nStrength=100\nArmor=light\nInsignificant=yes\n\
             [ENGINEER]\nStrength=100\nArmor=none\nSpecialThreatValue=1\nThreatPosed=0\n\
             [WALL]\nStrength=100\nArmor=wood\nFoundation=1x1\nInsignificant=yes\nThreatPosed=0\n\
             [POWER]\nStrength=750\nArmor=wood\nFoundation=1x1\nThreatPosed=0\n\
             [PILLBOX]\nStrength=400\nArmor=wood\nFoundation=1x1\nPrimary=Vulcan\nThreatPosed=30\n\
             [CHRONO]\nStrength=1000\nArmor=wood\nFoundation=1x1\nThreatPosed=1\n\
             [GATTLING]\nStrength=400\nArmor=wood\nFoundation=1x1\nTurret=yes\nTurretCount=1\n\
             WeaponCount=2\nWeapon1=Vulcan\nWeapon2=Vulcan\nWeaponStages=1\nThreatPosed=30\n\
             [105mm]\nDamage=100\nROF=110\nRange=5\nWarhead=AP\n\
             [Vulcan]\nDamage=15\nROF=10\nRange=5\nWarhead=AP\n\
             [AP]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
        ))
        .expect("greatest-threat fixture")
    }

    fn place(
        entities: &mut EntityStore,
        id: u64,
        type_id: &str,
        owner: &str,
        rx: u16,
        ry: u16,
        category: EntityCategory,
    ) {
        let mut entity = GameEntity::test_default(id, type_id, owner, rx, ry);
        entity.category = category;
        entity.lifecycle.in_limbo = false;
        if category == EntityCategory::Structure {
            entity.foundation = "1x1".to_string();
        }
        entities.insert(entity);
    }

    fn pick(entities: &EntityStore, rules: &RuleSet, attacker: u64) -> Option<u64> {
        let interner = test_interner();
        super::super::acquire_best_target_for_entity(
            entities, rules, &interner, attacker, None, None, false,
        )
    }

    /// The headline scenario for row 121: a gun tank sitting on Guard with an
    /// enemy WALL one cell away and an enemy ENGINEER three cells away shoots
    /// the engineer.
    ///
    /// The wall is refused by the human-attacker building gate at
    /// `TechnoClass::Evaluate_Candidate @ 0x006F85AB` — no weapon, so no threat
    /// posed, so not a legal passive target however close it is. VERA's retired
    /// nearest-first key picked the wall.
    #[test]
    fn gsi_08_01_a_guard_tank_shoots_the_engineer_not_the_nearer_wall() {
        let rules = scan_rules();
        let mut entities = EntityStore::new();
        place(
            &mut entities,
            1,
            "GRIZZLY",
            "Americans",
            10,
            10,
            EntityCategory::Unit,
        );
        place(
            &mut entities,
            2,
            "WALL",
            "Russians",
            11,
            10,
            EntityCategory::Structure,
        );
        place(
            &mut entities,
            3,
            "ENGINEER",
            "Russians",
            13,
            10,
            EntityCategory::Infantry,
        );
        assert_eq!(pick(&entities, &rules, 1), Some(3));
    }

    /// The other half of the same gate: an armed defence with a real
    /// `ThreatPosed=` IS auto-acquired, an unarmed structure never is. Stock
    /// authors 30-40 on the Pillbox, Sentry Gun, Tesla Coil, Prism Tower,
    /// Gattling Cannon and Psychic Tower, and 0 (or nothing) on every economic
    /// building, on walls, and on the SAM Site, Flak Cannon and Grand Cannon.
    #[test]
    fn gsi_08_01_threat_posed_decides_which_enemy_buildings_are_auto_targets() {
        let rules = scan_rules();
        let mut entities = EntityStore::new();
        place(
            &mut entities,
            1,
            "GRIZZLY",
            "Americans",
            10,
            10,
            EntityCategory::Unit,
        );
        place(
            &mut entities,
            2,
            "POWER",
            "Russians",
            11,
            10,
            EntityCategory::Structure,
        );
        assert_eq!(
            pick(&entities, &rules, 1),
            None,
            "an unarmed enemy structure is invisible to auto-acquire"
        );

        place(
            &mut entities,
            3,
            "PILLBOX",
            "Russians",
            12,
            10,
            EntityCategory::Structure,
        );
        assert_eq!(
            pick(&entities, &rules, 1),
            Some(3),
            "a defence with ThreatPosed=30 is acquired even though it is further away"
        );
    }

    /// The conjunct the gate would be missing without the weapon test: an
    /// UNARMED building that authors a non-zero `ThreatPosed=` is still refused.
    ///
    /// Native rejects on **(no current weapon) OR (`ThreatPosed` == 0)** —
    /// `0x006F85D3` calls `GetCurrentWeapon` and both misses jump to the reject
    /// at `0x006F85F0` before `ThreatPosed` is ever asked for. Seven stock
    /// buildings sit in exactly this shape: the Chronosphere `GACSPH`, Weather
    /// Control `GAWEAT`, Iron Curtain `NAIRON` and Genetic Mutator `YAGNTC`
    /// (`ThreatPosed=1`, no weapon key), plus `YAPPET`, `GADUMY` and
    /// `AMMOCRAT`. A player's units must never spontaneously open fire on an
    /// enemy superweapon.
    #[test]
    fn gsi_08_01_an_unarmed_building_with_threat_posed_is_still_refused() {
        let rules = scan_rules();
        let mut entities = EntityStore::new();
        place(
            &mut entities,
            1,
            "GRIZZLY",
            "Americans",
            10,
            10,
            EntityCategory::Unit,
        );
        place(
            &mut entities,
            2,
            "CHRONO",
            "Russians",
            11,
            10,
            EntityCategory::Structure,
        );
        assert_eq!(
            pick(&entities, &rules, 1),
            None,
            "ThreatPosed=1 with no weapon is not a legal passive target"
        );

        place(
            &mut entities,
            3,
            "PILLBOX",
            "Russians",
            12,
            10,
            EntityCategory::Structure,
        );
        assert_eq!(
            pick(&entities, &rules, 1),
            Some(3),
            "the armed defence further out is taken over the nearer superweapon"
        );
    }

    /// The trap in the other direction: the weapon test is
    /// `GetCurrentWeapon @ 0x0070E1A0`, not `Primary=`. `[YAGGUN]` the Gattling
    /// Cannon authors `TurretCount=1`, `WeaponCount=6` and `Weapon1..6=` with no
    /// `Primary=` key at all, so a naive `primary.is_some()` would disarm it and
    /// hand the player a base defence their units silently ignore.
    #[test]
    fn gsi_08_01_a_weapon_array_defence_with_no_primary_key_stays_a_target() {
        let rules = scan_rules();
        let gattling = rules.object("GATTLING").expect("GATTLING");
        let mut building = GameEntity::test_default(2, "GATTLING", "Russians", 11, 10);
        building.category = EntityCategory::Structure;
        assert!(
            is_armed(&building, gattling),
            "a WeaponCount= defence resolves slot 0 through GetCurrentWeapon"
        );

        let mut entities = EntityStore::new();
        place(
            &mut entities,
            1,
            "GRIZZLY",
            "Americans",
            10,
            10,
            EntityCategory::Unit,
        );
        place(
            &mut entities,
            2,
            "GATTLING",
            "Russians",
            11,
            10,
            EntityCategory::Structure,
        );
        assert_eq!(pick(&entities, &rules, 1), Some(2));
    }

    /// `Insignificant=` at `0x006F8451`: civilian traffic is not a target.
    /// Stock puts the flag on 26 vehicle types and 22 infantry types.
    #[test]
    fn gsi_08_01_insignificant_civilians_are_skipped_for_the_real_enemy() {
        let rules = scan_rules();
        let mut entities = EntityStore::new();
        place(
            &mut entities,
            1,
            "GRIZZLY",
            "Americans",
            10,
            10,
            EntityCategory::Unit,
        );
        place(
            &mut entities,
            2,
            "CAR",
            "Russians",
            11,
            10,
            EntityCategory::Unit,
        );
        place(
            &mut entities,
            3,
            "SCOUT",
            "Russians",
            13,
            10,
            EntityCategory::Unit,
        );
        assert_eq!(pick(&entities, &rules, 1), Some(3));
    }

    /// Inside one band the maximum score wins, not the first cell reached.
    ///
    /// Both enemies sit on ring 2, and the ring's row pass reaches the
    /// north-west cell before the south-east one, so walk order alone would
    /// pick the plain SCOUT. `PRIZE` carries `SpecialThreatValue=10`, worth
    /// `+2000` through the `C` coefficient, and displaces it on the
    /// strictly-greater keep at `0x006F94A0`.
    #[test]
    fn gsi_08_01_the_higher_score_beats_the_earlier_cell_in_the_same_band() {
        let rules = scan_rules();
        let mut entities = EntityStore::new();
        place(
            &mut entities,
            1,
            "GRIZZLY",
            "Americans",
            10,
            10,
            EntityCategory::Unit,
        );
        // Owning a building puts this house on the per-type coefficient set,
        // which is what `HouseClass+0x1FB` selects once an MCV has deployed.
        place(
            &mut entities,
            9,
            "POWER",
            "Americans",
            20,
            20,
            EntityCategory::Structure,
        );
        place(
            &mut entities,
            2,
            "SCOUT",
            "Russians",
            8,
            8,
            EntityCategory::Unit,
        );
        place(
            &mut entities,
            3,
            "PRIZE",
            "Russians",
            12,
            12,
            EntityCategory::Unit,
        );
        assert_eq!(pick(&entities, &rules, 1), Some(3));
    }

    /// The band early-return at `0x006F94D0` is what makes a defence commit to
    /// whatever is at its feet: with a radius of 6 the walk returns as soon as
    /// ring 1 has produced anything, so the far higher-scoring `PRIZE` is never
    /// evaluated at all.
    #[test]
    fn gsi_08_01_the_first_band_ends_the_scan_before_the_better_target_is_seen() {
        let rules = scan_rules();
        let mut entities = EntityStore::new();
        place(
            &mut entities,
            1,
            "GRIZZLY",
            "Americans",
            10,
            10,
            EntityCategory::Unit,
        );
        place(
            &mut entities,
            9,
            "POWER",
            "Americans",
            20,
            20,
            EntityCategory::Structure,
        );
        place(
            &mut entities,
            2,
            "SCOUT",
            "Russians",
            11,
            10,
            EntityCategory::Unit,
        );
        place(
            &mut entities,
            3,
            "PRIZE",
            "Russians",
            14,
            10,
            EntityCategory::Unit,
        );
        assert_eq!(pick(&entities, &rules, 1), Some(2));
    }

    /// `0x0070CD4E` picks between two coefficient sets on the scorer house's
    /// `+0x1FB` byte. The sets are not variations on a theme: two of the five
    /// weights change sign and the distance weight changes by 10x, so a scorer
    /// on the wrong set can rank the same two candidates in the opposite order.
    ///
    /// Every house in a skirmish carries the byte SET from construction
    /// (`HouseClass::Constructor @ 0x004F644E`), so the per-type branch is the
    /// live one for the whole match — see [`HOUSE_SELECTS_OWN_COEFFICIENTS`].
    #[test]
    fn gsi_08_01_every_house_takes_the_per_type_coefficient_set() {
        let rules = scan_rules();
        let obj = rules.object("GRIZZLY").expect("GRIZZLY");
        let live = ThreatCoefficients::resolve(&rules, obj, HOUSE_SELECTS_OWN_COEFFICIENTS);
        let byte_clear = ThreatCoefficients::resolve(&rules, obj, false);
        assert_eq!(live.target_effectiveness, -200.0);
        assert_eq!(live.target_strength, -200.0);
        assert_eq!(live.target_distance, -10.0);
        assert_eq!(byte_clear.target_effectiveness, 200.0);
        assert_eq!(byte_clear.target_strength, 200.0);
        assert_eq!(byte_clear.target_distance, -1.0);
    }

    /// `TechnoClass::Get_ThreatPosed @ 0x00708B40`: a garrisoned building's
    /// threat comes from its occupants, not its own type value, which is how a
    /// civilian house full of GIs becomes a target for units that would ignore
    /// the empty one.
    #[test]
    fn gsi_08_01_threat_per_occupant_replaces_the_type_value() {
        let rules = scan_rules();
        let obj = rules.object("POWER").expect("POWER");
        let mut building = GameEntity::test_default(1, "POWER", "Russians", 5, 5);
        building.category = EntityCategory::Structure;
        assert_eq!(live_threat_posed(&rules, &building, obj), 0);
        assert_eq!(rules.general.threat_per_occupant, 10);
    }

    /// The literal loop order of `Greatest_Threat`'s ring walk, including the
    /// centre cell being visited twice at r = 0. Both facts are load-bearing:
    /// the order decides which of two equal-scoring candidates wins, and the
    /// duplicate is what the native `do/while` does.
    #[test]
    fn ring_zero_visits_the_centre_twice() {
        assert_eq!(ring_cells(10, 10, 0), vec![(10, 10), (10, 10)]);
    }

    #[test]
    fn ring_one_is_the_eight_neighbours_row_pass_first() {
        assert_eq!(
            ring_cells(10, 10, 1),
            vec![
                // Row pass: north row and south row interleaved, west to east.
                (9, 9),
                (9, 11),
                (10, 9),
                (10, 11),
                (11, 9),
                (11, 11),
                // Column pass: west then east, dy = 0 only.
                (9, 10),
                (11, 10),
            ]
        );
    }

    #[test]
    fn ring_two_has_the_sixteen_perimeter_cells_once_each() {
        let cells = ring_cells(10, 10, 2);
        assert_eq!(cells.len(), 16);
        let mut sorted = cells.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 16, "no cell is visited twice on ring 2");
        for (x, y) in cells {
            assert!(
                (x - 10).abs() == 2 || (y - 10).abs() == 2,
                "({x}, {y}) is not on the ring-2 perimeter"
            );
        }
    }

    /// `0x006F94D0`: the walk gives up early at a quarter and a half of the
    /// radius. For a stock Grizzly (radius 6) that means rings {0,1} decide
    /// first, then {2,3}, then {4,5}.
    #[test]
    fn early_return_rings_for_a_stock_grizzly_radius() {
        let radius = 6;
        let bands: Vec<i32> = (0..radius)
            .filter(|ring| is_early_return_ring(*ring, radius))
            .collect();
        assert_eq!(bands, vec![1, 3]);
    }

    /// A score of exactly zero is a rejection, not a low accept; a negative one
    /// is clamped up to 1 and accepted.
    #[test]
    fn zero_score_rejects_and_negative_clamps_to_one() {
        assert_eq!(finish_score(X87Chop53::load_i32(0)), None);
        assert_eq!(finish_score(X87Chop53::load_i32(-5)), Some(1));
        assert_eq!(finish_score(X87Chop53::load_i32(42)), Some(42));
    }

    /// The `0.02f` Verses floor is a FLOAT constant widened to double, so an
    /// authored `Verses=2%` sits just above it and survives while 1% does not.
    #[test]
    fn verses_floor_rejects_one_percent_but_not_two() {
        assert!(0.01_f64 <= VERSES_FLOOR);
        assert!(0.02_f64 > VERSES_FLOOR);
        assert!(0.0_f64 <= VERSES_FLOOR);
    }
}
