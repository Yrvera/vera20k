//! Compare bridge oracle JSON traces without inferring missing gamemd data.

use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const REQUIRED_TOP_LEVEL: [&str; 5] = [
    "schema_version",
    "scenario",
    "cell_facts",
    "astar_steps",
    "runtime_ticks",
];

const REQUIRED_COMMON_PATHS: [&str; 9] = [
    "schema_version",
    "scenario.id",
    "scenario.map",
    "scenario.theater",
    "scenario.unit",
    "scenario.house",
    "scenario.start_cell",
    "scenario.target_cell",
    "scenario.route_window",
];

const REQUIRED_GAMEMD_ACTIVATION_PATHS: [&str; 7] = [
    "scenario.activation_proof.unit_pointer",
    "scenario.activation_proof.unit_type",
    "scenario.activation_proof.house",
    "scenario.activation_proof.issued_order_id",
    "scenario.activation_proof.issued_order_tick",
    "scenario.activation_proof.pathfinder_search_id",
    "scenario.activation_proof.callsite_category",
];

const ASTAR_VALIDATION_FIELDS: [&str; 4] = [
    "current_cell",
    "candidate_cell",
    "direction",
    "incoming_path_height",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Pass,
    Fail,
    Unchecked,
}

impl Verdict {
    fn as_str(self) -> &'static str {
        match self {
            Verdict::Pass => "PASS",
            Verdict::Fail => "FAIL",
            Verdict::Unchecked => "UNCHECKED",
        }
    }
}

#[derive(Debug)]
struct FieldVerdict {
    path: String,
    verdict: Verdict,
    gamemd: Option<Value>,
    rust: Option<Value>,
}

fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        bail!("usage: bridge-oracle-compare <gamemd.json> <rust.json>");
    }
    let gamemd = read_json(PathBuf::from(&args[0]))?;
    let rust = read_json(PathBuf::from(&args[1]))?;
    let verdicts = compare_traces(&gamemd, &rust)?;

    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for row in &verdicts {
        *counts.entry(row.verdict.as_str()).or_default() += 1;
    }
    println!(
        "PASS={} FAIL={} UNCHECKED={}",
        counts.get("PASS").copied().unwrap_or(0),
        counts.get("FAIL").copied().unwrap_or(0),
        counts.get("UNCHECKED").copied().unwrap_or(0)
    );
    for row in verdicts {
        if row.verdict != Verdict::Pass {
            println!(
                "{} {} gamemd={} rust={}",
                row.verdict.as_str(),
                row.path,
                value_label(row.gamemd.as_ref()),
                value_label(row.rust.as_ref())
            );
        }
    }
    Ok(())
}

fn read_json(path: PathBuf) -> Result<Value> {
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.to_string_lossy()))?;
    serde_json::from_str(&text).with_context(|| format!("parse {}", path.to_string_lossy()))
}

fn compare_traces(gamemd: &Value, rust: &Value) -> Result<Vec<FieldVerdict>> {
    if gamemd.get("schema_version") != rust.get("schema_version") {
        return Ok(vec![FieldVerdict {
            path: "schema_version".to_string(),
            verdict: Verdict::Fail,
            gamemd: gamemd.get("schema_version").cloned(),
            rust: rust.get("schema_version").cloned(),
        }]);
    }

    let mut rows = Vec::new();
    for path in REQUIRED_COMMON_PATHS {
        compare_value(
            path.to_string(),
            get_path(gamemd, path),
            get_path(rust, path),
            &mut rows,
        );
    }
    for path in REQUIRED_GAMEMD_ACTIVATION_PATHS {
        if get_path(gamemd, path).is_none() {
            rows.push(FieldVerdict {
                path: format!("gamemd.{path}"),
                verdict: Verdict::Unchecked,
                gamemd: None,
                rust: None,
            });
        }
    }
    compare_astar_expansion_order(gamemd, rust, &mut rows);
    for key in REQUIRED_TOP_LEVEL {
        compare_value(key.to_string(), gamemd.get(key), rust.get(key), &mut rows);
    }
    let mut seen = BTreeSet::new();
    rows.retain(|row| seen.insert(row.path.clone()));
    Ok(rows)
}

fn get_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for part in path.split('.') {
        current = current.get(part)?;
    }
    if current.is_null() {
        None
    } else {
        Some(current)
    }
}

fn compare_astar_expansion_order(gamemd: &Value, rust: &Value, rows: &mut Vec<FieldVerdict>) {
    let (Some(Value::Array(g)), Some(Value::Array(r))) =
        (gamemd.get("astar_steps"), rust.get("astar_steps"))
    else {
        rows.push(FieldVerdict {
            path: "astar_steps.expansion_order".to_string(),
            verdict: Verdict::Unchecked,
            gamemd: gamemd.get("astar_steps").cloned(),
            rust: rust.get("astar_steps").cloned(),
        });
        return;
    };
    let overlap = g.len().min(r.len());
    for idx in 0..overlap {
        let g_tuple = astar_validation_tuple(&g[idx]);
        let r_tuple = astar_validation_tuple(&r[idx]);
        if g_tuple != r_tuple {
            rows.push(FieldVerdict {
                path: format!("astar_steps.first_divergent_expansion[{idx}]"),
                verdict: Verdict::Fail,
                gamemd: Some(g_tuple),
                rust: Some(r_tuple),
            });
            return;
        }
    }
    if g.len() != r.len() {
        rows.push(FieldVerdict {
            path: "astar_steps.expansion_order.length".to_string(),
            verdict: Verdict::Unchecked,
            gamemd: Some(Value::from(g.len())),
            rust: Some(Value::from(r.len())),
        });
    }
}

