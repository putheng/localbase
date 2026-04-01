use actix_web::{delete, get, post, web, Responder};
use uuid::Uuid;

use crate::{
    error::ApiError,
    helpers::{now_ts, parse_uuid, ts_to_iso, CqlTimestamp},
    models::{BucketPath, BucketResponse, CreateBucketBody, MessageResponse},
    state::AppState,
};

#[get("/storage/buckets")]
pub async fn list_buckets(data: web::Data<AppState>) -> Result<impl Responder, ApiError> {
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

#[post("/storage/buckets")]
pub async fn create_bucket(
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

#[delete("/storage/buckets/{bucket_id}")]
pub async fn delete_bucket(
    data: web::Data<AppState>,
    path: web::Path<BucketPath>,
) -> Result<impl Responder, ApiError> {
    let bucket_id = parse_uuid(&path.bucket_id, "bucket_id")?;

    // Cascade: files → folders → bucket
    data.session
        .query_unpaged(
            "DELETE FROM storage.files WHERE bucket_id = ?",
            (bucket_id,),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    data.session
        .query_unpaged(
            "DELETE FROM storage.folders WHERE bucket_id = ?",
            (bucket_id,),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

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
