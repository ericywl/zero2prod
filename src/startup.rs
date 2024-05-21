use std::sync::Arc;

use super::routes;
use axum::{routing, Router};
use tokio::net::TcpListener;

pub struct AppState {
    pub db_pool: sqlx::PgPool,
}

pub fn app(state: Arc<AppState>) -> Router {
    // Build our application
    Router::new()
        .route("/health", routing::get(routes::health_check))
        .route("/subscribe", routing::post(routes::subscribe))
        .with_state(state)
}

pub async fn run(listener: TcpListener, state: Arc<AppState>) -> Result<(), std::io::Error> {
    let app = app(state);
    axum::serve(listener, app).await
}
