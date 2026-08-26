//! Map action parsing.
//!
//! `[Actions]` stores a counted list of 8-field action chunks per trigger id.
//! We preserve the raw field list and also expose normalized action entries so
//! runtime code can stop guessing at the original payload shape.

use std::collections::HashMap;

use crate::rules::ini_parser::IniFile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionEntry {
    pub kind: i32,
    pub params: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapAction {
    pub id: String,
    pub fields: Vec<String>,
    pub entries: Vec<ActionEntry>,
}

pub type ActionMap = HashMap<String, MapAction>;

/// Decode the alphabetic waypoint token stored in action field 8.
///
/// gamemd-derived: `TActionClass::Read @ 0x006DD5B0` routes non-numeric
/// parameter types through `FUN_00763690`. That helper examines at most two
/// ASCII letters, case-insensitively: `A..Z` are `0..25`, and a second letter
/// extends the index as `26 * first + second + 26`. A non-letter first byte is
/// the native `-1` sentinel, represented safely as `None` here.
pub fn decode_waypoint_token(token: &str) -> Option<u32> {
    let bytes = token.as_bytes();
    let first = ascii_waypoint_letter(*bytes.first()?)?;
    let Some(&second_byte) = bytes.get(1) else {
        return Some(first);
    };
    let Some(second) = ascii_waypoint_letter(second_byte) else {
        return Some(first);
    };
    Some(26 * first + second + 26)
}

/// Apply the `TActionClass` constructor/read contract around the decoder.
///
/// The constructor initializes the destination to waypoint 0. `Read` replaces
/// it only when `strtok` returns token 8, so an absent or empty-at-end token
/// retains zero. A present whitespace/non-letter token does reach the decoder
/// and is invalid.
pub fn read_waypoint_token(token: Option<&str>) -> Option<u32> {
    match token {
        None | Some("") => Some(0),
        Some(token) => decode_waypoint_token(token),
    }
}

fn ascii_waypoint_letter(byte: u8) -> Option<u32> {
    byte.to_ascii_uppercase()
        .checked_sub(b'A')
        .filter(|value| *value < 26)
        .map(u32::from)
}

/// Parse `[Actions]` into an id -> action record map.
pub fn parse_actions(ini: &IniFile) -> ActionMap {
    let Some(section) = ini.section("Actions") else {
        return HashMap::new();
    };

    let mut actions: ActionMap = HashMap::new();
    for key in section.keys() {
        let Some(raw_value) = section.get(key) else {
            continue;
        };
        let id = key.trim();
        if id.is_empty() {
            continue;
        }
        let id = id.to_ascii_uppercase();
        let raw_fields: Vec<&str> = raw_value.split(',').collect();
        let fields: Vec<String> = raw_fields
            .iter()
            .map(|part| part.trim().to_string())
            .collect();
        let entries = parse_action_entries(&fields, &raw_fields);
        actions.insert(
            id.clone(),
            MapAction {
                id,
                fields,
                entries,
            },
        );
    }

    if !actions.is_empty() {
        log::info!("Parsed {} actions from [Actions]", actions.len());
    }
    actions
}

fn parse_action_entries(fields: &[String], raw_fields: &[&str]) -> Vec<ActionEntry> {
    if fields.is_empty() {
        return Vec::new();
    }

    let declared_count = fields[0].trim().parse::<usize>().ok();
    if let Some(count) = declared_count {
        let payload = &fields[1..];
        let chunk_len = 8;
        let max_chunks = payload.len() / chunk_len;
        let chunk_count = count.min(max_chunks);
        if chunk_count > 0 {
            let raw_payload = &raw_fields[1..];
            return payload
                .chunks_exact(chunk_len)
                .zip(raw_payload.chunks_exact(chunk_len))
                .take(chunk_count)
                .filter_map(|(chunk, raw_chunk)| {
                    let kind = chunk[0].trim().parse::<i32>().ok()?;
                    let mut params = chunk[1..].to_vec();
                    params[6] = raw_chunk[7].to_string();
                    Some(ActionEntry { kind, params })
                })
                .collect();
        }
    }

    let kind = fields[0].trim().parse::<i32>().ok();
    kind.map(|kind| {
        let mut params = fields[1..].to_vec();
        if let (Some(token), Some(slot)) = (raw_fields.get(7), params.get_mut(6)) {
            *slot = (*token).to_string();
        }
        vec![ActionEntry { kind, params }]
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_actions() {
        let ini = IniFile::from_str(
            "[Actions]\nAC_A=2,28,7,0,0,0,0,0,0,112,0,0,0,0,0,0,9\nAC_B=11,Americans,5\n",
        );
        let actions = parse_actions(&ini);
        assert_eq!(actions.len(), 2);
        assert_eq!(
            actions.get("AC_A"),
            Some(&MapAction {
                id: "AC_A".to_string(),
                fields: vec![
                    "2".to_string(),
                    "28".to_string(),
                    "7".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "112".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "9".to_string(),
                ],
                entries: vec![
                    ActionEntry {
                        kind: 28,
                        params: vec![
                            "7".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                        ],
                    },
                    ActionEntry {
                        kind: 112,
                        params: vec![
                            "0".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                            "0".to_string(),
                            "9".to_string(),
                        ],
                    },
                ],
            })
        );
        assert_eq!(
            actions.get("AC_B").map(|action| action.fields.as_slice()),
            Some(&["11".to_string(), "Americans".to_string(), "5".to_string()][..])
        );
        assert_eq!(
            actions.get("AC_B").map(|action| action.entries.as_slice()),
            Some(
                &[ActionEntry {
                    kind: 11,
                    params: vec!["Americans".to_string(), "5".to_string()],
                }][..]
            )
        );
    }

    #[test]
    fn test_missing_actions_is_empty() {
        let ini = IniFile::from_str("[Map]\nTheater=TEMPERATE\n");
        assert!(parse_actions(&ini).is_empty());
    }

    #[test]
    fn waypoint_tokens_follow_the_native_two_letter_decoder() {
        for (token, expected) in [
            ("A", Some(0)),
            ("Z", Some(25)),
            ("a", Some(0)),
            ("P", Some(15)),
            ("AA", Some(26)),
            ("aa", Some(26)),
            ("NZ", Some(389)),
            ("ZZ", Some(701)),
            ("", None),
            ("7", None),
            ("   ", None),
            ("A7", Some(0)),
            ("NZignored", Some(389)),
        ] {
            assert_eq!(decode_waypoint_token(token), expected, "token {token:?}");
        }
    }

    #[test]
    fn action_read_preserves_ctor_zero_but_not_present_whitespace() {
        assert_eq!(read_waypoint_token(None), Some(0));
        assert_eq!(read_waypoint_token(Some("")), Some(0));
        assert_eq!(read_waypoint_token(Some("   ")), None);

        let ini = IniFile::from_str("[Actions]\nEMPTY=1,137,0,0,0,0,0,0,\n");
        let actions = parse_actions(&ini);
        assert_eq!(actions["EMPTY"].entries[0].params[6], "");
    }
}
