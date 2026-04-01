use std::collections::HashMap;

use scylla::Session;
use serde_json::Value;

use crate::errors::ApiError;

/// Validates a CQL identifier (keyspace / table / column name) to prevent injection.
/// Only allows ASCII alphanumerics and underscores.
pub fn validate_identifier(s: &str) -> Result<String, ApiError> {
    let s = s.trim().to_string();
    if s.is_empty() {
        return Err(ApiError::BadRequest(
            "identifier cannot be empty".to_string(),
        ));
    }
    if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(ApiError::BadRequest(format!(
            "identifier '{s}' may only contain ASCII letters, digits, and underscores"
        )));
    }
    Ok(s)
}

/// Converts a JSON value into a CQL literal string, quoting appropriately
/// based on the column's declared CQL type.
pub fn json_to_cql_literal(val: &Value, data_type: &str) -> String {
    let dt = data_type.to_lowercase();
    match val {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => {
            // UUID, timeuuid, and inet literals are unquoted in CQL
            if dt.contains("uuid") || dt.contains("inet") {
                s.clone()
            } else {
                // Escape embedded single quotes
                format!("'{}'", s.replace('\'', "''"))
            }
        }
        Value::Array(_) | Value::Object(_) => {
            // Represents CQL collections / UDTs as JSON strings
            serde_json::to_string(val).unwrap_or_else(|_| "null".to_string())
        }
    }
}

/// For SELECT queries in the query endpoint, injects the JSON keyword so the
/// driver returns each row as a single JSON string column (`[json]`).
/// "SELECT * FROM t" → "SELECT JSON * FROM t"
/// "SELECT JSON * FROM t" → unchanged
pub fn maybe_inject_select_json(query: &str) -> String {
    let t = query.trim();
    if t.len() >= 6 && t[..6].eq_ignore_ascii_case("select") {
        let after = t[6..].trim_start();
        if after.len() >= 4 && after[..4].eq_ignore_ascii_case("json") {
            return t.to_string(); // already SELECT JSON
        }
        format!("SELECT JSON {}", after)
    } else {
        t.to_string()
    }
}

/// Fetches a column-name → CQL-type map for a given table from system_schema.
pub async fn fetch_column_types(
    session: &Session,
    keyspace: &str,
    table: &str,
) -> Result<HashMap<String, String>, ApiError> {
    let result = session
        .query_unpaged(
            "SELECT column_name, type \
             FROM system_schema.columns \
             WHERE keyspace_name = ? AND table_name = ?",
            (keyspace.to_string(), table.to_string()),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let rows_result = result
        .into_rows_result()
        .map_err(|e| ApiError::Db(e.to_string()))?;
    let map = rows_result
        .rows::<(String, String)>()
        .map_err(|e| ApiError::Db(e.to_string()))?
        .filter_map(|r: Result<(String, String), _>| r.ok())
        .collect::<HashMap<_, _>>();

    Ok(map)
}
