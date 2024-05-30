use anyhow::Context;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{session_state::TypedSession, startup::AppState, telemetry};

#[derive(thiserror::Error)]
pub enum AdminDashboardError {
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for AdminDashboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        telemetry::error_chain_fmt(self, f)
    }
}

impl IntoResponse for AdminDashboardError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::UnexpectedError(e) => {
                // Log unexpected error
                tracing::error!("{:?}", e);

                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong with admin dashboard".to_string(),
                )
                    .into_response()
            }
        }
    }
}

pub async fn admin_dashboard(
    State(AppState { db_pool, .. }): State<AppState>,
    session: TypedSession,
) -> Result<Response, AdminDashboardError> {
    let user_id = session
        .get_user_id()
        .await
        .map_err(|e| AdminDashboardError::UnexpectedError(e.into()))?;

    let username = match user_id {
        Some(id) => get_username(&db_pool, id).await?,
        None => return Ok(Redirect::to("/login").into_response()),
    };

    Ok(Html(format!(
        r#"<!DOCTYPE html>
        <html lang="en">
        <head>
        <meta http-equiv="content-type" content="text/html; charset=utf-8">
        <title>Admin dashboard</title>
        </head>
        <body>
        <p>Welcome {username}!</p>
        </body>
        </html>"#
    ))
    .into_response())
}

#[tracing::instrument(name = "Get username", skip(db_pool))]
async fn get_username(db_pool: &PgPool, user_id: Uuid) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT username
        FROM users
        WHERE user_id = $1
        "#,
        user_id,
    )
    .fetch_one(db_pool)
    .await
    .context("Failed to perform a query to retrieve a username")?;
    Ok(row.username)
}
