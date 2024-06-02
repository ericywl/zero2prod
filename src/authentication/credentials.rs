use anyhow::{Context, Ok};
use argon2::{password_hash::SaltString, Argon2, PasswordHasher, PasswordVerifier};
use secrecy::{ExposeSecret, Secret, SecretString};
use sqlx::PgPool;
use uuid::Uuid;

use crate::telemetry::{self, spawn_blocking_with_tracing};

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials(#[source] anyhow::Error),

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

#[tracing::instrument(name = "Validate credentials", skip(pool, credentials))]
pub async fn validate_credentials(
    pool: &PgPool,
    credentials: Credentials,
) -> Result<Uuid, AuthError> {
    // Have a fallback password hash so that we always perform the password hash verification.
    // This is so that we will not be susceptible to timing attacks (against username) as
    // the verification will always be done, albeit against a dummy password hash if user does
    // not exist.
    let mut user_id = None;
    let mut expected_password_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
gZiV/M1gPc22ElAH/Jh1Hw$\
CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );

    if let Some((stored_user_id, stored_password_hash)) =
        get_stored_credentials(pool, &credentials.username)
            .await
            .map_err(AuthError::UnexpectedError)?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    }

    let verify_result = telemetry::spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task.")
    .map_err(AuthError::UnexpectedError)?;

    verify_result?;

    // This is only set to `Some` if we found credentials in the store
    // So, even if the default password ends up matching (somehow) with the provided password,
    // we never authenticate a non-existing user.
    user_id.ok_or_else(|| AuthError::InvalidCredentials(anyhow::anyhow!("Unknown username.")))
}

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_password_hash, password_candidate)
)]
fn verify_password_hash(
    expected_password_hash: SecretString,
    password_candidate: SecretString,
) -> Result<(), AuthError> {
    let expected_password_hash = argon2::PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format")
        .map_err(AuthError::UnexpectedError)?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid password")
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(name = "Get stored credentials", skip(pool, username))]
async fn get_stored_credentials(
    pool: &PgPool,
    username: &str,
) -> Result<Option<(Uuid, SecretString)>, anyhow::Error> {
    let row: Option<_> = sqlx::query!(
        r#"SELECT user_id, password_hash FROM users
        WHERE username = $1"#,
        username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform query to validate auth credentials")?
    .map(|row| (row.user_id, Secret::new(row.password_hash)));

    Ok(row)
}

#[tracing::instrument(name = "Change password", skip(pool, password))]
pub async fn change_password(
    pool: &PgPool,
    user_id: Uuid,
    password: SecretString,
) -> Result<(), anyhow::Error> {
    let password_hash = spawn_blocking_with_tracing(move || compute_password_hash(password))
        .await?
        .context("Failed to hash password")?;

    sqlx::query!(
        r#"
        UPDATE users SET password_hash = $1
        WHERE user_id = $2
        "#,
        password_hash.expose_secret(),
        user_id
    )
    .execute(pool)
    .await
    .context("Failed to change user's password in the database")?;

    Ok(())
}

fn compute_password_hash(password: SecretString) -> Result<SecretString, anyhow::Error> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(15000, 2, 1, None).unwrap(),
    )
    .hash_password(password.expose_secret().as_bytes(), &salt)?
    .to_string();

    Ok(Secret::new(password_hash))
}
