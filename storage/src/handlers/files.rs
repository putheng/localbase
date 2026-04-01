use actix_web::{delete, get, post, put, web, HttpRequest, Responder};
use aws_sdk_s3::primitives::ByteStream;
use bytes::Bytes;
use futures_util::StreamExt;
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

/// PUT /storage/buckets/{bucket_id}/files/{file_id}/upload
/// Streams the raw request body to MinIO at the file's storage_path,
/// then updates the file size in ScyllaDB.
#[put("/storage/buckets/{bucket_id}/files/{file_id}/upload")]
pub async fn upload_file(
    data: web::Data<AppState>,
    path: web::Path<BucketFilePath>,
    req: HttpRequest,
    mut payload: web::Payload,
) -> Result<impl Responder, ApiError> {
    let bucket_id = parse_uuid(&path.bucket_id, "bucket_id")?;
    let file_id = parse_uuid(&path.file_id, "file_id")?;

    // Fetch bucket name and file metadata from ScyllaDB
    let bucket_row = data
        .session
        .query_unpaged(
            "SELECT name FROM storage.buckets WHERE id = ?",
            (bucket_id,),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let bucket_rows = bucket_row
        .into_rows_result()
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let bucket_name = bucket_rows
        .rows::<(String,)>()
        .map_err(|e| ApiError::Db(e.to_string()))?
        .filter_map(|r| r.ok())
        .map(|(n,)| n)
        .next()
        .ok_or_else(|| ApiError::NotFound(format!("bucket {bucket_id} not found")))?;

    let file_row = data
        .session
        .query_unpaged(
            "SELECT storage_path, content_type FROM storage.files WHERE bucket_id = ? AND id = ?",
            (bucket_id, file_id),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let file_rows = file_row
        .into_rows_result()
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let (storage_path, content_type) = file_rows
        .rows::<(Option<String>, Option<String>)>()
        .map_err(|e| ApiError::Db(e.to_string()))?
        .filter_map(|r| r.ok())
        .next()
        .map(|(sp, ct)| (sp.unwrap_or_default(), ct.unwrap_or_default()))
        .ok_or_else(|| ApiError::NotFound(format!("file {file_id} not found")))?;

    // Read entire body into memory
    let mut body_bytes = Vec::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk.map_err(|e| ApiError::BadRequest(e.to_string()))?;
        body_bytes.extend_from_slice(&chunk);
    }
    let size = body_bytes.len() as i64;

    // Determine content type: prefer request header over stored value
    let ct = req
        .headers()
        .get(actix_web::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_owned())
        .unwrap_or(content_type);

    // Upload to MinIO
    data.s3
        .put_object()
        .bucket(&bucket_name)
        .key(&storage_path)
        .content_type(&ct)
        .body(ByteStream::from(Bytes::from(body_bytes)))
        .send()
        .await
        .map_err(|e| ApiError::Storage(e.to_string()))?;

    // Update file size + content_type in ScyllaDB
    let now = now_ts();
    data.session
        .query_unpaged(
            "UPDATE storage.files SET size = ?, content_type = ?, updated_at = ? \
             WHERE bucket_id = ? AND id = ?",
            (size, ct.clone(), now, bucket_id, file_id),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    Ok(web::Json(MessageResponse {
        message: format!("File {file_id} uploaded ({size} bytes)"),
    }))
}

#[delete("/storage/buckets/{bucket_id}/files/{file_id}")]
pub async fn delete_file(
    data: web::Data<AppState>,
    path: web::Path<BucketFilePath>,
) -> Result<impl Responder, ApiError> {
    let bucket_id = parse_uuid(&path.bucket_id, "bucket_id")?;
    let file_id = parse_uuid(&path.file_id, "file_id")?;

    // Fetch folder_id + storage_path + bucket name for cleanup
    let result = data
        .session
        .query_unpaged(
            "SELECT folder_id, storage_path FROM storage.files WHERE bucket_id = ? AND id = ?",
            (bucket_id, file_id),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let rows = result
        .into_rows_result()
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let (folder_id, storage_path) = rows
        .rows::<(Uuid, Option<String>)>()
        .map_err(|e| ApiError::Db(e.to_string()))?
        .filter_map(|r| r.ok())
        .next()
        .map(|(fid, sp)| (fid, sp.unwrap_or_default()))
        .ok_or_else(|| ApiError::NotFound(format!("file {file_id} not found")))?;

    // Fetch bucket name for MinIO
    let bucket_row = data
        .session
        .query_unpaged(
            "SELECT name FROM storage.buckets WHERE id = ?",
            (bucket_id,),
        )
        .await
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let bucket_rows = bucket_row
        .into_rows_result()
        .map_err(|e| ApiError::Db(e.to_string()))?;

    let bucket_name = bucket_rows
        .rows::<(String,)>()
        .map_err(|e| ApiError::Db(e.to_string()))?
        .filter_map(|r| r.ok())
        .map(|(n,)| n)
        .next()
        .ok_or_else(|| ApiError::NotFound(format!("bucket {bucket_id} not found")))?;

    // Delete from MinIO (ignore not-found — object may not have been uploaded yet)
    if !storage_path.is_empty() {
        let _ = data
            .s3
            .delete_object()
            .bucket(&bucket_name)
            .key(&storage_path)
            .send()
            .await;
    }

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

