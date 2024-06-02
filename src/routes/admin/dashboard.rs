use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};

use crate::{
    database::user_db, domain::Name, session_state::TypedSession, startup::AppState, telemetry,
    template,
};

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
        Some(id) => user_db::get_username(&db_pool, id).await?,
        None => return Ok(Redirect::to("/login").into_response()),
    };

    let name =
        Name::parse(&username).map_err(|e| AdminDashboardError::UnexpectedError(e.into()))?;
    Ok(Html(template::admin_dashboard_html(&name)).into_response())
}
