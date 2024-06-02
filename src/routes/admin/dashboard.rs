use axum::{
    extract::State,
    response::{Html, IntoResponse, Response},
    Extension,
};

use crate::{
    authentication::UserId,
    database::user_db,
    domain::Name,
    startup::AppState,
    template,
    utils::{e500, InternalServerError},
};

pub async fn admin_dashboard(
    State(AppState { db_pool, .. }): State<AppState>,
    Extension(user_id): Extension<UserId>,
) -> Result<Response, InternalServerError> {
    let username = user_db::get_username(&db_pool, *user_id).await?;

    let name = Name::parse(&username).map_err(e500)?;
    Ok(Html(template::admin_dashboard_html(&name)).into_response())
}
