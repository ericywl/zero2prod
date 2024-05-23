use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Form};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use thiserror::Error;
use uuid::{NoContext, Timestamp, Uuid};

use crate::domain::{
    Email, Name, NewSubscriber, ParseEmailError, ParseNameError, SubscriptionStatus,
};
use crate::email_client::{EmailClient, SendEmailError};
use crate::startup::AppState;

#[derive(Debug, Error)]
pub enum FormDataError {
    #[error(transparent)]
    ParseName(#[from] ParseNameError),

    #[error(transparent)]
    ParseEmail(#[from] ParseEmailError),
}

#[derive(Debug, Deserialize)]
pub struct FormData {
    pub name: String,
    pub email: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = FormDataError;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = Name::parse(value.name)?;
        let email = Email::parse(value.email)?;
        Ok(Self { name, email })
    }
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(state, data),
    fields(
        subscriber_email = %data.email,
        subscriber_name = %data.name
    )
)]
pub async fn subscribe(
    State(state): State<Arc<AppState>>,
    Form(data): Form<FormData>,
) -> impl IntoResponse {
    let new_subscriber: NewSubscriber = match data.try_into() {
        Ok(sub) => sub,
        Err(e) => {
            tracing::error!("Failed to parse new subscriber: {:?}", e);
            return StatusCode::UNPROCESSABLE_ENTITY;
        }
    };

    // Insert subscriber to DB
    if insert_subscriber(&state.db_pool, &new_subscriber)
        .await
        .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    // Send email
    if send_confirmation_email(&state.email_client, &new_subscriber)
        .await
        .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(pool, new_subscriber)
)]
pub async fn insert_subscriber(
    pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
            INSERT INTO subscriptions (id, name, email, subscribed_at, status)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        Uuid::new_v7(Timestamp::now(NoContext)),
        new_subscriber.name.as_ref(),
        new_subscriber.email.as_ref(),
        Utc::now(),
        SubscriptionStatus::PendingConfirmation.to_string(),
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}

#[tracing::instrument(
    name = "Sending confirmation email to new subscriber",
    skip(email_client, new_subscriber)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: &NewSubscriber,
) -> Result<(), SendEmailError> {
    let confirmation_link = "https://there-is-no-such-domain.com/subscribe/confirm";
    let html_body = format!(
        "Welcome to our newsletter!<br />\
        Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link,
    );
    let plain_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );

    email_client
        .send_email(&new_subscriber.email, "Welcome!", &html_body, &plain_body)
        .await
        .map_err(|e| {
            tracing::error!("Failed to send email: {:?}", e);
            e
        })
}
