use axum::{http::StatusCode, response::IntoResponse};
use axum_flash::IncomingFlashes;

pub struct InternalServerError(pub anyhow::Error);

impl From<anyhow::Error> for InternalServerError {
    fn from(value: anyhow::Error) -> Self {
        Self(value)
    }
}

impl IntoResponse for InternalServerError {
    fn into_response(self) -> axum::response::Response {
        // Log unexpected error
        tracing::error!("{:?}", self.0);

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Something went wrong".to_string(),
        )
            .into_response()
    }
}

pub fn e500<T>(e: T) -> InternalServerError
where
    T: std::error::Error + Sync + Send + 'static,
{
    InternalServerError(e.into())
}

pub fn get_success_and_error_flash_message(
    flashes: &IncomingFlashes,
) -> (Option<String>, Option<String>) {
    let success_msg = flashes
        .iter()
        .find(|(l, _)| l == &axum_flash::Level::Success)
        .map(|(_, m)| m.to_string());
    let error_msg = flashes
        .iter()
        .find(|(l, _)| l == &axum_flash::Level::Error)
        .map(|(_, m)| m.to_string());

    (success_msg, error_msg)
}
