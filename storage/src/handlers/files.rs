use actix_web::{delete, get, post, web, Responder};
use uuid::Uuid;

use crate::{
    error::ApiError,
    helpers::{now_ts, parse_uuid, ts_to_iso, CqlTimestamp},
    models::{BucketFilePath, BucketPath, CreateFileBody, FileQuery, FileResponse, MessageResponse},
    state::AppState,
};

#[get("/storage/buckets/{bucket_id}/files")]
pub async fn list_files(
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

#[post("/storage/buckets/{bucket_id}/files")]
pub async fn create_file(
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

#[delete("/storage/buckets/{bucket_id}/files/{file_id}")]
pub async fn delete_file(
    data: web::Data<AppState>,
    path: web::Path<BucketFilePath>,
) -> Result<impl Responder, ApiError> {
    let bucket_id = parse_uuid(&path.bucket_id, "bucket_id")?;
    let file_id = parse_uuid(&path.file_id, "file_id")?;

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

    data.session
        .query_unpaged(
            "DELETE FROM storage.files WHERE bucket_id = ? AND id = ?",
            (bucket_id, file_id),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

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
