use axum::{http::StatusCode, routing, Router};
use tokio::net::TcpListener;

async fn health_check() -> StatusCode {
    StatusCode::OK
}

async fn subscribe() -> StatusCode {
    StatusCode::OK
}

pub fn app() -> Router {
    // Build our application
    Router::new()
        .route("/health", routing::get(health_check))
        .route("/subscribe", routing::post(subscribe))
}

pub async fn run(listener: TcpListener) -> Result<(), std::io::Error> {
    let app = app();

    // Run our app with hyper, listening globally on port 3000
    axum::serve(listener, app).await
}
