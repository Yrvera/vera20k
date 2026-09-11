//! Live high-body continuation and CellClass setter publication.
//!
//! Native576BA0/47E040; evidence: bridge_body_publication native corpus and
//! HIGH_BRIDGE_RIM_REFRESH_ALGORITHM_GHIDRA_REPORT.md. Scalar writes must not
//! dispatch extra callbacks. The host keeps world authorities resident while
//! fallout, perpendicular helpers, rim and zone work execute synchronously.
//! The production host is world/bridge_publication.rs; its existing tile/rim
//! callback projections remain explicitly outside this core's parity claim.

use super::{Axis, Phase};

#[cfg(test)]
#[path = "publication_tests.rs"]
mod tests;

pub(crate) type CellCoord = (i16, i16);

pub(crate) trait BridgePublicationHost {
    /// Stable allocation identity; a shared dummy remains the same identity
    /// when another lookup changes its coordinate.
    type Cell: Copy;
    fn lookup(&mut self, coord: CellCoord) -> Self::Cell;
    fn coord(&self, cell: Self::Cell) -> CellCoord;
    fn flags(&self, cell: Self::Cell) -> u32;
    fn state(&self, cell: Self::Cell) -> u8;
    fn write_flags(&mut self, cell: Self::Cell, flags: u32);
    fn write_state(&mut self, cell: Self::Cell, state: u8);
    /// Literal native +2C pointer, distinct from a derived self relation.
    fn write_anchor(&mut self, cell: Self::Cell, anchor: Option<Self::Cell>);
    /// Literal +44=-1, without an implicit Recalc or zone callback.
    fn clear_overlay(&mut self, cell: Self::Cell);
    fn fallout(&mut self, cell: Self::Cell);
    fn radar(&mut self, cell: Self::Cell);
    fn perpendicular(&mut self, coord: CellCoord, axis: Axis, phase: Phase, direction: u8);
    fn rim(&mut self, coord: CellCoord);
    fn zones(&mut self, anchor: Self::Cell);
}

fn step(coord: CellCoord, direction: u8) -> CellCoord {
    let (dx, dy) = crate::util::direction::DIRECTION_DELTAS[usize::from(direction & 7)];
    (
        coord.0.wrapping_add(dx as i16),
        coord.1.wrapping_add(dy as i16),
    )
}

fn finish_slot<H: BridgePublicationHost>(host: &mut H, cell: H::Cell, direction: u8, set: bool) {
    host.write_state(cell, if set && direction != 0 { 9 } else { 0 });
    if !set {
        host.fallout(cell);
    }
    host.radar(cell);
}

/// Original47E040's body/ramp callers supply direction0 or6 and state0/1.
/// Every following lookup uses current +24 after the preceding callback.
pub(crate) fn set_bridge_direction<H: BridgePublicationHost>(
    host: &mut H,
    anchor: H::Cell,
    direction: u8,
    set: bool,
) {
    debug_assert!(matches!(direction, 0 | 6));
    let intact = u32::from(set);
    let structural = intact << 8;
    let transition = intact << 9;
    let forward = intact << 12;
    let extra = intact << 16;
    let destroyed = u32::from(!set) << 10;
    let direction_zero = u32::from(direction == 0) << 11;

    // The first +11E store precedes the anchor flag word, even for destruction.
    host.write_state(anchor, if direction == 0 { 0 } else { 9 });
    host.write_flags(
        anchor,
        (host.flags(anchor) & 0xfffe_e07f)
            | structural
            | transition
            | forward
            | extra
            | destroyed
            | (intact << 7)
            | direction_zero,
    );
    finish_slot(host, anchor, direction, set);

    let mut cell = host.lookup(step(host.coord(anchor), direction));
    host.write_anchor(cell, set.then_some(anchor));
    host.write_flags(
        cell,
        ((host.flags(cell) & 0xfffe_e8ff) | structural | transition | forward | extra | destroyed)
            & 0xffff_f7ff
            | direction_zero,
    );
    finish_slot(host, cell, direction, set);

    cell = host.lookup(step(host.coord(cell), direction));
    host.write_anchor(cell, set.then_some(anchor));
    host.write_flags(
        cell,
        ((host.flags(cell) & 0xfffe_e8ff) | structural | forward | extra | destroyed) & 0xffff_f7ff
            | direction_zero,
    );
    finish_slot(host, cell, direction, set);

    cell = host.lookup(step(host.coord(cell), direction));
    host.write_flags(cell, (host.flags(cell) & 0xffff_efff) | forward);

    // Opposite is relative to the retained anchor's current coordinate, not
    // the original input coordinate or the third forward lookup.
    cell = host.lookup(step(host.coord(anchor), direction.wrapping_sub(4) & 7));
    host.write_anchor(cell, set.then_some(anchor));
    host.write_flags(
        cell,
        ((host.flags(cell) & 0xffff_f8ff) | structural | transition | destroyed) & 0xfffe_e7ff
            | extra
            | direction_zero,
    );
    finish_slot(host, cell, direction, set);
    if direction == 6 {
        cell = host.lookup(step(host.coord(cell), 2));
        host.write_anchor(cell, set.then_some(anchor));
        host.write_flags(cell, (host.flags(cell) & 0xfffe_ffff) | extra);
    }
}

/// Original576BA0's structural-body branch, after its caller has resolved the
/// stable self/+2C anchor. State/axis is selected once before any callback.
/// Returns native0 for damage/no transition and1 for collapse.
pub(crate) fn advance_body_at_anchor<H: BridgePublicationHost>(
    host: &mut H,
    input: CellCoord,
    anchor: H::Cell,
) -> bool {
    let state = host.state(anchor);
    let (axis, first_direction, second_direction, setter_direction) = if state <= 8 {
        (Axis::NS, 2, 6, 0)
    } else {
        (Axis::EW, 4, 0, 6)
    };
    let (first, second, collapse) = match state {
        0..=5 | 9..=14 => {
            host.write_state(anchor, if state <= 8 { 6 } else { 15 });
            (Some(Phase::DamageA), Some(Phase::DamageB), false)
        }
        6 | 15 => (Some(Phase::CollapseA), Some(Phase::CollapseB), true),
        7 | 17 => (Some(Phase::CollapseA), None, true),
        8 | 16 => (None, Some(Phase::CollapseB), true),
        _ => return false,
    };
    if let Some(phase) = first {
        host.perpendicular(host.coord(anchor), axis, phase, first_direction);
    }
    if let Some(phase) = second {
        host.perpendicular(host.coord(anchor), axis, phase, second_direction);
    }
    if collapse {
        set_bridge_direction(host, anchor, setter_direction, false);
        host.write_state(anchor, 0);
        host.clear_overlay(anchor);
        host.rim(input);
        host.zones(anchor);
    }
    collapse
}
