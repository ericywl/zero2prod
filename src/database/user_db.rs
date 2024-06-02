use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

#[tracing::instrument(name = "Get username", skip(db_pool))]
pub async fn get_username(db_pool: &PgPool, user_id: Uuid) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT username
        FROM users
        WHERE user_id = $1
        "#,
        user_id,
    )
    .fetch_one(db_pool)
    .await
    .context("Failed to perform a query to retrieve a username")?;
    Ok(row.username)
}
