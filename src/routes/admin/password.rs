use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Extension, Form,
};
use axum_flash::{Flash, IncomingFlashes};
use secrecy::{ExposeSecret, SecretString};

use crate::{
    authentication::{self, UserId},
    database::user_db,
    startup::AppState,
    telemetry, template,
    utils::{get_success_and_error_flash_message, InternalServerError},
};

pub async fn change_password_form(flashes: IncomingFlashes) -> impl IntoResponse {
    let (success_msg, error_msg) = get_success_and_error_flash_message(&flashes);
    (
        flashes,
        Html(template::admin_change_password_html(success_msg, error_msg)),
    )
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
    user_id_ext: Extension<UserId>,
    form: Form<ChangePasswordFormData>,
) -> Response {
    match change_password(state, user_id_ext, form).await {
        Ok(_) => (
            flash.success("Your password has been changed"),
            Redirect::to("/admin/password"),
        )
            .into_response(),
        Err(e) => match e {
            ChangePasswordError::DifferentNewPasswords | ChangePasswordError::IncorrectPassword => {
                (flash.error(e.to_string()), Redirect::to("/admin/password")).into_response()
            }
            _ => e.into_response(),
        },
    }
}

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
                (StatusCode::UNAUTHORIZED, "Authentication error".to_string()).into_response()
            }

            Self::DifferentNewPasswords => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Different new passwords".to_string(),
            )
                .into_response(),

            Self::UnexpectedError(e) => InternalServerError(e).into_response(),
        }
    }
}

pub async fn change_password(
    State(AppState { db_pool, .. }): State<AppState>,
    Extension(user_id): Extension<UserId>,
    Form(data): Form<ChangePasswordFormData>,
) -> Result<(), ChangePasswordError> {
    // New passwords mismatch
    if data.new_password.expose_secret() != data.new_password_check.expose_secret() {
        return Err(ChangePasswordError::DifferentNewPasswords);
    }

    // Validate current password
    let username = user_db::get_username(&db_pool, *user_id)
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

    authentication::change_password(&db_pool, *user_id, data.new_password)
        .await
        .map_err(ChangePasswordError::UnexpectedError)?;

    Ok(())
}
