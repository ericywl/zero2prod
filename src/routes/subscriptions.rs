use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Form};
use chrono::Utc;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::Deserialize;
use sqlx::PgPool;
use thiserror::Error;
use uuid::{NoContext, Timestamp, Uuid};

use crate::domain::{
    Email, Name, NewSubscriber, ParseEmailError, ParseNameError, SubscriptionStatus, Url,
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

    // Insert subscriber into DB
    let subscriber_id = match insert_subscriber(&state.db_pool, &new_subscriber).await {
        Ok(id) => id,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    // Generate and insert subscription token into DB
    let subscription_token = generate_subscription_token();
    if store_token(&state.db_pool, subscriber_id, &subscription_token)
        .await
        .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    // Send confirmation email with subscription token
    if send_confirmation_email(
        &state.email_client,
        &new_subscriber,
        &state.app_base_url,
        &subscription_token,
    )
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
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v7(Timestamp::now(NoContext));

    sqlx::query!(
        r#"
            INSERT INTO subscriptions (id, name, email, subscribed_at, status)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        subscriber_id,
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

    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Sending confirmation email to new subscriber",
    skip(email_client, new_subscriber, app_base_url, subscription_token)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: &NewSubscriber,
    app_base_url: &Url,
    subscription_token: &str,
) -> Result<(), SendEmailError> {
    let mut confirmation_link = app_base_url.join("subscribe/confirm").unwrap(); // safely unwrap since it's proper url
    confirmation_link.set_query(Some(&format!("subscription_token={}", subscription_token)));

    let html_body = format!(
        "Welcome to our newsletter!<br />\
        Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link.to_string(),
    );
    let plain_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link.to_string()
    );

    email_client
        .send_email(&new_subscriber.email, "Welcome!", &html_body, &plain_body)
        .await
        .map_err(|e| {
            tracing::error!("Failed to send email: {:?}", e);
            e
        })
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(pool, subscription_token)
)]
pub async fn store_token(
    pool: &PgPool,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)"#,
        subscription_token,
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

/// Generate a random 25-characters-long case-sensitive subscription token.
fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
