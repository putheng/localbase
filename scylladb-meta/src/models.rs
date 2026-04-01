use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Response types ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
}

/// Keyspace metadata from system_schema.keyspaces
#[derive(Debug, Serialize)]
pub struct KeyspaceMeta {
    pub name: String,
    pub replication: HashMap<String, String>,
    pub durable_writes: bool,
}

/// Table metadata from system_schema.tables
#[derive(Debug, Serialize)]
pub struct TableMeta {
    pub keyspace: String,
    pub name: String,
}

/// Column metadata from system_schema.columns
#[derive(Debug, Serialize)]
pub struct ColumnMeta {
    pub keyspace: String,
    pub table: String,
    pub name: String,
    /// "partition_key" | "clustering" | "regular" | "static"
    pub kind: String,
    pub position: i32,
    pub data_type: String,
}

/// Paginated row result (rows are returned as JSON objects via SELECT JSON)
#[derive(Debug, Serialize)]
pub struct RowsResponse {
    pub keyspace: String,
    pub table: String,
    pub rows: Vec<Value>,
    pub page_size: i32,
}

/// Generic CQL execution result
#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub query: String,
    pub rows: Option<Vec<Value>>,
    pub message: Option<String>,
    pub execution_time_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

// ── Request types ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct KeyspacePath {
    pub keyspace: String,
}

#[derive(Debug, Deserialize)]
pub struct TablePath {
    pub keyspace: String,
    pub table: String,
}

#[derive(Debug, Deserialize)]
pub struct RowsQuery {
    /// Max rows returned (1-1000, default 100)
    pub page_size: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateKeyspaceBody {
    pub name: String,
    #[serde(default = "default_rf")]
    pub replication_factor: i32,
    #[serde(default = "default_strategy")]
    pub strategy: String,
}

fn default_rf() -> i32 {
    1
}
fn default_strategy() -> String {
    "SimpleStrategy".to_string()
}

#[derive(Debug, Deserialize)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: String,
    /// "partition_key" | "clustering" | "regular" (default)
    #[serde(default)]
    pub kind: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTableBody {
    pub name: String,
    pub columns: Vec<ColumnDef>,
}

#[derive(Debug, Deserialize)]
pub struct InsertRowBody {
    /// JSON object mapping column names to values
    pub data: Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRowBody {
    /// Columns to update: { "col": new_value }
    pub set: HashMap<String, Value>,
    /// Primary key to identify the row: { "pk_col": value }
    #[serde(rename = "where")]
    pub where_clause: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteRowBody {
    /// Primary key to identify the row: { "pk_col": value }
    #[serde(rename = "where")]
    pub where_clause: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteQueryBody {
    pub query: String,
}
