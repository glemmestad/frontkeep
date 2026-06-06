//! Output rendering. Tool results are JSON; `table` is the default human view,
//! `--output json|yaml` for machines and pipes. `Shape` lets typed subcommands
//! hint a nicer layout; the generic `call` uses `Auto`.

use comfy_table::{presets::UTF8_FULL, Table};
use serde_json::Value;

use crate::CliError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output {
    Json,
    Table,
    Yaml,
}

impl std::str::FromStr for Output {
    type Err = CliError;
    fn from_str(s: &str) -> Result<Self, CliError> {
        match s.to_ascii_lowercase().as_str() {
            "json" => Ok(Output::Json),
            "table" => Ok(Output::Table),
            "yaml" | "yml" => Ok(Output::Yaml),
            other => Err(CliError::Args(format!(
                "unknown output '{other}' (expected json|table|yaml)"
            ))),
        }
    }
}

/// How to lay a value out as a table. `Auto` infers from the JSON shape.
pub enum Shape {
    Auto,
    KeyValue,
    Rows(Vec<&'static str>),
}

pub fn render(value: &Value, shape: Shape, output: Output) -> String {
    match output {
        Output::Json => serde_json::to_string_pretty(value).unwrap_or_default(),
        Output::Yaml => serde_yaml::to_string(value).unwrap_or_default(),
        Output::Table => table(value, shape),
    }
}

fn table(value: &Value, shape: Shape) -> String {
    match shape {
        Shape::Rows(cols) => rows(value, &cols),
        Shape::KeyValue => kv(value),
        Shape::Auto => match value {
            Value::Array(a) if a.iter().any(Value::is_object) => auto_rows(a),
            Value::Object(_) => kv(value),
            Value::Array(a) => a.iter().map(scalar).collect::<Vec<_>>().join("\n"),
            other => scalar(other),
        },
    }
}

/// Array of objects → a table over the named columns.
fn rows(value: &Value, cols: &[&str]) -> String {
    let Some(arr) = value.as_array() else {
        return scalar(value);
    };
    let mut t = base_table();
    t.set_header(cols.to_vec());
    for item in arr {
        let cells: Vec<String> = cols
            .iter()
            .map(|c| scalar(item.get(*c).unwrap_or(&Value::Null)))
            .collect();
        t.add_row(cells);
    }
    t.to_string()
}

/// Array of objects with no column hint → infer columns from the first object.
fn auto_rows(arr: &[Value]) -> String {
    let cols: Vec<String> = arr
        .iter()
        .find_map(|v| v.as_object())
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default();
    if cols.is_empty() {
        return scalar(&Value::Array(arr.to_vec()));
    }
    let mut t = base_table();
    t.set_header(cols.clone());
    for item in arr {
        let cells: Vec<String> = cols
            .iter()
            .map(|c| scalar(item.get(c).unwrap_or(&Value::Null)))
            .collect();
        t.add_row(cells);
    }
    t.to_string()
}

/// Single object → a two-column field/value table.
fn kv(value: &Value) -> String {
    let Some(obj) = value.as_object() else {
        return scalar(value);
    };
    let mut t = base_table();
    t.set_header(vec!["field", "value"]);
    for (k, v) in obj {
        t.add_row(vec![k.clone(), scalar(v)]);
    }
    t.to_string()
}

fn base_table() -> Table {
    let mut t = Table::new();
    t.load_preset(UTF8_FULL);
    t
}

/// A JSON value as one cell: scalars verbatim, nested structures as compact JSON.
fn scalar(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        _ => serde_json::to_string(v).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn output_parses() {
        assert_eq!("json".parse::<Output>().unwrap(), Output::Json);
        assert_eq!("TABLE".parse::<Output>().unwrap(), Output::Table);
        assert_eq!("yml".parse::<Output>().unwrap(), Output::Yaml);
        assert!("xml".parse::<Output>().is_err());
    }

    #[test]
    fn json_and_yaml_roundtrip_shape() {
        let v = json!({"a": 1, "b": "two"});
        assert!(render(&v, Shape::Auto, Output::Json).contains("\"a\""));
        assert!(render(&v, Shape::Auto, Output::Yaml).contains("a:"));
    }

    #[test]
    fn auto_table_of_objects_has_headers_and_rows() {
        let v = json!([{"name": "x", "n": 1}, {"name": "y", "n": 2}]);
        let out = render(&v, Shape::Auto, Output::Table);
        assert!(out.contains("name") && out.contains('x') && out.contains('y'));
    }

    #[test]
    fn auto_object_is_key_value() {
        let v = json!({"project_id": "proj-2026-0001", "spent_usd": 12.5});
        let out = render(&v, Shape::Auto, Output::Table);
        assert!(out.contains("project_id") && out.contains("proj-2026-0001"));
    }

    #[test]
    fn nested_value_becomes_compact_json_cell() {
        let v = json!({"spec": {"size": 1}});
        let out = render(&v, Shape::KeyValue, Output::Table);
        assert!(out.contains("{\"size\":1}"));
    }

    #[test]
    fn scalar_array_lists_lines() {
        let v = json!(["a", "b"]);
        let out = render(&v, Shape::Auto, Output::Table);
        assert_eq!(out, "a\nb");
    }
}
