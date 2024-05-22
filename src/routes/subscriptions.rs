use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Form};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use thiserror::Error;
use uuid::{NoContext, Timestamp, Uuid};

use crate::domain::{Email, Name, NewSubscriber, ParseEmailError, ParseNameError};
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
        Err(_) => return StatusCode::UNPROCESSABLE_ENTITY,
    };

    match insert_subscriber(&state.db_pool, &new_subscriber).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
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
            INSERT INTO subscriptions (id, name, email, subscribed_at)
            VALUES ($1, $2, $3, $4)
            "#,
        Uuid::new_v7(Timestamp::now(NoContext)),
        new_subscriber.name.as_ref(),
        new_subscriber.email.as_ref(),
        Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}
