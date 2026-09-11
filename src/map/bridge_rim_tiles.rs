//! Theater identity used by native high-bridge edge cleanup, 576770/576200.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HighBridgeRimTiles {
    pub base: i32,
    pub top_left: [i32; 2],
    pub bottom_right: [i32; 2],
    pub top_right: [i32; 2],
    pub bottom_left: [i32; 2],
    pub middle: [i32; 2],
}

impl HighBridgeRimTiles {
    /// Original ReadTheater545150 reads these ten signed keys independently.
    /// Their values are BridgeSet-relative tile identities, not TileSet indices.
    pub(crate) fn from_theater(theater: &super::theater::TheaterData) -> Self {
        let base = theater
            .bridge_set
            .and_then(|set| theater.lookup.bounds().get(usize::from(set)))
            .map_or(-1, |bounds| i32::from(bounds.start));
        Self::from_ini(base, &theater.ini_data)
    }

    pub(crate) fn from_ini(base: i32, bytes: &[u8]) -> Self {
        let ini = crate::rules::ini_parser::IniFile::from_bytes(bytes).ok();
        let general = ini.as_ref().and_then(|ini| ini.section("General"));
        // These native globals are signed ReadInteger results. The older
        // presentation-facing Option<u16> fields lose negative/large keys.
        let value = |key| general.map_or(-1, |section| section.read_int(key, -1));
        Self {
            base,
            top_left: [value("BridgeTopLeft1"), value("BridgeTopLeft2")],
            bottom_right: [value("BridgeBottomRight1"), value("BridgeBottomRight2")],
            top_right: [value("BridgeTopRight1"), value("BridgeTopRight2")],
            bottom_left: [value("BridgeBottomLeft1"), value("BridgeBottomLeft2")],
            middle: [value("BridgeMiddle1"), value("BridgeMiddle2")],
        }
    }

    fn relative(self, tile: i32) -> i32 {
        tile.wrapping_sub(self.base).wrapping_add(1)
    }

    fn middle_matches(self, relative: i32, axis: usize) -> bool {
        (0..4).any(|variant| relative == self.middle[axis].wrapping_add(variant))
    }

    /// Direction of the edge search dispatched by 576770. Preserve the native
    /// first-match order, including aliased theater keys.
    pub(crate) fn start_direction(self, tile: i32, subtile: u8) -> Option<u8> {
        let relative = self.relative(tile);
        if (subtile == 8 && self.top_left.contains(&relative))
            || (subtile == 5 && self.middle_matches(relative, 0))
        {
            Some(2)
        } else if (subtile == 12 && self.top_right.contains(&relative))
            || (subtile == 7 && self.middle_matches(relative, 1))
        {
            Some(4)
        } else {
            None
        }
    }

    pub(crate) fn is_end(self, tile: i32, subtile: u8, direction: u8) -> bool {
        let relative = self.relative(tile);
        match direction {
            2 => {
                subtile == 4
                    && (self.bottom_right.contains(&relative) || self.middle_matches(relative, 0))
            }
            4 => {
                subtile == 2
                    && (self.bottom_left.contains(&relative) || self.middle_matches(relative, 1))
            }
            _ => false,
        }
    }
}
