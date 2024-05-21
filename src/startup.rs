use super::routes;
use axum::{routing, Router};
use tokio::net::TcpListener;

pub fn app() -> Router {
    // Build our application
    Router::new()
        .route("/health", routing::get(routes::health_check))
        .route("/subscribe", routing::post(routes::subscribe))
}

pub async fn run(listener: TcpListener) -> Result<(), std::io::Error> {
    let app = app();

    // Run our app with hyper, listening globally on port 3000
    axum::serve(listener, app).await
}
