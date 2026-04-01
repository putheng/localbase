use actix_web::{delete, get, post, web, Responder};
use uuid::Uuid;

use crate::{
    error::ApiError,
    helpers::{now_ts, parse_uuid, ts_to_iso, CqlTimestamp},
    models::{
        BucketFolderPath, BucketPath, CreateFolderBody, FolderQuery, FolderResponse,
        MessageResponse,
    },
    state::AppState,
};

#[get("/storage/buckets/{bucket_id}/folders")]
pub async fn list_folders(
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

#[post("/storage/buckets/{bucket_id}/folders")]
pub async fn create_folder(
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

    data.session
        .query_unpaged(
            "INSERT INTO storage.folders \
             (bucket_id, id, parent_id, name, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
            (bucket_id, id, parent_id, name.clone(), now, now),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

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

#[delete("/storage/buckets/{bucket_id}/folders/{folder_id}")]
pub async fn delete_folder(
    data: web::Data<AppState>,
    path: web::Path<BucketFolderPath>,
) -> Result<impl Responder, ApiError> {
    let bucket_id = parse_uuid(&path.bucket_id, "bucket_id")?;
    let folder_id = parse_uuid(&path.folder_id, "folder_id")?;

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

    data.session
        .query_unpaged(
            "DELETE FROM storage.folders WHERE bucket_id = ? AND id = ?",
            (bucket_id, folder_id),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

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
