use std::{fs, path::Path};

use anyhow::Context;
use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use crate::telemetry;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EmailRequest {
    from: String,
    to: String,
    subject: String,
    html_body: String,
    text_body: String,
}

#[derive(thiserror::Error)]
pub enum FakeEmailError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for FakeEmailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        telemetry::error_chain_fmt(self, f)
    }
}

impl IntoResponse for FakeEmailError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::UnexpectedError(e) => {
                // Log unexpected error
                tracing::error!("{:?}", e);

                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong with saving fake email".to_string(),
                )
                    .into_response()
            }
        }
    }
}

#[tracing::instrument(name = "Saving fake email request", skip(request))]
pub async fn fake_email(Json(request): Json<EmailRequest>) -> Result<(), FakeEmailError> {
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut buf = Vec::new();
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);

    request
        .serialize(&mut ser)
        .context("Failed to format email request")?;

    // Create directory if not exist
    fs::create_dir_all(".fake_emails/").context("Failed to create fake emails directory")?;

    // Write fake email to file
    let pretty_request = String::from_utf8(buf).unwrap();
    fs::write(Path::new(".fake_emails").join(request.to), pretty_request)
        .context("Failed to write fake email request")?;

    Ok(())
}
