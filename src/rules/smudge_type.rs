//! Smudge type definitions parsed from rulesmd.ini.
//!
//! [SmudgeTypes] numeric list maps to per-name sections (e.g. [CRATER01]).
//! Each type carries the four INI keys that gate spawn behavior:
//! Crater, Burn, Width, Height.
//!
//! Dependency rules: depends on rules/ini_parser only. No sim dependency.

use std::collections::HashMap;

use crate::rules::ini_parser::IniFile;

#[derive(Debug, Clone)]
pub struct SmudgeTypeDef {
    pub name: String,
    pub crater: bool,
    pub burn: bool,
    pub width: u8,
    pub height: u8,
    pub image_name: Option<String>,
    pub is_theater: bool,
}

#[derive(Debug, Clone, Default)]
pub struct SmudgeTypeRegistry {
    types: Vec<SmudgeTypeDef>,
    by_name: HashMap<String, u16>,
}

impl SmudgeTypeRegistry {
    pub fn from_rules_ini(ini: &IniFile) -> Self {
        let mut types: Vec<SmudgeTypeDef> = Vec::new();
        let mut by_name: HashMap<String, u16> = HashMap::new();

        let Some(list_section) = ini.section("SmudgeTypes") else {
            return Self::default();
        };

        // Matches the canonical [XxxTypes] numbered-list pattern used by
        // ruleset.rs:1594 — get_values() returns the values of `1=NAME, 2=NAME, ...`
        // sorted by numeric index, with empty strings filtered out.
        for value in list_section.get_values() {
            let name_upper: String = value.trim().to_uppercase();
            if name_upper.is_empty() {
                continue;
            }
            if by_name.contains_key(&name_upper) {
                continue;
            }
            let Some(section) = ini.section(&name_upper) else {
                continue;
            };
            let crater: bool = section.get_bool("Crater").unwrap_or(false);
            let burn: bool = section.get_bool("Burn").unwrap_or(false);
            let width: u8 = section
                .get_i32("Width")
                .map(|v| v.clamp(1, 255) as u8)
                .unwrap_or(1);
            let height: u8 = section
                .get_i32("Height")
                .map(|v| v.clamp(1, 255) as u8)
                .unwrap_or(1);
            let image_name: Option<String> = section
                .get("Image")
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
            let is_theater: bool = section.get_bool("Theater").unwrap_or(false);

            let id: u16 = types.len() as u16;
            by_name.insert(name_upper.clone(), id);
            types.push(SmudgeTypeDef {
                name: name_upper,
                crater,
                burn,
                width,
                height,
                image_name,
                is_theater,
            });
        }

        Self { types, by_name }
    }

    pub fn get(&self, id: u16) -> Option<&SmudgeTypeDef> {
        self.types.get(id as usize)
    }

    pub fn find_by_name(&self, name: &str) -> Option<u16> {
        self.by_name.get(&name.to_uppercase()).copied()
    }

    pub fn len(&self) -> usize {
        self.types.len()
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    pub fn iter_with_id(&self) -> impl Iterator<Item = (u16, &SmudgeTypeDef)> {
        self.types
            .iter()
            .enumerate()
            .map(|(i, t)| (i as u16, t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ini(s: &str) -> IniFile {
        IniFile::from_bytes(s.as_bytes()).unwrap()
    }

    #[test]
    fn parses_smudge_types_list_with_per_name_sections() {
        let ini = parse_ini(
            "[SmudgeTypes]\n\
             1=CR1\n\
             2=BURN01\n\
             \n\
             [CR1]\n\
             Crater=yes\n\
             Width=1\n\
             Height=1\n\
             \n\
             [BURN01]\n\
             Burn=yes\n\
             Width=2\n\
             Height=2\n",
        );
        let reg = SmudgeTypeRegistry::from_rules_ini(&ini);
        assert_eq!(reg.len(), 2);
        let cr1 = reg.get(0).unwrap();
        assert_eq!(cr1.name, "CR1");
        assert!(cr1.crater);
        assert!(!cr1.burn);
        assert_eq!(cr1.width, 1);
        let burn01 = reg.get(1).unwrap();
        assert!(burn01.burn);
        assert_eq!(burn01.width, 2);
        assert_eq!(burn01.height, 2);
    }

    #[test]
    fn missing_section_skipped() {
        let ini = parse_ini("[SmudgeTypes]\n1=DOES_NOT_EXIST\n");
        let reg = SmudgeTypeRegistry::from_rules_ini(&ini);
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn defaults_apply() {
        let ini = parse_ini("[SmudgeTypes]\n1=X\n[X]\n");
        let reg = SmudgeTypeRegistry::from_rules_ini(&ini);
        let x = reg.get(0).unwrap();
        assert!(!x.crater);
        assert!(!x.burn);
        assert_eq!(x.width, 1);
        assert_eq!(x.height, 1);
    }

    #[test]
    fn find_by_name_case_insensitive() {
        let ini = parse_ini("[SmudgeTypes]\n1=CR1\n[CR1]\nCrater=yes\n");
        let reg = SmudgeTypeRegistry::from_rules_ini(&ini);
        assert_eq!(reg.find_by_name("cr1"), Some(0));
        assert_eq!(reg.find_by_name("CR1"), Some(0));
        assert_eq!(reg.find_by_name("nope"), None);
    }
}
