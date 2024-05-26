use anyhow::Context;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{ParseSubscriptionTokenError, SubscriptionStatus, SubscriptionToken},
    startup::AppState,
    telemetry,
};

#[derive(Debug, Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[derive(thiserror::Error)]
pub enum ConfirmSubscriptionError {
    #[error("{0}")]
    TokenValidationError(#[from] ParseSubscriptionTokenError),

    #[error("Token not found")]
    TokenNotFound,

    #[error("Subscription already confirmed")]
    AlreadyConfirmed,

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for ConfirmSubscriptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        telemetry::error_chain_fmt(self, f)
    }
}

impl IntoResponse for ConfirmSubscriptionError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::TokenValidationError(_) | Self::TokenNotFound => {
                // User error, ignore logging
                (
                    StatusCode::UNAUTHORIZED,
                    "Subscription token validation error".to_string(),
                )
                    .into_response()
            }
            Self::AlreadyConfirmed => {
                // Probably user error, ignore logging
                (
                    StatusCode::CONFLICT,
                    "Subscription already confirmed".to_string(),
                )
                    .into_response()
            }
            Self::UnexpectedError(e) => {
                // Log unexpected error
                tracing::error!("{:?}", e);

                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong with subscription".to_string(),
                )
                    .into_response()
            }
        }
    }
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(state, params))]
pub async fn confirm(
    State(state): State<AppState>,
    Query(params): Query<Parameters>,
) -> Result<(), ConfirmSubscriptionError> {
    let subscription_token = SubscriptionToken::parse(&params.subscription_token)?;

    // Get subscriber ID from token
    let subscriber_id = get_subscriber_id_from_token(&state.db_pool, &subscription_token)
        .await
        .context("Failed to get subscriber_id associated with the provided token")?;

    // Token not found, return error
    if subscriber_id.is_none() {
        return Err(ConfirmSubscriptionError::TokenNotFound);
    }

    // Check if subscription already confirmed
    let status = get_subscriber_status(&state.db_pool, subscriber_id.unwrap())
        .await
        .context("Failed to get subscriber status")?;
    if status == SubscriptionStatus::Confirmed {
        return Err(ConfirmSubscriptionError::AlreadyConfirmed);
    }

    // Confirm subscriber using retrieved ID
    confirm_subscriber(&state.db_pool, subscriber_id.unwrap())
        .await
        .context("Failed to confirm subscriber in the database")?;

    Ok(())
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(pool, subscriber_id))]
async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = $1 WHERE id = $2"#,
        SubscriptionStatus::Confirmed.to_string(),
        subscriber_id,
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[tracing::instrument(name = "Get subscriber id using token", skip(pool, subscription_token))]
async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &SubscriptionToken,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT subscriber_id FROM subscription_tokens \
        WHERE subscription_token = $1",
        subscription_token.as_str(),
    )
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|r| r.subscriber_id))
}

#[tracing::instrument(name = "Get subscription status", skip(pool, subscriber_id))]
async fn get_subscriber_status(
    pool: &PgPool,
    subscriber_id: Uuid,
) -> Result<SubscriptionStatus, anyhow::Error> {
    let result = sqlx::query!(
        "SELECT status FROM subscriptions \
        WHERE id = $1",
        subscriber_id,
    )
    .fetch_one(pool)
    .await?;

    Ok(result.status.try_into()?)
}
