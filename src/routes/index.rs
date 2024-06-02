use axum::response::{Html, IntoResponse};
use axum_flash::IncomingFlashes;

use crate::{session_state::TypedSession, template};

pub async fn index(flashes: IncomingFlashes, session: TypedSession) -> impl IntoResponse {
    let success_msg = flashes
        .iter()
        .find(|(l, _)| l == &axum_flash::Level::Success)
        .map(|(_, m)| m.to_string());
    let error_msg = flashes
        .iter()
        .find(|(l, _)| l == &axum_flash::Level::Error)
        .map(|(_, m)| m.to_string());
    let user_id = session.get_user_id().await.unwrap_or(None);

    (
        flashes,
        Html(template::index_html(user_id, success_msg, error_msg)),
    )
}
