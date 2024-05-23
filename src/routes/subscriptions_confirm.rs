use axum::{extract::Query, http::StatusCode};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Parameters {
    confirmation_token: String,
}

pub async fn confirm(_params: Query<Parameters>) -> StatusCode {
    StatusCode::OK
}
