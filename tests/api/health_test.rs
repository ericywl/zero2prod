use sqlx::PgPool;

#[cfg(test)]
#[sqlx::test]
async fn health_check_works(pool: PgPool) {
    // Arrange

    use crate::helpers;
    let setup = helpers::test_setup(pool).await;

    // Act
    let response = setup.server.get("/health").await;

    // Assert
    response.assert_status_ok();
    response.assert_text("");
}
