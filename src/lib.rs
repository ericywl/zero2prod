use axum::{http::StatusCode, routing, Router};

async fn health_check() -> StatusCode {
    StatusCode::OK
}

pub fn app() -> Router {
    // Build our application
    Router::new().route("/health", routing::get(health_check))
}

pub async fn run() -> Result<(), std::io::Error> {
    let app = app();
    // Run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await
}
