use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::utils::InternalServerError;

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
pub async fn fake_email(Json(request): Json<EmailRequest>) -> Result<(), InternalServerError> {
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut buf = Vec::new();
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);

    request
        .serialize(&mut ser)
        .context("Failed to format email request")
        .map_err(InternalServerError)?;

    // Create directory if not exist
    fs::create_dir_all(".fake_emails/")
        .context("Failed to create fake emails directory")
        .map_err(InternalServerError)?;

    // Write fake email to file
    let unix_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Unexpected time error");

    let pretty_request = String::from_utf8(buf).unwrap();
    fs::write(
        Path::new(".fake_emails").join(format!("{}__{}.json", unix_time.as_millis(), request.to)),
        pretty_request,
    )
    .context("Failed to write fake email request")
    .map_err(InternalServerError)?;

    Ok(())
}
