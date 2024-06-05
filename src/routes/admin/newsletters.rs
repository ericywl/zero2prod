use anyhow::Context;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
    Extension, Form,
};
use axum_flash::{Flash, IncomingFlashes};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{authentication::UserId, idempotency::IdempotencyKey};
use crate::{
    domain::{Email, SubscriptionStatus},
    idempotency::get_saved_response,
};
use crate::{idempotency::save_response, utils::get_success_and_error_flash_message};
use crate::{startup::AppState, utils::InternalServerError};
use crate::{template, utils::e500};

pub async fn publish_newsletter_form(flashes: IncomingFlashes) -> impl IntoResponse {
    let (success_msg, error_msg) = get_success_and_error_flash_message(&flashes);
    (
        flashes,
        Html(template::admin_newsletter_html(
            success_msg,
            error_msg,
            Uuid::new_v4().to_string(),
        )),
    )
}

struct ConfirmedSubscriber {
    email: Email,
}

#[derive(Debug, Deserialize)]
pub struct NewsletterFormData {
    title: String,
    html_content: String,
    text_content: String,
    idempotency_key: String,
}

pub async fn publish_newsletter_with_flash(
    State(state): State<AppState>,
    flash: Flash,
    Extension(user_id): Extension<UserId>,
    Form(data): Form<NewsletterFormData>,
) -> Response {
    match publish_newsletter_with_idempotent_handling(state, user_id, data).await {
        Ok(r) => (flash.success("Newsletter successfully published"), r).into_response(),
        Err(e) => (flash.error(e.message()), Redirect::to("/admin/newsletters")).into_response(),
    }
}

async fn publish_newsletter_with_idempotent_handling(
    state: AppState,
    user_id: UserId,
    data: NewsletterFormData,
) -> Result<Response, InternalServerError> {
    let idempotency_key: IdempotencyKey =
        data.idempotency_key.to_string().try_into().map_err(e500)?;

    // Return early if we have a saved response in the database
    if let Some(saved_response) = get_saved_response(&state.db_pool, &idempotency_key, *user_id)
        .await
        .map_err(e500)?
    {
        return Ok(saved_response);
    }

    // Publish newsletter
    publish_newsletter(state.clone(), user_id, data).await?;

    // Save response
    let response = Redirect::to("/admin/newsletters").into_response();
    let response = save_response(&state.db_pool, &idempotency_key, *user_id, response)
        .await
        .map_err(e500)?;
    Ok(response)
}

#[tracing::instrument(name = "Publishing newsletter", skip(db_pool, email_client, data))]
async fn publish_newsletter(
    AppState {
        db_pool,
        email_client,
        ..
    }: AppState,
    user_id: UserId,
    data: NewsletterFormData,
) -> Result<(), InternalServerError> {
    let subscribers = get_confirmed_subscribers(&db_pool)
        .await
        .context("Failed to get confirmed subscribers from the database")
        .map_err(e500)?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &data.title,
                        &data.html_content,
                        &data.text_content,
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })
                    .map_err(e500)?;
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
