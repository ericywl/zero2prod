use axum::{http::StatusCode, response::IntoResponse};

pub async fn publish_newsletter() -> impl IntoResponse {
    StatusCode::OK
}
