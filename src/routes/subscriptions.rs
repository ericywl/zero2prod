use axum::{http::StatusCode, Form};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct FormData {
    pub name: String,
    pub email: String,
}

pub async fn subscribe(Form(data): Form<FormData>) -> StatusCode {
    println!("{} {}", data.name, data.email);
    StatusCode::OK
}
