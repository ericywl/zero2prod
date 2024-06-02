use anyhow::Context;
use axum::{extract::State, Extension, Json};
use serde::Deserialize;
use sqlx::PgPool;

use crate::authentication::UserId;
use crate::domain::{Email, SubscriptionStatus};
use crate::{startup::AppState, utils::InternalServerError};

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

#[tracing::instrument(name = "Publishing newsletter", skip(db_pool, email_client, body))]
pub async fn publish_newsletter(
    State(AppState {
        db_pool,
        email_client,
        ..
    }): State<AppState>,
    Extension(user_id): Extension<UserId>,
    Json(body): Json<BodyData>,
) -> Result<(), InternalServerError> {
    let subscribers = get_confirmed_subscribers(&db_pool)
        .await
        .context("Failed to get confirmed subscribers from the database")
        .map_err(InternalServerError)?;

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
                    .map_err(InternalServerError)?;
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
