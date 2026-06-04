//! Bridge topology read service (Slice 3).
//!
//! Single owner of the gamemd-native bridge bit semantics and signed
//! effective-height math, plus the service-facing handle for the traversal gate.
//! Consumers (movement, combat-AoE, occupancy, pathfinding) construct a borrowed
//! `CellBridgeView` over the canonical cell store and read these predicates
//! instead of re-deriving each one at their own call site.
//!
//! Scope of THIS slice: the verified, hash-neutral predicate/offset
//! consolidation only — the structural/bridgehead/anchor flag predicates, signed
//! effective-height, the concrete- and wood-bridge tileset windows (kept SEPARATE
//! from the structural `0x100` flag), the low-bridge/tube predicate, the list
//! layer enum, and a delegating handle to the existing pathfinding traversal
//! gate. The AoE/occupancy layer selectors and the gate authority-flip are NOT
//! folded here: they depend on unproven binary operands (GetGroundHeight vs raw
//! level) and/or change hashed state, so they stay at their current authoritative
//! call sites.
//!
//! ## Dependency rules
//! - Depends on map/bridge_facts (flag bits), map/resolved_terrain, sim/pathfinding
//!   (the traversal gate it delegates to). All within sim/ + map/.
//! - NEVER depends on render/ — the render draw-offset lives behind a separate
//!   render-facing trait so this module stays render-free (invariant #1).
//! - All math is integer / `i8`-signed. No f32/f64 (the float boundary is INI
//!   parse only, which this module does not touch).

use crate::map::bridge_facts::BridgeFlags;
use crate::map::resolved_terrain::{ResolvedTerrainCell, YR_CELL_LAND_TUNNEL};

/// Anchor cells add the full 4-level bridge deck to their effective height.
///
/// This is the CELL-LEVEL deck height (deck = 4 levels above ground), the same
/// unit the existing combat-AoE selector compares `cell.level` against. It is NOT
/// a lepton height: the binary frames the deck height in leptons (per-level x 4),
/// but every Rust selector pre-divides to levels, so the resolved value here is
/// the level count `4`. Single-sourced as one named const; if the AoE selector's
/// own `BRIDGE_AOE_SELECTOR_HEIGHT_LEVELS` is later folded into this service it
/// must collapse onto this same value.
pub const BRIDGE_DECK_HEIGHT_LEVELS: i32 = 4;

/// Width of a tileset window: a concrete- or wood-bridge tileset occupies the
/// first 16 tiles `[base, base + 0x10)` of its theater set. Gated on base != -1.
const BRIDGE_TILESET_WINDOW: i32 = 0x10;

/// Which persistent cell list an object belongs to. The ground list and the
/// bridge-deck list are distinct so movers/projectiles on a high bridge do not
/// interact with whatever sits underneath the span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListLayer {
    Ground,
    Bridge,
}

/// Borrowed read view of one cell's bridge-relevant substrate fields.
///
/// This is NOT a new owned store — it is an adapter built from the canonical
/// post-map-load cell store (`ResolvedTerrainCell`) so the seven predicates read
/// the same data the rest of the engine stamps at map load. All fields are copied
/// out as the gamemd-native signed/typed forms (e.g. `level` is reinterpreted as
/// `i8`, mirroring `PathCell::signed_level`), so the predicates never re-do the
/// sign or window math.
#[derive(Debug, Clone, Copy)]
pub struct CellBridgeView {
    /// Signed cell height level. Reinterpreted `u8 -> i8` exactly like
    /// `PathCell::signed_level()` so a raw level > 127 reads as negative.
    pub level: i8,
    /// CellClass flag word as a typed handle, single-sourced from `bridge_facts`.
    pub flags: BridgeFlags,
    /// Slope/ramp passability byte (CellClass+0x11C / `PathCell.slope_type`),
    /// reinterpreted signed for the diff-1 traversal sub-branch the gate uses.
    pub ramp_byte: i8,
    /// Final resolved isometric tile id (`ResolvedTerrainCell.final_tile_index`).
    /// Window-compared against the theater tileset bases for the tileset
    /// predicates — NOT the structural flag.
    pub iso_tile_index: i32,
    /// Low-bridge tube index (CellClass+0x116), `None` when not a tube cell.
    pub tube_index: Option<i16>,
    /// Final CellClass LandType (`yr_cell_land_type`), used by the low-bridge
    /// predicate (`== YR_CELL_LAND_TUNNEL`).
    pub land_type: u8,
    /// Bridge state byte (0 = NS base, 9 = EW base, etc.). Carried for the
    /// render draw-offset trait; not used by the sim predicates.
    pub state_byte: u8,
}

