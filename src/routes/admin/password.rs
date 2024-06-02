use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use axum_flash::{Flash, IncomingFlashes};
use secrecy::{ExposeSecret, SecretString};

use crate::{
    authentication, database::user_db, session_state::TypedSession, startup::AppState, telemetry,
    template,
};

#[derive(thiserror::Error)]
pub enum ChangePasswordError {
    #[error("The current password is incorrect")]
    IncorrectPassword,

    #[error("You entered two different new passwords")]
    DifferentNewPasswords,

    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for ChangePasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        telemetry::error_chain_fmt(self, f)
    }
}

impl IntoResponse for ChangePasswordError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::IncorrectPassword => {
                (StatusCode::UNAUTHORIZED, "Incorrect password".to_string()).into_response()
            }

            Self::DifferentNewPasswords => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Different new passwords".to_string(),
            )
                .into_response(),

            Self::UnexpectedError(e) => {
                // Log unexpected error
                tracing::error!("{:?}", e);

                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong with change password".to_string(),
                )
                    .into_response()
            }
        }
    }
}

pub async fn change_password_form(
    flashes: IncomingFlashes,
    session: TypedSession,
) -> Result<Response, ChangePasswordError> {
    if session
        .get_user_id()
        .await
        .map_err(|e| ChangePasswordError::UnexpectedError(e.into()))?
        .is_none()
    {
        return Ok(Redirect::to("/login").into_response());
    }

    let error_msg = flashes
        .iter()
        // We only have at most 1 error message
        .find(|(l, _)| l == &axum_flash::Level::Error)
        .map(|(_, m)| m.to_string());

    Ok((
        flashes,
        Html(template::admin_change_password_html(error_msg)),
    )
        .into_response())
}

#[derive(serde::Deserialize)]
pub struct ChangePasswordFormData {
    current_password: SecretString,
    new_password: SecretString,
    new_password_check: SecretString,
}

pub async fn change_password_with_flash(
    state: State<AppState>,
    flash: Flash,
    session: TypedSession,
    form: Form<ChangePasswordFormData>,
) -> Response {
    match change_password(state, session, form).await {
        Ok(r) => (flash, r).into_response(),
        Err(e) => match e {
            ChangePasswordError::DifferentNewPasswords | ChangePasswordError::IncorrectPassword => {
                (flash.error(e.to_string()), Redirect::to("/admin/password")).into_response()
            }
            _ => e.into_response(),
        },
    }
}

pub async fn change_password(
    State(AppState { db_pool, .. }): State<AppState>,
    session: TypedSession,
    Form(data): Form<ChangePasswordFormData>,
) -> Result<Response, ChangePasswordError> {
    let user_id = session
        .get_user_id()
        .await
        .map_err(|e| ChangePasswordError::UnexpectedError(e.into()))?;
    // Redirect to login is not logged in
    if user_id.is_none() {
        return Ok(Redirect::to("/login").into_response());
    }
    let user_id = user_id.unwrap();

    // New passwords mismatch
    if data.new_password.expose_secret() != data.new_password_check.expose_secret() {
        return Err(ChangePasswordError::DifferentNewPasswords);
    }

    // Validate current password
    let username = user_db::get_username(&db_pool, user_id)
        .await
        .map_err(ChangePasswordError::UnexpectedError)?;
    let credentials = authentication::Credentials {
        username,
        password: data.current_password,
    };
    authentication::validate_credentials(&db_pool, credentials)
        .await
        .map_err(|e| match e {
            authentication::AuthError::InvalidCredentials(_) => {
                ChangePasswordError::IncorrectPassword
            }
            authentication::AuthError::UnexpectedError(e) => {
                ChangePasswordError::UnexpectedError(e)
            }
        })?;

    todo!()
}
