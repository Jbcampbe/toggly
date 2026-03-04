mod eval;
mod management;
mod middleware;
mod state;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let db_path = std::env::var("TOGGLY_DB_PATH").unwrap_or_else(|_| "toggly.db".into());
    let admin_api_key =
        std::env::var("TOGGLY_ADMIN_API_KEY").expect("TOGGLY_ADMIN_API_KEY must be set");
    let host = std::env::var("TOGGLY_HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port: u16 = std::env::var("TOGGLY_PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse()
        .expect("TOGGLY_PORT must be a valid port number");

    let db = toggly_core::db::Database::new(&db_path).expect("Failed to initialize database");
    let state = Arc::new(AppState::new(db, admin_api_key));

    let app = Router::new()
        .nest("/api/v1", management::router(state.clone()))
        .nest("/eval/v1", eval::router(state.clone()))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = format!("{host}:{port}").parse().unwrap();
    tracing::info!("Toggly server listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
