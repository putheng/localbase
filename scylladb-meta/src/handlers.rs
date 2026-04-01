use actix_web::{delete, get, patch, post, web, Responder};
use serde_json::Value;

use crate::errors::ApiError;
use crate::helpers::{
    fetch_column_types, json_to_cql_literal, maybe_inject_select_json, validate_identifier,
};
use crate::models::{
    ColumnMeta, CreateKeyspaceBody, CreateTableBody, DeleteRowBody, ExecuteQueryBody,
    HealthResponse, InsertRowBody, KeyspaceMeta, KeyspacePath, MessageResponse, QueryResponse,
    RowsQuery, RowsResponse, TableMeta, TablePath, UpdateRowBody,
};
use crate::state::AppState;

// GET /health
#[get("/health")]
pub async fn health() -> impl Responder {
    web::Json(HealthResponse {
        status: "ok",
        service: "scylladb-meta",
    })
}

// GET /meta/keyspaces
#[get("/meta/keyspaces")]
pub async fn list_keyspaces(data: web::Data<AppState>) -> Result<impl Responder, ApiError> {
    use std::collections::HashMap;

    let result: scylla::QueryResult = data
        .session
        .query_unpaged(
            "SELECT keyspace_name, replication, durable_writes \
             FROM system_schema.keyspaces",
            (),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let rows_result: scylla::QueryRowsResult = result
        .into_rows_result()
        .map_err(|e| ApiError::Db(e.to_string()))?;
    let keyspaces: Vec<KeyspaceMeta> = rows_result
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
pub async fn create_keyspace(
    data: web::Data<AppState>,
    body: web::Json<CreateKeyspaceBody>,
) -> Result<impl Responder, ApiError> {
    let name: String = validate_identifier(&body.name)?;
    let strategy = match body.strategy.as_str() {
        "SimpleStrategy" | "NetworkTopologyStrategy" => body.strategy.as_str(),
        _ => {
            return Err(ApiError::BadRequest(
                "strategy must be SimpleStrategy or NetworkTopologyStrategy".to_string(),
            ))
        }
    };
    let rf: i32 = body.replication_factor.clamp(1, 10);

    let cql: String = format!(
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
pub async fn drop_keyspace(
    data: web::Data<AppState>,
    path: web::Path<KeyspacePath>,
) -> Result<impl Responder, ApiError> {
    let ks: String = validate_identifier(&path.keyspace)?;
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
pub async fn list_tables(
    data: web::Data<AppState>,
    path: web::Path<KeyspacePath>,
) -> Result<impl Responder, ApiError> {
    let ks: String = validate_identifier(&path.keyspace)?;

    let result: scylla::QueryResult = data
        .session
        .query_unpaged(
            "SELECT keyspace_name, table_name \
             FROM system_schema.tables \
             WHERE keyspace_name = ?",
            (ks.clone(),),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let rows_result: scylla::QueryRowsResult = result
        .into_rows_result()
        .map_err(|e| ApiError::Db(e.to_string()))?;
    let tables: Vec<TableMeta> = rows_result
        .rows::<(String, String)>()
        .map_err(|e| ApiError::Db(e.to_string()))?
        .filter_map(|r: Result<(String, String), _>| r.ok())
        .map(|(keyspace, name)| TableMeta { keyspace, name })
        .collect::<Vec<_>>();

    Ok(web::Json(tables))
}

// POST /meta/keyspaces/{keyspace}/tables  — create table
#[post("/meta/keyspaces/{keyspace}/tables")]
pub async fn create_table(
    data: web::Data<AppState>,
    path: web::Path<KeyspacePath>,
    body: web::Json<CreateTableBody>,
) -> Result<impl Responder, ApiError> {
    let ks: String = validate_identifier(&path.keyspace)?;
    let tbl: String = validate_identifier(&body.name)?;

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
        let first: String = col_defs[0]
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string();
        pk_cols.push(first);
    }

    let pk_inner: String = if pk_cols.len() == 1 {
        pk_cols[0].clone()
    } else {
        format!("({})", pk_cols.join(", "))
    };

    let primary_key: String = if ck_cols.is_empty() {
        pk_inner
    } else {
        format!("{}, {}", pk_inner, ck_cols.join(", "))
    };

    let cql: String = format!(
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
pub async fn drop_table(
    data: web::Data<AppState>,
    path: web::Path<TablePath>,
) -> Result<impl Responder, ApiError> {
    let ks: String = validate_identifier(&path.keyspace)?;
    let tbl: String = validate_identifier(&path.table)?;
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
pub async fn list_columns(
    data: web::Data<AppState>,
    path: web::Path<TablePath>,
) -> Result<impl Responder, ApiError> {
    let ks: String = validate_identifier(&path.keyspace)?;
    let tbl: String = validate_identifier(&path.table)?;

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

    let rows_result: scylla::QueryRowsResult = result
        .into_rows_result()
        .map_err(|e| ApiError::Db(e.to_string()))?;
    let columns: Vec<ColumnMeta> = rows_result
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
#[get("/meta/keyspaces/{keyspace}/tables/{table}/rows")]
pub async fn get_rows(
    data: web::Data<AppState>,
    path: web::Path<TablePath>,
    query: web::Query<RowsQuery>,
) -> Result<impl Responder, ApiError> {
    let ks: String = validate_identifier(&path.keyspace)?;
    let tbl: String = validate_identifier(&path.table)?;
    let page_size: i32 = query.page_size.unwrap_or(100).clamp(1, 1000);

    let cql: String = format!("SELECT JSON * FROM {}.{} LIMIT {}", ks, tbl, page_size);
    let result: scylla::QueryResult = data
        .session
        .query_unpaged(cql, ())
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let rows_result: scylla::QueryRowsResult = result
        .into_rows_result()
        .map_err(|e| ApiError::Db(e.to_string()))?;
    let rows: Vec<Value> = rows_result
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
#[post("/meta/keyspaces/{keyspace}/tables/{table}/rows")]
pub async fn insert_row(
    data: web::Data<AppState>,
    path: web::Path<TablePath>,
    body: web::Json<InsertRowBody>,
) -> Result<impl Responder, ApiError> {
    let ks: String = validate_identifier(&path.keyspace)?;
    let tbl: String = validate_identifier(&path.table)?;

    if !body.data.is_object() {
        return Err(ApiError::BadRequest(
            "data must be a JSON object".to_string(),
        ));
    }

    let json_str: String = serde_json::to_string(&body.data)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    let escaped: String = json_str.replace('\'', "''");
    let cql: String = format!("INSERT INTO {}.{} JSON '{}'", ks, tbl, escaped);

    data.session
        .query_unpaged(cql, ())
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    Ok(web::Json(MessageResponse {
        message: "Row inserted".to_string(),
    }))
}

// PATCH /meta/keyspaces/{keyspace}/tables/{table}/rows
#[patch("/meta/keyspaces/{keyspace}/tables/{table}/rows")]
pub async fn update_row(
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

    for col in body.set.keys().chain(body.where_clause.keys()) {
        validate_identifier(col)?;
    }

    let col_types: std::collections::HashMap<String, String> = fetch_column_types(&data.session, &ks, &tbl).await?;

    let set_clause: String = body
        .set
        .iter()
        .map(|(col, val)| {
            let dt = col_types.get(col).map(String::as_str).unwrap_or("text");
            format!("{} = {}", col, json_to_cql_literal(val, dt))
        })
        .collect::<Vec<_>>()
        .join(", ");

    let where_clause: String = body
        .where_clause
        .iter()
        .map(|(col, val)| {
            let dt = col_types.get(col).map(String::as_str).unwrap_or("text");
            format!("{} = {}", col, json_to_cql_literal(val, dt))
        })
        .collect::<Vec<_>>()
        .join(" AND ");

    let cql: String = format!(
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
#[delete("/meta/keyspaces/{keyspace}/tables/{table}/rows")]
pub async fn delete_row(
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
#[post("/meta/query")]
pub async fn execute_query(
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
