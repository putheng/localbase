use std::env;
use std::sync::Arc;

use actix_cors::Cors;
use actix_web::http::header;
use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer};
use dotenvy::dotenv;
use log::info;
use scylla::SessionBuilder;

mod error;
mod handlers;
mod helpers;
mod migrations;
mod models;
mod state;

use error::ApiErrorBody;
use migrations::run_migrations;
use state::AppState;

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

    info!("Running CQL migrations...");
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
            .service(handlers::health)
            .service(handlers::buckets::list_buckets)
            .service(handlers::buckets::create_bucket)
            .service(handlers::buckets::delete_bucket)
            .service(handlers::folders::list_folders)
            .service(handlers::folders::create_folder)
            .service(handlers::folders::delete_folder)
            .service(handlers::files::list_files)
            .service(handlers::files::create_file)
            .service(handlers::files::delete_file)
    })
    .bind(&bind_addr)
    .map_err(|e| std::io::Error::other(format!("Failed to bind to {bind_addr}: {e}")))?
    .run()
    .await
}
