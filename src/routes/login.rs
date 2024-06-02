use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect},
    Form,
};
use axum_flash::{Flash, IncomingFlashes};
use secrecy::SecretString;
use serde::Deserialize;

use crate::{authentication, session_state::TypedSession, startup::AppState, telemetry, template};

pub async fn login_form(flashes: IncomingFlashes) -> impl IntoResponse {
    let error_msg = flashes
        .iter()
        // We only have at most 1 error message
        .find(|(l, _)| l == &axum_flash::Level::Error)
        .map(|(_, m)| m.to_string());

    (flashes, Html(template::login_html(error_msg)))
}

#[derive(Deserialize)]
pub struct LoginFormData {
    username: String,
    password: SecretString,
}

impl From<LoginFormData> for authentication::Credentials {
    fn from(val: LoginFormData) -> authentication::Credentials {
        authentication::Credentials {
            username: val.username,
            password: val.password,
        }
    }
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),

    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        telemetry::error_chain_fmt(self, f)
    }
}

pub async fn login_with_flash(
    state: State<AppState>,
    flash: Flash,
    session: TypedSession,
    form: Form<LoginFormData>,
) -> impl IntoResponse {
    match login(state, session, form).await {
        Ok(()) => (flash, Redirect::to("/admin/dashboard")),
        // Redirect back to login page with flash message
        Err(e) => (flash.error(e.to_string()), Redirect::to("/login")),
    }
}

#[tracing::instrument(skip(db_pool, session, data), fields(username=tracing::field::Empty, user_id=tracing::field::Empty))]
pub async fn login(
    State(AppState { db_pool, .. }): State<AppState>,
    session: TypedSession,
    Form(data): Form<LoginFormData>,
) -> Result<(), LoginError> {
    let credentials: authentication::Credentials = data.into();
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    let user_id = authentication::validate_credentials(&db_pool, credentials)
        .await
        .map_err(|e| match e {
            authentication::AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
            authentication::AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
        })?;

    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
    session
        .insert_user_id(user_id)
        .await
        .map_err(|e| LoginError::UnexpectedError(e.into()))?;
    Ok(())
}
