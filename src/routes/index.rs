use axum::response::{Html, IntoResponse};
use axum_flash::IncomingFlashes;

use crate::{session_state::TypedSession, template};

use super::utils::get_success_and_error_flash_message;

pub async fn index(flashes: IncomingFlashes, session: TypedSession) -> impl IntoResponse {
    let user_id = session.get_user_id().await.unwrap_or(None);
    let (success_msg, error_msg) = get_success_and_error_flash_message(&flashes);
    (
        flashes,
        Html(template::index_html(user_id, success_msg, error_msg)),
    )
}
