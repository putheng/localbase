use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use actix_cors::Cors;
use actix_web::http::header;
use actix_web::{
    delete, get, middleware::Logger, patch, post, web, App, HttpResponse, HttpServer,
    Responder, ResponseError,
};
use dotenvy::dotenv;
use log::info;
use scylla::{Session, SessionBuilder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

// ── State ──────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct AppState {
    session: Arc<Session>,
}

// ── Errors ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ApiErrorBody {
    error: String,
}

#[derive(Debug, Error)]
enum ApiError {
    #[error("Database error: {0}")]
    Db(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        let status = match self {
            ApiError::BadRequest(_) => actix_web::http::StatusCode::BAD_REQUEST,
            ApiError::Db(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        };
        HttpResponse::build(status).json(ApiErrorBody {
            error: self.to_string(),
        })
    }
}

// ── Response types ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

/// Keyspace metadata from system_schema.keyspaces
#[derive(Debug, Serialize)]
struct KeyspaceMeta {
    name: String,
    /// replication map, e.g. {"class":"SimpleStrategy","replication_factor":"1"}
    replication: HashMap<String, String>,
    durable_writes: bool,
}

/// Table metadata from system_schema.tables
#[derive(Debug, Serialize)]
struct TableMeta {
    keyspace: String,
    name: String,
}

/// Column metadata from system_schema.columns
#[derive(Debug, Serialize)]
struct ColumnMeta {
    keyspace: String,
    table: String,
    name: String,
    /// "partition_key" | "clustering" | "regular" | "static"
    kind: String,
    position: i32,
    data_type: String,
}

/// Paginated row result (rows are returned as JSON objects via SELECT JSON)
#[derive(Debug, Serialize)]
struct RowsResponse {
    keyspace: String,
    table: String,
    rows: Vec<Value>,
    page_size: i32,
}

/// Generic CQL execution result
#[derive(Debug, Serialize)]
struct QueryResponse {
    query: String,
    rows: Option<Vec<Value>>,
    message: Option<String>,
    execution_time_ms: u64,
}

#[derive(Debug, Serialize)]
struct MessageResponse {
    message: String,
}

// ── Request types ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct KeyspacePath {
    keyspace: String,
}

#[derive(Debug, Deserialize)]
struct TablePath {
    keyspace: String,
    table: String,
}

