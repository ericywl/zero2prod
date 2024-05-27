use axum::{
    extract::State,
    http::{header, HeaderValue},
    response::{Html, IntoResponse, Redirect},
    Form,
};
use axum_extra::extract::CookieJar;
use secrecy::SecretString;
use serde::Deserialize;

use crate::{authentication, startup::AppState, telemetry, template};

pub async fn login_form(cookie_jar: CookieJar) -> Html<String> {
    let error_msg = match cookie_jar.get("_flash") {
        None => None,
        Some(cookie) => Some(cookie.value().to_string()),
    };

    Html(template::login_page_html(error_msg))
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

impl IntoResponse for LoginError {
    fn into_response(self) -> axum::response::Response {
        // Redirect back to login but with error
        let mut response = Redirect::to("/login").into_response();
        response.headers_mut().insert(
            header::SET_COOKIE,
            HeaderValue::from_str(&format!("_flash={}", self)).unwrap(),
        );
        response
    }
}

#[tracing::instrument(skip(state, form), fields(username=tracing::field::Empty, user_id=tracing::field::Empty))]
pub async fn login(
    State(state): State<AppState>,
    Form(form): Form<LoginFormData>,
) -> Result<Redirect, LoginError> {
    let credentials: authentication::Credentials = form.into();
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    let user_id = authentication::validate_credentials(&state.db_pool, credentials)
        .await
        .map_err(|e| match e {
            authentication::AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
            authentication::AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
        })?;

    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
    Ok(Redirect::to("/"))
}
