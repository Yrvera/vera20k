//! Authoritative high-bridge cell facts stamped from map overlay data.

pub const BRIDGE_FLAG_ANCHOR_SELF: u32 = 0x80;
pub const BRIDGE_FLAG_STRUCTURAL: u32 = 0x100;
pub const BRIDGE_FLAG_TRANSITION: u32 = 0x200;
pub const BRIDGE_FLAG_DESTROYED_OR_RAMP: u32 = 0x400;
pub const BRIDGE_FLAG_DIRECTION_ZERO: u32 = 0x800;
pub const BRIDGE_FLAG_FORWARD_SIDE: u32 = 0x1000;
pub const BRIDGE_FLAG_EXTRA_SIDE: u32 = 0x10000;

/// Typed view of the CellClass flag word, single-sourced from the consts above.
///
/// Bit values are NOT redefined here — every predicate references the
/// `BRIDGE_FLAG_*` consts so map-load (this file), the topology service, and the
/// render draw-offset trait all read one source of truth. A thin newtype keeps
/// the existing const style (no `bitflags!` dep) while giving the topology
/// service a borrowable typed handle instead of a raw `u32`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BridgeFlags(pub u32);

impl BridgeFlags {
    #[inline]
    pub fn has(self, bit: u32) -> bool {
        self.0 & bit != 0
    }
    /// `0x80` — anchor cell of the stamp (adds the deck height in effective-Z).
    #[inline]
    pub fn anchor(self) -> bool {
        self.has(BRIDGE_FLAG_ANCHOR_SELF)
    }
    /// `0x100` — authoritative structural high-bridge cell. Distinct from the
    /// concrete/wood tileset windows (those are tile-id ranges, not this flag).
    #[inline]
    pub fn structural(self) -> bool {
        self.has(BRIDGE_FLAG_STRUCTURAL)
    }
    /// `0x200` — bridgehead/transition cell (the on/off-ramp boundary).
    #[inline]
    pub fn bridgehead(self) -> bool {
        self.has(BRIDGE_FLAG_TRANSITION)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BridgeStampFamily {
    #[default]
    None,
    Nesw,
    Nwse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeStampSlot {
    Anchor,
    Forward1,
    Forward2,
    Forward3,
    Opposite,
    ExtraDir6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BridgeAnchorRelation {
    pub anchor: (u16, u16),
    pub slot: BridgeStampSlot,
    pub family: BridgeStampFamily,
    pub direction: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeRampKind {
    TopRight,
    TopLeft,
    Middle1,
    Middle2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BridgeRampTile {
    pub kind: BridgeRampKind,
    pub relative_tile_index: u16,
    pub height_byte: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BridgeCellFacts {
    pub raw_flags: u32,
    pub state_byte: u8,
    pub overlay_id: Option<u8>,
    pub family: BridgeStampFamily,
    pub direction: Option<u8>,
    pub anchor: Option<BridgeAnchorRelation>,
    pub ramp_tile: Option<BridgeRampTile>,
}

impl BridgeCellFacts {
    pub fn has_flag(self, flag: u32) -> bool {
        self.raw_flags & flag != 0
    }

    pub fn has_structural_bridge(self) -> bool {
        self.has_flag(BRIDGE_FLAG_STRUCTURAL)
    }

    pub fn has_transition_flag(self) -> bool {
        self.has_flag(BRIDGE_FLAG_TRANSITION)
    }

    pub fn is_anchor_self(self) -> bool {
        self.has_flag(BRIDGE_FLAG_ANCHOR_SELF)
    }
}

pub fn high_bridge_stamp_for_overlay(id: u8) -> Option<(BridgeStampFamily, u8)> {
    match id {
        0x18 => Some((BridgeStampFamily::Nesw, 0)),
        0x19 => Some((BridgeStampFamily::Nesw, 6)),
        0xED => Some((BridgeStampFamily::Nwse, 0)),
        0xEE => Some((BridgeStampFamily::Nwse, 6)),
        _ => None,
    }
}

pub fn stamp_set_bridge_direction(
    cells: &mut [BridgeCellFacts],
    width: u16,
    height: u16,
    anchor: (u16, u16),
    family: BridgeStampFamily,
    direction: u8,
    set: bool,
) {
    if crate::util::direction::direction_delta(direction).is_none() {
        return;
    }
    let slots = stamp_slots(anchor, direction);
    for (slot, pos) in slots {
        let Some((rx, ry)) = pos else {
            continue;
        };
        let Some(idx) = index(width, height, rx, ry) else {
            continue;
        };
        if idx >= cells.len() {
            continue;
        }
        let relation = BridgeAnchorRelation {
            anchor,
            slot,
            family,
            direction,
        };
        if set {
            stamp_intact(&mut cells[idx], slot, relation);
        } else {
            stamp_destroy(&mut cells[idx], slot);
        }
    }
}

fn stamp_intact(cell: &mut BridgeCellFacts, slot: BridgeStampSlot, relation: BridgeAnchorRelation) {
    let direction_zero = relation.direction == 0;
    match slot {
        BridgeStampSlot::Anchor => {
            cell.raw_flags &= !BRIDGE_FLAG_DESTROYED_OR_RAMP;
            cell.raw_flags |= BRIDGE_FLAG_ANCHOR_SELF
                | BRIDGE_FLAG_STRUCTURAL
                | BRIDGE_FLAG_TRANSITION
                | BRIDGE_FLAG_FORWARD_SIDE
                | BRIDGE_FLAG_EXTRA_SIDE;
            set_direction_zero_flag(cell, direction_zero);
            write_default_state(cell, relation.direction);
            attach(cell, relation);
        }
        BridgeStampSlot::Forward1 => {
            cell.raw_flags &= !BRIDGE_FLAG_DESTROYED_OR_RAMP;
            cell.raw_flags |= BRIDGE_FLAG_STRUCTURAL
                | BRIDGE_FLAG_TRANSITION
                | BRIDGE_FLAG_FORWARD_SIDE
                | BRIDGE_FLAG_EXTRA_SIDE;
            set_direction_zero_flag(cell, direction_zero);
            write_default_state(cell, relation.direction);
            attach(cell, relation);
        }
        BridgeStampSlot::Forward2 => {
            cell.raw_flags &= !(BRIDGE_FLAG_TRANSITION | BRIDGE_FLAG_DESTROYED_OR_RAMP);
            cell.raw_flags |=
                BRIDGE_FLAG_STRUCTURAL | BRIDGE_FLAG_FORWARD_SIDE | BRIDGE_FLAG_EXTRA_SIDE;
            set_direction_zero_flag(cell, direction_zero);
            write_default_state(cell, relation.direction);
            attach(cell, relation);
        }
        BridgeStampSlot::Forward3 => {
            cell.raw_flags |= BRIDGE_FLAG_FORWARD_SIDE;
        }
        BridgeStampSlot::Opposite => {
            cell.raw_flags &= !(BRIDGE_FLAG_DESTROYED_OR_RAMP | BRIDGE_FLAG_FORWARD_SIDE);
            cell.raw_flags |=
                BRIDGE_FLAG_STRUCTURAL | BRIDGE_FLAG_TRANSITION | BRIDGE_FLAG_EXTRA_SIDE;
            set_direction_zero_flag(cell, direction_zero);
            write_default_state(cell, relation.direction);
            attach(cell, relation);
        }
        BridgeStampSlot::ExtraDir6 => {
            cell.raw_flags &= !BRIDGE_FLAG_EXTRA_SIDE;
            cell.raw_flags |= BRIDGE_FLAG_EXTRA_SIDE;
            attach(cell, relation);
        }
    }
}

fn stamp_destroy(cell: &mut BridgeCellFacts, slot: BridgeStampSlot) {
    match slot {
        BridgeStampSlot::Anchor => {
            cell.raw_flags &= !(BRIDGE_FLAG_ANCHOR_SELF
                | BRIDGE_FLAG_STRUCTURAL
                | BRIDGE_FLAG_TRANSITION
                | BRIDGE_FLAG_DIRECTION_ZERO
                | BRIDGE_FLAG_FORWARD_SIDE
                | BRIDGE_FLAG_EXTRA_SIDE);
            cell.raw_flags |= BRIDGE_FLAG_DESTROYED_OR_RAMP;
            cell.state_byte = 0;
            detach(cell);
        }
        BridgeStampSlot::Forward1 | BridgeStampSlot::Forward2 | BridgeStampSlot::Opposite => {
            cell.raw_flags &= !(BRIDGE_FLAG_STRUCTURAL
                | BRIDGE_FLAG_TRANSITION
                | BRIDGE_FLAG_DIRECTION_ZERO
                | BRIDGE_FLAG_FORWARD_SIDE
                | BRIDGE_FLAG_EXTRA_SIDE);
            cell.raw_flags |= BRIDGE_FLAG_DESTROYED_OR_RAMP;
            cell.state_byte = 0;
            detach(cell);
        }
        BridgeStampSlot::Forward3 => {
            cell.raw_flags &= !BRIDGE_FLAG_FORWARD_SIDE;
        }
        BridgeStampSlot::ExtraDir6 => {
            cell.raw_flags &= !BRIDGE_FLAG_EXTRA_SIDE;
            detach(cell);
        }
    }
}

fn attach(cell: &mut BridgeCellFacts, relation: BridgeAnchorRelation) {
    cell.family = relation.family;
    cell.direction = Some(relation.direction);
    cell.anchor = Some(relation);
}

fn detach(cell: &mut BridgeCellFacts) {
    cell.family = BridgeStampFamily::None;
    cell.direction = None;
    cell.anchor = None;
}

fn write_default_state(cell: &mut BridgeCellFacts, direction: u8) {
    cell.state_byte = if direction == 0 { 0 } else { 9 };
}

fn set_direction_zero_flag(cell: &mut BridgeCellFacts, set: bool) {
    if set {
        cell.raw_flags |= BRIDGE_FLAG_DIRECTION_ZERO;
    } else {
        cell.raw_flags &= !BRIDGE_FLAG_DIRECTION_ZERO;
    }
}

fn stamp_slots(anchor: (u16, u16), direction: u8) -> [(BridgeStampSlot, Option<(u16, u16)>); 6] {
    let f1 = step(anchor, direction);
    let f2 = f1.and_then(|cell| step(cell, direction));
    let f3 = f2.and_then(|cell| step(cell, direction));
    let opposite = crate::util::direction::opposite_direction(direction)
        .and_then(|opposite| step(anchor, opposite));
    let extra = if direction == 6 {
        opposite.and_then(|cell| step(cell, 2))
    } else {
        None
    };
    [
        (BridgeStampSlot::Anchor, Some(anchor)),
        (BridgeStampSlot::Forward1, f1),
        (BridgeStampSlot::Forward2, f2),
        (BridgeStampSlot::Forward3, f3),
        (BridgeStampSlot::Opposite, opposite),
        (BridgeStampSlot::ExtraDir6, extra),
    ]
}

fn index(width: u16, height: u16, rx: u16, ry: u16) -> Option<usize> {
    if rx >= width || ry >= height {
        return None;
    }
    Some(ry as usize * width as usize + rx as usize)
}

fn step(cell: (u16, u16), direction: u8) -> Option<(u16, u16)> {
    let (dx, dy) = crate::util::direction::direction_delta(direction)?;
    let rx = cell.0 as i32 + dx;
    let ry = cell.1 as i32 + dy;
    if rx < 0 || ry < 0 || rx > u16::MAX as i32 || ry > u16::MAX as i32 {
        return None;
    }
    Some((rx as u16, ry as u16))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts_at(cells: &[BridgeCellFacts], width: u16, rx: u16, ry: u16) -> BridgeCellFacts {
        cells[ry as usize * width as usize + rx as usize]
    }

    #[test]
    fn stamp_dir0_intact_sets_anchor_north_slots_and_south_opposite() {
        let width = 12;
        let mut cells = vec![BridgeCellFacts::default(); 12 * 12];
        stamp_set_bridge_direction(
            &mut cells,
            width as u16,
            12,
            (5, 5),
            BridgeStampFamily::Nesw,
            0,
            true,
        );

        assert!(facts_at(&cells, width as u16, 5, 5).has_flag(BRIDGE_FLAG_ANCHOR_SELF));
        assert!(facts_at(&cells, width as u16, 5, 4).has_structural_bridge());
        assert!(facts_at(&cells, width as u16, 5, 3).has_structural_bridge());
        assert!(facts_at(&cells, width as u16, 5, 2).has_flag(BRIDGE_FLAG_FORWARD_SIDE));
        assert!(!facts_at(&cells, width as u16, 5, 2).has_structural_bridge());
        assert!(facts_at(&cells, width as u16, 5, 6).has_structural_bridge());
        assert!(facts_at(&cells, width as u16, 5, 6).has_transition_flag());
    }

    #[test]
    fn stamp_dir6_intact_sets_west_slots_and_two_east_slots() {
        let width = 12;
        let mut cells = vec![BridgeCellFacts::default(); 12 * 12];
        stamp_set_bridge_direction(
            &mut cells,
            width as u16,
            12,
            (5, 5),
            BridgeStampFamily::Nesw,
            6,
            true,
        );

        assert!(facts_at(&cells, width as u16, 4, 5).has_structural_bridge());
        assert!(facts_at(&cells, width as u16, 4, 5).has_transition_flag());
        assert!(facts_at(&cells, width as u16, 3, 5).has_structural_bridge());
        assert!(!facts_at(&cells, width as u16, 3, 5).has_transition_flag());
        assert!(facts_at(&cells, width as u16, 5, 5).has_transition_flag());
        assert!(facts_at(&cells, width as u16, 2, 5).has_flag(BRIDGE_FLAG_FORWARD_SIDE));
        assert!(facts_at(&cells, width as u16, 6, 5).has_structural_bridge());
        assert!(facts_at(&cells, width as u16, 6, 5).has_transition_flag());
        assert!(facts_at(&cells, width as u16, 7, 5).has_flag(BRIDGE_FLAG_EXTRA_SIDE));
        assert!(!facts_at(&cells, width as u16, 7, 5).has_structural_bridge());
    }

    #[test]
    fn invalid_direction_does_not_stamp_any_bridge_cells() {
        let width = 12;
        let mut cells = vec![BridgeCellFacts::default(); 12 * 12];

        stamp_set_bridge_direction(
            &mut cells,
            width as u16,
            12,
            (5, 5),
            BridgeStampFamily::Nesw,
            9,
            true,
        );

        assert!(!facts_at(&cells, width as u16, 5, 5).has_flag(BRIDGE_FLAG_ANCHOR_SELF));
        assert!(!cells.iter().any(|cell| cell.raw_flags != 0));
    }

    #[test]
    fn stamp_intact_writes_default_state_bytes_before_overlay_data_overwrite() {
        let width = 12;
        let mut cells = vec![BridgeCellFacts::default(); 12 * 12];
        stamp_set_bridge_direction(
            &mut cells,
            width as u16,
            12,
            (5, 5),
            BridgeStampFamily::Nesw,
            6,
            true,
        );

        for (rx, ry) in [(5, 5), (4, 5), (3, 5), (6, 5)] {
            assert_eq!(facts_at(&cells, width as u16, rx, ry).state_byte, 9);
        }
        assert_eq!(facts_at(&cells, width as u16, 2, 5).state_byte, 0);
        assert_eq!(facts_at(&cells, width as u16, 7, 5).state_byte, 0);
    }

    #[test]
    fn stamp_intact_sets_0x80_only_on_anchor() {
        let width = 12;
        let mut cells = vec![BridgeCellFacts::default(); 12 * 12];
        stamp_set_bridge_direction(
            &mut cells,
            width as u16,
            12,
            (5, 5),
            BridgeStampFamily::Nesw,
            0,
            true,
        );

        assert!(facts_at(&cells, width as u16, 5, 5).has_flag(BRIDGE_FLAG_ANCHOR_SELF));
        for (rx, ry) in [(5, 4), (5, 3), (5, 2), (5, 6)] {
            assert!(!facts_at(&cells, width as u16, rx, ry).has_flag(BRIDGE_FLAG_ANCHOR_SELF));
        }
    }

    #[test]
    fn stamp_destroy_emits_destroy_flags_only_on_anchor_forward1_forward2_opposite() {
        let width = 12;
        let mut cells = vec![BridgeCellFacts::default(); 12 * 12];
        stamp_set_bridge_direction(
            &mut cells,
            width as u16,
            12,
            (5, 5),
            BridgeStampFamily::Nesw,
            6,
            true,
        );
        stamp_set_bridge_direction(
            &mut cells,
            width as u16,
            12,
            (5, 5),
            BridgeStampFamily::Nesw,
            6,
            false,
        );

        for (rx, ry) in [(5, 5), (4, 5), (3, 5), (6, 5)] {
            assert!(facts_at(&cells, width as u16, rx, ry).has_flag(BRIDGE_FLAG_DESTROYED_OR_RAMP));
        }
        assert!(!facts_at(&cells, width as u16, 2, 5).has_flag(BRIDGE_FLAG_DESTROYED_OR_RAMP));
        assert!(!facts_at(&cells, width as u16, 7, 5).has_flag(BRIDGE_FLAG_DESTROYED_OR_RAMP));
    }

    #[test]
    fn bridge_flags_newtype_matches_const_predicates() {
        // Single-source check: the typed newtype must agree bit-for-bit with the
        // `BridgeCellFacts` predicate path for the same raw flag word, across
        // each individual bit and a combined word.
        for raw in [
            0u32,
            BRIDGE_FLAG_ANCHOR_SELF,
            BRIDGE_FLAG_STRUCTURAL,
            BRIDGE_FLAG_TRANSITION,
            BRIDGE_FLAG_ANCHOR_SELF | BRIDGE_FLAG_STRUCTURAL | BRIDGE_FLAG_TRANSITION,
            BRIDGE_FLAG_STRUCTURAL | BRIDGE_FLAG_FORWARD_SIDE,
        ] {
            let flags = BridgeFlags(raw);
            let facts = BridgeCellFacts {
                raw_flags: raw,
                ..BridgeCellFacts::default()
            };
            assert_eq!(flags.anchor(), facts.is_anchor_self(), "anchor raw={raw:#x}");
            assert_eq!(
                flags.structural(),
                facts.has_structural_bridge(),
                "structural raw={raw:#x}"
            );
            assert_eq!(
                flags.bridgehead(),
                facts.has_transition_flag(),
                "bridgehead raw={raw:#x}"
            );
        }
    }

    #[test]
    fn high_bridge_stamp_classifier_ignores_low_bridge_ids() {
        for id in [0x18, 0x19, 0xED, 0xEE] {
            assert!(high_bridge_stamp_for_overlay(id).is_some());
        }
        for id in [0x4A, 0x7A, 0xCD, 0xE9] {
            assert_eq!(high_bridge_stamp_for_overlay(id), None);
        }
    }
}
