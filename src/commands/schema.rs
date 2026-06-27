//! `okq schema` — emit the JSON Schema for a command's `--json` envelope,
//! generated from the `schemars` derives. See `docs/features/schema.md`.

use schemars::schema_for;

use crate::cli::SchemaArgs;
use crate::error::AppError;

/// The commands that produce a `--json` envelope, in stable order. `neighbors`
/// and `backlinks` share an output type (and therefore a schema).
pub const COMMANDS: [&str; 9] = [
    "get",
    "find",
    "search",
    "neighbors",
    "backlinks",
    "path",
    "orphans",
    "deadlinks",
    "stats",
];

fn schema_for_command(command: &str) -> Option<serde_json::Value> {
    let schema = match command {
        "get" => schema_for!(crate::commands::get::GetOutput),
        "find" => schema_for!(crate::commands::find::FindOutput),
        "search" => schema_for!(crate::commands::search::SearchOutput),
        "neighbors" | "backlinks" => schema_for!(crate::commands::graph::GraphListOutput),
        "path" => schema_for!(crate::commands::graph::PathOutput),
        "orphans" => schema_for!(crate::commands::graph::OrphansOutput),
        "deadlinks" => schema_for!(crate::commands::graph::DeadlinksOutput),
        "stats" => schema_for!(crate::commands::stats::StatsOutput),
        _ => return None,
    };
    Some(serde_json::to_value(schema).expect("a schema is always serializable"))
}

/// Runs `schema`: one command's schema, or all keyed by command name.
pub fn run(args: &SchemaArgs) -> Result<serde_json::Value, AppError> {
    match &args.command {
        Some(command) => schema_for_command(command).ok_or_else(|| {
            AppError::Usage(format!(
                "unknown command {command:?}; known: {}",
                COMMANDS.join(", ")
            ))
        }),
        None => {
            let mut map = serde_json::Map::new();
            for command in COMMANDS {
                if let Some(schema) = schema_for_command(command) {
                    map.insert(command.to_string(), schema);
                }
            }
            Ok(serde_json::Value::Object(map))
        }
    }
}

/// Serializes a schema document as pretty JSON.
pub fn to_json(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).expect("schema is serializable")
}
