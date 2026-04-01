use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct BucketResponse {
    pub id: String,
    pub name: String,
    pub public: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct FolderResponse {
    pub id: String,
    pub bucket_id: String,
    pub parent_id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct FileResponse {
    pub id: String,
    pub bucket_id: String,
    pub folder_id: String,
    pub name: String,
    pub size: i64,
    pub content_type: String,
    pub storage_path: String,
    pub created_at: String,
    pub updated_at: String,
}

// ── Request bodies ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateBucketBody {
    pub name: String,
    #[serde(default)]
    pub public: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateFolderBody {
    pub name: String,
    pub parent_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateFileBody {
    pub name: String,
    #[serde(default)]
    pub size: i64,
    #[serde(default)]
    pub content_type: String,
    #[serde(default)]
    pub storage_path: String,
    pub folder_id: Option<String>,
}

// ── Query params ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct FolderQuery {
    pub parent_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FileQuery {
    pub folder_id: Option<String>,
}

// ── Path extractors ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct BucketPath {
    pub bucket_id: String,
}

#[derive(Debug, Deserialize)]
pub struct BucketFolderPath {
    pub bucket_id: String,
    pub folder_id: String,
}

#[derive(Debug, Deserialize)]
pub struct BucketFilePath {
    pub bucket_id: String,
    pub file_id: String,
}
