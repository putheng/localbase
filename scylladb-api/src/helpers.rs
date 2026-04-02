use std::collections::HashMap;

use scylla::Session;
use serde_json::Value;

use crate::errors::ApiError;

/// Allows only ASCII alphanumerics and underscores — prevents CQL injection.
pub fn validate_identifier(s: &str) -> Result<String, ApiError> {
    let s = s.trim().to_string();
    if s.is_empty() {
        return Err(ApiError::BadRequest("identifier cannot be empty".into()));
    }
    if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(ApiError::BadRequest(format!(
            "identifier '{s}' may only contain ASCII letters, digits, and underscores"
        )));
    }
    Ok(s)
}

/// Converts a serde_json Value into a CQL literal appropriate for the declared column type.
pub fn json_to_cql_literal(val: &Value, data_type: &str) -> String {
    let dt = data_type.to_lowercase();
    match val {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => {
            if dt.contains("uuid") || dt.contains("inet") {
                s.clone()
            } else {
                format!("'{}'", s.replace('\'', "''"))
            }
        }
        Value::Array(_) | Value::Object(_) => {
            serde_json::to_string(val).unwrap_or_else(|_| "null".to_string())
        }
    }
}

/// Converts a raw query-param string value into a CQL literal.
/// Tries JSON parsing first so `123`, `true`, `null` are handled correctly.
pub fn str_to_cql_literal(raw: &str, data_type: &str) -> String {
    if let Ok(v) = serde_json::from_str::<Value>(raw) {
        return json_to_cql_literal(&v, data_type);
    }
    let dt = data_type.to_lowercase();
    if dt.contains("uuid") || dt.contains("inet") {
        raw.to_string()
    } else {
        format!("'{}'", raw.replace('\'', "''"))
    }
}

/// A parsed filter ready to embed in a WHERE clause.
pub struct Filter {
    pub cql_fragment: String,
}

/// Parse a PostgREST-style filter param: `col=op.value`.
/// Supported operators: eq neq gt gte lt lte in is
pub fn parse_filter(
    col: &str,
    raw: &str,
    col_types: &HashMap<String, String>,
) -> Result<Filter, ApiError> {
    let col = validate_identifier(col)?;
    let dt = col_types.get(&col).map(String::as_str).unwrap_or("text");

    let (op, val_part) = raw.split_once('.').ok_or_else(|| {
        ApiError::BadRequest(format!(
            "filter for '{col}' must be in format 'op.value' (e.g. eq.5, gt.10, in.(a,b,c))"
        ))
    })?;

    let fragment = match op {
        "eq" => format!("{col} = {}", str_to_cql_literal(val_part, dt)),
        "neq" => format!("{col} != {}", str_to_cql_literal(val_part, dt)),
        "gt" => format!("{col} > {}", str_to_cql_literal(val_part, dt)),
        "gte" => format!("{col} >= {}", str_to_cql_literal(val_part, dt)),
        "lt" => format!("{col} < {}", str_to_cql_literal(val_part, dt)),
        "lte" => format!("{col} <= {}", str_to_cql_literal(val_part, dt)),
        "in" => {
            // val_part looks like "(a,b,c)"
            let inner = val_part.trim_matches(|c| c == '(' || c == ')');
            let items: Vec<String> = inner
                .split(',')
                .map(|v| str_to_cql_literal(v.trim(), dt))
                .collect();
            format!("{col} IN ({})", items.join(", "))
        }
        "is" => {
            if val_part.eq_ignore_ascii_case("null") {
                format!("{col} IS NULL")
            } else {
                return Err(ApiError::BadRequest(
                    "'is' operator only supports 'null'".into(),
                ));
            }
        }
        _ => {
            return Err(ApiError::BadRequest(format!(
                "unknown operator '{op}'; supported: eq neq gt gte lt lte in is"
            )))
        }
    };

    Ok(Filter { cql_fragment: fragment })
}

/// Parse "col1.asc,col2.desc" → "col1 ASC, col2 DESC"
pub fn parse_order_clause(order_str: &str) -> Result<String, ApiError> {
    let parts: Result<Vec<String>, ApiError> = order_str
        .split(',')
        .map(|part| {
            let part = part.trim();
            if let Some((col, dir)) = part.split_once('.') {
                let col = validate_identifier(col)?;
                let dir = match dir.to_lowercase().as_str() {
                    "asc" => "ASC",
                    "desc" => "DESC",
                    other => {
                        return Err(ApiError::BadRequest(format!(
                            "order direction must be 'asc' or 'desc', got '{other}'"
                        )))
                    }
                };
                Ok(format!("{col} {dir}"))
            } else {
                let col = validate_identifier(part)?;
                Ok(format!("{col} ASC"))
            }
        })
        .collect();
    Ok(parts?.join(", "))
}

/// Fetches a column-name → CQL-type map from system_schema for the given table.
/// Returns NotFound if the table has no columns (i.e. it doesn't exist).
pub async fn fetch_column_types(
    session: &Session,
    keyspace: &str,
    table: &str,
) -> Result<HashMap<String, String>, ApiError> {
    let result = session
        .query_unpaged(
            "SELECT column_name, type FROM system_schema.columns \
             WHERE keyspace_name = ? AND table_name = ?",
            (keyspace.to_string(), table.to_string()),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let rows_result = result
        .into_rows_result()
        .map_err(|e| ApiError::Db(e.to_string()))?;
    let map: HashMap<String, String> = rows_result
        .rows::<(String, String)>()
        .map_err(|e| ApiError::Db(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    if map.is_empty() {
        return Err(ApiError::NotFound(format!(
            "table '{table}' not found in keyspace '{keyspace}'"
        )));
    }
    Ok(map)
}
