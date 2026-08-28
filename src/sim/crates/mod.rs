//! Active-retail crate runtime.
//!
//! Static crate and Powerups data belongs to `rules/`. This subsystem owns the
//! persistent MapClass slot table, placement/removal lifecycle, pickup
//! transaction, and effects. It never depends on render, UI, audio, or net.

mod effects;
mod placement;
mod pickup;
mod state;

pub use placement::{CratePlacement, human_player_count, scenario_start_crate_count};
pub(crate) use placement::{CratePlacementFaults, place_scenario_start_crates};
pub(crate) use state::{CRATE_SLOT_CAPACITY, CrateAuthority, CrateSlot};
pub(crate) use pickup::{CratePickupInputs, NativePickupReturn};

/// Ordered, transient presentation facts produced by the native crate tails.
/// They are drained at the sim-to-app frame boundary and are neither saved nor
/// hashed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CratePresentationEvent {
    /// Native `TacticalClass__DirtyScreenRect(..., force=0)`. `cell=None` is
    /// the zero-rectangle request retained by accepted placement ghosts.
    DirtyScreenRect {
        cell: Option<(u16, u16)>,
        force: bool,
    },
    /// Native `FUN_006DA7D0` cell redraw request after its app-side visibility,
    /// viewport, suppression, last-frame, and 799-entry gates.
    CellRedraw { cell: (u16, u16), frame: i32 },
}

#[cfg(test)]
mod tests;
