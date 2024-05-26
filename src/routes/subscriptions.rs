use anyhow::Context;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Form};
use chrono::Utc;
use serde::Deserialize;
use sqlx::{Postgres, Transaction};
use uuid::{NoContext, Timestamp, Uuid};

use crate::domain::{
    Email, Name, ParseEmailError, ParseNameError, SubscriptionStatus, SubscriptionToken, Url,
};
use crate::email_client::{EmailClient, SendEmailError};
use crate::startup::AppState;
use crate::{telemetry, template};

#[derive(Debug, thiserror::Error)]
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

struct NewSubscriber {
    name: Name,
    email: Email,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = FormDataError;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = Name::parse(&value.name)?;
        let email = Email::parse(&value.email)?;
        Ok(Self { name, email })
    }
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    FormValidationError(#[from] FormDataError),

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        telemetry::error_chain_fmt(self, f)
    }
}

impl IntoResponse for SubscribeError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::FormValidationError(_) => {
                // User error, ignore logging
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "New subscriber form validation error".to_string(),
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

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(state, data),
    fields(
        subscriber_email = %data.email,
        subscriber_name = %data.name
    )
)]
pub async fn subscribe(
    State(state): State<AppState>,
    Form(data): Form<FormData>,
) -> Result<(), SubscribeError> {
    let new_subscriber: NewSubscriber = data.try_into()?;

    // Begin transaction
    let mut transaction = state
        .db_pool
        .begin()
        .await
        .context("Failed to acquirre Postgres connection from the pool")?;

    // Insert subscriber into DB
    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .context("Failed to insert new subscriber into the database")?;

    // Generate and insert subscription token into DB
    let subscription_token = SubscriptionToken::generate();
    store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .context("Failed to store the confirmation token for a new subscriber")?;

    // Commit transaction
    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new subscriber")?;

    // Send confirmation email with subscription token
    send_confirmation_email(
        &state.email_client,
        &new_subscriber,
        &state.app_base_url,
        &subscription_token,
    )
    .await
    .context("Failed to send a confirmation email")?;

    Ok(())
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(transaction, new_subscriber)
)]
async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
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
    .execute(&mut **transaction)
    .await?;

    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Sending confirmation email to new subscriber",
    skip(email_client, new_subscriber, app_base_url, subscription_token)
)]
async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: &NewSubscriber,
    app_base_url: &Url,
    subscription_token: &SubscriptionToken,
) -> Result<(), SendEmailError> {
    // The confirmation link should be `<BASE_URL>/subscribe/confirm?subscription_token=<TOKEN>`
    let mut confirmation_link = app_base_url.join("subscribe/confirm").unwrap(); // safely unwrap since it's proper url
    confirmation_link.set_query(Some(&format!(
        "subscription_token={}",
        subscription_token.as_str()
    )));

    let html_body = template::confirmation_email_with_link(&confirmation_link);
    let plain_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link,
    );

    email_client
        .send_email(&new_subscriber.email, "Welcome!", &html_body, &plain_body)
        .await
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(transaction, subscription_token)
)]
async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &SubscriptionToken,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)"#,
        subscription_token.as_str(),
        subscriber_id,
    )
    .execute(&mut **transaction)
    .await?;

    Ok(())
}
