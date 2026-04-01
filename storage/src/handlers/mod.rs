pub mod buckets;
pub mod files;
pub mod folders;

use actix_web::{get, web, Responder};

use crate::models::HealthResponse;

#[get("/health")]
pub async fn health() -> impl Responder {
    web::Json(HealthResponse {
        status: "ok",
        service: "storage",
    })
}
