use actix_web::HttpResponse;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
    pub error: String,
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("database error: {0}")]
    Db(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("storage error: {0}")]
    Storage(String),
}

impl actix_web::ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        let status = match self {
            ApiError::BadRequest(_) => actix_web::http::StatusCode::BAD_REQUEST,
            ApiError::NotFound(_) => actix_web::http::StatusCode::NOT_FOUND,
            ApiError::Db(_) | ApiError::Storage(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        };
        HttpResponse::build(status).json(ApiErrorBody {
            error: self.to_string(),
        })
    }
}
