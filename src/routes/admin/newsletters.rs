use anyhow::Context;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect},
    Extension, Form,
};
use axum_flash::{Flash, IncomingFlashes};
use serde::Deserialize;
use sqlx::PgPool;

use crate::authentication::UserId;
use crate::domain::{Email, SubscriptionStatus};
use crate::template;
use crate::utils::get_success_and_error_flash_message;
use crate::{startup::AppState, utils::InternalServerError};

pub async fn publish_newsletter_form(flashes: IncomingFlashes) -> impl IntoResponse {
    let (success_msg, error_msg) = get_success_and_error_flash_message(&flashes);
    (
        flashes,
        Html(template::admin_newsletter_html(success_msg, error_msg)),
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
}

pub async fn publish_newsletter_with_flash(
    state: State<AppState>,
    flash: Flash,
    user_id_ext: Extension<UserId>,
    form: Form<NewsletterFormData>,
) -> impl IntoResponse {
    match publish_newsletter(state, user_id_ext, form).await {
        Ok(()) => (
            flash.success("Newsletter successfully published"),
            Redirect::to("/admin/newsletters"),
        ),
        Err(e) => (flash.error(e.message()), Redirect::to("/admin/newsletters")),
    }
}

#[tracing::instrument(name = "Publishing newsletter", skip(db_pool, email_client, data))]
pub async fn publish_newsletter(
    State(AppState {
        db_pool,
        email_client,
        ..
    }): State<AppState>,
    Extension(user_id): Extension<UserId>,
    Form(data): Form<NewsletterFormData>,
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
                        &data.title,
                        &data.html_content,
                        &data.text_content,
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
