//! Shared Infantry51BF90 entry inputs and the Foot4D9C60 height/list prelude.
//! The failure receiver consumes zero/nonzero, repair consumes seven, and the
//! Foot path wrapper distinguishes six. Keep those answers in one owner.

use crate::map::cell_index::NativeCellIdentity as Cell;
use crate::map::resolved_terrain::{NativeCellQuery, ResolvedTerrainGrid};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InfantryEntryClass {
    Clear,
    SoftOther,
    Obstructed6,
    Impassable7,
}

impl InfantryEntryClass {
    pub(crate) fn from_raw(code: u8) -> Self {
        match code {
            0 => Self::Clear,
            1..=5 => Self::SoftOther,
            6 => Self::Obstructed6,
            7 => Self::Impassable7,
            _ => unreachable!("Infantry51BF90 result domain"),
        }
    }

    pub(crate) fn is_nonzero(self) -> bool {
        self != Self::Clear
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct InfantryEntryArgs {
    pub(crate) direction: i32,
    pub(crate) height: i32,
    pub(crate) previous_cell: Option<Cell>,
}

impl InfantryEntryArgs {
    pub(crate) const REPAIR: Self = Self {
        direction: -1,
        height: -1,
        previous_cell: None,
    };
}

fn level_slope(terrain: &ResolvedTerrainGrid, cell: Cell) -> (i32, u8) {
    match cell {
        Cell::Real(index) => {
            let cell = &terrain.cells()[index];
            (i32::from(cell.level as i8), cell.slope_type)
        }
        Cell::Dummy => {
            let state = terrain.shared_cell_dummy().snapshot();
            (i32::from(state.level), state.slope_type)
        }
    }
}

pub(crate) fn backstep_cell(cells: &NativeCellQuery<'_>, target: Cell, direction: i32) -> Cell {
    let coord = cells.coord(target);
    let (dx, dy) = crate::util::direction::DIRECTION_DELTAS[((direction - 4) & 7) as usize];
    cells.lookup((
        coord.0.wrapping_add(dx as i16),
        coord.1.wrapping_add(dy as i16),
    ))
}

/// Foot+1B0. The caller has already performed its separate GetNeighbour /
/// GetTube query. A NULL previous Cell performs this second lookup, preserving
/// retained Dummy aliases and the exact mutable height/object-list outputs.
pub(crate) fn adjust_height_and_list(
    terrain: &ResolvedTerrainGrid,
    target: Cell,
    direction: i32,
    previous: Option<Cell>,
    height: &mut i32,
    list_bridge: &mut bool,
) -> bool {
    let cells = NativeCellQuery::canonical(terrain);
    let source = previous.unwrap_or_else(|| backstep_cell(&cells, target, direction));
    let target_level = level_slope(terrain, target).0;
    if direction == -1 {
        if *height == -1 && cells.flags(target) & 0x100 != 0 {
            *height = target_level + 4;
        }
        return true;
    }
    let (source_level, source_slope) = level_slope(terrain, source);
    if *height == -1 && cells.flags(source) & 0x100 != 0 {
        *height = source_level + 4;
        if cells.flags(target) & 0x200 == 0 {
            return false;
        }
    }
    let difference = if cells.flags(source) & 0x100 != 0 {
        source_level
    } else {
        *height
    } - target_level;
    match difference.abs() {
        0 => {
            if cells.flags(target) & 0x300 == 0x300 && cells.flags(source) & 0x100 != 0 {
                return true;
            }
            *height == -1 || *height == target_level
        }
        1 => {
            if difference > 0 {
                level_slope(terrain, target).1 != 0
            } else {
                source_slope != 0
            }
        }
        4 => {
            if source_level == target_level - 4
                && (*height != target_level || cells.flags(source) & 0x100 == 0)
            {
                return false;
            }
            if target_level == source_level - 4 {
                if cells.flags(target) & 0x300 != 0x300 {
                    return false;
                }
                *list_bridge = true;
            }
            true
        }
        _ => false,
    }
}
