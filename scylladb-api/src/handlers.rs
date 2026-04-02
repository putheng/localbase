use std::collections::HashMap;

use actix_web::{delete, get, patch, post, web, HttpRequest, HttpResponse, Responder};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use scylla::statement::query::Query;
use scylla::transport::{PagingState, PagingStateResponse};
use serde_json::Value;
use std::sync::Arc;

use crate::errors::ApiError;
use crate::helpers::{
    fetch_column_types, json_to_cql_literal, parse_filter, parse_order_clause,
    validate_identifier,
};
use crate::models::HealthResponse;
use crate::state::AppState;

/// Query param names that are not filter column names.
const RESERVED: &[&str] = &["select", "order", "limit", "page_token"];

// ── GET /health ────────────────────────────────────────────────────────────────

#[get("/health")]
pub async fn health() -> impl Responder {
    web::Json(HealthResponse {
        status: "ok",
        service: "scylladb-api",
    })
}

// ── GET /{table} ───────────────────────────────────────────────────────────────
//
// Query params:
//   select=col1,col2           column projection
//   col=op.value               filter  (eq neq gt gte lt lte in is)
//   order=col.asc,col2.desc    ordering (clustering columns only in CQL)
//   limit=100                  page size (1-1000, default 100)
//   page_token=<base64>        cursor returned in X-Next-Page-Token header

#[get("/{table}")]
pub async fn select_rows(
    data: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<HashMap<String, String>>,
) -> Result<HttpResponse, ApiError> {
    let table = validate_identifier(&path)?;
    let col_types = fetch_column_types(&data.session, &data.keyspace, &table).await?;

    // -- SELECT clause --------------------------------------------------------
    let select_clause = if let Some(s) = query.get("select") {
        let cols: Result<Vec<String>, _> =
            s.split(',').map(|c| validate_identifier(c.trim())).collect();
        format!("JSON {}", cols?.join(", "))
    } else {
        "JSON *".to_string()
    };

    // -- WHERE clause ---------------------------------------------------------
    let mut where_parts: Vec<String> = Vec::new();
    for (key, val) in query.iter() {
        if RESERVED.contains(&key.as_str()) {
            continue;
        }
        where_parts.push(parse_filter(key, val, &col_types)?.cql_fragment);
    }

    // -- ORDER BY clause ------------------------------------------------------
    let order_clause = query
        .get("order")
        .map(|s| parse_order_clause(s))
        .transpose()?;

    // -- LIMIT ----------------------------------------------------------------
    let limit: i32 = query
        .get("limit")
        .and_then(|l| l.parse().ok())
        .unwrap_or(100)
        .clamp(1, 1000);

    // -- Build CQL ------------------------------------------------------------
    let mut cql = format!(
        "SELECT {} FROM {}.{}",
        select_clause, data.keyspace, table
    );
    if !where_parts.is_empty() {
        cql.push_str(&format!(" WHERE {}", where_parts.join(" AND ")));
    }
    if let Some(order) = order_clause {
        cql.push_str(&format!(" ORDER BY {order}"));
    }
    cql.push_str(&format!(" LIMIT {limit}"));
    // ALLOW FILTERING must come last in CQL (after LIMIT).
    // Required for non-primary-key column filters; clients should add
    // secondary indexes on frequently-filtered columns for performance.
    if !where_parts.is_empty() {
        cql.push_str(" ALLOW FILTERING");
    }

    // -- Paging state ---------------------------------------------------------
    let paging_state = if let Some(token) = query.get("page_token") {
        match BASE64.decode(token) {
            Ok(raw) => PagingState::new_from_raw_bytes(Arc::<[u8]>::from(raw.as_slice())),
            Err(_) => return Err(ApiError::BadRequest("invalid page_token".into())),
        }
    } else {
        PagingState::start()
    };

    let mut scylla_query = Query::new(cql);
    scylla_query.set_page_size(limit);

    // -- Execute --------------------------------------------------------------
    let (result, paging_response) = data
        .session
        .query_single_page(scylla_query, (), paging_state)
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let rows_result = result
        .into_rows_result()
        .map_err(|e| ApiError::Db(e.to_string()))?;
    let rows: Vec<Value> = rows_result
        .rows::<(String,)>()
        .map_err(|e| ApiError::Db(e.to_string()))?
        .filter_map(|r| r.ok())
        .filter_map(|(s,)| serde_json::from_str::<Value>(&s).ok())
        .collect();

    // -- Build HTTP response --------------------------------------------------
    let mut resp = HttpResponse::Ok();
    resp.append_header((
        "Content-Range",
        format!("0-{}/{}", rows.len().saturating_sub(1), rows.len()),
    ));

    if !where_parts.is_empty() {
        resp.append_header(("X-Warning", "allow-filtering-applied"));
    }

    if let PagingStateResponse::HasMorePages { state } = paging_response {
        if let Some(arc_bytes) = state.as_bytes_slice() {
            resp.append_header(("X-Next-Page-Token", BASE64.encode(arc_bytes.as_ref())));
        }
    }

    Ok(resp.json(rows))
}

