use std::sync::Arc;

use super::routes;
use axum::{http::Request, routing, Router};
use tokio::net::TcpListener;
use tower_http::{
    trace::{DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::Level;

pub struct AppState {
    pub db_pool: sqlx::PgPool,
}

pub fn app(state: Arc<AppState>) -> Router {
    // Build our application
    Router::new()
        .route("/health", routing::get(routes::health_check))
        .route("/subscribe", routing::post(routes::subscribe))
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    let trace_id = uuid::Uuid::new_v4().to_string();
                    tracing::info_span!(
                        "request",
                        trace_id = trace_id,
                        method = ?request.method(),
                        uri = %request.uri(),
                        version = ?request.version(),
                    )
                })
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(LatencyUnit::Millis),
                ),
        )
}

pub async fn run(listener: TcpListener, state: Arc<AppState>) -> Result<(), std::io::Error> {
    let app = app(state);
    axum::serve(listener, app).await
}
