//! Per-cell radiation field service: radiation sites spread an additive
//! linear-falloff level over a square of cells, step it back down on a
//! per-site countdown, and self-delete when their lifetime expires. Foot
//! units standing on irradiated cells take periodic damage through the
//! `[Radiation] RadSiteWarhead=` warhead (applied by the combat tick, not
//! here — this module owns only the field state and its evolution).
//!
//! Depends on `rules/` ([Radiation] constants), `map/resolved_terrain`
//! (cell ground height for 3D falloff distance) and `util/lepton`.
//!
//! ## Float exception
//! Cell levels and the falloff/decay kernel are carried as `f64` with
//! truncate-toward-zero staging at every integer boundary — the same
//! documented float exception as `combat::damage` (the original engine
//! computes this exact pipeline in doubles; IEEE `+ - * / sqrt` are
//! bit-deterministic, so lockstep safety is preserved). Site bookkeeping
//! (level/duration/remaining/steps) is integer math.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::ruleset::RadiationRules;
use crate::util::lepton::LEPTONS_PER_LEVEL;

/// Leptons per cell edge (cell-center to cell-center step).
const LEPTONS_PER_CELL: i64 = 256;

/// Fallback square bound when no terrain grid is loaded (headless tests):
/// the engine-native full grid extent.
const FALLBACK_BOUNDS: (u16, u16) = (512, 512);

/// One radiation site, keyed by its center cell. A second detonation on the
/// same center MERGES into the existing site (level/duration reset); different
/// centers stack additively per cell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadSite {
    /// Center cell.
    pub center: (u16, u16),
    /// Spread in whole cells (truncated warhead CellSpread); the affected
    /// square is (2·spread+1)² around the center. Fixed at creation — a merge
    /// does NOT update it even if the new weapon's warhead differs.
    pub spread: i32,
    /// Falloff radius in leptons = spread × 256 + 128.
    pub radius_leptons: i32,
    /// Site level at the last (re)activation. Constant between activations;
    /// the decay step recomputes per-cell falloff from this value.
    pub level: i32,
    /// Number of decay steps over the lifetime = duration / level_delay
    /// (integer division — 0 is possible for short-lived sites and makes the
    /// per-step amount unbounded, clearing affected cells on the first step).
    pub level_steps: i32,
    /// Total lifetime in frames = duration_multiple × level.
    pub duration: i32,
    /// Frames left. Decremented once per tick; the site self-deletes when it
    /// drops below 1 (leaving residual cell levels in place — only the center
    /// marker is released).
    pub remaining: i32,
    /// Frame the level-decay countdown last (re)started.
    pub level_timer_start: u32,
    /// Countdown length in frames (level_delay at activation).
    pub level_timer_duration: i32,
}

/// The per-cell radiation field + site registry. Persisted and state-hashed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RadiationState {
    /// Sparse per-cell radiation level. Entries are removed when they decay
    /// to exactly 0 (the subtraction clamps negative results to 0).
    cells: BTreeMap<(u16, u16), f64>,
    /// One site per center cell.
    sites: BTreeMap<(u16, u16), RadSite>,
}

/// A weapon detonation that emits radiation, recorded by the combat tick and
/// folded into `RadiationState` the same tick.
#[derive(Debug, Clone, Copy)]
pub struct RadDetonation {
    pub rx: u16,
    pub ry: u16,
    /// Weapon `RadLevel=` (> 0, checked by the producer).
    pub rad_level: i32,
    /// Truncated warhead `CellSpread=` in whole cells.
    pub spread: i32,
}

/// Ground height of a cell center in leptons (level × 104). Flat 0 when no
/// terrain grid is loaded.
fn cell_height_leptons(terrain: Option<&ResolvedTerrainGrid>, rx: u16, ry: u16) -> i64 {
    terrain
        .and_then(|t| t.cell(rx, ry))
        .map(|c| c.level as i64 * LEPTONS_PER_LEVEL)
        .unwrap_or(0)
}

