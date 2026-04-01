use std::env;
use std::sync::Arc;

use actix_cors::Cors;
use actix_web::http::header;
use actix_web::{
    delete, get, middleware::Logger, post, web, App, HttpResponse, HttpServer, Responder,
    ResponseError,
};
use chrono::{DateTime, Utc};
use dotenvy::dotenv;
use log::info;
use scylla::frame::value::CqlTimestamp;
use scylla::{Session, SessionBuilder};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

// ── Migrations ─────────────────────────────────────────────────────────────────

const MIGRATIONS: &[&str] = &[
    // V001 – keyspace
    "CREATE KEYSPACE IF NOT EXISTS storage \
     WITH replication = {'class': 'SimpleStrategy', 'replication_factor': 1} \
     AND durable_writes = true",
    // V002 – buckets
    "CREATE TABLE IF NOT EXISTS storage.buckets ( \
         id uuid, \
         name text, \
         public boolean, \
         created_at timestamp, \
         updated_at timestamp, \
         PRIMARY KEY (id) \
     )",
    // V003a – folders (primary: lookup by bucket + id)
    "CREATE TABLE IF NOT EXISTS storage.folders ( \
         bucket_id uuid, \
         id uuid, \
         parent_id uuid, \
         name text, \
         created_at timestamp, \
         updated_at timestamp, \
         PRIMARY KEY (bucket_id, id) \
     ) WITH CLUSTERING ORDER BY (id ASC)",
    // V003b – folders_by_parent (children listing)
    "CREATE TABLE IF NOT EXISTS storage.folders_by_parent ( \
         bucket_id uuid, \
         parent_id uuid, \
         id uuid, \
         name text, \
         created_at timestamp, \
         updated_at timestamp, \
         PRIMARY KEY ((bucket_id, parent_id), id) \
     ) WITH CLUSTERING ORDER BY (id ASC)",
    // V004a – files (primary: lookup by bucket + id)
    "CREATE TABLE IF NOT EXISTS storage.files ( \
         bucket_id uuid, \
         id uuid, \
         folder_id uuid, \
         name text, \
         size bigint, \
         content_type text, \
         storage_path text, \
         created_at timestamp, \
         updated_at timestamp, \
         PRIMARY KEY (bucket_id, id) \
     ) WITH CLUSTERING ORDER BY (id ASC)",
    // V004b – files_by_folder (folder-scoped listing)
    "CREATE TABLE IF NOT EXISTS storage.files_by_folder ( \
         bucket_id uuid, \
         folder_id uuid, \
         id uuid, \
         name text, \
         size bigint, \
         content_type text, \
         storage_path text, \
         created_at timestamp, \
         updated_at timestamp, \
         PRIMARY KEY ((bucket_id, folder_id), id) \
     ) WITH CLUSTERING ORDER BY (id ASC)",
];

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
    #[error("database error: {0}")]
    Db(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("not found: {0}")]
    NotFound(String),
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        let status = match self {
            ApiError::BadRequest(_) => actix_web::http::StatusCode::BAD_REQUEST,
            ApiError::NotFound(_) => actix_web::http::StatusCode::NOT_FOUND,
            ApiError::Db(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        };
        HttpResponse::build(status).json(ApiErrorBody {
            error: self.to_string(),
        })
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn ts_to_iso(ts: CqlTimestamp) -> String {
    DateTime::<Utc>::from_timestamp_millis(ts.0)
        .unwrap_or_default()
        .to_rfc3339()
}

fn now_ts() -> CqlTimestamp {
    CqlTimestamp(Utc::now().timestamp_millis())
}

fn parse_uuid(s: &str, label: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(s).map_err(|_| ApiError::BadRequest(format!("invalid {label} UUID: {s}")))
}

// ── Response types ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

#[derive(Debug, Serialize)]
struct MessageResponse {
    message: String,
}

#[derive(Debug, Serialize)]
struct BucketResponse {
    id: String,
    name: String,
    public: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct FolderResponse {
    id: String,
    bucket_id: String,
    parent_id: String,
    name: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct FileResponse {
    id: String,
    bucket_id: String,
    folder_id: String,
    name: String,
    size: i64,
    content_type: String,
    storage_path: String,
    created_at: String,
    updated_at: String,
}

// ── Request bodies ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CreateBucketBody {
    name: String,
    #[serde(default)]
    public: bool,
}

#[derive(Debug, Deserialize)]
struct CreateFolderBody {
    name: String,
    /// UUID string; omit or null → root (nil UUID sentinel)
    parent_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateFileBody {
    name: String,
    #[serde(default)]
    size: i64,
    #[serde(default)]
    content_type: String,
    #[serde(default)]
    storage_path: String,
    /// UUID string; omit or null → root (nil UUID sentinel)
    folder_id: Option<String>,
}

// ── Query params ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct FolderQuery {
    parent_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FileQuery {
    folder_id: Option<String>,
}

// ── Path extractors ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct BucketPath {
    bucket_id: String,
}

#[derive(Debug, Deserialize)]
struct BucketFolderPath {
    bucket_id: String,
    folder_id: String,
}

#[derive(Debug, Deserialize)]
struct BucketFilePath {
    bucket_id: String,
    file_id: String,
}

// ── Handlers ───────────────────────────────────────────────────────────────────

// GET /health
#[get("/health")]
async fn health() -> impl Responder {
    web::Json(HealthResponse {
        status: "ok",
        service: "storage",
    })
}

// GET /storage/buckets
#[get("/storage/buckets")]
async fn list_buckets(data: web::Data<AppState>) -> Result<impl Responder, ApiError> {
    let result = data
        .session
        .query_unpaged(
            "SELECT id, name, public, created_at, updated_at FROM storage.buckets",
            (),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let rows = result
        .into_rows_result()
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let buckets = rows
        .rows::<(Uuid, String, bool, CqlTimestamp, CqlTimestamp)>()
        .map_err(|e| ApiError::Db(e.to_string()))?
        .filter_map(|r| r.ok())
        .map(|(id, name, public, created_at, updated_at)| BucketResponse {
            id: id.to_string(),
            name,
            public,
            created_at: ts_to_iso(created_at),
            updated_at: ts_to_iso(updated_at),
        })
        .collect::<Vec<_>>();

    Ok(web::Json(buckets))
}

// POST /storage/buckets
#[post("/storage/buckets")]
async fn create_bucket(
    data: web::Data<AppState>,
    body: web::Json<CreateBucketBody>,
) -> Result<impl Responder, ApiError> {
    let name = body.name.trim().to_lowercase();
    if name.is_empty() {
        return Err(ApiError::BadRequest("name is required".into()));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(ApiError::BadRequest(
            "name may only contain lowercase letters, numbers, and hyphens".into(),
        ));
    }

    let id = Uuid::new_v4();
    let now = now_ts();

    data.session
        .query_unpaged(
            "INSERT INTO storage.buckets (id, name, public, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?)",
            (id, name.clone(), body.public, now, now),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    Ok(web::Json(BucketResponse {
        id: id.to_string(),
        name,
        public: body.public,
        created_at: ts_to_iso(now),
        updated_at: ts_to_iso(now),
    }))
}

// DELETE /storage/buckets/{bucket_id}
#[delete("/storage/buckets/{bucket_id}")]
async fn delete_bucket(
    data: web::Data<AppState>,
    path: web::Path<BucketPath>,
) -> Result<impl Responder, ApiError> {
    let bucket_id = parse_uuid(&path.bucket_id, "bucket_id")?;

    // Cascade: delete all files in the bucket partition
    data.session
        .query_unpaged(
            "DELETE FROM storage.files WHERE bucket_id = ?",
            (bucket_id,),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    // Cascade: delete all folders in the bucket partition
    data.session
        .query_unpaged(
            "DELETE FROM storage.folders WHERE bucket_id = ?",
            (bucket_id,),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    // Delete the bucket itself
    data.session
        .query_unpaged(
            "DELETE FROM storage.buckets WHERE id = ?",
            (bucket_id,),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    Ok(web::Json(MessageResponse {
        message: format!("Bucket {bucket_id} deleted"),
    }))
}

// GET /storage/buckets/{bucket_id}/folders?parent_id={uuid}
// Omit parent_id to list root-level folders.
#[get("/storage/buckets/{bucket_id}/folders")]
async fn list_folders(
    data: web::Data<AppState>,
    path: web::Path<BucketPath>,
    query: web::Query<FolderQuery>,
) -> Result<impl Responder, ApiError> {
    let bucket_id = parse_uuid(&path.bucket_id, "bucket_id")?;
    let parent_id = match &query.parent_id {
        Some(pid) => parse_uuid(pid, "parent_id")?,
        None => Uuid::nil(),
    };

    let result = data
        .session
        .query_unpaged(
            "SELECT id, parent_id, name, created_at, updated_at \
             FROM storage.folders_by_parent \
             WHERE bucket_id = ? AND parent_id = ?",
            (bucket_id, parent_id),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let rows = result
        .into_rows_result()
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let folders = rows
        .rows::<(Uuid, Uuid, String, CqlTimestamp, CqlTimestamp)>()
        .map_err(|e| ApiError::Db(e.to_string()))?
        .filter_map(|r| r.ok())
        .map(|(id, par_id, name, created_at, updated_at)| FolderResponse {
            id: id.to_string(),
            bucket_id: bucket_id.to_string(),
            parent_id: par_id.to_string(),
            name,
            created_at: ts_to_iso(created_at),
            updated_at: ts_to_iso(updated_at),
        })
        .collect::<Vec<_>>();

    Ok(web::Json(folders))
}

// POST /storage/buckets/{bucket_id}/folders
#[post("/storage/buckets/{bucket_id}/folders")]
async fn create_folder(
    data: web::Data<AppState>,
    path: web::Path<BucketPath>,
    body: web::Json<CreateFolderBody>,
) -> Result<impl Responder, ApiError> {
    let bucket_id = parse_uuid(&path.bucket_id, "bucket_id")?;
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::BadRequest("name is required".into()));
    }

    let parent_id = match &body.parent_id {
        Some(pid) => parse_uuid(pid, "parent_id")?,
        None => Uuid::nil(),
    };

    let id = Uuid::new_v4();
    let now = now_ts();

    // Write to primary table
    data.session
        .query_unpaged(
            "INSERT INTO storage.folders \
             (bucket_id, id, parent_id, name, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
            (bucket_id, id, parent_id, name.clone(), now, now),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    // Write to denormalized by_parent table
    data.session
        .query_unpaged(
            "INSERT INTO storage.folders_by_parent \
             (bucket_id, parent_id, id, name, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
            (bucket_id, parent_id, id, name.clone(), now, now),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    Ok(web::Json(FolderResponse {
        id: id.to_string(),
        bucket_id: bucket_id.to_string(),
        parent_id: parent_id.to_string(),
        name,
        created_at: ts_to_iso(now),
        updated_at: ts_to_iso(now),
    }))
}

// DELETE /storage/buckets/{bucket_id}/folders/{folder_id}
#[delete("/storage/buckets/{bucket_id}/folders/{folder_id}")]
async fn delete_folder(
    data: web::Data<AppState>,
    path: web::Path<BucketFolderPath>,
) -> Result<impl Responder, ApiError> {
    let bucket_id = parse_uuid(&path.bucket_id, "bucket_id")?;
    let folder_id = parse_uuid(&path.folder_id, "folder_id")?;

    // Look up parent_id so we can delete from the by_parent table
    let result = data
        .session
        .query_unpaged(
            "SELECT parent_id FROM storage.folders WHERE bucket_id = ? AND id = ?",
            (bucket_id, folder_id),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let rows = result
        .into_rows_result()
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let parent_id = rows
        .rows::<(Uuid,)>()
        .map_err(|e| ApiError::Db(e.to_string()))?
        .filter_map(|r| r.ok())
        .next()
        .map(|(pid,)| pid)
        .ok_or_else(|| ApiError::NotFound(format!("folder {folder_id} not found")))?;

    // Delete from primary table
    data.session
        .query_unpaged(
            "DELETE FROM storage.folders WHERE bucket_id = ? AND id = ?",
            (bucket_id, folder_id),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    // Delete from denormalized by_parent table
    data.session
        .query_unpaged(
            "DELETE FROM storage.folders_by_parent \
             WHERE bucket_id = ? AND parent_id = ? AND id = ?",
            (bucket_id, parent_id, folder_id),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    Ok(web::Json(MessageResponse {
        message: format!("Folder {folder_id} deleted"),
    }))
}

// GET /storage/buckets/{bucket_id}/files?folder_id={uuid}
// Omit folder_id to list root-level files.
#[get("/storage/buckets/{bucket_id}/files")]
async fn list_files(
    data: web::Data<AppState>,
    path: web::Path<BucketPath>,
    query: web::Query<FileQuery>,
) -> Result<impl Responder, ApiError> {
    let bucket_id = parse_uuid(&path.bucket_id, "bucket_id")?;
    let folder_id = match &query.folder_id {
        Some(fid) => parse_uuid(fid, "folder_id")?,
        None => Uuid::nil(),
    };

    let result = data
        .session
        .query_unpaged(
            "SELECT id, folder_id, name, size, content_type, storage_path, \
                     created_at, updated_at \
             FROM storage.files_by_folder \
             WHERE bucket_id = ? AND folder_id = ?",
            (bucket_id, folder_id),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let rows = result
        .into_rows_result()
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let files = rows
        .rows::<(
            Uuid,
            Uuid,
            String,
            Option<i64>,
            Option<String>,
            Option<String>,
            CqlTimestamp,
            CqlTimestamp,
        )>()
        .map_err(|e| ApiError::Db(e.to_string()))?
        .filter_map(|r| r.ok())
        .map(
            |(id, fid, name, size, content_type, storage_path, created_at, updated_at)| {
                FileResponse {
                    id: id.to_string(),
                    bucket_id: bucket_id.to_string(),
                    folder_id: fid.to_string(),
                    name,
                    size: size.unwrap_or(0),
                    content_type: content_type.unwrap_or_default(),
                    storage_path: storage_path.unwrap_or_default(),
                    created_at: ts_to_iso(created_at),
                    updated_at: ts_to_iso(updated_at),
                }
            },
        )
        .collect::<Vec<_>>();

    Ok(web::Json(files))
}

// POST /storage/buckets/{bucket_id}/files
#[post("/storage/buckets/{bucket_id}/files")]
async fn create_file(
    data: web::Data<AppState>,
    path: web::Path<BucketPath>,
    body: web::Json<CreateFileBody>,
) -> Result<impl Responder, ApiError> {
    let bucket_id = parse_uuid(&path.bucket_id, "bucket_id")?;
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::BadRequest("name is required".into()));
    }

    let folder_id = match &body.folder_id {
        Some(fid) => parse_uuid(fid, "folder_id")?,
        None => Uuid::nil(),
    };

    let id = Uuid::new_v4();
    let now = now_ts();
    let size = body.size;
    let content_type = body.content_type.clone();
    let storage_path = if body.storage_path.is_empty() {
        format!("{}/{}", bucket_id, id)
    } else {
        body.storage_path.clone()
    };

    // Write to primary table
    data.session
        .query_unpaged(
            "INSERT INTO storage.files \
             (bucket_id, id, folder_id, name, size, content_type, \
              storage_path, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                bucket_id,
                id,
                folder_id,
                name.clone(),
                size,
                content_type.clone(),
                storage_path.clone(),
                now,
                now,
            ),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    // Write to denormalized by_folder table
    data.session
        .query_unpaged(
            "INSERT INTO storage.files_by_folder \
             (bucket_id, folder_id, id, name, size, content_type, \
              storage_path, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                bucket_id,
                folder_id,
                id,
                name.clone(),
                size,
                content_type.clone(),
                storage_path.clone(),
                now,
                now,
            ),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    Ok(web::Json(FileResponse {
        id: id.to_string(),
        bucket_id: bucket_id.to_string(),
        folder_id: folder_id.to_string(),
        name,
        size,
        content_type,
        storage_path,
        created_at: ts_to_iso(now),
        updated_at: ts_to_iso(now),
    }))
}

// DELETE /storage/buckets/{bucket_id}/files/{file_id}
#[delete("/storage/buckets/{bucket_id}/files/{file_id}")]
async fn delete_file(
    data: web::Data<AppState>,
    path: web::Path<BucketFilePath>,
) -> Result<impl Responder, ApiError> {
    let bucket_id = parse_uuid(&path.bucket_id, "bucket_id")?;
    let file_id = parse_uuid(&path.file_id, "file_id")?;

    // Look up folder_id so we can delete from the by_folder table
    let result = data
        .session
        .query_unpaged(
            "SELECT folder_id FROM storage.files WHERE bucket_id = ? AND id = ?",
            (bucket_id, file_id),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let rows = result
        .into_rows_result()
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let folder_id = rows
        .rows::<(Uuid,)>()
        .map_err(|e| ApiError::Db(e.to_string()))?
        .filter_map(|r| r.ok())
        .next()
        .map(|(fid,)| fid)
        .ok_or_else(|| ApiError::NotFound(format!("file {file_id} not found")))?;

    // Delete from primary table
    data.session
        .query_unpaged(
            "DELETE FROM storage.files WHERE bucket_id = ? AND id = ?",
            (bucket_id, file_id),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    // Delete from denormalized by_folder table
    data.session
        .query_unpaged(
            "DELETE FROM storage.files_by_folder \
             WHERE bucket_id = ? AND folder_id = ? AND id = ?",
            (bucket_id, folder_id, file_id),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    Ok(web::Json(MessageResponse {
        message: format!("File {file_id} deleted"),
    }))
}

// ── Migration runner ───────────────────────────────────────────────────────────

async fn run_migrations(session: &Session) -> Result<(), Box<dyn std::error::Error>> {
    for (i, stmt) in MIGRATIONS.iter().enumerate() {
        info!("Running migration {}/{}", i + 1, MIGRATIONS.len());
        session.query_unpaged(*stmt, ()).await?;
    }
    info!("All {} migrations completed", MIGRATIONS.len());
    Ok(())
}

// ── Main ───────────────────────────────────────────────────────────────────────

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let host = env::var("SCYLLA_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("SCYLLA_PORT").unwrap_or_else(|_| "9042".to_string());
    let username = env::var("SCYLLA_USERNAME").unwrap_or_else(|_| "cassandra".to_string());
    let password = env::var("SCYLLA_PASSWORD").unwrap_or_else(|_| "cassandra".to_string());
    let api_host = env::var("STORAGE_API_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let api_port = env::var("STORAGE_API_PORT").unwrap_or_else(|_| "8081".to_string());
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

    info!("Running CQL migrations…");
    run_migrations(&session)
        .await
        .map_err(|e| std::io::Error::other(format!("Migration failed: {e}")))?;

    let state = AppState {
        session: Arc::new(session),
    };

    let bind_addr = format!("{}:{}", api_host, api_port);
    info!("Starting storage API at http://{}", bind_addr);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(&allowed_origin)
            .allowed_methods(["GET", "POST", "DELETE"])
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
            .service(list_buckets)
            .service(create_bucket)
            .service(delete_bucket)
            .service(list_folders)
            .service(create_folder)
            .service(delete_folder)
            .service(list_files)
            .service(create_file)
            .service(delete_file)
    })
    .bind(&bind_addr)
    .map_err(|e| std::io::Error::other(format!("Failed to bind to {bind_addr}: {e}")))?
    .run()
    .await
}
