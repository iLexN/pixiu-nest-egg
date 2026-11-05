//! Dump the OpenAPI JSON to a path (arg 1) or stdout — the frontend's
//! `pnpm gen:api` runs this so codegen always types the current source.
//!
//! utoipa does not mark serde `Option<T>` fields `required`, but serde always
//! emits them as `null` — so request *response* schemas get every property
//! marked required here, while schemas referenced by `requestBody` (whose
//! fields genuinely may be absent) are left alone.

use wealth_backend::db;
use wealth_backend::routes::{AppState, api_router};

/// Names of schemas any `requestBody` references, recursively through `$ref`s.
fn write_schemas(spec: &serde_json::Value, out: &mut std::collections::HashSet<String>) {
    fn refs(v: &serde_json::Value, out: &mut std::collections::HashSet<String>) {
        match v {
            serde_json::Value::Object(map) => {
                for (k, v) in map {
                    if k == "$ref" {
                        out.insert(
                            v.as_str()
                                .unwrap_or("")
                                .rsplit('/')
                                .next()
                                .unwrap_or("")
                                .into(),
                        );
                    } else {
                        refs(v, out);
                    }
                }
            }
            serde_json::Value::Array(items) => items.iter().for_each(|v| refs(v, out)),
            _ => {}
        }
    }
    if let Some(paths) = spec["paths"].as_object() {
        for path in paths.values() {
            for op in path.as_object().into_iter().flatten().map(|(_, op)| op) {
                refs(&op["requestBody"], out);
            }
        }
    }
}

/// Mark every property of non-request schemas `required` — serde emits them
/// all (as `null` when `Option`), so codegen should not type them optional.
fn strict_required(spec: &mut serde_json::Value) {
    let mut write = std::collections::HashSet::new();
    write_schemas(spec, &mut write);
    if let Some(schemas) = spec["components"]["schemas"].as_object_mut() {
        for (name, schema) in schemas {
            if write.contains(name) {
                continue;
            }
            mark(schema);
        }
    }
}

/// Recursively mark every object's `properties` as `required` — nested inline
/// objects (e.g. `YearReviewRow.assets`) are not named components.
fn mark(v: &mut serde_json::Value) {
    if let serde_json::Value::Object(map) = v {
        if let Some(serde_json::Value::Object(props)) = map.get_mut("properties") {
            let all: Vec<serde_json::Value> = props
                .keys()
                .cloned()
                .map(serde_json::Value::String)
                .collect();
            map.insert("required".into(), serde_json::Value::Array(all));
        }
        map.values_mut().for_each(mark);
    } else if let serde_json::Value::Array(items) = v {
        items.iter_mut().for_each(mark);
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pool = db::connect_memory().await?;
    let (_, openapi) = api_router(AppState { pool });
    let mut spec: serde_json::Value = serde_json::from_str(&openapi.to_json()?)?;
    strict_required(&mut spec);
    let json = serde_json::to_string_pretty(&spec)?;
    match std::env::args().nth(1) {
        Some(path) => std::fs::write(path, format!("{json}\n"))?,
        None => println!("{json}"),
    }
    Ok(())
}