/// 3D cell-center distance in whole leptons, truncated toward zero.
fn cell_distance_leptons(
    terrain: Option<&ResolvedTerrainGrid>,
    a: (u16, u16),
    b: (u16, u16),
) -> i32 {
    let dx = (b.0 as i64 - a.0 as i64) * LEPTONS_PER_CELL;
    let dy = (b.1 as i64 - a.1 as i64) * LEPTONS_PER_CELL;
    let dz = cell_height_leptons(terrain, b.0, b.1) - cell_height_leptons(terrain, a.0, a.1);
    let dist_sq = (dx * dx + dy * dy + dz * dz) as f64;
    dist_sq.sqrt() as i32
}

/// Linear falloff contribution of `site` at `cell`:
/// `(radius − dist) / radius × level` when `dist <= radius`, else 0.
fn falloff_at(site: &RadSite, terrain: Option<&ResolvedTerrainGrid>, cell: (u16, u16)) -> f64 {
    let dist = cell_distance_leptons(terrain, site.center, cell);
    let radius = site.radius_leptons;
    if dist <= radius {
        ((radius - dist) as f64 / radius as f64) * site.level as f64
    } else {
        0.0
    }
}

/// Iterate the (2·spread+1)² square around the center, clamped to `bounds`.
fn for_each_square_cell(
    center: (u16, u16),
    spread: i32,
    bounds: (u16, u16),
    mut f: impl FnMut((u16, u16)),
) {
    let (cx, cy) = (center.0 as i32, center.1 as i32);
    for y in (cy - spread)..=(cy + spread) {
        if y < 0 || y >= bounds.1 as i32 {
            continue;
        }
        for x in (cx - spread)..=(cx + spread) {
            if x < 0 || x >= bounds.0 as i32 {
                continue;
            }
            f((x as u16, y as u16));
        }
    }
}

impl RadiationState {
    /// Raw level of one cell (un-clamped, may exceed `RadLevelMax`).
    pub fn cell_level(&self, cell: (u16, u16)) -> f64 {
        self.cells.get(&cell).copied().unwrap_or(0.0)
    }

    /// The level a cell DAMAGES as: `trunc(min(level, RadLevelMax))`. The
    /// clamp caps damage only — the stored level itself is unbounded.
    pub fn damaging_level(&self, cell: (u16, u16), level_max: i32) -> i32 {
        let level = self.cell_level(cell);
        let max = level_max as f64;
        let clamped = if level < max { level } else { max };
        clamped as i32
    }

    pub fn site_at(&self, center: (u16, u16)) -> Option<&RadSite> {
        self.sites.get(&center)
    }

    pub fn sites(&self) -> impl Iterator<Item = &RadSite> {
        self.sites.values()
    }

    /// Deterministic iteration over irradiated cells (sorted by coord), for
    /// state hashing and the render glow layer.
    pub fn iter_cells(&self) -> impl Iterator<Item = (&(u16, u16), &f64)> {
        self.cells.iter()
    }

