use anyhow::Context;
use axum::response::Redirect;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Form};
use axum_flash::Flash;
use chrono::Utc;
use serde::Deserialize;
use sqlx::{Postgres, Transaction};
use uuid::{NoContext, Timestamp, Uuid};

use crate::domain::{
    Email, Name, ParseEmailError, ParseNameError, SubscriptionStatus, SubscriptionToken, Url,
};
use crate::email_client::{EmailClient, SendEmailError};
use crate::startup::AppState;
use crate::utils::InternalServerError;
use crate::{telemetry, template};

#[derive(Debug, thiserror::Error)]
pub enum FormDataError {
    #[error(transparent)]
    ParseName(#[from] ParseNameError),

    #[error(transparent)]
    ParseEmail(#[from] ParseEmailError),
}

#[derive(Debug, Deserialize)]
pub struct SubscribeFormData {
    pub name: String,
    pub email: String,
}

struct NewSubscriber {
    name: Name,
    email: Email,
}

impl TryFrom<SubscribeFormData> for NewSubscriber {
    type Error = FormDataError;

    fn try_from(value: SubscribeFormData) -> Result<Self, Self::Error> {
        let name = Name::parse(&value.name)?;
        let email = Email::parse(&value.email)?;
        Ok(Self { name, email })
    }
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    FormValidationError(#[from] FormDataError),

    #[error("subscription already confirmed")]
    AlreadyConfirmed,

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
            Self::AlreadyConfirmed => {
                // Probably user error, ignore logging
                (
                    StatusCode::CONFLICT,
                    "Subscription already confirmed".to_string(),
                )
                    .into_response()
            }
            Self::UnexpectedError(e) => InternalServerError(e).into_response(),
        }
    }
}

pub async fn subscribe_with_flash(
    state: State<AppState>,
    flash: Flash,
    form: Form<SubscribeFormData>,
) -> impl IntoResponse {
    match subscribe(state, form).await {
        Ok(()) => (flash.success("Thanks for subscribing!"), Redirect::to("/")),
        Err(e) => (flash.error(e.to_string()), Redirect::to("/")),
    }
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(db_pool, email_client, app_base_url, data),
    fields(
        subscriber_email = %data.email,
        subscriber_name = %data.name
    )
)]
pub async fn subscribe(
    State(AppState {
        db_pool,
        email_client,
        app_base_url,
        ..
    }): State<AppState>,
    Form(data): Form<SubscribeFormData>,
) -> Result<(), SubscribeError> {
    let mut new_subscriber: NewSubscriber = data.try_into()?;
    let subscription_token: SubscriptionToken;

    // Begin transaction
    let mut transaction = db_pool
        .begin()
        .await
        .context("Failed to acquirre Postgres connection from the pool")?;

    // Try to get existing subscriber from DB
    let existing_subscriber = get_existing_subscriber(&mut transaction, &new_subscriber.email)
        .await
        .context("Failed to get existing subscriber from the database")?;

    match existing_subscriber {
        // Subscriber already exists
        Some(subscriber) => {
            // Subscription status already confirmed, return error
            if subscriber.status == SubscriptionStatus::Confirmed {
                return Err(SubscribeError::AlreadyConfirmed);
            }
            // Get existing subscription token
            subscription_token = get_existing_subscription_token(&mut transaction, subscriber.id)
                .await
                .context("Failed to get existing subscription token from the database")?;
            // Replace name with saved name
            // TODO: Update database with new name if it's different
            new_subscriber.name = subscriber.name;

            // Rollback transaction
            transaction
                .rollback()
                .await
                .context("Failed to rollback SQL transaction after getting existing token")?;
        }
        // Subscriber does not exist
        None => {
            // Insert new subscriber into DB
            let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
                .await
                .context("Failed to insert new subscriber into the database")?;

            // Generate and insert subscription token into DB
            subscription_token = SubscriptionToken::generate();
            store_token(&mut transaction, subscriber_id, &subscription_token)
                .await
                .context("Failed to store the confirmation token for a new subscriber")?;

            // Commit transaction
            transaction
                .commit()
                .await
                .context("Failed to commit SQL transaction to store a new subscriber")?;
        }
    }

    // Send confirmation email with subscription token
    send_confirmation_email(
        &email_client,
        &new_subscriber,
        &app_base_url,
        &subscription_token,
    )
    .await
    .context("Failed to send a new confirmation email")?;

    Ok(())
}

struct ExistingSubscriber {
    id: uuid::Uuid,
    name: Name,
    _email: Email,
    status: SubscriptionStatus,
}

#[tracing::instrument(name = "Get existing subscriber using email", skip(transaction, email))]
async fn get_existing_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    email: &Email,
) -> Result<Option<ExistingSubscriber>, anyhow::Error> {
    let result = sqlx::query!(
        "SELECT id, name, email, status FROM subscriptions \
        WHERE email = $1",
        email.as_ref()
    )
    .fetch_optional(&mut **transaction)
    .await?;

    match result {
        Some(r) => Ok(Some(ExistingSubscriber {
            id: r.id,
            name: Name::parse(&r.name)?,
            _email: Email::parse(&r.email)?,
            status: r.status.try_into()?,
        })),
        None => Ok(None),
    }
}

#[tracing::instrument(
    name = "Get existing token using subscriber id",
    skip(transaction, subscriber_id)
)]
async fn get_existing_subscription_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
) -> Result<SubscriptionToken, anyhow::Error> {
    let result = sqlx::query!(
        "SELECT subscription_token FROM subscription_tokens \
        WHERE subscriber_id = $1",
        subscriber_id
    )
    .fetch_one(&mut **transaction)
    .await?;

    Ok(SubscriptionToken::parse(&result.subscription_token)?)
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
    skip(email_client, subscriber, app_base_url, subscription_token)
)]
async fn send_confirmation_email(
    email_client: &EmailClient,
    subscriber: &NewSubscriber,
    app_base_url: &Url,
    subscription_token: &SubscriptionToken,
) -> Result<(), SendEmailError> {
    // The confirmation link should be `<BASE_URL>/subscribe/confirm?subscription_token=<TOKEN>`
    let mut confirmation_link = app_base_url.join("subscribe/confirm").unwrap(); // safely unwrap since it's proper url
    confirmation_link.set_query(Some(&format!(
        "subscription_token={}",
        subscription_token.as_str()
    )));

    let html_body = template::confirmation_email_html(&subscriber.name, &confirmation_link);
    let plain_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link,
    );

    email_client
        .send_email(&subscriber.email, "Welcome!", &html_body, &plain_body)
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
        r#"
        INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)
        "#,
        subscription_token.as_str(),
        subscriber_id,
    )
    .execute(&mut **transaction)
    .await?;

    Ok(())
}
