use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{domain::SubscriptionStatus, startup::AppState};

#[derive(Debug, Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(state, params))]
pub async fn confirm(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Parameters>,
) -> StatusCode {
    let id = match get_subscriber_id_from_token(&state.db_pool, &params.subscription_token).await {
        Ok(id) => id,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    match id {
        Some(subscriber_id) => match confirm_subscriber(&state.db_pool, subscriber_id).await {
            Ok(_) => StatusCode::OK,
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
        },
        None => StatusCode::UNAUTHORIZED,
    }
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(pool, subscriber_id))]
pub async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = $1 WHERE id = $2"#,
        SubscriptionStatus::Confirmed.to_string(),
        subscriber_id,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(pool, subscription_token))]
pub async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT subscriber_id FROM subscription_tokens \
        WHERE subscription_token = $1",
        subscription_token,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(result.map(|r| r.subscriber_id))
}
