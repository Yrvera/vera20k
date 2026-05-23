//! Building foundation name table used by the original game.
//!
//! `Foundation=` is not a free-form `WxH` parser in gamemd.exe. INI values are
//! resolved through this fixed, case-insensitive table; unknown values fall back
//! to table entry 0 (`1x1`).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoundationDef {
    pub id: u8,
    pub name: &'static str,
    pub width: u16,
    pub height: u16,
}

pub const DEFAULT_FOUNDATION_ID: u8 = 0;

pub const FOUNDATION_TABLE: [FoundationDef; 22] = [
    FoundationDef {
        id: 0,
        name: "1x1",
        width: 1,
        height: 1,
    },
    FoundationDef {
        id: 1,
        name: "2x1",
        width: 2,
        height: 1,
    },
    FoundationDef {
        id: 2,
        name: "1x2",
        width: 1,
        height: 2,
    },
    FoundationDef {
        id: 3,
        name: "2x2",
        width: 2,
        height: 2,
    },
    FoundationDef {
        id: 4,
        name: "2x3",
        width: 2,
        height: 3,
    },
    FoundationDef {
        id: 5,
        name: "3x2",
        width: 3,
        height: 2,
    },
    FoundationDef {
        id: 6,
        name: "3x3",
        width: 3,
        height: 3,
    },
    FoundationDef {
        id: 7,
        name: "3x5",
        width: 3,
        height: 5,
    },
    FoundationDef {
        id: 8,
        name: "4x2",
        width: 4,
        height: 2,
    },
    FoundationDef {
        id: 9,
        name: "3x3Refinery",
        width: 3,
        height: 3,
    },
    FoundationDef {
        id: 10,
        name: "1x3",
        width: 1,
        height: 3,
    },
    FoundationDef {
        id: 11,
        name: "3x1",
        width: 3,
        height: 1,
    },
    FoundationDef {
        id: 12,
        name: "4x3",
        width: 4,
        height: 3,
    },
    FoundationDef {
        id: 13,
        name: "1x4",
        width: 1,
        height: 4,
    },
    FoundationDef {
        id: 14,
        name: "1x5",
        width: 1,
        height: 5,
    },
    FoundationDef {
        id: 15,
        name: "2x6",
        width: 2,
        height: 6,
    },
    FoundationDef {
        id: 16,
        name: "2x5",
        width: 2,
        height: 5,
    },
    FoundationDef {
        id: 17,
        name: "5x3",
        width: 5,
        height: 3,
    },
    FoundationDef {
        id: 18,
        name: "4x4",
        width: 4,
        height: 4,
    },
    FoundationDef {
        id: 19,
        name: "3x4",
        width: 3,
        height: 4,
    },
    FoundationDef {
        id: 20,
        name: "6x4",
        width: 6,
        height: 4,
    },
    FoundationDef {
        id: 21,
        name: "0x0",
        width: 0,
        height: 0,
    },
];

pub fn foundation_def(value: &str) -> &'static FoundationDef {
    let trimmed = value.trim();
    FOUNDATION_TABLE
        .iter()
        .find(|def| def.name.eq_ignore_ascii_case(trimmed))
        .unwrap_or(&FOUNDATION_TABLE[DEFAULT_FOUNDATION_ID as usize])
}

pub fn foundation_id(value: &str) -> u8 {
    foundation_def(value).id
}

pub fn foundation_name(value: &str) -> &'static str {
    foundation_def(value).name
}

pub fn foundation_dimensions(value: &str) -> (u16, u16) {
    let def = foundation_def(value);
    (def.width, def.height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_table_is_case_insensitive() {
        assert_eq!(foundation_id("3x3refinery"), 9);
        assert_eq!(foundation_dimensions("3x3Refinery"), (3, 3));
    }

    #[test]
    fn unknown_values_fall_back_to_1x1() {
        assert_eq!(foundation_id("7x7"), 0);
        assert_eq!(foundation_dimensions("custom"), (1, 1));
    }

    #[test]
    fn zero_foundation_is_table_entry() {
        assert_eq!(foundation_id("0x0"), 21);
        assert_eq!(foundation_dimensions("0x0"), (0, 0));
    }
}
