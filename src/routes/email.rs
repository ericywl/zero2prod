use std::{fs, path::Path};

use axum::Json;
use serde::{Deserialize, Serialize};

use super::handler_response::HandlerResponse;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EmailRequest {
    from: String,
    to: String,
    subject: String,
    html_body: String,
    text_body: String,
}

#[tracing::instrument(name = "Saving fake email request", skip(request))]
pub async fn fake_email(Json(request): Json<EmailRequest>) -> HandlerResponse {
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut buf = Vec::new();
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);

    if request
        .serialize(&mut ser)
        .inspect_err(|e| tracing::error!("Failed to format email request: {:?}", e))
        .is_err()
    {
        return HandlerResponse::server_error();
    }

    // Create directory if not exist
    if fs::create_dir_all(".fake_emails/")
        .inspect_err(|e| tracing::error!("Failed to create fake emails directory: {:?}", e))
        .is_err()
    {
        return HandlerResponse::server_error();
    };

    // Write fake email to file
    let pretty_request = String::from_utf8(buf).unwrap();
    if fs::write(Path::new(".fake_emails").join(request.to), pretty_request)
        .inspect_err(|e| tracing::error!("Failed to write fake email request: {:?}", e))
        .is_err()
    {
        return HandlerResponse::server_error();
    };

    HandlerResponse::ok()
}