impl CellBridgeView {
    /// Build a view from the canonical resolved-terrain cell.
    ///
    /// The `u8 -> i8` casts on `level` and `slope_type` are deliberate: they
    /// reproduce gamemd's signed reinterpretation of those bytes (a raw level of
    /// `0xFE` is height `-2`, not `254`). This is the only place the cast lives.
    pub fn from_resolved(cell: &ResolvedTerrainCell) -> Self {
        CellBridgeView {
            level: cell.level as i8,
            flags: BridgeFlags(cell.bridge_facts.raw_flags),
            ramp_byte: cell.slope_type as i8,
            iso_tile_index: cell.final_tile_index,
            tube_index: cell.tube_index.map(|t| t.0 as i16),
            land_type: cell.yr_cell_land_type,
            state_byte: cell.bridge_facts.state_byte,
        }
    }

    // --- Flag predicates (L1 / C1-C3) ----------------------------------------

    /// `0x100` — authoritative structural high-bridge cell.
    #[inline]
    pub fn is_bridge_cell(&self) -> bool {
        self.flags.structural()
    }

    /// `0x200` — bridgehead/transition (on/off-ramp boundary) cell.
    #[inline]
    pub fn is_bridgehead(&self) -> bool {
        self.flags.bridgehead()
    }

    /// `0x80` — anchor cell of the stamp.
    #[inline]
    pub fn is_anchor(&self) -> bool {
        self.flags.anchor()
    }

    // --- Effective height (L2 / C4) ------------------------------------------

    /// Signed level plus the deck height for anchor cells.
    ///
    /// This is the `(i8)level + ((flags >> 7) & 1) * 4` form: the level read is
    /// signed and an anchor adds exactly the deck height. It is intentionally NOT
    /// the layer-driven `effective_cell_z_for_layer` form (which keys off the
    /// mover's current layer instead of the anchor flag).
    #[inline]
    pub fn effective_height(&self) -> i32 {
        self.level as i32 + if self.is_anchor() { BRIDGE_DECK_HEIGHT_LEVELS } else { 0 }
    }

    // --- Tileset windows (L3 / L4 / C5) --------------------------------------
    //
    // These are tile-id range checks, completely DISTINCT from the structural
    // `0x100` flag. Conflating a tileset window with structural is DRIFT #6, so
    // the windows are their own predicates and never alias `is_bridge_cell`.

    /// Concrete-bridge tileset window `[base, base + 0x10)`, gated on `base >= 0`.
    /// `base` is the theater-loaded `g_BridgeSet_TileSetBase` equivalent (passed
    /// in because it is theater state, not cell-local). Returns `false` when no
    /// concrete-bridge set is loaded (`base == None` / `< 0`).
    #[inline]
    pub fn is_bridge_tileset(&self, base: Option<i32>) -> bool {
        base.is_some_and(|b| b >= 0 && (b..b + BRIDGE_TILESET_WINDOW).contains(&self.iso_tile_index))
    }

    /// Wood-bridge tileset window `[wood_base, wood_base + 0x10)`, gated on
    /// `wood_base >= 0`. Distinct from the concrete window AND from structural.
    /// `wood_base` is the theater-loaded `g_WoodBridgeSet_TileSetBase` equivalent.
    ///
    /// NOTE: the canonical store already precomputes the wood-window membership
    /// for `final_tile_index` as `ResolvedTerrainCell.is_wood_bridge_repair_tile`
    /// (the CABHUT repair-dispatch predicate). When a `CellBridgeView` is built
    /// from a resolved cell, prefer routing through that precompute rather than
    /// re-deriving the window with a re-passed base (single-source). This method
    /// exists for callers that hold only the tile id + base.
    #[inline]
    pub fn is_wood_bridge_tileset(&self, wood_base: Option<i32>) -> bool {
        wood_base
            .is_some_and(|b| b >= 0 && (b..b + BRIDGE_TILESET_WINDOW).contains(&self.iso_tile_index))
    }

