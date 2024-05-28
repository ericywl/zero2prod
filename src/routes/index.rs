use axum::response::{Html, IntoResponse};
use axum_flash::IncomingFlashes;

use crate::template;

pub async fn index(flashes: IncomingFlashes) -> impl IntoResponse {
    let success_msg = flashes
        .iter()
        .find(|(l, _)| l == &axum_flash::Level::Success)
        .map(|(_, m)| m.to_string());
    let error_msg = flashes
        .iter()
        .find(|(l, _)| l == &axum_flash::Level::Error)
        .map(|(_, m)| m.to_string());

    (flashes, Html(template::index_html(success_msg, error_msg)))
}
