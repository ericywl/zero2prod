use std::sync::Arc;

use axum::extract::{Query, State};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{SubscriptionStatus, SubscriptionToken},
    startup::AppState,
};

use super::handler_response::HandlerResponse;

#[derive(Debug, Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(state, params))]
pub async fn confirm(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Parameters>,
) -> HandlerResponse {
    let subscription_token = match SubscriptionToken::parse(&params.subscription_token) {
        Ok(token) => token,
        Err(e) => {
            tracing::error!("Failed to parse subscription token: {:?}", e);
            return HandlerResponse::authorization_error("Invalid subscription token");
        }
    };

    // Get subscriber ID from token
    let subscriber_id =
        match get_subscriber_id_from_token(&state.db_pool, &subscription_token).await {
            Ok(id) => id,
            Err(_) => return HandlerResponse::server_error(),
        };

    // Token not found, return error
    if subscriber_id.is_none() {
        return HandlerResponse::authorization_error("Invalid subscription token");
    }

    // Confirm subscriber using retrieved ID
    match confirm_subscriber(&state.db_pool, subscriber_id.unwrap()).await {
        Ok(_) => HandlerResponse::ok(),
        Err(_) => HandlerResponse::server_error(),
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
    subscription_token: &SubscriptionToken,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT subscriber_id FROM subscription_tokens \
        WHERE subscription_token = $1",
        subscription_token.as_str(),
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(result.map(|r| r.subscriber_id))
}