    // --- Low bridge / tube (L5 / C6) -----------------------------------------

    /// Low-bridge/tube cell: BOTH a valid tube index in `[0, tube_count)` AND a
    /// LandType of `YR_CELL_LAND_TUNNEL` (10). Both conditions are required —
    /// either alone is not a tube cell. This is the low-bridge tube, NOT
    /// subterranean/tunnel (TS-legacy, not modelled).
    #[inline]
    pub fn is_low_bridge_cell(&self, tube_count: usize) -> bool {
        self.tube_index
            .is_some_and(|t| t >= 0 && (t as usize) < tube_count)
            && self.land_type == YR_CELL_LAND_TUNNEL
    }
}

// --- P2: service-facing traversal-gate handle --------------------------------
//
// The binary-shaped traversal gate already exists and is correct in
// `pathfinding::core`. This slice does NOT relocate it (that flip is hash-relevant
// and out of scope here); it only re-exports the existing owner at crate
// visibility under a service-facing alias so combat/occupancy could reach the
// same gate the same way A*/runtime do. The pathfinding owner stays authoritative;
// this is a delegating handle that proves the seam is identical (see the shadow
// test below), not a second implementation.
//
// Visibility note: the gate and its input/result types are `pub(crate)` in
// pathfinding, so this handle is `pub(crate)` too — it cannot be made more public
// than its owner, and making it so would be the authority-flip this slice avoids.
pub(crate) use crate::sim::pathfinding::{
    BridgeTraversalInput, BridgeTraversalResult, check_bridge_traversal as bridge_traversal_gate,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::bridge_facts::{
        BRIDGE_FLAG_ANCHOR_SELF, BRIDGE_FLAG_STRUCTURAL, BRIDGE_FLAG_TRANSITION,
    };
    use crate::sim::pathfinding::PathGrid;

    /// Build a view directly from raw fields (bypasses `from_resolved` so a test
    /// can pin a precise flag/level/tile combination without a full terrain cell).
    fn view(level: i8, raw_flags: u32, iso_tile_index: i32) -> CellBridgeView {
        CellBridgeView {
            level,
            flags: BridgeFlags(raw_flags),
            ramp_byte: 0,
            iso_tile_index,
            tube_index: None,
            land_type: 0,
            state_byte: 0,
        }
    }

    #[test]
    fn bridge_topology_predicates_match_pathcell() {
        // Shadow assert-equal: for a battery of raw-flag fixtures, the view's
        // flag predicates must agree with the canonical `BridgeFlags` consts the
        // pathfinding/load views also read.
        for raw in [
            0u32,
            BRIDGE_FLAG_STRUCTURAL,
            BRIDGE_FLAG_TRANSITION,
            BRIDGE_FLAG_ANCHOR_SELF,
            BRIDGE_FLAG_STRUCTURAL | BRIDGE_FLAG_TRANSITION | BRIDGE_FLAG_ANCHOR_SELF,
        ] {
            let v = view(2, raw, 0);
            assert_eq!(v.is_bridge_cell(), raw & BRIDGE_FLAG_STRUCTURAL != 0);
            assert_eq!(v.is_bridgehead(), raw & BRIDGE_FLAG_TRANSITION != 0);
            assert_eq!(v.is_anchor(), raw & BRIDGE_FLAG_ANCHOR_SELF != 0);
        }
    }

    #[test]
    fn effective_height_anchor_plus4_signed_level() {
        // L2: signed level + exactly the deck height for an anchor; NOT the layer
        // form. Feed a raw level byte > 127 (0xFE) to prove the i8 reinterpret.
        let raw_level = 0xFEu8 as i8; // -2
        assert_eq!(raw_level, -2, "0xFE must reinterpret as -2");

        let anchor = view(raw_level, BRIDGE_FLAG_ANCHOR_SELF, 0);
        assert_eq!(anchor.effective_height(), -2 + 4, "anchor adds the deck height");

        let non_anchor = view(raw_level, 0, 0);
        assert_eq!(non_anchor.effective_height(), -2, "non-anchor is the bare level");
    }

    #[test]
    fn is_bridge_tileset_distinct_from_structural_flag() {
        // DRIFT #6: a cell whose tile id falls in the bridge window but which has
        // NO structural flag is a tileset hit, NOT a structural-bridge cell. The
        // two predicates must never alias.
        let base = Some(100);
        let v = view(0, 0, 105); // in [100, 116), structural flag clear
        assert!(v.is_bridge_tileset(base));
        assert!(!v.is_bridge_cell());

        // Boundary: base + 0x10 is exclusive.
        assert!(view(0, 0, 100).is_bridge_tileset(base)); // lower bound inclusive
        assert!(view(0, 0, 115).is_bridge_tileset(base)); // last in-window tile
        assert!(!view(0, 0, 116).is_bridge_tileset(base)); // upper bound exclusive
        assert!(!view(0, 0, 99).is_bridge_tileset(base)); // below window

        // No set loaded -> never a tileset bridge.
        assert!(!view(0, 0, 105).is_bridge_tileset(None));
        assert!(!view(0, 0, 105).is_bridge_tileset(Some(-1)));
    }

    #[test]
    fn is_wood_bridge_tileset_distinct_from_concrete_and_structural() {
        // L4: the wood window is distinct from the concrete window AND from
        // structural. A tile in the wood window but outside the concrete window
        // and without the structural flag is wood-only.
        let concrete_base = Some(100);
        let wood_base = Some(200);
        let v = view(0, 0, 205); // in wood window, outside concrete window
        assert!(v.is_wood_bridge_tileset(wood_base));
        assert!(!v.is_bridge_tileset(concrete_base));
        assert!(!v.is_bridge_cell());
    }

    #[test]
    fn is_low_bridge_requires_landtype10_and_tube_in_range() {
        // L5: BOTH conditions required.
        let tube_count = 4;
        let with = |tube: Option<i16>, land: u8| CellBridgeView {
            level: 0,
            flags: BridgeFlags(0),
            ramp_byte: 0,
            iso_tile_index: 0,
            tube_index: tube,
            land_type: land,
            state_byte: 0,
        };

        // tube in range + land 10 -> true
        assert!(with(Some(0), YR_CELL_LAND_TUNNEL).is_low_bridge_cell(tube_count));
        assert!(with(Some(3), YR_CELL_LAND_TUNNEL).is_low_bridge_cell(tube_count));
        // tube in range + wrong land -> false
        assert!(!with(Some(2), 9).is_low_bridge_cell(tube_count));
        // tube out of range + land 10 -> false
        assert!(!with(Some(4), YR_CELL_LAND_TUNNEL).is_low_bridge_cell(tube_count));
        // negative tube index (signed compare) + land 10 -> false
        assert!(!with(Some(-1), YR_CELL_LAND_TUNNEL).is_low_bridge_cell(tube_count));
        // no tube + land 10 -> false
        assert!(!with(None, YR_CELL_LAND_TUNNEL).is_low_bridge_cell(tube_count));
    }

    #[test]
    fn service_gate_handle_is_bit_identical_to_pathfinding_gate() {
        // P2 shadow: the service-facing handle must produce the exact same
        // `BridgeTraversalResult` as calling the pathfinding owner directly, for a
        // direction == -1 candidate-only seed over a 1x1 structural fixture. This
        // proves the delegating seam is identical without relocating the gate.
        use crate::sim::pathfinding::PathCell;

        let candidate = PathCell {
            ground_walkable: true,
            bridge_walkable: true,
            bridge_structural: true,
            bridge_marker_0x80: false,
            transition: false,
            ground_level: 2,
            bridge_deck_level: 6,
            slope_type: 0,
            tube_index: None,
            low_bridge_tube_cell: false,
        };
        let grid: PathGrid = PathGrid::from_cells(vec![candidate], 1, 1);

        let input = BridgeTraversalInput {
            candidate: grid.cell(0, 0).unwrap(),
            candidate_coord: (0, 0),
            direction: -1,
            path_height: -1,
            parent: None,
        };

        let via_service: BridgeTraversalResult = bridge_traversal_gate(&grid, input);
        let via_owner =
            crate::sim::pathfinding::check_bridge_traversal(&grid, input);

        assert_eq!(via_service, via_owner, "service handle must equal the owner");
        assert!(via_service.allowed);
        assert_eq!(via_service.path_height, 6); // signed_level(2) + 4
        assert!(!via_service.force_bridge_list);
    }
}