fn astar_validation_tuple(row: &Value) -> Value {
    let mut map = serde_json::Map::new();
    for field in ASTAR_VALIDATION_FIELDS {
        map.insert(
            field.to_string(),
            row.get(field).cloned().unwrap_or(Value::Null),
        );
    }
    Value::Object(map)
}

fn compare_value(
    path: String,
    gamemd: Option<&Value>,
    rust: Option<&Value>,
    rows: &mut Vec<FieldVerdict>,
) {
    match (gamemd, rust) {
        (None, _) | (_, None) => rows.push(FieldVerdict {
            path,
            verdict: Verdict::Unchecked,
            gamemd: gamemd.cloned(),
            rust: rust.cloned(),
        }),
        (Some(Value::Object(g)), Some(Value::Object(r))) => {
            let mut keys = g.keys().chain(r.keys()).collect::<Vec<_>>();
            keys.sort();
            keys.dedup();
            for key in keys {
                compare_value(format!("{path}.{key}"), g.get(key), r.get(key), rows);
            }
        }
        (Some(Value::Array(g)), Some(Value::Array(r))) => {
            let max_len = g.len().max(r.len());
            for idx in 0..max_len {
                compare_value(format!("{path}[{idx}]"), g.get(idx), r.get(idx), rows);
            }
        }
        (Some(g), Some(r)) => rows.push(FieldVerdict {
            path,
            verdict: if g == r { Verdict::Pass } else { Verdict::Fail },
            gamemd: Some(g.clone()),
            rust: Some(r.clone()),
        }),
    }
}

fn value_label(value: Option<&Value>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "<missing>".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace(extra: &str) -> Value {
        serde_json::from_str(&format!(
            r#"{{
              "schema_version": 1,
              "scenario": {{
                "id": "sample",
                "map": "sample.map",
                "theater": "TEMPERATE",
                "unit": "MTNK",
                "house": "Americans",
                "start_cell": [10, 10],
                "target_cell": [20, 10],
                "route_window": [[10, 10], [20, 10]],
                "activation_proof": {{
                  "unit_pointer": "0x1",
                  "unit_type": "MTNK",
                  "house": "Americans",
                  "issued_order_id": 1,
                  "issued_order_tick": 2,
                  "pathfinder_search_id": 3,
                  "callsite_category": "UnitClass::Can_Enter_Cell"
                }}
                {extra}
              }},
              "cell_facts": [],
              "astar_steps": [],
              "runtime_ticks": []
            }}"#
        ))
        .unwrap()
    }

    #[test]
    fn equal_fields_pass() {
        let rows = compare_traces(&trace(""), &trace("")).unwrap();
        assert!(rows.iter().all(|row| row.verdict == Verdict::Pass));
    }

    #[test]
    fn unequal_fields_fail() {
        let rows = compare_traces(
            &trace(r#", "route_note": [1, 2]"#),
            &trace(r#", "route_note": [1, 3]"#),
        )
        .unwrap();
        assert!(
            rows.iter()
                .any(|row| row.path == "scenario.route_note[1]" && row.verdict == Verdict::Fail)
        );
    }

    #[test]
    fn missing_fields_are_unchecked() {
        let gamemd = trace("");
        let mut rust = trace("");
        rust["scenario"]
            .as_object_mut()
            .expect("scenario object")
            .remove("house");
        let rows = compare_traces(&gamemd, &rust).unwrap();
        assert!(
            rows.iter()
                .any(|row| row.path == "scenario.house" && row.verdict == Verdict::Unchecked)
        );
    }

    #[test]
    fn missing_gamemd_activation_proof_is_unchecked() {
        let gamemd = serde_json::json!({
            "schema_version": 1,
            "scenario": {
                "id": "sample",
                "map": "sample.map",
                "theater": "TEMPERATE",
                "unit": "MTNK",
                "house": "Americans",
                "start_cell": [10, 10],
                "target_cell": [20, 10],
                "route_window": [[10, 10], [20, 10]]
            },
            "cell_facts": [],
            "astar_steps": [],
            "runtime_ticks": []
        });
        let rows = compare_traces(&gamemd, &trace("")).unwrap();
        assert!(rows.iter().any(|row| {
            row.path == "gamemd.scenario.activation_proof.unit_pointer"
                && row.verdict == Verdict::Unchecked
        }));
    }

    #[test]
    fn divergent_astar_expansion_order_reports_first_divergence() {
        let gamemd = serde_json::json!({
            "schema_version": 1,
            "scenario": trace("")["scenario"].clone(),
            "cell_facts": [],
            "astar_steps": [
                {
                    "search_id": 7,
                    "expansion_index": 0,
                    "current_cell": [1, 1],
                    "candidate_cell": [2, 1],
                    "direction": 0,
                    "incoming_path_height": 0
                }
            ],
            "runtime_ticks": []
        });
        let rust = serde_json::json!({
            "schema_version": 1,
            "scenario": trace("")["scenario"].clone(),
            "cell_facts": [],
            "astar_steps": [
                {
                    "search_id": 7,
                    "expansion_index": 0,
                    "current_cell": [1, 1],
                    "candidate_cell": [1, 2],
                    "direction": 2,
                    "incoming_path_height": 0
                }
            ],
            "runtime_ticks": []
        });
        let rows = compare_traces(&gamemd, &rust).unwrap();
        assert!(rows.iter().any(|row| {
            row.path == "astar_steps.first_divergent_expansion[0]" && row.verdict == Verdict::Fail
        }));
    }
}
