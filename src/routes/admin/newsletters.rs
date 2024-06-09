use anyhow::Context;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
    Extension, Form,
};
use axum_flash::{Flash, IncomingFlashes};
use serde::Deserialize;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    authentication::UserId,
    domain::SubscriptionStatus,
    idempotency::{save_response, try_processing, IdempotencyKey, NextAction},
    startup::AppState,
    template,
    utils::{e500, get_success_and_error_flash_message, InternalServerError},
};

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
        Err(e) => {
            tracing::error!("{:?}", e);
            (
                flash.error(e.to_string()),
                Redirect::to("/admin/newsletters"),
            )
                .into_response()
        }
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
    let mut transaction = match try_processing(&state.db_pool, &idempotency_key, *user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(saved_response) => {
            return Ok(saved_response);
        }
    };

    // Publish newsletter
    publish_newsletter(&mut transaction, user_id, data).await?;

    // Save response
    let response = Redirect::to("/admin/newsletters").into_response();
    let response = save_response(transaction, &idempotency_key, *user_id, response)
        .await
        .map_err(e500)?;
    Ok(response)
}

#[tracing::instrument(name = "Publishing newsletter", skip(transaction, data))]
async fn publish_newsletter(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: UserId,
    data: NewsletterFormData,
) -> Result<(), InternalServerError> {
    let issue_id = insert_newsletter_issue(
        transaction,
        &data.title,
        &data.text_content,
        &data.html_content,
    )
    .await
    .context("Failed to store newsletter issue details")
    .map_err(e500)?;

    enqueue_delivery_tasks(transaction, issue_id)
        .await
        .context("Failed to enqueue delivery tasks")
        .map_err(e500)?;

    Ok(())
}

#[tracing::instrument(name = "Insert newsletter issue", skip_all)]
async fn insert_newsletter_issue(
    transaction: &mut Transaction<'_, Postgres>,
    title: &str,
    text_content: &str,
    html_content: &str,
) -> Result<Uuid, sqlx::Error> {
    let newsletter_issue_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO newsletter_issues (
            newsletter_issue_id,
            title,
            text_content,
            html_content,
            published_at
        )
        VALUES ($1, $2, $3, $4, now())
        "#,
        newsletter_issue_id,
        title,
        text_content,
        html_content
    )
    .execute(&mut **transaction)
    .await?;

    Ok(newsletter_issue_id)
}

#[tracing::instrument(name = "Enqueue delivery tasks", skip_all)]
async fn enqueue_delivery_tasks(
    transaction: &mut Transaction<'_, Postgres>,
    newsletter_issue_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO issue_delivery_queue (
            newsletter_issue_id,
            subscriber_email
        )
        SELECT $1, email
        FROM subscriptions
        WHERE status = $2
        "#,
        newsletter_issue_id,
        SubscriptionStatus::Confirmed.to_string()
    )
    .execute(&mut **transaction)
    .await?;

    Ok(())
}
