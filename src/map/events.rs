//! Map event parsing.
//!
//! `[Events]` stores a counted list of event-condition chunks per trigger id.
//! We preserve the raw field list and also expose normalized conditions so
//! runtime code can evaluate them without hardcoding a flat row assumption.

use std::collections::HashMap;

use crate::rules::ini_parser::IniFile;
use crate::rules::ini_value::atoi_lenient;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventCondition {
    pub kind: i32,
    pub params: Vec<String>,
}

impl EventCondition {
    pub fn param_type(&self) -> i32 {
        self.params.first().map_or(0, |value| atoi_lenient(value))
    }

    pub fn scalar(&self) -> i32 {
        self.params.get(1).map_or(0, |value| atoi_lenient(value))
    }

    pub fn type_name(&self) -> Option<&str> {
        (self.param_type() == 2)
            .then(|| self.params.get(2).map(String::as_str))
            .flatten()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapEvent {
    pub id: String,
    pub fields: Vec<String>,
    pub conditions: Vec<EventCondition>,
}

pub type EventMap = HashMap<String, MapEvent>;

/// Parse `[Events]` into an id -> event record map.
pub fn parse_events(ini: &IniFile) -> EventMap {
    let Some(section) = ini.section("Events") else {
        return HashMap::new();
    };

    let mut events: EventMap = HashMap::new();
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
        let conditions = parse_event_conditions(&fields);
        events.insert(
            id.clone(),
            MapEvent {
                id,
                fields,
                conditions,
            },
        );
    }

    if !events.is_empty() {
        log::info!("Parsed {} events from [Events]", events.len());
    }
    events
}

fn parse_event_conditions(fields: &[String]) -> Vec<EventCondition> {
    if fields.is_empty() {
        return Vec::new();
    }

    let declared_count = fields[0].trim().parse::<usize>().ok();
    if let Some(count) = declared_count {
        let mut cursor = 1usize;
        let mut parsed = Vec::with_capacity(count);
        for _ in 0..count {
            let Some(kind) = fields.get(cursor) else {
                break;
            };
            let Some(param_type) = fields.get(cursor + 1) else {
                break;
            };
            let Some(scalar) = fields.get(cursor + 2) else {
                break;
            };
            let kind = atoi_lenient(kind);
            let param_type = atoi_lenient(param_type);
            let scalar = atoi_lenient(scalar);
            cursor += 3;
            let type_name = if param_type == 2 {
                let Some(name) = fields.get(cursor) else {
                    break;
                };
                cursor += 1;
                Some(name.clone())
            } else {
                None
            };
            let mut params = vec![param_type.to_string(), scalar.to_string()];
            if let Some(name) = type_name.as_ref() {
                params.push(name.clone());
            }
            parsed.push(EventCondition { kind, params });
        }
        if !parsed.is_empty() {
            return parsed;
        }
    }

    let kind = fields[0].trim().parse::<i32>().ok();
    kind.map(|kind| {
        vec![EventCondition {
            kind,
            params: fields[1..].to_vec(),
        }]
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_events() {
        let ini = IniFile::from_str("[Events]\nEV_A=2,47,3,0,27,7,0\nEV_B=2,7\n");
        let events = parse_events(&ini);
        assert_eq!(events.len(), 2);
        assert_eq!(
            events.get("EV_A"),
            Some(&MapEvent {
                id: "EV_A".to_string(),
                fields: vec![
                    "2".to_string(),
                    "47".to_string(),
                    "3".to_string(),
                    "0".to_string(),
                    "27".to_string(),
                    "7".to_string(),
                    "0".to_string()
                ],
                conditions: vec![
                    EventCondition {
                        kind: 47,
                        params: vec!["3".to_string(), "0".to_string()],
                    },
                    EventCondition {
                        kind: 27,
                        params: vec!["7".to_string(), "0".to_string()],
                    },
                ],
            })
        );
        assert_eq!(
            events.get("EV_B").map(|event| event.fields.as_slice()),
            Some(&["2".to_string(), "7".to_string()][..])
        );
        assert_eq!(
            events.get("EV_B").map(|event| event.conditions.as_slice()),
            Some(
                &[EventCondition {
                    kind: 2,
                    params: vec!["7".to_string()],
                }][..]
            )
        );
    }

    #[test]
    fn test_missing_events_is_empty() {
        let ini = IniFile::from_str("[Map]\nTheater=TEMPERATE\n");
        assert!(parse_events(&ini).is_empty());
    }

    #[test]
    fn variable_arity_type_two_event_keeps_scalar_and_type_identity() {
        let events = parse_events(&IniFile::from_str("[Events]\nT=2,60,2,-3,HTNK,27,0,9\n"));
        let conditions = &events["T"].conditions;
        assert_eq!(conditions.len(), 2);
        assert_eq!(
            (
                conditions[0].kind,
                conditions[0].param_type(),
                conditions[0].scalar()
            ),
            (60, 2, -3)
        );
        assert_eq!(conditions[0].type_name(), Some("HTNK"));
        assert_eq!((conditions[1].kind, conditions[1].scalar()), (27, 9));
    }
}