#[derive(Debug, Deserialize)]
struct RowsQuery {
    /// Max rows returned (1-1000, default 100)
    page_size: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct CreateKeyspaceBody {
    name: String,
    #[serde(default = "default_rf")]
    replication_factor: i32,
    #[serde(default = "default_strategy")]
    strategy: String,
}
fn default_rf() -> i32 {
    1
}
fn default_strategy() -> String {
    "SimpleStrategy".to_string()
}

#[derive(Debug, Deserialize)]
struct ColumnDef {
    name: String,
    data_type: String,
    /// "partition_key" | "clustering" | "regular" (default)
    #[serde(default)]
    kind: String,
}

#[derive(Debug, Deserialize)]
struct CreateTableBody {
    name: String,
    columns: Vec<ColumnDef>,
}

#[derive(Debug, Deserialize)]
struct InsertRowBody {
    /// JSON object mapping column names to values
    data: Value,
}

#[derive(Debug, Deserialize)]
struct UpdateRowBody {
    /// Columns to update: { "col": new_value }
    set: HashMap<String, Value>,
    /// Primary key to identify the row: { "pk_col": value }
    #[serde(rename = "where")]
    where_clause: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
struct DeleteRowBody {
    /// Primary key to identify the row: { "pk_col": value }
    #[serde(rename = "where")]
    where_clause: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
struct ExecuteQueryBody {
    query: String,
}

// ── Helpers ────────────────────────────────────────────────────────────────────

/// Validates a CQL identifier (keyspace / table / column name) to prevent injection.
/// Only allows ASCII alphanumerics and underscores.
fn validate_identifier(s: &str) -> Result<String, ApiError> {
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
fn json_to_cql_literal(val: &Value, data_type: &str) -> String {
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
fn maybe_inject_select_json(query: &str) -> String {
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
async fn fetch_column_types(
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

// ── Handlers ───────────────────────────────────────────────────────────────────

// GET /health
#[get("/health")]
async fn health() -> impl Responder {
    web::Json(HealthResponse {
        status: "ok",
        service: "scylladb-meta",
    })
}

// GET /meta/keyspaces
#[get("/meta/keyspaces")]
async fn list_keyspaces(data: web::Data<AppState>) -> Result<impl Responder, ApiError> {
    let result = data
        .session
        .query_unpaged(
            "SELECT keyspace_name, replication, durable_writes \
             FROM system_schema.keyspaces",
            (),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let rows_result = result
        .into_rows_result()
        .map_err(|e| ApiError::Db(e.to_string()))?;
    let keyspaces = rows_result
        .rows::<(String, HashMap<String, String>, bool)>()
        .map_err(|e| ApiError::Db(e.to_string()))?
        .filter_map(|r: Result<(String, HashMap<String, String>, bool), _>| r.ok())
        .map(|(name, replication, durable_writes)| KeyspaceMeta {
            name,
            replication,
            durable_writes,
        })
        .collect::<Vec<_>>();

    Ok(web::Json(keyspaces))
}

// POST /meta/keyspaces  — create keyspace
#[post("/meta/keyspaces")]
async fn create_keyspace(
    data: web::Data<AppState>,
    body: web::Json<CreateKeyspaceBody>,
) -> Result<impl Responder, ApiError> {
    let name = validate_identifier(&body.name)?;
    let strategy = match body.strategy.as_str() {
        "SimpleStrategy" | "NetworkTopologyStrategy" => body.strategy.as_str(),
        _ => {
            return Err(ApiError::BadRequest(
                "strategy must be SimpleStrategy or NetworkTopologyStrategy".to_string(),
            ))
        }
    };
    let rf = body.replication_factor.clamp(1, 10);

    let cql = format!(
        "CREATE KEYSPACE IF NOT EXISTS {} \
         WITH REPLICATION = {{'class': '{}', 'replication_factor': {}}} \
         AND DURABLE_WRITES = true",
        name, strategy, rf
    );
    data.session
        .query_unpaged(cql, ())
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    Ok(web::Json(MessageResponse {
        message: format!("Keyspace '{name}' created"),
    }))
}

// DELETE /meta/keyspaces/{keyspace}
#[delete("/meta/keyspaces/{keyspace}")]
async fn drop_keyspace(
    data: web::Data<AppState>,
    path: web::Path<KeyspacePath>,
) -> Result<impl Responder, ApiError> {
    let ks = validate_identifier(&path.keyspace)?;
    data.session
        .query_unpaged(format!("DROP KEYSPACE IF EXISTS {}", ks), ())
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;
    Ok(web::Json(MessageResponse {
        message: format!("Keyspace '{ks}' dropped"),
    }))
}

// GET /meta/keyspaces/{keyspace}/tables
#[get("/meta/keyspaces/{keyspace}/tables")]
async fn list_tables(
    data: web::Data<AppState>,
    path: web::Path<KeyspacePath>,
) -> Result<impl Responder, ApiError> {
    let ks = validate_identifier(&path.keyspace)?;

    let result = data
        .session
        .query_unpaged(
            "SELECT keyspace_name, table_name \
             FROM system_schema.tables \
             WHERE keyspace_name = ?",
            (ks.clone(),),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let rows_result = result
        .into_rows_result()
        .map_err(|e| ApiError::Db(e.to_string()))?;
    let tables = rows_result
        .rows::<(String, String)>()
        .map_err(|e| ApiError::Db(e.to_string()))?
        .filter_map(|r: Result<(String, String), _>| r.ok())
        .map(|(keyspace, name)| TableMeta { keyspace, name })
        .collect::<Vec<_>>();

    Ok(web::Json(tables))
}

// POST /meta/keyspaces/{keyspace}/tables  — create table
#[post("/meta/keyspaces/{keyspace}/tables")]
async fn create_table(
    data: web::Data<AppState>,
    path: web::Path<KeyspacePath>,
    body: web::Json<CreateTableBody>,
) -> Result<impl Responder, ApiError> {
    let ks = validate_identifier(&path.keyspace)?;
    let tbl = validate_identifier(&body.name)?;

    if body.columns.is_empty() {
        return Err(ApiError::BadRequest(
            "at least one column is required".to_string(),
        ));
    }

    let mut col_defs: Vec<String> = Vec::new();
    let mut pk_cols: Vec<String> = Vec::new();
    let mut ck_cols: Vec<String> = Vec::new();

    for col in &body.columns {
        let cname = validate_identifier(&col.name)?;
        col_defs.push(format!("{} {}", cname, col.data_type));
        match col.kind.to_lowercase().as_str() {
            "partition_key" | "partition" => pk_cols.push(cname),
            "clustering" => ck_cols.push(cname),
            _ => {} // regular / static
        }
    }

    // Fall back: use the first column as partition key if none declared
    if pk_cols.is_empty() {
        let first = col_defs[0]
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string();
        pk_cols.push(first);
    }

    let pk_inner = if pk_cols.len() == 1 {
        pk_cols[0].clone()
    } else {
        format!("({})", pk_cols.join(", "))
    };

    let primary_key = if ck_cols.is_empty() {
        pk_inner
    } else {
        format!("{}, {}", pk_inner, ck_cols.join(", "))
    };

    let cql = format!(
        "CREATE TABLE IF NOT EXISTS {}.{} ({}, PRIMARY KEY ({}))",
        ks,
        tbl,
        col_defs.join(", "),
        primary_key
    );
    data.session
        .query_unpaged(cql, ())
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    Ok(web::Json(MessageResponse {
        message: format!("Table '{ks}.{tbl}' created"),
    }))
}

// DELETE /meta/keyspaces/{keyspace}/tables/{table}
#[delete("/meta/keyspaces/{keyspace}/tables/{table}")]
async fn drop_table(
    data: web::Data<AppState>,
    path: web::Path<TablePath>,
) -> Result<impl Responder, ApiError> {
    let ks = validate_identifier(&path.keyspace)?;
    let tbl = validate_identifier(&path.table)?;
    data.session
        .query_unpaged(format!("DROP TABLE IF EXISTS {}.{}", ks, tbl), ())
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;
    Ok(web::Json(MessageResponse {
        message: format!("Table '{ks}.{tbl}' dropped"),
    }))
}

// GET /meta/keyspaces/{keyspace}/tables/{table}/columns
#[get("/meta/keyspaces/{keyspace}/tables/{table}/columns")]
async fn list_columns(
    data: web::Data<AppState>,
    path: web::Path<TablePath>,
) -> Result<impl Responder, ApiError> {
    let ks = validate_identifier(&path.keyspace)?;
    let tbl = validate_identifier(&path.table)?;

    let result = data
        .session
        .query_unpaged(
            "SELECT keyspace_name, table_name, column_name, kind, position, type \
             FROM system_schema.columns \
             WHERE keyspace_name = ? AND table_name = ?",
            (ks.clone(), tbl.clone()),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let rows_result = result
        .into_rows_result()
        .map_err(|e| ApiError::Db(e.to_string()))?;
    let columns = rows_result
        .rows::<(String, String, String, String, i32, String)>()
        .map_err(|e| ApiError::Db(e.to_string()))?
        .filter_map(|r: Result<(String, String, String, String, i32, String), _>| r.ok())
        .map(
            |(keyspace, table, name, kind, position, data_type)| ColumnMeta {
                keyspace,
                table,
                name,
                kind,
                position,
                data_type,
            },
        )
        .collect::<Vec<_>>();

    Ok(web::Json(columns))
}

// GET /meta/keyspaces/{keyspace}/tables/{table}/rows?page_size=100
// Uses SELECT JSON so rows are returned as arbitrary JSON objects
#[get("/meta/keyspaces/{keyspace}/tables/{table}/rows")]
async fn get_rows(
    data: web::Data<AppState>,
    path: web::Path<TablePath>,
    query: web::Query<RowsQuery>,
) -> Result<impl Responder, ApiError> {
    let ks = validate_identifier(&path.keyspace)?;
    let tbl = validate_identifier(&path.table)?;
    let page_size = query.page_size.unwrap_or(100).clamp(1, 1000);

    let cql = format!("SELECT JSON * FROM {}.{} LIMIT {}", ks, tbl, page_size);
    let result = data
        .session
        .query_unpaged(cql, ())
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let rows_result = result
        .into_rows_result()
        .map_err(|e| ApiError::Db(e.to_string()))?;
    let rows = rows_result
        .rows::<(String,)>()
        .map_err(|e| ApiError::Db(e.to_string()))?
        .filter_map(|r: Result<(String,), _>| r.ok())
        .filter_map(|(s,): (String,)| serde_json::from_str::<Value>(&s).ok())
        .collect::<Vec<_>>();

    Ok(web::Json(RowsResponse {
        keyspace: ks,
        table: tbl,
        rows,
        page_size,
    }))
}

// POST /meta/keyspaces/{keyspace}/tables/{table}/rows  — insert a row
// Uses INSERT INTO … JSON which maps JSON field names directly to column names
#[post("/meta/keyspaces/{keyspace}/tables/{table}/rows")]
async fn insert_row(
    data: web::Data<AppState>,
    path: web::Path<TablePath>,
    body: web::Json<InsertRowBody>,
) -> Result<impl Responder, ApiError> {
    let ks = validate_identifier(&path.keyspace)?;
    let tbl = validate_identifier(&path.table)?;

    if !body.data.is_object() {
        return Err(ApiError::BadRequest(
            "data must be a JSON object".to_string(),
        ));
    }

    let json_str = serde_json::to_string(&body.data)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    // Escape embedded single quotes inside the JSON literal
    let escaped = json_str.replace('\'', "''");
    let cql = format!("INSERT INTO {}.{} JSON '{}'", ks, tbl, escaped);

    data.session
        .query_unpaged(cql, ())
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    Ok(web::Json(MessageResponse {
        message: "Row inserted".to_string(),
    }))
}

// PATCH /meta/keyspaces/{keyspace}/tables/{table}/rows
// Body: { "set": { "col": value }, "where": { "pk_col": value } }
#[patch("/meta/keyspaces/{keyspace}/tables/{table}/rows")]
async fn update_row(
    data: web::Data<AppState>,
    path: web::Path<TablePath>,
    body: web::Json<UpdateRowBody>,
) -> Result<impl Responder, ApiError> {
    let ks = validate_identifier(&path.keyspace)?;
    let tbl = validate_identifier(&path.table)?;

    if body.set.is_empty() {
        return Err(ApiError::BadRequest("`set` must not be empty".to_string()));
    }
    if body.where_clause.is_empty() {
        return Err(ApiError::BadRequest(
            "`where` must not be empty".to_string(),
        ));
    }

    // Validate every supplied column name
    for col in body.set.keys().chain(body.where_clause.keys()) {
        validate_identifier(col)?;
    }

    let col_types = fetch_column_types(&data.session, &ks, &tbl).await?;

    let set_clause = body
        .set
        .iter()
        .map(|(col, val)| {
            let dt = col_types.get(col).map(String::as_str).unwrap_or("text");
            format!("{} = {}", col, json_to_cql_literal(val, dt))
        })
        .collect::<Vec<_>>()
        .join(", ");

    let where_clause = body
        .where_clause
        .iter()
        .map(|(col, val)| {
            let dt = col_types.get(col).map(String::as_str).unwrap_or("text");
            format!("{} = {}", col, json_to_cql_literal(val, dt))
        })
        .collect::<Vec<_>>()
        .join(" AND ");

    let cql = format!(
        "UPDATE {}.{} SET {} WHERE {}",
        ks, tbl, set_clause, where_clause
    );
    data.session
        .query_unpaged(cql, ())
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    Ok(web::Json(MessageResponse {
        message: "Row updated".to_string(),
    }))
}

// DELETE /meta/keyspaces/{keyspace}/tables/{table}/rows
// Body: { "where": { "pk_col": value } }
#[delete("/meta/keyspaces/{keyspace}/tables/{table}/rows")]
async fn delete_row(
    data: web::Data<AppState>,
    path: web::Path<TablePath>,
    body: web::Json<DeleteRowBody>,
) -> Result<impl Responder, ApiError> {
    let ks = validate_identifier(&path.keyspace)?;
    let tbl = validate_identifier(&path.table)?;

    if body.where_clause.is_empty() {
        return Err(ApiError::BadRequest(
            "`where` must not be empty".to_string(),
        ));
    }

    for col in body.where_clause.keys() {
        validate_identifier(col)?;
    }

    let col_types = fetch_column_types(&data.session, &ks, &tbl).await?;

    let where_clause = body
        .where_clause
        .iter()
        .map(|(col, val)| {
            let dt = col_types.get(col).map(String::as_str).unwrap_or("text");
            format!("{} = {}", col, json_to_cql_literal(val, dt))
        })
        .collect::<Vec<_>>()
        .join(" AND ");

    let cql = format!("DELETE FROM {}.{} WHERE {}", ks, tbl, where_clause);
    data.session
        .query_unpaged(cql, ())
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    Ok(web::Json(MessageResponse {
        message: "Row deleted".to_string(),
    }))
}

// POST /meta/query  — execute arbitrary CQL
// SELECT queries are automatically rewritten to SELECT JSON for JSON row output.
#[post("/meta/query")]
async fn execute_query(
    data: web::Data<AppState>,
    body: web::Json<ExecuteQueryBody>,
) -> Result<impl Responder, ApiError> {
    let raw = body.query.trim();
    if raw.is_empty() {
        return Err(ApiError::BadRequest("query cannot be empty".to_string()));
    }

    let is_select = raw.len() >= 6 && raw[..6].eq_ignore_ascii_case("select");
    let cql = if is_select {
        maybe_inject_select_json(raw)
    } else {
        raw.to_string()
    };

    let start = std::time::Instant::now();
    let result = data
        .session
        .query_unpaged(cql, ())
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;
    let execution_time_ms = start.elapsed().as_millis() as u64;

    let (rows, message) = if is_select {
        let r = result.into_rows_result().ok().and_then(|rr| {
            rr.rows::<(String,)>().ok().map(|iter| {
                iter.filter_map(|row: Result<(String,), _>| row.ok())
                    .filter_map(|(s,): (String,)| serde_json::from_str::<Value>(&s).ok())
                    .collect::<Vec<_>>()
            })
        });
        (r, None)
    } else {
        (None, Some("Query executed successfully".to_string()))
    };

    Ok(web::Json(QueryResponse {
        query: body.query.clone(),
        rows,
        message,
        execution_time_ms,
    }))
}

// ── Main ───────────────────────────────────────────────────────────────────────

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let host = env::var("SCYLLA_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("SCYLLA_PORT").unwrap_or_else(|_| "9042".to_string());
    let username = env::var("SCYLLA_USERNAME").unwrap_or_else(|_| "scylla_app".to_string());
    let password =
        env::var("SCYLLA_PASSWORD").unwrap_or_else(|_| "change_me_local".to_string());
    let api_host = env::var("META_API_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let api_port = env::var("META_API_PORT").unwrap_or_else(|_| "8080".to_string());
    let allowed_origin =
        env::var("CORS_ORIGIN").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let scylla_uri = format!("{}:{}", host, port);
    info!("Connecting to ScyllaDB at {}", scylla_uri);

    let session = SessionBuilder::new()
        .known_node(&scylla_uri)
        .user(username, password)
        .build()
        .await
        .map_err(|e| std::io::Error::other(format!("Failed to connect to ScyllaDB: {e}")))?;

    let state = AppState {
        session: Arc::new(session),
    };

    let bind_addr = format!("{}:{}", api_host, api_port);
    info!("Starting scylladb-meta API at http://{}", bind_addr);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(&allowed_origin)
            .allowed_methods(["GET", "POST", "PATCH", "DELETE"])
            .allowed_headers([header::CONTENT_TYPE, header::ACCEPT])
            .supports_credentials()
            .max_age(3600);

        App::new()
            .app_data(web::Data::new(state.clone()))
            .app_data(web::JsonConfig::default().error_handler(|err, _| {
                let body = ApiErrorBody {
                    error: err.to_string(),
                };
                actix_web::error::InternalError::from_response(
                    err,
                    HttpResponse::BadRequest().json(body),
                )
                .into()
            }))
            .wrap(Logger::default())
            .wrap(cors)
            .service(health)
            .service(list_keyspaces)
            .service(create_keyspace)
            .service(drop_keyspace)
            .service(list_tables)
            .service(create_table)
            .service(drop_table)
            .service(list_columns)
            .service(get_rows)
            .service(insert_row)
            .service(update_row)
            .service(delete_row)
            .service(execute_query)
    })
    .bind(bind_addr)?
    .run()
    .await
}

