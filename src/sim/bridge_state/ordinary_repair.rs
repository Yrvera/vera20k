//! Original57F200/57F440 and57F6A0/57FBC0/5800D0/580600.
//! The caller retains each three-cell receiver group across synchronous world
//! effects. Evidence: tools/spatial_oracle/bridge_ordinary_repair.{py,json}.

use super::publication::CellCoord;
use super::ramp_repair::{Family, Rect};

pub(crate) trait OrdinaryRepairHost {
    type Cell: Copy;
    type Error;
    fn lookup(&mut self, coord: CellCoord) -> Self::Cell;
    fn overlay(&self, cell: Self::Cell) -> i32;
    fn write_overlay(&mut self, cell: Self::Cell, overlay: u8);
    fn variant(&mut self) -> u8;
    fn redraw(&mut self, cell: Self::Cell);
    fn radar(&mut self, coord: CellCoord);
    fn recalc(&mut self, cell: Self::Cell) -> Result<(), Self::Error>;
    fn occupants(&mut self, cell: Self::Cell) -> Result<(), Self::Error>;
    fn connectivity(&mut self) -> Result<(), Self::Error>;
    fn rebuild_rectangle(&mut self, rect: Rect) -> Result<(), Self::Error>;
}

fn member(overlay: i32, family: Family) -> bool {
    match family {
        Family::Low => (74..=101).contains(&overlay),
        Family::High => (205..=232).contains(&overlay),
    }
}

fn north_south(overlay: i32, family: Family) -> bool {
    match family {
        Family::Low => matches!(overlay, 74..=82 | 92..=95 | 100),
        Family::High => matches!(overlay, 205..=213 | 223..=226 | 231),
    }
}

fn offset(point: CellCoord, x: i16, y: i16) -> CellCoord {
    (point.0.wrapping_add(x), point.1.wrapping_add(y))
}

// The native union expands a newly extended right/bottom edge by one. This
// deliberately differs from a conventional half-open rectangle union.
fn extend_rectangle(old: Rect, new: Rect) -> Rect {
    if old[2] <= 0 || old[3] <= 0 {
        return new;
    }
    let [mut x, mut y, mut w, mut h] = old;
    if x > new[0] {
        w = w.wrapping_add(x.wrapping_sub(new[0]));
        x = new[0];
    }
    if y > new[1] {
        h = h.wrapping_add(y.wrapping_sub(new[1]));
        y = new[1];
    }
    if x.wrapping_add(w) < new[0].wrapping_add(new[2]) {
        w = new[0].wrapping_sub(x).wrapping_add(new[2]).wrapping_add(1);
    }
    if y.wrapping_add(h) < new[1].wrapping_add(new[3]) {
        h = new[1].wrapping_sub(y).wrapping_add(new[3]).wrapping_add(1);
    }
    [x, y, w, h]
}

/// Family selector chooses the middle row/column, then scans backwards along
/// the span. Missing cells retain the native shared dummy identity.
pub(crate) fn repair<H: OrdinaryRepairHost>(
    host: &mut H,
    input: CellCoord,
    family: Family,
) -> Result<(), H::Error> {
    let selected = host.lookup(input);
    let overlay = host.overlay(selected);
    if !member(overlay, family) {
        return Ok(());
    }
    let ns = north_south(overlay, family);
    let across = if ns { (0, -1) } else { (-1, 0) };
    let before = offset(input, across.0, across.1);
    let previous = host.lookup(before);
    let mut point = if !member(host.overlay(previous), family) {
        offset(input, -across.0, -across.1)
    } else {
        let previous = host.lookup(offset(before, across.0, across.1));
        if member(host.overlay(previous), family) {
            before
        } else {
            input
        }
    };
    let along = if ns { (1, 0) } else { (0, 1) };
    loop {
        point = offset(point, -along.0, -along.1);
        let cell = host.lookup(point);
        if !member(host.overlay(cell), family) {
            break;
        }
    }
    point = offset(point, along.0, along.1);
    let mut rectangle = [0; 4];
    let mut connectivity = false;
    loop {
        // All three identities are selected before any store/callback. A
        // shared dummy can therefore be aliased more than once in this group.
        let center = host.lookup(point);
        let negative_coord = offset(point, across.0, across.1);
        let positive_coord = offset(point, -across.0, -across.1);
        let negative = host.lookup(negative_coord);
        let positive = host.lookup(positive_coord);
        let prior = host.overlay(center);
        let base = match (family, ns) {
            (Family::Low, true) => 74,
            (Family::Low, false) => 83,
            (Family::High, true) => 205,
            (Family::High, false) => 214,
        };
        let destroyed = match (family, ns) {
            (Family::Low, true) => 100,
            (Family::Low, false) => 101,
            (Family::High, true) => 231,
            (Family::High, false) => 232,
        };
        let end = match (family, ns) {
            (Family::Low, true) => 92,
            (Family::Low, false) => 96,
            (Family::High, true) => 223,
            (Family::High, false) => 227,
        };
        let next = if (base + 4..=base + 8).contains(&prior) || prior == destroyed {
            let variant = host.variant();
            connectivity = true;
            rectangle = extend_rectangle(
                rectangle,
                if ns {
                    [i32::from(point.0), i32::from(point.1) - 1, 1, 3]
                } else {
                    [i32::from(point.0) - 1, i32::from(point.1), 3, 1]
                },
            );
            Some(base + i32::from(variant))
        } else if (end..end + 4).contains(&prior) {
            Some(end + (prior - end) / 2 * 2)
        } else {
            None
        };
        if let Some(next) = next.filter(|&next| next != prior) {
            for cell in [center, negative, positive] {
                host.write_overlay(cell, next as u8);
            }
            host.redraw(center);
            if prior == destroyed {
                // 6551C0 uses center,+1,-1, independently of Recalc order.
                for coord in [point, positive_coord, negative_coord] {
                    host.radar(coord);
                }
            }
            for cell in [center, negative, positive] {
                host.recalc(cell)?;
            }
            for cell in [center, negative, positive] {
                host.occupants(cell)?;
            }
        }
        point = offset(point, along.0, along.1);
        let next = host.lookup(point);
        if !member(host.overlay(next), family) {
            break;
        }
    }
    if connectivity {
        host.connectivity()?;
    }
    if rectangle[2] > 0 && rectangle[3] > 0 {
        host.rebuild_rectangle(rectangle)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "ordinary_repair_tests.rs"]
mod tests;
