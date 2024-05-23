use axum::{extract::Query, http::StatusCode};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(params))]
pub async fn confirm(Query(params): Query<Parameters>) -> StatusCode {
    StatusCode::OK
}
