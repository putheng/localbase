use std::env;
use std::sync::Arc;

use actix_cors::Cors;
use actix_web::http::header;
use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer};
use dotenvy::dotenv;
use log::info;
use scylla::SessionBuilder;
use scylla::transport::session::{CurrentDeserializationApi, GenericSession};

mod errors;
mod handlers;
mod helpers;
mod models;
mod state;

use errors::ApiErrorBody;
use state::AppState;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let host = env::var("SCYLLA_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("SCYLLA_PORT").unwrap_or_else(|_| "9042".to_string());
    let username = env::var("SCYLLA_USERNAME").unwrap_or_else(|_| "scylla_app".to_string());
    let password = env::var("SCYLLA_PASSWORD").unwrap_or_else(|_| "change_me_local".to_string());
    let keyspace = env::var("SCYLLA_KEYSPACE").unwrap_or_else(|_| "public".to_string());
    let api_host = env::var("API_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let api_port = env::var("API_PORT").unwrap_or_else(|_| "8082".to_string());
    let allowed_origin =
        env::var("CORS_ORIGIN").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let scylla_uri = format!("{host}:{port}");
    info!("Connecting to ScyllaDB at {scylla_uri}");

    let session: GenericSession<CurrentDeserializationApi> = SessionBuilder::new()
        .known_node(&scylla_uri)
        .user(username, password)
        .build()
        .await
        .map_err(|e| {
            std::io::Error::other(format!("Failed to connect to ScyllaDB: {e}"))
        })?;

    let state = AppState {
        session: Arc::new(session),
        keyspace,
    };

    let bind_addr = format!("{api_host}:{api_port}");
    info!("Starting scylladb-api at http://{bind_addr}");

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(&allowed_origin)
            .allowed_methods(["GET", "POST", "PATCH", "DELETE"])
            .allowed_headers([
                header::CONTENT_TYPE,
                header::ACCEPT,
                header::HeaderName::from_static("prefer"),
            ])
            .expose_headers([
                header::HeaderName::from_static("x-next-page-token"),
                header::HeaderName::from_static("x-warning"),
                header::HeaderName::from_static("content-range"),
            ])
            .supports_credentials()
            .max_age(3600);

        App::new()
            .app_data(web::Data::new(state.clone()))
            .app_data(web::JsonConfig::default().error_handler(|err, _| {
                let body = ApiErrorBody { error: err.to_string() };
                actix_web::error::InternalError::from_response(
                    err,
                    HttpResponse::BadRequest().json(body),
                )
                .into()
            }))
            .wrap(Logger::default())
            .wrap(cors)
            .service(handlers::health)
            .service(handlers::select_rows)
            .service(handlers::insert_rows)
            .service(handlers::update_rows)
            .service(handlers::delete_rows)
    })
    .bind(bind_addr)?
    .run()
    .await
}
