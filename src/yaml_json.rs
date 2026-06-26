//! Bridge okf's YAML value model into `serde_json` for `--json` output.
//!
//! okf preserves frontmatter as an ordered [`okf::Mapping`] of [`okf::Value`];
//! we project it into `serde_json::Value` so the full frontmatter (well-known
//! keys *and* producer extensions) round-trips into the JSON envelope.

use okf::{Mapping, Value as Yaml};
use serde_json::{Map, Number, Value as Json};

/// Converts an okf YAML value into a JSON value.
pub fn yaml_to_json(value: &Yaml) -> Json {
    match value {
        Yaml::Null => Json::Null,
        Yaml::Bool(b) => Json::Bool(*b),
        Yaml::Int(i) => Json::Number((*i).into()),
        Yaml::Float(f) => Number::from_f64(*f).map(Json::Number).unwrap_or(Json::Null),
        Yaml::String(s) => Json::String(s.clone()),
        Yaml::Sequence(items) => Json::Array(items.iter().map(yaml_to_json).collect()),
        Yaml::Mapping(m) => mapping_to_json(m),
    }
}

/// Converts an okf ordered mapping into a JSON object, preserving key order.
pub fn mapping_to_json(mapping: &Mapping) -> Json {
    let mut obj = Map::new();
    for (key, val) in mapping.iter() {
        let k = key
            .as_str()
            .map(str::to_string)
            .or_else(|| key.as_display_string())
            .unwrap_or_default();
        obj.insert(k, yaml_to_json(val));
    }
    Json::Object(obj)
}