// ── POST /{table} ──────────────────────────────────────────────────────────────
//
// Body: single JSON object OR JSON array of objects.
// Header: Prefer: return=representation → 201 with body, else 201 empty.

#[post("/{table}")]
pub async fn insert_rows(
    data: web::Data<AppState>,
    path: web::Path<String>,
    req: HttpRequest,
    body: web::Json<Value>,
) -> Result<HttpResponse, ApiError> {
    let table = validate_identifier(&path)?;

    let prefer_return = req
        .headers()
        .get("Prefer")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("return=representation"))
        .unwrap_or(false);

    let rows: Vec<Value> = match body.into_inner() {
        Value::Array(arr) => arr,
        obj @ Value::Object(_) => vec![obj],
        _ => {
            return Err(ApiError::BadRequest(
                "body must be a JSON object or array of objects".into(),
            ))
        }
    };

    for row in &rows {
        if !row.is_object() {
            return Err(ApiError::BadRequest(
                "each element must be a JSON object".into(),
            ));
        }
        let json_str = serde_json::to_string(row)
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;
        let escaped = json_str.replace('\'', "''");
        let cql = format!(
            "INSERT INTO {}.{} JSON '{}'",
            data.keyspace, table, escaped
        );
        data.session
            .query_unpaged(cql, ())
            .await
            .map_err(|e| ApiError::Db(e.to_string()))?;
    }

    if prefer_return {
        Ok(HttpResponse::Created().json(rows))
    } else {
        Ok(HttpResponse::Created().finish())
    }
}

// ── PATCH /{table}?col=op.value ───────────────────────────────────────────────
//
// Body: JSON object with column→value pairs to set.
// At least one filter query param is required.

#[patch("/{table}")]
pub async fn update_rows(
    data: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<HashMap<String, String>>,
    body: web::Json<Value>,
) -> Result<HttpResponse, ApiError> {
    let table = validate_identifier(&path)?;
    let col_types = fetch_column_types(&data.session, &data.keyspace, &table).await?;

    let set_obj = body.as_object().ok_or_else(|| {
        ApiError::BadRequest("request body must be a JSON object".into())
    })?;
    if set_obj.is_empty() {
        return Err(ApiError::BadRequest("request body must not be empty".into()));
    }

    let set_parts: Result<Vec<String>, ApiError> = set_obj
        .iter()
        .map(|(col, val)| {
            let col = validate_identifier(col)?;
            let dt = col_types.get(&col).map(String::as_str).unwrap_or("text");
            Ok(format!("{col} = {}", json_to_cql_literal(val, dt)))
        })
        .collect();

    let mut where_parts: Vec<String> = Vec::new();
    for (key, val) in query.iter() {
        if RESERVED.contains(&key.as_str()) {
            continue;
        }
        where_parts.push(parse_filter(key, val, &col_types)?.cql_fragment);
    }
    if where_parts.is_empty() {
        return Err(ApiError::BadRequest(
            "PATCH requires at least one filter query param (e.g. ?id=eq.1)".into(),
        ));
    }

    let cql = format!(
        "UPDATE {}.{} SET {} WHERE {}",
        data.keyspace,
        table,
        set_parts?.join(", "),
        where_parts.join(" AND ")
    );
    data.session
        .query_unpaged(cql, ())
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    Ok(HttpResponse::NoContent().finish())
}

// ── DELETE /{table}?col=op.value ──────────────────────────────────────────────
//
// At least one filter query param is required.

#[delete("/{table}")]
pub async fn delete_rows(
    data: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<HashMap<String, String>>,
) -> Result<HttpResponse, ApiError> {
    let table = validate_identifier(&path)?;
    let col_types = fetch_column_types(&data.session, &data.keyspace, &table).await?;

    let mut where_parts: Vec<String> = Vec::new();
    for (key, val) in query.iter() {
        if RESERVED.contains(&key.as_str()) {
            continue;
        }
        where_parts.push(parse_filter(key, val, &col_types)?.cql_fragment);
    }
    if where_parts.is_empty() {
        return Err(ApiError::BadRequest(
            "DELETE requires at least one filter query param (e.g. ?id=eq.1)".into(),
        ));
    }

    let cql = format!(
        "DELETE FROM {}.{} WHERE {}",
        data.keyspace,
        table,
        where_parts.join(" AND ")
    );
    data.session
        .query_unpaged(cql, ())
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    Ok(HttpResponse::NoContent().finish())
}
