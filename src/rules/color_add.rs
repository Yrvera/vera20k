//! Source-ordered `[ColorAdd]` rules data retained for later rendering consumers.
//!
//! The three channel values are raw RGB565 magnitudes, not display RGB bytes.
//! This module assigns no consumer meaning to the fixed native slots.

use crate::rules::ini_parser::IniFile;

/// Fixed capacity of the active-YR `[ColorAdd]` table.
pub const COLOR_ADD_SLOT_COUNT: usize = 16;

/// One source entry in the fixed `[ColorAdd]` table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ColorAddEntry {
    /// Exact key spelling retained from the source section; `None` marks an unused tail slot.
    pub name: Option<String>,
    /// Raw RGB565 channel magnitudes. These values are deliberately not normalized.
    pub rgb: [u8; 3],
}

/// Rules-owned fixed `[ColorAdd]` table in source declaration order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorAddTable {
    pub slots: [ColorAddEntry; COLOR_ADD_SLOT_COUNT],
}

impl Default for ColorAddTable {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| ColorAddEntry::default()),
        }
    }
}

impl ColorAddTable {
    /// Parse at most 16 source entries, leaving the unused tail zero-filled.
    pub fn from_ini(ini: &IniFile) -> Self {
        let mut table = Self::default();
        let Some(section) = ini.section("ColorAdd") else {
            return table;
        };

        for (slot, key) in table.slots.iter_mut().zip(section.keys()) {
            *slot = ColorAddEntry {
                name: Some(key.to_string()),
                rgb: section.read_color_rgb(key, [0; 3]),
            };
        }
        table
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gsi_02_13_color_add_retains_fixed_source_ordered_raw_slots() {
        let ini = IniFile::from_str(
            "[ColorAdd]\n\
             None=0,0,0\n\
             StrongRed=31,0,0\n\
             StrongGreen=0,63,0\n\
             StrongBlue=0,0,31\n\
             HighRed=24,0,0\n\
             HighGreen=0,56,0\n\
             HighBlue=0,0,24\n\
             BrightWhite=31,63,31\n\
             LowWhite=7,7,7\n\
             HighWhite=24,56,24\n\
             MidWhite=14,28,14\n\
             Purple=15,0,15\n\
             HighYellow=24,56,0\n\
             TopYellow=16,32,0\n",
        );

        let table = ColorAddTable::from_ini(&ini);
        let names: Vec<_> = table.slots[..14]
            .iter()
            .map(|entry| entry.name.as_deref().expect("source entry"))
            .collect();
        assert_eq!(
            names.as_slice(),
            &[
                "None",
                "StrongRed",
                "StrongGreen",
                "StrongBlue",
                "HighRed",
                "HighGreen",
                "HighBlue",
                "BrightWhite",
                "LowWhite",
                "HighWhite",
                "MidWhite",
                "Purple",
                "HighYellow",
                "TopYellow",
            ]
        );
        assert_eq!(
            table
                .slots
                .iter()
                .filter(|entry| entry.name.is_some())
                .count(),
            14
        );
        assert_eq!(table.slots[2].rgb, [0, 63, 0]);
        assert_eq!(table.slots[7].rgb, [31, 63, 31]);
        assert_eq!(table.slots[14], ColorAddEntry::default());
        assert_eq!(table.slots[15], ColorAddEntry::default());
    }
}
