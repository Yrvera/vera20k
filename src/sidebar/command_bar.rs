//! Bottom command-bar layout and command identities.
//!
//! Original layout `72FC60`, draw `6D0A20`, and UIMD ButtonList reader
//! `674650`. Assets own dimensions; the bar tiles at integer native pixels.

use super::Rect;

pub const COMMAND_NAMES: [&str; 11] = [
    "Team01",
    "Team02",
    "Team03",
    "TypeSelect",
    "Deploy",
    "AttackMove",
    "Guard",
    "Beacon",
    "Stop",
    "PlanningMode",
    "Cheer",
];

/// UIMD.INI maps command identities to slots, rather than numbering artwork
/// by slot. Unknown entries leave a slot unused, as the native name search does.
pub fn parse_button_list(value: &str) -> Vec<Option<usize>> {
    let mut slots: Vec<_> = value
        .split(',')
        .map(|name| {
            COMMAND_NAMES
                .iter()
                .position(|known| known.eq_ignore_ascii_case(name.trim()))
        })
        .collect();
    // 674650 stores command -> slot; a repeated name replaces its earlier slot.
    for i in 0..slots.len() {
        if slots[i].is_some() && slots[i + 1..].contains(&slots[i]) {
            slots[i] = None;
        }
    }
    slots
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommandBarLayout {
    pub bounds: Rect,
    pub left_cap: Rect,
    pub background: Rect,
    pub right_cap: Rect,
    pub tile_count: usize,
}

impl CommandBarLayout {
    pub fn new(
        screen: [u32; 2],
        sidebar_width: u32,
        left: [u32; 2],
        tile: [u32; 2],
        right: [u32; 2],
        open: bool,
    ) -> Option<Self> {
        let width = screen[0].checked_sub(sidebar_width)?;
        let available = width.checked_sub(left[0])?.checked_sub(right[0])?;
        if tile[0] == 0 || tile[1] == 0 || screen[1] < tile[1] {
            return None;
        }
        let count = available / tile[0];
        let right_x = width - right[0];
        let first_x = right_x - count * tile[0];
        let y = screen[1] - tile[1];
        let rect = |x, size: [u32; 2]| Rect {
            x: x as f32,
            y: y as f32,
            w: size[0] as f32,
            h: size[1] as f32,
        };
        Some(Self {
            bounds: rect(0, [width, tile[1]]),
            left_cap: rect(
                if open {
                    first_x - left[0]
                } else {
                    right_x - left[0]
                },
                left,
            ),
            background: rect(first_x, tile),
            right_cap: rect(right_x, right),
            tile_count: if open { count as usize } else { 0 },
        })
    }

    pub fn slot(self, slot: usize) -> Option<Rect> {
        (slot < self.tile_count).then(|| Rect {
            x: self.background.x + slot as f32 * self.background.w,
            ..self.background
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_identity_keeps_unknown_slots_and_last_duplicate() {
        assert_eq!(
            parse_button_list("Team01,unknown,Deploy,team01"),
            [None, None, Some(4), Some(0)]
        );
    }
    #[test]
    fn original_800_by_600_command_bar_layout() {
        // Original complete72FC60, retail canvas headers: tools/sidebar_oracle/command_bar/layout-native.json.
        let layout =
            CommandBarLayout::new([800, 600], 168, [28, 32], [52, 32], [28, 32], true).unwrap();
        assert_eq!(layout.tile_count, 11);
        assert_eq!(
            layout.bounds,
            Rect {
                x: 0.,
                y: 568.,
                w: 632.,
                h: 32.
            }
        );
        assert_eq!(layout.left_cap.x, 4.);
        assert_eq!(layout.background.x, 32.);
        assert_eq!(layout.right_cap.x, 604.);
        assert_eq!(
            CommandBarLayout::new([800, 600], 168, [28, 32], [52, 32], [28, 32], false)
                .unwrap()
                .left_cap
                .x,
            576.
        );
    }
}
