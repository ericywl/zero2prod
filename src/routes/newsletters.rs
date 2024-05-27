use anyhow::Context;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use sqlx::PgPool;

use crate::startup::AppState;
use crate::telemetry;
use crate::{
    authentication,
    domain::{Email, SubscriptionStatus},
};

struct ConfirmedSubscriber {
    email: Email,
}

#[derive(Debug, Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(Debug, Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        telemetry::error_chain_fmt(self, f)
    }
}

impl IntoResponse for PublishError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::AuthError(_) => {
                // Auth error, ignore logging
                let mut response = (
                    StatusCode::UNAUTHORIZED,
                    "Authentication failed".to_string(),
                )
                    .into_response();

                authentication::add_basic_auth_header(response.headers_mut());
                response
            }
            Self::UnexpectedError(e) => {
                // Log unexpected error
                tracing::error!("{:?}", e);

                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong with publishing newsletter".to_string(),
                )
                    .into_response()
            }
        }
    }
}

#[tracing::instrument(
    name = "Publishing newsletter",
    skip(db_pool, email_client, headers, body)
)]
pub async fn publish_newsletter(
    State(AppState {
        db_pool,
        email_client,
        ..
    }): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<BodyData>,
) -> Result<(), PublishError> {
    let credentials =
        authentication::retrieve_basic_auth(&headers).map_err(PublishError::AuthError)?;
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    let user_id = authentication::validate_credentials(&db_pool, credentials)
        .await
        .map_err(|e| match e {
            authentication::AuthError::InvalidCredentials(_) => PublishError::AuthError(e.into()),
            authentication::AuthError::UnexpectedError(_) => {
                PublishError::UnexpectedError(e.into())
            }
        })?;
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    let subscribers = get_confirmed_subscribers(&db_pool)
        .await
        .context("Failed to get confirmed subscribers from the database")
        .map_err(PublishError::UnexpectedError)?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })
                    .map_err(PublishError::UnexpectedError)?;
            }
            Err(e) => {
                tracing::warn!(
                    // We record the error chain as a structured field
                    // on the log record.
                    error.cause_chain = ?e,
                    "Skipping a confirmed subscriber. The stored email is invalid."
                );
            }
        }
    }

    Ok(())
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, sqlx::Error> {
    // We are returning a `Vec` of `Result`s in the happy case.
    // This allows the caller to bubble up errors due to network issues or other
    // transient failures using the `?` operator, while the compiler
    // forces them to handle the subtler mapping error.
    // See http://sled.rs/errors.html for a deep-dive about this technique.

    struct Row {
        email: String,
    }

    let rows = sqlx::query_as!(
        Row,
        r#"SELECT email FROM subscriptions WHERE status = $1"#,
        SubscriptionStatus::Confirmed.to_string()
    )
    .fetch_all(pool)
    .await?;

    let confirmed_subscribers: Vec<_> = rows
        .into_iter()
        // Filter out invalid emails
        .map(|r| match Email::parse(&r.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(e) => Err(anyhow::anyhow!(e)),
        })
        .collect();

    Ok(confirmed_subscribers)
}
