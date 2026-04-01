use std::env;
use std::sync::Arc;

use actix_cors::Cors;
use actix_web::http::header;
use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer};
use dotenvy::dotenv;
use log::info;
use scylla::SessionBuilder;
use scylla::transport::session::{GenericSession, CurrentDeserializationApi};

mod errors;
mod handlers;
mod helpers;
mod models;
mod state;

use errors::ApiErrorBody;
use state::AppState;

// ── Main ───────────────────────────────────────────────────────────────────────

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let host: String = env::var("SCYLLA_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: String = env::var("SCYLLA_PORT").unwrap_or_else(|_| "9042".to_string());
    let username: String = env::var("SCYLLA_USERNAME").unwrap_or_else(|_| "scylla_app".to_string());
    let password: String =
        env::var("SCYLLA_PASSWORD").unwrap_or_else(|_| "change_me_local".to_string());
    let api_host: String = env::var("META_API_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let api_port: String = env::var("META_API_PORT").unwrap_or_else(|_| "8080".to_string());
    let allowed_origin: String =
        env::var("CORS_ORIGIN").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let scylla_uri = format!("{}:{}", host, port);
    info!("Connecting to ScyllaDB at {}", scylla_uri);

    let session: GenericSession<CurrentDeserializationApi> = SessionBuilder::new()
        .known_node(&scylla_uri)
        .user(username, password)
        .build()
        .await
        .map_err(|e: scylla::transport::errors::NewSessionError| std::io::Error::other(format!("Failed to connect to ScyllaDB: {e}")))?;

    let state: AppState = AppState {
        session: Arc::new(session),
    };

    let bind_addr: String = format!("{}:{}", api_host, api_port);
    info!("Starting scylladb-meta API at http://{}", bind_addr);

    HttpServer::new(move || {
        let cors: Cors = Cors::default()
            .allowed_origin(&allowed_origin)
            .allowed_methods(["GET", "POST", "PATCH", "DELETE"])
            .allowed_headers([header::CONTENT_TYPE, header::ACCEPT])
            .supports_credentials()
            .max_age(3600);

        App::new()
            .app_data(web::Data::new(state.clone()))
            .app_data(web::JsonConfig::default().error_handler(|err, _| {
                let body: ApiErrorBody = ApiErrorBody {
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
            .service(handlers::list_keyspaces)
            .service(handlers::create_keyspace)
            .service(handlers::drop_keyspace)
            .service(handlers::list_tables)
            .service(handlers::create_table)
            .service(handlers::drop_table)
            .service(handlers::list_columns)
            .service(handlers::get_rows)
            .service(handlers::insert_row)
            .service(handlers::update_row)
            .service(handlers::delete_row)
            .service(handlers::execute_query)
    })
    .bind(bind_addr)?
    .run()
    .await
}