    /// True when nothing is irradiated and no site is alive.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty() && self.sites.is_empty()
    }

    /// Effective level of a site right now: `remaining × level / duration`
    /// (integer math), 0 when the duration is degenerate. Drives the deployed
    /// self-irradiator re-fire gate.
    pub fn current_site_level(site: &RadSite) -> i32 {
        if site.duration < 1 {
            return 0;
        }
        site.remaining.wrapping_mul(site.level) / site.duration
    }

    fn add_cell(&mut self, cell: (u16, u16), amount: f64) {
        if amount != 0.0 {
            *self.cells.entry(cell).or_insert(0.0) += amount;
        }
    }

    /// Subtract clamping at exactly 0 (negative results store 0; the entry is
    /// then dropped to keep the map sparse).
    fn sub_cell(&mut self, cell: (u16, u16), amount: f64) {
        if let Some(v) = self.cells.get_mut(&cell) {
            *v -= amount;
            if *v <= 0.0 {
                self.cells.remove(&cell);
            }
        }
    }

    /// Spread the site's full falloff into the field (runs at every
    /// (re)activation — a merge re-spreads with the new level).
    fn spread_site(&mut self, center: (u16, u16), terrain: Option<&ResolvedTerrainGrid>) {
        let Some(site) = self.sites.get(&center).cloned() else {
            return;
        };
        let bounds = terrain
            .map(|t| (t.width(), t.height()))
            .unwrap_or(FALLBACK_BOUNDS);
        for_each_square_cell(site.center, site.spread, bounds, |cell| {
            self.add_cell(cell, falloff_at(&site, terrain, cell));
        });
    }

    /// Remove the site's OUTSTANDING contribution before a merge: per cell,
    /// `(falloff / level_steps) × (remaining/level_delay + 1)` — the per-step
    /// decay amount times the decay steps the site still had ahead of it.
    /// With `level_steps == 0` the division is unbounded and clears the
    /// touched cells outright (instead of poisoning them — a guard the
    /// original double pipeline lacks for the falloff==0 corner; unreachable
    /// on stock rules where every RadLevel ≥ the decay delay).
    fn decrease_site_levels(
        &mut self,
        center: (u16, u16),
        terrain: Option<&ResolvedTerrainGrid>,
        rules: &RadiationRules,
    ) {
        let Some(site) = self.sites.get(&center).cloned() else {
            return;
        };
        let bounds = terrain
            .map(|t| (t.width(), t.height()))
            .unwrap_or(FALLBACK_BOUNDS);
        let steps_ahead = (site.remaining / rules.level_delay + 1) as f64;
        for_each_square_cell(site.center, site.spread, bounds, |cell| {
            let falloff = falloff_at(&site, terrain, cell);
            if falloff != 0.0 {
                self.sub_cell(cell, (falloff / site.level_steps as f64) * steps_ahead);
            }
        });
    }

    /// (Re)arm the decay countdown and derive the per-lifetime step count.
    fn activate_site(&mut self, center: (u16, u16), frame: u32, rules: &RadiationRules) {
        if let Some(site) = self.sites.get_mut(&center) {
            site.level_steps = site.duration / rules.level_delay;
            site.level_timer_start = frame;
            site.level_timer_duration = rules.level_delay;
        }
    }

    /// Fold one radiation-emitting detonation into the field: create a site
    /// on a fresh center, or merge into the existing site (current effective
    /// level + added level, lifetime reset, field re-spread with the new
    /// level after the old outstanding contribution is removed).
    pub fn apply_detonation(
        &mut self,
        det: RadDetonation,
        frame: u32,
        rules: &RadiationRules,
        terrain: Option<&ResolvedTerrainGrid>,
    ) {
        let center = (det.rx, det.ry);
        let merged = self
            .sites
            .get(&center)
            .map(|site| Self::current_site_level(site).wrapping_add(det.rad_level));
        match merged {
            None => {
                let level = det.rad_level;
                let duration = rules.duration_multiple.wrapping_mul(level);
                self.sites.insert(
                    center,
                    RadSite {
                        center,
                        spread: det.spread,
                        radius_leptons: det.spread * LEPTONS_PER_CELL as i32 + 128,
                        level,
                        level_steps: 0,
                        duration,
                        remaining: duration,
                        level_timer_start: frame,
                        level_timer_duration: rules.level_delay,
                    },
                );
            }
            Some(merged) => {
                self.decrease_site_levels(center, terrain, rules);
                let site = self
                    .sites
                    .get_mut(&center)
                    .expect("site present in merge branch");
                site.level = merged;
                let duration = rules.duration_multiple.wrapping_mul(merged);
                site.duration = duration;
                site.remaining = duration;
            }
        }
        self.activate_site(center, frame, rules);
        self.spread_site(center, terrain);
    }

    /// Per-tick site evolution: lifetime countdown, the periodic per-cell
    /// decay step, and site self-deletion. Residual cell levels survive a
    /// site's death — only the center registration is released.
    pub fn tick_decay(
        &mut self,
        frame: u32,
        rules: &RadiationRules,
        terrain: Option<&ResolvedTerrainGrid>,
    ) {
        if self.sites.is_empty() {
            return;
        }
        let centers: Vec<(u16, u16)> = self.sites.keys().copied().collect();
        for center in centers {
            // Lifetime ticks down every frame, independent of the decay timer.
            let (expired, dead) = {
                let Some(site) = self.sites.get_mut(&center) else {
                    continue;
                };
                site.remaining -= 1;
                let elapsed = frame.wrapping_sub(site.level_timer_start) as i64;
                let expired =
                    site.level_timer_duration <= 0 || elapsed >= site.level_timer_duration as i64;
                (expired, site.remaining < 1)
            };
            if expired {
                self.decay_step(center, terrain);
                if let Some(site) = self.sites.get_mut(&center) {
                    site.level_timer_start = frame;
                    site.level_timer_duration = rules.level_delay;
                }
            }
            if dead {
                self.sites.remove(&center);
            }
        }
    }

    /// One decay step: subtract `falloff / level_steps` from every cell of
    /// the square (falloff recomputed from the activation-time level, so each
    /// step removes a constant amount; the subtraction clamps at 0).
    fn decay_step(&mut self, center: (u16, u16), terrain: Option<&ResolvedTerrainGrid>) {
        let Some(site) = self.sites.get(&center).cloned() else {
            return;
        };
        let bounds = terrain
            .map(|t| (t.width(), t.height()))
            .unwrap_or(FALLBACK_BOUNDS);
        for_each_square_cell(site.center, site.spread, bounds, |cell| {
            let falloff = falloff_at(&site, terrain, cell);
            if falloff != 0.0 {
                self.sub_cell(cell, falloff / site.level_steps as f64);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stock_rules() -> RadiationRules {
        RadiationRules {
            duration_multiple: 1,
            application_delay: 16,
            level_max: 500,
            level_delay: 90,
            light_delay: 90,
            level_factor: 0.2,
            light_factor: crate::util::fixed_math::sim_from_f32(0.1),
            tint_factor: crate::util::fixed_math::sim_from_f32(1.0),
            color: (0, 255, 0),
            site_warhead: "RadSite".to_string(),
        }
    }

    fn desolator_det(rx: u16, ry: u16) -> RadDetonation {
        // RadEruptionWeapon: RadLevel=500; RadEruptionWarhead: CellSpread=10.
        RadDetonation {
            rx,
            ry,
            rad_level: 500,
            spread: 10,
        }
    }

    /// Exact falloff values at center / edge / corner of the square
    /// (flat terrain, radius = 10·256+128 = 2688).
    #[test]
    fn desolator_deploy_irradiates_square_with_linear_falloff() {
        let rules = stock_rules();
        let mut rad = RadiationState::default();
        rad.apply_detonation(desolator_det(100, 100), 0, &rules, None);

        // Center: dist 0 → full level.
        assert_eq!(rad.cell_level((100, 100)), 500.0);
        // Edge of the square, straight east: dist = 10·256 = 2560 ≤ 2688 →
        // (2688−2560)/2688 × 500.
        let edge = (2688.0 - 2560.0) / 2688.0 * 500.0;
        assert_eq!(rad.cell_level((110, 100)), edge);
        // One cell out (11 east): dist 2816 > 2688 → no radiation.
        assert_eq!(rad.cell_level((111, 100)), 0.0);
        // Corner of the square: dist = trunc(sqrt(2)·2560) = 3620 > radius →
        // the corner stays clean (the square overshoots the circle).
        assert_eq!(rad.cell_level((110, 110)), 0.0);
        // Diagonal inside the circle: (105,105): dist = trunc(sqrt(2)·1280)
        // = 1810 → (2688−1810)/2688 × 500.
        let diag = (2688.0 - 1810.0) / 2688.0 * 500.0;
        assert_eq!(rad.cell_level((105, 105)), diag);
        // Damaging level clamps at RadLevelMax but the raw value is stored.
        assert_eq!(rad.damaging_level((100, 100), rules.level_max), 500);
        assert_eq!(rad.damaging_level((105, 105), rules.level_max), diag as i32);
    }

    /// Same-center re-detonation merges (effective + added), it does not
    /// stack to 2× full level.
    #[test]
    fn same_center_redetonation_merges_not_stacks() {
        let rules = stock_rules();
        let mut rad = RadiationState::default();
        rad.apply_detonation(desolator_det(50, 50), 0, &rules, None);
        // Immediately re-detonate: effective is still the full 500 (no frames
        // elapsed), so the merged level is exactly 500 + 500.
        rad.apply_detonation(desolator_det(50, 50), 0, &rules, None);
        let site = rad.site_at((50, 50)).expect("merged site survives");
        assert_eq!(site.level, 1000);
        assert_eq!(site.duration, 1000);
        assert_eq!(site.remaining, 1000);
        // Field: spread(500) − outstanding(500·(0/90+1 step)) + spread(1000)
        // = 1000 at the center.
        assert_eq!(rad.cell_level((50, 50)), 1000.0);
        // Exactly ONE site exists.
        assert_eq!(rad.sites().count(), 1);
    }

    /// Different centers stack additively on shared cells.
    #[test]
    fn different_center_sites_stack_additively() {
        let rules = stock_rules();
        let mut rad = RadiationState::default();
        rad.apply_detonation(desolator_det(100, 100), 0, &rules, None);
        rad.apply_detonation(desolator_det(102, 100), 0, &rules, None);
        assert_eq!(rad.sites().count(), 2);
        // (101,100) is 1 cell from each center: dist 256 →
        // (2688−256)/2688 × 500 from each site.
        let one_cell = (2688.0 - 256.0) / 2688.0 * 500.0;
        assert_eq!(rad.cell_level((101, 100)), one_cell + one_cell);
        // Each center also receives the other site's 2-cell falloff.
        let two_cell = (2688.0 - 512.0) / 2688.0 * 500.0;
        assert_eq!(rad.cell_level((100, 100)), 500.0 + two_cell);
    }

    /// The site counts down once per tick, steps the field down every
    /// `RadLevelDelay` frames, and self-deletes once `remaining < 1` —
    /// releasing only the center registration.
    #[test]
    fn site_self_deletes_and_clears_center_ptr() {
        let rules = stock_rules();
        let mut rad = RadiationState::default();
        rad.apply_detonation(desolator_det(10, 10), 0, &rules, None);
        let full = rad.cell_level((10, 10));
        // duration 500, level_steps = 500/90 = 5, step every 90 frames.
        for frame in 1..=89 {
            rad.tick_decay(frame, &rules, None);
        }
        assert_eq!(
            rad.cell_level((10, 10)),
            full,
            "no decay step before the 90-frame countdown expires"
        );
        rad.tick_decay(90, &rules, None);
        let after_one = rad.cell_level((10, 10));
        assert_eq!(
            after_one,
            full - full / 5.0,
            "first step removes falloff/level_steps"
        );
        // Run out the rest of the lifetime.
        for frame in 91..=520 {
            rad.tick_decay(frame, &rules, None);
        }
        assert!(
            rad.site_at((10, 10)).is_none(),
            "site self-deletes at remaining < 1"
        );
        // 5 steps of full/5 each have cleared the center cell exactly (its
        // per-step amount, 100.0, is representable). Off-center cells may
        // keep ±1-ulp float residue — the sequential double subtraction
        // leaves the same residue in the original engine — but nothing may
        // remain at a level that truncates to a damaging value.
        assert_eq!(rad.cell_level((10, 10)), 0.0);
        assert!(
            rad.iter_cells().all(|(_, &v)| v < 1e-9),
            "only sub-damaging float residue may survive the full decay"
        );
        assert_eq!(rad.sites().count(), 0);
    }

    /// The decay countdown is per-site (anchored at activation), not a global
    /// frame-modulo: a site created mid-stream steps 90 frames after ITS
    /// activation.
    #[test]
    fn decay_countdown_is_activation_anchored() {
        let rules = stock_rules();
        let mut rad = RadiationState::default();
        rad.apply_detonation(desolator_det(10, 10), 37, &rules, None);
        let full = rad.cell_level((10, 10));
        for frame in 38..=126 {
            rad.tick_decay(frame, &rules, None);
        }
        assert_eq!(rad.cell_level((10, 10)), full);
        rad.tick_decay(127, &rules, None);
        assert!(rad.cell_level((10, 10)) < full);
    }

    /// Merge bookkeeping mid-life: outstanding contribution is removed with
    /// the old level before the new level is spread.
    #[test]
    fn midlife_merge_resets_to_effective_plus_added() {
        let rules = stock_rules();
        let mut rad = RadiationState::default();
        rad.apply_detonation(desolator_det(10, 10), 0, &rules, None);
        // Advance 180 frames: two decay steps, remaining 500→320.
        for frame in 1..=180 {
            rad.tick_decay(frame, &rules, None);
        }
        let site = rad.site_at((10, 10)).unwrap();
        assert_eq!(site.remaining, 320);
        // Effective = 320×500/500 = 320; merge adds 500 → 820.
        rad.apply_detonation(desolator_det(10, 10), 180, &rules, None);
        let site = rad.site_at((10, 10)).unwrap();
        assert_eq!(site.level, 820);
        assert_eq!(site.duration, 820);
        assert_eq!(site.remaining, 820);
        // Field at center: 500 − 2×(500/5) [two steps] = 300, then the
        // outstanding removal (500/5)×(320/90+1) = 400 over-subtracts and
        // clamps at exactly 0 (the subtraction stores 0 on a negative
        // result), then the re-spread adds the full 820.
        assert_eq!(rad.cell_level((10, 10)), 820.0);
    }

    /// Square bounds clamp at the map edge without wrapping or panicking.
    #[test]
    fn detonation_near_origin_clamps_square() {
        let rules = stock_rules();
        let mut rad = RadiationState::default();
        rad.apply_detonation(desolator_det(2, 2), 0, &rules, None);
        assert_eq!(rad.cell_level((2, 2)), 500.0);
        assert!(rad.cell_level((0, 0)) > 0.0);
    }

    /// Serde round-trip preserves the field and the sites bit-exactly.
    #[test]
    fn radiation_state_serde_round_trip() {
        let rules = stock_rules();
        let mut rad = RadiationState::default();
        rad.apply_detonation(desolator_det(10, 10), 5, &rules, None);
        for frame in 6..=130 {
            rad.tick_decay(frame, &rules, None);
        }
        let bytes = bincode::serialize(&rad).expect("serialize");
        let restored: RadiationState = bincode::deserialize(&bytes).expect("deserialize");
        let a: Vec<_> = rad.iter_cells().map(|(c, v)| (*c, v.to_bits())).collect();
        let b: Vec<_> = restored
            .iter_cells()
            .map(|(c, v)| (*c, v.to_bits()))
            .collect();
        assert_eq!(a, b);
        assert_eq!(
            restored.site_at((10, 10)).map(|s| (s.level, s.remaining)),
            rad.site_at((10, 10)).map(|s| (s.level, s.remaining)),
        );
    }
}
