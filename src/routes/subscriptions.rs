use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Form};
use chrono::Utc;
use serde::Deserialize;
use uuid::{NoContext, Timestamp, Uuid};

use super::super::startup::AppState;

#[derive(Debug, Deserialize)]
pub struct FormData {
    pub name: String,
    pub email: String,
}

pub async fn subscribe(
    State(state): State<Arc<AppState>>,
    Form(data): Form<FormData>,
) -> StatusCode {
    let mut connection = match state.db_pool.acquire().await {
        Ok(c) => c,
        Err(e) => {
            println!("Failed to get connection from pool: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v7(Timestamp::now(NoContext)),
        data.email,
        data.name,
        Utc::now()
    )
    .execute(&mut *connection)
    .await
    {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            println!("Failed to execute query: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
