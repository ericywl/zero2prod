use anyhow::Context;
use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use base64::Engine;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use sha3::Digest;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::{Email, SubscriptionStatus};
use crate::startup::AppState;
use crate::telemetry;

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
    AuthenticationError(#[source] anyhow::Error),

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
            Self::AuthenticationError(_) => {
                // Auth error, ignore logging
                let mut response = (
                    StatusCode::UNAUTHORIZED,
                    "Authentication failed".to_string(),
                )
                    .into_response();

                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);
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

pub async fn publish_newsletter(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<BodyData>,
) -> Result<(), PublishError> {
    let credentials = basic_authentication(&headers).map_err(PublishError::AuthenticationError)?;
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    let user_id = validate_credentials(&state.db_pool, &credentials).await?;
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    let subscribers = get_confirmed_subscribers(&state.db_pool)
        .await
        .context("Failed to get confirmed subscribers from the database")?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                state
                    .email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })?;
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

struct Credentials {
    username: String,
    password: Secret<String>,
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header was missing")?
        .to_str()
        .context("The 'Authorization' header was not a valid UTF-8 string")?;
    let base64_segment = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'")?;
    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_segment)
        .context("Failed to decode base64 'Basic' credentials")?;
    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not valid UTF-8")?;

    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth."))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth."))?
        .to_string();

    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}

async fn validate_credentials(
    pool: &PgPool,
    credentials: &Credentials,
) -> Result<Uuid, PublishError> {
    let password_hash = sha3::Sha3_256::digest(credentials.password.expose_secret().as_bytes());
    let password_hash = format!("{:x}", password_hash);

    let user_id: Option<_> = sqlx::query!(
        r#"SELECT user_id FROM users
        WHERE username = $1 AND password_hash = $2"#,
        credentials.username,
        password_hash
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform query to validate auth credentials")
    .map_err(PublishError::UnexpectedError)?;

    user_id
        .map(|r| r.user_id)
        .ok_or_else(|| anyhow::anyhow!("Invalid username or password"))
        .map_err(PublishError::AuthenticationError)
}
