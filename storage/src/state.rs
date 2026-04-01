use std::sync::Arc;

use aws_sdk_s3::Client as S3Client;
use scylla::Session;

#[derive(Clone)]
pub struct AppState {
    pub session: Arc<Session>,
    pub s3: S3Client,
}
