//! Map tag parsing.
//!
//! `[Tags]` assigns stable identifiers to raw map tag records. We keep the
//! parsing low-assumption for now so later trigger work can decide semantics
//! without having to undo an early guess.

use std::collections::HashMap;

use crate::rules::ini_parser::IniFile;
use crate::rules::ini_value::atoi_lenient;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTag {
    pub id: String,
    pub fields: Vec<String>,
    /// Native TagType field zero. Repetition belongs to the Tag, not to any
    /// TriggerType in its linked chain.
    pub repeat_mode: i32,
    /// Native TagType field one. This is presentation/diagnostic identity;
    /// execution links through `trigger_type_head_id`.
    pub name: String,
    /// Native TagType field two, the first TriggerType in the forward chain.
    pub trigger_type_head_id: Option<String>,
}

pub type TagMap = HashMap<String, MapTag>;

/// Parse `[Tags]` into a tag-id keyed map.
pub fn parse_tags(ini: &IniFile) -> TagMap {
    let Some(section) = ini.section("Tags") else {
        return HashMap::new();
    };

    let mut tags: TagMap = HashMap::new();
    for key in section.keys() {
        let Some(raw_value) = section.get(key) else {
            continue;
        };
        let id = key.trim();
        if id.is_empty() {
            continue;
        }
        let id = id.to_ascii_uppercase();
        let fields: Vec<String> = raw_value
            .split(',')
            .map(|part| part.trim().to_string())
            .collect();
        let repeat_mode = fields.first().map_or(0, |value| atoi_lenient(value));
        let name = fields.get(1).cloned().unwrap_or_default();
        let trigger_type_head_id = fields
            .get(2)
            .map(|value| value.trim())
            .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("<none>"))
            .map(str::to_ascii_uppercase);
        tags.insert(
            id.clone(),
            MapTag {
                id,
                fields,
                repeat_mode,
                name,
                trigger_type_head_id,
            },
        );
    }

    if !tags.is_empty() {
        log::info!("Parsed {} tags from [Tags]", tags.len());
    }
    tags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tags() {
        let ini = IniFile::from_str("[Tags]\nTRIG_A=0,1,SomeName\nOBJ_01=2,0\n");
        let tags = parse_tags(&ini);
        assert_eq!(tags.len(), 2);
        assert_eq!(
            tags.get("TRIG_A"),
            Some(&MapTag {
                id: "TRIG_A".to_string(),
                fields: vec!["0".to_string(), "1".to_string(), "SomeName".to_string()],
                repeat_mode: 0,
                name: "1".to_string(),
                trigger_type_head_id: Some("SOMENAME".to_string()),
            })
        );
        assert_eq!(
            tags.get("OBJ_01").map(|tag| tag.fields.as_slice()),
            Some(&["2".to_string(), "0".to_string()][..])
        );
    }

    #[test]
    fn test_missing_tags_is_empty() {
        let ini = IniFile::from_str("[Map]\nTheater=TEMPERATE\n");
        assert!(parse_tags(&ini).is_empty());
    }
}
